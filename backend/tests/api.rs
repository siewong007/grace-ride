//! End-to-end API tests against a fresh SQLite database with demo data.
//! Each test builds its own app (own DB file, own rate limiter).

use axum::{
    body::Body,
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use grace_ride::{auth::hash_token, init, jobs, router, state::AppState, state::Config};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const PW: &str = "GraceRide2026!";
const ORIGIN: &str = "http://localhost:5173";

async fn setup() -> (Router, AppState) {
    let dir = std::env::temp_dir().join(format!("gr-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        dev_mode: true,
        database_url: format!("sqlite://{}/test.db", dir.display()),
        upload_dir: dir.join("uploads"),
        static_dir: None,
        public_url: ORIGIN.into(),
        bind: "127.0.0.1:0".into(),
        max_live_drivers: 10,
        trust_proxy: false,
        chat_retention_days: 180,
        audit_retention_days: 730,
        json_logs: false,
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    (router(state.clone()), state)
}

struct Res {
    status: StatusCode,
    headers: HeaderMap,
    json: Value,
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

async fn login(app: &Router, email: &str) -> String {
    let r = call(
        app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": email, "password": PW})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "login {email}: {}", r.json);
    let set = r
        .headers
        .get(header::SET_COOKIE)
        .expect("cookie")
        .to_str()
        .unwrap();
    set.split(';').next().unwrap().to_string()
}

fn tomorrow(hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (chrono::Local::now() + chrono::Duration::days(1)).format("%Y-%m-%d")
    )
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
async fn session_tokens_are_stored_hashed() {
    let (app, st) = setup().await;
    let cookie = login(&app, "member@example.org").await;
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
async fn account_lockout_after_repeated_failures() {
    let (app, _) = setup().await;
    for _ in 0..5 {
        let r = call(
            &app,
            Method::POST,
            "/api/auth/login",
            None,
            Some(json!({"email": "member@example.org", "password": "wrong-password"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
    }
    // Correct password is refused while locked.
    let r = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "member@example.org", "password": PW})),
    )
    .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    // Unknown emails get the same generic error (no account enumeration).
    let r = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({"email": "nobody@example.org", "password": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["error"], "电邮或密码不正确");
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
    let (app, _) = setup().await;
    let cookie = login(&app, "member@example.org").await;
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
    let (app, _) = setup().await;
    let member = login(&app, "member@example.org").await;
    let transport = login(&app, "transport@example.org").await;
    let finance = login(&app, "finance@example.org").await;
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
    assert_eq!(
        call(&app, Method::GET, "/api/admin/users", Some(&finance), None)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
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
    let (app, _) = setup().await;
    let passenger = login(&app, "member@example.org").await;
    let driver = login(&app, "driver@example.org").await;
    let driver2 = login(&app, "driver2@example.org").await;
    let transport = login(&app, "transport@example.org").await;
    let other = login(&app, "sophie@example.org").await;

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
    let (app, _) = setup().await;
    let owner = login(&app, "member@example.org").await;
    let stranger = login(&app, "sophie@example.org").await;

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
    let (app, _) = setup().await;
    let leader = login(&app, "driver@example.org").await;
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
    let member = login(&app, "member@example.org").await;
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
async fn reminder_job_notifies_passenger_and_driver() {
    let (_app, st) = setup().await;
    let soon = (chrono::Local::now() + chrono::Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    sqlx::query("INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, status) VALUES (6, 4, ?, 2, 'x', 'y', 'confirmed')")
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
        database_url: "sqlite::memory:".into(),
        upload_dir: "/tmp".into(),
        static_dir: None,
        public_url: "https://ride.example.org".into(),
        bind: "0.0.0.0:8080".into(),
        max_live_drivers: 10,
        trust_proxy: true,
        chat_retention_days: 180,
        audit_retention_days: 730,
        json_logs: true,
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
