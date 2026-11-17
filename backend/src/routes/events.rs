//! 运动与团契活动: group-targeted events, signup / waitlist, ride link, attendance check-in.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audience::{audience_users_sql, join_choices, visible_events_sql};
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, audit_many, norm_dt, notify, notify_many, now, require, Lang};

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

/// A member sees an event if it targets one of their groups or a group above one of
/// theirs (or the 全体会员 group), or if they created it / are staff. See `crate::audience`.
pub async fn can_see_event(st: &AppState, u: &CurrentUser, event_id: i64) -> AppResult<bool> {
    let (n,): (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM events e WHERE e.id = $1 AND {}",
        visible_events_sql("$2", "$3")
    ))
    .bind(event_id)
    .bind(u.id)
    .bind(&u.group_scope)
    .fetch_one(&st.db)
    .await?;
    Ok((n > 0 && u.can(Page::Events, Act::View)) || u.can(Page::AdminEvents, Act::View))
}

/// The organiser may change their own event while they hold the matching 活动
/// permission; 全部活动 (admin_events) reaches every event.
pub(crate) fn can_manage(u: &CurrentUser, created_by: i64, act: Act) -> bool {
    (created_by == u.id && u.can(Page::Events, act)) || u.can(Page::AdminEvents, act)
}

/// An organiser always sees who signed up for their own event.
fn can_see_signups(u: &CurrentUser, created_by: i64) -> bool {
    created_by == u.id || u.can(Page::AdminEvents, Act::View)
}

async fn list_groups(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    // Leaders choose target groups, so they see all group names (not member lists).
    let all = [
        (Page::Events, Act::Create),
        (Page::Events, Act::Update),
        (Page::AdminEvents, Act::View),
        (Page::AdminGroups, Act::View),
        (Page::AdminMembers, Act::View),
    ]
    .iter()
    .any(|(p, a)| u.can(*p, *a));
    type GroupRow = (i64, String, String, i64, i64, Option<i64>, i64);
    let rows: Vec<GroupRow> = if all {
        sqlx::query_as("SELECT g.id, g.name, g.description, g.is_all, (SELECT COUNT(*) FROM group_members m WHERE m.group_id=g.id), g.parent_id, g.is_visitor FROM groups g ORDER BY g.is_all DESC, g.name")
            .fetch_all(&st.db)
            .await?
    } else {
        sqlx::query_as(
            "SELECT g.id, g.name, g.description, g.is_all, 0::BIGINT, g.parent_id, g.is_visitor FROM groups g WHERE g.is_all=1 OR g.id = ANY($1) ORDER BY g.is_all DESC, g.name",
        )
        .bind(&u.group_scope)
        .fetch_all(&st.db)
        .await?
    };
    Ok(Json(json!(rows.into_iter().map(|(id, name, d, all, n, parent, visitor)| json!({"id": id, "name": name, "description": d, "is_all": all == 1, "member_count": n, "parent_id": parent, "is_visitor": visitor == 1})).collect::<Vec<_>>())))
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
    my_as_group_id: Option<i64>,
    my_as_group: Option<String>,
    groups: Option<String>,
    photo_file_id: Option<i64>,
}

const EVENT_SELECT: &str = "SELECT e.id, e.title, e.kind, e.description, e.starts_at, e.location, e.capacity, e.cost_note,
    e.signup_deadline, e.status, e.created_by, c.name AS creator_name, e.photo_file_id,
    (SELECT COUNT(*) FROM event_signups s WHERE s.event_id=e.id AND s.status='going') AS going,
    (SELECT COUNT(*) FROM event_signups s WHERE s.event_id=e.id AND s.status='waitlist') AS waitlist,
    ms.status AS my_status, ms.need_ride AS my_need_ride, ms.offer_drive AS my_offer_drive, ms.attended AS my_attended,
    ms.as_group_id AS my_as_group_id, (SELECT name FROM groups WHERE id = ms.as_group_id) AS my_as_group,
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
        "my_signup": e.my_status.as_ref().map(|s| json!({"status": s, "need_ride": e.my_need_ride == Some(1), "offer_drive": e.my_offer_drive == Some(1), "attended": e.my_attended,
            "as_group": e.my_as_group_id.map(|id| json!({"id": id, "name": e.my_as_group}))})),
        "groups": e.groups.clone().unwrap_or_default(),
        "photo_file_id": e.photo_file_id,
        "can_manage": can_manage(u, e.created_by, Act::Update),
        "can_cancel": can_manage(u, e.created_by, Act::Delete),
        "can_see_signups": can_see_signups(u, e.created_by),
    })
}

#[derive(Deserialize)]
struct ListQuery {
    scope: Option<String>, // upcoming | past | managing
    before: Option<String>,
    limit: Option<i64>,
}

async fn list_events(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Events, Act::View)?;
    let visibility = visible_events_sql("$1", "$2");
    let now_str = now()[..16].to_string();

    if q.scope.as_deref() == Some("past") && (q.before.is_some() || q.limit.is_some()) {
        let limit = q.limit.unwrap_or(25).clamp(1, 100);
        let cutoff = q.before.as_ref().unwrap_or(&now_str);
        let sql = format!("{EVENT_SELECT} WHERE {visibility} AND e.starts_at < $3 ORDER BY e.starts_at DESC LIMIT $4");
        let mut rows = sqlx::query_as::<_, EventRow>(&sql)
            .bind(u.id)
            .bind(&u.group_scope)
            .bind(cutoff)
            .bind(limit + 1)
            .fetch_all(&st.db)
            .await?;
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = if more {
            rows.last().map(|e| e.starts_at.clone())
        } else {
            None
        };
        return Ok(Json(json!({
            "items": rows.iter().map(|e| event_json(e, &u)).collect::<Vec<_>>(),
            "next_before": next_before,
        })));
    }

    let sql = match q.scope.as_deref() {
        Some("past") => format!("{EVENT_SELECT} WHERE {visibility} AND (e.starts_at < $3 OR e.status='finished') ORDER BY e.starts_at DESC LIMIT 50"),
        Some("managing") => format!("{EVENT_SELECT} WHERE e.created_by = $1 ORDER BY e.starts_at DESC LIMIT 100"),
        _ => format!("{EVENT_SELECT} WHERE {visibility} AND e.starts_at >= $3 AND e.status = 'published' ORDER BY e.starts_at LIMIT 100"),
    };
    let cutoff = now_str;
    let rows = if q.scope.as_deref() == Some("managing") {
        sqlx::query_as::<_, EventRow>(&sql)
            .bind(u.id)
            .fetch_all(&st.db)
            .await?
    } else {
        sqlx::query_as::<_, EventRow>(&sql)
            .bind(u.id)
            .bind(&u.group_scope)
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
        return Err(forbidden(u.lang, ErrorKey::EventNotOpenToGroup));
    }
    let e = load_event(&st, &u, id).await?;
    let mut v = event_json(&e, &u);
    let gids: Vec<(i64,)> = sqlx::query_as("SELECT group_id FROM event_groups WHERE event_id=$1")
        .bind(id)
        .fetch_all(&st.db)
        .await?;
    v["group_ids"] = json!(gids.into_iter().map(|g| g.0).collect::<Vec<_>>());
    let narrowed: Vec<i64> =
        sqlx::query_scalar("SELECT group_id FROM event_attribution_groups WHERE event_id=$1")
            .bind(id)
            .fetch_all(&st.db)
            .await?;
    v["as_group_ids"] = json!(narrowed);
    v["join_as"] = json!(join_choices(&st.db, &u, id)
        .await?
        .into_iter()
        .map(|(gid, name)| json!({"id": gid, "name": name}))
        .collect::<Vec<_>>());
    let my_ride: Option<(i64, String)> = sqlx::query_as(
        // Their own ride, not one they booked for a visitor.
        "SELECT id, status FROM rides WHERE event_id=$1 AND passenger_id=$2 AND is_proxy=0 AND status NOT IN ('cancelled') ORDER BY id DESC LIMIT 1",
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
    /// Narrow the "joining as" choices. Empty or missing: the audience and its sub-groups.
    as_group_ids: Option<Vec<i64>>,
    /// Optional cover photo: an image the saver uploaded (or the event's current photo).
    photo_file_id: Option<i64>,
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
    let found: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM groups WHERE id = ANY($1) FOR SHARE")
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

async fn create_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<EventIn>,
) -> AppResult<Json<Value>> {
    if !u.can(Page::Events, Act::Create) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanCreate));
    }
    let (at, dl) = validate(&req, u.lang)?;
    check_photo(&st, &u, req.photo_file_id, None).await?;
    // The event and its audience are saved together, or not at all.
    let mut tx = st.db.begin().await?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO events (title, kind, description, starts_at, location, capacity, cost_note, signup_deadline, created_by, created_at, photo_file_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
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
    .bind(req.photo_file_id)
    .fetch_one(&mut *tx)
    .await?;
    save_event_groups(&mut tx, id, &req, u.lang).await?;
    tx.commit().await?;
    // Invite members of the target groups and their sub-groups (相关活动邀请).
    let members: Vec<(i64,)> = sqlx::query_as(&audience_users_sql("$1"))
        .bind(id)
        .fetch_all(&st.db)
        .await?;
    let invited: Vec<i64> = members
        .into_iter()
        .map(|(m,)| m)
        .filter(|m| *m != u.id)
        .collect();
    let body = format!("{} · {}", req.title.trim(), at.replace('T', " "));
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
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanEdit));
    }
    let (at, dl) = validate(&req, u.lang)?;
    check_photo(&st, &u, req.photo_file_id, e.photo_file_id).await?;
    // Same event lock as `signup` / `withdraw`: places a raised capacity opens go to the
    // waitlist, in order, before any signup queued behind this save can take them.
    let mut tx = st.db.begin().await?;
    let (event_status,): (String,) =
        sqlx::query_as("SELECT status FROM events WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let capacity = req.capacity.unwrap_or(0).max(0);
    sqlx::query("UPDATE events SET title=$1, kind=$2, description=$3, starts_at=$4, location=$5, capacity=$6, cost_note=$7, signup_deadline=$8, photo_file_id=$9 WHERE id=$10")
        .bind(req.title.trim())
        .bind(&req.kind)
        .bind(req.description.clone().unwrap_or_default())
        .bind(&at)
        .bind(req.location.trim())
        .bind(capacity)
        .bind(req.cost_note.clone().unwrap_or_default())
        .bind(&dl)
        .bind(req.photo_file_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // Details, audience and waitlist promotions commit together: a failed save (say, a
    // group deleted since the form opened) must not keep the new venue and drop the audience.
    save_event_groups(&mut tx, id, &req, u.lang).await?;
    let promoted = promote_waitlist(&mut tx, id, capacity, &event_status).await?;
    tx.commit().await?;
    notify_promoted(&st, id, req.title.trim(), &promoted).await;
    if at != e.starts_at || req.location.trim() != e.location {
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
    crate::routes::rides::event_moved(
        &st,
        id,
        (&e.starts_at, &at),
        (&e.location, req.location.trim()),
    )
    .await?;
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

async fn cancel_event(
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
    sqlx::query("UPDATE events SET status='cancelled' WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let rides: Vec<(i64, Option<i64>)> = sqlx::query_as(
        "UPDATE rides SET status='cancelled', attention_reason='活动已取消', updated_at=$2
         WHERE event_id=$1 AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')
         RETURNING id, driver_id",
    )
    .bind(id)
    .bind(now())
    .fetch_all(&mut *tx)
    .await?;
    let ride_ids: Vec<i64> = rides.iter().map(|(rid, _)| *rid).collect();
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

    notify_signups(
        &st,
        id,
        ("活动已取消", "Event cancelled"),
        (e.title.as_str(), e.title.as_str()),
    )
    .await?;
    for (_, d) in rides {
        if let Some(d) = d {
            notify(
                &st.db,
                d,
                (
                    "活动已取消，接送随之取消",
                    "Event cancelled, ride cancelled with it",
                ),
                (e.title.as_str(), e.title.as_str()),
                "/drive",
            )
            .await;
        }
    }
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

async fn finish_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanPerformAction));
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
    /// The group the member joins as. Required; filled in when only one fits.
    as_group_id: Option<i64>,
}

async fn signup(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SignupIn>,
) -> AppResult<Json<Value>> {
    if !can_see_event(&st, &u, id).await? {
        return Err(forbidden(u.lang, ErrorKey::EventNotOpenToGroup));
    }
    if req.offer_drive.unwrap_or(false) && !u.can(Page::Drive, Act::Create) {
        return Err(bad(u.lang, ErrorKey::DriverRegistrationRequired));
    }
    let choices = join_choices(&st.db, &u, id).await?;
    // Lock the event row while counting seats and inserting the signup. Concurrent
    // signups then queue behind each other and the last seat cannot be handed out twice.
    let mut tx = st.db.begin().await?;
    let (capacity, status_now, deadline, starts_at): (i64, String, Option<String>, String) =
        sqlx::query_as(
            "SELECT capacity, status, signup_deadline, starts_at FROM events WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if status_now != "published" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    let n = now()[..16].to_string();
    if deadline.as_deref().map(|d| d < n.as_str()).unwrap_or(false) || starts_at < n {
        return Err(bad(u.lang, ErrorKey::SignupClosed));
    }
    // Which group they come as: checked once the event is known to be open, so a closed
    // event says so rather than asking for a group.
    let as_group = match req.as_group_id {
        Some(g) if choices.iter().any(|(c, _)| *c == g) => g,
        Some(_) => return Err(bad(u.lang, ErrorKey::JoinAsNotAllowed)),
        None if choices.len() == 1 => choices[0].0,
        None => return Err(bad(u.lang, ErrorKey::JoinAsRequired)),
    };
    let (going,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND status='going'")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let mine: Option<(String,)> =
        sqlx::query_as("SELECT status FROM event_signups WHERE event_id=$1 AND user_id=$2")
            .bind(id)
            .bind(u.id)
            .fetch_optional(&mut *tx)
            .await?;
    let already_going = mine.as_ref().is_some_and(|(s,)| s == "going");
    let status = if already_going || capacity == 0 || going < capacity {
        "going"
    } else {
        "waitlist"
    };
    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats, created_at, as_group_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT(event_id, user_id) DO UPDATE SET status=excluded.status, need_ride=excluded.need_ride, offer_drive=excluded.offer_drive, offer_seats=excluded.offer_seats, as_group_id=excluded.as_group_id",
    )
    .bind(id)
    .bind(u.id)
    .bind(status)
    .bind(req.need_ride.unwrap_or(false) as i64)
    .bind(req.offer_drive.unwrap_or(false) as i64)
    .bind(req.offer_seats.unwrap_or(0))
    .bind(now())
    .bind(as_group)
    .execute(&mut *tx)
    .await?;
    // A ride already booked for this event follows a changed "joining as".
    sqlx::query("UPDATE rides SET as_group_id=$1 WHERE event_id=$2 AND passenger_id=$3 AND status NOT IN ('completed','cancelled','no_show','alt_transport') AND is_proxy=0")
        .bind(as_group)
        .bind(id)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "event.signup",
        "event",
        Some(id),
        format!(
            "{status}; as {}{}{}",
            choices
                .iter()
                .find(|(c, _)| *c == as_group)
                .map_or("?", |(_, n)| n.as_str()),
            if req.need_ride.unwrap_or(false) {
                "; needs a ride"
            } else {
                ""
            },
            if req.offer_drive.unwrap_or(false) {
                "; offers to drive"
            } else {
                ""
            },
        ),
    )
    .await;
    Ok(Json(json!({"ok": true, "status": status})))
}

/// Moves waitlisted members into every free place, oldest signup first. Unlimited
/// capacity (0) frees them all. Runs inside the caller's transaction, which must hold the
/// event row lock so concurrent signups and withdrawals see the same count.
async fn promote_waitlist(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: i64,
    capacity: i64,
    event_status: &str,
) -> AppResult<Vec<i64>> {
    if event_status != "published" {
        return Ok(Vec::new());
    }
    let rows: Vec<(i64,)> = sqlx::query_as(
        "UPDATE event_signups SET status='going'
         WHERE id IN (
           SELECT id FROM event_signups WHERE event_id=$1 AND status='waitlist'
           ORDER BY created_at, id
           LIMIT CASE WHEN $2 = 0 THEN NULL
                      ELSE GREATEST(0, $2 - (SELECT COUNT(*) FROM event_signups WHERE event_id=$1 AND status='going'))
                 END
         )
         RETURNING user_id",
    )
    .bind(id)
    .bind(capacity)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows.into_iter().map(|(uid,)| uid).collect())
}

/// Audits and tells each promoted member they have a place. No actor: the place passes
/// on automatically.
async fn notify_promoted(st: &AppState, id: i64, title: &str, promoted: &[i64]) {
    for &uid in promoted {
        audit(
            &st.db,
            None,
            "event.waitlist_promote",
            "event",
            Some(id),
            format!("user {uid} waitlist → going"),
        )
        .await;
        let zh = format!("您已获得「{title}」的名额");
        let en = format!("You now have a spot in \"{title}\"");
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

/// 乘客无法出席: cancel signup → release seat → promote waitlist → cancel linked rides → tell driver & organiser.
async fn withdraw(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_event(&st, &u, id).await?;
    // Same event lock as `signup`: concurrent withdrawals (and signups) queue behind each
    // other, so each sees the seats the others freed or took and the same waitlisted
    // member is never promoted twice while others are left waiting.
    let mut tx = st.db.begin().await?;
    let (capacity, event_status): (i64, String) =
        sqlx::query_as("SELECT capacity, status FROM events WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("UPDATE event_signups SET status='cancelled' WHERE event_id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    // Fill every free place from the waitlist, in order, rather than exactly one per
    // withdrawal; this also heals any place left open earlier.
    let promoted = promote_waitlist(&mut tx, id, capacity, &event_status).await?;
    tx.commit().await?;
    notify_promoted(&st, id, &e.title, &promoted).await;
    let rides: Vec<(i64, Option<i64>)> = sqlx::query_as(
        // Only their own ride: a visitor they invited may still come, and the host can cancel
        // that ride from the ride page.
        "UPDATE rides SET status='cancelled', attention_reason='乘客取消活动报名', updated_at=$3
         WHERE event_id=$1 AND passenger_id=$2 AND is_proxy=0
           AND status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm','confirmed')
         RETURNING id, driver_id",
    )
    .bind(id)
    .bind(u.id)
    .bind(now())
    .fetch_all(&st.db)
    .await?;
    let ride_ids: Vec<i64> = rides.iter().map(|(rid, _)| *rid).collect();
    audit_many(
        &st.db,
        Some(u.id),
        "ride.cancel",
        "ride",
        &ride_ids,
        &format!("withdrew from event {id}"),
    )
    .await?;
    for (_, d) in rides {
        if let Some(d) = d {
            let body = format!("{} · {}", u.name, e.title);
            notify(
                &st.db,
                d,
                (
                    "乘客已取消，座位已释放",
                    "Passenger cancelled, seat released",
                ),
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
    if !can_see_signups(&u, e.created_by) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanViewSignups));
    }
    type SignupRow = (
        i64,
        i64,
        String,
        String,
        i64,
        i64,
        i64,
        Option<i64>,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<SignupRow> = sqlx::query_as(
        "SELECT s.id, s.user_id, u.name, s.status, s.need_ride, s.offer_drive, s.offer_seats, s.attended,
            (SELECT r.status FROM rides r WHERE r.event_id = s.event_id AND r.passenger_id = s.user_id AND r.is_proxy = 0 AND r.status != 'cancelled' ORDER BY r.id DESC LIMIT 1),
            (SELECT name FROM groups WHERE id = s.as_group_id)
         FROM event_signups s JOIN users u ON u.id = s.user_id WHERE s.event_id=$1 ORDER BY s.status, s.created_at",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(sid, uid, name, status, nr, od, os, att, ride, as_group)| json!({
        "id": sid, "user_id": uid, "name": name, "status": status, "need_ride": nr == 1, "offer_drive": od == 1,
        "offer_seats": os, "attended": att, "ride_status": ride, "as_group": as_group,
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
    if !can_manage(&u, e.created_by, Act::Update) {
        return Err(forbidden(
            u.lang,
            ErrorKey::OnlyOrganiserCanRecordAttendance,
        ));
    }
    let v = req.attended.map(|a| a as i64);
    // Recording someone as attended also makes them a participant: a member who withdrew
    // (or was still waitlisted) but came anyway is a walk-in and must show in the list
    // and count, which only include 'going'.
    let r = sqlx::query(
        "UPDATE event_signups SET attended=$1,
           status = CASE WHEN $1 = 1 AND status IN ('cancelled','waitlist') THEN 'going' ELSE status END
         WHERE event_id=$2 AND user_id=$3",
    )
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
