//! 登入与账号安全: invites, passkeys (WebAuthn), Google OAuth, password + TOTP 2FA, account recovery.

mod biometrics;
mod google;
mod invites_recovery;
mod passkeys;
mod totp;

pub use invites_recovery::TERMS_VERSION;

use std::{net::SocketAddr, sync::OnceLock, time::Duration};

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

use crate::auth::{
    hash_token, session_cookie_name, CurrentUser, SESSION_COOKIE, SESSION_COOKIE_SECURE,
};
use crate::error::{bad, forbidden, not_found, too_many_requests, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms;
use crate::state::AppState;
use crate::util::{audit, in_hours, minutes_ago, now, random_token, Lang};

/// Invite lookup by the token in the link. Recovery links sign their holder in as an
/// existing member, so only their SHA-256 is stored ($2); a leaked database or backup
/// can't be replayed. Ordinary invites ($1) only open a pending profile and are kept as
/// is so admins can copy the link again. The kind check stops a stored hash being
/// presented as if it were a token.
pub(super) const INVITE_BY_TOKEN: &str =
    "((kind <> 'recovery' AND token = $1) OR (kind = 'recovery' AND token = $2))";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/turnstile", get(turnstile_config))
        .route("/auth/google/config", get(google::google_config))
        .route("/auth/google/start", post(google::google_start))
        .route("/auth/google/link/start", post(google::google_link_start))
        .route("/auth/google/callback", get(google::google_callback))
        .route("/auth/google", delete(google::google_unlink))
        .route("/auth/google/totp/status", get(google::google_totp_status))
        .route(
            "/auth/google/totp/complete",
            post(google::google_totp_complete),
        )
        .route(
            "/auth/password-reset/status",
            get(google::password_reset_status),
        )
        .route(
            "/auth/password-reset/complete",
            post(google::password_reset_complete),
        )
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
        .route(
            "/auth/biometric/register/start",
            post(biometrics::biometric_register_start),
        )
        .route(
            "/auth/biometric/register/finish",
            post(biometrics::biometric_register_finish),
        )
        .route(
            "/auth/biometric/login/start",
            post(biometrics::biometric_login_start),
        )
        .route(
            "/auth/biometric/login/finish",
            post(biometrics::biometric_login_finish),
        )
        .route(
            "/auth/biometric/reauth/start",
            post(biometrics::biometric_reauth_start),
        )
        .route(
            "/auth/biometric/reauth/finish",
            post(biometrics::biometric_reauth_finish),
        )
        .route("/auth/biometrics", get(biometrics::list_biometrics))
        .route(
            "/auth/biometrics/{id}",
            delete(biometrics::delete_biometric),
        )
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

pub(super) const MAX_PASSWORD_BYTES: usize = 1024;

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

pub(super) async fn hash_password_async(pw: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || hash_password(&pw))
        .await
        .map_err(|e| AppError::Other(anyhow::anyhow!("argon2 task: {e}")))?
}

pub(super) async fn verify_password_async(pw: String, hash: String) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || verify_password(&pw, &hash))
        .await
        .map_err(|e| AppError::Other(anyhow::anyhow!("argon2 task: {e}")))
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

fn build_session_cookie(state: &AppState, token: String, hours: i64) -> Cookie<'static> {
    let secure = state.cfg.secure_cookies();
    Cookie::build((
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
    .build()
}

/// Members stay signed in for 30 days on their own phone; accounts that can see other
/// members' data (管理员 / 交通组 / 财务) get 12-hour sessions.
pub(super) async fn create_session(
    state: &AppState,
    jar: CookieJar,
    user_id: i64,
    mfa_ok: bool,
) -> AppResult<CookieJar> {
    let cookie_name = session_cookie_name(state.cfg.secure_cookies());
    if let Some(prev) = jar
        .get(cookie_name)
        .map(|c| c.value().to_string())
        .filter(|t| t.len() == 48)
    {
        sqlx::query("DELETE FROM sessions WHERE token=$1")
            .bind(hash_token(&prev))
            .execute(&state.db)
            .await?;
    }
    let jar = google::clear_pending_auth_cookies(jar);
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
    Ok(jar.add(build_session_cookie(state, token, hours)))
}

/// Rotate the current session token atomically upon privilege/MFA elevation or unlock,
/// preserving push subscription linkage before deleting the superseded session row.
pub(super) async fn rotate_session(
    state: &AppState,
    jar: CookieJar,
    u: &CurrentUser,
    mfa_ok: bool,
    clear_lock: bool,
    refresh_sign_in: bool,
) -> AppResult<CookieJar> {
    let mut tx = state.db.begin().await?;
    let (roles,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let role_list = perms::parse_roles(&roles);
    let hours: i64 = if perms::for_roles(&mut *tx, &role_list).await?.privileged() {
        12
    } else {
        24 * 30
    };
    let old_row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT created_at, expires_at, locked_at FROM sessions WHERE token=$1 FOR UPDATE",
    )
    .bind(&u.session_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((old_created_at, old_expires_at, old_locked_at)) = old_row else {
        return Err(crate::error::unauthorized(u.lang));
    };
    let ts_now = now();
    let fresh_exp = in_hours(hours);
    let new_created_at = if refresh_sign_in {
        ts_now.clone()
    } else {
        old_created_at
    };
    let new_expires_at = if refresh_sign_in {
        fresh_exp
    } else if old_expires_at < fresh_exp {
        old_expires_at
    } else {
        fresh_exp
    };
    let new_locked_at = if clear_lock { None } else { old_locked_at };

    let raw = random_token(48);
    let new_hash = hash_token(&raw);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at, locked_at, last_seen_at, failed_totp_attempts)
         VALUES ($1,$2,$3,$4,$5,$6,$7,0)",
    )
    .bind(&new_hash)
    .bind(u.id)
    .bind(mfa_ok as i64)
    .bind(&new_expires_at)
    .bind(&new_created_at)
    .bind(&new_locked_at)
    .bind(&ts_now)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE push_subscriptions SET session_token = $1 WHERE session_token = $2")
        .bind(&new_hash)
        .bind(&u.session_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(&u.session_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(jar.add(build_session_cookie(state, raw, hours)))
}

/// Revoke all other sessions for a user after credential/factor lifecycle changes.
pub(super) async fn revoke_other_sessions(
    db: &sqlx::PgPool,
    user_id: i64,
    keep_session_hash: &str,
) -> sqlx::Result<u64> {
    let res = sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token <> $2")
        .bind(user_id)
        .bind(keep_session_hash)
        .execute(db)
        .await?;
    Ok(res.rows_affected())
}

fn clear_cookies(jar: CookieJar) -> CookieJar {
    let jar = jar
        .remove(Cookie::build(SESSION_COOKIE).path("/").build())
        .remove(
            Cookie::build(SESSION_COOKIE_SECURE)
                .path("/")
                .secure(true)
                .build(),
        );
    google::clear_all_ephemeral_cookies(jar)
}

#[derive(sqlx::FromRow)]
struct MeRow {
    phone: String,
    photo_file_id: Option<i64>,
    avatar_color: Option<String>,
    avatar_icon: Option<String>,
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
    onboarding_dismissed_at: Option<String>,
    locked_at: Option<String>,
    passkey_count: i64,
    biometric_count: i64,
    google_email: Option<String>,
    unread_count: i64,
    org_timeout: Option<String>,
    groups_json: Value,
}

async fn me(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let _ = crate::security::evaluate_session_lock(&st, &u.session_id, false).await;
    let row: MeRow = sqlx::query_as(
        "SELECT u.phone, u.photo_file_id, u.avatar_color, u.avatar_icon, u.totp_enabled, u.password_hash,
                u.badges_opt_out, u.carpool_enabled, u.spare_seats, u.terms_accepted_at,
                u.readiness_prompt_mins, u.gate_wait_mins, u.auto_lock_mins, u.totp_backup_codes,
                u.terms_version, u.telegram_chat_id, u.whatsapp_opt_in, u.push_opt_in,
                u.caregiver_user_id, u.onboarding_dismissed_at,
                s.locked_at,
                (SELECT COUNT(*) FROM passkeys p WHERE p.user_id = u.id) AS passkey_count,
                (SELECT COUNT(*) FROM biometric_credentials b WHERE b.user_id = u.id) AS biometric_count,
                (SELECT oa.email FROM oauth_accounts oa WHERE oa.user_id = u.id AND oa.provider = 'google' AND lower(oa.email) = lower(u.email) LIMIT 1) AS google_email,
                (SELECT COUNT(*) FROM notifications n WHERE n.user_id = u.id AND n.read_at IS NULL) AS unread_count,
                (SELECT value FROM system_settings WHERE key = 'session_timeout_mins') AS org_timeout,
                COALESCE((
                    SELECT jsonb_agg(jsonb_build_object('id', g.id, 'name', g.name) ORDER BY g.name)
                    FROM groups g
                    JOIN group_members m ON m.group_id = g.id
                    WHERE m.user_id = u.id
                ), '[]'::jsonb) AS groups_json
         FROM users u
         JOIN sessions s ON s.token = $2
         WHERE u.id = $1",
    )
    .bind(u.id)
    .bind(&u.session_id)
    .fetch_one(&st.db)
    .await?;
    let backup_codes_count = row
        .totp_backup_codes
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .map(|v| v.len())
        .unwrap_or(0);
    // Built-in roles are named by the app in the member's language; custom ones carry
    // the name their administrator typed.
    let custom: Vec<(String, String)> = if u.roles.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as("SELECT key, name FROM roles WHERE key = ANY($1) AND is_system = 0")
            .bind(&u.roles)
            .fetch_all(&st.db)
            .await?
    };
    let role_names: serde_json::Map<String, Value> =
        custom.into_iter().map(|(k, n)| (k, json!(n))).collect();
    let session_timeout = row
        .org_timeout
        .as_deref()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(st.cfg.session_timeout_mins);
    if row.locked_at.is_some() {
        return Ok(Json(json!({
            "id": u.id, "name": u.name, "email": "", "phone": "", "photo_file_id": Value::Null,
            "avatar_color": row.avatar_color, "avatar_icon": row.avatar_icon,
            "status": u.status,
            "lang": u.lang.code(),
            "roles": Vec::<String>::new(), "role_names": serde_json::Map::new(),
            "permissions": Vec::<String>::new(),
            "mfa_ok": u.mfa_ok, "totp_enabled": row.totp_enabled == 1,
            "has_password": row.password_hash.is_some(), "passkey_count": row.passkey_count, "biometric_count": row.biometric_count,
            "google_verified": row.google_email.is_some(), "google_email": Value::Null,
            "badges_opt_out": row.badges_opt_out == 1, "carpool_enabled": row.carpool_enabled == 1, "spare_seats": row.spare_seats,
            "readiness_prompt_mins": row.readiness_prompt_mins.unwrap_or(45), "gate_wait_mins": row.gate_wait_mins.unwrap_or(5),
            "auto_lock_mins": row.auto_lock_mins.unwrap_or(0), "session_timeout_mins": session_timeout, "backup_codes_count": 0,
            "locked": true,
            "telegram_chat_id": "",
            "whatsapp_opt_in": false,
            "push_opt_in": false,
            "caregiver_user_id": Value::Null,
            "onboarding_dismissed": row.onboarding_dismissed_at.is_some(),
            "terms_accepted": row.terms_accepted_at.is_some() && row.terms_version >= TERMS_VERSION,
            "terms_version": TERMS_VERSION,
            "privileged": false, "unread": 0,
            "groups": Vec::<Value>::new(),
            "dev_mode": st.cfg.dev_mode,
        })));
    }
    Ok(Json(json!({
        "id": u.id, "name": u.name, "email": u.email, "phone": row.phone, "photo_file_id": row.photo_file_id,
        "avatar_color": row.avatar_color, "avatar_icon": row.avatar_icon,
        "status": u.status,
        "lang": u.lang.code(),
        "roles": u.roles, "role_names": role_names,
        // What this session may do, as '<page>.<action>'. Staff permissions are left out
        // until the member signs in with a passkey or 2FA (`privileged` says they exist).
        "permissions": u.perms.usable_keys(u.mfa_ok),
        "mfa_ok": u.mfa_ok, "totp_enabled": row.totp_enabled == 1,
        "has_password": row.password_hash.is_some(), "passkey_count": row.passkey_count, "biometric_count": row.biometric_count,
        "google_verified": row.google_email.is_some(), "google_email": row.google_email,
        "badges_opt_out": row.badges_opt_out == 1, "carpool_enabled": row.carpool_enabled == 1, "spare_seats": row.spare_seats,
        "readiness_prompt_mins": row.readiness_prompt_mins.unwrap_or(45), "gate_wait_mins": row.gate_wait_mins.unwrap_or(5),
        "auto_lock_mins": row.auto_lock_mins.unwrap_or(0), "session_timeout_mins": session_timeout, "backup_codes_count": backup_codes_count,
        "locked": row.locked_at.is_some(),
        "telegram_chat_id": row.telegram_chat_id,
        "whatsapp_opt_in": row.whatsapp_opt_in == 1,
        "push_opt_in": row.push_opt_in == 1,
        "caregiver_user_id": row.caregiver_user_id,
        "onboarding_dismissed": row.onboarding_dismissed_at.is_some(),
        "terms_accepted": row.terms_accepted_at.is_some() && row.terms_version >= TERMS_VERSION,
        "terms_version": TERMS_VERSION,
        "privileged": u.perms.privileged(), "unread": row.unread_count,
        "groups": row.groups_json,
        "dev_mode": st.cfg.dev_mode,
    })))
}

/// Public Turnstile configuration for the SPA (never exposes secret_key or verify_url).
async fn turnstile_config(State(st): State<AppState>) -> Json<Value> {
    let enabled = st.cfg.turnstile.is_enabled();
    Json(json!({
        "enabled": enabled,
        "site_key": if enabled { st.cfg.turnstile.site_key.as_deref() } else { None },
    }))
}

fn dummy_password_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| {
        hash_password("dummy-timing-guard-placeholder")
            .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string())
    })
}

pub(super) enum SecondFactorOutcome {
    Verified,
    InvalidCode,
}

/// Shared TOTP / backup-code check used by both password login and Google 2FA completion.
pub(super) async fn verify_totp_or_backup(
    st: &AppState,
    user_id: i64,
    totp_secret: Option<&str>,
    email: &str,
    raw_code: &str,
    lang: Lang,
) -> AppResult<SecondFactorOutcome> {
    if st
        .limiter
        .check(
            &format!("totp:user:{user_id}"),
            10,
            Duration::from_secs(300),
        )
        .is_err()
    {
        return Err(too_many_requests(lang, ErrorKey::RateLimited));
    }
    let clean = raw_code.trim();
    if clean.len() == 6 && clean.chars().all(|c| c.is_ascii_digit()) {
        if let Some(sec_raw) = totp_secret {
            let sec = crate::crypto::decrypt_secret(&format!("totp_secret:{user_id}"), sec_raw)
                .map_err(|_| bad(lang, ErrorKey::Invalid2FaSecret))?;
            match verify_totp(st, user_id, &sec, email, clean, lang).await {
                Ok(()) => return Ok(SecondFactorOutcome::Verified),
                Err(
                    err @ AppError::BadRequest {
                        key: ErrorKey::CodeAlreadyUsed,
                        ..
                    },
                ) => {
                    return Err(err);
                }
                Err(_) => {}
            }
        }
    }
    let norm = clean.replace(['-', ' '], "").to_uppercase();
    if norm.len() == 8 {
        match totp::consume_backup_code(st, user_id, &norm, lang).await {
            Ok(_) => return Ok(SecondFactorOutcome::Verified),
            Err(err @ AppError::TooManyRequests { .. }) => return Err(err),
            Err(_) => {}
        }
    }
    Ok(SecondFactorOutcome::InvalidCode)
}

#[derive(Deserialize)]
struct LoginReq {
    email: String,
    password: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    turnstile_token: Option<String>,
}

async fn login_password(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<LoginReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = visitor_lang(&headers);
    let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
    st.turnstile
        .verify(
            req.turnstile_token.as_deref(),
            "password_login",
            Some(&ip),
            lang,
        )
        .await?;

    let email = req.email.trim().to_lowercase();
    if email.is_empty()
        || email.len() > 254
        || req.password.is_empty()
        || req.password.len() > MAX_PASSWORD_BYTES
    {
        return Err(bad(lang, ErrorKey::InvalidCredentials));
    }

    let recent_failures: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM login_failures WHERE email = $1 AND created_at >= $2",
    )
    .bind(&email)
    .bind(minutes_ago(15))
    .fetch_one(&st.db)
    .await?;
    if recent_failures >= 5 {
        return Err(too_many_requests(lang, ErrorKey::TooManyFailedLogins));
    }
    let attempt_key = format!("pw_attempt:user:{email}");
    if st
        .limiter
        .check(&attempt_key, 5, Duration::from_secs(900))
        .is_err()
    {
        return Err(too_many_requests(lang, ErrorKey::TooManyFailedLogins));
    }

    let user_row: Option<(i64, String, Option<String>, i64, Option<String>)> = sqlx::query_as(
        "SELECT id, status, password_hash, totp_enabled, totp_secret
         FROM users
         WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&st.db)
    .await?;

    let (uid, status, stored_hash, totp_enabled, totp_secret) = match user_row {
        Some(r) => (Some(r.0), Some(r.1), r.2, r.3, r.4),
        None => (None, None, None, 0, None),
    };

    let hash_to_check = stored_hash
        .clone()
        .unwrap_or_else(|| dummy_password_hash().to_string());
    let pw_ok = verify_password_async(req.password, hash_to_check).await?;
    if !pw_ok || stored_hash.is_none() || uid.is_none() {
        sqlx::query("INSERT INTO login_failures (email, ip, created_at) VALUES ($1, $2, $3)")
            .bind(&email)
            .bind(&ip)
            .bind(now())
            .execute(&st.db)
            .await?;
        audit(
            &st.db,
            None,
            "password.failed",
            "user",
            uid,
            format!("from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::InvalidCredentials));
    }

    let uid = uid.unwrap_or_default();
    if !matches!(status.as_deref(), Some("active" | "pending")) {
        st.limiter.decrement(&attempt_key);
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            Some(uid),
            format!("account disabled; from {ip}"),
        )
        .await;
        return Err(forbidden(lang, ErrorKey::AuthDisabled));
    }

    let mut mfa_ok = false;
    if totp_enabled == 1 {
        let code = req.code.as_deref().map(str::trim).unwrap_or("");
        if code.is_empty() {
            st.limiter.decrement(&attempt_key);
            return Err(bad(lang, ErrorKey::TotpRequired));
        }
        match verify_totp_or_backup(&st, uid, totp_secret.as_deref(), &email, code, lang).await {
            Ok(SecondFactorOutcome::Verified) => mfa_ok = true,
            Ok(SecondFactorOutcome::InvalidCode) => {
                sqlx::query(
                    "INSERT INTO login_failures (email, ip, created_at) VALUES ($1, $2, $3)",
                )
                .bind(&email)
                .bind(&ip)
                .bind(now())
                .execute(&st.db)
                .await?;
                audit(&st.db, None, "totp.failed", "user", Some(uid), "login").await;
                return Err(bad(lang, ErrorKey::InvalidCode));
            }
            Err(err) => {
                st.limiter.decrement(&attempt_key);
                return Err(err);
            }
        }
    }

    st.limiter.reset(&attempt_key);
    sqlx::query("DELETE FROM login_failures WHERE email = $1")
        .bind(&email)
        .execute(&st.db)
        .await?;

    let jar = create_session(&st, jar, uid, mfa_ok).await?;
    audit(
        &st.db,
        Some(uid),
        "login.password",
        "user",
        Some(uid),
        if mfa_ok { "2fa" } else { "" },
    )
    .await;
    Ok((jar, Json(json!({ "ok": true }))))
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

#[derive(Deserialize)]
struct LogoutIn {
    push_endpoint: Option<String>,
    mobile_token: Option<String>,
}

async fn logout(
    State(st): State<AppState>,
    jar: CookieJar,
    u: CurrentUser,
    body: axum::body::Bytes,
) -> AppResult<(CookieJar, Json<Value>)> {
    if !body.is_empty() {
        if let Ok(req) = serde_json::from_slice::<LogoutIn>(&body) {
            if let Some(ep) = req
                .push_endpoint
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let _ =
                    sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1 AND endpoint=$2")
                        .bind(u.id)
                        .bind(ep)
                        .execute(&st.db)
                        .await;
            }
            if let Some(tok) = req
                .mobile_token
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let _ = sqlx::query("DELETE FROM mobile_push_tokens WHERE user_id=$1 AND token=$2")
                    .bind(u.id)
                    .bind(tok)
                    .execute(&st.db)
                    .await;
            }
        }
    }
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
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM password_reset_tokens WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM oauth_states WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM mobile_push_tokens WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(&st.db, Some(u.id), "logout.all", "user", Some(u.id), "").await;
    Ok((clear_cookies(jar), Json(json!({"ok": true}))))
}
