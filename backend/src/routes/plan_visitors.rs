//! Visitors on an event's transport plan, for the record.
//!
//! The traffic lead (Head of Traffic, i.e. the traffic group's 组长, plus admin, President,
//! Vice-President and super admin: `perms::can_approve_transport_plan`) notes down people
//! without an account who travel to the event:
//! name, optional phone, area, pickup address, seats and, if known, the car they ride in.
//! Nothing is sent to anyone: no invite, no sign-in, no notification to the visitor or the
//! driver. The board counts their seats in that car so the plan stays honest.
//!
//! Only a traffic lead may add, change or remove visitors; anyone who can see the plan
//! sees them. Traffic group members, transport, secretary and cell leaders cannot record
//! visitors even though they may edit the plan.

use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{self, Act, Page, Perm};
use crate::routes::plan::load_event;
use crate::state::AppState;
use crate::util::{audit, check_phone, now, require};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/events/{id}/plan/visitors",
            get(list_visitors).post(create_visitor),
        )
        .route(
            "/admin/events/{id}/plan/visitors/{vid}",
            put(update_visitor).delete(delete_visitor),
        )
}

const NAME_MAX: usize = 80;
const TEXT_MAX: usize = 500;

#[derive(sqlx::FromRow)]
struct VisitorRow {
    id: i64,
    name: String,
    phone: String,
    notes: String,
    area_id: Option<i64>,
    area_name: Option<String>,
    pickup_address: String,
    seats: i64,
    leg: String,
    driver_id: Option<i64>,
    driver_name: Option<String>,
    created_by: Option<i64>,
    created_by_name: Option<String>,
    created_at: String,
    updated_at: String,
}

/// Recording visitors is for the traffic lead only (any visitor, whoever added it).
fn may_edit(u: &CurrentUser) -> bool {
    u.is_traffic_lead()
}

/// The event's recorded visitors, as the plan board shows them.
pub(crate) async fn list(st: &AppState, u: &CurrentUser, event: i64) -> AppResult<Vec<Value>> {
    let rows: Vec<VisitorRow> = sqlx::query_as(
        "SELECT v.id, v.name, v.phone, v.notes, v.area_id, a.name AS area_name, v.pickup_address,
                v.seats, v.leg, v.driver_id, d.name AS driver_name, v.created_by,
                c.name AS created_by_name, v.created_at, v.updated_at
         FROM plan_visitors v
         LEFT JOIN areas a ON a.id = v.area_id
         LEFT JOIN users d ON d.id = v.driver_id
         LEFT JOIN users c ON c.id = v.created_by
         WHERE v.event_id = $1
         ORDER BY v.leg, v.name, v.id",
    )
    .bind(event)
    .fetch_all(&st.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|v| {
            json!({
                "id": v.id,
                "name": v.name,
                "phone": v.phone,
                "notes": v.notes,
                "area": v.area_id.map(|id| json!({"id": id, "name": v.area_name})),
                "pickup_address": v.pickup_address,
                "seats": v.seats,
                "leg": v.leg,
                "driver_id": v.driver_id,
                "driver_name": v.driver_name,
                "created_by": v.created_by.map(|id| json!({"id": id, "name": v.created_by_name})),
                "created_at": v.created_at,
                "updated_at": v.updated_at,
                "can_edit": may_edit(u),
            })
        })
        .collect())
}

async fn list_visitors(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::View)?;
    load_event(&st, &u, id).await?;
    Ok(Json(json!({ "visitors": list(&st, &u, id).await? })))
}

#[derive(Deserialize)]
struct VisitorIn {
    name: String,
    #[serde(default)]
    phone: String,
    #[serde(default)]
    notes: String,
    area_id: Option<i64>,
    #[serde(default)]
    pickup_address: String,
    seats: Option<i64>,
    leg: Option<String>,
    driver_id: Option<i64>,
}

/// A checked visitor, ready to store.
struct Clean {
    name: String,
    phone: String,
    notes: String,
    area_id: Option<i64>,
    pickup_address: String,
    seats: i64,
    leg: String,
    driver_id: Option<i64>,
}

async fn clean(st: &AppState, u: &CurrentUser, req: VisitorIn) -> AppResult<Clean> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(bad(u.lang, ErrorKey::VisitorNameRequired));
    }
    if name.chars().count() > NAME_MAX {
        return Err(bad(u.lang, ErrorKey::NameTooLong));
    }
    let phone = req.phone.trim().to_string();
    check_phone(u.lang, &phone)?;
    let notes = req.notes.trim().to_string();
    let pickup_address = req.pickup_address.trim().to_string();
    if notes.chars().count() > TEXT_MAX || pickup_address.chars().count() > TEXT_MAX {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let seats = req.seats.unwrap_or(1);
    if !(1..=8).contains(&seats) {
        return Err(bad(u.lang, ErrorKey::SeatsRange));
    }
    let leg = req.leg.unwrap_or_else(|| "outbound".into());
    if leg != "outbound" && leg != "return" {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if let Some(a) = req.area_id {
        let found: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM areas WHERE id = $1)")
            .bind(a)
            .fetch_one(&st.db)
            .await?;
        if !found {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if let Some(d) = req.driver_id {
        // The same people the board offers as drivers: may drive, with a vehicle on file.
        let found: bool = sqlx::query_scalar(crate::dyn_sql!(
            "SELECT EXISTS (SELECT 1 FROM users u JOIN vehicles v ON v.user_id = u.id AND v.is_active = 1
                            WHERE u.id = $1 AND {})",
            perms::sql_holds("u", Perm(Page::Drive, Act::Create))
        ))
        .bind(d)
        .fetch_one(&st.db)
        .await?;
        if !found {
            return Err(bad(u.lang, ErrorKey::DriverNotFound));
        }
    }
    Ok(Clean {
        name,
        phone,
        notes,
        area_id: req.area_id,
        pickup_address,
        seats,
        leg,
        driver_id: req.driver_id,
    })
}

/// This user may record visitors on this event's plan, and the event is not cancelled.
async fn open_event(st: &AppState, u: &CurrentUser, id: i64) -> AppResult<()> {
    require(u, Page::AdminPlan, Act::Update)?;
    let ev = load_event(st, u, id).await?;
    if !may_edit(u) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    if ev.status == "cancelled" {
        return Err(conflict(u.lang, ErrorKey::InvalidStatus));
    }
    Ok(())
}

async fn create_visitor(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<VisitorIn>,
) -> AppResult<Json<Value>> {
    open_event(&st, &u, id).await?;
    let v = clean(&st, &u, req).await?;
    let stamp = now();
    let vid: i64 = sqlx::query_scalar(
        "INSERT INTO plan_visitors (event_id, name, phone, notes, area_id, pickup_address, seats, leg,
                                    driver_id, created_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11) RETURNING id",
    )
    .bind(id)
    .bind(&v.name)
    .bind(&v.phone)
    .bind(&v.notes)
    .bind(v.area_id)
    .bind(&v.pickup_address)
    .bind(v.seats)
    .bind(&v.leg)
    .bind(v.driver_id)
    .bind(u.id)
    .bind(&stamp)
    .fetch_one(&st.db)
    .await?;
    audit(
        &st.db,
        Some(u.id),
        "plan.visitor_add",
        "event",
        Some(id),
        format!("event {id}: visitor #{vid} ({} seat(s))", v.seats),
    )
    .await;
    Ok(Json(json!({ "id": vid })))
}

/// The visitor exists under this event, and this user may change it.
async fn owned(st: &AppState, u: &CurrentUser, event: i64, vid: i64) -> AppResult<()> {
    if !may_edit(u) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    let found: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM plan_visitors WHERE id = $1 AND event_id = $2)",
    )
    .bind(vid)
    .bind(event)
    .fetch_one(&st.db)
    .await?;
    if !found {
        return Err(not_found(u.lang));
    }
    Ok(())
}

async fn update_visitor(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, vid)): Path<(i64, i64)>,
    Json(req): Json<VisitorIn>,
) -> AppResult<Json<Value>> {
    open_event(&st, &u, id).await?;
    owned(&st, &u, id, vid).await?;
    let v = clean(&st, &u, req).await?;
    sqlx::query(
        "UPDATE plan_visitors SET name=$3, phone=$4, notes=$5, area_id=$6, pickup_address=$7,
                seats=$8, leg=$9, driver_id=$10, updated_at=$11
         WHERE id=$1 AND event_id=$2",
    )
    .bind(vid)
    .bind(id)
    .bind(&v.name)
    .bind(&v.phone)
    .bind(&v.notes)
    .bind(v.area_id)
    .bind(&v.pickup_address)
    .bind(v.seats)
    .bind(&v.leg)
    .bind(v.driver_id)
    .bind(now())
    .execute(&st.db)
    .await?;
    audit(
        &st.db,
        Some(u.id),
        "plan.visitor_update",
        "event",
        Some(id),
        format!("event {id}: visitor #{vid}"),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_visitor(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, vid)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    load_event(&st, &u, id).await?;
    owned(&st, &u, id, vid).await?;
    sqlx::query("DELETE FROM plan_visitors WHERE id=$1 AND event_id=$2")
        .bind(vid)
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "plan.visitor_delete",
        "event",
        Some(id),
        format!("event {id}: visitor #{vid}"),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
