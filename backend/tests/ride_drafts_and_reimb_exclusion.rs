//! Integration tests for single-ride change drafts (shared visibility, single-editor
//! lease lock, Lead-only approval/return) and per-ride `reimbursement_excluded` handling.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::json;

async fn id_of(st: &AppState, email: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
        .0
}

#[tokio::test]
async fn single_ride_draft_lifecycle_locking_and_lead_review() {
    let (app, st) = setup().await;

    let (m1_id,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Ride Member 1', 'rdm1@example.org', '012-400 0011', 'traffic_member', 'active', 'hash', 'uuid-rdm1')
         RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Ride Member 2', 'rdm2@example.org', '012-400 0022', 'traffic_member', 'active', 'hash', 'uuid-rdm2')",
    )
    .execute(&st.db)
    .await
    .unwrap();

    let rdm1 = login(&st, "rdm1@example.org").await;
    let rdm2 = login(&st, "rdm2@example.org").await;
    let tlead = login(&st, "traffichead@example.org").await;
    let tlead_id = id_of(&st, "traffichead@example.org").await;
    let driver_id = id_of(&st, "driver@example.org").await;
    let passenger_id = id_of(&st, "member@example.org").await;
    let admin_id = id_of(&st, "admin@example.org").await;

    let (area_id,): (i64,) = sqlx::query_as("SELECT id FROM areas ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, starts_at, location, status, created_by, area_id)
         VALUES ('Prayer Night', $1, 'Main Hall', 'published', $2, $3) RETURNING id",
    )
    .bind(tomorrow("19:30"))
    .bind(admin_id)
    .bind(area_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let (ride_id,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats)
         VALUES ($1, $2, $3, $4, '10 Jalan SS2', 'Main Hall', 1) RETURNING id",
    )
    .bind(passenger_id)
    .bind(event_id)
    .bind(tomorrow("19:00"))
    .bind(area_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // 1. Traffic member 1 creates single-ride change draft v1 -> gets edit lock automatically.
    let c1 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts"),
        Some(&rdm1),
        Some(json!({
            "action": "reassign",
            "driver_id": driver_id,
            "reason": "Closest volunteer in SS2",
            "submit": false,
        })),
    )
    .await;
    assert_eq!(c1.status, StatusCode::OK, "{}", c1.json);
    let d1_id = c1.json["draft"]["id"].as_i64().unwrap();
    assert_eq!(c1.json["draft"]["version"], 1);
    assert_eq!(c1.json["draft"]["lock_is_mine"], true);
    assert_eq!(c1.json["draft"]["locked_by"]["id"], m1_id);

    // 2. Visible to traffic member 2 on both /rides/:id/drafts and /admin/ride-drafts.
    let list_ride = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride_id}/drafts"),
        Some(&rdm2),
        None,
    )
    .await;
    assert_eq!(list_ride.status, StatusCode::OK);
    assert_eq!(list_ride.json.as_array().unwrap().len(), 1);
    assert_eq!(list_ride.json[0]["lock_is_mine"], false);

    let list_all = call(
        &app,
        Method::GET,
        "/api/admin/ride-drafts",
        Some(&rdm2),
        None,
    )
    .await;
    assert_eq!(list_all.status, StatusCode::OK);
    assert!(list_all
        .json
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"] == d1_id));

    // 3. Single-editor lock blocks member 2 with 423 LOCKED on edit or force lock.
    let edit_blocked = call(
        &app,
        Method::PUT,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}"),
        Some(&rdm2),
        Some(json!({
            "action": "alt_transport",
            "reason": "Attempt overwrite",
        })),
    )
    .await;
    assert_eq!(
        edit_blocked.status,
        StatusCode::LOCKED,
        "{}",
        edit_blocked.json
    );
    assert_eq!(edit_blocked.json["code"], "DRAFT_LOCKED_BY_OTHER");

    let lock_blocked = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}/lock"),
        Some(&rdm2),
        Some(json!({ "force": true })),
    )
    .await;
    assert_eq!(lock_blocked.status, StatusCode::LOCKED);

    // 4. Member 2 can create parallel v2 in the meantime.
    let c2 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts"),
        Some(&rdm2),
        Some(json!({
            "action": "alt_transport",
            "reason": "Use Grab if driver full",
            "submit": false,
        })),
    )
    .await;
    assert_eq!(c2.status, StatusCode::OK, "{}", c2.json);
    let d2_id = c2.json["draft"]["id"].as_i64().unwrap();
    assert_eq!(c2.json["draft"]["version"], 2);

    // 5. Member 1 submits v1; non-lead cannot approve or return (403).
    let sub = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}/request-approval"),
        Some(&rdm1),
        Some(json!({})),
    )
    .await;
    assert_eq!(sub.status, StatusCode::OK, "{}", sub.json);
    assert_eq!(sub.json["draft"]["status"], "pending_approval");

    let nonlead_app = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}/approve"),
        Some(&rdm1),
        Some(json!({})),
    )
    .await;
    assert_eq!(nonlead_app.status, StatusCode::FORBIDDEN);

    let nonlead_ret = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}/return"),
        Some(&rdm1),
        Some(json!({ "notes": "no" })),
    )
    .await;
    assert_eq!(nonlead_ret.status, StatusCode::FORBIDDEN);

    // 6. Lead sees pending_ride_drafts in dashboard, returns v1 -> member sees my_returned_ride_drafts.
    let lead_dash = call(
        &app,
        Method::GET,
        "/api/admin/dashboard",
        Some(&tlead),
        None,
    )
    .await;
    assert!(lead_dash.json["pending_ride_drafts"].as_i64().unwrap() >= 1);

    let ret = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}/return"),
        Some(&tlead),
        Some(json!({ "notes": "Verify pickup gate" })),
    )
    .await;
    assert_eq!(ret.status, StatusCode::OK, "{}", ret.json);
    assert_eq!(ret.json["draft"]["status"], "returned");

    let rdm1_dash = call(&app, Method::GET, "/api/admin/dashboard", Some(&rdm1), None).await;
    assert!(rdm1_dash.json["my_returned_ride_drafts"].as_i64().unwrap() >= 1);

    // 7. Member 1 edits returned v1, resubmits, Lead approves -> ride reassigned & v2 superseded.
    let upd = call(
        &app,
        Method::PUT,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}"),
        Some(&rdm1),
        Some(json!({
            "action": "reassign",
            "driver_id": driver_id,
            "reason": "Confirmed North gate SS2",
        })),
    )
    .await;
    assert_eq!(upd.status, StatusCode::OK, "{}", upd.json);

    call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}/request-approval"),
        Some(&rdm1),
        Some(json!({})),
    )
    .await;

    let approved = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d1_id}/approve"),
        Some(&tlead),
        Some(json!({})),
    )
    .await;
    assert_eq!(approved.status, StatusCode::OK, "{}", approved.json);
    assert_eq!(approved.json["draft"]["status"], "approved");

    let (r_status, r_driver): (String, Option<i64>) =
        sqlx::query_as("SELECT status, driver_id FROM rides WHERE id=$1")
            .bind(ride_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(r_status, "pending_driver_confirm");
    assert_eq!(r_driver, Some(driver_id));

    let (d2_status,): (String,) =
        sqlx::query_as("SELECT status FROM ride_change_drafts WHERE id=$1")
            .bind(d2_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(d2_status, "superseded");

    // 8. Verify Lead force-takeover on a new draft locked by member 2.
    let c3 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts"),
        Some(&rdm2),
        Some(json!({
            "action": "reimbursement_excluded",
            "reimbursement_excluded": true,
            "reason": "Self-waived driver",
        })),
    )
    .await;
    let d3_id = c3.json["draft"]["id"].as_i64().unwrap();
    let force_take = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/drafts/{d3_id}/lock"),
        Some(&tlead),
        Some(json!({ "force": true })),
    )
    .await;
    assert_eq!(force_take.status, StatusCode::OK, "{}", force_take.json);
    assert_eq!(force_take.json["draft"]["locked_by"]["id"], tlead_id);
}

#[tokio::test]
async fn reimbursement_excluded_suppresses_auto_reimb_and_routes_post_event_to_pending() {
    let (app, st) = setup().await;

    sqlx::query(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Reimb Check Member', 'rcm@example.org', '012-400 0033', 'traffic_member', 'active', 'hash', 'uuid-rcm')",
    )
    .execute(&st.db)
    .await
    .unwrap();

    let tmember = login(&st, "rcm@example.org").await;
    let tlead = login(&st, "traffichead@example.org").await;
    let driver1_ck = login(&st, "driver@example.org").await;
    let driver2_ck = login(&st, "driver2@example.org").await;

    let driver1_id = id_of(&st, "driver@example.org").await;
    let driver2_id = id_of(&st, "driver2@example.org").await;
    let passenger1_id = id_of(&st, "member@example.org").await;
    let passenger2_id = id_of(&st, "admin@example.org").await;
    let lead_id = id_of(&st, "traffichead@example.org").await;

    // Seed two distinct areas + area fare (1 <-> 2 = RM 25.00 = 2500 cents).
    let areas: Vec<(i64,)> = sqlx::query_as("SELECT id FROM areas ORDER BY id LIMIT 2")
        .fetch_all(&st.db)
        .await
        .unwrap();
    assert_eq!(areas.len(), 2);
    let event_area_id = areas[0].0;
    let pickup_area_id = areas[1].0;
    let (a, b) = if event_area_id <= pickup_area_id {
        (event_area_id, pickup_area_id)
    } else {
        (pickup_area_id, event_area_id)
    };
    sqlx::query(
        "INSERT INTO area_fares (area_a_id, area_b_id, amount_cents, updated_at)
         VALUES ($1, $2, 2500, app_now())
         ON CONFLICT (area_a_id, area_b_id) DO UPDATE SET amount_cents = 2500",
    )
    .bind(a)
    .bind(b)
    .execute(&st.db)
    .await
    .unwrap();

    let (event_id,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, starts_at, location, status, created_by, area_id, mileage_claimable)
         VALUES ('Outreach Sunday', $1, 'Sanctuary', 'published', $2, $3, 1) RETURNING id",
    )
    .bind(tomorrow("09:00"))
    .bind(lead_id)
    .bind(event_area_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Confirmed Ride A (driver1, to be marked reimbursement_excluded = 1)
    let (ride_a,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, leg, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, '22 Jalan Alpha', 'Sanctuary', 'outbound', 'confirmed', app_now(), app_now())
         RETURNING id",
    )
    .bind(passenger1_id)
    .bind(driver1_id)
    .bind(event_id)
    .bind(tomorrow("08:30"))
    .bind(pickup_area_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Confirmed Ride B (driver2, regular mileage claimable)
    let (ride_b,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, leg, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, '33 Jalan Beta', 'Sanctuary', 'outbound', 'confirmed', app_now(), app_now())
         RETURNING id",
    )
    .bind(passenger2_id)
    .bind(driver2_id)
    .bind(event_id)
    .bind(tomorrow("08:30"))
    .bind(pickup_area_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Non-lead cannot directly toggle reimbursement_excluded -> 403.
    let direct_deny = call(
        &app,
        Method::PUT,
        &format!("/api/rides/{ride_a}/reimbursement-excluded"),
        Some(&tmember),
        Some(json!({ "excluded": true })),
    )
    .await;
    assert_eq!(direct_deny.status, StatusCode::FORBIDDEN);

    // Lead toggles reimbursement_excluded = true on Ride A -> 200.
    let lead_excl = call(
        &app,
        Method::PUT,
        &format!("/api/rides/{ride_a}/reimbursement-excluded"),
        Some(&tlead),
        Some(json!({ "excluded": true })),
    )
    .await;
    assert_eq!(lead_excl.status, StatusCode::OK, "{}", lead_excl.json);
    assert_eq!(lead_excl.json["reimbursement_excluded"], true);

    // Driver 1 checks Ride A -> sees reimbursement_excluded = true, route_fare = null, mileage_receipt_needed = false.
    let ride_a_view = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ride_a}"),
        Some(&driver1_ck),
        None,
    )
    .await;
    assert_eq!(ride_a_view.status, StatusCode::OK);
    assert_eq!(ride_a_view.json["reimbursement_excluded"], true);
    assert!(ride_a_view.json["route_fare"].is_null());
    assert_eq!(ride_a_view.json["mileage_receipt_needed"], false);

    // Driver 1 completes Ride A -> auto-reimbursement is suppressed (amount = 0, receipt_needed = 0).
    for step in ["start", "pickup", "complete"] {
        let res = call(
            &app,
            Method::POST,
            &format!("/api/rides/{ride_a}/{step}"),
            Some(&driver1_ck),
            Some(json!({})),
        )
        .await;
        assert_eq!(res.status, StatusCode::OK, "ride_a {step}: {}", res.json);
    }
    let (a_amt, a_receipt): (i64, i64) = sqlx::query_as(
        "SELECT reimbursement_amount_cents, mileage_receipt_needed FROM rides WHERE id=$1",
    )
    .bind(ride_a)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(a_amt, 0);
    assert_eq!(a_receipt, 0);

    // Driver 2 completes regular Ride B -> fixed-rate auto-reimbursement = 2500 cents.
    for step in ["start", "pickup", "complete"] {
        let res = call(
            &app,
            Method::POST,
            &format!("/api/rides/{ride_b}/{step}"),
            Some(&driver2_ck),
            Some(json!({})),
        )
        .await;
        assert_eq!(res.status, StatusCode::OK, "ride_b {step}: {}", res.json);
    }
    let (b_amt,): (i64,) =
        sqlx::query_as("SELECT reimbursement_amount_cents FROM rides WHERE id=$1")
            .bind(ride_b)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(b_amt, 2500);

    // Post-event reimbursement behavior:
    // Driver 1 has only excluded ride(s) on this event -> flagged reimbursement_excluded = true.
    // Driver 2 has non-excluded ride(s) -> reimbursement_excluded = false.
    let roster = call(
        &app,
        Method::GET,
        &format!("/api/events/{event_id}/post-event-drivers"),
        Some(&tlead),
        None,
    )
    .await;
    assert_eq!(roster.status, StatusCode::OK, "{}", roster.json);
    let drivers = roster.json["drivers"].as_array().unwrap();
    let d1_row = drivers.iter().find(|d| d["id"] == driver1_id).unwrap();
    let d2_row = drivers.iter().find(|d| d["id"] == driver2_id).unwrap();
    assert_eq!(d1_row["reimbursement_excluded"], true);
    assert_eq!(d2_row["reimbursement_excluded"], false);

    // Submit post-event batch for both -> Driver 2 auto-approved, Driver 1 routed to pending review!
    let batch = call(
        &app,
        Method::POST,
        &format!("/api/events/{event_id}/post-event-reimbursements"),
        Some(&tlead),
        Some(json!({
            "claims": [
                {
                    "driver_id": driver1_id,
                    "trip_type": "direct_return",
                    "destination_area_id": pickup_area_id,
                    "amount_cents": 2500,
                    "note": "Exception request",
                },
                {
                    "driver_id": driver2_id,
                    "trip_type": "direct_return",
                    "destination_area_id": pickup_area_id,
                    "amount_cents": 2500,
                }
            ]
        })),
    )
    .await;
    assert_eq!(batch.status, StatusCode::OK, "{}", batch.json);
    assert_eq!(batch.json["created_count"], 2);
    assert_eq!(batch.json["auto_approved_count"], 1);
    assert_eq!(batch.json["pending_review_count"], 1);

    let (d1_claim_status,): (String,) =
        sqlx::query_as("SELECT status FROM reimbursements WHERE event_id=$1 AND user_id=$2")
            .bind(event_id)
            .bind(driver1_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    let (d2_claim_status,): (String,) =
        sqlx::query_as("SELECT status FROM reimbursements WHERE event_id=$1 AND user_id=$2")
            .bind(event_id)
            .bind(driver2_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(d1_claim_status, "pending");
    assert_eq!(d2_claim_status, "approved");
}
