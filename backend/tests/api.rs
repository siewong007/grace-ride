//! End-to-end API tests against a fresh PostgreSQL schema with demo data.
//! Each test builds its own app (own schema, own rate limiter).

mod common;
use common::schema;

use crate::common::upload_png;
use axum::{
    body::Body,
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use grace_ride::{
    auth::{hash_token, SESSION_COOKIE},
    init, jobs, router,
    state::AppState,
    state::Config,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const PW: &str = "GraceRide2026!";
const ORIGIN: &str = "http://localhost:5173";

async fn fresh_db() -> String {
    schema::fresh_schema("gr_").await
}

async fn setup() -> (Router, AppState) {
    let dir = std::env::temp_dir().join(format!("gr-test-{}", uuid::Uuid::new_v4()));
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
        db: Default::default(),
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
        turnstile: Default::default(),
        google_oauth: Default::default(),
        android_cert_sha256_fingerprints: Vec::new(),
        ios_app_ids: Vec::new(),
        require_biometric_attestation: false,
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    common::register_test_db(&state);
    (router(state.clone()), state)
}

struct Res {
    status: StatusCode,
    headers: HeaderMap,
    json: Value,
}

fn cookie_from(res: &Res) -> Option<String> {
    res.headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|raw| {
            let pair = raw.split(';').next()?.trim();
            if (pair.starts_with("gr_session=") || pair.starts_with("__Host-gr_session="))
                && !pair.ends_with('=')
            {
                Some(pair.to_string())
            } else {
                None
            }
        })
}

async fn call(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Res {
    call_with(app, method, path, cookie, body, Some(ORIGIN)).await
}

async fn call_with(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
    origin: Option<&str>,
) -> Res {
    let body = common::with_event_shim(&method, path, body).await;
    let mut b = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    if let Some(o) = origin {
        b = b.header(header::ORIGIN, o);
    }
    let req = match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Res {
        status,
        headers,
        json,
    }
}

/// Test-only auth: mint a hashed session row and return the Cookie header value.
/// Password login is disabled in the app (passkeys); tests must not call /auth/login.
async fn login(st: &AppState, email: &str) -> String {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email=$1 AND status='active'")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|e| panic!("demo user {email}: {e}"));
    let token =
        uuid::Uuid::new_v4().simple().to_string() + &uuid::Uuid::new_v4().simple().to_string();
    let token = token[..48].to_string();
    let expires = (grace_ride::util::local_now() + chrono::Duration::days(7))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,1,$3,$4)",
    )
    .bind(hash_token(&token))
    .bind(id)
    .bind(&expires)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    format!("{SESSION_COOKIE}={token}")
}

fn tomorrow(hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (grace_ride::util::local_now() + chrono::Duration::days(1)).format("%Y-%m-%d")
    )
}

fn totp_code(secret_b32: &str, email: &str) -> String {
    use totp_rs::{Builder, Secret};
    let secret = Secret::try_from_base32(secret_b32).expect("secret bytes");
    let totp = Builder::new()
        .with_secret(secret)
        .with_issuer(Some("Grace Ride"))
        .with_account_name(email)
        .build()
        .expect("totp");
    totp.generate_current().to_string()
}

#[tokio::test]
async fn health_headers_and_unknown_api() {
    let (app, _) = setup().await;
    let r = call(&app, Method::GET, "/api/health", None, None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["db"], true);
    for h in [
        "content-security-policy",
        "x-frame-options",
        "x-content-type-options",
        "referrer-policy",
        "permissions-policy",
    ] {
        assert!(r.headers.contains_key(h), "missing {h}");
    }
    assert_eq!(r.headers["cache-control"], "no-store");
    let r = call(&app, Method::GET, "/api/does-not-exist", None, None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert!(r.json["error"].is_string());
}

#[tokio::test]
async fn stream_needs_a_session_and_pushes_events() {
    let (app, st) = setup().await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/stream")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let cookie = login(&st, "member@example.org").await;
    let (uid,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='member@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/stream")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "text/event-stream");

    let mut body = res.into_body();
    st.events
        .send(grace_ride::stream::Push {
            user_id: uid,
            kind: "notify",
        })
        .unwrap();
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), body.frame())
        .await
        .expect("event should arrive")
        .expect("body frame")
        .expect("data frame");
    let bytes = frame.into_data().expect("data frame");
    assert!(String::from_utf8_lossy(&bytes).contains("data: notify"));
}

#[tokio::test]
async fn session_tokens_are_stored_hashed() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let token = cookie.split('=').nth(1).unwrap();
    let (stored,): (String,) = sqlx::query_as("SELECT token FROM sessions LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_ne!(stored, token);
    assert_eq!(stored, hash_token(token));
    let me = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me.status, StatusCode::OK);
    // Logout revokes server-side.
    assert_eq!(
        call(&app, Method::POST, "/api/auth/logout", Some(&cookie), None)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&cookie), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn logout_all_revokes_every_session() {
    let (app, st) = setup().await;
    let cookie1 = login(&st, "member@example.org").await;
    let cookie2 = login(&st, "member@example.org").await;

    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&cookie1), None)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&cookie2), None)
            .await
            .status,
        StatusCode::OK
    );

    let logout_res = call(
        &app,
        Method::POST,
        "/api/auth/logout-all",
        Some(&cookie1),
        None,
    )
    .await;
    assert_eq!(logout_res.status, StatusCode::OK);

    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&cookie1), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&cookie2), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn totp_backup_codes_and_auto_lock_flow() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;

    // 1. Initial /auth/me
    let me = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json["auto_lock_mins"], 0);
    assert_eq!(me.json["session_timeout_mins"], 15);
    assert_eq!(me.json["backup_codes_count"], 0);

    // 2. Set auto_lock_mins
    let put_me = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&cookie),
        Some(json!({ "auto_lock_mins": 30 })),
    )
    .await;
    assert_eq!(put_me.status, StatusCode::OK);

    let me_after = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me_after.json["auto_lock_mins"], 30);

    // 3. Setup TOTP
    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(setup_res.status, StatusCode::OK);
    let secret = setup_res.json["secret"].as_str().unwrap();
    let code = totp_code(secret, "member@example.org");

    // 4. Enable TOTP -> receives 8 backup codes
    let enable_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&cookie),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(enable_res.status, StatusCode::OK);
    let cookie = cookie_from(&enable_res).expect("rotated cookie");
    let codes = enable_res.json["backup_codes"]
        .as_array()
        .expect("backup_codes array");
    assert_eq!(codes.len(), 8);

    let me_totp = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me_totp.json["totp_enabled"], true);
    assert_eq!(me_totp.json["backup_codes_count"], 8);

    // 5. Invalid verify code fails
    let bad_verify = call(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&cookie),
        Some(json!({ "code": "INVALID" })),
    )
    .await;
    assert_eq!(bad_verify.status, StatusCode::BAD_REQUEST);

    // 6. Use one backup recovery code to verify/elevate session
    let first_code = codes[0].as_str().unwrap();
    let verify_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&cookie),
        Some(json!({ "code": first_code })),
    )
    .await;
    assert_eq!(verify_res.status, StatusCode::OK);
    assert_eq!(verify_res.json["method"], "backup_code");
    assert_eq!(verify_res.json["remaining_backup_codes"], 7);
    let cookie = cookie_from(&verify_res).expect("rotated cookie");

    let me_after_use = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me_after_use.json["backup_codes_count"], 7);

    // 7. Reusing the same single-use backup code fails
    let reuse_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&cookie),
        Some(json!({ "code": first_code })),
    )
    .await;
    assert_eq!(reuse_res.status, StatusCode::BAD_REQUEST);

    // 8. Regenerating backup codes
    let regen_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/backup-codes/regenerate",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(regen_res.status, StatusCode::OK);
    let new_codes = regen_res.json["backup_codes"].as_array().unwrap();
    assert_eq!(new_codes.len(), 8);

    let me_after_regen = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(me_after_regen.json["backup_codes_count"], 8);
}

/// Codes printed before backup-code hashes were salted with the member id were stored
/// as a bare SHA-256; members who still hold them must be able to use them.
#[tokio::test]
async fn backup_codes_issued_before_salting_still_work() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&cookie),
        None,
    )
    .await;
    let secret = setup_res.json["secret"].as_str().unwrap();
    let code = totp_code(secret, "member@example.org");
    let enable_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&cookie),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(enable_res.status, StatusCode::OK, "{}", enable_res.json);
    let cookie = cookie_from(&enable_res).expect("rotated cookie");

    // Replace the stored codes with one in the old, unsalted format.
    let legacy = json!([hash_token("ABCD2345")]).to_string();
    sqlx::query("UPDATE users SET totp_backup_codes=$1 WHERE email='member@example.org'")
        .bind(&legacy)
        .execute(&st.db)
        .await
        .unwrap();

    let r = call(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&cookie),
        Some(json!({ "code": "ABCD-2345" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["method"], "backup_code");
    assert_eq!(r.json["remaining_backup_codes"], 0);
    let rotated = cookie_from(&r).expect("rotated cookie");
    let stale_old = call(&app, Method::GET, "/api/auth/me", Some(&cookie), None).await;
    assert_eq!(stale_old.status, StatusCode::UNAUTHORIZED);

    // Still single-use.
    let again = call(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&rotated),
        Some(json!({ "code": "ABCD-2345" })),
    )
    .await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn totp_backup_code_concurrent_use_one_wins() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;

    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(setup_res.status, StatusCode::OK);
    let secret = setup_res.json["secret"].as_str().unwrap();
    let code = totp_code(secret, "member@example.org");

    let enable_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&cookie),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(enable_res.status, StatusCode::OK);
    let cookie1 = cookie_from(&enable_res).expect("rotated cookie");
    let cookie2 = login(&st, "member@example.org").await;
    let codes = enable_res.json["backup_codes"]
        .as_array()
        .expect("backup_codes array");
    assert_eq!(codes.len(), 8);

    let first_code = codes[0].as_str().unwrap();

    // Two concurrent sessions from the same account race to consume the same single-use
    // code. PostgreSQL serialises the UPDATEs on the same users row, so only one can
    // consume it. The loser re-reads after the row lock and finds the code gone.
    let app1 = app.clone();
    let app2 = app.clone();
    let path = "/api/auth/totp/verify";
    let (verify1, verify2) = tokio::join!(
        call(
            &app1,
            Method::POST,
            path,
            Some(&cookie1),
            Some(json!({ "code": first_code }))
        ),
        call(
            &app2,
            Method::POST,
            path,
            Some(&cookie2),
            Some(json!({ "code": first_code }))
        ),
    );

    let ok = [verify1.status, verify2.status]
        .iter()
        .filter(|s| **s == StatusCode::OK)
        .count();
    let bad = [verify1.status, verify2.status]
        .iter()
        .filter(|s| **s == StatusCode::BAD_REQUEST)
        .count();
    assert_eq!(
        ok, 1,
        "exactly one session can authenticate with a single-use code: {:?}, {:?}",
        verify1.status, verify2.status
    );
    assert_eq!(
        bad, 1,
        "the losing request must be rejected, not both accepted: {:?}, {:?}",
        verify1.json, verify2.json
    );

    let live_cookie = if verify1.status == StatusCode::OK {
        cookie_from(&verify1).expect("rotated cookie1")
    } else {
        cookie_from(&verify2).expect("rotated cookie2")
    };
    // The account still has 7 codes left and is still signed in.
    let me = call(&app, Method::GET, "/api/auth/me", Some(&live_cookie), None).await;
    assert_eq!(me.json["backup_codes_count"], 7);
}

#[tokio::test]
async fn password_login_lockout_and_totp() {
    let (app, st) = setup().await;
    // Unknown email fails with INVALID_CREDENTIALS.
    let unknown = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "nobody@example.org", "password": PW})),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST, "{}", unknown.json);
    assert_eq!(unknown.json["code"], "INVALID_CREDENTIALS");

    // Valid member password login succeeds and sets a session cookie (mfa_ok=false when no 2FA).
    let ok = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "member@example.org", "password": PW})),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);
    let member_cookie = cookie_from(&ok).expect("session cookie on password login");
    let me = call(
        &app,
        Method::GET,
        "/api/auth/me",
        Some(&member_cookie),
        None,
    )
    .await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json["email"], "member@example.org");

    // Disabled user with right password gets AUTH_DISABLED; wrong password still gets INVALID_CREDENTIALS.
    sqlx::query("UPDATE users SET status='disabled' WHERE email='yawen@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let disabled_bad_pw = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "yawen@example.org", "password": "wrong-password"})),
    )
    .await;
    assert_eq!(disabled_bad_pw.status, StatusCode::BAD_REQUEST);
    assert_eq!(disabled_bad_pw.json["code"], "INVALID_CREDENTIALS");
    let disabled_ok_pw = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "yawen@example.org", "password": PW})),
    )
    .await;
    assert_eq!(disabled_ok_pw.status, StatusCode::FORBIDDEN);
    assert_eq!(disabled_ok_pw.json["code"], "AUTH_DISABLED");

    // 5 failed password attempts lock out that email for 15 minutes.
    for i in 0..5 {
        let fail = call(
            &app,
            Method::POST,
            "/api/auth/login",
            None,
            Some(json!({"email": "member@example.org", "password": format!("wrong-{i}")})),
        )
        .await;
        assert_eq!(fail.status, StatusCode::BAD_REQUEST, "{i}: {}", fail.json);
        assert_eq!(fail.json["code"], "INVALID_CREDENTIALS");
    }
    let locked = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "member@example.org", "password": PW})),
    )
    .await;
    assert_eq!(
        locked.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        locked.json
    );
    assert_eq!(locked.json["code"], "TOO_MANY_FAILED_LOGINS");

    // TOTP-enabled user requires a valid 2FA code to complete password sign-in
    // (fresh app state avoids hitting the per-IP 10/min login rate limit above).
    let (app, st) = setup().await;
    let admin_cookie = login(&st, "admin@example.org").await;
    let setup_totp = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&admin_cookie),
        None,
    )
    .await;
    let secret = setup_totp.json["secret"].as_str().unwrap().to_string();
    let code = totp_code(&secret, "admin@example.org");
    let enable = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&admin_cookie),
        Some(json!({"code": code})),
    )
    .await;
    assert_eq!(enable.status, StatusCode::OK, "{}", enable.json);

    let need_code = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "admin@example.org", "password": PW})),
    )
    .await;
    assert_eq!(
        need_code.status,
        StatusCode::BAD_REQUEST,
        "{}",
        need_code.json
    );
    assert_eq!(need_code.json["code"], "TOTP_REQUIRED");

    let bad_code = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "admin@example.org", "password": PW, "code": "000000"})),
    )
    .await;
    assert_eq!(
        bad_code.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_code.json
    );
    assert_eq!(bad_code.json["code"], "INVALID_CODE");

    // Replaying an already-consumed TOTP step returns CODE_ALREADY_USED deterministically.
    sqlx::query(
        "UPDATE users SET totp_last_step = (EXTRACT(EPOCH FROM NOW())::BIGINT / 30) + 10 WHERE email = 'admin@example.org'",
    )
    .execute(&st.db)
    .await
    .unwrap();
    let cur_code = totp_code(&secret, "admin@example.org");
    let replay_code = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "admin@example.org", "password": PW, "code": cur_code})),
    )
    .await;
    assert_eq!(
        replay_code.status,
        StatusCode::BAD_REQUEST,
        "{}",
        replay_code.json
    );
    assert_eq!(replay_code.json["code"], "CODE_ALREADY_USED");

    // Backup code succeeds and yields mfa_ok=true.
    let backup_code = enable.json["backup_codes"][0].as_str().unwrap();
    let totp_ok = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "admin@example.org", "password": PW, "code": backup_code})),
    )
    .await;
    assert_eq!(totp_ok.status, StatusCode::OK, "{}", totp_ok.json);
    let admin_pw_cookie = cookie_from(&totp_ok).expect("admin cookie after 2fa");
    let admin_me = call(
        &app,
        Method::GET,
        "/api/auth/me",
        Some(&admin_pw_cookie),
        None,
    )
    .await;
    assert_eq!(admin_me.json["mfa_ok"], true);
}

#[tokio::test]
async fn per_ip_rate_limit_on_login() {
    let (app, _) = setup().await;
    let mut limited = false;
    for i in 0..12 {
        let r = call(
            &app,
            Method::POST,
            "/api/auth/login",
            None,
            Some(json!({"email": format!("x{i}@example.org"), "password": "x"})),
        )
        .await;
        if r.status == StatusCode::TOO_MANY_REQUESTS {
            assert!(r.headers.contains_key(header::RETRY_AFTER));
            limited = true;
            break;
        }
    }
    assert!(limited, "expected 429 within 12 attempts");
}

#[tokio::test]
async fn cross_origin_writes_are_blocked() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let r = call_with(
        &app,
        Method::POST,
        "/api/help",
        Some(&cookie),
        Some(json!({"subject": "x", "body": "y"})),
        Some("https://evil.example"),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&cookie),
        Some(json!({"subject": "x", "body": "y"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
}

#[tokio::test]
async fn role_based_access() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/admin/dashboard",
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/admin/dashboard",
            Some(&transport),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    // Finance reads member reports, so it may pick names from the directory — but the
    // answer carries no contact details (that needs 成员 · view).
    let users = call(&app, Method::GET, "/api/admin/users", Some(&finance), None).await;
    assert_eq!(users.status, StatusCode::OK, "{}", users.json);
    for row in users.json.as_array().unwrap() {
        assert!(row.get("email").is_none(), "{}", users.json);
        assert!(row.get("phone").is_none(), "{}", users.json);
    }
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/reimbursements?scope=review",
            Some(&finance),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/reimbursements?scope=payout",
            Some(&finance),
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
            "/api/admin/invites",
            Some(&transport),
            Some(json!({"email": "a@b.c", "roles": [], "group_ids": []}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn ride_matching_flow_protects_address() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let driver2 = login(&st, "driver2@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let other = login(&st, "sophie@example.org").await;

    let r = call(&app, Method::POST, "/api/rides", Some(&passenger), Some(json!({
        "pickup_at": tomorrow("15:00"), "pickup_area_id": 2, "pickup_address": "SECRET 12", "destination": "教会", "seats": 1
    }))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let id = r.json["id"].as_i64().unwrap();

    let list = call(
        &app,
        Method::GET,
        "/api/driver/requests?area_id=2",
        Some(&driver),
        None,
    )
    .await;
    let mine = list
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == id)
        .unwrap()
        .clone();
    assert!(
        mine["pickup_address"].is_null(),
        "address must be hidden before approval"
    );
    assert!(mine["passenger"]["phone"].is_null());

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/claim"),
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
            &format!("/api/rides/{id}/claim"),
            Some(&driver2),
            None
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/rides/{id}"),
            Some(&other),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/respond"),
            Some(&passenger),
            Some(json!({"accept": true}))
        )
        .await
        .status,
        StatusCode::OK
    );
    let before = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&driver),
        None,
    )
    .await;
    assert!(before.json["pickup_address"].is_null());

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/approve"),
            Some(&passenger),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/approve"),
            Some(&transport),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let after = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(after.json["status"], "confirmed");
    assert_eq!(after.json["pickup_address"], "SECRET 12");

    // Location can't be read by outsiders, and isn't shared before the trip starts.
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/rides/{id}/location"),
            Some(&other),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    let loc = call(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&driver),
        Some(json!({"lat": 3.1, "lng": 101.6})),
    )
    .await;
    assert_eq!(loc.json["sharing"], false);
}

fn multipart(content_type: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "XgrBOUNDARY";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"r.png\"\r\nContent-Type: {content_type}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn upload(app: &Router, cookie: &str, ct: &str, bytes: &[u8]) -> Res {
    let (mct, body) = multipart(ct, bytes);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, mct)
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let b = res.into_body().collect().await.unwrap().to_bytes();
    Res {
        status,
        headers,
        json: serde_json::from_slice(&b).unwrap_or(Value::Null),
    }
}

#[tokio::test]
async fn uploads_are_sniffed_and_access_controlled() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    let stranger = login(&st, "sophie@example.org").await;

    // HTML disguised as a PNG is rejected.
    let r = upload(&app, &owner, "image/png", b"<script>alert(1)</script>").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&[0u8; 32]);
    let r = upload(&app, &owner, "application/octet-stream", &png).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["mime"], "image/png");
    let id = r.json["id"].as_i64().unwrap();

    let got = call(
        &app,
        Method::GET,
        &format!("/api/files/{id}"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(got.status, StatusCode::OK);
    assert!(got.headers["content-security-policy"]
        .to_str()
        .unwrap()
        .contains("sandbox"));
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/files/{id}"),
            Some(&stranger),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn settlement_custom_split_must_match_total() {
    let (app, st) = setup().await;
    let leader = login(&st, "driver@example.org").await;
    let body = |a: i64| {
        json!({
            "title": "羽球", "items": [{"label": "场租", "amount_cents": 10000}], "split_mode": "custom",
            "participants": [{"user_id": 4, "amount_cents": a}, {"user_id": 6, "amount_cents": 5000}],
            "payee_name": "李德明", "payee_account": "TNG 012"
        })
    };
    let r = call(
        &app,
        Method::POST,
        "/api/events/5/settlements",
        Some(&leader),
        Some(body(4000)),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = call(
        &app,
        Method::POST,
        "/api/events/5/settlements",
        Some(&leader),
        Some(body(5000)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    // Only the organiser of the event may create settlements.
    let member = login(&st, "member@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/events/5/settlements",
            Some(&member),
            Some(body(5000))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn ride_readiness_and_delay_workflow() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let transport = login(&st, "transport@example.org").await;

    // 1. Create and confirm ride
    let r = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": tomorrow("08:15"),
            "pickup_area_id": 2,
            "pickup_address": "SS2 PJ",
            "destination": "教会",
            "seats": 1
        })),
    )
    .await;
    let id = r.json["id"].as_i64().unwrap();
    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/claim"),
        Some(&driver),
        None,
    )
    .await;
    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/respond"),
        Some(&passenger),
        Some(json!({ "accept": true })),
    )
    .await;
    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/approve"),
        Some(&transport),
        None,
    )
    .await;

    // 2. Passenger indicates running 5m late
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&passenger),
        Some(json!({ "status": "delayed_5m" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    let ride = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(ride.json["readiness_status"], "delayed_5m");

    // 3. Driver reports 10m traffic delay
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/driver-delay"),
        Some(&driver),
        Some(json!({ "delay_minutes": 10 })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    let ride = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(ride.json["delay_minutes"], 10);

    // 4. Driver arrives at gate
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/arrived-gate"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    let ride = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&passenger),
        None,
    )
    .await;
    assert!(ride.json["arrived_at_gate_at"].is_string());

    // 5. Passenger confirms unable to attend (e.g. sick)
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&passenger),
        Some(json!({ "status": "unable_to_join", "notes": "woke up with fever" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    let ride = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(ride.json["status"], "cancelled");
    assert_eq!(ride.json["readiness_status"], "unable_to_join");
    // The optional reason reaches the ride for the driver and transport team.
    assert!(ride.json["attention_reason"]
        .as_str()
        .unwrap()
        .contains("woke up with fever"));

    // 6. User profile readiness preference
    let res = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&passenger),
        Some(json!({ "readiness_prompt_mins": 30 })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    let auth_me = call(&app, Method::GET, "/api/auth/me", Some(&passenger), None).await;
    assert_eq!(auth_me.json["readiness_prompt_mins"], 30);

    // 7. Driver profile gate wait tolerance
    let res = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "spare_seats": 2,
            "carpool_enabled": true,
            "area_ids": [1, 2],
            "gate_wait_mins": 8
        })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);

    let d_profile = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(d_profile.json["gate_wait_mins"], 8);
}

#[tokio::test]
async fn ride_trip_progress_updates_rejected_after_complete() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let transport = login(&st, "transport@example.org").await;

    let r = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": tomorrow("08:15"), "pickup_area_id": 2, "pickup_address": "SS2 PJ",
            "destination": "教会", "seats": 1,
        })),
    )
    .await;
    let id = r.json["id"].as_i64().unwrap();

    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/claim"),
        Some(&driver),
        None,
    )
    .await;
    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/respond"),
        Some(&passenger),
        Some(json!({ "accept": true })),
    )
    .await;
    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/approve"),
        Some(&transport),
        None,
    )
    .await;
    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/start"),
        Some(&driver),
        None,
    )
    .await;

    let complete = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/complete"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(complete.status, StatusCode::OK);

    // Once the ride is completed, these trip-progress updates must be rejected so no
    // misleading notifications are sent for a trip that has already ended.
    let readiness = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&passenger),
        Some(json!({ "status": "ready" })),
    )
    .await;
    assert_eq!(readiness.status, StatusCode::CONFLICT);

    let delay = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/driver-delay"),
        Some(&driver),
        Some(json!({ "delay_minutes": 10 })),
    )
    .await;
    assert_eq!(delay.status, StatusCode::CONFLICT);

    let arrived = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/arrived-gate"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(arrived.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn proxy_booking_and_frequent_riders_workflow() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let transport = login(&st, "transport@example.org").await;

    // 1. Saved proxy riders CRUD
    let r = call(
        &app,
        Method::POST,
        "/api/proxy-riders",
        Some(&passenger),
        Some(json!({
            "name": "陈妈妈",
            "phone": "012-3456789",
            "relationship": "parent",
            "notes": "需拐杖协助",
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let saved_rider_id = r.json["id"].as_i64().unwrap();

    let list = call(
        &app,
        Method::GET,
        "/api/proxy-riders",
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(list.json.as_array().unwrap().len(), 1);
    assert_eq!(list.json[0]["name"], "陈妈妈");
    assert_eq!(list.json[0]["phone"], "012-3456789");

    // 2. Create proxy ride
    let when = (grace_ride::util::local_now() + chrono::Duration::minutes(90))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let create_res = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": when,
            "pickup_area_id": 1,
            "pickup_address": "123 Persiaran Utama",
            "destination": "PJ State CMC",
            "seats": 1,
            "is_proxy": true,
            "rider_name": "陈妈妈",
            "rider_phone": "012-3456789",
            "rider_relationship": "parent",
        })),
    )
    .await;
    assert_eq!(create_res.status, StatusCode::OK);
    let ride_id = create_res.json["id"].as_i64().unwrap();

    // 3. Driver sees request with masked address and phone
    let drv_view = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride_id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(drv_view.status, StatusCode::OK);
    assert!(drv_view.json["pickup_address"].is_null());
    assert_eq!(drv_view.json["is_proxy"], true);
    assert_eq!(drv_view.json["proxy"]["rider_name"], "陈妈妈");
    assert_eq!(drv_view.json["proxy"]["rider_relationship"], "parent");
    assert!(drv_view.json["proxy"]["rider_phone"].is_null()); // masked until approved

    // 4. Driver claims ride
    let claim = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/claim"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(claim.status, StatusCode::OK);

    // 5. Passenger accepts
    let resp = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/respond"),
        Some(&passenger),
        Some(json!({ "accept": true })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);

    // 6. Transport staff approves
    let appr = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/approve"),
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(appr.status, StatusCode::OK);

    // 7. Approved driver now has unmasked address and rider phone
    let drv_view2 = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride_id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(drv_view2.status, StatusCode::OK);
    assert_eq!(drv_view2.json["pickup_address"], "123 Persiaran Utama");
    assert_eq!(drv_view2.json["proxy"]["rider_phone"], "012-3456789");
    assert!(!drv_view2.json["proxy"]["booker_phone"].is_null());

    // 8. Driver arrives at gate -> passenger receives notification mentioning rider name
    let arr = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/arrived-gate"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(arr.status, StatusCode::OK);

    let notifs = call(
        &app,
        Method::GET,
        "/api/notifications",
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(notifs.status, StatusCode::OK);
    let items = notifs.json["items"]
        .as_array()
        .or_else(|| notifs.json.as_array())
        .unwrap();
    let gate_msg = items
        .iter()
        .find(|n| n["body"].as_str().unwrap_or("").contains("陈妈妈"));
    assert!(
        gate_msg.is_some(),
        "Notification should mention rider name '陈妈妈'"
    );

    // 9. Delete proxy rider
    let del = call(
        &app,
        Method::DELETE,
        &format!("/api/proxy-riders/{saved_rider_id}"),
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(del.status, StatusCode::OK);

    let list2 = call(
        &app,
        Method::GET,
        "/api/proxy-riders",
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(list2.json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn reminder_job_notifies_passenger_and_driver() {
    let (_app, st) = setup().await;
    // Silence seeded rides so a run whose wall-clock time happens to sit within 60 minutes
    // of a demo ride does not generate extra reminder notifications.
    sqlx::query("UPDATE rides SET reminder_sent=1, driver_reminder_sent=1")
        .execute(&st.db)
        .await
        .unwrap();
    let soon = (grace_ride::util::local_now() + chrono::Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    sqlx::query("INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, status) VALUES (6, 4, $1, 2, 'x', 'y', 'confirmed')")
        .bind(&soon)
        .execute(&st.db)
        .await
        .unwrap();
    jobs::run_once(&st.db, &st.cfg).await.unwrap();
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notifications WHERE title='接送提醒' AND user_id IN (4,6)",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(n, 2);
    // Idempotent: a second run sends nothing new.
    jobs::run_once(&st.db, &st.cfg).await.unwrap();
    let (n2,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE title='接送提醒'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(n2, 2);
}

#[test]
fn production_config_rejects_dev_mode_and_plain_http() {
    let mut cfg = Config {
        dev_mode: true,
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
        google_oauth: Default::default(),
        android_cert_sha256_fingerprints: Vec::new(),
        ios_app_ids: Vec::new(),
        require_biometric_attestation: false,
    };
    assert!(
        cfg.validate().is_err(),
        "dev mode on a public domain must be refused"
    );
    cfg.dev_mode = false;
    assert!(cfg.validate().is_ok());
    cfg.public_url = "http://ride.example.org".into();
    assert!(
        cfg.validate().is_err(),
        "plain http on a public domain must be refused"
    );
}

// ---------------- Ride full lifecycle: start → pickup → complete (T-4/T-5) ----------------

#[tokio::test]
async fn ride_full_lifecycle_start_pickup_complete() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await; // id 6
    let driver = login(&st, "driver@example.org").await; // id 4
    let transport = login(&st, "transport@example.org").await; // id 2
    let outsider = login(&st, "sophie@example.org").await; // id 8

    let r = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(json!({
            "pickup_at": tomorrow("09:00"),
            "pickup_area_id": 2,
            "pickup_address": "SECRET ADDR",
            "destination": "教会",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let id = r.json["id"].as_i64().unwrap();

    let driver2 = login(&st, "driver2@example.org").await; // id 5
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/claim"),
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
            &format!("/api/rides/{id}/claim"),
            Some(&driver2),
            None
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/rides/{id}"),
            Some(&outsider),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/respond"),
            Some(&passenger),
            Some(json!({ "accept": true }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/approve"),
            Some(&transport),
            None
        )
        .await
        .status,
        StatusCode::OK
    );

    let after = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(after.json["status"], "confirmed");
    assert_eq!(after.json["pickup_address"], "SECRET ADDR");
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/rides/{id}"),
            Some(&outsider),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    let loc = call(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&driver),
        Some(json!({ "lat": 3.1, "lng": 101.6 })),
    )
    .await;
    assert_eq!(loc.json["sharing"], false);
    let loc = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}/location"),
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(loc.status, StatusCode::OK);
    assert_eq!(loc.json["sharing"], false);
    // Passenger can't share before the driver sets off; others can't share for them.
    let ploc = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&passenger),
        Some(json!({ "lat": 3.2, "lng": 101.7 })),
    )
    .await;
    assert_eq!(ploc.status, StatusCode::OK);
    assert_eq!(ploc.json["sharing"], false);

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/start"),
            Some(&driver),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let loc = call(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&driver),
        Some(json!({ "lat": 3.1, "lng": 101.6 })),
    )
    .await;
    assert_eq!(loc.json["sharing"], true);
    let loc = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}/location"),
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(loc.status, StatusCode::OK);
    assert_eq!(loc.json["sharing"], true);
    assert_eq!(loc.json["lat"], 3.1);
    assert!(loc.json.get("passenger").is_none());

    // Passenger opts in to showing where they're waiting; the driver sees it.
    for (who, want) in [
        (&driver, StatusCode::FORBIDDEN),
        (&outsider, StatusCode::FORBIDDEN),
    ] {
        let res = call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/passenger-location"),
            Some(who),
            Some(json!({ "lat": 3.2, "lng": 101.7 })),
        )
        .await;
        assert_eq!(res.status, want);
    }
    let bad_loc = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&passenger),
        Some(json!({ "lat": 91.0, "lng": 101.7 })),
    )
    .await;
    assert_eq!(bad_loc.status, StatusCode::BAD_REQUEST);
    let ploc = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&passenger),
        Some(json!({ "lat": 3.2, "lng": 101.7 })),
    )
    .await;
    assert_eq!(ploc.json["sharing"], true);
    let seen = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}/location"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(seen.json["passenger"]["lat"], 3.2);
    assert_eq!(seen.json["passenger"]["lng"], 101.7);

    // Stopping removes it; sharing again brings it back.
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/rides/{id}/passenger-location"),
            Some(&passenger),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let seen = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}/location"),
        Some(&driver),
        None,
    )
    .await;
    assert!(seen.json.get("passenger").is_none());
    call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&passenger),
        Some(json!({ "lat": 3.2, "lng": 101.7 })),
    )
    .await;

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/pickup"),
            Some(&driver),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    // Once in the car, the passenger's spot is dropped and no longer accepted.
    let seen = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}/location"),
        Some(&driver),
        None,
    )
    .await;
    assert!(seen.json.get("passenger").is_none());
    let (left,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM passenger_locations WHERE ride_id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(left, 0);
    let ploc = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&passenger),
        Some(json!({ "lat": 3.2, "lng": 101.7 })),
    )
    .await;
    assert_eq!(ploc.json["sharing"], false);

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/messages"),
            Some(&passenger),
            Some(json!({ "body": "在北门见" }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/rides/{id}/messages"),
            Some(&outsider),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/complete"),
            Some(&driver),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let done = call(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(done.json["status"], "completed");

    let loc = call(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&driver),
        Some(json!({ "lat": 3.1, "lng": 101.6 })),
    )
    .await;
    assert_eq!(loc.json["sharing"], false);
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/messages"),
            Some(&passenger),
            Some(json!({ "body": "after" }))
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
}

// ---------------- Live-location cap: MAX_LIVE_DRIVERS=10 (T-6) ----------------

async fn insert_driver_session(st: &AppState, n: i64) -> (i64, String) {
    use sha2::{Digest, Sha256};
    let raw = format!(
        "cap-driver-{:03}-{}",
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut token = raw;
    while token.len() < 48 {
        token.push('0');
    }
    token.truncate(48);
    let hash = hex::encode(Sha256::digest(token.as_bytes()));
    let pw_hash = grace_ride::routes::auth::hash_password("GraceRide2026!").unwrap();
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, password_hash, webauthn_uuid, status) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(format!("容量司机{n}"))
    .bind(format!("capdriver{n}@example.org"))
    .bind("012-000 0000")
    .bind("driver")
    .bind(&pw_hash)
    .bind(hash.as_str())
    .bind("active")
    .fetch_one(&st.db)
    .await
    .unwrap();
    // Give the driver a vehicle so driver_load seat-capacity check passes.
    sqlx::query(
        "INSERT INTO vehicles (user_id, model, color, plate, seats) VALUES ($1, 'Toyota', 'White', 'CAP-000', 4)",
    )
    .bind(id)
    .execute(&st.db)
    .await
    .unwrap();
    let expires = (grace_ride::util::local_now() + chrono::Duration::days(7))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,1,$3,$4)",
    )
    .bind(&hash)
    .bind(id)
    .bind(&expires)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    (id, token)
}

#[tokio::test]
async fn live_location_max_drivers_cap() {
    let (app, st) = setup().await;
    let transport = login(&st, "transport@example.org").await;
    let _passenger = login(&st, "member@example.org").await;
    let tomorrow_ts = tomorrow("10:00");

    // Insert 10 confirmed rides directly into the DB (bypasses create_ride validation quirks),
    // each with a distinct driver. Then start them via the API to exercise the cap logic.
    let mut drivers = Vec::new();
    for n in 0..10 {
        let (driver_id, token) = insert_driver_session(&st, n).await;
        let cookie = format!("gr_session={token}");
        let (rid,): (i64,) = sqlx::query_as(
            "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, status, created_at, updated_at)
             VALUES ($1, $2, $3, 1, 'CAP ADDR', '教会', 1, 'confirmed', $4, $5) RETURNING id",
        )
        .bind(6i64) // passenger = member@example.org
        .bind(driver_id)
        .bind(&tomorrow_ts)
        .bind(&tomorrow_ts)
        .bind(&tomorrow_ts)
        .fetch_one(&st.db)
        .await
        .unwrap();
        assert_eq!(
            call(
                &app,
                Method::POST,
                &format!("/api/rides/{rid}/start"),
                Some(&cookie),
                None
            )
            .await
            .status,
            StatusCode::OK,
            "ride {} start",
            n
        );
        drivers.push((rid, cookie));
    }

    let (live,): (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(live, 10, "expected 10 concurrent live drivers");

    // 11th driver: create ride directly, approve, then start must exceed cap.
    let (driver_id_11, token11) = insert_driver_session(&st, 50).await;
    let cookie11 = format!("gr_session={token11}");
    let (id11,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, status, created_at, updated_at)
         VALUES ($1, $2, $3, 1, 'CAP 11', '教会', 1, 'pending_approval', $4, $5) RETURNING id",
    )
    .bind(6i64)
    .bind(driver_id_11)
    .bind(&tomorrow_ts)
    .bind(&tomorrow_ts)
    .bind(&tomorrow_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{id11}/approve"),
            Some(&transport),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let r11start = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id11}/start"),
        Some(&cookie11),
        None,
    )
    .await;
    // Debug: verify driver 11's session is valid
    let me = call(&app, Method::GET, "/api/auth/me", Some(&cookie11), None).await;
    assert_eq!(
        me.status,
        StatusCode::OK,
        "driver 11 session invalid: {}",
        me.json
    );
    assert_eq!(
        r11start.status,
        StatusCode::CONFLICT,
        "11th driver must exceed cap"
    );

    // Completing one ride frees a slot: the 11th can now start.
    let (rid0, c0) = drivers.remove(0);
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{rid0}/complete"),
            Some(&c0),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let r11start2 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id11}/start"),
        Some(&cookie11),
        None,
    )
    .await;
    assert_eq!(
        r11start2.status,
        StatusCode::OK,
        "11th driver can start after a slot frees"
    );
}

// ---------------- Reimbursement flow: claim → transport approve → finance pay (T-8) ----------------

#[tokio::test]
async fn reimbursement_flow_claim_approve_pay() {
    let (app, st) = setup().await;
    let claimant = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let finance = login(&st, "finance@example.org").await;

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&[0u8; 32]);
    let boundary = "XgrBOUNDARY";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"r.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(&png);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let mct = format!("multipart/form-data; boundary={boundary}");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(header::COOKIE, &claimant)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, &mct)
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let file_json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK);
    let receipt_id = file_json["id"].as_i64().unwrap();

    let r = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "ride_id": null,
            "amount_cents": 2850,
            "description": "Grab 替代交通",
            "receipt_file_id": receipt_id,
            "payee_name": "陈法拉",
            "payee_phone": "012-300 0006"
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let reimb_id = r.json["id"].as_i64().unwrap();

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{reimb_id}/review"),
            Some(&transport),
            Some(json!({ "action": "approve", "note": "" }))
        )
        .await
        .status,
        StatusCode::OK
    );

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{reimb_id}/pay"),
            Some(&finance),
            Some(json!({ "payee_verified": true, "paid_ref": "DuitNow12345" }))
        )
        .await
        .status,
        StatusCode::OK
    );

    let got = call(
        &app,
        Method::GET,
        &format!("/api/reimbursements/{reimb_id}"),
        Some(&finance),
        None,
    )
    .await;
    assert_eq!(got.json["status"], "paid");
    assert_eq!(got.json["paid_ref"], "DuitNow12345");

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{reimb_id}/pay"),
            Some(&claimant),
            Some(json!({ "payee_verified": true, "paid_ref": "X" }))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
}

// ---------------- Monthly report (T-10 / R-21) ----------------

#[tokio::test]
async fn monthly_report_returns_user_stats() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;

    let this_month = grace_ride::util::local_now().format("%Y-%m").to_string();
    let r = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month={this_month}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["user"]["id"], 6);
    assert_eq!(r.json["month"], this_month);
    for key in [
        "rides_driven",
        "rides_received",
        "events_attended",
        "costs_paid_cents",
        "by_kind",
        "events",
        "badges",
    ] {
        assert!(
            r.json[key].is_array() || r.json[key].is_number(),
            "missing {key}"
        );
    }

    let stranger = login(&st, "sophie@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/reports/monthly?user_id=6&month={this_month}"),
            Some(&stranger),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    let admin = login(&st, "admin@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/reports/monthly?user_id=6&month={this_month}"),
            Some(&admin),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
}

// ---------------- Volunteer badges: public criteria + opt-out (T-11 / R-29) ----------------

#[tokio::test]
async fn badges_public_criteria_and_opt_out() {
    let (app, st) = setup().await;
    let driver = login(&st, "driver@example.org").await;

    let r = call(&app, Method::GET, "/api/badges", Some(&driver), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let badges = r.json.as_array().unwrap();
    assert!(!badges.is_empty(), "driver should have badge definitions");
    for b in badges {
        assert!(b["key"].is_string());
        assert!(b["char"].is_string());
        assert!(b["name"].is_string());
        assert!(b["rule"].is_string());
        assert!(b["progress"].is_number());
        assert!(b["target"].is_number());
        assert!(b["earned"].is_boolean());
    }

    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&driver),
            Some(json!({ "name": "李德明", "phone": "012-300 0004", "badges_opt_out": true }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let r = call(&app, Method::GET, "/api/badges", Some(&driver), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.json.as_array().unwrap().is_empty(),
        "opted-out user has no badges"
    );
}

// ---------------- Targeted activity events + signups/waitlist (T-12) ----------------

#[tokio::test]
async fn activity_events_group_scoped_signup_waitlist() {
    let (app, st) = setup().await;
    let leader = login(&st, "driver@example.org").await;
    let member = login(&st, "member@example.org").await;
    let outsider = login(&st, "sophie@example.org").await;

    let r = call(&app, Method::GET, "/api/events", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let ids: Vec<_> = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_i64().unwrap())
        .collect();
    assert!(ids.contains(&1), "member sees event 1: {ids:?}");
    assert!(ids.contains(&4), "everyone sees event 4: {ids:?}");

    let r = call(
        &app,
        Method::POST,
        "/api/events/1/signup",
        Some(&member),
        // In both of the event's groups, so they say which one they come as.
        Some(json!({ "need_ride": true, "offer_drive": false, "as_group_id": 2 })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["status"], "going");

    let r = call(
        &app,
        Method::POST,
        "/api/events/2/signup",
        Some(&outsider),
        Some(json!({ "need_ride": false, "offer_drive": false })),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::FORBIDDEN,
        "outsider cannot join group-3 event: {}",
        r.json
    );

    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/events/1/signups",
            Some(&leader),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/events/1/signups",
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/events/1/attendance",
            Some(&leader),
            Some(json!({ "user_id": 6, "attended": true }))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Already marked attended: withdrawal must be refused until attendance is cleared.
    let blocked = call(
        &app,
        Method::POST,
        "/api/events/1/withdraw",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(blocked.status, StatusCode::BAD_REQUEST, "{}", blocked.json);
    assert_eq!(blocked.json["code"], "INVALID_ACTION");

    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/events/1/attendance",
            Some(&leader),
            Some(json!({ "user_id": 6, "attended": false }))
        )
        .await
        .status,
        StatusCode::OK
    );

    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/events/1/withdraw",
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
}

// ---------------- Retention housekeeping: config validates range (R-30) ----------------

#[tokio::test]
async fn retention_config_range_and_housekeeping_ids() {
    let (_app, st) = setup().await;
    // Config validates max_live_drivers in 1..=100; chat/audit retention days
    // (defaults 180/730) are referenced by jobs.rs housekeeping.
    let cfg = &st.cfg;
    assert!(cfg.max_live_drivers >= 1 && cfg.max_live_drivers <= 100);
    assert!(cfg.chat_retention_days > 0);
    assert!(cfg.audit_retention_days > 0);
    // Housekeeping purges old ride_messages / audit_logs / read notifications.
    jobs::run_once(&st.db, &st.cfg).await.unwrap();
}

// ---------------- Disaster recovery: backup + restore drill (T-13) ----------------

#[tokio::test]
async fn disaster_recovery_backup_restore_drill() {
    use std::path::PathBuf;
    use std::process::Command;

    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    assert_eq!(
        call(&app, Method::GET, "/api/admin/audit", Some(&admin), None)
            .await
            .status,
        StatusCode::OK
    );

    // Upload a file and insert a custom DB row before taking the snapshot so restore.sh
    // proves both DB rows and uploaded files round-trip cleanly.
    let uploaded_id = upload_png(&app, &admin).await;
    let stored_name: String = sqlx::query_scalar("SELECT stored_name FROM files WHERE id=$1")
        .bind(uploaded_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let orig_bytes = std::fs::read(st.cfg.upload_dir.join(&stored_name)).unwrap();
    sqlx::query("INSERT INTO areas (name, active) VALUES ('备份恢复演练区', 1)")
        .execute(&st.db)
        .await
        .unwrap();

    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/grace_ride_test".into());
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root");
    let backup_dir = std::env::temp_dir().join(format!("gr-backups-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&backup_dir).unwrap();
    let data_dir = st.cfg.upload_dir.parent().expect("upload parent");
    let test_schema = st
        .cfg
        .database_url
        .split("options[search_path]=")
        .nth(1)
        .and_then(|search_path| search_path.split(',').next())
        .expect("test database schema")
        .to_string();
    let out = Command::new("bash")
        .env("DATABASE_URL", &database_url)
        .env("BACKUP_SCHEMA", &test_schema)
        .env("REQUIRE_ENCRYPTION", "false")
        .current_dir(&repo)
        .arg("./scripts/backup.sh")
        .arg(data_dir)
        .arg(&backup_dir)
        .arg("1")
        .output()
        .expect("backup script failed to start");
    assert!(
        out.status.success(),
        "backup.sh exit: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let entries: Vec<_> = std::fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(!entries.is_empty(), "backup produced at least one artifact");
    let newest = entries
        .into_iter()
        .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
        .unwrap();
    assert!(
        newest.metadata().unwrap().len() > 0,
        "backup artifact is non-empty"
    );

    // Mutate DB and wipe uploads into a fresh restore target, close active pool connections
    // on `test_schema`, then run scripts/restore.sh and verify full round-trip recovery.
    sqlx::query("DELETE FROM areas WHERE name='备份恢复演练区'")
        .execute(&st.db)
        .await
        .unwrap();
    st.db.close().await;
    drop(app);

    let restore_data_dir =
        std::env::temp_dir().join(format!("gr-restored-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&restore_data_dir).unwrap();
    let restore_out = Command::new("bash")
        .env("DATABASE_URL", &database_url)
        .current_dir(&repo)
        .arg("./scripts/restore.sh")
        .arg(newest.path())
        .arg(&restore_data_dir)
        .output()
        .expect("restore script failed to start");
    assert!(
        restore_out.status.success(),
        "restore.sh exit: stdout={} stderr={}",
        String::from_utf8_lossy(&restore_out.stdout),
        String::from_utf8_lossy(&restore_out.stderr)
    );

    let restored_file = restore_data_dir.join("uploads").join(&stored_name);
    assert_eq!(
        std::fs::read(&restored_file).expect("restored upload file"),
        orig_bytes
    );
    let verify_db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(60))
        .connect(&st.cfg.database_url)
        .await
        .unwrap();
    let area_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM areas WHERE name='备份恢复演练区'")
            .fetch_one(&verify_db)
            .await
            .unwrap();
    assert_eq!(area_count, 1, "restore.sh recovered custom DB row");
    verify_db.close().await;

    let _ = std::fs::remove_dir_all(&restore_data_dir);
    let _ = std::fs::remove_dir_all(&backup_dir);
}

#[tokio::test]
async fn stream_session_check_sees_sign_out_and_disabling() {
    let (_app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let token = cookie.split('=').nth(1).unwrap();
    let session = grace_ride::auth::hash_token(token);
    assert!(grace_ride::stream::session_alive(&st.db, &session).await);

    sqlx::query("UPDATE users SET status='disabled' WHERE email='member@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    assert!(!grace_ride::stream::session_alive(&st.db, &session).await);

    sqlx::query("UPDATE users SET status='active' WHERE email='member@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM sessions WHERE token=$1")
        .bind(&session)
        .execute(&st.db)
        .await
        .unwrap();
    assert!(!grace_ride::stream::session_alive(&st.db, &session).await);
}

#[tokio::test]
async fn two_step_code_guesses_are_limited_per_member() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let mut limited_at = None;
    for i in 0..12 {
        let r = call(
            &app,
            Method::POST,
            "/api/auth/totp/verify",
            Some(&cookie),
            Some(json!({"code": format!("{:06}", i)})),
        )
        .await;
        if r.status == StatusCode::TOO_MANY_REQUESTS {
            assert!(r.headers.contains_key(header::RETRY_AFTER));
            limited_at = Some(i);
            break;
        }
    }
    assert_eq!(limited_at, Some(10), "the 11th guess should be refused");

    // Signing in again doesn't reset the allowance: it belongs to the member.
    let again = login(&st, "member@example.org").await;
    assert_ne!(again, cookie);
    let r = call(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&again),
        Some(json!({"code": "000000"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);

    // Another member's session has its own allowance.
    let other = login(&st, "sophie@example.org").await;
    let r = call(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&other),
        Some(json!({"code": "000000"})),
    )
    .await;
    assert_ne!(r.status, StatusCode::TOO_MANY_REQUESTS);
}
