//! Live driver and passenger location sharing for active rides.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::perms::Act;
use crate::state::AppState;
use crate::util::{audit, now, ErrorKey};

use super::{load_ride, traffic_can};

pub(super) async fn clear_passenger_location(st: &AppState, ride: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM passenger_locations WHERE ride_id=$1")
        .bind(ride)
        .execute(&st.db)
        .await?;
    Ok(())
}

pub(super) async fn clear_location_if_idle(st: &AppState, driver: i64) -> AppResult<()> {
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rides WHERE driver_id=$1 AND status='in_progress'")
            .bind(driver)
            .fetch_one(&st.db)
            .await?;
    if n == 0 {
        sqlx::query("DELETE FROM driver_locations WHERE driver_id=$1")
            .bind(driver)
            .execute(&st.db)
            .await?;
    }
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct LocIn {
    lat: f64,
    lng: f64,
    heading: Option<f64>,
}

/// Location is accepted only while the driver has a ride in progress (行程开始才共享位置).
pub(super) async fn post_location(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<LocIn>,
) -> AppResult<Json<Value>> {
    if !(-90.0..=90.0).contains(&req.lat) || !(-180.0..=180.0).contains(&req.lng) {
        return Err(bad(u.lang, ErrorKey::InvalidLocation));
    }
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rides WHERE driver_id=$1 AND status='in_progress'")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    if n == 0 {
        sqlx::query("DELETE FROM driver_locations WHERE driver_id=$1")
            .bind(u.id)
            .execute(&st.db)
            .await?;
        return Ok(Json(json!({"ok": false, "sharing": false})));
    }
    sqlx::query(
        "INSERT INTO driver_locations (driver_id, lat, lng, heading, updated_at) VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT(driver_id) DO UPDATE SET lat=excluded.lat, lng=excluded.lng, heading=excluded.heading, updated_at=excluded.updated_at",
    )
    .bind(u.id)
    .bind(req.lat)
    .bind(req.lng)
    .bind(req.heading)
    .bind(now())
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true, "sharing": true})))
}

pub(super) async fn stop_location(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let stopped = sqlx::query("DELETE FROM driver_locations WHERE driver_id=$1")
        .bind(u.id)
        .execute(&st.db)
        .await?
        .rows_affected();
    // Position updates themselves are never logged; switching sharing off is.
    if stopped > 0 {
        audit(
            &st.db,
            Some(u.id),
            "location.stop",
            "user",
            Some(u.id),
            "driver",
        )
        .await;
    }
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn ride_location(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.passenger_id != u.id && r.driver_id != Some(u.id) && !traffic_can(&u, &r, Act::View) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewLocation));
    }
    if r.status != "in_progress" {
        return Ok(Json(json!({"sharing": false})));
    }
    let loc: Option<(f64, f64, Option<f64>, String)> = sqlx::query_as(
        "SELECT lat, lng, heading, updated_at FROM driver_locations WHERE driver_id=$1",
    )
    .bind(r.driver_id)
    .fetch_optional(&st.db)
    .await?;
    let mut out = match loc {
        Some((lat, lng, h, at)) => {
            json!({"sharing": true, "lat": lat, "lng": lng, "heading": h, "updated_at": at})
        }
        None => json!({"sharing": false}),
    };
    // The passenger's own spot, only until they're in the car.
    if r.picked_up_at.is_none() {
        let p: Option<(f64, f64, String)> =
            sqlx::query_as("SELECT lat, lng, updated_at FROM passenger_locations WHERE ride_id=$1")
                .bind(id)
                .fetch_optional(&st.db)
                .await?;
        if let Some((lat, lng, at)) = p {
            out["passenger"] = json!({"lat": lat, "lng": lng, "updated_at": at});
        }
    }
    // Distance and ETA to this passenger (or to the destination once aboard).
    if let (true, Some(driver)) = (out["sharing"] == true, r.driver_id) {
        let trip = r.trip_id.unwrap_or(r.id);
        let full_view = r.driver_id == Some(u.id) || traffic_can(&u, &r, Act::View);
        if let Ok(Some(route)) = crate::routing::driver_route(&st, driver, Some(trip)).await {
            if let Some(mut eta) =
                crate::routing::ride_eta(&route, id, r.picked_up_at.is_some(), full_view)
            {
                if r.picked_up_at.is_some() && !full_view {
                    eta["destination"] = json!(r.destination);
                }
                out["eta"] = eta;
            }
        }
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
pub(super) struct PassengerLocIn {
    lat: f64,
    lng: f64,
}

/// Passenger opts in to showing the driver where they are waiting. Accepted only
/// while the driver is on the way (in progress, not yet picked up); otherwise the
/// stored spot is dropped and `sharing: false` tells the client to stop.
pub(super) async fn post_passenger_location(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<PassengerLocIn>,
) -> AppResult<Json<Value>> {
    if !(-90.0..=90.0).contains(&req.lat) || !(-180.0..=180.0).contains(&req.lng) {
        return Err(bad(u.lang, ErrorKey::InvalidLocation));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if r.passenger_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NotRidePassenger));
    }
    if r.status != "in_progress" || r.picked_up_at.is_some() {
        clear_passenger_location(&st, id).await?;
        return Ok(Json(json!({"ok": false, "sharing": false})));
    }
    // `xmax = 0` is true only for a newly inserted row: the first position of this ride.
    let (first,): (bool,) = sqlx::query_as(
        "INSERT INTO passenger_locations (ride_id, lat, lng, updated_at) VALUES ($1,$2,$3,$4)
         ON CONFLICT(ride_id) DO UPDATE SET lat=excluded.lat, lng=excluded.lng, updated_at=excluded.updated_at
         RETURNING (xmax = 0)",
    )
    .bind(id)
    .bind(req.lat)
    .bind(req.lng)
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    // The passenger opting in is logged once; the positions that follow are not.
    if first {
        audit(
            &st.db,
            Some(u.id),
            "location.share",
            "ride",
            Some(id),
            "passenger",
        )
        .await;
    }
    Ok(Json(json!({"ok": true, "sharing": true})))
}

pub(super) async fn stop_passenger_location(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.passenger_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NotRidePassenger));
    }
    let stopped = sqlx::query("DELETE FROM passenger_locations WHERE ride_id=$1")
        .bind(id)
        .execute(&st.db)
        .await?
        .rows_affected();
    if stopped > 0 {
        audit(
            &st.db,
            Some(u.id),
            "location.stop",
            "ride",
            Some(id),
            "passenger",
        )
        .await;
    }
    Ok(Json(json!({"ok": true})))
}
