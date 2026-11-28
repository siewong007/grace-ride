//! Admin event participation, attendance, and per-group headcount analytics reports.

use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::{json, Value};

use super::{parse_period_window, prev_period_window, PeriodQuery};
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
        created_by: i64,
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

    enum EvParam<'a> {
        I64(i64),
        Str(&'a str),
    }
    let build_ev_filters = |include_group: bool,
                            group_mode: u8|
     -> (String, String, String, Vec<EvParam<'_>>) {
        let mut ev_cond = String::new();
        let mut s_cond = String::new();
        let mut r_cond = String::new();
        let mut params: Vec<EvParam<'_>> = Vec::new();
        if let Some(cid) = scope_creator {
            params.push(EvParam::I64(cid));
            ev_cond.push_str(&format!(" AND e.created_by = ${}", 2 + params.len()));
        }
        if let Some(k) = kind {
            params.push(EvParam::Str(k));
            ev_cond.push_str(&format!(" AND e.kind = ${}", 2 + params.len()));
        }
        if include_group {
            if let Some(gid) = q.group_id {
                params.push(EvParam::I64(gid));
                let p = 2 + params.len();
                match group_mode {
                    0 => {
                        ev_cond.push_str(&format!(
                            " AND (EXISTS (SELECT 1 FROM event_groups eg WHERE eg.event_id = e.id AND eg.group_id = ${p}) OR EXISTS (SELECT 1 FROM event_signups es WHERE es.event_id = e.id AND es.as_group_id = ${p}))"
                        ));
                        s_cond.push_str(&format!(" AND s.as_group_id = ${p}"));
                        r_cond.push_str(&format!(" AND r.as_group_id = ${p}"));
                    }
                    1 => ev_cond.push_str(&format!(" AND s.as_group_id = ${p}")),
                    _ => ev_cond.push_str(&format!(" AND vp.host_group_id = ${p}")),
                }
            }
        }
        (ev_cond, s_cond, r_cond, params)
    };

    let (ev_cond0, s_cond0, r_cond0, ev_params0) = build_ev_filters(true, 0);
    let ev_sql = format!(
        "WITH filtered_events AS (
            SELECT e.id, e.title, e.kind, e.starts_at, e.status, e.created_by, u.name AS creator_name,
                   e.attendance_enabled
            FROM events e
            JOIN users u ON u.id = e.created_by
            WHERE e.status <> 'draft'
              AND e.starts_at >= $1 AND e.starts_at < $2{ev_cond0}
         ),
         signup_agg AS (
            SELECT s.event_id,
                   COUNT(*) FILTER (WHERE s.status = 'going')::BIGINT AS going,
                   COUNT(*) FILTER (WHERE s.status = 'waitlist')::BIGINT AS waitlist,
                   COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS attended,
                   COUNT(*) FILTER (WHERE s.status = 'going' AND s.need_ride = 1)::BIGINT AS need_ride,
                   COUNT(*) FILTER (WHERE s.status = 'going' AND s.offer_drive = 1)::BIGINT AS offer_drive,
                   COUNT(*) FILTER (WHERE s.status = 'going' AND s.self_transport = 1)::BIGINT AS self_transport
            FROM event_signups s
            JOIN filtered_events fe ON fe.id = s.event_id
            WHERE TRUE{s_cond0}
            GROUP BY s.event_id
         ),
         ride_agg AS (
            SELECT r.event_id,
                   COUNT(*) FILTER (WHERE r.status IN ('confirmed', 'in_progress', 'completed'))::BIGINT AS rides_matched,
                   COUNT(*) FILTER (WHERE r.status NOT IN ('cancelled', 'no_show', 'alt_transport') AND r.driver_id IS NULL)::BIGINT AS rides_unfilled
            FROM rides r
            JOIN filtered_events fe ON fe.id = r.event_id
            WHERE TRUE{r_cond0}
            GROUP BY r.event_id
         )
         SELECT fe.id, fe.title, fe.kind, fe.starts_at, fe.status, fe.created_by, fe.creator_name,
                fe.attendance_enabled,
                COALESCE(sa.going, 0)::BIGINT AS going,
                COALESCE(sa.waitlist, 0)::BIGINT AS waitlist,
                COALESCE(sa.attended, 0)::BIGINT AS attended,
                COALESCE(sa.need_ride, 0)::BIGINT AS need_ride,
                COALESCE(sa.offer_drive, 0)::BIGINT AS offer_drive,
                COALESCE(sa.self_transport, 0)::BIGINT AS self_transport,
                COALESCE(ra.rides_matched, 0)::BIGINT AS rides_matched,
                COALESCE(ra.rides_unfilled, 0)::BIGINT AS rides_unfilled
         FROM filtered_events fe
         LEFT JOIN signup_agg sa ON sa.event_id = fe.id
         LEFT JOIN ride_agg ra ON ra.event_id = fe.id
         ORDER BY fe.starts_at DESC, fe.id DESC"
    );
    let mut ev_q = sqlx::query_as::<_, EvRow>(sqlx::AssertSqlSafe(&*ev_sql))
        .bind(&w.start)
        .bind(&w.end);
    for p in &ev_params0 {
        ev_q = match p {
            EvParam::I64(v) => ev_q.bind(*v),
            EvParam::Str(s) => ev_q.bind(*s),
        };
    }

    #[derive(sqlx::FromRow)]
    struct GroupRow {
        group_id: Option<i64>,
        group_name: Option<String>,
        going: i64,
        attended: i64,
        tracked_going: i64,
        tracked_attended: i64,
        need_ride: i64,
        self_transport: i64,
        offer_drive: i64,
    }

    let (ev_cond1, _, _, ev_params1) = build_ev_filters(true, 1);
    let group_sql = format!(
        "SELECT s.as_group_id AS group_id, g.name AS group_name,
                COUNT(*) FILTER (WHERE s.status = 'going')::BIGINT AS going,
                COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS attended,
                COUNT(*) FILTER (WHERE e.attendance_enabled = 1 AND s.status = 'going')::BIGINT AS tracked_going,
                COUNT(*) FILTER (WHERE e.attendance_enabled = 1 AND s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS tracked_attended,
                COUNT(*) FILTER (WHERE s.status = 'going' AND s.need_ride = 1)::BIGINT AS need_ride,
                COUNT(*) FILTER (WHERE s.status = 'going' AND s.self_transport = 1)::BIGINT AS self_transport,
                COUNT(*) FILTER (WHERE s.status = 'going' AND s.offer_drive = 1)::BIGINT AS offer_drive
         FROM event_signups s
         JOIN events e ON e.id = s.event_id
         LEFT JOIN groups g ON g.id = s.as_group_id
         WHERE e.status NOT IN ('draft', 'cancelled')
           AND e.starts_at >= $1 AND e.starts_at < $2{ev_cond1}
         GROUP BY s.as_group_id, g.name
         ORDER BY going DESC, group_name ASC NULLS LAST"
    );
    let mut group_q = sqlx::query_as::<_, GroupRow>(sqlx::AssertSqlSafe(&*group_sql))
        .bind(&w.start)
        .bind(&w.end);
    for p in &ev_params1 {
        group_q = match p {
            EvParam::I64(v) => group_q.bind(*v),
            EvParam::Str(s) => group_q.bind(*s),
        };
    }

    let (ev_cond2, _, _, ev_params2) = build_ev_filters(true, 2);
    let vp_sql = format!(
        "SELECT COUNT(*)::BIGINT
         FROM visitor_passes vp
         JOIN events e ON e.id = vp.event_id
         WHERE e.status NOT IN ('draft', 'cancelled')
           AND e.starts_at >= $1 AND e.starts_at < $2{ev_cond2}"
    );
    let mut vp_q = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(&*vp_sql))
        .bind(&w.start)
        .bind(&w.end);
    for p in &ev_params2 {
        vp_q = match p {
            EvParam::I64(v) => vp_q.bind(*v),
            EvParam::Str(s) => vp_q.bind(*s),
        };
    }

    let rides_global = u.traffic_is_global();
    let pv_fut = async {
        if !(can_rides && q.group_id.is_none()) {
            return Ok::<_, crate::error::AppError>(None);
        }
        let mut pv_cond = String::new();
        let mut pv_params: Vec<EvParam<'_>> = Vec::new();
        if !(global_events && rides_global) {
            pv_params.push(EvParam::I64(u.id));
            pv_cond.push_str(&format!(" AND e.created_by = ${}", 2 + pv_params.len()));
        }
        if let Some(k) = kind {
            pv_params.push(EvParam::Str(k));
            pv_cond.push_str(&format!(" AND e.kind = ${}", 2 + pv_params.len()));
        }
        let pv_sql = format!(
            "SELECT COUNT(*)::BIGINT
             FROM plan_visitors pv
             JOIN events e ON e.id = pv.event_id
             WHERE e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2{pv_cond}"
        );
        let mut pq = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(&*pv_sql))
            .bind(&w.start)
            .bind(&w.end);
        for p in &pv_params {
            pq = match p {
                EvParam::I64(v) => pq.bind(*v),
                EvParam::Str(s) => pq.bind(*s),
            };
        }
        Ok(Some(pq.fetch_one(&st.db).await?))
    };

    let prev_win = prev_period_window(&w);
    let prev_sql = format!(
        "WITH prev_ev AS (
            SELECT e.id, e.attendance_enabled
            FROM events e
            WHERE e.status NOT IN ('draft', 'cancelled')
              AND e.starts_at >= $1 AND e.starts_at < $2{ev_cond0}
         )
         SELECT
            (SELECT COUNT(*) FROM prev_ev)::BIGINT,
            COALESCE(COUNT(*) FILTER (WHERE s.status = 'going'), 0)::BIGINT,
            COALESCE(COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT,
            COALESCE(COUNT(*) FILTER (WHERE pe.attendance_enabled = 1 AND s.status = 'going'), 0)::BIGINT,
            COALESCE(COUNT(*) FILTER (WHERE pe.attendance_enabled = 1 AND s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT
         FROM prev_ev pe
         LEFT JOIN event_signups s ON s.event_id = pe.id{s_cond0}"
    );
    let prev_fut = async {
        let Some(pw) = prev_win.as_ref() else {
            return Ok::<_, crate::error::AppError>(None);
        };
        let mut pq =
            sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(sqlx::AssertSqlSafe(&*prev_sql))
                .bind(&pw.start)
                .bind(&pw.end);
        for p in &ev_params0 {
            pq = match p {
                EvParam::I64(v) => pq.bind(*v),
                EvParam::Str(s) => pq.bind(*s),
            };
        }
        Ok(Some((pw.period_label.clone(), pq.fetch_one(&st.db).await?)))
    };

    let (ev_rows, group_rows, visitor_passes_count, plan_visitors_count, prev_ev_stats) = tokio::try_join!(
        async { Ok::<_, crate::error::AppError>(ev_q.fetch_all(&st.db).await?) },
        async { Ok::<_, crate::error::AppError>(group_q.fetch_all(&st.db).await?) },
        async { Ok::<_, crate::error::AppError>(vp_q.fetch_one(&st.db).await?) },
        pv_fut,
        prev_fut,
    )?;

    let mut events_count = 0_i64;
    let mut tracked_events_count = 0_i64;
    let mut cancelled_events = 0_i64;
    let mut total_going = 0_i64;
    let mut tracked_going = 0_i64;
    let mut total_waitlist = 0_i64;
    let mut total_attended = 0_i64;
    let mut tracked_attended = 0_i64;
    let mut need_ride_count = 0_i64;
    let mut offer_drive_count = 0_i64;
    let mut self_transport_count = 0_i64;

    #[derive(Default)]
    struct KindBucket {
        events_count: i64,
        going: i64,
        attended: i64,
        tracked_going: i64,
        tracked_attended: i64,
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
        if ev.attendance_enabled == 1 {
            tracked_events_count += 1;
            tracked_going += ev.going;
            tracked_attended += ev.attended;
            kb.tracked_going += ev.going;
            kb.tracked_attended += ev.attended;
        }
    }

    let attendance_rate_pct = if tracked_going > 0 {
        (tracked_attended * 100) / tracked_going
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
                "attendance_rate_pct": if b.tracked_going > 0 { (b.tracked_attended * 100) / b.tracked_going } else { 0 },
            })
        })
        .collect();
    by_kind_list.sort_by(|a, b| b["going"].as_i64().cmp(&a["going"].as_i64()));

    let by_group_list: Vec<Value> = group_rows
        .into_iter()
        .map(|g| {
            let rate = if g.tracked_going > 0 {
                (g.tracked_attended * 100) / g.tracked_going
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

    let events_total = ev_rows.len() as i64;
    let events_truncated = events_total > 200;
    let events_list: Vec<Value> = ev_rows
        .into_iter()
        .take(200)
        .map(|ev| {
            let tracked = ev.attendance_enabled == 1;
            let rate = if tracked {
                Some(if ev.going > 0 {
                    (ev.attended * 100) / ev.going
                } else {
                    0
                })
            } else {
                None
            };
            let show_ev_rides = can_rides && (rides_global || ev.created_by == u.id);
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
                "attendance_enabled": tracked,
                "attendance_rate_pct": rate,
                "need_ride": ev.need_ride,
                "self_transport": ev.self_transport,
                "rides_matched": if show_ev_rides { Some(ev.rides_matched) } else { None },
                "rides_unfilled": if show_ev_rides { Some(ev.rides_unfilled) } else { None },
            })
        })
        .collect();

    let comparison = if let Some((
        prev_period,
        (
            prev_events_count,
            prev_total_going,
            prev_total_attended,
            prev_tracked_going,
            prev_tracked_attended,
        ),
    )) = prev_ev_stats
    {
        let prev_rate = if prev_tracked_going > 0 {
            (prev_tracked_attended * 100) / prev_tracked_going
        } else {
            0
        };
        json!({
            "prev_period": prev_period,
            "events_count_delta": events_count - prev_events_count,
            "total_going_delta": total_going - prev_total_going,
            "total_attended_delta": total_attended - prev_total_attended,
            "attendance_rate_delta": attendance_rate_pct - prev_rate,
        })
    } else {
        Value::Null
    };

    Ok(Json(json!({
        "scope": if global_events { "global" } else { "own_events" },
        "period": w.period_label,
        "range": w.range,
        "from": w.start,
        "to": w.display_to,
        "events_total": events_total,
        "events_truncated": events_truncated,
        "comparison": comparison,
        "totals": {
            "events_count": events_count,
            "tracked_events_count": tracked_events_count,
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
