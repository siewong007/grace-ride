//! Monthly finance reimbursement payout batches (create, CSV export, cancel, settle).

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, now, require};

use super::reimbursements::PayIn;
use super::reports::parse_report_month;

// ---------------- Monthly Finance Payout Batches ----------------

#[derive(sqlx::FromRow)]
struct PayoutBatchRow {
    id: i64,
    month: String,
    title: String,
    total_cents: i64,
    claim_count: i64,
    status: String,
    paid_ref: String,
    created_by: String,
    paid_by: Option<String>,
    paid_at: Option<String>,
    created_at: String,
}

pub(super) async fn list_payout_batches(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::View)?;
    let rows: Vec<PayoutBatchRow> = sqlx::query_as(
        "SELECT b.id, b.month, b.title, b.total_cents, b.claim_count, b.status, b.paid_ref,
                c.name AS created_by, p.name AS paid_by, b.paid_at, b.created_at
         FROM payout_batches b
         JOIN users c ON c.id = b.created_by
         LEFT JOIN users p ON p.id = b.paid_by
         ORDER BY b.id DESC LIMIT 50",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|b| json!({
            "id": b.id,
            "month": b.month,
            "title": b.title,
            "total_cents": b.total_cents,
            "claim_count": b.claim_count,
            "status": b.status,
            "paid_ref": b.paid_ref,
            "created_by": b.created_by,
            "paid_by": b.paid_by,
            "paid_at": b.paid_at,
            "created_at": b.created_at,
        }))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub(super) struct CreateBatchIn {
    month: Option<String>,
    title: Option<String>,
    claim_ids: Option<Vec<i64>>,
}

pub(super) async fn create_payout_batch(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CreateBatchIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    let month = req
        .month
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| now()[..7].to_string());
    if parse_report_month(&month).is_none() {
        return Err(bad(u.lang, ErrorKey::MonthFormatInvalid));
    }

    let mut tx = st.db.begin().await?;
    let eligible: Vec<(i64, i64)> = if let Some(ref ids) = req.claim_ids {
        let mut unique_ids = ids.clone();
        unique_ids.sort_unstable();
        unique_ids.dedup();
        if unique_ids.is_empty() {
            return Err(bad(u.lang, ErrorKey::BatchEmpty));
        }
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT id, amount_cents FROM reimbursements
             WHERE id = ANY($1) AND status = 'approved' AND batch_id IS NULL
               AND user_id <> $2 AND reviewed_by IS DISTINCT FROM $2
             FOR UPDATE",
        )
        .bind(&unique_ids)
        .bind(u.id)
        .fetch_all(&mut *tx)
        .await?;
        if rows.len() != unique_ids.len() {
            return Err(conflict(u.lang, ErrorKey::ReimbursementProcessed));
        }
        rows
    } else {
        sqlx::query_as(
            "SELECT id, amount_cents FROM reimbursements
             WHERE status = 'approved' AND batch_id IS NULL
               AND substr(created_at, 1, 7) <= $1
               AND user_id <> $2 AND reviewed_by IS DISTINCT FROM $2
             FOR UPDATE",
        )
        .bind(&month)
        .bind(u.id)
        .fetch_all(&mut *tx)
        .await?
    };

    if eligible.is_empty() {
        return Err(bad(u.lang, ErrorKey::BatchEmpty));
    }
    let claim_ids: Vec<i64> = eligible.iter().map(|(id, _)| *id).collect();
    let total_cents: i64 = eligible.iter().map(|(_, amt)| *amt).sum();
    let claim_count = claim_ids.len() as i64;
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Payout {month}"));

    let (batch_id,): (i64,) = sqlx::query_as(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, created_by, created_at)
         VALUES ($1,$2,$3,$4,'open',$5,$6) RETURNING id",
    )
    .bind(&month)
    .bind(&title)
    .bind(total_cents)
    .bind(claim_count)
    .bind(u.id)
    .bind(now())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE reimbursements SET batch_id=$1, updated_at=$2 WHERE id = ANY($3)")
        .bind(batch_id)
        .bind(now())
        .bind(&claim_ids)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "reimbursement.batch_create",
        "payout_batch",
        Some(batch_id),
        format!("{title}: {claim_count} claims, {}", fmt_rm(total_cents)),
    )
    .await;
    Ok(Json(json!({
        "id": batch_id,
        "month": month,
        "title": title,
        "total_cents": total_cents,
        "claim_count": claim_count,
    })))
}

fn csv_cell(v: &str) -> String {
    let needs_neutralize = v.starts_with(['=', '+', '-', '@', '\t', '\r'])
        || v.trim_start().starts_with(['=', '+', '-', '@']);
    let safe = if needs_neutralize {
        format!("'{v}")
    } else {
        v.to_string()
    };
    if safe.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}

pub(super) async fn export_payout_batch_csv(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    require(&u, Page::AdminReimbPay, Act::View)?;
    let exists: Option<(String,)> = sqlx::query_as("SELECT month FROM payout_batches WHERE id=$1")
        .bind(id)
        .fetch_optional(&st.db)
        .await?;
    let Some((month,)) = exists else {
        return Err(not_found(u.lang));
    };
    let rows: Vec<(i64, String, String, i64, String, Option<i64>, String, String)> = sqlx::query_as(
        "SELECT r.id, r.payee_name, r.payee_phone, r.amount_cents, u.name, r.ride_id, r.description, r.status
         FROM reimbursements r JOIN users u ON u.id = r.user_id
         WHERE r.batch_id = $1 ORDER BY r.id",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;

    let mut out = String::from(
        "Claim ID,Payee Name,DuitNow / Phone,Amount (MYR),Applicant,Ride ID,Description,Status\r\n",
    );
    for (cid, pname, pphone, cents, uname, ride_id, desc, status) in rows {
        let sign = if cents < 0 { "-" } else { "" };
        let abs_cents = cents.abs();
        let rm_str = format!("{sign}{}.{:02}", abs_cents / 100, abs_cents % 100);
        let eff_payee = if pname.is_empty() { &uname } else { &pname };
        let phone_cell = csv_cell(&pphone);
        let quoted_phone = if phone_cell.starts_with('"') {
            phone_cell
        } else {
            format!("\"{phone_cell}\"")
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\r\n",
            cid,
            csv_cell(eff_payee),
            quoted_phone,
            rm_str,
            csv_cell(&uname),
            ride_id.map(|x| x.to_string()).unwrap_or_default(),
            csv_cell(&desc),
            csv_cell(&status),
        ));
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"grace-ride-payout-batch-{id}-{month}.csv\""),
        )
        .body(Body::from(out))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

pub(super) async fn cancel_payout_batch(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    let mut tx = st.db.begin().await?;
    let batch: Option<(String,)> =
        sqlx::query_as("SELECT status FROM payout_batches WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((status,)) = batch else {
        return Err(not_found(u.lang));
    };
    if status != "open" {
        return Err(bad(u.lang, ErrorKey::BatchAlreadyPaid));
    }
    sqlx::query("UPDATE reimbursements SET batch_id=NULL, updated_at=$1 WHERE batch_id=$2")
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM payout_batches WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.batch_cancel",
        "payout_batch",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn pay_payout_batch(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<PayIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    if !req.payee_verified {
        return Err(bad(u.lang, ErrorKey::PayeeVerificationRequired));
    }
    let paid_ref = req.paid_ref.trim();
    if paid_ref.len() < 4 {
        return Err(bad(u.lang, ErrorKey::TxnRefRequired));
    }

    let mut tx = st.db.begin().await?;
    let batch: Option<(String,)> =
        sqlx::query_as("SELECT status FROM payout_batches WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((status,)) = batch else {
        return Err(not_found(u.lang));
    };
    if status != "open" {
        return Err(bad(u.lang, ErrorKey::BatchAlreadyPaid));
    }
    let claims: Vec<(i64, i64, Option<i64>, i64)> = sqlx::query_as(
        "SELECT id, user_id, reviewed_by, amount_cents FROM reimbursements
         WHERE batch_id=$1 AND status='approved' FOR UPDATE",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    if claims.is_empty() {
        return Err(bad(u.lang, ErrorKey::BatchEmpty));
    }
    for &(_cid, uid, reviewed_by, _amt) in &claims {
        if uid == u.id {
            return Err(forbidden(u.lang, ErrorKey::CannotPayOwnRequest));
        }
        if reviewed_by == Some(u.id) {
            return Err(forbidden(u.lang, ErrorKey::CannotPayOwnApproval));
        }
    }

    let ts_now = now();
    let claim_ids: Vec<i64> = claims.iter().map(|(cid, ..)| *cid).collect();
    sqlx::query(
        "UPDATE reimbursements SET status='paid', payee_verified=1, paid_ref=$1, paid_by=$2, paid_at=$3, updated_at=$3
         WHERE id = ANY($4)",
    )
    .bind(paid_ref)
    .bind(u.id)
    .bind(&ts_now)
    .bind(&claim_ids)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE payout_batches SET status='paid', paid_ref=$1, paid_by=$2, paid_at=$3 WHERE id=$4",
    )
    .bind(paid_ref)
    .bind(u.id)
    .bind(&ts_now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    for (cid, uid, _, amt) in claims {
        notify(
            &st.db,
            uid,
            ("报销款项已转账", "Reimbursement paid"),
            (
                &format!("{} · 交易编号 {}", fmt_rm(amt), paid_ref),
                &format!("{} · Transaction ref {}", fmt_rm(amt), paid_ref),
            ),
            &format!("/reimbursements/{cid}"),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.batch_pay",
        "payout_batch",
        Some(id),
        paid_ref,
    )
    .await;
    Ok(Json(json!({ "ok": true, "paid_count": claim_ids.len() })))
}
