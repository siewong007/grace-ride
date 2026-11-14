//! Member and staff workflow guide API: editor-only writes.

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
async fn guides_are_visible_by_audience() {
    let (app, st) = setup().await;
    seed_guide(
        &st,
        "published-guide",
        true,
        r#"[{"title_en":"Step","title_zh":"步骤","body_en":"","body_zh":"","link":"/admin"}]"#,
    )
    .await;
    sqlx::query("UPDATE guide_workflows SET audience='member' WHERE slug='published-guide'")
        .execute(&st.db)
        .await
        .unwrap();
    seed_guide(&st, "staff-guide", true, "[]").await;
    let member = login(&st, "member@example.org").await;
    let r = call(&app, Method::GET, "/api/help/guides", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let guides = r.json.as_array().unwrap();
    assert!(guides.iter().any(|g| g["slug"] == "published-guide"));
    assert!(!guides.iter().any(|g| g["slug"] == "staff-guide"));
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides/published-guide",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["audience"], "member");
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides/staff-guide",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // transport (admin_help) and finance (admin_reimb_pay) both hold staff pages.
    for email in ["transport@example.org", "finance@example.org"] {
        let cookie = login(&st, email).await;
        let r = call(&app, Method::GET, "/api/help/guides", Some(&cookie), None).await;
        assert_eq!(r.status, StatusCode::OK, "{email} should list guides");
        let published = r
            .json
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["slug"] == "published-guide")
            .expect("published-guide should be listed");
        assert_eq!(published["step_count"], 1);
        assert_eq!(published["audience"], "member");
        assert!(r
            .json
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["slug"] == "staff-guide"));
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
        assert_eq!(r.json["audience"], "member");
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
    let slugs: Vec<&str> = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["slug"].as_str().unwrap())
        .collect();
    assert!(slugs.contains(&"live-guide"));
    assert!(!slugs.contains(&"draft-guide"));
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
    let slugs: Vec<&str> = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["slug"].as_str().unwrap())
        .collect();
    assert!(slugs.contains(&"draft-guide"));
    assert!(slugs.contains(&"live-guide"));
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

fn guide_body() -> Value {
    json!({
        "slug": "new-guide",
        "title_en": "New guide",
        "title_zh": "新指南",
        "summary_en": "Summary",
        "summary_zh": "摘要",
        "published": true,
        "steps": [
            {"title_en": "First", "title_zh": "第一步", "body_en": "Do it", "body_zh": "这样做", "link": "/admin/plan"},
            {"title_en": "Second", "title_zh": "第二步", "body_en": "", "body_zh": "", "link": ""}
        ]
    })
}

#[tokio::test]
async fn guide_crud_is_editor_only_and_audited() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let finance = login(&st, "finance@example.org").await; // staff pages, no admin_help update
    let transport = login(&st, "transport@example.org").await; // admin_help update

    // Non-editors cannot write.
    for cookie in [&member, &finance] {
        let r = call(
            &app,
            Method::POST,
            "/api/help/guides",
            Some(cookie),
            Some(guide_body()),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN);
    }

    // Create → round-trip → audit row.
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(guide_body()),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let id = r.json["id"].as_i64().unwrap();
    let denied = call(
        &app,
        Method::PUT,
        &format!("/api/help/guides/{id}"),
        Some(&member),
        Some(guide_body()),
    )
    .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides/new-guide",
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(r.json["steps"][0]["link"], "/admin/plan");
    assert_eq!(r.json["steps"].as_array().unwrap().len(), 2);
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='guide.create' AND entity='guide_workflow' AND entity_id=$1",
    )
    .bind(id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(n, 1);

    // Update.
    let mut edited = guide_body();
    edited["summary_en"] = json!("Edited");
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/help/guides/{id}"),
        Some(&transport),
        Some(edited),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    // Delete → then gone.
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/help/guides/{id}"),
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides/new-guide",
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn guide_validation() {
    let (app, st) = setup().await;
    let transport = login(&st, "transport@example.org").await;
    seed_guide(&st, "taken", true, "[]").await;

    let mut b = guide_body();
    b["title_en"] = json!(""); // no title in either language
    b["title_zh"] = json!("");
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(b),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["code"], "GUIDE_TITLE_REQUIRED");

    let mut b = guide_body();
    b["slug"] = json!("Bad Slug!"); // not url-safe
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(b),
    )
    .await;
    assert_eq!(r.json["code"], "GUIDE_SLUG_REQUIRED");

    let mut b = guide_body();
    b["slug"] = json!("taken"); // unique conflict
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(b),
    )
    .await;
    assert_eq!(r.json["code"], "GUIDE_SLUG_EXISTS");

    let mut b = guide_body();
    b["steps"][0]["link"] = json!("https://evil.example"); // external link
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(b),
    )
    .await;
    assert_eq!(r.json["code"], "GUIDE_LINK_INVALID");

    let mut b = guide_body();
    b["steps"] =
        json!([{"title_en": "", "title_zh": "", "body_en": "", "body_zh": "", "link": ""}]);
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(b),
    )
    .await;
    assert_eq!(r.json["code"], "GUIDE_STEP_INVALID");

    let mut b = guide_body();
    b["audience"] = json!("everyone");
    let r = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(b),
    )
    .await;
    assert_eq!(r.json["code"], "GUIDE_AUDIENCE_INVALID");
}

#[tokio::test]
async fn starter_guides_are_seeded() {
    let (app, st) = setup().await;
    let transport = login(&st, "transport@example.org").await;
    let r = call(
        &app,
        Method::GET,
        "/api/help/guides",
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let slugs: Vec<&str> = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["slug"].as_str().unwrap())
        .collect();
    for expected in [
        "plan-transport",
        "review-reimbursements",
        "send-announcement",
        "manage-members",
        "work-help-desk",
        "audit-settings",
    ] {
        assert!(slugs.contains(&expected), "missing seeded guide {expected}");
    }
    // Every seeded guide has at least one titled step.
    for g in r.json.as_array().unwrap() {
        assert!(
            g["step_count"].as_i64().unwrap() >= 1,
            "guide {} has no steps",
            g["slug"]
        );
    }
}

fn route_matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<_> = pattern.trim_matches('/').split('/').collect();
    let path: Vec<_> = path.trim_matches('/').split('/').collect();
    pattern.len() == path.len()
        && pattern
            .iter()
            .zip(path.iter())
            .all(|(a, b)| a.starts_with(':') || a == b)
}

#[tokio::test]
async fn seeded_guide_coverage_and_links_match_app_routes() {
    let (_app, st) = setup().await;
    let expected = [
        ("plan-transport", "staff"),
        ("review-reimbursements", "staff"),
        ("send-announcement", "staff"),
        ("manage-members", "staff"),
        ("work-help-desk", "staff"),
        ("audit-settings", "staff"),
        ("getting-started", "member"),
        ("request-a-ride", "member"),
        ("track-your-ride", "member"),
        ("volunteer-driving", "member"),
        ("join-events", "member"),
        ("organise-an-event", "member"),
        ("split-activity-costs", "member"),
        ("pay-your-share", "member"),
        ("claim-reimbursement", "member"),
        ("ask-for-help", "member"),
        ("notifications-and-report", "member"),
        ("admin-dashboard", "staff"),
        ("manage-rides", "staff"),
        ("live-map", "staff"),
        ("manage-any-event", "staff"),
        ("pay-reimbursements", "staff"),
        ("view-all-bills", "staff"),
        ("staff-reports", "staff"),
        ("invites", "staff"),
        ("account-recovery", "staff"),
        ("roles-and-permissions", "staff"),
        ("groups-areas-rates", "staff"),
        ("edit-help-guides", "staff"),
    ];
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT slug, audience, steps::text FROM guide_workflows ORDER BY slug")
            .fetch_all(&st.db)
            .await
            .unwrap();
    let routes = [
        "/",
        "/login",
        "/terms",
        "/invite/:token",
        "/rides",
        "/rides/new",
        "/rides/:id",
        "/drive",
        "/events",
        "/events/new",
        "/events/:id",
        "/events/:id/edit",
        "/events/:eventId/settle",
        "/settlements",
        "/settlements/:id",
        "/costs",
        "/reimbursements",
        "/reimbursements/new",
        "/reimbursements/:id",
        "/help",
        "/help/:id",
        "/help/guides/:slug",
        "/notifications",
        "/report",
        "/me",
        "/admin",
        "/admin/rides",
        "/admin/rides/:id",
        "/admin/live",
        "/admin/members",
        "/admin/settings",
        "/admin/reimbursements",
        "/admin/announcements",
        "/admin/audit",
        "/admin/reports",
        "/admin/roles",
        "/admin/events/:id/plan",
    ];
    for (slug, audience) in expected {
        let (_, actual_audience, steps) = rows
            .iter()
            .find(|(actual, _, _)| actual == slug)
            .unwrap_or_else(|| panic!("missing seeded guide {slug}"));
        assert_eq!(actual_audience, audience, "wrong audience for {slug}");
        let steps: Value = serde_json::from_str(steps).unwrap();
        for step in steps.as_array().unwrap() {
            let link = step["link"].as_str().unwrap_or("");
            if link.is_empty() {
                continue;
            }
            assert!(
                !link.starts_with("/admin") || audience == "staff",
                "member guide {slug} links to {link}"
            );
            let path = link.split('?').next().unwrap();
            assert!(
                routes.iter().any(|route| route_matches(route, path)),
                "unknown route {link} in {slug}"
            );
        }
    }
}
