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
use crate::notifications::notify_perm_except;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, now, require, require_one, Lang};

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
    event_id: Option<i64>,
    event_title: Option<String>,
    event_starts_at: Option<String>,
    amount_cents: i64,
    description: String,
    receipt_file_id: Option<i64>,
    payee_name: String,
    payee_phone: String,
    status: String,
    review_note: String,
    reviewer: Option<String>,
    payee_verified: i64,
    paid_ref: String,
    paid_at: Option<String>,
    created_at: String,
    updated_at: String,
    plan_visitor_id: Option<i64>,
    visitor_name: Option<String>,
    special_reason: String,
    requested_by: Option<i64>,
    requester_name: Option<String>,
}

const REIMB_SELECT: &str = "SELECT r.id, r.user_id, u.name AS user_name, r.ride_id, rd.pickup_at AS ride_at, r.kind,
    CASE WHEN pa.id IS NOT NULL THEN pa.name || ' ↔ ' || COALESCE(ea.name, '?') END AS ride_route,
    COALESCE(r.event_id, rd.event_id) AS event_id, ev.title AS event_title, ev.starts_at AS event_starts_at,
    r.amount_cents, r.description,
    r.receipt_file_id, r.payee_name, r.payee_phone, r.status, r.review_note, rv.name AS reviewer, r.payee_verified, r.paid_ref, r.paid_at, r.created_at, r.updated_at,
    r.plan_visitor_id, pv.name AS visitor_name, r.special_reason, r.requested_by, rq.name AS requester_name
    FROM reimbursements r
    JOIN users u ON u.id=r.user_id
    LEFT JOIN rides rd ON rd.id=r.ride_id
    LEFT JOIN areas pa ON pa.id=rd.pickup_area_id
    LEFT JOIN events ev ON ev.id = COALESCE(r.event_id, rd.event_id)
    LEFT JOIN areas ea ON ea.id = ev.area_id
    LEFT JOIN users rv ON rv.id=r.reviewed_by
    LEFT JOIN plan_visitors pv ON pv.id=r.plan_visitor_id
    LEFT JOIN users rq ON rq.id=r.requested_by";

fn reimb_json(r: &ReimbRow, u: &CurrentUser) -> Value {
    let finance = u.can(Page::AdminReimbPay, Act::View);
    let own = r.user_id == u.id;
    json!({
        "id": r.id, "user": {"id": r.user_id, "name": r.user_name}, "ride_id": r.ride_id, "ride_at": r.ride_at, "kind": r.kind, "ride_route": r.ride_route,
        "event_id": r.event_id, "event_title": r.event_title, "event_starts_at": r.event_starts_at,
        "amount_cents": r.amount_cents, "description": r.description, "receipt_file_id": r.receipt_file_id,
        // Payee details only to the claimant and finance.
        "payee_name": if own || finance { Some(r.payee_name.clone()) } else { None },
        "payee_phone": if own || finance { Some(r.payee_phone.clone()) } else { None },
        "status": r.status, "review_note": r.review_note, "reviewer": r.reviewer, "payee_verified": r.payee_verified == 1,
        "paid_ref": if own || finance { Some(r.paid_ref.clone()) } else { None }, "paid_at": r.paid_at, "created_at": r.created_at, "updated_at": r.updated_at,
        // A traffic lead's special reimbursement for a visitor: who, why and who asked.
        "visitor": r.plan_visitor_id.map(|id| json!({"id": id, "name": r.visitor_name})),
        "special_reason": if r.kind == "visitor_special" { Some(r.special_reason.clone()) } else { None },
        // Over RM 100: reviewers see a warning (no ceiling, never a block).
        "high_amount": r.kind == "visitor_special"
            && r.amount_cents > crate::routes::plan_visitors::SPECIAL_HIGH_CENTS,
        "requested_by": r.requested_by.map(|id| json!({"id": id, "name": r.requester_name})),
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
    enum ReimbScope {
        Mine,
        Review,
        Payout,
    }
    let (permission, scope_kind, default_order, unpaginated_limit) = match q.scope.as_deref() {
        Some("review") => (
            Page::AdminReimbReview,
            ReimbScope::Review,
            "CASE r.status WHEN 'pending' THEN 0 ELSE 1 END, r.id DESC",
            200,
        ),
        Some("payout") => (
            Page::AdminReimbPay,
            ReimbScope::Payout,
            "CASE r.status WHEN 'approved' THEN 0 ELSE 1 END, r.id DESC",
            200,
        ),
        _ => (Page::Reimbursements, ReimbScope::Mine, "r.id DESC", 100),
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

    enum BindVal<'a> {
        I64(i64),
        Str(&'a str),
    }
    let mut conds: Vec<String> = Vec::new();
    let mut params: Vec<BindVal<'_>> = Vec::new();

    match scope_kind {
        ReimbScope::Review => {}
        ReimbScope::Payout => conds.push("r.status IN ('approved','paid')".to_string()),
        ReimbScope::Mine => {
            params.push(BindVal::I64(u.id));
            conds.push(format!("r.user_id = ${}", params.len()));
        }
    }
    if let Some(st_val) = status {
        params.push(BindVal::Str(st_val));
        conds.push(format!("r.status = ${}", params.len()));
    }
    if let Some(s) = search.as_deref() {
        params.push(BindVal::Str(s));
        let p_idx = params.len();
        if finance || matches!(scope_kind, ReimbScope::Mine) {
            conds.push(format!(
                "(u.name ILIKE ${p_idx} ESCAPE '\\' OR r.description ILIKE ${p_idx} ESCAPE '\\' OR r.payee_name ILIKE ${p_idx} ESCAPE '\\')"
            ));
        } else {
            params.push(BindVal::I64(u.id));
            let u_idx = params.len();
            conds.push(format!(
                "(u.name ILIKE ${p_idx} ESCAPE '\\' OR r.description ILIKE ${p_idx} ESCAPE '\\' OR (r.user_id = ${u_idx} AND r.payee_name ILIKE ${p_idx} ESCAPE '\\'))"
            ));
        }
    }
    let where_sql = if conds.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conds.join(" AND "))
    };
    let need_u_filter = search.is_some();
    let need_u_sort = q.sort.as_deref() == Some("applicant");
    let count_join = if need_u_filter {
        " JOIN users u ON u.id=r.user_id"
    } else {
        ""
    };
    let page_join = if need_u_filter || need_u_sort {
        " JOIN users u ON u.id=r.user_id"
    } else {
        ""
    };

    let total = if is_paginated {
        let count_sql = format!("SELECT COUNT(*) FROM reimbursements r{count_join} {where_sql}");
        let mut cq = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(&*count_sql));
        for p in &params {
            cq = match p {
                BindVal::I64(v) => cq.bind(*v),
                BindVal::Str(v) => cq.bind(*v),
            };
        }
        Some(cq.fetch_one(&st.db).await?)
    } else {
        None
    };

    let lim_p = params.len() + 1;
    let off_p = params.len() + 2;
    let sql = format!(
        "WITH page_ids AS MATERIALIZED (
             SELECT r.id, ROW_NUMBER() OVER () AS _ord
             FROM (
                 SELECT r.id FROM reimbursements r{page_join} {where_sql} ORDER BY {order} LIMIT ${lim_p} OFFSET ${off_p}
             ) r
         )
         {REIMB_SELECT}
         JOIN page_ids pid ON pid.id = r.id
         WHERE r.id IN (SELECT id FROM page_ids)
         ORDER BY pid._ord"
    );
    let mut query = sqlx::query_as::<_, ReimbRow>(sqlx::AssertSqlSafe(&*sql));
    for p in &params {
        query = match p {
            BindVal::I64(v) => query.bind(*v),
            BindVal::Str(v) => query.bind(*v),
        };
    }
    let rows = query.bind(limit).bind(offset).fetch_all(&st.db).await?;
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
    if r.user_id == u.id {
        require(&u, Page::Reimbursements, Act::View)?;
    } else if matches!(r.status.as_str(), "approved" | "paid") {
        require_one(
            &u,
            &[
                (Page::AdminReimbReview, Act::View),
                (Page::AdminReimbPay, Act::View),
            ],
        )?;
    } else {
        require(&u, Page::AdminReimbReview, Act::View)?;
    }
    Ok(Json(reimb_json(&r, &u)))
}

#[derive(Deserialize)]
pub(super) struct ReimbIn {
    ride_id: Option<i64>,
    event_id: Option<i64>,
    amount_cents: i64,
    description: String,
    receipt_file_id: Option<i64>,
    payee_name: String,
    payee_phone: String,
    /// 'grab' (default) or 'mileage' (a driver's receipt for a ride with no fare yet).
    kind: Option<String>,
}

fn is_mileage(req: &ReimbIn) -> bool {
    req.kind.as_deref() == Some("mileage")
}

async fn validate_reimb(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    u: &CurrentUser,
    req: &ReimbIn,
) -> AppResult<Option<i64>> {
    if let Some(ref k) = req.kind {
        if k != "grab" && k != "mileage" {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if req.amount_cents <= 0 || req.amount_cents > 50_000 {
        return Err(bad(u.lang, ErrorKey::AmountRangeInvalid));
    }
    if req.payee_name.trim().is_empty() || req.payee_phone.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::PayeeNameAndPhoneRequired));
    }
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1 FOR KEY SHARE")
        .bind(req.receipt_file_id)
        .fetch_optional(&mut **tx)
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
        let (driver, needed, ride_event_id, excluded, status): (
            Option<i64>,
            i64,
            Option<i64>,
            i64,
            String,
        ) = sqlx::query_as(
            "SELECT driver_id, mileage_receipt_needed, event_id, reimbursement_excluded, status
             FROM rides WHERE id=$1 FOR SHARE",
        )
        .bind(rid)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(not_found(u.lang))?;
        if driver != Some(u.id) {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
        if status != "completed" || needed != 1 || excluded == 1 {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        let Some(eid) = ride_event_id else {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        };
        let ev_row: Option<(String, i64)> =
            sqlx::query_as("SELECT status, mileage_claimable FROM events WHERE id=$1 FOR SHARE")
                .bind(eid)
                .fetch_optional(&mut **tx)
                .await?;
        let Some((ev_status, claimable)) = ev_row else {
            return Err(not_found(u.lang));
        };
        if !matches!(ev_status.as_str(), "published" | "finished") || claimable != 1 {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        Ok(Some(eid))
    } else {
        let ride_info: Option<(i64, Option<i64>)> = if let Some(rid) = req.ride_id {
            let row: Option<(i64, Option<i64>, String)> = sqlx::query_as(
                "SELECT passenger_id, event_id, status FROM rides WHERE id=$1 FOR SHARE",
            )
            .bind(rid)
            .fetch_optional(&mut **tx)
            .await?;
            let (p, eid, r_status) = row.ok_or(not_found(u.lang))?;
            if p != u.id {
                return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
            }
            if matches!(r_status.as_str(), "completed" | "no_show" | "in_progress") {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
            Some((p, eid))
        } else {
            None
        };

        let ride_event_id = ride_info.and_then(|(_, eid)| eid);
        let target_event_id = match (req.event_id, ride_event_id) {
            (Some(eid), Some(reid)) => {
                if eid != reid {
                    return Err(bad(u.lang, ErrorKey::InvalidAction));
                }
                eid
            }
            (Some(eid), None) => eid,
            (None, Some(reid)) => reid,
            (None, None) => return Err(bad(u.lang, ErrorKey::EventRequiredForReimbursement)),
        };

        let event_row: Option<(String,)> =
            sqlx::query_as("SELECT status FROM events WHERE id=$1 FOR SHARE")
                .bind(target_event_id)
                .fetch_optional(&mut **tx)
                .await?;
        let Some((status,)) = event_row else {
            return Err(not_found(u.lang));
        };
        if status == "cancelled" || status == "draft" {
            return Err(bad(u.lang, ErrorKey::EventNotOpen));
        }

        let signup: Option<(String,)> = sqlx::query_as(
            "SELECT status FROM event_signups WHERE event_id=$1 AND user_id=$2 FOR SHARE",
        )
        .bind(target_event_id)
        .bind(u.id)
        .fetch_optional(&mut **tx)
        .await?;

        if signup.as_ref().map(|(s,)| s.as_str()) != Some("going") {
            return Err(bad(u.lang, ErrorKey::EventReimbursementNeedsSignup));
        }

        Ok(Some(target_event_id))
    }
}

/// A receipt, or a ride, pays out once: refuse it while another request for it is still
/// live (anything but rejected). Runs in the caller's transaction after taking a lock,
/// so two requests sent together can't both pass.
async fn ensure_not_claimed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lang: Lang,
    user_id: i64,
    resolved_event_id: Option<i64>,
    req: &ReimbIn,
    except: Option<i64>,
) -> AppResult<()> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_reimbursement_claim', 0))",
    )
    .execute(&mut **tx)
    .await?;
    let claim_kind = if is_mileage(req) { "mileage" } else { "grab" };
    let (taken,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reimbursements
         WHERE status <> 'rejected' AND id IS DISTINCT FROM $1
           AND (receipt_file_id = $2 OR ($3::BIGINT IS NOT NULL AND ride_id = $3 AND kind = $4))",
    )
    .bind(except)
    .bind(req.receipt_file_id)
    .bind(req.ride_id)
    .bind(claim_kind)
    .fetch_one(&mut **tx)
    .await?;
    if taken > 0 {
        return Err(bad(lang, ErrorKey::ReimbursementAlreadyClaimed));
    }
    if is_mileage(req) {
        if let Some(rid) = req.ride_id {
            let trip_id: Option<Option<i64>> =
                sqlx::query_scalar("SELECT trip_id FROM rides WHERE id = $1 FOR SHARE")
                    .bind(rid)
                    .fetch_optional(&mut **tx)
                    .await?;
            if let Some(Some(tid)) = trip_id {
                let sibling_taken: bool = sqlx::query_scalar(
                    "SELECT EXISTS (
                         SELECT 1 FROM rides s
                         WHERE s.trip_id = $1 AND s.driver_id = $2 AND s.id <> $3
                           AND (
                               s.reimbursement_amount_cents > 0
                               OR EXISTS (
                                   SELECT 1 FROM reimbursements r
                                   WHERE r.ride_id = s.id AND r.kind = 'mileage'
                                     AND r.status <> 'rejected' AND r.id IS DISTINCT FROM $4
                               )
                           )
                     )",
                )
                .bind(tid)
                .bind(user_id)
                .bind(rid)
                .bind(except)
                .fetch_one(&mut **tx)
                .await?;
                if sibling_taken {
                    return Err(bad(lang, ErrorKey::ReimbursementAlreadyClaimed));
                }
            }
        }
        if let Some(eid) = resolved_event_id {
            let (post_event_taken,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM reimbursements
                 WHERE status <> 'rejected' AND id IS DISTINCT FROM $1
                   AND event_id = $2 AND user_id = $3
                   AND kind = 'mileage' AND ride_id IS NULL",
            )
            .bind(except)
            .bind(eid)
            .bind(user_id)
            .fetch_one(&mut **tx)
            .await?;
            if post_event_taken > 0 {
                return Err(bad(lang, ErrorKey::ReimbursementAlreadyClaimed));
            }
        }
    }
    Ok(())
}

pub(super) async fn create_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Reimbursements, Act::Create)?;
    let mut tx = st.db.begin().await?;
    let resolved_event_id = validate_reimb(&mut tx, &u, &req).await?;
    ensure_not_claimed(&mut tx, u.lang, u.id, resolved_event_id, &req, None).await?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO reimbursements (user_id, ride_id, event_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, created_at, updated_at, kind) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
    )
    .bind(u.id)
    .bind(req.ride_id)
    .bind(resolved_event_id)
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
    notify_perm_except(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        Some(u.id),
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
    let mut tx = st.db.begin().await?;
    let locked: Option<(i64, String, String)> =
        sqlx::query_as("SELECT user_id, status, kind FROM reimbursements WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let (r_user_id, r_status, r_kind) = locked.ok_or_else(|| not_found(u.lang))?;
    // A visitor special reimbursement is corrected by the traffic lead, not the driver.
    if r_user_id != u.id || r_status != "returned" || r_kind == "visitor_special" {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    require(&u, Page::Reimbursements, Act::Update)?;
    if let Some(ref k) = req.kind {
        if k != &r_kind {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    let mut normalized = req;
    normalized.kind = Some(r_kind);
    let resolved_event_id = validate_reimb(&mut tx, &u, &normalized).await?;
    ensure_not_claimed(
        &mut tx,
        u.lang,
        u.id,
        resolved_event_id,
        &normalized,
        Some(id),
    )
    .await?;
    // Still sent back and still theirs at the moment of writing: a review or another
    // resubmission in between wins, and this one is refused.
    let changed = sqlx::query("UPDATE reimbursements SET ride_id=$1, event_id=$2, amount_cents=$3, description=$4, receipt_file_id=$5, payee_name=$6, payee_phone=$7, status='pending', payee_verified=0, updated_at=$8 WHERE id=$9 AND user_id=$10 AND status='returned' AND kind <> 'visitor_special'")
        .bind(normalized.ride_id)
        .bind(resolved_event_id)
        .bind(normalized.amount_cents)
        .bind(normalized.description.trim())
        .bind(normalized.receipt_file_id)
        .bind(normalized.payee_name.trim())
        .bind(normalized.payee_phone.trim())
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
    notify_perm_except(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        Some(u.id),
        ("报销申请已补资料", "Reimbursement updated and resubmitted"),
        (
            &format!("{} · {}", u.name, fmt_rm(normalized.amount_cents)),
            &format!("{} · {}", u.name, fmt_rm(normalized.amount_cents)),
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
        fmt_rm(normalized.amount_cents),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct ReviewIn {
    action: String, // approve | return | reject
    note: Option<String>,
    #[serde(default)]
    expected_updated_at: Option<String>,
}

pub(super) async fn review_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReviewIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbReview, Act::Update)?;
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
    // Preserve HTTP 400 semantics when reviewing an already-processed claim sequentially.
    let pre_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM reimbursements WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    if let Some(ref s) = pre_status {
        if s != "pending" {
            return Err(bad(u.lang, ErrorKey::ReimbAlreadyReviewed));
        }
    }
    let mut tx = st.db.begin().await?;
    let locked: Option<(
        i64,
        Option<i64>,
        String,
        String,
        Option<i64>,
        Option<i64>,
        i64,
        String,
        String,
    )> = sqlx::query_as(
        "SELECT r.user_id, r.requested_by, r.status, r.kind, r.ride_id, r.event_id, r.amount_cents,
                    r.updated_at, u.name
             FROM reimbursements r
             JOIN users u ON u.id = r.user_id
             WHERE r.id = $1
             FOR UPDATE OF r",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let (
        r_user_id,
        r_requested_by,
        r_status,
        r_kind,
        r_ride_id,
        r_event_id,
        r_amount_cents,
        r_updated_at,
        r_user_name,
    ) = locked.ok_or_else(|| not_found(u.lang))?;
    if r_status != "pending" {
        return Err(conflict(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
    if let Some(ref exp) = req.expected_updated_at {
        if exp != &r_updated_at {
            return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
        }
    }
    // Neither the payee nor whoever asked for it (a visitor special reimbursement) reviews it.
    if r_user_id == u.id || r_requested_by == Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::CannotReviewOwnRequest));
    }
    // Returning a post-event mileage batch claim has no driver-editable resubmit path.
    if status == "returned" && r_kind == "mileage" && r_ride_id.is_none() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if status == "approved" && matches!(r_kind.as_str(), "mileage" | "visitor_special") {
        let linked_event = match r_event_id {
            Some(eid) => Some(eid),
            None => match r_ride_id {
                Some(rid) => {
                    sqlx::query_scalar::<_, Option<i64>>("SELECT event_id FROM rides WHERE id=$1")
                        .bind(rid)
                        .fetch_optional(&mut *tx)
                        .await?
                        .flatten()
                }
                None => None,
            },
        };
        if let Some(eid) = linked_event {
            let ev_status: Option<String> =
                sqlx::query_scalar("SELECT status FROM events WHERE id=$1 FOR SHARE")
                    .bind(eid)
                    .fetch_optional(&mut *tx)
                    .await?;
            if ev_status.as_deref() == Some("cancelled") {
                return Err(bad(u.lang, ErrorKey::EventNotOpen));
            }
        }
    }
    let changed = sqlx::query(
        "UPDATE reimbursements SET status=$1, review_note=$2, reviewed_by=$3, updated_at=$4
         WHERE id=$5 AND status='pending' AND user_id <> $3 AND requested_by IS DISTINCT FROM $3",
    )
    .bind(status)
    .bind(&note)
    .bind(u.id)
    .bind(now())
    .bind(id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
    // Approving a mileage receipt confirms the ride's reimbursement (unless excluded) and
    // clears any sibling trip receipt-needed flags.
    if status == "approved" && r_kind == "mileage" {
        if let Some(rid) = r_ride_id {
            let ride_row: Option<(i64, Option<i64>)> = sqlx::query_as(
                "SELECT reimbursement_excluded, trip_id FROM rides WHERE id=$1 FOR UPDATE",
            )
            .bind(rid)
            .fetch_optional(&mut *tx)
            .await?;
            let Some((excluded, ride_trip_id)) = ride_row else {
                return Err(not_found(u.lang));
            };
            if excluded == 1 {
                return Err(conflict(u.lang, ErrorKey::InvalidAction));
            }
            sqlx::query(
                "UPDATE rides SET reimbursement_amount_cents=$1, mileage_receipt_needed=0 WHERE id=$2",
            )
            .bind(r_amount_cents)
            .bind(rid)
            .execute(&mut *tx)
            .await?;
            if let Some(tid) = ride_trip_id {
                sqlx::query(
                    "UPDATE rides SET mileage_receipt_needed=0
                     WHERE trip_id=$1 AND driver_id=$2 AND id <> $3 AND mileage_receipt_needed=1",
                )
                .bind(tid)
                .bind(r_user_id)
                .bind(rid)
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    tx.commit().await?;
    let title = match status {
        "approved" => ("报销申请已批准", "Reimbursement approved"),
        "returned" => ("报销申请需补资料", "Reimbursement needs more info"),
        _ => ("报销申请未获批准", "Reimbursement not approved"),
    };
    notify(
        &st.db,
        r_user_id,
        title,
        (
            &format!("{} {}", fmt_rm(r_amount_cents), note),
            &format!("{} {}", fmt_rm(r_amount_cents), note),
        ),
        &format!("/reimbursements/{id}"),
    )
    .await;
    if status == "approved" {
        notify_perm_except(
            &st.db,
            Page::AdminReimbPay,
            Act::Update,
            Some(u.id),
            ("有报销待付款", "Reimbursement awaiting payment"),
            (
                &format!("{} · {}", r_user_name, fmt_rm(r_amount_cents)),
                &format!("{} · {}", r_user_name, fmt_rm(r_amount_cents)),
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
    if r.user_id == u.id || r.requested_by == Some(u.id) {
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
    // Lock the claim (and any linked event for mileage/visitor_special) inside a transaction
    // so payout cannot race event cancellation or a concurrent payout on the same claim.
    let mut tx = st.db.begin().await?;
    if matches!(r.kind.as_str(), "mileage" | "visitor_special") {
        if let Some(eid) = r.event_id {
            let ev_status: Option<String> =
                sqlx::query_scalar("SELECT status FROM events WHERE id=$1 FOR SHARE")
                    .bind(eid)
                    .fetch_optional(&mut *tx)
                    .await?;
            if ev_status.as_deref() == Some("cancelled") {
                return Err(bad(u.lang, ErrorKey::EventNotOpen));
            }
        }
    }
    let paid = sqlx::query(
        "UPDATE reimbursements
         SET status='paid', payee_verified=1, paid_ref=$1, paid_by=$2, paid_at=$3, updated_at=$4
         WHERE id=$5 AND status='approved' AND batch_id IS NULL
           AND user_id <> $2 AND reviewed_by IS DISTINCT FROM $2 AND requested_by IS DISTINCT FROM $2",
    )
    .bind(req.paid_ref.trim())
    .bind(u.id)
    .bind(now())
    .bind(now())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if paid.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbursementProcessed));
    }
    tx.commit().await?;
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
