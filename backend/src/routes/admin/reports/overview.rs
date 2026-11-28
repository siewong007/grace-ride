//! Admin multi-domain overview report strictly filtered by each caller's permissions and scope.

use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::{json, Value};

use super::{month_buckets, parse_period_window, prev_period_window, PeriodQuery, ReportCaps};
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

    let scope_rides_sql = |alias: &str, p: usize| -> String {
        if caps.demand_global {
            String::new()
        } else {
            format!(" AND {alias}.event_id IN (SELECT id FROM events WHERE created_by = ${p})")
        }
    };
    let scope_ev_sql = |alias: Option<&str>, p: usize| -> String {
        if caps.events_global {
            String::new()
        } else if let Some(a) = alias {
            format!(" AND {a}.created_by = ${p}")
        } else {
            format!(" AND created_by = ${p}")
        }
    };

    #[derive(sqlx::FromRow)]
    struct RideStatsRow {
        total_rides: i64,
        completed_rides: i64,
        fulfilled_rides: i64,
        pending_confirmation_rides: i64,
        unfilled_rides: i64,
        cancelled_rides: i64,
        no_show_rides: i64,
        alt_transport_rides: i64,
        total_distance_km: i64,
        active_drivers: i64,
        unique_passengers: i64,
    }
    #[derive(sqlx::FromRow)]
    struct TopDriverRow {
        user_id: i64,
        name: String,
        completed_rides: i64,
        distance_km: i64,
        carpool_trips: i64,
    }

    let tr_fut = async {
        if !caps.demand {
            return Ok::<_, crate::error::AppError>((Value::Null, HashMap::new()));
        }
        let ride_filter = scope_rides_sql("r", 3);
        let rs_sql = format!(
            "SELECT
                COUNT(*) FILTER (WHERE r.status <> 'cancelled')::BIGINT AS total_rides,
                COUNT(*) FILTER (WHERE r.status = 'completed')::BIGINT AS completed_rides,
                COUNT(*) FILTER (WHERE r.status IN ('confirmed', 'in_progress', 'completed'))::BIGINT AS fulfilled_rides,
                COUNT(*) FILTER (WHERE r.status NOT IN ('cancelled', 'no_show', 'alt_transport', 'confirmed', 'in_progress', 'completed') AND r.driver_id IS NOT NULL)::BIGINT AS pending_confirmation_rides,
                COUNT(*) FILTER (WHERE r.status NOT IN ('cancelled', 'no_show', 'alt_transport') AND r.driver_id IS NULL)::BIGINT AS unfilled_rides,
                COUNT(*) FILTER (WHERE r.status = 'cancelled')::BIGINT AS cancelled_rides,
                COUNT(*) FILTER (WHERE r.status = 'no_show')::BIGINT AS no_show_rides,
                COUNT(*) FILTER (WHERE r.status = 'alt_transport')::BIGINT AS alt_transport_rides,
                COALESCE(SUM(r.distance_km) FILTER (WHERE r.status = 'completed'), 0)::BIGINT AS total_distance_km,
                COUNT(DISTINCT r.driver_id) FILTER (WHERE r.status <> 'cancelled' AND r.driver_id IS NOT NULL)::BIGINT AS active_drivers,
                COUNT(DISTINCT r.passenger_id) FILTER (WHERE r.status <> 'cancelled')::BIGINT AS unique_passengers
             FROM rides r
             WHERE r.pickup_at >= $1 AND r.pickup_at < $2{ride_filter}"
        );
        let months_sql = format!(
            "SELECT substr(r.pickup_at, 1, 7) AS m, COUNT(*)::BIGINT
             FROM rides r
             WHERE r.status = 'completed'
               AND r.pickup_at >= $1 AND r.pickup_at < $2{ride_filter}
             GROUP BY m"
        );
        let top_sql = format!(
            "WITH scoped AS (
                SELECT r.driver_id, r.trip_id, COALESCE(r.distance_km, 0) AS distance_km
                FROM rides r
                WHERE r.status = 'completed' AND r.driver_id IS NOT NULL
                  AND r.pickup_at >= $1 AND r.pickup_at < $2{ride_filter}
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
             LIMIT 10"
        );

        let mut rs_q = sqlx::query_as::<_, RideStatsRow>(sqlx::AssertSqlSafe(&*rs_sql))
            .bind(&w.start)
            .bind(&w.end);
        let mut rm_q = sqlx::query_as::<_, (String, i64)>(sqlx::AssertSqlSafe(&*months_sql))
            .bind(&w.start)
            .bind(&w.end);
        let mut td_q = sqlx::query_as::<_, TopDriverRow>(sqlx::AssertSqlSafe(&*top_sql))
            .bind(&w.start)
            .bind(&w.end);
        if !caps.demand_global {
            rs_q = rs_q.bind(u.id);
            rm_q = rm_q.bind(u.id);
            td_q = td_q.bind(u.id);
        }
        let (rs, ride_months, top_drivers) = tokio::try_join!(
            rs_q.fetch_one(&st.db),
            rm_q.fetch_all(&st.db),
            td_q.fetch_all(&st.db),
        )?;
        let rides_by_month: HashMap<String, i64> = ride_months.into_iter().collect();
        let fulfillment_rate_pct = if rs.total_rides > 0 {
            (rs.fulfilled_rides * 100) / rs.total_rides
        } else {
            100
        };
        Ok((
            json!({
                "scope": if caps.demand_global { "global" } else { "own_events" },
                "total_rides": rs.total_rides,
                "completed_rides": rs.completed_rides,
                "pending_confirmation_rides": rs.pending_confirmation_rides,
                "unfilled_rides": rs.unfilled_rides,
                "cancelled_rides": rs.cancelled_rides,
                "no_show_rides": rs.no_show_rides,
                "alt_transport_rides": rs.alt_transport_rides,
                "fulfillment_rate_pct": fulfillment_rate_pct,
                "total_distance_km": rs.total_distance_km,
                "active_drivers": rs.active_drivers,
                "unique_passengers": rs.unique_passengers,
                "top_drivers": top_drivers.into_iter().map(|d| json!({
                    "user_id": d.user_id,
                    "name": d.name,
                    "completed_rides": d.completed_rides,
                    "distance_km": d.distance_km,
                    "carpool_trips": d.carpool_trips,
                })).collect::<Vec<_>>(),
            }),
            rides_by_month,
        ))
    };

    let ev_fut = async {
        if !caps.events {
            return Ok::<_, crate::error::AppError>((Value::Null, HashMap::new()));
        }
        let ev_cond = scope_ev_sql(None, 3);
        let e_cond = scope_ev_sql(Some("e"), 3);
        let totals_sql = format!(
            "WITH ev AS (
                 SELECT id, attendance_enabled
                 FROM events
                 WHERE status NOT IN ('draft', 'cancelled')
                   AND starts_at >= $1 AND starts_at < $2{ev_cond}
             ), sg AS (
                 SELECT
                     COUNT(*) FILTER (WHERE s.status = 'going')::BIGINT AS total_going,
                     COUNT(*) FILTER (WHERE ev.attendance_enabled = 1 AND s.status = 'going')::BIGINT AS tracked_going,
                     COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS total_attended,
                     COUNT(*) FILTER (WHERE ev.attendance_enabled = 1 AND s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS tracked_attended
                 FROM ev
                 JOIN event_signups s ON s.event_id = ev.id
             )
             SELECT
                 (SELECT COUNT(*)::BIGINT FROM ev),
                 (SELECT COUNT(*) FILTER (WHERE attendance_enabled = 1)::BIGINT FROM ev),
                 COALESCE((SELECT total_going FROM sg), 0)::BIGINT,
                 COALESCE((SELECT tracked_going FROM sg), 0)::BIGINT,
                 COALESCE((SELECT total_attended FROM sg), 0)::BIGINT,
                 COALESCE((SELECT tracked_attended FROM sg), 0)::BIGINT"
        );
        let ev_months_sql = format!(
            "SELECT substr(e.starts_at, 1, 7) AS m, COUNT(*)::BIGINT
             FROM event_signups s
             JOIN events e ON e.id = s.event_id
             WHERE e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2{e_cond}
               AND s.status <> 'cancelled' AND s.attended = 1
             GROUP BY m"
        );
        let top_groups_sql = format!(
            "SELECT s.as_group_id, g.name,
                    COUNT(*) FILTER (WHERE s.status = 'going')::BIGINT AS going,
                    COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1)::BIGINT AS attended
             FROM event_signups s
             JOIN events e ON e.id = s.event_id
             LEFT JOIN groups g ON g.id = s.as_group_id
             WHERE e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2{e_cond}
             GROUP BY s.as_group_id, g.name
             ORDER BY going DESC, g.name ASC NULLS LAST
             LIMIT 5"
        );
        let mut t_q =
            sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(sqlx::AssertSqlSafe(&*totals_sql))
                .bind(&w.start)
                .bind(&w.end);
        let mut em_q = sqlx::query_as::<_, (String, i64)>(sqlx::AssertSqlSafe(&*ev_months_sql))
            .bind(&w.start)
            .bind(&w.end);
        let mut tg_q = sqlx::query_as::<_, (Option<i64>, Option<String>, i64, i64)>(
            sqlx::AssertSqlSafe(&*top_groups_sql),
        )
        .bind(&w.start)
        .bind(&w.end);
        if !caps.events_global {
            t_q = t_q.bind(u.id);
            em_q = em_q.bind(u.id);
            tg_q = tg_q.bind(u.id);
        }
        let (
            (
                events_count,
                tracked_events_count,
                total_going,
                tracked_going,
                total_attended,
                tracked_attended,
            ),
            ev_months,
            top_groups,
        ) = tokio::try_join!(
            t_q.fetch_one(&st.db),
            em_q.fetch_all(&st.db),
            tg_q.fetch_all(&st.db),
        )?;
        let events_by_month: HashMap<String, i64> = ev_months.into_iter().collect();
        let attendance_rate_pct = if tracked_going > 0 {
            (tracked_attended * 100) / tracked_going
        } else {
            0
        };
        Ok((
            json!({
                "scope": if caps.events_global { "global" } else { "own_events" },
                "events_count": events_count,
                "tracked_events_count": tracked_events_count,
                "total_going": total_going,
                "total_attended": total_attended,
                "attendance_rate_pct": attendance_rate_pct,
                "top_groups": top_groups.into_iter().map(|(gid, gname, going, attended)| json!({
                    "group_id": gid,
                    "group_name": gname,
                    "going": going,
                    "attended": attended,
                })).collect::<Vec<_>>(),
            }),
            events_by_month,
        ))
    };

    let rev_fut = async {
        if !caps.reimb_review {
            return Ok::<_, crate::error::AppError>(Value::Null);
        }
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
        Ok(json!({
            "pending_count": pending_count,
            "pending_cents": pending_cents,
            "returned_count": returned_count,
            "approved_count": approved_count,
            "approved_cents": approved_cents,
        }))
    };

    let fin_fut = async {
        if !(caps.reimb_pay || caps.bills) {
            return Ok::<_, crate::error::AppError>((Value::Null, HashMap::new(), HashMap::new()));
        }
        let reimb_pay_fut = async {
            if !caps.reimb_pay {
                return Ok::<_, crate::error::AppError>((Value::Null, HashMap::new()));
            }
            let app_q = sqlx::query_as::<_, (i64, i64)>(
                "SELECT COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
                 FROM reimbursements
                 WHERE status = 'approved' AND created_at >= $1 AND created_at < $2",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_one(&st.db);
            let paid_q = sqlx::query_as::<_, (String, i64, i64)>(
                "SELECT substr(COALESCE(NULLIF(paid_at, ''), updated_at, created_at), 1, 7) AS m,
                        COUNT(*)::BIGINT,
                        COALESCE(SUM(amount_cents), 0)::BIGINT
                 FROM reimbursements
                 WHERE status = 'paid'
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2
                 GROUP BY m",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_all(&st.db);
            let batch_q = sqlx::query_as::<_, (i64, i64)>(
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
            .fetch_one(&st.db);

            let (
                (approved_unpaid_count, approved_unpaid_cents),
                paid_months,
                (open_batches, paid_batches),
            ) = tokio::try_join!(app_q, paid_q, batch_q)?;
            let mut paid_count = 0_i64;
            let mut paid_cents = 0_i64;
            let mut reimb_paid_by_month = HashMap::new();
            for (m, c, cents) in paid_months {
                paid_count += c;
                paid_cents += cents;
                reimb_paid_by_month.insert(m, cents);
            }
            Ok((
                json!({
                    "approved_unpaid_count": approved_unpaid_count,
                    "approved_unpaid_cents": approved_unpaid_cents,
                    "paid_count": paid_count,
                    "paid_cents": paid_cents,
                    "open_batches": open_batches,
                    "paid_batches": paid_batches,
                }),
                reimb_paid_by_month,
            ))
        };

        let bills_fut = async {
            if !caps.bills {
                return Ok::<_, crate::error::AppError>((Value::Null, HashMap::new()));
            }
            let sc_q = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, i64)>(
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
                    COALESCE(SUM(GREATEST(0, gross_cents - subsidy_cents)), 0)::BIGINT,
                    COALESCE(SUM(collected_cents), 0)::BIGINT,
                    COALESCE(SUM(GREATEST(0, GREATEST(0, gross_cents - subsidy_cents) - collected_cents)), 0)::BIGINT
                FROM sc",
            )
            .bind(&w.start)
            .bind(&w.end)
            .fetch_one(&st.db);
            let bm_q = sqlx::query_as::<_, (String, i64)>(
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
            .fetch_all(&st.db);

            let (
                (
                    published_count,
                    closed_count,
                    gross_cents,
                    subsidy_cents,
                    net_cents,
                    collected_cents,
                    outstanding_cents,
                ),
                bill_months,
            ) = tokio::try_join!(sc_q, bm_q)?;
            let bills_collected_by_month: HashMap<String, i64> = bill_months.into_iter().collect();
            let collection_rate_pct = if net_cents > 0 {
                (collected_cents * 100) / net_cents
            } else {
                0
            };
            Ok((
                json!({
                    "published_count": published_count,
                    "closed_count": closed_count,
                    "gross_cents": gross_cents,
                    "subsidy_cents": subsidy_cents,
                    "collected_cents": collected_cents,
                    "outstanding_cents": outstanding_cents,
                    "collection_rate_pct": collection_rate_pct,
                }),
                bills_collected_by_month,
            ))
        };

        let ((reimb_payouts, reimb_paid_by_month), (bills, bills_collected_by_month)) =
            tokio::try_join!(reimb_pay_fut, bills_fut)?;
        Ok((
            json!({
                "reimb_payouts": reimb_payouts,
                "bills": bills,
            }),
            reimb_paid_by_month,
            bills_collected_by_month,
        ))
    };

    let mem_fut = async {
        if !caps.membership {
            return Ok::<_, crate::error::AppError>(Value::Null);
        }
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
        Ok(json!({
            "active_members": active_members,
            "pending_members": pending_members,
            "new_members_in_period": new_members_in_period,
            "active_drivers_total": active_drivers_total,
        }))
    };

    let (
        (transport, rides_by_month),
        (events, events_by_month),
        reimb_review,
        (finance, reimb_paid_by_month, bills_collected_by_month),
        membership,
    ) = tokio::try_join!(tr_fut, ev_fut, rev_fut, fin_fut, mem_fut)?;

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

    let comparison = if let Some(pw) = prev_period_window(&w) {
        let tr_comp_fut = async {
            if !caps.demand {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let ride_filter = scope_rides_sql("r", 3);
            let sql = format!(
                "SELECT
                    COUNT(*) FILTER (WHERE r.status <> 'cancelled')::BIGINT,
                    COUNT(*) FILTER (WHERE r.status = 'completed')::BIGINT,
                    COUNT(*) FILTER (WHERE r.status IN ('confirmed', 'in_progress', 'completed'))::BIGINT
                 FROM rides r
                 WHERE r.pickup_at >= $1 AND r.pickup_at < $2{ride_filter}"
            );
            let mut q_tr = sqlx::query_as::<_, (i64, i64, i64)>(sqlx::AssertSqlSafe(&*sql))
                .bind(&pw.start)
                .bind(&pw.end);
            if !caps.demand_global {
                q_tr = q_tr.bind(u.id);
            }
            let (ptot, pcomp, pful) = q_tr.fetch_one(&st.db).await?;
            let prate = if ptot > 0 { (pful * 100) / ptot } else { 100 };
            Ok(json!({
                "total_rides_delta": transport["total_rides"].as_i64().unwrap_or(0) - ptot,
                "completed_rides_delta": transport["completed_rides"].as_i64().unwrap_or(0) - pcomp,
                "fulfillment_rate_delta": transport["fulfillment_rate_pct"].as_i64().unwrap_or(0) - prate,
            }))
        };

        let ev_comp_fut = async {
            if !caps.events {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let e_cond = scope_ev_sql(Some("e"), 3);
            let sql = format!(
                "WITH pe AS (
                    SELECT e.id, e.attendance_enabled
                    FROM events e
                    WHERE e.status NOT IN ('draft', 'cancelled')
                      AND e.starts_at >= $1 AND e.starts_at < $2{e_cond}
                 )
                 SELECT
                    (SELECT COUNT(*) FROM pe)::BIGINT,
                    COALESCE(COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT,
                    COALESCE(COUNT(*) FILTER (WHERE pe.attendance_enabled = 1 AND s.status = 'going'), 0)::BIGINT,
                    COALESCE(COUNT(*) FILTER (WHERE pe.attendance_enabled = 1 AND s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT
                 FROM pe
                 LEFT JOIN event_signups s ON s.event_id = pe.id"
            );
            let mut q_ev = sqlx::query_as::<_, (i64, i64, i64, i64)>(sqlx::AssertSqlSafe(&*sql))
                .bind(&pw.start)
                .bind(&pw.end);
            if !caps.events_global {
                q_ev = q_ev.bind(u.id);
            }
            let (pec, pta, ptg, ptr_att) = q_ev.fetch_one(&st.db).await?;
            let prate = if ptg > 0 { (ptr_att * 100) / ptg } else { 0 };
            Ok(json!({
                "events_count_delta": events["events_count"].as_i64().unwrap_or(0) - pec,
                "total_attended_delta": events["total_attended"].as_i64().unwrap_or(0) - pta,
                "attendance_rate_delta": events["attendance_rate_pct"].as_i64().unwrap_or(0) - prate,
            }))
        };

        let rev_comp_fut = async {
            if !caps.reimb_review {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let (ppend,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FILTER (WHERE status = 'pending')::BIGINT
                 FROM reimbursements WHERE created_at >= $1 AND created_at < $2",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            Ok(json!({
                "pending_count_delta": reimb_review["pending_count"].as_i64().unwrap_or(0) - ppend,
            }))
        };

        let pay_comp_fut = async {
            if !caps.reimb_pay {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let (ppaid_c,): (i64,) = sqlx::query_as(
                "SELECT COALESCE(SUM(amount_cents), 0)::BIGINT
                 FROM reimbursements
                 WHERE status = 'paid'
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            Ok(json!({
                "paid_cents_delta": finance["reimb_payouts"]["paid_cents"].as_i64().unwrap_or(0) - ppaid_c,
            }))
        };

        let bill_comp_fut = async {
            if !caps.bills {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let (pnet, pcol): (i64, i64) = sqlx::query_as(
                "WITH sc AS (
                    SELECT s.subsidy_cents,
                           COALESCE((SELECT SUM(si.amount_cents) FROM settlement_items si WHERE si.settlement_id = s.id), 0)::BIGINT AS gross_cents,
                           COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'confirmed'), 0)::BIGINT AS collected_cents
                    FROM settlements s
                    JOIN events e ON e.id = s.event_id
                    WHERE s.status <> 'draft' AND e.status <> 'draft'
                      AND e.starts_at >= $1 AND e.starts_at < $2
                 )
                 SELECT COALESCE(SUM(GREATEST(0, gross_cents - subsidy_cents)), 0)::BIGINT,
                        COALESCE(SUM(collected_cents), 0)::BIGINT
                 FROM sc",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            let prate = if pnet > 0 { (pcol * 100) / pnet } else { 0 };
            Ok(json!({
                "collected_cents_delta": finance["bills"]["collected_cents"].as_i64().unwrap_or(0) - pcol,
                "collection_rate_delta": finance["bills"]["collection_rate_pct"].as_i64().unwrap_or(0) - prate,
            }))
        };

        let mem_comp_fut = async {
            if !caps.membership {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let (pnew,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FILTER (WHERE created_at >= $1 AND created_at < $2)::BIGINT FROM users",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            Ok(json!({
                "new_members_delta": membership["new_members_in_period"].as_i64().unwrap_or(0) - pnew,
            }))
        };

        let (tr_comp, ev_comp, rev_comp, pay_comp, bill_comp, mem_comp) = tokio::try_join!(
            tr_comp_fut,
            ev_comp_fut,
            rev_comp_fut,
            pay_comp_fut,
            bill_comp_fut,
            mem_comp_fut,
        )?;

        json!({
            "prev_period": pw.period_label,
            "transport": tr_comp,
            "events": ev_comp,
            "reimb_review": rev_comp,
            "reimb_payouts": pay_comp,
            "bills": bill_comp,
            "membership": mem_comp,
        })
    } else {
        Value::Null
    };

    Ok(Json(json!({
        "period": w.period_label,
        "range": w.range,
        "from": w.start,
        "to": w.display_to,
        "comparison": comparison,
        "transport": transport,
        "events": events,
        "reimb_review": reimb_review,
        "finance": finance,
        "membership": membership,
        "monthly_series": monthly_series,
    })))
}
