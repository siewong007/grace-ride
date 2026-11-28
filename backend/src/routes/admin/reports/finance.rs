//! Admin financial & reimbursement reports, strictly separating transport review
//! (`admin_reimb_review`), finance payouts (`admin_reimb_pay`), and bills (`admin_bills`).

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
use crate::perms::{Act, Page};
use crate::routes::plan_visitors::SPECIAL_HIGH_CENTS;
use crate::state::AppState;
use crate::util::{audit, minutes_ago, now, require};

pub(in crate::routes::admin) async fn finance_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<PeriodQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReports, Act::View)?;
    let caps = ReportCaps::from_user(&u);
    if !caps.finance {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }

    let w = parse_period_window(
        u.lang,
        q.month.as_deref(),
        q.range.as_deref(),
        q.from.as_deref(),
        q.to.as_deref(),
    )?;

    let cutoff_7d = minutes_ago(7 * 24 * 60);
    let today = now()[..10].to_string();

    let rev_fut = async {
        if !caps.reimb_review {
            return Ok::<_, crate::error::AppError>(Value::Null);
        }
        let breakdown_q = sqlx::query_as::<_, (String, String, i64, i64, i64)>(
            "SELECT status, kind,
                    COUNT(*)::BIGINT,
                    COALESCE(SUM(amount_cents), 0)::BIGINT,
                    COUNT(*) FILTER (WHERE kind = 'visitor_special' AND amount_cents > $3)::BIGINT
             FROM reimbursements
             WHERE created_at >= $1 AND created_at < $2
             GROUP BY status, kind",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(SPECIAL_HIGH_CENTS)
        .fetch_all(&st.db);

        let overdue_q = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT
             FROM reimbursements
             WHERE status = 'pending' AND created_at < $1",
        )
        .bind(&cutoff_7d)
        .fetch_one(&st.db);

        let rides_q = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            "SELECT
                COUNT(*) FILTER (WHERE reimbursement_excluded = 0 AND reimbursement_amount_cents > 0)::BIGINT,
                COALESCE(SUM(reimbursement_amount_cents) FILTER (WHERE reimbursement_excluded = 0 AND reimbursement_amount_cents > 0), 0)::BIGINT,
                COUNT(*) FILTER (WHERE reimbursement_excluded = 1)::BIGINT,
                COUNT(*) FILTER (WHERE mileage_receipt_needed = 1)::BIGINT
             FROM rides
             WHERE status = 'completed' AND pickup_at >= $1 AND pickup_at < $2",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_one(&st.db);

        let (
            breakdown,
            overdue_pending_count,
            (eligible_count, eligible_cents, excluded_count, receipt_needed_count),
        ) = tokio::try_join!(breakdown_q, overdue_q, rides_q)?;

        let mut status_map: HashMap<String, (i64, i64)> = HashMap::new();
        let mut kind_map: HashMap<String, (i64, i64)> = HashMap::new();
        let mut high_amount_special_count = 0_i64;
        for (st_val, k_val, c, a, hi) in breakdown {
            let es = status_map.entry(st_val).or_insert((0, 0));
            es.0 += c;
            es.1 += a;
            let ek = kind_map.entry(k_val).or_insert((0, 0));
            ek.0 += c;
            ek.1 += a;
            high_amount_special_count += hi;
        }
        let status_obj = |k: &str| {
            let (c, a) = status_map.get(k).copied().unwrap_or((0, 0));
            json!({ "count": c, "amount_cents": a })
        };
        let by_kind: Vec<Value> = ["grab", "mileage", "visitor_special"]
            .into_iter()
            .map(|k| {
                let (c, a) = kind_map.get(k).copied().unwrap_or((0, 0));
                json!({ "kind": k, "count": c, "amount_cents": a })
            })
            .collect();

        Ok(json!({
            "by_status": {
                "pending": status_obj("pending"),
                "returned": status_obj("returned"),
                "rejected": status_obj("rejected"),
                "approved": status_obj("approved"),
                "paid": status_obj("paid"),
            },
            "overdue_pending_count": overdue_pending_count,
            "by_kind": by_kind,
            "high_amount_special_count": high_amount_special_count,
            "driver_mileage_rides": {
                "eligible_count": eligible_count,
                "eligible_cents": eligible_cents,
                "excluded_count": excluded_count,
                "receipt_needed_count": receipt_needed_count,
            },
        }))
    };

    let pay_fut = async {
        if !caps.reimb_pay {
            return Ok::<_, crate::error::AppError>(Value::Null);
        }
        #[derive(sqlx::FromRow)]
        struct BatchRow {
            id: i64,
            month: String,
            title: String,
            status: String,
            claim_count: i64,
            total_cents: i64,
            created_by_name: String,
            paid_by: Option<i64>,
            paid_by_name: Option<String>,
            paid_at: Option<String>,
            paid_ref: String,
            sod_violations: i64,
        }
        let summary_q = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            "SELECT
                COUNT(*) FILTER (WHERE status = 'approved')::BIGINT,
                COALESCE(SUM(amount_cents) FILTER (WHERE status = 'approved'), 0)::BIGINT,
                COUNT(*) FILTER (WHERE payee_verified = 1)::BIGINT,
                COUNT(*) FILTER (WHERE payee_verified = 0)::BIGINT
             FROM reimbursements
             WHERE status IN ('approved', 'paid')
               AND created_at >= $1 AND created_at < $2",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_one(&st.db);

        let awaiting_q = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            "SELECT
                COUNT(*)::BIGINT,
                COALESCE(SUM(amount_cents), 0)::BIGINT,
                COUNT(*) FILTER (WHERE updated_at < $1)::BIGINT,
                COALESCE(SUM(amount_cents) FILTER (WHERE updated_at < $1), 0)::BIGINT
             FROM reimbursements
             WHERE status = 'approved'",
        )
        .bind(&cutoff_7d)
        .fetch_one(&st.db);

        let app_q = sqlx::query_as::<_, (String, String, i64, i64)>(
            "SELECT kind, substr(created_at, 1, 7) AS m,
                    COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
             FROM reimbursements
             WHERE status = 'approved' AND created_at >= $1 AND created_at < $2
             GROUP BY kind, m",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db);

        let paid_q = sqlx::query_as::<_, (String, String, i64, i64)>(
            "SELECT kind,
                    substr(COALESCE(NULLIF(paid_at, ''), updated_at, created_at), 1, 7) AS m,
                    COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
             FROM reimbursements
             WHERE status = 'paid'
               AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
               AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2
             GROUP BY kind, m",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db);

        let bt_q = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT
             FROM payout_batches b
             WHERE (b.created_at >= $1 AND b.created_at < $2)
                OR (b.month >= $3 AND b.month <= $4)",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(&w.start[..7])
        .bind(&w.end_ym_incl)
        .fetch_one(&st.db);

        let br_q = sqlx::query_as::<_, BatchRow>(
            "SELECT b.id, b.month, b.title, b.status, b.claim_count, b.total_cents,
                    c.name AS created_by_name, b.paid_by, p.name AS paid_by_name,
                    b.paid_at, b.paid_ref,
                    ((CASE WHEN b.paid_by IS NOT NULL AND b.paid_by = b.created_by THEN 1 ELSE 0 END)
                     + COALESCE((SELECT COUNT(*) FROM reimbursements r
                                 WHERE r.batch_id = b.id
                                   AND (COALESCE(r.paid_by, b.paid_by) = r.user_id
                                        OR COALESCE(r.paid_by, b.paid_by) = r.reviewed_by
                                        OR COALESCE(r.paid_by, b.paid_by) = r.requested_by)), 0))::BIGINT AS sod_violations
             FROM payout_batches b
             JOIN users c ON c.id = b.created_by
             LEFT JOIN users p ON p.id = b.paid_by
             WHERE (b.created_at >= $1 AND b.created_at < $2)
                OR (b.month >= $3 AND b.month <= $4)
             ORDER BY b.id DESC
             LIMIT 50",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(&w.start[..7])
        .bind(&w.end_ym_incl)
        .fetch_all(&st.db);

        let (
            (
                approved_unpaid_count,
                approved_unpaid_cents,
                payee_verified_count,
                payee_unverified_count,
            ),
            (
                all_time_awaiting_count,
                all_time_awaiting_cents,
                overdue_approved_count,
                overdue_approved_cents,
            ),
            app_rows,
            paid_rows,
            batches_total,
            batch_rows,
        ) = tokio::try_join!(summary_q, awaiting_q, app_q, paid_q, bt_q, br_q)?;

        let mut app_map: HashMap<String, (i64, i64)> = HashMap::new();
        let mut trend_app_map: HashMap<String, (i64, i64)> = HashMap::new();
        for (k, m, c, a) in app_rows {
            let e_k = app_map.entry(k).or_insert((0, 0));
            e_k.0 += c;
            e_k.1 += a;
            let e_m = trend_app_map.entry(m).or_insert((0, 0));
            e_m.0 += c;
            e_m.1 += a;
        }
        let mut paid_count = 0i64;
        let mut paid_cents = 0i64;
        let mut paid_map: HashMap<String, (i64, i64)> = HashMap::new();
        let mut trend_paid_map: HashMap<String, (i64, i64)> = HashMap::new();
        for (k, m, c, a) in paid_rows {
            paid_count += c;
            paid_cents += a;
            let e_k = paid_map.entry(k).or_insert((0, 0));
            e_k.0 += c;
            e_k.1 += a;
            let e_m = trend_paid_map.entry(m).or_insert((0, 0));
            e_m.0 += c;
            e_m.1 += a;
        }
        let by_kind: Vec<Value> = ["grab", "mileage", "visitor_special"]
            .into_iter()
            .map(|k| {
                let (ac, aa) = app_map.get(k).copied().unwrap_or((0, 0));
                let (pc, pa) = paid_map.get(k).copied().unwrap_or((0, 0));
                json!({
                    "kind": k,
                    "approved_count": ac,
                    "approved_cents": aa,
                    "paid_count": pc,
                    "paid_cents": pa,
                })
            })
            .collect();

        let batches_truncated = batches_total > 50;
        let batches: Vec<Value> = batch_rows
            .into_iter()
            .map(|b| {
                let maker_checker_ok =
                    (b.status != "paid" || b.paid_by.is_some()) && b.sod_violations == 0;
                json!({
                    "id": b.id,
                    "month": b.month,
                    "title": b.title,
                    "status": b.status,
                    "claim_count": b.claim_count,
                    "total_cents": b.total_cents,
                    "created_by_name": b.created_by_name,
                    "paid_by_name": b.paid_by_name,
                    "paid_at": b.paid_at,
                    "paid_ref": b.paid_ref,
                    "maker_checker_ok": maker_checker_ok,
                })
            })
            .collect();

        let monthly_trend: Vec<Value> = month_buckets(&w)
            .into_iter()
            .map(|m| {
                let (ac, aa) = trend_app_map.get(&m).copied().unwrap_or((0, 0));
                let (pc, pa) = trend_paid_map.get(&m).copied().unwrap_or((0, 0));
                json!({
                    "month": m,
                    "approved_count": ac,
                    "approved_cents": aa,
                    "paid_count": pc,
                    "paid_cents": pa,
                    "claim_count": ac + pc,
                })
            })
            .collect();

        Ok(json!({
            "approved_unpaid_count": approved_unpaid_count,
            "approved_unpaid_cents": approved_unpaid_cents,
            "all_time_awaiting_count": all_time_awaiting_count,
            "all_time_awaiting_cents": all_time_awaiting_cents,
            "overdue_approved_count": overdue_approved_count,
            "overdue_approved_cents": overdue_approved_cents,
            "paid_count": paid_count,
            "paid_cents": paid_cents,
            "payee_verified_count": payee_verified_count,
            "payee_unverified_count": payee_unverified_count,
            "by_kind": by_kind,
            "batches_total": batches_total,
            "batches_truncated": batches_truncated,
            "batches": batches,
            "monthly_trend": monthly_trend,
        }))
    };

    let bills_fut = async {
        if !caps.bills {
            return Ok::<_, crate::error::AppError>(Value::Null);
        }
        #[derive(sqlx::FromRow)]
        struct BillRow {
            id: i64,
            title: String,
            event_id: i64,
            event_title: String,
            event_starts_at: String,
            organiser_name: String,
            status: String,
            subsidy_cents: i64,
            due_date: Option<String>,
            gross_cents: i64,
            collected_cents: i64,
            pending_verification_cents: i64,
            needs_info_cents: i64,
            unpaid_cents: i64,
            paid_cents: i64,
            overpaid_cents: i64,
            participant_count: i64,
            confirmed_count: i64,
        }
        let draft_q = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT
             FROM settlements s
             JOIN events e ON e.id = s.event_id
             WHERE s.status = 'draft' AND e.status NOT IN ('draft', 'cancelled')
               AND e.starts_at >= $1 AND e.starts_at < $2",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_one(&st.db);

        let br_q = sqlx::query_as::<_, BillRow>(
            "WITH scoped AS (
                SELECT s.id, s.title, s.event_id, e.title AS event_title, e.starts_at AS event_starts_at,
                       u.name AS organiser_name, s.status, s.subsidy_cents, s.due_date
                FROM settlements s
                JOIN events e ON e.id = s.event_id
                JOIN users u ON u.id = s.created_by
                WHERE s.status <> 'draft' AND e.status NOT IN ('draft', 'cancelled')
                  AND e.starts_at >= $1 AND e.starts_at < $2
             ),
             item_agg AS (
                SELECT si.settlement_id, COALESCE(SUM(si.amount_cents), 0)::BIGINT AS gross_cents
                FROM settlement_items si
                JOIN scoped sc ON sc.id = si.settlement_id
                GROUP BY si.settlement_id
             ),
             share_agg AS (
                SELECT sh.settlement_id,
                       COALESCE(SUM(sh.amount_cents) FILTER (WHERE sh.status = 'confirmed'), 0)::BIGINT AS collected_cents,
                       COALESCE(SUM(sh.amount_cents) FILTER (WHERE sh.status = 'pending'), 0)::BIGINT AS pending_verification_cents,
                       COALESCE(SUM(sh.amount_cents) FILTER (WHERE sh.status = 'needs_info'), 0)::BIGINT AS needs_info_cents,
                       COALESCE(SUM(sh.amount_cents) FILTER (WHERE sh.status = 'unpaid'), 0)::BIGINT AS unpaid_cents,
                       COALESCE(SUM(sh.paid_cents), 0)::BIGINT AS paid_cents,
                       COALESCE(SUM(GREATEST(0, sh.paid_cents - sh.amount_cents)), 0)::BIGINT AS overpaid_cents,
                       COUNT(*)::BIGINT AS participant_count,
                       COUNT(*) FILTER (WHERE sh.status = 'confirmed')::BIGINT AS confirmed_count
                FROM settlement_shares sh
                JOIN scoped sc ON sc.id = sh.settlement_id
                GROUP BY sh.settlement_id
             )
             SELECT sc.id, sc.title, sc.event_id, sc.event_title, sc.event_starts_at,
                    sc.organiser_name, sc.status, sc.subsidy_cents, sc.due_date,
                    COALESCE(ia.gross_cents, 0)::BIGINT AS gross_cents,
                    COALESCE(sa.collected_cents, 0)::BIGINT AS collected_cents,
                    COALESCE(sa.pending_verification_cents, 0)::BIGINT AS pending_verification_cents,
                    COALESCE(sa.needs_info_cents, 0)::BIGINT AS needs_info_cents,
                    COALESCE(sa.unpaid_cents, 0)::BIGINT AS unpaid_cents,
                    COALESCE(sa.paid_cents, 0)::BIGINT AS paid_cents,
                    COALESCE(sa.overpaid_cents, 0)::BIGINT AS overpaid_cents,
                    COALESCE(sa.participant_count, 0)::BIGINT AS participant_count,
                    COALESCE(sa.confirmed_count, 0)::BIGINT AS confirmed_count
             FROM scoped sc
             LEFT JOIN item_agg ia ON ia.settlement_id = sc.id
             LEFT JOIN share_agg sa ON sa.settlement_id = sc.id
             ORDER BY sc.event_starts_at DESC, sc.id DESC",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db);

        let (draft_count, bill_rows) = tokio::try_join!(draft_q, br_q)?;

        let mut published_count = 0_i64;
        let mut closed_count = 0_i64;
        let mut overdue_settlements_count = 0_i64;
        let mut gross_total = 0_i64;
        let mut subsidy_total = 0_i64;
        let mut net_billed_total = 0_i64;
        let mut collected_total = 0_i64;
        let mut pending_ver_total = 0_i64;
        let mut needs_info_total = 0_i64;
        let mut unpaid_total = 0_i64;
        let mut paid_total = 0_i64;
        let mut overpaid_total = 0_i64;

        for b in &bill_rows {
            if b.status == "closed" {
                closed_count += 1;
            } else {
                published_count += 1;
            }
            let net = (b.gross_cents - b.subsidy_cents).max(0);
            let outstanding = (net - b.collected_cents).max(0);
            if b.status == "published"
                && outstanding > 0
                && b.due_date
                    .as_deref()
                    .is_some_and(|d| !d.is_empty() && d < today.as_str())
            {
                overdue_settlements_count += 1;
            }
            gross_total += b.gross_cents;
            subsidy_total += b.subsidy_cents;
            net_billed_total += net;
            collected_total += b.collected_cents;
            pending_ver_total += b.pending_verification_cents;
            needs_info_total += b.needs_info_cents;
            unpaid_total += b.unpaid_cents;
            paid_total += b.paid_cents;
            overpaid_total += b.overpaid_cents;
        }

        let total_settlements = published_count + closed_count;
        let settlements_total = bill_rows.len() as i64;
        let settlements_truncated = settlements_total > 100;
        let collection_rate_pct = if net_billed_total > 0 {
            (collected_total * 100) / net_billed_total
        } else {
            0
        };

        let settlements: Vec<Value> = bill_rows
            .into_iter()
            .take(100)
            .map(|b| {
                let net = (b.gross_cents - b.subsidy_cents).max(0);
                let outstanding = (net - b.collected_cents).max(0);
                json!({
                    "id": b.id,
                    "title": b.title,
                    "event_id": b.event_id,
                    "event_title": b.event_title,
                    "event_starts_at": b.event_starts_at,
                    "organiser_name": b.organiser_name,
                    "status": b.status,
                    "due_date": b.due_date,
                    "gross_cents": b.gross_cents,
                    "subsidy_cents": b.subsidy_cents,
                    "net_billed_cents": net,
                    "collected_cents": b.collected_cents,
                    "outstanding_cents": outstanding,
                    "paid_cents": b.paid_cents,
                    "overpaid_cents": b.overpaid_cents,
                    "participant_count": b.participant_count,
                    "confirmed_count": b.confirmed_count,
                })
            })
            .collect();

        Ok(json!({
            "totals": {
                "total_settlements": total_settlements,
                "draft_count": draft_count,
                "published_count": published_count,
                "closed_count": closed_count,
                "overdue_settlements_count": overdue_settlements_count,
                "settlements_total": settlements_total,
                "settlements_truncated": settlements_truncated,
                "gross_cents": gross_total,
                "subsidy_cents": subsidy_total,
                "net_billed_cents": net_billed_total,
                "collected_cents": collected_total,
                "pending_verification_cents": pending_ver_total,
                "needs_info_cents": needs_info_total,
                "unpaid_cents": unpaid_total,
                "paid_cents": paid_total,
                "overpaid_cents": overpaid_total,
                "collection_rate_pct": collection_rate_pct,
            },
            "settlements": settlements,
        }))
    };

    let (reimbursement_review, reimbursement_payouts, bills) =
        tokio::try_join!(rev_fut, pay_fut, bills_fut)?;

    let comparison = if let Some(pw) = prev_period_window(&w) {
        let rev_comp_fut = async {
            if !caps.reimb_review {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let (prev_pend, prev_app_cents): (i64, i64) = sqlx::query_as(
                "SELECT
                    COUNT(*) FILTER (WHERE status = 'pending')::BIGINT,
                    COALESCE(SUM(amount_cents) FILTER (WHERE status = 'approved'), 0)::BIGINT
                 FROM reimbursements
                 WHERE created_at >= $1 AND created_at < $2",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            let cur_pend = reimbursement_review["by_status"]["pending"]["count"]
                .as_i64()
                .unwrap_or(0);
            let cur_app_cents = reimbursement_review["by_status"]["approved"]["amount_cents"]
                .as_i64()
                .unwrap_or(0);
            Ok(json!({
                "pending_count_delta": cur_pend - prev_pend,
                "approved_cents_delta": cur_app_cents - prev_app_cents,
            }))
        };

        let pay_comp_fut = async {
            if !caps.reimb_pay {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let (prev_paid_count, prev_paid_cents): (i64, i64) = sqlx::query_as(
                "SELECT COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
                 FROM reimbursements
                 WHERE status = 'paid'
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
                   AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2",
            )
            .bind(&pw.start)
            .bind(&pw.end)
            .fetch_one(&st.db)
            .await?;
            let cur_paid_count = reimbursement_payouts["paid_count"].as_i64().unwrap_or(0);
            let cur_paid_cents = reimbursement_payouts["paid_cents"].as_i64().unwrap_or(0);
            Ok(json!({
                "paid_count_delta": cur_paid_count - prev_paid_count,
                "paid_cents_delta": cur_paid_cents - prev_paid_cents,
            }))
        };

        let bill_comp_fut = async {
            if !caps.bills {
                return Ok::<_, crate::error::AppError>(Value::Null);
            }
            let (prev_net, prev_collected): (i64, i64) = sqlx::query_as(
                "WITH sc AS (
                    SELECT s.subsidy_cents,
                           COALESCE((SELECT SUM(si.amount_cents) FROM settlement_items si WHERE si.settlement_id = s.id), 0)::BIGINT AS gross_cents,
                           COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'confirmed'), 0)::BIGINT AS collected_cents
                    FROM settlements s
                    JOIN events e ON e.id = s.event_id
                    WHERE s.status <> 'draft' AND e.status NOT IN ('draft', 'cancelled')
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
            let prev_rate = if prev_net > 0 {
                (prev_collected * 100) / prev_net
            } else {
                0
            };
            let cur_collected = bills["totals"]["collected_cents"].as_i64().unwrap_or(0);
            let cur_rate = bills["totals"]["collection_rate_pct"].as_i64().unwrap_or(0);
            Ok(json!({
                "collected_cents_delta": cur_collected - prev_collected,
                "collection_rate_delta": cur_rate - prev_rate,
            }))
        };

        let (rev_comp, pay_comp, bill_comp) =
            tokio::try_join!(rev_comp_fut, pay_comp_fut, bill_comp_fut)?;

        json!({
            "prev_period": pw.period_label,
            "reimb_review": rev_comp,
            "reimb_payouts": pay_comp,
            "bills": bill_comp,
        })
    } else {
        Value::Null
    };

    audit(
        &st.db,
        Some(u.id),
        "report.finance_view",
        "finance_report",
        None,
        &format!(
            "{} [review={},pay={},bills={}]",
            w.period_label, caps.reimb_review, caps.reimb_pay, caps.bills
        ),
    )
    .await;

    Ok(Json(json!({
        "period": w.period_label,
        "range": w.range,
        "from": w.start,
        "to": w.display_to,
        "permissions": {
            "reimb_review": caps.reimb_review,
            "reimb_payouts": caps.reimb_pay,
            "bills": caps.bills,
        },
        "comparison": comparison,
        "reimbursement_review": reimbursement_review,
        "reimbursement_payouts": reimbursement_payouts,
        "bills": bills,
    })))
}
