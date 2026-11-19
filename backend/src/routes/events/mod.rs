//! 运动与团契活动: group-targeted events, signup / waitlist, ride link, attendance check-in.

mod checkin;
mod manage;
mod signups;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audience::{join_choices, visible_events_sql};
use crate::auth::CurrentUser;
use crate::error::{forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{now, require};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/groups", get(list_groups))
        .route("/events", get(list_events).post(manage::create_event))
        .route("/events/{id}", get(get_event).put(manage::update_event))
        .route("/events/{id}/duplicate", post(manage::duplicate_event))
        .route("/events/{id}/cancel", post(manage::cancel_event))
        .route("/events/{id}/finish", post(manage::finish_event))
        .route("/events/{id}/signup", post(signups::signup))
        .route("/events/{id}/withdraw", post(signups::withdraw))
        .route("/events/{id}/signups", get(signups::list_signups))
        .route("/events/{id}/attendance", post(checkin::mark_attendance))
        .route(
            "/events/{id}/checkin",
            get(checkin::get_checkin_info).post(checkin::self_checkin),
        )
        .route("/events/checkin", post(checkin::checkin_by_code))
        .route(
            "/events/{id}/checkin/scan-member",
            post(checkin::scan_member_pass),
        )
        .route(
            "/events/{id}/checkin-code/reset",
            post(checkin::reset_checkin_code),
        )
}

pub const KINDS: &[&str] = &["badminton", "bowling", "kayak", "walk", "other"];

/// A member sees an event if it targets one of their groups or a group above one of
/// theirs (or the 全体会员 group), or if they created it / are staff. See `crate::audience`.
pub async fn can_see_event(st: &AppState, u: &CurrentUser, event_id: i64) -> AppResult<bool> {
    let (n,): (i64,) = sqlx::query_as(crate::dyn_sql!(
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
pub(super) fn can_see_signups(u: &CurrentUser, created_by: i64) -> bool {
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
        // Announcements can be for any group.
        (Page::AdminAnnouncements, Act::Create),
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
pub(super) struct EventRow {
    pub(super) id: i64,
    pub(super) title: String,
    pub(super) kind: String,
    pub(super) description: String,
    pub(super) starts_at: String,
    pub(super) location: String,
    pub(super) capacity: i64,
    pub(super) cost_note: String,
    pub(super) signup_deadline: Option<String>,
    pub(super) status: String,
    pub(super) created_by: i64,
    pub(super) creator_name: String,
    pub(super) going: i64,
    pub(super) waitlist: i64,
    pub(super) my_status: Option<String>,
    pub(super) my_need_ride: Option<i64>,
    pub(super) my_offer_drive: Option<i64>,
    pub(super) my_attended: Option<i64>,
    pub(super) my_as_group_id: Option<i64>,
    pub(super) my_as_group: Option<String>,
    pub(super) my_pass_code: Option<String>,
    pub(super) series_id: Option<String>,
    pub(super) groups: Option<String>,
    pub(super) photo_file_id: Option<i64>,
    pub(super) attendance_enabled: i64,
    pub(super) checkin_code: Option<String>,
}

const EVENT_SELECT: &str = "SELECT e.id, e.title, e.kind, e.description, e.starts_at, e.location, e.capacity, e.cost_note,
    e.signup_deadline, e.status, e.created_by, c.name AS creator_name, e.photo_file_id,
    e.attendance_enabled, e.checkin_code, e.series_id,
    (SELECT COUNT(*) FROM event_signups s WHERE s.event_id=e.id AND s.status='going') AS going,
    (SELECT COUNT(*) FROM event_signups s WHERE s.event_id=e.id AND s.status='waitlist') AS waitlist,
    ms.status AS my_status, ms.need_ride AS my_need_ride, ms.offer_drive AS my_offer_drive, ms.attended AS my_attended,
    ms.as_group_id AS my_as_group_id, (SELECT name FROM groups WHERE id = ms.as_group_id) AS my_as_group, ms.pass_code AS my_pass_code,
    (SELECT string_agg(g.name, '、') FROM event_groups eg JOIN groups g ON g.id=eg.group_id WHERE eg.event_id=e.id) AS groups
    FROM events e JOIN users c ON c.id = e.created_by
    LEFT JOIN event_signups ms ON ms.event_id = e.id AND ms.user_id = $1";

fn event_json(e: &EventRow, u: &CurrentUser) -> Value {
    let can_edit = can_manage(u, e.created_by, Act::Update);
    json!({
        "id": e.id, "title": e.title, "kind": e.kind, "description": e.description, "starts_at": e.starts_at,
        "location": e.location, "capacity": e.capacity, "cost_note": e.cost_note, "signup_deadline": e.signup_deadline,
        "status": e.status, "created_by": {"id": e.created_by, "name": e.creator_name},
        "series_id": e.series_id,
        "going": e.going, "waitlist": e.waitlist,
        "spots_left": if e.capacity > 0 { Some((e.capacity - e.going).max(0)) } else { None },
        "my_signup": e.my_status.as_ref().map(|s| json!({
            "status": s,
            "need_ride": e.my_need_ride == Some(1),
            "offer_drive": e.my_offer_drive == Some(1),
            "attended": e.my_attended,
            "pass_code": if s == "going" { e.my_pass_code.clone() } else { None },
            "as_group": e.my_as_group_id.map(|id| json!({"id": id, "name": e.my_as_group})),
        })),
        "groups": e.groups.clone().unwrap_or_default(),
        "photo_file_id": e.photo_file_id,
        "attendance_enabled": e.attendance_enabled == 1,
        "checkin_code": if can_edit { e.checkin_code.clone() } else { None },
        "can_manage": can_edit,
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
        let mut conds = vec![
            visibility,
            "(e.starts_at < $3 OR e.status='finished')".to_string(),
        ];
        let cursor = q.before.as_deref().map(crate::util::split_cursor);
        let mut next_param = 4usize;
        if let Some((_, Some(_))) = cursor {
            conds.push(format!(
                "(e.starts_at < ${next_param} OR (e.starts_at = ${next_param} AND e.id < ${}))",
                next_param + 1
            ));
            next_param += 2;
        } else if cursor.is_some() {
            conds.push(format!("e.starts_at < ${next_param}"));
            next_param += 1;
        }
        let sql = format!(
            "{EVENT_SELECT} WHERE {} ORDER BY e.starts_at DESC, e.id DESC LIMIT ${next_param}",
            conds.join(" AND ")
        );
        let mut query = sqlx::query_as::<_, EventRow>(sqlx::AssertSqlSafe(&*sql))
            .bind(u.id)
            .bind(&u.group_scope)
            .bind(&now_str);
        if let Some((ts, Some(bid))) = cursor {
            query = query.bind(ts).bind(bid);
        } else if let Some((ts, None)) = cursor {
            query = query.bind(ts);
        }
        let mut rows = query.bind(limit + 1).fetch_all(&st.db).await?;
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = if more {
            rows.last().map(|e| format!("{}|{}", e.starts_at, e.id))
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
        sqlx::query_as::<_, EventRow>(sqlx::AssertSqlSafe(&*sql))
            .bind(u.id)
            .fetch_all(&st.db)
            .await?
    } else {
        sqlx::query_as::<_, EventRow>(sqlx::AssertSqlSafe(&*sql))
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

pub(super) async fn load_event(st: &AppState, u: &CurrentUser, id: i64) -> AppResult<EventRow> {
    sqlx::query_as::<_, EventRow>(crate::dyn_sql!("{EVENT_SELECT} WHERE e.id = $2"))
        .bind(u.id)
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))
}

pub(super) async fn load_visible_event(
    st: &AppState,
    u: &CurrentUser,
    id: i64,
) -> AppResult<EventRow> {
    if !can_see_event(st, u, id).await? {
        return Err(forbidden(u.lang, ErrorKey::EventNotOpenToGroup));
    }
    load_event(st, u, id).await
}

async fn get_event(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let e = load_visible_event(&st, &u, id).await?;
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
        // Their own ride, not one they booked for a visitor; prefer outbound when round-trip.
        "SELECT id, status FROM rides WHERE event_id=$1 AND passenger_id=$2 AND is_proxy=0 AND status NOT IN ('cancelled')
         ORDER BY CASE WHEN leg='outbound' THEN 0 ELSE 1 END, id DESC LIMIT 1",
    )
    .bind(id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    v["my_ride"] = json!(my_ride.map(|(i, s)| json!({"id": i, "status": s})));
    let my_return_ride: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, status FROM rides WHERE event_id=$1 AND passenger_id=$2 AND is_proxy=0 AND leg='return' AND status NOT IN ('cancelled')
         ORDER BY id DESC LIMIT 1",
    )
    .bind(id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    v["my_return_ride"] = json!(my_return_ride.map(|(i, s)| json!({"id": i, "status": s})));
    let settlement: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, status FROM settlements WHERE event_id=$1 ORDER BY id DESC LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    v["settlement"] = json!(settlement.map(|(i, s)| json!({"id": i, "status": s})));
    Ok(Json(v))
}
