//! Admin ride-demand breakdown reports by area, church group, and time slot.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{Datelike, NaiveDate};
use serde_json::{json, Value};

use super::{parse_period_window, prev_period_window, PeriodQuery};
use crate::auth::CurrentUser;
use crate::error::{forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{sql_holds, Act, Page, Perm};
use crate::state::AppState;
use crate::util::require;

pub(in crate::routes::admin) async fn demand_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<PeriodQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReports, Act::View)?;
    if !(u.can(Page::AdminRides, Act::View) || u.can(Page::AdminPlan, Act::View)) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }

    let w = parse_period_window(
        u.lang,
        q.month.as_deref(),
        q.range.as_deref(),
        q.from.as_deref(),
        q.to.as_deref(),
    )?;

    #[derive(sqlx::FromRow)]
    struct DemandRow {
        area_id: i64,
        area_zh: String,
        area_en: String,
        pickup_at: String,
        status: String,
        driver_id: Option<i64>,
        passenger_id: i64,
        seats_needed: i64,
        assistance: String,
        leg: String,
        is_proxy: i64,
        as_group_id: Option<i64>,
        group_name: Option<String>,
    }

    let global_scope = u.traffic_is_global();
    let scope_creator: Option<i64> = if global_scope { None } else { Some(u.id) };

    let rows: Vec<DemandRow> = sqlx::query_as(
        "SELECT r.pickup_area_id AS area_id, COALESCE(a.name, '') AS area_zh, COALESCE(a.name, '') AS area_en,
                r.pickup_at, r.status, r.driver_id, r.passenger_id, r.seats AS seats_needed,
                COALESCE(r.assistance, '') AS assistance, COALESCE(r.leg, 'outbound') AS leg,
                r.is_proxy, r.as_group_id, g.name AS group_name
         FROM rides r
         LEFT JOIN areas a ON a.id = r.pickup_area_id
         LEFT JOIN groups g ON g.id = r.as_group_id
         WHERE r.pickup_at >= $1 AND r.pickup_at < $2
           AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))
           AND ($4::BIGINT IS NULL OR r.as_group_id = $4)",
    )
    .bind(&w.start)
    .bind(&w.end)
    .bind(scope_creator)
    .bind(q.group_id)
    .fetch_all(&st.db)
    .await?;

    let (plan_visitors_count, plan_visitors_seats): (Option<i64>, Option<i64>) =
        if q.group_id.is_some() {
            (None, None)
        } else {
            let (cnt, seats): (i64, i64) = sqlx::query_as(
                "SELECT COUNT(*)::BIGINT, COALESCE(SUM(pv.seats), 0)::BIGINT
                 FROM plan_visitors pv
                 JOIN events e ON e.id = pv.event_id
                 WHERE e.starts_at >= $1 AND e.starts_at < $2 AND e.status <> 'draft'
                   AND ($3::BIGINT IS NULL OR e.created_by = $3)",
            )
            .bind(&w.start)
            .bind(&w.end)
            .bind(scope_creator)
            .fetch_one(&st.db)
            .await?;
            (Some(cnt), Some(seats))
        };

    let (visitor_passes_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::BIGINT
         FROM visitor_passes vp
         JOIN events e ON e.id = vp.event_id
         WHERE e.starts_at >= $1 AND e.starts_at < $2 AND e.status <> 'draft'
           AND ($3::BIGINT IS NULL OR e.created_by = $3)
           AND ($4::BIGINT IS NULL OR vp.host_group_id = $4)",
    )
    .bind(&w.start)
    .bind(&w.end)
    .bind(scope_creator)
    .bind(q.group_id)
    .fetch_one(&st.db)
    .await?;

    let drivers_by_area: HashMap<i64, i64> = if global_scope {
        let drive_cond = sql_holds("u", Perm(Page::Drive, Act::Create));
        let area_rows: Vec<(i64, i64)> = sqlx::query_as(crate::dyn_sql!(
            "SELECT da.area_id, COUNT(DISTINCT da.user_id)::BIGINT
             FROM driver_areas da
             JOIN users u ON u.id = da.user_id
             WHERE {drive_cond}
             GROUP BY da.area_id"
        ))
        .fetch_all(&st.db)
        .await?;
        area_rows.into_iter().collect()
    } else {
        HashMap::new()
    };

    let mut total_requests = 0_i64;
    let mut fulfilled = 0_i64;
    let mut pending_confirmation = 0_i64;
    let mut unfilled = 0_i64;
    let mut completed = 0_i64;
    let mut no_show = 0_i64;
    let mut alt_transport = 0_i64;
    let mut cancelled_total = 0_i64;
    let mut cancelled_unassigned = 0_i64;
    let mut wheelchair_requests = 0_i64;
    let mut stroller_requests = 0_i64;
    let mut oxygen_requests = 0_i64;
    let mut return_requests = 0_i64;
    let mut proxy_rides = 0_i64;
    let mut seats_unfilled = 0_i64;
    let mut drivers_active: HashSet<i64> = HashSet::new();
    let mut passengers_seen: HashSet<i64> = HashSet::new();

    #[derive(Default)]
    struct Bucket {
        area_zh: String,
        area_en: String,
        total: i64,
        fulfilled: i64,
        unfilled: i64,
        cancelled_unassigned: i64,
        seats_needed: i64,
        seats_unfilled: i64,
    }

    #[derive(Default)]
    struct GroupBucket {
        group_name: Option<String>,
        total: i64,
        fulfilled: i64,
        unfilled: i64,
        seats_needed: i64,
    }

    let mut by_area: HashMap<i64, Bucket> = HashMap::new();
    let mut by_slot: BTreeMap<(u32, String), Bucket> = BTreeMap::new();
    let mut by_group: HashMap<Option<i64>, GroupBucket> = HashMap::new();

    for r in &rows {
        let is_cancelled = r.status == "cancelled";
        let is_no_show = r.status == "no_show";
        let is_alt = r.status == "alt_transport";
        let is_closed_alt = is_no_show || is_alt;
        let is_fulfilled = matches!(r.status.as_str(), "confirmed" | "in_progress" | "completed");
        let is_unfilled = !is_cancelled && !is_closed_alt && r.driver_id.is_none();
        let is_pending_confirm =
            !is_cancelled && !is_closed_alt && !is_fulfilled && r.driver_id.is_some();

        if is_cancelled {
            cancelled_total += 1;
            if r.driver_id.is_none() {
                cancelled_unassigned += 1;
            }
        } else {
            total_requests += 1;
            passengers_seen.insert(r.passenger_id);
            if r.status == "completed" {
                completed += 1;
            }
            if is_no_show {
                no_show += 1;
            }
            if is_alt {
                alt_transport += 1;
            }
            if let Some(did) = r.driver_id {
                drivers_active.insert(did);
            }
            if is_fulfilled {
                fulfilled += 1;
            }
            if is_pending_confirm {
                pending_confirmation += 1;
            }
            if is_unfilled {
                unfilled += 1;
                seats_unfilled += r.seats_needed;
            }
            let assist_lower = r.assistance.to_ascii_lowercase();
            if assist_lower.contains("wheelchair") || r.assistance.contains("轮椅") {
                wheelchair_requests += 1;
            }
            if assist_lower.contains("stroller")
                || r.assistance.contains("婴儿车")
                || r.assistance.contains("推车")
            {
                stroller_requests += 1;
            }
            if assist_lower.contains("oxygen") || r.assistance.contains("氧气") {
                oxygen_requests += 1;
            }
            if r.leg == "return" {
                return_requests += 1;
            }
            if r.is_proxy == 1 {
                proxy_rides += 1;
            }

            let gb = by_group
                .entry(r.as_group_id)
                .or_insert_with(|| GroupBucket {
                    group_name: r.group_name.clone(),
                    ..Default::default()
                });
            gb.total += 1;
            gb.seats_needed += r.seats_needed;
            if is_fulfilled {
                gb.fulfilled += 1;
            }
            if is_unfilled {
                gb.unfilled += 1;
            }
        }

        let ab = by_area.entry(r.area_id).or_insert_with(|| Bucket {
            area_zh: r.area_zh.clone(),
            area_en: r.area_en.clone(),
            ..Default::default()
        });
        if is_cancelled {
            if r.driver_id.is_none() {
                ab.cancelled_unassigned += 1;
            }
        } else {
            ab.total += 1;
            ab.seats_needed += r.seats_needed;
            if is_fulfilled {
                ab.fulfilled += 1;
            }
            if is_unfilled {
                ab.unfilled += 1;
                ab.seats_unfilled += r.seats_needed;
            }
        }

        if let (Some(date_part), Some(hour_part)) = (r.pickup_at.get(..10), r.pickup_at.get(11..13))
        {
            if let Ok(d) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                let wd = d.weekday().num_days_from_sunday();
                let hour: u32 = hour_part.parse().unwrap_or(0);
                let bucket_h = (hour / 2) * 2;
                let slot_label = format!("{bucket_h:02}:00-{:02}:00", (bucket_h + 2).min(24));
                let sb = by_slot.entry((wd, slot_label)).or_default();
                if is_cancelled {
                    if r.driver_id.is_none() {
                        sb.cancelled_unassigned += 1;
                    }
                } else {
                    sb.total += 1;
                    sb.seats_needed += r.seats_needed;
                    if is_fulfilled {
                        sb.fulfilled += 1;
                    }
                    if is_unfilled {
                        sb.unfilled += 1;
                        sb.seats_unfilled += r.seats_needed;
                    }
                }
            }
        }
    }

    let mut area_list: Vec<Value> = by_area
        .into_iter()
        .map(|(area_id, b)| {
            json!({
                "area_id": area_id,
                "area_name_zh": b.area_zh,
                "area_name_en": b.area_en,
                "total": b.total,
                "fulfilled": b.fulfilled,
                "unfilled": b.unfilled,
                "cancelled_unassigned": b.cancelled_unassigned,
                "seats_needed": b.seats_needed,
                "seats_unfilled": b.seats_unfilled,
                "fulfillment_rate_pct": if b.total > 0 { (b.fulfilled * 100) / b.total } else { 100 },
                "registered_drivers": if global_scope { Some(drivers_by_area.get(&area_id).copied().unwrap_or(0)) } else { None },
            })
        })
        .collect();
    area_list.sort_by(|a, b| {
        b["unfilled"]
            .as_i64()
            .cmp(&a["unfilled"].as_i64())
            .then_with(|| b["total"].as_i64().cmp(&a["total"].as_i64()))
    });

    let mut slot_list: Vec<Value> = by_slot
        .into_iter()
        .filter(|(_, b)| b.total > 0 || b.cancelled_unassigned > 0)
        .map(|((wd, slot), b)| {
            json!({
                "weekday": wd,
                "slot": slot,
                "total": b.total,
                "fulfilled": b.fulfilled,
                "unfilled": b.unfilled,
                "cancelled_unassigned": b.cancelled_unassigned,
                "seats_needed": b.seats_needed,
            })
        })
        .collect();
    slot_list.sort_by(|a, b| {
        b["unfilled"]
            .as_i64()
            .cmp(&a["unfilled"].as_i64())
            .then_with(|| b["total"].as_i64().cmp(&a["total"].as_i64()))
    });

    let mut group_list: Vec<Value> = by_group
        .into_iter()
        .map(|(group_id, b)| {
            json!({
                "group_id": group_id,
                "group_name": b.group_name,
                "total": b.total,
                "fulfilled": b.fulfilled,
                "unfilled": b.unfilled,
                "seats_needed": b.seats_needed,
                "fulfillment_rate_pct": if b.total > 0 { (b.fulfilled * 100) / b.total } else { 100 },
            })
        })
        .collect();
    group_list.sort_by(|a, b| {
        b["unfilled"]
            .as_i64()
            .cmp(&a["unfilled"].as_i64())
            .then_with(|| b["total"].as_i64().cmp(&a["total"].as_i64()))
    });

    let fulfillment_rate_pct = if total_requests > 0 {
        (fulfilled * 100) / total_requests
    } else {
        100
    };

    let comparison = if let Some(pw) = prev_period_window(&w) {
        let (prev_total, prev_fulfilled, prev_unfilled): (i64, i64, i64) = sqlx::query_as(
            "SELECT
                COUNT(*) FILTER (WHERE r.status <> 'cancelled')::BIGINT,
                COUNT(*) FILTER (WHERE r.status IN ('confirmed', 'in_progress', 'completed'))::BIGINT,
                COUNT(*) FILTER (WHERE r.status NOT IN ('cancelled', 'no_show', 'alt_transport') AND r.driver_id IS NULL)::BIGINT
             FROM rides r
             WHERE r.pickup_at >= $1 AND r.pickup_at < $2
               AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))
               AND ($4::BIGINT IS NULL OR r.as_group_id = $4)",
        )
        .bind(&pw.start)
        .bind(&pw.end)
        .bind(scope_creator)
        .bind(q.group_id)
        .fetch_one(&st.db)
        .await?;
        let prev_rate = if prev_total > 0 {
            (prev_fulfilled * 100) / prev_total
        } else {
            100
        };
        json!({
            "prev_period": pw.period_label,
            "total_requests_delta": total_requests - prev_total,
            "unfilled_delta": unfilled - prev_unfilled,
            "fulfillment_rate_delta": fulfillment_rate_pct - prev_rate,
        })
    } else {
        Value::Null
    };

    Ok(Json(json!({
        "scope": if global_scope { "global" } else { "own_events" },
        "period": w.period_label,
        "range": w.range,
        "from": w.start,
        "to": w.display_to,
        "comparison": comparison,
        "totals": {
            "total_requests": total_requests,
            "fulfilled": fulfilled,
            "pending_confirmation": pending_confirmation,
            "unfilled": unfilled,
            "completed": completed,
            "no_show": no_show,
            "alt_transport": alt_transport,
            "cancelled_total": cancelled_total,
            "cancelled_unassigned": cancelled_unassigned,
            "wheelchair_requests": wheelchair_requests,
            "stroller_requests": stroller_requests,
            "oxygen_requests": oxygen_requests,
            "return_requests": return_requests,
            "proxy_rides": proxy_rides,
            "plan_visitors_count": plan_visitors_count,
            "plan_visitors_seats": plan_visitors_seats,
            "visitor_passes_count": visitor_passes_count,
            "seats_unfilled": seats_unfilled,
            "active_drivers": drivers_active.len(),
            "unique_passengers": passengers_seen.len(),
            "fulfillment_rate_pct": fulfillment_rate_pct,
        },
        "by_area": area_list,
        "by_slot": slot_list,
        "by_group": group_list,
    })))
}
