//! Driver requests board, schedule, route, and driver/vehicle profile management.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audience::MATCH_GROUP_SQL;
use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, now, require, ErrorKey};

use super::{ride_json, RideRow, RIDE_SELECT};

#[derive(Deserialize)]
pub(super) struct RequestsQuery {
    area_id: Option<i64>,
    date: Option<String>,
    mine_areas: Option<bool>,
    event_id: Option<i64>,
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
    if q.event_id.is_some() {
        conds.push(format!("r.event_id = ${n}"));
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
    let mut query = sqlx::query_as::<_, RideRow>(sqlx::AssertSqlSafe(&*sql))
        .bind(u.id)
        .bind(now()[..16].to_string())
        .bind(&u.drive_scope);
    if let Some(a) = q.area_id {
        query = query.bind(a);
    } else if q.mine_areas.unwrap_or(false) {
        query = query.bind(u.id);
    }
    if let Some(e) = q.event_id {
        query = query.bind(e);
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
    let rows = sqlx::query_as::<_, RideRow>(crate::dyn_sql!(
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
    if !u.can(Page::Drive, Act::View) {
        return Ok(Json(json!({ "route": null })));
    }
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
    let vehicles: Vec<(i64, String, String, String, i64, String, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT id, model, color, plate, seats, vehicle_type, wheelchair_ok, low_step, child_seat, is_active FROM vehicles WHERE user_id=$1 ORDER BY is_active DESC, id ASC",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;

    let v_active = vehicles
        .iter()
        .find(|row| row.9 == 1)
        .or_else(|| vehicles.first());
    let vehicles_json: Vec<Value> = vehicles
        .iter()
        .map(|(id, m, c, p, s, t, w, l, cs, act)| {
            json!({
                "id": id,
                "model": m,
                "color": c,
                "plate": p,
                "seats": s,
                "type": t,
                "vehicle_type": t,
                "wheelchair_ok": *w == 1,
                "low_step": *l == 1,
                "child_seat": *cs == 1,
                "is_active": *act == 1,
            })
        })
        .collect();

    let vehicle_json = v_active.map(|(id, m, c, p, s, t, w, l, cs, act)| {
        json!({
            "id": id,
            "model": m,
            "color": c,
            "plate": p,
            "seats": s,
            "type": t,
            "vehicle_type": t,
            "wheelchair_ok": *w == 1,
            "low_step": *l == 1,
            "child_seat": *cs == 1,
            "is_active": *act == 1,
        })
    });
    let active_id = v_active.map(|row| row.0);

    Ok(Json(json!({
        "carpool_enabled": carpool == 1,
        "spare_seats": spare,
        "gate_wait_mins": wait.unwrap_or(5),
        "area_ids": areas.into_iter().map(|a| a.0).collect::<Vec<_>>(),
        "vehicle": vehicle_json,
        "vehicles": vehicles_json,
        "active_vehicle_id": active_id,
    })))
}

#[derive(Deserialize)]
pub(super) struct VehicleIn {
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
    #[serde(default)]
    is_active: Option<bool>,
}

fn default_vehicle_type() -> String {
    "standard".to_string()
}

/// Keeps `users.spare_seats` within `[1, max(1, seats - 1)]` after switching or editing
/// the active vehicle so carpool assignment capacity never exceeds physical seats.
async fn clamp_spare_seats(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: i64,
    vehicle_seats: i64,
) -> AppResult<()> {
    if vehicle_seats < 2 {
        return Ok(());
    }
    sqlx::query(
        "UPDATE users SET spare_seats = GREATEST(1, LEAST(spare_seats, $1 - 1)) WHERE id=$2",
    )
    .bind(vehicle_seats)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct DriverProfileIn {
    carpool_enabled: bool,
    spare_seats: i64,
    area_ids: Vec<i64>,
    #[serde(default)]
    vehicle: Option<VehicleIn>,
    #[serde(default)]
    active_vehicle_id: Option<i64>,
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
    if let Some(ref v) = req.vehicle {
        if v.model.trim().is_empty() || v.plate.trim().is_empty() || v.color.trim().is_empty() {
            return Err(bad(u.lang, ErrorKey::VehicleModelAndPlateRequired));
        }
        if !matches!(v.vehicle_type.trim(), "standard" | "other") {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        if !(2..=9).contains(&v.seats) {
            return Err(bad(u.lang, ErrorKey::VehicleSeatsRange));
        }
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    let uniq_areas: Vec<i64> = {
        let mut s = std::collections::BTreeSet::new();
        req.area_ids
            .iter()
            .copied()
            .filter(|a| s.insert(*a))
            .collect()
    };
    if !uniq_areas.is_empty() {
        let valid_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM areas WHERE id = ANY($1)")
            .bind(&uniq_areas)
            .fetch_one(&mut *tx)
            .await?;
        if valid_count != uniq_areas.len() as i64 {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    let mut active_seats: Option<i64> = req.vehicle.as_ref().map(|v| v.seats);
    if active_seats.is_none() {
        active_seats = if let Some(target_id) = req.active_vehicle_id {
            sqlx::query_scalar("SELECT seats FROM vehicles WHERE id=$1 AND user_id=$2")
                .bind(target_id)
                .bind(u.id)
                .fetch_optional(&mut *tx)
                .await?
        } else {
            sqlx::query_scalar(
                "SELECT seats FROM vehicles WHERE user_id=$1 AND is_active=1 LIMIT 1",
            )
            .bind(u.id)
            .fetch_optional(&mut *tx)
            .await?
        };
    }
    let effective_spare = match active_seats {
        Some(seats) if seats >= 2 => req.spare_seats.min((seats - 1).max(1)),
        _ => req.spare_seats,
    };
    sqlx::query("UPDATE users SET carpool_enabled=$1, spare_seats=$2 WHERE id=$3")
        .bind(req.carpool_enabled as i64)
        .bind(effective_spare)
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
        effective_spare,
        req.area_ids
    );

    if let Some(target_id) = req.active_vehicle_id {
        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM vehicles WHERE id=$1 AND user_id=$2")
                .bind(target_id)
                .bind(u.id)
                .fetch_optional(&mut *tx)
                .await?;
        if exists.is_some() {
            sqlx::query("UPDATE vehicles SET is_active=0 WHERE user_id=$1")
                .bind(u.id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE vehicles SET is_active=1 WHERE id=$1 AND user_id=$2")
                .bind(target_id)
                .bind(u.id)
                .execute(&mut *tx)
                .await?;
            detail.push_str(&format!("; activated vehicle {target_id}"));
        }
    }

    if let Some(v) = req.vehicle {
        if v.model.trim().is_empty() || v.plate.trim().is_empty() {
            return Err(bad(u.lang, ErrorKey::VehicleModelAndPlateRequired));
        }
        if !(2..=9).contains(&v.seats) {
            return Err(bad(u.lang, ErrorKey::VehicleSeatsRange));
        }
        let active_v: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM vehicles WHERE user_id=$1 AND is_active=1")
                .bind(u.id)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some((active_id,)) = active_v {
            sqlx::query(
                "UPDATE vehicles SET
                     model=$1, color=$2, plate=$3, seats=$4, vehicle_type=$5,
                     wheelchair_ok=COALESCE($6, wheelchair_ok),
                     low_step=COALESCE($7, low_step),
                     child_seat=COALESCE($8, child_seat),
                     updated_at=app_now()
                 WHERE id=$9",
            )
            .bind(v.model.trim())
            .bind(v.color.trim())
            .bind(v.plate.trim().to_uppercase())
            .bind(v.seats)
            .bind(v.vehicle_type.trim())
            .bind(v.wheelchair_ok.map(|b| if b { 1i64 } else { 0i64 }))
            .bind(v.low_step.map(|b| if b { 1i64 } else { 0i64 }))
            .bind(v.child_seat.map(|b| if b { 1i64 } else { 0i64 }))
            .bind(active_id)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO vehicles (user_id, model, color, plate, seats, vehicle_type, wheelchair_ok, low_step, child_seat, is_active)
                 VALUES ($1,$2,$3,$4,$5,$6,COALESCE($7, 0),COALESCE($8, 0),COALESCE($9, 0),1)
                 ON CONFLICT (user_id, plate) DO UPDATE SET
                     model=excluded.model, color=excluded.color, seats=excluded.seats, vehicle_type=excluded.vehicle_type,
                     wheelchair_ok=COALESCE($7, vehicles.wheelchair_ok),
                     low_step=COALESCE($8, vehicles.low_step),
                     child_seat=COALESCE($9, vehicles.child_seat),
                     is_active=1, updated_at=app_now()"
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
        }
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

pub(super) async fn list_driver_vehicles(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::Drive, Act::View)?;
    let vehicles: Vec<(i64, String, String, String, i64, String, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT id, model, color, plate, seats, vehicle_type, wheelchair_ok, low_step, child_seat, is_active FROM vehicles WHERE user_id=$1 ORDER BY is_active DESC, id ASC",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;

    let list: Vec<Value> = vehicles
        .into_iter()
        .map(|(id, m, c, p, s, t, w, l, cs, act)| {
            json!({
                "id": id,
                "model": m,
                "color": c,
                "plate": p,
                "seats": s,
                "type": t,
                "vehicle_type": t,
                "wheelchair_ok": w == 1,
                "low_step": l == 1,
                "child_seat": cs == 1,
                "is_active": act == 1,
            })
        })
        .collect();
    Ok(Json(json!(list)))
}

pub(super) async fn create_driver_vehicle(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<VehicleIn>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Drive, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
    }
    let model = req.model.trim();
    let plate = req.plate.trim().to_uppercase();
    let color = req.color.trim();
    let vtype = req.vehicle_type.trim();
    if model.is_empty() || plate.is_empty() || color.is_empty() {
        return Err(bad(u.lang, ErrorKey::VehicleModelAndPlateRequired));
    }
    if !matches!(vtype, "standard" | "other") {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(2..=9).contains(&req.seats) {
        return Err(bad(u.lang, ErrorKey::VehicleSeatsRange));
    }

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;

    let existing_by_plate: Option<(i64, i64)> = sqlx::query_as(
        "SELECT id, is_active FROM vehicles WHERE user_id=$1 AND plate=$2 FOR UPDATE",
    )
    .bind(u.id)
    .bind(&plate)
    .fetch_optional(&mut *tx)
    .await?;
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vehicles WHERE user_id=$1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    if existing_by_plate.is_none() && count.0 >= 10 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let mut should_activate = req.is_active.unwrap_or(false) || count.0 == 0;
    if !should_activate && existing_by_plate.as_ref().is_some_and(|(_, act)| *act == 1) {
        let other: Option<(i64, i64)> = sqlx::query_as(
            "SELECT id, seats FROM vehicles WHERE user_id=$1 AND plate<>$2 ORDER BY id ASC LIMIT 1",
        )
        .bind(u.id)
        .bind(&plate)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((other_id, other_seats)) = other {
            sqlx::query("UPDATE vehicles SET is_active=1 WHERE id=$1 AND user_id=$2")
                .bind(other_id)
                .bind(u.id)
                .execute(&mut *tx)
                .await?;
            clamp_spare_seats(&mut tx, u.id, other_seats).await?;
        } else {
            should_activate = true;
        }
    }
    if should_activate {
        sqlx::query("UPDATE vehicles SET is_active=0 WHERE user_id=$1 AND plate<>$2")
            .bind(u.id)
            .bind(&plate)
            .execute(&mut *tx)
            .await?;
    }
    let is_act_val = if should_activate { 1i64 } else { 0i64 };
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "INSERT INTO vehicles (user_id, model, color, plate, seats, vehicle_type, wheelchair_ok, low_step, child_seat, is_active)
         VALUES ($1,$2,$3,$4,$5,$6,COALESCE($7, 0),COALESCE($8, 0),COALESCE($9, 0),$10)
         ON CONFLICT (user_id, plate) DO UPDATE SET
             model=excluded.model,
             color=excluded.color,
             seats=excluded.seats,
             vehicle_type=excluded.vehicle_type,
             wheelchair_ok=COALESCE($7, vehicles.wheelchair_ok),
             low_step=COALESCE($8, vehicles.low_step),
             child_seat=COALESCE($9, vehicles.child_seat),
             is_active=excluded.is_active,
             updated_at=app_now()
         RETURNING id, wheelchair_ok, low_step, child_seat",
    )
    .bind(u.id)
    .bind(model)
    .bind(color)
    .bind(&plate)
    .bind(req.seats)
    .bind(vtype)
    .bind(req.wheelchair_ok.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(req.low_step.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(req.child_seat.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(is_act_val)
    .fetch_one(&mut *tx)
    .await?;
    if should_activate {
        clamp_spare_seats(&mut tx, u.id, req.seats).await?;
    }

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "driver.vehicle_create",
        "user",
        Some(u.id),
        format!(
            "vehicle {} {} ({} seats, active: {})",
            plate, model, req.seats, should_activate
        ),
    )
    .await;

    Ok(Json(json!({
        "id": row.0,
        "model": model,
        "color": color,
        "plate": plate,
        "seats": req.seats,
        "type": vtype,
        "vehicle_type": vtype,
        "wheelchair_ok": row.1 != 0,
        "low_step": row.2 != 0,
        "child_seat": row.3 != 0,
        "is_active": should_activate,
    })))
}

pub(super) async fn update_driver_vehicle(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<VehicleIn>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Drive, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
    }
    let model = req.model.trim();
    let plate = req.plate.trim().to_uppercase();
    let color = req.color.trim();
    let vtype = req.vehicle_type.trim();
    if model.is_empty() || plate.is_empty() || color.is_empty() {
        return Err(bad(u.lang, ErrorKey::VehicleModelAndPlateRequired));
    }
    if !matches!(vtype, "standard" | "other") {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(2..=9).contains(&req.seats) {
        return Err(bad(u.lang, ErrorKey::VehicleSeatsRange));
    }

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;

    let existing: Option<(i64, i64)> =
        sqlx::query_as("SELECT id, is_active FROM vehicles WHERE id=$1 AND user_id=$2 FOR UPDATE")
            .bind(id)
            .bind(u.id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((_, current_active)) = existing else {
        return Err(not_found(u.lang));
    };

    let mut should_activate = req.is_active.unwrap_or(current_active == 1);
    if !should_activate && current_active == 1 {
        let other: Option<(i64, i64)> = sqlx::query_as(
            "SELECT id, seats FROM vehicles WHERE user_id=$1 AND id<>$2 ORDER BY id ASC LIMIT 1",
        )
        .bind(u.id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((other_id, other_seats)) = other {
            sqlx::query("UPDATE vehicles SET is_active=1 WHERE id=$1 AND user_id=$2")
                .bind(other_id)
                .bind(u.id)
                .execute(&mut *tx)
                .await?;
            clamp_spare_seats(&mut tx, u.id, other_seats).await?;
        } else {
            should_activate = true;
        }
    } else if should_activate && current_active != 1 {
        sqlx::query("UPDATE vehicles SET is_active=0 WHERE user_id=$1 AND id<>$2")
            .bind(u.id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    let is_act_val = if should_activate { 1i64 } else { 0i64 };
    let updated: (i64, i64, i64) = match sqlx::query_as(
        "UPDATE vehicles SET
            model=$1, color=$2, plate=$3, seats=$4, vehicle_type=$5,
            wheelchair_ok=COALESCE($6, wheelchair_ok),
            low_step=COALESCE($7, low_step),
            child_seat=COALESCE($8, child_seat),
            is_active=$9,
            updated_at=app_now()
         WHERE id=$10 AND user_id=$11
         RETURNING wheelchair_ok, low_step, child_seat",
    )
    .bind(model)
    .bind(color)
    .bind(&plate)
    .bind(req.seats)
    .bind(vtype)
    .bind(req.wheelchair_ok.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(req.low_step.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(req.child_seat.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(is_act_val)
    .bind(id)
    .bind(u.id)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(row) => row,
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
            return Err(conflict(u.lang, ErrorKey::InvalidAction));
        }
        Err(e) => return Err(e.into()),
    };
    if should_activate {
        clamp_spare_seats(&mut tx, u.id, req.seats).await?;
    }

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "driver.vehicle_update",
        "user",
        Some(u.id),
        format!(
            "updated vehicle {} {} ({} seats, active: {})",
            plate, model, req.seats, should_activate
        ),
    )
    .await;

    Ok(Json(json!({
        "id": id,
        "model": model,
        "color": color,
        "plate": plate,
        "seats": req.seats,
        "type": vtype,
        "vehicle_type": vtype,
        "wheelchair_ok": updated.0 != 0,
        "low_step": updated.1 != 0,
        "child_seat": updated.2 != 0,
        "is_active": should_activate,
    })))
}

pub(super) async fn delete_driver_vehicle(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Drive, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;

    let existing: Option<(i64, String, i64)> = sqlx::query_as(
        "SELECT id, plate, is_active FROM vehicles WHERE id=$1 AND user_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(u.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((_, plate, is_active)) = existing else {
        return Err(not_found(u.lang));
    };

    sqlx::query("DELETE FROM vehicles WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;

    if is_active == 1 {
        let next_v: Option<(i64, i64)> = sqlx::query_as(
            "SELECT id, seats FROM vehicles WHERE user_id=$1 ORDER BY id ASC LIMIT 1",
        )
        .bind(u.id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((next_id, next_seats)) = next_v {
            sqlx::query("UPDATE vehicles SET is_active=1 WHERE id=$1")
                .bind(next_id)
                .execute(&mut *tx)
                .await?;
            clamp_spare_seats(&mut tx, u.id, next_seats).await?;
        }
    }
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "driver.vehicle_delete",
        "user",
        Some(u.id),
        format!("deleted vehicle {plate} ({id})"),
    )
    .await;

    Ok(Json(json!({"ok": true})))
}

pub(super) async fn select_driver_vehicle(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Drive, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
    }
    let existing: Option<(i64, String, String, i64)> =
        sqlx::query_as("SELECT id, model, plate, seats FROM vehicles WHERE id=$1 AND user_id=$2")
            .bind(id)
            .bind(u.id)
            .fetch_optional(&st.db)
            .await?;
    let Some((_, model, plate, seats)) = existing else {
        return Err(not_found(u.lang));
    };

    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE vehicles SET is_active=0 WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE vehicles SET is_active=1 WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    clamp_spare_seats(&mut tx, u.id, seats).await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "driver.vehicle_select",
        "user",
        Some(u.id),
        format!("selected active vehicle {plate} {model}"),
    )
    .await;

    Ok(Json(json!({"ok": true, "active_vehicle_id": id})))
}
