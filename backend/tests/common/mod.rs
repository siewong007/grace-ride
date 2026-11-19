//! Shared helpers for integration tests.
#![allow(dead_code)]

pub mod schema;

use axum::{
    body::Body,
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use grace_ride::{
    auth::{hash_token, SESSION_COOKIE},
    init, router, seed,
    state::{AppState, Config},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

pub const PW: &str = "GraceRide2026!";
pub const ORIGIN: &str = "http://localhost:5173";

pub struct Res {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub json: Value,
}

pub async fn fresh_db() -> String {
    schema::fresh_schema("gr_").await
}

pub async fn setup() -> (Router, AppState) {
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
        db: grace_ride::state::DbConfig {
            max_connections: 5,
            acquire_timeout_secs: 60,
            ..Default::default()
        },
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    register_test_db(&state);
    (router(state.clone()), state)
}

/// Lets a suite with its own `setup()` opt into [`with_event_shim`].
pub fn register_test_db(state: &AppState) {
    TEST_DB.with(|d| *d.borrow_mut() = Some(state.db.clone()));
}

thread_local! {
    /// The pool of the test running on this thread (each `#[tokio::test]` has its own thread).
    static TEST_DB: std::cell::RefCell<Option<sqlx::PgPool>> = const { std::cell::RefCell::new(None) };
}

/// Rides now exist only for events, and a driver may claim only for an event they joined.
/// Most workflow tests just need *a* ride, so a `POST /api/rides` without an `event_id`
/// gets a fresh published event (everyone signed up as going) and the body is pointed at it;
/// a claim re-signs everyone up first. Tests of the event rules pass `event_id` themselves.
pub async fn with_event_shim(method: &Method, path: &str, body: Option<Value>) -> Option<Value> {
    let pool = TEST_DB.with(|d| d.borrow().clone())?;
    let is_claim =
        *method == Method::POST && path.starts_with("/api/rides/") && path.ends_with("/claim");
    let is_book = *method == Method::POST && path == "/api/rides";
    if !is_claim && !is_book {
        return body;
    }
    if is_book {
        let mut v = body?;
        if v.get("event_id").is_some_and(|e| !e.is_null()) {
            return Some(v);
        }
        let (event,): (i64,) = sqlx::query_as(
            "INSERT INTO events (title, starts_at, location, created_by, reminder_sent)
             VALUES ('Test event', $1, 'Church hall', (SELECT MIN(id) FROM users), 1) RETURNING id",
        )
        .bind(tomorrow("19:30"))
        .fetch_one(&pool)
        .await
        .ok()?;
        sqlx::query(
            "INSERT INTO event_groups (event_id, group_id) SELECT $1, id FROM groups WHERE is_all = 1",
        )
        .bind(event)
        .execute(&pool)
        .await
        .ok()?;
        sqlx::query(
            "INSERT INTO event_signups (event_id, user_id, status, created_at)
             SELECT $1, id, 'going', '2026-01-01 00:00' FROM users ON CONFLICT DO NOTHING",
        )
        .bind(event)
        .execute(&pool)
        .await
        .ok()?;
        v["event_id"] = Value::from(event);
        return Some(v);
    }
    // Claim: sign every user up to the ride's event.
    let id: i64 = path.split('/').nth(3)?.parse().ok()?;
    let _ = sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, created_at)
         SELECT r.event_id, u.id, 'going', '2026-01-01 00:00' FROM users u, rides r
         WHERE r.id = $1 AND r.event_id IS NOT NULL ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .execute(&pool)
    .await;
    body
}

/// Seed the edge-case fixtures (`EDGE_BATCH`) into a fresh test DB.
///
/// Opt-in: call this when a test needs disabled/no-pw/TOTP users, every ride
/// status, all reimbursement/settlement statuses, help tickets, notifications,
/// proxy riders, etc. Users use stable IDs 90001..=90010; look them up by
/// stable email (e.g. `edgemember@example.org`, `edgestaff@example.org`).
pub async fn seed_edge(st: &AppState) {
    seed::seed_edge(&st.db).await.expect("seed_edge");
}

pub async fn call(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Res {
    call_with(app, method, path, cookie, body, Some(ORIGIN)).await
}

pub async fn call_with(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
    origin: Option<&str>,
) -> Res {
    let body = with_event_shim(&method, path, body).await;
    call_exact(app, method, path, cookie, body, origin).await
}

/// Like [`call_with`] but sends the request untouched, for tests of the event rules.
pub async fn call_exact(
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

pub async fn call_raw(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    b = b.header(header::ORIGIN, ORIGIN);
    let req = b.body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&bytes).to_string();
    (status, headers, body)
}

/// Mint a hashed session. `mfa_ok` controls privileged MFA for staff roles.
pub async fn login_mfa(st: &AppState, email: &str, mfa_ok: bool) -> String {
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
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(hash_token(&token))
    .bind(id)
    .bind(if mfa_ok { 1i64 } else { 0 })
    .bind(&expires)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    format!("{SESSION_COOKIE}={token}")
}

pub async fn login(st: &AppState, email: &str) -> String {
    login_mfa(st, email, true).await
}

pub fn tomorrow(hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (grace_ride::util::today_local() + chrono::Duration::days(1)).format("%Y-%m-%d")
    )
}

pub async fn upload_png(app: &Router, cookie: &str) -> i64 {
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
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, &mct)
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let file_json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "{file_json}");
    file_json["id"].as_i64().unwrap()
}

pub fn totp_code(secret_b32: &str, email: &str) -> String {
    use totp_rs::{Builder, Secret};
    let secret = Secret::try_from_base32(secret_b32).expect("secret");
    let totp = Builder::new()
        .with_secret(secret)
        .with_issuer(Some("Grace Ride"))
        .with_account_name(email)
        .build()
        .expect("totp");
    totp.generate_current().to_string()
}

/// `member_roles` (kept in step by triggers in 0001_init.sql) must hold exactly the names in each member's
/// `users.roles` that are real roles, apart from the implicit `member`.
pub async fn assert_member_roles_in_step(db: &sqlx::PgPool) {
    let drift: Vec<(i64, String)> = sqlx::query_as(
        "WITH want AS (
             SELECT u.id AS user_id, k AS role_key FROM users u, unnest(listed_role_keys(u.roles)) AS k
             WHERE EXISTS (SELECT 1 FROM roles r WHERE r.key = k))
         (SELECT user_id, '+' || role_key FROM want EXCEPT SELECT user_id, '+' || role_key FROM member_roles)
         UNION ALL
         (SELECT user_id, '-' || role_key FROM member_roles EXCEPT SELECT user_id, '-' || role_key FROM want)",
    )
    .fetch_all(db)
    .await
    .unwrap();
    assert!(
        drift.is_empty(),
        "member_roles out of step (+missing, -extra): {drift:?}"
    );
}
