//! Regressions for workflow fixes: ride state guards, duplicate event rides, disabled
//! drivers, settlement close and event cancel/finish.
mod common;
use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

async fn book(app: &axum::Router, cookie: &str) -> i64 {
    let r = call(
        app,
        Method::POST,
        "/api/rides",
        Some(cookie),
        Some(json!({"pickup_at": tomorrow("09:00"), "pickup_area_id": 2,
                    "pickup_address": "x", "destination": "教会", "seats": 1})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    r.json["id"].as_i64().unwrap()
}

async fn status_of(st: &grace_ride::state::AppState, id: i64) -> String {
    sqlx::query_scalar("SELECT status FROM rides WHERE id=$1")
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn unable_to_join_is_refused_after_pickup() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let id = book(&app, &passenger).await;
    sqlx::query("UPDATE rides SET status='in_progress', driver_id=4, picked_up_at=$1 WHERE id=$2")
        .bind(grace_ride::util::now())
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&passenger),
        Some(json!({"status": "unable_to_join"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.json);
    assert_eq!(status_of(&st, id).await, "in_progress");
}

#[tokio::test]
async fn confirm_by_replaced_driver_is_refused() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let old_driver = login(&st, "driver@example.org").await; // id 4
    let id = book(&app, &passenger).await;
    // Staff handed the ride to driver 5 after driver 4's page loaded.
    sqlx::query("UPDATE rides SET status='pending_driver_confirm', driver_id=5 WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/confirm-assignment"),
        Some(&old_driver),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(status_of(&st, id).await, "pending_driver_confirm");
}

#[tokio::test]
async fn disabling_a_driver_releases_unstarted_rides() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let id = book(&app, &passenger).await;
    sqlx::query("UPDATE rides SET status='confirmed', driver_id=4 WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    let roles: String = sqlx::query_scalar("SELECT roles FROM users WHERE id=4")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/users/4",
        Some(&admin),
        Some(
            json!({"roles": roles.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                    "status": "disabled"}),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let (status, driver, attention): (String, Option<i64>, i64) =
        sqlx::query_as("SELECT status, driver_id, needs_attention FROM rides WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(
        (status.as_str(), driver, attention),
        ("pending_driver", None, 1)
    );
}

#[tokio::test]
async fn reminder_job_respects_passenger_readiness_prompt_mins() {
    let (app, st) = setup().await;
    // Silence seeded rides so a run whose wall-clock time happens to sit within reminder
    // window of a demo ride does not generate extra reminder notifications.
    sqlx::query("UPDATE rides SET reminder_sent=1")
        .execute(&st.db)
        .await
        .unwrap();

    let passenger = login(&st, "member@example.org").await;
    let passenger_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    // 1. Passenger sets readiness_prompt_mins = 30.
    sqlx::query("UPDATE users SET readiness_prompt_mins=30 WHERE id=$1")
        .bind(passenger_id)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM notifications WHERE user_id IN ($1, 4)")
        .bind(passenger_id)
        .execute(&st.db)
        .await
        .unwrap();

    let ride_id = book(&app, &passenger).await;
    let at_50m = grace_ride::util::in_minutes(50)[..16].to_string();
    sqlx::query("UPDATE rides SET status='confirmed', driver_id=4, pickup_at=$1, reminder_sent=0 WHERE id=$2")
        .bind(&at_50m)
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();

    // At T-50m, a 30-minute preference must NOT fire yet.
    grace_ride::jobs::run_reminders(&st.db).await.unwrap();
    let sent: i64 = sqlx::query_scalar("SELECT reminder_sent FROM rides WHERE id=$1")
        .bind(ride_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(sent, 0, "reminder should wait until within 30m window");

    // Move pickup into the 30-minute window (T+20m) -> both passenger and driver get notified.
    let at_20m = grace_ride::util::in_minutes(20)[..16].to_string();
    sqlx::query("UPDATE rides SET pickup_at=$1 WHERE id=$2")
        .bind(&at_20m)
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();
    grace_ride::jobs::run_reminders(&st.db).await.unwrap();

    let sent_after: i64 = sqlx::query_scalar("SELECT reminder_sent FROM rides WHERE id=$1")
        .bind(ride_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(sent_after, 1);

    let p_notifs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id=$1 AND link=$2")
            .bind(passenger_id)
            .bind(format!("/rides/{ride_id}?action=readiness"))
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(p_notifs, 1);

    // 2. Passenger disables readiness prompt (0): driver still gets notified at T-45m, passenger gets 0 new reminders.
    sqlx::query("UPDATE rides SET reminder_sent=1")
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET readiness_prompt_mins=0 WHERE id=$1")
        .bind(passenger_id)
        .execute(&st.db)
        .await
        .unwrap();
    let ride2 = book(&app, &passenger).await;
    let at_45m = grace_ride::util::in_minutes(45)[..16].to_string();
    sqlx::query("UPDATE rides SET status='confirmed', driver_id=4, pickup_at=$1, reminder_sent=0 WHERE id=$2")
        .bind(&at_45m)
        .bind(ride2)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM notifications WHERE user_id IN ($1, 4)")
        .bind(passenger_id)
        .execute(&st.db)
        .await
        .unwrap();

    grace_ride::jobs::run_reminders(&st.db).await.unwrap();

    let p_notifs_opt_out: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id=$1")
            .bind(passenger_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    let d_notifs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id=4")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(
        p_notifs_opt_out, 0,
        "opted-out passenger should receive no readiness prompt"
    );
    assert_eq!(d_notifs, 1, "driver should still receive pre-trip reminder");
}

#[tokio::test]
async fn care_pass_blocks_unable_to_join_and_post_pickup_readiness() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let ride_id = book(&app, &passenger).await;
    sqlx::query("UPDATE rides SET status='confirmed', driver_id=4 WHERE id=$1")
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();

    let created = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/care-link"),
        Some(&passenger),
        Some(json!({"label": "Family link"})),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let token = created.json["token"].as_str().unwrap();

    // Unauthenticated care link cannot cancel ride via unable_to_join.
    let cancel_attempt = call(
        &app,
        Method::POST,
        &format!("/api/care/{token}/readiness"),
        None,
        Some(json!({"status": "unable_to_join"})),
    )
    .await;
    assert_eq!(
        cancel_attempt.status,
        StatusCode::BAD_REQUEST,
        "{}",
        cancel_attempt.json
    );
    assert_eq!(status_of(&st, ride_id).await, "confirmed");

    // Ready status works before pickup.
    let ready_ok = call(
        &app,
        Method::POST,
        &format!("/api/care/{token}/readiness"),
        None,
        Some(json!({"status": "ready"})),
    )
    .await;
    assert_eq!(ready_ok.status, StatusCode::OK, "{}", ready_ok.json);

    // Once picked_up_at is set, readiness updates are refused with 409 Conflict.
    sqlx::query("UPDATE rides SET status='in_progress', picked_up_at=$1 WHERE id=$2")
        .bind(grace_ride::util::now())
        .bind(ride_id)
        .execute(&st.db)
        .await
        .unwrap();
    let post_pickup = call(
        &app,
        Method::POST,
        &format!("/api/care/{token}/readiness"),
        None,
        Some(json!({"status": "delayed_5m"})),
    )
    .await;
    assert_eq!(
        post_pickup.status,
        StatusCode::CONFLICT,
        "{}",
        post_pickup.json
    );
}

#[tokio::test]
async fn past_events_first_paginated_page_includes_finished_events() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;

    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Early Finished Fellowship",
            "kind": "other",
            "starts_at": tomorrow("18:00"),
            "location": "Hall B",
            "group_ids": [1]
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev_id = ev.json["id"].as_i64().unwrap();

    sqlx::query("UPDATE events SET status='finished' WHERE id=$1")
        .bind(ev_id)
        .execute(&st.db)
        .await
        .unwrap();

    let past_page = call(
        &app,
        Method::GET,
        "/api/events?scope=past&limit=25",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(past_page.status, StatusCode::OK, "{}", past_page.json);
    let items = past_page.json["items"]
        .as_array()
        .expect("paginated items array");
    assert!(
        items.iter().any(|e| e["id"].as_i64() == Some(ev_id)),
        "finished event should appear on first paginated past page"
    );
}

#[tokio::test]
async fn rides_and_driving_exist_only_for_events_the_member_joined() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let body = json!({"pickup_at": tomorrow("09:00"), "pickup_area_id": 2,
                      "pickup_address": "x", "destination": "教会", "seats": 1});

    // A ride with no event is refused outright.
    let r = call_exact(
        &app,
        Method::POST,
        "/api/rides",
        Some(&passenger),
        Some(body.clone()),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "EVENT_RIDE_NEEDS_SIGNUP");

    // Two events, each with one ride (the shim gives each its own event, everyone signed up).
    let a = book(&app, &passenger).await;
    let b = book(&app, &passenger).await;
    let event_of = |id: i64| {
        let db = st.db.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT event_id FROM rides WHERE id=$1")
                .bind(id)
                .fetch_one(&db)
                .await
                .unwrap()
        }
    };
    let (ea, eb) = (event_of(a).await, event_of(b).await);
    assert_ne!(ea, eb);

    // The requests list can be narrowed to one event.
    let list = call(
        &app,
        Method::GET,
        &format!("/api/driver/requests?event_id={ea}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.json);
    let ids: Vec<i64> = list
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect();
    assert!(ids.contains(&a) && !ids.contains(&b), "{ids:?}");

    // A driver who has not signed up for the event cannot claim its ride...
    sqlx::query("DELETE FROM event_signups WHERE event_id=$1 AND user_id=(SELECT id FROM users WHERE email='driver@example.org')")
        .bind(ea)
        .execute(&st.db)
        .await
        .unwrap();
    let claim = format!("/api/rides/{a}/claim");
    let r = call_exact(
        &app,
        Method::POST,
        &claim,
        Some(&driver),
        None,
        Some(ORIGIN),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "EVENT_DRIVE_NEEDS_SIGNUP");

    // ...and can once they have.
    sqlx::query("INSERT INTO event_signups (event_id, user_id, status, created_at) SELECT $1, id, 'going', '2026-01-01 00:00' FROM users WHERE email='driver@example.org'")
        .bind(ea)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call_exact(
        &app,
        Method::POST,
        &claim,
        Some(&driver),
        None,
        Some(ORIGIN),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
}

#[tokio::test]
async fn a_driver_leaving_the_event_releases_their_rides_and_standing_rides_are_refused() {
    let (app, st) = setup().await;
    let passenger = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let id = book(&app, &passenger).await;
    let event: i64 = sqlx::query_scalar("SELECT event_id FROM rides WHERE id=$1")
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/claim"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{event}/withdraw"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let (status, driver_id): (String, Option<i64>) =
        sqlx::query_as("SELECT status, driver_id FROM rides WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!((status.as_str(), driver_id), ("pending_driver", None));

    // A standing weekly passenger ride has no event, so it can no longer be set up.
    let r = call(
        &app,
        Method::POST,
        "/api/recurring-schedules",
        Some(&passenger),
        Some(json!({
            "kind": "passenger", "label": "Sunday", "weekday": 0, "pickup_time": "08:30",
            "area_id": 2, "pickup_address": "x", "destination": "教会", "seats": 1
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
}
