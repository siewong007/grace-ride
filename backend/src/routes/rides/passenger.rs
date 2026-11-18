//! Passenger ride booking, listing, detail, cancellation, and offer response.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, AppResult};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{
    audit, norm_dt, notify, notify_perm, now, require, split_cursor, system_message, ErrorKey,
};

use super::driver::driver_load;
use super::{can_view, load_ride, ride_json, set_status, traffic_can, RideRow, RIDE_SELECT};

// ---------------- Passenger ----------------

#[derive(Deserialize)]
pub(super) struct CreateRide {
    pickup_at: String,
    pickup_area_id: i64,
    pickup_address: String,
    destination: String,
    seats: Option<i64>,
    assistance: Option<String>,
    notes: Option<String>,
    event_id: Option<i64>,
    is_proxy: Option<bool>,
    rider_name: Option<String>,
    rider_phone: Option<String>,
    rider_relationship: Option<String>,
    save_proxy_rider: Option<bool>,
    leg: Option<String>,
    return_pickup_at: Option<String>,
    return_pickup_area_id: Option<i64>,
    return_pickup_address: Option<String>,
    return_destination: Option<String>,
}

/// Normalize UI assistance chips or free text into the canonical DB enum (`'' | wheelchair |
/// stroller | oxygen | other`) while preserving richer descriptive details in `notes`.
pub(crate) fn normalize_assistance(raw: &str, notes: &str) -> (String, String) {
    let trimmed = raw.trim();
    let clean_notes = notes.trim();
    if trimmed.is_empty() {
        return (String::new(), clean_notes.to_string());
    }
    let lower = trimmed.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "wheelchair" | "stroller" | "oxygen" | "other"
    ) {
        return (lower, clean_notes.to_string());
    }
    let canonical = if lower.contains("wheelchair") || trimmed.contains("轮椅") {
        "wheelchair"
    } else if lower.contains("stroller")
        || lower.contains("child seat")
        || lower.contains("child_seat")
        || lower.contains("infant")
        || lower.contains("baby")
        || trimmed.contains("婴儿")
        || trimmed.contains("儿童座椅")
        || trimmed.contains("安全座椅")
    {
        "stroller"
    } else if lower.contains("oxygen") || trimmed.contains("氧") {
        "oxygen"
    } else {
        "other"
    };
    let merged_notes = if clean_notes.is_empty() {
        trimmed.to_string()
    } else if clean_notes.contains(trimmed) {
        clean_notes.to_string()
    } else {
        format!("{trimmed} · {clean_notes}")
    };
    (canonical.to_string(), merged_notes)
}

pub(super) async fn create_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CreateRide>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::Create)?;
    let at = norm_dt(&req.pickup_at).ok_or_else(|| bad(u.lang, ErrorKey::InvalidPickupTime))?;
    if at.as_str() < &now()[..16] {
        return Err(bad(u.lang, ErrorKey::PickupTimeInPast));
    }
    let leg = req
        .leg
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("outbound")
        .to_string();
    if leg != "outbound" && leg != "return" {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let ret_at = match req
        .return_pickup_at
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => {
            if leg == "return" {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
            let parsed = norm_dt(raw).ok_or_else(|| bad(u.lang, ErrorKey::InvalidPickupTime))?;
            if parsed <= at {
                return Err(bad(u.lang, ErrorKey::InvalidPickupTime));
            }
            Some(parsed)
        }
        None => None,
    };

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
    let ret_area_id = req.return_pickup_area_id.unwrap_or(req.pickup_area_id);
    if ret_at.is_some() && ret_area_id != req.pickup_area_id {
        let ret_area: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM areas WHERE id=$1 AND active=1")
                .bind(ret_area_id)
                .fetch_optional(&st.db)
                .await?;
        ret_area.ok_or_else(|| bad(u.lang, ErrorKey::PickupAreaRequired))?;
    }

    let mut as_group: Option<i64> = None;
    // The event checks and the insert share one transaction holding a share lock on the
    // event row: cancelling the event updates that row, so it either finishes first (and
    // the booking is refused) or waits and then cancels this new ride with the rest.
    let mut tx = st.db.begin().await?;
    if let Some(eid) = req.event_id {
        if !crate::routes::events::can_see_event(&st, &u, eid).await? {
            return Err(forbidden(u.lang, ErrorKey::CannotViewEvent));
        }
        let event_status: Option<(String,)> =
            sqlx::query_as("SELECT status FROM events WHERE id=$1 FOR SHARE")
                .bind(eid)
                .fetch_optional(&mut *tx)
                .await?;
        if event_status.as_ref().map(|(s,)| s.as_str()) != Some("published") {
            return Err(bad(u.lang, ErrorKey::EventNotOpen));
        }
        // A ride to an event goes with a place at it, and carries the group the member
        // joined as so the right drivers are asked.
        let signup: Option<(Option<i64>,)> = sqlx::query_as(
            "SELECT as_group_id FROM event_signups WHERE event_id=$1 AND user_id=$2 AND status='going'",
        )
        .bind(eid)
        .bind(u.id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((g,)) = signup else {
            return Err(bad(u.lang, ErrorKey::EventRideNeedsSignup));
        };
        as_group = g;
        // One own ride per leg per event: a double tap or a second tab must not book two cars.
        // The lock serialises two submissions by the same member.
        sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(eid as i32)
            .bind(u.id as i32)
            .execute(&mut *tx)
            .await?;
        if !req.is_proxy.unwrap_or(false)
            && req
                .rider_name
                .as_deref()
                .is_none_or(|s| s.trim().is_empty())
        {
            let (dup,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM rides WHERE event_id=$1 AND passenger_id=$2 AND is_proxy=0 AND leg=$3
                 AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed','in_progress')",
            )
            .bind(eid)
            .bind(u.id)
            .bind(&leg)
            .fetch_one(&mut *tx)
            .await?;
            if dup > 0 {
                return Err(bad(u.lang, ErrorKey::DuplicateEventRide));
            }
            if ret_at.is_some() {
                let (dup_ret,): (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM rides WHERE event_id=$1 AND passenger_id=$2 AND is_proxy=0 AND leg='return'
                     AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed','in_progress')",
                )
                .bind(eid)
                .bind(u.id)
                .fetch_one(&mut *tx)
                .await?;
                if dup_ret > 0 {
                    return Err(bad(u.lang, ErrorKey::DuplicateEventRide));
                }
            }
        }
    }

    let is_proxy = req.is_proxy.unwrap_or(false)
        || req
            .rider_name
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
    let rider_name = req
        .rider_name
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let rider_phone = req
        .rider_phone
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let rider_relationship = req
        .rider_relationship
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if is_proxy && rider_name.is_empty() {
        return Err(bad(u.lang, ErrorKey::RiderNameRequired));
    }
    let is_proxy_int = if is_proxy { 1i64 } else { 0i64 };
    let (assistance, notes) = normalize_assistance(
        req.assistance.as_deref().unwrap_or_default(),
        req.notes.as_deref().unwrap_or_default(),
    );

    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, notes, is_proxy, rider_name, rider_phone, rider_relationship, created_at, updated_at, as_group_id, leg)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17) RETURNING id",
    )
    .bind(u.id)
    .bind(req.event_id)
    .bind(&at)
    .bind(req.pickup_area_id)
    .bind(req.pickup_address.trim())
    .bind(req.destination.trim())
    .bind(seats)
    .bind(&assistance)
    .bind(&notes)
    .bind(is_proxy_int)
    .bind(&rider_name)
    .bind(&rider_phone)
    .bind(&rider_relationship)
    .bind(now())
    .bind(now())
    .bind(as_group)
    .bind(&leg)
    .fetch_one(&mut *tx)
    .await?;

    let mut return_ride_id: Option<i64> = None;
    if let Some(ref rat) = ret_at {
        let ret_addr = req
            .return_pickup_address
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(req.destination.trim());
        let ret_dest = req
            .return_destination
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(req.pickup_address.trim());
        let (rid,): (i64,) = sqlx::query_as(
            "INSERT INTO rides (passenger_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, notes, is_proxy, rider_name, rider_phone, rider_relationship, created_at, updated_at, as_group_id, leg, paired_ride_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,'return',$17) RETURNING id",
        )
        .bind(u.id)
        .bind(req.event_id)
        .bind(rat)
        .bind(ret_area_id)
        .bind(ret_addr)
        .bind(ret_dest)
        .bind(seats)
        .bind(&assistance)
        .bind(&notes)
        .bind(is_proxy_int)
        .bind(&rider_name)
        .bind(&rider_phone)
        .bind(&rider_relationship)
        .bind(now())
        .bind(now())
        .bind(as_group)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("UPDATE rides SET paired_ride_id=$1 WHERE id=$2")
            .bind(rid)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        return_ride_id = Some(rid);
    }
    tx.commit().await?;

    if is_proxy && req.save_proxy_rider.unwrap_or(false) && !rider_name.is_empty() {
        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM proxy_riders WHERE user_id=$1 AND name=$2")
                .bind(u.id)
                .bind(&rider_name)
                .fetch_optional(&st.db)
                .await?;
        if exists.is_none() {
            let _ = sqlx::query(
                "INSERT INTO proxy_riders (user_id, name, phone, relationship, notes, area_id, pickup_address, gate_notes, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            )
            .bind(u.id)
            .bind(&rider_name)
            .bind(&rider_phone)
            .bind(&rider_relationship)
            .bind(&notes)
            .bind(req.pickup_area_id)
            .bind(req.pickup_address.trim())
            .bind(&notes)
            .bind(now())
            .bind(now())
            .execute(&st.db)
            .await;
        }
    }

    audit(&st.db, Some(u.id), "ride.create", "ride", Some(id), &at).await;
    let rider_label = if is_proxy && !rider_name.is_empty() {
        format!("{} ({} 代订)", rider_name, u.name)
    } else {
        u.name.clone()
    };
    notify_area_drivers(
        &st,
        req.pickup_area_id,
        as_group,
        u.id,
        &at,
        &rider_label,
        seats,
    )
    .await?;
    if let (Some(rid), Some(ref rat)) = (return_ride_id, ret_at) {
        audit(&st.db, Some(u.id), "ride.create", "ride", Some(rid), rat).await;
        notify_area_drivers(&st, ret_area_id, as_group, u.id, rat, &rider_label, seats).await?;
    }
    Ok(Json(json!({ "id": id, "return_ride_id": return_ride_id })))
}

/// Let drivers who serve the area know there's a new request. For an event ride matched
/// to a group, only drivers in that group or a group above it are told.
pub(crate) async fn notify_area_drivers(
    st: &AppState,
    area: i64,
    match_group: Option<i64>,
    passenger: i64,
    at: &str,
    rider_label: &str,
    seats: i64,
) -> AppResult<()> {
    notify_area_drivers_pool(&st.db, area, match_group, passenger, at, rider_label, seats).await
}

pub(crate) async fn notify_area_drivers_pool(
    db: &PgPool,
    area: i64,
    match_group: Option<i64>,
    passenger: i64,
    at: &str,
    rider_label: &str,
    seats: i64,
) -> AppResult<()> {
    let drivers: Vec<(i64,)> = sqlx::query_as(
        "WITH RECURSIVE up AS (
             SELECT $3::BIGINT AS id WHERE $3::BIGINT IS NOT NULL
             UNION
             SELECT g.parent_id FROM groups g JOIN up ON g.id = up.id WHERE g.parent_id IS NOT NULL
         )
         SELECT da.user_id FROM driver_areas da JOIN users u ON u.id = da.user_id
         WHERE da.area_id=$1 AND u.status='active' AND u.id != $2
           AND ($3::BIGINT IS NULL OR u.id IN (SELECT gm.user_id FROM group_members gm WHERE gm.group_id IN (SELECT id FROM up)))",
    )
    .bind(area)
    .bind(passenger)
    .bind(match_group)
    .fetch_all(db)
    .await?;
    let when = format!("{} · {} · {} 位", at.replace('T', " "), rider_label, seats);
    for (d,) in drivers {
        notify(
            db,
            d,
            ("新的接送报名", "New ride request"),
            (&when, &when),
            "/drive",
        )
        .await;
    }
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct MineQuery {
    scope: Option<String>, // upcoming | past
    before: Option<String>,
    limit: Option<i64>,
}

pub(super) async fn my_rides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<MineQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::View)?;
    let past = q.scope.as_deref() == Some("past");
    if past && (q.before.is_some() || q.limit.is_some()) {
        let limit = q.limit.unwrap_or(30).clamp(1, 100);
        let mut conds = vec![
            "r.passenger_id=$1".to_string(),
            "r.status IN ('completed','cancelled','no_show','alt_transport')".to_string(),
        ];
        let cursor = q.before.as_deref().map(split_cursor);
        let mut n = 2;
        if let Some((_, Some(_))) = cursor {
            conds.push(format!(
                "(r.pickup_at < ${n} OR (r.pickup_at = ${n} AND r.id < ${}))",
                n + 1
            ));
            n += 2;
        } else if cursor.is_some() {
            conds.push(format!("r.pickup_at < ${n}"));
            n += 1;
        }
        let sql = format!(
            "{RIDE_SELECT} WHERE {} ORDER BY r.pickup_at DESC, r.id DESC LIMIT ${n}",
            conds.join(" AND ")
        );
        let mut query = sqlx::query_as::<_, RideRow>(&sql).bind(u.id);
        if let Some((ts, Some(bid))) = cursor {
            query = query.bind(ts).bind(bid);
        } else if let Some((ts, None)) = cursor {
            query = query.bind(ts);
        }
        query = query.bind(limit + 1);
        let mut rows = query.fetch_all(&st.db).await?;
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = if more {
            rows.last().map(|r| format!("{}|{}", r.pickup_at, r.id))
        } else {
            None
        };
        return Ok(Json(json!({
            "items": rows.iter().map(|r| ride_json(r, &u)).collect::<Vec<_>>(),
            "next_before": next_before,
        })));
    }

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

async fn is_active_caregiver_for_ride(st: &AppState, r: &RideRow, user_id: i64) -> AppResult<bool> {
    if user_id == r.passenger_id {
        return Ok(false);
    }
    let hit: Option<i64> = sqlx::query_scalar(
        "SELECT cg.id
         FROM users cg
         WHERE cg.id = $1 AND cg.status = 'active'
           AND (
               EXISTS (SELECT 1 FROM users p WHERE p.id = $2 AND p.caregiver_user_id = $1)
               OR ($3 = 1 AND $4 <> '' AND EXISTS (
                   SELECT 1 FROM proxy_riders pr
                   WHERE pr.user_id = $2 AND pr.name = $4 AND pr.caregiver_user_id = $1
               ))
           )",
    )
    .bind(user_id)
    .bind(r.passenger_id)
    .bind(r.is_proxy)
    .bind(&r.rider_name)
    .fetch_optional(&st.db)
    .await?;
    Ok(hit.is_some())
}

pub(super) async fn get_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if !can_view(&r, &u) && !is_active_caregiver_for_ride(&st, &r, u.id).await? {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let mut v = ride_json(&r, &u);
    if traffic_can(&u, &r, Act::View) {
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
pub(super) struct ReasonReq {
    pub(super) reason: Option<String>,
}

pub(super) async fn cancel_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    body: Option<Json<ReasonReq>>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    let own = r.passenger_id == u.id && u.can(Page::Rides, Act::Delete);
    if !own && !traffic_can(&u, &r, Act::Delete) {
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
    if let (Some(d), Some(t)) = (r.driver_id, r.trip_id) {
        st.routing.invalidate(d, t).await;
    }
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
    if r.leg == "outbound" {
        if let Some(paired_id) = r.paired_ride_id {
            let _ = sqlx::query(
                "UPDATE rides
                 SET needs_attention=1,
                     attention_reason=COALESCE(NULLIF(attention_reason,''), '去程已取消，请确认返程接送 / Outbound ride cancelled; confirm return leg'),
                     updated_at=$1
                 WHERE id=$2 AND leg='return' AND driver_id IS NOT NULL
                   AND status NOT IN ('completed','cancelled','no_show','alt_transport')",
            )
            .bind(now())
            .bind(paired_id)
            .execute(&st.db)
            .await;
        }
    }
    audit(&st.db, Some(u.id), "ride.cancel", "ride", Some(id), reason).await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct RespondReq {
    accept: bool,
}

/// 乘客接受 / 婉拒 司机邀请
pub(super) async fn passenger_respond(
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
        let accepted = sqlx::query(
            "UPDATE rides SET status='pending_approval', updated_at=$1
             WHERE id=$2 AND driver_id IS NOT DISTINCT FROM $3 AND status='pending_passenger'",
        )
        .bind(now())
        .bind(id)
        .bind(r.driver_id)
        .execute(&st.db)
        .await?;
        if accepted.rows_affected() == 0 {
            return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
        }
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
        // Reopen and let the driver go in one statement (see `driver_withdraw`).
        let declined = sqlx::query(
            "UPDATE rides SET status='pending_driver', driver_id=NULL, updated_at=$1
             WHERE id=$2 AND driver_id IS NOT DISTINCT FROM $3 AND status='pending_passenger'",
        )
        .bind(now())
        .bind(id)
        .bind(r.driver_id)
        .execute(&st.db)
        .await?;
        if declined.rows_affected() == 0 {
            return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
        }
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
