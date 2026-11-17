//! A failed event edit leaves the event exactly as it was: no half-saved venue, and the
//! previous audience keeps access.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn event_edit_with_deleted_group_changes_nothing() {
    let (app, st) = setup().await;
    let youth: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
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
        .bind(youth)
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let doomed: i64 =
        sqlx::query_scalar("INSERT INTO groups (name) VALUES ('临时小组') RETURNING id")
            .fetch_one(&st.db)
            .await
            .unwrap();
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let starts = tomorrow("19:00");

    let r = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({"title": "青年营", "kind": "other", "starts_at": starts, "location": "教会", "group_ids": [youth]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let ev = r.json["id"].as_i64().unwrap();

    // The group is deleted after the edit form opened.
    sqlx::query("DELETE FROM groups WHERE id=$1")
        .bind(doomed)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/events/{ev}"),
        Some(&admin),
        Some(json!({"title": "青年营", "kind": "other", "starts_at": starts, "location": "新场地", "group_ids": [youth, doomed]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "SELECT_VISIBLE_GROUP");

    let r = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["location"], "教会");
    assert_eq!(r.json["group_ids"], json!([youth]));

    // A creation naming a missing group leaves no orphaned event behind.
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({"title": "幽灵", "kind": "other", "starts_at": starts, "location": "教会", "group_ids": [doomed]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM events")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(before, after);
}
