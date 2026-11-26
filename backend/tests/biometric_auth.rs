//! Integration tests for hardware-backed biometric ECDSA P-256 registration, login, reauth, and revocation.

mod common;

use axum::http::{header, Method, StatusCode};
use base64::Engine as _;
use common::*;
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
use serde_json::json;

const P256_SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

struct TestEcKey {
    pair: EcdsaKeyPair,
    rng: SystemRandom,
}

impl TestEcKey {
    fn generate() -> Self {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
        let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng)
            .unwrap();
        Self { pair, rng }
    }

    fn spki_b64(&self) -> String {
        let mut spki = Vec::with_capacity(91);
        spki.extend_from_slice(&P256_SPKI_PREFIX);
        spki.extend_from_slice(self.pair.public_key().as_ref());
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(spki)
    }

    fn sign_challenge(&self, purpose: &str, challenge: &str) -> String {
        let msg = format!("graceride-biometric-v1:{purpose}:{challenge}");
        let sig = self.pair.sign(&self.rng, msg.as_bytes()).unwrap();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.as_ref())
    }
}

fn extract_session_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|raw| {
            let pair = raw.split(';').next()?.trim();
            if pair.starts_with("gr_session=") && !pair.ends_with('=') {
                Some(pair.to_string())
            } else {
                None
            }
        })
}

#[tokio::test]
async fn biometric_register_login_reauth_and_delete_round_trip() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let key = TestEcKey::generate();

    // 1. Register hardware-backed public key.
    let reg_start = call(
        &app,
        Method::POST,
        "/api/auth/biometric/register/start",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(reg_start.status, StatusCode::OK, "{}", reg_start.json);
    let cid = reg_start.json["challenge_id"].as_str().unwrap();
    let challenge = reg_start.json["challenge"].as_str().unwrap();

    // Reject wrong-domain signature during registration.
    let wrong_reg = call(
        &app,
        Method::POST,
        "/api/auth/biometric/register/finish",
        Some(&cookie),
        Some(json!({
            "challenge_id": cid,
            "public_key": key.spki_b64(),
            "signature": key.sign_challenge("login", challenge),
            "label": "Pixel 9"
        })),
    )
    .await;
    assert_eq!(
        wrong_reg.status,
        StatusCode::BAD_REQUEST,
        "{}",
        wrong_reg.json
    );
    assert_eq!(wrong_reg.json["code"], "BIOMETRIC_REGISTRATION_FAILED");

    // Fresh registration succeeds.
    let reg_start2 = call(
        &app,
        Method::POST,
        "/api/auth/biometric/register/start",
        Some(&cookie),
        None,
    )
    .await;
    let cid2 = reg_start2.json["challenge_id"].as_str().unwrap();
    let challenge2 = reg_start2.json["challenge"].as_str().unwrap();
    let reg_finish = call(
        &app,
        Method::POST,
        "/api/auth/biometric/register/finish",
        Some(&cookie),
        Some(json!({
            "challenge_id": cid2,
            "public_key": key.spki_b64(),
            "signature": key.sign_challenge("register", challenge2),
            "label": "Pixel 9"
        })),
    )
    .await;
    assert_eq!(reg_finish.status, StatusCode::OK, "{}", reg_finish.json);
    let cookie = extract_session_cookie(&reg_finish.headers).expect("rotated cookie");
    let bio_id = reg_finish.json["id"].as_i64().unwrap();
    let cred_id = reg_finish.json["credential_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(reg_finish.json["label"], "Pixel 9");

    let me_res = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me_res.status, StatusCode::OK);
    assert_eq!(me_res.json["biometric_count"], 1);

    let listed = call(
        &app,
        Method::GET,
        "/api/auth/biometrics",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.json.as_array().unwrap().len(), 1);
    assert_eq!(listed.json[0]["credential_id"], cred_id);

    // 2. Sign out and log back in directly with biometric challenge-response.
    assert_eq!(
        call(&app, Method::POST, "/api/auth/logout", Some(&cookie), None)
            .await
            .status,
        StatusCode::OK
    );

    // Unknown credential_id gets an indistinguishable decoy challenge.
    let decoy = call(
        &app,
        Method::POST,
        "/api/auth/biometric/login/start",
        None,
        Some(json!({ "credential_id": "nonexistent_credential_id_0123456789abcdef" })),
    )
    .await;
    assert_eq!(decoy.status, StatusCode::OK, "{}", decoy.json);
    assert!(decoy.json["challenge_id"].as_str().is_some());
    assert!(decoy.json["challenge"].as_str().is_some());

    let decoy_finish = call(
        &app,
        Method::POST,
        "/api/auth/biometric/login/finish",
        None,
        Some(json!({
            "challenge_id": decoy.json["challenge_id"],
            "credential_id": "nonexistent_credential_id_0123456789abcdef",
            "signature": key.sign_challenge("login", decoy.json["challenge"].as_str().unwrap()),
        })),
    )
    .await;
    assert_eq!(decoy_finish.status, StatusCode::BAD_REQUEST);
    assert_eq!(decoy_finish.json["code"], "BIOMETRIC_FAILED");

    // Real direct biometric sign-in succeeds and mints an mfa_ok session.
    let lstart = call(
        &app,
        Method::POST,
        "/api/auth/biometric/login/start",
        None,
        Some(json!({ "credential_id": cred_id })),
    )
    .await;
    assert_eq!(lstart.status, StatusCode::OK, "{}", lstart.json);
    let lcid = lstart.json["challenge_id"].as_str().unwrap();
    let lchal = lstart.json["challenge"].as_str().unwrap();

    // Cross-endpoint domain replay ("reauth" sig on "login") is rejected.
    let bad_domain = call(
        &app,
        Method::POST,
        "/api/auth/biometric/login/finish",
        None,
        Some(json!({
            "challenge_id": lcid,
            "credential_id": cred_id,
            "signature": key.sign_challenge("reauth", lchal),
        })),
    )
    .await;
    assert_eq!(bad_domain.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_domain.json["code"], "BIOMETRIC_FAILED");

    let lstart2 = call(
        &app,
        Method::POST,
        "/api/auth/biometric/login/start",
        None,
        Some(json!({ "credential_id": cred_id })),
    )
    .await;
    let lcid2 = lstart2.json["challenge_id"].as_str().unwrap();
    let lchal2 = lstart2.json["challenge"].as_str().unwrap();
    let lfinish = call(
        &app,
        Method::POST,
        "/api/auth/biometric/login/finish",
        None,
        Some(json!({
            "challenge_id": lcid2,
            "credential_id": cred_id,
            "signature": key.sign_challenge("login", lchal2),
        })),
    )
    .await;
    assert_eq!(lfinish.status, StatusCode::OK, "{}", lfinish.json);
    let bio_cookie = extract_session_cookie(&lfinish.headers).expect("session cookie");

    let bio_me = call(&app, Method::GET, "/api/auth/me", Some(&bio_cookie), None).await;
    assert_eq!(bio_me.status, StatusCode::OK);
    assert_eq!(bio_me.json["email"], "member@example.org");
    assert_eq!(bio_me.json["mfa_ok"], true);

    // 3. Lock session and unlock via biometric reauth.
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/lock",
            Some(&bio_cookie),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let blocked = call(&app, Method::GET, "/api/rides", Some(&bio_cookie), None).await;
    assert_eq!(blocked.status, StatusCode::LOCKED);

    // dev_unlock must be refused once a biometric credential exists.
    let dev_unlock = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&bio_cookie),
        Some(json!({ "dev_unlock": true })),
    )
    .await;
    assert_eq!(dev_unlock.status, StatusCode::BAD_REQUEST);
    assert_eq!(dev_unlock.json["code"], "UNLOCK_METHOD_REQUIRED");

    let rstart = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/start",
        Some(&bio_cookie),
        Some(json!({ "credential_id": cred_id })),
    )
    .await;
    assert_eq!(rstart.status, StatusCode::OK, "{}", rstart.json);
    let rfinish = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/finish",
        Some(&bio_cookie),
        Some(json!({
            "challenge_id": rstart.json["challenge_id"],
            "credential_id": cred_id,
            "signature": key.sign_challenge("reauth", rstart.json["challenge"].as_str().unwrap()),
        })),
    )
    .await;
    assert_eq!(rfinish.status, StatusCode::OK, "{}", rfinish.json);
    let bio_cookie = extract_session_cookie(&rfinish.headers).expect("rotated cookie");

    let unblocked = call(&app, Method::GET, "/api/rides", Some(&bio_cookie), None).await;
    assert_eq!(unblocked.status, StatusCode::OK);

    // 4. Stale sign-in requires recent reauth before deleting biometric credential.
    let raw_tok = bio_cookie.strip_prefix("gr_session=").unwrap();
    sqlx::query("UPDATE sessions SET created_at=$1 WHERE token=$2")
        .bind(grace_ride::util::minutes_ago(20))
        .bind(grace_ride::auth::hash_token(raw_tok))
        .execute(&st.db)
        .await
        .unwrap();

    let stale_del = call(
        &app,
        Method::DELETE,
        &format!("/api/auth/biometrics/{bio_id}"),
        Some(&bio_cookie),
        None,
    )
    .await;
    assert_eq!(stale_del.status, StatusCode::FORBIDDEN);
    assert_eq!(stale_del.json["code"], "RECENT_SIGN_IN_REQUIRED");

    // Re-authenticate with biometrics, then delete succeeds.
    let rstart2 = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/start",
        Some(&bio_cookie),
        Some(json!({ "credential_id": cred_id })),
    )
    .await;
    let rfinish2 = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/finish",
        Some(&bio_cookie),
        Some(json!({
            "challenge_id": rstart2.json["challenge_id"],
            "credential_id": cred_id,
            "signature": key.sign_challenge("reauth", rstart2.json["challenge"].as_str().unwrap()),
        })),
    )
    .await;
    assert_eq!(rfinish2.status, StatusCode::OK);
    let bio_cookie = extract_session_cookie(&rfinish2.headers).expect("rotated cookie");

    let del_ok = call(
        &app,
        Method::DELETE,
        &format!("/api/auth/biometrics/{bio_id}"),
        Some(&bio_cookie),
        None,
    )
    .await;
    assert_eq!(del_ok.status, StatusCode::OK);

    let after = call(
        &app,
        Method::GET,
        "/api/auth/biometrics",
        Some(&bio_cookie),
        None,
    )
    .await;
    assert_eq!(after.json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn biometric_registration_caps_at_ten_and_recovery_revokes_credentials() {
    let (app, st) = setup().await;
    let mut member_cookie = login(&st, "member@example.org").await;

    let mut cred_ids = Vec::new();
    for idx in 0..11 {
        let key = TestEcKey::generate();
        let rs = call(
            &app,
            Method::POST,
            "/api/auth/biometric/register/start",
            Some(&member_cookie),
            None,
        )
        .await;
        assert_eq!(rs.status, StatusCode::OK, "{idx}: {}", rs.json);
        let rf = call(
            &app,
            Method::POST,
            "/api/auth/biometric/register/finish",
            Some(&member_cookie),
            Some(json!({
                "challenge_id": rs.json["challenge_id"],
                "public_key": key.spki_b64(),
                "signature": key.sign_challenge("register", rs.json["challenge"].as_str().unwrap()),
                "label": format!("Device {idx}"),
            })),
        )
        .await;
        assert_eq!(rf.status, StatusCode::OK, "{idx}: {}", rf.json);
        member_cookie = extract_session_cookie(&rf.headers).expect("rotated cookie");
        cred_ids.push((rf.json["credential_id"].as_str().unwrap().to_string(), key));
    }

    let listed = call(
        &app,
        Method::GET,
        "/api/auth/biometrics",
        Some(&member_cookie),
        None,
    )
    .await;
    let arr = listed.json.as_array().unwrap();
    assert_eq!(arr.len(), 10);
    assert!(arr.iter().all(|row| row["credential_id"] != cred_ids[0].0));

    // Request and approve recovery as admin; member's biometric credentials must be revoked.
    let req_rec = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({ "email": "member@example.org", "reason": "Lost phone" })),
    )
    .await;
    assert_eq!(req_rec.status, StatusCode::OK, "{}", req_rec.json);

    let admin_cookie = login(&st, "admin@example.org").await;
    let rec_list = call(
        &app,
        Method::GET,
        "/api/admin/recovery",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(rec_list.status, StatusCode::OK, "{}", rec_list.json);
    let rec_id = rec_list.json[0]["id"].as_i64().unwrap();

    let approve = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rec_id}/approve"),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(approve.status, StatusCode::OK, "{}", approve.json);

    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM biometric_credentials bc JOIN users u ON u.id=bc.user_id WHERE u.email='member@example.org'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(count, 0);
}
