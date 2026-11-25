//! Transport planning board: transport staff arrange every ride request for one event at
//! once (e.g. 6 drivers and 24 passengers), instead of one reassignment at a time.
//!
//! - `GET  /admin/traffic/plans` — upcoming / active events with ride & draft counts for the Traffic Hub.
//! - `GET  /admin/events/{id}/plan` — the event's open ride requests, the active drivers,
//!   all draft versions (with single-editor lock state) and visitors.
//! - `POST /admin/events/{id}/plan` — apply a batch of changes (and optionally approve a draft) in one transaction.
//! - `POST /admin/events/{id}/plan/drafts` — create a new draft version (optionally cloned) & lock it.
//! - `PUT/DELETE /admin/events/{id}/plan/drafts/{did}` — update or delete a draft version.
//! - `POST/DELETE /admin/events/{id}/plan/drafts/{did}/lock` — acquire, renew, or release the single-editor lock.
//! - `POST /admin/events/{id}/plan/arrange` — suggest drivers for passengers without one.

mod apply;
mod drafts;

use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::arrange::{arrange, Car, Rider};
use crate::auth::CurrentUser;
use crate::error::{forbidden, not_found, AppResult};
use crate::perms::{self, Act, Page, Perm};
use crate::routes::rides::seat_capacity;
use crate::state::AppState;
use crate::util::{now, require, ErrorKey};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/traffic/plans", get(list_traffic_plans))
        .route(
            "/admin/events/{id}/plan",
            get(get_plan).post(apply::save_plan),
        )
        .route("/admin/events/{id}/plan/draft", post(drafts::save_draft))
        .route("/admin/events/{id}/plan/drafts", post(drafts::create_draft))
        .route(
            "/admin/events/{id}/plan/drafts/{did}",
            put(drafts::update_draft).delete(drafts::delete_draft),
        )
        .route(
            "/admin/events/{id}/plan/drafts/{did}/lock",
            post(drafts::acquire_draft_lock).delete(drafts::release_draft_lock),
        )
        .route(
            "/admin/events/{id}/plan/request-approval",
            post(drafts::request_approval),
        )
        .route("/admin/events/{id}/plan/return", post(drafts::return_draft))
        .route("/admin/events/{id}/plan/approve", post(apply::approve_plan))
        .route("/admin/events/{id}/plan/arrange", post(arrange_plan))
}

/// Ride states the board may (re)assign. In-progress and finished rides are left alone.
pub(super) const PLANNABLE: [&str; 5] = [
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
pub(super) struct PlanRide {
    pub(super) id: i64,
    pub(super) passenger_id: i64,
    pub(super) p_name: String,
    pub(super) seats: i64,
    pub(super) area_id: i64,
    pub(super) area_name: String,
    pub(super) pickup_at: String,
    pub(super) pickup_address: String,
    pub(super) assistance: String,
    pub(super) notes: String,
    pub(super) status: String,
    pub(super) driver_id: Option<i64>,
    pub(super) trip_id: Option<i64>,
    pub(super) leg: String,
    pub(super) paired_ride_id: Option<i64>,
    /// The group drivers are matched on (`audience::MATCH_GROUP_SQL`).
    pub(super) match_group_id: Option<i64>,
    pub(super) reimbursement_excluded: i64,
}

#[derive(sqlx::FromRow)]
pub(super) struct PlanDriver {
    pub(super) id: i64,
    pub(super) name: String,
    pub(super) carpool_enabled: i64,
    pub(super) spare_seats: i64,
    pub(super) v_model: Option<String>,
    pub(super) v_plate: Option<String>,
    pub(super) v_seats: Option<i64>,
    pub(super) wheelchair_ok: i64,
    pub(super) low_step: i64,
    pub(super) child_seat: i64,
}

pub(crate) struct EventRow {
    pub(super) title: String,
    pub(super) starts_at: String,
    pub(super) location: String,
    pub(crate) status: String,
    pub(super) created_by: i64,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct DraftEntry {
    pub(super) ride_id: i64,
    pub(super) driver_id: Option<i64>,
}

pub(crate) async fn load_event(st: &AppState, u: &CurrentUser, id: i64) -> AppResult<EventRow> {
    let row: Option<(String, String, String, String, i64)> = sqlx::query_as(
        "SELECT title, starts_at, location, status, created_by FROM events WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    let (title, starts_at, location, status, created_by) = row.ok_or_else(|| not_found(u.lang))?;
    // Global traffic staff plan any event; a cell group leader only events they created.
    if !u.traffic_is_global() && created_by != u.id {
        return Err(forbidden(u.lang, ErrorKey::CannotViewEvent));
    }
    Ok(EventRow {
        title,
        starts_at,
        location,
        status,
        created_by,
    })
}

pub(super) async fn plan_rides_on<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    event: i64,
) -> AppResult<Vec<PlanRide>> {
    Ok(sqlx::query_as(crate::dyn_sql!(
        "SELECT r.id, r.passenger_id,
                CASE WHEN r.is_proxy = 1 AND r.rider_name != '' THEN r.rider_name || ' (' || p.name || ' 代订)' ELSE p.name END AS p_name,
                r.seats, r.pickup_area_id AS area_id,
                a.name AS area_name, r.pickup_at, r.pickup_address, r.assistance, r.notes,
                r.status, r.driver_id, r.trip_id, r.leg, r.paired_ride_id, {} AS match_group_id,
                r.reimbursement_excluded
         FROM rides r JOIN users p ON p.id = r.passenger_id JOIN areas a ON a.id = r.pickup_area_id
         WHERE r.event_id = $1 AND r.status = ANY($2)
         ORDER BY a.name, r.pickup_at, p.name",
        crate::audience::MATCH_GROUP_SQL
    ))
    .bind(event)
    .bind(&PLANNABLE[..])
    .fetch_all(db)
    .await?)
}

pub(super) async fn plan_rides(st: &AppState, event: i64) -> AppResult<Vec<PlanRide>> {
    plan_rides_on(&st.db, event).await
}

/// Active members who may take rides (接送 · create) and have a vehicle on file.
pub(super) async fn plan_drivers_on<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
) -> AppResult<Vec<PlanDriver>> {
    Ok(sqlx::query_as::<_, PlanDriver>(crate::dyn_sql!(
        "SELECT u.id, u.name, u.carpool_enabled, u.spare_seats, v.model AS v_model, v.plate AS v_plate, v.seats AS v_seats,
                v.wheelchair_ok, v.low_step, v.child_seat
         FROM users u JOIN vehicles v ON v.user_id = u.id AND v.is_active = 1 WHERE {} ORDER BY u.name",
        perms::sql_holds("u", Perm(Page::Drive, Act::Create))
    ))
    .fetch_all(db)
    .await?)
}

pub(super) async fn plan_drivers(st: &AppState) -> AppResult<Vec<PlanDriver>> {
    plan_drivers_on(&st.db).await
}

/// Seats and bookings each driver already holds outside this event, partitioned by leg window.
pub(super) struct OtherLegLoad {
    pub(super) outbound: HashMap<i64, (i64, i64)>,
    pub(super) ret: HashMap<i64, (i64, i64)>,
}

async fn query_other_window_on<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    event: i64,
    from: &str,
    to: &str,
) -> AppResult<HashMap<i64, (i64, i64)>> {
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
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(d, s, n)| (d, (s, n))).collect())
}

pub(super) async fn other_load_on(
    conn: &mut sqlx::PgConnection,
    event: i64,
    rides: &[PlanRide],
    starts_at: &str,
) -> AppResult<OtherLegLoad> {
    let (out_from, out_to) = match (
        rides
            .iter()
            .filter(|r| r.leg != "return")
            .map(|r| r.pickup_at.as_str())
            .min(),
        rides
            .iter()
            .filter(|r| r.leg != "return")
            .map(|r| r.pickup_at.as_str())
            .max(),
    ) {
        (Some(a), Some(b)) => (a.to_string(), b.to_string()),
        _ => (starts_at.to_string(), starts_at.to_string()),
    };
    let outbound = query_other_window_on(&mut *conn, event, &out_from, &out_to).await?;
    let ret = match (
        rides
            .iter()
            .filter(|r| r.leg == "return")
            .map(|r| r.pickup_at.as_str())
            .min(),
        rides
            .iter()
            .filter(|r| r.leg == "return")
            .map(|r| r.pickup_at.as_str())
            .max(),
    ) {
        (Some(a), Some(b)) => query_other_window_on(&mut *conn, event, a, b).await?,
        _ => HashMap::new(),
    };
    Ok(OtherLegLoad { outbound, ret })
}

pub(super) async fn other_load(
    st: &AppState,
    event: i64,
    rides: &[PlanRide],
    starts_at: &str,
) -> AppResult<OtherLegLoad> {
    let mut conn = st.db.acquire().await?;
    other_load_on(&mut conn, event, rides, starts_at).await
}

/// Everything the board shows, loaded once for viewing or arranging.
struct Board {
    ev: EventRow,
    rides: Vec<PlanRide>,
    drivers: Vec<PlanDriver>,
    other: OtherLegLoad,
    driver_areas: HashMap<i64, Vec<i64>>,
    /// Passenger → (previous driver, name, rides together, last ride time).
    last: HashMap<i64, (i64, String, i64, String)>,
    /// Driver → the groups they may drive for (`audience::drive_scopes`).
    scopes: HashMap<i64, Vec<i64>>,
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
    let scopes = crate::audience::drive_scopes(&st.db).await?;
    Ok(Board {
        ev,
        rides,
        drivers,
        other,
        driver_areas,
        last,
        scopes,
    })
}

async fn list_traffic_plans(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::View)?;
    let today = crate::util::today_local().format("%Y-%m-%d").to_string();
    let cutoff = format!("{today}T00:00");
    let now_str = now();
    let scope_cond = if u.traffic_is_global() {
        "TRUE".to_string()
    } else {
        "e.created_by = $1".to_string()
    };

    #[derive(sqlx::FromRow)]
    struct TrafficEventRow {
        id: i64,
        title: String,
        kind: String,
        icon: Option<String>,
        starts_at: String,
        location: String,
        status: String,
        area_name: Option<String>,
        groups: Option<String>,
        mileage_claimable: i64,
        ride_count: i64,
        unassigned_count: i64,
        pending_confirm_count: i64,
        visitor_count: i64,
        draft_count: i64,
        pending_draft_count: i64,
        returned_draft_count: i64,
    }

    let sql = format!(
        "SELECT e.id, e.title, e.kind, e.icon, e.starts_at, e.location, e.status,
                (SELECT name FROM areas WHERE id = e.area_id) AS area_name,
                (SELECT string_agg(g.name, '、') FROM event_groups eg JOIN groups g ON g.id = eg.group_id WHERE eg.event_id = e.id) AS groups,
                e.mileage_claimable,
                (SELECT COUNT(*) FROM rides r WHERE r.event_id = e.id AND r.status NOT IN ('cancelled','no_show','alt_transport')) AS ride_count,
                (SELECT COUNT(*) FROM rides r WHERE r.event_id = e.id AND r.status = 'pending_driver') AS unassigned_count,
                (SELECT COUNT(*) FROM rides r WHERE r.event_id = e.id AND r.status IN ('pending_driver_confirm','pending_passenger','pending_approval')) AS pending_confirm_count,
                (SELECT COUNT(*) FROM plan_visitors pv WHERE pv.event_id = e.id) AS visitor_count,
                (SELECT COUNT(*) FROM event_plan_drafts d WHERE d.event_id = e.id AND d.status <> 'superseded') AS draft_count,
                (SELECT COUNT(*) FROM event_plan_drafts d WHERE d.event_id = e.id AND d.status = 'pending_approval') AS pending_draft_count,
                (SELECT COUNT(*) FROM event_plan_drafts d WHERE d.event_id = e.id AND d.status = 'returned') AS returned_draft_count
         FROM events e
         WHERE {scope_cond}
           AND (
               (e.status = 'published' AND e.starts_at >= $2)
               OR EXISTS (
                   SELECT 1 FROM event_plan_drafts d
                   WHERE d.event_id = e.id AND d.status IN ('draft', 'pending_approval', 'returned')
               )
           )
         ORDER BY (e.starts_at < $2) ASC,
                  CASE WHEN e.starts_at < $2 THEN NULL ELSE e.starts_at END ASC,
                  e.starts_at DESC, e.id ASC
         LIMIT 100"
    );

    let rows = sqlx::query_as::<_, TrafficEventRow>(sqlx::AssertSqlSafe(&*sql))
        .bind(u.id)
        .bind(&cutoff)
        .fetch_all(&st.db)
        .await?;

    let event_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    let draft_rows: Vec<drafts::DraftRow> = if event_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(crate::dyn_sql!(
            "{} WHERE d.event_id = ANY($1) AND d.status <> 'superseded' ORDER BY d.version DESC, d.id DESC",
            drafts::DRAFT_SELECT
        ))
        .bind(&event_ids)
        .fetch_all(&st.db)
        .await?
    };

    let mut drafts_by_event: HashMap<i64, Vec<drafts::DraftRow>> = HashMap::new();
    for d in draft_rows {
        drafts_by_event.entry(d.event_id).or_default().push(d);
    }

    let list: Vec<Value> = rows
        .into_iter()
        .map(|ev| {
            let ev_drafts = drafts_by_event.get(&ev.id);
            let latest = ev_drafts.and_then(|list| {
                list.iter()
                    .find(|d| d.status == "pending_approval")
                    .or_else(|| list.iter().find(|d| d.status == "returned"))
                    .or_else(|| list.iter().find(|d| d.status == "draft"))
                    .or_else(|| list.iter().find(|d| d.status == "approved"))
                    .or_else(|| list.first())
            });
            json!({
                "id": ev.id,
                "title": ev.title,
                "kind": ev.kind,
                "icon": ev.icon,
                "starts_at": ev.starts_at,
                "location": ev.location,
                "status": ev.status,
                "area_name": ev.area_name,
                "groups": ev.groups.unwrap_or_default(),
                "mileage_claimable": ev.mileage_claimable == 1,
                "ride_count": ev.ride_count,
                "unassigned_count": ev.unassigned_count,
                "pending_confirm_count": ev.pending_confirm_count,
                "visitor_count": ev.visitor_count,
                "draft_count": ev.draft_count,
                "pending_draft_count": ev.pending_draft_count,
                "returned_draft_count": ev.returned_draft_count,
                "latest_draft": latest.map(|d| drafts::draft_to_json(d, u.id, &now_str)),
            })
        })
        .collect();

    Ok(Json(json!(list)))
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
        scopes,
    } = load_board(&st, &u, id).await?;

    let now_str = now();
    let draft_rows = drafts::load_event_drafts(&st, id).await?;
    let drafts_json: Vec<Value> = draft_rows
        .iter()
        .map(|d| drafts::draft_to_json(d, u.id, &now_str))
        .collect();

    let primary_draft = draft_rows
        .iter()
        .find(|d| d.status == "pending_approval")
        .or_else(|| {
            draft_rows
                .iter()
                .find(|d| d.status == "draft" || d.status == "returned")
        })
        .or_else(|| draft_rows.iter().find(|d| d.status == "approved"))
        .or_else(|| draft_rows.first())
        .map(|d| drafts::draft_to_json(d, u.id, &now_str));

    let visitors = crate::routes::plan_visitors::list(&st, &u, id).await?;
    Ok(Json(json!({
        "event": { "id": id, "title": ev.title, "starts_at": ev.starts_at, "location": ev.location, "status": ev.status },
        "draft": primary_draft,
        "drafts": drafts_json,
        "can_approve": u.is_traffic_lead(),
        "visitors": visitors,
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
            "leg": r.leg,
            "paired_ride_id": r.paired_ride_id,
            "match_group_id": r.match_group_id,
            "reimbursement_excluded": r.reimbursement_excluded == 1,
            "last": last.get(&r.passenger_id).map(|(d, name, times, at)| json!({
                "driver_id": d, "driver_name": name, "times": times, "at": at,
            })),
        })).collect::<Vec<_>>(),
        "drivers": drivers.iter().map(|d| {
            let (out_load, out_bookings) = other.outbound.get(&d.id).copied().unwrap_or((0, 0));
            let (ret_load, ret_bookings) = other.ret.get(&d.id).copied().unwrap_or((0, 0));
            json!({
                "id": d.id,
                "name": d.name,
                "carpool": d.carpool_enabled == 1,
                "cap": seat_capacity(d.spare_seats, d.v_seats),
                "other_seats": out_load,
                "other_rides": out_bookings,
                "other_outbound_seats": out_load,
                "other_outbound_rides": out_bookings,
                "other_return_seats": ret_load,
                "other_return_rides": ret_bookings,
                "area_ids": driver_areas.get(&d.id).cloned().unwrap_or_default(),
                "drive_group_ids": scopes.get(&d.id).cloned().unwrap_or_default(),
                "vehicle": {
                    "model": d.v_model,
                    "plate": d.v_plate,
                    "seats": d.v_seats,
                    "wheelchair_ok": d.wheelchair_ok == 1,
                    "low_step": d.low_step == 1,
                    "child_seat": d.child_seat == 1,
                },
            })
        }).collect::<Vec<_>>(),
    })))
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

    // Visitors recorded in a car take its seats too (they are not rides, so no booking).
    let visitor_rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT driver_id, leg, COALESCE(SUM(seats), 0)::BIGINT FROM plan_visitors
         WHERE event_id = $1 AND driver_id IS NOT NULL GROUP BY driver_id, leg",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    let visitor_seats = |driver: i64, want_return: bool| -> i64 {
        visitor_rows
            .iter()
            .filter(|(d, leg, _)| *d == driver && (leg == "return") == want_return)
            .map(|(_, _, n)| n)
            .sum()
    };

    let arrange_leg = |want_return: bool, other_map: &HashMap<i64, (i64, i64)>| {
        let leg_rides: Vec<&PlanRide> = b
            .rides
            .iter()
            .filter(|r| (r.leg == "return") == want_return)
            .collect();
        let riders: Vec<Rider> = leg_rides
            .iter()
            .map(|r| Rider {
                ride_id: r.id,
                passenger_id: r.passenger_id,
                seats: r.seats,
                area_id: r.area_id,
                point: points.get(&r.pickup_address).copied(),
                needs_assistance: !r.assistance.is_empty(),
                assistance: r.assistance.clone(),
                last: b.last.get(&r.passenger_id).map(|(d, _, t, _)| (*d, *t)),
                group: r.match_group_id,
            })
            .collect();
        let cars: Vec<Car> = b
            .drivers
            .iter()
            .map(|d| {
                let (o_seats, o_rides) = other_map.get(&d.id).copied().unwrap_or((0, 0));
                Car {
                    driver_id: d.id,
                    seats_left: seat_capacity(d.spare_seats, d.v_seats)
                        - o_seats
                        - visitor_seats(d.id, want_return),
                    carpool: d.carpool_enabled == 1,
                    other_rides: o_rides,
                    area_ids: b.driver_areas.get(&d.id).cloned().unwrap_or_default(),
                    drive_groups: b.scopes.get(&d.id).cloned().unwrap_or_default(),
                    wheelchair_ok: d.wheelchair_ok == 1,
                    low_step: d.low_step == 1,
                    child_seat: d.child_seat == 1,
                }
            })
            .collect();
        let known: HashSet<i64> = leg_rides.iter().map(|r| r.id).collect();
        let fixed: HashMap<i64, i64> = req
            .draft
            .iter()
            .filter(|e| known.contains(&e.ride_id))
            .filter_map(|e| e.driver_id.map(|d| (e.ride_id, d)))
            .collect();
        arrange(&riders, &cars, &fixed)
    };

    let out_res = arrange_leg(false, &b.other.outbound);
    let ret_res = arrange_leg(true, &b.other.ret);
    let placed_all: Vec<_> = out_res.placed.into_iter().chain(ret_res.placed).collect();
    let unplaced_all: Vec<i64> = out_res
        .unplaced
        .into_iter()
        .chain(ret_res.unplaced)
        .collect();
    let names: HashMap<i64, &str> = b.rides.iter().map(|r| (r.id, r.p_name.as_str())).collect();
    Ok(Json(json!({
        "placed": placed_all.iter().map(|p| json!({
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
        "unplaced": unplaced_all,
        "approximate": approximate,
    })))
}
