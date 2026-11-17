mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

/// An organiser must not be able to bill a member the event is not for, either by
/// switching attendance tracking off or by recording the member as a walk-in.
#[tokio::test]
async fn bills_only_reach_people_connected_to_the_event() {
    let (app, st) = setup().await;
    let organiser = login(&st, "admin@example.org").await;

    let youth: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let outsider_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email='amina@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    sqlx::query("DELETE FROM group_members WHERE user_id=$1")
        .bind(outsider_id)
        .execute(&st.db)
        .await
        .unwrap();

    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&organiser),
        Some(json!({
            "title": "Youth Outing",
            "kind": "other",
            "starts_at": "2026-12-10T19:00:00",
            "location": "Youth Hall",
            "capacity": 25,
            "group_ids": [youth],
            "attendance_enabled": true
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let event_id = ev.json["id"].as_i64().unwrap();

    // Walk-in for a member outside the event's groups is refused while attendance is on.
    let walk = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/attendance"),
        Some(&organiser),
        Some(json!({"user_id": outsider_id, "attended": true})),
    )
    .await;
    assert_eq!(walk.status, StatusCode::FORBIDDEN, "{}", walk.json);
    let signups: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND user_id=$2")
            .bind(event_id)
            .bind(outsider_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(signups, 0);

    // Turning attendance off must not open the bill to anyone.
    sqlx::query("UPDATE events SET attendance_enabled=0 WHERE id=$1")
        .bind(event_id)
        .execute(&st.db)
        .await
        .unwrap();
    let bill = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/settlements"),
        Some(&organiser),
        Some(json!({
            "title": "Lure",
            "items": [{"label": "snacks", "amount_cents": 5000}],
            "split_mode": "equal",
            "participants": [{"user_id": outsider_id, "amount_cents": 5000}],
            "payee_name": "Organiser",
            "payee_account": "111 222 3333",
            "due_date": null
        })),
    )
    .await;
    assert_eq!(bill.status, StatusCode::OK, "{}", bill.json);
    let sid = bill.json["id"].as_i64().unwrap();
    let publish = call(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(publish.status, StatusCode::BAD_REQUEST, "{}", publish.json);
    let status: String = sqlx::query_scalar("SELECT status FROM settlements WHERE id=$1")
        .bind(sid)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(status, "draft");
}
