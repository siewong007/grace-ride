//! 费用分摊 (e.g. 羽球场租): equal or custom split, payment proof upload, payee verification,
//! revision history. Grace Ride never moves money — members pay via TNG / DuitNow / bank and
//! the organiser confirms receipt ("上传收据 ≠ 自动证明收到钱").

mod manage;
mod shares;

use axum::{
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::audience::audience_users_sql;
use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{fmt_rm, now, Lang};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/{id}/settlements", post(manage::create))
        .route("/settlements", get(manage::list_mine_organised))
        .route(
            "/settlements/{id}",
            get(manage::get_one)
                .put(manage::update)
                .delete(manage::delete_draft),
        )
        .route("/settlements/{id}/publish", post(manage::publish))
        .route("/settlements/{id}/close", post(manage::close))
        .route("/settlements/{id}/revisions", get(manage::revisions))
        .route("/me/shares", get(shares::my_shares))
        .route("/shares/{id}/proof", post(shares::submit_proof))
        .route("/shares/{id}/verify", post(shares::verify))
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
pub(super) struct ItemIn {
    label: String,
    amount_cents: i64,
    receipt_file_id: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct ParticipantIn {
    user_id: i64,
    amount_cents: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(super) struct SettlementIn {
    title: String,
    items: Vec<ItemIn>,
    subsidy_cents: Option<i64>,
    split_mode: String,
    participants: Vec<ParticipantIn>,
    payee_name: String,
    payee_account: String,
    payee_qr_file_id: Option<i64>,
    due_date: Option<String>,
    pub(super) reason: Option<String>,
}

const MAX_AMOUNT_CENTS: i64 = 1_000_000_000;

/// Validates the request and returns (user_id, amount) pairs.
pub(super) fn compute_shares(lang: Lang, req: &SettlementIn) -> AppResult<Vec<(i64, i64)>> {
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
    if req.items.iter().any(|i| i.amount_cents > MAX_AMOUNT_CENTS) {
        return Err(bad(lang, ErrorKey::InvalidAction));
    }
    let subsidy = req.subsidy_cents.unwrap_or(0);
    if subsidy < 0 {
        return Err(bad(lang, ErrorKey::SubsidyCannotBeNegative));
    }
    if subsidy > MAX_AMOUNT_CENTS {
        return Err(bad(lang, ErrorKey::InvalidAction));
    }
    let gross: i64 = req
        .items
        .iter()
        .try_fold(0i64, |acc, i| acc.checked_add(i.amount_cents))
        .ok_or_else(|| bad(lang, ErrorKey::InvalidAction))?;
    if gross > MAX_AMOUNT_CENTS {
        return Err(bad(lang, ErrorKey::InvalidAction));
    }
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
            let mut sum = 0i64;
            for p in &req.participants {
                let a = p
                    .amount_cents
                    .ok_or_else(|| bad(lang, ErrorKey::MemberAmountRequired))?;
                if a < 0 {
                    return Err(bad(lang, ErrorKey::AmountCannotBeNegative));
                }
                if a > MAX_AMOUNT_CENTS {
                    return Err(bad(lang, ErrorKey::InvalidAction));
                }
                sum = sum
                    .checked_add(a)
                    .ok_or_else(|| bad(lang, ErrorKey::InvalidAction))?;
                out.push((p.user_id, a));
            }
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

/// A receipt on a bill must be a file the organiser uploaded themself. Attaching a
/// file makes it readable through the bill, so someone else's file id is refused
/// (and an unknown id answers the same way, so ids can't be probed).
/// Locks referenced `files` rows in sorted ID order (`FOR KEY SHARE`) inside the
/// caller's write transaction so background orphan GC cannot delete them mid-save.
pub(super) async fn check_receipts_on(
    conn: &mut sqlx::PgConnection,
    u: &CurrentUser,
    req: &SettlementIn,
) -> AppResult<()> {
    let mut receipt_ids: Vec<i64> = req.items.iter().filter_map(|i| i.receipt_file_id).collect();
    receipt_ids.sort_unstable();
    receipt_ids.dedup();
    for file_id in receipt_ids {
        let owner: Option<(i64,)> =
            sqlx::query_as("SELECT owner_id FROM files WHERE id=$1 FOR KEY SHARE")
                .bind(file_id)
                .fetch_optional(&mut *conn)
                .await?;
        if owner != Some((u.id,)) {
            return Err(forbidden(u.lang, ErrorKey::InvalidReceipt));
        }
    }
    if let Some(qr_id) = req.payee_qr_file_id {
        let row: Option<(i64, String)> =
            sqlx::query_as("SELECT owner_id, mime FROM files WHERE id=$1 FOR KEY SHARE")
                .bind(qr_id)
                .fetch_optional(&mut *conn)
                .await?;
        match row {
            Some((owner, mime)) if owner == u.id && mime.starts_with("image/") => {}
            _ => return Err(bad(u.lang, ErrorKey::InvalidPhoto)),
        }
    }
    Ok(())
}

pub(super) async fn event_owner(
    st: &AppState,
    lang: Lang,
    event_id: i64,
) -> AppResult<(i64, String)> {
    sqlx::query_as("SELECT created_by, title FROM events WHERE id=$1")
        .bind(event_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(lang))
}

/// Verify all referenced participant user IDs exist before writing settlement shares.
pub(super) async fn ensure_users_exist_on(
    conn: &mut sqlx::PgConnection,
    lang: Lang,
    user_ids: &[i64],
) -> AppResult<()> {
    let unique: Vec<i64> = {
        let mut s = std::collections::BTreeSet::new();
        user_ids
            .iter()
            .copied()
            .filter(|id| s.insert(*id))
            .collect()
    };
    if unique.is_empty() {
        return Ok(());
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT id) FROM users WHERE id = ANY($1)")
        .bind(&unique)
        .fetch_one(&mut *conn)
        .await?;
    if count != unique.len() as i64 {
        return Err(bad(lang, ErrorKey::InvalidAction));
    }
    Ok(())
}

/// Only members recorded as attended may be charged. Publish and later revisions use
/// the same gate, otherwise a published bill could silently add an absent member.
pub(super) async fn ensure_all_attended_on(
    conn: &mut sqlx::PgConnection,
    lang: Lang,
    event_id: i64,
    user_ids: Vec<i64>,
) -> AppResult<()> {
    let att_enabled: Option<(i64,)> =
        sqlx::query_as("SELECT attendance_enabled FROM events WHERE id=$1")
            .bind(event_id)
            .fetch_optional(&mut *conn)
            .await?;
    if att_enabled.map(|(a,)| a == 0).unwrap_or(false) {
        // Attendance is not tracked, but a bill must still only reach people connected
        // to the event: the organiser, a real 'going' signup, or the event's audience.
        // Without this the organiser could switch tracking off and bill any member.
        let (outside,): (i64,) = sqlx::query_as(crate::dyn_sql!(
            "SELECT COUNT(*) FROM unnest($2::BIGINT[]) AS u(user_id) WHERE NOT (
                EXISTS (SELECT 1 FROM events ev WHERE ev.id=$1 AND ev.created_by=u.user_id)
                OR EXISTS (SELECT 1 FROM event_signups es WHERE es.event_id=$1 AND es.user_id=u.user_id AND es.status='going')
                OR u.user_id IN ({}))",
            audience_users_sql("$1")
        ))
        .bind(event_id)
        .bind(user_ids)
        .fetch_one(&mut *conn)
        .await?;
        if outside > 0 {
            return Err(bad(lang, ErrorKey::UnattendedMembers(outside)));
        }
        return Ok(());
    }
    let (not_attended,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM unnest($2::BIGINT[]) AS u(user_id) WHERE NOT EXISTS
            (SELECT 1 FROM event_signups es WHERE es.event_id=$1 AND es.user_id=u.user_id AND es.attended=1)",
    )
    .bind(event_id)
    .bind(user_ids)
    .fetch_one(&mut *conn)
    .await?;
    if not_attended > 0 {
        return Err(bad(lang, ErrorKey::UnattendedMembers(not_attended)));
    }
    Ok(())
}

/// What a member has paid towards a share, 0 if nothing. `paid_cents` is recorded when
/// proof is sent or a payment confirmed, and kept through revisions; a confirmed or
/// pending share from before it was recorded counts its own amount as paid.
pub(super) fn paid_amount(amount_cents: i64, status: &str, paid_cents: i64) -> i64 {
    match status {
        "unpaid" => 0,
        _ if paid_cents > 0 => paid_cents,
        "confirmed" | "pending" => amount_cents,
        _ => 0,
    }
}

/// Writes the bill's fields, items and shares inside the caller's transaction, so a
/// failure part-way leaves the bill as it was.
pub(super) async fn write_body(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    sid: i64,
    req: &SettlementIn,
    shares: &[(i64, i64)],
) -> AppResult<()> {
    sqlx::query("UPDATE settlements SET title=$1, split_mode=$2, subsidy_cents=$3, payee_name=$4, payee_account=$5, payee_qr_file_id=$6, due_date=$7, updated_at=$8 WHERE id=$9")
        .bind(req.title.trim())
        .bind(&req.split_mode)
        .bind(req.subsidy_cents.unwrap_or(0))
        .bind(req.payee_name.trim())
        .bind(req.payee_account.trim())
        .bind(req.payee_qr_file_id)
        .bind(req.due_date.clone().filter(|d| !d.is_empty()))
        .bind(now())
        .bind(sid)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM settlement_items WHERE settlement_id=$1")
        .bind(sid)
        .execute(&mut **tx)
        .await?;
    for i in &req.items {
        sqlx::query("INSERT INTO settlement_items (settlement_id, label, amount_cents, receipt_file_id) VALUES ($1,$2,$3,$4)")
            .bind(sid)
            .bind(i.label.trim())
            .bind(i.amount_cents)
            .bind(i.receipt_file_id)
            .execute(&mut **tx)
            .await?;
    }
    // Upsert shares; keep payment state where the amount did not change.
    let existing: Vec<(i64, i64, String, i64)> = sqlx::query_as(
        "SELECT user_id, amount_cents, status, paid_cents FROM settlement_shares WHERE settlement_id=$1",
    )
    .bind(sid)
    .fetch_all(&mut **tx)
    .await?;
    for (uid, old_amt, status, paid_cents) in &existing {
        match shares.iter().find(|s| s.0 == *uid) {
            None => {
                sqlx::query("DELETE FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2")
                    .bind(sid)
                    .bind(uid)
                    .execute(&mut **tx)
                    .await?;
            }
            Some((_, amt)) if amt != old_amt => {
                // Once money has changed hands the share is reconciled against what was
                // paid, not against the previous amount, and stays so through any number
                // of revisions: a share already waiting on a reconciliation keeps its
                // payment. Anything else starts afresh: a share of nothing is settled,
                // any other amount is owed.
                let paid = paid_amount(*old_amt, status, *paid_cents);
                let (new_status, note) = if paid > 0 && paid == *amt {
                    // Revised back to what was paid: nothing to reconcile, but the payee
                    // checks it again rather than it being confirmed on their behalf.
                    ("pending", String::new())
                } else if paid > 0 {
                    (
                        "needs_info",
                        format!(
                            "已付 {}，金额已调整为 {}，请与发起者核对差额。",
                            fmt_rm(paid),
                            fmt_rm(*amt)
                        ),
                    )
                } else if *amt == 0 {
                    ("confirmed", String::new())
                } else {
                    ("unpaid", String::new())
                };
                sqlx::query("UPDATE settlement_shares SET amount_cents=$1, status=$2, note=$3, paid_cents=$4, updated_at=$5 WHERE settlement_id=$6 AND user_id=$7")
                    .bind(amt)
                    .bind(new_status)
                    .bind(note)
                    .bind(paid)
                    .bind(now())
                    .bind(sid)
                    .bind(uid)
                    .execute(&mut **tx)
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
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
pub(super) struct SRow {
    pub(super) id: i64,
    pub(super) event_id: i64,
    pub(super) created_by: i64,
    pub(super) title: String,
    pub(super) split_mode: String,
    pub(super) subsidy_cents: i64,
    pub(super) payee_name: String,
    pub(super) payee_account: String,
    pub(super) payee_qr_file_id: Option<i64>,
    pub(super) due_date: Option<String>,
    pub(super) status: String,
    pub(super) version: i64,
    pub(super) created_at: String,
    pub(super) event_title: String,
    pub(super) event_starts_at: String,
    pub(super) creator_name: String,
}

pub(super) async fn load_on<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    lang: Lang,
    id: i64,
) -> AppResult<SRow> {
    sqlx::query_as::<_, SRow>(
        "SELECT s.id, s.event_id, s.created_by, s.title, s.split_mode, s.subsidy_cents, s.payee_name, s.payee_account, s.payee_qr_file_id, s.due_date,
                s.status, s.version, s.created_at, e.title AS event_title, e.starts_at AS event_starts_at, u.name AS creator_name
         FROM settlements s JOIN events e ON e.id=s.event_id JOIN users u ON u.id=s.created_by WHERE s.id=$1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or(not_found(lang))
}

pub(super) async fn load(st: &AppState, lang: Lang, id: i64) -> AppResult<SRow> {
    load_on(&st.db, lang, id).await
}

pub(super) async fn snapshot_on(db: &mut sqlx::PgConnection, s: &SRow) -> AppResult<Value> {
    let items: Vec<(i64, String, i64, Option<i64>)> = sqlx::query_as("SELECT id, label, amount_cents, receipt_file_id FROM settlement_items WHERE settlement_id=$1 ORDER BY id")
        .bind(s.id)
        .fetch_all(&mut *db)
        .await?;
    let shares: Vec<(i64, i64, String, i64, i64, String, Option<i64>, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT sh.id, sh.user_id, u.name, sh.amount_cents, sh.paid_cents, sh.status, sh.proof_file_id, sh.txn_ref, sh.paid_date, sh.note, sh.updated_at
         FROM settlement_shares sh JOIN users u ON u.id=sh.user_id WHERE sh.settlement_id=$1 ORDER BY u.name",
    )
    .bind(s.id)
    .fetch_all(&mut *db)
    .await?;
    let gross: i64 = items.iter().map(|i| i.2).sum();
    Ok(json!({
        "id": s.id, "event": {"id": s.event_id, "title": s.event_title, "starts_at": s.event_starts_at},
        "created_by": {"id": s.created_by, "name": s.creator_name},
        "title": s.title, "split_mode": s.split_mode, "subsidy_cents": s.subsidy_cents,
        "payee_name": s.payee_name, "payee_account": s.payee_account, "payee_qr_file_id": s.payee_qr_file_id, "due_date": s.due_date,
        "status": s.status, "version": s.version, "created_at": s.created_at,
        "gross_cents": gross, "total_cents": gross - s.subsidy_cents,
        "items": items.into_iter().map(|(id, l, a, r)| json!({"id": id, "label": l, "amount_cents": a, "receipt_file_id": r})).collect::<Vec<_>>(),
        "shares": shares.into_iter().map(|(id, uid, name, amt, paid_cents, status, proof, txn, pd, note, upd)| json!({
            "id": id, "user_id": uid, "name": name, "amount_cents": amt, "paid_cents": paid_cents, "status": status, "proof_file_id": proof,
            "txn_ref": txn, "paid_date": pd, "note": note, "updated_at": upd,
        })).collect::<Vec<_>>(),
    }))
}

pub(super) async fn snapshot(st: &AppState, s: &SRow) -> AppResult<Value> {
    let mut conn = st.db.acquire().await?;
    snapshot_on(&mut conn, s).await
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
