//! Hardware-backed Android biometric authentication (ECDSA P-256 challenge-response).

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

use super::create_session;
use super::passkeys::{require_recent_sign_in, save_challenge, take_challenge};
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, now, random_token, t, Lang};

const MAX_BIOMETRICS_PER_USER: usize = 10;

fn attestation_required(st: &AppState) -> bool {
    st.cfg.require_biometric_attestation && !st.cfg.dev_mode
}

/// ASN.1 DER SubjectPublicKeyInfo header for an EC P-256 (`secp256r1`) uncompressed key.
const P256_SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

fn decode_b64_any(raw: &str) -> Option<Vec<u8>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(trimmed)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(trimmed))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(trimmed))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(trimmed))
        .ok()
}

/// Parses an ECDSA P-256 public key from either 91-byte X.509 SPKI DER (`PublicKey.getEncoded()`)
/// or 65-byte uncompressed SEC1 (`0x04 || X || Y`) base64/base64url into the 65-byte uncompressed point.
pub(crate) fn parse_p256_public_key(raw_b64: &str) -> Option<Vec<u8>> {
    let bytes = decode_b64_any(raw_b64)?;
    if bytes.len() == 91 && bytes[..26] == P256_SPKI_PREFIX && bytes[26] == 0x04 {
        return Some(bytes[26..91].to_vec());
    }
    if bytes.len() == 65 && bytes[0] == 0x04 {
        return Some(bytes);
    }
    None
}

/// Verifies an ASN.1 DER ECDSA P-256 SHA-256 signature over the domain-separated message
/// `graceride-biometric-v1:{purpose}:{challenge}`.
pub(crate) fn verify_p256_signature(
    pubkey_65: &[u8],
    purpose: &str,
    challenge: &str,
    sig_b64: &str,
) -> bool {
    if pubkey_65.len() != 65 {
        return false;
    }
    let Some(sig_der) = decode_b64_any(sig_b64) else {
        return false;
    };
    let msg = format!("graceride-biometric-v1:{purpose}:{challenge}");
    ring::signature::UnparsedPublicKey::new(&ring::signature::ECDSA_P256_SHA256_ASN1, pubkey_65)
        .verify(msg.as_bytes(), &sig_der)
        .is_ok()
}

fn random_challenge_b64() -> String {
    let bytes: [u8; 32] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Serialize, Deserialize)]
struct BioChallengeState {
    cred_row_id: i64,
    challenge: String,
}

pub(super) async fn biometric_register_start(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    let challenge = random_challenge_b64();
    let cid = save_challenge(&st, u.id, "bio_reg", challenge.clone()).await?;
    Ok(Json(json!({ "challenge_id": cid, "challenge": challenge })))
}

#[derive(Deserialize)]
pub(super) struct BioRegFinishReq {
    challenge_id: String,
    public_key: String,
    signature: String,
    label: Option<String>,
    #[serde(default)]
    attestation_chain: Option<Vec<String>>,
}

pub(super) async fn biometric_register_finish(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<BioRegFinishReq>,
) -> AppResult<Json<Value>> {
    let (uid, challenge) = take_challenge(&st, &req.challenge_id, "bio_reg", u.lang).await?;
    if uid != u.id {
        return Err(forbidden(u.lang, ErrorKey::ChallengeMismatch));
    }
    let Some(pubkey) = parse_p256_public_key(&req.public_key) else {
        return Err(bad(u.lang, ErrorKey::BiometricRegistrationFailed));
    };
    if !verify_p256_signature(&pubkey, "register", &challenge, &req.signature) {
        return Err(bad(u.lang, ErrorKey::BiometricRegistrationFailed));
    }
    let mut attested = false;
    if let Some(chain_b64) = req.attestation_chain.as_ref().filter(|v| !v.is_empty()) {
        use base64::Engine as _;
        let mut chain_der = Vec::with_capacity(chain_b64.len());
        for entry in chain_b64 {
            let der = base64::engine::general_purpose::STANDARD
                .decode(entry.trim())
                .map_err(|_| bad(u.lang, ErrorKey::BiometricRegistrationFailed))?;
            chain_der.push(der);
        }
        if attestation_required(&st) && st.cfg.android_cert_sha256_fingerprints.is_empty() {
            return Err(bad(u.lang, ErrorKey::BiometricRegistrationFailed));
        }
        let trusted_roots = crate::attestation::trusted_root_spki_hashes();
        if let Err(err) = crate::attestation::verify_android_key_attestation(
            &chain_der,
            &pubkey,
            &challenge,
            &st.cfg.android_cert_sha256_fingerprints,
            &trusted_roots,
            chrono::Utc::now(),
        ) {
            tracing::warn!(user_id = u.id, error = %err, "Android biometric attestation rejected");
            return Err(bad(u.lang, ErrorKey::BiometricRegistrationFailed));
        }
        attested = true;
    } else if attestation_required(&st) {
        return Err(bad(u.lang, ErrorKey::BiometricRegistrationFailed));
    }
    let label = req
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(60).collect::<String>())
        .unwrap_or_else(|| t(u.lang, "我的安卓设备", "My Android device").into());

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let existing_ids: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM biometric_credentials
         WHERE user_id=$1
         ORDER BY COALESCE(last_used, created_at) ASC, id ASC",
    )
    .bind(u.id)
    .fetch_all(&mut *tx)
    .await?;
    if existing_ids.len() >= MAX_BIOMETRICS_PER_USER {
        let to_drop = existing_ids.len() - (MAX_BIOMETRICS_PER_USER - 1);
        for (old_id,) in existing_ids.into_iter().take(to_drop) {
            sqlx::query("DELETE FROM biometric_credentials WHERE id=$1 AND user_id=$2")
                .bind(old_id)
                .bind(u.id)
                .execute(&mut *tx)
                .await?;
        }
    }

    let cred_id = random_token(48);
    let ts_now = now();
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO biometric_credentials (user_id, cred_id, public_key, label, created_at, updated_at, attested)
         VALUES ($1,$2,$3,$4,$5,$5,$6) RETURNING id",
    )
    .bind(u.id)
    .bind(&cred_id)
    .bind(&pubkey)
    .bind(&label)
    .bind(&ts_now)
    .bind(attested)
    .fetch_one(&mut *tx)
    .await?;
    // Registering hardware-backed biometrics upgrades the current session to strong auth.
    sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
        .bind(&u.session_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "biometric.register",
        "user",
        Some(u.id),
        &label,
    )
    .await;
    Ok(Json(json!({
        "ok": true,
        "id": id,
        "credential_id": cred_id,
        "label": label,
    })))
}

#[derive(Deserialize)]
pub(super) struct BioLoginStartReq {
    credential_id: String,
    #[serde(default)]
    turnstile_token: Option<String>,
}

pub(super) async fn biometric_login_start(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Json(req): Json<BioLoginStartReq>,
) -> AppResult<Json<Value>> {
    let lang = Lang::from_headers(&headers);
    let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
    st.turnstile
        .verify(
            req.turnstile_token.as_deref(),
            "biometric_login",
            Some(&ip),
            lang,
        )
        .await?;
    let challenge = random_challenge_b64();
    let cred_id = req.credential_id.trim();
    let row: Option<(i64, i64, bool)> = if !cred_id.is_empty() && cred_id.len() <= 128 {
        sqlx::query_as(
            "SELECT bc.id, bc.user_id, bc.attested
             FROM biometric_credentials bc
             JOIN users u ON u.id = bc.user_id
             WHERE bc.cred_id = $1 AND u.status IN ('active', 'pending')",
        )
        .bind(cred_id)
        .fetch_optional(&st.db)
        .await?
    } else {
        None
    };

    // Unknown, revoked, or unattested (when attestation required) credential_id:
    // answer with an indistinguishable decoy challenge.
    let Some((bc_id, uid, attested)) = row else {
        return Ok(Json(json!({
            "challenge_id": uuid::Uuid::new_v4().to_string(),
            "challenge": challenge,
        })));
    };
    if attestation_required(&st) && !attested {
        return Ok(Json(json!({
            "challenge_id": uuid::Uuid::new_v4().to_string(),
            "challenge": challenge,
        })));
    }

    let state = serde_json::to_string(&BioChallengeState {
        cred_row_id: bc_id,
        challenge: challenge.clone(),
    })
    .map_err(anyhow::Error::from)?;
    let cid = save_challenge(&st, uid, "bio_login", state).await?;
    Ok(Json(json!({ "challenge_id": cid, "challenge": challenge })))
}

#[derive(Deserialize)]
pub(super) struct BioFinishReq {
    challenge_id: String,
    credential_id: String,
    signature: String,
}

pub(super) async fn biometric_login_finish(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<BioFinishReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = Lang::from_headers(&headers);
    let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
    let Ok((uid, state_json)) = take_challenge(&st, &req.challenge_id, "bio_login", lang).await
    else {
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            None,
            format!("unknown or expired biometric sign-in challenge; from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::BiometricFailed));
    };
    let st_data: BioChallengeState =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let row: Option<(Vec<u8>, String, bool)> = sqlx::query_as(
        "SELECT bc.public_key, u.status, bc.attested
         FROM biometric_credentials bc
         JOIN users u ON u.id = bc.user_id
         WHERE bc.id = $1 AND bc.user_id = $2 AND bc.cred_id = $3",
    )
    .bind(st_data.cred_row_id)
    .bind(uid)
    .bind(req.credential_id.trim())
    .fetch_optional(&st.db)
    .await?;

    let Some((pubkey, status, attested)) = row else {
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            Some(uid),
            format!("biometric credential not found or revoked; from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::BiometricFailed));
    };
    if attestation_required(&st) && !attested {
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            Some(uid),
            format!("unattested biometric credential rejected; from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::BiometricFailed));
    }
    if !matches!(status.as_str(), "active" | "pending") {
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
    if !verify_p256_signature(&pubkey, "login", &st_data.challenge, &req.signature) {
        audit(
            &st.db,
            None,
            "login.failed",
            "user",
            Some(uid),
            format!("biometric signature rejected; from {ip}"),
        )
        .await;
        return Err(bad(lang, ErrorKey::BiometricFailed));
    }

    let ts_now = now();
    sqlx::query("UPDATE biometric_credentials SET last_used=$1, updated_at=$1 WHERE id=$2")
        .bind(&ts_now)
        .bind(st_data.cred_row_id)
        .execute(&st.db)
        .await?;

    let jar = create_session(&st, jar, uid, true).await?;
    audit(&st.db, Some(uid), "login.biometric", "user", Some(uid), "").await;
    Ok((jar, Json(json!({ "ok": true }))))
}

#[derive(Deserialize)]
pub(super) struct BioReauthStartReq {
    credential_id: String,
}

pub(super) async fn biometric_reauth_start(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<BioReauthStartReq>,
) -> AppResult<Json<Value>> {
    let cred_id = req.credential_id.trim();
    let row: Option<(i64, bool)> = if !cred_id.is_empty() && cred_id.len() <= 128 {
        sqlx::query_as(
            "SELECT id, attested FROM biometric_credentials WHERE user_id=$1 AND cred_id=$2",
        )
        .bind(u.id)
        .bind(cred_id)
        .fetch_optional(&st.db)
        .await?
    } else {
        None
    };
    let Some((bc_id, attested)) = row else {
        return Err(bad(u.lang, ErrorKey::BiometricFailed));
    };
    if attestation_required(&st) && !attested {
        return Err(bad(u.lang, ErrorKey::BiometricFailed));
    }
    let challenge = random_challenge_b64();
    let state = serde_json::to_string(&BioChallengeState {
        cred_row_id: bc_id,
        challenge: challenge.clone(),
    })
    .map_err(anyhow::Error::from)?;
    let cid = save_challenge(&st, u.id, "bio_reauth", state).await?;
    Ok(Json(json!({ "challenge_id": cid, "challenge": challenge })))
}

pub(super) async fn biometric_reauth_finish(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<BioFinishReq>,
) -> AppResult<Json<Value>> {
    let (uid, state_json) = take_challenge(&st, &req.challenge_id, "bio_reauth", u.lang).await?;
    if uid != u.id {
        return Err(forbidden(u.lang, ErrorKey::ChallengeMismatch));
    }
    let st_data: BioChallengeState =
        serde_json::from_str(&state_json).map_err(anyhow::Error::from)?;
    let row: Option<(Vec<u8>, bool)> = sqlx::query_as(
        "SELECT public_key, attested FROM biometric_credentials WHERE id=$1 AND user_id=$2 AND cred_id=$3",
    )
    .bind(st_data.cred_row_id)
    .bind(u.id)
    .bind(req.credential_id.trim())
    .fetch_optional(&st.db)
    .await?;
    let Some((pubkey, attested)) = row else {
        return Err(bad(u.lang, ErrorKey::BiometricFailed));
    };
    if attestation_required(&st) && !attested {
        return Err(bad(u.lang, ErrorKey::BiometricFailed));
    }
    if !verify_p256_signature(&pubkey, "reauth", &st_data.challenge, &req.signature) {
        audit(
            &st.db,
            Some(u.id),
            "login.failed",
            "user",
            Some(u.id),
            "biometric rejected on re-check",
        )
        .await;
        return Err(bad(u.lang, ErrorKey::BiometricFailed));
    }

    let ts_now = now();
    sqlx::query("UPDATE biometric_credentials SET last_used=$1, updated_at=$1 WHERE id=$2")
        .bind(&ts_now)
        .bind(st_data.cred_row_id)
        .execute(&st.db)
        .await?;
    sqlx::query(
        "UPDATE sessions SET created_at=$1, locked_at=NULL, last_seen_at=$1, mfa_ok=1 WHERE token=$2",
    )
    .bind(&ts_now)
    .bind(&u.session_id)
    .execute(&st.db)
    .await?;
    audit(
        &st.db,
        Some(u.id),
        "login.biometric_reauth",
        "user",
        Some(u.id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn list_biometrics(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, String, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT id, cred_id, label, created_at, last_used, attested
         FROM biometric_credentials
         WHERE user_id=$1
         ORDER BY id",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(
            |(id, cred_id, label, created_at, last_used, attested)| json!({
                "id": id,
                "credential_id": cred_id,
                "label": label,
                "created_at": created_at,
                "last_used": last_used,
                "attested": attested,
            })
        )
        .collect::<Vec<_>>())))
}

pub(super) async fn delete_biometric(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    let gone: Option<(String,)> = sqlx::query_as(
        "DELETE FROM biometric_credentials WHERE id=$1 AND user_id=$2 RETURNING label",
    )
    .bind(id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    if let Some((label,)) = gone {
        audit(
            &st.db,
            Some(u.id),
            "biometric.delete",
            "user",
            Some(u.id),
            format!("biometric {id} ({label})"),
        )
        .await;
    }
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};

    #[test]
    fn parses_spki_and_raw_sec1_and_verifies_domain_separated_signatures() {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
        let kp = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng)
            .unwrap();
        let raw_pub = kp.public_key().as_ref();
        assert_eq!(raw_pub.len(), 65);
        assert_eq!(raw_pub[0], 0x04);

        let raw_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_pub);
        assert_eq!(parse_p256_public_key(&raw_b64).as_deref(), Some(raw_pub));

        let mut spki = Vec::with_capacity(91);
        spki.extend_from_slice(&P256_SPKI_PREFIX);
        spki.extend_from_slice(raw_pub);
        let spki_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&spki);
        assert_eq!(parse_p256_public_key(&spki_b64).as_deref(), Some(raw_pub));

        let mut bad_spki = spki.clone();
        bad_spki[0] = 0x31;
        let bad_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bad_spki);
        assert!(parse_p256_public_key(&bad_b64).is_none());

        let challenge = random_challenge_b64();
        let sig = kp
            .sign(
                &rng,
                format!("graceride-biometric-v1:login:{challenge}").as_bytes(),
            )
            .unwrap();
        let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.as_ref());

        assert!(verify_p256_signature(
            raw_pub, "login", &challenge, &sig_b64
        ));
        // Domain separation prevents cross-endpoint signature replay.
        assert!(!verify_p256_signature(
            raw_pub, "reauth", &challenge, &sig_b64
        ));
        assert!(!verify_p256_signature(
            raw_pub, "register", &challenge, &sig_b64
        ));
        assert!(!verify_p256_signature(
            raw_pub,
            "login",
            "other-challenge",
            &sig_b64
        ));
    }
}
