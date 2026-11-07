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
use crate::error::{bad, forbidden, AppError, AppResult};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, now};

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
fn compute_shares(req: &SettlementIn) -> AppResult<Vec<(i64, i64)>> {
    if req.title.trim().is_empty() {
        return Err(bad("请填写结算名称"));
    }
    if req.items.is_empty() {
        return Err(bad("请至少填写一项开销"));
    }
    if req
        .items
        .iter()
        .any(|i| i.amount_cents < 0 || i.label.trim().is_empty())
    {
        return Err(bad("开销项目不能为负数或空白"));
    }
    let subsidy = req.subsidy_cents.unwrap_or(0);
    if subsidy < 0 {
        return Err(bad("补贴不能为负数"));
    }
    let gross: i64 = req.items.iter().map(|i| i.amount_cents).sum();
    let total = gross - subsidy;
    if total < 0 {
        return Err(bad("补贴不能超过开销总额"));
    }
    let mut seen = std::collections::HashSet::new();
    if !req.participants.iter().all(|p| seen.insert(p.user_id)) {
        return Err(bad("参与名单有重复"));
    }
    if req.participants.is_empty() {
        return Err(bad("请选择分摊成员"));
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
                let a = p.amount_cents.ok_or_else(|| bad("请为每位成员填写金额"))?;
                if a < 0 {
                    return Err(bad("金额不能为负数"));
                }
                out.push((p.user_id, a));
            }
            let sum: i64 = out.iter().map(|x| x.1).sum();
            if sum != total {
                return Err(bad(format!(
                    "个别金额合计 {} 与应分摊总额 {} 不一致",
                    fmt_rm(sum),
                    fmt_rm(total)
                )));
            }
            Ok(out)
        }
        _ => Err(bad("分摊方式无效")),
    }
}

async fn event_owner(st: &AppState, event_id: i64) -> AppResult<(i64, String)> {
    sqlx::query_as("SELECT created_by, title FROM events WHERE id=?")
        .bind(event_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(AppError::NotFound)
}

async fn write_body(
    st: &AppState,
    sid: i64,
    req: &SettlementIn,
    shares: &[(i64, i64)],
) -> AppResult<()> {
    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE settlements SET title=?, split_mode=?, subsidy_cents=?, payee_name=?, payee_account=?, due_date=? WHERE id=?")
        .bind(req.title.trim())
        .bind(&req.split_mode)
        .bind(req.subsidy_cents.unwrap_or(0))
        .bind(req.payee_name.trim())
        .bind(req.payee_account.trim())
        .bind(req.due_date.clone().filter(|d| !d.is_empty()))
        .bind(sid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM settlement_items WHERE settlement_id=?")
        .bind(sid)
        .execute(&mut *tx)
        .await?;
    for i in &req.items {
        sqlx::query("INSERT INTO settlement_items (settlement_id, label, amount_cents, receipt_file_id) VALUES (?,?,?,?)")
            .bind(sid)
            .bind(i.label.trim())
            .bind(i.amount_cents)
            .bind(i.receipt_file_id)
            .execute(&mut *tx)
            .await?;
    }
    // Upsert shares; keep payment state where the amount did not change.
    let existing: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT user_id, amount_cents, status FROM settlement_shares WHERE settlement_id=?",
    )
    .bind(sid)
    .fetch_all(&mut *tx)
    .await?;
    for (uid, old_amt, status) in &existing {
        match shares.iter().find(|s| s.0 == *uid) {
            None => {
                sqlx::query("DELETE FROM settlement_shares WHERE settlement_id=? AND user_id=?")
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
                sqlx::query("UPDATE settlement_shares SET amount_cents=?, status=?, note=?, updated_at=? WHERE settlement_id=? AND user_id=?")
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
            sqlx::query("INSERT INTO settlement_shares (settlement_id, user_id, amount_cents, status, updated_at) VALUES (?,?,?,?,?)")
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
    let (owner, _) = event_owner(&st, event_id).await?;
    if owner != u.id {
        return Err(forbidden("只有活动发起者可以发起费用分摊"));
    }
    let shares = compute_shares(&req)?;
    let sid = sqlx::query(
        "INSERT INTO settlements (event_id, created_by, title, created_at) VALUES (?,?,?,?)",
    )
    .bind(event_id)
    .bind(u.id)
    .bind(req.title.trim())
    .bind(now())
    .execute(&st.db)
    .await?
    .last_insert_rowid();
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

async fn load(st: &AppState, id: i64) -> AppResult<SRow> {
    sqlx::query_as::<_, SRow>(
        "SELECT s.id, s.event_id, s.created_by, s.title, s.split_mode, s.subsidy_cents, s.payee_name, s.payee_account, s.due_date,
                s.status, s.version, s.created_at, e.title AS event_title, e.starts_at AS event_starts_at, u.name AS creator_name
         FROM settlements s JOIN events e ON e.id=s.event_id JOIN users u ON u.id=s.created_by WHERE s.id=?",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?
    .ok_or(AppError::NotFound)
}

async fn snapshot(st: &AppState, s: &SRow) -> AppResult<Value> {
    let items: Vec<(i64, String, i64, Option<i64>)> = sqlx::query_as("SELECT id, label, amount_cents, receipt_file_id FROM settlement_items WHERE settlement_id=? ORDER BY id")
        .bind(s.id)
        .fetch_all(&st.db)
        .await?;
    let shares: Vec<(i64, i64, String, i64, String, Option<i64>, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT sh.id, sh.user_id, u.name, sh.amount_cents, sh.status, sh.proof_file_id, sh.txn_ref, sh.paid_date, sh.note, sh.updated_at
         FROM settlement_shares sh JOIN users u ON u.id=sh.user_id WHERE sh.settlement_id=? ORDER BY u.name",
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
    let s = load(&st, id).await?;
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
        return Err(forbidden("您不在此分摊名单中"));
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
            (SELECT COALESCE(SUM(amount_cents),0) FROM settlement_shares WHERE settlement_id=s.id),
            (SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=s.id AND status='confirmed'),
            (SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=s.id)
         FROM settlements s JOIN events e ON e.id=s.event_id WHERE s.created_by=? OR ? ORDER BY s.id DESC LIMIT 200",
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
    let s = load(&st, id).await?;
    if s.created_by != u.id {
        return Err(forbidden("只有发起者可以修改"));
    }
    if s.status == "closed" {
        return Err(bad("账单已关闭，不能修改"));
    }
    let shares = compute_shares(&req)?;
    if s.status == "published" {
        let reason = req
            .reason
            .clone()
            .filter(|r| !r.trim().is_empty())
            .ok_or_else(|| bad("发布后修改金额，请填写修改原因"))?;
        // 发布后若改金额，必须保留旧版并通知
        let snap = snapshot(&st, &s).await?;
        sqlx::query("INSERT INTO settlement_revisions (settlement_id, version, snapshot, reason, created_by, created_at) VALUES (?,?,?,?,?,?)")
            .bind(id)
            .bind(s.version)
            .bind(snap.to_string())
            .bind(&reason)
            .bind(u.id)
            .bind(now())
            .execute(&st.db)
            .await?;
        sqlx::query("UPDATE settlements SET version=version+1 WHERE id=?")
            .bind(id)
            .execute(&st.db)
            .await?;
        let before: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT user_id, amount_cents FROM settlement_shares WHERE settlement_id=?",
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
                        "费用分摊已更新",
                        format!(
                            "{}：{} → {}（{}）",
                            s.title,
                            fmt_rm(*old),
                            fmt_rm(*new),
                            reason
                        ),
                        "/costs",
                    )
                    .await
                }
                None => {
                    notify(
                        &st.db,
                        *uid,
                        "您已从分摊名单移除",
                        format!("{}（{}）", s.title, reason),
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
                    "新的费用分摊",
                    format!("{} · 您应付 {}", s.title, fmt_rm(*amt)),
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
    let s = load(&st, id).await?;
    if s.created_by != u.id || s.status != "draft" {
        return Err(bad("只有草稿可以删除"));
    }
    sqlx::query("DELETE FROM settlements WHERE id=?")
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
    let s = load(&st, id).await?;
    if s.created_by != u.id {
        return Err(forbidden("只有发起者可以发布"));
    }
    if s.status != "draft" {
        return Err(bad("此结算已发布"));
    }
    if s.payee_account.trim().is_empty() {
        return Err(bad("请填写收款资料（TNG 手机号 / DuitNow / 银行）"));
    }
    // 分摊前检查出席: only members whose attendance is confirmed may be charged.
    let (not_attended,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares sh WHERE sh.settlement_id=? AND NOT EXISTS
            (SELECT 1 FROM event_signups es WHERE es.event_id=? AND es.user_id=sh.user_id AND es.attended=1)",
    )
    .bind(id)
    .bind(s.event_id)
    .fetch_one(&st.db)
    .await?;
    if not_attended > 0 {
        return Err(bad(format!(
            "有 {not_attended} 位成员尚未记录为“已出席”，请先在活动页面核实出席。"
        )));
    }
    sqlx::query("UPDATE settlements SET status='published' WHERE id=?")
        .bind(id)
        .execute(&st.db)
        .await?;
    let shares: Vec<(i64, i64)> =
        sqlx::query_as("SELECT user_id, amount_cents FROM settlement_shares WHERE settlement_id=?")
            .bind(id)
            .fetch_all(&st.db)
            .await?;
    for (uid, amt) in shares {
        if uid != u.id && amt > 0 {
            notify(
                &st.db,
                uid,
                "费用分摊通知",
                format!(
                    "{} · 应付 {} · 付给 {}{}",
                    s.title,
                    fmt_rm(amt),
                    s.payee_name,
                    s.due_date
                        .as_deref()
                        .map(|d| format!(" · {d} 前"))
                        .unwrap_or_default()
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
    let s = load(&st, id).await?;
    if s.created_by != u.id {
        return Err(forbidden("只有发起者可以关闭账单"));
    }
    let (open,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=? AND status != 'confirmed'",
    )
    .bind(id)
    .fetch_one(&st.db)
    .await?;
    if open > 0 {
        return Err(bad(format!("尚有 {open} 位成员未确认付款")));
    }
    sqlx::query("UPDATE settlements SET status='closed' WHERE id=?")
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
    let s = load(&st, id).await?;
    if s.created_by != u.id && !(u.has("finance") && u.mfa_ok) {
        return Err(forbidden("无权限"));
    }
    let rows: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT r.version, r.snapshot, r.reason, u.name, r.created_at FROM settlement_revisions r JOIN users u ON u.id=r.created_by WHERE r.settlement_id=? ORDER BY r.version DESC",
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
         WHERE sh.user_id=? AND s.status IN ('published','closed') AND s.created_by != ?
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
        "SELECT sh.user_id, s.id, sh.status, s.status, s.created_by, s.title FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id WHERE sh.id=?",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    let (owner, sid, status, s_status, payee, title) = row.ok_or(AppError::NotFound)?;
    if owner != u.id {
        return Err(forbidden("只能为自己的费用上传凭证"));
    }
    if s_status != "published" || !matches!(status.as_str(), "unpaid" | "needs_info") {
        return Err(bad("此费用目前不需要上传凭证"));
    }
    let (fowner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=?")
        .bind(req.proof_file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| bad("请先上传收据"))?;
    if fowner != u.id {
        return Err(forbidden("凭证无效"));
    }
    let txn = req.txn_ref.trim();
    if txn.len() < 4 {
        return Err(bad("请填写交易参考编号"));
    }
    // Duplicate screenshot / reference protection.
    let (dup,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM settlement_shares WHERE id != ? AND (txn_ref = ? OR proof_file_id = ?)")
        .bind(id)
        .bind(txn)
        .bind(req.proof_file_id)
        .fetch_one(&st.db)
        .await?;
    if dup > 0 {
        return Err(bad("此交易编号或收据已被使用，请确认是否重复上传"));
    }
    sqlx::query("UPDATE settlement_shares SET status='pending', proof_file_id=?, txn_ref=?, paid_date=?, updated_at=? WHERE id=?")
        .bind(req.proof_file_id)
        .bind(txn)
        .bind(req.paid_date)
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    notify(
        &st.db,
        payee,
        "有新的付款凭证待核对",
        format!("{} · {}", u.name, title),
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
        "SELECT sh.user_id, s.id, s.created_by, s.title, sh.amount_cents FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id WHERE sh.id=?",
    )
    .bind(id)
    .fetch_optional(&st.db)
    .await?;
    let (member, _sid, payee, title, amt) = row.ok_or(AppError::NotFound)?;
    if payee != u.id {
        return Err(forbidden("只有收款者可以核对"));
    }
    let note = req.note.unwrap_or_default();
    let (status, msg) = match req.action.as_str() {
        "confirm" => (
            "confirmed",
            format!("{} · {} 已确认到账", title, fmt_rm(amt)),
        ),
        "needs_info" => {
            if note.trim().is_empty() {
                return Err(bad("请说明需补充的资料"));
            }
            ("needs_info", format!("{} · 需补资料：{}", title, note))
        }
        "reset" => ("unpaid", format!("{} · 付款状态已重设", title)),
        _ => return Err(bad("操作无效")),
    };
    sqlx::query("UPDATE settlement_shares SET status=?, note=?, updated_at=? WHERE id=?")
        .bind(status)
        .bind(&note)
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    notify(
        &st.db,
        member,
        if status == "confirmed" {
            "付款已确认"
        } else {
            "付款需要跟进"
        },
        msg,
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
