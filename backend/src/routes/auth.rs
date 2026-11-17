//! 登入与账号安全: invites, passkeys (WebAuthn), password + TOTP 2FA, account recovery.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::{json, Value};
use totp_rs::{Algorithm, Secret, TOTP};
use webauthn_rs::prelude::*;

use std::sync::OnceLock;

use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use std::net::SocketAddr;

use crate::auth::{hash_token, CurrentUser, SESSION_COOKIE, SESSION_COOKIE_SECURE};
use crate::error::{bad, forbidden, not_found, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{self, Act, Page};
use crate::state::AppState;
use crate::util::{
    audit, check_name, check_phone, in_hours, minutes_ago, notify_perm, now, random_token, t, Lang,
};
use webauthn_rs::fake::{FakePasskeyDistribution, WebauthnFakeCredentialGenerator};

/// Invite lookup by the token in the link. Recovery links sign their holder in as an
/// existing member, so only their SHA-256 is stored ($2); a leaked database or backup
/// can't be replayed. Ordinary invites ($1) only open a pending profile and are kept as
/// is so admins can copy the link again. The kind check stops a stored hash being
/// presented as if it were a token.
const INVITE_BY_TOKEN: &str =
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
        .route("/auth/lock", post(lock_session))
        .route("/auth/unlock", post(unlock_session))
        .route("/auth/terms", post(accept_terms))
        .route("/auth/passkey/login/start", post(passkey_login_start))
        .route("/auth/passkey/login/finish", post(passkey_login_finish))
        .route("/auth/passkey/reauth/start", post(passkey_reauth_start))
        .route("/auth/passkey/reauth/finish", post(passkey_reauth_finish))
        .route("/auth/passkey/register/start", post(passkey_register_start))
        .route(
            "/auth/passkey/register/finish",
            post(passkey_register_finish),
        )
        .route("/auth/passkeys", get(list_passkeys))
        .route("/auth/passkeys/{id}", delete(delete_passkey))
        .route("/auth/totp/setup", post(totp_setup))
        .route("/auth/totp/enable", post(totp_enable))
        .route("/auth/totp/verify", post(totp_verify))
        .route(
            "/auth/totp/backup-codes/regenerate",
            post(totp_regenerate_backup_codes),
        )
        .route("/auth/password", post(set_password))
        .route("/auth/recovery-request", post(recovery_request))
        .route("/invites/{token}", get(invite_preview))
        .route("/invites/{token}/accept", post(invite_accept))
}

pub fn hash_password(pw: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Other(anyhow::anyhow!("hash: {e}")))
}

fn verify_password(pw: &str, hash: &str) -> bool {
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
    for skew in [-1i64, 0, 1] {
        let t_slot = (now_s as i64 + skew * 30).max(0) as u64;
        if totp.generate(t_slot) == code {
            let step = (t_slot / 30) as i64;
            if step <= last {
                return Err(bad(lang, ErrorKey::CodeAlreadyUsed));
            }
            sqlx::query("UPDATE users SET totp_last_step=$1 WHERE id=$2")
                .bind(step)
                .bind(user_id)
                .execute(&st.db)
                .await?;
            return Ok(());
        }
    }
    Err(bad(lang, ErrorKey::InvalidCode))
}

fn totp_for(secret_b32: &str, email: &str, lang: Lang) -> AppResult<TOTP> {
    let bytes = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .map_err(|_| bad(lang, ErrorKey::Invalid2FaSecret))?;
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some("GraceRide".into()),
        email.to_string(),
    )
    .map_err(|e| AppError::Other(anyhow::anyhow!("totp: {e}")))
}

/// Members stay signed in for 30 days on their own phone; accounts that can see other
/// members' data (管理员 / 交通组 / 财务) get 12-hour sessions.
async fn create_session(
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

fn visitor_lang(headers: &HeaderMap) -> Lang {
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
    audit(&st.db, Some(u.id), "logout.all", "user", Some(u.id), "").await;
    Ok((clear_cookies(jar), Json(json!({"ok": true}))))
}

// ---------- Passkeys ----------

async fn load_passkeys(st: &AppState, user_id: i64) -> AppResult<Vec<Passkey>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT passkey FROM passkeys WHERE user_id=$1")
        .bind(user_id)
        .fetch_all(&st.db)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(p,)| serde_json::from_str(&p).ok())
        .collect())
}

async fn save_challenge(
    st: &AppState,
    user_id: i64,
    kind: &str,
    state: String,
) -> AppResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("DELETE FROM auth_challenges WHERE created_at < $1")
        .bind(crate::util::minutes_ago(10))
        .execute(&st.db)
        .await?;
    sqlx::query(
        "INSERT INTO auth_challenges (id, user_id, kind, state, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(kind)
    .bind(state)
    .bind(now())
    .execute(&st.db)
    .await?;
    Ok(id)
}

async fn take_challenge(
    st: &AppState,
    id: &str,
    kind: &str,
    lang: Lang,
) -> AppResult<(i64, String)> {
    let row: Option<(i64, String)> = sqlx::query_as(
        "DELETE FROM auth_challenges WHERE id=$1 AND kind=$2 AND created_at >= $3 RETURNING user_id, state",
    )
    .bind(id)
    .bind(kind)
    .bind(minutes_ago(10))
    .fetch_optional(&st.db)
    .await?;
    row.ok_or_else(|| bad(lang, ErrorKey::VerificationExpired))
}

#[derive(Deserialize)]
struct EmailReq {
    email: String,
}

async fn passkey_login_start(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EmailReq>,
) -> AppResult<Json<Value>> {
    let lang = Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    let _ = lang;
    // Emails are stored lowercase; "Alice@…" is the same member as "alice@…".
    let email = req.email.trim().to_lowercase();
    let email = email.as_str();
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE email=$1 AND status IN ('active', 'pending')")
            .bind(email)
            .fetch_optional(&st.db)
            .await?;
    let keys = match row {
        Some((uid,)) => Some((uid, load_passkeys(&st, uid).await?)),
        None => None,
    };
    // Unknown email, or a member with no passkey yet: answer with a believable challenge
    // instead of an error, so this endpoint can't be used to find out who is a member.
    let Some((uid, keys)) = keys.filter(|(_, k)| !k.is_empty()) else {
        return Ok(Json(json!({
            "challenge_id": uuid::Uuid::new_v4().to_string(),
            "options": decoy_challenge(&st, email).await?,
        })));
    };
    let (rcr, auth_state) = st
        .webauthn
        .start_passkey_authentication(&keys)
        .map_err(|e| AppError::Other(anyhow::anyhow!("webauthn: {e:?}")))?;
    let cid = save_challenge(
        &st,
        uid,
        "auth",
        serde_json::to_string(&auth_state).map_err(anyhow::Error::from)?,
    )
    .await?;
    Ok(Json(json!({ "challenge_id": cid, "options": rcr })))
}

/// The persisted HMAC key behind the decoy challenges, created on first use.
async fn decoy_key(st: &AppState) -> AppResult<Vec<u8>> {
    let fresh = WebauthnFakeCredentialGenerator::<FakePasskeyDistribution>::new_hmac_key()
        .map_err(|e| AppError::Other(anyhow::anyhow!("webauthn: {e:?}")))?;
    sqlx::query(
        "INSERT INTO app_secrets (name, value) VALUES ('decoy_passkey_hmac', $1) ON CONFLICT (name) DO NOTHING",
    )
    .bind(hex::encode(fresh))
    .execute(&st.db)
    .await?;
    let (key,): (String,) =
        sqlx::query_as("SELECT value FROM app_secrets WHERE name='decoy_passkey_hmac'")
            .fetch_one(&st.db)
            .await?;
    hex::decode(key).map_err(|e| AppError::Other(anyhow::anyhow!("decoy key: {e}")))
}

/// Believable credential ids for an email, derived only from the email and the key.
///
/// The generator gives about one email in six no credentials at all. A real challenge
/// always lists at least one, so an empty list would mark the email as not a passkey
/// member. Such emails are derived again (same email, same key, next round) until there
/// is one; emails that got credentials on the first try keep exactly those ids.
fn decoy_credential_ids(key: &[u8], email: &str) -> AppResult<Vec<CredentialID>> {
    let fail = |e| AppError::Other(anyhow::anyhow!("webauthn: {e:?}"));
    let generator =
        WebauthnFakeCredentialGenerator::<FakePasskeyDistribution>::new(key).map_err(fail)?;
    let name = email.to_lowercase();
    for round in 0..=u8::MAX {
        let mut input = name.as_bytes().to_vec();
        if round > 0 {
            input.extend([0, round]);
        }
        let ids = generator.generate(&input).map_err(fail)?;
        if !ids.is_empty() {
            return Ok(ids);
        }
    }
    Err(AppError::Other(anyhow::anyhow!("no decoy credentials")))
}

/// Sign-in options for an email with no passkey. Same shape as a real challenge, with
/// credential ids derived from the email under a secret key: the same email always gets
/// the same ids, with the count and lengths real devices produce. No browser holds them,
/// so the sign-in fails the way a wrong device would.
async fn decoy_challenge(st: &AppState, email: &str) -> AppResult<Value> {
    let ids = decoy_credential_ids(&decoy_key(st).await?, email)?;
    let rp_id = url::Url::parse(&st.cfg.public_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_default();
    let challenge: [u8; 32] = rand::random();
    // Parsed back through webauthn-rs's own type so the field names and encoding are
    // exactly what a real challenge serialises to.
    let options: RequestChallengeResponse = serde_json::from_value(json!({
        "publicKey": {
            "challenge": Base64UrlSafeData::from(challenge.to_vec()),
            "timeout": 300_000,
            "rpId": rp_id,
            "allowCredentials": ids.iter().map(|id| json!({"type": "public-key", "id": id})).collect::<Vec<_>>(),
            "userVerification": "required",
        }
    }))
    .map_err(anyhow::Error::from)?;
    Ok(serde_json::to_value(options).map_err(anyhow::Error::from)?)
}

/// Adding or removing a passkey or authenticator needs a sign-in from the last
/// 15 minutes once the account has one, so a stolen or left-open session can't
/// quietly add the thief's own device and keep access after the session ends.
/// Accounts with nothing set up yet (new members, after recovery) are not held back.
async fn require_recent_sign_in(st: &AppState, u: &CurrentUser) -> AppResult<()> {
    let (keys, totp, signed_in): (i64, i64, String) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM passkeys WHERE user_id=$1),
                (SELECT totp_enabled FROM users WHERE id=$1),
                (SELECT created_at FROM sessions WHERE token=$2)",
    )
    .bind(u.id)
    .bind(&u.session_id)
    .fetch_one(&st.db)
    .await?;
    if (keys > 0 || totp == 1) && signed_in < minutes_ago(15) {
        return Err(forbidden(u.lang, ErrorKey::RecentSignInRequired));
    }
    Ok(())
}

#[derive(Deserialize)]
struct LoginFinish {
    challenge_id: String,
    credential: PublicKeyCredential,
}

async fn passkey_login_finish(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<LoginFinish>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    // A decoy challenge has no stored state. Answer exactly as a wrong passkey would,
    // so finishing a decoy doesn't reveal that the email has no account.
    // Failed sign-ins are recorded with no actor: nobody has proved who they are.
    let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
    let Ok((uid, state_json)) = take_challenge(&st, &req.challenge_id, "auth", lang).await else {
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            None,
            format!("unknown or expired sign-in challenge; from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::PasskeyFailed));
    };
    let auth_state: PasskeyAuthentication =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let Ok(res) = st
        .webauthn
        .finish_passkey_authentication(&req.credential, &auth_state)
    else {
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            Some(uid),
            format!("passkey rejected; from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::PasskeyFailed));
    };
    let key_found = touch_passkey(&st, uid, &res).await?;
    if !key_found {
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            Some(uid),
            format!("passkey credential not found or revoked; from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::PasskeyFailed));
    }
    let jar = create_session(&st, jar, uid, true).await?;
    audit(&st.db, Some(uid), "login.passkey", "user", Some(uid), "").await;
    Ok((jar, Json(json!({ "ok": true }))))
}

/// Stores the credential's new signature counter and last-used time after a successful sign-in.
async fn touch_passkey(st: &AppState, uid: i64, res: &AuthenticationResult) -> AppResult<bool> {
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, passkey FROM passkeys WHERE user_id=$1")
            .bind(uid)
            .fetch_all(&st.db)
            .await?;
    let mut found = false;
    for (pid, pj) in rows {
        if let Ok(mut pk) = serde_json::from_str::<Passkey>(&pj) {
            if pk.cred_id() == res.cred_id() {
                found = true;
                pk.update_credential(res);
                sqlx::query("UPDATE passkeys SET passkey=$1, last_used=$2 WHERE id=$3")
                    .bind(serde_json::to_string(&pk).map_err(anyhow::Error::from)?)
                    .bind(now())
                    .bind(pid)
                    .execute(&st.db)
                    .await?;
            }
        }
    }
    Ok(found)
}

/// Starts a passkey check for the member who is already signed in, so they can renew
/// their "recent sign-in" without signing out first.
async fn passkey_reauth_start(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let keys = load_passkeys(&st, u.id).await?;
    if keys.is_empty() {
        return Err(bad(u.lang, ErrorKey::PasskeyFailed));
    }
    let (rcr, auth_state) = st
        .webauthn
        .start_passkey_authentication(&keys)
        .map_err(|e| AppError::Other(anyhow::anyhow!("webauthn: {e:?}")))?;
    let cid = save_challenge(
        &st,
        u.id,
        "auth",
        serde_json::to_string(&auth_state).map_err(anyhow::Error::from)?,
    )
    .await?;
    Ok(Json(json!({ "challenge_id": cid, "options": rcr })))
}

/// Finishes the check. The passkey must belong to the signed-in member; on success the
/// current session counts as signed in just now (the session itself is unchanged).
async fn passkey_reauth_finish(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<LoginFinish>,
) -> AppResult<Json<Value>> {
    let (uid, state_json) = take_challenge(&st, &req.challenge_id, "auth", u.lang).await?;
    if uid != u.id {
        return Err(forbidden(u.lang, ErrorKey::ChallengeMismatch));
    }
    let auth_state: PasskeyAuthentication =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let Ok(res) = st
        .webauthn
        .finish_passkey_authentication(&req.credential, &auth_state)
    else {
        audit(
            &st.db,
            Some(u.id),
            "login.failed",
            "user",
            Some(u.id),
            "passkey rejected on re-check",
        )
        .await;
        return Err(bad(u.lang, ErrorKey::PasskeyFailed));
    };
    if !touch_passkey(&st, u.id, &res).await? {
        return Err(bad(u.lang, ErrorKey::PasskeyFailed));
    }
    sqlx::query(
        "UPDATE sessions SET created_at=$1, locked_at=NULL, last_seen_at=$1 WHERE token=$2",
    )
    .bind(now())
    .bind(&u.session_id)
    .execute(&st.db)
    .await?;
    audit(&st.db, Some(u.id), "login.reauth", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true })))
}

async fn passkey_register_start(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    let (wuuid,): (String,) = sqlx::query_as("SELECT webauthn_uuid FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    let uuid = uuid::Uuid::parse_str(&wuuid).map_err(anyhow::Error::from)?;
    let existing: Vec<CredentialID> = load_passkeys(&st, u.id)
        .await?
        .iter()
        .map(|p| p.cred_id().clone())
        .collect();
    let (ccr, reg_state) = st
        .webauthn
        .start_passkey_registration(uuid, &u.email, &u.name, Some(existing))
        .map_err(|e| AppError::Other(anyhow::anyhow!("webauthn: {e:?}")))?;
    let cid = save_challenge(
        &st,
        u.id,
        "reg",
        serde_json::to_string(&reg_state).map_err(anyhow::Error::from)?,
    )
    .await?;
    Ok(Json(json!({ "challenge_id": cid, "options": ccr })))
}

#[derive(Deserialize)]
struct RegFinish {
    challenge_id: String,
    credential: RegisterPublicKeyCredential,
    label: Option<String>,
}

async fn passkey_register_finish(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<RegFinish>,
) -> AppResult<Json<Value>> {
    let (uid, state_json) = take_challenge(&st, &req.challenge_id, "reg", u.lang).await?;
    if uid != u.id {
        return Err(forbidden(u.lang, ErrorKey::ChallengeMismatch));
    }
    let reg_state: PasskeyRegistration =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let pk = st
        .webauthn
        .finish_passkey_registration(&req.credential, &reg_state)
        .map_err(|e| {
            bad(
                u.lang,
                ErrorKey::PasskeyRegistrationFailed(format!("{e:?}")),
            )
        })?;
    let cred_id = serde_json::to_string(pk.cred_id()).map_err(anyhow::Error::from)?;
    let label = req
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(60).collect::<String>())
        .unwrap_or_else(|| t(u.lang, "我的设备", "My device").into());
    sqlx::query(
        "INSERT INTO passkeys (user_id, cred_id, passkey, label, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(u.id)
    .bind(cred_id)
    .bind(serde_json::to_string(&pk).map_err(anyhow::Error::from)?)
    .bind(&label)
    .bind(now())
    .execute(&st.db)
    .await?;
    // Registering a passkey upgrades the current session to strong auth.
    sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "passkey.register",
        "user",
        Some(u.id),
        &label,
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn list_passkeys(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, label, created_at, last_used FROM passkeys WHERE user_id=$1 ORDER BY id",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, label, c, l)| json!({"id": id, "label": label, "created_at": c, "last_used": l}))
        .collect::<Vec<_>>())))
}

async fn delete_passkey(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    let gone: Option<(String,)> =
        sqlx::query_as("DELETE FROM passkeys WHERE id=$1 AND user_id=$2 RETURNING label")
            .bind(id)
            .bind(u.id)
            .fetch_optional(&st.db)
            .await?;
    if let Some((label,)) = gone {
        audit(
            &st.db,
            Some(u.id),
            "passkey.delete",
            "user",
            Some(u.id),
            format!("passkey {id} ({label})"),
        )
        .await;
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------- TOTP 2FA ----------

async fn totp_setup(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    // A stolen session must not be able to silently replace an existing second factor.
    let (enabled,): (i64,) = sqlx::query_as("SELECT totp_enabled FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if enabled == 1 {
        return Err(bad(u.lang, ErrorKey::TwoFactorAlreadyEnabled));
    }
    require_recent_sign_in(&st, &u).await?;
    let secret = Secret::generate_secret().to_encoded().to_string();
    sqlx::query("UPDATE users SET totp_secret=$1, totp_enabled=0 WHERE id=$2")
        .bind(&secret)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    let totp = totp_for(&secret, &u.email, u.lang)?;
    audit(&st.db, Some(u.id), "totp.setup", "user", Some(u.id), "").await;
    Ok(Json(json!({ "secret": secret, "url": totp.get_url() })))
}

#[derive(Deserialize)]
struct CodeReq {
    code: String,
}

fn hash_backup_code(user_id: i64, norm: &str) -> String {
    crate::auth::hash_token(&format!("backup:{user_id}:{norm}"))
}

fn generate_backup_codes(user_id: i64) -> (Vec<String>, Vec<String>) {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    let mut plain = Vec::with_capacity(8);
    let mut hashed = Vec::with_capacity(8);
    for _ in 0..8 {
        let p1: String = (0..4)
            .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
            .collect();
        let p2: String = (0..4)
            .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
            .collect();
        let code = format!("{p1}-{p2}");
        let norm = format!("{p1}{p2}");
        hashed.push(hash_backup_code(user_id, &norm));
        plain.push(code);
    }
    (plain, hashed)
}

async fn totp_enable(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CodeReq>,
) -> AppResult<Json<Value>> {
    let (secret,): (Option<String>,) = sqlx::query_as("SELECT totp_secret FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    let secret = secret.ok_or_else(|| bad(u.lang, ErrorKey::StartSetupFirst))?;
    if let Err(e) = verify_totp(&st, u.id, &secret, &u.email, &req.code, u.lang).await {
        audit(&st.db, Some(u.id), "totp.failed", "user", Some(u.id), "").await;
        return Err(e);
    }
    let (plain_codes, hashed_codes) = generate_backup_codes(u.id);
    let backup_json =
        serde_json::to_string(&hashed_codes).map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    sqlx::query("UPDATE users SET totp_enabled=1, totp_backup_codes=$1 WHERE id=$2")
        .bind(&backup_json)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "totp.enable", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true, "backup_codes": plain_codes })))
}

async fn consume_backup_code(
    st: &AppState,
    user_id: i64,
    raw: &str,
    lang: Lang,
) -> AppResult<usize> {
    let norm = raw.trim().replace(['-', ' '], "").to_uppercase();
    if norm.len() != 8 {
        return Err(bad(lang, ErrorKey::InvalidCode));
    }
    let mut consumed: Option<(String,)> = None;
    for code_hash in [
        hash_backup_code(user_id, &norm),
        crate::auth::hash_token(&norm),
    ] {
        consumed = sqlx::query_as(
            "UPDATE users
             SET totp_backup_codes = (
                 SELECT COALESCE(jsonb_agg(elem ORDER BY seq), '[]'::jsonb)::text
                 FROM jsonb_array_elements_text(totp_backup_codes::jsonb)
                      WITH ORDINALITY AS t(elem, seq)
                 WHERE elem <> $2
             )
             WHERE id = $1 AND totp_backup_codes::jsonb ? $2
             RETURNING totp_backup_codes",
        )
        .bind(user_id)
        .bind(&code_hash)
        .fetch_optional(&st.db)
        .await?;
        if consumed.is_some() {
            break;
        }
    }
    let Some((remaining_json,)) = consumed else {
        audit(
            &st.db,
            Some(user_id),
            "totp.backup_code_used",
            "user",
            Some(user_id),
            "already used",
        )
        .await;
        return Err(bad(lang, ErrorKey::InvalidCode));
    };
    let remaining: usize = serde_json::from_str::<Vec<String>>(&remaining_json)
        .map(|v| v.len())
        .unwrap_or(0);
    Ok(remaining)
}

async fn lock_session(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE sessions SET locked_at=$1 WHERE token=$2")
        .bind(now())
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "session.lock", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct UnlockReq {
    totp_code: Option<String>,
    backup_code: Option<String>,
    dev_unlock: Option<bool>,
}

async fn unlock_session(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UnlockReq>,
) -> AppResult<Json<Value>> {
    let (totp_enabled, totp_secret, passkey_count): (i64, Option<String>, i64) = sqlx::query_as(
        "SELECT u.totp_enabled, u.totp_secret, (SELECT COUNT(*) FROM passkeys WHERE user_id=u.id)
         FROM users u WHERE u.id=$1",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;

    let mut mfa_verified = false;
    if let Some(code) = req
        .totp_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if totp_enabled != 1 {
            return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
        }
        let sec = totp_secret.ok_or_else(|| bad(u.lang, ErrorKey::TwoFactorNotConfigured))?;
        verify_totp(&st, u.id, &sec, &u.email, code, u.lang).await?;
        mfa_verified = true;
    } else if let Some(bcode) = req
        .backup_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if totp_enabled != 1 {
            return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
        }
        consume_backup_code(&st, u.id, bcode, u.lang).await?;
        mfa_verified = true;
    } else if req.dev_unlock == Some(true) && st.cfg.dev_mode {
        if passkey_count > 0 || totp_enabled == 1 {
            return Err(bad(u.lang, ErrorKey::UnlockMethodRequired));
        }
    } else {
        return Err(bad(u.lang, ErrorKey::UnlockMethodRequired));
    }

    let ts_now = now();
    if mfa_verified {
        sqlx::query("UPDATE sessions SET locked_at=NULL, last_seen_at=$1, mfa_ok=1 WHERE token=$2")
            .bind(&ts_now)
            .bind(&u.session_id)
            .execute(&st.db)
            .await?;
    } else {
        sqlx::query("UPDATE sessions SET locked_at=NULL, last_seen_at=$1 WHERE token=$2")
            .bind(&ts_now)
            .bind(&u.session_id)
            .execute(&st.db)
            .await?;
    }
    audit(
        &st.db,
        Some(u.id),
        "session.unlock",
        "user",
        Some(u.id),
        if mfa_verified { "2fa" } else { "dev" },
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn totp_verify(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CodeReq>,
) -> AppResult<Json<Value>> {
    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT totp_secret, totp_backup_codes FROM users WHERE id=$1 AND totp_enabled=1",
    )
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;

    let Some((secret, _backup_codes)) = row else {
        return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
    };

    let clean = req.code.trim();
    // 1. Check if 6 digits TOTP
    if clean.len() == 6 && clean.chars().all(|c| c.is_ascii_digit()) {
        if let Some(sec) = secret {
            if verify_totp(&st, u.id, &sec, &u.email, clean, u.lang)
                .await
                .is_ok()
            {
                sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
                    .bind(&u.session_id)
                    .execute(&st.db)
                    .await?;
                audit(
                    &st.db,
                    Some(u.id),
                    "totp.verify",
                    "user",
                    Some(u.id),
                    "totp",
                )
                .await;
                return Ok(Json(json!({ "ok": true, "method": "totp" })));
            }
        }
    }

    // 2. Check if single-use backup recovery code (8 characters ignoring hyphens/spaces)
    // Consume the code atomically: the UPDATE only takes effect while the hash is still
    // present in the array, so two concurrent requests with the same code cannot both
    // succeed. PostgreSQL serializes row-level UPDATEs; the loser re-reads the row once
    // it gets the lock, finds the code already removed, affects 0 rows and gets an error
    // instead of HTTP 200.
    let norm = clean.replace(['-', ' '], "").to_uppercase();
    if norm.len() == 8 {
        let remaining = consume_backup_code(&st, u.id, &norm, u.lang).await?;

        sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
            .bind(&u.session_id)
            .execute(&st.db)
            .await?;
        audit(
            &st.db,
            Some(u.id),
            "totp.backup_code_used",
            "user",
            Some(u.id),
            format!("remaining: {remaining}"),
        )
        .await;
        return Ok(Json(json!({
            "ok": true,
            "method": "backup_code",
            "remaining_backup_codes": remaining
        })));
    }

    audit(
        &st.db,
        Some(u.id),
        "totp.failed",
        "user",
        Some(u.id),
        "verify",
    )
    .await;
    Err(bad(u.lang, ErrorKey::InvalidCode))
}

async fn totp_regenerate_backup_codes(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    let (enabled,): (i64,) = sqlx::query_as("SELECT totp_enabled FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if enabled != 1 {
        return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
    }
    let (plain_codes, hashed_codes) = generate_backup_codes(u.id);
    let backup_json =
        serde_json::to_string(&hashed_codes).map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    sqlx::query("UPDATE users SET totp_backup_codes=$1 WHERE id=$2")
        .bind(&backup_json)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "totp.backup_codes_regenerated",
        "user",
        Some(u.id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true, "backup_codes": plain_codes })))
}

#[derive(Deserialize)]
struct PwReq {
    current: Option<String>,
    new_password: String,
}

async fn set_password(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<PwReq>,
) -> AppResult<Json<Value>> {
    if req.new_password.chars().count() < 10 {
        return Err(bad(u.lang, ErrorKey::PasswordTooShort));
    }
    let (hash,): (Option<String>,) = sqlx::query_as("SELECT password_hash FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if let Some(h) = hash {
        if !verify_password(req.current.as_deref().unwrap_or(""), &h) {
            audit(
                &st.db,
                Some(u.id),
                "password.failed",
                "user",
                Some(u.id),
                "current password incorrect",
            )
            .await;
            return Err(bad(u.lang, ErrorKey::CurrentPasswordIncorrect));
        }
    }
    sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
        .bind(hash_password(&req.new_password)?)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "password.set", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true })))
}

// ---------- Recovery ----------

#[derive(Deserialize)]
struct RecoveryReq {
    email: String,
    reason: String,
}

async fn recovery_request(
    State(st): State<AppState>,
    Json(req): Json<RecoveryReq>,
) -> AppResult<Json<Value>> {
    // Always answer the same way so the endpoint can't be used to probe member emails.
    let email = req.email.trim().to_lowercase();
    if email.is_empty() || email.len() > 254 {
        return Ok(Json(json!({ "ok": true })));
    }
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(&email)
        .fetch_optional(&st.db)
        .await?;
    if let Some((uid,)) = exists {
        sqlx::query("INSERT INTO recovery_requests (email, reason, created_at) VALUES ($1,$2,$3)")
            .bind(&email)
            .bind(req.reason.chars().take(500).collect::<String>())
            .bind(now())
            .execute(&st.db)
            .await?;
        // No actor: the request comes from someone who cannot sign in.
        audit(
            &st.db,
            None,
            "recovery.request",
            "user",
            Some(uid),
            req.email.trim(),
        )
        .await;
        notify_perm(
            &st.db,
            Page::AdminRecovery,
            Act::Update,
            ("新的账号恢复申请", "New account recovery request"),
            (
                &format!("会员 {} 提交了账号恢复申请", req.email),
                &format!("Member {} submitted an account recovery request", req.email),
            ),
            "/admin/members",
        )
        .await;
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------- Invites ----------

async fn invite_preview(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> AppResult<Json<Value>> {
    let lang = Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    let row: Option<(String, String, String, String, Option<String>)> = sqlx::query_as(&format!(
        "SELECT kind, email, name, expires_at, used_at FROM invites WHERE {INVITE_BY_TOKEN}"
    ))
    .bind(&token)
    .bind(hash_token(&token))
    .fetch_optional(&st.db)
    .await?;
    let Some((kind, email, name, expires, used)) = row else {
        return Err(not_found(lang));
    };
    if used.is_some() || expires < now() {
        return Err(bad(lang, ErrorKey::InviteExpired));
    }
    Ok(Json(json!({ "kind": kind, "email": email, "name": name })))
}

/// Version of the terms and privacy notice in `frontend/src/locales/terms.*.ts`. Raise it
/// when their meaning changes (and the column default in a new migration, see 0007): every
/// member who agreed to an older version is asked to opt in again.
pub const TERMS_VERSION: i32 = 2;

#[derive(Deserialize)]
struct TermsIn {
    accepted: bool,
}

async fn accept_terms(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<TermsIn>,
) -> AppResult<Json<Value>> {
    if !req.accepted {
        return Err(bad(u.lang, ErrorKey::TermsRequired));
    }
    // Each opt-in records when and to which version, so a later version asks again.
    sqlx::query("UPDATE users SET terms_accepted_at=$1, terms_version=$2 WHERE id=$3")
        .bind(now())
        .bind(TERMS_VERSION)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "terms.accept",
        "user",
        Some(u.id),
        &format!("version {TERMS_VERSION}"),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct AcceptReq {
    name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    terms: Option<bool>,
}

async fn invite_accept(
    State(st): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(token): Path<String>,
    Json(req): Json<AcceptReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    let row: Option<(i64, String, String, String, String, String, Option<i64>, String, Option<String>)> = sqlx::query_as(
        &format!("SELECT id, kind, email, name, roles, group_ids::text, user_id, expires_at, used_at FROM invites WHERE {INVITE_BY_TOKEN}"),
    )
    .bind(&token)
    .bind(hash_token(&token))
    .fetch_optional(&st.db)
    .await?;
    let Some((inv_id, kind, email, inv_name, _roles, _group_ids, rec_user, expires, used)) = row
    else {
        return Err(not_found(lang));
    };
    if used.is_some() || expires < now() {
        return Err(bad(lang, ErrorKey::InviteExpired));
    }
    if req.terms != Some(true) {
        return Err(bad(lang, ErrorKey::TermsRequired));
    }
    let joined_email;
    let joined_name;
    if kind == "recovery" {
        joined_email = email;
        joined_name = inv_name;
    } else {
        joined_name = req
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        joined_email = req.email.unwrap_or_default().trim().to_lowercase();
        check_name(lang, &joined_name)?;
        // Checked before the invite is marked used: a value the database would refuse
        // used to burn the single-use link and then report a misleading error.
        check_phone(lang, req.phone.as_deref().unwrap_or(""))?;
        if !joined_email.contains('@') {
            return Err(bad(lang, ErrorKey::EmailInvalid));
        }
        let (exists,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email=$1")
            .bind(&joined_email)
            .fetch_one(&st.db)
            .await?;
        if exists > 0 {
            return Err(bad(lang, ErrorKey::EmailAlreadyRegistered));
        }
    }
    // Consuming the invite and opening the account happen together or not at all: an
    // account insert that fails (e.g. a concurrent signup took the email) rolls the invite
    // back to unused, so the person can retry with a corrected email. The conditional
    // UPDATE still makes the link single use under double submit.
    let mut tx = st.db.begin().await?;
    let r = sqlx::query(
        "UPDATE invites SET used_at=$1, email=$2, name=$3 WHERE id=$4 AND used_at IS NULL",
    )
    .bind(now())
    .bind(&joined_email)
    .bind(&joined_name)
    .bind(inv_id)
    .execute(&mut *tx)
    .await?;
    if r.rows_affected() == 0 {
        return Err(bad(lang, ErrorKey::InviteAlreadyUsed));
    }

    let uid = if kind == "recovery" {
        let id = rec_user.ok_or_else(|| bad(lang, ErrorKey::RecoveryLinkInvalid))?;
        // A recovery link signs in as the member. Once one is used, no sibling
        // recovery link may remain live (e.g. one issued before a replacement).
        sqlx::query("UPDATE invites SET expires_at=$1 WHERE kind='recovery' AND user_id=$2 AND id<>$3 AND used_at IS NULL")
            .bind(minutes_ago(1))
            .bind(id)
            .bind(inv_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE users SET terms_accepted_at=$1, terms_version=$2 WHERE id=$3")
            .bind(now())
            .bind(TERMS_VERSION)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        id
    } else {
        // The invite only opens a profile. Role and group are assigned when an admin approves.
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid, terms_accepted_at, terms_version) VALUES ($1,$2,$3,'','pending',$4,$5,$6,$7) RETURNING id",
        )
        .bind(&joined_name)
        .bind(&joined_email)
        .bind(req.phone.unwrap_or_default().trim())
        .bind(None::<String>)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(now())
        .bind(TERMS_VERSION)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| bad(lang, ErrorKey::EmailAlreadyRegistered))?;
        id
    };
    tx.commit().await?;
    let jar = create_session(&st, jar, uid, false).await?;
    audit(
        &st.db,
        Some(uid),
        if kind == "recovery" {
            "recovery.use"
        } else {
            "invite.accept"
        },
        "user",
        Some(uid),
        &joined_email,
    )
    .await;
    Ok((jar, Json(json!({ "ok": true, "kind": kind }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoy_ids_are_never_empty_and_stay_the_same() {
        let key = [7u8; 32];
        let raw = WebauthnFakeCredentialGenerator::<FakePasskeyDistribution>::new(&key).unwrap();
        let mut filled_in = 0;
        for i in 0..200 {
            let email = format!("Person{i}@Example.org");
            let ids = decoy_credential_ids(&key, &email).unwrap();
            assert!(!ids.is_empty(), "{email}");
            // Same email (in any letter case) and key: same ids, on every call.
            assert_eq!(
                ids,
                decoy_credential_ids(&key, &email.to_lowercase()).unwrap()
            );
            // Emails the library already gave credentials keep exactly those.
            let first_try = raw.generate(email.to_lowercase().as_bytes()).unwrap();
            if first_try.is_empty() {
                filled_in += 1;
            } else {
                assert_eq!(ids, first_try, "{email}");
            }
        }
        // About one in six needs the extra round; the sample must include some.
        assert!(filled_in > 0);
    }
}
