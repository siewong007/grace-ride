//! 运动与团契活动: group-targeted events, signup / waitlist, ride link, attendance check-in.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult, not_found};
use crate::state::AppState;
use crate::util::{audit, norm_dt, notify, now, t, Lang};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/groups", get(list_groups))
        .route("/events", get(list_events).post(create_event))
        .route("/events/{id}", get(get_event).put(update_event))
        .route("/events/{id}/cancel", post(cancel_event))
        .route("/events/{id}/finish", post(finish_event))
        .route("/events/{id}/signup", post(signup))
        .route("/events/{id}/withdraw", post(withdraw))
        .route("/events/{id}/signups", get(list_signups))
        .route("/events/{id}/attendance", post(mark_attendance))
}

pub const KINDS: &[&str] = &["badminton", "bowling", "kayak", "walk", "other"];

/// A member sees an event if it targets one of their groups (or the 全体会员 group),
/// or if they created it / are staff.
pub async fn can_see_event(st: &AppState, u: &CurrentUser, event_id: i64) -> AppResult<bool> {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM events e WHERE e.id = $1 AND (
            e.created_by = $2 OR EXISTS (
              SELECT 1 FROM event_groups eg JOIN groups g ON g.id = eg.group_id
              WHERE eg.event_id = e.id AND (g.is_all = 1 OR EXISTS (SELECT 1 FROM group_members gm WHERE gm.group_id = g.id AND gm.user_id = $3))
            ))",
    )
    .bind(event_id)
    .bind(u.id)
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    Ok(n > 0 || (u.has("admin") && u.mfa_ok))
}

fn can_manage(u: &CurrentUser, created_by: i64) -> bool {
    created_by == u.id || (u.has("admin") && u.mfa_ok)
}

async fn list_groups(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    // Leaders choose target groups, so they see all group names (not member lists).
    let rows: Vec<(i64, String, String, i64, i64)> = if u.has("leader") || u.has("admin") {
        sqlx::query_as("SELECT g.id, g.name, g.description, g.is_all, (SELECT COUNT(*) FROM group_members m WHERE m.group_id=g.id) FROM groups g ORDER BY g.is_all DESC, g.name")
            .fetch_all(&st.db)
            .await?
    } else {
        sqlx::query_as(
            "SELECT g.id, g.name, g.description, g.is_all, 0 FROM groups g WHERE g.is_all=1 OR g.id IN (SELECT group_id FROM group_members WHERE user_id=$1) ORDER BY g.is_all DESC, g.name",
        )
        .bind(u.id)
        .fetch_all(&st.db)
        .await?
    };
    Ok(Json(json!(rows.into_iter().map(|(id, name, d, all, n)| json!({"id": id, "name": name, "description": d, "is_all": all == 1, "member_count": n})).collect::<Vec<_>>())))
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: i64,
    title: String,
    kind: String,
    description: String,
    starts_at: String,
    location: String,
    capacity: i64,
    cost_note: String,
    signup_deadline: Option<String>,
    status: String,
    created_by: i64,
    creator_name: String,
    going: i64,
    waitlist: i64,
    my_status: Option<String>,
    my_need_ride: Option<i64>,
    my_offer_drive: Option<i64>,
    my_attended: Option<i64>,
    groups: Option<String>,
}

const EVENT_SELECT: &str = "SELECT e.id, e.title, e.kind, e.description, e.starts_at, e.location, e.capacity, e.cost_note,
    e.signup_deadline, e.status, e.created_by, c.name AS creator_name,
    (SELECT COUNT(*) FROM event_signups s WHERE s.event_id=e.id AND s.status='going') AS going,
    (SELECT COUNT(*) FROM event_signups s WHERE s.event_id=e.id AND s.status='waitlist') AS waitlist,
    ms.status AS my_status, ms.need_ride AS my_need_ride, ms.offer_drive AS my_offer_drive, ms.attended AS my_attended,
    (SELECT string_agg(g.name, '、') FROM event_groups eg JOIN groups g ON g.id=eg.group_id WHERE eg.event_id=e.id) AS groups
    FROM events e JOIN users c ON c.id = e.created_by
    LEFT JOIN event_signups ms ON ms.event_id = e.id AND ms.user_id = $1";

fn event_json(e: &EventRow, u: &CurrentUser) -> Value {
    json!({
        "id": e.id, "title": e.title, "kind": e.kind, "description": e.description, "starts_at": e.starts_at,
        "location": e.location, "capacity": e.capacity, "cost_note": e.cost_note, "signup_deadline": e.signup_deadline,
        "status": e.status, "created_by": {"id": e.created_by, "name": e.creator_name},
        "going": e.going, "waitlist": e.waitlist,
        "spots_left": if e.capacity > 0 { Some((e.capacity - e.going).max(0)) } else { None },
        "my_signup": e.my_status.as_ref().map(|s| json!({"status": s, "need_ride": e.my_need_ride == Some(1), "offer_drive": e.my_offer_drive == Some(1), "attended": e.my_attended})),
        "groups": e.groups.clone().unwrap_or_default(),
        "can_manage": can_manage(u, e.created_by),
    })
}

#[derive(Deserialize)]
struct ListQuery {
    scope: Option<String>, // upcoming | past | managing
}

async fn list_events(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let visibility = "(e.created_by = $1 OR EXISTS (SELECT 1 FROM event_groups eg JOIN groups g ON g.id = eg.group_id
        WHERE eg.event_id = e.id AND (g.is_all = 1 OR EXISTS (SELECT 1 FROM group_members gm WHERE gm.group_id = g.id AND gm.user_id = $2))))";
    let sql = match q.scope.as_deref() {
        Some("past") => format!("{EVENT_SELECT} WHERE {visibility} AND (e.starts_at < $3 OR e.status='finished') ORDER BY e.starts_at DESC LIMIT 50"),
        Some("managing") => format!("{EVENT_SELECT} WHERE e.created_by = $1 ORDER BY e.starts_at DESC LIMIT 100"),
        _ => format!("{EVENT_SELECT} WHERE {visibility} AND e.starts_at >= $3 AND e.status = 'published' ORDER BY e.starts_at LIMIT 100"),
    };
    let cutoff = now()[..16].to_string();
    let rows = if q.scope.as_deref() == Some("managing") {
        sqlx::query_as::<_, EventRow>(&sql)
            .bind(u.id)
            .fetch_all(&st.db)
            .await?
    } else {
        sqlx::query_as::<_, EventRow>(&sql)
            .bind(u.id)
            .bind(u.id)
            .bind(&cutoff)
            .fetch_all(&st.db)
            .await?
    };
    Ok(Json(json!(rows
        .iter()
        .map(|e| event_json(e, &u))
        .collect::<Vec<_>>())))
}

async fn load_event(st: &AppState, u: &CurrentUser, id: i64) -> AppResult<EventRow> {
    sqlx::query_as::<_, EventRow>(&format!("{EVENT_SELECT} WHERE e.id = $2"))
        .bind(u.id)
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))
}

async fn get_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    if !can_see_event(&st, &u, id).await? {
        return Err(forbidden(t(u.lang, "此活动不对您所属的群体开放", "This event is not open to your groups")));
    }
    let e = load_event(&st, &u, id).await?;
    let mut v = event_json(&e, &u);
    let gids: Vec<(i64,)> = sqlx::query_as("SELECT group_id FROM event_groups WHERE event_id=$1")
        .bind(id)
        .fetch_all(&st.db)
        .await?;
    v["group_ids"] = json!(gids.into_iter().map(|g| g.0).collect::<Vec<_>>());
    let my_ride: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, status FROM rides WHERE event_id=$1 AND passenger_id=$2 AND status NOT IN ('cancelled') ORDER BY id DESC LIMIT 1",
    )
    .bind(id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    v["my_ride"] = json!(my_ride.map(|(i, s)| json!({"id": i, "status": s})));
    let settlement: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, status FROM settlements WHERE event_id=$1 ORDER BY id DESC LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    v["settlement"] = json!(settlement.map(|(i, s)| json!({"id": i, "status": s})));
    Ok(Json(v))
}

#[derive(Deserialize)]
struct EventIn {
    title: String,
    kind: String,
    description: Option<String>,
    starts_at: String,
    location: String,
    capacity: Option<i64>,
    cost_note: Option<String>,
    signup_deadline: Option<String>,
    group_ids: Vec<i64>,
}

fn validate(req: &EventIn, lang: Lang) -> AppResult<(String, Option<String>)> {
    if req.title.trim().is_empty() {
        return Err(bad(t(lang, "请填写活动名称", "Please enter an event name")));
    }
    if !KINDS.contains(&req.kind.as_str()) {
        return Err(bad(t(lang, "活动类型无效", "Invalid event type")));
    }
    if req.group_ids.is_empty() {
        return Err(bad(t(lang, "请至少选择一个可见群体", "Please select at least one visible group")));
    }
    let at = norm_dt(&req.starts_at).ok_or_else(|| bad(t(lang, "请填写正确的活动时间", "Please enter a valid event time")))?;
    let dl = match req.signup_deadline.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => {
            let d = norm_dt(s).ok_or_else(|| bad(t(lang, "报名截止时间格式不正确", "Signup deadline format is invalid")))?;
            if d > at {
                return Err(bad(t(lang, "报名截止不能晚于活动时间", "Signup deadline cannot be later than the event time")));
            }
            Some(d)
        }
        None => None,
    };
    Ok((at, dl))
}

async fn create_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<EventIn>,
) -> AppResult<Json<Value>> {
    if !u.has("leader") {
        return Err(forbidden(t(u.lang, "只有活动负责人可以发起活动", "Only event organisers can create events")));
    }
    let (at, dl) = validate(&req, u.lang)?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, kind, description, starts_at, location, capacity, cost_note, signup_deadline, created_by, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
    )
    .bind(req.title.trim())
    .bind(&req.kind)
    .bind(req.description.clone().unwrap_or_default())
    .bind(&at)
    .bind(req.location.trim())
    .bind(req.capacity.unwrap_or(0).max(0))
    .bind(req.cost_note.clone().unwrap_or_default())
    .bind(&dl)
    .bind(u.id)
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    for g in &req.group_ids {
        sqlx::query(
            "INSERT INTO event_groups (event_id, group_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(g)
        .execute(&st.db)
        .await?;
    }
    // Invite members of the target groups (相关活动邀请).
    let members: Vec<(i64,)> = sqlx::query_as(
        "SELECT DISTINCT u.id FROM users u WHERE u.status='active' AND u.id != $1 AND (
            EXISTS (SELECT 1 FROM event_groups eg JOIN groups g ON g.id=eg.group_id WHERE eg.event_id=$2 AND g.is_all=1)
            OR u.id IN (SELECT gm.user_id FROM group_members gm JOIN event_groups eg ON eg.group_id=gm.group_id WHERE eg.event_id=$3))",
    )
    .bind(u.id)
    .bind(id)
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    for (m,) in members {
        let body = format!("{} · {}", req.title.trim(), at.replace('T', " "));
        notify(
            &st.db,
            m,
            ("新的活动邀请", "New event invitation"),
            (&body, &body),
            &format!("/events/{id}"),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "event.create",
        "event",
        Some(id),
        req.title.trim(),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

async fn update_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<EventIn>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by) {
        return Err(forbidden(t(u.lang, "只有活动负责人可以修改", "Only the event organiser can edit")));
    }
    let (at, dl) = validate(&req, u.lang)?;
    sqlx::query("UPDATE events SET title=$1, kind=$2, description=$3, starts_at=$4, location=$5, capacity=$6, cost_note=$7, signup_deadline=$8 WHERE id=$9")
        .bind(req.title.trim())
        .bind(&req.kind)
        .bind(req.description.clone().unwrap_or_default())
        .bind(&at)
        .bind(req.location.trim())
        .bind(req.capacity.unwrap_or(0).max(0))
        .bind(req.cost_note.clone().unwrap_or_default())
        .bind(&dl)
        .bind(id)
        .execute(&st.db)
        .await?;
    sqlx::query("DELETE FROM event_groups WHERE event_id=$1")
        .bind(id)
        .execute(&st.db)
        .await?;
    for g in &req.group_ids {
        sqlx::query(
            "INSERT INTO event_groups (event_id, group_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(g)
        .execute(&st.db)
        .await?;
    }
    if at != e.starts_at || req.location.trim() != e.location {
        let body = format!("{} · {} · {}", req.title.trim(), at.replace('T', " "), req.location.trim());
        notify_signups(
            &st,
            id,
            ("活动资料已更新", "Event details updated"),
            (&body, &body),
        )
        .await?;
    }
    audit(&st.db, Some(u.id), "event.update", "event", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

async fn notify_signups(st: &AppState, id: i64, title: (&str, &str), body: (&str, &str)) -> AppResult<()> {
    let users: Vec<(i64,)> = sqlx::query_as(
        "SELECT user_id FROM event_signups WHERE event_id=$1 AND status IN ('going','waitlist')",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    for (uid,) in users {
        notify(&st.db, uid, title, body, &format!("/events/{id}")).await;
    }
    Ok(())
}

async fn cancel_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by) {
        return Err(forbidden(t(u.lang, "只有活动负责人可以取消", "Only the event organiser can cancel")));
    }
    sqlx::query("UPDATE events SET status='cancelled' WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await?;
    notify_signups(&st, id, ("活动已取消", "Event cancelled"), (e.title.as_str(), e.title.as_str())).await?;
    // Release linked ride requests that are not yet underway.
    let rides: Vec<(i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, driver_id FROM rides WHERE event_id=$1 AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    for (rid, d) in rides {
        sqlx::query("UPDATE rides SET status='cancelled', attention_reason='活动已取消', updated_at=$1 WHERE id=$2").bind(now()).bind(rid).execute(&st.db).await?;
        if let Some(d) = d {
            notify(
                &st.db,
                d,
                ("活动已取消，接送随之取消", "Event cancelled, ride cancelled with it"),
                (e.title.as_str(), e.title.as_str()),
                "/drive",
            )
            .await;
        }
    }
    audit(&st.db, Some(u.id), "event.cancel", "event", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

async fn finish_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by) {
        return Err(forbidden(t(u.lang, "只有活动负责人可以操作", "Only the event organiser can perform this action")));
    }
    sqlx::query("UPDATE events SET status='finished' WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "event.finish", "event", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct SignupIn {
    need_ride: Option<bool>,
    offer_drive: Option<bool>,
    offer_seats: Option<i64>,
}

async fn signup(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SignupIn>,
) -> AppResult<Json<Value>> {
    if !can_see_event(&st, &u, id).await? {
        return Err(forbidden(t(u.lang, "此活动不对您所属的群体开放", "This event is not open to your groups")));
    }
    let e = load_event(&st, &u, id).await?;
    if e.status != "published" {
        return Err(bad(t(u.lang, "此活动不开放报名", "This event is not open for signup")));
    }
    let n = now()[..16].to_string();
    if e.signup_deadline
        .as_deref()
        .map(|d| d < n.as_str())
        .unwrap_or(false)
        || e.starts_at < n
    {
        return Err(bad(t(u.lang, "报名已截止", "Signup has closed")));
    }
    if req.offer_drive.unwrap_or(false) && !u.has("driver") {
        return Err(bad(t(u.lang, "需由交通组登记为志愿司机后才能提供接送", "You must be registered by the transport team as a volunteer driver to offer rides")));
    }
    let already_going = e.my_status.as_deref() == Some("going");
    let status = if already_going || e.capacity == 0 || e.going < e.capacity {
        "going"
    } else {
        "waitlist"
    };
    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)
         ON CONFLICT(event_id, user_id) DO UPDATE SET status=excluded.status, need_ride=excluded.need_ride, offer_drive=excluded.offer_drive, offer_seats=excluded.offer_seats",
    )
    .bind(id)
    .bind(u.id)
    .bind(status)
    .bind(req.need_ride.unwrap_or(false) as i64)
    .bind(req.offer_drive.unwrap_or(false) as i64)
    .bind(req.offer_seats.unwrap_or(0))
    .bind(now())
    .execute(&st.db)
    .await?;
    audit(
        &st.db,
        Some(u.id),
        "event.signup",
        "event",
        Some(id),
        status,
    )
    .await;
    Ok(Json(json!({"ok": true, "status": status})))
}

/// 乘客无法出席: cancel signup → release seat → promote waitlist → cancel linked rides → tell driver & organiser.
async fn withdraw(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    let was_going = e.my_status.as_deref() == Some("going");
    sqlx::query("UPDATE event_signups SET status='cancelled' WHERE event_id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if was_going && e.capacity > 0 {
        let next: Option<(i64, i64)> = sqlx::query_as("SELECT id, user_id FROM event_signups WHERE event_id=$1 AND status='waitlist' ORDER BY created_at, id LIMIT 1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
        if let Some((sid, uid)) = next {
            sqlx::query("UPDATE event_signups SET status='going' WHERE id=$1")
                .bind(sid)
                .execute(&st.db)
                .await?;
            let zh = format!("您已获得「{}」的名额", e.title);
            let en = format!("You now have a spot in \"{}\"", e.title);
            notify(
                &st.db,
                uid,
                ("候补成功", "Off the waitlist"),
                (&zh, &en),
                &format!("/events/{id}"),
            )
            .await;
        }
    }
    let rides: Vec<(i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, driver_id FROM rides WHERE event_id=$1 AND passenger_id=$2 AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')",
    )
    .bind(id)
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    for (rid, d) in rides {
        sqlx::query("UPDATE rides SET status='cancelled', attention_reason='乘客取消活动报名', updated_at=$1 WHERE id=$2").bind(now()).bind(rid).execute(&st.db).await?;
        if let Some(d) = d {
            let body = format!("{} · {}", u.name, e.title);
            notify(
                &st.db,
                d,
                ("乘客已取消，座位已释放", "Passenger cancelled, seat released"),
                (&body, &body),
                "/drive",
            )
            .await;
        }
    }
    if e.created_by != u.id {
        let body = format!("{} · {}", u.name, e.title);
        notify(
            &st.db,
            e.created_by,
            ("有会员取消报名", "A member withdrew their signup"),
            (&body, &body),
            &format!("/events/{id}"),
        )
        .await;
    }
    audit(&st.db, Some(u.id), "event.withdraw", "event", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

/// Signup list is visible only to the organiser (不公开全体报名名单).
async fn list_signups(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by) {
        return Err(forbidden(t(u.lang, "只有活动负责人可以查看报名名单", "Only the event organiser can view the signup list")));
    }
    let rows: Vec<(i64, i64, String, String, i64, i64, i64, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT s.id, s.user_id, u.name, s.status, s.need_ride, s.offer_drive, s.offer_seats, s.attended,
            (SELECT r.status FROM rides r WHERE r.event_id = s.event_id AND r.passenger_id = s.user_id AND r.status != 'cancelled' ORDER BY r.id DESC LIMIT 1)
         FROM event_signups s JOIN users u ON u.id = s.user_id WHERE s.event_id=$1 ORDER BY s.status, s.created_at",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(sid, uid, name, status, nr, od, os, att, ride)| json!({
        "id": sid, "user_id": uid, "name": name, "status": status, "need_ride": nr == 1, "offer_drive": od == 1,
        "offer_seats": os, "attended": att, "ride_status": ride,
    })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct AttendanceIn {
    user_id: i64,
    attended: Option<bool>,
}

async fn mark_attendance(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<AttendanceIn>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by) {
        return Err(forbidden(t(u.lang, "只有活动负责人可以记录出席", "Only the event organiser can record attendance")));
    }
    let v = req.attended.map(|a| a as i64);
    let r = sqlx::query("UPDATE event_signups SET attended=$1 WHERE event_id=$2 AND user_id=$3")
        .bind(v)
        .bind(id)
        .bind(req.user_id)
        .execute(&st.db)
        .await?;
    if r.rows_affected() == 0 {
        // Walk-in: organiser may record someone who attended without signing up.
        sqlx::query("INSERT INTO event_signups (event_id, user_id, status, attended, created_at) VALUES ($1,$2, 'going', $3, $4)")
            .bind(id)
            .bind(req.user_id)
            .bind(v)
            .bind(now())
            .execute(&st.db)
            .await?;
    }
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
