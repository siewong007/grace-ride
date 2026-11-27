//! End-to-end regression tests for the BugHunter security audit remediations
//! (H-01..H-09, M-02..M-05, M-08, M-14, M-15, and fail-closed backup encryption).

mod common;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::Command;

use axum::body::Body;
use axum::http::{header, header::HeaderName, HeaderMap, HeaderValue, Method, Request, StatusCode};
use base64::Engine as _;
use common::*;
use http_body_util::BodyExt;
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
use serde_json::json;
use tower::ServiceExt;

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

async fn insert_all_audience_event(
    st: &grace_ride::state::AppState,
    created_by: i64,
    title: &str,
    status: &str,
    mileage_claimable: i64,
) -> i64 {
    let ev = insert_event(st, created_by, title, status, mileage_claimable).await;
    sqlx::query(
        "INSERT INTO event_groups (event_id, group_id)
         SELECT $1, id FROM groups WHERE is_all = 1
         ON CONFLICT DO NOTHING",
    )
    .bind(ev)
    .execute(&st.db)
    .await
    .unwrap();
    ev
}

#[tokio::test]
async fn pass2_rbac_recovery_dual_control_and_vp_president_ack() {
    let (app, st) = setup().await;
    let president = login(&st, "president@example.org").await;
    let vp = login(&st, "vicepresident@example.org").await;
    let pres_id = user_id(&st, "president@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let sophie_id = user_id(&st, "sophie@example.org").await;

    // 1. President creates two unprivileged custom roles: one for recovery guard, one for VP ack guard.
    let cr1 = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&president),
        Some(json!({
            "name": "Recovery Guard Role",
            "description": "Unprivileged",
            "permissions": ["rides.view"]
        })),
    )
    .await;
    assert_eq!(cr1.status, StatusCode::OK, "{}", cr1.json);
    let key1 = cr1.json["key"].as_str().unwrap().to_string();

    let cr2 = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&president),
        Some(json!({
            "name": "VP Ack Role",
            "description": "Unprivileged",
            "permissions": ["rides.view"]
        })),
    )
    .await;
    assert_eq!(cr2.status, StatusCode::OK, "{}", cr2.json);
    let key2 = cr2.json["key"].as_str().unwrap().to_string();

    let assign1 = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}"),
        Some(&president),
        Some(json!({ "roles": [key1], "status": "active" })),
    )
    .await;
    assert_eq!(assign1.status, StatusCode::OK, "{}", assign1.json);

    let assign2 = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{sophie_id}"),
        Some(&president),
        Some(json!({ "roles": [key2], "status": "active" })),
    )
    .await;
    assert_eq!(assign2.status, StatusCode::OK, "{}", assign2.json);

    // Simulate President having completed recovery for `member_id` within the past 72h.
    let tok_hash = grace_ride::auth::hash_token(&uuid::Uuid::new_v4().simple().to_string());
    sqlx::query(
        "INSERT INTO invites (token, email, roles, created_by, user_id, kind, expires_at, used_at)
         VALUES ($1, 'member@example.org', 'member', $2, $3, 'recovery', $4, $5)",
    )
    .bind(tok_hash)
    .bind(pres_id)
    .bind(member_id)
    .bind(tomorrow("23:59"))
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();

    // President cannot bypass dual-control by upgrading `key1` to privileged permissions.
    let bypass = call(
        &app,
        Method::PUT,
        &format!("/api/admin/roles/{key1}"),
        Some(&president),
        Some(json!({
            "name": "Recovery Guard Role",
            "description": "Upgraded",
            "permissions": ["rides.view", "admin_rides.view"]
        })),
    )
    .await;
    assert_eq!(bypass.status, StatusCode::FORBIDDEN, "{}", bypass.json);
    assert_eq!(bypass.json["code"], "SECOND_ADMIN_REQUIRED");

    // 2. Vice-President adding admin power to a role held by non-admin requires president_acknowledged.
    let vp_no_ack = call(
        &app,
        Method::PUT,
        &format!("/api/admin/roles/{key2}"),
        Some(&vp),
        Some(json!({
            "name": "VP Ack Role",
            "description": "With admin power",
            "permissions": ["rides.view", "admin_roles.update"]
        })),
    )
    .await;
    assert_eq!(
        vp_no_ack.status,
        StatusCode::FORBIDDEN,
        "{}",
        vp_no_ack.json
    );
    assert_eq!(vp_no_ack.json["code"], "PRESIDENT_ACKNOWLEDGEMENT_REQUIRED");

    let vp_with_ack = call(
        &app,
        Method::PUT,
        &format!("/api/admin/roles/{key2}"),
        Some(&vp),
        Some(json!({
            "name": "VP Ack Role",
            "description": "With admin power",
            "permissions": ["rides.view", "admin_roles.update"],
            "president_acknowledged": true
        })),
    )
    .await;
    assert_eq!(vp_with_ack.status, StatusCode::OK, "{}", vp_with_ack.json);
}

#[tokio::test]
async fn pass2_google_reset_prompt_and_biometric_reauth_session_binding() {
    // 1. Google password reset uses prompt=login and max_age=0.
    let cfg = grace_ride::google_oauth::GoogleOAuthConfig {
        client_id: Some("cid-test".into()),
        client_secret: Some("csec-test".into()),
        ..Default::default()
    };
    let oauth = grace_ride::google_oauth::GoogleOAuthClient::new(cfg, "https://graceride.test");
    let reset_url = oauth
        .authorization_url_with_prompt(
            "state-rst",
            "verifier-0123456789abcdefghijklmnopqrstuvwxyzABCDEFG",
            None,
            "login",
            Some(0),
        )
        .unwrap();
    let parsed = url::Url::parse(&reset_url).unwrap();
    let params: std::collections::HashMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    assert_eq!(params.get("prompt").map(String::as_str), Some("login"));
    assert_eq!(params.get("max_age").map(String::as_str), Some("0"));

    let login_url = oauth
        .authorization_url(
            "state-log",
            "verifier-0123456789abcdefghijklmnopqrstuvwxyzABCDEFG",
            None,
        )
        .unwrap();
    let parsed_login = url::Url::parse(&login_url).unwrap();
    let login_params: std::collections::HashMap<String, String> =
        parsed_login.query_pairs().into_owned().collect();
    assert_eq!(
        login_params.get("prompt").map(String::as_str),
        Some("select_account")
    );
    assert!(!login_params.contains_key("max_age"));

    // 2. Biometric reauth challenge is strictly bound to the originating session.
    let (app, st) = setup().await;
    let cookie_a = login(&st, "member@example.org").await;
    let cookie_b = login(&st, "member@example.org").await;

    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
    let pair =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng).unwrap();
    let spki_prefix: [u8; 26] = [
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ];
    let mut spki = Vec::with_capacity(91);
    spki.extend_from_slice(&spki_prefix);
    spki.extend_from_slice(pair.public_key().as_ref());
    let spki_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(spki);

    let reg_start = call(
        &app,
        Method::POST,
        "/api/auth/biometric/register/start",
        Some(&cookie_a),
        None,
    )
    .await;
    assert_eq!(reg_start.status, StatusCode::OK, "{}", reg_start.json);
    let reg_ch = reg_start.json["challenge"].as_str().unwrap();
    let reg_sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        pair.sign(
            &rng,
            format!("graceride-biometric-v1:register:{reg_ch}").as_bytes(),
        )
        .unwrap()
        .as_ref(),
    );
    let reg_finish = call(
        &app,
        Method::POST,
        "/api/auth/biometric/register/finish",
        Some(&cookie_a),
        Some(json!({
            "challenge_id": reg_start.json["challenge_id"],
            "public_key": spki_b64,
            "signature": reg_sig,
            "label": "Pass2 Key",
        })),
    )
    .await;
    assert_eq!(reg_finish.status, StatusCode::OK, "{}", reg_finish.json);
    let cookie_a = cookie_from(&reg_finish).expect("rotated biometric cookie");
    let cred_id = reg_finish.json["credential_id"]
        .as_str()
        .unwrap()
        .to_string();

    let reauth_start = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/start",
        Some(&cookie_a),
        Some(json!({ "credential_id": cred_id })),
    )
    .await;
    assert_eq!(reauth_start.status, StatusCode::OK, "{}", reauth_start.json);
    let reauth_ch = reauth_start.json["challenge"].as_str().unwrap();
    let reauth_sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        pair.sign(
            &rng,
            format!("graceride-biometric-v1:reauth:{reauth_ch}").as_bytes(),
        )
        .unwrap()
        .as_ref(),
    );

    let cross_session = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/finish",
        Some(&cookie_b),
        Some(json!({
            "challenge_id": reauth_start.json["challenge_id"],
            "credential_id": cred_id,
            "signature": reauth_sig,
        })),
    )
    .await;
    assert_eq!(
        cross_session.status,
        StatusCode::FORBIDDEN,
        "{}",
        cross_session.json
    );
    assert_eq!(cross_session.json["code"], "CHALLENGE_MISMATCH");

    let reauth_start2 = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/start",
        Some(&cookie_a),
        Some(json!({ "credential_id": cred_id })),
    )
    .await;
    let reauth_ch2 = reauth_start2.json["challenge"].as_str().unwrap();
    let reauth_sig2 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        pair.sign(
            &rng,
            format!("graceride-biometric-v1:reauth:{reauth_ch2}").as_bytes(),
        )
        .unwrap()
        .as_ref(),
    );
    let same_session = call(
        &app,
        Method::POST,
        "/api/auth/biometric/reauth/finish",
        Some(&cookie_a),
        Some(json!({
            "challenge_id": reauth_start2.json["challenge_id"],
            "credential_id": cred_id,
            "signature": reauth_sig2,
        })),
    )
    .await;
    assert_eq!(same_session.status, StatusCode::OK, "{}", same_session.json);
}

#[tokio::test]
async fn pass2_visitor_carpool_and_driver_capacity_reduction_guards() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let head = login(&st, "traffichead@example.org").await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;

    let admin_id = user_id(&st, "admin@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let area = first_area_id(&st).await;

    ensure_driver_vehicle(&st, driver_id, "WXX1122").await;
    sqlx::query(
        "INSERT INTO driver_areas (user_id, area_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(driver_id)
    .bind(area)
    .execute(&st.db)
    .await
    .unwrap();

    // Turn off carpool first, create an active booking on a published event.
    let prof_off = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "carpool_enabled": false,
            "spare_seats": 3,
            "area_ids": [area]
        })),
    )
    .await;
    assert_eq!(prof_off.status, StatusCode::OK, "{}", prof_off.json);

    let ev =
        insert_all_audience_event(&st, admin_id, "Carpool Visitor Event", "published", 1).await;
    signup_going(&st, ev, member_id).await;
    signup_going(&st, ev, driver_id).await;

    let ride_res = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev,
            "leg": "outbound",
            "pickup_at": tomorrow("09:00"),
            "pickup_area_id": area,
            "pickup_address": "10 SS2",
            "destination": "Church",
            "seats": 2
        })),
    )
    .await;
    assert_eq!(ride_res.status, StatusCode::OK, "{}", ride_res.json);
    let ride_id = ride_res.json["id"].as_i64().unwrap();

    let claim = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/claim"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(claim.status, StatusCode::OK, "{}", claim.json);

    // Assigning a visitor to a carpool_enabled=0 driver who already has a ride booking must fail.
    let bad_vis = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{ev}/plan/visitors"),
        Some(&head),
        Some(json!({
            "name": "Visitor One",
            "phone": "0123334444",
            "area_id": area,
            "pickup_address": "12 SS2",
            "seats": 1,
            "leg": "outbound",
            "driver_id": driver_id
        })),
    )
    .await;
    assert_ne!(bad_vis.status, StatusCode::OK, "{}", bad_vis.json);

    // Re-enable carpool=1, approve ride (so 2 active seats are loaded on driver).
    let prof_on = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 3,
            "area_ids": [area]
        })),
    )
    .await;
    assert_eq!(prof_on.status, StatusCode::OK, "{}", prof_on.json);
    let accept_ride = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/respond"),
        Some(&member),
        Some(json!({ "accept": true })),
    )
    .await;
    assert_eq!(accept_ride.status, StatusCode::OK, "{}", accept_ride.json);
    let approve = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/approve"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(approve.status, StatusCode::OK, "{}", approve.json);

    // Reducing spare_seats below active load (2 -> 1) must be rejected.
    let shrink_profile = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 1,
            "area_ids": [area]
        })),
    )
    .await;
    assert_eq!(
        shrink_profile.status,
        StatusCode::BAD_REQUEST,
        "{}",
        shrink_profile.json
    );
    assert_eq!(shrink_profile.json["code"], "NOT_ENOUGH_DRIVER_SEATS");

    // Selecting a tiny vehicle whose max spare capacity (seats - 1 = 1) < active load (2) must fail.
    let small_v = call(
        &app,
        Method::POST,
        "/api/driver/vehicles",
        Some(&driver),
        Some(json!({
            "model": "Smart Two",
            "color": "Red",
            "plate": "SM2026",
            "seats": 2,
            "vehicle_type": "standard"
        })),
    )
    .await;
    assert_eq!(small_v.status, StatusCode::OK, "{}", small_v.json);
    let small_vid = small_v.json["id"].as_i64().unwrap();
    let sel_small = call(
        &app,
        Method::POST,
        &format!("/api/driver/vehicles/{small_vid}/select"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(
        sel_small.status,
        StatusCode::BAD_REQUEST,
        "{}",
        sel_small.json
    );
    assert_eq!(sel_small.json["code"], "NOT_ENOUGH_DRIVER_SEATS");

    // Remove any seeded spare larger vehicles so ONLY active vehicle and small SM2026 exist.
    let active_vid: i64 =
        sqlx::query_scalar("SELECT id FROM vehicles WHERE user_id = $1 AND is_active = 1")
            .bind(driver_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    sqlx::query("DELETE FROM vehicles WHERE user_id = $1 AND id <> $2 AND id <> $3")
        .bind(driver_id)
        .bind(active_vid)
        .bind(small_vid)
        .execute(&st.db)
        .await
        .unwrap();

    // Deleting the active vehicle while only smaller vehicle SM2026 remains must fail.
    let del_active = call(
        &app,
        Method::DELETE,
        &format!("/api/driver/vehicles/{active_vid}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(
        del_active.status,
        StatusCode::BAD_REQUEST,
        "{}",
        del_active.json
    );

    let _ = admin;
}

#[tokio::test]
async fn pass2_cancelled_event_guards_overpayment_finance_and_fare_transfer() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let head = login(&st, "traffichead@example.org").await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let _driver = login(&st, "driver@example.org").await;
    let finance = login(&st, "finance@example.org").await;

    let admin_id = user_id(&st, "admin@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let sophie_id = user_id(&st, "sophie@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let area = first_area_id(&st).await;

    ensure_driver_vehicle(&st, driver_id, "FARE100").await;

    // 1. Settlement downward revision preserves paid_cents and exposes overpaid_cents in finance report.
    let ev = insert_all_audience_event(&st, admin_id, "Badminton Settlement", "published", 1).await;
    signup_going(&st, ev, member_id).await;
    signup_going(&st, ev, sophie_id).await;
    signup_going(&st, ev, driver_id).await;
    sqlx::query("UPDATE event_signups SET attended = 1 WHERE event_id = $1")
        .bind(ev)
        .execute(&st.db)
        .await
        .unwrap();

    let settle_res = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/settlements"),
        Some(&admin),
        Some(json!({
            "title": "Hall Split",
            "items": [{"label": "Court", "amount_cents": 2000}],
            "subsidy_cents": 0,
            "split_mode": "equal",
            "participants": [{"user_id": member_id}, {"user_id": sophie_id}],
            "payee_name": "Organiser",
            "payee_account": "012-3456789"
        })),
    )
    .await;
    assert_eq!(settle_res.status, StatusCode::OK, "{}", settle_res.json);
    let sid = settle_res.json["id"].as_i64().unwrap();
    let pub_res = call(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(pub_res.status, StatusCode::OK, "{}", pub_res.json);

    let proof_file = upload_png(&app, &member).await;
    let member_share_id: i64 = sqlx::query_scalar(
        "SELECT id FROM settlement_shares WHERE settlement_id = $1 AND user_id = $2",
    )
    .bind(sid)
    .bind(member_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let proof_res = call(
        &app,
        Method::POST,
        &format!("/api/shares/{member_share_id}/proof"),
        Some(&member),
        Some(json!({ "proof_file_id": proof_file, "txn_ref": "PASS2-OVERPAY-1" })),
    )
    .await;
    assert_eq!(proof_res.status, StatusCode::OK, "{}", proof_res.json);
    let ver_res = call(
        &app,
        Method::POST,
        &format!("/api/shares/{member_share_id}/verify"),
        Some(&admin),
        Some(json!({ "action": "confirm" })),
    )
    .await;
    assert_eq!(ver_res.status, StatusCode::OK, "{}", ver_res.json);

    // Revise court fee down from 2000 -> 1000 (member's share drops 1000 -> 500, paid_cents stays 1000).
    let rev_res = call(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&admin),
        Some(json!({
            "title": "Hall Split Revised",
            "items": [{"label": "Court", "amount_cents": 1000}],
            "subsidy_cents": 0,
            "split_mode": "equal",
            "participants": [{"user_id": member_id}, {"user_id": sophie_id}],
            "payee_name": "Organiser",
            "payee_account": "012-3456789",
            "reason": "Discount received"
        })),
    )
    .await;
    assert_eq!(rev_res.status, StatusCode::OK, "{}", rev_res.json);

    let my_shares = call(&app, Method::GET, "/api/me/shares", Some(&member), None).await;
    assert_eq!(my_shares.status, StatusCode::OK, "{}", my_shares.json);
    let mine = my_shares
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == member_share_id)
        .unwrap();
    assert_eq!(mine["paid_cents"], 1000);

    let ev_day = tomorrow("09:00")[..10].to_string();
    let fin_before = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/finance?from={ev_day}&to={ev_day}"),
        Some(&finance),
        None,
    )
    .await;
    assert_eq!(fin_before.status, StatusCode::OK, "{}", fin_before.json);
    assert!(
        fin_before.json["bills"]["totals"]["paid_cents"]
            .as_i64()
            .unwrap()
            >= 1000,
        "{}",
        fin_before.json
    );
    assert!(
        fin_before.json["bills"]["totals"]["overpaid_cents"]
            .as_i64()
            .unwrap()
            >= 500,
        "{}",
        fin_before.json
    );

    // 2. Create pending ride & unbatched post-event mileage claim, then cancel event.
    let pending_ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&sophie),
        Some(json!({
            "event_id": ev,
            "leg": "outbound",
            "pickup_at": tomorrow("09:00"),
            "pickup_area_id": area,
            "pickup_address": "22 SS2",
            "destination": "Church",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(pending_ride.status, StatusCode::OK, "{}", pending_ride.json);
    let prid = pending_ride.json["id"].as_i64().unwrap();
    sqlx::query("UPDATE rides SET driver_id = $1, status = 'pending_passenger' WHERE id = $2")
        .bind(driver_id)
        .bind(prid)
        .execute(&st.db)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, destination_area_id, trip_type, amount_cents, description, status, kind, created_at, updated_at)
         VALUES ($1, $2, $3, 'standard', 1200, 'Cancelled event mileage', 'approved', 'mileage', $4, $4)",
    )
    .bind(driver_id)
    .bind(ev)
    .bind(area)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    let reimb_id: i64 = sqlx::query_scalar(
        "SELECT id FROM reimbursements WHERE event_id = $1 AND kind = 'mileage'",
    )
    .bind(ev)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let cancel_ev = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/cancel"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(cancel_ev.status, StatusCode::OK, "{}", cancel_ev.json);

    // Unpaid unbatched mileage claim was automatically rejected upon event cancellation.
    let claim_status: String =
        sqlx::query_scalar("SELECT status FROM reimbursements WHERE id = $1")
            .bind(reimb_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(claim_status, "rejected");

    // Approving a pending claim or paying/batching an approved claim on a cancelled event must fail.
    let pend_reimb_id: i64 = sqlx::query_scalar(
        "INSERT INTO reimbursements (user_id, event_id, destination_area_id, trip_type, amount_cents, description, status, kind, created_at, updated_at)
         VALUES ($1, $2, $3, 'standard', 1200, 'Pending on cancelled', 'pending', 'mileage', $4, $4) RETURNING id",
    )
    .bind(member_id)
    .bind(ev)
    .bind(area)
    .bind(grace_ride::util::now())
    .fetch_one(&st.db)
    .await
    .unwrap();
    // Duplicate live post-event mileage claim for same (event, user) must be blocked by 0019 unique index.
    let dup_post_mileage = sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, destination_area_id, trip_type, amount_cents, description, status, kind, created_at, updated_at)
         VALUES ($1, $2, $3, 'standard', 1200, 'Dup post mileage', 'approved', 'mileage', $4, $4)",
    )
    .bind(member_id)
    .bind(ev)
    .bind(area)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await;
    assert!(
        dup_post_mileage.is_err(),
        "uq_reimbursements_post_event_mileage must reject duplicate"
    );

    let app_reimb_id: i64 = sqlx::query_scalar(
        "INSERT INTO reimbursements (user_id, event_id, destination_area_id, trip_type, amount_cents, description, status, kind, created_at, updated_at)
         VALUES ($1, $2, $3, 'standard', 1200, 'Approved on cancelled', 'approved', 'mileage', $4, $4) RETURNING id",
    )
    .bind(sophie_id)
    .bind(ev)
    .bind(area)
    .bind(grace_ride::util::now())
    .fetch_one(&st.db)
    .await
    .unwrap();
    let bad_rev = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{pend_reimb_id}/review"),
        Some(&head),
        Some(json!({ "action": "approve", "note": "" })),
    )
    .await;
    assert_eq!(bad_rev.status, StatusCode::BAD_REQUEST, "{}", bad_rev.json);
    let bad_pay = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{app_reimb_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "CAN-1" })),
    )
    .await;
    assert_eq!(bad_pay.status, StatusCode::BAD_REQUEST, "{}", bad_pay.json);
    let bad_batch = call(
        &app,
        Method::POST,
        "/api/reimbursements/batches",
        Some(&finance),
        Some(json!({ "claim_ids": [app_reimb_id] })),
    )
    .await;
    assert_eq!(
        bad_batch.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_batch.json
    );

    // Cancelled event rejects new settlements, visitor passes, passenger accept, and finance stats exclude it.
    let bad_settle = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/settlements"),
        Some(&admin),
        Some(json!({
            "title": "Blocked",
            "items": [{"label": "Court", "amount_cents": 500}],
            "subsidy_cents": 0,
            "split_mode": "equal",
            "participants": [{"user_id": member_id}],
            "payee_name": "Organiser",
            "payee_account": "012-3456789"
        })),
    )
    .await;
    assert_eq!(
        bad_settle.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_settle.json
    );

    let bad_vis = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev}/visitors"),
        Some(&member),
        Some(json!({
            "name": "Late Guest",
            "phone": "0129998888",
            "pickup_at": tomorrow("09:00"),
            "pickup_area_id": area,
            "pickup_address": "1 SS2"
        })),
    )
    .await;
    assert_eq!(bad_vis.status, StatusCode::BAD_REQUEST, "{}", bad_vis.json);

    sqlx::query("UPDATE rides SET status = 'pending_passenger' WHERE id = $1")
        .bind(prid)
        .execute(&st.db)
        .await
        .unwrap();
    let bad_accept = call(
        &app,
        Method::POST,
        &format!("/api/rides/{prid}/respond"),
        Some(&sophie),
        Some(json!({ "accept": true })),
    )
    .await;
    assert_eq!(
        bad_accept.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_accept.json
    );

    let fin_after = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/finance?from={ev_day}&to={ev_day}"),
        Some(&finance),
        None,
    )
    .await;
    let listed = fin_after.json["bills"]["settlements"].as_array().unwrap();
    assert!(
        !listed.iter().any(|s| s["event_id"] == ev),
        "cancelled event settlement must be excluded from finance report"
    );

    // 3. Excluding a completed carpool ride transfers fare to eligible sibling completed ride on same trip.
    let ev2 = insert_all_audience_event(&st, admin_id, "Transfer Fare Event", "published", 1).await;
    let trip_id: i64 = 90_099;
    sqlx::query("INSERT INTO trips (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(trip_id)
        .execute(&st.db)
        .await
        .unwrap();
    let r1: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, trip_id, leg, pickup_at, pickup_area_id, pickup_address, destination, seats, status, reimbursement_amount_cents, reimbursement_excluded)
         VALUES ($1, $2, $3, $4, 'outbound', $5, $6, 'A1', 'Hall', 1, 'completed', 1500, 0) RETURNING id",
    )
    .bind(member_id)
    .bind(driver_id)
    .bind(ev2)
    .bind(trip_id)
    .bind(tomorrow("09:00"))
    .bind(area)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let r2: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, trip_id, leg, pickup_at, pickup_area_id, pickup_address, destination, seats, status, reimbursement_amount_cents, reimbursement_excluded)
         VALUES ($1, $2, $3, $4, 'outbound', $5, $6, 'A2', 'Hall', 1, 'completed', 0, 0) RETURNING id",
    )
    .bind(sophie_id)
    .bind(driver_id)
    .bind(ev2)
    .bind(trip_id)
    .bind(tomorrow("09:00"))
    .bind(area)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let excl_res = call(
        &app,
        Method::PUT,
        &format!("/api/rides/{r1}/reimbursement-excluded"),
        Some(&head),
        Some(json!({ "excluded": true })),
    )
    .await;
    assert_eq!(excl_res.status, StatusCode::OK, "{}", excl_res.json);
    let amounts: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT id, reimbursement_excluded, reimbursement_amount_cents FROM rides WHERE id IN ($1, $2) ORDER BY id",
    )
    .bind(r1)
    .bind(r2)
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(amounts, vec![(r1, 1, 0), (r2, 0, 1500)]);
}

#[tokio::test]
async fn pass2_proxy_case_caregiver_files_robots_sse_and_bounded_http() {
    use tokio::io::AsyncWriteExt;

    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let sophie_id = user_id(&st, "sophie@example.org").await;
    let area = first_area_id(&st).await;

    // 1. Proxy rider case/whitespace duplicate conflict & case-insensitive caregiver care link access.
    let p1 = call(
        &app,
        Method::POST,
        "/api/proxy-riders",
        Some(&member),
        Some(json!({ "name": "Alice Tan", "caregiver_user_id": sophie_id })),
    )
    .await;
    assert_eq!(p1.status, StatusCode::OK, "{}", p1.json);

    let p_dup = call(
        &app,
        Method::POST,
        "/api/proxy-riders",
        Some(&member),
        Some(json!({ "name": "  alice tan  " })),
    )
    .await;
    assert_eq!(p_dup.status, StatusCode::CONFLICT, "{}", p_dup.json);

    let ev = insert_all_audience_event(
        &st,
        user_id(&st, "admin@example.org").await,
        "Care Ev",
        "published",
        0,
    )
    .await;
    signup_going(&st, ev, member_id).await;
    let pr = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev,
            "leg": "outbound",
            "pickup_at": &grace_ride::util::in_hours(1)[..16],
            "pickup_area_id": area,
            "pickup_address": "18 SS2",
            "destination": "Church",
            "seats": 1,
            "is_proxy": true,
            "rider_name": "  ALICE TAN  "
        })),
    )
    .await;
    assert_eq!(pr.status, StatusCode::OK, "{}", pr.json);
    let prid = pr.json["id"].as_i64().unwrap();
    let cg_ok = call(
        &app,
        Method::POST,
        &format!("/api/rides/{prid}/care-link"),
        Some(&sophie),
        None,
    )
    .await;
    assert_eq!(cg_ok.status, StatusCode::OK, "{}", cg_ok.json);
    let care_url = cg_ok.json["url"].as_str().unwrap();
    let care_tok = care_url.rsplit('/').next().unwrap();
    let cg_view = call(
        &app,
        Method::GET,
        &format!("/api/care/{care_tok}"),
        None,
        None,
    )
    .await;
    assert_eq!(cg_view.status, StatusCode::OK, "{}", cg_view.json);

    // 2. Reject malformed JPEG and malformed IHDR PNG; serve file CSP frame-ancestors 'none' & X-Robots-Tag.
    let upload_raw = |bytes: Vec<u8>, fname: &'static str, ct: &'static str| {
        let app = app.clone();
        let cookie = member.clone();
        async move {
            let boundary = "XgrPASS2FILE";
            let mut body = format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{fname}\"\r\nContent-Type: {ct}\r\n\r\n"
            )
            .into_bytes();
            body.extend_from_slice(&bytes);
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            let req = Request::builder()
                .method(Method::POST)
                .uri("/api/files")
                .header(header::COOKIE, cookie)
                .header(header::ORIGIN, ORIGIN)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap();
            app.oneshot(req).await.unwrap()
        }
    };
    let bad_jpg = upload_raw(vec![0xFF, 0xD8, 0xFF, 0xE1, 0xFF], "bad.jpg", "image/jpeg").await;
    assert_eq!(bad_jpg.status(), StatusCode::BAD_REQUEST);

    let mut bad_ihdr_png = b"\x89PNG\r\n\x1a\n".to_vec();
    bad_ihdr_png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D', b'R', 1, 2, 3]);
    let bad_png = upload_raw(bad_ihdr_png, "bad.png", "image/png").await;
    assert_eq!(bad_png.status(), StatusCode::BAD_REQUEST);

    let good_id = upload_png(&app, &member).await;
    let file_res = call(
        &app,
        Method::GET,
        &format!("/api/files/{good_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(file_res.status, StatusCode::OK);
    let csp = file_res.headers["content-security-policy"]
        .to_str()
        .unwrap();
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    assert_eq!(
        file_res.headers["x-robots-tag"].to_str().unwrap(),
        "noindex, nofollow, noarchive"
    );

    // 3. session_alive fails closed on closed pool + immediate SSE revocation via revoke_session/revoke_user.
    let raw_tok = member.strip_prefix("gr_session=").unwrap();
    let sess_hash = grace_ride::auth::hash_token(raw_tok);
    let sse_req = Request::builder()
        .method(Method::GET)
        .uri("/api/stream")
        .header(header::COOKIE, &member)
        .body(Body::empty())
        .unwrap();
    let sse_res = app.clone().oneshot(sse_req).await.unwrap();
    assert_eq!(sse_res.status(), StatusCode::OK);
    grace_ride::stream::revoke_session(&sess_hash);
    let drained = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sse_res.into_body().collect(),
    )
    .await
    .expect("SSE stream must close promptly upon revoke_session");
    assert!(drained.is_ok());

    // Bounded HTTP reader enforces max_bytes over real HTTP socket.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let srv = tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut req_buf = [0u8; 512];
            let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut req_buf).await;
            let body = "X".repeat(2048);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });
    let client = reqwest::Client::new();
    let http_res = client
        .get(format!("http://{addr}/big"))
        .send()
        .await
        .unwrap();
    let err = grace_ride::util::read_response_bytes_bounded(http_res, 512).await;
    assert!(err.is_err(), "expected bounded read error");
    srv.await.unwrap();

    // Closed DB pool fails session_alive closed.
    let closed_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&st.cfg.database_url)
        .unwrap();
    closed_pool.close().await;
    assert!(!grace_ride::stream::session_alive(&closed_pool, &sess_hash).await);
}
