//! Event attendance marking, self check-in, member pass scanning, and check-in code reset.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::signups::allocate_pass_code;
use super::{can_manage, load_event, load_visible_event};
use crate::audience::{audience_users_sql, join_choices};
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::Act;
use crate::state::AppState;
use crate::util::{audit, now, parse_dt, random_token};

#[derive(Deserialize)]
pub(super) struct AttendanceIn {
    user_id: i64,
    attended: Option<bool>,
}

pub(super) async fn mark_attendance(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<AttendanceIn>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(
            u.lang,
            ErrorKey::OnlyOrganiserCanRecordAttendance,
        ));
    }
    if e.status == "draft" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    if e.attendance_enabled != 1 {
        return Err(bad(u.lang, ErrorKey::AttendanceModuleDisabled));
    }
    let v = req.attended.map(|a| a as i64);
    let mut tx = st.db.begin().await?;
    let target_status: Option<(String,)> = sqlx::query_as("SELECT status FROM users WHERE id=$1")
        .bind(req.user_id)
        .fetch_optional(&mut *tx)
        .await?;
    if target_status.as_ref().map(|(s,)| s.as_str()) != Some("active") {
        return Err(forbidden(u.lang, ErrorKey::MembersOnlyAttendance));
    }
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM event_signups WHERE event_id=$1 AND user_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(req.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    // Recording someone as attended also makes them a participant: a member who withdrew
    // (or was still waitlisted) but came anyway is a walk-in and must show in the list
    // and count, which only include 'going'.
    if existing.is_some() {
        sqlx::query(
            "UPDATE event_signups SET attended=$1,
               status = CASE WHEN $1 = 1 AND status IN ('cancelled','waitlist') THEN 'going' ELSE status END
             WHERE event_id=$2 AND user_id=$3",
        )
        .bind(v)
        .bind(id)
        .bind(req.user_id)
        .execute(&mut *tx)
        .await?;
    } else if v == Some(1) {
        // Walk-in: organiser may record someone who attended without signing up, but
        // only a member the event is for (or the organiser): otherwise this would let any
        // member id be marked present and then billed.
        let (in_audience,): (i64,) = sqlx::query_as(crate::dyn_sql!(
            "SELECT COUNT(*) FROM users u WHERE u.id=$2 AND u.status='active' AND (
                u.id=$3 OR u.id IN ({}))",
            audience_users_sql("$1")
        ))
        .bind(id)
        .bind(req.user_id)
        .bind(e.created_by)
        .fetch_one(&mut *tx)
        .await?;
        if in_audience == 0 {
            return Err(forbidden(u.lang, ErrorKey::EventNotOpenToGroup));
        }
        let pass_code = allocate_pass_code(&mut tx, id).await?;
        sqlx::query("INSERT INTO event_signups (event_id, user_id, status, attended, created_at, pass_code) VALUES ($1,$2, 'going', $3, $4, $5)
             ON CONFLICT (event_id, user_id) DO UPDATE SET attended=excluded.attended, status='going', pass_code=COALESCE(event_signups.pass_code, excluded.pass_code)")
            .bind(id)
            .bind(req.user_id)
            .bind(v)
            .bind(now())
            .bind(pass_code)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "event.attendance",
        "event",
        Some(id),
        format!("user {} = {:?}", req.user_id, req.attended),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct CheckinIn {
    code: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct CheckinByCodeIn {
    code: String,
}

/// Shift a stored local time by whole hours (same shape as care.rs::shift).
fn shift_hours(at: &str, hours: i64) -> String {
    parse_dt(at)
        .map(|d| {
            (d + chrono::Duration::hours(hours))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| at.to_string())
}

async fn do_checkin(
    st: &AppState,
    u: &CurrentUser,
    id: i64,
    e: &crate::routes::events::EventRow,
    code: Option<&str>,
) -> AppResult<Json<Value>> {
    if e.status != "published" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    if e.attendance_enabled != 1 {
        return Err(bad(u.lang, ErrorKey::AttendanceModuleDisabled));
    }
    // Self check-in belongs to the event itself: it opens 2 hours before the start
    // and closes 12 hours after (events carry no end time). Organisers may record
    // attendance outside this window via mark_attendance / scan_member_pass.
    let n = now();
    if n < shift_hours(&e.starts_at, -2) || n > shift_hours(&e.starts_at, 12) {
        return Err(bad(u.lang, ErrorKey::CheckinOutsideWindow));
    }
    if let Some(expected) = &e.checkin_code {
        let matches = code
            .map(str::trim)
            .is_some_and(|c| c.eq_ignore_ascii_case(expected.trim()));
        if !matches {
            return Err(bad(u.lang, ErrorKey::InvalidCheckinCode));
        }
    }

    let mut tx = st.db.begin().await?;
    let (capacity,): (i64,) = sqlx::query_as("SELECT capacity FROM events WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let existing: Option<(Option<i64>, String)> = sqlx::query_as(
        "SELECT attended, status FROM event_signups WHERE event_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(u.id)
    .fetch_optional(&mut *tx)
    .await?;

    let already_attended = existing.as_ref().is_some_and(|(att, _)| *att == Some(1));

    if !already_attended {
        let needs_spot = existing.as_ref().map(|(_, st_s)| st_s.as_str()) != Some("going");
        if needs_spot && capacity > 0 {
            let (going,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND status='going'",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if going >= capacity {
                return Err(bad(u.lang, ErrorKey::SignupClosed));
            }
        }
        if existing.is_some() {
            sqlx::query(
                "UPDATE event_signups SET attended = 1,
                   status = CASE WHEN status IN ('cancelled','waitlist') THEN 'going' ELSE status END
                 WHERE event_id = $1 AND user_id = $2",
            )
            .bind(id)
            .bind(u.id)
            .execute(&mut *tx)
            .await?;
        } else {
            // Member participates directly via check-in without requiring a ride.
            let choices = join_choices(&st.db, u, id).await.unwrap_or_default();
            let as_group = choices.first().map(|c| c.0);
            let pass_code = allocate_pass_code(&mut tx, id).await?;
            sqlx::query(
                "INSERT INTO event_signups (event_id, user_id, status, attended, need_ride, offer_drive, offer_seats, as_group_id, created_at, pass_code)
                 VALUES ($1, $2, 'going', 1, 0, 0, 0, $3, $4, $5)
                 ON CONFLICT (event_id, user_id) DO UPDATE SET attended = 1, status = 'going', pass_code = COALESCE(event_signups.pass_code, excluded.pass_code)",
            )
            .bind(id)
            .bind(u.id)
            .bind(as_group)
            .bind(now())
            .bind(pass_code)
            .execute(&mut *tx)
            .await?;
        }

        audit(
            &st.db,
            Some(u.id),
            "event.attendance",
            "event",
            Some(id),
            format!("user {} = Some(true)", u.id),
        )
        .await;
    }
    tx.commit().await?;

    Ok(Json(json!({
        "ok": true,
        "already_attended": already_attended,
        "event_id": id,
        "title": e.title,
        "starts_at": e.starts_at,
        "location": e.location,
    })))
}

pub(super) async fn self_checkin(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<CheckinIn>,
) -> AppResult<Json<Value>> {
    // Attendance is for members only!
    if u.status != "active" {
        return Err(forbidden(u.lang, ErrorKey::MembersOnlyAttendance));
    }
    let e = load_visible_event(&st, &u, id).await?;
    do_checkin(&st, &u, id, &e, req.code.as_deref()).await
}

pub(super) async fn checkin_by_code(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CheckinByCodeIn>,
) -> AppResult<Json<Value>> {
    if u.status != "active" {
        return Err(forbidden(u.lang, ErrorKey::MembersOnlyAttendance));
    }
    let raw = req.code.trim();
    if raw.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidCheckinCode));
    }
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM events
         WHERE status = 'published' AND attendance_enabled = 1
           AND LOWER(TRIM(checkin_code)) = LOWER(TRIM($1))
         ORDER BY starts_at ASC",
    )
    .bind(raw)
    .fetch_all(&st.db)
    .await?;
    if rows.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidCheckinCode));
    }
    // A code shared by several events must not silently check the member into the
    // earliest one: ask for the event's own QR code instead.
    if rows.len() > 1 {
        return Err(bad(u.lang, ErrorKey::CheckinCodeAmbiguous));
    }
    let (id,) = rows[0];

    let e = load_visible_event(&st, &u, id).await?;
    do_checkin(&st, &u, id, &e, Some(raw)).await
}

pub(super) async fn get_checkin_info(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    if u.status != "active" {
        return Err(forbidden(u.lang, ErrorKey::MembersOnlyAttendance));
    }
    let e = load_visible_event(&st, &u, id).await?;
    let my_attended: Option<(Option<i64>,)> =
        sqlx::query_as("SELECT attended FROM event_signups WHERE event_id = $1 AND user_id = $2")
            .bind(id)
            .bind(u.id)
            .fetch_optional(&st.db)
            .await?;

    Ok(Json(json!({
        "id": e.id,
        "title": e.title,
        "starts_at": e.starts_at,
        "location": e.location,
        "attendance_enabled": e.attendance_enabled == 1,
        "my_attended": my_attended.and_then(|(a,)| a) == Some(1),
        "is_member": true,
    })))
}

#[derive(Deserialize)]
pub(super) struct ScanMemberIn {
    pass_code: String,
}

pub(super) async fn scan_member_pass(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ScanMemberIn>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(
            u.lang,
            ErrorKey::OnlyOrganiserCanRecordAttendance,
        ));
    }
    if e.status != "published" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    if e.attendance_enabled != 1 {
        return Err(bad(u.lang, ErrorKey::AttendanceModuleDisabled));
    }
    let raw = req.pass_code.trim();
    let prefix = format!("GRPASS:{id}:");
    let clean = raw
        .strip_prefix(&prefix)
        .or_else(|| raw.strip_prefix(&prefix.to_ascii_lowercase()))
        .unwrap_or(raw)
        .trim();
    if clean.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidPassCode));
    }

    let mut tx = st.db.begin().await?;
    let (capacity,): (i64,) = sqlx::query_as("SELECT capacity FROM events WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let row: Option<(i64, String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT es.user_id, u.name, u.status, es.status, es.attended
         FROM event_signups es JOIN users u ON u.id = es.user_id
         WHERE es.event_id = $1 AND es.pass_code IS NOT NULL AND lower(es.pass_code) = lower($2)
         FOR UPDATE OF es",
    )
    .bind(id)
    .bind(clean)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((uid, member_name, member_status, signup_status, prev_att)) = row else {
        return Err(bad(u.lang, ErrorKey::InvalidPassCode));
    };
    if member_status != "active" {
        return Err(forbidden(u.lang, ErrorKey::MembersOnlyAttendance));
    }
    let already_attended = prev_att == Some(1);
    if !already_attended {
        if signup_status != "going" && capacity > 0 {
            let (going,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND status='going'",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if going >= capacity {
                return Err(bad(u.lang, ErrorKey::SignupClosed));
            }
        }
        sqlx::query(
            "UPDATE event_signups SET attended = 1,
                status = CASE WHEN status IN ('cancelled','waitlist') THEN 'going' ELSE status END
             WHERE event_id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        audit(
            &st.db,
            Some(u.id),
            "event.attendance",
            "event",
            Some(id),
            format!("scan member {uid} ({member_name}) = Some(true)"),
        )
        .await;
    } else {
        tx.commit().await?;
    }
    Ok(Json(json!({
        "ok": true,
        "user": { "id": uid, "name": member_name },
        "user_id": uid,
        "member_name": member_name,
        "already_attended": already_attended,
    })))
}

pub(super) async fn reset_checkin_code(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanEdit));
    }
    let new_code = random_token(10);
    sqlx::query("UPDATE events SET checkin_code = $1 WHERE id = $2")
        .bind(&new_code)
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "event.update",
        "event",
        Some(id),
        "reset checkin code",
    )
    .await;
    Ok(Json(json!({ "checkin_code": new_code })))
}
