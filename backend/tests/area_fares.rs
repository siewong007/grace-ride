//! Fixed fares between service areas: the traffic lead and admins price each pair of
//! areas, the same in both directions; drivers see the fare for a ride's route.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

async fn area_id(st: &grace_ride::state::AppState, name: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM areas WHERE name=$1")
        .bind(name)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|_| panic!("area {name} missing"))
}

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn default_service_areas_exist_without_invented_fares() {
    let (_app, st) = setup().await;
    for name in ["UT", "Sec 17", "UM", "KK", "Pacific/RM"] {
        let active: i64 = sqlx::query_scalar("SELECT active FROM areas WHERE name=$1")
            .bind(name)
            .fetch_one(&st.db)
            .await
            .unwrap_or_else(|_| panic!("area {name} missing"));
        assert_eq!(active, 1, "{name} should be open");
    }
    let fares: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM area_fares f JOIN areas a ON a.id IN (f.area_a_id, f.area_b_id)
         WHERE a.name IN ('UT','Sec 17','UM','KK','Pacific/RM')",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(fares, 0, "rates are left for the traffic lead to fill in");
}

#[tokio::test]
async fn only_traffic_lead_and_admin_can_set_fares() {
    let (app, st) = setup().await;
    let (ut, kk) = (area_id(&st, "UT").await, area_id(&st, "KK").await);
    let body = |cents: i64| json!({"area_a_id": kk, "area_b_id": ut, "amount_cents": cents});

    // Head of Traffic sets and changes a fare; the pair is stored once.
    let head = login(&st, "traffichead@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&head),
        Some(body(1200)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let admin = login(&st, "admin@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&admin),
        Some(body(1300)),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let list = call(
        &app,
        Method::GET,
        "/api/admin/settings/fares",
        Some(&head),
        None,
    )
    .await;
    assert_eq!(list.status, StatusCode::OK);
    let fares = list.json["fares"].as_array().unwrap();
    assert_eq!(fares.len(), 1);
    assert_eq!(fares[0]["amount_cents"], 1300);
    let fare_id = fares[0]["id"].as_i64().unwrap();

    // Leadership can read fares but not change them.
    for email in ["president@example.org", "vicepresident@example.org"] {
        let who = login(&st, email).await;
        let r = call(
            &app,
            Method::GET,
            "/api/admin/settings/fares",
            Some(&who),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{email} reads fares");
        let r = call(
            &app,
            Method::PUT,
            "/api/admin/settings/fares",
            Some(&who),
            Some(body(1)),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not set fares"
        );
        let r = call(
            &app,
            Method::DELETE,
            &format!("/api/admin/settings/fares/{fare_id}"),
            Some(&who),
            None,
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not clear fares"
        );
    }
    // Everyone else can't even see them.
    for email in [
        "secretary@example.org",
        "transport@example.org",
        "cellleader@example.org",
        "finance@example.org",
        "driver@example.org",
        "member@example.org",
    ] {
        let who = login(&st, email).await;
        let r = call(
            &app,
            Method::PUT,
            "/api/admin/settings/fares",
            Some(&who),
            Some(body(1)),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not set fares"
        );
        let r = call(
            &app,
            Method::DELETE,
            &format!("/api/admin/settings/fares/{fare_id}"),
            Some(&who),
            None,
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not clear fares"
        );
    }
    let stored: i64 = sqlx::query_scalar("SELECT amount_cents FROM area_fares WHERE id=$1")
        .bind(fare_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(stored, 1300);

    // A same-area fare is allowed (pickup and event in the same area).
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&head),
        Some(json!({"area_a_id": ut, "area_b_id": ut, "amount_cents": 500})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // The traffic lead clears a fare.
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/settings/fares/{fare_id}"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    // The traffic lead adds, renames and closes areas.
    let r = call(
        &app,
        Method::POST,
        "/api/admin/areas",
        Some(&head),
        Some(json!({"name": "Damansara"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let new_id = area_id(&st, "Damansara").await;
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/areas/{new_id}"),
        Some(&head),
        Some(json!({"name": "Damansara Utama", "active": false})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let active: i64 = sqlx::query_scalar("SELECT active FROM areas WHERE name='Damansara Utama'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(active, 0);
}

#[tokio::test]
async fn ride_shows_its_route_fare_to_the_driver_not_the_rider() {
    let (app, st) = setup().await;
    let (ut, kk) = (area_id(&st, "UT").await, area_id(&st, "KK").await);
    let passenger = user_id(&st, "member@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;

    // A mileage-claimable event held in KK, open to everyone.
    let event: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, starts_at, created_by, mileage_claimable, area_id)
         VALUES ('Fare test', $1, $2, 1, $3) RETURNING id",
    )
    .bind(tomorrow("19:00"))
    .bind(admin_id)
    .bind(kk)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_groups (event_id, group_id) SELECT $1, id FROM groups WHERE is_all = 1",
    )
    .bind(event)
    .execute(&st.db)
    .await
    .unwrap();
    let ride: i64 = sqlx::query_scalar(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, notes, status)
         VALUES ($1, $2, $3, $4, $5, 'Jalan 1', 'Hall', 1, '', '', 'confirmed') RETURNING id",
    )
    .bind(passenger)
    .bind(driver_id)
    .bind(event)
    .bind(tomorrow("18:30"))
    .bind(ut)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let driver = login(&st, "driver@example.org").await;
    let rider = login(&st, "member@example.org").await;
    let path = format!("/api/rides/{ride}");

    // No fare yet: the driver sees the route with an unset amount.
    let r = call(&app, Method::GET, &path, Some(&driver), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["route_fare"]["from"], "UT");
    assert_eq!(r.json["route_fare"]["to"], "KK");
    assert!(r.json["route_fare"]["amount_cents"].is_null());

    // Priced KK -> UT; the UT -> KK ride picks it up.
    let head = login(&st, "traffichead@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&head),
        Some(json!({"area_a_id": kk, "area_b_id": ut, "amount_cents": 1500})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(&app, Method::GET, &path, Some(&driver), None).await;
    assert_eq!(r.json["route_fare"]["amount_cents"], 1500);

    // Riders pay no fare, so they aren't shown it.
    let r = call(&app, Method::GET, &path, Some(&rider), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.json["route_fare"].is_null());

    // Completing the ride pays the driver that fare.
    sqlx::query("UPDATE rides SET status='in_progress', started_at=$2 WHERE id=$1")
        .bind(ride)
        .bind(tomorrow("18:31"))
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride}/complete"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let paid: Option<i64> =
        sqlx::query_scalar("SELECT reimbursement_amount_cents FROM rides WHERE id=$1")
            .bind(ride)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(paid, Some(1500));

    // A non-claimable event shows no fare.
    sqlx::query("UPDATE events SET mileage_claimable=0 WHERE id=$1")
        .bind(event)
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(&app, Method::GET, &path, Some(&driver), None).await;
    assert!(r.json["route_fare"].is_null());
}
