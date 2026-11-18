//! Driver requests board, schedule, route, and driver/vehicle profile management.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audience::MATCH_GROUP_SQL;
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, now, require, ErrorKey};

use super::{ride_json, RideRow, RIDE_SELECT};

#[derive(Deserialize)]
pub(super) struct RequestsQuery {
    area_id: Option<i64>,
    date: Option<String>,
    mine_areas: Option<bool>,
}

pub(super) async fn driver_requests(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<RequestsQuery>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Drive, Act::View) {
        return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanView));
    }
    let mut conds = vec![
        "r.status='pending_driver'".to_string(),
        "r.passenger_id != $1".to_string(),
        "r.pickup_at >= $2".to_string(),
        // Event rides go only to drivers from the rider's group or a group above it.
        format!("({MATCH_GROUP_SQL} IS NULL OR {MATCH_GROUP_SQL} = ANY($3))"),
    ];
    let mut n = 4;
    if q.area_id.is_some() {
        conds.push(format!("r.pickup_area_id = ${n}"));
        n += 1;
    } else if q.mine_areas.unwrap_or(false) {
        conds.push(format!(
            "r.pickup_area_id IN (SELECT area_id FROM driver_areas WHERE user_id = ${n})"
        ));
        n += 1;
    }
    if q.date.is_some() {
        // Same rows as `substr(pickup_at,1,10) = date`, written as a range so the pickup
        // index can serve it: every time on that day sorts between "dateT" and "dateU".
        conds.push(format!(
            "(r.pickup_at >= (${n} || 'T') AND r.pickup_at < (${n} || 'U'))"
        ));
    }
    let sql = format!(
        "{RIDE_SELECT} WHERE {} ORDER BY r.pickup_at LIMIT 100",
        conds.join(" AND ")
    );
    let mut query = sqlx::query_as::<_, RideRow>(&sql)
        .bind(u.id)
        .bind(now()[..16].to_string())
        .bind(&u.drive_scope);
    if let Some(a) = q.area_id {
        query = query.bind(a);
    } else if q.mine_areas.unwrap_or(false) {
        query = query.bind(u.id);
    }
    if let Some(d) = &q.date {
        query = query.bind(d);
    }
    let rows = query.fetch_all(&st.db).await?;
    Ok(Json(json!(rows
        .iter()
        .map(|r| ride_json(r, &u))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub(super) struct RangeQuery {
    from: Option<String>,
    to: Option<String>,
}

pub(super) async fn driver_schedule(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<RangeQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Drive, Act::View)?;
    let from = q.from.unwrap_or_else(|| now()[..10].to_string());
    let to = q.to.unwrap_or_else(|| from.clone());
    let rows = sqlx::query_as::<_, RideRow>(&format!(
        "{RIDE_SELECT} WHERE r.driver_id=$1 AND substr(r.pickup_at,1,10) BETWEEN $2 AND $3
         AND r.status NOT IN ('cancelled') ORDER BY r.pickup_at"
    ))
    .bind(u.id)
    .bind(&from)
    .bind(&to)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .iter()
        .map(|r| ride_json(r, &u))
        .collect::<Vec<_>>())))
}

/// The driver's current (or next) trip: pickups in the best order with road distance,
/// ETAs and the route line. `{"route": null}` when there's nothing to drive.
pub(super) async fn driver_route(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let route = crate::routing::driver_route(&st, u.id, None).await?;
    Ok(Json(json!({ "route": route })))
}

pub(super) async fn get_driver_profile(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::Drive, Act::View)?;
    let (carpool, spare, wait): (i64, i64, Option<i64>) = sqlx::query_as(
        "SELECT carpool_enabled, spare_seats, gate_wait_mins FROM users WHERE id=$1",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    let areas: Vec<(i64,)> = sqlx::query_as("SELECT area_id FROM driver_areas WHERE user_id=$1")
        .bind(u.id)
        .fetch_all(&st.db)
        .await?;
    let v: Option<(String, String, String, i64, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT model, color, plate, seats, vehicle_type, wheelchair_ok, low_step, child_seat FROM vehicles WHERE user_id=$1",
    )
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    Ok(Json(json!({
        "carpool_enabled": carpool == 1,
        "spare_seats": spare,
        "gate_wait_mins": wait.unwrap_or(5),
        "area_ids": areas.into_iter().map(|a| a.0).collect::<Vec<_>>(),
        "vehicle": v.map(|(m, c, p, s, t, w, l, cs)| json!({
            "model": m,
            "color": c,
            "plate": p,
            "seats": s,
            "type": t,
            "vehicle_type": t,
            "wheelchair_ok": w == 1,
            "low_step": l == 1,
            "child_seat": cs == 1,
        })),
    })))
}

#[derive(Deserialize)]
struct VehicleIn {
    model: String,
    color: String,
    plate: String,
    seats: i64,
    #[serde(default = "default_vehicle_type")]
    vehicle_type: String,
    #[serde(default)]
    wheelchair_ok: Option<bool>,
    #[serde(default)]
    low_step: Option<bool>,
    #[serde(default)]
    child_seat: Option<bool>,
}

fn default_vehicle_type() -> String {
    "standard".to_string()
}

#[derive(Deserialize)]
pub(super) struct DriverProfileIn {
    carpool_enabled: bool,
    spare_seats: i64,
    area_ids: Vec<i64>,
    vehicle: Option<VehicleIn>,
    gate_wait_mins: Option<i64>,
}

pub(super) async fn put_driver_profile(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<DriverProfileIn>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Drive, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
    }
    if !(1..=7).contains(&req.spare_seats) {
        return Err(bad(u.lang, ErrorKey::SpareSeatsRange));
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE users SET carpool_enabled=$1, spare_seats=$2 WHERE id=$3")
        .bind(req.carpool_enabled as i64)
        .bind(req.spare_seats)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    if let Some(wait) = req.gate_wait_mins {
        let wait = wait.clamp(1, 30);
        sqlx::query("UPDATE users SET gate_wait_mins=$1 WHERE id=$2")
            .bind(wait)
            .bind(u.id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM driver_areas WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    for a in &req.area_ids {
        sqlx::query(
            "INSERT INTO driver_areas (user_id, area_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(u.id)
        .bind(a)
        .execute(&mut *tx)
        .await?;
    }
    let mut detail = format!(
        "carpool {}; spare seats {}; areas {:?}",
        if req.carpool_enabled { "on" } else { "off" },
        req.spare_seats,
        req.area_ids
    );
    if let Some(v) = req.vehicle {
        if v.model.trim().is_empty() || v.plate.trim().is_empty() {
            return Err(bad(u.lang, ErrorKey::VehicleModelAndPlateRequired));
        }
        if !(2..=9).contains(&v.seats) {
            return Err(bad(u.lang, ErrorKey::VehicleSeatsRange));
        }
        sqlx::query(
            "INSERT INTO vehicles (user_id, model, color, plate, seats, vehicle_type, wheelchair_ok, low_step, child_seat)
             VALUES ($1,$2,$3,$4,$5,$6,COALESCE($7, 0),COALESCE($8, 0),COALESCE($9, 0))
             ON CONFLICT(user_id) DO UPDATE SET
                 model=excluded.model,
                 color=excluded.color,
                 plate=excluded.plate,
                 seats=excluded.seats,
                 vehicle_type=excluded.vehicle_type,
                 wheelchair_ok=COALESCE($7, vehicles.wheelchair_ok),
                 low_step=COALESCE($8, vehicles.low_step),
                 child_seat=COALESCE($9, vehicles.child_seat)",
        )
        .bind(u.id)
        .bind(v.model.trim())
        .bind(v.color.trim())
        .bind(v.plate.trim().to_uppercase())
        .bind(v.seats)
        .bind(&v.vehicle_type)
        .bind(v.wheelchair_ok.map(|b| if b { 1i64 } else { 0i64 }))
        .bind(v.low_step.map(|b| if b { 1i64 } else { 0i64 }))
        .bind(v.child_seat.map(|b| if b { 1i64 } else { 0i64 }))
        .execute(&mut *tx)
        .await?;
        detail.push_str(&format!(
            "; vehicle {} {} ({}, {} seats)",
            v.plate.trim().to_uppercase(),
            v.model.trim(),
            v.vehicle_type,
            v.seats
        ));
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "driver.profile_update",
        "user",
        Some(u.id),
        detail,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}
