//! Admin event participation, attendance, and per-group headcount analytics reports.

use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::{json, Value};

use super::{parse_period_window, PeriodQuery};
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::require;

pub(in crate::routes::admin) async fn events_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<PeriodQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReports, Act::View)?;
    let global_events = u.can(Page::AdminEvents, Act::View);
    if !global_events && !u.can(Page::Events, Act::Create) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let scope_creator: Option<i64> = if global_events { None } else { Some(u.id) };
    let can_rides = u.can(Page::AdminRides, Act::View) || u.can(Page::AdminPlan, Act::View);

    let kind = q.kind.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(k) = kind {
        if !crate::routes::events::KINDS.contains(&k) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }

    let w = parse_period_window(
        u.lang,
        q.month.as_deref(),
        q.range.as_deref(),
        q.from.as_deref(),
        q.to.as_deref(),
    )?;

    #[derive(sqlx::FromRow)]
    struct EvRow {
        id: i64,
        title: String,
        kind: String,
        starts_at: String,
        status: String,
        creator_name: String,
        attendance_enabled: i64,
        going: i64,
        waitlist: i64,
        attended: i64,
        need_ride: i64,
        offer_drive: i64,
        self_transport: i64,
        rides_matched: i64,
        rides_unfilled: i64,
    }

    let ev_rows: Vec<EvRow> = sqlx::query_as(
        "SELECT e.id, e.title, e.kind, e.starts_at, e.status, u.name AS creator_name,
                e.attendance_enabled,
                COALESCE((SELECT COUNT(*) FROM event_signups s WHERE s.event_id = e.id AND s.status = 'going'), 0)::BIGINT AS going,
                COALESCE((SELECT COUNT(*) FROM event_signups s WHERE s.event_id = e.id AND s.status = 'waitlist'), 0)::BIGINT AS waitlist,
                COALESCE((SELECT COUNT(*) FROM event_signups s WHERE s.event_id = e.id AND s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT AS attended,
                COALESCE((SELECT COUNT(*) FROM event_signups s WHERE s.event_id = e.id AND s.status = 'going' AND s.need_ride = 1), 0)::BIGINT AS need_ride,
                COALESCE((SELECT COUNT(*) FROM event_signups s WHERE s.event_id = e.id AND s.status = 'going' AND s.offer_drive = 1), 0)::BIGINT AS offer_drive,
                COALESCE((SELECT COUNT(*) FROM event_signups s WHERE s.event_id = e.id AND s.status = 'going' AND s.self_transport = 1), 0)::BIGINT AS self_transport,
                COALESCE((SELECT COUNT(*) FROM rides r WHERE r.event_id = e.id AND r.status IN ('confirmed', 'in_progress', 'completed')), 0)::BIGINT AS rides_matched,
                COALESCE((SELECT COUNT(*) FROM rides r WHERE r.event_id = e.id AND r.status NOT IN ('cancelled', 'no_show', 'alt_transport') AND r.driver_id IS NULL), 0)::BIGINT AS rides_unfilled
         FROM events e
         JOIN users u ON u.id = e.created_by
         WHERE e.status <> 'draft'
           AND e.starts_at >= $1 AND e.starts_at < $2
           AND ($3::BIGINT IS NULL OR e.created_by = $3)
           AND ($4::TEXT IS NULL OR e.kind = $4)
           AND ($5::BIGINT IS NULL
                OR EXISTS (SELECT 1 FROM event_groups eg WHERE eg.event_id = e.id AND eg.group_id = $5)
                OR EXISTS (SELECT 1 FROM event_signups es WHERE es.event_id = e.id AND es.as_group_id = $5))
         ORDER BY e.starts_at DESC, e.id DESC",
    )
    .bind(&w.start)
    .bind(&w.end)
    .bind(scope_creator)
    .bind(kind)
    .bind(q.group_id)
    .fetch_all(&st.db)
    .await?;

    #[derive(sqlx::FromRow)]
    struct GroupRow {
        group_id: Option<i64>,
        group_name: Option<String>,
        going: i64,
        attended: i64,
        need_ride: i64,
        self_transport: i64,
        offer_drive: i64,
    }

    let group_rows: Vec<GroupRow> = sqlx::query_as(
        "SELECT s.as_group_id AS group_id, g.name AS group_name,
                COUNT(*) FILTER (WHERE s.status = 'going')::BIGINT AS going,
                COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS attended,
                COUNT(*) FILTER (WHERE s.status = 'going' AND s.need_ride = 1)::BIGINT AS need_ride,
                COUNT(*) FILTER (WHERE s.status = 'going' AND s.self_transport = 1)::BIGINT AS self_transport,
                COUNT(*) FILTER (WHERE s.status = 'going' AND s.offer_drive = 1)::BIGINT AS offer_drive
         FROM event_signups s
         JOIN events e ON e.id = s.event_id
         LEFT JOIN groups g ON g.id = s.as_group_id
         WHERE e.status NOT IN ('draft', 'cancelled')
           AND e.starts_at >= $1 AND e.starts_at < $2
           AND ($3::BIGINT IS NULL OR e.created_by = $3)
           AND ($4::TEXT IS NULL OR e.kind = $4)
           AND ($5::BIGINT IS NULL OR s.as_group_id = $5)
         GROUP BY s.as_group_id, g.name
         ORDER BY going DESC, group_name ASC NULLS LAST",
    )
    .bind(&w.start)
    .bind(&w.end)
    .bind(scope_creator)
    .bind(kind)
    .bind(q.group_id)
    .fetch_all(&st.db)
    .await?;

    let (visitor_passes_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::BIGINT
         FROM visitor_passes vp
         JOIN events e ON e.id = vp.event_id
         WHERE e.status NOT IN ('draft', 'cancelled')
           AND e.starts_at >= $1 AND e.starts_at < $2
           AND ($3::BIGINT IS NULL OR e.created_by = $3)
           AND ($4::TEXT IS NULL OR e.kind = $4)
           AND ($5::BIGINT IS NULL OR vp.host_group_id = $5)",
    )
    .bind(&w.start)
    .bind(&w.end)
    .bind(scope_creator)
    .bind(kind)
    .bind(q.group_id)
    .fetch_one(&st.db)
    .await?;

    let plan_visitors_count: Option<i64> = if can_rides && q.group_id.is_none() {
        let (cnt,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT
             FROM plan_visitors pv
             JOIN events e ON e.id = pv.event_id
             WHERE e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2
               AND ($3::BIGINT IS NULL OR e.created_by = $3)
               AND ($4::TEXT IS NULL OR e.kind = $4)",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .bind(kind)
        .fetch_one(&st.db)
        .await?;
        Some(cnt)
    } else if can_rides {
        Some(0)
    } else {
        None
    };

    let mut events_count = 0_i64;
    let mut cancelled_events = 0_i64;
    let mut total_going = 0_i64;
    let mut total_waitlist = 0_i64;
    let mut total_attended = 0_i64;
    let mut need_ride_count = 0_i64;
    let mut offer_drive_count = 0_i64;
    let mut self_transport_count = 0_i64;

    #[derive(Default)]
    struct KindBucket {
        events_count: i64,
        going: i64,
        attended: i64,
    }
    let mut by_kind: HashMap<String, KindBucket> = HashMap::new();

    for ev in &ev_rows {
        if ev.status == "cancelled" {
            cancelled_events += 1;
            continue;
        }
        events_count += 1;
        total_going += ev.going;
        total_waitlist += ev.waitlist;
        total_attended += ev.attended;
        need_ride_count += ev.need_ride;
        offer_drive_count += ev.offer_drive;
        self_transport_count += ev.self_transport;

        let kb = by_kind.entry(ev.kind.clone()).or_default();
        kb.events_count += 1;
        kb.going += ev.going;
        kb.attended += ev.attended;
    }

    let attendance_rate_pct = if total_going > 0 {
        (total_attended * 100) / total_going
    } else {
        0
    };

    let mut by_kind_list: Vec<Value> = by_kind
        .into_iter()
        .map(|(k, b)| {
            json!({
                "kind": k,
                "events_count": b.events_count,
                "going": b.going,
                "attended": b.attended,
                "attendance_rate_pct": if b.going > 0 { (b.attended * 100) / b.going } else { 0 },
            })
        })
        .collect();
    by_kind_list.sort_by(|a, b| b["going"].as_i64().cmp(&a["going"].as_i64()));

    let by_group_list: Vec<Value> = group_rows
        .into_iter()
        .map(|g| {
            let rate = if g.going > 0 {
                (g.attended * 100) / g.going
            } else {
                0
            };
            json!({
                "group_id": g.group_id,
                "group_name": g.group_name,
                "going": g.going,
                "attended": g.attended,
                "attendance_rate_pct": rate,
                "need_ride": g.need_ride,
                "self_transport": g.self_transport,
                "offer_drive": g.offer_drive,
            })
        })
        .collect();

    let events_list: Vec<Value> = ev_rows
        .into_iter()
        .take(200)
        .map(|ev| {
            let rate = if ev.going > 0 {
                (ev.attended * 100) / ev.going
            } else {
                0
            };
            json!({
                "id": ev.id,
                "title": ev.title,
                "kind": ev.kind,
                "starts_at": ev.starts_at,
                "status": ev.status,
                "creator_name": ev.creator_name,
                "going": ev.going,
                "waitlist": ev.waitlist,
                "attended": ev.attended,
                "attendance_enabled": ev.attendance_enabled == 1,
                "attendance_rate_pct": rate,
                "need_ride": ev.need_ride,
                "self_transport": ev.self_transport,
                "rides_matched": if can_rides { Some(ev.rides_matched) } else { None },
                "rides_unfilled": if can_rides { Some(ev.rides_unfilled) } else { None },
            })
        })
        .collect();

    Ok(Json(json!({
        "scope": if global_events { "global" } else { "own_events" },
        "period": w.period_label,
        "range": w.range,
        "from": w.start,
        "to": w.display_to,
        "totals": {
            "events_count": events_count,
            "cancelled_events": cancelled_events,
            "total_going": total_going,
            "total_waitlist": total_waitlist,
            "total_attended": total_attended,
            "attendance_rate_pct": attendance_rate_pct,
            "need_ride_count": need_ride_count,
            "offer_drive_count": offer_drive_count,
            "self_transport_count": self_transport_count,
            "visitor_passes_count": visitor_passes_count,
            "plan_visitors_count": plan_visitors_count,
        },
        "by_group": by_group_list,
        "by_kind": by_kind_list,
        "events": events_list,
    })))
}
