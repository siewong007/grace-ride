//! Admin multi-domain overview report strictly filtered by each caller's permissions and scope.

use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::{json, Value};

use super::{month_buckets, parse_period_window, PeriodQuery, ReportCaps};
use crate::auth::CurrentUser;
use crate::error::{forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{sql_holds, Act, Page, Perm};
use crate::state::AppState;
use crate::util::require;

pub(in crate::routes::admin) async fn overview_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<PeriodQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReports, Act::View)?;
    let caps = ReportCaps::from_user(&u);
    if !caps.overview {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }

    let w = parse_period_window(
        u.lang,
        q.month.as_deref(),
        q.range.as_deref(),
        q.from.as_deref(),
        q.to.as_deref(),
    )?;

    let mut rides_by_month: HashMap<String, i64> = HashMap::new();
    let transport = if caps.demand {
        let scope_creator: Option<i64> = if caps.demand_global { None } else { Some(u.id) };

        #[derive(sqlx::FromRow)]
        struct RideLite {
            pickup_at: String,
            status: String,
            driver_id: Option<i64>,
            passenger_id: i64,
            distance_km: Option<i64>,
        }
        let rides: Vec<RideLite> = sqlx::query_as(
            "SELECT r.pickup_at, r.status, r.driver_id, r.passenger_id, r.distance_km
             FROM rides r
             WHERE r.pickup_at >= $1 AND r.pickup_at < $2
               AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .fetch_all(&st.db)
        .await?;

        let mut total_rides = 0_i64;
        let mut completed_rides = 0_i64;
        let mut fulfilled_rides = 0_i64;
        let mut unfilled_rides = 0_i64;
        let mut cancelled_rides = 0_i64;
        let mut no_show_rides = 0_i64;
        let mut alt_transport_rides = 0_i64;
        let mut total_distance_km = 0_i64;
        let mut active_drivers: HashSet<i64> = HashSet::new();
        let mut unique_passengers: HashSet<i64> = HashSet::new();

        for r in &rides {
            match r.status.as_str() {
                "cancelled" => cancelled_rides += 1,
                "no_show" => {
                    total_rides += 1;
                    no_show_rides += 1;
                    unique_passengers.insert(r.passenger_id);
                    if let Some(d) = r.driver_id {
                        active_drivers.insert(d);
                    }
                }
                "alt_transport" => {
                    total_rides += 1;
                    alt_transport_rides += 1;
                    unique_passengers.insert(r.passenger_id);
                    if let Some(d) = r.driver_id {
                        active_drivers.insert(d);
                    }
                }
                _ => {
                    total_rides += 1;
                    unique_passengers.insert(r.passenger_id);
                    if let Some(d) = r.driver_id {
                        active_drivers.insert(d);
                    } else {
                        unfilled_rides += 1;
                    }
                    if matches!(r.status.as_str(), "confirmed" | "in_progress" | "completed") {
                        fulfilled_rides += 1;
                    }
                    if r.status == "completed" {
                        completed_rides += 1;
                        total_distance_km += r.distance_km.unwrap_or(0);
                        if r.pickup_at.len() >= 7 {
                            *rides_by_month
                                .entry(r.pickup_at[..7].to_string())
                                .or_default() += 1;
                        }
                    }
                }
            }
        }

        let fulfillment_rate_pct = if total_rides > 0 {
            (fulfilled_rides * 100) / total_rides
        } else {
            100
        };

        #[derive(sqlx::FromRow)]
        struct TopDriverRow {
            user_id: i64,
            name: String,
            completed_rides: i64,
            distance_km: i64,
            carpool_trips: i64,
        }
        let top_drivers: Vec<TopDriverRow> = sqlx::query_as(
            "WITH scoped AS (
                SELECT r.driver_id, r.trip_id, COALESCE(r.distance_km, 0) AS distance_km
                FROM rides r
                WHERE r.status = 'completed' AND r.driver_id IS NOT NULL
                  AND r.pickup_at >= $1 AND r.pickup_at < $2
                  AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))
             ),
             carpool AS (
                SELECT driver_id, COUNT(*)::BIGINT AS carpool_trips
                FROM (
                    SELECT driver_id, trip_id
                    FROM scoped
                    WHERE trip_id IS NOT NULL
                    GROUP BY driver_id, trip_id
                    HAVING COUNT(*) >= 2
                ) t
                GROUP BY driver_id
             )
             SELECT s.driver_id AS user_id, u.name,
                    COUNT(*)::BIGINT AS completed_rides,
                    COALESCE(SUM(s.distance_km), 0)::BIGINT AS distance_km,
                    COALESCE(MAX(c.carpool_trips), 0)::BIGINT AS carpool_trips
             FROM scoped s
             JOIN users u ON u.id = s.driver_id
             LEFT JOIN carpool c ON c.driver_id = s.driver_id
             GROUP BY s.driver_id, u.name
             ORDER BY completed_rides DESC, distance_km DESC, u.name ASC
             LIMIT 10",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .fetch_all(&st.db)
        .await?;

        json!({
            "scope": if caps.demand_global { "global" } else { "own_events" },
            "total_rides": total_rides,
            "completed_rides": completed_rides,
            "unfilled_rides": unfilled_rides,
            "cancelled_rides": cancelled_rides,
            "no_show_rides": no_show_rides,
            "alt_transport_rides": alt_transport_rides,
            "fulfillment_rate_pct": fulfillment_rate_pct,
            "total_distance_km": total_distance_km,
            "active_drivers": active_drivers.len(),
            "unique_passengers": unique_passengers.len(),
            "top_drivers": top_drivers.into_iter().map(|d| json!({
                "user_id": d.user_id,
                "name": d.name,
                "completed_rides": d.completed_rides,
                "distance_km": d.distance_km,
                "carpool_trips": d.carpool_trips,
            })).collect::<Vec<_>>(),
        })
    } else {
        Value::Null
    };

    let mut events_by_month: HashMap<String, i64> = HashMap::new();
    let events = if caps.events {
        let scope_creator: Option<i64> = if caps.events_global { None } else { Some(u.id) };
        let (events_count, total_going, total_attended): (i64, i64, i64) = sqlx::query_as(
            "SELECT
                COUNT(*)::BIGINT,
                COALESCE((SELECT COUNT(*) FROM event_signups s JOIN events e2 ON e2.id = s.event_id
                          WHERE e2.status NOT IN ('draft', 'cancelled')
                            AND e2.starts_at >= $1 AND e2.starts_at < $2
                            AND ($3::BIGINT IS NULL OR e2.created_by = $3)
                            AND s.status = 'going'), 0)::BIGINT,
                COALESCE((SELECT COUNT(*) FROM event_signups s JOIN events e2 ON e2.id = s.event_id
                          WHERE e2.status NOT IN ('draft', 'cancelled')
                            AND e2.starts_at >= $1 AND e2.starts_at < $2
                            AND ($3::BIGINT IS NULL OR e2.created_by = $3)
                            AND s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT
             FROM events e
             WHERE e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2
               AND ($3::BIGINT IS NULL OR e.created_by = $3)",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .fetch_one(&st.db)
        .await?;

        let ev_months: Vec<(String, i64)> = sqlx::query_as(
            "SELECT substr(e.starts_at, 1, 7) AS m, COUNT(*)::BIGINT
             FROM event_signups s
             JOIN events e ON e.id = s.event_id
             WHERE e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2
               AND ($3::BIGINT IS NULL OR e.created_by = $3)
               AND s.status <> 'cancelled' AND s.attended = 1
             GROUP BY m",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .fetch_all(&st.db)
        .await?;
        for (m, c) in ev_months {
            events_by_month.insert(m, c);
        }

        let top_groups: Vec<(Option<i64>, Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT s.as_group_id, g.name,
                    COUNT(*) FILTER (WHERE s.status = 'going')::BIGINT AS going,
                    COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS attended
             FROM event_signups s
             JOIN events e ON e.id = s.event_id
             LEFT JOIN groups g ON g.id = s.as_group_id
             WHERE e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2
               AND ($3::BIGINT IS NULL OR e.created_by = $3)
             GROUP BY s.as_group_id, g.name
             ORDER BY going DESC, g.name ASC NULLS LAST
             LIMIT 5",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .fetch_all(&st.db)
        .await?;

        let attendance_rate_pct = if total_going > 0 {
            (total_attended * 100) / total_going
        } else {
            0
        };
        json!({
            "scope": if caps.events_global { "global" } else { "own_events" },
            "events_count": events_count,
            "total_going": total_going,
            "total_attended": total_attended,
            "attendance_rate_pct": attendance_rate_pct,
            "top_groups": top_groups.into_iter().map(|(gid, gname, going, attended)| json!({
                "group_id": gid,
                "group_name": gname,
                "going": going,
                "attended": attended,
            })).collect::<Vec<_>>(),
        })
    } else {
        Value::Null
    };

    let reimb_review = if caps.reimb_review {
        let (pending_count, pending_cents, returned_count, approved_count, approved_cents): (
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = sqlx::query_as(
            "SELECT
                COUNT(*) FILTER (WHERE status = 'pending')::BIGINT,
                COALESCE(SUM(amount_cents) FILTER (WHERE status = 'pending'), 0)::BIGINT,
                COUNT(*) FILTER (WHERE status = 'returned')::BIGINT,
                COUNT(*) FILTER (WHERE status = 'approved')::BIGINT,
                COALESCE(SUM(amount_cents) FILTER (WHERE status = 'approved'), 0)::BIGINT
             FROM reimbursements
             WHERE created_at >= $1 AND created_at < $2",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_one(&st.db)
        .await?;
        json!({
            "pending_count": pending_count,
            "pending_cents": pending_cents,
            "returned_count": returned_count,
            "approved_count": approved_count,
            "approved_cents": approved_cents,
        })
    } else {
        Value::Null
    };

    let mut reimb_paid_by_month: HashMap<String, i64> = HashMap::new();
    let mut bills_collected_by_month: HashMap<String, i64> = HashMap::new();
    let finance = if caps.reimb_pay || caps.bills {
        let reimb_payouts = if caps.reimb_pay {
            let (approved_unpaid_count, approved_unpaid_cents): (i64, i64) = sqlx::query_as(
                "SELECT COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
                 FROM reimbursements
                 WHERE status = 'approved' AND created_at >= $1 AND created_at < $2",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_one(&st.db)
            .await?;

            let (paid_count, paid_cents): (i64, i64) = sqlx::query_as(
                "SELECT COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
                 FROM reimbursements
                 WHERE status = 'paid'
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_one(&st.db)
            .await?;

            let paid_months: Vec<(String, i64)> = sqlx::query_as(
                "SELECT substr(COALESCE(NULLIF(paid_at, ''), updated_at, created_at), 1, 7) AS m,
                        COALESCE(SUM(amount_cents), 0)::BIGINT
                 FROM reimbursements
                 WHERE status = 'paid'
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2
                 GROUP BY m",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_all(&st.db)
            .await?;
            for (m, c) in paid_months {
                reimb_paid_by_month.insert(m, c);
            }

            let (open_batches, paid_batches): (i64, i64) = sqlx::query_as(
                "SELECT
                    COUNT(*) FILTER (WHERE status = 'open')::BIGINT,
                    COUNT(*) FILTER (WHERE status = 'paid')::BIGINT
                 FROM payout_batches
                 WHERE (created_at >= $1 AND created_at < $2)
                    OR (month >= $3 AND month <= $4)",
            )
            .bind(&w.start)
            .bind(&w.end)
            .bind(&w.start[..7])
            .bind(&w.end_ym_incl)
            .fetch_one(&st.db)
            .await?;

            json!({
                "approved_unpaid_count": approved_unpaid_count,
                "approved_unpaid_cents": approved_unpaid_cents,
                "paid_count": paid_count,
                "paid_cents": paid_cents,
                "open_batches": open_batches,
                "paid_batches": paid_batches,
            })
        } else {
            Value::Null
        };

        let bills = if caps.bills {
            let (published_count, closed_count, gross_cents, subsidy_cents, collected_cents, outstanding_cents): (
                i64,
                i64,
                i64,
                i64,
                i64,
                i64,
            ) = sqlx::query_as(
                "WITH sc AS (
                    SELECT s.status,
                           s.subsidy_cents,
                           COALESCE((SELECT SUM(si.amount_cents) FROM settlement_items si WHERE si.settlement_id = s.id), 0)::BIGINT AS gross_cents,
                           COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'confirmed'), 0)::BIGINT AS collected_cents
                    FROM settlements s
                    JOIN events e ON e.id = s.event_id
                    WHERE s.status <> 'draft' AND e.status <> 'draft'
                      AND e.starts_at >= $1 AND e.starts_at < $2
                )
                SELECT
                    COUNT(*) FILTER (WHERE status = 'published')::BIGINT,
                    COUNT(*) FILTER (WHERE status = 'closed')::BIGINT,
                    COALESCE(SUM(gross_cents), 0)::BIGINT,
                    COALESCE(SUM(subsidy_cents), 0)::BIGINT,
                    COALESCE(SUM(collected_cents), 0)::BIGINT,
                    COALESCE(SUM(GREATEST(0, (gross_cents - subsidy_cents) - collected_cents)), 0)::BIGINT
                FROM sc",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_one(&st.db)
            .await?;

            let bill_months: Vec<(String, i64)> = sqlx::query_as(
                "SELECT substr(e.starts_at, 1, 7) AS m,
                        COALESCE(SUM(sh.amount_cents), 0)::BIGINT
                 FROM settlement_shares sh
                 JOIN settlements s ON s.id = sh.settlement_id
                 JOIN events e ON e.id = s.event_id
                 WHERE s.status <> 'draft' AND e.status <> 'draft'
                   AND sh.status = 'confirmed'
                   AND e.starts_at >= $1 AND e.starts_at < $2
                 GROUP BY m",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_all(&st.db)
            .await?;
            for (m, c) in bill_months {
                bills_collected_by_month.insert(m, c);
            }

            let net = (gross_cents - subsidy_cents).max(0);
            let collection_rate_pct = if net > 0 {
                (collected_cents * 100) / net
            } else {
                0
            };
            json!({
                "published_count": published_count,
                "closed_count": closed_count,
                "gross_cents": gross_cents,
                "subsidy_cents": subsidy_cents,
                "collected_cents": collected_cents,
                "outstanding_cents": outstanding_cents,
                "collection_rate_pct": collection_rate_pct,
            })
        } else {
            Value::Null
        };

        json!({
            "reimb_payouts": reimb_payouts,
            "bills": bills,
        })
    } else {
        Value::Null
    };

    let membership = if caps.membership {
        let drive_cond = sql_holds("u", Perm(Page::Drive, Act::Create));
        let (active_members, pending_members, new_members_in_period, active_drivers_total): (
            i64,
            i64,
            i64,
            i64,
        ) = sqlx::query_as(crate::dyn_sql!(
            "SELECT
                COUNT(*) FILTER (WHERE u.status = 'active')::BIGINT,
                COUNT(*) FILTER (WHERE u.status = 'pending')::BIGINT,
                COUNT(*) FILTER (WHERE u.created_at >= $1 AND u.created_at < $2)::BIGINT,
                COUNT(*) FILTER (WHERE {drive_cond})::BIGINT
             FROM users u"
        ))
        .bind(&w.start)
        .bind(&w.end)
        .fetch_one(&st.db)
        .await?;
        json!({
            "active_members": active_members,
            "pending_members": pending_members,
            "new_members_in_period": new_members_in_period,
            "active_drivers_total": active_drivers_total,
        })
    } else {
        Value::Null
    };

    let monthly_series: Vec<Value> = month_buckets(&w)
        .into_iter()
        .map(|m| {
            json!({
                "month": m,
                "rides_completed": if caps.demand { Some(rides_by_month.get(&m).copied().unwrap_or(0)) } else { None },
                "events_attended": if caps.events { Some(events_by_month.get(&m).copied().unwrap_or(0)) } else { None },
                "reimb_paid_cents": if caps.reimb_pay { Some(reimb_paid_by_month.get(&m).copied().unwrap_or(0)) } else { None },
                "bills_collected_cents": if caps.bills { Some(bills_collected_by_month.get(&m).copied().unwrap_or(0)) } else { None },
            })
        })
        .collect();

    Ok(Json(json!({
        "period": w.period_label,
        "range": w.range,
        "from": w.start,
        "to": w.display_to,
        "transport": transport,
        "events": events,
        "reimb_review": reimb_review,
        "finance": finance,
        "membership": membership,
        "monthly_series": monthly_series,
    })))
}
