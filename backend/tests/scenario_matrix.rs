//! Comprehensive scenario matrix across session auto-lock, pending/disabled account guards,
//! caregiver safe-arrival links, recurring schedules, round-trips & stroller matching, live
//! calendar feeds, member QR pass scanning, and finance payout batch concurrency/SOD.
mod common;

use axum::http::{header, Method, StatusCode};
use common::*;
use serde_json::json;

fn future_dt(days: i64, hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (grace_ride::util::today_local() + chrono::Duration::days(days)).format("%Y-%m-%d")
    )
}

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn first_area_id(st: &grace_ride::state::AppState) -> i64 {
    sqlx::query_scalar("SELECT id FROM areas WHERE active=1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn all_group_id(st: &grace_ride::state::AppState) -> i64 {
    sqlx::query_scalar("SELECT id FROM groups WHERE is_all=1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn idle_auto_lock_org_default_user_override_disabled_and_backup_code_unlock() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    // Org default inactivity timeout = 10 minutes
    sqlx::query(
        "INSERT INTO system_settings (key, value, updated_at) VALUES ('session_timeout_mins', '10', $1)
         ON CONFLICT (key) DO UPDATE SET value='10', updated_at=$1",
    )
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();

    // Enable TOTP to obtain single-use backup recovery codes while session is unlocked
    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(setup_res.status, StatusCode::OK, "{}", setup_res.json);
    let secret = setup_res.json["secret"].as_str().unwrap();
    let code = totp_code(secret, "member@example.org");
    let enable_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&member),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(enable_res.status, StatusCode::OK, "{}", enable_res.json);
    let backup_codes: Vec<String> = enable_res.json["backup_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(backup_codes.len(), 8);

    // 1. User has auto_lock_mins = 0 (inherit org default 10m): backdate last_seen_at by 12m -> 423 SESSION_LOCKED
    sqlx::query("UPDATE users SET auto_lock_mins=0 WHERE id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("UPDATE sessions SET locked_at=NULL, last_seen_at=$1 WHERE user_id=$2")
        .bind(grace_ride::util::minutes_ago(12))
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();

    let locked_rides = call(&app, Method::GET, "/api/rides", Some(&member), None).await;
    assert_eq!(
        locked_rides.status,
        StatusCode::LOCKED,
        "{}",
        locked_rides.json
    );
    assert_eq!(locked_rides.json["code"], "SESSION_LOCKED");

    let me_locked = call(&app, Method::GET, "/api/auth/me", Some(&member), None).await;
    assert_eq!(me_locked.status, StatusCode::OK);
    assert_eq!(me_locked.json["locked"], true);

    // Invalid TOTP / backup codes are rejected and leave session locked
    let bad_totp = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&member),
        Some(json!({ "totp_code": "000000" })),
    )
    .await;
    assert_eq!(
        bad_totp.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_totp.json
    );

    let bad_backup = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&member),
        Some(json!({ "backup_code": "BADCODE1" })),
    )
    .await;
    assert_eq!(
        bad_backup.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_backup.json
    );

    let still_locked = call(&app, Method::GET, "/api/rides", Some(&member), None).await;
    assert_eq!(still_locked.status, StatusCode::LOCKED);

    // Valid single-use backup code unlocks the session
    let ok_unlock = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&member),
        Some(json!({ "backup_code": backup_codes[0] })),
    )
    .await;
    assert_eq!(ok_unlock.status, StatusCode::OK, "{}", ok_unlock.json);

    let ok_rides = call(&app, Method::GET, "/api/rides", Some(&member), None).await;
    assert_eq!(ok_rides.status, StatusCode::OK, "{}", ok_rides.json);

    // 2. Explicit opt-out (auto_lock_mins = -1) bypasses org 10m timeout even after 120m
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&member),
            Some(json!({ "auto_lock_mins": -1 })),
        )
        .await
        .status,
        StatusCode::OK
    );
    sqlx::query("UPDATE sessions SET locked_at=NULL, last_seen_at=$1 WHERE user_id=$2")
        .bind(grace_ride::util::minutes_ago(120))
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    assert_eq!(
        call(&app, Method::GET, "/api/rides", Some(&member), None)
            .await
            .status,
        StatusCode::OK
    );

    // 3. Per-member shorter override (auto_lock_mins = 5) locks after 6m (< org 10m)
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&member),
            Some(json!({ "auto_lock_mins": 5 })),
        )
        .await
        .status,
        StatusCode::OK
    );
    sqlx::query("UPDATE sessions SET locked_at=NULL, last_seen_at=$1 WHERE user_id=$2")
        .bind(grace_ride::util::minutes_ago(6))
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let short_lock = call(&app, Method::GET, "/api/rides", Some(&member), None).await;
    assert_eq!(short_lock.status, StatusCode::LOCKED);

    // Reusing already-consumed backup_codes[0] fails; unused backup_codes[1] succeeds
    let reuse = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&member),
        Some(json!({ "backup_code": backup_codes[0] })),
    )
    .await;
    assert_eq!(reuse.status, StatusCode::BAD_REQUEST, "{}", reuse.json);

    let ok_unlock_2 = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&member),
        Some(json!({ "backup_code": backup_codes[1] })),
    )
    .await;
    assert_eq!(ok_unlock_2.status, StatusCode::OK, "{}", ok_unlock_2.json);
    assert_eq!(
        call(&app, Method::GET, "/api/rides", Some(&member), None)
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn pending_and_disabled_account_api_guards() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;

    let inv = call(&app, Method::POST, "/api/admin/invites", Some(&admin), None).await;
    assert_eq!(inv.status, StatusCode::OK, "{}", inv.json);
    let token = inv.json["token"].as_str().unwrap();

    let acc = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "Pending Tester",
            "email": "pending.tester@example.org",
            "phone": "+60129876543",
            "terms": true
        })),
    )
    .await;
    assert_eq!(acc.status, StatusCode::OK, "{}", acc.json);

    let pending_cookie = acc
        .headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|s| s.split(';').next())
        .find(|kv| !kv.ends_with('='))
        .expect("session cookie from invite accept")
        .to_string();

    // Profile, /me, and passkey routes remain accessible while pending
    let me = call(
        &app,
        Method::GET,
        "/api/auth/me",
        Some(&pending_cookie),
        None,
    )
    .await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json["status"], "pending");
    let uid = me.json["id"].as_i64().unwrap();

    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&pending_cookie),
            Some(json!({ "name": "Pending Updated" })),
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            "/api/auth/passkeys",
            Some(&pending_cookie),
            None
        )
        .await
        .status,
        StatusCode::OK
    );

    // Every member operational API is blocked for pending users
    for (method, path, body) in [
        (Method::GET, "/api/rides", None),
        (
            Method::POST,
            "/api/rides",
            Some(
                json!({"pickup_at": future_dt(1, "09:00"), "pickup_area_id": 1, "pickup_address": "x", "destination": "Church", "seats": 1}),
            ),
        ),
        (Method::GET, "/api/events", None),
        (Method::GET, "/api/settlements", None),
        (Method::GET, "/api/reimbursements", None),
        (Method::GET, "/api/help", None),
    ] {
        let r = call(&app, method, path, Some(&pending_cookie), body).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{path}: {}", r.json);
        assert_eq!(r.json["code"], "ACCOUNT_PENDING", "{path}: {}", r.json);
    }

    // Approve account -> active -> operational routes open
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{uid}"),
            Some(&admin),
            Some(json!({ "roles": [], "status": "active" })),
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, Method::GET, "/api/rides", Some(&pending_cookie), None)
            .await
            .status,
        StatusCode::OK
    );

    // Disable account -> session rejected with 401
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{uid}"),
            Some(&admin),
            Some(json!({ "roles": [], "status": "disabled" })),
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, Method::GET, "/api/rides", Some(&pending_cookie), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn caregiver_pass_lifecycle_privacy_reissue_revoke_and_expiry() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let outsider = login(&st, "amina@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let area = first_area_id(&st).await;

    let ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": future_dt(1, "09:30"),
            "pickup_area_id": area,
            "pickup_address": "88 Secret Home Street",
            "destination": "PJ State CMC",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(ride.status, StatusCode::OK, "{}", ride.json);
    let ride_id = ride.json["id"].as_i64().unwrap();

    // Unrelated user cannot mint or revoke another member's care link
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{ride_id}/care-link"),
            Some(&outsider),
            Some(json!({ "label": "Snoop" })),
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/rides/{ride_id}/care-link"),
            Some(&outsider),
            None,
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Mint link #1 while pending_driver: hides street address & driver info
    let link1 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/care-link"),
        Some(&member),
        Some(json!({ "label": "Care View 1" })),
    )
    .await;
    assert_eq!(link1.status, StatusCode::OK, "{}", link1.json);
    let tok1 = link1.json["token"].as_str().unwrap().to_string();

    let s1 = call(&app, Method::GET, &format!("/api/care/{tok1}"), None, None).await;
    assert_eq!(s1.status, StatusCode::OK, "{}", s1.json);
    assert!(s1.json["driver"].is_null());
    assert!(s1.json.get("pickup_address").is_none());
    let dumped1 = s1.json.to_string();
    assert!(!dumped1.contains("88 Secret Home Street"));

    // Reissuing mints link #2 and immediately closes link #1
    let link2 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/care-link"),
        Some(&member),
        Some(json!({ "label": "Care View 2" })),
    )
    .await;
    assert_eq!(link2.status, StatusCode::OK);
    let tok2 = link2.json["token"].as_str().unwrap().to_string();

    let old_s1 = call(&app, Method::GET, &format!("/api/care/{tok1}"), None, None).await;
    assert_eq!(old_s1.status, StatusCode::BAD_REQUEST);
    assert_eq!(old_s1.json["code"], "CARE_PASS_CLOSED");

    // In-progress ride exposes driver & live location while still withholding street address
    sqlx::query("UPDATE rides SET status='in_progress', driver_id=$1, started_at=$2 WHERE id=$3")
        .bind(driver_id)
        .bind(grace_ride::util::now())
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO driver_locations (driver_id, lat, lng, updated_at) VALUES ($1, 3.101, 101.645, $2)
         ON CONFLICT (driver_id) DO UPDATE SET lat=3.101, lng=101.645, updated_at=$2",
    )
    .bind(driver_id)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();

    let s2 = call(&app, Method::GET, &format!("/api/care/{tok2}"), None, None).await;
    assert_eq!(s2.status, StatusCode::OK, "{}", s2.json);
    assert!(s2.json["driver"]["name"].is_string());
    assert_eq!(s2.json["location"]["sharing"], true);
    assert!(!s2.json.to_string().contains("88 Secret Home Street"));

    // Revoke closes link #2
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/rides/{ride_id}/care-link"),
            Some(&member),
            None,
        )
        .await
        .status,
        StatusCode::OK
    );
    let revoked_s2 = call(&app, Method::GET, &format!("/api/care/{tok2}"), None, None).await;
    assert_eq!(revoked_s2.status, StatusCode::BAD_REQUEST);
    assert_eq!(revoked_s2.json["code"], "CARE_PASS_CLOSED");

    // Issue link #3, backdate valid_until into the past -> expired
    let link3 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/care-link"),
        Some(&member),
        Some(json!({ "label": "Expiring" })),
    )
    .await;
    let tok3 = link3.json["token"].as_str().unwrap().to_string();
    sqlx::query(
        "UPDATE caregiver_passes SET valid_until=$1 WHERE ride_id=$2 AND closed_at IS NULL",
    )
    .bind(grace_ride::util::minutes_ago(5))
    .bind(ride_id)
    .execute(&st.db)
    .await
    .unwrap();
    let expired_s3 = call(&app, Method::GET, &format!("/api/care/{tok3}"), None, None).await;
    assert_eq!(expired_s3.status, StatusCode::BAD_REQUEST);
    assert_eq!(expired_s3.json["code"], "CARE_PASS_CLOSED");

    // Cannot cancel while in_progress (409); cancel once reverted to confirmed, then care-link creation is blocked
    let cancel_in_prog = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/cancel"),
        Some(&member),
        Some(json!({ "reason": "No longer needed" })),
    )
    .await;
    assert_eq!(cancel_in_prog.status, StatusCode::CONFLICT);

    sqlx::query("UPDATE rides SET status='confirmed' WHERE id=$1")
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{ride_id}/cancel"),
            Some(&member),
            Some(json!({ "reason": "No longer needed" })),
        )
        .await
        .status,
        StatusCode::OK
    );
    let blocked_after_cancel = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/care-link"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(blocked_after_cancel.status, StatusCode::BAD_REQUEST);
    assert_eq!(blocked_after_cancel.json["code"], "CARE_PASS_CLOSED");
}

#[tokio::test]
async fn recurring_schedules_are_driver_only_with_rbac_and_seat_bounds() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let other = login(&st, "amina@example.org").await;
    let area = first_area_id(&st).await;

    let (inactive_area,): (i64,) = sqlx::query_as(
        "INSERT INTO areas (name, active) VALUES ('Closed Test Zone', 0) RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Non-driver cannot create a driver standing schedule
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/recurring-schedules",
            Some(&member),
            Some(json!({
                "kind": "driver",
                "weekday": 0,
                "pickup_time": "08:00",
                "area_id": area,
                "seats": 4
            })),
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Seat bounds: driver 1..=8
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/recurring-schedules",
            Some(&driver),
            Some(json!({
                "kind": "driver",
                "weekday": 0,
                "pickup_time": "08:00",
                "area_id": area,
                "seats": 8
            })),
        )
        .await
        .status,
        StatusCode::OK
    );
    let drv_9 = call(
        &app,
        Method::POST,
        "/api/recurring-schedules",
        Some(&driver),
        Some(json!({
            "kind": "driver",
            "weekday": 0,
            "pickup_time": "08:00",
            "area_id": area,
            "seats": 9
        })),
    )
    .await;
    assert_eq!(drv_9.status, StatusCode::BAD_REQUEST);
    assert_eq!(drv_9.json["code"], "SEATS_RANGE");

    // Inactive area blocked on create
    let bad_area_create = call(
        &app,
        Method::POST,
        "/api/recurring-schedules",
        Some(&driver),
        Some(json!({
            "kind": "driver",
            "weekday": 0,
            "pickup_time": "08:30",
            "area_id": inactive_area,
            "seats": 2
        })),
    )
    .await;
    assert_eq!(bad_area_create.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_area_create.json["code"], "PICKUP_AREA_REQUIRED");

    // Rides exist only for events, so a standing weekly passenger ride is refused.
    let passenger = call(
        &app,
        Method::POST,
        "/api/recurring-schedules",
        Some(&member),
        Some(json!({
            "kind": "passenger",
            "label": "Sunday Worship",
            "weekday": 0,
            "pickup_time": "08:30",
            "area_id": area,
            "pickup_address": "1 Jalan Test",
            "destination": "Church",
            "seats": 2
        })),
    )
    .await;
    assert_eq!(
        passenger.status,
        StatusCode::BAD_REQUEST,
        "{}",
        passenger.json
    );
    assert_eq!(passenger.json["code"], "EVENT_RIDE_NEEDS_SIGNUP");

    // Another member cannot touch someone else's schedule.
    let (sched_id,): (i64,) = sqlx::query_as("SELECT id FROM recurring_ride_schedules LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/recurring-schedules/{sched_id}"),
            Some(&other),
            None,
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn round_trip_bounds_stroller_arrange_rule_and_proxy_vs_self_duplicate_guard() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let area = first_area_id(&st).await;
    let all_group = all_group_id(&st).await;

    let (inactive_area,): (i64,) = sqlx::query_as(
        "INSERT INTO areas (name, active) VALUES ('Inactive Return Area', 0) RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Return time earlier than or equal to outbound pickup is rejected
    let bad_ret_time = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": future_dt(2, "10:00"),
            "pickup_area_id": area,
            "pickup_address": "12 Jalan Satu",
            "destination": "Church",
            "return_pickup_at": future_dt(2, "09:30"),
            "seats": 1
        })),
    )
    .await;
    assert_eq!(bad_ret_time.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_ret_time.json["code"], "INVALID_PICKUP_TIME");

    // Inactive return area is rejected
    let bad_ret_area = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": future_dt(2, "09:00"),
            "pickup_area_id": area,
            "pickup_address": "12 Jalan Satu",
            "destination": "Church",
            "return_pickup_at": future_dt(2, "12:00"),
            "return_pickup_area_id": inactive_area,
            "seats": 1
        })),
    )
    .await;
    assert_eq!(bad_ret_area.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_ret_area.json["code"], "PICKUP_AREA_REQUIRED");

    // Event + round-trip + duplicate self guard vs proxy allowance + stroller child_seat rule
    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Family Service Day",
            "kind": "other",
            "starts_at": future_dt(2, "09:30"),
            "location": "Sanctuary",
            "capacity": 40,
            "group_ids": [all_group]
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev_id = ev.json["id"].as_i64().unwrap();

    let ev_detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&member),
        None,
    )
    .await;
    let join_group = ev_detail.json["join_as"][0]["id"].as_i64().unwrap();

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{ev_id}/signup"),
            Some(&member),
            Some(json!({ "as_group_id": join_group })),
        )
        .await
        .status,
        StatusCode::OK
    );

    // Book round-trip self ride for the event
    let rt = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(2, "08:30"),
            "pickup_area_id": area,
            "pickup_address": "12 Jalan Satu",
            "destination": "Sanctuary",
            "return_pickup_at": future_dt(2, "12:30"),
            "seats": 1
        })),
    )
    .await;
    assert_eq!(rt.status, StatusCode::OK, "{}", rt.json);
    assert!(rt.json["return_ride_id"].as_i64().is_some());

    // Second self booking for the same event must be rejected
    let dup_self = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(2, "08:45"),
            "pickup_area_id": area,
            "pickup_address": "12 Jalan Satu",
            "destination": "Sanctuary",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(dup_self.status, StatusCode::BAD_REQUEST);
    assert_eq!(dup_self.json["code"], "DUPLICATE_EVENT_RIDE");

    // Proxy ride with Stroller assistance alongside own ride succeeds
    let proxy_stroller = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(2, "08:45"),
            "pickup_area_id": area,
            "pickup_address": "12 Jalan Satu",
            "destination": "Sanctuary",
            "seats": 1,
            "is_proxy": true,
            "rider_name": "Baby Noah",
            "assistance": "Stroller"
        })),
    )
    .await;
    assert_eq!(
        proxy_stroller.status,
        StatusCode::OK,
        "{}",
        proxy_stroller.json
    );
    let stroller_ride_id = proxy_stroller.json["id"].as_i64().unwrap();

    // Driver with wheelchair_ok=1, child_seat=0 must NOT receive the Stroller ride
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/driver/profile",
            Some(&driver),
            Some(json!({
                "carpool_enabled": true,
                "spare_seats": 4,
                "area_ids": [area],
                "vehicle": {
                    "model": "Alphard",
                    "color": "White",
                    "plate": "WAA 101",
                    "seats": 6,
                    "vehicle_type": "standard",
                    "wheelchair_ok": true,
                    "low_step": true,
                    "child_seat": false
                }
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let arr1 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{ev_id}/plan/arrange"),
        Some(&transport),
        Some(json!({ "draft": [] })),
    )
    .await;
    assert_eq!(arr1.status, StatusCode::OK, "{}", arr1.json);
    assert!(
        arr1.json["unplaced"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_i64() == Some(stroller_ride_id)),
        "stroller ride must stay unplaced when child_seat=false: {}",
        arr1.json
    );

    // Enabling child_seat=true allows placement on the driver
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/driver/profile",
            Some(&driver),
            Some(json!({
                "carpool_enabled": true,
                "spare_seats": 4,
                "area_ids": [area],
                "vehicle": {
                    "model": "Alphard",
                    "color": "White",
                    "plate": "WAA 101",
                    "seats": 6,
                    "vehicle_type": "standard",
                    "wheelchair_ok": true,
                    "low_step": true,
                    "child_seat": true
                }
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let arr2 = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{ev_id}/plan/arrange"),
        Some(&transport),
        Some(json!({ "draft": [] })),
    )
    .await;
    assert_eq!(arr2.status, StatusCode::OK, "{}", arr2.json);
    assert!(
        arr2.json["placed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["ride_id"] == stroller_ride_id && p["driver_id"] == driver_id),
        "stroller ride should be placed once child_seat=true: {}",
        arr2.json
    );

    // Composite UI assistance text normalizes safely to canonical DB enum while preserving original detail in notes
    let chips_ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(2, "08:50"),
            "pickup_area_id": area,
            "pickup_address": "12 Jalan Satu",
            "destination": "Sanctuary",
            "seats": 1,
            "is_proxy": true,
            "rider_name": "Elder Tan",
            "assistance": "Wheelchair, Walking aid",
            "notes": "Ring doorbell"
        })),
    )
    .await;
    assert_eq!(chips_ride.status, StatusCode::OK, "{}", chips_ride.json);
    let chips_id = chips_ride.json["id"].as_i64().unwrap();
    let chips_detail = call(
        &app,
        Method::GET,
        &format!("/api/rides/{chips_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(chips_detail.status, StatusCode::OK);
    assert_eq!(chips_detail.json["assistance"], "wheelchair");
    assert_eq!(
        chips_detail.json["notes"],
        "Wheelchair, Walking aid · Ring doorbell"
    );
}

#[tokio::test]
async fn calendar_ics_rotation_revocation_rfc5545_and_disabled_user_block() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let area = first_area_id(&st).await;

    let created_ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": future_dt(1, "09:00"),
            "pickup_area_id": area,
            "pickup_address": "15 Jalan SS2",
            "destination": "ICS Destination Hall",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(created_ride.status, StatusCode::OK);
    let ride_id = created_ride.json["id"].as_i64().unwrap();
    sqlx::query("UPDATE rides SET status='confirmed' WHERE id=$1")
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();

    let r1 = call(
        &app,
        Method::POST,
        "/api/me/calendar-feed/rotate",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r1.status, StatusCode::OK, "{}", r1.json);
    let path1 = r1.json["ics_path"].as_str().unwrap().to_string();

    let (st1, hdr1, body1) = call_raw(&app, Method::GET, &path1, None).await;
    assert_eq!(st1, StatusCode::OK);
    assert!(hdr1
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/calendar"));
    assert!(body1.contains("BEGIN:VCALENDAR"));
    assert!(body1.contains("END:VCALENDAR"));
    assert!(body1.contains("ICS Destination Hall"));

    // Rotate again -> path1 invalidated (404), path2 valid (200)
    let r2 = call(
        &app,
        Method::POST,
        "/api/me/calendar-feed/rotate",
        Some(&member),
        None,
    )
    .await;
    let path2 = r2.json["ics_path"].as_str().unwrap().to_string();
    assert_ne!(path1, path2);

    let (old_st, _, _) = call_raw(&app, Method::GET, &path1, None).await;
    assert_eq!(old_st, StatusCode::NOT_FOUND);
    let (st2, _, _) = call_raw(&app, Method::GET, &path2, None).await;
    assert_eq!(st2, StatusCode::OK);

    // Disabled user cannot serve calendar feed even with valid token
    sqlx::query("UPDATE users SET status='disabled' WHERE id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let (dis_st, _, _) = call_raw(&app, Method::GET, &path2, None).await;
    assert_eq!(dis_st, StatusCode::NOT_FOUND);

    sqlx::query("UPDATE users SET status='active' WHERE id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    assert_eq!(
        call_raw(&app, Method::GET, &path2, None).await.0,
        StatusCode::OK
    );

    // Explicit DELETE revokes feed permanently
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            "/api/me/calendar-feed",
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call_raw(&app, Method::GET, &path2, None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn member_pass_scan_permissions_group_scope_and_idempotency() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let all_group = all_group_id(&st).await;

    let ev_a = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Scan Event A",
            "kind": "other",
            "starts_at": future_dt(1, "19:00"),
            "location": "Hall A",
            "capacity": 30,
            "group_ids": [all_group],
            "attendance_enabled": true
        })),
    )
    .await;
    assert_eq!(ev_a.status, StatusCode::OK);
    let ev_a_id = ev_a.json["id"].as_i64().unwrap();

    let ev_b = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Scan Event B",
            "kind": "other",
            "starts_at": future_dt(2, "19:00"),
            "location": "Hall B",
            "capacity": 30,
            "group_ids": [all_group],
            "attendance_enabled": true
        })),
    )
    .await;
    let ev_b_id = ev_b.json["id"].as_i64().unwrap();

    let detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_a_id}"),
        Some(&member),
        None,
    )
    .await;
    let join_group = detail.json["join_as"][0]["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{ev_a_id}/signup"),
            Some(&member),
            Some(json!({ "as_group_id": join_group })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let after_signup = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_a_id}"),
        Some(&member),
        None,
    )
    .await;
    let pass_code = after_signup.json["my_signup"]["pass_code"]
        .as_str()
        .expect("personal pass_code")
        .to_string();

    // Ordinary member cannot operate organiser scanner
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{ev_a_id}/checkin/scan-member"),
            Some(&member),
            Some(json!({ "pass_code": pass_code })),
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Unknown code returns 400
    let unknown = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_a_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": "NONEXISTENT99" })),
    )
    .await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST, "{}", unknown.json);

    // Cross-event pass_code is rejected on Event B
    let cross_ev = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_b_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": pass_code })),
    )
    .await;
    assert_eq!(
        cross_ev.status,
        StatusCode::BAD_REQUEST,
        "{}",
        cross_ev.json
    );

    // Prefixed GRPASS format works on Event A & is idempotent on repeat scan
    let scan1 = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_a_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": format!("GRPASS:{ev_a_id}:{}", pass_code.to_lowercase()) })),
    )
    .await;
    assert_eq!(scan1.status, StatusCode::OK, "{}", scan1.json);
    assert_eq!(scan1.json["already_attended"], false);
    assert!(scan1.json["user"]["name"].is_string());

    let scan2 = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_a_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": pass_code })),
    )
    .await;
    assert_eq!(scan2.status, StatusCode::OK, "{}", scan2.json);
    assert_eq!(scan2.json["already_attended"], true);
}

#[tokio::test]
async fn payout_batch_validation_separation_of_duties_and_concurrent_pay_race() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let transport_id = user_id(&st, "transport@example.org").await;
    let finance_id = user_id(&st, "finance@example.org").await;

    // Month without approved claims fails with BATCH_EMPTY
    let empty_batch = call(
        &app,
        Method::POST,
        "/api/reimbursements/batches",
        Some(&finance),
        Some(json!({ "month": "2020-01" })),
    )
    .await;
    assert_eq!(
        empty_batch.status,
        StatusCode::BAD_REQUEST,
        "{}",
        empty_batch.json
    );
    assert_eq!(empty_batch.json["code"], "BATCH_EMPTY");

    // Submit claim as member, approve as transport, create payout batch as finance
    let receipt = upload_png(&app, &member).await;
    let claim = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&member),
        Some(json!({
            "amount_cents": 3200,
            "description": "Grab ride to workshop",
            "receipt_file_id": receipt,
            "payee_name": "Grace Member",
            "payee_phone": "012-3456789"
        })),
    )
    .await;
    assert_eq!(claim.status, StatusCode::OK, "{}", claim.json);
    let claim_id = claim.json["id"].as_i64().unwrap();

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{claim_id}/review"),
            Some(&transport),
            Some(json!({ "action": "approve", "note": "" })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let month = grace_ride::util::now()[..7].to_string();
    let batch = call(
        &app,
        Method::POST,
        "/api/reimbursements/batches",
        Some(&finance),
        Some(json!({ "month": month, "title": "Concurrency Batch" })),
    )
    .await;
    assert_eq!(batch.status, StatusCode::OK, "{}", batch.json);
    let batch_id = batch.json["id"].as_i64().unwrap();

    // Validation: must verify payee and supply non-trivial txn ref
    let unverified = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": false, "paid_ref": "FT-999" })),
    )
    .await;
    assert_eq!(unverified.status, StatusCode::BAD_REQUEST);
    assert_eq!(unverified.json["code"], "PAYEE_VERIFICATION_REQUIRED");

    let short_ref = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "ab" })),
    )
    .await;
    assert_eq!(short_ref.status, StatusCode::BAD_REQUEST);
    assert_eq!(short_ref.json["code"], "TXN_REF_REQUIRED");

    // 2-person separation of duties: payer cannot equal approver or requester
    sqlx::query("UPDATE reimbursements SET reviewed_by=$1 WHERE id=$2")
        .bind(finance_id)
        .bind(claim_id)
        .execute(&st.db)
        .await
        .unwrap();
    let own_approval = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-SELF-APP" })),
    )
    .await;
    assert_eq!(own_approval.status, StatusCode::FORBIDDEN);
    assert_eq!(own_approval.json["code"], "CANNOT_PAY_OWN_APPROVAL");

    sqlx::query("UPDATE reimbursements SET user_id=$1, reviewed_by=$2 WHERE id=$3")
        .bind(finance_id)
        .bind(transport_id)
        .bind(claim_id)
        .execute(&st.db)
        .await
        .unwrap();
    let own_request = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-SELF-REQ" })),
    )
    .await;
    assert_eq!(own_request.status, StatusCode::FORBIDDEN);
    assert_eq!(own_request.json["code"], "CANNOT_PAY_OWN_REQUEST");

    sqlx::query("UPDATE reimbursements SET user_id=$1, reviewed_by=$2 WHERE id=$3")
        .bind(member_id)
        .bind(transport_id)
        .bind(claim_id)
        .execute(&st.db)
        .await
        .unwrap();

    // Concurrent double-pay: exactly one succeeds (200) and one gets BATCH_ALREADY_PAID (400)
    let pay_url = format!("/api/reimbursements/batches/{batch_id}/pay");
    let (p1, p2) = tokio::join!(
        call(
            &app,
            Method::POST,
            &pay_url,
            Some(&finance),
            Some(json!({ "payee_verified": true, "paid_ref": "FT-RACE-1" })),
        ),
        call(
            &app,
            Method::POST,
            &pay_url,
            Some(&finance),
            Some(json!({ "payee_verified": true, "paid_ref": "FT-RACE-2" })),
        )
    );
    let mut statuses = [p1.status, p2.status];
    statuses.sort_by_key(|s| s.as_u16());
    assert_eq!(statuses, [StatusCode::OK, StatusCode::BAD_REQUEST]);
    let loser = if p1.status == StatusCode::BAD_REQUEST {
        &p1
    } else {
        &p2
    };
    assert_eq!(loser.json["code"], "BATCH_ALREADY_PAID");
}
