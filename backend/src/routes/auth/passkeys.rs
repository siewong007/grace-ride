//! Passkey (WebAuthn) authentication, re-authentication, registration, and management.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use webauthn_rs::fake::{FakePasskeyDistribution, WebauthnFakeCredentialGenerator};
use webauthn_rs::prelude::*;

use super::{create_session, revoke_other_sessions, rotate_session};
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, minutes_ago, now, t, Lang};

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

pub(super) async fn save_challenge(
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
    if user_id > 0 {
        // Bound concurrent live challenges per `(user_id, kind)` (at most 5 after insert).
        sqlx::query(
            "DELETE FROM auth_challenges
             WHERE user_id = $1 AND kind = $2
               AND id NOT IN (
                   SELECT id FROM auth_challenges
                   WHERE user_id = $1 AND kind = $2
                   ORDER BY created_at DESC, id DESC
                   LIMIT 4
               )",
        )
        .bind(user_id)
        .bind(kind)
        .execute(&st.db)
        .await?;
    }
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

pub(super) async fn take_challenge(
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
pub(super) struct EmailReq {
    email: String,
    #[serde(default)]
    turnstile_token: Option<String>,
}

pub(super) async fn passkey_login_start(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Json(req): Json<EmailReq>,
) -> AppResult<Json<Value>> {
    let lang = Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
    st.turnstile
        .verify(
            req.turnstile_token.as_deref(),
            "passkey_login",
            Some(&ip),
            lang,
        )
        .await?;
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
    let encoded =
        crate::crypto::encrypt_secret("app_secret:decoy_passkey_hmac", &hex::encode(fresh))
            .map_err(AppError::Other)?;
    sqlx::query(
        "INSERT INTO app_secrets (name, value) VALUES ('decoy_passkey_hmac', $1) ON CONFLICT (name) DO NOTHING",
    )
    .bind(&encoded)
    .execute(&st.db)
    .await?;
    let (key_raw,): (String,) =
        sqlx::query_as("SELECT value FROM app_secrets WHERE name='decoy_passkey_hmac'")
            .fetch_one(&st.db)
            .await?;
    let key = crate::crypto::decrypt_secret("app_secret:decoy_passkey_hmac", &key_raw)
        .map_err(AppError::Other)?;
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
            "challenge": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(challenge),
            "timeout": 300_000,
            "rpId": rp_id,
            "allowCredentials": ids.iter().map(|id| json!({"type": "public-key", "id": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id.as_ref() as &[u8])})).collect::<Vec<_>>(),
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
pub(super) async fn require_recent_sign_in(st: &AppState, u: &CurrentUser) -> AppResult<()> {
    let (keys, biometrics, totp, google_links, has_password, signed_in): (
        i64,
        i64,
        i64,
        i64,
        bool,
        String,
    ) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM passkeys WHERE user_id=$1),
            (SELECT COUNT(*) FROM biometric_credentials WHERE user_id=$1),
            (SELECT totp_enabled FROM users WHERE id=$1),
            (SELECT COUNT(*) FROM oauth_accounts WHERE user_id=$1 AND provider='google'),
            (SELECT password_hash IS NOT NULL FROM users WHERE id=$1),
            (SELECT created_at FROM sessions WHERE token=$2)",
    )
    .bind(u.id)
    .bind(&u.session_id)
    .fetch_one(&st.db)
    .await?;
    let has_strong_factor = keys > 0 || biometrics > 0 || totp == 1;
    let has_any_factor = has_strong_factor || has_password || google_links > 0;
    if has_any_factor && signed_in < minutes_ago(15) {
        return Err(forbidden(u.lang, ErrorKey::RecentSignInRequired));
    }
    if has_strong_factor && !u.mfa_ok && !st.cfg.dev_mode {
        return Err(forbidden(u.lang, ErrorKey::MfaRequired));
    }
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct LoginFinish {
    challenge_id: String,
    credential: PublicKeyCredential,
}

pub(super) async fn passkey_login_finish(
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
    let u_status: Option<(String,)> = sqlx::query_as("SELECT status FROM users WHERE id=$1")
        .bind(uid)
        .fetch_optional(&st.db)
        .await?;
    if !matches!(
        u_status.as_ref().map(|s| s.0.as_str()),
        Some("active" | "pending")
    ) {
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

#[derive(Serialize, Deserialize)]
struct ReauthChallengeState {
    session_id: String,
    auth_state: PasskeyAuthentication,
}

/// Starts a passkey check for the member who is already signed in, so they can renew
/// their "recent sign-in" without signing out first.
pub(super) async fn passkey_reauth_start(
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
    let wrapped = ReauthChallengeState {
        session_id: u.session_id.clone(),
        auth_state,
    };
    let cid = save_challenge(
        &st,
        u.id,
        "reauth",
        serde_json::to_string(&wrapped).map_err(anyhow::Error::from)?,
    )
    .await?;
    Ok(Json(json!({ "challenge_id": cid, "options": rcr })))
}

/// Finishes the check. The passkey must belong to the signed-in member; on success the
/// session is rotated and counts as signed in just now.
pub(super) async fn passkey_reauth_finish(
    State(st): State<AppState>,
    jar: CookieJar,
    u: CurrentUser,
    Json(req): Json<LoginFinish>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let (uid, state_json) = take_challenge(&st, &req.challenge_id, "reauth", u.lang).await?;
    if uid != u.id {
        return Err(forbidden(u.lang, ErrorKey::ChallengeMismatch));
    }
    let wrapped: ReauthChallengeState =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    if wrapped.session_id != u.session_id {
        return Err(forbidden(u.lang, ErrorKey::ChallengeMismatch));
    }
    let Ok(res) = st
        .webauthn
        .finish_passkey_authentication(&req.credential, &wrapped.auth_state)
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
    let jar = rotate_session(&st, jar, &u, true, true, true).await?;
    audit(&st.db, Some(u.id), "login.reauth", "user", Some(u.id), "").await;
    Ok((jar, Json(json!({ "ok": true }))))
}

pub(super) async fn passkey_register_start(
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
pub(super) struct RegFinish {
    challenge_id: String,
    credential: RegisterPublicKeyCredential,
    label: Option<String>,
}

pub(super) async fn passkey_register_finish(
    State(st): State<AppState>,
    jar: CookieJar,
    u: CurrentUser,
    Json(req): Json<RegFinish>,
) -> AppResult<(CookieJar, Json<Value>)> {
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
    // Registering a passkey upgrades and rotates the current session.
    let jar = rotate_session(&st, jar, &u, true, true, false).await?;
    audit(
        &st.db,
        Some(u.id),
        "passkey.register",
        "user",
        Some(u.id),
        &label,
    )
    .await;
    Ok((jar, Json(json!({ "ok": true }))))
}

pub(super) async fn list_passkeys(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
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

pub(super) async fn delete_passkey(
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
        revoke_other_sessions(&st.db, u.id, &u.session_id).await?;
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
