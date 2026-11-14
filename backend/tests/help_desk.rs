//! Help desk API: ticket filters, counts, reopen, on-behalf requests, guide feedback and ordering.

#[path = "common/schema.rs"]
mod schema;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use grace_ride::{
    auth::{hash_token, SESSION_COOKIE},
    init, router,
    state::{AppState, Config},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

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
        google_client_id: None,
        google_client_secret: None,
        routing: Default::default(),
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    (router(state.clone()), state)
}

struct Res {
    status: StatusCode,
    json: Value,
}

async fn call(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Res {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    b = b.header(header::ORIGIN, ORIGIN);
    let req = match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Res { status, json }
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
    let expires = (chrono::Local::now() + chrono::Duration::days(7))
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

async fn seed_guide(st: &AppState, slug: &str) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO guide_workflows (slug, title_en, title_zh, steps, published, audience, updated_at)
         VALUES ($1,'Guide','指南','[]'::jsonb,TRUE,'member',$2) RETURNING id",
    )
    .bind(slug)
    .bind(grace_ride::util::now())
    .fetch_one(&st.db)
    .await
    .unwrap();
    id
}

async fn user_id(st: &AppState, email: &str) -> i64 {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn tickets_filter_count_and_reopen() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let staff = login(&st, "transport@example.org").await;
    let r = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject":"Driver late","body":"He has not come"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let id = r.json["id"].as_i64().unwrap();
    call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject":"Payment","body":"Question"})),
    )
    .await;
    let r = call(
        &app,
        Method::POST,
        &format!("/api/help/{id}/replies"),
        Some(&staff),
        Some(json!({"body":"On it"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    let r = call(
        &app,
        Method::GET,
        "/api/help?status=answered",
        Some(&member),
        None,
    )
    .await;
    let rows = r.json.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["last_body"], "On it");
    assert_eq!(rows[0]["last_staff"], true);

    let r = call(
        &app,
        Method::GET,
        "/api/help?all=true&q=pay",
        Some(&staff),
        None,
    )
    .await;
    assert_eq!(r.json.as_array().unwrap().len(), 1);
    let r = call(
        &app,
        Method::GET,
        "/api/help?all=true&q=%25",
        Some(&staff),
        None,
    )
    .await;
    assert_eq!(r.json.as_array().unwrap().len(), 0, "wildcards are literal");

    let r = call(&app, Method::GET, "/api/help/counts", Some(&member), None).await;
    assert_eq!(
        (
            r.json["open"].as_i64(),
            r.json["answered"].as_i64(),
            r.json["closed"].as_i64()
        ),
        (Some(1), Some(1), Some(0))
    );

    call(
        &app,
        Method::POST,
        &format!("/api/help/{id}/close"),
        Some(&member),
        None,
    )
    .await;
    let r = call(
        &app,
        Method::POST,
        &format!("/api/help/{id}/reopen"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(
        &app,
        Method::GET,
        &format!("/api/help/{id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.json["status"], "open");
    assert!(r.json["member"].is_null(), "member block is staff only");
    let r = call(
        &app,
        Method::GET,
        &format!("/api/help/{id}"),
        Some(&staff),
        None,
    )
    .await;
    assert!(r.json["member"]["name"].is_string());
}

#[tokio::test]
async fn staff_can_raise_request_for_member() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let staff = login(&st, "transport@example.org").await;
    let mid = user_id(&st, "member@example.org").await;

    let r = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject":"x","body":"y","for_user_id":mid + 999})),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::FORBIDDEN,
        "members cannot raise on behalf"
    );
    let r = call(
        &app,
        Method::GET,
        "/api/help/members?q=",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    let r = call(
        &app,
        Method::GET,
        "/api/help/members?q=",
        Some(&staff),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.json.as_array().unwrap().iter().any(|m| m["id"] == mid));

    let r = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&staff),
        Some(json!({"subject":"Phone call","body":"We rang you","for_user_id":mid})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let id = r.json["id"].as_i64().unwrap();
    let r = call(
        &app,
        Method::GET,
        &format!("/api/help/{id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["user_id"], mid);
    assert_eq!(r.json["status"], "answered");
    assert_eq!(r.json["replies"][0]["staff"], true);
    let r = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&staff),
        Some(json!({"subject":"x","body":"y","for_user_id":999_999_999})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn guide_feedback_and_reorder() {
    let (app, st) = setup().await;
    let a = seed_guide(&st, "a-guide").await;
    let b = seed_guide(&st, "b-guide").await;
    let member = login(&st, "member@example.org").await;
    let staff = login(&st, "transport@example.org").await;

    let r = call(
        &app,
        Method::POST,
        &format!("/api/help/guides/{a}/feedback"),
        Some(&member),
        Some(json!({"helpful":true})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    call(
        &app,
        Method::POST,
        &format!("/api/help/guides/{a}/feedback"),
        Some(&member),
        Some(json!({"helpful":false})),
    )
    .await;
    call(
        &app,
        Method::POST,
        &format!("/api/help/guides/{b}/feedback"),
        Some(&staff),
        Some(json!({"helpful":true})),
    )
    .await;

    let r = call(&app, Method::GET, "/api/help/guides", Some(&staff), None).await;
    let guides = r.json.as_array().unwrap();
    let ga = guides.iter().find(|g| g["slug"] == "a-guide").unwrap();
    assert_eq!(
        (ga["helpful_yes"].as_i64(), ga["helpful_no"].as_i64()),
        (Some(0), Some(1)),
        "re-vote replaces"
    );
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides?full=true",
        Some(&member),
        None,
    )
    .await;
    let ga = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["slug"] == "a-guide")
        .unwrap()
        .clone();
    assert!(ga["steps"].is_array());
    assert!(ga.get("helpful_yes").is_none(), "tallies are editor only");

    let r = call(
        &app,
        Method::POST,
        "/api/help/guides/reorder",
        Some(&member),
        Some(json!({"ids":[b,a]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides/reorder",
        Some(&staff),
        Some(json!({"ids":[b,a]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(&app, Method::GET, "/api/help/guides", Some(&member), None).await;
    let slugs: Vec<_> = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["slug"].as_str().unwrap().to_string())
        .collect();
    let (pb, pa) = (
        slugs.iter().position(|s| s == "b-guide").unwrap(),
        slugs.iter().position(|s| s == "a-guide").unwrap(),
    );
    assert!(pb < pa);
}
