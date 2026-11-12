//! Transport planning board: arrange all of an event's ride requests at once.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::{json, Value};

async fn id_of(st: &AppState, email: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
        .0
}

struct Fixture {
    event: i64,
    rides: Vec<i64>,
    driver: i64,
    driver2: i64,
}

/// One event with four open requests; `driver` carpools with 3 seats, `driver2` takes one booking.
async fn fixture(st: &AppState) -> Fixture {
    let driver = id_of(st, "driver@example.org").await;
    let driver2 = id_of(st, "driver2@example.org").await;
    let staff = id_of(st, "transport@example.org").await;
    sqlx::query("UPDATE users SET carpool_enabled=1, spare_seats=3 WHERE id=$1")
        .bind(driver)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET carpool_enabled=0, spare_seats=1 WHERE id=$1")
        .bind(driver2)
        .execute(&st.db)
        .await
        .unwrap();
    let starts = tomorrow("19:30");
    let (event,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, starts_at, location, created_by) VALUES ('Plan night', $1, 'Church hall', $2) RETURNING id",
    )
    .bind(&starts)
    .bind(staff)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let (area,): (i64,) = sqlx::query_as("SELECT id FROM areas ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let passengers: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM users WHERE status='active' AND roles NOT LIKE '%driver%' ORDER BY id LIMIT 4",
    )
    .fetch_all(&st.db)
    .await
    .unwrap();
    let mut rides = Vec::new();
    for (p,) in passengers {
        let (r,): (i64,) = sqlx::query_as(
            "INSERT INTO rides (passenger_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats)
             VALUES ($1, $2, $3, $4, '1 Jalan Test', 'Church hall', 1) RETURNING id",
        )
        .bind(p)
        .bind(event)
        .bind(tomorrow("19:00"))
        .bind(area)
        .fetch_one(&st.db)
        .await
        .unwrap();
        rides.push(r);
    }
    Fixture {
        event,
        rides,
        driver,
        driver2,
    }
}

async fn ride_state(st: &AppState, id: i64) -> (String, Option<i64>) {
    sqlx::query_as("SELECT status, driver_id FROM rides WHERE id=$1")
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

fn change(ride: i64, from: Option<i64>, to: Option<i64>) -> Value {
    json!({ "ride_id": ride, "from_driver_id": from, "driver_id": to })
}

#[tokio::test]
async fn board_lists_requests_drivers_and_last_driver() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;
    // The first passenger rode with `driver` before this event.
    let (p0,): (i64,) = sqlx::query_as("SELECT passenger_id FROM rides WHERE id=$1")
        .bind(f.rides[0])
        .fetch_one(&st.db)
        .await
        .unwrap();
    let (area,): (i64,) = sqlx::query_as("SELECT id FROM areas ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, destination, status)
         VALUES ($1, $2, '2026-01-04T09:00', $3, 'Church', 'completed')",
    )
    .bind(p0)
    .bind(f.driver)
    .bind(area)
    .execute(&st.db)
    .await
    .unwrap();

    let staff = login(&st, "transport@example.org").await;
    let r = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&staff),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["rides"].as_array().unwrap().len(), 4);
    let d = r.json["drivers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == f.driver)
        .expect("driver on the board");
    assert_eq!(d["cap"], 3);
    assert_eq!(d["carpool"], true);
    let first = r.json["rides"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == f.rides[0])
        .unwrap();
    assert_eq!(first["last"]["driver_id"], f.driver);
    assert!(first["last"]["times"].as_i64().unwrap() >= 1);

    // Members can't see the board.
    let member = login(&st, "member@example.org").await;
    let r = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn batch_assigns_for_driver_confirmation_with_one_summary() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;
    let staff = login(&st, "transport@example.org").await;
    let before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
        .bind(f.driver)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let body = json!({ "changes": [
        change(f.rides[0], None, Some(f.driver)),
        change(f.rides[1], None, Some(f.driver)),
        change(f.rides[2], None, Some(f.driver)),
        change(f.rides[3], None, Some(f.driver2)),
    ]});
    let r = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&staff),
        Some(body),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["changed"], 4);
    for id in &f.rides[..3] {
        assert_eq!(
            ride_state(&st, *id).await,
            ("pending_driver_confirm".into(), Some(f.driver))
        );
    }
    let after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
        .bind(f.driver)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(after.0 - before.0, 1, "one summary for three passengers");

    // The driver confirms through the existing flow.
    let drv = login(&st, "driver@example.org").await;
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{}/confirm-assignment", f.rides[0]),
        Some(&drv),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(ride_state(&st, f.rides[0]).await.0, "confirmed");

    // Taking a ride back off a driver returns it to the open pool.
    let r = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&staff),
        Some(json!({ "changes": [change(f.rides[0], Some(f.driver), None)] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(
        ride_state(&st, f.rides[0]).await,
        ("pending_driver".into(), None)
    );
}

#[tokio::test]
async fn seat_limits_and_stale_boards_apply_nothing() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;
    let staff = login(&st, "transport@example.org").await;
    let url = format!("/api/admin/events/{}/plan", f.event);

    // Four passengers in a three-seat car.
    let all: Vec<Value> = f
        .rides
        .iter()
        .map(|r| change(*r, None, Some(f.driver)))
        .collect();
    let r = call(
        &app,
        Method::POST,
        &url,
        Some(&staff),
        Some(json!({ "changes": all })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    // Two bookings for a driver who doesn't carpool.
    let two = json!({ "changes": [
        change(f.rides[0], None, Some(f.driver2)),
        change(f.rides[1], None, Some(f.driver2)),
    ]});
    let r = call(&app, Method::POST, &url, Some(&staff), Some(two)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);

    // Someone else assigned ride 1 after this board loaded: the whole batch is refused.
    sqlx::query("UPDATE rides SET driver_id=$1, status='pending_passenger' WHERE id=$2")
        .bind(f.driver2)
        .bind(f.rides[1])
        .execute(&st.db)
        .await
        .unwrap();
    let stale = json!({ "changes": [
        change(f.rides[0], None, Some(f.driver)),
        change(f.rides[1], None, Some(f.driver)),
    ]});
    let r = call(&app, Method::POST, &url, Some(&staff), Some(stale)).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    assert_eq!(
        ride_state(&st, f.rides[0]).await,
        ("pending_driver".into(), None)
    );
}
