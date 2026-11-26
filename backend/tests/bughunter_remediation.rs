//! End-to-end regression tests for the BugHunter security audit remediations
//! (H-01..H-09, M-02..M-05, M-08, M-14, M-15, and fail-closed backup encryption).

mod common;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::Command;

use axum::http::{header::HeaderName, HeaderMap, HeaderValue, Method, StatusCode};
use common::*;
use serde_json::json;

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn first_area_id(st: &grace_ride::state::AppState) -> i64 {
    sqlx::query_scalar("SELECT id FROM areas WHERE active = 1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn ensure_driver_vehicle(st: &grace_ride::state::AppState, uid: i64, plate: &str) {
    sqlx::query("UPDATE users SET spare_seats = 4, carpool_enabled = 1 WHERE id = $1")
        .bind(uid)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO vehicles (user_id, model, color, plate, seats, is_active)
         VALUES ($1, 'Myvi', 'White', $2, 5, 1)
         ON CONFLICT DO NOTHING",
    )
    .bind(uid)
    .bind(plate)
    .execute(&st.db)
    .await
    .unwrap();
}

async fn insert_event(
    st: &grace_ride::state::AppState,
    created_by: i64,
    title: &str,
    status: &str,
    mileage_claimable: i64,
) -> i64 {
    let area = first_area_id(st).await;
    sqlx::query_scalar(
        "INSERT INTO events (title, kind, starts_at, location, status, created_by, area_id, mileage_claimable, reminder_sent)
         VALUES ($1, 'other', $2, 'Church Hall', $3, $4, $5, $6, 1)
         RETURNING id",
    )
    .bind(title)
    .bind(tomorrow("09:00"))
    .bind(status)
    .bind(created_by)
    .bind(area)
    .bind(mileage_claimable)
    .fetch_one(&st.db)
    .await
    .unwrap()
}

async fn signup_going(st: &grace_ride::state::AppState, event_id: i64, uid: i64) {
    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status)
         VALUES ($1, $2, 'going')
         ON CONFLICT (event_id, user_id) DO UPDATE SET status = 'going'",
    )
    .bind(event_id)
    .bind(uid)
    .execute(&st.db)
    .await
    .unwrap();
}

#[tokio::test]
async fn h01_admin_user_update_by_rank_requires_mfa() {
    let (app, st) = setup().await;
    let auditor_id = user_id(&st, "auditor@example.org").await;
    let president_no_mfa = login_mfa(&st, "president@example.org", false).await;
    let president_mfa = login_mfa(&st, "president@example.org", true).await;

    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{auditor_id}"),
        Some(&president_no_mfa),
        Some(json!({ "roles": ["auditor"], "status": "disabled" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "MFA_REQUIRED");

    let ok = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{auditor_id}"),
        Some(&president_mfa),
        Some(json!({ "roles": ["auditor"], "status": "disabled" })),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);

    let restore = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{auditor_id}"),
        Some(&president_mfa),
        Some(json!({ "roles": ["auditor"], "status": "active" })),
    )
    .await;
    assert_eq!(restore.status, StatusCode::OK, "{}", restore.json);
}

#[tokio::test]
async fn h02_recently_privileged_recovery_cooldown_and_reimb_sod_constraints() {
    let (app, st) = setup().await;
    let superadmin = login_mfa(&st, "superadmin@example.org", true).await;
    let admin = login_mfa(&st, "admin@example.org", true).await;
    let president = login_mfa(&st, "president@example.org", true).await;
    let transport_id = user_id(&st, "transport@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let finance_id = user_id(&st, "finance@example.org").await;

    // Temporarily demote a privileged account to ordinary member.
    let demote = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{transport_id}"),
        Some(&superadmin),
        Some(json!({ "roles": [], "status": "active" })),
    )
    .await;
    assert_eq!(demote.status, StatusCode::OK, "{}", demote.json);

    let last_priv: Option<String> =
        sqlx::query_scalar("SELECT last_privileged_at FROM users WHERE id = $1")
            .bind(transport_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(
        last_priv.is_some(),
        "demoting a privileged user must stamp last_privileged_at"
    );

    // Open recovery request and verify single admin cannot bypass dual-admin recovery during cooldown.
    let req_res = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({ "email": "transport@example.org", "reason": "Lost passkey device" })),
    )
    .await;
    assert_eq!(req_res.status, StatusCode::OK, "{}", req_res.json);

    let rec_id: i64 = sqlx::query_scalar(
        "SELECT id FROM recovery_requests WHERE email = 'transport@example.org' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    let first_approve = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rec_id}/approve"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(
        first_approve.status,
        StatusCode::OK,
        "{}",
        first_approve.json
    );
    assert_eq!(first_approve.json["awaiting_second"], true);
    assert!(first_approve.json.get("url").is_none());

    let same_admin_again = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rec_id}/approve"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(
        same_admin_again.status,
        StatusCode::FORBIDDEN,
        "{}",
        same_admin_again.json
    );
    assert_eq!(same_admin_again.json["code"], "SECOND_ADMIN_REQUIRED");

    let second_approve = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rec_id}/approve"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(
        second_approve.status,
        StatusCode::OK,
        "{}",
        second_approve.json
    );
    assert!(second_approve.json["url"]
        .as_str()
        .unwrap_or_default()
        .contains("/invite/"));

    // Verify PostgreSQL CHECK constraints enforce reimbursement Segregation of Duties at DB layer.
    let payee_eq_payer = sqlx::query(
        "INSERT INTO reimbursements (user_id, amount_cents, description, payee_name, payee_phone, status, paid_by)
         VALUES ($1, 1000, 'SoD test', 'Member', '012-300 0001', 'paid', $1)",
    )
    .bind(member_id)
    .execute(&st.db)
    .await;
    assert!(
        payee_eq_payer.is_err(),
        "chk_reimbursements_payee_not_payer must reject paid_by == user_id"
    );

    let reviewer_eq_payer = sqlx::query(
        "INSERT INTO reimbursements (user_id, amount_cents, description, payee_name, payee_phone, status, reviewed_by, paid_by)
         VALUES ($1, 1000, 'SoD test', 'Member', '012-300 0001', 'paid', $2, $2)",
    )
    .bind(member_id)
    .bind(finance_id)
    .execute(&st.db)
    .await;
    assert!(
        reviewer_eq_payer.is_err(),
        "chk_reimbursements_reviewer_not_payer must reject paid_by == reviewed_by"
    );

    let payee_eq_reviewer = sqlx::query(
        "INSERT INTO reimbursements (user_id, amount_cents, description, payee_name, payee_phone, status, reviewed_by)
         VALUES ($1, 1000, 'SoD test', 'Member', '012-300 0001', 'approved', $1)",
    )
    .bind(member_id)
    .execute(&st.db)
    .await;
    assert!(
        payee_eq_reviewer.is_err(),
        "chk_reimbursements_payee_not_reviewer must reject reviewed_by == user_id"
    );
}

#[tokio::test]
async fn h03_admin_rides_update_notifications_scoped_to_event_audience() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let cell_id = user_id(&st, "cellleader@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let area = first_area_id(&st).await;

    let ev = insert_event(&st, admin_id, "Private Retreat 9912", "published", 0).await;
    signup_going(&st, ev, member_id).await;
    signup_going(&st, ev, driver_id).await;

    let ride_id: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, destination, seats, status)
         VALUES ($1, $2, $3, $4, $5, 'Private Retreat 9912', 1, 'confirmed')
         RETURNING id",
    )
    .bind(member_id)
    .bind(driver_id)
    .bind(ev)
    .bind(tomorrow("08:30"))
    .bind(area)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Driver withdraws -> triggers notify_traffic_for_event for this event's scoped staff.
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/withdraw"),
        Some(&driver),
        Some(json!({ "reason": "Flat tyre" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);

    let link = format!("/admin/rides/{ride_id}");
    let admin_notifs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND link = $2")
            .bind(admin_id)
            .bind(&link)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(
        admin_notifs >= 1,
        "global transport/admin staff should receive the notification"
    );

    let cell_notifs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND link = $2")
            .bind(cell_id)
            .bind(&link)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(
        cell_notifs, 0,
        "cell_leader outside the event scope must not receive AdminRides.Update fan-out notifications"
    );
}

#[tokio::test]
async fn h04_removed_carpool_passenger_does_not_receive_trip_chat_notifications() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let active_p_id = user_id(&st, "member@example.org").await;
    let removed_p_id = user_id(&st, "amina@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let removed_p = login(&st, "amina@example.org").await;
    let area = first_area_id(&st).await;

    let ev = insert_event(&st, admin_id, "Sunday Carpool", "published", 0).await;
    let trip_id: i64 = 90_001;
    sqlx::query("INSERT INTO trips (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(trip_id)
        .execute(&st.db)
        .await
        .unwrap();

    let past = grace_ride::util::in_hours(-1);
    let active_ride: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, trip_id, pickup_at, pickup_area_id, destination, seats, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'Church', 1, 'in_progress')
         RETURNING id",
    )
    .bind(active_p_id)
    .bind(driver_id)
    .bind(ev)
    .bind(trip_id)
    .bind(&past)
    .bind(area)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let removed_ride: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, trip_id, pickup_at, pickup_area_id, destination, seats, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'Church', 1, 'in_progress')
         RETURNING id",
    )
    .bind(removed_p_id)
    .bind(driver_id)
    .bind(ev)
    .bind(trip_id)
    .bind(&past)
    .bind(area)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Mark second passenger as no_show.
    let ns = call(
        &app,
        Method::POST,
        &format!("/api/rides/{removed_ride}/no-show"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(ns.status, StatusCode::OK, "{}", ns.json);

    // Removed passenger can no longer read trip chat.
    let removed_read = call(
        &app,
        Method::GET,
        &format!("/api/rides/{removed_ride}/messages"),
        Some(&removed_p),
        None,
    )
    .await;
    assert_eq!(
        removed_read.status,
        StatusCode::FORBIDDEN,
        "{}",
        removed_read.json
    );

    // Subscribe both passengers to the SSE event hub and drain any stale events.
    let hub = grace_ride::stream::channel();
    let mut rx_active = hub.subscribe(active_p_id);
    let mut rx_removed = hub.subscribe(removed_p_id);
    while rx_active.try_recv().is_ok() {}
    while rx_removed.try_recv().is_ok() {}

    let msg = call(
        &app,
        Method::POST,
        &format!("/api/rides/{active_ride}/messages"),
        Some(&driver),
        Some(json!({ "body": "Arriving at Gate B in 2 minutes" })),
    )
    .await;
    assert_eq!(msg.status, StatusCode::OK, "{}", msg.json);

    let mut active_got_chat = false;
    while let Ok(evt) = rx_active.try_recv() {
        if evt.kind == "chat" {
            active_got_chat = true;
        }
    }
    assert!(
        active_got_chat,
        "active carpool passenger must receive chat SSE tick"
    );

    let mut removed_got_chat = false;
    while let Ok(evt) = rx_removed.try_recv() {
        if evt.kind == "chat" {
            removed_got_chat = true;
        }
    }
    assert!(
        !removed_got_chat,
        "no_show carpool passenger must not receive trip chat SSE ticks"
    );
}

#[tokio::test]
async fn h05_review_reimbursement_rejects_stale_expected_updated_at() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let member = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;

    let ev = insert_event(&st, admin_id, "Reimb Event", "published", 0).await;
    signup_going(&st, ev, member_id).await;
    let receipt = upload_png(&app, &member).await;

    let created = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&member),
        Some(json!({
            "event_id": ev,
            "amount_cents": 1800,
            "description": "Grab ride",
            "receipt_file_id": receipt,
            "payee_name": "Member",
            "payee_phone": "012-300 0006",
            "kind": "grab"
        })),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let rid = created.json["id"].as_i64().unwrap();

    let detail = call(
        &app,
        Method::GET,
        &format!("/api/reimbursements/{rid}"),
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(detail.status, StatusCode::OK, "{}", detail.json);
    let current_updated = detail.json["updated_at"].as_str().unwrap().to_string();

    // Stale expected_updated_at -> 409 CONFLICT, claim stays pending.
    let stale = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{rid}/review"),
        Some(&transport),
        Some(json!({
            "action": "approve",
            "note": "",
            "expected_updated_at": "2020-01-01 00:00:00"
        })),
    )
    .await;
    assert_eq!(stale.status, StatusCode::CONFLICT, "{}", stale.json);
    assert_eq!(stale.json["code"], "REQUEST_ALREADY_HANDLED");

    let status_after_stale: String =
        sqlx::query_scalar("SELECT status FROM reimbursements WHERE id = $1")
            .bind(rid)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(status_after_stale, "pending");

    // Matching expected_updated_at -> 200 OK.
    let fresh = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{rid}/review"),
        Some(&transport),
        Some(json!({
            "action": "approve",
            "note": "",
            "expected_updated_at": current_updated
        })),
    )
    .await;
    assert_eq!(fresh.status, StatusCode::OK, "{}", fresh.json);
}

#[tokio::test]
async fn h06_h07_grab_reimbursement_status_guard_and_kind_scoped_ride_uniqueness() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let amina_id = user_id(&st, "amina@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let area = first_area_id(&st).await;

    let ev = insert_event(&st, admin_id, "Outreach Camp", "published", 1).await;
    signup_going(&st, ev, member_id).await;
    signup_going(&st, ev, amina_id).await;
    signup_going(&st, ev, driver_id).await;

    // H-07: Grab reimbursement must be refused for completed, no_show, and in_progress rides.
    for ride_status in ["completed", "no_show", "in_progress"] {
        let rid: i64 = sqlx::query_scalar(
            "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, destination, seats, status)
             VALUES ($1, $2, $3, $4, $5, 'Camp', 1, $6)
             RETURNING id",
        )
        .bind(member_id)
        .bind(driver_id)
        .bind(ev)
        .bind(tomorrow("08:00"))
        .bind(area)
        .bind(ride_status)
        .fetch_one(&st.db)
        .await
        .unwrap();
        let receipt = upload_png(&app, &member).await;
        let res = call(
            &app,
            Method::POST,
            "/api/reimbursements",
            Some(&member),
            Some(json!({
                "ride_id": rid,
                "event_id": ev,
                "amount_cents": 1500,
                "description": "Grab",
                "receipt_file_id": receipt,
                "payee_name": "Member",
                "payee_phone": "012-300 0006",
                "kind": "grab"
            })),
        )
        .await;
        assert_eq!(
            res.status,
            StatusCode::BAD_REQUEST,
            "{ride_status}: {}",
            res.json
        );
        assert_eq!(res.json["code"], "INVALID_ACTION", "{ride_status}");
    }

    // H-06: Two completed unpriced rides on the same carpool trip cannot both claim mileage.
    let trip_id: i64 = 90_002;
    sqlx::query("INSERT INTO trips (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(trip_id)
        .execute(&st.db)
        .await
        .unwrap();
    let ride_a: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, trip_id, pickup_at, pickup_area_id, destination, seats, status, mileage_receipt_needed)
         VALUES ($1, $2, $3, $4, $5, $6, 'Camp', 1, 'completed', 1)
         RETURNING id",
    )
    .bind(member_id)
    .bind(driver_id)
    .bind(ev)
    .bind(trip_id)
    .bind(tomorrow("08:00"))
    .bind(area)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let ride_b: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, trip_id, pickup_at, pickup_area_id, destination, seats, status, mileage_receipt_needed)
         VALUES ($1, $2, $3, $4, $5, $6, 'Camp', 1, 'completed', 1)
         RETURNING id",
    )
    .bind(amina_id)
    .bind(driver_id)
    .bind(ev)
    .bind(trip_id)
    .bind(tomorrow("08:00"))
    .bind(area)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let rec1 = upload_png(&app, &driver).await;
    let claim1 = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&driver),
        Some(json!({
            "ride_id": ride_a,
            "amount_cents": 2500,
            "description": "Fuel receipt",
            "receipt_file_id": rec1,
            "payee_name": "Driver",
            "payee_phone": "012-300 0004",
            "kind": "mileage"
        })),
    )
    .await;
    assert_eq!(claim1.status, StatusCode::OK, "{}", claim1.json);

    let rec2 = upload_png(&app, &driver).await;
    let claim2 = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&driver),
        Some(json!({
            "ride_id": ride_b,
            "amount_cents": 2500,
            "description": "Duplicate sibling mileage",
            "receipt_file_id": rec2,
            "payee_name": "Driver",
            "payee_phone": "012-300 0004",
            "kind": "mileage"
        })),
    )
    .await;
    assert_eq!(claim2.status, StatusCode::BAD_REQUEST, "{}", claim2.json);
    assert_eq!(claim2.json["code"], "REIMBURSEMENT_ALREADY_CLAIMED");

    // H-07 index check: idx_reimb_live_ride_kind allows distinct kinds ('grab' + 'mileage')
    // on the same ride_id, while blocking a duplicate live (ride_id, kind).
    let coexist = sqlx::query(
        "INSERT INTO reimbursements (user_id, ride_id, event_id, amount_cents, description, payee_name, payee_phone, status, kind)
         VALUES ($1, $2, $3, 1200, 'Grab coexist', 'Member', '012-300 0006', 'pending', 'grab')",
    )
    .bind(member_id)
    .bind(ride_a)
    .bind(ev)
    .execute(&st.db)
    .await;
    assert!(
        coexist.is_ok(),
        "live grab and live mileage claims on the same ride must not collide: {:?}",
        coexist.err()
    );

    let dup_same_kind = sqlx::query(
        "INSERT INTO reimbursements (user_id, ride_id, event_id, amount_cents, description, payee_name, payee_phone, status, kind)
         VALUES ($1, $2, $3, 1200, 'Second live grab', 'Member', '012-300 0006', 'pending', 'grab')",
    )
    .bind(member_id)
    .bind(ride_a)
    .bind(ev)
    .execute(&st.db)
    .await;
    assert!(
        dup_same_kind.is_err(),
        "duplicate live (ride_id, kind) must be blocked by idx_reimb_live_ride_kind"
    );
}

#[tokio::test]
async fn h08_purge_orphan_files_preserves_settlement_revision_files() {
    let (_app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let ev = insert_event(&st, admin_id, "Bill Event", "published", 0).await;

    std::fs::create_dir_all(&st.cfg.upload_dir).unwrap();
    let keep_name = "rev_keep_test.png";
    let orphan_name = "orphan_del_test.png";
    let keep_path = st.cfg.upload_dir.join(keep_name);
    let orphan_path = st.cfg.upload_dir.join(orphan_name);
    std::fs::write(&keep_path, b"PNG_KEEP").unwrap();
    std::fs::write(&orphan_path, b"PNG_ORPHAN").unwrap();

    let old_ts = grace_ride::util::in_hours(-48);
    let keep_id: i64 = sqlx::query_scalar(
        "INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at)
         VALUES ($1, $2, 'image/png', 'keep.png', 8, $3) RETURNING id",
    )
    .bind(admin_id)
    .bind(keep_name)
    .bind(&old_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let orphan_id: i64 = sqlx::query_scalar(
        "INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at)
         VALUES ($1, $2, 'image/png', 'orphan.png', 10, $3) RETURNING id",
    )
    .bind(admin_id)
    .bind(orphan_name)
    .bind(&old_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let settlement_id: i64 = sqlx::query_scalar(
        "INSERT INTO settlements (event_id, created_by, title, status)
         VALUES ($1, $2, 'Historical Bill', 'published') RETURNING id",
    )
    .bind(ev)
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let snapshot = json!({
        "payee_qr_file_id": null,
        "items": [{ "receipt_file_id": keep_id, "label": "Shuttle" }],
        "shares": []
    });
    sqlx::query(
        "INSERT INTO settlement_revisions (settlement_id, version, snapshot, reason, created_by)
         VALUES ($1, 1, $2::jsonb, 'Initial publish', $3)",
    )
    .bind(settlement_id)
    .bind(snapshot.to_string())
    .bind(admin_id)
    .execute(&st.db)
    .await
    .unwrap();

    grace_ride::jobs::purge_orphan_files(&st.db, &st.cfg.upload_dir)
        .await
        .unwrap();

    let keep_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM files WHERE id = $1)")
        .bind(keep_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let orphan_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM files WHERE id = $1)")
            .bind(orphan_id)
            .fetch_one(&st.db)
            .await
            .unwrap();

    assert!(
        keep_exists && keep_path.exists(),
        "file referenced in settlement_revisions snapshot must survive purge_orphan_files"
    );
    assert!(
        !orphan_exists && !orphan_path.exists(),
        "truly unreferenced >24h file must be purged from DB and disk"
    );
    let _ = std::fs::remove_file(&keep_path);
}

#[tokio::test]
async fn h09_plan_visitors_draft_event_and_driver_reassignment_with_live_reimb() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let driver1 = user_id(&st, "driver@example.org").await;
    let driver2 = user_id(&st, "driver2@example.org").await;
    let head = login(&st, "traffichead@example.org").await;

    ensure_driver_vehicle(&st, driver1, "BHV 1001").await;
    ensure_driver_vehicle(&st, driver2, "BHV 1002").await;

    // 1. Draft event: requesting visitor_special reimbursement must be rejected with EVENT_NOT_OPEN.
    let draft_ev = insert_event(&st, admin_id, "Draft Camp", "draft", 0).await;
    let draft_vid: i64 = sqlx::query_scalar(
        "INSERT INTO plan_visitors (event_id, name, seats, leg, driver_id, created_by)
         VALUES ($1, 'Draft Guest', 1, 'outbound', $2, $3) RETURNING id",
    )
    .bind(draft_ev)
    .bind(driver1)
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let draft_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{draft_ev}/plan/visitors/{draft_vid}/reimbursement"),
        Some(&head),
        Some(json!({ "amount_cents": 2000, "reason": "Draft event detour" })),
    )
    .await;
    assert_eq!(
        draft_res.status,
        StatusCode::BAD_REQUEST,
        "{}",
        draft_res.json
    );
    assert_eq!(draft_res.json["code"], "EVENT_NOT_OPEN");

    // 2. Published event: reassigning driver_id on a visitor with a live reimbursement must return 409.
    let pub_ev = insert_event(&st, admin_id, "Published Camp", "published", 0).await;
    let create_vis = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{pub_ev}/plan/visitors"),
        Some(&head),
        Some(json!({ "name": "Guest Tan", "seats": 1, "driver_id": driver1 })),
    )
    .await;
    assert_eq!(create_vis.status, StatusCode::OK, "{}", create_vis.json);
    let vid = create_vis.json["id"].as_i64().unwrap();

    let req_reimb = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{pub_ev}/plan/visitors/{vid}/reimbursement"),
        Some(&head),
        Some(json!({ "amount_cents": 2500, "reason": "Airport pickup" })),
    )
    .await;
    assert_eq!(req_reimb.status, StatusCode::OK, "{}", req_reimb.json);

    let reassign = call(
        &app,
        Method::PUT,
        &format!("/api/admin/events/{pub_ev}/plan/visitors/{vid}"),
        Some(&head),
        Some(json!({ "name": "Guest Tan", "seats": 1, "driver_id": driver2 })),
    )
    .await;
    assert_eq!(reassign.status, StatusCode::CONFLICT, "{}", reassign.json);
    assert_eq!(reassign.json["code"], "VISITOR_HAS_REIMBURSEMENT");
}

#[tokio::test]
async fn m02_m03_m04_m05_m08_m14_m15_and_backup_hardening() {
    let (app, st) = setup().await;

    // M-02: Session cookie rotates upon TOTP MFA enable, invalidating the pre-elevation token.
    let old_cookie = login(&st, "member@example.org").await;
    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&old_cookie),
        None,
    )
    .await;
    assert_eq!(setup_res.status, StatusCode::OK, "{}", setup_res.json);
    let secret = setup_res.json["secret"].as_str().unwrap().to_string();
    let code = totp_code(&secret, "member@example.org");
    let enable_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&old_cookie),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(enable_res.status, StatusCode::OK, "{}", enable_res.json);
    let new_cookie = cookie_from(&enable_res).expect("totp/enable must rotate session cookie");
    assert_ne!(new_cookie, old_cookie);
    let old_me = call(&app, Method::GET, "/api/auth/me", Some(&old_cookie), None).await;
    assert_eq!(old_me.status, StatusCode::UNAUTHORIZED);
    let new_me = call(&app, Method::GET, "/api/auth/me", Some(&new_cookie), None).await;
    assert_eq!(new_me.status, StatusCode::OK);

    // M-03: client_ip parses ALL X-Forwarded-For headers and skips IPv4-mapped IPv6 & CGNAT hops.
    let mut headers = HeaderMap::new();
    let xff = HeaderName::from_static("x-forwarded-for");
    headers.append(
        xff.clone(),
        HeaderValue::from_static("1.2.3.4, 198.51.100.7"),
    );
    headers.append(xff, HeaderValue::from_static("::ffff:10.0.0.1, 100.64.0.9"));
    let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 443);
    assert_eq!(
        grace_ride::security::client_ip(&headers, Some(peer), true),
        "198.51.100.7"
    );

    // M-04: POST /api/visit/me/readiness rate-limits rapid calls (bucket limit = 15/300s).
    let mut saw_429 = false;
    for _ in 0..16 {
        let r = call(
            &app,
            Method::POST,
            "/api/visit/me/readiness",
            None,
            Some(json!({ "status": "ready" })),
        )
        .await;
        if r.status == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }
    assert!(
        saw_429,
        "POST /api/visit/me/readiness must enforce rate limiting"
    );

    // M-05: Read-only AdminSettings.View (president) must not receive plaintext telegram_webhook_secret.
    let enc_tg = grace_ride::crypto::encrypt_secret(
        "app_secret:telegram_bot_token",
        "123456:TEST_BOT_TOKEN_SECRET",
    )
    .unwrap();
    sqlx::query(
        "INSERT INTO app_secrets (name, value) VALUES ('telegram_bot_token', $1)
         ON CONFLICT (name) DO UPDATE SET value = excluded.value",
    )
    .bind(&enc_tg)
    .execute(&st.db)
    .await
    .unwrap();

    let president = login(&st, "president@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let ro_alerts = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&president),
        None,
    )
    .await;
    assert_eq!(ro_alerts.status, StatusCode::OK, "{}", ro_alerts.json);
    assert_eq!(ro_alerts.json["telegram_bot_configured"], true);
    assert_eq!(ro_alerts.json["telegram_webhook_secret"], "");
    assert_eq!(
        ro_alerts.json["telegram_webhook_path"],
        "/api/push/telegram/webhook/<redacted>"
    );

    let rw_alerts = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(rw_alerts.status, StatusCode::OK, "{}", rw_alerts.json);
    assert!(
        !rw_alerts.json["telegram_webhook_secret"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "AdminSettings.Update holder should still receive telegram_webhook_secret"
    );

    // M-08: Pagination multiplication overflow safety & picker-mode role/PII stripping.
    let big_page = call(
        &app,
        Method::GET,
        "/api/admin/users?page=999999999999999&limit=100",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(big_page.status, StatusCode::OK, "{}", big_page.json);
    assert_eq!(big_page.json["items"].as_array().unwrap().len(), 0);

    // Auditor holds AdminReports.View without AdminMembers.View -> picker mode.
    let auditor = login(&st, "auditor@example.org").await;
    let picker = call(
        &app,
        Method::GET,
        "/api/admin/users?role=superadmin",
        Some(&auditor),
        None,
    )
    .await;
    assert_eq!(picker.status, StatusCode::OK, "{}", picker.json);
    let items = picker.json.as_array().unwrap();
    assert!(
        items.len() > 1,
        "picker mode must ignore role filter to prevent role enumeration"
    );
    for item in items {
        let obj = item.as_object().unwrap();
        assert_eq!(obj.len(), 2, "picker item must expose only id and name");
        assert!(obj.contains_key("id") && obj.contains_key("name"));
    }

    // M-14: SSRF validator rejects loopback, metadata, IPv4-mapped IPv6, ULA, link-local, CGNAT, and non-HTTP ports.
    for blocked in [
        "http://127.0.0.1/hook",
        "https://169.254.169.254/latest/meta-data",
        "https://[::ffff:10.0.0.1]/hook",
        "https://[fc00::1]/hook",
        "https://[fe80::1]/hook",
        "https://100.64.0.1/hook",
        "https://metadata.google.internal/computeMetadata/v1/",
        "https://example.com:22/hook",
    ] {
        assert!(
            grace_ride::notifications::resolve_and_validate_http_target(blocked, 1000, false)
                .await
                .is_none(),
            "SSRF validator must reject {blocked}"
        );
    }

    // M-15: strip_jpeg_metadata strips APP1/EXIF, APP2, COM, and trailing post-EOI bytes.
    let mut jpeg = Vec::new();
    jpeg.extend_from_slice(&[0xFF, 0xD8]); // SOI
    let exif_payload = b"Exif\0\0GPS_SECRET";
    jpeg.extend_from_slice(&[0xFF, 0xE1]);
    jpeg.extend_from_slice(&((exif_payload.len() + 2) as u16).to_be_bytes());
    jpeg.extend_from_slice(exif_payload);
    let app2_payload = b"APP2_ICC_META";
    jpeg.extend_from_slice(&[0xFF, 0xE2]);
    jpeg.extend_from_slice(&((app2_payload.len() + 2) as u16).to_be_bytes());
    jpeg.extend_from_slice(app2_payload);
    let com_payload = b"SECRET_COMMENT";
    jpeg.extend_from_slice(&[0xFF, 0xFE]);
    jpeg.extend_from_slice(&((com_payload.len() + 2) as u16).to_be_bytes());
    jpeg.extend_from_slice(com_payload);
    // Minimal SOS header + entropy scan + EOI + trailing Motion Photo bytes
    jpeg.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x04, 0x00, 0x00, 0x11, 0x22, 0xFF, 0xD9]);
    jpeg.extend_from_slice(b"TRAILING_MOTION_PHOTO");

    let cleaned = grace_ride::routes::files::strip_jpeg_metadata(&jpeg)
        .expect("strip_jpeg_metadata must strip APP1/APP2/COM/trailing bytes");
    assert!(cleaned.starts_with(&[0xFF, 0xD8]));
    assert!(cleaned.ends_with(&[0xFF, 0xD9]));
    assert!(!cleaned.windows(4).any(|w| w == b"Exif"));
    assert!(!cleaned.windows(9).any(|w| w == b"APP2_ICC_"));
    assert!(!cleaned.windows(14).any(|w| w == b"SECRET_COMMENT"));
    assert!(!cleaned.windows(8).any(|w| w == b"TRAILING"));

    // M-17 / L-12: scripts/backup.sh must fail closed when REQUIRE_ENCRYPTION is unset and no recipient is set.
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let script = repo_root.join("scripts/backup.sh");
    let tmp_base =
        std::env::temp_dir().join(format!("gr_backup_failclosed_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_base);
    std::fs::create_dir_all(tmp_base.join("data")).unwrap();
    let out = Command::new("bash")
        .arg(&script)
        .arg(tmp_base.join("data"))
        .arg(tmp_base.join("out"))
        .arg("7")
        .env("DATABASE_URL", "postgres://localhost/grace_ride_test")
        .env_remove("REQUIRE_ENCRYPTION")
        .env_remove("BACKUP_AGE_RECIPIENT")
        .env_remove("BACKUP_GPG_RECIPIENT")
        .output()
        .expect("run scripts/backup.sh");
    let _ = std::fs::remove_dir_all(&tmp_base);
    assert!(
        !out.status.success(),
        "backup.sh must fail closed without encryption recipient"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("REQUIRE_ENCRYPTION=true"),
        "expected encryption-required stderr, got: {stderr}"
    );
}
