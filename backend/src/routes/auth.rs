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

use crate::auth::{
    hash_token, is_privileged, CurrentUser, PRIVILEGED, SESSION_COOKIE, SESSION_COOKIE_SECURE,
};
use crate::error::{bad, forbidden, not_found, too_many_requests, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::security::client_ip;
use crate::state::AppState;
use crate::util::{audit, in_hours, notify_roles, now, random_token, t, Lang};

/// Password login is disabled. All authentication is via passkey (WebAuthn).
/// Per-account lockout does not apply to passkey auth.
const MAX_FAILURES: i64 = 0; // disable password lockout since password login is disabled

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/login", post(login_password))
        .route("/auth/dev-users", get(dev_users))
        .route("/auth/dev-login", post(dev_login))
        .route("/auth/logout", post(logout))
        .route("/auth/terms", post(accept_terms))
        .route("/auth/passkey/login/start", post(passkey_login_start))
        .route("/auth/passkey/login/finish", post(passkey_login_finish))
        .route("/auth/passkey/register/start", post(passkey_register_start))
        .route(
            "/auth/passkey/register/finish",
            post(passkey_register_finish),
        )
        .route("/auth/passkeys", get(list_passkeys))
        .route("/auth/passkeys/{id}", delete(delete_passkey))
        .route("/auth/totp/setup", post(totp_setup))
        .route("/auth/totp/enable", post(totp_enable))
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
    let hours: i64 = if is_privileged(&roles) { 12 } else { 24 * 30 };
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

async fn me(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let row: (String, Option<i64>, i64, Option<String>, i64, i64, i64, Option<String>) = sqlx::query_as(
        "SELECT phone, photo_file_id, totp_enabled, password_hash, badges_opt_out, carpool_enabled, spare_seats, terms_accepted_at FROM users WHERE id=$1",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
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
    let privileged = u.roles.iter().any(|r| PRIVILEGED.contains(&r.as_str()));
    Ok(Json(json!({
        "id": u.id, "name": u.name, "email": u.email, "phone": row.0, "photo_file_id": row.1,
        "status": u.status,
        "lang": u.lang.code(),
        "roles": u.roles, "mfa_ok": u.mfa_ok, "totp_enabled": row.2 == 1,
        "has_password": row.3.is_some(), "passkey_count": passkeys.0,
        "badges_opt_out": row.4 == 1, "carpool_enabled": row.5 == 1, "spare_seats": row.6,
        "terms_accepted": row.7.is_some(),
        "privileged": privileged, "unread": unread.0,
        "groups": groups.into_iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
        "dev_mode": st.cfg.dev_mode,
    })))
}

#[derive(Deserialize)]
struct LoginReq {
    email: String,
    password: String,
    code: Option<String>,
}

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
        headers.get(axum::http::header::ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok()).unwrap_or(""),
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
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT name, email, roles FROM users WHERE status='active' ORDER BY id",
    )
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
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, status FROM users WHERE email=$1")
            .bind(req.email.trim())
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
    sqlx::query("DELETE FROM auth_challenges WHERE created_at < to_char(LOCALTIMESTAMP - interval '10 minutes', 'YYYY-MM-DD\"T\"HH24:MI:SS')")
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

async fn take_challenge(st: &AppState, id: &str, kind: &str, lang: Lang) -> AppResult<(i64, String)> {
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT user_id, state FROM auth_challenges WHERE id=$1 AND kind=$2")
            .bind(id)
            .bind(kind)
            .fetch_optional(&st.db)
            .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE id=$1")
        .bind(id)
        .execute(&st.db)
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
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE email=$1 AND status IN ('active', 'pending')")
            .bind(req.email.trim())
            .fetch_optional(&st.db)
            .await?;
    let Some((uid,)) = row else {
        return Err(bad(lang, ErrorKey::PasskeyNotFound));
    };
    let keys = load_passkeys(&st, uid).await?;
    if keys.is_empty() {
        return Err(bad(lang, ErrorKey::PasskeyNotFound));
    }
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

#[derive(Deserialize)]
struct LoginFinish {
    challenge_id: String,
    credential: PublicKeyCredential,
}

async fn passkey_login_finish(
    State(st): State<AppState>,
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
    let (uid, state_json) = take_challenge(&st, &req.challenge_id, "auth", lang).await?;
    let auth_state: PasskeyAuthentication =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let res = st
        .webauthn
        .finish_passkey_authentication(&req.credential, &auth_state)
        .map_err(|_| bad(lang, ErrorKey::PasskeyFailed))?;
    // Update stored credential counters if needed.
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, passkey FROM passkeys WHERE user_id=$1")
            .bind(uid)
            .fetch_all(&st.db)
            .await?;
    for (pid, pj) in rows {
        if let Ok(mut pk) = serde_json::from_str::<Passkey>(&pj) {
            if pk.cred_id() == res.cred_id() {
                pk.update_credential(&res);
                sqlx::query("UPDATE passkeys SET passkey=$1, last_used=$2 WHERE id=$3")
                    .bind(serde_json::to_string(&pk).map_err(anyhow::Error::from)?)
                    .bind(now())
                    .bind(pid)
                    .execute(&st.db)
                    .await?;
            }
        }
    }
    let jar = create_session(&st, jar, uid, true).await?;
    audit(&st.db, Some(uid), "login.passkey", "user", Some(uid), "").await;
    Ok((jar, Json(json!({ "ok": true }))))
}

async fn passkey_register_start(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
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
        .map_err(|e| bad(u.lang, ErrorKey::PasskeyRegistrationFailed(format!("{e:?}"))))?;
    let cred_id = serde_json::to_string(pk.cred_id()).map_err(anyhow::Error::from)?;
    sqlx::query(
        "INSERT INTO passkeys (user_id, cred_id, passkey, label, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(u.id)
    .bind(cred_id)
    .bind(serde_json::to_string(&pk).map_err(anyhow::Error::from)?)
    .bind(req.label.unwrap_or_else(|| t(u.lang, "我的设备", "My device").into()))
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
        "",
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
    sqlx::query("DELETE FROM passkeys WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "passkey.delete",
        "user",
        Some(u.id),
        format!("passkey {id}"),
    )
    .await;
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
    let secret = Secret::generate_secret().to_encoded().to_string();
    sqlx::query("UPDATE users SET totp_secret=$1, totp_enabled=0 WHERE id=$2")
        .bind(&secret)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    let totp = totp_for(&secret, &u.email, u.lang)?;
    Ok(Json(json!({ "secret": secret, "url": totp.get_url() })))
}

#[derive(Deserialize)]
struct CodeReq {
    code: String,
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
    verify_totp(
        &st,
        u.id,
        secret
            .as_deref()
            .ok_or_else(|| bad(u.lang, ErrorKey::StartSetupFirst))?,
        &u.email,
        &req.code,
        u.lang,
    )
    .await?;
    sqlx::query("UPDATE users SET totp_enabled=1 WHERE id=$1")
        .bind(u.id)
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "totp.enable", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true })))
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
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(req.email.trim())
        .fetch_optional(&st.db)
        .await?;
    if exists.is_some() {
        sqlx::query("INSERT INTO recovery_requests (email, reason, created_at) VALUES ($1,$2,$3)")
            .bind(req.email.trim())
            .bind(req.reason.chars().take(500).collect::<String>())
            .bind(now())
            .execute(&st.db)
            .await?;
        notify_roles(
            &st.db,
            &["admin"],
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
    let row: Option<(String, String, String, String, Option<String>)> =
        sqlx::query_as("SELECT kind, email, name, expires_at, used_at FROM invites WHERE token=$1")
            .bind(&token)
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
    sqlx::query("UPDATE users SET terms_accepted_at=COALESCE(terms_accepted_at, $1) WHERE id=$2")
        .bind(now())
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "terms.accept", "user", Some(u.id), "").await;
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
        "SELECT id, kind, email, name, roles, group_ids, user_id, expires_at, used_at FROM invites WHERE token=$1",
    )
    .bind(&token)
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
        joined_email = req
            .email
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        if joined_name.is_empty() {
            return Err(bad(lang, ErrorKey::NameRequired));
        }
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
    // Mark used first (single use, guards against double submit).
    let r = sqlx::query("UPDATE invites SET used_at=$1, email=$2, name=$3 WHERE id=$4 AND used_at IS NULL")
        .bind(now())
        .bind(&joined_email)
        .bind(&joined_name)
        .bind(inv_id)
        .execute(&st.db)
        .await?;
    if r.rows_affected() == 0 {
        return Err(bad(lang, ErrorKey::InviteAlreadyUsed));
    }

    let uid = if kind == "recovery" {
        let id = rec_user.ok_or_else(|| bad(lang, ErrorKey::RecoveryLinkInvalid))?;
        sqlx::query("UPDATE users SET terms_accepted_at=COALESCE(terms_accepted_at, $1) WHERE id=$2")
            .bind(now())
            .bind(id)
            .execute(&st.db)
            .await?;
        id
    } else {
        // The invite only opens a profile. Role and group are assigned when an admin approves.
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid, terms_accepted_at) VALUES ($1,$2,$3,'','pending',$4,$5,$6) RETURNING id",
        )
        .bind(&joined_name)
        .bind(&joined_email)
        .bind(req.phone.unwrap_or_default())
        .bind(None::<String>)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(now())
        .fetch_one(&st.db)
        .await
        .map_err(|_| bad(lang, ErrorKey::EmailAlreadyRegistered))?;
        id
    };
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
