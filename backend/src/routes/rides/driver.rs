//! Driver ride lifecycle actions: claim, withdraw, confirm, start, pickup, no-show, complete.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, AppResult};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, notify_perm, now, require, system_message, ErrorKey};

use super::location::{clear_location_if_idle, clear_passenger_location};
use super::passenger::ReasonReq;
use super::{load_ride, traffic_can, RideRow};

// ---------------- Driver ----------------

/// Seats the driver already holds within ±60 min of `at`, and their seat capacity.
pub(super) async fn driver_load_on(
    conn: &mut sqlx::PgConnection,
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
    .fetch_one(&mut *conn)
    .await?;
    let row: Option<(i64, i64, Option<i64>)> = sqlx::query_as(
        "SELECT u.carpool_enabled, u.spare_seats, v.seats FROM users u LEFT JOIN vehicles v ON v.user_id=u.id AND v.is_active=1 WHERE u.id=$1",
    )
    .bind(driver)
    .fetch_optional(&mut *conn)
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

pub(super) async fn driver_load(
    st: &AppState,
    driver: i64,
    at: &str,
    exclude: Option<i64>,
) -> AppResult<(i64, i64)> {
    let mut conn = st.db.acquire().await?;
    driver_load_on(&mut conn, driver, at, exclude).await
}

/// Passenger seats a driver offers: their spare seats, never more than the car holds
/// besides the driver. No vehicle on file ⇒ 0.
pub(crate) fn seat_capacity(spare_seats: i64, vehicle_seats: Option<i64>) -> i64 {
    let vehicle_cap = vehicle_seats.map(|s| (s - 1).max(1)).unwrap_or(0);
    spare_seats.max(1).min(vehicle_cap)
}

pub(super) async fn driver_claim(
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
    if !crate::audience::drives_for(&u, r.match_group_id) {
        return Err(forbidden(u.lang, ErrorKey::DriverOutsideGroup));
    }
    // Driving is volunteered per event: the driver must hold a place at it first.
    let Some(event_id) = r.event_id else {
        return Err(bad(u.lang, ErrorKey::EventDriveNeedsSignup));
    };
    let signed: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM event_signups WHERE event_id=$1 AND user_id=$2 AND status='going'",
    )
    .bind(event_id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    if signed.is_none() {
        return Err(bad(u.lang, ErrorKey::EventDriveNeedsSignup));
    }
    // The request list already hides past pickups; the claim itself must refuse them too.
    if r.pickup_at.as_str() < &now()[..16] {
        return Err(bad(u.lang, ErrorKey::PickupTimeInPast));
    }
    // Lock the driver's row for the whole capacity check + claim. Two concurrent claims
    // for different rides by the same driver then serialise; the second sees the first
    // booking in `load` and is rejected instead of overbooking the car.
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(u.id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((_v_seats, wheelchair_ok, child_seat)): Option<(i64, i64, i64)> = sqlx::query_as(
        "SELECT seats, wheelchair_ok, child_seat FROM vehicles WHERE user_id=$1 AND is_active=1",
    )
    .bind(u.id)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Err(bad(u.lang, ErrorKey::FillVehicleDetailsFirst));
    };
    let assist = r.assistance.trim().to_ascii_lowercase();
    let needs_wheelchair = assist.contains("wheelchair") || r.assistance.contains("轮椅");
    let needs_child_seat = assist.contains("stroller")
        || assist.contains("child")
        || r.assistance.contains("婴儿车")
        || r.assistance.contains("安全座椅");
    if (needs_wheelchair && wheelchair_ok == 0) || (needs_child_seat && child_seat == 0) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let (load, cap) = driver_load_on(&mut tx, u.id, &r.pickup_at, Some(id)).await?;
    if load + r.seats > cap {
        return Err(bad(u.lang, ErrorKey::NotEnoughSeatsSlot { load, cap }));
    }
    // Atomic claim prevents two drivers choosing the same request.
    let res = sqlx::query("UPDATE rides SET driver_id=$1, status='pending_passenger', updated_at=$2 WHERE id=$3 AND status='pending_driver'")
        .bind(u.id)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RidePickedByAnotherDriver));
    }
    tx.commit().await?;
    let target_zh = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        format!("长辈/组员 {}", r.rider_name)
    } else {
        "您".to_string()
    };
    let target_en = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        format!("rider {}", r.rider_name)
    } else {
        "you".to_string()
    };
    notify(
        &st.db,
        r.passenger_id,
        ("有司机愿意接送", "A driver offered to take you"),
        (
            &format!(
                "{} 邀请接送 {}（{}），请查看并确认。",
                u.name,
                target_zh,
                r.pickup_at.replace('T', " ")
            ),
            &format!(
                "{} offered to drive {} ({}), please review and confirm.",
                u.name,
                target_en,
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
pub(super) async fn driver_withdraw(
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
    let reason = req.reason.unwrap_or_else(|| "司机无法出车".into());
    // One statement, and only while this driver still holds the ride: reopening it and
    // letting go of the driver as two steps let another driver claim it in between,
    // and the second step then dropped the new driver, leaving the ride stuck.
    // (`status` on the right-hand side is the value before this update.)
    let released = sqlx::query(
        "UPDATE rides SET status='pending_driver', driver_id=NULL, trip_id=NULL,
                needs_attention = CASE WHEN status IN ('confirmed','pending_driver_confirm') THEN 1 ELSE 0 END,
                attention_reason=$1, updated_at=$2
         WHERE id=$3 AND driver_id=$4
           AND status IN ('pending_passenger','pending_approval','pending_driver_confirm','confirmed')",
    )
    .bind(format!("原司机 {} 无法出车：{}", u.name, reason))
    .bind(now())
    .bind(id)
    .bind(u.id)
    .execute(&st.db)
    .await?;
    if released.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
    if let (Some(d), Some(t)) = (r.driver_id, r.trip_id) {
        st.routing.invalidate(d, t).await;
    }
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

/// A trip id no other trip has used. The ride's own id is the usual choice, but once a
/// ride has started a carpool and then moved to another driver, its id still names the
/// old car's trip (the other riders keep it, and so does its chat), so reusing it would
/// put the new driver in the old car's chat. Then a fresh number comes from the rides
/// id sequence, which no ride — and so no ride-id trip — will ever get.
async fn new_trip_id(conn: &mut sqlx::PgConnection, ride_id: i64) -> AppResult<i64> {
    let claimed: Option<i64> = sqlx::query_scalar(
        "INSERT INTO trips (id) VALUES ($1) ON CONFLICT (id) DO NOTHING RETURNING id",
    )
    .bind(ride_id)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(t) = claimed {
        return Ok(t);
    }
    let fresh: i64 = sqlx::query_scalar(
        "INSERT INTO trips (id) VALUES (nextval(pg_get_serial_sequence('rides', 'id'))) RETURNING id",
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(fresh)
}

/// Assign a confirmed ride to a trip: same driver, same destination, within 60 minutes.
/// Run while holding the driver's row lock, so two rides confirmed for one driver at the
/// same moment join the same trip instead of each opening their own.
pub(super) async fn assign_trip(conn: &mut sqlx::PgConnection, r: &RideRow) -> AppResult<i64> {
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT trip_id FROM rides
         WHERE driver_id=$1 AND id != $2 AND trip_id IS NOT NULL AND status IN ('confirmed','in_progress')
           AND leg = $5
           AND (
               (leg <> 'return' AND destination = $3)
               OR (leg = 'return' AND (($6::BIGINT IS NOT NULL AND event_id = $6) OR pickup_address = $7))
           )
           AND abs(EXTRACT(EPOCH FROM (pickup_at::timestamp - $4::timestamp))) < 3600
         LIMIT 1",
    )
    .bind(r.driver_id)
    .bind(r.id)
    .bind(&r.destination)
    .bind(&r.pickup_at)
    .bind(&r.leg)
    .bind(r.event_id)
    .bind(&r.pickup_address)
    .fetch_optional(&mut *conn)
    .await?;
    let trip = match existing {
        Some((t,)) => t,
        None => new_trip_id(&mut *conn, r.id).await?,
    };
    sqlx::query("UPDATE rides SET trip_id=$1 WHERE id=$2")
        .bind(trip)
        .bind(r.id)
        .execute(&mut *conn)
        .await?;
    Ok(trip)
}

pub(super) async fn announce_confirmed(st: &AppState, r: &RideRow, trip: i64, reassigned: bool) {
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
        let p_desc = if r.is_proxy == 1 && !r.rider_name.is_empty() {
            format!("{}（由 {} 代订）", r.rider_name, r.p_name)
        } else {
            r.p_name.clone()
        };
        let line = format!(
            "{} · {} · {}",
            r.pickup_at.replace('T', " "),
            p_desc,
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
    let rider_chat_desc = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        format!("{}（由 {} 代订）", r.rider_name, r.p_name)
    } else {
        r.p_name.clone()
    };
    system_message(
        &st.db,
        trip,
        &format!(
            "{} 已加入本趟群聊（{}）。",
            rider_chat_desc,
            r.pickup_at.replace('T', " ")
        ),
        &format!(
            "{} joined this trip's group chat ({}).",
            rider_chat_desc,
            r.pickup_at.replace('T', " ")
        ),
    )
    .await;
}

/// Driver accepts a reassignment made by 交通组.
pub(super) async fn driver_confirm_assignment(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Drive, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    // Only while this driver still holds the assignment: staff may have reassigned it.
    // `reminder_sent=0`: a new driver means the pickup reminder goes out again.
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(u.id)
        .fetch_optional(&mut *tx)
        .await?;
    let (load, cap) = driver_load_on(&mut tx, u.id, &r.pickup_at, Some(id)).await?;
    if load + r.seats > cap {
        return Err(bad(u.lang, ErrorKey::NotEnoughSeatsSlot { load, cap }));
    }
    let confirmed = sqlx::query(
        "UPDATE rides SET status='confirmed', needs_attention=0, reminder_sent=0, updated_at=$1
         WHERE id=$2 AND driver_id=$3 AND status='pending_driver_confirm'",
    )
    .bind(now())
    .bind(id)
    .bind(u.id)
    .execute(&mut *tx)
    .await?;
    if confirmed.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
    assign_trip(&mut tx, &r).await?;
    tx.commit().await?;
    let r = load_ride(&st, u.lang, id).await?;
    let trip = r.trip_id.unwrap_or(id);
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

pub(super) async fn start_ride(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Drive, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    // 同一时间最多追踪 N 位司机. Counting the live drivers and starting this trip happen
    // in one transaction behind one lock, so two drivers setting off at the same moment
    // cannot both see room under the limit and both go live.
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_live_drivers', 0))")
        .execute(&mut *tx)
        .await?;
    let (live,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress' AND driver_id != $1",
    )
    .bind(u.id)
    .fetch_one(&mut *tx)
    .await?;
    let max_live_drivers: i64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM system_settings WHERE key='max_live_drivers'",
    )
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten()
    .and_then(|s| s.parse::<i64>().ok())
    .filter(|n| (1..=100).contains(n))
    .unwrap_or(st.cfg.max_live_drivers);
    if live >= max_live_drivers {
        return Err(conflict(
            u.lang,
            ErrorKey::MaxLiveDriversReached {
                live,
                limit: max_live_drivers,
            },
        ));
    }
    let started_at = now();
    let began = sqlx::query(
        "UPDATE rides SET status='in_progress', started_at=$1, updated_at=$1
         WHERE id=$2 AND driver_id=$3 AND status='confirmed'",
    )
    .bind(&started_at)
    .bind(id)
    .bind(u.id)
    .execute(&mut *tx)
    .await?;
    if began.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
    // The driver sets off once for the whole carpool: every other confirmed passenger in
    // this trip starts too, so each of them can follow the car and see their ETA.
    let trip = r.trip_id.unwrap_or(id);
    let others: Vec<(i64,)> = sqlx::query_as(
        "UPDATE rides SET status='in_progress', started_at=$1, updated_at=$1
         WHERE driver_id=$2 AND trip_id=$3 AND id != $4 AND status='confirmed' RETURNING id",
    )
    .bind(&started_at)
    .bind(u.id)
    .bind(trip)
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
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

pub(super) async fn pickup(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Drive, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    if r.status != "in_progress" {
        return Err(bad(u.lang, ErrorKey::StartTripFirst));
    }
    // Only while the trip is still running: a no-show or completion saved since the read
    // above has ended the ride.
    let picked = sqlx::query(
        "UPDATE rides SET picked_up_at=$1, updated_at=$2 WHERE id=$3 AND status='in_progress'",
    )
    .bind(now())
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?;
    if picked.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
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
    let rider_disp = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        r.rider_name.clone()
    } else {
        r.p_name.clone()
    };
    let dest_disp = if r.leg == "return" {
        &r.area_name
    } else {
        &r.destination
    };
    if r.is_proxy == 1 {
        notify(
            &st.db,
            r.passenger_id,
            ("平安上车通知", "Picked up safely"),
            (
                &format!(
                    "{} 已由司机 {} 接上车，正在前往 {}。",
                    rider_disp, u.name, dest_disp
                ),
                &format!(
                    "{} has been picked up by {} and is heading to {}.",
                    rider_disp, u.name, dest_disp
                ),
            ),
            &format!("/rides/{id}"),
        )
        .await;
    }
    let caregivers = caregiver_ids_for_ride(&st, &r).await;
    if !caregivers.is_empty() {
        crate::util::notify_many(
            &st.db,
            &caregivers,
            ("平安上车通知", "Picked up safely"),
            (
                &format!(
                    "{} 已由司机 {} 接上车，正在前往 {}。",
                    rider_disp, u.name, dest_disp
                ),
                &format!(
                    "{} has been picked up by {} and is heading to {}.",
                    rider_disp, u.name, dest_disp
                ),
            ),
            &format!("/rides/{id}"),
        )
        .await;
    }
    audit(&st.db, Some(u.id), "ride.pickup", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

async fn caregiver_ids_for_ride(st: &AppState, r: &RideRow) -> Vec<i64> {
    let mut ids = Vec::new();
    if let Ok(Some(cg)) = sqlx::query_scalar::<_, i64>(
        "SELECT cg.id FROM users u JOIN users cg ON cg.id = u.caregiver_user_id
         WHERE u.id=$1 AND cg.status = 'active'",
    )
    .bind(r.passenger_id)
    .fetch_optional(&st.db)
    .await
    {
        if cg != r.passenger_id && !ids.contains(&cg) {
            ids.push(cg);
        }
    }
    if r.is_proxy == 1 && !r.rider_name.is_empty() {
        if let Ok(Some(cg)) = sqlx::query_scalar::<_, i64>(
            "SELECT cg.id FROM proxy_riders p JOIN users cg ON cg.id = p.caregiver_user_id
             WHERE p.user_id=$1 AND p.name=$2 AND cg.status = 'active'
             ORDER BY p.id ASC LIMIT 1",
        )
        .bind(r.passenger_id)
        .bind(&r.rider_name)
        .fetch_optional(&st.db)
        .await
        {
            if cg != r.passenger_id && !ids.contains(&cg) {
                ids.push(cg);
            }
        }
    }
    ids
}

pub(super) async fn no_show(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) && !traffic_can(&u, &r, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }
    // A passenger who has been picked up showed up. Checked in the update itself too, so a
    // pickup recorded between this read and the write still wins.
    if r.picked_up_at.is_some() {
        return Err(conflict(u.lang, ErrorKey::PassengerAlreadyPickedUp));
    }
    let ended = sqlx::query(
        "UPDATE rides SET status='no_show', needs_attention=0, completed_at=$1, updated_at=$1
         WHERE id=$2 AND status IN ('in_progress', 'confirmed') AND picked_up_at IS NULL",
    )
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?;
    if ended.rows_affected() == 0 {
        let picked: Option<(Option<String>,)> =
            sqlx::query_as("SELECT picked_up_at FROM rides WHERE id=$1")
                .bind(id)
                .fetch_optional(&st.db)
                .await?;
        return Err(match picked {
            Some((Some(_),)) => conflict(u.lang, ErrorKey::PassengerAlreadyPickedUp),
            _ => conflict(u.lang, ErrorKey::RideStatusChanged),
        });
    }
    if r.leg == "outbound" {
        if let Some(pair_id) = r.paired_ride_id {
            sqlx::query(
                "UPDATE rides SET needs_attention=1, attention_reason='去程未到场，请确认回程是否仍需接送', updated_at=$1
                 WHERE id=$2 AND status NOT IN ('completed','cancelled','no_show','alt_transport')",
            )
            .bind(now())
            .bind(pair_id)
            .execute(&st.db)
            .await?;
        }
    }
    if let Some(d) = r.driver_id {
        clear_location_if_idle(&st, d).await?;
    }
    if let (Some(d), Some(t)) = (r.driver_id, r.trip_id) {
        st.routing.invalidate(d, t).await;
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
    audit(&st.db, Some(u.id), "ride.no_show", "ride", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn complete(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Drive, Act::Update)?;
    let r = load_ride(&st, u.lang, id).await?;
    if r.driver_id != Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }

    // --- Auto reimbursement: a flat fare per pair of areas, once per driver trip ---
    // Only rides for events with mileage_claimable = 1 are eligible. The fare is the one
    // for (pickup area, event area), the same both ways. A trip picking up from several
    // areas pays the highest of its fares once, so the rides of one trip share it.
    // A pair with no fare yet pays nothing and asks the driver for their mileage receipt.
    let mut tx = st.db.begin().await?;
    if let Some(t) = r.trip_id {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_trip_fare', $1))")
            .bind(t)
            .execute(&mut *tx)
            .await?;
    }
    let (reimbursement_cents, receipt_needed): (i64, i64) = if r.event_id.is_some()
        && r.event_mileage_claimable == 1
    {
        // Rides still travelling in this trip count too, so the fare is the same
        // whichever rider is dropped off first.
        let fares: Vec<Option<i64>> = sqlx::query_scalar(
            "SELECT (SELECT f.amount_cents FROM area_fares f
                          WHERE f.area_a_id = LEAST(x.pickup_area_id, e.area_id)
                            AND f.area_b_id = GREATEST(x.pickup_area_id, e.area_id))
                 FROM rides x JOIN events e ON e.id = x.event_id
                 WHERE x.id = $1
                    OR ($2::BIGINT IS NOT NULL AND x.trip_id = $2 AND x.driver_id = $3
                        AND x.status IN ('confirmed','in_progress','completed'))",
        )
        .bind(id)
        .bind(r.trip_id)
        .bind(u.id)
        .fetch_all(&mut *tx)
        .await?;
        match fares.into_iter().flatten().max() {
            Some(fare) => {
                let paid: i64 = match r.trip_id {
                        Some(t) => {
                            sqlx::query_scalar(
                                "SELECT COALESCE(SUM(reimbursement_amount_cents),0)::BIGINT
                                 FROM rides WHERE trip_id=$1 AND id<>$2 AND driver_id=$3 AND status='completed'",
                            )
                            .bind(t)
                            .bind(id)
                            .bind(u.id)
                            .fetch_one(&mut *tx)
                            .await?
                        }
                        None => 0,
                    };
                ((fare - paid).max(0), 0)
            }
            None => (0, 1),
        }
    } else {
        (0, 0)
    };

    // One statement: status, completion time and reimbursement land together, and only
    // while this driver's trip is still running.
    let done = sqlx::query(
        "UPDATE rides SET status='completed', updated_at=$1, completed_at=$1,
                picked_up_at=COALESCE(picked_up_at, $1), reimbursement_amount_cents=$2,
                mileage_receipt_needed=$3
         WHERE id=$4 AND driver_id=$5 AND status='in_progress'",
    )
    .bind(now())
    .bind(reimbursement_cents)
    .bind(receipt_needed)
    .bind(id)
    .bind(u.id)
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }
    tx.commit().await?;
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
    let caregivers = caregiver_ids_for_ride(&st, &r).await;
    if !caregivers.is_empty() {
        let rider_disp = if r.is_proxy == 1 && !r.rider_name.is_empty() {
            r.rider_name.clone()
        } else {
            r.p_name.clone()
        };
        let dest_disp = if r.leg == "return" {
            &r.area_name
        } else {
            &r.destination
        };
        crate::util::notify_many(
            &st.db,
            &caregivers,
            ("平安抵达通知", "Safe arrival"),
            (
                &format!("{} 已安全抵达 {}。", rider_disp, dest_disp),
                &format!("{} has arrived safely at {}.", rider_disp, dest_disp),
            ),
            &format!("/rides/{id}"),
        )
        .await;
    }
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
    audit(
        &st.db,
        Some(u.id),
        "ride.complete",
        "ride",
        Some(id),
        if receipt_needed == 1 {
            "no fare for this route; mileage receipt needed".to_string()
        } else {
            format!("reimbursement {}", fmt_rm(reimbursement_cents))
        },
    )
    .await;
    Ok(Json(json!({"ok": true})))
}
