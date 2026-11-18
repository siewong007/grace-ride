//! 登入与账号安全: invites, passkeys (WebAuthn), password + TOTP 2FA, account recovery.

mod invites_recovery;
mod passkeys;
mod totp;

pub use invites_recovery::TERMS_VERSION;

use std::net::SocketAddr;
use std::sync::OnceLock;

use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};
use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::{json, Value};
use totp_rs::{Secret, Totp};

use crate::auth::{hash_token, CurrentUser, SESSION_COOKIE, SESSION_COOKIE_SECURE};
use crate::error::{bad, not_found, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms;
use crate::state::AppState;
use crate::util::{audit, in_hours, now, random_token, Lang};

/// Invite lookup by the token in the link. Recovery links sign their holder in as an
/// existing member, so only their SHA-256 is stored ($2); a leaked database or backup
/// can't be replayed. Ordinary invites ($1) only open a pending profile and are kept as
/// is so admins can copy the link again. The kind check stops a stored hash being
/// presented as if it were a token.
pub(super) const INVITE_BY_TOKEN: &str =
    "((kind <> 'recovery' AND token = $1) OR (kind = 'recovery' AND token = $2))";

/// Password login is disabled. All authentication is via passkey (WebAuthn).
/// Per-account lockout does not apply to passkey auth.
#[allow(dead_code)]
const MAX_FAILURES: i64 = 0; // disable password lockout since password login is disabled

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/login", post(login_password))
        .route("/auth/dev-users", get(dev_users))
        .route("/auth/dev-login", post(dev_login))
        .route("/auth/logout", post(logout))
        .route("/auth/logout-all", post(logout_all))
        .route("/auth/lock", post(totp::lock_session))
        .route("/auth/unlock", post(totp::unlock_session))
        .route("/auth/terms", post(invites_recovery::accept_terms))
        .route(
            "/auth/passkey/login/start",
            post(passkeys::passkey_login_start),
        )
        .route(
            "/auth/passkey/login/finish",
            post(passkeys::passkey_login_finish),
        )
        .route(
            "/auth/passkey/reauth/start",
            post(passkeys::passkey_reauth_start),
        )
        .route(
            "/auth/passkey/reauth/finish",
            post(passkeys::passkey_reauth_finish),
        )
        .route(
            "/auth/passkey/register/start",
            post(passkeys::passkey_register_start),
        )
        .route(
            "/auth/passkey/register/finish",
            post(passkeys::passkey_register_finish),
        )
        .route("/auth/passkeys", get(passkeys::list_passkeys))
        .route("/auth/passkeys/{id}", delete(passkeys::delete_passkey))
        .route("/auth/totp/setup", post(totp::totp_setup))
        .route("/auth/totp/enable", post(totp::totp_enable))
        .route("/auth/totp/verify", post(totp::totp_verify))
        .route(
            "/auth/totp/backup-codes/regenerate",
            post(totp::totp_regenerate_backup_codes),
        )
        .route("/auth/password", post(totp::set_password))
        .route(
            "/auth/recovery-request",
            post(invites_recovery::recovery_request),
        )
        .route("/invites/{token}", get(invites_recovery::invite_preview))
        .route(
            "/invites/{token}/accept",
            post(invites_recovery::invite_accept),
        )
}

pub fn hash_password(pw: &str) -> AppResult<String> {
    Argon2::default()
        .hash_password(pw.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| AppError::Other(anyhow::anyhow!("hash: {e}")))
}

pub(super) fn verify_password(pw: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(pw.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Verifying against a fixed dummy hash when the account does not exist keeps response
/// time the same, so attackers cannot discover which emails are members.
#[allow(dead_code)]
fn dummy_hash() -> &'static str {
    static H: OnceLock<String> = OnceLock::new();
    H.get_or_init(|| hash_password("dummy-password-for-timing").unwrap_or_default())
}

/// Accepts the code for the previous, current or next 30 s step, and refuses any step
/// at or before the last one used (no replay of an observed code).
pub async fn verify_totp(
    st: &AppState,
    user_id: i64,
    secret_b32: &str,
    email: &str,
    code: &str,
    lang: Lang,
) -> AppResult<()> {
    let totp = totp_for(secret_b32, email, lang)?;
    let (last,): (i64,) = sqlx::query_as("SELECT totp_last_step FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&st.db)
        .await?;
    let now_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?
        .as_secs();
    let code = code.trim();
    if let Some(matched_step) = totp.check(code, now_s) {
        let step = matched_step as i64;
        if step <= last {
            return Err(bad(lang, ErrorKey::CodeAlreadyUsed));
        }
        let updated = sqlx::query(
            "UPDATE users SET totp_last_step=$1 WHERE id=$2 AND COALESCE(totp_last_step, -1) < $1",
        )
        .bind(step)
        .bind(user_id)
        .execute(&st.db)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(bad(lang, ErrorKey::CodeAlreadyUsed));
        }
        return Ok(());
    }
    Err(bad(lang, ErrorKey::InvalidCode))
}

pub(super) fn totp_for(secret_b32: &str, email: &str, lang: Lang) -> AppResult<Totp> {
    let secret =
        Secret::try_from_base32(secret_b32).map_err(|_| bad(lang, ErrorKey::Invalid2FaSecret))?;
    totp_rs::Builder::new()
        .with_secret(secret)
        .with_issuer(Some("GraceRide"))
        .with_account_name(email)
        .build()
        .map_err(|e| AppError::Other(anyhow::anyhow!("totp: {e}")))
}

/// Members stay signed in for 30 days on their own phone; accounts that can see other
/// members' data (管理员 / 交通组 / 财务) get 12-hour sessions.
pub(super) async fn create_session(
    state: &AppState,
    jar: CookieJar,
    user_id: i64,
    mfa_ok: bool,
) -> AppResult<CookieJar> {
    let (roles,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    let hours: i64 = if perms::is_privileged(&state.db, &roles).await? {
        12
    } else {
        24 * 30
    };
    let token = random_token(48);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(hash_token(&token))
    .bind(user_id)
    .bind(mfa_ok as i64)
    .bind(in_hours(hours))
    .bind(now())
    .execute(&state.db)
    .await?;
    let secure = state.cfg.secure_cookies();
    let cookie = Cookie::build((
        if secure {
            SESSION_COOKIE_SECURE
        } else {
            SESSION_COOKIE
        },
        token,
    ))
    .http_only(true)
    .same_site(SameSite::Lax)
    .secure(secure)
    .path("/")
    .max_age(time::Duration::hours(hours))
    .build();
    Ok(jar.add(cookie))
}

fn clear_cookies(jar: CookieJar) -> CookieJar {
    jar.remove(Cookie::build(SESSION_COOKIE).path("/").build())
        .remove(
            Cookie::build(SESSION_COOKIE_SECURE)
                .path("/")
                .secure(true)
                .build(),
        )
}

#[derive(sqlx::FromRow)]
struct MeRow {
    phone: String,
    photo_file_id: Option<i64>,
    totp_enabled: i64,
    password_hash: Option<String>,
    badges_opt_out: i64,
    carpool_enabled: i64,
    spare_seats: i64,
    terms_accepted_at: Option<String>,
    readiness_prompt_mins: Option<i64>,
    gate_wait_mins: Option<i64>,
    auto_lock_mins: Option<i64>,
    totp_backup_codes: Option<String>,
    terms_version: i32,
    telegram_chat_id: String,
    whatsapp_opt_in: i64,
    push_opt_in: i64,
    caregiver_user_id: Option<i64>,
}

async fn me(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let row: MeRow = sqlx::query_as(
        "SELECT phone, photo_file_id, totp_enabled, password_hash, badges_opt_out, carpool_enabled, spare_seats, terms_accepted_at, readiness_prompt_mins, gate_wait_mins, auto_lock_mins, totp_backup_codes, terms_version, telegram_chat_id, whatsapp_opt_in, push_opt_in, caregiver_user_id FROM users WHERE id=$1",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    let _ = crate::security::evaluate_session_lock(&st, &u.session_id, false).await;
    let (locked_at,): (Option<String>,) =
        sqlx::query_as("SELECT locked_at FROM sessions WHERE token=$1")
            .bind(&u.session_id)
            .fetch_one(&st.db)
            .await?;
    let backup_codes_count = row
        .totp_backup_codes
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .map(|v| v.len())
        .unwrap_or(0);
    let passkeys: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM passkeys WHERE user_id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    let unread: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1 AND read_at IS NULL")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let groups: Vec<(i64, String)> = sqlx::query_as(
        "SELECT g.id, g.name FROM groups g JOIN group_members m ON m.group_id=g.id WHERE m.user_id=$1 ORDER BY g.name",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    // Built-in roles are named by the app in the member's language; custom ones carry
    // the name their administrator typed.
    let custom: Vec<(String, String)> =
        sqlx::query_as("SELECT key, name FROM roles WHERE key = ANY($1) AND is_system = 0")
            .bind(&u.roles)
            .fetch_all(&st.db)
            .await?;
    let role_names: serde_json::Map<String, Value> =
        custom.into_iter().map(|(k, n)| (k, json!(n))).collect();
    let session_timeout: i64 =
        sqlx::query_scalar("SELECT value FROM system_settings WHERE key = 'session_timeout_mins'")
            .fetch_optional(&st.db)
            .await?
            .and_then(|v: String| v.parse::<i64>().ok())
            .unwrap_or(st.cfg.session_timeout_mins);
    Ok(Json(json!({
        "id": u.id, "name": u.name, "email": u.email, "phone": row.phone, "photo_file_id": row.photo_file_id,
        "status": u.status,
        "lang": u.lang.code(),
        "roles": u.roles, "role_names": role_names,
        // What this session may do, as '<page>.<action>'. Staff permissions are left out
        // until the member signs in with a passkey or 2FA (`privileged` says they exist).
        "permissions": u.perms.usable_keys(u.mfa_ok),
        "mfa_ok": u.mfa_ok, "totp_enabled": row.totp_enabled == 1,
        "has_password": row.password_hash.is_some(), "passkey_count": passkeys.0,
        "badges_opt_out": row.badges_opt_out == 1, "carpool_enabled": row.carpool_enabled == 1, "spare_seats": row.spare_seats,
        "readiness_prompt_mins": row.readiness_prompt_mins.unwrap_or(45), "gate_wait_mins": row.gate_wait_mins.unwrap_or(5),
        "auto_lock_mins": row.auto_lock_mins.unwrap_or(0), "session_timeout_mins": session_timeout, "backup_codes_count": backup_codes_count,
        "locked": locked_at.is_some(),
        "telegram_chat_id": row.telegram_chat_id,
        "whatsapp_opt_in": row.whatsapp_opt_in == 1,
        "push_opt_in": row.push_opt_in == 1,
        "caregiver_user_id": row.caregiver_user_id,
        "terms_accepted": row.terms_accepted_at.is_some() && row.terms_version >= TERMS_VERSION,
        "terms_version": TERMS_VERSION,
        "privileged": u.perms.privileged(), "unread": unread.0,
        "groups": groups.into_iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        "dev_mode": st.cfg.dev_mode,
    })))
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct LoginReq {
    email: String,
    password: String,
    code: Option<String>,
}

#[allow(dead_code)]
async fn record_failure(st: &AppState, email: &str, ip: &str) -> AppResult<()> {
    sqlx::query("INSERT INTO login_failures (email, ip, created_at) VALUES ($1,$2,$3)")
        .bind(email)
        .bind(ip)
        .bind(now())
        .execute(&st.db)
        .await?;
    Ok(())
}

async fn login_password(
    State(_st): State<AppState>,
    _peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    _jar: CookieJar,
    Json(_req): Json<LoginReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    // Password login is disabled. Members sign in with a passkey.
    let lang = Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    let _ = (_st, _req);
    Err(bad(lang, ErrorKey::AuthDisabled))
}

/// Demo accounts for local development. Production returns 404 and never lists members.
async fn dev_users(State(st): State<AppState>, headers: HeaderMap) -> AppResult<Json<Value>> {
    let lang = visitor_lang(&headers);
    if !st.cfg.dev_mode {
        return Err(not_found(lang));
    }
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT name, email, roles FROM users WHERE status='active' ORDER BY id")
            .fetch_all(&st.db)
            .await?;
    let users: Vec<Value> = rows
        .into_iter()
        .map(|(name, email, roles)| json!({ "name": name, "email": email, "roles": roles }))
        .collect();
    Ok(Json(json!(users)))
}

#[derive(Deserialize)]
struct DevLoginReq {
    email: String,
}

/// Sign in as a seeded test account. Only available when DEV_MODE is on.
async fn dev_login(
    State(st): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<DevLoginReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = visitor_lang(&headers);
    if !st.cfg.dev_mode {
        return Err(not_found(lang));
    }
    let row: Option<(i64, String)> = sqlx::query_as("SELECT id, status FROM users WHERE email=$1")
        .bind(req.email.trim().to_lowercase())
        .fetch_optional(&st.db)
        .await?;
    let Some((id, status)) = row else {
        return Err(bad(lang, ErrorKey::InvalidCredentials));
    };
    if status != "active" {
        return Err(bad(lang, ErrorKey::InvalidCredentials));
    }
    let jar = create_session(&st, jar, id, true).await?;
    audit(&st.db, Some(id), "login.dev", "user", Some(id), "").await;
    Ok((jar, Json(json!({ "ok": true }))))
}

pub(super) fn visitor_lang(headers: &HeaderMap) -> Lang {
    Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    )
}

async fn logout(
    State(st): State<AppState>,
    jar: CookieJar,
    u: CurrentUser,
) -> AppResult<(CookieJar, Json<Value>)> {
    sqlx::query("DELETE FROM sessions WHERE token=$1")
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "logout", "user", Some(u.id), "").await;
    Ok((clear_cookies(jar), Json(json!({"ok": true}))))
}

async fn logout_all(
    State(st): State<AppState>,
    jar: CookieJar,
    u: CurrentUser,
) -> AppResult<(CookieJar, Json<Value>)> {
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(u.id)
        .execute(&st.db)
        .await?;
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1")
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "logout.all", "user", Some(u.id), "").await;
    Ok((clear_cookies(jar), Json(json!({"ok": true}))))
}
