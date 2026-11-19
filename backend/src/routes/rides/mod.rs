//! 接送配对 workflow:
//! 乘客报名 → 司机选择 → 乘客接受 → 交通组批准 → 行程中 → 已完成
//! plus 临时变动 (driver withdraws, reassignment, cancellations, no-shows, Grab alternative),
//! per-trip chat and live location while a trip is in progress.

use axum::{
    extract::{Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{conflict, not_found, AppResult};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{now, ErrorKey, Lang};

mod chat;
mod driver;
mod driver_profile;
mod location;
mod passenger;
mod places;
mod proxy;
mod readiness;
mod schedules;
mod staff;

pub(crate) use driver::seat_capacity;
pub(crate) use passenger::{normalize_assistance, notify_area_drivers, notify_area_drivers_pool};
pub(crate) use readiness::{apply_readiness, ReadinessReq};
pub use schedules::{event_moved, process_recurring_schedules};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/areas", get(list_areas))
        .route("/places", get(search_places))
        .route(
            "/proxy-riders",
            get(proxy::list_proxy_riders).post(proxy::create_proxy_rider),
        )
        .route(
            "/proxy-riders/{id}",
            put(proxy::update_proxy_rider).delete(proxy::delete_proxy_rider),
        )
        .route(
            "/saved-places",
            get(places::list_saved_places).post(places::create_saved_place),
        )
        .route(
            "/saved-places/{id}",
            put(places::update_saved_place).delete(places::delete_saved_place),
        )
        .route(
            "/recurring-schedules",
            get(schedules::list_recurring_schedules).post(schedules::create_recurring_schedule),
        )
        .route(
            "/recurring-schedules/{id}",
            put(schedules::update_recurring_schedule).delete(schedules::delete_recurring_schedule),
        )
        .route(
            "/recurring-schedules/{id}/generate",
            post(schedules::generate_recurring_schedule),
        )
        .route(
            "/rides",
            get(passenger::my_rides).post(passenger::create_ride),
        )
        .route("/rides/{id}", get(passenger::get_ride))
        .route("/rides/{id}/cancel", post(passenger::cancel_ride))
        .route("/rides/{id}/respond", post(passenger::passenger_respond))
        .route("/rides/{id}/readiness", post(readiness::post_readiness))
        .route(
            "/rides/{id}/driver-delay",
            post(readiness::post_driver_delay),
        )
        .route(
            "/rides/{id}/arrived-gate",
            post(readiness::post_arrived_gate),
        )
        .route("/rides/{id}/claim", post(driver::driver_claim))
        .route("/rides/{id}/withdraw", post(driver::driver_withdraw))
        .route(
            "/rides/{id}/confirm-assignment",
            post(driver::driver_confirm_assignment),
        )
        .route("/rides/{id}/start", post(driver::start_ride))
        .route("/rides/{id}/pickup", post(driver::pickup))
        .route("/rides/{id}/no-show", post(driver::no_show))
        .route("/rides/{id}/complete", post(driver::complete))
        .route("/rides/{id}/approve", post(staff::staff_approve))
        .route("/rides/{id}/return", post(staff::staff_return))
        .route("/rides/{id}/reassign", post(staff::staff_reassign))
        .route(
            "/rides/{id}/alt-transport",
            post(staff::staff_alt_transport),
        )
        .route(
            "/rides/{id}/messages",
            get(chat::list_messages).post(chat::post_message),
        )
        .route("/rides/{id}/messages/read", post(chat::mark_read))
        .route("/rides/{id}/messages/{mid}/reactions", post(chat::react))
        .route("/rides/{id}/location", get(location::ride_location))
        .route(
            "/rides/{id}/passenger-location",
            post(location::post_passenger_location).delete(location::stop_passenger_location),
        )
        .route("/driver/requests", get(driver_profile::driver_requests))
        .route("/driver/schedule", get(driver_profile::driver_schedule))
        .route("/driver/route", get(driver_profile::driver_route))
        .route(
            "/driver/profile",
            get(driver_profile::get_driver_profile).put(driver_profile::put_driver_profile),
        )
        .route(
            "/driver/location",
            post(location::post_location).delete(location::stop_location),
        )
        .route("/admin/rides", get(staff::admin_rides))
        .route("/admin/live", get(staff::admin_live))
        .route("/admin/drivers", get(staff::admin_drivers))
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
    pub d_photo_file_id: Option<i64>,
    pub d_avatar_color: Option<String>,
    pub d_avatar_icon: Option<String>,
    pub v_model: Option<String>,
    pub v_color: Option<String>,
    pub v_plate: Option<String>,
    pub v_type: Option<String>,
    pub v_wheelchair_ok: Option<i64>,
    pub v_low_step: Option<i64>,
    pub v_child_seat: Option<i64>,
    pub distance_km: Option<i64>,
    pub reimbursement_amount_cents: Option<i64>,
    pub event_title: Option<String>,
    pub readiness_status: Option<String>,
    pub readiness_updated_at: Option<String>,
    pub delay_minutes: Option<i64>,
    pub arrived_at_gate_at: Option<String>,
    pub is_proxy: i64,
    pub rider_name: String,
    pub rider_phone: String,
    pub rider_relationship: String,
    pub leg: String,
    pub paired_ride_id: Option<i64>,
    /// The group the rider joined the event as (visitors: the visitor group).
    pub as_group_id: Option<i64>,
    pub as_group_name: Option<String>,
    /// The group drivers are matched on: `as_group_id`, or the host's group for a visitor.
    pub match_group_id: Option<i64>,
    pub event_created_by: Option<i64>,
    pub event_is_all: bool,
    pub event_group_ids: Vec<i64>,
    pub event_mileage_claimable: i64,
}

pub(super) const RIDE_SELECT: &str = "SELECT r.id, r.passenger_id, r.driver_id, r.event_id, r.trip_id, r.pickup_at, r.pickup_area_id,
    r.pickup_address, r.destination, r.seats, r.assistance, r.notes, r.status, r.needs_attention, r.attention_reason,
    r.started_at, r.picked_up_at, r.completed_at, r.distance_km, r.reimbursement_amount_cents, r.created_at, r.updated_at,
    a.name AS area_name, p.name AS p_name, p.phone AS p_phone, d.name AS d_name, d.phone AS d_phone,
    d.photo_file_id AS d_photo_file_id, d.avatar_color AS d_avatar_color, d.avatar_icon AS d_avatar_icon,
    v.model AS v_model, v.color AS v_color, v.plate AS v_plate, v.vehicle_type AS v_type,
    v.wheelchair_ok AS v_wheelchair_ok, v.low_step AS v_low_step, v.child_seat AS v_child_seat,
    e.title AS event_title,
    r.readiness_status, r.readiness_updated_at, r.delay_minutes, r.arrived_at_gate_at,
    r.is_proxy, r.rider_name, r.rider_phone, r.rider_relationship,
    r.leg, r.paired_ride_id,
    r.as_group_id, ag.name AS as_group_name,
    COALESCE((SELECT vp.host_group_id FROM visitor_passes vp WHERE vp.ride_id = r.id ORDER BY vp.id DESC LIMIT 1), r.as_group_id) AS match_group_id,
    e.created_by AS event_created_by,
    COALESCE((SELECT bool_or(g.is_all = 1) FROM event_groups eg JOIN groups g ON g.id = eg.group_id WHERE eg.event_id = r.event_id), false) AS event_is_all,
    COALESCE((SELECT array_agg(eg.group_id) FROM event_groups eg WHERE eg.event_id = r.event_id), '{}'::BIGINT[]) AS event_group_ids,
    COALESCE(e.mileage_claimable, 0) AS event_mileage_claimable
    FROM rides r
    JOIN areas a ON a.id = r.pickup_area_id
    JOIN users p ON p.id = r.passenger_id
    LEFT JOIN users d ON d.id = r.driver_id
    LEFT JOIN vehicles v ON v.user_id = r.driver_id
    LEFT JOIN events e ON e.id = r.event_id
    LEFT JOIN groups ag ON ag.id = r.as_group_id";

pub async fn load_ride(st: &AppState, lang: Lang, id: i64) -> AppResult<RideRow> {
    sqlx::query_as::<_, RideRow>(crate::dyn_sql!("{RIDE_SELECT} WHERE r.id = $1"))
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
    let staff = traffic_can(u, r, Act::View);
    let approved = matches!(r.status.as_str(), "confirmed" | "in_progress");
    let show_address = is_passenger || staff || (is_driver && approved);
    let show_contact = is_passenger || staff || (is_driver && approved);
    let is_proxy = r.is_proxy == 1;
    let proxy = if is_proxy {
        Some(json!({
            "is_proxy": true,
            "rider_name": r.rider_name,
            "rider_phone": if show_contact { Some(r.rider_phone.clone()) } else { None },
            "rider_relationship": r.rider_relationship,
            "booker_name": r.p_name,
            "booker_phone": if show_contact { Some(r.p_phone.clone()) } else { None },
        }))
    } else {
        None
    };
    let driver = r.driver_id.map(|id| {
        json!({
            "id": id,
            "name": r.d_name,
            "photo_file_id": r.d_photo_file_id,
            "avatar_color": r.d_avatar_color,
            "avatar_icon": r.d_avatar_icon,
            "phone": if is_driver || staff || (is_passenger && approved) { r.d_phone.clone() } else { None },
            "vehicle": r.v_model.as_ref().map(|m| json!({
                "model": m,
                "color": r.v_color,
                "plate": r.v_plate,
                "type": r.v_type,
                "wheelchair_ok": r.v_wheelchair_ok.unwrap_or(0) == 1,
                "low_step": r.v_low_step.unwrap_or(0) == 1,
                "child_seat": r.v_child_seat.unwrap_or(0) == 1,
            })),
        })
    });
    json!({
        "id": r.id,
        "status": r.status,
        "leg": r.leg,
        "paired_ride_id": r.paired_ride_id,
        "pickup_at": r.pickup_at,
        "area": {"id": r.pickup_area_id, "name": r.area_name},
        "pickup_address": if show_address { Some(r.pickup_address.clone()) } else { None },
        "destination": if !show_address && r.leg == "return" {
            r.area_name.clone()
        } else {
            r.destination.clone()
        },
        "seats": r.seats,
        "assistance": r.assistance,
        "notes": r.notes,
        // Someone who may not see the event (an admin-assigned driver from outside its
        // groups, say) learns only that it is a church event.
        "event": r.event_id.map(|id| {
            let sees = is_passenger
                || staff
                || crate::audience::sees_event(u, r.event_created_by.unwrap_or(0), r.event_is_all, &r.event_group_ids);
            if sees {
                json!({"id": id, "title": r.event_title, "mileage_claimable": r.event_mileage_claimable == 1})
            } else {
                json!({"id": null, "title": null, "private": true})
            }
        }),
        "as_group": r.as_group_id.map(|id| json!({"id": id, "name": r.as_group_name})),
        "trip_id": r.trip_id,
        "distance_km": r.distance_km,
        "reimbursement_amount_cents": r.reimbursement_amount_cents,
        "readiness_status": r.readiness_status.as_deref().unwrap_or("unconfirmed"),
        "readiness_updated_at": r.readiness_updated_at,
        "delay_minutes": r.delay_minutes.unwrap_or(0),
        "arrived_at_gate_at": r.arrived_at_gate_at,
        "is_proxy": is_proxy,
        "proxy": proxy,
        "passenger": {
            "id": r.passenger_id,
            "name": if is_proxy && !r.rider_name.is_empty() {
                format!("{} (由 {} 代订)", r.rider_name, r.p_name)
            } else {
                r.p_name.clone()
            },
            "phone": if show_contact { Some(r.p_phone.clone()) } else { None },
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

pub(super) fn can_view(r: &RideRow, u: &CurrentUser) -> bool {
    r.passenger_id == u.id
        || r.driver_id == Some(u.id)
        || traffic_can(u, r, Act::View)
        || (u.can(Page::Drive, Act::View)
            && r.status == "pending_driver"
            && crate::audience::drives_for(u, r.match_group_id))
}

/// Global traffic staff (traffic dashboard holders) manage every event. A cell group
/// leader also holds the traffic tools, but only for events they created.
pub(super) fn traffic_scope_ok(u: &CurrentUser, event_created_by: Option<i64>) -> bool {
    u.traffic_is_global() || event_created_by == Some(u.id)
}

/// Traffic-page permission for one ride, honouring the event scope above.
pub(crate) fn traffic_can(u: &CurrentUser, r: &RideRow, act: Act) -> bool {
    u.can(Page::AdminRides, act) && traffic_scope_ok(u, r.event_created_by)
}

/// SQL condition limiting the traffic pages to events the caller created.
pub(super) fn traffic_owned_events_sql(u: &CurrentUser) -> String {
    format!(
        "(r.event_id IN (SELECT id FROM events WHERE created_by = {}))",
        u.id
    )
}

pub(super) async fn set_status(
    st: &AppState,
    id: i64,
    from: &[&str],
    to: &str,
    lang: Lang,
) -> AppResult<()> {
    let placeholders = (0..from.len())
        .map(|i| format!("${}", i + 4))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "UPDATE rides SET status=$1, updated_at=$2 WHERE id=$3 AND status IN ({placeholders})"
    );
    let mut q = sqlx::query(sqlx::AssertSqlSafe(&*sql))
        .bind(to)
        .bind(now())
        .bind(id);
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
    // Per member, and for the whole server: Photon bans by source IP, so it is the
    // total from this server that must stay modest.
    let minute = std::time::Duration::from_secs(60);
    if st
        .limiter
        .check(&format!("places:{}", u.id), 30, minute)
        .is_err()
        || st.limiter.check("places:all", 300, minute).is_err()
    {
        return Ok(Json(json!([])));
    }
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
