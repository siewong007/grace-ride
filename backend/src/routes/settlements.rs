//! 费用分摊 (e.g. 羽球场租): equal or custom split, payment proof upload, payee verification,
//! revision history. Grace Ride never moves money — members pay via TNG / DuitNow / bank and
//! the organiser confirms receipt ("上传收据 ≠ 自动证明收到钱").

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, now, Lang};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/{id}/settlements", post(create))
        .route("/settlements", get(list_mine_organised))
        .route(
            "/settlements/{id}",
            get(get_one).put(update).delete(delete_draft),
        )
        .route("/settlements/{id}/publish", post(publish))
        .route("/settlements/{id}/close", post(close))
        .route("/settlements/{id}/revisions", get(revisions))
        .route("/me/shares", get(my_shares))
        .route("/shares/{id}/proof", post(submit_proof))
        .route("/shares/{id}/verify", post(verify))
}

/// Split `total` cents among `n` people. Remainder cents go to the first members
/// so the sum always equals the total exactly (不整除时以分为单位处理尾差).
pub fn equal_split(total: i64, n: usize) -> Vec<i64> {
    if n == 0 {
        return vec![];
    }
    let n_i = n as i64;
    let base = total / n_i;
    let rem = total % n_i;
    (0..n_i)
        .map(|i| base + if i < rem { 1 } else { 0 })
        .collect()
}

#[derive(Deserialize, Serialize, Clone)]
struct ItemIn {
    label: String,
    amount_cents: i64,
    receipt_file_id: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone)]
struct ParticipantIn {
    user_id: i64,
    amount_cents: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone)]
struct SettlementIn {
    title: String,
    items: Vec<ItemIn>,
    subsidy_cents: Option<i64>,
    split_mode: String,
    participants: Vec<ParticipantIn>,
    payee_name: String,
    payee_account: String,
    due_date: Option<String>,
    reason: Option<String>,
}

/// Validates the request and returns (user_id, amount) pairs.
fn compute_shares(lang: Lang, req: &SettlementIn) -> AppResult<Vec<(i64, i64)>> {
    if req.title.trim().is_empty() {
        return Err(bad(lang, ErrorKey::SettlementNameRequired));
    }
    if req.items.is_empty() {
        return Err(bad(lang, ErrorKey::AtLeastOneExpenseItem));
    }
    if req
        .items
        .iter()
        .any(|i| i.amount_cents < 0 || i.label.trim().is_empty())
    {
        return Err(bad(lang, ErrorKey::ExpenseItemNegativeOrBlank));
    }
    let subsidy = req.subsidy_cents.unwrap_or(0);
    if subsidy < 0 {
        return Err(bad(lang, ErrorKey::SubsidyCannotBeNegative));
    }
    let gross: i64 = req.items.iter().map(|i| i.amount_cents).sum();
    let total = gross - subsidy;
    if total < 0 {
        return Err(bad(lang, ErrorKey::SubsidyExceedsTotal));
    }
    let mut seen = std::collections::HashSet::new();
    if !req.participants.iter().all(|p| seen.insert(p.user_id)) {
        return Err(bad(lang, ErrorKey::DuplicateParticipant));
    }
    if req.participants.is_empty() {
        return Err(bad(lang, ErrorKey::SelectParticipants));
    }
    match req.split_mode.as_str() {
        "equal" => {
            let amounts = equal_split(total, req.participants.len());
            Ok(req
                .participants
                .iter()
                .zip(amounts)
                .map(|(p, a)| (p.user_id, a))
                .collect())
        }
        "custom" => {
            let mut out = vec![];
            for p in &req.participants {
                let a = p
                    .amount_cents
                    .ok_or_else(|| bad(lang, ErrorKey::MemberAmountRequired))?;
                if a < 0 {
                    return Err(bad(lang, ErrorKey::AmountCannotBeNegative));
                }
                out.push((p.user_id, a));
            }
            let sum: i64 = out.iter().map(|x| x.1).sum();
            if sum != total {
                return Err(bad(
                    lang,
                    ErrorKey::CustomSplitMismatch {
                        sum: fmt_rm(sum),
                        total: fmt_rm(total),
                    },
                ));
            }
            Ok(out)
        }
        _ => Err(bad(lang, ErrorKey::InvalidSplitMode)),
    }
}

async fn event_owner(st: &AppState, lang: Lang, event_id: i64) -> AppResult<(i64, String)> {
    sqlx::query_as("SELECT created_by, title FROM events WHERE id=$1")
        .bind(event_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(lang))
}

async fn write_body(
    st: &AppState,
    sid: i64,
    req: &SettlementIn,
    shares: &[(i64, i64)],
) -> AppResult<()> {
    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE settlements SET title=$1, split_mode=$2, subsidy_cents=$3, payee_name=$4, payee_account=$5, due_date=$6, updated_at=$7 WHERE id=$8")
        .bind(req.title.trim())
        .bind(&req.split_mode)
        .bind(req.subsidy_cents.unwrap_or(0))
        .bind(req.payee_name.trim())
        .bind(req.payee_account.trim())
        .bind(req.due_date.clone().filter(|d| !d.is_empty()))
        .bind(now())
        .bind(sid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM settlement_items WHERE settlement_id=$1")
        .bind(sid)
        .execute(&mut *tx)
        .await?;
    for i in &req.items {
        sqlx::query("INSERT INTO settlement_items (settlement_id, label, amount_cents, receipt_file_id) VALUES ($1,$2,$3,$4)")
            .bind(sid)
            .bind(i.label.trim())
            .bind(i.amount_cents)
            .bind(i.receipt_file_id)
            .execute(&mut *tx)
            .await?;
    }
    // Upsert shares; keep payment state where the amount did not change.
    let existing: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT user_id, amount_cents, status FROM settlement_shares WHERE settlement_id=$1",
    )
    .bind(sid)
    .fetch_all(&mut *tx)
    .await?;
    for (uid, old_amt, status) in &existing {
        match shares.iter().find(|s| s.0 == *uid) {
            None => {
                sqlx::query("DELETE FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2")
                    .bind(sid)
                    .bind(uid)
                    .execute(&mut *tx)
                    .await?;
            }
            Some((_, amt)) if amt != old_amt => {
                let (new_status, note) = if status == "confirmed" || status == "pending" {
                    (
                        "needs_info",
                        format!(
                            "金额已由 {} 调整为 {}，请与发起者核对差额。",
                            fmt_rm(*old_amt),
                            fmt_rm(*amt)
                        ),
                    )
                } else {
                    ("unpaid", String::new())
                };
                sqlx::query("UPDATE settlement_shares SET amount_cents=$1, status=$2, note=$3, updated_at=$4 WHERE settlement_id=$5 AND user_id=$6")
                    .bind(amt)
                    .bind(new_status)
                    .bind(note)
                    .bind(now())
                    .bind(sid)
                    .bind(uid)
                    .execute(&mut *tx)
                    .await?;
            }
            _ => {}
        }
    }
    for (uid, amt) in shares {
        if !existing.iter().any(|e| e.0 == *uid) {
            sqlx::query("INSERT INTO settlement_shares (settlement_id, user_id, amount_cents, status, updated_at) VALUES ($1,$2,$3,$4,$5)")
                .bind(sid)
                .bind(uid)
                .bind(amt)
                .bind(if *amt == 0 { "confirmed" } else { "unpaid" })
                .bind(now())
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

async fn create(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(event_id): Path<i64>,
    Json(req): Json<SettlementIn>,
) -> AppResult<Json<Value>> {
    let (owner, _) = event_owner(&st, u.lang, event_id).await?;
    if owner != u.id {
        return Err(forbidden(
            u.lang,
            ErrorKey::OnlyOrganiserCanInitiateCostSplit,
        ));
    }
    let shares = compute_shares(u.lang, &req)?;
    let (sid,): (i64,) = sqlx::query_as(
        "INSERT INTO settlements (event_id, created_by, title, created_at) VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(event_id)
    .bind(u.id)
    .bind(req.title.trim())
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    write_body(&st, sid, &req, &shares).await?;
    audit(
        &st.db,
        Some(u.id),
        "settlement.create",
        "settlement",
        Some(sid),
        "",
    )
    .await;
    Ok(Json(json!({"id": sid})))
}

#[derive(sqlx::FromRow)]
struct SRow {
    id: i64,
    event_id: i64,
    created_by: i64,
    title: String,
    split_mode: String,
    subsidy_cents: i64,
    payee_name: String,
    payee_account: String,
    due_date: Option<String>,
    status: String,
    version: i64,
    created_at: String,
    event_title: String,
    event_starts_at: String,
    creator_name: String,
}

async fn load(st: &AppState, lang: Lang, id: i64) -> AppResult<SRow> {
    sqlx::query_as::<_, SRow>(
        "SELECT s.id, s.event_id, s.created_by, s.title, s.split_mode, s.subsidy_cents, s.payee_name, s.payee_account, s.due_date,
                s.status, s.version, s.created_at, e.title AS event_title, e.starts_at AS event_starts_at, u.name AS creator_name
         FROM settlements s JOIN events e ON e.id=s.event_id JOIN users u ON u.id=s.created_by WHERE s.id=$1",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?
    .ok_or(not_found(lang))
}

async fn snapshot(st: &AppState, s: &SRow) -> AppResult<Value> {
    let items: Vec<(i64, String, i64, Option<i64>)> = sqlx::query_as("SELECT id, label, amount_cents, receipt_file_id FROM settlement_items WHERE settlement_id=$1 ORDER BY id")
        .bind(s.id)
        .fetch_all(&st.db)
        .await?;
    let shares: Vec<(i64, i64, String, i64, String, Option<i64>, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT sh.id, sh.user_id, u.name, sh.amount_cents, sh.status, sh.proof_file_id, sh.txn_ref, sh.paid_date, sh.note, sh.updated_at
         FROM settlement_shares sh JOIN users u ON u.id=sh.user_id WHERE sh.settlement_id=$1 ORDER BY u.name",
    )
    .bind(s.id)
    .fetch_all(&st.db)
    .await?;
    let gross: i64 = items.iter().map(|i| i.2).sum();
    Ok(json!({
        "id": s.id, "event": {"id": s.event_id, "title": s.event_title, "starts_at": s.event_starts_at},
        "created_by": {"id": s.created_by, "name": s.creator_name},
        "title": s.title, "split_mode": s.split_mode, "subsidy_cents": s.subsidy_cents,
        "payee_name": s.payee_name, "payee_account": s.payee_account, "due_date": s.due_date,
        "status": s.status, "version": s.version, "created_at": s.created_at,
        "gross_cents": gross, "total_cents": gross - s.subsidy_cents,
        "items": items.into_iter().map(|(id, l, a, r)| json!({"id": id, "label": l, "amount_cents": a, "receipt_file_id": r})).collect::<Vec<_>>(),
        "shares": shares.into_iter().map(|(id, uid, name, amt, status, proof, txn, pd, note, upd)| json!({
            "id": id, "user_id": uid, "name": name, "amount_cents": amt, "status": status, "proof_file_id": proof,
            "txn_ref": txn, "paid_date": pd, "note": note, "updated_at": upd,
        })).collect::<Vec<_>>(),
    }))
}

async fn get_one(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    let mut v = snapshot(&st, &s).await?;
    let organiser = s.created_by == u.id || (u.has("finance") && u.mfa_ok);
    if organiser {
        v["can_manage"] = json!(s.created_by == u.id);
        return Ok(Json(v));
    }
    // Participants see the bill and only their own share.
    let shares = v["shares"].as_array().cloned().unwrap_or_default();
    let mine: Vec<Value> = shares
        .into_iter()
        .filter(|sh| sh["user_id"] == json!(u.id))
        .collect();
    if mine.is_empty() || s.status == "draft" {
        return Err(forbidden(u.lang, ErrorKey::NotOnCostSplitList));
    }
    v["participant_count"] = json!(v["shares"].as_array().map(|a| a.len()).unwrap_or(0));
    v["shares"] = json!(mine);
    v["can_manage"] = json!(false);
    Ok(Json(v))
}

async fn list_mine_organised(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let finance = u.has("finance") && u.mfa_ok;
    let rows: Vec<(i64, String, String, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT s.id, s.title, e.title, s.status,
            (SELECT COALESCE(SUM(amount_cents),0)::BIGINT FROM settlement_shares WHERE settlement_id=s.id),
            (SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=s.id AND status='confirmed'),
            (SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=s.id)
         FROM settlements s JOIN events e ON e.id=s.event_id WHERE s.created_by=$1 OR $2 ORDER BY s.id DESC LIMIT 200",
    )
    .bind(u.id)
    .bind(finance)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, et, s, total, conf, n)| json!({
        "id": id, "title": t, "event_title": et, "status": s, "total_cents": total, "confirmed": conf, "count": n,
    })).collect::<Vec<_>>())))
}

async fn update(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SettlementIn>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanEditBill));
    }
    if s.status == "closed" {
        return Err(bad(u.lang, ErrorKey::BillClosedCannotEdit));
    }
    let shares = compute_shares(u.lang, &req)?;
    if s.status == "published" {
        let reason = req
            .reason
            .clone()
            .filter(|r| !r.trim().is_empty())
            .ok_or_else(|| bad(u.lang, ErrorKey::RevisionReasonRequired))?;
        // 发布后若改金额，必须保留旧版并通知
        let snap = snapshot(&st, &s).await?;
        sqlx::query("INSERT INTO settlement_revisions (settlement_id, version, snapshot, reason, created_by, created_at) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(id)
            .bind(s.version)
            .bind(snap.to_string())
            .bind(&reason)
            .bind(u.id)
            .bind(now())
            .execute(&st.db)
            .await?;
        sqlx::query("UPDATE settlements SET version=version+1, updated_at=$1 WHERE id=$2")
            .bind(now())
            .bind(id)
            .execute(&st.db)
            .await?;
        let before: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT user_id, amount_cents FROM settlement_shares WHERE settlement_id=$1",
        )
        .bind(id)
        .fetch_all(&st.db)
        .await?;
        write_body(&st, id, &req, &shares).await?;
        for (uid, old) in &before {
            match shares.iter().find(|x| x.0 == *uid) {
                Some((_, new)) if new != old => {
                    notify(
                        &st.db,
                        *uid,
                        ("费用分摊已更新", "Cost split updated"),
                        (
                            &format!(
                                "{}：{} → {}（{}）",
                                s.title,
                                fmt_rm(*old),
                                fmt_rm(*new),
                                reason
                            ),
                            &format!(
                                "{}: {} → {} ({})",
                                s.title,
                                fmt_rm(*old),
                                fmt_rm(*new),
                                reason
                            ),
                        ),
                        "/costs",
                    )
                    .await
                }
                None => {
                    notify(
                        &st.db,
                        *uid,
                        ("您已从分摊名单移除", "Removed from the cost split"),
                        (
                            &format!("{}（{}）", s.title, reason),
                            &format!("{} ({})", s.title, reason),
                        ),
                        "/costs",
                    )
                    .await
                }
                _ => {}
            }
        }
        for (uid, amt) in &shares {
            if !before.iter().any(|b| b.0 == *uid) {
                notify(
                    &st.db,
                    *uid,
                    ("新的费用分摊", "New cost split"),
                    (
                        &format!("{} · 您应付 {}", s.title, fmt_rm(*amt)),
                        &format!("{} · You owe {}", s.title, fmt_rm(*amt)),
                    ),
                    "/costs",
                )
                .await;
            }
        }
        audit(
            &st.db,
            Some(u.id),
            "settlement.revise",
            "settlement",
            Some(id),
            reason,
        )
        .await;
    } else {
        write_body(&st, id, &req, &shares).await?;
        audit(
            &st.db,
            Some(u.id),
            "settlement.edit_draft",
            "settlement",
            Some(id),
            "",
        )
        .await;
    }
    Ok(Json(json!({"ok": true})))
}

async fn delete_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id || s.status != "draft" {
        return Err(bad(u.lang, ErrorKey::OnlyDraftsCanBeDeleted));
    }
    sqlx::query("DELETE FROM settlements WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({"ok": true})))
}

async fn publish(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanPublish));
    }
    if s.status != "draft" {
        return Err(bad(u.lang, ErrorKey::SettlementAlreadyPublished));
    }
    if s.payee_account.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::PayeeDetailsRequired));
    }
    // 分摊前检查出席: only members whose attendance is confirmed may be charged.
    let (not_attended,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares sh WHERE sh.settlement_id=$1 AND NOT EXISTS
            (SELECT 1 FROM event_signups es WHERE es.event_id=$2 AND es.user_id=sh.user_id AND es.attended=1)",
    )
    .bind(id)
    .bind(s.event_id)
    .fetch_one(&st.db)
    .await?;
    if not_attended > 0 {
        return Err(bad(u.lang, ErrorKey::UnattendedMembers(not_attended)));
    }
    sqlx::query("UPDATE settlements SET status='published', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    let shares: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, amount_cents FROM settlement_shares WHERE settlement_id=$1",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    for (uid, amt) in shares {
        if uid != u.id && amt > 0 {
            let due_zh = s
                .due_date
                .as_deref()
                .map(|d| format!(" · {d} 前"))
                .unwrap_or_default();
            let due_en = s
                .due_date
                .as_deref()
                .map(|d| format!(" · due by {d}"))
                .unwrap_or_default();
            notify(
                &st.db,
                uid,
                ("费用分摊通知", "Cost split issued"),
                (
                    &format!(
                        "{} · 应付 {} · 付给 {}{}",
                        s.title,
                        fmt_rm(amt),
                        s.payee_name,
                        due_zh
                    ),
                    &format!(
                        "{} · You owe {} · Pay to {}{}",
                        s.title,
                        fmt_rm(amt),
                        s.payee_name,
                        due_en
                    ),
                ),
                "/costs",
            )
            .await;
        }
    }
    audit(
        &st.db,
        Some(u.id),
        "settlement.publish",
        "settlement",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn close(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanCloseBill));
    }
    let (open,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=$1 AND status != 'confirmed'",
    )
    .bind(id)
    .fetch_one(&st.db)
    .await?;
    if open > 0 {
        return Err(bad(u.lang, ErrorKey::OpenPaymentsRemain(open)));
    }
    sqlx::query("UPDATE settlements SET status='closed', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "settlement.close",
        "settlement",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn revisions(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id && !(u.has("finance") && u.mfa_ok) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    let rows: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT r.version, r.snapshot, r.reason, u.name, r.created_at FROM settlement_revisions r JOIN users u ON u.id=r.created_by WHERE r.settlement_id=$1 ORDER BY r.version DESC",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(v, snap, reason, by, at)| json!({
        "version": v, "snapshot": serde_json::from_str::<Value>(&snap).unwrap_or(Value::Null), "reason": reason, "by": by, "created_at": at,
    })).collect::<Vec<_>>())))
}

async fn my_shares(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, i64, String, String, i64, String, String, String, Option<String>, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT sh.id, s.id, s.title, e.title, sh.amount_cents, sh.status, s.payee_name, s.payee_account, s.due_date, sh.note, sh.txn_ref, sh.proof_file_id
         FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id JOIN events e ON e.id=s.event_id
         WHERE sh.user_id=$1 AND s.status IN ('published','closed') AND s.created_by != $2
         ORDER BY CASE sh.status WHEN 'confirmed' THEN 1 ELSE 0 END, s.id DESC LIMIT 100",
    )
    .bind(u.id)
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, sid, t, et, amt, status, pn, pa, due, note, txn, proof)| json!({
        "id": id, "settlement_id": sid, "title": t, "event_title": et, "amount_cents": amt, "status": status,
        "payee_name": pn, "payee_account": pa, "due_date": due, "note": note, "txn_ref": txn, "proof_file_id": proof,
    })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct ProofIn {
    proof_file_id: i64,
    txn_ref: String,
    paid_date: Option<String>,
}

async fn submit_proof(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ProofIn>,
) -> AppResult<Json<Value>> {
    let row: Option<(i64, i64, String, String, i64, String)> = sqlx::query_as(
        "SELECT sh.user_id, s.id, sh.status, s.status, s.created_by, s.title FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id WHERE sh.id=$1",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    let (owner, sid, status, s_status, payee, title) = row.ok_or(not_found(u.lang))?;
    if owner != u.id {
        return Err(forbidden(u.lang, ErrorKey::ProofOnlyForOwnShare));
    }
    if s_status != "published" || !matches!(status.as_str(), "unpaid" | "needs_info") {
        return Err(bad(u.lang, ErrorKey::ProofNotNeededNow));
    }
    let (fowner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
        .bind(req.proof_file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::UploadReceiptFirst))?;
    if fowner != u.id {
        return Err(forbidden(u.lang, ErrorKey::InvalidProofDocument));
    }
    let txn = req.txn_ref.trim();
    if txn.len() < 4 {
        return Err(bad(u.lang, ErrorKey::TxnRefRequired));
    }
    // Duplicate screenshot / reference protection.
    let (dup,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM settlement_shares WHERE id != $1 AND (txn_ref = $2 OR proof_file_id = $3)")
        .bind(id)
        .bind(txn)
        .bind(req.proof_file_id)
        .fetch_one(&st.db)
        .await?;
    if dup > 0 {
        return Err(bad(u.lang, ErrorKey::DuplicateTxnOrProof));
    }
    sqlx::query("UPDATE settlement_shares SET status='pending', proof_file_id=$1, txn_ref=$2, paid_date=$3, updated_at=$4 WHERE id=$5")
        .bind(req.proof_file_id)
        .bind(txn)
        .bind(req.paid_date)
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    let by = format!("{} · {}", u.name, title);
    notify(
        &st.db,
        payee,
        ("有新的付款凭证待核对", "New payment proof to verify"),
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
        txn,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct VerifyIn {
    action: String, // confirm | needs_info | reset
    note: Option<String>,
}

async fn verify(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<VerifyIn>,
) -> AppResult<Json<Value>> {
    let row: Option<(i64, i64, i64, String, i64)> = sqlx::query_as(
        "SELECT sh.user_id, s.id, s.created_by, s.title, sh.amount_cents FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id WHERE sh.id=$1",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    let (member, _sid, payee, title, amt) = row.ok_or(not_found(u.lang))?;
    if payee != u.id {
        return Err(forbidden(u.lang, ErrorKey::OnlyPayeeCanVerify));
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
    sqlx::query("UPDATE settlement_shares SET status=$1, note=$2, updated_at=$3 WHERE id=$4")
        .bind(status)
        .bind(&note)
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
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

#[cfg(test)]
mod tests {
    use super::equal_split;

    #[test]
    fn split_exact() {
        assert_eq!(equal_split(16800, 6), vec![2800; 6]);
    }

    #[test]
    fn split_remainder_in_cents() {
        let v = equal_split(10000, 3);
        assert_eq!(v, vec![3334, 3333, 3333]);
        assert_eq!(v.iter().sum::<i64>(), 10000);
    }

    #[test]
    fn split_zero_people() {
        assert!(equal_split(100, 0).is_empty());
    }
}
