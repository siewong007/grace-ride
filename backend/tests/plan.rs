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

#[tokio::test]
async fn auto_arrange_suggests_without_saving_and_keeps_pinned_rides() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;
    let staff = login(&st, "transport@example.org").await;
    let url = format!("/api/admin/events/{}/plan/arrange", f.event);

    // Pin ride 0 on the driver who takes one booking; the rest are open.
    let draft: Vec<Value> = f
        .rides
        .iter()
        .enumerate()
        .map(|(i, r)| json!({ "ride_id": r, "driver_id": if i == 0 { Some(f.driver2) } else { None } }))
        .collect();
    let r = call(
        &app,
        Method::POST,
        &url,
        Some(&staff),
        Some(json!({ "draft": draft })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let placed = r.json["placed"].as_array().unwrap();
    assert!(
        placed.iter().all(|p| p["ride_id"] != f.rides[0]),
        "pinned ride left alone"
    );
    assert!(
        placed.iter().all(|p| p["driver_id"] != f.driver2),
        "one booking for a non-carpool driver"
    );
    assert_eq!(
        placed.len() + r.json["unplaced"].as_array().unwrap().len(),
        3
    );
    // No coordinates are cached in tests, so every pickup is grouped by area only.
    assert_eq!(r.json["approximate"], 4);

    // Suggesting changes nothing.
    for id in &f.rides {
        assert_eq!(ride_state(&st, *id).await, ("pending_driver".into(), None));
    }

    let member = login(&st, "member@example.org").await;
    let r = call(
        &app,
        Method::POST,
        &url,
        Some(&member),
        Some(json!({ "draft": [] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn traffic_group_member_can_save_draft_and_request_traffic_lead_approval() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;

    // Create a traffic group member user with role 'traffic_member'
    let (tmember_id,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Traffic Member Bob', 'tmember@example.org', '012-300 0088', 'traffic_member', 'active', 'hash', 'uuid-tmember')
         RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    let tmember = login(&st, "tmember@example.org").await;
    let tlead = login(&st, "traffichead@example.org").await;
    let tlead_id = id_of(&st, "traffichead@example.org").await;

    // 1. Traffic member views the plan board: can_approve should be false
    let r = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tmember),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["can_approve"], false);
    assert!(r.json["draft"].is_null());

    // 2. Traffic member cannot directly approve / apply plan: forbidden!
    let direct_apply = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tmember),
        Some(json!({
            "changes": [
                change(f.rides[0], None, Some(f.driver)),
                change(f.rides[1], None, Some(f.driver)),
            ]
        })),
    )
    .await;
    assert_eq!(direct_apply.status, StatusCode::FORBIDDEN);
    assert_eq!(direct_apply.json["code"], "ONLY_TRAFFIC_LEAD_CAN_APPROVE");

    // 3. Traffic member saves draft
    let draft_body = json!({
        "assignments": [
            { "ride_id": f.rides[0], "driver_id": f.driver },
            { "ride_id": f.rides[1], "driver_id": f.driver },
            { "ride_id": f.rides[2], "driver_id": f.driver },
            { "ride_id": f.rides[3], "driver_id": f.driver2 },
        ]
    });
    let save_draft_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/draft", f.event),
        Some(&tmember),
        Some(draft_body.clone()),
    )
    .await;
    assert_eq!(
        save_draft_res.status,
        StatusCode::OK,
        "{}",
        save_draft_res.json
    );
    assert_eq!(save_draft_res.json["status"], "draft");

    // Verify rides are untouched
    assert_eq!(
        ride_state(&st, f.rides[0]).await,
        ("pending_driver".into(), None)
    );

    // 4. Traffic member requests traffic lead to approve
    let lead_notif_before: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
            .bind(tlead_id)
            .fetch_one(&st.db)
            .await
            .unwrap();

    let req_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/request-approval", f.event),
        Some(&tmember),
        Some(json!({})),
    )
    .await;
    assert_eq!(req_res.status, StatusCode::OK, "{}", req_res.json);
    assert_eq!(req_res.json["status"], "pending_approval");

    // Verify traffic lead received notification
    let lead_notif_after: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
            .bind(tlead_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(
        lead_notif_after.0 - lead_notif_before.0,
        1,
        "traffic lead received approval request notification"
    );

    // 5. Traffic lead views the plan board: can_approve is true, draft is pending_approval
    let lead_view = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tlead),
        None,
    )
    .await;
    assert_eq!(lead_view.status, StatusCode::OK);
    assert_eq!(lead_view.json["can_approve"], true);
    assert_eq!(lead_view.json["draft"]["status"], "pending_approval");
    assert_eq!(lead_view.json["draft"]["submitted_by"]["id"], tmember_id);

    // 6. Traffic lead returns draft with notes
    let tmember_notif_before: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
            .bind(tmember_id)
            .fetch_one(&st.db)
            .await
            .unwrap();

    let ret_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/return", f.event),
        Some(&tlead),
        Some(json!({ "notes": "Please check driver 2 capacity" })),
    )
    .await;
    assert_eq!(ret_res.status, StatusCode::OK, "{}", ret_res.json);
    assert_eq!(ret_res.json["status"], "returned");

    // Verify traffic member received return notification
    let tmember_notif_after: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
            .bind(tmember_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(
        tmember_notif_after.0 - tmember_notif_before.0,
        1,
        "traffic member received returned notification"
    );

    // 7. Traffic member re-submits for approval
    let resubmit = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/request-approval", f.event),
        Some(&tmember),
        Some(draft_body),
    )
    .await;
    assert_eq!(resubmit.status, StatusCode::OK);

    // 8. Traffic lead approves the plan
    let approve_body = json!({
        "changes": [
            change(f.rides[0], None, Some(f.driver)),
            change(f.rides[1], None, Some(f.driver)),
            change(f.rides[2], None, Some(f.driver)),
            change(f.rides[3], None, Some(f.driver2)),
        ]
    });
    let approve_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tlead),
        Some(approve_body),
    )
    .await;
    assert_eq!(approve_res.status, StatusCode::OK, "{}", approve_res.json);

    // Verify rides are now confirmed
    for id in &f.rides[..3] {
        assert_eq!(
            ride_state(&st, *id).await,
            ("pending_driver_confirm".into(), Some(f.driver))
        );
    }

    // Verify draft status is approved
    let final_view = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tlead),
        None,
    )
    .await;
    assert_eq!(final_view.json["draft"]["status"], "approved");

    // Verify traffic member received approval notification
    let tmember_notif_final: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
            .bind(tmember_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(
        tmember_notif_final.0 - tmember_notif_after.0,
        1,
        "traffic member notified of approval"
    );
}

#[tokio::test]
async fn multi_draft_versions_and_single_editor_lock() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;

    let (m1_id,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Member One', 'tm1@example.org', '012-300 0011', 'traffic_member', 'active', 'hash', 'uuid-tm1')
         RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let (m2_id,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Member Two', 'tm2@example.org', '012-300 0022', 'traffic_member', 'active', 'hash', 'uuid-tm2')
         RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    let tm1 = login(&st, "tm1@example.org").await;
    let tm2 = login(&st, "tm2@example.org").await;
    let tlead = login(&st, "traffichead@example.org").await;
    let tlead_id = id_of(&st, "traffichead@example.org").await;

    // 1. Member 1 creates draft v1 -> gets active edit lock automatically.
    let c1 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts", f.event),
        Some(&tm1),
        Some(json!({
            "label": "Option A",
            "assignments": [{ "ride_id": f.rides[0], "driver_id": f.driver }],
        })),
    )
    .await;
    assert_eq!(c1.status, StatusCode::OK, "{}", c1.json);
    let d1_id = c1.json["draft"]["id"].as_i64().unwrap();
    assert_eq!(c1.json["draft"]["version"], 1);
    assert_eq!(c1.json["draft"]["lock_is_mine"], true);
    assert_eq!(c1.json["draft"]["locked_by"]["id"], m1_id);

    // 2. Member 2 sees v1 locked by Member 1 and gets 423 LOCKED on edit/lock/submit.
    let v2_view = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tm2),
        None,
    )
    .await;
    assert_eq!(v2_view.status, StatusCode::OK);
    assert_eq!(v2_view.json["drafts"].as_array().unwrap().len(), 1);
    assert_eq!(v2_view.json["drafts"][0]["lock_is_mine"], false);
    assert_eq!(v2_view.json["drafts"][0]["locked_by"]["id"], m1_id);

    let edit_blocked = call(
        &app,
        Method::PUT,
        &format!("/api/admin/events/{}/plan/drafts/{d1_id}", f.event),
        Some(&tm2),
        Some(json!({ "label": "Hijack" })),
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
        &format!("/api/admin/events/{}/plan/drafts/{d1_id}/lock", f.event),
        Some(&tm2),
        Some(json!({ "force": true })),
    )
    .await;
    assert_eq!(lock_blocked.status, StatusCode::LOCKED);

    let submit_blocked = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/request-approval", f.event),
        Some(&tm2),
        Some(json!({ "draft_id": d1_id })),
    )
    .await;
    assert_eq!(submit_blocked.status, StatusCode::LOCKED);

    // 3. Member 2 creates parallel draft v2 without disturbing v1.
    let c2 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts", f.event),
        Some(&tm2),
        Some(json!({ "label": "Option B", "clone_from_draft_id": d1_id })),
    )
    .await;
    assert_eq!(c2.status, StatusCode::OK, "{}", c2.json);
    let d2_id = c2.json["draft"]["id"].as_i64().unwrap();
    assert_eq!(c2.json["draft"]["version"], 2);
    assert_ne!(d2_id, d1_id);

    // 4. Member 1 releases lock on v1 -> Member 2 acquires lock and edits v1.
    let rel = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/events/{}/plan/drafts/{d1_id}/lock", f.event),
        Some(&tm1),
        None,
    )
    .await;
    assert_eq!(rel.status, StatusCode::OK);
    assert!(rel.json["draft"]["locked_by"].is_null());

    let acq = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts/{d1_id}/lock", f.event),
        Some(&tm2),
        None,
    )
    .await;
    assert_eq!(acq.status, StatusCode::OK, "{}", acq.json);
    assert_eq!(acq.json["draft"]["locked_by"]["id"], m2_id);

    let upd = call(
        &app,
        Method::PUT,
        &format!("/api/admin/events/{}/plan/drafts/{d1_id}", f.event),
        Some(&tm2),
        Some(json!({
            "label": "Option A Revised",
            "assignments": [{ "ride_id": f.rides[0], "driver_id": f.driver }],
        })),
    )
    .await;
    assert_eq!(upd.status, StatusCode::OK, "{}", upd.json);
    assert_eq!(upd.json["draft"]["label"], "Option A Revised");

    // 5. Traffic lead force-takes over lock on v1.
    let takeover = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts/{d1_id}/lock", f.event),
        Some(&tlead),
        Some(json!({ "force": true })),
    )
    .await;
    assert_eq!(takeover.status, StatusCode::OK, "{}", takeover.json);
    assert_eq!(takeover.json["draft"]["locked_by"]["id"], tlead_id);
}

#[tokio::test]
async fn approving_draft_syncs_snapshot_supersedes_others_and_supports_zero_diff() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;
    sqlx::query(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Member Three', 'tm3@example.org', '012-300 0033', 'traffic_member', 'active', 'hash', 'uuid-tm3')",
    )
    .execute(&st.db)
    .await
    .unwrap();
    let tm3 = login(&st, "tm3@example.org").await;
    let tlead = login(&st, "traffichead@example.org").await;

    // Create v1 and v2; approve v1 -> v2 becomes superseded & snapshot stays clean.
    let d1 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts", f.event),
        Some(&tm3),
        Some(json!({
            "label": "V1",
            "assignments": [{ "ride_id": f.rides[0], "driver_id": f.driver }],
        })),
    )
    .await;
    let d1_id = d1.json["draft"]["id"].as_i64().unwrap();
    let d2 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts", f.event),
        Some(&tm3),
        Some(json!({
            "label": "V2",
            "assignments": [{ "ride_id": f.rides[0], "driver_id": f.driver2 }],
        })),
    )
    .await;
    let d2_id = d2.json["draft"]["id"].as_i64().unwrap();

    call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/request-approval", f.event),
        Some(&tm3),
        Some(json!({ "draft_id": d1_id })),
    )
    .await;

    let app1 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/approve", f.event),
        Some(&tlead),
        Some(json!({
            "draft_id": d1_id,
            "changes": [change(f.rides[0], None, Some(f.driver))],
        })),
    )
    .await;
    assert_eq!(app1.status, StatusCode::OK, "{}", app1.json);

    let board = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tlead),
        None,
    )
    .await;
    let drafts = board.json["drafts"].as_array().unwrap();
    let v1_after = drafts.iter().find(|d| d["id"] == d1_id).unwrap();
    let v2_after = drafts.iter().find(|d| d["id"] == d2_id).unwrap();
    assert_eq!(v1_after["status"], "approved");
    assert_eq!(v2_after["status"], "superseded");
    // Approved snapshot stores every ride including rides untouched (ride 1 is still null).
    assert!(v1_after["assignments"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["ride_id"] == f.rides[0] && a["driver_id"] == f.driver));

    // Zero-diff draft: create v3 matching current live state, submit, approve with empty changes [].
    let d3 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts", f.event),
        Some(&tm3),
        Some(json!({ "label": "Zero diff" })),
    )
    .await;
    let d3_id = d3.json["draft"]["id"].as_i64().unwrap();
    call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/request-approval", f.event),
        Some(&tm3),
        Some(json!({ "draft_id": d3_id })),
    )
    .await;

    let app_zero = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/approve", f.event),
        Some(&tlead),
        Some(json!({ "draft_id": d3_id, "changes": [] })),
    )
    .await;
    assert_eq!(app_zero.status, StatusCode::OK, "{}", app_zero.json);
    assert_eq!(app_zero.json["approved_draft_id"], d3_id);
}

#[tokio::test]
async fn draft_visitors_commit_on_lead_approval_while_direct_remains_forbidden() {
    let (app, st) = setup().await;
    let f = fixture(&st).await;
    sqlx::query(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Member Four', 'tm4@example.org', '012-300 0044', 'traffic_member', 'active', 'hash', 'uuid-tm4')",
    )
    .execute(&st.db)
    .await
    .unwrap();
    let tm4 = login(&st, "tm4@example.org").await;
    let tlead = login(&st, "traffichead@example.org").await;

    // Direct visitor add by traffic_member stays 403.
    let direct = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/visitors", f.event),
        Some(&tm4),
        Some(json!({ "name": "Direct Denied", "seats": 1, "leg": "outbound" })),
    )
    .await;
    assert_eq!(direct.status, StatusCode::FORBIDDEN);

    // Propose visitor inside an event plan draft.
    let c = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/drafts", f.event),
        Some(&tm4),
        Some(json!({
            "label": "With visitor",
            "visitors": [{
                "temp_id": "tv-1",
                "op": "add",
                "name": "Sister Guest",
                "phone": "019-9998888",
                "notes": "Wait at Gate B",
                "seats": 2,
                "leg": "outbound",
                "driver_id": f.driver,
            }],
        })),
    )
    .await;
    assert_eq!(c.status, StatusCode::OK, "{}", c.json);
    let did = c.json["draft"]["id"].as_i64().unwrap();

    // Still zero live visitors before approval.
    let before = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tm4),
        None,
    )
    .await;
    assert_eq!(before.json["visitors"].as_array().unwrap().len(), 0);
    assert_eq!(
        before.json["drafts"][0]["visitors"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/request-approval", f.event),
        Some(&tm4),
        Some(json!({ "draft_id": did })),
    )
    .await;

    let app_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{}/plan/approve", f.event),
        Some(&tlead),
        Some(json!({ "draft_id": did, "changes": [] })),
    )
    .await;
    assert_eq!(app_res.status, StatusCode::OK, "{}", app_res.json);

    let after = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", f.event),
        Some(&tlead),
        None,
    )
    .await;
    let visitors = after.json["visitors"].as_array().unwrap();
    assert_eq!(visitors.len(), 1);
    assert_eq!(visitors[0]["name"], "Sister Guest");
    assert_eq!(visitors[0]["driver_id"], f.driver);
    assert_eq!(visitors[0]["seats"], 2);
}

#[tokio::test]
async fn traffic_plans_endpoint_shows_cross_group_events_and_dashboard_draft_counts() {
    let (app, st) = setup().await;
    let (cell_id,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Cell Lead Other', 'cellother@example.org', '012-300 0055', 'cell_leader', 'active', 'hash', 'uuid-cellother')
         RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid)
         VALUES ('Member Five', 'tm5@example.org', '012-300 0066', 'traffic_member', 'active', 'hash', 'uuid-tm5')",
    )
    .execute(&st.db)
    .await
    .unwrap();

    let (cross_ev,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, starts_at, location, status, created_by)
         VALUES ('Other Group Cell Night', $1, 'PJ Centre', 'published', $2) RETURNING id",
    )
    .bind(tomorrow("20:00"))
    .bind(cell_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let tm5 = login(&st, "tm5@example.org").await;
    let tlead = login(&st, "traffichead@example.org").await;

    // Cross-group discovery works for traffic_member without admin_events.view!
    let list_res = call(
        &app,
        Method::GET,
        "/api/admin/traffic/plans",
        Some(&tm5),
        None,
    )
    .await;
    assert_eq!(list_res.status, StatusCode::OK, "{}", list_res.json);
    assert!(
        list_res
            .json
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == cross_ev),
        "traffic member sees cross-group event in /admin/traffic/plans"
    );

    // Create + submit draft on cross_ev -> Lead sees pending_plan_drafts == 1.
    let dr = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{cross_ev}/plan/drafts"),
        Some(&tm5),
        Some(json!({ "label": "Cross group draft", "assignments": [] })),
    )
    .await;
    let did = dr.json["draft"]["id"].as_i64().unwrap();
    call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{cross_ev}/plan/request-approval"),
        Some(&tm5),
        Some(json!({ "draft_id": did })),
    )
    .await;

    let lead_dash = call(
        &app,
        Method::GET,
        "/api/admin/dashboard",
        Some(&tlead),
        None,
    )
    .await;
    assert_eq!(lead_dash.status, StatusCode::OK);
    assert!(lead_dash.json["pending_plan_drafts"].as_i64().unwrap() >= 1);

    // Lead returns draft -> Member sees my_returned_plan_drafts == 1.
    call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{cross_ev}/plan/return"),
        Some(&tlead),
        Some(json!({ "draft_id": did, "notes": "Add backup driver" })),
    )
    .await;

    let member_dash = call(&app, Method::GET, "/api/admin/dashboard", Some(&tm5), None).await;
    assert_eq!(member_dash.status, StatusCode::OK);
    assert!(
        member_dash.json["my_returned_plan_drafts"]
            .as_i64()
            .unwrap()
            >= 1
    );
}
