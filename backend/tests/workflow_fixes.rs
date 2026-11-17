//! Regressions for workflow fixes: ride state guards, duplicate event rides, disabled
//! drivers, settlement close and event cancel/finish.
mod common;
use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

async fn book(app: &axum::Router, cookie: &str) -> i64 {
    let r = call(
        app,
        Method::POST,
        "/api/rides",
        Some(cookie),
        Some(json!({"pickup_at": tomorrow("09:00"), "pickup_area_id": 2,
                    "pickup_address": "x", "destination": "教会", "seats": 1})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    r.json["id"].as_i64().unwrap()
}

async fn status_of(st: &grace_ride::state::AppState, id: i64) -> String {
    sqlx::query_scalar("SELECT status FROM rides WHERE id=$1")
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn unable_to_join_is_refused_after_pickup() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let id = book(&app, &passenger).await;
    sqlx::query("UPDATE rides SET status='in_progress', driver_id=4, picked_up_at=$1 WHERE id=$2")
        .bind(grace_ride::util::now())
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&passenger),
        Some(json!({"status": "unable_to_join"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    assert_eq!(status_of(&st, id).await, "in_progress");
}

#[tokio::test]
async fn confirm_by_replaced_driver_is_refused() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let old_driver = login(&st, "driver@example.org").await; // id 4
    let id = book(&app, &passenger).await;
    // Staff handed the ride to driver 5 after driver 4's page loaded.
    sqlx::query("UPDATE rides SET status='pending_driver_confirm', driver_id=5 WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/confirm-assignment"),
        Some(&old_driver),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(status_of(&st, id).await, "pending_driver_confirm");
}

#[tokio::test]
async fn disabling_a_driver_releases_unstarted_rides() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let id = book(&app, &passenger).await;
    sqlx::query("UPDATE rides SET status='confirmed', driver_id=4 WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    let roles: String = sqlx::query_scalar("SELECT roles FROM users WHERE id=4")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/users/4",
        Some(&admin),
        Some(json!({"roles": roles.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                    "status": "disabled"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let (status, driver, attention): (String, Option<i64>, i64) =
        sqlx::query_as("SELECT status, driver_id, needs_attention FROM rides WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!((status.as_str(), driver, attention), ("pending_driver", None, 1));
}
