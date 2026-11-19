//! Integration tests verifying:
//! Grab reimbursement requires an event and claimant must be registered to the event.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn grab_reimbursement_requires_event_and_signup() {
    let (app, st) = setup().await;
    let claimant = login(&st, "member@example.org").await;
    let other_user = login(&st, "sophie@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let receipt = upload_png(&app, &claimant).await;

    // 1. Calling without an event_id or ride using call_exact is rejected with EVENT_REQUIRED_FOR_REIMBURSEMENT
    let res = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "amount_cents": 2500,
            "description": "Grab with no event",
            "receipt_file_id": receipt,
            "payee_name": "Test Payee",
            "payee_phone": "012-3456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{}", res.json);
    assert_eq!(res.json["code"], "EVENT_REQUIRED_FOR_REIMBURSEMENT");

    // Create an event as admin
    let event_res = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Sunday Worship Service",
            "kind": "other",
            "starts_at": tomorrow("09:30"),
            "location": "Sanctuary",
            "capacity": 50,
            "group_ids": [1]
        })),
    )
    .await;
    assert_eq!(event_res.status, StatusCode::OK, "{}", event_res.json);
    let event_id = event_res.json["id"].as_i64().unwrap();

    // 2. Calling with event_id but claimant is NOT registered to the event -> EVENT_REIMBURSEMENT_NEEDS_SIGNUP
    let res = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "event_id": event_id,
            "amount_cents": 2500,
            "description": "Grab before registering",
            "receipt_file_id": receipt,
            "payee_name": "Test Payee",
            "payee_phone": "012-3456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{}", res.json);
    assert_eq!(res.json["code"], "EVENT_REIMBURSEMENT_NEEDS_SIGNUP");

    // 3. Other user signs up, but claimant is still not signed up -> claimant still rejected
    let signup_other = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/signup"),
        Some(&other_user),
        Some(json!({
            "need_ride": false,
            "offer_drive": false
        })),
    )
    .await;
    assert_eq!(signup_other.status, StatusCode::OK, "{}", signup_other.json);

    let res = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "event_id": event_id,
            "amount_cents": 2500,
            "description": "Grab still not registered",
            "receipt_file_id": receipt,
            "payee_name": "Test Payee",
            "payee_phone": "012-3456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{}", res.json);
    assert_eq!(res.json["code"], "EVENT_REIMBURSEMENT_NEEDS_SIGNUP");

    // 4. Claimant signs up for the event -> status is 'going'
    let signup_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/signup"),
        Some(&claimant),
        Some(json!({
            "need_ride": false,
            "offer_drive": false,
            "as_group_id": 2
        })),
    )
    .await;
    assert_eq!(signup_res.status, StatusCode::OK, "{}", signup_res.json);

    // Verify GET /events?scope=registered shows this event for claimant
    let reg_events = call(
        &app,
        Method::GET,
        "/api/events?scope=registered",
        Some(&claimant),
        None,
    )
    .await;
    assert_eq!(reg_events.status, StatusCode::OK, "{}", reg_events.json);
    let items = reg_events.json.as_array().unwrap();
    assert!(items.iter().any(|e| e["id"] == event_id));

    // 5. Claimant submits Grab reimbursement with event_id -> succeeds!
    let res = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "event_id": event_id,
            "amount_cents": 2850,
            "description": "Grab to Sunday Service",
            "receipt_file_id": receipt,
            "payee_name": "Test Payee",
            "payee_phone": "012-3456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let reimb_id = res.json["id"].as_i64().unwrap();

    // Verify detail contains event_id and event_title
    let detail = call(
        &app,
        Method::GET,
        &format!("/api/reimbursements/{reimb_id}"),
        Some(&claimant),
        None,
    )
    .await;
    assert_eq!(detail.status, StatusCode::OK, "{}", detail.json);
    assert_eq!(detail.json["event_id"], event_id);
    assert_eq!(detail.json["event_title"], "Sunday Worship Service");

    // 6. Return and Resubmit preserves / validates event
    let ret_res = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{reimb_id}/review"),
        Some(&transport),
        Some(json!({
            "action": "return",
            "note": "Please update payee phone"
        })),
    )
    .await;
    assert_eq!(ret_res.status, StatusCode::OK, "{}", ret_res.json);

    // Resubmitting without event or with invalid event is rejected
    let resub_bad = call_exact(
        &app,
        Method::PUT,
        &format!("/api/reimbursements/{reimb_id}"),
        Some(&claimant),
        Some(json!({
            "event_id": null,
            "amount_cents": 2850,
            "description": "Grab updated",
            "receipt_file_id": receipt,
            "payee_name": "Test Payee",
            "payee_phone": "012-9999999"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(
        resub_bad.status,
        StatusCode::BAD_REQUEST,
        "{}",
        resub_bad.json
    );

    // Resubmitting with valid event succeeds
    let resub_ok = call_exact(
        &app,
        Method::PUT,
        &format!("/api/reimbursements/{reimb_id}"),
        Some(&claimant),
        Some(json!({
            "event_id": event_id,
            "amount_cents": 2850,
            "description": "Grab updated",
            "receipt_file_id": receipt,
            "payee_name": "Test Payee",
            "payee_phone": "012-9999999"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(resub_ok.status, StatusCode::OK, "{}", resub_ok.json);

    // 7. Test withdrawal: create second event, sign up, then withdraw -> status becomes 'cancelled', Grab claim fails
    let event2_res = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Prayer Meeting",
            "kind": "other",
            "starts_at": tomorrow("19:30"),
            "location": "Prayer Room",
            "capacity": 50,
            "group_ids": [1]
        })),
    )
    .await;
    let event2_id = event2_res.json["id"].as_i64().unwrap();

    call(
        &app,
        Method::POST,
        &format!("/api/events/{event2_id}/signup"),
        Some(&claimant),
        Some(json!({
            "need_ride": false,
            "offer_drive": false,
            "as_group_id": 2
        })),
    )
    .await;

    // Withdraw from event2
    let withdraw_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{event2_id}/withdraw"),
        Some(&claimant),
        None,
    )
    .await;
    assert_eq!(withdraw_res.status, StatusCode::OK, "{}", withdraw_res.json);

    let receipt2 = upload_png(&app, &claimant).await;
    let res = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "event_id": event2_id,
            "amount_cents": 1500,
            "description": "Grab to cancelled event signup",
            "receipt_file_id": receipt2,
            "payee_name": "Test Payee",
            "payee_phone": "012-3456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{}", res.json);
    assert_eq!(res.json["code"], "EVENT_REIMBURSEMENT_NEEDS_SIGNUP");

    // 8. Test with ride linkage:
    // Create Event 3, sign up, book a ride
    let event3_res = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Fellowship Night",
            "kind": "other",
            "starts_at": tomorrow("20:00"),
            "location": "Fellowship Hall",
            "capacity": 50,
            "group_ids": [1]
        })),
    )
    .await;
    let event3_id = event3_res.json["id"].as_i64().unwrap();

    call(
        &app,
        Method::POST,
        &format!("/api/events/{event3_id}/signup"),
        Some(&claimant),
        Some(json!({
            "need_ride": true,
            "offer_drive": false,
            "as_group_id": 2
        })),
    )
    .await;

    let book_res = call_exact(
        &app,
        Method::POST,
        "/api/rides",
        Some(&claimant),
        Some(json!({
            "event_id": event3_id,
            "pickup_at": tomorrow("19:00"),
            "pickup_area_id": 1,
            "pickup_address": "Home",
            "destination": "Fellowship Hall",
            "seats": 1
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(book_res.status, StatusCode::OK, "{}", book_res.json);
    let ride_id = book_res.json["id"].as_i64().unwrap();

    // Mismatched event_id vs ride's event_id is rejected
    let receipt3 = upload_png(&app, &claimant).await;
    let mismatch = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "event_id": event_id, // Event 1 instead of Event 3
            "ride_id": ride_id,
            "amount_cents": 1800,
            "description": "Mismatched event and ride",
            "receipt_file_id": receipt3,
            "payee_name": "Test Payee",
            "payee_phone": "012-3456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(
        mismatch.status,
        StatusCode::BAD_REQUEST,
        "{}",
        mismatch.json
    );
    assert_eq!(mismatch.json["code"], "INVALID_ACTION");

    // Correct event_id matching ride succeeds
    let ok_ride_claim = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&claimant),
        Some(json!({
            "event_id": event3_id,
            "ride_id": ride_id,
            "amount_cents": 1800,
            "description": "Grab for ride to Fellowship",
            "receipt_file_id": receipt3,
            "payee_name": "Test Payee",
            "payee_phone": "012-3456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(
        ok_ride_claim.status,
        StatusCode::OK,
        "{}",
        ok_ride_claim.json
    );
    let ok_reimb_id = ok_ride_claim.json["id"].as_i64().unwrap();

    let detail = call(
        &app,
        Method::GET,
        &format!("/api/reimbursements/{ok_reimb_id}"),
        Some(&claimant),
        None,
    )
    .await;
    assert_eq!(detail.status, StatusCode::OK, "{}", detail.json);
    assert_eq!(detail.json["event_id"], event3_id);
    assert_eq!(detail.json["event_title"], "Fellowship Night");
    assert_eq!(detail.json["ride_id"], ride_id);
}
