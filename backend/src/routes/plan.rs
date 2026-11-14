//! Transport planning board: transport staff arrange every ride request for one event at
//! once (e.g. 6 drivers and 24 passengers), instead of one reassignment at a time.
//!
//! - `GET  /admin/events/{id}/plan` — the event's open ride requests, the active drivers
//!   with their seats and areas, and each passenger's previous driver ("same as last time").
//! - `POST /admin/events/{id}/plan` — apply a batch of changes in one transaction.
//! - `POST /admin/events/{id}/plan/arrange` — suggest drivers for passengers without one
//!   (see `crate::arrange`); changes nothing until the board sends it.
//!
//! Assigning a driver is the same transition as a single staff reassignment: the ride
//! becomes `pending_driver_confirm` and the driver confirms it. Each driver gets one
//! summary notification for the batch. Every change names the driver the board showed
//! (`from_driver_id`); if any ride changed underneath, nothing is applied (`409`).

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::arrange::{arrange, Car, Rider};
use crate::auth::CurrentUser;
use crate::error::{bad, conflict, not_found, AppResult};
use crate::perms::{self, Act, Page, Perm};
use crate::routes::rides::seat_capacity;
use crate::state::AppState;
use crate::util::{audit, notify, now, require, system_message, ErrorKey};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/events/{id}/plan", get(get_plan).post(save_plan))
        .route("/admin/events/{id}/plan/arrange", post(arrange_plan))
}

/// Ride states the board may (re)assign. In-progress and finished rides are left alone.
const PLANNABLE: [&str; 5] = [
    "pending_driver",
    "pending_passenger",
    "pending_approval",
    "pending_driver_confirm",
    "confirmed",
];
/// States that hold a driver's seats (mirrors `driver_load`).
const HOLDING: [&str; 5] = [
    "pending_passenger",
    "pending_approval",
    "pending_driver_confirm",
    "confirmed",
    "in_progress",
];

#[derive(sqlx::FromRow)]
struct PlanRide {
    id: i64,
    passenger_id: i64,
    p_name: String,
    seats: i64,
    area_id: i64,
    area_name: String,
    pickup_at: String,
    pickup_address: String,
    assistance: String,
    notes: String,
    status: String,
    driver_id: Option<i64>,
    trip_id: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct PlanDriver {
    id: i64,
    name: String,
    carpool_enabled: i64,
    spare_seats: i64,
    v_model: Option<String>,
    v_plate: Option<String>,
    v_seats: Option<i64>,
}

struct EventRow {
    title: String,
    starts_at: String,
    location: String,
    status: String,
}

async fn load_event(st: &AppState, u: &CurrentUser, id: i64) -> AppResult<EventRow> {
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT title, starts_at, location, status FROM events WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    let (title, starts_at, location, status) = row.ok_or_else(|| not_found(u.lang))?;
    Ok(EventRow {
        title,
        starts_at,
        location,
        status,
    })
}

async fn plan_rides(st: &AppState, event: i64) -> AppResult<Vec<PlanRide>> {
    Ok(sqlx::query_as(
        "SELECT r.id, r.passenger_id, p.name AS p_name, r.seats, r.pickup_area_id AS area_id,
                a.name AS area_name, r.pickup_at, r.pickup_address, r.assistance, r.notes,
                r.status, r.driver_id, r.trip_id
         FROM rides r JOIN users p ON p.id = r.passenger_id JOIN areas a ON a.id = r.pickup_area_id
         WHERE r.event_id = $1 AND r.status = ANY($2)
         ORDER BY a.name, r.pickup_at, p.name",
    )
    .bind(event)
    .bind(&PLANNABLE[..])
    .fetch_all(&st.db)
    .await?)
}

/// Active members who may take rides (接送 · create) and have a vehicle on file.
async fn plan_drivers(st: &AppState) -> AppResult<Vec<PlanDriver>> {
    Ok(sqlx::query_as::<
        _,
        (
            i64,
            String,
            i64,
            i64,
            Option<String>,
            Option<String>,
            Option<i64>,
        ),
    >(&format!(
        "SELECT u.id, u.name, u.carpool_enabled, u.spare_seats, v.model, v.plate, v.seats
         FROM users u JOIN vehicles v ON v.user_id = u.id WHERE {} ORDER BY u.name",
        perms::sql_holds("u", Perm(Page::Drive, Act::Create))
    ))
    .fetch_all(&st.db)
    .await?
    .into_iter()
    .map(
        |(id, name, carpool_enabled, spare_seats, v_model, v_plate, v_seats)| PlanDriver {
            id,
            name,
            carpool_enabled,
            spare_seats,
            v_model,
            v_plate,
            v_seats,
        },
    )
    .collect())
}

/// Seats and bookings each driver already holds outside this event, around its pickup times.
async fn other_load(
    st: &AppState,
    event: i64,
    rides: &[PlanRide],
    starts_at: &str,
) -> AppResult<HashMap<i64, (i64, i64)>> {
    let (from, to) = match (
        rides.iter().map(|r| r.pickup_at.as_str()).min(),
        rides.iter().map(|r| r.pickup_at.as_str()).max(),
    ) {
        (Some(a), Some(b)) => (a.to_string(), b.to_string()),
        _ => (starts_at.to_string(), starts_at.to_string()),
    };
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(
        "SELECT driver_id, COALESCE(SUM(seats),0)::BIGINT, COUNT(*) FROM rides
         WHERE driver_id IS NOT NULL AND event_id IS DISTINCT FROM $1 AND status = ANY($2)
           AND pickup_at::timestamp BETWEEN $3::timestamp - interval '1 hour' AND $4::timestamp + interval '1 hour'
         GROUP BY driver_id",
    )
    .bind(event)
    .bind(&HOLDING[..])
    .bind(from)
    .bind(to)
    .fetch_all(&st.db)
    .await?;
    Ok(rows.into_iter().map(|(d, s, n)| (d, (s, n))).collect())
}

/// Everything the board shows, loaded once for viewing or arranging.
struct Board {
    ev: EventRow,
    rides: Vec<PlanRide>,
    drivers: Vec<PlanDriver>,
    other: HashMap<i64, (i64, i64)>,
    driver_areas: HashMap<i64, Vec<i64>>,
    /// Passenger → (previous driver, name, rides together, last ride time).
    last: HashMap<i64, (i64, String, i64, String)>,
}

async fn load_board(st: &AppState, u: &CurrentUser, id: i64) -> AppResult<Board> {
    let ev = load_event(st, u, id).await?;
    let rides = plan_rides(st, id).await?;
    let drivers = plan_drivers(st).await?;
    let other = other_load(st, id, &rides, &ev.starts_at).await?;

    let areas: Vec<(i64, i64)> = sqlx::query_as("SELECT user_id, area_id FROM driver_areas")
        .fetch_all(&st.db)
        .await?;
    let mut driver_areas: HashMap<i64, Vec<i64>> = HashMap::new();
    for (d, a) in areas {
        driver_areas.entry(d).or_default().push(a);
    }

    // "Same as last time": each passenger's most recent driver outside this event, and how
    // often they've ridden together.
    let passengers: Vec<i64> = rides.iter().map(|r| r.passenger_id).collect();
    let history: Vec<(i64, i64, String, i64, String)> = sqlx::query_as(
        "SELECT r.passenger_id, r.driver_id, d.name, COUNT(*), MAX(r.pickup_at)
         FROM rides r JOIN users d ON d.id = r.driver_id
         WHERE r.passenger_id = ANY($1) AND r.event_id IS DISTINCT FROM $2
           AND r.status IN ('completed','confirmed','in_progress')
         GROUP BY r.passenger_id, r.driver_id, d.name",
    )
    .bind(&passengers)
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    let mut last: HashMap<i64, (i64, String, i64, String)> = HashMap::new();
    for (p, d, name, times, at) in history {
        let newer = last.get(&p).is_none_or(|cur| at > cur.3);
        if newer {
            last.insert(p, (d, name, times, at));
        }
    }
    Ok(Board {
        ev,
        rides,
        drivers,
        other,
        driver_areas,
        last,
    })
}

async fn get_plan(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::View)?;
    let Board {
        ev,
        rides,
        drivers,
        other,
        driver_areas,
        last,
    } = load_board(&st, &u, id).await?;

    Ok(Json(json!({
        "event": { "id": id, "title": ev.title, "starts_at": ev.starts_at, "location": ev.location, "status": ev.status },
        "rides": rides.iter().map(|r| json!({
            "id": r.id,
            "passenger": { "id": r.passenger_id, "name": r.p_name },
            "seats": r.seats,
            "area": { "id": r.area_id, "name": r.area_name },
            "pickup_at": r.pickup_at,
            "pickup_address": r.pickup_address,
            "assistance": r.assistance,
            "notes": r.notes,
            "status": r.status,
            "driver_id": r.driver_id,
            "last": last.get(&r.passenger_id).map(|(d, name, times, at)| json!({
                "driver_id": d, "driver_name": name, "times": times, "at": at,
            })),
        })).collect::<Vec<_>>(),
        "drivers": drivers.iter().map(|d| {
            let (load, bookings) = other.get(&d.id).copied().unwrap_or((0, 0));
            json!({
                "id": d.id,
                "name": d.name,
                "carpool": d.carpool_enabled == 1,
                "cap": seat_capacity(d.spare_seats, d.v_seats),
                "other_seats": load,
                "other_rides": bookings,
                "area_ids": driver_areas.get(&d.id).cloned().unwrap_or_default(),
                "vehicle": { "model": d.v_model, "plate": d.v_plate, "seats": d.v_seats },
            })
        }).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct DraftEntry {
    ride_id: i64,
    driver_id: Option<i64>,
}

#[derive(Deserialize)]
struct ArrangeReq {
    /// The board's current draft; passengers with a driver here stay where they are.
    draft: Vec<DraftEntry>,
}

/// Suggest drivers for every passenger the draft leaves without one. Changes nothing:
/// the admin reviews the suggestion and sends it with "Confirm & notify".
async fn arrange_plan(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ArrangeReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let b = load_board(&st, &u, id).await?;

    let addresses: Vec<String> = b
        .rides
        .iter()
        .map(|r| r.pickup_address.clone())
        .filter(|a| !a.trim().is_empty())
        .collect();
    let points = crate::routing::cached_points(&st, &addresses).await;
    let missing: Vec<String> = addresses
        .iter()
        .filter(|a| !points.contains_key(*a))
        .cloned()
        .collect();
    let approximate = b
        .rides
        .iter()
        .filter(|r| !points.contains_key(&r.pickup_address))
        .count();
    crate::routing::warm_geocodes(&st, missing);

    let riders: Vec<Rider> = b
        .rides
        .iter()
        .map(|r| Rider {
            ride_id: r.id,
            passenger_id: r.passenger_id,
            seats: r.seats,
            area_id: r.area_id,
            point: points.get(&r.pickup_address).copied(),
            needs_assistance: !r.assistance.is_empty(),
            last: b.last.get(&r.passenger_id).map(|(d, _, t, _)| (*d, *t)),
        })
        .collect();
    let cars: Vec<Car> = b
        .drivers
        .iter()
        .map(|d| {
            let (o_seats, o_rides) = b.other.get(&d.id).copied().unwrap_or((0, 0));
            Car {
                driver_id: d.id,
                seats_left: seat_capacity(d.spare_seats, d.v_seats) - o_seats,
                carpool: d.carpool_enabled == 1,
                other_rides: o_rides,
                area_ids: b.driver_areas.get(&d.id).cloned().unwrap_or_default(),
            }
        })
        .collect();
    let known: HashSet<i64> = b.rides.iter().map(|r| r.id).collect();
    let fixed: HashMap<i64, i64> = req
        .draft
        .iter()
        .filter(|e| known.contains(&e.ride_id))
        .filter_map(|e| e.driver_id.map(|d| (e.ride_id, d)))
        .collect();

    let result = arrange(&riders, &cars, &fixed);
    let names: HashMap<i64, &str> = b.rides.iter().map(|r| (r.id, r.p_name.as_str())).collect();
    Ok(Json(json!({
        "placed": result.placed.iter().map(|p| json!({
            "ride_id": p.ride_id,
            "driver_id": p.driver_id,
            "near": p.near.map(|(ride, m)| json!({
                "ride_id": ride,
                "name": names.get(&ride),
                "km": m.map(|m| (m / 100.0).round() / 10.0),
            })),
            "in_area": p.in_area,
            "together": p.together,
        })).collect::<Vec<_>>(),
        "unplaced": result.unplaced,
        "approximate": approximate,
    })))
}

#[derive(Deserialize)]
struct Change {
    ride_id: i64,
    /// The driver the board showed for this ride; must still be current.
    from_driver_id: Option<i64>,
    /// New driver, or none to put the ride back in the open pool.
    driver_id: Option<i64>,
}

#[derive(Deserialize)]
struct SavePlan {
    changes: Vec<Change>,
}

async fn save_plan(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SavePlan>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let ev = load_event(&st, &u, id).await?;
    let rides = plan_rides(&st, id).await?;
    let by_id: HashMap<i64, &PlanRide> = rides.iter().map(|r| (r.id, r)).collect();

    // Only real changes, each ride once.
    let mut changes: BTreeMap<i64, &Change> = BTreeMap::new();
    for c in &req.changes {
        let r = by_id
            .get(&c.ride_id)
            .ok_or_else(|| conflict(u.lang, ErrorKey::RideStatusChanged))?;
        if r.driver_id != c.from_driver_id {
            return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
        }
        if c.driver_id != r.driver_id {
            changes.insert(c.ride_id, c);
        }
    }
    if changes.is_empty() {
        return Ok(Json(json!({ "ok": true, "changed": 0 })));
    }

    // Validate every new driver and the seats each driver ends up with.
    let drivers = plan_drivers(&st).await?;
    let driver: HashMap<i64, &PlanDriver> = drivers.iter().map(|d| (d.id, d)).collect();
    let final_driver = |r: &PlanRide| match changes.get(&r.id) {
        Some(c) => c.driver_id,
        None => r.driver_id,
    };
    for c in changes.values() {
        if let Some(d) = c.driver_id {
            if !driver.contains_key(&d) {
                return Err(bad(u.lang, ErrorKey::MemberNotVolunteerDriver));
            }
            if by_id[&c.ride_id].passenger_id == d {
                return Err(bad(u.lang, ErrorKey::DriverCannotBePassenger));
            }
        }
    }
    let other = other_load(&st, id, &rides, &ev.starts_at).await?;
    let touched: HashSet<i64> = changes.values().filter_map(|c| c.driver_id).collect();
    for d in &touched {
        let mine: Vec<&PlanRide> = rides
            .iter()
            .filter(|r| final_driver(r) == Some(*d))
            .collect();
        let (o_seats, o_rides) = other.get(d).copied().unwrap_or((0, 0));
        let load = o_seats + mine.iter().map(|r| r.seats).sum::<i64>();
        let dr = driver[d];
        let cap = seat_capacity(dr.spare_seats, dr.v_seats);
        // Without 共乘模式 a driver takes one booking per time slot.
        let one_booking = dr.carpool_enabled == 1 || o_rides + mine.len() as i64 <= 1;
        if load > cap || !one_booking {
            return Err(bad(
                u.lang,
                ErrorKey::NotEnoughDriverSeatsReassign { load, cap },
            ));
        }
    }

    // Apply all or nothing; each update re-checks the state the board saw.
    let stamp = now();
    let mut tx = st.db.begin().await?;
    for c in changes.values() {
        let res = match c.driver_id {
            Some(d) => sqlx::query(
                "UPDATE rides SET driver_id=$1, status='pending_driver_confirm', trip_id=NULL, approved_by=$2,
                        needs_attention=1, attention_reason='等待新司机确认', updated_at=$3
                 WHERE id=$4 AND event_id=$5 AND status = ANY($6) AND driver_id IS NOT DISTINCT FROM $7",
            )
            .bind(d)
            .bind(u.id),
            None => sqlx::query(
                "UPDATE rides SET driver_id=NULL, status='pending_driver', trip_id=NULL, approved_by=NULL,
                        needs_attention=0, attention_reason='', updated_at=$1
                 WHERE id=$2 AND event_id=$3 AND status = ANY($4) AND driver_id IS NOT DISTINCT FROM $5",
            ),
        };
        let res = res
            .bind(&stamp)
            .bind(c.ride_id)
            .bind(id)
            .bind(&PLANNABLE[..])
            .bind(c.from_driver_id)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
        }
    }
    tx.commit().await?;

    // One summary per driver, after the batch is in.
    let when = ev.starts_at.replace('T', " ");
    let mut gained: BTreeMap<i64, usize> = BTreeMap::new();
    let mut lost: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
    for c in changes.values() {
        let r = by_id[&c.ride_id];
        if let Some(d) = c.driver_id {
            *gained.entry(d).or_default() += 1;
        }
        if let Some(old) = r.driver_id {
            lost.entry(old).or_default().push(r.p_name.as_str());
            if let Some(t) = r.trip_id {
                system_message(
                    &st.db,
                    t,
                    &format!("交通组已重新安排 {} 的接送。", r.p_name),
                    &format!("Transport staff have rearranged {}'s ride.", r.p_name),
                )
                .await;
            }
        }
        // A passenger who already had a confirmed driver hears that it's changing.
        if r.status == "confirmed" {
            notify(
                &st.db,
                r.passenger_id,
                (
                    "交通组正在调整您的司机",
                    "Transport staff are changing your driver",
                ),
                (
                    &format!("{} · {}，确认后会再通知您。", ev.title, when),
                    &format!(
                        "{} · {}. We'll let you know once it's confirmed.",
                        ev.title, when
                    ),
                ),
                &format!("/rides/{}", r.id),
            )
            .await;
        }
        audit(
            &st.db,
            Some(u.id),
            "ride.plan_assign",
            "ride",
            Some(r.id),
            format!("event {id}: driver {:?} → {:?}", r.driver_id, c.driver_id),
        )
        .await;
    }
    for (d, n) in &gained {
        notify(
            &st.db,
            *d,
            (
                &format!("交通组为您安排了 {n} 位乘客"),
                &format!(
                    "Transport staff arranged {n} passenger{} for you",
                    if *n == 1 { "" } else { "s" }
                ),
            ),
            (
                &format!("{} · {} · 请在「开车」确认", ev.title, when),
                &format!("{} · {} · please confirm in Driving", ev.title, when),
            ),
            "/drive",
        )
        .await;
    }
    for (d, names) in &lost {
        let line = format!("{} · {}", ev.title, names.join(", "));
        notify(
            &st.db,
            *d,
            ("接送已改派", "Ride reassigned"),
            (&line, &line),
            "/drive",
        )
        .await;
    }

    Ok(Json(json!({ "ok": true, "changed": changes.len() })))
}
