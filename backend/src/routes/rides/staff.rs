//! Transport-staff coordination endpoints: approve, return, reassign, alt-transport, and admin views.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, AppResult};
use crate::perms::{self, Act, Page, Perm};
use crate::state::AppState;
use crate::util::{audit, escape_like, notify, now, require, system_message, ErrorKey};

use super::chat::AfterQuery;
use super::driver::{announce_confirmed, assign_trip, driver_load_on};
use super::passenger::ReasonReq;
use super::{
    load_ride, ride_json, traffic_owned_events_sql, traffic_scope_ok, RideRow, RIDE_SELECT,
};

// ---------------- 交通组 (staff) ----------------

pub(super) async fn staff_approve(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let d = r
        .driver_id
        .ok_or_else(|| bad(u.lang, ErrorKey::NoDriverYet))?;
    // Same driver-row lock as `driver_claim` / `staff_reassign`, so two approvals for one
    // driver can't both pass the capacity check and overbook the car.
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(d)
        .fetch_optional(&mut *tx)
        .await?;
    let (load, cap) = driver_load_on(&mut tx, d, &r.pickup_at, Some(id)).await?;
    if load + r.seats > cap {
        return Err(bad(u.lang, ErrorKey::NotEnoughDriverSeats { load, cap }));
    }
    let approved = sqlx::query(
        "UPDATE rides SET status='confirmed', approved_by=$1, needs_attention=0, reminder_sent=0, updated_at=$2
         WHERE id=$3 AND driver_id=$4 AND status='pending_approval'",
    )
    .bind(u.id)
    .bind(now())
    .bind(id)
    .bind(d)
    .execute(&mut *tx)
    .await?;
    if approved.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
    assign_trip(&mut tx, &r).await?;
    tx.commit().await?;
    let r = load_ride(&st, u.lang, id).await?;
    let trip = r.trip_id.unwrap_or(id);
    announce_confirmed(&st, &r, trip, false).await;
    audit(&st.db, Some(u.id), "ride.approve", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

/// 退回: send back to the pool (e.g. conflict or unsuitable match).
pub(super) async fn staff_return(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReasonReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let reason = req.reason.unwrap_or_default();
    // One statement, and only while the ride still has the driver loaded above: as two
    // steps a driver could claim the reopened ride in between, and clearing the driver
    // afterwards left it waiting on a passenger reply with nobody driving.
    let returned = sqlx::query(
        "UPDATE rides SET status='pending_driver', driver_id=NULL, trip_id=NULL,
                attention_reason=$1, updated_at=$2
         WHERE id=$3 AND driver_id IS NOT DISTINCT FROM $4
           AND status IN ('pending_approval','pending_passenger','pending_driver_confirm')",
    )
    .bind(format!("交通组退回：{reason}"))
    .bind(now())
    .bind(id)
    .bind(r.driver_id)
    .execute(&st.db)
    .await?;
    if returned.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
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
pub(super) struct ReassignReq {
    driver_id: i64,
}

pub(super) async fn staff_reassign(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReassignReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
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
    // Transport staff may go outside the rider's group; that is recorded.
    if let Some(g) = r.match_group_id {
        let (_, scope) = crate::audience::group_scopes(&st.db, req.driver_id).await?;
        if !scope.contains(&g) {
            audit(
                &st.db,
                Some(u.id),
                "ride.assign_outside_group",
                "ride",
                Some(id),
                format!("driver {} is outside group {g}", req.driver_id),
            )
            .await;
        }
    }
    // Lock the new driver's row for the capacity check + reassignment, the same lock
    // `driver_claim` takes, so concurrent assignments to one driver serialise and the
    // second sees the first booking in `load` instead of overbooking the car.
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(req.driver_id)
        .fetch_optional(&mut *tx)
        .await?;
    let (load, cap) = driver_load_on(&mut tx, req.driver_id, &r.pickup_at, Some(id)).await?;
    if load + r.seats > cap {
        return Err(bad(
            u.lang,
            ErrorKey::NotEnoughDriverSeatsReassign { load, cap },
        ));
    }
    // One statement, and only while the ride still has the driver loaded above, so a
    // withdrawal or claim in between is not silently overwritten (or half-applied).
    let reassigned = sqlx::query(
        "UPDATE rides SET status='pending_driver_confirm', driver_id=$1, trip_id=NULL, approved_by=$2,
                needs_attention=1, attention_reason='等待新司机确认', updated_at=$3
         WHERE id=$4 AND driver_id IS NOT DISTINCT FROM $5
           AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')",
    )
    .bind(req.driver_id)
    .bind(u.id)
    .bind(now())
    .bind(id)
    .bind(r.driver_id)
    .execute(&mut *tx)
    .await?;
    if reassigned.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
    tx.commit().await?;
    if let Some(old) = r.driver_id.filter(|o| *o != req.driver_id) {
        let line = format!("{} · {}", r.p_name, r.pickup_at.replace('T', " "));
        notify(
            &st.db,
            old,
            ("接送已改派", "Ride reassigned"),
            (&line, &line),
            "/drive",
        )
        .await;
        if let Some(t) = r.trip_id {
            st.routing.invalidate(old, t).await;
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
pub(super) async fn staff_alt_transport(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReasonReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let note = req
        .reason
        .unwrap_or_else(|| "已批准使用 Grab 替代交通".into());
    // One statement, and only while the ride still has the driver loaded above, so the
    // driver who is told the ride was taken off them is the one it was taken from.
    let switched = sqlx::query(
        "UPDATE rides SET status='alt_transport', driver_id=NULL, trip_id=NULL, needs_attention=0,
                attention_reason=$1, approved_by=$2, updated_at=$3
         WHERE id=$4 AND driver_id IS NOT DISTINCT FROM $5
           AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')",
    )
    .bind(&note)
    .bind(u.id)
    .bind(now())
    .bind(id)
    .bind(r.driver_id)
    .execute(&st.db)
    .await?;
    if switched.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
    if let (Some(d), Some(t)) = (r.driver_id, r.trip_id) {
        st.routing.invalidate(d, t).await;
    }
    if let Some(driver_id) = r.driver_id {
        notify(
            &st.db,
            driver_id,
            (
                "接送已改为替代交通",
                "Ride changed to alternative transport",
            ),
            (
                &format!("{note}。此行程已从你的安排中移除。"),
                &format!("{note}. This ride has been removed from your assignments."),
            ),
            "/drive",
        )
        .await;
    }
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
pub(super) struct AdminRidesQuery {
    date: Option<String>,
    status: Option<String>,
    /// Passenger, rider, driver or destination, in any letter case.
    q: Option<String>,
    attention: Option<bool>,
    sort: Option<String>,
    dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}

pub(super) async fn admin_rides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AdminRidesQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::View)?;
    let mut conds = vec!["1=1".to_string()];
    // A cell group leader only sees the rides of events they created.
    if !u.traffic_is_global() {
        conds.push(traffic_owned_events_sql(&u));
    }
    let mut n = 1;
    if q.date.is_some() {
        // Same rows as `substr(pickup_at,1,10) = date`, written as a range so the pickup
        // index can serve it: every time on that day sorts between "dateT" and "dateU".
        conds.push(format!(
            "(r.pickup_at >= (${n} || 'T') AND r.pickup_at < (${n} || 'U'))"
        ));
        n += 1;
    }
    if q.status.is_some() {
        conds.push(format!("r.status = ${n}"));
        n += 1;
    }
    let search =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", escape_like(s)));
    if search.is_some() {
        conds.push(format!(
            "(p.name ILIKE ${n} ESCAPE '\\' OR r.rider_name ILIKE ${n} ESCAPE '\\' OR d.name ILIKE ${n} ESCAPE '\\' OR r.destination ILIKE ${n} ESCAPE '\\')"
        ));
        n += 1;
    }
    if q.attention.unwrap_or(false) {
        // Unmatched within a day. The cut-off comes from our own clock in the app's zone,
        // so it is safe to interpolate (and never depends on the session's time zone).
        let within_a_day = crate::util::in_hours(24);
        conds.push(format!(
            "((r.needs_attention = 1 AND r.status NOT IN ('cancelled','completed','no_show','alt_transport')) OR (r.status='pending_driver' AND r.pickup_at < '{within_a_day}'))"
        ));
    }
    let today = crate::util::today_local().format("%Y-%m-%d").to_string();
    // No sort chosen: upcoming and today first (soonest first), then past rides newest first.
    // `today` is YYYY-MM-DD from our own clock, so it is safe to interpolate.
    let default_order = format!(
        "(substr(r.pickup_at, 1, 10) < '{today}'), \
         CASE WHEN substr(r.pickup_at, 1, 10) < '{today}' THEN NULL ELSE r.pickup_at END ASC, \
         r.pickup_at DESC, r.id ASC"
    );
    let order = crate::util::order_by(
        q.sort.as_deref(),
        q.dir.as_deref(),
        &[
            ("pickup_at", "r.pickup_at"),
            (
                "passenger",
                // Match the passenger name returned by ride_json.
                "CASE WHEN r.is_proxy = 1 AND r.rider_name <> '' \
                 THEN r.rider_name || ' (由 ' || p.name || ' 代订)' ELSE p.name END",
            ),
            ("area", "a.name"),
            ("driver", "d.name"),
            ("status", "r.status"),
        ],
        &default_order,
        "r.id ASC",
    );

    if q.page.is_some() || q.limit.is_some() {
        let count_sql = format!(
            "SELECT COUNT(*) FROM rides r JOIN users p ON p.id = r.passenger_id \
             LEFT JOIN users d ON d.id = r.driver_id WHERE {}",
            conds.join(" AND ")
        );
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(d) = &q.date {
            count_query = count_query.bind(d);
        }
        if let Some(s) = &q.status {
            count_query = count_query.bind(s);
        }
        if let Some(s) = &search {
            count_query = count_query.bind(s);
        }
        let (total,) = count_query.fetch_one(&st.db).await?;

        let page = q.page.unwrap_or(1).max(1);
        let limit = q.limit.unwrap_or(50).clamp(1, 200);
        let offset = (page - 1) * limit;

        let sql = format!(
            "{RIDE_SELECT} WHERE {} ORDER BY {order} LIMIT ${n} OFFSET ${}",
            conds.join(" AND "),
            n + 1
        );
        let mut query = sqlx::query_as::<_, RideRow>(&sql);
        if let Some(d) = &q.date {
            query = query.bind(d);
        }
        if let Some(s) = &q.status {
            query = query.bind(s);
        }
        if let Some(s) = &search {
            query = query.bind(s);
        }
        query = query.bind(limit).bind(offset);
        let rows = query.fetch_all(&st.db).await?;
        let total_pages = if total == 0 {
            1
        } else {
            (total + limit - 1) / limit
        };

        return Ok(Json(json!({
            "items": rows.iter().map(|r| ride_json(r, &u)).collect::<Vec<_>>(),
            "total": total,
            "page": page,
            "page_size": limit,
            "total_pages": total_pages,
        })));
    }

    let sql = format!(
        "{RIDE_SELECT} WHERE {} ORDER BY {order} LIMIT 300",
        conds.join(" AND ")
    );
    let mut query = sqlx::query_as::<_, RideRow>(&sql);
    if let Some(d) = &q.date {
        query = query.bind(d);
    }
    if let Some(s) = &q.status {
        query = query.bind(s);
    }
    if let Some(s) = &search {
        query = query.bind(s);
    }
    let rows = query.fetch_all(&st.db).await?;
    Ok(Json(json!(rows
        .iter()
        .map(|r| ride_json(r, &u))
        .collect::<Vec<_>>())))
}

pub(super) async fn admin_live(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminLive, Act::View)?;
    let max_live_drivers = sqlx::query_scalar::<_, String>(
        "SELECT value FROM system_settings WHERE key='max_live_drivers'",
    )
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten()
    .and_then(|value| value.parse::<i64>().ok())
    .unwrap_or(st.cfg.max_live_drivers);
    // A cell group leader only sees drivers carrying rides for events they created.
    let scope = if u.traffic_is_global() {
        String::new()
    } else {
        format!(" AND {}", traffic_owned_events_sql(&u))
    };
    let rows: Vec<(i64, String, Option<f64>, Option<f64>, Option<String>, i64, String)> = sqlx::query_as(&format!(
        "SELECT d.id, d.name, l.lat, l.lng, l.updated_at, COUNT(r.id), MIN(r.pickup_at)
         FROM rides r JOIN users d ON d.id = r.driver_id LEFT JOIN driver_locations l ON l.driver_id = d.id
         WHERE r.status = 'in_progress'{scope} GROUP BY d.id, l.driver_id",
    ))
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!({
        "max": max_live_drivers,
        "drivers": rows.into_iter().map(|(id, name, lat, lng, at, n, first)| json!({
            "driver_id": id, "name": name, "lat": lat, "lng": lng, "updated_at": at, "rides": n, "first_pickup": first,
        })).collect::<Vec<_>>(),
    })))
}

pub(super) async fn admin_drivers(
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
