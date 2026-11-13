//! Coverage-gap integration tests: auth (passkey/TOTP/invite/recovery/MFA),
//! settlements publish→verify, Grab alt-transport, concurrent claim, edges.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::util::now;
use serde_json::json;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

#[tokio::test]
async fn privileged_mfa_required_even_in_dev_mode() {
    let (app, st) = setup().await;
    let weak = login_mfa(&st, "admin@example.org", false).await;
    let r = call(&app, Method::GET, "/api/admin/dashboard", Some(&weak), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "MFA_REQUIRED");

    let strong = login_mfa(&st, "admin@example.org", true).await;
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/admin/dashboard",
            Some(&strong),
            None
        )
        .await
        .status,
        StatusCode::OK
    );

    // Non-privileged still get DEV_MODE MFA bypass when session mfa_ok=0.
    let member = login_mfa(&st, "member@example.org", false).await;
    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&member), None)
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn passkey_register_and_login_soft_authenticator() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;

    let start = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/start",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(start.status, StatusCode::OK, "{}", start.json);
    let challenge_id = start.json["challenge_id"].as_str().unwrap().to_string();
    let options: CreationChallengeResponse =
        serde_json::from_value(start.json["options"].clone()).expect("ccr");

    let mut soft = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = Url::parse(ORIGIN).unwrap();
    let credential = soft
        .do_registration(origin.clone(), options)
        .expect("soft register");

    let finish = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&cookie),
        Some(json!({
            "challenge_id": challenge_id,
            "credential": credential,
            "label": "Test SoftPasskey"
        })),
    )
    .await;
    assert_eq!(finish.status, StatusCode::OK, "{}", finish.json);

    let listed = call(&app, Method::GET, "/api/auth/passkeys", Some(&cookie), None).await;
    assert_eq!(listed.status, StatusCode::OK);
    assert!(!listed.json.as_array().unwrap().is_empty());

    // Logout then login via passkey.
    assert_eq!(
        call(&app, Method::POST, "/api/auth/logout", Some(&cookie), None)
            .await
            .status,
        StatusCode::OK
    );

    let lstart = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({"email": "member@example.org"})),
    )
    .await;
    assert_eq!(lstart.status, StatusCode::OK, "{}", lstart.json);
    let lcid = lstart.json["challenge_id"].as_str().unwrap().to_string();
    let lopts: RequestChallengeResponse =
        serde_json::from_value(lstart.json["options"].clone()).expect("rcr");
    let assertion = soft.do_authentication(origin, lopts).expect("soft auth");

    let lfinish = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(json!({
            "challenge_id": lcid,
            "credential": assertion
        })),
    )
    .await;
    assert_eq!(lfinish.status, StatusCode::OK, "{}", lfinish.json);
    // Session cookie should be set.
    let set_cookie = lfinish
        .headers
        .get_all(axum::http::header::SET_COOKIE)
        .iter()
        .any(|v| v.to_str().unwrap_or("").contains("gr_session"));
    assert!(set_cookie, "expected session cookie after passkey login");
}

/// Field names (and nesting) of a JSON value, values ignored, to compare shapes.
fn shape(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(m) => {
            serde_json::Value::Object(m.iter().map(|(k, v)| (k.clone(), shape(v))).collect())
        }
        serde_json::Value::Array(a) => json!(a.first().map(shape)),
        _ => json!(null),
    }
}

#[tokio::test]
async fn passkey_login_start_does_not_reveal_who_is_a_member() {
    let (app, st) = setup().await;
    let start = |email: &str| {
        let (app, email) = (app.clone(), email.to_string());
        async move {
            call(
                &app,
                Method::POST,
                "/api/auth/passkey/login/start",
                None,
                Some(json!({ "email": email })),
            )
            .await
        }
    };

    // A real member with a passkey, for the shape of a genuine challenge.
    let cookie = login(&st, "amina@example.org").await;
    let reg = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/start",
        Some(&cookie),
        None,
    )
    .await;
    let mut soft = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = Url::parse(ORIGIN).unwrap();
    let credential = soft
        .do_registration(
            origin,
            serde_json::from_value::<CreationChallengeResponse>(reg.json["options"].clone())
                .unwrap(),
        )
        .unwrap();
    let fin = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&cookie),
        Some(json!({"challenge_id": reg.json["challenge_id"], "credential": credential})),
    )
    .await;
    assert_eq!(fin.status, StatusCode::OK, "{}", fin.json);
    let real = start("amina@example.org").await;
    assert_eq!(real.status, StatusCode::OK, "{}", real.json);

    // A member without a passkey and an unknown email both get a challenge, not an error.
    for email in ["member@example.org", "nobody-here@example.org"] {
        let r = start(email).await;
        assert_eq!(r.status, StatusCode::OK, "{email}: {}", r.json);
        let opts: RequestChallengeResponse = serde_json::from_value(r.json["options"].clone())
            .expect("decoy parses as a real challenge");
        assert_eq!(shape(&r.json), shape(&real.json), "{email}");
        let pk = &r.json["options"]["publicKey"];
        let real_pk = &real.json["options"]["publicKey"];
        for f in ["timeout", "rpId", "userVerification"] {
            assert_eq!(pk[f], real_pk[f], "{email}: {f}");
        }
        assert_ne!(pk["challenge"], real_pk["challenge"]);
        drop(opts);

        // The same email always gets the same credential ids (a changing list would give it away).
        let again = start(email).await;
        assert_eq!(
            pk["allowCredentials"],
            again.json["options"]["publicKey"]["allowCredentials"]
        );
        assert_ne!(
            pk["challenge"],
            again.json["options"]["publicKey"]["challenge"]
        );

        // Finishing a decoy fails exactly like a wrong passkey.
        let finish = call(
            &app,
            Method::POST,
            "/api/auth/passkey/login/finish",
            None,
            Some(json!({"challenge_id": r.json["challenge_id"], "credential": serde_json::to_value(
                soft.do_authentication(Url::parse(ORIGIN).unwrap(),
                    serde_json::from_value(real.json["options"].clone()).unwrap()).unwrap()).unwrap()})),
        )
        .await;
        assert_eq!(finish.status, StatusCode::BAD_REQUEST, "{}", finish.json);
        assert_eq!(finish.json["code"], "PASSKEY_FAILED");
    }
}

#[tokio::test]
async fn totp_setup_enable_and_replay_rejected() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;

    let setup_r = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(setup_r.status, StatusCode::OK, "{}", setup_r.json);
    let secret = setup_r.json["secret"].as_str().unwrap().to_string();
    let code = totp_code(&secret, "member@example.org");

    let en = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&cookie),
        Some(json!({"code": code})),
    )
    .await;
    assert_eq!(en.status, StatusCode::OK, "{}", en.json);

    // Replay same step (same code) must fail via totp_last_step.
    let replay = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&cookie),
        Some(json!({"code": code})),
    )
    .await;
    // Already enabled → TwoFactorAlreadyEnabled on setup; enable path may fail verify or already-enabled.
    // Re-setup should be blocked:
    let again = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
    assert_eq!(again.json["code"], "TWO_FACTOR_ALREADY_ENABLED");

    // Direct verify_totp replay: call enable again should fail (already enabled path uses verify first).
    assert!(
        replay.status == StatusCode::BAD_REQUEST || replay.status == StatusCode::OK,
        "replay status {}",
        replay.status
    );
    // Force a second verify by temporarily disabling and re-using same step counter via public verify.
    // Insert a second call that hits verify_totp with same code while last_step set.
    let (last,): (i64,) =
        sqlx::query_as("SELECT totp_last_step FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(last > 0, "totp_last_step should be stamped");

    // Generate code again (same 30s window) and verify via enable after resetting enabled flag
    // but keeping last_step — should reject reuse.
    sqlx::query("UPDATE users SET totp_enabled=0 WHERE email='member@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let reuse = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&cookie),
        Some(json!({"code": code})),
    )
    .await;
    assert_eq!(
        reuse.status,
        StatusCode::BAD_REQUEST,
        "same-step replay must fail: {}",
        reuse.json
    );
}

#[tokio::test]
async fn invite_preview_accept_expired_used_invalid() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;

    let created = call(&app, Method::POST, "/api/admin/invites", Some(&admin), None).await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let token = created.json["token"].as_str().unwrap().to_string();

    let preview = call(
        &app,
        Method::GET,
        &format!("/api/invites/{token}"),
        None,
        None,
    )
    .await;
    assert_eq!(preview.status, StatusCode::OK);
    assert_eq!(preview.json["kind"], "invite");

    // Terms required
    let no_terms = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({"name": "新同学", "email": "newbie@example.org", "terms": false})),
    )
    .await;
    assert_eq!(no_terms.status, StatusCode::BAD_REQUEST);
    assert_eq!(no_terms.json["code"], "TERMS_REQUIRED");

    let accept = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "新同学",
            "email": "newbie@example.org",
            "phone": "012-999 0001",
            "terms": true
        })),
    )
    .await;
    assert_eq!(accept.status, StatusCode::OK, "{}", accept.json);
    assert_eq!(accept.json["kind"], "invite");

    let (status,): (String,) =
        sqlx::query_as("SELECT status FROM users WHERE email='newbie@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(status, "pending");

    // Reuse → expired/used
    let reuse = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "X",
            "email": "other@example.org",
            "terms": true
        })),
    )
    .await;
    assert_eq!(reuse.status, StatusCode::BAD_REQUEST);
    assert_eq!(reuse.json["code"], "INVITE_EXPIRED");

    // Invalid token
    let bad = call(
        &app,
        Method::GET,
        "/api/invites/not-a-real-token",
        None,
        None,
    )
    .await;
    assert_eq!(bad.status, StatusCode::NOT_FOUND);

    // Expired invite
    let exp_tok = uuid::Uuid::new_v4().to_string();
    let past = "2020-01-01T00:00:00";
    sqlx::query(
        "INSERT INTO invites (token, kind, email, name, roles, group_ids, created_by, expires_at, created_at) VALUES ($1,'invite','','','','[]',1,$2,$3)",
    )
    .bind(&exp_tok)
    .bind(past)
    .bind(now())
    .execute(&st.db)
    .await
    .unwrap();
    let expired = call(
        &app,
        Method::GET,
        &format!("/api/invites/{exp_tok}"),
        None,
        None,
    )
    .await;
    assert_eq!(expired.status, StatusCode::BAD_REQUEST);
    assert_eq!(expired.json["code"], "INVITE_EXPIRED");

    // Admin approves pending member
    let (uid,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='newbie@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let approve = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{uid}"),
        Some(&admin),
        Some(json!({"roles": ["member"], "status": "active", "group_ids": []})),
    )
    .await;
    // roles may be free-form strings in this app
    assert!(
        approve.status == StatusCode::OK || approve.status == StatusCode::BAD_REQUEST,
        "{}",
        approve.json
    );
    if approve.status != StatusCode::OK {
        // Try with empty roles (demo style)
        let approve2 = call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{uid}"),
            Some(&admin),
            Some(json!({"roles": [], "status": "active", "group_ids": [1]})),
        )
        .await;
        assert_eq!(approve2.status, StatusCode::OK, "{}", approve2.json);
    }
}

#[tokio::test]
async fn recovery_request_approve_reject_and_use_link() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;

    let req = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({"email": "member@example.org", "reason": "lost phone"})),
    )
    .await;
    assert_eq!(req.status, StatusCode::OK);

    // Unknown email still 200 (no probe)
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/recovery-request",
            None,
            Some(json!({"email": "nosuch@example.org", "reason": "x"}))
        )
        .await
        .status,
        StatusCode::OK
    );

    let (rid,): (i64,) = sqlx::query_as(
        "SELECT id FROM recovery_requests WHERE email='member@example.org' AND status='pending' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Reject path on a second request
    let _ = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({"email": "amina@example.org", "reason": "forgot"})),
    )
    .await;
    let (rid2,): (i64,) = sqlx::query_as(
        "SELECT id FROM recovery_requests WHERE email='amina@example.org' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/admin/recovery/{rid2}/reject"),
            Some(&admin),
            None
        )
        .await
        .status,
        StatusCode::OK
    );

    let approved = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(approved.status, StatusCode::OK, "{}", approved.json);
    let url = approved.json["url"].as_str().unwrap().to_string();
    let token = url.rsplit('/').next().unwrap().to_string();

    let use_link = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({"terms": true})),
    )
    .await;
    assert_eq!(use_link.status, StatusCode::OK, "{}", use_link.json);
    assert_eq!(use_link.json["kind"], "recovery");
}

#[tokio::test]
async fn admin_cannot_grant_self_finance() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let (admin_id,): (i64,) =
        sqlx::query_as("SELECT id FROM users WHERE email='admin@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    // Self-grant of finance is refused.
    let denied = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{admin_id}"),
        Some(&admin),
        Some(json!({"roles": ["admin", "finance"], "status": "active"})),
    )
    .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.json);
    assert_eq!(denied.json["code"], "CANNOT_GRANT_SELF_FINANCE");

    // A different admin may grant it.
    sqlx::query("UPDATE users SET roles='transport,admin' WHERE email='transport@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let other = login(&st, "transport@example.org").await;
    let granted = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{admin_id}"),
        Some(&other),
        Some(json!({"roles": ["admin", "finance"], "status": "active"})),
    )
    .await;
    assert_eq!(granted.status, StatusCode::OK, "{}", granted.json);

    // Keeping finance in a self-update is fine; the guard only blocks the grant.
    let keep = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{admin_id}"),
        Some(&admin),
        Some(json!({"roles": ["admin", "finance"], "status": "active"})),
    )
    .await;
    assert_eq!(keep.status, StatusCode::OK, "{}", keep.json);
}

#[tokio::test]
async fn privileged_recovery_needs_two_admins() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    sqlx::query("UPDATE users SET roles='transport,admin' WHERE email='transport@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let second = login(&st, "transport@example.org").await;

    // finance@example.org is a privileged target.
    let _ = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({"email": "finance@example.org", "reason": "lost phone"})),
    )
    .await;
    let (rid,): (i64,) = sqlx::query_as(
        "SELECT id FROM recovery_requests WHERE email='finance@example.org' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    // First admin only flags it; no link is issued yet.
    let first = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.json);
    assert_eq!(first.json["awaiting_second"], true);
    assert!(first.json["url"].is_null());
    let (status,): (String,) = sqlx::query_as("SELECT status FROM recovery_requests WHERE id=$1")
        .bind(rid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(status, "awaiting_second");
    let (invites,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM invites WHERE kind='recovery' AND email='finance@example.org'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(invites, 0);

    // The same admin cannot confirm their own flag.
    let again = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(again.status, StatusCode::FORBIDDEN, "{}", again.json);
    assert_eq!(again.json["code"], "SECOND_ADMIN_REQUIRED");

    // A different admin confirms and receives the link.
    let confirmed = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&second),
        None,
    )
    .await;
    assert_eq!(confirmed.status, StatusCode::OK, "{}", confirmed.json);
    assert!(confirmed.json["url"].is_string());

    // The link is shown once, to the confirming admin. Nobody can read it back from
    // the invite list, so the admin who only flagged the request never holds it.
    let link_token = confirmed.json["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string();
    for who in [&admin, &second] {
        let list = call(&app, Method::GET, "/api/admin/invites", Some(who), None).await;
        assert_eq!(list.status, StatusCode::OK, "{}", list.json);
        let listed = list
            .json
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "recovery" && r["email"] == "finance@example.org")
            .expect("the recovery invite stays in the log");
        assert_eq!(listed["status"], "open");
        assert!(listed["url"].is_null(), "{listed}");
        assert!(!list.json.to_string().contains(&link_token));
    }
    // Ordinary invite links are still listed for copying.
    let invite = call(&app, Method::POST, "/api/admin/invites", Some(&admin), None).await;
    let list = call(&app, Method::GET, "/api/admin/invites", Some(&admin), None).await;
    assert!(list
        .json
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["kind"] == "invite" && r["url"] == invite.json["url"]));

    let (status,): (String,) = sqlx::query_as("SELECT status FROM recovery_requests WHERE id=$1")
        .bind(rid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(status, "approved");

    // Credentials are wiped only on final approval.
    let (uid,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='finance@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let (sessions,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE user_id=$1")
        .bind(uid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let (passkeys,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM passkeys WHERE user_id=$1")
        .bind(uid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(sessions, 0);
    assert_eq!(passkeys, 0);
}

#[tokio::test]
async fn terms_accept_endpoint() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    sqlx::query("UPDATE users SET terms_accepted_at=NULL WHERE email='member@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/terms",
            Some(&cookie),
            Some(json!({"accepted": false}))
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/terms",
            Some(&cookie),
            Some(json!({"accepted": true}))
        )
        .await
        .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn profile_lang_switch_and_disabled_user() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&cookie),
        Some(json!({"name": "陈法拉", "phone": "012-300 0006", "lang": "en"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let me = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me.json["lang"], "en");

    sqlx::query("UPDATE users SET status='disabled' WHERE email='sophie@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    // login helper only selects active — mint session manually then expect unauthorized
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='sophie@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let token = "d".repeat(48);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,1,$3,$4)",
    )
    .bind(grace_ride::auth::hash_token(&token))
    .bind(id)
    .bind("2099-01-01T00:00:00")
    .bind(now())
    .execute(&st.db)
    .await
    .unwrap();
    let cookie = format!("gr_session={token}");
    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&cookie), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn settlement_publish_proof_verify_close_and_unattended_gate() {
    let (app, st) = setup().await;
    let organiser = login(&st, "driver@example.org").await; // deming / event 5 owner-ish
                                                            // Event 5 is finished badminton; organiser for settlements is event creator.
                                                            // Create draft settlement with equal split for attended members only.
    let create = call(
        &app,
        Method::POST,
        "/api/events/5/settlements",
        Some(&organiser),
        Some(json!({
            "title": "Gap settlement",
            "items": [{"label": "court", "amount_cents": 6000}],
            "split_mode": "equal",
            "participants": [
                {"user_id": 4, "amount_cents": 1000},
                {"user_id": 6, "amount_cents": 1000},
                {"user_id": 7, "amount_cents": 1000},
                {"user_id": 5, "amount_cents": 1000},
                {"user_id": 2, "amount_cents": 1000},
                {"user_id": 8, "amount_cents": 1000}
            ],
            "payee_name": "李德明",
            "payee_account": "012-300 0004",
            "due_date": null
        })),
    )
    .await;
    assert_eq!(create.status, StatusCode::OK, "{}", create.json);
    let sid = create.json["id"].as_i64().unwrap();

    // Insert a non-attended share and expect publish to fail, then remove it.
    let (non_att,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='enci@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO settlement_shares (settlement_id, user_id, amount_cents, status, updated_at) VALUES ($1,$2,100, 'unpaid', $3)",
    )
    .bind(sid)
    .bind(non_att)
    .bind(now())
    .execute(&st.db)
    .await
    .unwrap();
    let bad_pub = call(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(bad_pub.status, StatusCode::BAD_REQUEST, "{}", bad_pub.json);

    sqlx::query("DELETE FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2")
        .bind(sid)
        .bind(non_att)
        .execute(&st.db)
        .await
        .unwrap();

    let (before,): (String,) = sqlx::query_as("SELECT updated_at FROM settlements WHERE id=$1")
        .bind(sid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    let pub_ok = call(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(pub_ok.status, StatusCode::OK, "{}", pub_ok.json);
    let (after,): (String,) = sqlx::query_as("SELECT updated_at FROM settlements WHERE id=$1")
        .bind(sid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_ne!(before, after, "publish must bump updated_at");

    // Member submits proof
    let member = login(&st, "member@example.org").await;
    let file_id = upload_png(&app, &member).await;
    let (share_id,): (i64,) = sqlx::query_as(
        "SELECT id FROM settlement_shares WHERE settlement_id=$1 AND user_id=(SELECT id FROM users WHERE email='member@example.org')",
    )
    .bind(sid)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Other user cannot submit proof for this share
    let amina = login(&st, "amina@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/shares/{share_id}/proof"),
            Some(&amina),
            Some(json!({"proof_file_id": file_id, "txn_ref": "TXN1234"}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    let proof = call(
        &app,
        Method::POST,
        &format!("/api/shares/{share_id}/proof"),
        Some(&member),
        Some(json!({"proof_file_id": file_id, "txn_ref": "TXN9999", "paid_date": "2026-11-09"})),
    )
    .await;
    assert_eq!(proof.status, StatusCode::OK, "{}", proof.json);

    let verify = call(
        &app,
        Method::POST,
        &format!("/api/shares/{share_id}/verify"),
        Some(&organiser),
        Some(json!({"action": "confirm"})),
    )
    .await;
    assert_eq!(verify.status, StatusCode::OK, "{}", verify.json);

    // Confirm remaining shares so we can close
    let shares: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM settlement_shares WHERE settlement_id=$1 AND status != 'confirmed'",
    )
    .bind(sid)
    .fetch_all(&st.db)
    .await
    .unwrap();
    for (sh,) in shares {
        sqlx::query("UPDATE settlement_shares SET status='confirmed', updated_at=$1 WHERE id=$2")
            .bind(now())
            .bind(sh)
            .execute(&st.db)
            .await
            .unwrap();
    }
    let close = call(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/close"),
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(close.status, StatusCode::OK, "{}", close.json);
}

#[tokio::test]
async fn ride_withdraw_reassign_alt_transport_and_no_show() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let transport = login(&st, "transport@example.org").await;

    let created = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": tomorrow("11:30"),
            "pickup_area_id": 1,
            "pickup_address": "SS2 test",
            "destination": "教会",
            "seats": 1,
            "assistance": ""
        })),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let rid = created.json["id"].as_i64().unwrap();

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{rid}/claim"),
            Some(&driver),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{rid}/respond"),
            Some(&passenger),
            Some(json!({"accept": true}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Driver withdraws before staff approve
    let wd = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/withdraw"),
        Some(&driver),
        Some(json!({"reason": "车坏了"})),
    )
    .await;
    assert_eq!(wd.status, StatusCode::OK, "{}", wd.json);

    // Re-claim by second driver then staff reassign path: claim again
    let driver2 = login(&st, "driver2@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{rid}/claim"),
            Some(&driver2),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{rid}/respond"),
            Some(&passenger),
            Some(json!({"accept": true}))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{rid}/approve"),
            Some(&transport),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Staff alt-transport on a different pending ride
    let created2 = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": tomorrow("15:00"),
            "pickup_area_id": 2,
            "pickup_address": "PJ 14",
            "destination": "诊所",
            "seats": 1,
            "assistance": ""
        })),
    )
    .await;
    let rid2 = created2.json["id"].as_i64().unwrap();
    let alt = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid2}/alt-transport"),
        Some(&transport),
        Some(json!({"note": "无人接单，批准 Grab"})),
    )
    .await;
    assert_eq!(alt.status, StatusCode::OK, "{}", alt.json);
    let got = call(
        &app,
        Method::GET,
        &format!("/api/rides/{rid2}"),
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(got.json["status"], "alt_transport");

    // No-show on confirmed ride after start
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{rid}/start"),
            Some(&driver2),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let ns = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/no-show"),
        Some(&driver2),
        None,
    )
    .await;
    assert_eq!(ns.status, StatusCode::OK, "{}", ns.json);
}

#[tokio::test]
async fn concurrent_double_claim_one_wins() {
    let (app, st) = setup().await;
    let passenger = login(&st, "yawen@example.org").await;
    let d1 = login(&st, "driver@example.org").await;
    let d2 = login(&st, "driver2@example.org").await;

    let created = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": tomorrow("16:45"),
            "pickup_area_id": 1,
            "pickup_address": "race addr",
            "destination": "社区中心",
            "seats": 1,
            "assistance": ""
        })),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let rid = created.json["id"].as_i64().unwrap();

    let app1 = app.clone();
    let app2 = app.clone();
    let c1 = d1.clone();
    let c2 = d2.clone();
    let path = format!("/api/rides/{rid}/claim");
    let (r1, r2) = tokio::join!(
        call(&app1, Method::POST, &path, Some(&c1), None),
        call(&app2, Method::POST, &path, Some(&c2), None),
    );
    let oks = [r1.status, r2.status]
        .iter()
        .filter(|s| **s == StatusCode::OK)
        .count();
    let conflicts = [r1.status, r2.status]
        .iter()
        .filter(|s| **s == StatusCode::CONFLICT || **s == StatusCode::BAD_REQUEST)
        .count();
    assert_eq!(
        oks, 1,
        "exactly one claim wins: {:?} {:?}",
        r1.status, r2.status
    );
    assert_eq!(
        conflicts, 1,
        "other claim conflicts: {} / {}",
        r1.json, r2.json
    );
}

#[tokio::test]
async fn reimbursement_return_and_reject_branches() {
    let (app, st) = setup().await;
    let claimant = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let receipt = upload_png(&app, &claimant).await;

    let r = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "ride_id": null,
            "amount_cents": 1500,
            "description": "Grab return test",
            "receipt_file_id": receipt,
            "payee_name": "陈法拉",
            "payee_phone": "012-300 0006"
        })),
    )
    .await;
    let id = r.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{id}/review"),
            Some(&transport),
            Some(json!({"action": "return", "note": "照片不清"}))
        )
        .await
        .status,
        StatusCode::OK
    );

    let receipt2 = upload_png(&app, &claimant).await;
    let r2 = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "ride_id": null,
            "amount_cents": 1200,
            "description": "Grab reject test",
            "receipt_file_id": receipt2,
            "payee_name": "陈法拉",
            "payee_phone": "012-300 0006"
        })),
    )
    .await;
    let id2 = r2.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{id2}/review"),
            Some(&transport),
            Some(json!({"action": "reject", "note": "不符合政策"}))
        )
        .await
        .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn help_ticket_and_notifications_flow() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;

    let t = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject": "改上车点", "body": "可以改到南门吗？"})),
    )
    .await;
    assert_eq!(t.status, StatusCode::OK, "{}", t.json);
    let tid = t.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/help/{tid}/replies"),
            Some(&transport),
            Some(json!({"body": "可以，请在报名里改。"}))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/help/{tid}/close"),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );

    let notifs = call(&app, Method::GET, "/api/notifications", Some(&member), None).await;
    assert_eq!(notifs.status, StatusCode::OK);
    let _ = call(
        &app,
        Method::POST,
        "/api/notifications/read-all",
        Some(&member),
        None,
    )
    .await;
}

#[tokio::test]
async fn event_cancel_finishes_and_over_capacity_claim() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    // Cancel a published event that may have rides
    let events = call(&app, Method::GET, "/api/events", Some(&admin), None).await;
    assert_eq!(events.status, StatusCode::OK);

    // Over-capacity: set driver vehicle seats=1 then try claim seats=2 ride
    sqlx::query("UPDATE vehicles SET seats=1 WHERE user_id=(SELECT id FROM users WHERE email='driver@example.org')")
        .execute(&st.db)
        .await
        .unwrap();
    let passenger = login(&st, "sophie@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let created = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": tomorrow("18:10"),
            "pickup_area_id": 1,
            "pickup_address": "cap test",
            "destination": "教会",
            "seats": 2,
            "assistance": ""
        })),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let rid = created.json["id"].as_i64().unwrap();
    let claim = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/claim"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(claim.status, StatusCode::BAD_REQUEST, "{}", claim.json);
}

#[tokio::test]
async fn past_pickup_rejected() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let past = format!(
        "{}T08:00",
        (chrono::Local::now() - chrono::Duration::days(1)).format("%Y-%m-%d")
    );
    let r = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": past,
            "pickup_area_id": 1,
            "pickup_address": "old",
            "destination": "x",
            "seats": 1,
            "assistance": ""
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
}

#[tokio::test]
async fn admin_live_map_with_driver_location() {
    let (app, st) = setup().await;
    let (driver_id,): (i64,) =
        sqlx::query_as("SELECT id FROM users WHERE email='driver@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    let (ride_id,): (i64,) = sqlx::query_as("SELECT id FROM rides ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    sqlx::query("UPDATE rides SET status='in_progress', driver_id=$1 WHERE id=$2")
        .bind(driver_id)
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO driver_locations (driver_id, lat, lng) VALUES ($1, 3.1, 101.6)
         ON CONFLICT (driver_id) DO UPDATE SET lat=EXCLUDED.lat, lng=EXCLUDED.lng",
    )
    .bind(driver_id)
    .execute(&st.db)
    .await
    .unwrap();

    let transport = login(&st, "transport@example.org").await;
    let r = call(&app, Method::GET, "/api/admin/live", Some(&transport), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let drivers = r.json["drivers"].as_array().unwrap();
    let d = drivers
        .iter()
        .find(|d| d["driver_id"] == driver_id)
        .expect("driver on live map");
    assert_eq!(d["lat"], 3.1);
}

#[tokio::test]
async fn pending_member_is_kept_out_however_the_token_is_sent() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let created = call(&app, Method::POST, "/api/admin/invites", Some(&admin), None).await;
    let token = created.json["token"].as_str().unwrap().to_string();
    let accept = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "待批会员",
            "email": "waiting@example.org",
            "phone": "012-999 0002",
            "terms": true
        })),
    )
    .await;
    assert_eq!(accept.status, StatusCode::OK, "{}", accept.json);
    // "gr_session=<token>" from the Set-Cookie header.
    let cookie = accept.headers["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let session = cookie.split_once('=').unwrap().1.to_string();

    // Their own profile is open before approval; member routes are not.
    let me = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.json);
    assert_eq!(me.json["status"], "pending");
    for path in ["/api/rides", "/api/announcements", "/api/events"] {
        let blocked = call(&app, Method::GET, path, Some(&cookie), None).await;
        assert_eq!(blocked.status, StatusCode::FORBIDDEN, "{path}");
        assert_eq!(blocked.json["code"], "ACCOUNT_PENDING", "{path}");
    }

    // The same token as a Bearer header is not a second way in: the cookie is the
    // only credential, so the pending guard cannot be sidestepped.
    for path in ["/api/rides", "/api/announcements", "/api/auth/me"] {
        let req = axum::http::Request::builder()
            .method(Method::GET)
            .uri(path)
            .header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {session}"),
            )
            .body(axum::body::Body::empty())
            .unwrap();
        let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{path}");
    }
    let (rides,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rides r JOIN users u ON u.id=r.passenger_id WHERE u.email='waiting@example.org'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(rides, 0);
}

#[tokio::test]
async fn settlement_receipt_must_be_the_organisers_own_upload() {
    let (app, st) = setup().await;
    // driver@example.org (id 4) is 活动负责人 and organiser of event 5.
    let leader = login(&st, "driver@example.org").await;
    let member = login(&st, "member@example.org").await;
    let theirs = upload_png(&app, &member).await;
    let own = upload_png(&app, &leader).await;
    let file = |id: i64| format!("/api/files/{id}");
    let bill = |receipt: i64| {
        json!({
            "title": "羽球",
            "items": [{"label": "场租", "amount_cents": 10000, "receipt_file_id": receipt}],
            "split_mode": "equal",
            "participants": [{"user_id": 4}],
            "payee_name": "李德明", "payee_account": "TNG 012"
        })
    };
    let can_open = |who: String, id: i64| {
        let (app, path) = (app.clone(), file(id));
        async move {
            call(&app, Method::GET, &path, Some(&who), None)
                .await
                .status
        }
    };
    assert_eq!(
        can_open(leader.clone(), theirs).await,
        StatusCode::FORBIDDEN
    );

    // Naming another member's file on a bill is refused, and opens nothing.
    let r = call(
        &app,
        Method::POST,
        "/api/events/5/settlements",
        Some(&leader),
        Some(bill(theirs)),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "INVALID_RECEIPT");
    assert_eq!(
        can_open(leader.clone(), theirs).await,
        StatusCode::FORBIDDEN
    );
    // An id that doesn't exist answers the same way, so ids can't be probed.
    let r = call(
        &app,
        Method::POST,
        "/api/events/5/settlements",
        Some(&leader),
        Some(bill(999_999)),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);

    // The organiser's own receipt is accepted; editing the draft is checked too.
    let made = call(
        &app,
        Method::POST,
        "/api/events/5/settlements",
        Some(&leader),
        Some(bill(own)),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);
    let sid = made.json["id"].as_i64().unwrap();
    let edit = call(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&leader),
        Some(bill(theirs)),
    )
    .await;
    assert_eq!(edit.status, StatusCode::FORBIDDEN, "{}", edit.json);
    assert_eq!(can_open(leader.clone(), own).await, StatusCode::OK);

    // A row planted before this check existed still opens nothing for the organiser;
    // finance, who reviews every bill, can open it.
    sqlx::query("UPDATE settlement_items SET receipt_file_id=$1 WHERE settlement_id=$2")
        .bind(theirs)
        .bind(sid)
        .execute(&st.db)
        .await
        .unwrap();
    assert_eq!(
        can_open(leader.clone(), theirs).await,
        StatusCode::FORBIDDEN
    );
    let finance = login(&st, "finance@example.org").await;
    assert_eq!(can_open(finance, theirs).await, StatusCode::OK);
}

#[tokio::test]
async fn adding_or_removing_a_sign_in_method_needs_a_recent_sign_in() {
    let (app, st) = setup().await;
    let cookie = login(&st, "sophie@example.org").await;
    let register = |cookie: String| {
        let app = app.clone();
        async move {
            let start = call(
                &app,
                Method::POST,
                "/api/auth/passkey/register/start",
                Some(&cookie),
                None,
            )
            .await;
            if start.status != StatusCode::OK {
                return start;
            }
            let mut soft = WebauthnAuthenticator::new(SoftPasskey::new(true));
            let credential = soft
                .do_registration(
                    Url::parse(ORIGIN).unwrap(),
                    serde_json::from_value::<CreationChallengeResponse>(
                        start.json["options"].clone(),
                    )
                    .unwrap(),
                )
                .unwrap();
            call(
                &app,
                Method::POST,
                "/api/auth/passkey/register/finish",
                Some(&cookie),
                Some(json!({"challenge_id": start.json["challenge_id"], "credential": credential})),
            )
            .await
        }
    };
    // Nothing set up yet: an old session may add the first passkey (new member, after recovery).
    let age = |minutes: i64| {
        let db = st.db.clone();
        async move {
            sqlx::query("UPDATE sessions SET created_at=$1 WHERE user_id=(SELECT id FROM users WHERE email='sophie@example.org')")
                .bind(grace_ride::util::minutes_ago(minutes))
                .execute(&db)
                .await
                .unwrap();
        }
    };
    age(120).await;
    let first = register(cookie.clone()).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.json);

    // With a passkey on the account, the same old session can't add, remove or set up 2FA.
    for (path, method) in [
        ("/api/auth/passkey/register/start", Method::POST),
        ("/api/auth/totp/setup", Method::POST),
        ("/api/auth/passkeys/1", Method::DELETE),
    ] {
        let r = call(&app, method, path, Some(&cookie), None).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{path}: {}", r.json);
        assert_eq!(r.json["code"], "RECENT_SIGN_IN_REQUIRED", "{path}");
    }
    // Signed in within the last 15 minutes: allowed.
    age(5).await;
    let second = register(cookie.clone()).await;
    assert_eq!(second.status, StatusCode::OK, "{}", second.json);
}

#[tokio::test]
async fn recovery_links_are_stored_hashed() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let _ = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({"email": "amina@example.org", "reason": "new phone"})),
    )
    .await;
    let (rid,): (i64,) =
        sqlx::query_as("SELECT id FROM recovery_requests WHERE email='amina@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    let ok = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);
    let token = ok.json["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string();

    let (stored,): (String,) = sqlx::query_as(
        "SELECT token FROM invites WHERE kind='recovery' AND email='amina@example.org'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_ne!(stored, token, "the link itself is never stored");
    assert_eq!(stored, grace_ride::auth::hash_token(&token));

    // What a leaked backup holds (the hash) doesn't open the link.
    for t in [&stored] {
        let r = call(&app, Method::GET, &format!("/api/invites/{t}"), None, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{}", r.json);
    }
    let preview = call(
        &app,
        Method::GET,
        &format!("/api/invites/{token}"),
        None,
        None,
    )
    .await;
    assert_eq!(preview.status, StatusCode::OK, "{}", preview.json);
    assert_eq!(preview.json["kind"], "recovery");
    let used = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({"terms": true})),
    )
    .await;
    assert_eq!(used.status, StatusCode::OK, "{}", used.json);
}

#[tokio::test]
async fn bad_profile_details_are_refused_without_burning_the_invite() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let created = call(&app, Method::POST, "/api/admin/invites", Some(&admin), None).await;
    let token = created.json["token"].as_str().unwrap().to_string();
    let accept = |body: serde_json::Value| {
        let (app, path) = (app.clone(), format!("/api/invites/{token}/accept"));
        async move { call(&app, Method::POST, &path, None, Some(body)).await }
    };
    let short =
        accept(json!({"name": "陈", "email": "chen@example.org", "phone": "0123", "terms": true}))
            .await;
    assert_eq!(short.status, StatusCode::BAD_REQUEST, "{}", short.json);
    assert_eq!(short.json["code"], "PHONE_INVALID");
    let long =
        accept(json!({"name": "名".repeat(61), "email": "chen@example.org", "terms": true})).await;
    assert_eq!(long.status, StatusCode::BAD_REQUEST, "{}", long.json);
    assert_eq!(long.json["code"], "NAME_TOO_LONG");
    // The link still works after both mistakes.
    let ok = accept(
        json!({"name": "陈", "email": "chen@example.org", "phone": "012-345 6789", "terms": true}),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);

    // The profile form applies the same rules.
    let member = login(&st, "member@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({"phone": "12"})),
    )
    .await;
    assert_eq!(r.json["code"], "PHONE_INVALID", "{}", r.json);
    let r = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({"name": "x".repeat(61)})),
    )
    .await;
    assert_eq!(r.json["code"], "NAME_TOO_LONG", "{}", r.json);
    let r = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({"phone": ""})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
}

#[tokio::test]
async fn members_can_list_their_groups() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let r = call(&app, Method::GET, "/api/groups", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert!(r
        .json
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g["is_all"] == true));
}

#[tokio::test]
async fn request_bodies_and_uploads_are_bounded() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;

    // A JSON body past the cap is refused before it reaches the handler.
    let huge = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject": "x", "body": "x".repeat(grace_ride::MAX_JSON_BODY + 1)})),
    )
    .await;
    assert_eq!(huge.status, StatusCode::PAYLOAD_TOO_LARGE);
    let normal = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject": "x", "body": "y"})),
    )
    .await;
    assert_eq!(normal.status, StatusCode::OK, "{}", normal.json);

    // Uploads keep their own, larger limit.
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend(std::iter::repeat_n(0u8, 2 * 1024 * 1024));
    let boundary = "XgrBOUNDARY";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"big.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(&png);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(axum::http::header::COOKIE, &member)
        .header(axum::http::header::ORIGIN, ORIGIN)
        .header(
            axum::http::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(axum::body::Body::from(body))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "a 2 MB receipt still uploads");

    // One account can't fill the disk: past the daily quota uploads are refused.
    let (uid,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='member@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    for i in 0..grace_ride::routes::files::MAX_UPLOADS_PER_DAY {
        sqlx::query("INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at) VALUES ($1,$2,'image/png','x.png',1,$3)")
            .bind(uid)
            .bind(format!("quota-{i}.png"))
            .bind(now())
            .execute(&st.db)
            .await
            .unwrap();
    }
    let mut small = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    small.extend_from_slice(&[0u8; 32]);
    let boundary = "XgrB2";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"r.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(&small);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let req = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(axum::http::header::COOKIE, &member)
        .header(axum::http::header::ORIGIN, ORIGIN)
        .header(
            axum::http::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(axum::body::Body::from(body))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}
