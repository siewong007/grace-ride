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
//!
//! A traffic lead may also ask for a **special reimbursement** for a visitor: an amount and
//! a required reason, paid to the driver the visitor rode with. It is an ordinary claim
//! (`kind = 'visitor_special'`, status `pending`), so it goes through the usual review
//! (approve / return / reject, not by whoever asked for it) and then finance payout. One
//! live request per visitor: another only once the last one was rejected. A returned one
//! is corrected and sent again here, by a traffic lead, not by the driver.

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
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
use crate::util::{audit, check_phone, fmt_rm, notify, notify_perm, now, require, Lang};

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
        .route(
            "/admin/events/{id}/plan/visitors/{vid}/reimbursement",
            post(request_special_reimbursement),
        )
        .route(
            "/admin/events/{id}/plan/visitors/{vid}/reimbursement/{rid}",
            put(resubmit_special_reimbursement),
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
    reimb_id: Option<i64>,
    reimb_status: Option<String>,
    reimb_amount_cents: Option<i64>,
    reimb_reason: Option<String>,
    reimb_review_note: Option<String>,
    reimb_payee: Option<String>,
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
                c.name AS created_by_name, v.created_at, v.updated_at,
                sr.id AS reimb_id, sr.status AS reimb_status, sr.amount_cents AS reimb_amount_cents,
                sr.special_reason AS reimb_reason, sr.review_note AS reimb_review_note,
                sr.payee_name AS reimb_payee
         FROM plan_visitors v
         LEFT JOIN areas a ON a.id = v.area_id
         LEFT JOIN users d ON d.id = v.driver_id
         LEFT JOIN users c ON c.id = v.created_by
         -- The visitor's latest special reimbursement, if any.
         LEFT JOIN LATERAL (
             SELECT r.id, r.status, r.amount_cents, r.special_reason, r.review_note, r.payee_name
             FROM reimbursements r
             WHERE r.plan_visitor_id = v.id AND r.kind = 'visitor_special'
             ORDER BY r.id DESC LIMIT 1
         ) sr ON TRUE
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
                "special_reimbursement": v.reimb_id.map(|rid| json!({
                    "id": rid,
                    "status": v.reimb_status,
                    "amount_cents": v.reimb_amount_cents,
                    "high_amount": v.reimb_amount_cents.is_some_and(|c| c > SPECIAL_HIGH_CENTS),
                    "reason": v.reimb_reason,
                    "review_note": v.reimb_review_note,
                    "payee_name": v.reimb_payee,
                })),
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
pub(crate) struct VisitorIn {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) phone: String,
    #[serde(default)]
    pub(crate) notes: String,
    pub(crate) area_id: Option<i64>,
    #[serde(default)]
    pub(crate) pickup_address: String,
    pub(crate) seats: Option<i64>,
    pub(crate) leg: Option<String>,
    pub(crate) driver_id: Option<i64>,
}

/// A checked visitor, ready to store.
pub(crate) struct Clean {
    pub(crate) name: String,
    pub(crate) phone: String,
    pub(crate) notes: String,
    pub(crate) area_id: Option<i64>,
    pub(crate) pickup_address: String,
    pub(crate) seats: i64,
    pub(crate) leg: String,
    pub(crate) driver_id: Option<i64>,
}

pub(crate) struct ValidatedFields {
    pub(crate) name: String,
    pub(crate) phone: String,
    pub(crate) notes: String,
    pub(crate) pickup_address: String,
    pub(crate) seats: i64,
    pub(crate) leg: String,
}

pub(crate) fn validate_fields(u: &CurrentUser, req: &VisitorIn) -> AppResult<ValidatedFields> {
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
    let leg = req.leg.as_deref().unwrap_or("outbound").to_string();
    if leg != "outbound" && leg != "return" {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    Ok(ValidatedFields {
        name,
        phone,
        notes,
        pickup_address,
        seats,
        leg,
    })
}

pub(crate) async fn clean(st: &AppState, u: &CurrentUser, req: VisitorIn) -> AppResult<Clean> {
    let fields = validate_fields(u, &req)?;
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
        name: fields.name,
        phone: fields.phone,
        notes: fields.notes,
        area_id: req.area_id,
        pickup_address: fields.pickup_address,
        seats: fields.seats,
        leg: fields.leg,
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

async fn check_driver_for_visitor_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lang: Lang,
    event_id: i64,
    driver_id: i64,
    leg: &str,
    seats: i64,
    exclude_visitor_id: Option<i64>,
) -> AppResult<()> {
    sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(driver_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| bad(lang, ErrorKey::DriverNotFound))?;
    let row: Option<(i64, i64, Option<i64>)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT u.carpool_enabled, u.spare_seats, v.seats
         FROM users u
         JOIN vehicles v ON v.user_id = u.id AND v.is_active = 1
         WHERE u.id = $1 AND {}",
        perms::sql_holds("u", Perm(Page::Drive, Act::Create))
    ))
    .bind(driver_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((_carpool, spare, v_seats)) = row else {
        return Err(bad(lang, ErrorKey::DriverNotFound));
    };
    let cap = super::rides::seat_capacity(spare, v_seats);
    let (ride_seats,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(seats), 0)::BIGINT FROM rides
         WHERE event_id = $1 AND driver_id = $2 AND leg = $3
           AND status IN ('pending_passenger','pending_approval','pending_driver_confirm','confirmed','in_progress')",
    )
    .bind(event_id)
    .bind(driver_id)
    .bind(leg)
    .fetch_one(&mut **tx)
    .await?;
    let (visitor_seats,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(seats), 0)::BIGINT FROM plan_visitors
         WHERE event_id = $1 AND driver_id = $2 AND leg = $3 AND id IS DISTINCT FROM $4",
    )
    .bind(event_id)
    .bind(driver_id)
    .bind(leg)
    .bind(exclude_visitor_id)
    .fetch_one(&mut **tx)
    .await?;
    let load = ride_seats + visitor_seats;
    if load + seats > cap {
        return Err(bad(
            lang,
            ErrorKey::NotEnoughDriverSeatsReassign { load, cap },
        ));
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
    let mut tx = st.db.begin().await?;
    let ev_status: String = sqlx::query_scalar("SELECT status FROM events WHERE id = $1 FOR SHARE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    if ev_status == "cancelled" {
        return Err(conflict(u.lang, ErrorKey::InvalidStatus));
    }
    if let Some(d) = v.driver_id {
        check_driver_for_visitor_tx(&mut tx, u.lang, id, d, &v.leg, v.seats, None).await?;
    }
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
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
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
    let mut tx = st.db.begin().await?;
    let ev_status: String = sqlx::query_scalar("SELECT status FROM events WHERE id = $1 FOR SHARE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    if ev_status == "cancelled" {
        return Err(conflict(u.lang, ErrorKey::InvalidStatus));
    }
    let old_driver: Option<Option<i64>> = sqlx::query_scalar(
        "SELECT driver_id FROM plan_visitors WHERE id = $1 AND event_id = $2 FOR UPDATE",
    )
    .bind(vid)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(old_driver) = old_driver else {
        return Err(not_found(u.lang));
    };
    if old_driver != v.driver_id {
        let live_reimb: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM reimbursements WHERE plan_visitor_id = $1 AND status <> 'rejected')",
        )
        .bind(vid)
        .fetch_one(&mut *tx)
        .await?;
        if live_reimb {
            return Err(conflict(u.lang, ErrorKey::VisitorHasReimbursement));
        }
    }
    if let Some(d) = v.driver_id {
        check_driver_for_visitor_tx(&mut tx, u.lang, id, d, &v.leg, v.seats, Some(vid)).await?;
    }
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
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
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
    if !may_edit(&u) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    let mut tx = st.db.begin().await?;
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM plan_visitors WHERE id = $1 AND event_id = $2 FOR UPDATE")
            .bind(vid)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    if row.is_none() {
        return Err(not_found(u.lang));
    }
    // A visitor with a live (not rejected) special reimbursement stays on record.
    let live: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM reimbursements WHERE plan_visitor_id = $1 AND status <> 'rejected')",
    )
    .bind(vid)
    .fetch_one(&mut *tx)
    .await?;
    if live {
        return Err(conflict(u.lang, ErrorKey::VisitorHasReimbursement));
    }
    sqlx::query("DELETE FROM plan_visitors WHERE id=$1 AND event_id=$2")
        .bind(vid)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
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

// ---------------- Special reimbursement for a visitor ----------------

/// No ceiling on a special reimbursement (John: "no maximum"); above this the request
/// form and reviewers are warned. A warning only, never a block.
pub(crate) const SPECIAL_HIGH_CENTS: i64 = 10_000;
const REASON_MAX: usize = 300;

#[derive(Deserialize)]
struct SpecialIn {
    amount_cents: i64,
    #[serde(default)]
    reason: String,
}

fn clean_special(u: &CurrentUser, req: &SpecialIn) -> AppResult<String> {
    if req.amount_cents <= 0 {
        return Err(bad(u.lang, ErrorKey::AmountMustBePositive));
    }
    if req.amount_cents > 10_000_000 {
        return Err(bad(u.lang, ErrorKey::AmountRangeInvalid));
    }
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        return Err(bad(u.lang, ErrorKey::ReasonRequired));
    }
    if reason.chars().count() > REASON_MAX {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    Ok(reason)
}

fn special_description(visitor: &str, reason: &str) -> String {
    format!("[访客特别报销 Visitor special] {visitor} · {reason}")
}

async fn tell_reviewers(st: &AppState, payee: &str, visitor: &str, amount: i64, event_title: &str) {
    notify_perm(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        if amount > SPECIAL_HIGH_CENTS {
            (
                "访客特别报销待审核（高额，超过 RM100）",
                "Visitor special reimbursement to review (high amount, over RM 100)",
            )
        } else {
            (
                "访客特别报销待审核",
                "Visitor special reimbursement to review",
            )
        },
        (
            &format!(
                "{event_title} · 访客 {visitor} · 付给 {payee} · {}",
                fmt_rm(amount)
            ),
            &format!(
                "{event_title} · visitor {visitor} · pays {payee} · {}",
                fmt_rm(amount)
            ),
        ),
        "/admin/reimbursements",
    )
    .await;
}

/// A traffic lead asks for a special reimbursement for this visitor, paid to their driver.
async fn request_special_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, vid)): Path<(i64, i64)>,
    Json(req): Json<SpecialIn>,
) -> AppResult<Json<Value>> {
    open_event(&st, &u, id).await?;
    owned(&st, &u, id, vid).await?;
    let reason = clean_special(&u, &req)?;

    let mut tx = st.db.begin().await?;
    // Lock the visitor so two requests sent together can't both pass the live check.
    let (visitor, driver, event_title, ev_status): (String, Option<i64>, String, String) =
        sqlx::query_as(
            "SELECT v.name, v.driver_id, e.title, e.status FROM plan_visitors v JOIN events e ON e.id = v.event_id
             WHERE v.id = $1 AND v.event_id = $2 FOR UPDATE OF v",
        )
        .bind(vid)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    if !matches!(ev_status.as_str(), "published" | "finished") {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    let driver = driver.ok_or_else(|| bad(u.lang, ErrorKey::VisitorReimbNeedsDriver))?;
    let (payee, phone): (String, String) =
        sqlx::query_as("SELECT name, phone FROM users WHERE id = $1 AND status = 'active'")
            .bind(driver)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| bad(u.lang, ErrorKey::VisitorReimbNeedsDriver))?;
    let live: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM reimbursements WHERE plan_visitor_id = $1 AND status <> 'rejected')",
    )
    .bind(vid)
    .fetch_one(&mut *tx)
    .await?;
    if live {
        return Err(conflict(u.lang, ErrorKey::VisitorReimbAlreadyLive));
    }
    let stamp = now();
    let rid: i64 = sqlx::query_scalar(
        "INSERT INTO reimbursements (user_id, event_id, amount_cents, description, receipt_file_id,
                                     payee_name, payee_phone, status, kind, plan_visitor_id,
                                     special_reason, requested_by, created_at, updated_at)
         VALUES ($1, $2, $3, $4, NULL, $5, $6, 'pending', 'visitor_special', $7, $8, $9, $10, $10)
         RETURNING id",
    )
    .bind(driver)
    .bind(id)
    .bind(req.amount_cents)
    .bind(special_description(&visitor, &reason))
    .bind(&payee)
    .bind(&phone)
    .bind(vid)
    .bind(&reason)
    .bind(u.id)
    .bind(&stamp)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "reimbursement.visitor_special",
        "reimbursement",
        Some(rid),
        format!(
            "event {id}: visitor #{vid} {visitor} · {} to {payee} · {reason}",
            fmt_rm(req.amount_cents)
        ),
    )
    .await;
    notify(
        &st.db,
        driver,
        ("访客特别报销已申请", "Visitor special reimbursement requested"),
        (
            &format!(
                "交通组为您载送访客 {visitor} 申请了特别报销 {}：{reason}。审核后由财务付款。",
                fmt_rm(req.amount_cents)
            ),
            &format!(
                "The traffic team asked for a special reimbursement of {} for carrying visitor {visitor}: {reason}. Finance pays it once approved.",
                fmt_rm(req.amount_cents)
            ),
        ),
        &format!("/reimbursements/{rid}"),
    )
    .await;
    tell_reviewers(&st, &payee, &visitor, req.amount_cents, &event_title).await;
    Ok(Json(json!({ "id": rid })))
}

/// A traffic lead corrects a returned special reimbursement and sends it for review again.
async fn resubmit_special_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, vid, rid)): Path<(i64, i64, i64)>,
    Json(req): Json<SpecialIn>,
) -> AppResult<Json<Value>> {
    open_event(&st, &u, id).await?;
    owned(&st, &u, id, vid).await?;
    let reason = clean_special(&u, &req)?;
    let mut tx = st.db.begin().await?;
    let row: Option<(String, Option<i64>, i64, String, String, String, String)> = sqlx::query_as(
        "SELECT v.name, v.driver_id, r.user_id, r.payee_name, e.title, e.status, r.status
         FROM reimbursements r
         JOIN plan_visitors v ON v.id = r.plan_visitor_id
         JOIN events e ON e.id = v.event_id
         WHERE r.id = $1 AND r.plan_visitor_id = $2 AND v.event_id = $3 AND r.kind = 'visitor_special'
         FOR UPDATE OF r, v",
    )
    .bind(rid)
    .bind(vid)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let (visitor, v_driver_id, r_user_id, payee, event_title, ev_status, r_status) =
        row.ok_or_else(|| not_found(u.lang))?;
    if !matches!(ev_status.as_str(), "published" | "finished") {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    if v_driver_id != Some(r_user_id) {
        return Err(bad(u.lang, ErrorKey::VisitorReimbNeedsDriver));
    }
    if r_status != "returned" {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    // Only a returned one, and only if nobody changed it in between.
    let changed = sqlx::query(
        "UPDATE reimbursements SET amount_cents = $1, special_reason = $2, description = $3,
                status = 'pending', payee_verified = 0, requested_by = $4, updated_at = $5
         WHERE id = $6 AND status = 'returned'",
    )
    .bind(req.amount_cents)
    .bind(&reason)
    .bind(special_description(&visitor, &reason))
    .bind(u.id)
    .bind(now())
    .bind(rid)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.visitor_special_resubmit",
        "reimbursement",
        Some(rid),
        format!(
            "event {id}: visitor #{vid} {visitor} · {} to {payee} · {reason}",
            fmt_rm(req.amount_cents)
        ),
    )
    .await;
    tell_reviewers(&st, &payee, &visitor, req.amount_cents, &event_title).await;
    Ok(Json(json!({ "ok": true })))
}
