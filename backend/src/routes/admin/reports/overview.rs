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

    let mut rides_by_month: HashMap<String, i64> = HashMap::new();
    let transport = if caps.demand {
        let scope_creator: Option<i64> = if caps.demand_global { None } else { Some(u.id) };

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
        let rs: RideStatsRow = sqlx::query_as(
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
             WHERE r.pickup_at >= $1 AND r.pickup_at < $2
               AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .fetch_one(&st.db)
        .await?;

        let ride_months: Vec<(String, i64)> = sqlx::query_as(
            "SELECT substr(r.pickup_at, 1, 7) AS m, COUNT(*)::BIGINT
             FROM rides r
             WHERE r.status = 'completed'
               AND r.pickup_at >= $1 AND r.pickup_at < $2
               AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))
             GROUP BY m",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(scope_creator)
        .fetch_all(&st.db)
        .await?;
        for (m, c) in ride_months {
            rides_by_month.insert(m, c);
        }

        let fulfillment_rate_pct = if rs.total_rides > 0 {
            (rs.fulfilled_rides * 100) / rs.total_rides
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
        })
    } else {
        Value::Null
    };

    let mut events_by_month: HashMap<String, i64> = HashMap::new();
    let events = if caps.events {
        let scope_creator: Option<i64> = if caps.events_global { None } else { Some(u.id) };
        let (
            events_count,
            tracked_events_count,
            total_going,
            tracked_going,
            total_attended,
            tracked_attended,
        ): (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT
                COUNT(*)::BIGINT,
                COUNT(*) FILTER (WHERE e.attendance_enabled = 1)::BIGINT,
                COALESCE((SELECT COUNT(*) FROM event_signups s JOIN events e2 ON e2.id = s.event_id
                          WHERE e2.status NOT IN ('draft', 'cancelled')
                            AND e2.starts_at >= $1 AND e2.starts_at < $2
                            AND ($3::BIGINT IS NULL OR e2.created_by = $3)
                            AND s.status = 'going'), 0)::BIGINT,
                COALESCE((SELECT COUNT(*) FROM event_signups s JOIN events e2 ON e2.id = s.event_id
                          WHERE e2.status NOT IN ('draft', 'cancelled')
                            AND e2.attendance_enabled = 1
                            AND e2.starts_at >= $1 AND e2.starts_at < $2
                            AND ($3::BIGINT IS NULL OR e2.created_by = $3)
                            AND s.status = 'going'), 0)::BIGINT,
                COALESCE((SELECT COUNT(*) FROM event_signups s JOIN events e2 ON e2.id = s.event_id
                          WHERE e2.status NOT IN ('draft', 'cancelled')
                            AND e2.starts_at >= $1 AND e2.starts_at < $2
                            AND ($3::BIGINT IS NULL OR e2.created_by = $3)
                            AND s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT,
                COALESCE((SELECT COUNT(*) FROM event_signups s JOIN events e2 ON e2.id = s.event_id
                          WHERE e2.status NOT IN ('draft', 'cancelled')
                            AND e2.attendance_enabled = 1
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

        let attendance_rate_pct = if tracked_going > 0 {
            (tracked_attended * 100) / tracked_going
        } else {
            0
        };
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
            let (
                published_count,
                closed_count,
                gross_cents,
                subsidy_cents,
                net_cents,
                collected_cents,
                outstanding_cents,
            ): (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
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

            let collection_rate_pct = if net_cents > 0 {
                (collected_cents * 100) / net_cents
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

    let comparison = if let Some(pw) = prev_period_window(&w) {
        let tr_comp = if caps.demand {
            let scope_creator: Option<i64> = if caps.demand_global { None } else { Some(u.id) };
            let (ptot, pcomp, pful): (i64, i64, i64) = sqlx::query_as(
                "SELECT
                    COUNT(*) FILTER (WHERE r.status <> 'cancelled')::BIGINT,
                    COUNT(*) FILTER (WHERE r.status = 'completed')::BIGINT,
                    COUNT(*) FILTER (WHERE r.status IN ('confirmed', 'in_progress', 'completed'))::BIGINT
                 FROM rides r
                 WHERE r.pickup_at >= $1 AND r.pickup_at < $2
                   AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .bind(scope_creator)
            .fetch_one(&st.db)
            .await?;
            let prate = if ptot > 0 { (pful * 100) / ptot } else { 100 };
            json!({
                "total_rides_delta": transport["total_rides"].as_i64().unwrap_or(0) - ptot,
                "completed_rides_delta": transport["completed_rides"].as_i64().unwrap_or(0) - pcomp,
                "fulfillment_rate_delta": transport["fulfillment_rate_pct"].as_i64().unwrap_or(0) - prate,
            })
        } else {
            Value::Null
        };

        let ev_comp = if caps.events {
            let scope_creator: Option<i64> = if caps.events_global { None } else { Some(u.id) };
            let (pec, pta, ptg, ptr_att): (i64, i64, i64, i64) = sqlx::query_as(
                "WITH pe AS (
                    SELECT e.id, e.attendance_enabled
                    FROM events e
                    WHERE e.status NOT IN ('draft', 'cancelled')
                      AND e.starts_at >= $1 AND e.starts_at < $2
                      AND ($3::BIGINT IS NULL OR e.created_by = $3)
                 )
                 SELECT
                    (SELECT COUNT(*) FROM pe)::BIGINT,
                    COALESCE(COUNT(*) FILTER (WHERE s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT,
                    COALESCE(COUNT(*) FILTER (WHERE pe.attendance_enabled = 1 AND s.status = 'going'), 0)::BIGINT,
                    COALESCE(COUNT(*) FILTER (WHERE pe.attendance_enabled = 1 AND s.status <> 'cancelled' AND s.attended = 1), 0)::BIGINT
                 FROM pe
                 LEFT JOIN event_signups s ON s.event_id = pe.id",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .bind(scope_creator)
            .fetch_one(&st.db)
            .await?;
            let prate = if ptg > 0 { (ptr_att * 100) / ptg } else { 0 };
            json!({
                "events_count_delta": events["events_count"].as_i64().unwrap_or(0) - pec,
                "total_attended_delta": events["total_attended"].as_i64().unwrap_or(0) - pta,
                "attendance_rate_delta": events["attendance_rate_pct"].as_i64().unwrap_or(0) - prate,
            })
        } else {
            Value::Null
        };

        let rev_comp = if caps.reimb_review {
            let (ppend,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FILTER (WHERE status = 'pending')::BIGINT
                 FROM reimbursements WHERE created_at >= $1 AND created_at < $2",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            json!({
                "pending_count_delta": reimb_review["pending_count"].as_i64().unwrap_or(0) - ppend,
            })
        } else {
            Value::Null
        };

        let pay_comp = if caps.reimb_pay {
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
            json!({
                "paid_cents_delta": finance["reimb_payouts"]["paid_cents"].as_i64().unwrap_or(0) - ppaid_c,
            })
        } else {
            Value::Null
        };

        let bill_comp = if caps.bills {
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
            json!({
                "collected_cents_delta": finance["bills"]["collected_cents"].as_i64().unwrap_or(0) - pcol,
                "collection_rate_delta": finance["bills"]["collection_rate_pct"].as_i64().unwrap_or(0) - prate,
            })
        } else {
            Value::Null
        };

        let mem_comp = if caps.membership {
            let (pnew,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FILTER (WHERE created_at >= $1 AND created_at < $2)::BIGINT FROM users",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            json!({
                "new_members_delta": membership["new_members_in_period"].as_i64().unwrap_or(0) - pnew,
            })
        } else {
            Value::Null
        };

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
