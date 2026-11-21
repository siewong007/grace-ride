//! Regression tests for the bugs found in the 2026-11-21 review:
//! proxy double-booking, premature driver no-show, fabricated pickup timestamps,
//! care links open too early, self check-in outside the event window, ambiguous
//! check-in codes, and claimants locked out of staff-uploaded receipts.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::util::{in_hours, now};
use serde_json::json;

async fn uid(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

/// An event the member can see, starting at `starts_at`, with a check-in code.
async fn event_for(
    st: &grace_ride::state::AppState,
    member: i64,
    starts_at: &str,
    code: &str,
) -> i64 {
    let admin = uid(st, "admin@example.org").await;
    let (eid,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, starts_at, created_by, status, attendance_enabled, checkin_code)
         VALUES ('Bugfix test event', $1, $2, 'published', 1, $3) RETURNING id",
    )
    .bind(starts_at)
    .bind(admin)
    .bind(code)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let group: Option<i64> = sqlx::query_scalar(
        "SELECT group_id FROM group_members WHERE user_id=$1 ORDER BY group_id LIMIT 1",
    )
    .bind(member)
    .fetch_optional(&st.db)
    .await
    .unwrap()
    .flatten();
    let group = match group {
        Some(g) => g,
        None => sqlx::query_scalar("SELECT id FROM groups WHERE is_all=1 LIMIT 1")
            .fetch_one(&st.db)
            .await
            .unwrap(),
    };
    sqlx::query("INSERT INTO event_groups (event_id, group_id) VALUES ($1,$2)")
        .bind(eid)
        .bind(group)
        .execute(&st.db)
        .await
        .unwrap();
    eid
}

async fn insert_ride(
    st: &grace_ride::state::AppState,
    passenger: i64,
    driver: Option<i64>,
    status: &str,
    pickup_at: &str,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, status, started_at)
         VALUES ($1,$2,$3,1,'test pickup','test destination',1,'',$4,$5) RETURNING id",
    )
    .bind(passenger)
    .bind(driver)
    .bind(pickup_at)
    .bind(status)
    .bind(if status == "in_progress" { Some(pickup_at.to_string()) } else { None })
    .fetch_one(&st.db)
    .await
    .unwrap()
}

#[tokio::test]
async fn proxy_ride_double_booking_is_refused() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let mid = uid(&st, "member@example.org").await;
    let eid = event_for(&st, mid, &in_hours(48), "PROXYDUP").await;
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status) VALUES ($1,$2,'going')")
        .bind(eid)
        .bind(mid)
        .execute(&st.db)
        .await
        .unwrap();

    let body = json!({
        "pickup_at": in_hours(24), "pickup_area_id": 1,
        "pickup_address": "12 Test Street", "destination": "PJ State CMC",
        "seats": 1, "event_id": eid, "is_proxy": true, "rider_name": "Mom",
    });
    let path = "/api/rides";
    let (r1, r2) = tokio::join!(
        call(&app, Method::POST, path, Some(&member), Some(body.clone())),
        call(&app, Method::POST, path, Some(&member), Some(body.clone())),
    );
    let oks = [&r1, &r2]
        .iter()
        .filter(|r| r.status == StatusCode::OK)
        .count();
    assert_eq!(
        oks, 1,
        "double-tap proxy booking booked two cars: {} {}",
        r1.json, r2.json
    );
    let refused = if r1.status == StatusCode::OK {
        &r2
    } else {
        &r1
    };
    assert_eq!(refused.json["code"], "DUPLICATE_EVENT_RIDE");

    // A different proxy rider is a separate booking and must still be allowed.
    let other = call(
        &app,
        Method::POST,
        path,
        Some(&member),
        Some(json!({
            "pickup_at": in_hours(24), "pickup_area_id": 1,
            "pickup_address": "12 Test Street", "destination": "PJ State CMC",
            "seats": 1, "event_id": eid, "is_proxy": true, "rider_name": "Dad",
        })),
    )
    .await;
    assert_eq!(other.status, StatusCode::OK, "{}", other.json);

    // The member's own ride is a different rider too.
    let own = call(
        &app,
        Method::POST,
        path,
        Some(&member),
        Some(json!({
            "pickup_at": in_hours(24), "pickup_area_id": 1,
            "pickup_address": "12 Test Street", "destination": "PJ State CMC",
            "seats": 1, "event_id": eid,
        })),
    )
    .await;
    assert_eq!(own.status, StatusCode::OK, "{}", own.json);

    // …but a second own ride is still refused.
    let own2 = call(
        &app,
        Method::POST,
        path,
        Some(&member),
        Some(json!({
            "pickup_at": in_hours(24), "pickup_area_id": 1,
            "pickup_address": "12 Test Street", "destination": "PJ State CMC",
            "seats": 1, "event_id": eid,
        })),
    )
    .await;
    assert_eq!(own2.json["code"], "DUPLICATE_EVENT_RIDE");
}

#[tokio::test]
async fn driver_no_show_requires_started_trip_and_passed_pickup() {
    let (app, st) = setup().await;
    let driver = login(&st, "driver@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let mid = uid(&st, "member@example.org").await;
    let did = uid(&st, "driver@example.org").await;

    // Confirmed ride, pickup in the future: the driver cannot call a no-show yet.
    let rid = insert_ride(&st, mid, Some(did), "confirmed", &in_hours(2)).await;
    let early = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/no-show"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(early.status, StatusCode::CONFLICT, "{}", early.json);
    assert_eq!(early.json["code"], "NO_SHOW_TOO_EARLY");

    // Trip started but pickup time still ahead: still too early.
    sqlx::query("UPDATE rides SET status='in_progress', started_at=$1 WHERE id=$2")
        .bind(now())
        .bind(rid)
        .execute(&st.db)
        .await
        .unwrap();
    let early2 = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/no-show"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(early2.json["code"], "NO_SHOW_TOO_EARLY");

    // Pickup time passed and the passenger never showed: now it is allowed.
    sqlx::query("UPDATE rides SET pickup_at=$1 WHERE id=$2")
        .bind(in_hours(-1))
        .bind(rid)
        .execute(&st.db)
        .await
        .unwrap();
    let ok = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/no-show"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);

    // Staff keep the pre-emptive override on a confirmed, future ride.
    let rid2 = insert_ride(&st, mid, Some(did), "confirmed", &in_hours(2)).await;
    let staff = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid2}/no-show"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(staff.status, StatusCode::OK, "{}", staff.json);
}

#[tokio::test]
async fn complete_without_pickup_does_not_fabricate_timestamp() {
    let (app, st) = setup().await;
    let driver = login(&st, "driver@example.org").await;
    let mid = uid(&st, "member@example.org").await;
    let did = uid(&st, "driver@example.org").await;

    let rid = insert_ride(&st, mid, Some(did), "in_progress", &in_hours(-1)).await;
    let done = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/complete"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(done.status, StatusCode::OK, "{}", done.json);
    let (status, picked, completed): (String, Option<String>, Option<String>) =
        sqlx::query_as("SELECT status, picked_up_at, completed_at FROM rides WHERE id=$1")
            .bind(rid)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(status, "completed");
    assert!(picked.is_none(), "completion fabricated a pickup timestamp");
    assert!(completed.is_some());
}

#[tokio::test]
async fn care_link_opens_24h_before_pickup() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let mid = uid(&st, "member@example.org").await;

    // A ride two days away: the link exists but must not expose the ride yet.
    let rid = insert_ride(&st, mid, None, "pending_driver", &in_hours(48)).await;
    let created = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rid}/care-link"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let token = created.json["token"].as_str().unwrap();

    let too_early = call(&app, Method::GET, &format!("/api/care/{token}"), None, None).await;
    assert_eq!(
        too_early.status,
        StatusCode::BAD_REQUEST,
        "{}",
        too_early.json
    );
    assert_eq!(too_early.json["code"], "CARE_PASS_NOT_OPEN_YET");

    // Inside the 24-hour window the same link works and reports when it opens.
    sqlx::query("UPDATE rides SET pickup_at=$1 WHERE id=$2")
        .bind(in_hours(1))
        .bind(rid)
        .execute(&st.db)
        .await
        .unwrap();
    let open = call(&app, Method::GET, &format!("/api/care/{token}"), None, None).await;
    assert_eq!(open.status, StatusCode::OK, "{}", open.json);
    assert_eq!(open.json["ride_id"], rid);
    assert!(open.json["opens_at"].as_str().is_some());
}

#[tokio::test]
async fn self_checkin_respects_the_event_window_and_ambiguous_codes() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let mid = uid(&st, "member@example.org").await;

    // Two days before the event: self check-in is not open yet.
    let future = event_for(&st, mid, &in_hours(48), "WINDOWCODE").await;
    let early = call(
        &app,
        Method::POST,
        &format!("/api/events/{future}/checkin"),
        Some(&member),
        Some(json!({"code": "WINDOWCODE"})),
    )
    .await;
    assert_eq!(early.status, StatusCode::BAD_REQUEST, "{}", early.json);
    assert_eq!(early.json["code"], "CHECKIN_OUTSIDE_WINDOW");

    // Long after the event: the window has closed.
    let past = event_for(&st, mid, &in_hours(-24), "PASTCODE").await;
    let late = call(
        &app,
        Method::POST,
        &format!("/api/events/{past}/checkin"),
        Some(&member),
        Some(json!({"code": "PASTCODE"})),
    )
    .await;
    assert_eq!(late.json["code"], "CHECKIN_OUTSIDE_WINDOW");

    // At the event: check-in works.
    sqlx::query("UPDATE events SET starts_at=$1 WHERE id=$2")
        .bind(now())
        .bind(future)
        .execute(&st.db)
        .await
        .unwrap();
    let ok = call(
        &app,
        Method::POST,
        &format!("/api/events/{future}/checkin"),
        Some(&member),
        Some(json!({"code": "WINDOWCODE"})),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);

    // A second event sharing the code: the code-only endpoint must not silently
    // pick the earliest event.
    let twin = event_for(&st, mid, &now(), "WINDOWCODE").await;
    let ambiguous = call(
        &app,
        Method::POST,
        "/api/events/checkin",
        Some(&member),
        Some(json!({"code": "WINDOWCODE"})),
    )
    .await;
    assert_eq!(
        ambiguous.status,
        StatusCode::BAD_REQUEST,
        "{}",
        ambiguous.json
    );
    assert_eq!(ambiguous.json["code"], "CHECKIN_CODE_AMBIGUOUS");
    let _ = twin;
}

#[tokio::test]
async fn claimant_can_view_receipt_uploaded_by_staff() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let stranger = login(&st, "amina@example.org").await;
    let mid = uid(&st, "member@example.org").await;

    // Staff uploads the receipt on the member's behalf.
    let fid = upload_png(&app, &admin).await;
    sqlx::query(
        "INSERT INTO reimbursements (user_id, amount_cents, receipt_file_id, status, kind)
         VALUES ($1, 1000, $2, 'pending', 'grab')",
    )
    .bind(mid)
    .bind(fid)
    .execute(&st.db)
    .await
    .unwrap();

    let mine = call(
        &app,
        Method::GET,
        &format!("/api/files/{fid}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(
        mine.status,
        StatusCode::OK,
        "claimant locked out of their own receipt"
    );
    let theirs = call(
        &app,
        Method::GET,
        &format!("/api/files/{fid}"),
        Some(&stranger),
        None,
    )
    .await;
    assert_eq!(theirs.status, StatusCode::FORBIDDEN);
}
