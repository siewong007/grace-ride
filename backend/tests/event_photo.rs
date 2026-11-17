//! Event cover photos: only the saver's own image is accepted, everyone who can see the
//! event can open the photo, and others cannot.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::json;

async fn id_of(st: &AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn only_in(st: &AppState, user: i64, groups: &[i64]) {
    sqlx::query("DELETE FROM group_members WHERE user_id=$1")
        .bind(user)
        .execute(&st.db)
        .await
        .unwrap();
    for g in groups {
        sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1,$2)")
            .bind(g)
            .bind(user)
            .execute(&st.db)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn event_photo_ownership_and_visibility() {
    let (app, st) = setup().await;
    let youth: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    only_in(&st, id_of(&st, "member@example.org").await, &[youth]).await;
    only_in(&st, id_of(&st, "amina@example.org").await, &[]).await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let amina = login(&st, "amina@example.org").await;

    let mine = upload_png(&app, &admin).await;
    let theirs = upload_png(&app, &member).await;
    let body = |photo: serde_json::Value| json!({"title": "青年营", "kind": "other", "starts_at": tomorrow("19:00"), "location": "教会", "group_ids": [youth], "photo_file_id": photo});

    // Someone else's upload is refused.
    let r = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(body(json!(theirs))),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "INVALID_PHOTO");

    let r = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(body(json!(mine))),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let ev = r.json["id"].as_i64().unwrap();
    let r = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.json["photo_file_id"], mine);

    // The audience can open it; someone who can't see the event can't.
    let path = format!("/api/files/{mine}");
    let r = call(&app, Method::GET, &path, Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(&app, Method::GET, &path, Some(&amina), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Saving with the current photo keeps it; null removes it.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/events/{ev}"),
        Some(&admin),
        Some(body(json!(mine))),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/events/{ev}"),
        Some(&admin),
        Some(body(json!(null))),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let r = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev}"),
        Some(&member),
        None,
    )
    .await;
    assert!(r.json["photo_file_id"].is_null());
    let r = call(&app, Method::GET, &path, Some(&member), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}
