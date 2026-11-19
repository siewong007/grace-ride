//! Grab alternative-transport reimbursement submission, review, and individual payout.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, notify_perm, now, require, require_one, Lang};

use super::like_pattern;

// ---------------- Grab reimbursements ----------------
// pending → (交通组) approved | returned | rejected → (财务) paid

#[derive(sqlx::FromRow)]
struct ReimbRow {
    id: i64,
    user_id: i64,
    user_name: String,
    ride_id: Option<i64>,
    ride_at: Option<String>,
    kind: String,
    ride_route: Option<String>,
    amount_cents: i64,
    description: String,
    receipt_file_id: i64,
    payee_name: String,
    payee_phone: String,
    status: String,
    review_note: String,
    reviewer: Option<String>,
    payee_verified: i64,
    paid_ref: String,
    paid_at: Option<String>,
    created_at: String,
}

const REIMB_SELECT: &str = "SELECT r.id, r.user_id, u.name AS user_name, r.ride_id, rd.pickup_at AS ride_at, r.kind,
    (SELECT pa.name || ' ↔ ' || COALESCE(ea.name, '?') FROM areas pa LEFT JOIN events ev ON ev.id = rd.event_id LEFT JOIN areas ea ON ea.id = ev.area_id WHERE pa.id = rd.pickup_area_id) AS ride_route,
    r.amount_cents, r.description,
    r.receipt_file_id, r.payee_name, r.payee_phone, r.status, r.review_note, rv.name AS reviewer, r.payee_verified, r.paid_ref, r.paid_at, r.created_at
    FROM reimbursements r JOIN users u ON u.id=r.user_id LEFT JOIN rides rd ON rd.id=r.ride_id LEFT JOIN users rv ON rv.id=r.reviewed_by";

fn reimb_json(r: &ReimbRow, u: &CurrentUser) -> Value {
    let finance = u.can(Page::AdminReimbPay, Act::View);
    let own = r.user_id == u.id;
    json!({
        "id": r.id, "user": {"id": r.user_id, "name": r.user_name}, "ride_id": r.ride_id, "ride_at": r.ride_at, "kind": r.kind, "ride_route": r.ride_route,
        "amount_cents": r.amount_cents, "description": r.description, "receipt_file_id": r.receipt_file_id,
        // Payee details only to the claimant and finance.
        "payee_name": if own || finance { Some(r.payee_name.clone()) } else { None },
        "payee_phone": if own || finance { Some(r.payee_phone.clone()) } else { None },
        "status": r.status, "review_note": r.review_note, "reviewer": r.reviewer, "payee_verified": r.payee_verified == 1,
        "paid_ref": if own || finance { Some(r.paid_ref.clone()) } else { None }, "paid_at": r.paid_at, "created_at": r.created_at,
    })
}

#[derive(Deserialize)]
pub(super) struct ReimbQuery {
    scope: Option<String>, // mine | review | payout
    q: Option<String>,
    status: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}

const REIMB_SORTS: &[(&str, &str)] = &[
    ("created_at", "r.created_at"),
    ("amount_cents", "r.amount_cents"),
    ("applicant", "u.name"),
    (
        "status",
        "CASE r.status WHEN 'pending' THEN 0 WHEN 'returned' THEN 1 WHEN 'approved' THEN 2 WHEN 'paid' THEN 3 ELSE 4 END",
    ),
    ("id", "r.id"),
];

pub(super) async fn list_reimbursements(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReimbQuery>,
) -> AppResult<Json<Value>> {
    let (permission, scope_filter, default_order, unpaginated_limit) = match q.scope.as_deref() {
        Some("review") => (
            Page::AdminReimbReview,
            "TRUE",
            "CASE r.status WHEN 'pending' THEN 0 ELSE 1 END, r.id DESC",
            200,
        ),
        Some("payout") => (
            Page::AdminReimbPay,
            "r.status IN ('approved','paid')",
            "CASE r.status WHEN 'approved' THEN 0 ELSE 1 END, r.id DESC",
            200,
        ),
        _ => (Page::Reimbursements, "r.user_id = $1", "r.id DESC", 100),
    };
    require(&u, permission, Act::View)?;

    let is_paginated = q.page.is_some() || q.limit.is_some();
    let page = q.page.unwrap_or(1).max(1);
    let limit = if is_paginated {
        q.limit.unwrap_or(25).clamp(1, 100)
    } else {
        unpaginated_limit
    };
    let offset = (page - 1).saturating_mul(limit);
    let order = crate::util::order_by(
        q.sort.as_deref(),
        q.dir.as_deref(),
        REIMB_SORTS,
        default_order,
        "r.id DESC",
    );
    let search =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(like_pattern);
    let status = q.status.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let finance = u.can(Page::AdminReimbPay, Act::View);
    // Payee-name matches follow the same per-claim visibility as reimb_json.
    let where_sql = format!(
        "WHERE {scope_filter}
         AND ($3::text IS NULL OR u.name ILIKE $3 OR r.description ILIKE $3
              OR (($2 OR r.user_id = $1) AND r.payee_name ILIKE $3))
         AND ($4::text IS NULL OR r.status = $4)"
    );
    let total = if is_paginated {
        let count_sql = format!(
            "SELECT COUNT(*) FROM reimbursements r JOIN users u ON u.id=r.user_id {where_sql}"
        );
        Some(
            sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(&*count_sql))
                .bind(u.id)
                .bind(finance)
                .bind(search.as_deref())
                .bind(status)
                .fetch_one(&st.db)
                .await?,
        )
    } else {
        None
    };

    let sql = format!("{REIMB_SELECT} {where_sql} ORDER BY {order} LIMIT $5 OFFSET $6");
    let rows = sqlx::query_as::<_, ReimbRow>(sqlx::AssertSqlSafe(&*sql))
        .bind(u.id)
        .bind(finance)
        .bind(search.as_deref())
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&st.db)
        .await?;
    let items = rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>();
    if let Some(total) = total {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i64;
        Ok(Json(json!({
            "items": items,
            "total": total,
            "page": page,
            "page_size": limit,
            "total_pages": total_pages.max(1),
        })))
    } else {
        Ok(Json(json!(items)))
    }
}

async fn load_reimb(st: &AppState, lang: Lang, id: i64) -> AppResult<ReimbRow> {
    sqlx::query_as::<_, ReimbRow>(crate::dyn_sql!("{REIMB_SELECT} WHERE r.id=$1"))
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(lang))
}

pub(super) async fn get_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_reimb(&st, u.lang, id).await?;
    if r.user_id != u.id {
        require_one(
            &u,
            &[
                (Page::AdminReimbReview, Act::View),
                (Page::AdminReimbPay, Act::View),
            ],
        )?;
    }
    Ok(Json(reimb_json(&r, &u)))
}

#[derive(Deserialize)]
pub(super) struct ReimbIn {
    ride_id: Option<i64>,
    amount_cents: i64,
    description: String,
    receipt_file_id: i64,
    payee_name: String,
    payee_phone: String,
    /// 'grab' (default) or 'mileage' (a driver's receipt for a ride with no fare yet).
    kind: Option<String>,
}

fn is_mileage(req: &ReimbIn) -> bool {
    req.kind.as_deref() == Some("mileage")
}

async fn validate_reimb(st: &AppState, u: &CurrentUser, req: &ReimbIn) -> AppResult<()> {
    if req.amount_cents <= 0 || req.amount_cents > 50_000 {
        return Err(bad(u.lang, ErrorKey::AmountRangeInvalid));
    }
    if req.payee_name.trim().is_empty() || req.payee_phone.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::PayeeNameAndPhoneRequired));
    }
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
        .bind(req.receipt_file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::ReceiptRequired))?;
    if owner != u.id {
        return Err(forbidden(u.lang, ErrorKey::InvalidReceipt));
    }
    if is_mileage(req) {
        // A driver's mileage receipt: for a ride they drove, completed with no fare for its route.
        let rid = req
            .ride_id
            .ok_or_else(|| bad(u.lang, ErrorKey::InvalidAction))?;
        let (driver, needed): (Option<i64>, i64) =
            sqlx::query_as("SELECT driver_id, mileage_receipt_needed FROM rides WHERE id=$1")
                .bind(rid)
                .fetch_optional(&st.db)
                .await?
                .ok_or(not_found(u.lang))?;
        if driver != Some(u.id) {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
        if needed != 1 {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    } else if let Some(rid) = req.ride_id {
        let (p,): (i64,) = sqlx::query_as("SELECT passenger_id FROM rides WHERE id=$1")
            .bind(rid)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
        if p != u.id {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
    }
    Ok(())
}

/// A receipt, or a ride, pays out once: refuse it while another request for it is still
/// live (anything but rejected). Runs in the caller's transaction after taking a lock,
/// so two requests sent together can't both pass.
async fn ensure_not_claimed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lang: Lang,
    req: &ReimbIn,
    except: Option<i64>,
) -> AppResult<()> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_reimbursement_claim', 0))",
    )
    .execute(&mut **tx)
    .await?;
    let (taken,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reimbursements
         WHERE status <> 'rejected' AND id IS DISTINCT FROM $1
           AND (receipt_file_id = $2 OR ($3::BIGINT IS NOT NULL AND ride_id = $3))",
    )
    .bind(except)
    .bind(req.receipt_file_id)
    .bind(req.ride_id)
    .fetch_one(&mut **tx)
    .await?;
    if taken > 0 {
        return Err(bad(lang, ErrorKey::ReimbursementAlreadyClaimed));
    }
    Ok(())
}

pub(super) async fn create_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Reimbursements, Act::Create)?;
    validate_reimb(&st, &u, &req).await?;
    let mut tx = st.db.begin().await?;
    ensure_not_claimed(&mut tx, u.lang, &req, None).await?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO reimbursements (user_id, ride_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, created_at, updated_at, kind) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
    )
    .bind(u.id)
    .bind(req.ride_id)
    .bind(req.amount_cents)
    .bind(req.description.trim())
    .bind(req.receipt_file_id)
    .bind(req.payee_name.trim())
    .bind(req.payee_phone.trim())
    .bind(now())
    .bind(now())
    .bind(if is_mileage(&req) { "mileage" } else { "grab" })
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    notify_perm(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        if is_mileage(&req) {
            ("新的里程报销收据", "New mileage receipt to confirm")
        } else {
            ("新的 Grab 报销申请", "New Grab reimbursement request")
        },
        (
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
        ),
        "/admin/reimbursements",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.create",
        "reimbursement",
        Some(id),
        fmt_rm(req.amount_cents),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

pub(super) async fn resubmit_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
    let r = load_reimb(&st, u.lang, id).await?;
    if r.user_id != u.id || r.status != "returned" {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    require(&u, Page::Reimbursements, Act::Update)?;
    validate_reimb(&st, &u, &req).await?;
    let mut tx = st.db.begin().await?;
    ensure_not_claimed(&mut tx, u.lang, &req, Some(id)).await?;
    // Still sent back and still theirs at the moment of writing: a review or another
    // resubmission in between wins, and this one is refused.
    let changed = sqlx::query("UPDATE reimbursements SET ride_id=$1, amount_cents=$2, description=$3, receipt_file_id=$4, payee_name=$5, payee_phone=$6, status='pending', payee_verified=0, updated_at=$7 WHERE id=$8 AND user_id=$9 AND status='returned'")
        .bind(req.ride_id)
        .bind(req.amount_cents)
        .bind(req.description.trim())
        .bind(req.receipt_file_id)
        .bind(req.payee_name.trim())
        .bind(req.payee_phone.trim())
        .bind(now())
        .bind(id)
        .bind(u.id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    tx.commit().await?;
    notify_perm(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        ("报销申请已补资料", "Reimbursement updated and resubmitted"),
        (
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
        ),
        "/admin/reimbursements",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.resubmit",
        "reimbursement",
        Some(id),
        fmt_rm(req.amount_cents),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct ReviewIn {
    action: String, // approve | return | reject
    note: Option<String>,
}

pub(super) async fn review_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReviewIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbReview, Act::Update)?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "pending" {
        return Err(bad(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
    if r.user_id == u.id {
        return Err(forbidden(u.lang, ErrorKey::CannotReviewOwnRequest));
    }
    let note = req.note.unwrap_or_default();
    let status = match req.action.as_str() {
        "approve" => "approved",
        "return" => "returned",
        "reject" => "rejected",
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    };
    if status != "approved" && note.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::ReasonRequired));
    }
    let changed = sqlx::query(
        "UPDATE reimbursements SET status=$1, review_note=$2, reviewed_by=$3, updated_at=$4 WHERE id=$5 AND status='pending'",
    )
    .bind(status)
    .bind(&note)
    .bind(u.id)
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
    // Approving a mileage receipt confirms the ride's reimbursement.
    if status == "approved" && r.kind == "mileage" {
        if let Some(rid) = r.ride_id {
            sqlx::query(
                "UPDATE rides SET reimbursement_amount_cents=$1, mileage_receipt_needed=0 WHERE id=$2",
            )
            .bind(r.amount_cents)
            .bind(rid)
            .execute(&st.db)
            .await?;
        }
    }
    let title = match status {
        "approved" => ("报销申请已批准", "Reimbursement approved"),
        "returned" => ("报销申请需补资料", "Reimbursement needs more info"),
        _ => ("报销申请未获批准", "Reimbursement not approved"),
    };
    notify(
        &st.db,
        r.user_id,
        title,
        (
            &format!("{} {}", fmt_rm(r.amount_cents), note),
            &format!("{} {}", fmt_rm(r.amount_cents), note),
        ),
        &format!("/reimbursements/{id}"),
    )
    .await;
    if status == "approved" {
        notify_perm(
            &st.db,
            Page::AdminReimbPay,
            Act::Update,
            ("有报销待付款", "Reimbursement awaiting payment"),
            (
                &format!("{} · {}", r.user_name, fmt_rm(r.amount_cents)),
                &format!("{} · {}", r.user_name, fmt_rm(r.amount_cents)),
            ),
            "/admin/reimbursements",
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        &format!("reimbursement.{}", req.action),
        "reimbursement",
        Some(id),
        note,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct PayIn {
    pub(super) payee_verified: bool,
    pub(super) paid_ref: String,
}

/// 财务核对收款人 → 转账并记录 (manual payout via church-approved bank / TNG channel).
pub(super) async fn pay_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<PayIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "approved" {
        return Err(bad(u.lang, ErrorKey::OnlyApprovedReimbCanBePaid));
    }
    if r.user_id == u.id {
        return Err(forbidden(u.lang, ErrorKey::CannotPayOwnRequest));
    }
    // Two people see every payout: whoever approved it doesn't also pay it.
    let (reviewed_by,): (Option<i64>,) =
        sqlx::query_as("SELECT reviewed_by FROM reimbursements WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await?;
    if reviewed_by == Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::CannotPayOwnApproval));
    }
    if !req.payee_verified {
        return Err(bad(u.lang, ErrorKey::PayeeVerificationRequired));
    }
    if req.paid_ref.trim().len() < 4 {
        return Err(bad(u.lang, ErrorKey::TxnRefRequired));
    }
    // The status check above and this update are separate statements, so a second
    // finance admin can pass the check before the first commit. Transitioning from
    // 'approved' in the update itself makes exactly one payout win.
    let paid = sqlx::query(
        "UPDATE reimbursements
         SET status='paid', payee_verified=1, paid_ref=$1, paid_by=$2, paid_at=$3, updated_at=$4
         WHERE id=$5 AND status='approved' AND batch_id IS NULL
           AND user_id <> $2 AND reviewed_by IS DISTINCT FROM $2",
    )
    .bind(req.paid_ref.trim())
    .bind(u.id)
    .bind(now())
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?;
    if paid.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbursementProcessed));
    }
    notify(
        &st.db,
        r.user_id,
        ("报销款项已转账", "Reimbursement paid"),
        (
            &format!(
                "{} · 交易编号 {}",
                fmt_rm(r.amount_cents),
                req.paid_ref.trim()
            ),
            &format!(
                "{} · Transaction ref {}",
                fmt_rm(r.amount_cents),
                req.paid_ref.trim()
            ),
        ),
        &format!("/reimbursements/{id}"),
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.pay",
        "reimbursement",
        Some(id),
        req.paid_ref.trim(),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}
