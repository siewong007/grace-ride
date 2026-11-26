//! Participant cost shares, payment proof submission, and payee verification.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, now, require};

pub(super) async fn my_shares(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::Costs, Act::View)?;
    let rows: Vec<(i64, i64, String, String, i64, String, String, String, Option<i64>, Option<String>, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT sh.id, s.id, s.title, e.title, sh.amount_cents, sh.status, s.payee_name, s.payee_account, s.payee_qr_file_id, s.due_date, sh.note, sh.txn_ref, sh.proof_file_id
         FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id JOIN events e ON e.id=s.event_id
         WHERE sh.user_id=$1 AND s.status IN ('published','closed') AND s.created_by != $2
         ORDER BY CASE sh.status WHEN 'confirmed' THEN 1 ELSE 0 END, s.id DESC",
    )
    .bind(u.id)
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, sid, t, et, amt, status, pn, pa, qr, due, note, txn, proof)| json!({
        "id": id, "settlement_id": sid, "title": t, "event_title": et, "amount_cents": amt, "status": status,
        "payee_name": pn, "payee_account": pa, "payee_qr_file_id": qr, "due_date": due, "note": note, "txn_ref": txn, "proof_file_id": proof,
    })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub(super) struct ProofIn {
    proof_file_id: i64,
    txn_ref: String,
    paid_date: Option<String>,
}

pub(super) async fn submit_proof(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ProofIn>,
) -> AppResult<Json<Value>> {
    // Serialise receipt/reference checks. Without a lock two submissions can both
    // pass the duplicate check before either writes, accepting the same proof twice.
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_settlement_proof', 0))")
        .execute(&mut *tx)
        .await?;

    // Lock the settlement first (matching revision/close lock order) and then the share,
    // so concurrent revisions cannot overwrite or orphan a just-submitted payment proof.
    let settlement_row: Option<(i64, String, i64, String)> = sqlx::query_as(
        "SELECT s.id, s.status, s.created_by, s.title
         FROM settlements s
         WHERE s.id = (SELECT settlement_id FROM settlement_shares WHERE id=$1)
         FOR UPDATE OF s",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let (sid, s_status, payee, title) = settlement_row.ok_or(not_found(u.lang))?;
    let share_row: Option<(i64, String)> = sqlx::query_as(
        "SELECT user_id, status FROM settlement_shares WHERE id=$1 AND settlement_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(sid)
    .fetch_optional(&mut *tx)
    .await?;
    let (owner, status) = share_row.ok_or(not_found(u.lang))?;
    if owner != u.id {
        return Err(forbidden(u.lang, ErrorKey::ProofOnlyForOwnShare));
    }
    require(&u, Page::Costs, Act::Update)?;
    if s_status != "published" || !matches!(status.as_str(), "unpaid" | "needs_info") {
        return Err(bad(u.lang, ErrorKey::ProofNotNeededNow));
    }
    let (fowner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1 FOR KEY SHARE")
        .bind(req.proof_file_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::UploadReceiptFirst))?;
    if fowner != u.id {
        return Err(forbidden(u.lang, ErrorKey::InvalidProofDocument));
    }
    let txn = req.txn_ref.trim();
    if txn.len() < 4 {
        return Err(bad(u.lang, ErrorKey::TxnRefRequired));
    }
    // Duplicate screenshot / reference protection: scoped to this settlement or this member's shares.
    let (dup,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares
         WHERE (settlement_id = $1 OR user_id = $2)
           AND id != $3
           AND (LOWER(BTRIM(txn_ref)) = LOWER($4) OR proof_file_id = $5)",
    )
    .bind(sid)
    .bind(u.id)
    .bind(id)
    .bind(txn)
    .bind(req.proof_file_id)
    .fetch_one(&mut *tx)
    .await?;
    if dup > 0 {
        return Err(bad(u.lang, ErrorKey::DuplicateTxnOrProof));
    }
    // The same reference on another member's bill is still a likely reused receipt, but
    // refusing here would tell the member that reference exists somewhere. Accept it and
    // flag it to whoever verifies this share instead.
    let (elsewhere,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares
         WHERE LOWER(BTRIM(txn_ref)) = LOWER($1) AND user_id <> $2 AND settlement_id <> $3",
    )
    .bind(txn)
    .bind(u.id)
    .bind(sid)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE settlement_shares SET status='pending', proof_file_id=$1, txn_ref=$2, paid_date=$3, updated_at=$4, paid_cents=amount_cents WHERE id=$5")
        .bind(req.proof_file_id)
        .bind(txn)
        .bind(req.paid_date)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let by = format!("{} · {}", u.name, title);
    let title_pair = if elsewhere > 0 {
        (
            "付款凭证待核对：交易编号已在其他账单使用",
            "Payment proof to verify: reference already used on another bill",
        )
    } else {
        ("有新的付款凭证待核对", "New payment proof to verify")
    };
    notify(
        &st.db,
        payee,
        title_pair,
        (&by, &by),
        &format!("/settlements/{sid}"),
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "share.proof",
        "settlement_share",
        Some(id),
        if elsewhere > 0 {
            format!("{txn} (reference also used on {elsewhere} other share(s))")
        } else {
            txn.to_string()
        },
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct VerifyIn {
    action: String, // confirm | needs_info | reset
    note: Option<String>,
}

pub(super) async fn verify(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<VerifyIn>,
) -> AppResult<Json<Value>> {
    // Lock the parent settlement before its share so lock order matches revisions/closing.
    let mut tx = st.db.begin().await?;
    let settlement_row: Option<(i64, i64, String, String)> = sqlx::query_as(
        "SELECT s.id, s.created_by, s.title, s.status
         FROM settlements s
         WHERE s.id = (SELECT settlement_id FROM settlement_shares WHERE id=$1)
         FOR UPDATE OF s",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let (sid, payee, title, s_status) = settlement_row.ok_or(not_found(u.lang))?;
    let share_row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, amount_cents FROM settlement_shares WHERE id=$1 AND settlement_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(sid)
    .fetch_optional(&mut *tx)
    .await?;
    let (member, amt) = share_row.ok_or(not_found(u.lang))?;
    if payee != u.id || !u.can(Page::Settlements, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyPayeeCanVerify));
    }
    // A closed bill is final: confirming, asking for more info, or resetting a share
    // must not mutate payment state after the organiser has closed it.
    if s_status == "closed" {
        return Err(bad(u.lang, ErrorKey::BillClosedCannotEdit));
    }
    if s_status != "published" {
        return Err(bad(u.lang, ErrorKey::ProofNotNeededNow));
    }
    let note = req.note.unwrap_or_default();
    let (status, zh_msg, en_msg) = match req.action.as_str() {
        "confirm" => (
            "confirmed",
            format!("{} · {} 已确认到账", title, fmt_rm(amt)),
            format!("{} · {} confirmed as received", title, fmt_rm(amt)),
        ),
        "needs_info" => {
            if note.trim().is_empty() {
                return Err(bad(u.lang, ErrorKey::NeedMoreInfoReasonRequired));
            }
            (
                "needs_info",
                format!("{} · 需补资料：{}", title, note),
                format!("{} · More info needed: {}", title, note),
            )
        }
        "reset" => (
            "unpaid",
            format!("{} · 付款状态已重设", title),
            format!("{} · Payment status has been reset", title),
        ),
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    };
    // Confirming records the share as paid in full; a reset says nothing was received.
    // Asking for more information leaves what was paid as it stands.
    sqlx::query(
        "UPDATE settlement_shares SET status=$1, note=$2, updated_at=$3,
            paid_cents = CASE $1 WHEN 'confirmed' THEN amount_cents WHEN 'unpaid' THEN 0 ELSE paid_cents END,
            proof_file_id = CASE WHEN $1 = 'unpaid' THEN NULL ELSE proof_file_id END,
            txn_ref = CASE WHEN $1 = 'unpaid' THEN '' ELSE txn_ref END,
            paid_date = CASE WHEN $1 = 'unpaid' THEN NULL ELSE paid_date END
         WHERE id=$4",
    )
        .bind(status)
        .bind(&note)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let (zh_title, en_title) = if status == "confirmed" {
        ("付款已确认", "Payment confirmed")
    } else {
        ("付款需要跟进", "Payment needs follow-up")
    };
    notify(
        &st.db,
        member,
        (zh_title, en_title),
        (&zh_msg, &en_msg),
        "/costs",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        &format!("share.{}", req.action),
        "settlement_share",
        Some(id),
        note,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}
