//! Event creation, updates, duplication, cancellation, and completion handlers.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::signups::{notify_promoted, promote_waitlist};
use super::{can_manage, load_event, load_visible_event, KINDS};
use crate::audience::audience_users_sql;
use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, audit_many, norm_dt, notify_many, now, random_token, require, Lang};

#[derive(Deserialize)]
pub(super) struct EventIn {
    title: String,
    kind: String,
    icon: Option<String>,
    description: Option<String>,
    starts_at: String,
    location: String,
    capacity: Option<i64>,
    cost_note: Option<String>,
    signup_deadline: Option<String>,
    group_ids: Vec<i64>,
    /// Narrow the "joining as" choices. Empty or missing: the audience and its sub-groups.
    as_group_ids: Option<Vec<i64>>,
    /// Optional cover photo: an image the saver uploaded (or the event's current photo).
    photo_file_id: Option<i64>,
    /// Optional attendance module: members can scan QR code to check in without a ride.
    attendance_enabled: Option<bool>,
    /// Optional mileage claimability: whether rides for this event are claimable for mileage.
    mileage_claimable: Option<bool>,
    /// The area the event is held in; with a rider's pickup area it sets the mileage fare.
    area_id: Option<i64>,
    /// Optional recurring schedule: none | weekly | biweekly | monthly
    repeat_frequency: Option<String>,
    /// Number of occurrences to generate (1..=12)
    repeat_count: Option<i64>,
    /// Optional initial status: "published" (default) or "draft" (hidden from members until published).
    status: Option<String>,
}

fn parse_create_status(raw: Option<&str>, lang: Lang) -> AppResult<&'static str> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None | Some("published") => Ok("published"),
        Some("draft") => Ok("draft"),
        Some(_) => Err(bad(lang, ErrorKey::InvalidStatus)),
    }
}

/// The event's area must be a real area.
async fn check_area(st: &AppState, u: &CurrentUser, area_id: Option<i64>) -> AppResult<()> {
    if let Some(id) = area_id {
        let found: Option<i64> = sqlx::query_scalar("SELECT id FROM areas WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
        if found.is_none() {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    Ok(())
}

/// A cover photo must be an image the saver uploaded themselves, so nobody can surface
/// someone else's receipt by naming its id. Keeping the event's current photo is always
/// fine, so an admin editing another organiser's event doesn't lose it.
async fn check_photo(
    st: &AppState,
    u: &CurrentUser,
    file_id: Option<i64>,
    current: Option<i64>,
) -> AppResult<()> {
    let Some(fid) = file_id else { return Ok(()) };
    if Some(fid) == current {
        return Ok(());
    }
    let ok: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM files WHERE id=$1 AND owner_id=$2 AND mime LIKE 'image/%'")
            .bind(fid)
            .bind(u.id)
            .fetch_optional(&st.db)
            .await?;
    ok.map(|_| ()).ok_or(bad(u.lang, ErrorKey::InvalidPhoto))
}

/// Replace an event's audience and its narrowed "joining as" choices. Runs on the caller's
/// transaction so a failure leaves the event's previous details and audience untouched.
async fn save_event_groups(
    tx: &mut sqlx::PgConnection,
    id: i64,
    req: &EventIn,
    lang: Lang,
) -> AppResult<()> {
    // Every audience group must still exist (one deleted since the form opened is a 400,
    // not a foreign-key 500). FOR SHARE keeps them from being deleted until we commit.
    let mut wanted = req.group_ids.clone();
    wanted.sort_unstable();
    wanted.dedup();
    let found: Vec<i64> = sqlx::query_scalar("SELECT id FROM groups WHERE id = ANY($1) FOR SHARE")
        .bind(&wanted)
        .fetch_all(&mut *tx)
        .await?;
    if found.len() != wanted.len() {
        return Err(bad(lang, ErrorKey::SelectVisibleGroup));
    }
    sqlx::query("DELETE FROM event_groups WHERE event_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO event_groups (event_id, group_id) SELECT $1, unnest($2::BIGINT[]) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(&wanted)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM event_attribution_groups WHERE event_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let narrowed = req.as_group_ids.clone().unwrap_or_default();
    // Only real, non-visitor groups.
    sqlx::query(
        "INSERT INTO event_attribution_groups (event_id, group_id)
         SELECT $1, g.id FROM groups g WHERE g.id = ANY($2) AND g.is_visitor = 0 ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(&narrowed)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

fn normalize_icon(raw: Option<&str>, lang: Lang) -> AppResult<Option<String>> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if s.chars().count() > 32 || s.chars().any(char::is_control) {
        return Err(bad(lang, ErrorKey::InvalidAction));
    }
    Ok(Some(s.to_string()))
}

fn validate(req: &EventIn, lang: Lang) -> AppResult<(String, Option<String>)> {
    if req.title.trim().is_empty() {
        return Err(bad(lang, ErrorKey::EventNameRequired));
    }
    if !KINDS.contains(&req.kind.as_str()) {
        return Err(bad(lang, ErrorKey::InvalidEventType));
    }
    if req.group_ids.is_empty() {
        return Err(bad(lang, ErrorKey::SelectVisibleGroup));
    }
    let at = norm_dt(&req.starts_at).ok_or_else(|| bad(lang, ErrorKey::InvalidEventTime))?;
    let dl = match req.signup_deadline.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => {
            let d = norm_dt(s).ok_or_else(|| bad(lang, ErrorKey::InvalidSignupDeadline))?;
            if d > at {
                return Err(bad(lang, ErrorKey::DeadlineAfterEvent));
            }
            Some(d)
        }
        None => None,
    };
    Ok((at, dl))
}

fn shift_occurrence(dt_str: &str, freq: &str, idx: i64) -> String {
    if idx <= 0 || freq == "none" {
        return dt_str.to_string();
    }
    let Some(dt) = crate::util::parse_dt(dt_str) else {
        return dt_str.to_string();
    };
    let shifted = match freq {
        "weekly" => dt + chrono::Duration::days(7 * idx),
        "biweekly" => dt + chrono::Duration::days(14 * idx),
        "monthly" => {
            let d = dt
                .date()
                .checked_add_months(chrono::Months::new(idx as u32))
                .unwrap_or(dt.date());
            chrono::NaiveDateTime::new(d, dt.time())
        }
        _ => dt,
    };
    shifted.format("%Y-%m-%dT%H:%M").to_string()
}

pub(super) async fn create_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<EventIn>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Events, Act::Create) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanCreate));
    }
    let status = parse_create_status(req.status.as_deref(), u.lang)?;
    let (at, dl) = validate(&req, u.lang)?;
    let icon = normalize_icon(req.icon.as_deref(), u.lang)?;
    check_photo(&st, &u, req.photo_file_id, None).await?;
    check_area(&st, &u, req.area_id).await?;
    let freq = req.repeat_frequency.as_deref().unwrap_or("none");
    if !matches!(freq, "none" | "weekly" | "biweekly" | "monthly") {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let repeat_count = if freq == "none" {
        1
    } else {
        req.repeat_count.unwrap_or(1).clamp(1, 12)
    };
    let series_id = if repeat_count > 1 {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        None
    };
    let att_enabled = req.attendance_enabled.unwrap_or(true);
    let mileage_claimable = req.mileage_claimable.unwrap_or(false);
    if mileage_claimable && !crate::perms::can_authorise_event_mileage(&u.roles) {
        return Err(forbidden(u.lang, ErrorKey::MileageAuthorisationRestricted));
    }

    let dl_delta = dl.as_deref().and_then(|d| {
        crate::util::parse_dt(d)
            .zip(crate::util::parse_dt(&at))
            .map(|(dd, ad)| ad - dd)
    });

    // The event(s) and their audience are saved together, or not at all.
    let mut tx = st.db.begin().await?;
    let mut created_ids = Vec::with_capacity(repeat_count as usize);
    for idx in 0..repeat_count {
        let occ_at = shift_occurrence(&at, freq, idx);
        let occ_dl = match (dl_delta, crate::util::parse_dt(&occ_at)) {
            (Some(delta), Some(occ_dt)) => {
                Some((occ_dt - delta).format("%Y-%m-%dT%H:%M").to_string())
            }
            _ => None,
        };
        let checkin_code = if att_enabled {
            Some(random_token(10))
        } else {
            None
        };
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO events (title, kind, description, starts_at, location, capacity, cost_note, signup_deadline, status, created_by, created_at, photo_file_id, attendance_enabled, checkin_code, series_id, mileage_claimable, area_id, icon)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) RETURNING id",
        )
        .bind(req.title.trim())
        .bind(&req.kind)
        .bind(req.description.clone().unwrap_or_default())
        .bind(&occ_at)
        .bind(req.location.trim())
        .bind(req.capacity.unwrap_or(0).max(0))
        .bind(req.cost_note.clone().unwrap_or_default())
        .bind(&occ_dl)
        .bind(status)
        .bind(u.id)
        .bind(now())
        .bind(req.photo_file_id)
        .bind(att_enabled as i64)
        .bind(checkin_code)
        .bind(&series_id)
        .bind(mileage_claimable as i64)
        .bind(req.area_id)
        .bind(&icon)
        .fetch_one(&mut *tx)
        .await?;
        save_event_groups(&mut tx, id, &req, u.lang).await?;
        created_ids.push(id);
    }
    tx.commit().await?;
    let first_id = created_ids[0];
    if status == "published" {
        // Invite members of the target groups and their sub-groups (相关活动邀请).
        let members: Vec<(i64,)> = sqlx::query_as(sqlx::AssertSqlSafe(audience_users_sql("$1")))
            .bind(first_id)
            .fetch_all(&st.db)
            .await?;
        let invited: Vec<i64> = members
            .into_iter()
            .map(|(m,)| m)
            .filter(|m| *m != u.id)
            .collect();
        let (body_zh, body_en) = if repeat_count > 1 {
            (
                format!(
                    "{} · {}（共 {} 场）",
                    req.title.trim(),
                    at.replace('T', " "),
                    repeat_count
                ),
                format!(
                    "{} · {} ({} sessions)",
                    req.title.trim(),
                    at.replace('T', " "),
                    repeat_count
                ),
            )
        } else {
            let single = format!("{} · {}", req.title.trim(), at.replace('T', " "));
            (single.clone(), single)
        };
        notify_many(
            &st.db,
            &invited,
            ("新的活动邀请", "New event invitation"),
            (&body_zh, &body_en),
            &format!("/events/{first_id}"),
        )
        .await;
    }
    let audit_detail = if status == "draft" {
        format!("{} (draft)", req.title.trim())
    } else {
        req.title.trim().to_string()
    };
    for &eid in &created_ids {
        audit(
            &st.db,
            Some(u.id),
            "event.create",
            "event",
            Some(eid),
            &audit_detail,
        )
        .await;
    }
    Ok(Json(json!({
        "id": first_id,
        "created_ids": created_ids,
        "series_id": series_id,
        "status": status,
    })))
}

#[derive(Deserialize)]
pub(super) struct DuplicateIn {
    starts_at: String,
    signup_deadline: Option<String>,
    title: Option<String>,
    status: Option<String>,
}

pub(super) async fn duplicate_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<DuplicateIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Events, Act::Create)?;
    let status = parse_create_status(req.status.as_deref(), u.lang)?;
    let src = load_visible_event(&st, &u, id).await?;
    let at = norm_dt(&req.starts_at).ok_or_else(|| bad(u.lang, ErrorKey::InvalidEventTime))?;
    let dl = match req.signup_deadline.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => {
            let d = norm_dt(s).ok_or_else(|| bad(u.lang, ErrorKey::InvalidSignupDeadline))?;
            if d > at {
                return Err(bad(u.lang, ErrorKey::DeadlineAfterEvent));
            }
            Some(d)
        }
        None => None,
    };
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&src.title)
        .to_string();
    let att_enabled = src.attendance_enabled == 1;
    let can_auth = crate::perms::can_authorise_event_mileage(&u.roles);
    let mileage_claimable = if can_auth {
        src.mileage_claimable == 1
    } else {
        false
    };
    let checkin_code = if att_enabled {
        Some(random_token(10))
    } else {
        None
    };
    let photo_file_id = if let Some(fid) = src.photo_file_id {
        let owner: Option<i64> = sqlx::query_scalar("SELECT owner_id FROM files WHERE id=$1")
            .bind(fid)
            .fetch_optional(&st.db)
            .await?;
        if owner == Some(u.id) {
            Some(fid)
        } else {
            None
        }
    } else {
        None
    };

    let mut tx = st.db.begin().await?;
    let (new_id,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, kind, description, starts_at, location, capacity, cost_note, signup_deadline, status, created_by, created_at, photo_file_id, attendance_enabled, checkin_code, mileage_claimable, area_id, icon)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17) RETURNING id",
    )
    .bind(&title)
    .bind(&src.kind)
    .bind(&src.description)
    .bind(&at)
    .bind(&src.location)
    .bind(src.capacity)
    .bind(&src.cost_note)
    .bind(&dl)
    .bind(status)
    .bind(u.id)
    .bind(now())
    .bind(photo_file_id)
    .bind(att_enabled as i64)
    .bind(checkin_code)
    .bind(mileage_claimable as i64)
    .bind(src.area_id)
    .bind(&src.icon)
    .fetch_one(&mut *tx)
    .await?;
    let groups_copied = sqlx::query(
        "INSERT INTO event_groups (event_id, group_id)
         SELECT $1, g.id
         FROM event_groups eg JOIN groups g ON g.id = eg.group_id
         WHERE eg.event_id=$2
         FOR SHARE OF g
         ON CONFLICT DO NOTHING",
    )
    .bind(new_id)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if groups_copied.rows_affected() == 0 {
        return Err(bad(u.lang, ErrorKey::SelectVisibleGroup));
    }
    sqlx::query(
        "INSERT INTO event_attribution_groups (event_id, group_id)
         SELECT $1, group_id FROM event_attribution_groups WHERE event_id=$2 ON CONFLICT DO NOTHING",
    )
    .bind(new_id)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    if status == "published" {
        let members: Vec<(i64,)> = sqlx::query_as(sqlx::AssertSqlSafe(audience_users_sql("$1")))
            .bind(new_id)
            .fetch_all(&st.db)
            .await?;
        let invited: Vec<i64> = members
            .into_iter()
            .map(|(m,)| m)
            .filter(|m| *m != u.id)
            .collect();
        let body = format!("{title} · {}", at.replace('T', " "));
        notify_many(
            &st.db,
            &invited,
            ("新的活动邀请", "New event invitation"),
            (&body, &body),
            &format!("/events/{new_id}"),
        )
        .await;
    }

    audit(
        &st.db,
        Some(u.id),
        "event.create",
        "event",
        Some(new_id),
        format!(
            "{title} (from #{id}{})",
            if status == "draft" { ", draft" } else { "" }
        ),
    )
    .await;
    Ok(Json(json!({ "id": new_id, "status": status })))
}

pub(super) async fn update_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<EventIn>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanEdit));
    }
    let (at, dl) = validate(&req, u.lang)?;
    let icon = normalize_icon(req.icon.as_deref(), u.lang)?;
    check_photo(&st, &u, req.photo_file_id, e.photo_file_id).await?;
    check_area(&st, &u, req.area_id).await?;
    // Same event lock as `signup` / `withdraw`: places a raised capacity opens go to the
    // waitlist, in order, before any signup queued behind this save can take them.
    let mut tx = st.db.begin().await?;
    let (event_status,): (String,) =
        sqlx::query_as("SELECT status FROM events WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    // A cancelled event stays cancelled: editing it would re-notify members and move its
    // (already cancelled) rides.
    if event_status == "cancelled" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    let capacity = req.capacity.unwrap_or(0).max(0);
    let att_enabled = req.attendance_enabled.unwrap_or(e.attendance_enabled == 1);
    let can_auth = crate::perms::can_authorise_event_mileage(&u.roles);
    let target_mileage = req.mileage_claimable.unwrap_or(e.mileage_claimable == 1);
    if target_mileage != (e.mileage_claimable == 1) && !can_auth {
        return Err(forbidden(u.lang, ErrorKey::MileageAuthorisationRestricted));
    }
    let checkin_code = if att_enabled && e.checkin_code.is_none() {
        Some(random_token(10))
    } else {
        e.checkin_code.clone()
    };
    sqlx::query("UPDATE events SET title=$1, kind=$2, description=$3, starts_at=$4, location=$5, capacity=$6, cost_note=$7, signup_deadline=$8, photo_file_id=$9, attendance_enabled=$10, checkin_code=$11, mileage_claimable=$12, area_id=$13, icon=$14 WHERE id=$15")
        .bind(req.title.trim())
        .bind(&req.kind)
        .bind(req.description.clone().unwrap_or_default())
        .bind(&at)
        .bind(req.location.trim())
        .bind(capacity)
        .bind(req.cost_note.clone().unwrap_or_default())
        .bind(&dl)
        .bind(req.photo_file_id)
        .bind(att_enabled as i64)
        .bind(checkin_code)
        .bind(target_mileage as i64)
        .bind(req.area_id)
        .bind(&icon)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // Details, audience and waitlist promotions commit together: a failed save (say, a
    // group deleted since the form opened) must not keep the new venue and drop the audience.
    save_event_groups(&mut tx, id, &req, u.lang).await?;
    let promoted = promote_waitlist(&mut tx, id, capacity, &event_status).await?;
    tx.commit().await?;
    notify_promoted(&st, id, req.title.trim(), &promoted).await;
    if event_status == "published" && (at != e.starts_at || req.location.trim() != e.location) {
        let body = format!(
            "{} · {} · {}",
            req.title.trim(),
            at.replace('T', " "),
            req.location.trim()
        );
        notify_signups(
            &st,
            id,
            ("活动资料已更新", "Event details updated"),
            (&body, &body),
        )
        .await?;
    }
    if event_status == "published" {
        crate::routes::rides::event_moved(
            &st,
            id,
            (&e.starts_at, &at),
            (&e.location, req.location.trim()),
        )
        .await?;
    }
    let mut detail = req.title.trim().to_string();
    if at != e.starts_at {
        detail.push_str(&format!("; time {} → {at}", e.starts_at));
    }
    if req.location.trim() != e.location {
        detail.push_str(&format!(
            "; location {} → {}",
            e.location,
            req.location.trim()
        ));
    }
    if req.photo_file_id != e.photo_file_id {
        detail.push_str(match req.photo_file_id {
            Some(_) => "; photo changed",
            None => "; photo removed",
        });
    }
    audit(
        &st.db,
        Some(u.id),
        "event.update",
        "event",
        Some(id),
        detail,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn notify_signups(
    st: &AppState,
    id: i64,
    title: (&str, &str),
    body: (&str, &str),
) -> AppResult<()> {
    let users: Vec<(i64,)> = sqlx::query_as(
        "SELECT user_id FROM event_signups WHERE event_id=$1 AND status IN ('going','waitlist')",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    let ids: Vec<i64> = users.into_iter().map(|(uid,)| uid).collect();
    notify_many(&st.db, &ids, title, body, &format!("/events/{id}")).await;
    Ok(())
}

pub(super) async fn publish_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanPerformAction));
    }
    let mut tx = st.db.begin().await?;
    let (status, title, starts_at): (String, String, String) =
        sqlx::query_as("SELECT status, title, starts_at FROM events WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if status != "draft" {
        return Err(bad(u.lang, ErrorKey::EventNotDraft));
    }
    sqlx::query("UPDATE events SET status='published' WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let members: Vec<(i64,)> = sqlx::query_as(sqlx::AssertSqlSafe(audience_users_sql("$1")))
        .bind(id)
        .fetch_all(&st.db)
        .await?;
    let invited: Vec<i64> = members
        .into_iter()
        .map(|(m,)| m)
        .filter(|m| *m != u.id)
        .collect();
    let body = format!("{title} · {}", starts_at.replace('T', " "));
    notify_many(
        &st.db,
        &invited,
        ("新的活动邀请", "New event invitation"),
        (&body, &body),
        &format!("/events/{id}"),
    )
    .await;

    audit(
        &st.db,
        Some(u.id),
        "event.publish",
        "event",
        Some(id),
        &title,
    )
    .await;
    Ok(Json(json!({"ok": true, "status": "published"})))
}

pub(super) async fn unpublish_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanPerformAction));
    }
    let mut tx = st.db.begin().await?;
    let (status, title): (String, String) =
        sqlx::query_as("SELECT status, title FROM events WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if status != "published" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    let (active_signups,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND status IN ('going','waitlist')",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let (active_rides,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rides WHERE event_id=$1 AND status != 'cancelled'")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let (settlements,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM settlements WHERE event_id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if active_signups > 0 || active_rides > 0 || settlements > 0 {
        return Err(conflict(u.lang, ErrorKey::EventCannotHideWithSignups));
    }
    sqlx::query("UPDATE events SET status='draft' WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "event.unpublish",
        "event",
        Some(id),
        &title,
    )
    .await;
    Ok(Json(json!({"ok": true, "status": "draft"})))
}

pub(super) async fn cancel_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Delete) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanCancel));
    }
    // The event and the ride requests not yet underway are cancelled together, or not at all.
    let mut tx = st.db.begin().await?;
    let prev_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM events WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    if !matches!(prev_status.as_deref(), Some("published" | "draft")) {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    sqlx::query("UPDATE events SET status='cancelled' WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let rides: Vec<(i64, Option<i64>, Option<i64>)> = sqlx::query_as(
        "UPDATE rides SET status='cancelled', needs_attention=0, attention_reason='活动已取消', updated_at=$2
         WHERE event_id=$1 AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')
         RETURNING id, driver_id, trip_id",
    )
    .bind(id)
    .bind(now())
    .fetch_all(&mut *tx)
    .await?;
    let ride_ids: Vec<i64> = rides.iter().map(|(rid, _, _)| *rid).collect();
    audit_many(
        &mut *tx,
        Some(u.id),
        "ride.cancel",
        "ride",
        &ride_ids,
        &format!("event {id} cancelled"),
    )
    .await?;
    tx.commit().await?;

    if prev_status.as_deref() == Some("published") {
        notify_signups(
            &st,
            id,
            ("活动已取消", "Event cancelled"),
            (e.title.as_str(), e.title.as_str()),
        )
        .await?;
    }
    let mut driver_ids: Vec<i64> = Vec::new();
    for (rid, d, t) in rides {
        if let Some(d) = d {
            st.routing.invalidate(d, t.unwrap_or(rid)).await;
            if !driver_ids.contains(&d) {
                driver_ids.push(d);
            }
        }
    }
    notify_many(
        &st.db,
        &driver_ids,
        (
            "活动已取消，接送随之取消",
            "Event cancelled, ride cancelled with it",
        ),
        (e.title.as_str(), e.title.as_str()),
        "/drive",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "event.cancel",
        "event",
        Some(id),
        &e.title,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn finish_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanPerformAction));
    }
    let finished =
        sqlx::query("UPDATE events SET status='finished' WHERE id=$1 AND status='published'")
            .bind(id)
            .execute(&st.db)
            .await?;
    if finished.rows_affected() == 0 {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    audit(&st.db, Some(u.id), "event.finish", "event", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}
