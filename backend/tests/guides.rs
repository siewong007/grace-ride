//! Admin workflow guide API: staff-only reads, editor-only writes.

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
use serde_json::Value;
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

/// Insert a guide row straight into the DB (test fixture).
async fn seed_guide(st: &AppState, slug: &str, published: bool, steps: &str) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO guide_workflows (slug, title_en, title_zh, steps, published, updated_at)
         VALUES ($1,'Test guide','测试指南',$2::jsonb,$3,$4) RETURNING id",
    )
    .bind(slug)
    .bind(steps)
    .bind(published)
    .bind(grace_ride::util::now())
    .fetch_one(&st.db)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn guides_are_staff_only() {
    let (app, st) = setup().await;
    seed_guide(
        &st,
        "published-guide",
        true,
        r#"[{"title_en":"Step","title_zh":"步骤","body_en":"","body_zh":"","link":"/admin"}]"#,
    )
    .await;
    let member = login(&st, "member@example.org").await;
    let r = call(&app, Method::GET, "/api/help/guides", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides/published-guide",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // transport (admin_help) and finance (admin_reimb_pay) both hold staff pages.
    for email in ["transport@example.org", "finance@example.org"] {
        let cookie = login(&st, email).await;
        let r = call(&app, Method::GET, "/api/help/guides", Some(&cookie), None).await;
        assert_eq!(r.status, StatusCode::OK, "{email} should list guides");
        assert_eq!(r.json.as_array().unwrap().len(), 1);
        assert_eq!(r.json[0]["slug"], "published-guide");
        assert_eq!(r.json[0]["step_count"], 1);
        let r = call(
            &app,
            Method::GET,
            "/api/help/guides/published-guide",
            Some(&cookie),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK);
        assert_eq!(r.json["steps"][0]["link"], "/admin");
    }
}

#[tokio::test]
async fn drafts_hidden_from_non_editors() {
    let (app, st) = setup().await;
    seed_guide(&st, "draft-guide", false, "[]").await;
    seed_guide(&st, "live-guide", true, "[]").await;

    // finance holds staff pages but not admin_help update → no drafts, even with the flag.
    let finance = login(&st, "finance@example.org").await;
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides?include_drafts=true",
        Some(&finance),
        None,
    )
    .await;
    assert_eq!(r.json.as_array().unwrap().len(), 1);
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides/draft-guide",
        Some(&finance),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // transport (admin_help update) sees drafts with the flag; detail loads.
    let transport = login(&st, "transport@example.org").await;
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides?include_drafts=true",
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(r.json.as_array().unwrap().len(), 2);
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides/draft-guide",
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["published"], false);
}
