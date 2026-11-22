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

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::arrange::{arrange, Car, Rider};
use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, locked, not_found, AppResult};
use crate::perms::{self, Act, Page, Perm};
use crate::routes::plan_visitors::{clean as clean_visitor, VisitorIn};
use crate::routes::rides::seat_capacity;
use crate::state::AppState;
use crate::util::{audit, in_minutes, notify, now, require, system_message, ErrorKey};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/traffic/plans", get(list_traffic_plans))
        .route("/admin/events/{id}/plan", get(get_plan).post(save_plan))
        .route("/admin/events/{id}/plan/draft", post(save_draft))
        .route("/admin/events/{id}/plan/drafts", post(create_draft))
        .route(
            "/admin/events/{id}/plan/drafts/{did}",
            put(update_draft).delete(delete_draft),
        )
        .route(
            "/admin/events/{id}/plan/drafts/{did}/lock",
            post(acquire_draft_lock).delete(release_draft_lock),
        )
        .route(
            "/admin/events/{id}/plan/request-approval",
            post(request_approval),
        )
        .route("/admin/events/{id}/plan/return", post(return_draft))
        .route("/admin/events/{id}/plan/approve", post(approve_plan))
        .route("/admin/events/{id}/plan/arrange", post(arrange_plan))
}

const DRAFT_LOCK_MINUTES: i64 = 10;
const LABEL_MAX: usize = 60;
const MAX_DRAFT_VISITORS: usize = 50;

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
    leg: String,
    paired_ride_id: Option<i64>,
    /// The group drivers are matched on (`audience::MATCH_GROUP_SQL`).
    match_group_id: Option<i64>,
    reimbursement_excluded: i64,
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
    wheelchair_ok: i64,
    low_step: i64,
    child_seat: i64,
}

pub(crate) struct EventRow {
    title: String,
    starts_at: String,
    location: String,
    pub(crate) status: String,
    created_by: i64,
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

async fn plan_rides(st: &AppState, event: i64) -> AppResult<Vec<PlanRide>> {
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
    .fetch_all(&st.db)
    .await?)
}

/// Active members who may take rides (接送 · create) and have a vehicle on file.
async fn plan_drivers(st: &AppState) -> AppResult<Vec<PlanDriver>> {
    Ok(sqlx::query_as::<_, PlanDriver>(crate::dyn_sql!(
        "SELECT u.id, u.name, u.carpool_enabled, u.spare_seats, v.model AS v_model, v.plate AS v_plate, v.seats AS v_seats,
                v.wheelchair_ok, v.low_step, v.child_seat
         FROM users u JOIN vehicles v ON v.user_id = u.id AND v.is_active = 1 WHERE {} ORDER BY u.name",
        perms::sql_holds("u", Perm(Page::Drive, Act::Create))
    ))
    .fetch_all(&st.db)
    .await?)
}

/// Seats and bookings each driver already holds outside this event, partitioned by leg window.
struct OtherLegLoad {
    outbound: HashMap<i64, (i64, i64)>,
    ret: HashMap<i64, (i64, i64)>,
}

async fn query_other_window(
    st: &AppState,
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
    .fetch_all(&st.db)
    .await?;
    Ok(rows.into_iter().map(|(d, s, n)| (d, (s, n))).collect())
}

async fn other_load(
    st: &AppState,
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
    let outbound = query_other_window(st, event, &out_from, &out_to).await?;
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
        (Some(a), Some(b)) => query_other_window(st, event, a, b).await?,
        _ => HashMap::new(),
    };
    Ok(OtherLegLoad { outbound, ret })
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

#[derive(sqlx::FromRow)]
struct DraftRow {
    id: i64,
    event_id: i64,
    version: i64,
    label: String,
    status: String,
    assignments: String,
    visitors: String,
    created_by: Option<i64>,
    created_by_name: Option<String>,
    updated_by: i64,
    updated_by_name: String,
    submitted_by: Option<i64>,
    submitted_by_name: Option<String>,
    submitted_at: Option<String>,
    reviewed_by: Option<i64>,
    reviewed_by_name: Option<String>,
    reviewed_at: Option<String>,
    review_notes: String,
    locked_by: Option<i64>,
    locked_by_name: Option<String>,
    locked_at: Option<String>,
    lock_expires_at: Option<String>,
    created_at: String,
    updated_at: String,
}

const DRAFT_SELECT: &str =
    "SELECT d.id, d.event_id, d.version, d.label, d.status, d.assignments, d.visitors,
        d.created_by, u0.name AS created_by_name,
        d.updated_by, u1.name AS updated_by_name,
        d.submitted_by, u2.name AS submitted_by_name, d.submitted_at,
        d.reviewed_by, u3.name AS reviewed_by_name, d.reviewed_at,
        d.review_notes,
        d.locked_by, u4.name AS locked_by_name, d.locked_at, d.lock_expires_at,
        d.created_at, d.updated_at
 FROM event_plan_drafts d
 JOIN users u1 ON u1.id = d.updated_by
 LEFT JOIN users u0 ON u0.id = d.created_by
 LEFT JOIN users u2 ON u2.id = d.submitted_by
 LEFT JOIN users u3 ON u3.id = d.reviewed_by
 LEFT JOIN users u4 ON u4.id = d.locked_by";

fn is_lock_active(locked_by: Option<i64>, lock_expires_at: Option<&str>, now_str: &str) -> bool {
    locked_by.is_some() && lock_expires_at.is_some_and(|exp| exp > now_str)
}

fn draft_to_json(d: &DraftRow, viewer_id: i64, now_str: &str) -> Value {
    let parsed_assignments: Value =
        serde_json::from_str(&d.assignments).unwrap_or_else(|_| json!([]));
    let parsed_visitors: Value = serde_json::from_str(&d.visitors).unwrap_or_else(|_| json!([]));
    let lock_active = is_lock_active(d.locked_by, d.lock_expires_at.as_deref(), now_str);
    let created_by = d
        .created_by
        .map(|cid| {
            json!({
                "id": cid,
                "name": d.created_by_name.clone().unwrap_or_else(|| d.updated_by_name.clone()),
            })
        })
        .unwrap_or_else(|| json!({ "id": d.updated_by, "name": d.updated_by_name }));
    json!({
        "id": d.id,
        "event_id": d.event_id,
        "version": d.version,
        "label": d.label,
        "status": d.status,
        "assignments": parsed_assignments,
        "visitors": parsed_visitors,
        "created_by": created_by,
        "updated_by": { "id": d.updated_by, "name": d.updated_by_name },
        "created_at": d.created_at,
        "updated_at": d.updated_at,
        "submitted_by": d.submitted_by.map(|sid| json!({ "id": sid, "name": d.submitted_by_name })),
        "submitted_at": d.submitted_at,
        "reviewed_by": d.reviewed_by.map(|rid| json!({ "id": rid, "name": d.reviewed_by_name })),
        "reviewed_at": d.reviewed_at,
        "review_notes": d.review_notes,
        "locked_by": if lock_active {
            d.locked_by.map(|lid| json!({ "id": lid, "name": d.locked_by_name }))
        } else {
            None
        },
        "locked_at": if lock_active { d.locked_at.clone() } else { None },
        "lock_expires_at": if lock_active { d.lock_expires_at.clone() } else { None },
        "lock_is_mine": lock_active && d.locked_by == Some(viewer_id),
    })
}

async fn load_event_drafts(st: &AppState, event_id: i64) -> AppResult<Vec<DraftRow>> {
    Ok(sqlx::query_as(crate::dyn_sql!(
        "{DRAFT_SELECT} WHERE d.event_id = $1 ORDER BY d.version DESC, d.id DESC"
    ))
    .bind(event_id)
    .fetch_all(&st.db)
    .await?)
}

async fn load_draft_by_id(
    st: &AppState,
    event_id: i64,
    draft_id: i64,
) -> AppResult<Option<DraftRow>> {
    Ok(sqlx::query_as(crate::dyn_sql!(
        "{DRAFT_SELECT} WHERE d.event_id = $1 AND d.id = $2"
    ))
    .bind(event_id)
    .bind(draft_id)
    .fetch_optional(&st.db)
    .await?)
}

fn assert_draft_editable(u: &CurrentUser, status: &str) -> AppResult<()> {
    match status {
        "draft" | "returned" => Ok(()),
        "pending_approval" if u.is_traffic_lead() => Ok(()),
        _ => Err(conflict(u.lang, ErrorKey::DraftNotEditable)),
    }
}

fn assert_not_locked_by_other(
    u: &CurrentUser,
    locked_by: Option<i64>,
    locked_by_name: Option<&str>,
    lock_expires_at: Option<&str>,
    now_str: &str,
) -> AppResult<()> {
    if is_lock_active(locked_by, lock_expires_at, now_str) && locked_by != Some(u.id) {
        return Err(locked(
            u.lang,
            ErrorKey::DraftLockedByOther {
                name: locked_by_name.unwrap_or("Someone").to_string(),
            },
        ));
    }
    Ok(())
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
    let draft_rows: Vec<DraftRow> = if event_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(crate::dyn_sql!(
            "{DRAFT_SELECT} WHERE d.event_id = ANY($1) AND d.status <> 'superseded' ORDER BY d.version DESC, d.id DESC"
        ))
        .bind(&event_ids)
        .fetch_all(&st.db)
        .await?
    };

    let mut drafts_by_event: HashMap<i64, Vec<DraftRow>> = HashMap::new();
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
                "latest_draft": latest.map(|d| draft_to_json(d, u.id, &now_str)),
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
    let draft_rows = load_event_drafts(&st, id).await?;
    let drafts_json: Vec<Value> = draft_rows
        .iter()
        .map(|d| draft_to_json(d, u.id, &now_str))
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
        .map(|d| draft_to_json(d, u.id, &now_str));

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

#[derive(Deserialize, Serialize, Clone)]
struct DraftEntry {
    ride_id: i64,
    driver_id: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct DraftVisitorEntry {
    #[serde(default)]
    pub temp_id: String,
    #[serde(default = "default_visitor_op")]
    pub op: String, // "add" | "update" | "delete"
    pub live_visitor_id: Option<i64>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub notes: String,
    pub area_id: Option<i64>,
    #[serde(default)]
    pub area_name: Option<String>,
    #[serde(default)]
    pub pickup_address: String,
    pub seats: Option<i64>,
    pub leg: Option<String>,
    pub driver_id: Option<i64>,
    #[serde(default)]
    pub driver_name: Option<String>,
}

fn default_visitor_op() -> String {
    "add".to_string()
}

fn clean_label(u: &CurrentUser, raw: Option<&str>) -> AppResult<String> {
    let s = raw.unwrap_or_default().trim();
    if s.chars().count() > LABEL_MAX {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    Ok(s.to_string())
}

async fn clean_draft_visitors(
    st: &AppState,
    u: &CurrentUser,
    event_id: i64,
    entries: &[DraftVisitorEntry],
) -> AppResult<Vec<DraftVisitorEntry>> {
    if entries.len() > MAX_DRAFT_VISITORS {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let mut out = Vec::with_capacity(entries.len());
    for (idx, v) in entries.iter().enumerate() {
        let op = v.op.trim().to_string();
        if !matches!(op.as_str(), "add" | "update" | "delete") {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        let temp_id = if v.temp_id.trim().is_empty() {
            format!("v-{}", idx + 1)
        } else {
            v.temp_id.trim().to_string()
        };
        if op == "update" || op == "delete" {
            let Some(vid) = v.live_visitor_id else {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            };
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM plan_visitors WHERE id = $1 AND event_id = $2)",
            )
            .bind(vid)
            .bind(event_id)
            .fetch_one(&st.db)
            .await?;
            if !exists {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
        }
        if op == "delete" {
            out.push(DraftVisitorEntry {
                temp_id,
                op,
                live_visitor_id: v.live_visitor_id,
                name: v.name.trim().to_string(),
                phone: String::new(),
                notes: String::new(),
                area_id: None,
                area_name: None,
                pickup_address: String::new(),
                seats: v.seats,
                leg: v.leg.clone(),
                driver_id: None,
                driver_name: None,
            });
            continue;
        }
        let cleaned = clean_visitor(
            st,
            u,
            VisitorIn {
                name: v.name.clone(),
                phone: v.phone.clone(),
                notes: v.notes.clone(),
                area_id: v.area_id,
                pickup_address: v.pickup_address.clone(),
                seats: v.seats,
                leg: v.leg.clone(),
                driver_id: v.driver_id,
            },
        )
        .await?;
        let area_name: Option<String> = match cleaned.area_id {
            Some(aid) => {
                sqlx::query_scalar("SELECT name FROM areas WHERE id = $1")
                    .bind(aid)
                    .fetch_optional(&st.db)
                    .await?
            }
            None => None,
        };
        let driver_name: Option<String> = match cleaned.driver_id {
            Some(did) => {
                sqlx::query_scalar("SELECT name FROM users WHERE id = $1")
                    .bind(did)
                    .fetch_optional(&st.db)
                    .await?
            }
            None => None,
        };
        out.push(DraftVisitorEntry {
            temp_id,
            op,
            live_visitor_id: v.live_visitor_id,
            name: cleaned.name,
            phone: cleaned.phone,
            notes: cleaned.notes,
            area_id: cleaned.area_id,
            area_name,
            pickup_address: cleaned.pickup_address,
            seats: Some(cleaned.seats),
            leg: Some(cleaned.leg),
            driver_id: cleaned.driver_id,
            driver_name,
        });
    }
    Ok(out)
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

#[derive(Deserialize)]
struct CreateDraftReq {
    label: Option<String>,
    clone_from_draft_id: Option<i64>,
    assignments: Option<Vec<DraftEntry>>,
    visitors: Option<Vec<DraftVisitorEntry>>,
}

async fn create_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    body: Option<Json<CreateDraftReq>>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let req = body.map(|b| b.0).unwrap_or(CreateDraftReq {
        label: None,
        clone_from_draft_id: None,
        assignments: None,
        visitors: None,
    });
    let label = clean_label(&u, req.label.as_deref())?;
    let cleaned_visitors = match req.visitors.as_ref() {
        Some(v) => Some(clean_draft_visitors(&st, &u, id, v).await?),
        None => None,
    };

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let (max_ver,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(version), 0)::BIGINT FROM event_plan_drafts WHERE event_id = $1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let next_version = max_ver + 1;

    let (cloned_assignments, cloned_visitors): (Option<String>, Option<String>) = if let Some(
        src_id,
    ) =
        req.clone_from_draft_id
    {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT assignments, visitors FROM event_plan_drafts WHERE id = $1 AND event_id = $2",
        )
        .bind(src_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let (a, v) = row.ok_or_else(|| bad(u.lang, ErrorKey::PlanDraftNotFound))?;
        (Some(a), Some(v))
    } else {
        (None, None)
    };

    let assignments_json = if let Some(a) = &req.assignments {
        serde_json::to_string(a).unwrap_or_else(|_| "[]".to_string())
    } else if let Some(ca) = cloned_assignments {
        ca
    } else {
        let live: Vec<DraftEntry> = plan_rides(&st, id)
            .await?
            .into_iter()
            .map(|r| DraftEntry {
                ride_id: r.id,
                driver_id: r.driver_id,
            })
            .collect();
        serde_json::to_string(&live).unwrap_or_else(|_| "[]".to_string())
    };

    let visitors_json = if let Some(v) = &cleaned_visitors {
        serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
    } else if let Some(cv) = cloned_visitors {
        cv
    } else {
        "[]".to_string()
    };

    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);

    let (draft_id,): (i64,) = sqlx::query_as(
        "INSERT INTO event_plan_drafts (
             event_id, version, label, status, assignments, visitors,
             created_by, updated_by, locked_by, locked_at, lock_expires_at,
             created_at, updated_at
         )
         VALUES ($1, $2, $3, 'draft', $4, $5, $6, $6, $6, $7, $8, $7, $7)
         RETURNING id",
    )
    .bind(id)
    .bind(next_version)
    .bind(&label)
    .bind(&assignments_json)
    .bind(&visitors_json)
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.create_draft",
        "event",
        Some(id),
        format!("event {id}: created draft v{next_version} (#{draft_id})"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, draft_id)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
struct UpdateDraftReq {
    label: Option<String>,
    assignments: Option<Vec<DraftEntry>>,
    visitors: Option<Vec<DraftVisitorEntry>>,
}

async fn update_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
    Json(req): Json<UpdateDraftReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let label_opt = match req.label.as_deref() {
        Some(raw) => Some(clean_label(&u, Some(raw))?),
        None => None,
    };
    let visitors_json = match req.visitors.as_ref() {
        Some(v) => Some(
            serde_json::to_string(&clean_draft_visitors(&st, &u, id, v).await?)
                .unwrap_or_else(|_| "[]".to_string()),
        ),
        None => None,
    };
    let assignments_json = req
        .assignments
        .as_ref()
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| "[]".to_string()));

    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);
    let mut tx = st.db.begin().await?;

    let row: Option<(
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT d.status, d.label, d.assignments, d.visitors,
                    d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((
        cur_status,
        cur_label,
        cur_assignments,
        cur_visitors,
        locked_by,
        locked_by_name,
        lock_expires_at,
    )) = row
    else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    assert_draft_editable(&u, &cur_status)?;
    assert_not_locked_by_other(
        &u,
        locked_by,
        locked_by_name.as_deref(),
        lock_expires_at.as_deref(),
        &stamp,
    )?;

    let next_label = label_opt.unwrap_or(cur_label);
    let next_assignments = assignments_json.unwrap_or(cur_assignments);
    let next_visitors = visitors_json.unwrap_or(cur_visitors);

    sqlx::query(
        "UPDATE event_plan_drafts
         SET label = $1, assignments = $2, visitors = $3,
             updated_by = $4, updated_at = $5,
             locked_by = $4, locked_at = $5, lock_expires_at = $6
         WHERE id = $7 AND event_id = $8",
    )
    .bind(&next_label)
    .bind(&next_assignments)
    .bind(&next_visitors)
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.save_draft",
        "event",
        Some(id),
        format!("event {id}: updated draft #{did}"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

async fn delete_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let mut tx = st.db.begin().await?;

    let row: Option<(
        String,
        Option<i64>,
        i64,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT d.status, d.created_by, d.updated_by, d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, created_by, updated_by, locked_by, locked_by_name, lock_expires_at)) = row
    else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    if status == "approved" {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }
    let is_owner = created_by.unwrap_or(updated_by) == u.id;
    if !is_owner && !u.is_traffic_lead() {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    if !u.is_traffic_lead() {
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
    }

    sqlx::query("DELETE FROM event_plan_drafts WHERE id = $1 AND event_id = $2")
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.delete_draft",
        "event",
        Some(id),
        format!("event {id}: deleted draft #{did}"),
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct LockReq {
    force: Option<bool>,
}

async fn acquire_draft_lock(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
    body: Option<Json<LockReq>>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let force = body.and_then(|b| b.0.force).unwrap_or(false);
    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);
    let mut tx = st.db.begin().await?;

    let row: Option<(String, Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.status, d.locked_by, u.name, d.lock_expires_at
         FROM event_plan_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.event_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    assert_draft_editable(&u, &status)?;

    if !(force && u.is_traffic_lead()) {
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
    }

    sqlx::query(
        "UPDATE event_plan_drafts
         SET locked_by = $1, locked_at = $2, lock_expires_at = $3
         WHERE id = $4 AND event_id = $5",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

async fn release_draft_lock(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let mut tx = st.db.begin().await?;

    let row: Option<(Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.locked_by, u.name, d.lock_expires_at
         FROM event_plan_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.event_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    if !u.is_traffic_lead() {
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
    }

    sqlx::query(
        "UPDATE event_plan_drafts
         SET locked_by = NULL, locked_at = NULL, lock_expires_at = NULL
         WHERE id = $1 AND event_id = $2",
    )
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
struct SaveDraftReq {
    draft_id: Option<i64>,
    label: Option<String>,
    assignments: Vec<DraftEntry>,
    visitors: Option<Vec<DraftVisitorEntry>>,
}

async fn save_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SaveDraftReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let label_opt = match req.label.as_deref() {
        Some(raw) => Some(clean_label(&u, Some(raw))?),
        None => None,
    };
    let visitors_opt = match req.visitors.as_ref() {
        Some(v) => Some(
            serde_json::to_string(&clean_draft_visitors(&st, &u, id, v).await?)
                .unwrap_or_else(|_| "[]".to_string()),
        ),
        None => None,
    };
    let assignments_json =
        serde_json::to_string(&req.assignments).unwrap_or_else(|_| "[]".to_string());
    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let existing: Option<(
        i64,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = if let Some(did) = req.draft_id {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.visitors, d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
        )
        .bind(did)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::PlanDraftNotFound))
        .map(Some)?
    } else {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.visitors, d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.event_id = $1 AND d.status IN ('draft', 'returned')
             ORDER BY d.version DESC, d.id DESC
             LIMIT 1
             FOR UPDATE OF d",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    };

    let draft_id = if let Some((
        did,
        cur_status,
        cur_label,
        cur_visitors,
        locked_by,
        locked_by_name,
        lock_expires_at,
    )) = existing
    {
        assert_draft_editable(&u, &cur_status)?;
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
        let next_status = if cur_status == "pending_approval" {
            "pending_approval"
        } else {
            "draft"
        };
        let next_label = label_opt.unwrap_or(cur_label);
        let next_visitors = visitors_opt.unwrap_or(cur_visitors);
        sqlx::query(
            "UPDATE event_plan_drafts
             SET status = $1, label = $2, assignments = $3, visitors = $4,
                 updated_by = $5, updated_at = $6,
                 locked_by = $5, locked_at = $6, lock_expires_at = $7
             WHERE id = $8 AND event_id = $9",
        )
        .bind(next_status)
        .bind(&next_label)
        .bind(&assignments_json)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .bind(&expires)
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        did
    } else {
        let (max_ver,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(version), 0)::BIGINT FROM event_plan_drafts WHERE event_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let next_version = max_ver + 1;
        let next_label = label_opt.unwrap_or_default();
        let next_visitors = visitors_opt.unwrap_or_else(|| "[]".to_string());
        let (did,): (i64,) = sqlx::query_as(
            "INSERT INTO event_plan_drafts (
                 event_id, version, label, status, assignments, visitors,
                 created_by, updated_by, locked_by, locked_at, lock_expires_at,
                 created_at, updated_at
             )
             VALUES ($1, $2, $3, 'draft', $4, $5, $6, $6, $6, $7, $8, $7, $7)
             RETURNING id",
        )
        .bind(id)
        .bind(next_version)
        .bind(&next_label)
        .bind(&assignments_json)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .bind(&expires)
        .fetch_one(&mut *tx)
        .await?;
        did
    };

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.save_draft",
        "event",
        Some(id),
        format!(
            "event {id}: saved draft #{draft_id} with {} assignments",
            req.assignments.len()
        ),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, draft_id)
        .await?
        .ok_or_else(|| not_found(u.lang))?;

    Ok(Json(json!({
        "ok": true,
        "status": draft_row.status,
        "assignments": req.assignments.len(),
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
struct RequestApprovalReq {
    draft_id: Option<i64>,
    assignments: Option<Vec<DraftEntry>>,
    visitors: Option<Vec<DraftVisitorEntry>>,
    label: Option<String>,
}

async fn request_approval(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<RequestApprovalReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let label_opt = match req.label.as_deref() {
        Some(raw) => Some(clean_label(&u, Some(raw))?),
        None => None,
    };
    let visitors_opt = match req.visitors.as_ref() {
        Some(v) => Some(
            serde_json::to_string(&clean_draft_visitors(&st, &u, id, v).await?)
                .unwrap_or_else(|_| "[]".to_string()),
        ),
        None => None,
    };
    let assignments_opt = req
        .assignments
        .as_ref()
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| "[]".to_string()));

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let existing: Option<(
        i64,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = if let Some(did) = req.draft_id {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.assignments, d.visitors,
                    d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
        )
        .bind(did)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::PlanDraftNotFound))
        .map(Some)?
    } else {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.assignments, d.visitors,
                    d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.event_id = $1 AND d.status IN ('draft', 'returned')
             ORDER BY d.version DESC, d.id DESC
             LIMIT 1
             FOR UPDATE OF d",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    };

    let draft_id = if let Some((
        did,
        cur_status,
        cur_label,
        cur_assignments,
        cur_visitors,
        locked_by,
        locked_by_name,
        lock_expires_at,
    )) = existing
    {
        if !matches!(cur_status.as_str(), "draft" | "returned") {
            return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
        }
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
        let next_label = label_opt.unwrap_or(cur_label);
        let next_assignments = assignments_opt.unwrap_or(cur_assignments);
        let next_visitors = visitors_opt.unwrap_or(cur_visitors);
        sqlx::query(
            "UPDATE event_plan_drafts
             SET status = 'pending_approval',
                 label = $1,
                 assignments = $2,
                 visitors = $3,
                 submitted_by = $4,
                 submitted_at = $5,
                 updated_by = $4,
                 updated_at = $5,
                 locked_by = NULL,
                 locked_at = NULL,
                 lock_expires_at = NULL
             WHERE id = $6 AND event_id = $7",
        )
        .bind(&next_label)
        .bind(&next_assignments)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        did
    } else if let Some(json_str) = assignments_opt {
        let (max_ver,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(version), 0)::BIGINT FROM event_plan_drafts WHERE event_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let next_version = max_ver + 1;
        let next_label = label_opt.unwrap_or_default();
        let next_visitors = visitors_opt.unwrap_or_else(|| "[]".to_string());
        let (did,): (i64,) = sqlx::query_as(
            "INSERT INTO event_plan_drafts (
                 event_id, version, label, status, assignments, visitors,
                 created_by, updated_by, submitted_by, submitted_at,
                 created_at, updated_at
             )
             VALUES ($1, $2, $3, 'pending_approval', $4, $5, $6, $6, $6, $7, $7, $7)
             RETURNING id",
        )
        .bind(id)
        .bind(next_version)
        .bind(&next_label)
        .bind(&json_str)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .fetch_one(&mut *tx)
        .await?;
        did
    } else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    tx.commit().await?;

    // Notify traffic lead(s) using exact role-token matching.
    let mut lead_rows: Vec<(i64,)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT id FROM users WHERE {}",
        perms::sql_has_role("users", perms::TRAFFIC_HEAD_ROLE)
    ))
    .fetch_all(&st.db)
    .await?;

    if lead_rows.is_empty() {
        lead_rows = sqlx::query_as(crate::dyn_sql!(
            "SELECT id FROM users WHERE {}",
            perms::sql_has_any_role("users", &[perms::ADMIN_ROLE, perms::SUPERADMIN_ROLE])
        ))
        .fetch_all(&st.db)
        .await?;
    }

    let lead_ids: Vec<i64> = lead_rows
        .into_iter()
        .map(|(lid,)| lid)
        .filter(|&lid| lid != u.id)
        .collect();

    crate::util::notify_many(
        &st.db,
        &lead_ids,
        ("接送安排待审批", "Transport arrangement awaiting approval"),
        (
            &format!(
                "{} 提交了「{}」的接送安排草稿，等待您审批。",
                u.name, ev.title
            ),
            &format!(
                "{} submitted a transport arrangement draft for \"{}\" for approval.",
                u.name, ev.title
            ),
        ),
        &format!("/admin/events/{}/plan?draft={}", id, draft_id),
    )
    .await;

    audit(
        &st.db,
        Some(u.id),
        "plan.request_approval",
        "event",
        Some(id),
        format!("event {id}: submitted draft #{draft_id} for traffic lead approval"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, draft_id)
        .await?
        .ok_or_else(|| not_found(u.lang))?;

    Ok(Json(json!({
        "ok": true,
        "status": "pending_approval",
        "draft_id": draft_id,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
struct ReturnDraftReq {
    draft_id: Option<i64>,
    notes: Option<String>,
}

async fn return_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReturnDraftReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    if !u.is_traffic_lead() {
        return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
    }
    let ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let notes = req.notes.unwrap_or_default().trim().to_string();

    let mut tx = st.db.begin().await?;
    let target: Option<(i64, String, Option<i64>, Option<i64>)> = if let Some(did) = req.draft_id {
        sqlx::query_as(
            "SELECT id, status, submitted_by, created_by
             FROM event_plan_drafts
             WHERE id = $1 AND event_id = $2
             FOR UPDATE",
        )
        .bind(did)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, status, submitted_by, created_by
             FROM event_plan_drafts
             WHERE event_id = $1 AND status = 'pending_approval'
             ORDER BY version DESC, id DESC
             LIMIT 1
             FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    };

    let Some((did, cur_status, submitted_by, created_by)) = target else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };
    if !matches!(
        cur_status.as_str(),
        "pending_approval" | "draft" | "returned"
    ) {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }

    sqlx::query(
        "UPDATE event_plan_drafts
         SET status = 'returned',
             reviewed_by = $1,
             reviewed_at = $2,
             review_notes = $3,
             updated_at = $2,
             locked_by = NULL,
             locked_at = NULL,
             lock_expires_at = NULL
         WHERE id = $4 AND event_id = $5",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(&notes)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let mut notify_targets = Vec::new();
    if let Some(sub) = submitted_by {
        if sub != u.id {
            notify_targets.push(sub);
        }
    }
    if let Some(creator) = created_by {
        if creator != u.id && !notify_targets.contains(&creator) {
            notify_targets.push(creator);
        }
    }
    if !notify_targets.is_empty() {
        let reason_zh = if notes.is_empty() {
            String::new()
        } else {
            format!("：{}", notes)
        };
        let reason_en = if notes.is_empty() {
            String::new()
        } else {
            format!(": {}", notes)
        };
        crate::util::notify_many(
            &st.db,
            &notify_targets,
            ("接送安排草稿已退回", "Transport arrangement draft returned"),
            (
                &format!(
                    "交通组组长退回了「{}」的接送安排草稿{}",
                    ev.title, reason_zh
                ),
                &format!(
                    "Traffic lead returned the transport draft for \"{}\"{}",
                    ev.title, reason_en
                ),
            ),
            &format!("/admin/events/{}/plan?draft={}", id, did),
        )
        .await;
    }

    audit(
        &st.db,
        Some(u.id),
        "plan.return_draft",
        "event",
        Some(id),
        format!("event {id}: returned draft #{did} with notes: {notes}"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;

    Ok(Json(json!({
        "ok": true,
        "status": "returned",
        "draft": draft_to_json(&draft_row, u.id, &stamp),
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
    draft_id: Option<i64>,
}

async fn save_plan(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SavePlan>,
) -> AppResult<Json<Value>> {
    apply_plan_impl(&st, &u, id, req, false).await
}

async fn approve_plan(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SavePlan>,
) -> AppResult<Json<Value>> {
    apply_plan_impl(&st, &u, id, req, true).await
}

async fn apply_plan_impl(
    st: &AppState,
    u: &CurrentUser,
    id: i64,
    req: SavePlan,
    is_approve_route: bool,
) -> AppResult<Json<Value>> {
    require(u, Page::AdminPlan, Act::Update)?;
    let ev = load_event(st, u, id).await?;

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let target_draft: Option<(i64, String, String, Option<i64>, Option<i64>, i64)> =
        if let Some(did) = req.draft_id {
            let row: Option<(i64, String, String, Option<i64>, Option<i64>, i64)> = sqlx::query_as(
                "SELECT id, status, visitors, submitted_by, created_by, updated_by
                 FROM event_plan_drafts
                 WHERE id = $1 AND event_id = $2
                 FOR UPDATE",
            )
            .bind(did)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(r) = row else {
                return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
            };
            if !matches!(r.1.as_str(), "draft" | "pending_approval" | "returned") {
                return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
            }
            Some(r)
        } else {
            sqlx::query_as(
                "SELECT id, status, visitors, submitted_by, created_by, updated_by
                 FROM event_plan_drafts
                 WHERE event_id = $1 AND status = 'pending_approval'
                 ORDER BY version DESC, id DESC
                 LIMIT 1
                 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
        };

    // Permission check:
    // - Approving a draft (or calling /plan/approve) always requires traffic lead authority.
    // - Direct live edits without a draft require traffic lead, legacy `transport` role, or
    //   `cell_leader` on an event they created. Non-lead `traffic_member` and other roles
    //   must save drafts and submit for traffic lead approval.
    let can_direct_apply = u.is_traffic_lead()
        || u.roles.iter().any(|r| r == perms::TRANSPORT_ROLE)
        || (u.roles.iter().any(|r| r == perms::CELL_LEADER_ROLE) && ev.created_by == u.id);
    if is_approve_route || target_draft.is_some() {
        if !u.is_traffic_lead() {
            return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
        }
    } else if !can_direct_apply {
        return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
    }

    let rides = plan_rides(st, id).await?;
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

    if changes.is_empty() && target_draft.is_none() {
        return Ok(Json(json!({ "ok": true, "changed": 0 })));
    }

    // Validate every new driver and the seats each driver ends up with.
    let drivers = plan_drivers(st).await?;
    let scopes = crate::audience::drive_scopes(&st.db).await?;
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
    let touched: HashSet<i64> = changes.values().filter_map(|c| c.driver_id).collect();
    let mut lock_order: Vec<i64> = touched.iter().copied().collect();
    lock_order.sort_unstable();
    for d in &lock_order {
        sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
            .bind(d)
            .fetch_optional(&mut *tx)
            .await?;
    }
    if !touched.is_empty() {
        let other = other_load(st, id, &rides, &ev.starts_at).await?;
        for d in &touched {
            let dr = driver[d];
            let cap = seat_capacity(dr.spare_seats, dr.v_seats);
            for (is_ret, other_map) in [(false, &other.outbound), (true, &other.ret)] {
                let assigned_on_leg = changes.values().any(|c| {
                    c.driver_id == Some(*d) && (by_id[&c.ride_id].leg == "return") == is_ret
                });
                if !assigned_on_leg {
                    continue;
                }
                let mine: Vec<&PlanRide> = rides
                    .iter()
                    .filter(|r| (r.leg == "return") == is_ret && final_driver(r) == Some(*d))
                    .collect();
                let (o_seats, o_rides) = other_map.get(d).copied().unwrap_or((0, 0));
                let load = o_seats + mine.iter().map(|r| r.seats).sum::<i64>();
                // Without 共乘模式 a driver takes one booking per leg/time slot.
                let one_booking = dr.carpool_enabled == 1 || o_rides + mine.len() as i64 <= 1;
                if load > cap || !one_booking {
                    return Err(bad(
                        u.lang,
                        ErrorKey::NotEnoughDriverSeatsReassign { load, cap },
                    ));
                }
            }
        }
    }

    // Apply all or nothing; each update re-checks the state the board saw.
    let stamp = now();
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

    // Commit draft visitors and sync the approved draft snapshot if approving a draft.
    let mut visitor_audits: Vec<(&'static str, String)> = Vec::new();
    let approved_draft_info: Option<(i64, Option<i64>, Option<i64>)> = if let Some((
        did,
        _status,
        visitors_raw,
        submitted_by,
        created_by,
        updated_by,
    )) = target_draft
    {
        let draft_visitors: Vec<DraftVisitorEntry> =
            serde_json::from_str(&visitors_raw).unwrap_or_default();
        let visitor_author = submitted_by.or(created_by).unwrap_or(updated_by);

        for dv in &draft_visitors {
            match dv.op.as_str() {
                "add" => {
                    if let Some(d) = dv.driver_id {
                        if !driver.contains_key(&d) {
                            return Err(bad(u.lang, ErrorKey::DriverNotFound));
                        }
                    }
                    let vid: i64 = sqlx::query_scalar(
                        "INSERT INTO plan_visitors (
                                 event_id, name, phone, notes, area_id, pickup_address,
                                 seats, leg, driver_id, created_by, created_at, updated_at
                             )
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
                             RETURNING id",
                    )
                    .bind(id)
                    .bind(dv.name.trim())
                    .bind(dv.phone.trim())
                    .bind(dv.notes.trim())
                    .bind(dv.area_id)
                    .bind(dv.pickup_address.trim())
                    .bind(dv.seats.unwrap_or(1))
                    .bind(dv.leg.as_deref().unwrap_or("outbound"))
                    .bind(dv.driver_id)
                    .bind(visitor_author)
                    .bind(&stamp)
                    .fetch_one(&mut *tx)
                    .await?;
                    visitor_audits.push((
                        "plan.visitor_add",
                        format!(
                            "event {id}: visitor #{vid} ({} seat(s)) via draft #{did}",
                            dv.seats.unwrap_or(1)
                        ),
                    ));
                }
                "update" => {
                    let Some(vid) = dv.live_visitor_id else {
                        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
                    };
                    if let Some(d) = dv.driver_id {
                        if !driver.contains_key(&d) {
                            return Err(bad(u.lang, ErrorKey::DriverNotFound));
                        }
                    }
                    let res = sqlx::query(
                        "UPDATE plan_visitors
                             SET name=$3, phone=$4, notes=$5, area_id=$6, pickup_address=$7,
                                 seats=$8, leg=$9, driver_id=$10, updated_at=$11
                             WHERE id=$1 AND event_id=$2",
                    )
                    .bind(vid)
                    .bind(id)
                    .bind(dv.name.trim())
                    .bind(dv.phone.trim())
                    .bind(dv.notes.trim())
                    .bind(dv.area_id)
                    .bind(dv.pickup_address.trim())
                    .bind(dv.seats.unwrap_or(1))
                    .bind(dv.leg.as_deref().unwrap_or("outbound"))
                    .bind(dv.driver_id)
                    .bind(&stamp)
                    .execute(&mut *tx)
                    .await?;
                    if res.rows_affected() == 0 {
                        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
                    }
                    visitor_audits.push((
                        "plan.visitor_update",
                        format!("event {id}: visitor #{vid} via draft #{did}"),
                    ));
                }
                "delete" => {
                    let Some(vid) = dv.live_visitor_id else {
                        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
                    };
                    sqlx::query("DELETE FROM plan_visitors WHERE id = $1 AND event_id = $2")
                        .bind(vid)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                    visitor_audits.push((
                        "plan.visitor_delete",
                        format!("event {id}: visitor #{vid} via draft #{did}"),
                    ));
                }
                _ => {}
            }
        }

        // Store the exact approved live snapshot on the approved draft so reloading never replays stale changes.
        let final_snapshot: Vec<DraftEntry> = rides
            .iter()
            .map(|r| DraftEntry {
                ride_id: r.id,
                driver_id: final_driver(r),
            })
            .collect();
        let final_snapshot_json =
            serde_json::to_string(&final_snapshot).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "UPDATE event_plan_drafts
                 SET status = 'approved',
                     assignments = $1,
                     reviewed_by = $2,
                     reviewed_at = $3,
                     updated_by = $2,
                     updated_at = $3,
                     locked_by = NULL,
                     locked_at = NULL,
                     lock_expires_at = NULL
                 WHERE id = $4 AND event_id = $5",
        )
        .bind(&final_snapshot_json)
        .bind(u.id)
        .bind(&stamp)
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Supersede any other open drafts on this event.
        sqlx::query(
                "UPDATE event_plan_drafts
                 SET status = 'superseded',
                     updated_at = $1,
                     locked_by = NULL,
                     locked_at = NULL,
                     lock_expires_at = NULL
                 WHERE event_id = $2 AND id <> $3 AND status IN ('draft', 'pending_approval', 'returned')",
            )
            .bind(&stamp)
            .bind(id)
            .bind(did)
            .execute(&mut *tx)
            .await?;

        Some((did, submitted_by, created_by))
    } else {
        None
    };

    tx.commit().await?;

    for (action, detail) in visitor_audits {
        audit(&st.db, Some(u.id), action, "event", Some(id), detail).await;
    }

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
            st.routing.invalidate(old, r.trip_id.unwrap_or(r.id)).await;
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
        // Staff may go outside the rider's group; that is recorded, as on a reassign.
        if let (Some(d), Some(g)) = (c.driver_id, r.match_group_id) {
            if !scopes.get(&d).is_some_and(|s| s.contains(&g)) {
                audit(
                    &st.db,
                    Some(u.id),
                    "ride.assign_outside_group",
                    "ride",
                    Some(r.id),
                    format!("driver {d} is outside group {g}"),
                )
                .await;
            }
        }
    }
    // A driver from outside the event's groups learns only that it is a church event.
    let title_for = |sees: bool| -> (String, String) {
        if sees {
            (ev.title.clone(), ev.title.clone())
        } else {
            ("教会活动".to_string(), "Church event".to_string())
        }
    };
    for (d, n) in &gained {
        let (zh, en) = title_for(crate::audience::member_sees_event(&st.db, *d, id).await?);
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
                &format!("{} · {} · 请在「开车」确认", zh, when),
                &format!("{} · {} · please confirm in Driving", en, when),
            ),
            "/drive",
        )
        .await;
    }
    for (d, names) in &lost {
        let (zh, en) = title_for(crate::audience::member_sees_event(&st.db, *d, id).await?);
        notify(
            &st.db,
            *d,
            ("接送已改派", "Ride reassigned"),
            (
                &format!("{} · {}", zh, names.join(", ")),
                &format!("{} · {}", en, names.join(", ")),
            ),
            "/drive",
        )
        .await;
    }

    if let Some((did, submitted_by, created_by)) = approved_draft_info {
        audit(
            &st.db,
            Some(u.id),
            "plan.approve_draft",
            "event",
            Some(id),
            format!(
                "event {id}: approved draft #{did} ({} changes)",
                changes.len()
            ),
        )
        .await;
        let mut notify_targets = Vec::new();
        if let Some(sub) = submitted_by {
            if sub != u.id {
                notify_targets.push(sub);
            }
        }
        if let Some(creator) = created_by {
            if creator != u.id && !notify_targets.contains(&creator) {
                notify_targets.push(creator);
            }
        }
        if !notify_targets.is_empty() {
            crate::util::notify_many(
                &st.db,
                &notify_targets,
                ("接送安排已审批通过", "Transport arrangement approved"),
                (
                    &format!(
                        "{} 审批通过了「{}」的接送安排，已通知司机确认。",
                        u.name, ev.title
                    ),
                    &format!(
                        "{} approved the transport arrangement for \"{}\", drivers notified.",
                        u.name, ev.title
                    ),
                ),
                &format!("/admin/events/{}/plan?draft={}", id, did),
            )
            .await;
        }
    }

    Ok(Json(json!({
        "ok": true,
        "changed": changes.len(),
        "approved_draft_id": approved_draft_info.map(|(did, _, _)| did),
    })))
}
