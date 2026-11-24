//! Admin financial & reimbursement reports, strictly separating transport review
//! (`admin_reimb_review`), finance payouts (`admin_reimb_pay`), and bills (`admin_bills`).

use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::{json, Value};

use super::{parse_period_window, PeriodQuery, ReportCaps};
use crate::auth::CurrentUser;
use crate::error::{forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::routes::plan_visitors::SPECIAL_HIGH_CENTS;
use crate::state::AppState;
use crate::util::require;

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

    let reimbursement_review = if caps.reimb_review {
        let status_rows: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
             FROM reimbursements
             WHERE created_at >= $1 AND created_at < $2
             GROUP BY status",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db)
        .await?;

        let mut status_map: HashMap<String, (i64, i64)> = HashMap::new();
        for (s, c, a) in status_rows {
            status_map.insert(s, (c, a));
        }
        let status_obj = |k: &str| {
            let (c, a) = status_map.get(k).copied().unwrap_or((0, 0));
            json!({ "count": c, "amount_cents": a })
        };

        let kind_rows: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT kind, COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
             FROM reimbursements
             WHERE created_at >= $1 AND created_at < $2
             GROUP BY kind",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db)
        .await?;
        let mut kind_map: HashMap<String, (i64, i64)> = HashMap::new();
        for (k, c, a) in kind_rows {
            kind_map.insert(k, (c, a));
        }
        let by_kind: Vec<Value> = ["grab", "mileage", "visitor_special"]
            .into_iter()
            .map(|k| {
                let (c, a) = kind_map.get(k).copied().unwrap_or((0, 0));
                json!({ "kind": k, "count": c, "amount_cents": a })
            })
            .collect();

        let (high_amount_special_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT
             FROM reimbursements
             WHERE created_at >= $1 AND created_at < $2
               AND kind = 'visitor_special' AND amount_cents > $3",
        )
        .bind(&w.start)
        .bind(&w.end)
        .bind(SPECIAL_HIGH_CENTS)
        .fetch_one(&st.db)
        .await?;

        let (eligible_count, eligible_cents, excluded_count, receipt_needed_count): (
            i64,
            i64,
            i64,
            i64,
        ) = sqlx::query_as(
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
        .fetch_one(&st.db)
        .await?;

        json!({
            "by_status": {
                "pending": status_obj("pending"),
                "returned": status_obj("returned"),
                "rejected": status_obj("rejected"),
                "approved": status_obj("approved"),
                "paid": status_obj("paid"),
            },
            "by_kind": by_kind,
            "high_amount_special_count": high_amount_special_count,
            "driver_mileage_rides": {
                "eligible_count": eligible_count,
                "eligible_cents": eligible_cents,
                "excluded_count": excluded_count,
                "receipt_needed_count": receipt_needed_count,
            },
        })
    } else {
        Value::Null
    };

    let reimbursement_payouts = if caps.reimb_pay {
        let (
            approved_unpaid_count,
            approved_unpaid_cents,
            payee_verified_count,
            payee_unverified_count,
        ): (i64, i64, i64, i64) = sqlx::query_as(
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
        .fetch_one(&st.db)
        .await?;

        let (all_time_awaiting_count, all_time_awaiting_cents): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
             FROM reimbursements
             WHERE status = 'approved'",
        )
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

        let kind_app: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT kind, COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
             FROM reimbursements
             WHERE status = 'approved' AND created_at >= $1 AND created_at < $2
             GROUP BY kind",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db)
        .await?;
        let kind_paid: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT kind, COUNT(*)::BIGINT, COALESCE(SUM(amount_cents), 0)::BIGINT
             FROM reimbursements
             WHERE status = 'paid'
               AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
               AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2
             GROUP BY kind",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db)
        .await?;
        let mut app_map: HashMap<String, (i64, i64)> = HashMap::new();
        for (k, c, a) in kind_app {
            app_map.insert(k, (c, a));
        }
        let mut paid_map: HashMap<String, (i64, i64)> = HashMap::new();
        for (k, c, a) in kind_paid {
            paid_map.insert(k, (c, a));
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

        #[derive(sqlx::FromRow)]
        struct BatchRow {
            id: i64,
            month: String,
            title: String,
            status: String,
            claim_count: i64,
            total_cents: i64,
            created_by: i64,
            created_by_name: String,
            paid_by: Option<i64>,
            paid_by_name: Option<String>,
            paid_at: Option<String>,
            paid_ref: String,
        }
        let batch_rows: Vec<BatchRow> = sqlx::query_as(
            "SELECT b.id, b.month, b.title, b.status, b.claim_count, b.total_cents,
                    b.created_by, c.name AS created_by_name, b.paid_by, p.name AS paid_by_name,
                    b.paid_at, b.paid_ref
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
        .fetch_all(&st.db)
        .await?;

        let batches: Vec<Value> = batch_rows
            .into_iter()
            .map(|b| {
                let maker_checker_ok =
                    b.status != "paid" || (b.paid_by.is_some() && b.paid_by != Some(b.created_by));
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

        let trend_rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
            "WITH entries AS (
                SELECT substr(created_at, 1, 7) AS m,
                       CASE WHEN status = 'approved' THEN amount_cents ELSE 0 END AS app_c,
                       0::BIGINT AS paid_c,
                       1::BIGINT AS cnt
                FROM reimbursements
                WHERE status IN ('approved', 'paid') AND created_at >= $1 AND created_at < $2
                UNION ALL
                SELECT substr(COALESCE(NULLIF(paid_at, ''), updated_at, created_at), 1, 7) AS m,
                       0::BIGINT AS app_c,
                       amount_cents AS paid_c,
                       0::BIGINT AS cnt
                FROM reimbursements
                WHERE status = 'paid'
                  AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= $1
                  AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) < $2
             )
             SELECT m, COALESCE(SUM(app_c), 0)::BIGINT, COALESCE(SUM(paid_c), 0)::BIGINT, COALESCE(SUM(cnt), 0)::BIGINT
             FROM entries
             GROUP BY m
             ORDER BY m ASC",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db)
        .await?;

        let monthly_trend: Vec<Value> = trend_rows
            .into_iter()
            .map(|(m, app_c, paid_c, cnt)| {
                json!({
                    "month": m,
                    "approved_cents": app_c,
                    "paid_cents": paid_c,
                    "claim_count": cnt,
                })
            })
            .collect();

        json!({
            "approved_unpaid_count": approved_unpaid_count,
            "approved_unpaid_cents": approved_unpaid_cents,
            "all_time_awaiting_count": all_time_awaiting_count,
            "all_time_awaiting_cents": all_time_awaiting_cents,
            "paid_count": paid_count,
            "paid_cents": paid_cents,
            "payee_verified_count": payee_verified_count,
            "payee_unverified_count": payee_unverified_count,
            "by_kind": by_kind,
            "batches": batches,
            "monthly_trend": monthly_trend,
        })
    } else {
        Value::Null
    };

    let bills = if caps.bills {
        let (draft_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT
             FROM settlements s
             JOIN events e ON e.id = s.event_id
             WHERE s.status = 'draft' AND e.status <> 'draft'
               AND e.starts_at >= $1 AND e.starts_at < $2",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_one(&st.db)
        .await?;

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
            gross_cents: i64,
            collected_cents: i64,
            pending_verification_cents: i64,
            needs_info_cents: i64,
            unpaid_cents: i64,
            participant_count: i64,
            confirmed_count: i64,
        }

        let bill_rows: Vec<BillRow> = sqlx::query_as(
            "SELECT s.id, s.title, s.event_id, e.title AS event_title, e.starts_at AS event_starts_at,
                    u.name AS organiser_name, s.status, s.subsidy_cents,
                    COALESCE((SELECT SUM(si.amount_cents) FROM settlement_items si WHERE si.settlement_id = s.id), 0)::BIGINT AS gross_cents,
                    COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'confirmed'), 0)::BIGINT AS collected_cents,
                    COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'pending'), 0)::BIGINT AS pending_verification_cents,
                    COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'needs_info'), 0)::BIGINT AS needs_info_cents,
                    COALESCE((SELECT SUM(sh.amount_cents) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'unpaid'), 0)::BIGINT AS unpaid_cents,
                    COALESCE((SELECT COUNT(*) FROM settlement_shares sh WHERE sh.settlement_id = s.id), 0)::BIGINT AS participant_count,
                    COALESCE((SELECT COUNT(*) FROM settlement_shares sh WHERE sh.settlement_id = s.id AND sh.status = 'confirmed'), 0)::BIGINT AS confirmed_count
             FROM settlements s
             JOIN events e ON e.id = s.event_id
             JOIN users u ON u.id = s.created_by
             WHERE s.status <> 'draft' AND e.status <> 'draft'
               AND e.starts_at >= $1 AND e.starts_at < $2
             ORDER BY e.starts_at DESC, s.id DESC",
        )
        .bind(&w.start)
        .bind(&w.end)
        .fetch_all(&st.db)
        .await?;

        let mut published_count = 0_i64;
        let mut closed_count = 0_i64;
        let mut gross_total = 0_i64;
        let mut subsidy_total = 0_i64;
        let mut net_billed_total = 0_i64;
        let mut collected_total = 0_i64;
        let mut pending_ver_total = 0_i64;
        let mut needs_info_total = 0_i64;
        let mut unpaid_total = 0_i64;

        for b in &bill_rows {
            if b.status == "closed" {
                closed_count += 1;
            } else {
                published_count += 1;
            }
            let net = (b.gross_cents - b.subsidy_cents).max(0);
            gross_total += b.gross_cents;
            subsidy_total += b.subsidy_cents;
            net_billed_total += net;
            collected_total += b.collected_cents;
            pending_ver_total += b.pending_verification_cents;
            needs_info_total += b.needs_info_cents;
            unpaid_total += b.unpaid_cents;
        }

        let total_settlements = published_count + closed_count;
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
                    "gross_cents": b.gross_cents,
                    "subsidy_cents": b.subsidy_cents,
                    "net_billed_cents": net,
                    "collected_cents": b.collected_cents,
                    "outstanding_cents": outstanding,
                    "participant_count": b.participant_count,
                    "confirmed_count": b.confirmed_count,
                })
            })
            .collect();

        json!({
            "totals": {
                "total_settlements": total_settlements,
                "draft_count": draft_count,
                "published_count": published_count,
                "closed_count": closed_count,
                "gross_cents": gross_total,
                "subsidy_cents": subsidy_total,
                "net_billed_cents": net_billed_total,
                "collected_cents": collected_total,
                "pending_verification_cents": pending_ver_total,
                "needs_info_cents": needs_info_total,
                "unpaid_cents": unpaid_total,
                "collection_rate_pct": collection_rate_pct,
            },
            "settlements": settlements,
        })
    } else {
        Value::Null
    };

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
        "reimbursement_review": reimbursement_review,
        "reimbursement_payouts": reimbursement_payouts,
        "bills": bills,
    })))
}
