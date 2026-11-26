mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn post_event_driver_reimbursement_and_passenger_supper_flow() {
    let (app, st) = setup().await;
    let organiser = login(&st, "admin@example.org").await;
    let passenger = login(&st, "member@example.org").await;
    let _driver = login(&st, "driver@example.org").await;
    let finance = login(&st, "finance@example.org").await;

    let (driver_id,): (i64,) =
        sqlx::query_as("SELECT id FROM users WHERE email='driver@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    let (passenger_id,): (i64,) =
        sqlx::query_as("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    // 1. Create a mileage-claimable event in area 1 (e.g. Subang Jaya)
    let root = login(&st, "superadmin@example.org").await;
    let at = "2026-12-10T19:00:00";
    let res = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&root),
        Some(json!({
            "title": "Youth Supper Night",
            "kind": "other",
            "starts_at": at,
            "location": "Church Hall",
            "capacity": 30,
            "group_ids": [1],
            "area_id": 1,
            "mileage_claimable": true
        })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let event_id = res.json["id"].as_i64().unwrap();

    // 2. Insert area fare between Area 1 and Area 2 (RM 15.00 = 1500 cents)
    sqlx::query(
        "INSERT INTO area_fares (area_a_id, area_b_id, amount_cents, updated_at)
         VALUES (1, 2, 1500, app_now())
         ON CONFLICT (area_a_id, area_b_id) DO UPDATE SET amount_cents = 1500",
    )
    .execute(&st.db)
    .await
    .unwrap();

    // 3. Create an outbound ride and a return ride for the passenger with driver assigned
    let (outbound_ride_id,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, leg, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 2, 'Home Address', 'Church Hall', 'outbound', 'completed', app_now(), app_now())
         RETURNING id",
    )
    .bind(passenger_id)
    .bind(driver_id)
    .bind(event_id)
    .bind(at)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let (return_ride_id,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, leg, status, created_at, updated_at)
         VALUES ($1, $2, $3, '2026-12-10T21:30:00', 1, 'Church Hall', 'Home Address', 'return', 'confirmed', app_now(), app_now())
         RETURNING id",
    )
    .bind(passenger_id)
    .bind(driver_id)
    .bind(event_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // 4. Passenger tests supper choice endpoint
    // a) Outbound ride cannot have supper choice
    let res_outbound = call(
        &app,
        Method::POST,
        &format!("/api/rides/{outbound_ride_id}/supper-choice"),
        Some(&passenger),
        Some(json!({ "choice": "supper" })),
    )
    .await;
    assert_eq!(res_outbound.status, StatusCode::BAD_REQUEST);

    // b) Return ride sets supper choice to 'supper'
    let res_supper = call(
        &app,
        Method::POST,
        &format!("/api/rides/{return_ride_id}/supper-choice"),
        Some(&passenger),
        Some(json!({ "choice": "supper" })),
    )
    .await;
    assert_eq!(res_supper.status, StatusCode::OK, "{}", res_supper.json);
    assert_eq!(res_supper.json["supper_choice"], "supper");

    // Verify ride detail has supper_choice: 'supper'
    let res_ride = call(
        &app,
        Method::GET,
        &format!("/api/rides/{return_ride_id}"),
        Some(&passenger),
        None,
    )
    .await;
    assert_eq!(res_ride.status, StatusCode::OK);
    assert_eq!(res_ride.json["supper_choice"], "supper");

    // 5. Leader queries post-event drivers
    let res_drivers = call(
        &app,
        Method::GET,
        &format!("/api/events/{event_id}/post-event-drivers"),
        Some(&organiser),
        None,
    )
    .await;
    assert_eq!(res_drivers.status, StatusCode::OK, "{}", res_drivers.json);
    let drivers = res_drivers.json["drivers"].as_array().unwrap();
    assert!(!drivers.is_empty());

    let d_item = drivers.iter().find(|d| d["id"] == driver_id).unwrap();
    assert_eq!(d_item["passenger_stats"]["supper"], 1);
    assert_eq!(d_item["suggested_trip_type"], "supper");
    assert_eq!(d_item["already_claimed"], false);

    // 6. Leader submits batch reimbursement for driver
    let res_batch = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/post-event-reimbursements"),
        Some(&organiser),
        Some(json!({
            "supper_area_id": 2,
            "supper_note": "Restoran SS2",
            "claims": [{
                "driver_id": driver_id,
                "trip_type": "supper",
                "destination_area_id": 2,
                "amount_cents": 1500,
                "note": "宵夜接送"
            }]
        })),
    )
    .await;
    assert_eq!(res_batch.status, StatusCode::OK, "{}", res_batch.json);
    assert_eq!(res_batch.json["created_count"], 1);
    assert_eq!(res_batch.json["total_cents"], 1500);

    // 7. Duplicate claim should be prevented
    let res_dup = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/post-event-reimbursements"),
        Some(&organiser),
        Some(json!({
            "claims": [{
                "driver_id": driver_id,
                "trip_type": "supper",
                "destination_area_id": 2,
                "amount_cents": 1500
            }]
        })),
    )
    .await;
    assert_eq!(res_dup.status, StatusCode::CONFLICT);

    // 8. Finance can see the approved reimbursement in payout queue
    let res_payout = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=payout",
        Some(&finance),
        None,
    )
    .await;
    assert_eq!(res_payout.status, StatusCode::OK, "{}", res_payout.json);
    let items_array = res_payout
        .json
        .as_array()
        .cloned()
        .or_else(|| res_payout.json["items"].as_array().cloned())
        .expect("items array");
    let our_claim = items_array
        .iter()
        .find(|i| i["user"]["id"] == driver_id)
        .unwrap();
    assert_eq!(our_claim["status"], "approved");
    assert_eq!(our_claim["amount_cents"], 1500);
    assert_eq!(our_claim["receipt_file_id"], serde_json::Value::Null);
}
