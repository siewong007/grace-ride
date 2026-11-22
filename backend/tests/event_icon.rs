//! Event custom activity icon CRUD, duplication preservation, and validation.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn event_icon_create_update_duplicate_reset_and_validate() {
    let (app, st) = setup().await;
    let youth: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;

    let body = |icon: serde_json::Value| {
        json!({
            "title": "爱宴聚餐",
            "kind": "other",
            "icon": icon,
            "starts_at": tomorrow("19:00"),
            "location": "教会副堂",
            "group_ids": [youth]
        })
    };

    // Create with preset Lucide icon key.
    let r = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(body(json!("utensils"))),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let ev = r.json["id"].as_i64().unwrap();

    // Returned on GET /events and GET /events/{id}.
    let list = call(&app, Method::GET, "/api/events", Some(&member), None).await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.json);
    let found = list
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"].as_i64() == Some(ev))
        .unwrap();
    assert_eq!(found["icon"], "utensils");

    let detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(detail.status, StatusCode::OK, "{}", detail.json);
    assert_eq!(detail.json["icon"], "utensils");

    // Update to custom emoji.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/events/{ev}"),
        Some(&admin),
        Some(body(json!("🍲"))),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(detail.json["icon"], "🍲");

    // Duplicate preserves the icon.
    let dup = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/duplicate"),
        Some(&admin),
        Some(json!({ "starts_at": tomorrow("20:00") })),
    )
    .await;
    assert_eq!(dup.status, StatusCode::OK, "{}", dup.json);
    let dup_id = dup.json["id"].as_i64().unwrap();
    let dup_detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{dup_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(dup_detail.json["icon"], "🍲");

    // Empty string resets icon to null.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/events/{ev}"),
        Some(&admin),
        Some(body(json!(""))),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&member),
        None,
    )
    .await;
    assert!(detail.json["icon"].is_null());

    // Overly long icon (> 32 chars) is rejected with 400.
    let long_icon = "x".repeat(33);
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/events/{ev}"),
        Some(&admin),
        Some(body(json!(long_icon))),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
}
