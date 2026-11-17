mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn event_attendance_module_lifecycle_and_checkin() {
    let (app, st) = setup().await;
    let organiser = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;

    let (member_id,): (i64,) =
        sqlx::query_as("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    // 1. Create an event with attendance_enabled: true (default)
    let at = "2026-12-01T18:00:00";
    let res = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&organiser),
        Some(json!({
            "title": "Badminton Fellowship",
            "kind": "badminton",
            "starts_at": at,
            "location": "Community Sports Center",
            "capacity": 20,
            "group_ids": [1],
            "attendance_enabled": true
        })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let event_id = res.json["id"].as_i64().unwrap();

    // Verify event detail has attendance_enabled: true and checkin_code for organiser
    let detail_org = call(
        &app,
        Method::GET,
        &format!("/api/events/{event_id}"),
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(detail_org.status, StatusCode::OK);
    assert_eq!(detail_org.json["attendance_enabled"], true);
    let code = detail_org.json["checkin_code"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(!code.is_empty());

    // Member viewing event detail sees attendance_enabled: true, but not checkin_code
    let detail_mem = call(
        &app,
        Method::GET,
        &format!("/api/events/{event_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(detail_mem.status, StatusCode::OK);
    assert_eq!(detail_mem.json["attendance_enabled"], true);
    assert!(detail_mem.json["checkin_code"].is_null());

    // 2. Member can check in via QR code without previously signing up (walk-in participation without requiring a ride)
    let checkin_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin"),
        Some(&member),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(checkin_res.status, StatusCode::OK, "{}", checkin_res.json);
    assert_eq!(checkin_res.json["ok"], true);
    assert_eq!(checkin_res.json["already_attended"], false);

    // Verify member is now participating (going) and attended, with need_ride = false
    let (status, attended, need_ride): (String, Option<i64>, i64) = sqlx::query_as(
        "SELECT status, attended, need_ride FROM event_signups WHERE event_id = $1 AND user_id = $2",
    )
    .bind(event_id)
    .bind(member_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(status, "going");
    assert_eq!(attended, Some(1));
    assert_eq!(need_ride, 0); // No ride required!

    // 3. Repeated check-in reports already_attended: true
    let repeat = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin"),
        Some(&member),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(repeat.status, StatusCode::OK);
    assert_eq!(repeat.json["already_attended"], true);

    // 4. Invalid check-in code is rejected
    let bad_code = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin"),
        Some(&member),
        Some(json!({ "code": "WRONG_CODE" })),
    )
    .await;
    assert_eq!(bad_code.status, StatusCode::BAD_REQUEST);

    // 5. Organiser can reset check-in code
    let reset_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin-code/reset"),
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(reset_res.status, StatusCode::OK);
    let new_code = reset_res.json["checkin_code"].as_str().unwrap();
    assert_ne!(new_code, code);

    // Member cannot reset check-in code
    let mem_reset = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin-code/reset"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(mem_reset.status, StatusCode::FORBIDDEN);

    // 6. Disable attendance module on the event
    let update_res = call(
        &app,
        Method::PUT,
        &format!("/api/events/{event_id}"),
        Some(&organiser),
        Some(json!({
            "title": "Badminton Fellowship",
            "kind": "badminton",
            "starts_at": at,
            "location": "Community Sports Center",
            "capacity": 20,
            "group_ids": [1],
            "attendance_enabled": false
        })),
    )
    .await;
    assert_eq!(update_res.status, StatusCode::OK);

    // Checkin on event with disabled attendance module is rejected
    let disabled_checkin = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin"),
        Some(&member),
        Some(json!({ "code": new_code })),
    )
    .await;
    assert_eq!(disabled_checkin.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn attendance_checkin_is_for_members_only() {
    let (app, st) = setup().await;
    let organiser = login(&st, "admin@example.org").await;

    // Create event with attendance enabled
    let res = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&organiser),
        Some(json!({
            "title": "Members Only Event",
            "kind": "walk",
            "starts_at": "2026-12-05T09:00:00",
            "location": "Park Trail",
            "capacity": 30,
            "group_ids": [1],
            "attendance_enabled": true
        })),
    )
    .await;
    let event_id = res.json["id"].as_i64().unwrap();

    let detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{event_id}"),
        Some(&organiser),
        None,
    )
    .await;
    let code = detail.json["checkin_code"].as_str().unwrap();

    // 1. Unauthenticated request gets 401 Unauthorized
    let unauth = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin"),
        None,
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(unauth.status, StatusCode::UNAUTHORIZED);

    // 2. Pending user (not active member) gets 403 Forbidden (members only)
    let (pete_id,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, status, webauthn_uuid) VALUES ('Pending Pete', 'pete@example.org', 'pending', 'uuid-pete') RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let pete_token = "pete_token_1234567890123456789012345678901234567";
    assert_eq!(pete_token.len(), 48);
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,0,$3,$4)",
    )
    .bind(grace_ride::auth::hash_token(pete_token))
    .bind(pete_id)
    .bind(grace_ride::util::in_hours(24))
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    let pending_cookie = format!("gr_session={pete_token}");

    let pending_checkin = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/checkin"),
        Some(&pending_cookie),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(pending_checkin.status, StatusCode::FORBIDDEN);
}
