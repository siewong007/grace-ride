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
use crate::error::{bad, forbidden, AppError, AppResult};
use crate::security::client_ip;
use crate::state::AppState;
use crate::util::{audit, in_hours, notify_roles, now, random_token};

/// Failed password attempts per account before a temporary lock (per 15 minutes).
const MAX_FAILURES: i64 = 5;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/auth/login", post(login_password))
        .route("/auth/logout", post(logout))
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
) -> AppResult<()> {
    let totp = totp_for(secret_b32, email)?;
    let (last,): (i64,) = sqlx::query_as("SELECT totp_last_step FROM users WHERE id=?")
        .bind(user_id)
        .fetch_one(&st.db)
        .await?;
    let now_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))?
        .as_secs();
    let code = code.trim();
    for skew in [-1i64, 0, 1] {
        let t = (now_s as i64 + skew * 30).max(0) as u64;
        if totp.generate(t) == code {
            let step = (t / 30) as i64;
            if step <= last {
                return Err(bad("此验证码已使用，请等待下一组数字"));
            }
            sqlx::query("UPDATE users SET totp_last_step=? WHERE id=?")
                .bind(step)
                .bind(user_id)
                .execute(&st.db)
                .await?;
            return Ok(());
        }
    }
    Err(bad("验证码不正确"))
}

fn totp_for(secret_b32: &str, email: &str) -> AppResult<TOTP> {
    let bytes = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .map_err(|_| bad("2FA 密钥无效"))?;
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
    let (roles,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id=?")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    let hours: i64 = if is_privileged(&roles) { 12 } else { 24 * 30 };
    let token = random_token(48);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES (?,?,?,?,?)",
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
    let row: (String, Option<i64>, i64, Option<String>, i64, i64, i64) = sqlx::query_as(
        "SELECT phone, photo_file_id, totp_enabled, password_hash, badges_opt_out, carpool_enabled, spare_seats FROM users WHERE id=?",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    let passkeys: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM passkeys WHERE user_id=?")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    let unread: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=? AND read_at IS NULL")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let groups: Vec<(i64, String)> = sqlx::query_as(
        "SELECT g.id, g.name FROM groups g JOIN group_members m ON m.group_id=g.id WHERE m.user_id=? ORDER BY g.name",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    let privileged = u.roles.iter().any(|r| PRIVILEGED.contains(&r.as_str()));
    Ok(Json(json!({
        "id": u.id, "name": u.name, "email": u.email, "phone": row.0, "photo_file_id": row.1,
        "roles": u.roles, "mfa_ok": u.mfa_ok, "totp_enabled": row.2 == 1,
        "has_password": row.3.is_some(), "passkey_count": passkeys.0,
        "badges_opt_out": row.4 == 1, "carpool_enabled": row.5 == 1, "spare_seats": row.6,
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
    sqlx::query("INSERT INTO login_failures (email, ip, created_at) VALUES (?,?,?)")
        .bind(email)
        .bind(ip)
        .bind(now())
        .execute(&st.db)
        .await?;
    Ok(())
}

async fn login_password(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<LoginReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let email = req.email.trim().to_lowercase();
    let ip = client_ip(&headers, peer.map(|p| (p.0).0), st.cfg.trust_proxy);
    // Per-account lockout (the per-IP limit lives in the rate-limit middleware).
    let (fails,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM login_failures WHERE email=? AND created_at > strftime('%Y-%m-%dT%H:%M:%S','now','localtime','-15 minutes')",
    )
    .bind(&email)
    .fetch_one(&st.db)
    .await?;
    if fails >= MAX_FAILURES {
        return Err(AppError::TooManyRequests(
            "登入失败次数过多，账号已暂时锁定 15 分钟。可改用通行密钥登入。".into(),
        ));
    }
    let row: Option<(i64, Option<String>, Option<String>, i64, String, String)> = sqlx::query_as(
        "SELECT id, password_hash, totp_secret, totp_enabled, roles, status FROM users WHERE email=?",
    )
    .bind(&email)
    .fetch_optional(&st.db)
    .await?;
    let (id, hash, secret, totp_enabled, roles, status) = match row {
        Some((id, Some(h), s, t, r, st_)) => (id, h, s, t, r, st_),
        _ => {
            let _ = verify_password(&req.password, dummy_hash());
            record_failure(&st, &email, &ip).await?;
            return Err(bad("电邮或密码不正确"));
        }
    };
    if !verify_password(&req.password, &hash) || status != "active" {
        record_failure(&st, &email, &ip).await?;
        audit(
            &st.db,
            Some(id),
            "login.failed",
            "user",
            Some(id),
            format!("ip {ip}"),
        )
        .await;
        return Err(bad("电邮或密码不正确"));
    }
    let mut mfa_ok = false;
    if totp_enabled == 1 {
        let Some(code) = req.code.as_deref().filter(|c| !c.trim().is_empty()) else {
            return Ok((jar, Json(json!({ "mfa_required": true }))));
        };
        if let Err(e) = verify_totp(&st, id, secret.as_deref().unwrap_or(""), &email, code).await {
            record_failure(&st, &email, &ip).await?;
            return Err(e);
        }
        mfa_ok = true;
    }
    sqlx::query("DELETE FROM login_failures WHERE email=?")
        .bind(&email)
        .execute(&st.db)
        .await?;
    let privileged = is_privileged(&roles);
    let jar = create_session(&st, jar, id, mfa_ok).await?;
    audit(
        &st.db,
        Some(id),
        "login.password",
        "user",
        Some(id),
        if mfa_ok { "with 2FA" } else { "" },
    )
    .await;
    Ok((
        jar,
        Json(json!({ "ok": true, "mfa_setup_needed": privileged && !mfa_ok && !st.cfg.dev_mode })),
    ))
}

async fn logout(
    State(st): State<AppState>,
    jar: CookieJar,
    u: CurrentUser,
) -> AppResult<(CookieJar, Json<Value>)> {
    sqlx::query("DELETE FROM sessions WHERE token=?")
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    Ok((clear_cookies(jar), Json(json!({"ok": true}))))
}

// ---------- Passkeys ----------

async fn load_passkeys(st: &AppState, user_id: i64) -> AppResult<Vec<Passkey>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT passkey FROM passkeys WHERE user_id=?")
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
    sqlx::query("DELETE FROM auth_challenges WHERE created_at < strftime('%Y-%m-%dT%H:%M:%S','now','localtime','-10 minutes')")
        .execute(&st.db)
        .await?;
    sqlx::query(
        "INSERT INTO auth_challenges (id, user_id, kind, state, created_at) VALUES (?,?,?,?,?)",
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

async fn take_challenge(st: &AppState, id: &str, kind: &str) -> AppResult<(i64, String)> {
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT user_id, state FROM auth_challenges WHERE id=? AND kind=?")
            .bind(id)
            .bind(kind)
            .fetch_optional(&st.db)
            .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE id=?")
        .bind(id)
        .execute(&st.db)
        .await?;
    row.ok_or_else(|| bad("验证已过期，请重试"))
}

#[derive(Deserialize)]
struct EmailReq {
    email: String,
}

async fn passkey_login_start(
    State(st): State<AppState>,
    Json(req): Json<EmailReq>,
) -> AppResult<Json<Value>> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE email=? AND status='active'")
            .bind(req.email.trim())
            .fetch_optional(&st.db)
            .await?;
    let Some((uid,)) = row else {
        return Err(bad("此电邮尚未设置通行密钥"));
    };
    let keys = load_passkeys(&st, uid).await?;
    if keys.is_empty() {
        return Err(bad("此电邮尚未设置通行密钥"));
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
    Json(req): Json<LoginFinish>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let (uid, state_json) = take_challenge(&st, &req.challenge_id, "auth").await?;
    let auth_state: PasskeyAuthentication =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let res = st
        .webauthn
        .finish_passkey_authentication(&req.credential, &auth_state)
        .map_err(|_| bad("通行密钥验证失败"))?;
    // Update stored credential counters if needed.
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, passkey FROM passkeys WHERE user_id=?")
            .bind(uid)
            .fetch_all(&st.db)
            .await?;
    for (pid, pj) in rows {
        if let Ok(mut pk) = serde_json::from_str::<Passkey>(&pj) {
            if pk.cred_id() == res.cred_id() {
                pk.update_credential(&res);
                sqlx::query("UPDATE passkeys SET passkey=?, last_used=? WHERE id=?")
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
    let (wuuid,): (String,) = sqlx::query_as("SELECT webauthn_uuid FROM users WHERE id=?")
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
    let (uid, state_json) = take_challenge(&st, &req.challenge_id, "reg").await?;
    if uid != u.id {
        return Err(forbidden("验证不匹配"));
    }
    let reg_state: PasskeyRegistration =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let pk = st
        .webauthn
        .finish_passkey_registration(&req.credential, &reg_state)
        .map_err(|e| bad(format!("通行密钥注册失败：{e:?}")))?;
    let cred_id = serde_json::to_string(pk.cred_id()).map_err(anyhow::Error::from)?;
    sqlx::query(
        "INSERT INTO passkeys (user_id, cred_id, passkey, label, created_at) VALUES (?,?,?,?,?)",
    )
    .bind(u.id)
    .bind(cred_id)
    .bind(serde_json::to_string(&pk).map_err(anyhow::Error::from)?)
    .bind(req.label.unwrap_or_else(|| "我的设备".into()))
    .bind(now())
    .execute(&st.db)
    .await?;
    // Registering a passkey upgrades the current session to strong auth.
    sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=?")
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
        "SELECT id, label, created_at, last_used FROM passkeys WHERE user_id=? ORDER BY id",
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
    sqlx::query("DELETE FROM passkeys WHERE id=? AND user_id=?")
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
    let (enabled,): (i64,) = sqlx::query_as("SELECT totp_enabled FROM users WHERE id=?")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if enabled == 1 {
        return Err(bad(
            "2FA 已开启。如需更换验证器，请通过管理员的账号恢复程序。",
        ));
    }
    let secret = Secret::generate_secret().to_encoded().to_string();
    sqlx::query("UPDATE users SET totp_secret=?, totp_enabled=0 WHERE id=?")
        .bind(&secret)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    let totp = totp_for(&secret, &u.email)?;
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
    let (secret,): (Option<String>,) = sqlx::query_as("SELECT totp_secret FROM users WHERE id=?")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    verify_totp(
        &st,
        u.id,
        secret.as_deref().ok_or_else(|| bad("请先开始设置"))?,
        &u.email,
        &req.code,
    )
    .await?;
    sqlx::query("UPDATE users SET totp_enabled=1 WHERE id=?")
        .bind(u.id)
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=?")
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
        return Err(bad("密码至少需要 10 个字符"));
    }
    let (hash,): (Option<String>,) = sqlx::query_as("SELECT password_hash FROM users WHERE id=?")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if let Some(h) = hash {
        if !verify_password(req.current.as_deref().unwrap_or(""), &h) {
            return Err(bad("目前密码不正确"));
        }
    }
    sqlx::query("UPDATE users SET password_hash=? WHERE id=?")
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
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE email=?")
        .bind(req.email.trim())
        .fetch_optional(&st.db)
        .await?;
    if exists.is_some() {
        sqlx::query("INSERT INTO recovery_requests (email, reason, created_at) VALUES (?,?,?)")
            .bind(req.email.trim())
            .bind(req.reason.chars().take(500).collect::<String>())
            .bind(now())
            .execute(&st.db)
            .await?;
        notify_roles(
            &st.db,
            &["admin"],
            "新的账号恢复申请",
            &req.email,
            "/admin/members",
        )
        .await;
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------- Invites ----------

async fn invite_preview(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<Value>> {
    let row: Option<(String, String, String, String, Option<String>)> =
        sqlx::query_as("SELECT kind, email, name, expires_at, used_at FROM invites WHERE token=?")
            .bind(&token)
            .fetch_optional(&st.db)
            .await?;
    let Some((kind, email, name, expires, used)) = row else {
        return Err(AppError::NotFound);
    };
    if used.is_some() || expires < now() {
        return Err(bad("邀请连结已失效，请联系管理员"));
    }
    Ok(Json(json!({ "kind": kind, "email": email, "name": name })))
}

#[derive(Deserialize)]
struct AcceptReq {
    name: Option<String>,
    phone: Option<String>,
    password: Option<String>,
}

async fn invite_accept(
    State(st): State<AppState>,
    jar: CookieJar,
    Path(token): Path<String>,
    Json(req): Json<AcceptReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let row: Option<(i64, String, String, String, String, String, Option<i64>, String, Option<String>)> = sqlx::query_as(
        "SELECT id, kind, email, name, roles, group_ids, user_id, expires_at, used_at FROM invites WHERE token=?",
    )
    .bind(&token)
    .fetch_optional(&st.db)
    .await?;
    let Some((inv_id, kind, email, inv_name, roles, group_ids, rec_user, expires, used)) = row
    else {
        return Err(AppError::NotFound);
    };
    if used.is_some() || expires < now() {
        return Err(bad("邀请连结已失效，请联系管理员"));
    }
    // Mark used first (single use, guards against double submit).
    let r = sqlx::query("UPDATE invites SET used_at=? WHERE id=? AND used_at IS NULL")
        .bind(now())
        .bind(inv_id)
        .execute(&st.db)
        .await?;
    if r.rows_affected() == 0 {
        return Err(bad("邀请连结已被使用"));
    }

    let uid = if kind == "recovery" {
        rec_user.ok_or_else(|| bad("恢复连结无效"))?
    } else {
        let name = req
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&inv_name)
            .to_string();
        if name.is_empty() {
            return Err(bad("请填写姓名"));
        }
        let pw_hash = match req.password.as_deref().filter(|p| !p.is_empty()) {
            Some(p) if p.chars().count() < 10 => return Err(bad("密码至少需要 10 个字符")),
            Some(p) => Some(hash_password(p)?),
            None => None,
        };
        let id = sqlx::query(
            "INSERT INTO users (name, email, phone, roles, password_hash, webauthn_uuid, created_at) VALUES (?,?,?,?,?,?,?)",
        )
        .bind(&name)
        .bind(&email)
        .bind(req.phone.unwrap_or_default())
        .bind(&roles)
        .bind(pw_hash)
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(now())
        .execute(&st.db)
        .await
        .map_err(|_| bad("此电邮已经注册"))?
        .last_insert_rowid();
        let gids: Vec<i64> = serde_json::from_str(&group_ids).unwrap_or_default();
        for g in gids {
            sqlx::query("INSERT OR IGNORE INTO group_members (group_id, user_id) VALUES (?,?)")
                .bind(g)
                .bind(id)
                .execute(&st.db)
                .await?;
        }
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
        &email,
    )
    .await;
    Ok((jar, Json(json!({ "ok": true, "kind": kind }))))
}
