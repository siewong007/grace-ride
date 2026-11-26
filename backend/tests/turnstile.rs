//! Cloudflare Turnstile protection on public web authentication, onboarding, and visitor flows.

mod common;

use std::sync::{Arc, Mutex};

use axum::{
    http::{Method, StatusCode},
    routing::post,
    Json, Router,
};
use common::*;
use grace_ride::{
    init, router,
    state::{AppState, Config},
    turnstile::TurnstileConfig,
};
use serde_json::{json, Value};

#[derive(Clone)]
struct MockOutcome {
    status: StatusCode,
    body: Value,
}

async fn start_mock_siteverify() -> (
    String,
    Arc<Mutex<MockOutcome>>,
    Arc<Mutex<Vec<Value>>>,
    tokio::task::JoinHandle<()>,
) {
    let outcome = Arc::new(Mutex::new(MockOutcome {
        status: StatusCode::OK,
        body: json!({ "success": true, "hostname": "localhost", "action": "passkey_login" }),
    }));
    let calls = Arc::new(Mutex::new(Vec::new()));

    let o = outcome.clone();
    let c = calls.clone();
    let app = Router::new().route(
        "/turnstile/v0/siteverify",
        post(move |Json(payload): Json<Value>| {
            let o = o.clone();
            let c = c.clone();
            async move {
                c.lock().unwrap().push(payload);
                let out = o.lock().unwrap().clone();
                (out.status, Json(out.body))
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (
        format!("http://{addr}/turnstile/v0/siteverify"),
        outcome,
        calls,
        handle,
    )
}

async fn setup_with_turnstile(turnstile: TurnstileConfig) -> (Router, AppState) {
    let dir = std::env::temp_dir().join(format!("gr-ts-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        dev_mode: true,
        database_url: fresh_db().await,
        upload_dir: dir.join("uploads"),
        static_dir: None,
        public_url: ORIGIN.into(),
        bind: "127.0.0.1:0".into(),
        max_live_drivers: 10,
        trust_proxy: false,
        chat_retention_days: 180,
        audit_retention_days: 730,
        session_timeout_mins: 15,
        json_logs: false,
        routing: Default::default(),
        db: grace_ride::state::DbConfig {
            max_connections: 5,
            acquire_timeout_secs: 60,
            ..Default::default()
        },
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
        turnstile,
        android_cert_sha256_fingerprints: Vec::new(),
        ios_app_ids: Vec::new(),
        require_biometric_attestation: false,
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    register_test_db(&state);
    (router(state.clone()), state)
}

#[tokio::test]
async fn turnstile_config_endpoint_and_csp_follow_env_toggle() {
    let (app_off, _) = setup().await;
    let res_off = call(&app_off, Method::GET, "/api/auth/turnstile", None, None).await;
    assert_eq!(res_off.status, StatusCode::OK);
    assert_eq!(res_off.json["enabled"], false);
    assert!(res_off.json["site_key"].is_null());
    let csp_off = res_off.headers["content-security-policy"].to_str().unwrap();
    assert!(!csp_off.contains("challenges.cloudflare.com"));

    let (verify_url, _outcome, _calls, h) = start_mock_siteverify().await;
    let (app_on, _) = setup_with_turnstile(TurnstileConfig {
        site_key: Some("0x4AAAAAArealSiteKey".into()),
        secret_key: Some("0x4AAAAAArealSecretKey".into()),
        verify_url,
    })
    .await;

    let res_on = call(&app_on, Method::GET, "/api/auth/turnstile", None, None).await;
    assert_eq!(res_on.status, StatusCode::OK);
    assert_eq!(res_on.json["enabled"], true);
    assert_eq!(res_on.json["site_key"], "0x4AAAAAArealSiteKey");
    assert!(res_on.json.get("secret_key").is_none());
    assert!(res_on.json.get("verify_url").is_none());

    let csp_on = res_on.headers["content-security-policy"].to_str().unwrap();
    assert!(csp_on.contains("script-src 'self' https://challenges.cloudflare.com"));
    assert!(csp_on.contains("frame-src 'self' https://challenges.cloudflare.com"));
    assert!(csp_on.contains("connect-src 'self' https://challenges.cloudflare.com"));
    h.abort();
}

#[test]
fn production_config_validates_turnstile_rules() {
    let base = || Config {
        dev_mode: false,
        database_url: "postgres://localhost/grace_ride_test".into(),
        upload_dir: "/tmp".into(),
        static_dir: None,
        public_url: "https://ride.example.org".into(),
        bind: "0.0.0.0:8080".into(),
        max_live_drivers: 10,
        trust_proxy: true,
        chat_retention_days: 180,
        audit_retention_days: 730,
        session_timeout_mins: 15,
        json_logs: true,
        routing: Default::default(),
        db: Default::default(),
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
        turnstile: Default::default(),
        android_cert_sha256_fingerprints: Vec::new(),
        ios_app_ids: Vec::new(),
        require_biometric_attestation: false,
    };

    // Half-configured rejected
    let mut cfg = base();
    cfg.turnstile = TurnstileConfig {
        site_key: Some("0x4AAAAAArealSiteKey".into()),
        secret_key: None,
        verify_url: grace_ride::turnstile::DEFAULT_VERIFY_URL.into(),
    };
    assert!(cfg.validate().is_err());

    // Dummy keys rejected in production
    let mut cfg = base();
    cfg.turnstile = TurnstileConfig {
        site_key: Some("1x00000000000000000000AA".into()),
        secret_key: Some("1x0000000000000000000000000000000AA".into()),
        verify_url: grace_ride::turnstile::DEFAULT_VERIFY_URL.into(),
    };
    assert!(cfg.validate().is_err());

    // Plain http verify URL rejected outside localhost
    let mut cfg = base();
    cfg.turnstile = TurnstileConfig {
        site_key: Some("0x4AAAAAArealSiteKey".into()),
        secret_key: Some("0x4AAAAAArealSecretKey".into()),
        verify_url: "http://127.0.0.1:9999/siteverify".into(),
    };
    assert!(cfg.validate().is_err());

    // Real HTTPS config passes
    let mut cfg = base();
    cfg.turnstile = TurnstileConfig {
        site_key: Some("0x4AAAAAArealSiteKey".into()),
        secret_key: Some("0x4AAAAAArealSecretKey".into()),
        verify_url: grace_ride::turnstile::DEFAULT_VERIFY_URL.into(),
    };
    assert!(cfg.validate().is_ok());
}

#[tokio::test]
async fn turnstile_protects_all_four_public_web_flows() {
    let (verify_url, outcome, calls, h) = start_mock_siteverify().await;
    let (app, st) = setup_with_turnstile(TurnstileConfig {
        site_key: Some("0x4AAAAAArealSiteKey".into()),
        secret_key: Some("0x4AAAAAArealSecretKey".into()),
        verify_url,
    })
    .await;
    let admin = login(&st, "admin@example.org").await;

    // 1. Passkey login start
    let r = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({ "email": "member@example.org" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "TURNSTILE_REQUIRED");

    // Wrong action fails closed
    *outcome.lock().unwrap() = MockOutcome {
        status: StatusCode::OK,
        body: json!({ "success": true, "hostname": "localhost", "action": "recovery_request" }),
    };
    let r = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({ "email": "member@example.org", "turnstile_token": "tok-wrong-action" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "TURNSTILE_FAILED");

    // Rejected by Cloudflare
    *outcome.lock().unwrap() = MockOutcome {
        status: StatusCode::OK,
        body: json!({ "success": false, "error-codes": ["invalid-input-response"] }),
    };
    let r = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({ "email": "member@example.org", "turnstile_token": "tok-bad" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "TURNSTILE_FAILED");

    // Valid passkey_login token succeeds
    *outcome.lock().unwrap() = MockOutcome {
        status: StatusCode::OK,
        body: json!({ "success": true, "hostname": "localhost", "action": "passkey_login" }),
    };
    let r = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({ "email": "member@example.org", "turnstile_token": "tok-passkey" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert!(r.json["challenge_id"].is_string());
    assert!(r.json["options"]["publicKey"].is_object());

    // 2. Account recovery request
    let r = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({ "email": "member@example.org", "reason": "lost phone" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["code"], "TURNSTILE_REQUIRED");
    let rec_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recovery_requests")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(rec_count, 0);

    *outcome.lock().unwrap() = MockOutcome {
        status: StatusCode::OK,
        body: json!({ "success": true, "hostname": "localhost", "action": "recovery_request" }),
    };
    let r = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({
            "email": "member@example.org",
            "reason": "lost phone",
            "turnstile_token": "tok-recovery"
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let rec_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recovery_requests")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(rec_count, 1);

    // 3. Invite acceptance (failed verification must not burn single-use invite)
    let inv = call(&app, Method::POST, "/api/admin/invites", Some(&admin), None).await;
    assert_eq!(inv.status, StatusCode::OK, "{}", inv.json);
    let inv_tok = inv.json["token"].as_str().unwrap();

    let r = call(
        &app,
        Method::POST,
        &format!("/api/invites/{inv_tok}/accept"),
        None,
        Some(json!({
            "name": "Turnstile Test User",
            "email": "turnstile-user@example.org",
            "phone": "012-300 4455",
            "terms": true
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["code"], "TURNSTILE_REQUIRED");
    let used_at: Option<String> = sqlx::query_scalar("SELECT used_at FROM invites WHERE token=$1")
        .bind(inv_tok)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert!(used_at.is_none());

    *outcome.lock().unwrap() = MockOutcome {
        status: StatusCode::OK,
        body: json!({ "success": true, "hostname": "localhost", "action": "invite_accept" }),
    };
    let r = call(
        &app,
        Method::POST,
        &format!("/api/invites/{inv_tok}/accept"),
        None,
        Some(json!({
            "name": "Turnstile Test User",
            "email": "turnstile-user@example.org",
            "phone": "012-300 4455",
            "terms": true,
            "turnstile_token": "tok-invite"
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let used_at: Option<String> = sqlx::query_scalar("SELECT used_at FROM invites WHERE token=$1")
        .bind(inv_tok)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert!(used_at.is_some());

    // 4. Visitor pass claim (empty body fails cleanly without consuming pass; valid token claims)
    // A single group membership so the signup needs no "joining as" choice.
    let all_group: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let member_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    sqlx::query("DELETE FROM group_members WHERE user_id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1,$2)")
        .bind(all_group)
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let starts_at = (grace_ride::util::local_now() + chrono::Duration::hours(5))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let pickup_at = (grace_ride::util::local_now() + chrono::Duration::hours(3))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Visitor Turnstile Event",
            "kind": "other",
            "starts_at": starts_at,
            "location": "Church Hall",
            "group_ids": [all_group]
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev_id = ev.json["id"].as_i64().unwrap();

    let member = login(&st, "member@example.org").await;
    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/signup"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let pass = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/visitors"),
        Some(&member),
        Some(json!({
            "name": "Guest Friend",
            "phone": "012-345 6789",
            "pickup_at": pickup_at,
            "pickup_area_id": 1,
            "pickup_address": "1 Jalan Turnstile"
        })),
    )
    .await;
    assert_eq!(pass.status, StatusCode::OK, "{}", pass.json);
    let pass_id = pass.json["id"].as_i64().unwrap();
    let visit_tok = pass.json["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap();

    // Empty body POST rejected before consuming pass
    let r = call(
        &app,
        Method::POST,
        &format!("/api/visit/{visit_tok}/claim"),
        None,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "TURNSTILE_REQUIRED");
    let claimed_at: Option<String> =
        sqlx::query_scalar("SELECT claimed_at FROM visitor_passes WHERE id=$1")
            .bind(pass_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(claimed_at.is_none());

    *outcome.lock().unwrap() = MockOutcome {
        status: StatusCode::OK,
        body: json!({ "success": true, "hostname": "localhost", "action": "visitor_claim" }),
    };
    let r = call(
        &app,
        Method::POST,
        &format!("/api/visit/{visit_tok}/claim"),
        None,
        Some(json!({ "turnstile_token": "tok-visit" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let has_visit_cookie = r
        .headers
        .get_all(axum::http::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|v| v.starts_with("gr_visit="));
    assert!(has_visit_cookie);

    // Check siteverify payloads recorded secret + idempotency_key + token
    let recorded = calls.lock().unwrap();
    assert!(recorded.len() >= 5);
    for req_body in recorded.iter() {
        assert_eq!(req_body["secret"], "0x4AAAAAArealSecretKey");
        assert!(req_body["idempotency_key"]
            .as_str()
            .is_some_and(|s| !s.is_empty()));
        assert!(req_body["response"].as_str().is_some_and(|s| !s.is_empty()));
    }

    h.abort();
}
