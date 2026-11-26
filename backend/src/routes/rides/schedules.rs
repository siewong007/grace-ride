//! Standing / recurring ride and driver schedules.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::audience::MATCH_GROUP_SQL;
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::perms::{Act, Page, Perm};
use crate::state::AppState;
use crate::util::{notify, now, ErrorKey};

// ---------------- Standing / Recurring Ride & Driver Schedules ----------------

#[derive(sqlx::FromRow, Serialize)]
struct RecurringScheduleRow {
    id: i64,
    user_id: i64,
    kind: String,
    label: String,
    weekday: i64,
    pickup_time: String,
    area_id: i64,
    area_name: String,
    pickup_address: String,
    destination: String,
    seats: i64,
    assistance: String,
    notes: String,
    lead_days: i64,
    active: i64,
    last_generated_date: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub(super) struct RecurringScheduleIn {
    kind: String, // passenger | driver
    #[serde(default)]
    label: Option<String>,
    weekday: i64,        // 0 (Sunday) .. 6 (Saturday)
    pickup_time: String, // HH:MM
    area_id: i64,
    #[serde(default)]
    pickup_address: Option<String>,
    #[serde(default)]
    destination: Option<String>,
    #[serde(default)]
    seats: Option<i64>,
    #[serde(default)]
    assistance: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    lead_days: Option<i64>,
    #[serde(default)]
    active: Option<bool>,
}

fn validate_hhmm(s: &str) -> Option<String> {
    let s = s.trim();
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h < 24 && m < 60 {
        Some(format!("{h:02}:{m:02}"))
    } else {
        None
    }
}

pub(super) async fn list_recurring_schedules(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    crate::util::require_one(&u, &[(Page::Rides, Act::View), (Page::Drive, Act::View)])?;
    let rows: Vec<RecurringScheduleRow> = sqlx::query_as(
        "SELECT s.id, s.user_id, s.kind, s.label, s.weekday, s.pickup_time, s.area_id, a.name AS area_name,
                s.pickup_address, s.destination, s.seats, s.assistance, s.notes, s.lead_days, s.active,
                s.last_generated_date, s.created_at, s.updated_at
         FROM recurring_ride_schedules s JOIN areas a ON a.id = s.area_id
         WHERE s.user_id=$1 ORDER BY s.kind, s.weekday, s.pickup_time",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows)))
}

pub(super) async fn create_recurring_schedule(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<RecurringScheduleIn>,
) -> AppResult<Json<Value>> {
    match req.kind.as_str() {
        // Rides exist only for events now; a standing weekly ride has no event to attach to.
        "passenger" => return Err(bad(u.lang, ErrorKey::EventRideNeedsSignup)),
        "driver" => {
            if !u.can(Page::Drive, Act::Create) && !u.can(Page::Drive, Act::Update) {
                return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
            }
        }
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    }
    if !(0..=6).contains(&req.weekday) {
        return Err(bad(u.lang, ErrorKey::InvalidDate));
    }
    let pickup_time =
        validate_hhmm(&req.pickup_time).ok_or_else(|| bad(u.lang, ErrorKey::InvalidPickupTime))?;
    let seats = req.seats.unwrap_or(1);
    let max_seats = if req.kind == "passenger" { 6 } else { 8 };
    if !(1..=max_seats).contains(&seats) {
        return Err(bad(u.lang, ErrorKey::SeatsRange));
    }
    let area_ok: Option<(i64,)> = sqlx::query_as("SELECT id FROM areas WHERE id=$1 AND active=1")
        .bind(req.area_id)
        .fetch_optional(&st.db)
        .await?;
    area_ok.ok_or_else(|| bad(u.lang, ErrorKey::PickupAreaRequired))?;
    let pickup_address = req.pickup_address.unwrap_or_default().trim().to_string();
    let destination = req.destination.unwrap_or_default().trim().to_string();
    if req.kind == "passenger" && (pickup_address.is_empty() || destination.is_empty()) {
        return Err(bad(u.lang, ErrorKey::AddressAndDestinationRequired));
    }
    let lead_days = req.lead_days.unwrap_or(3).clamp(1, 14);
    let active = if req.active.unwrap_or(true) {
        1i64
    } else {
        0i64
    };
    let (assistance, notes) = super::normalize_assistance(
        req.assistance.as_deref().unwrap_or_default(),
        req.notes.as_deref().unwrap_or_default(),
    );
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO recurring_ride_schedules (user_id, kind, label, weekday, pickup_time, area_id, pickup_address, destination, seats, assistance, notes, lead_days, active, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING id",
    )
    .bind(u.id)
    .bind(&req.kind)
    .bind(req.label.unwrap_or_default().trim())
    .bind(req.weekday)
    .bind(&pickup_time)
    .bind(req.area_id)
    .bind(&pickup_address)
    .bind(&destination)
    .bind(seats)
    .bind(&assistance)
    .bind(&notes)
    .bind(lead_days)
    .bind(active)
    .bind(now())
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    Ok(Json(json!({ "id": id, "ok": true })))
}

pub(super) async fn update_recurring_schedule(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<RecurringScheduleIn>,
) -> AppResult<Json<Value>> {
    match req.kind.as_str() {
        // Rides exist only for events now; a standing weekly passenger schedule has no event.
        "passenger" => return Err(bad(u.lang, ErrorKey::EventRideNeedsSignup)),
        "driver" => {
            if !u.can(Page::Drive, Act::Create) && !u.can(Page::Drive, Act::Update) {
                return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
            }
        }
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    }
    if !(0..=6).contains(&req.weekday) {
        return Err(bad(u.lang, ErrorKey::InvalidDate));
    }
    let pickup_time =
        validate_hhmm(&req.pickup_time).ok_or_else(|| bad(u.lang, ErrorKey::InvalidPickupTime))?;
    let seats = req.seats.unwrap_or(1);
    let max_seats = if req.kind == "passenger" { 6 } else { 8 };
    if !(1..=max_seats).contains(&seats) {
        return Err(bad(u.lang, ErrorKey::SeatsRange));
    }
    let area_ok: Option<(i64,)> = sqlx::query_as("SELECT id FROM areas WHERE id=$1 AND active=1")
        .bind(req.area_id)
        .fetch_optional(&st.db)
        .await?;
    area_ok.ok_or_else(|| bad(u.lang, ErrorKey::PickupAreaRequired))?;
    let pickup_address = req.pickup_address.unwrap_or_default().trim().to_string();
    let destination = req.destination.unwrap_or_default().trim().to_string();
    if req.kind == "passenger" && (pickup_address.is_empty() || destination.is_empty()) {
        return Err(bad(u.lang, ErrorKey::AddressAndDestinationRequired));
    }
    let lead_days = req.lead_days.unwrap_or(3).clamp(1, 14);
    let active = if req.active.unwrap_or(true) {
        1i64
    } else {
        0i64
    };
    let (assistance, notes) = super::normalize_assistance(
        req.assistance.as_deref().unwrap_or_default(),
        req.notes.as_deref().unwrap_or_default(),
    );
    let res = sqlx::query(
        "UPDATE recurring_ride_schedules
         SET last_generated_date = CASE WHEN weekday <> $3 OR pickup_time <> $4 THEN NULL ELSE last_generated_date END,
             kind=$1, label=$2, weekday=$3, pickup_time=$4, area_id=$5, pickup_address=$6, destination=$7,
             seats=$8, assistance=$9, notes=$10, lead_days=$11, active=$12, updated_at=$13
         WHERE id=$14 AND user_id=$15",
    )
    .bind(&req.kind)
    .bind(req.label.unwrap_or_default().trim())
    .bind(req.weekday)
    .bind(&pickup_time)
    .bind(req.area_id)
    .bind(&pickup_address)
    .bind(&destination)
    .bind(seats)
    .bind(&assistance)
    .bind(&notes)
    .bind(lead_days)
    .bind(active)
    .bind(now())
    .bind(id)
    .bind(u.id)
    .execute(&st.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn delete_recurring_schedule(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    crate::util::require_one(
        &u,
        &[(Page::Rides, Act::Delete), (Page::Drive, Act::Delete)],
    )?;
    let res = sqlx::query("DELETE FROM recurring_ride_schedules WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(sqlx::FromRow)]
struct ActiveScheduleRow {
    id: i64,
    user_id: i64,
    user_name: String,
    kind: String,
    label: String,
    weekday: i64,
    pickup_time: String,
    area_id: i64,
    area_name: String,
    lead_days: i64,
    active: i64,
    last_generated_date: Option<String>,
}

async fn apply_recurring_schedule_for_date(
    db: &PgPool,
    s: &ActiveScheduleRow,
    target_date: &str,
) -> anyhow::Result<Option<i64>> {
    if s.kind != "driver" {
        return Ok(None);
    }
    let mut tx = db.begin().await?;
    let locked: Option<(i64, Option<String>, i64, String)> = sqlx::query_as(
        "SELECT active, last_generated_date, weekday, pickup_time
         FROM recurring_ride_schedules WHERE id=$1 FOR UPDATE",
    )
    .bind(s.id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((active, last_gen, locked_weekday, locked_time)) = locked else {
        return Ok(None);
    };
    if active != 1
        || locked_weekday != s.weekday
        || locked_time != s.pickup_time
        || last_gen.as_deref().is_some_and(|d| d >= target_date)
    {
        return Ok(None);
    }

    sqlx::query(
        "UPDATE recurring_ride_schedules SET last_generated_date=$1, updated_at=$2 WHERE id=$3",
    )
    .bind(target_date)
    .bind(now())
    .bind(s.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let (_, drive_scope) = crate::audience::group_scopes(db, s.user_id).await?;
    let open_sql = format!(
        "SELECT COUNT(*) FROM rides r
         WHERE r.status='pending_driver' AND r.pickup_area_id=$1
           AND r.pickup_at >= ($2 || 'T') AND r.pickup_at < ($2 || 'U')
           AND r.passenger_id != $3
           AND (SELECT _mg.id IS NULL OR _mg.id = ANY($4) FROM (SELECT {MATCH_GROUP_SQL} AS id) _mg)"
    );
    let (open,): (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(&*open_sql))
        .bind(s.area_id)
        .bind(target_date)
        .bind(s.user_id)
        .bind(&drive_scope)
        .fetch_one(db)
        .await?;
    let label = if s.label.is_empty() {
        s.area_name.clone()
    } else {
        s.label.clone()
    };
    notify(
        db,
        s.user_id,
        ("周期出车时段提醒", "Standing drive schedule reminder"),
        (
            &format!(
                "{label} · {target_date} {} · 目前有 {open} 笔待接送请求（{}）",
                s.pickup_time, s.user_name
            ),
            &format!(
                "{label} · {target_date} {} · {open} open ride request(s) in your area",
                s.pickup_time
            ),
        ),
        "/drive",
    )
    .await;
    Ok(Some(0))
}

/// Generate upcoming rides / driver reminders from standing weekly schedules.
pub async fn process_recurring_schedules(db: &PgPool) -> anyhow::Result<usize> {
    use crate::perms;
    let today = crate::util::today_local();
    let now_hm = crate::util::local_now().format("%H:%M").to_string();
    let query_sql = format!(
        "SELECT s.id, s.user_id, u.name AS user_name, s.kind, s.label, s.weekday, s.pickup_time, s.area_id, a.name AS area_name,
                s.lead_days, s.active, s.last_generated_date
         FROM recurring_ride_schedules s
         JOIN users u ON u.id = s.user_id
         JOIN areas a ON a.id = s.area_id
         WHERE s.active = 1 AND u.status = 'active' AND a.active = 1
           AND s.kind = 'driver' AND {}",
        perms::sql_holds("u", Perm(Page::Drive, Act::Create)),
    );
    let rows: Vec<ActiveScheduleRow> = sqlx::query_as(sqlx::AssertSqlSafe(&*query_sql))
        .fetch_all(db)
        .await?;

    let mut count = 0usize;
    for s in rows {
        for offset in 0..=s.lead_days {
            let d = today + chrono::Duration::days(offset);
            if d.weekday().num_days_from_sunday() as i64 != s.weekday {
                continue;
            }
            if offset == 0 && s.pickup_time <= now_hm {
                continue;
            }
            let target_str = d.format("%Y-%m-%d").to_string();
            if s.last_generated_date
                .as_deref()
                .is_some_and(|last| last >= target_str.as_str())
            {
                continue;
            }
            if apply_recurring_schedule_for_date(db, &s, &target_str)
                .await?
                .is_some()
            {
                count += 1;
            }
            break;
        }
    }
    Ok(count)
}

pub(super) async fn generate_recurring_schedule(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let row: Option<(ActiveScheduleRow, i64)> = sqlx::query_as::<_, ActiveScheduleRow>(
        "SELECT s.id, s.user_id, u.name AS user_name, s.kind, s.label, s.weekday, s.pickup_time, s.area_id, a.name AS area_name,
                s.lead_days, s.active, s.last_generated_date
         FROM recurring_ride_schedules s
         JOIN users u ON u.id = s.user_id
         JOIN areas a ON a.id = s.area_id
         WHERE s.id=$1 AND s.user_id=$2",
    )
    .bind(id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?
    .zip(
        sqlx::query_scalar::<_, i64>(
            "SELECT a.active FROM recurring_ride_schedules s JOIN areas a ON a.id = s.area_id WHERE s.id=$1 AND s.user_id=$2",
        )
        .bind(id)
        .bind(u.id)
        .fetch_optional(&st.db)
        .await?,
    );
    let (s, area_active) = row.ok_or_else(|| not_found(u.lang))?;
    match s.kind.as_str() {
        // Rides exist only for events now; a standing weekly ride has no event to attach to.
        "passenger" => return Err(bad(u.lang, ErrorKey::EventRideNeedsSignup)),
        "driver" => {
            if !u.can(Page::Drive, Act::Create) && !u.can(Page::Drive, Act::Update) {
                return Err(forbidden(u.lang, ErrorKey::OnlyVolunteerDriversCanSet));
            }
        }
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    }
    if s.active != 1 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if area_active != 1 {
        return Err(bad(u.lang, ErrorKey::PickupAreaRequired));
    }

    let today = crate::util::today_local();
    let now_hm = crate::util::local_now().format("%H:%M").to_string();
    let mut chosen: Option<String> = None;
    for offset in 0..=s.lead_days.max(7) {
        let d = today + chrono::Duration::days(offset);
        if d.weekday().num_days_from_sunday() as i64 != s.weekday {
            continue;
        }
        if offset == 0 && s.pickup_time <= now_hm {
            continue;
        }
        chosen = Some(d.format("%Y-%m-%d").to_string());
        break;
    }
    let Some(target_date) = chosen else {
        return Err(bad(u.lang, ErrorKey::InvalidDate));
    };
    let generated = apply_recurring_schedule_for_date(&st.db, &s, &target_date)
        .await
        .map_err(crate::error::AppError::Other)?;
    Ok(Json(json!({
        "ok": true,
        "target_date": target_date,
        "ride_id": generated.filter(|&id| id > 0),
        "already_generated": generated.is_none(),
    })))
}

/// Ride states that have not started yet: these still follow the event they go to.
const NOT_STARTED: &[&str] = &[
    "pending_driver",
    "pending_passenger",
    "pending_approval",
    "pending_driver_confirm",
    "confirmed",
];

/// An event's time or venue changed: its rides that haven't started follow it.
///
/// - A ride headed for the old venue now goes to the new one.
/// - A new time is not applied to pickups (each rider chose theirs), so the rides are
///   flagged for the transport team to check.
/// - Drivers are told; members going were already told by the event change.
/// - Visitor links close relative to the new time.
/// - The event's reminder is sent again for the new time.
pub async fn event_moved(
    st: &AppState,
    event_id: i64,
    (old_at, new_at): (&str, &str),
    (old_place, new_place): (&str, &str),
) -> AppResult<()> {
    if old_at == new_at && old_place == new_place {
        return Ok(());
    }
    let statuses: Vec<String> = NOT_STARTED.iter().map(|s| s.to_string()).collect();
    let mut tx = st.db.begin().await?;
    if old_place != new_place {
        sqlx::query(
            "UPDATE rides SET destination=$1, updated_at=$2
             WHERE event_id=$3 AND leg='outbound' AND status = ANY($4) AND destination=$5",
        )
        .bind(new_place)
        .bind(now())
        .bind(event_id)
        .bind(&statuses)
        .bind(old_place)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE rides SET pickup_address=$1, updated_at=$2
             WHERE event_id=$3 AND leg='return' AND status = ANY($4) AND pickup_address=$5",
        )
        .bind(new_place)
        .bind(now())
        .bind(event_id)
        .bind(&statuses)
        .bind(old_place)
        .execute(&mut *tx)
        .await?;
    }
    if old_at != new_at {
        // The day-before reminder is for the new time, even if one went out for the old.
        sqlx::query("UPDATE events SET reminder_sent=0 WHERE id=$1")
            .bind(event_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE rides SET needs_attention=1, attention_reason=$1, updated_at=$2
             WHERE event_id=$3 AND status = ANY($4)",
        )
        .bind(format!(
            "活动时间已由 {} 改为 {}，请核对接送时间",
            old_at.replace('T', " "),
            new_at.replace('T', " ")
        ))
        .bind(now())
        .bind(event_id)
        .bind(&statuses)
        .execute(&mut *tx)
        .await?;
        if let (Some(old_dt), Some(new_dt)) =
            (crate::util::parse_dt(old_at), crate::util::parse_dt(new_at))
        {
            let delta = new_dt - old_dt;
            if delta > chrono::Duration::zero() {
                let open_passes: Vec<(i64, String)> = sqlx::query_as(
                    "SELECT cp.id, cp.valid_until
                     FROM caregiver_passes cp
                     JOIN rides r ON r.id = cp.ride_id
                     WHERE r.event_id=$1 AND r.status = ANY($2) AND cp.closed_at IS NULL",
                )
                .bind(event_id)
                .bind(&statuses)
                .fetch_all(&mut *tx)
                .await?;
                for (cp_id, vu) in open_passes {
                    if let Some(vu_dt) = crate::util::parse_dt(&vu) {
                        let shifted = (vu_dt + delta).format("%Y-%m-%dT%H:%M:%S").to_string();
                        sqlx::query("UPDATE caregiver_passes SET valid_until=$1 WHERE id=$2")
                            .bind(shifted)
                            .bind(cp_id)
                            .execute(&mut *tx)
                            .await?;
                    }
                }
            }
        }
    }
    let driver_trips: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT DISTINCT driver_id, COALESCE(trip_id, id) FROM rides
         WHERE event_id=$1 AND status = ANY($2) AND driver_id IS NOT NULL",
    )
    .bind(event_id)
    .bind(&statuses)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    for &(d, t) in &driver_trips {
        st.routing.invalidate(d, t).await;
    }

    crate::routes::visitors::event_moved(&st.db, event_id, new_at).await?;

    let (title,): (String,) = sqlx::query_as("SELECT title FROM events WHERE id=$1")
        .bind(event_id)
        .fetch_one(&st.db)
        .await?;
    let body = format!("{} · {} · {}", title, new_at.replace('T', " "), new_place);
    let body_masked_zh = format!("教会活动 · {} · {}", new_at.replace('T', " "), new_place);
    let body_masked_en = format!(
        "Church event · {} · {}",
        new_at.replace('T', " "),
        new_place
    );
    let ids: Vec<i64> = driver_trips
        .into_iter()
        .map(|(d, _)| d)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut visible_ids: Vec<i64> = Vec::new();
    let mut masked_ids: Vec<i64> = Vec::new();
    for d in ids {
        if crate::audience::member_sees_event(&st.db, d, event_id)
            .await
            .unwrap_or(false)
        {
            visible_ids.push(d);
        } else {
            masked_ids.push(d);
        }
    }
    crate::util::notify_many(
        &st.db,
        &visible_ids,
        ("接送的活动已更改", "Event for your ride has changed"),
        (&body, &body),
        "/drive",
    )
    .await;
    crate::util::notify_many(
        &st.db,
        &masked_ids,
        ("接送的活动已更改", "Event for your ride has changed"),
        (&body_masked_zh, &body_masked_en),
        "/drive",
    )
    .await;
    Ok(())
}
