//! 接送配对 workflow:
//! 乘客报名 → 司机选择 → 乘客接受 → 交通组批准 → 行程中 → 已完成
//! plus 临时变动 (driver withdraws, reassignment, cancellations, no-shows, Grab alternative),
//! per-trip chat and live location while a trip is in progress.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::perms::{self, Act, Page, Perm};
use crate::state::AppState;
use crate::util::{
    audit, norm_dt, notify, notify_perm, notify_trip, now, require, system_message, ErrorKey, Lang,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/areas", get(list_areas))
        .route("/places", get(search_places))
        .route("/rides", get(my_rides).post(create_ride))
        .route("/rides/{id}", get(get_ride))
        .route("/rides/{id}/cancel", post(cancel_ride))
        .route("/rides/{id}/respond", post(passenger_respond))
        .route("/rides/{id}/claim", post(driver_claim))
        .route("/rides/{id}/withdraw", post(driver_withdraw))
        .route(
            "/rides/{id}/confirm-assignment",
            post(driver_confirm_assignment),
        )
        .route("/rides/{id}/start", post(start_ride))
        .route("/rides/{id}/pickup", post(pickup))
        .route("/rides/{id}/no-show", post(no_show))
        .route("/rides/{id}/complete", post(complete))
        .route("/rides/{id}/approve", post(staff_approve))
        .route("/rides/{id}/return", post(staff_return))
        .route("/rides/{id}/reassign", post(staff_reassign))
        .route("/rides/{id}/alt-transport", post(staff_alt_transport))
        .route(
            "/rides/{id}/messages",
            get(list_messages).post(post_message),
        )
        .route("/rides/{id}/location", get(ride_location))
        .route(
            "/rides/{id}/passenger-location",
            post(post_passenger_location).delete(stop_passenger_location),
        )
        .route("/driver/requests", get(driver_requests))
        .route("/driver/schedule", get(driver_schedule))
        .route("/driver/route", get(driver_route))
        .route(
            "/driver/profile",
            get(get_driver_profile).put(put_driver_profile),
        )
        .route(
            "/driver/location",
            post(post_location).delete(stop_location),
        )
        .route("/admin/rides", get(admin_rides))
        .route("/admin/live", get(admin_live))
        .route("/admin/drivers", get(admin_drivers))
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct RideRow {
    pub id: i64,
    pub passenger_id: i64,
    pub driver_id: Option<i64>,
    pub event_id: Option<i64>,
    pub trip_id: Option<i64>,
    pub pickup_at: String,
    pub pickup_area_id: i64,
    pub pickup_address: String,
    pub destination: String,
    pub seats: i64,
    pub assistance: String,
    pub notes: String,
    pub status: String,
    pub needs_attention: i64,
    pub attention_reason: String,
    pub started_at: Option<String>,
    pub picked_up_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub area_name: String,
    pub p_name: String,
    pub p_phone: String,
    pub d_name: Option<String>,
    pub d_phone: Option<String>,
    pub v_model: Option<String>,
    pub v_color: Option<String>,
    pub v_plate: Option<String>,
    pub v_type: Option<String>,
    pub distance_km: Option<i64>,
    pub reimbursement_amount_cents: Option<i64>,
    pub event_title: Option<String>,
}

const RIDE_SELECT: &str = "SELECT r.id, r.passenger_id, r.driver_id, r.event_id, r.trip_id, r.pickup_at, r.pickup_area_id,
    r.pickup_address, r.destination, r.seats, r.assistance, r.notes, r.status, r.needs_attention, r.attention_reason,
    r.started_at, r.picked_up_at, r.completed_at, r.distance_km, r.reimbursement_amount_cents, r.created_at, r.updated_at,
    a.name AS area_name, p.name AS p_name, p.phone AS p_phone, d.name AS d_name, d.phone AS d_phone,
    v.model AS v_model, v.color AS v_color, v.plate AS v_plate, v.vehicle_type AS v_type, e.title AS event_title
    FROM rides r
    JOIN areas a ON a.id = r.pickup_area_id
    JOIN users p ON p.id = r.passenger_id
    LEFT JOIN users d ON d.id = r.driver_id
    LEFT JOIN vehicles v ON v.user_id = r.driver_id
    LEFT JOIN events e ON e.id = r.event_id";

pub async fn load_ride(st: &AppState, lang: Lang, id: i64) -> AppResult<RideRow> {
    sqlx::query_as::<_, RideRow>(&format!("{RIDE_SELECT} WHERE r.id = $1"))
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(lang))
}

/// Role-aware projection. Implements 按身份保护个人资料:
/// - home address only to the passenger, staff, and the assigned driver once approved;
/// - driver phone/vehicle to the passenger once a driver has chosen them;
/// - passenger phone to the driver once approved.
pub fn ride_json(r: &RideRow, u: &CurrentUser) -> Value {
    let is_passenger = r.passenger_id == u.id;
    let is_driver = r.driver_id == Some(u.id);
    let staff = u.can(Page::AdminRides, Act::View);
    let approved = matches!(r.status.as_str(), "confirmed" | "in_progress");
    let show_address = is_passenger || staff || (is_driver && approved);
    let driver = r.driver_id.map(|id| {
        json!({
            "id": id,
            "name": r.d_name,
            "phone": if is_driver || staff || (is_passenger && approved) { r.d_phone.clone() } else { None },
            "vehicle": r.v_model.as_ref().map(|m| json!({
                "model": m,
                "color": r.v_color,
                "plate": r.v_plate,
                "type": r.v_type,
            })),
        })
    });
    json!({
        "id": r.id,
        "status": r.status,
        "pickup_at": r.pickup_at,
        "area": {"id": r.pickup_area_id, "name": r.area_name},
        "pickup_address": if show_address { Some(r.pickup_address.clone()) } else { None },
        "destination": r.destination,
        "seats": r.seats,
        "assistance": r.assistance,
        "notes": r.notes,
        "event": r.event_id.map(|id| json!({"id": id, "title": r.event_title})),
        "trip_id": r.trip_id,
        "distance_km": r.distance_km,
        "reimbursement_amount_cents": r.reimbursement_amount_cents,
        "passenger": {
            "id": r.passenger_id,
            "name": r.p_name,
            "phone": if is_passenger || staff || (is_driver && approved) { Some(r.p_phone.clone()) } else { None },
        },
        "driver": driver,
        "needs_attention": r.needs_attention == 1,
        "attention_reason": if staff || is_passenger || is_driver { r.attention_reason.clone() } else { String::new() },
        "started_at": r.started_at,
        "picked_up_at": r.picked_up_at,
        "completed_at": r.completed_at,
        "created_at": r.created_at,
        "updated_at": r.updated_at,
        "viewer": {
            "is_passenger": is_passenger, "is_driver": is_driver, "is_staff": staff,
        },
    })
}

fn can_view(r: &RideRow, u: &CurrentUser) -> bool {
    r.passenger_id == u.id
        || r.driver_id == Some(u.id)
        || u.can(Page::AdminRides, Act::View)
        || (u.can(Page::Drive, Act::View) && r.status == "pending_driver")
}

async fn set_status(st: &AppState, id: i64, from: &[&str], to: &str, lang: Lang) -> AppResult<()> {
    let placeholders = (0..from.len())
        .map(|i| format!("${}", i + 4))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "UPDATE rides SET status=$1, updated_at=$2 WHERE id=$3 AND status IN ({placeholders})"
    );
    let mut q = sqlx::query(&sql).bind(to).bind(now()).bind(id);
    for f in from {
        q = q.bind(*f);
    }
    if q.execute(&st.db).await?.rows_affected() == 0 {
        return Err(conflict(lang, ErrorKey::RideStatusChanged));
    }
    Ok(())
}

// ---------------- Areas ----------------

#[derive(Deserialize)]
struct PlacesQuery {
    q: String,
}

/// Address autocomplete for the pickup, destination and event location fields.
/// Signed-in members only, so the proxy can't be used by anyone on the internet.
async fn search_places(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<PlacesQuery>,
) -> AppResult<Json<Value>> {
    let hits = crate::routing::search_places(&st, &q.q, u.lang.code()).await;
    Ok(Json(json!(hits)))
}

async fn list_areas(State(st): State<AppState>, _u: CurrentUser) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, i64)> =
        sqlx::query_as("SELECT id, name, active FROM areas ORDER BY name")
            .fetch_all(&st.db)
            .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, name, a)| json!({"id": id, "name": name, "active": a == 1}))
        .collect::<Vec<_>>())))
}

// ---------------- Passenger ----------------

#[derive(Deserialize)]
struct CreateRide {
    pickup_at: String,
    pickup_area_id: i64,
    pickup_address: String,
    destination: String,
    seats: Option<i64>,
    assistance: Option<String>,
    notes: Option<String>,
    event_id: Option<i64>,
}

async fn create_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CreateRide>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::Create)?;
    let at = norm_dt(&req.pickup_at).ok_or_else(|| bad(u.lang, ErrorKey::InvalidPickupTime))?;
    if at.as_str() < &now()[..16] {
        return Err(bad(u.lang, ErrorKey::PickupTimeInPast));
    }
    let seats = req.seats.unwrap_or(1);
    if !(1..=6).contains(&seats) {
        return Err(bad(u.lang, ErrorKey::SeatsRange));
    }
    if req.pickup_address.trim().is_empty() || req.destination.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::AddressAndDestinationRequired));
    }
    let area: Option<(i64,)> = sqlx::query_as("SELECT id FROM areas WHERE id=$1 AND active=1")
        .bind(req.pickup_area_id)
        .fetch_optional(&st.db)
        .await?;
    area.ok_or_else(|| bad(u.lang, ErrorKey::PickupAreaRequired))?;
    if let Some(eid) = req.event_id {
        if !crate::routes::events::can_see_event(&st, &u, eid).await? {
            return Err(forbidden(u.lang, ErrorKey::CannotViewEvent));
        }
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, notes, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
    )
    .bind(u.id)
    .bind(req.event_id)
    .bind(&at)
    .bind(req.pickup_area_id)
    .bind(req.pickup_address.trim())
    .bind(req.destination.trim())
    .bind(seats)
    .bind(req.assistance.unwrap_or_default())
    .bind(req.notes.unwrap_or_default())
    .bind(now())
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    audit(&st.db, Some(u.id), "ride.create", "ride", Some(id), &at).await;

    // Let drivers who serve that area know there's a new request.
    let drivers: Vec<(i64,)> = sqlx::query_as(
        "SELECT da.user_id FROM driver_areas da JOIN users u ON u.id = da.user_id
         WHERE da.area_id=$1 AND u.status='active' AND u.id != $2",
    )
    .bind(req.pickup_area_id)
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    for (d,) in drivers {
        let when = format!("{} · {}", at.replace('T', " "), seats);
        notify(
            &st.db,
            d,
            ("新的接送报名", "New ride request"),
            (&when, &when),
            "/drive",
        )
        .await;
    }
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
struct MineQuery {
    scope: Option<String>, // upcoming | past
}

async fn my_rides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<MineQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::View)?;
    let past = q.scope.as_deref() == Some("past");
    let sql = if past {
        format!("{RIDE_SELECT} WHERE r.passenger_id=$1 AND r.status IN ('completed','cancelled','no_show','alt_transport') ORDER BY r.pickup_at DESC LIMIT 50")
    } else {
        format!("{RIDE_SELECT} WHERE r.passenger_id=$1 AND r.status NOT IN ('completed','cancelled','no_show','alt_transport') ORDER BY r.pickup_at")
    };
    let rows = sqlx::query_as::<_, RideRow>(&sql)
        .bind(u.id)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(json!(rows
        .iter()
        .map(|r| ride_json(r, &u))
        .collect::<Vec<_>>())))
}

async fn get_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if !can_view(&r, &u) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let mut v = ride_json(&r, &u);
    if u.can(Page::AdminRides, Act::View) {
        if let Some(d) = r.driver_id {
            let (load, cap) = driver_load(&st, d, &r.pickup_at, Some(r.id)).await?;
            v["driver_load"] = json!({ "seats_used": load, "capacity": cap });
        }
        // Passenger time conflicts
        let conflicts: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, pickup_at FROM rides WHERE passenger_id=$1 AND id != $2 AND status IN ('pending_approval','confirmed','in_progress')
             AND abs(EXTRACT(EPOCH FROM (pickup_at::timestamp - $3::timestamp))) < 3600",
        )
        .bind(r.passenger_id)
        .bind(r.id)
        .bind(&r.pickup_at)
        .fetch_all(&st.db)
        .await?;
        v["passenger_conflicts"] = json!(conflicts
            .into_iter()
            .map(|(i, t)| json!({"id": i, "pickup_at": t}))
            .collect::<Vec<_>>());
    }
    Ok(Json(v))
}

#[derive(Deserialize, Default)]
struct ReasonReq {
    reason: Option<String>,
}

async fn cancel_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    body: Option<Json<ReasonReq>>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    let own = r.passenger_id == u.id && u.can(Page::Rides, Act::Delete);
    if !own && !u.can(Page::AdminRides, Act::Delete) {
        return Err(forbidden(u.lang, ErrorKey::OnlyPassengerOrStaffCanCancel));
    }
    let reason = body.and_then(|b| b.0.reason).unwrap_or_default();
    set_status(
        &st,
        id,
        &[
            "pending_driver",
            "pending_passenger",
            "pending_approval",
            "pending_driver_confirm",
            "confirmed",
        ],
        "cancelled",
        u.lang,
    )
    .await?;
    sqlx::query("UPDATE rides SET needs_attention=0, attention_reason=$1 WHERE id=$2")
        .bind(&reason)
        .bind(id)
        .execute(&st.db)
        .await?;
    if let Some(d) = r.driver_id {
        let line = format!(
            "{} · {} {}",
            r.p_name,
            r.pickup_at.replace('T', " "),
            reason
        );
        notify(
            &st.db,
            d,
            ("乘客已取消接送", "Passenger cancelled the ride"),
            (&line, &line),
            "/drive",
        )
        .await;
    }
    if let Some(t) = r.trip_id {
        system_message(
            &st.db,
            t,
            &format!("{} 已取消本趟接送，座位已释放。", r.p_name),
            &format!(
                "{} cancelled this ride; the seat has been released.",
                r.p_name
            ),
        )
        .await;
    }
    if r.status == "confirmed" {
        let line = format!("{} · {}", r.p_name, r.pickup_at.replace('T', " "));
        notify_perm(
            &st.db,
            Page::AdminRides,
            Act::Update,
            ("已确认的接送被取消", "A confirmed ride was cancelled"),
            (&line, &line),
            &format!("/admin/rides/{id}"),
        )
        .await;
    }
    audit(&st.db, Some(u.id), "ride.cancel", "ride", Some(id), reason).await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct RespondReq {
    accept: bool,
}

/// 乘客接受 / 婉拒 司机邀请
async fn passenger_respond(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<RespondReq>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.passenger_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::OnlyPassengerCanRespond));
    }
    require(&u, Page::Rides, Act::Update)?;
    if req.accept {
        set_status(&st, id, &["pending_passenger"], "pending_approval", u.lang).await?;
        let line = format!(
            "{} ← {} · {}",
            r.p_name,
            r.d_name.clone().unwrap_or_default(),
            r.pickup_at.replace('T', " ")
        );
        notify_perm(
            &st.db,
            Page::AdminRides,
            Act::Update,
            ("接送待审批", "Ride awaiting approval"),
            (&line, &line),
            &format!("/admin/rides/{id}"),
        )
        .await;
        if let Some(d) = r.driver_id {
            notify(
                &st.db,
                d,
                ("乘客已接受您的邀请", "Passenger accepted your invitation"),
                (
                    &format!("{} · 等待交通组批准", r.p_name),
                    &format!("{} · awaiting approval by transport staff", r.p_name),
                ),
                "/drive",
            )
            .await;
        }
    } else {
        set_status(&st, id, &["pending_passenger"], "pending_driver", u.lang).await?;
        sqlx::query("UPDATE rides SET driver_id=NULL WHERE id=$1")
            .bind(id)
            .execute(&st.db)
            .await?;
        if let Some(d) = r.driver_id {
            let line = format!("{} · {}", r.p_name, r.pickup_at.replace('T', " "));
            notify(
                &st.db,
                d,
                ("乘客婉拒了邀请", "Passenger declined the invitation"),
                (&line, &line),
                "/drive",
            )
            .await;
        }
    }
    audit(
        &st.db,
        Some(u.id),
        if req.accept {
            "ride.passenger_accept"
        } else {
            "ride.passenger_decline"
        },
        "ride",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Driver ----------------

/// Seats the driver already holds within ±60 min of `at`, and their seat capacity.
async fn driver_load(
    st: &AppState,
    driver: i64,
    at: &str,
    exclude: Option<i64>,
) -> AppResult<(i64, i64)> {
    let (load,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(seats),0)::BIGINT FROM rides WHERE driver_id=$1 AND id != $2 AND status IN ('pending_passenger','pending_approval','pending_driver_confirm','confirmed','in_progress')
         AND abs(EXTRACT(EPOCH FROM (pickup_at::timestamp - $3::timestamp))) < 3600",
    )
    .bind(driver)
    .bind(exclude.unwrap_or(-1))
    .bind(at)
    .fetch_one(&st.db)
    .await?;
    let row: Option<(i64, i64, Option<i64>)> = sqlx::query_as(
        "SELECT u.carpool_enabled, u.spare_seats, v.seats FROM users u LEFT JOIN vehicles v ON v.user_id=u.id WHERE u.id=$1",
    )
    .bind(driver)
    .fetch_optional(&st.db)
    .await?;
    let (carpool, spare, vseats) = row.unwrap_or((0, 1, None));
    let cap = seat_capacity(spare, vseats);
    // Without 共乘模式 a driver takes one booking per time slot.
    let cap = if carpool == 1 {
        cap
    } else if load > 0 {
        load
    } else {
        cap
    };
    Ok((load, cap))
}

/// Passenger seats a driver offers: their spare seats, never more than the car holds
/// besides the driver. No vehicle on file ⇒ 0.
pub(crate) fn seat_capacity(spare_seats: i64, vehicle_seats: Option<i64>) -> i64 {
    let vehicle_cap = vehicle_seats.map(|s| (s - 1).max(1)).unwrap_or(0);
    spare_seats.max(1).min(vehicle_cap)
}

async fn driver_claim(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Drive, Act::Create) {
        return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanChoose));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if r.passenger_id == u.id {
        return Err(bad(u.lang, ErrorKey::CannotPickOwnRide));
    }
    let has_vehicle: Option<(i64,)> = sqlx::query_as("SELECT seats FROM vehicles WHERE user_id=$1")
        .bind(u.id)
        .fetch_optional(&st.db)
        .await?;
    if has_vehicle.is_none() {
        return Err(bad(u.lang, ErrorKey::FillVehicleDetailsFirst));
    }
    let (load, cap) = driver_load(&st, u.id, &r.pickup_at, Some(id)).await?;
    if load + r.seats > cap {
        return Err(bad(u.lang, ErrorKey::NotEnoughSeatsSlot { load, cap }));
    }
    // Atomic claim prevents two drivers choosing the same request.
    let res = sqlx::query("UPDATE rides SET driver_id=$1, status='pending_passenger', updated_at=$2 WHERE id=$3 AND status='pending_driver'")
        .bind(u.id)
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RidePickedByAnotherDriver));
    }
    notify(
        &st.db,
        r.passenger_id,
        ("有司机愿意接送您", "A driver offered to take you"),
        (
            &format!(
                "{} 邀请接送您（{}），请查看并确认。",
                u.name,
                r.pickup_at.replace('T', " ")
            ),
            &format!(
                "{} offered to drive you ({}), please review and confirm.",
                u.name,
                r.pickup_at.replace('T', " ")
            ),
        ),
        &format!("/rides/{id}"),
    )
    .await;
    audit(&st.db, Some(u.id), "ride.claim", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

/// 司机不能出车 — release the ride back to the pool and alert 交通组.
async fn driver_withdraw(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReasonReq>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    require(&u, Page::Drive, Act::Delete)?;
    let was_confirmed = matches!(r.status.as_str(), "confirmed" | "pending_driver_confirm");
    set_status(
        &st,
        id,
        &[
            "pending_passenger",
            "pending_approval",
            "pending_driver_confirm",
            "confirmed",
        ],
        "pending_driver",
        u.lang,
    )
    .await?;
    let reason = req.reason.unwrap_or_else(|| "司机无法出车".into());
    sqlx::query("UPDATE rides SET driver_id=NULL, trip_id=NULL, needs_attention=$1, attention_reason=$2 WHERE id=$3")
        .bind(was_confirmed as i64)
        .bind(format!("原司机 {} 无法出车：{}", u.name, reason))
        .bind(id)
        .execute(&st.db)
        .await?;
    notify(
        &st.db,
        r.passenger_id,
        ("司机无法出车", "Driver cannot drive"),
        (
            "交通组正在为您安排其他司机，请留意通知。",
            "Transport staff are arranging another driver for you; please watch for notifications.",
        ),
        &format!("/rides/{id}"),
    )
    .await;
    let line = format!(
        "{} · {} · {}",
        r.p_name,
        r.pickup_at.replace('T', " "),
        reason
    );
    notify_perm(
        &st.db,
        Page::AdminRides,
        Act::Update,
        ("需要重新安排司机", "Driver needs reassignment"),
        (&line, &line),
        &format!("/admin/rides/{id}"),
    )
    .await;
    if let Some(t) = r.trip_id {
        system_message(
            &st.db,
            t,
            &format!("司机 {} 无法出车，交通组正在重新安排。", u.name),
            &format!(
                "Driver {} cannot drive; transport staff are rearranging.",
                u.name
            ),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "ride.driver_withdraw",
        "ride",
        Some(id),
        reason,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

/// Assign a confirmed ride to a trip: same driver, same destination, within 60 minutes.
async fn assign_trip(st: &AppState, r: &RideRow) -> AppResult<i64> {
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT trip_id FROM rides WHERE driver_id=$1 AND id != $2 AND trip_id IS NOT NULL AND status IN ('confirmed','in_progress')
         AND destination = $3 AND abs(EXTRACT(EPOCH FROM (pickup_at::timestamp - $4::timestamp))) < 3600 LIMIT 1",
    )
    .bind(r.driver_id)
    .bind(r.id)
    .bind(&r.destination)
    .bind(&r.pickup_at)
    .fetch_optional(&st.db)
    .await?;
    let trip = existing.map(|t| t.0).unwrap_or(r.id);
    sqlx::query("UPDATE rides SET trip_id=$1 WHERE id=$2")
        .bind(trip)
        .bind(r.id)
        .execute(&st.db)
        .await?;
    Ok(trip)
}

async fn announce_confirmed(st: &AppState, r: &RideRow, trip: i64, reassigned: bool) {
    let vehicle = format!(
        "{} {} · {}",
        r.v_color.clone().unwrap_or_default(),
        r.v_model.clone().unwrap_or_default(),
        r.v_plate.clone().unwrap_or_default()
    );
    let title_pairs = if reassigned {
        (
            "已为您安排新的司机",
            "A new driver has been arranged for you",
        )
    } else {
        ("接送已确认", "Ride confirmed")
    };
    notify(
        &st.db,
        r.passenger_id,
        title_pairs,
        (
            &format!(
                "{} · 司机 {} · {}",
                r.pickup_at.replace('T', " "),
                r.d_name.clone().unwrap_or_default(),
                vehicle
            ),
            &format!(
                "{} · driver {} · {}",
                r.pickup_at.replace('T', " "),
                r.d_name.clone().unwrap_or_default(),
                vehicle
            ),
        ),
        &format!("/rides/{}", r.id),
    )
    .await;
    if let Some(d) = r.driver_id {
        let line = format!(
            "{} · {} · {}",
            r.pickup_at.replace('T', " "),
            r.p_name,
            r.area_name
        );
        notify(
            &st.db,
            d,
            ("接送已获批准", "Ride approved"),
            (&line, &line),
            "/drive",
        )
        .await;
    }
    system_message(
        &st.db,
        trip,
        &format!(
            "{} 已加入本趟群聊（{}）。",
            r.p_name,
            r.pickup_at.replace('T', " ")
        ),
        &format!(
            "{} joined this trip's group chat ({}).",
            r.p_name,
            r.pickup_at.replace('T', " ")
        ),
    )
    .await;
}

/// Driver accepts a reassignment made by 交通组.
async fn driver_confirm_assignment(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    set_status(&st, id, &["pending_driver_confirm"], "confirmed", u.lang).await?;
    sqlx::query("UPDATE rides SET needs_attention=0 WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await?;
    let r = load_ride(&st, u.lang, id).await?;
    let trip = assign_trip(&st, &r).await?;
    announce_confirmed(&st, &r, trip, true).await;
    audit(
        &st.db,
        Some(u.id),
        "ride.driver_confirm",
        "ride",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn start_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    // 同一时间最多追踪 N 位司机
    let (live,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress' AND driver_id != $1",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    if live >= st.cfg.max_live_drivers {
        return Err(conflict(
            u.lang,
            ErrorKey::MaxLiveDriversReached {
                live,
                limit: st.cfg.max_live_drivers,
            },
        ));
    }
    set_status(&st, id, &["confirmed"], "in_progress", u.lang).await?;
    // The driver sets off once for the whole carpool: every other confirmed passenger in
    // this trip starts too, so each of them can follow the car and see their ETA.
    let started_at = now();
    let trip = r.trip_id.unwrap_or(id);
    let others: Vec<(i64,)> = sqlx::query_as(
        "UPDATE rides SET status='in_progress', started_at=$1, updated_at=$1
         WHERE driver_id=$2 AND trip_id=$3 AND id != $4 AND status='confirmed' RETURNING id",
    )
    .bind(&started_at)
    .bind(u.id)
    .bind(trip)
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    sqlx::query("UPDATE rides SET started_at=$1 WHERE id=$2")
        .bind(&started_at)
        .bind(id)
        .execute(&st.db)
        .await?;
    let mut started = vec![r.clone()];
    for (other,) in &others {
        started.push(load_ride(&st, u.lang, *other).await?);
    }
    for ride in &started {
        notify(
            &st.db,
            ride.passenger_id,
            ("司机已出发", "Driver departed"),
            (
                &format!(
                    "{} 正在前往 {}，可在应用内查看位置。",
                    u.name, ride.area_name
                ),
                &format!(
                    "{} is en route to {}; you can follow the location in the app.",
                    u.name, ride.area_name
                ),
            ),
            &format!("/rides/{}", ride.id),
        )
        .await;
        audit(&st.db, Some(u.id), "ride.start", "ride", Some(ride.id), "").await;
    }
    if let Some(t) = r.trip_id {
        let names: Vec<&str> = started.iter().map(|x| x.p_name.as_str()).collect();
        system_message(
            &st.db,
            t,
            &format!("系统：{} 的行程已开始。", names.join("、")),
            &format!("System: trip for {} has started.", names.join(", ")),
        )
        .await;
    }
    // Look up the trip's addresses now, so the first route and ETAs come back quickly.
    let (bg, driver) = (st.clone(), u.id);
    tokio::spawn(async move {
        let _ = crate::routing::driver_route(&bg, driver, Some(trip)).await;
    });
    let ids: Vec<i64> = started.iter().map(|x| x.id).collect();
    Ok(Json(json!({"ok": true, "started": ids})))
}

async fn pickup(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    if r.status != "in_progress" {
        return Err(bad(u.lang, ErrorKey::StartTripFirst));
    }
    sqlx::query("UPDATE rides SET picked_up_at=$1, updated_at=$2 WHERE id=$3")
        .bind(now())
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    clear_passenger_location(&st, id).await?;
    if let Some(t) = r.trip_id {
        system_message(
            &st.db,
            t,
            &format!("{} 已上车。", r.p_name),
            &format!("{} has been picked up.", r.p_name),
        )
        .await;
    }
    audit(&st.db, Some(u.id), "ride.pickup", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

async fn clear_passenger_location(st: &AppState, ride: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM passenger_locations WHERE ride_id=$1")
        .bind(ride)
        .execute(&st.db)
        .await?;
    Ok(())
}

async fn clear_location_if_idle(st: &AppState, driver: i64) -> AppResult<()> {
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

async fn no_show(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) && !u.can(Page::AdminRides, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    set_status(&st, id, &["in_progress", "confirmed"], "no_show", u.lang).await?;
    sqlx::query("UPDATE rides SET completed_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    if let Some(d) = r.driver_id {
        clear_location_if_idle(&st, d).await?;
    }
    clear_passenger_location(&st, id).await?;
    notify(
        &st.db,
        r.passenger_id,
        ("司机记录您未到场", "Driver marked you as a no-show"),
        (
            "如有误会或需要帮助，请联系帮助中心。",
            "If this is a mistake or you need help, please contact the help centre.",
        ),
        &format!("/rides/{id}"),
    )
    .await;
    let line = format!("{} · {}", r.p_name, r.pickup_at.replace('T', " "));
    notify_perm(
        &st.db,
        Page::AdminRides,
        Act::Update,
        ("乘客未到场", "Passenger no-show"),
        (&line, &line),
        &format!("/admin/rides/{id}"),
    )
    .await;
    if let Some(t) = r.trip_id {
        system_message(
            &st.db,
            t,
            &format!("{} 未到场。", r.p_name),
            &format!("{} did not show up.", r.p_name),
        )
        .await;
    }
    audit(&st.db, Some(u.id), "ride.no_show", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

async fn complete(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }

    // --- Auto reimbursement: distance × per-km rate for the driver's vehicle type ---
    let distance_km: Option<i64> = r.distance_km;
    let reimbursement_cents: i64 = match (distance_km, &r.v_type) {
        (Some(km), Some(vt)) => {
            let rate: Option<(i64,)> = sqlx::query_as(
                "SELECT rate_cents_per_km FROM reimbursement_rates WHERE vehicle_type = $1",
            )
            .bind(vt)
            .fetch_optional(&st.db)
            .await?;
            km * rate.map(|r| r.0).unwrap_or(0)
        }
        _ => 0,
    };

    set_status(&st, id, &["in_progress"], "completed", u.lang).await?;
    sqlx::query(
        "UPDATE rides SET completed_at=$1, picked_up_at=COALESCE(picked_up_at, $2), reimbursement_amount_cents=$3 WHERE id=$4",
    )
    .bind(now())
    .bind(now())
    .bind(reimbursement_cents)
    .bind(id)
    .execute(&st.db)
    .await?;
    clear_location_if_idle(&st, u.id).await?;
    clear_passenger_location(&st, id).await?;
    notify(
        &st.db,
        r.passenger_id,
        ("已安全送达", "Dropped off safely"),
        (
            "感谢使用恩典同行，愿主祝福您！",
            "Thank you for riding with Grace Ride; may the Lord bless you!",
        ),
        &format!("/rides/{id}"),
    )
    .await;
    if let Some(t) = r.trip_id {
        system_message(
            &st.db,
            t,
            &format!("{} 已送达。", r.p_name),
            &format!("{} has been dropped off.", r.p_name),
        )
        .await;
        let (open,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rides WHERE trip_id=$1 AND status IN ('confirmed','in_progress')",
        )
        .bind(t)
        .fetch_one(&st.db)
        .await?;
        if open == 0 {
            system_message(
                &st.db,
                t,
                "本趟接送已结束，群聊已关闭（只读）。",
                "This ride is finished; the group chat is now closed (read-only).",
            )
            .await;
        }
    }
    audit(&st.db, Some(u.id), "ride.complete", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct RequestsQuery {
    area_id: Option<i64>,
    date: Option<String>,
    mine_areas: Option<bool>,
}

async fn driver_requests(
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
    ];
    let mut n = 3;
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
        conds.push(format!("substr(r.pickup_at,1,10) = ${n}"));
    }
    let sql = format!(
        "{RIDE_SELECT} WHERE {} ORDER BY r.pickup_at LIMIT 100",
        conds.join(" AND ")
    );
    let mut query = sqlx::query_as::<_, RideRow>(&sql)
        .bind(u.id)
        .bind(now()[..16].to_string());
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
struct RangeQuery {
    from: Option<String>,
    to: Option<String>,
}

async fn driver_schedule(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<RangeQuery>,
) -> AppResult<Json<Value>> {
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
async fn driver_route(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let route = crate::routing::driver_route(&st, u.id, None).await?;
    Ok(Json(json!({ "route": route })))
}

async fn get_driver_profile(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let (carpool, spare): (i64, i64) =
        sqlx::query_as("SELECT carpool_enabled, spare_seats FROM users WHERE id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let areas: Vec<(i64,)> = sqlx::query_as("SELECT area_id FROM driver_areas WHERE user_id=$1")
        .bind(u.id)
        .fetch_all(&st.db)
        .await?;
    let v: Option<(String, String, String, i64, String)> = sqlx::query_as(
        "SELECT model, color, plate, seats, vehicle_type FROM vehicles WHERE user_id=$1",
    )
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    Ok(Json(json!({
        "carpool_enabled": carpool == 1,
        "spare_seats": spare,
        "area_ids": areas.into_iter().map(|a| a.0).collect::<Vec<_>>(),
        "vehicle": v.map(|(m, c, p, s, t)| json!({"model": m, "color": c, "plate": p, "seats": s, "type": t})),
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
}

fn default_vehicle_type() -> String {
    "standard".to_string()
}

#[derive(Deserialize)]
struct DriverProfileIn {
    carpool_enabled: bool,
    spare_seats: i64,
    area_ids: Vec<i64>,
    vehicle: Option<VehicleIn>,
}

async fn put_driver_profile(
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
    if let Some(v) = req.vehicle {
        if v.model.trim().is_empty() || v.plate.trim().is_empty() {
            return Err(bad(u.lang, ErrorKey::VehicleModelAndPlateRequired));
        }
        if !(2..=9).contains(&v.seats) {
            return Err(bad(u.lang, ErrorKey::VehicleSeatsRange));
        }
        sqlx::query(
            "INSERT INTO vehicles (user_id, model, color, plate, seats, vehicle_type) VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT(user_id) DO UPDATE SET model=excluded.model, color=excluded.color, plate=excluded.plate, seats=excluded.seats, vehicle_type=excluded.vehicle_type",
        )
        .bind(u.id)
        .bind(v.model.trim())
        .bind(v.color.trim())
        .bind(v.plate.trim().to_uppercase())
        .bind(v.seats)
        .bind(v.vehicle_type)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct LocIn {
    lat: f64,
    lng: f64,
    heading: Option<f64>,
}

/// Location is accepted only while the driver has a ride in progress (行程开始才共享位置).
async fn post_location(
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

async fn stop_location(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM driver_locations WHERE driver_id=$1")
        .bind(u.id)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({"ok": true})))
}

async fn ride_location(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.passenger_id != u.id && r.driver_id != Some(u.id) && !u.can(Page::AdminRides, Act::View) {
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
        let full_view = r.driver_id == Some(u.id) || u.can(Page::AdminRides, Act::View);
        if let Ok(Some(route)) = crate::routing::driver_route(&st, driver, Some(trip)).await {
            if let Some(eta) =
                crate::routing::ride_eta(&route, id, r.picked_up_at.is_some(), full_view)
            {
                out["eta"] = eta;
            }
        }
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct PassengerLocIn {
    lat: f64,
    lng: f64,
}

/// Passenger opts in to showing the driver where they are waiting. Accepted only
/// while the driver is on the way (in progress, not yet picked up); otherwise the
/// stored spot is dropped and `sharing: false` tells the client to stop.
async fn post_passenger_location(
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
    sqlx::query(
        "INSERT INTO passenger_locations (ride_id, lat, lng, updated_at) VALUES ($1,$2,$3,$4)
         ON CONFLICT(ride_id) DO UPDATE SET lat=excluded.lat, lng=excluded.lng, updated_at=excluded.updated_at",
    )
    .bind(id)
    .bind(req.lat)
    .bind(req.lng)
    .bind(now())
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true, "sharing": true})))
}

async fn stop_passenger_location(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.passenger_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NotRidePassenger));
    }
    clear_passenger_location(&st, id).await?;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Trip chat ----------------

async fn chat_access(st: &AppState, u: &CurrentUser, ride_id: i64) -> AppResult<(i64, bool)> {
    let r = load_ride(st, u.lang, ride_id).await?;
    let trip = r
        .trip_id
        .ok_or_else(|| bad(u.lang, ErrorKey::GroupChatNotApproved))?;
    let members: Vec<(i64, Option<i64>, String)> =
        sqlx::query_as("SELECT passenger_id, driver_id, status FROM rides WHERE trip_id=$1")
            .bind(trip)
            .fetch_all(&st.db)
            .await?;
    let is_member = members
        .iter()
        .any(|(p, d, s)| (*p == u.id && s != "cancelled") || *d == Some(u.id));
    if !is_member && !u.can(Page::AdminRides, Act::View) {
        return Err(forbidden(u.lang, ErrorKey::OnlyTripMembersCanViewChat));
    }
    let open = members
        .iter()
        .any(|(_, _, s)| s == "confirmed" || s == "in_progress");
    Ok((trip, open))
}

#[derive(Deserialize)]
struct AfterQuery {
    after: Option<i64>,
}

async fn list_messages(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Query(q): Query<AfterQuery>,
) -> AppResult<Json<Value>> {
    let (trip, open) = chat_access(&st, &u, id).await?;
    let rows: Vec<(i64, Option<i64>, Option<String>, String, String)> = sqlx::query_as(
        "SELECT m.id, m.user_id, u.name, m.body, m.created_at FROM ride_messages m LEFT JOIN users u ON u.id=m.user_id
         WHERE m.trip_id=$1 AND m.id > $2 ORDER BY m.id LIMIT 300",
    )
    .bind(trip)
    .bind(q.after.unwrap_or(0))
    .fetch_all(&st.db)
    .await?;
    let members: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT u.name FROM rides r JOIN users u ON u.id = r.passenger_id OR u.id = r.driver_id
         WHERE r.trip_id=$1 AND r.status NOT IN ('cancelled')",
    )
    .bind(trip)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!({
        "open": open,
        "members": members.into_iter().map(|m| m.0).collect::<Vec<_>>(),
        "messages": rows.into_iter().map(|(id, uid, name, body, at)| json!({"id": id, "user_id": uid, "name": name, "body": body, "created_at": at, "mine": uid == Some(u.id)})).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct MsgIn {
    body: String,
}

async fn post_message(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<MsgIn>,
) -> AppResult<Json<Value>> {
    let (trip, open) = chat_access(&st, &u, id).await?;
    if !open {
        return Err(bad(u.lang, ErrorKey::GroupChatClosed));
    }
    let body = req.body.trim();
    if body.is_empty() || body.chars().count() > 1000 {
        return Err(bad(u.lang, ErrorKey::InvalidMessageLength));
    }
    sqlx::query(
        "INSERT INTO ride_messages (trip_id, user_id, body, created_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(trip)
    .bind(u.id)
    .bind(body)
    .bind(now())
    .execute(&st.db)
    .await?;
    notify_trip(&st.db, trip).await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- 交通组 (staff) ----------------

async fn staff_approve(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    let d = r
        .driver_id
        .ok_or_else(|| bad(u.lang, ErrorKey::NoDriverYet))?;
    let (load, cap) = driver_load(&st, d, &r.pickup_at, Some(id)).await?;
    if load + r.seats > cap {
        return Err(bad(u.lang, ErrorKey::NotEnoughDriverSeats { load, cap }));
    }
    set_status(&st, id, &["pending_approval"], "confirmed", u.lang).await?;
    sqlx::query("UPDATE rides SET approved_by=$1, needs_attention=0 WHERE id=$2")
        .bind(u.id)
        .bind(id)
        .execute(&st.db)
        .await?;
    let r = load_ride(&st, u.lang, id).await?;
    let trip = assign_trip(&st, &r).await?;
    announce_confirmed(&st, &r, trip, false).await;
    audit(&st.db, Some(u.id), "ride.approve", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

/// 退回: send back to the pool (e.g. conflict or unsuitable match).
async fn staff_return(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReasonReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    set_status(
        &st,
        id,
        &[
            "pending_approval",
            "pending_passenger",
            "pending_driver_confirm",
        ],
        "pending_driver",
        u.lang,
    )
    .await?;
    let reason = req.reason.unwrap_or_default();
    sqlx::query("UPDATE rides SET driver_id=NULL, trip_id=NULL, attention_reason=$1 WHERE id=$2")
        .bind(format!("交通组退回：{reason}"))
        .bind(id)
        .execute(&st.db)
        .await?;
    notify(
        &st.db,
        r.passenger_id,
        ("接送安排需重新配对", "Your ride needs to be re-matched"),
        (&reason, &reason),
        &format!("/rides/{id}"),
    )
    .await;
    if let Some(d) = r.driver_id {
        let line = format!("{} · {}", r.p_name, reason);
        notify(
            &st.db,
            d,
            (
                "接送安排已被交通组退回",
                "The ride was returned by transport staff",
            ),
            (&line, &line),
            "/drive",
        )
        .await;
    }
    audit(&st.db, Some(u.id), "ride.return", "ride", Some(id), reason).await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct ReassignReq {
    driver_id: i64,
}

async fn staff_reassign(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReassignReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    let (can_drive,): (bool,) = sqlx::query_as(&format!(
        "SELECT {} FROM users u WHERE u.id=$1 AND u.status='active'",
        perms::sql_holds("u", Perm(Page::Drive, Act::Create))
    ))
    .bind(req.driver_id)
    .fetch_optional(&st.db)
    .await?
    .ok_or_else(|| bad(u.lang, ErrorKey::DriverNotFound))?;
    if !can_drive {
        return Err(bad(u.lang, ErrorKey::MemberNotVolunteerDriver));
    }
    if req.driver_id == r.passenger_id {
        return Err(bad(u.lang, ErrorKey::DriverCannotBePassenger));
    }
    let (load, cap) = driver_load(&st, req.driver_id, &r.pickup_at, Some(id)).await?;
    if load + r.seats > cap {
        return Err(bad(
            u.lang,
            ErrorKey::NotEnoughDriverSeatsReassign { load, cap },
        ));
    }
    set_status(
        &st,
        id,
        &[
            "pending_driver",
            "pending_passenger",
            "pending_approval",
            "pending_driver_confirm",
            "confirmed",
        ],
        "pending_driver_confirm",
        u.lang,
    )
    .await?;
    sqlx::query("UPDATE rides SET driver_id=$1, trip_id=NULL, approved_by=$2, needs_attention=1, attention_reason='等待新司机确认' WHERE id=$3")
        .bind(req.driver_id)
        .bind(u.id)
        .bind(id)
        .execute(&st.db)
        .await?;
    if let (Some(old), Some(t)) = (r.driver_id, r.trip_id) {
        if old != req.driver_id {
            let line = format!("{} · {}", r.p_name, r.pickup_at.replace('T', " "));
            notify(
                &st.db,
                old,
                ("接送已改派", "Ride reassigned"),
                (&line, &line),
                "/drive",
            )
            .await;
            system_message(
                &st.db,
                t,
                &format!("交通组已将 {} 的接送改派给其他司机。", r.p_name),
                &format!(
                    "Transport staff have reassigned {}'s ride to another driver.",
                    r.p_name
                ),
            )
            .await;
        }
    }
    notify(
        &st.db,
        req.driver_id,
        (
            "交通组为您安排了接送",
            "Transport staff arranged a ride for you",
        ),
        (
            &format!(
                "{} · {} · 请确认",
                r.pickup_at.replace('T', " "),
                r.area_name
            ),
            &format!(
                "{} · {} · please confirm",
                r.pickup_at.replace('T', " "),
                r.area_name
            ),
        ),
        "/drive",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "ride.reassign",
        "ride",
        Some(id),
        format!("driver {:?} → {}", r.driver_id, req.driver_id),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

/// 没人接单 → 按政策批准 Grab 替代交通 (passenger may then claim reimbursement).
async fn staff_alt_transport(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReasonReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    set_status(
        &st,
        id,
        &[
            "pending_driver",
            "pending_passenger",
            "pending_approval",
            "pending_driver_confirm",
            "confirmed",
        ],
        "alt_transport",
        u.lang,
    )
    .await?;
    let note = req
        .reason
        .unwrap_or_else(|| "已批准使用 Grab 替代交通".into());
    sqlx::query("UPDATE rides SET driver_id=NULL, needs_attention=0, attention_reason=$1, approved_by=$2 WHERE id=$3")
        .bind(&note)
        .bind(u.id)
        .bind(id)
        .execute(&st.db)
        .await?;
    notify(
        &st.db,
        r.passenger_id,
        ("已批准替代交通", "Alternative transport approved"),
        (
            &format!("{note}。行程后可上传 Grab 收据申请报销。"),
            &format!(
                "{note}. After the trip you can upload your Grab receipt to claim reimbursement."
            ),
        ),
        "/reimbursements/new",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "ride.alt_transport",
        "ride",
        Some(id),
        note,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct AdminRidesQuery {
    date: Option<String>,
    status: Option<String>,
    attention: Option<bool>,
}

async fn admin_rides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AdminRidesQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::View)?;
    let mut conds = vec!["1=1".to_string()];
    let mut n = 1;
    if q.date.is_some() {
        conds.push(format!("substr(r.pickup_at,1,10) = ${n}"));
        n += 1;
    }
    if q.status.is_some() {
        conds.push(format!("r.status = ${n}"));
    }
    if q.attention.unwrap_or(false) {
        conds.push(
            "(r.needs_attention = 1 OR (r.status='pending_driver' AND r.pickup_at::timestamp - LOCALTIMESTAMP < interval '1 day'))".to_string(),
        );
    }
    let sql = format!(
        "{RIDE_SELECT} WHERE {} ORDER BY r.pickup_at LIMIT 300",
        conds.join(" AND ")
    );
    let mut query = sqlx::query_as::<_, RideRow>(&sql);
    if let Some(d) = &q.date {
        query = query.bind(d);
    }
    if let Some(s) = &q.status {
        query = query.bind(s);
    }
    let rows = query.fetch_all(&st.db).await?;
    Ok(Json(json!(rows
        .iter()
        .map(|r| ride_json(r, &u))
        .collect::<Vec<_>>())))
}

async fn admin_live(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminLive, Act::View)?;
    let rows: Vec<(i64, String, Option<f64>, Option<f64>, Option<String>, i64, String)> = sqlx::query_as(
        "SELECT d.id, d.name, l.lat, l.lng, l.updated_at, COUNT(r.id), MIN(r.pickup_at)
         FROM rides r JOIN users d ON d.id = r.driver_id LEFT JOIN driver_locations l ON l.driver_id = d.id
         WHERE r.status = 'in_progress' GROUP BY d.id, l.driver_id",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!({
        "max": st.cfg.max_live_drivers,
        "drivers": rows.into_iter().map(|(id, name, lat, lng, at, n, first)| json!({
            "driver_id": id, "name": name, "lat": lat, "lng": lng, "updated_at": at, "rides": n, "first_pickup": first,
        })).collect::<Vec<_>>(),
    })))
}

async fn admin_drivers(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AfterQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::View)?;
    let _ = q;
    let rows: Vec<(i64, String, i64, i64, Option<String>, Option<String>, Option<i64>, Option<String>)> = sqlx::query_as(&format!(
        "SELECT u.id, u.name, u.carpool_enabled, u.spare_seats, v.model, v.plate, v.seats,
                (SELECT string_agg(a.name, ', ') FROM driver_areas da JOIN areas a ON a.id = da.area_id WHERE da.user_id = u.id)
         FROM users u LEFT JOIN vehicles v ON v.user_id = u.id WHERE {} ORDER BY u.name",
        perms::sql_holds("u", Perm(Page::Drive, Act::Create))
    ))
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, name, carpool, spare, model, plate, seats, areas)| json!({
            "id": id, "name": name, "carpool_enabled": carpool == 1, "spare_seats": spare,
            "vehicle": model.map(|m| json!({"model": m, "plate": plate, "seats": seats})), "areas": areas.unwrap_or_default(),
        }))
        .collect::<Vec<_>>())))
}
