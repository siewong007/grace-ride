//! Visitors recorded on an event's transport plan: the traffic lead notes people
//! without an account. Record only: no rides, invites or notifications.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::{json, Value};

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn event_by(st: &grace_ride::state::AppState, creator: i64, title: &str) -> i64 {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, starts_at, created_by) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(title)
    .bind(tomorrow("19:00"))
    .bind(creator)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_groups (event_id, group_id) SELECT $1, id FROM groups WHERE is_all = 1",
    )
    .bind(id)
    .execute(&st.db)
    .await
    .unwrap();
    id
}

async fn counts(st: &grace_ride::state::AppState) -> (i64, i64, i64) {
    let rides: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rides")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let passes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM visitor_passes")
        .fetch_one(&st.db)
        .await
        .unwrap();
    (rides, notes, passes)
}

fn visitors(plan: &Value) -> &Vec<Value> {
    plan["visitors"].as_array().expect("plan lists visitors")
}

#[tokio::test]
async fn traffic_lead_records_visitors_without_side_effects() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let cell_id = user_id(&st, "cellleader@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    sqlx::query(
        "INSERT INTO vehicles (user_id, model, color, plate, seats, is_active)
         VALUES ($1, 'Myvi', 'White', 'VIS 1', 5, 1) ON CONFLICT DO NOTHING",
    )
    .bind(driver_id)
    .execute(&st.db)
    .await
    .unwrap();
    let church = event_by(&st, admin_id, "Sunday service").await;
    let cell = event_by(&st, cell_id, "Cell supper").await;
    let area: i64 = sqlx::query_scalar("SELECT id FROM areas WHERE active = 1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let before = counts(&st).await;

    // Head of Traffic (the traffic group's lead) records a visitor in a driver's car.
    let head = login(&st, "traffichead@example.org").await;
    let r = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{church}/plan/visitors"),
        Some(&head),
        Some(json!({
            "name": "  Mr Tan (visitor)  ", "phone": "012-345 6789", "area_id": area,
            "pickup_address": "Jalan 5", "seats": 2, "driver_id": driver_id, "notes": "First visit",
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let v1 = r.json["id"].as_i64().unwrap();

    let plan = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{church}/plan"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(plan.status, StatusCode::OK, "{}", plan.json);
    let list = visitors(&plan.json);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "Mr Tan (visitor)");
    assert_eq!(list[0]["seats"], 2);
    assert_eq!(list[0]["leg"], "outbound");
    assert_eq!(list[0]["driver_id"], driver_id);
    assert_eq!(list[0]["area"]["id"], area);
    assert_eq!(list[0]["can_edit"], true);

    // The Head of Traffic records on a cell leader's event too.
    let r = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{cell}/plan/visitors"),
        Some(&head),
        Some(json!({"name": "Auntie Lim", "leg": "return"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let v2 = r.json["id"].as_i64().unwrap();

    // The cell leader sees visitors on their own event's plan but cannot record or change them.
    let leader = login(&st, "cellleader@example.org").await;
    let r = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{cell}/plan/visitors"),
        Some(&leader),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["visitors"][0]["can_edit"], false);
    let r = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{cell}/plan/visitors"),
        Some(&leader),
        Some(json!({"name": "Nope"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/events/{cell}/plan/visitors/{v2}"),
        Some(&leader),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Record only: no ride, visitor pass or notification came of it.
    assert_eq!(counts(&st).await, before);

    // An admin corrects the Head of Traffic's visitor; the Head of Traffic removes it.
    let admin = login(&st, "admin@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/events/{cell}/plan/visitors/{v2}"),
        Some(&admin),
        Some(json!({"name": "Auntie Lim Siew", "leg": "return", "seats": 1})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/events/{cell}/plan/visitors/{v2}"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    // A visitor id only works under its own event.
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/events/{cell}/plan/visitors/{v1}"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    let audited: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs WHERE action LIKE 'plan.visitor_%'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(audited, 4);
}

#[tokio::test]
async fn only_traffic_leads_record_visitors() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let ev = event_by(&st, admin_id, "Camp").await;
    sqlx::query("UPDATE users SET roles = 'traffic_member' WHERE email = 'amina@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let path = format!("/api/admin/events/{ev}/plan/visitors");

    // Traffic leads: Head of Traffic, admin, President, Vice-President.
    let mut ids = vec![];
    for email in [
        "traffichead@example.org",
        "admin@example.org",
        "president@example.org",
        "vicepresident@example.org",
    ] {
        let who = login(&st, email).await;
        let r = call(
            &app,
            Method::POST,
            &path,
            Some(&who),
            Some(json!({"name": "Visitor"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{email}: {}", r.json);
        ids.push(r.json["id"].as_i64().unwrap());
    }
    // Any traffic lead may change a visitor another one added.
    let head = login(&st, "traffichead@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        &format!("{path}/{}", ids[1]),
        Some(&head),
        Some(json!({"name": "Visitor B"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // Plan staff below the lead may see visitors but not record or change them.
    for email in [
        "amina@example.org",
        "transport@example.org",
        "secretary@example.org",
    ] {
        let who = login(&st, email).await;
        let list = call(&app, Method::GET, &path, Some(&who), None).await;
        assert_eq!(list.status, StatusCode::OK, "{email} sees the plan");
        assert_eq!(list.json["visitors"][0]["can_edit"], false, "{email}");
        let r = call(
            &app,
            Method::POST,
            &path,
            Some(&who),
            Some(json!({"name": "Visitor"})),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not add visitors"
        );
        let one = format!("{path}/{}", ids[0]);
        let r = call(
            &app,
            Method::PUT,
            &one,
            Some(&who),
            Some(json!({"name": "Hijack"})),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not edit visitors"
        );
        let r = call(&app, Method::DELETE, &one, Some(&who), None).await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not remove visitors"
        );
    }
    // Without plan rights at all: no access.
    for email in [
        "member@example.org",
        "driver@example.org",
        "finance@example.org",
        "auditor@example.org",
    ] {
        let who = login(&st, email).await;
        let r = call(
            &app,
            Method::POST,
            &path,
            Some(&who),
            Some(json!({"name": "Visitor"})),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not record visitors"
        );
        let r = call(&app, Method::GET, &path, Some(&who), None).await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not list visitors"
        );
    }
}

#[tokio::test]
async fn visitor_input_is_checked_and_follows_the_event() {
    let (app, st) = setup().await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let ev = event_by(&st, admin_id, "Retreat").await;
    let head = login(&st, "traffichead@example.org").await;
    let path = format!("/api/admin/events/{ev}/plan/visitors");
    for body in [
        json!({"name": "   "}),
        json!({"name": "X", "seats": 9}),
        json!({"name": "X", "seats": 0}),
        json!({"name": "X", "leg": "sideways"}),
        json!({"name": "X", "phone": "123"}),
        json!({"name": "X", "area_id": 999_999}),
        json!({"name": "X", "driver_id": user_id(&st, "member@example.org").await}),
        json!({"name": "x".repeat(81)}),
    ] {
        let r = call(&app, Method::POST, &path, Some(&head), Some(body.clone())).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}: {}", r.json);
    }
    let r = call(
        &app,
        Method::POST,
        &path,
        Some(&head),
        Some(json!({"name": "Kept"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    // Nothing more is recorded once the event is cancelled.
    sqlx::query("UPDATE events SET status='cancelled' WHERE id=$1")
        .bind(ev)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &path,
        Some(&head),
        Some(json!({"name": "Late"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    // Removing the event removes its visitor records.
    sqlx::query("DELETE FROM events WHERE id=$1")
        .bind(ev)
        .execute(&st.db)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plan_visitors")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(left, 0);
}
