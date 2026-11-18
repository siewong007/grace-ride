//! Regressions for the 2026-11-17 database audit:
//! - email lookups ignore case (C2);
//! - the "needs attention" window is 24 hours by the app's clock, not the session's (C1);
//! - cancelling an event cancels its open rides in one go, with one audit row each (C3);
//! - a reminder is claimed once, however many ticks see it (P3).

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::util::{in_hours, in_minutes};
use serde_json::json;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn any_area(st: &grace_ride::state::AppState) -> i64 {
    sqlx::query_scalar("SELECT MIN(id) FROM areas")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

/// Insert a ride directly, minute precision like the app writes.
async fn ride(
    st: &grace_ride::state::AppState,
    passenger: i64,
    driver: Option<i64>,
    event: Option<i64>,
    pickup_at: &str,
    status: &str,
) -> i64 {
    let area = any_area(st).await;
    sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, status)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(passenger)
    .bind(driver)
    .bind(event)
    .bind(&pickup_at[..16])
    .bind(area)
    .bind(status)
    .fetch_one(&st.db)
    .await
    .unwrap()
}

#[tokio::test]
async fn passkey_sign_in_ignores_the_case_of_the_email() {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let start = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/start",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(start.status, StatusCode::OK, "{}", start.json);
    let options: CreationChallengeResponse =
        serde_json::from_value(start.json["options"].clone()).unwrap();
    let mut soft = SoftPasskey::new(true);
    let origin = Url::parse(ORIGIN).unwrap();
    let credential = soft.do_registration(origin.clone(), options).unwrap();
    let finish = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&cookie),
        Some(json!({
            "challenge_id": start.json["challenge_id"],
            "credential": credential,
            "label": "Soft"
        })),
    )
    .await;
    assert_eq!(finish.status, StatusCode::OK, "{}", finish.json);

    // Typed with capitals: still this member's passkey, not a decoy challenge.
    let lstart = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({"email": "  Member@Example.ORG "})),
    )
    .await;
    assert_eq!(lstart.status, StatusCode::OK, "{}", lstart.json);
    let lopts: RequestChallengeResponse =
        serde_json::from_value(lstart.json["options"].clone()).unwrap();
    let assertion = soft
        .do_authentication(origin, lopts)
        .expect("the challenge lists the member's own passkey");
    let lfinish = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(json!({"challenge_id": lstart.json["challenge_id"], "credential": assertion})),
    )
    .await;
    assert_eq!(lfinish.status, StatusCode::OK, "{}", lfinish.json);
}

#[tokio::test]
async fn a_recovery_request_matches_the_member_whatever_the_case() {
    let (app, st) = setup().await;
    let r = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({"email": "Member@Example.org", "reason": "lost phone"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let stored: Vec<String> = sqlx::query_scalar("SELECT email FROM recovery_requests")
        .fetch_all(&st.db)
        .await
        .unwrap();
    assert_eq!(stored, ["member@example.org"]);
}

#[tokio::test]
async fn needs_attention_means_unmatched_within_24_hours() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = user_id(&st, "member@example.org").await;
    let soon = ride(&st, member, None, None, &in_hours(20), "pending_driver").await;
    let later = ride(&st, member, None, None, &in_hours(30), "pending_driver").await;

    let r = call(
        &app,
        Method::GET,
        "/api/admin/rides?attention=true",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let ids: Vec<i64> = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["id"].as_i64())
        .collect();
    assert!(ids.contains(&soon), "20 hours away needs attention");
    assert!(!ids.contains(&later), "30 hours away does not yet");
}

#[tokio::test]
async fn cancelling_an_event_cancels_its_open_rides_together() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let member = user_id(&st, "member@example.org").await;
    let event: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, starts_at, created_by) VALUES ('审计测试', $1, $2) RETURNING id",
    )
    .bind(&in_hours(48)[..16])
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let open = ride(
        &st,
        member,
        None,
        Some(event),
        &in_hours(47),
        "pending_driver",
    )
    .await;
    let confirmed = ride(
        &st,
        member,
        Some(admin_id),
        Some(event),
        &in_hours(47),
        "confirmed",
    )
    .await;
    let done = ride(
        &st,
        member,
        Some(admin_id),
        Some(event),
        &in_hours(47),
        "completed",
    )
    .await;

    let r = call(
        &app,
        Method::POST,
        &format!("/api/events/{event}/cancel"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let statuses: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, status FROM rides WHERE event_id = $1 ORDER BY id")
            .bind(event)
            .fetch_all(&st.db)
            .await
            .unwrap();
    assert_eq!(
        statuses,
        [
            (open, "cancelled".to_string()),
            (confirmed, "cancelled".to_string()),
            (done, "completed".to_string())
        ]
    );
    let audited: Vec<i64> = sqlx::query_scalar(
        "SELECT entity_id FROM audit_logs WHERE action = 'ride.cancel' AND entity_id = ANY($1) ORDER BY entity_id",
    )
    .bind([open, confirmed, done])
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(audited, [open, confirmed]);
    let event_status: String = sqlx::query_scalar("SELECT status FROM events WHERE id = $1")
        .bind(event)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(event_status, "cancelled");
}

#[tokio::test]
async fn a_ride_reminder_is_sent_once_however_many_ticks_see_it() {
    let (_app, st) = setup().await;
    let member = user_id(&st, "member@example.org").await;
    let driver = user_id(&st, "admin@example.org").await;
    let r = ride(
        &st,
        member,
        Some(driver),
        None,
        &in_minutes(30),
        "confirmed",
    )
    .await;
    let reminders = || async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND link LIKE $2",
        )
        .bind(member)
        .bind(format!("/rides/{r}%"))
        .fetch_one(&st.db)
        .await
        .unwrap()
    };
    // Two ticks at once, as if a tick overran into the next one.
    let (a, b) = tokio::join!(
        grace_ride::jobs::run_reminders(&st.db),
        grace_ride::jobs::run_reminders(&st.db)
    );
    a.unwrap();
    b.unwrap();
    grace_ride::jobs::run_reminders(&st.db).await.unwrap();
    assert_eq!(reminders().await, 1);
}
