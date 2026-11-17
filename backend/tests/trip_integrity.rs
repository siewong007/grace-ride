//! Trip identity and the live-driver limit under reassignment and concurrency.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::json;

async fn user_id(st: &AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn confirmed_ride(st: &AppState, passenger: i64, driver: i64, at: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, status)
         VALUES ($1,$2,$3,2,'Somewhere','Test Church',1,'confirmed') RETURNING id",
    )
    .bind(passenger)
    .bind(driver)
    .bind(at)
    .fetch_one(&st.db)
    .await
    .unwrap()
}

async fn trip_of(st: &AppState, ride: i64) -> Option<i64> {
    sqlx::query_scalar("SELECT trip_id FROM rides WHERE id=$1")
        .bind(ride)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

/// Rides A and B share a carpool whose trip id is A's ride id. Moving A to another driver
/// must give A a new trip, not A's id again, or the new driver joins B's chat.
#[tokio::test]
async fn reassigning_a_carpools_first_ride_gives_it_a_distinct_trip() {
    let (app, st) = setup().await;
    let driver = user_id(&st, "driver@example.org").await;
    let driver2 = user_id(&st, "driver2@example.org").await;
    let a_pass = user_id(&st, "member@example.org").await;
    let b_pass = user_id(&st, "amina@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let new_driver = login(&st, "driver2@example.org").await;

    let at = tomorrow("09:00");
    let a = confirmed_ride(&st, a_pass, driver, &at).await;
    let b = confirmed_ride(&st, b_pass, driver, &at).await;
    sqlx::query("UPDATE rides SET trip_id=$1 WHERE id = ANY($2)")
        .bind(a)
        .bind(vec![a, b])
        .execute(&st.db)
        .await
        .unwrap();

    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{a}/reassign"),
        Some(&transport),
        Some(json!({"driver_id": driver2})),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{a}/confirm-assignment"),
        Some(&new_driver),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);

    let new_trip = trip_of(&st, a).await.expect("A joins a trip");
    assert_eq!(trip_of(&st, b).await, Some(a), "B keeps the original car's trip");
    assert_ne!(new_trip, a, "A must not reuse the trip B is still in");

    // A message in B's car stays out of A's chat.
    let original = login(&st, "driver@example.org").await;
    let sent = call(
        &app,
        Method::POST,
        &format!("/api/rides/{b}/messages"),
        Some(&original),
        Some(json!({"body": "for the original car only"})),
    )
    .await;
    assert_eq!(sent.status, StatusCode::OK, "{}", sent.json);
    let leaked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ride_messages WHERE trip_id=$1 AND body='for the original car only'",
    )
    .bind(new_trip)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(leaked, 0);
}

/// With the limit at one, two drivers starting at the same moment: exactly one goes live.
#[tokio::test]
async fn concurrent_trip_starts_respect_the_live_driver_limit() {
    let (app, st) = setup().await;
    sqlx::query(
        "INSERT INTO system_settings (key, value) VALUES ('max_live_drivers', '1')
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .execute(&st.db)
    .await
    .unwrap();
    let d1 = user_id(&st, "driver@example.org").await;
    let d2 = user_id(&st, "driver2@example.org").await;
    let p1 = user_id(&st, "member@example.org").await;
    let p2 = user_id(&st, "amina@example.org").await;
    let c1 = login(&st, "driver@example.org").await;
    let c2 = login(&st, "driver2@example.org").await;

    for round in 0..5 {
        let at = tomorrow(&format!("1{round}:00"));
        let r1 = confirmed_ride(&st, p1, d1, &at).await;
        let r2 = confirmed_ride(&st, p2, d2, &at).await;
        let path1 = format!("/api/rides/{r1}/start");
        let path2 = format!("/api/rides/{r2}/start");
        let (s1, s2) = tokio::join!(
            call(&app, Method::POST, &path1, Some(&c1), None),
            call(&app, Method::POST, &path2, Some(&c2), None),
        );
        let ok = [s1.status, s2.status]
            .iter()
            .filter(|s| **s == StatusCode::OK)
            .count();
        assert_eq!(ok, 1, "round {round}: {} / {}", s1.json, s2.json);
        let live: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress'",
        )
        .fetch_one(&st.db)
        .await
        .unwrap();
        assert_eq!(live, 1, "round {round}");
        sqlx::query("UPDATE rides SET status='completed' WHERE status='in_progress'")
            .execute(&st.db)
            .await
            .unwrap();
    }
}
