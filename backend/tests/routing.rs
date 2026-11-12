//! Multi-passenger pickup routing: best pickup order for the driver, and each
//! passenger's distance/ETA without exposing other passengers' positions.
//! Runs offline (routing network calls are off in tests): addresses resolve from
//! `geocode_cache` rows inserted here and estimates are straight-line.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::json;

async fn user_id(st: &AppState, email: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
        .0
}

async fn insert_ride(
    st: &AppState,
    passenger: i64,
    driver: i64,
    at: &str,
    address: &str,
    status: &str,
) -> i64 {
    sqlx::query_as::<_, (i64,)>(
        "INSERT INTO rides (passenger_id, driver_id, pickup_at, pickup_area_id, pickup_address, destination, seats, status, started_at)
         VALUES ($1,$2,$3,2,$4,'Test Church',1,$5, CASE WHEN $5='in_progress' THEN $3 END) RETURNING id",
    )
    .bind(passenger)
    .bind(driver)
    .bind(at)
    .bind(address)
    .bind(status)
    .fetch_one(&st.db)
    .await
    .unwrap()
    .0
}

async fn cache_address(st: &AppState, query: &str, lat: f64, lng: f64) {
    sqlx::query("INSERT INTO geocode_cache (query, lat, lng) VALUES ($1,$2,$3)")
        .bind(query)
        .bind(lat)
        .bind(lng)
        .execute(&st.db)
        .await
        .unwrap();
}

#[tokio::test]
async fn driver_gets_best_pickup_order_and_riders_get_their_eta() {
    let (app, st) = setup().await;
    let driver_id = user_id(&st, "driver2@example.org").await;
    let a_id = user_id(&st, "member@example.org").await;
    let b_id = user_id(&st, "amina@example.org").await;
    let c_id = user_id(&st, "sophie@example.org").await;
    let driver = login(&st, "driver2@example.org").await;
    let a = login(&st, "member@example.org").await;
    let b = login(&st, "amina@example.org").await;
    let c = login(&st, "sophie@example.org").await;
    let outsider = login(&st, "yawen@example.org").await;

    // Driver at lng 101.600 heading east to the church at 101.700. A waits near the
    // church (shared GPS), B near the driver (geocoded address), C's address can't be found.
    // Booked A, B, C — the best order is B, then A.
    let at = tomorrow("09:00");
    let ra = insert_ride(&st, a_id, driver_id, &at, "A home", "in_progress").await;
    let rb = insert_ride(
        &st,
        b_id,
        driver_id,
        &at,
        "B Street,  Petaling Jaya",
        "in_progress",
    )
    .await;
    let rc = insert_ride(&st, c_id, driver_id, &at, "Nowhere 999", "confirmed").await;
    sqlx::query("UPDATE rides SET trip_id=$1 WHERE id = ANY($2)")
        .bind(ra)
        .bind(vec![ra, rb, rc])
        .execute(&st.db)
        .await
        .unwrap();
    cache_address(&st, "b street, petaling jaya", 3.1, 101.62).await;
    cache_address(&st, "test church", 3.1, 101.70).await;

    let res = call(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&driver),
        Some(json!({ "lat": 3.1, "lng": 101.6 })),
    )
    .await;
    assert_eq!(res.json["sharing"], true);
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ra}/passenger-location"),
        Some(&a),
        Some(json!({ "lat": 3.1, "lng": 101.68 })),
    )
    .await;
    assert_eq!(res.json["sharing"], true);

    // ---- Driver: ordered stops with distances and ETAs ----
    let res = call(&app, Method::GET, "/api/driver/route", Some(&driver), None).await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let route = &res.json["route"];
    let stops = route["stops"].as_array().unwrap();
    let ids: Vec<i64> = stops
        .iter()
        .map(|s| s["ride_id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![rb, ra, rc], "{route}");
    assert_eq!(stops[0]["location"], "address");
    assert_eq!(stops[1]["location"], "live");
    assert!(stops[2]["location"].is_null() && stops[2]["eta"].is_null());
    assert!(stops[0]["eta"].is_string() && stops[1]["eta"].is_string());
    let d0 = stops[0]["distance_m"].as_f64().unwrap();
    let d1 = stops[1]["distance_m"].as_f64().unwrap();
    assert!(d0 > 0.0 && d1 > d0, "cumulative distance grows: {d0} {d1}");
    assert_eq!(route["live"], true);
    assert_eq!(route["approximate"], true, "routing is offline in tests");
    assert_eq!(route["destination"]["name"], "Test Church");
    assert!(route["destination"]["eta"].is_string());
    assert_eq!(route["geometry"].as_array().unwrap().len(), 4); // driver, B, A, church
    assert!(route["destination"]["distance_m"].as_f64().unwrap() > d1);

    // ---- Passengers: their own ETA only ----
    let res = call(
        &app,
        Method::GET,
        &format!("/api/rides/{rb}/location"),
        Some(&b),
        None,
    )
    .await;
    let eta = &res.json["eta"];
    assert_eq!(eta["phase"], "pickup", "{}", res.json);
    assert_eq!(eta["stops_before"], 0);
    assert_eq!(eta["located"], true);
    assert_eq!(eta["distance_m"].as_f64().unwrap(), d0);
    assert!(eta["eta"].is_string());
    assert_eq!(eta["path"].as_array().unwrap().len(), 2, "straight to B");

    let res = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ra}/location"),
        Some(&a),
        None,
    )
    .await;
    let eta = &res.json["eta"];
    assert_eq!(eta["stops_before"], 1, "{}", res.json);
    assert_eq!(eta["distance_m"].as_f64().unwrap(), d1);
    assert!(eta.get("path").is_none(), "the line would pass B's pickup");
    assert!(
        !res.json.to_string().contains("101.62"),
        "B's position leaked to A: {}",
        res.json
    );

    // The driver sees the line to any passenger.
    let res = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ra}/location"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(res.json["eta"]["path"].as_array().unwrap().len(), 3);

    // C's ride hasn't started: no location, no ETA. Outsiders get no route.
    let res = call(
        &app,
        Method::GET,
        &format!("/api/rides/{rc}/location"),
        Some(&c),
        None,
    )
    .await;
    assert_eq!(res.json["sharing"], false);
    assert!(res.json.get("eta").is_none());
    let res = call(
        &app,
        Method::GET,
        "/api/driver/route",
        Some(&outsider),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(res.json["route"].is_null());
    let res = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ra}/location"),
        Some(&outsider),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    // ---- After B boards: B gets the ETA to the church, behind the remaining pickups ----
    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rb}/pickup"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let res = call(
        &app,
        Method::GET,
        &format!("/api/rides/{rb}/location"),
        Some(&b),
        None,
    )
    .await;
    let eta = &res.json["eta"];
    assert_eq!(eta["phase"], "dropoff", "{}", res.json);
    assert_eq!(eta["destination"], "Test Church");
    assert_eq!(eta["stops_before"], 2);
    assert!(eta["eta"].is_string());
    assert!(eta.get("path").is_none());
    let res = call(&app, Method::GET, "/api/driver/route", Some(&driver), None).await;
    let route = &res.json["route"];
    assert_eq!(route["onboard"], 1);
    let ids: Vec<i64> = route["stops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["ride_id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![ra, rc]);
}

#[tokio::test]
async fn upcoming_trip_is_planned_without_etas() {
    let (app, st) = setup().await;
    let driver_id = user_id(&st, "driver2@example.org").await;
    let p = user_id(&st, "member@example.org").await;
    let driver = login(&st, "driver2@example.org").await;
    let soon = (chrono::Local::now() + chrono::Duration::minutes(60))
        .format("%Y-%m-%dT%H:%M")
        .to_string();
    let id = insert_ride(
        &st,
        p,
        driver_id,
        &soon,
        "B Street, Petaling Jaya",
        "confirmed",
    )
    .await;
    cache_address(&st, "b street, petaling jaya", 3.1, 101.62).await;

    let res = call(&app, Method::GET, "/api/driver/route", Some(&driver), None).await;
    let route = &res.json["route"];
    assert_eq!(route["trip_id"], id, "{}", res.json);
    assert_eq!(route["live"], false);
    assert!(route["origin"].is_null());
    assert_eq!(route["stops"][0]["ride_id"], id);
    assert_eq!(route["stops"][0]["started"], false);
    assert!(
        route["stops"][0]["eta"].is_null(),
        "no ETA before the driver sets off"
    );
}

#[tokio::test]
async fn start_trip_starts_every_passenger_in_the_carpool() {
    let (app, st) = setup().await;
    let driver_id = user_id(&st, "driver2@example.org").await;
    let a_id = user_id(&st, "member@example.org").await;
    let b_id = user_id(&st, "amina@example.org").await;
    let c_id = user_id(&st, "sophie@example.org").await;
    let driver = login(&st, "driver2@example.org").await;
    let b = login(&st, "amina@example.org").await;

    // A and B share a trip; C is the same driver's separate, later trip.
    let at = tomorrow("09:00");
    let ra = insert_ride(&st, a_id, driver_id, &at, "A home", "confirmed").await;
    let rb = insert_ride(&st, b_id, driver_id, &at, "B home", "confirmed").await;
    let rc = insert_ride(
        &st,
        c_id,
        driver_id,
        &tomorrow("18:00"),
        "C home",
        "confirmed",
    )
    .await;
    for (ride, trip) in [(ra, ra), (rb, ra), (rc, rc)] {
        sqlx::query("UPDATE rides SET trip_id=$1 WHERE id=$2")
            .bind(trip)
            .bind(ride)
            .execute(&st.db)
            .await
            .unwrap();
    }

    let res = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ra}/start"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    let mut started: Vec<i64> = res.json["started"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    started.sort();
    assert_eq!(started, vec![ra, rb]);

    let rows: Vec<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT id, status, started_at FROM rides WHERE id = ANY($1) ORDER BY id")
            .bind(vec![ra, rb, rc])
            .fetch_all(&st.db)
            .await
            .unwrap();
    assert_eq!(rows[0].1, "in_progress");
    assert_eq!(rows[1].1, "in_progress");
    assert!(rows[1].2.is_some(), "B's trip got a start time");
    assert_eq!(rows[2].1, "confirmed", "a different trip isn't started");

    // Both passengers are told the driver is on the way; one chat line names them both.
    let (notified,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT user_id) FROM notifications WHERE user_id = ANY($1) AND (title LIKE '%Driver departed%' OR title LIKE '%司机已出发%')",
    )
    .bind(vec![a_id, b_id])
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(notified, 2);
    let (msgs,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ride_messages WHERE trip_id=$1 AND user_id IS NULL AND (body LIKE '%started%' OR body LIKE '%已开始%')",
    )
    .bind(ra)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(msgs, 1);

    // B never tapped anything, yet sees the driver's distance and ETA once the car moves.
    cache_address(&st, "b home", 3.1, 101.62).await;
    call(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&driver),
        Some(json!({ "lat": 3.1, "lng": 101.6 })),
    )
    .await;
    let res = call(
        &app,
        Method::GET,
        &format!("/api/rides/{rb}/location"),
        Some(&b),
        None,
    )
    .await;
    assert_eq!(res.json["sharing"], true, "{}", res.json);
    assert_eq!(res.json["eta"]["phase"], "pickup");
    assert!(res.json["eta"]["eta"].is_string());

    // Starting again is refused: the ride is already under way.
    let again = call(
        &app,
        Method::POST,
        &format!("/api/rides/{rb}/start"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(again.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn place_search_is_members_only_and_quiet_when_routing_is_off() {
    let (app, st) = setup().await;
    let anon = call(&app, Method::GET, "/api/places?q=jalan%20ss2", None, None).await;
    assert_eq!(anon.status, StatusCode::UNAUTHORIZED);

    // Tests run with routing network calls off: the field falls back to plain text.
    let member = login(&st, "member@example.org").await;
    let r = call(
        &app,
        Method::GET,
        "/api/places?q=jalan%20ss2",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json, json!([]));
}
