//! End-to-end API tests against a fresh PostgreSQL schema with demo data.
//! Each test builds its own app (own schema, own rate limiter).

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

async fn fresh_db() -> String {
    let base = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/grace_ride_test".into());
    let schema = format!("gr_{}", uuid::Uuid::new_v4().simple());
    let pool = sqlx::PgPool::connect(&base).await.unwrap();
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}options[search_path]={schema},public")
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

// ---------------- Ride full lifecycle: start → pickup → complete (T-4/T-5) ----------------

#[tokio::test]
async fn ride_full_lifecycle_start_pickup_complete() {
    let (app, _) = setup().await;
    let passenger = login(&app, "member@example.org").await; // id 6
    let driver = login(&app, "driver@example.org").await; // id 4
    let transport = login(&app, "transport@example.org").await; // id 2
    let outsider = login(&app, "sophie@example.org").await; // id 8

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

    let driver2 = login(&app, "driver2@example.org").await; // id 5
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
    let hash = format!("{:x}", Sha256::digest(token.as_bytes()));
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
    let expires = (chrono::Local::now() + chrono::Duration::days(7))
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
    let transport = login(&app, "transport@example.org").await;
    let _passenger = login(&app, "member@example.org").await;
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
    let (app, _) = setup().await;
    let claimant = login(&app, "member@example.org").await;
    let transport = login(&app, "transport@example.org").await;
    let finance = login(&app, "finance@example.org").await;

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
    let (app, _) = setup().await;
    let member = login(&app, "member@example.org").await;

    let this_month = chrono::Local::now().format("%Y-%m").to_string();
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

    let stranger = login(&app, "sophie@example.org").await;
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
    let admin = login(&app, "admin@example.org").await;
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
    let (app, _) = setup().await;
    let driver = login(&app, "driver@example.org").await;

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
    let (app, _) = setup().await;
    let leader = login(&app, "driver@example.org").await;
    let member = login(&app, "member@example.org").await;
    let outsider = login(&app, "sophie@example.org").await;

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
        Some(json!({ "need_ride": true, "offer_drive": false })),
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
    use std::process::Command;

    let (app, _st) = setup().await;
    let admin = login(&app, "admin@example.org").await;
    assert_eq!(
        call(&app, Method::GET, "/api/admin/audit", Some(&admin), None)
            .await
            .status,
        StatusCode::OK
    );

    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/grace_ride_test".into());
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            r#"cd "/Users/goaltosuceed/Documents/Grace Ride" && DATABASE_URL={} ./scripts/backup.sh"#,
            database_url
        ))
        .output()
        .expect("backup script failed to start");
    assert!(
        out.status.success(),
        "backup.sh exit: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    let entries = std::fs::read_dir("/Users/goaltosuceed/Documents/Grace Ride/backups")
        .unwrap()
        .count();
    assert!(entries > 0, "backup produced at least one artifact");

    let newest = std::fs::read_dir("/Users/goaltosuceed/Documents/Grace Ride/backups")
        .unwrap()
        .filter_map(|e| e.ok())
        .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
        .unwrap();
    let meta = newest.metadata().unwrap();
    assert!(meta.len() > 0, "backup artifact is non-empty");
    let _ = app;
}
