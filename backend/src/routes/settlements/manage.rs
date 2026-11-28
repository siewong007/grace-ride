//! Settlement creation, organiser views, edits/revisions, publishing, and closing.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{
    check_receipts_on, compute_shares, ensure_all_attended_on, ensure_users_exist_on, event_owner,
    load, load_on, paid_amount, snapshot, snapshot_on, write_body, SettlementIn,
};
use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::notifications::{notify_batch, BatchNotifItem, NotifMeta};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, now, require};

pub(super) async fn create(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(event_id): Path<i64>,
    Json(req): Json<SettlementIn>,
) -> AppResult<Json<Value>> {
    let (owner, _) = event_owner(&st, u.lang, event_id).await?;
    if owner != u.id || !u.can(Page::Settlements, Act::Create) {
        return Err(forbidden(
            u.lang,
            ErrorKey::OnlyOrganiserCanInitiateCostSplit,
        ));
    }
    let shares = compute_shares(u.lang, &req)?;
    let share_uids: Vec<i64> = shares.iter().map(|s| s.0).collect();
    let mut tx = st.db.begin().await?;
    let event_status: String =
        sqlx::query_scalar("SELECT status FROM events WHERE id=$1 FOR SHARE")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await?;
    if matches!(event_status.as_str(), "draft" | "cancelled") {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    check_receipts_on(&mut tx, &u, &req).await?;
    ensure_users_exist_on(&mut tx, u.lang, &share_uids).await?;
    let (sid,): (i64,) = sqlx::query_as(
        "INSERT INTO settlements (event_id, created_by, title, created_at) VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(event_id)
    .bind(u.id)
    .bind(req.title.trim())
    .bind(now())
    .fetch_one(&mut *tx)
    .await?;
    write_body(&mut tx, sid, &req, &shares).await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "settlement.create",
        "settlement",
        Some(sid),
        req.title.trim(),
    )
    .await;
    Ok(Json(json!({"id": sid})))
}

pub(super) async fn get_one(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    let mut v = snapshot(&st, &s).await?;
    let organiser = s.created_by == u.id || u.can(Page::AdminBills, Act::View);
    if organiser {
        v["can_manage"] = json!(s.created_by == u.id && u.can(Page::Settlements, Act::Update));
        v["can_delete"] = json!(s.created_by == u.id && u.can(Page::Settlements, Act::Delete));
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

#[derive(Deserialize)]
pub(super) struct SettlementListQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

pub(super) async fn list_mine_organised(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<SettlementListQuery>,
) -> AppResult<Json<Value>> {
    // 费用分摊 lists the bills a member organised; 账单查阅 (finance) lists every bill.
    let own = u.can(Page::Settlements, Act::View);
    let finance = u.can(Page::AdminBills, Act::View);
    if !own && !finance {
        crate::util::require_one(
            &u,
            &[
                (Page::Settlements, Act::View),
                (Page::AdminBills, Act::View),
            ],
        )?;
    }

    let scope_cond = if finance { "TRUE" } else { "s.created_by = $1" };

    if q.page.is_some() || q.limit.is_some() {
        let count_sql = format!("SELECT COUNT(*) FROM settlements s WHERE {scope_cond}");
        let mut count_q = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(&*count_sql));
        if !finance {
            count_q = count_q.bind(u.id);
        }
        let total = count_q.fetch_one(&st.db).await?;

        let page = q.page.unwrap_or(1).max(1);
        let limit = q.limit.unwrap_or(25).clamp(1, 100);
        let offset = (page - 1).saturating_mul(limit);
        let (lim_p, off_p) = if finance { (1, 2) } else { (2, 3) };

        let rows_sql = format!(
            "WITH page_ids AS (
                 SELECT s.id, s.title, s.event_id, s.status
                 FROM settlements s
                 WHERE {scope_cond}
                 ORDER BY s.id DESC
                 LIMIT ${lim_p} OFFSET ${off_p}
             )
             SELECT s.id, s.title, e.title, s.status,
                    COALESCE(sh.total_cents, 0)::BIGINT,
                    COALESCE(sh.confirmed_cnt, 0)::BIGINT,
                    COALESCE(sh.total_cnt, 0)::BIGINT
             FROM page_ids s
             JOIN events e ON e.id = s.event_id
             LEFT JOIN LATERAL (
                 SELECT COALESCE(SUM(amount_cents), 0)::BIGINT AS total_cents,
                        COUNT(*) FILTER (WHERE status = 'confirmed')::BIGINT AS confirmed_cnt,
                        COUNT(*)::BIGINT AS total_cnt
                 FROM settlement_shares WHERE settlement_id = s.id
             ) sh ON TRUE
             ORDER BY s.id DESC"
        );
        let mut rq = sqlx::query_as::<_, (i64, String, String, String, i64, i64, i64)>(
            sqlx::AssertSqlSafe(&*rows_sql),
        );
        if !finance {
            rq = rq.bind(u.id);
        }
        let rows = rq.bind(limit).bind(offset).fetch_all(&st.db).await?;

        let total_pages = if total == 0 {
            1
        } else {
            (total + limit - 1) / limit
        };
        return Ok(Json(json!({
            "items": rows.into_iter().map(|(id, t, et, s, total, conf, n)| json!({
                "id": id, "title": t, "event_title": et, "status": s, "total_cents": total, "confirmed": conf, "count": n,
            })).collect::<Vec<_>>(),
            "total": total,
            "page": page,
            "page_size": limit,
            "total_pages": total_pages,
        })));
    }

    let rows_sql = format!(
        "WITH page_ids AS (
             SELECT s.id, s.title, s.event_id, s.status
             FROM settlements s
             WHERE {scope_cond}
             ORDER BY s.id DESC
             LIMIT 200
         )
         SELECT s.id, s.title, e.title, s.status,
                COALESCE(sh.total_cents, 0)::BIGINT,
                COALESCE(sh.confirmed_cnt, 0)::BIGINT,
                COALESCE(sh.total_cnt, 0)::BIGINT
         FROM page_ids s
         JOIN events e ON e.id = s.event_id
         LEFT JOIN LATERAL (
             SELECT COALESCE(SUM(amount_cents), 0)::BIGINT AS total_cents,
                    COUNT(*) FILTER (WHERE status = 'confirmed')::BIGINT AS confirmed_cnt,
                    COUNT(*)::BIGINT AS total_cnt
             FROM settlement_shares WHERE settlement_id = s.id
         ) sh ON TRUE
         ORDER BY s.id DESC"
    );
    let mut rq = sqlx::query_as::<_, (i64, String, String, String, i64, i64, i64)>(
        sqlx::AssertSqlSafe(&*rows_sql),
    );
    if !finance {
        rq = rq.bind(u.id);
    }
    let rows = rq.fetch_all(&st.db).await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, et, s, total, conf, n)| json!({
        "id": id, "title": t, "event_title": et, "status": s, "total_cents": total, "confirmed": conf, "count": n,
    })).collect::<Vec<_>>())))
}

pub(super) async fn update(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SettlementIn>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id || !u.can(Page::Settlements, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanEditBill));
    }
    if s.status == "closed" {
        return Err(bad(u.lang, ErrorKey::BillClosedCannotEdit));
    }
    let shares = compute_shares(u.lang, &req)?;
    let share_uids: Vec<i64> = shares.iter().map(|s| s.0).collect();
    if s.status == "published" {
        if req.payee_account.trim().is_empty() {
            return Err(bad(u.lang, ErrorKey::PayeeDetailsRequired));
        }
        let reason = req
            .reason
            .clone()
            .filter(|r| !r.trim().is_empty())
            .ok_or_else(|| bad(u.lang, ErrorKey::RevisionReasonRequired))?;
        // 发布后若改金额，必须保留旧版并通知
        // One transaction for the saved old version, the version bump and the new body,
        // with the bill row locked so two revisions at once can't share a version number.
        let mut tx = st.db.begin().await?;
        let ev_status: String =
            sqlx::query_scalar("SELECT status FROM events WHERE id=$1 FOR SHARE")
                .bind(s.event_id)
                .fetch_one(&mut *tx)
                .await?;
        if matches!(ev_status.as_str(), "draft" | "cancelled") {
            return Err(bad(u.lang, ErrorKey::EventNotOpen));
        }
        sqlx::query("SELECT id FROM settlements WHERE id=$1 FOR UPDATE")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        // Re-read now that the bill is ours: another revision may have just finished.
        let s = load_on(&mut *tx, u.lang, id).await?;
        if s.status != "published" {
            return Err(bad(u.lang, ErrorKey::BillClosedCannotEdit));
        }
        check_receipts_on(&mut tx, &u, &req).await?;
        ensure_users_exist_on(&mut tx, u.lang, &share_uids).await?;
        // A revision must obey the same attendance rule as publishing: never charge
        // someone who was not recorded as attending the event.
        ensure_all_attended_on(&mut tx, u.lang, s.event_id, share_uids.clone()).await?;
        let before: Vec<(i64, i64, String, i64)> = sqlx::query_as(
            "SELECT user_id, amount_cents, status, paid_cents FROM settlement_shares WHERE settlement_id=$1",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        // A member who has paid, or sent proof, keeps their record: removing them would
        // delete the proof and the only sign that a refund is owed.
        let paid_removed = before
            .iter()
            .filter(|(uid, amt, st, paid)| {
                paid_amount(*amt, st, *paid) > 0 && !shares.iter().any(|x| x.0 == *uid)
            })
            .count() as i64;
        if paid_removed > 0 {
            return Err(bad(
                u.lang,
                ErrorKey::PaidMembersCannotBeRemoved(paid_removed),
            ));
        }
        let snap = snapshot_on(&mut tx, &s).await?;
        sqlx::query("INSERT INTO settlement_revisions (settlement_id, version, snapshot, reason, created_by, created_at) VALUES ($1,$2,$3::jsonb,$4,$5,$6)")
            .bind(id)
            .bind(s.version)
            .bind(snap.to_string())
            .bind(&reason)
            .bind(u.id)
            .bind(now())
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE settlements SET version=version+1, updated_at=$1 WHERE id=$2")
            .bind(now())
            .bind(id)
            .execute(&mut *tx)
            .await?;
        write_body(&mut tx, id, &req, &shares).await?;
        tx.commit().await?;
        let before: Vec<(i64, i64)> = before
            .into_iter()
            .map(|(uid, amt, _, _)| (uid, amt))
            .collect();
        let mut batch = Vec::new();
        for (uid, old) in &before {
            match shares.iter().find(|x| x.0 == *uid) {
                Some((_, new)) if new != old => batch.push(BatchNotifItem {
                    user_id: *uid,
                    title_zh: "费用分摊已更新".into(),
                    title_en: "Cost split updated".into(),
                    body_zh: format!(
                        "{}：{} → {}（{}）",
                        s.title,
                        fmt_rm(*old),
                        fmt_rm(*new),
                        reason
                    ),
                    body_en: format!(
                        "{}: {} → {} ({})",
                        s.title,
                        fmt_rm(*old),
                        fmt_rm(*new),
                        reason
                    ),
                    link: "/costs".into(),
                    meta: NotifMeta {
                        category: "cost",
                        event_type: "settlement_updated".into(),
                        priority: "normal",
                        entity_type: "settlement",
                        entity_id: Some(id),
                        tag: format!("settlement-{id}"),
                    },
                }),
                None => batch.push(BatchNotifItem {
                    user_id: *uid,
                    title_zh: "您已从分摊名单移除".into(),
                    title_en: "Removed from the cost split".into(),
                    body_zh: format!("{}（{}）", s.title, reason),
                    body_en: format!("{} ({})", s.title, reason),
                    link: "/costs".into(),
                    meta: NotifMeta {
                        category: "cost",
                        event_type: "settlement_removed".into(),
                        priority: "normal",
                        entity_type: "settlement",
                        entity_id: Some(id),
                        tag: format!("settlement-{id}"),
                    },
                }),
                _ => {}
            }
        }
        for (uid, amt) in &shares {
            if !before.iter().any(|b| b.0 == *uid) {
                batch.push(BatchNotifItem {
                    user_id: *uid,
                    title_zh: "新的费用分摊".into(),
                    title_en: "New cost split".into(),
                    body_zh: format!("{} · 您应付 {}", s.title, fmt_rm(*amt)),
                    body_en: format!("{} · You owe {}", s.title, fmt_rm(*amt)),
                    link: "/costs".into(),
                    meta: NotifMeta {
                        category: "cost",
                        event_type: "settlement_added".into(),
                        priority: "normal",
                        entity_type: "settlement",
                        entity_id: Some(id),
                        tag: format!("settlement-{id}"),
                    },
                });
            }
        }
        notify_batch(&st.db, batch).await;
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
        // Hold the bill and look again: a publish that got in first means these changes
        // would land on a published bill with no revision kept and no attendance check.
        let mut tx = st.db.begin().await?;
        let ev_status: String =
            sqlx::query_scalar("SELECT status FROM events WHERE id=$1 FOR SHARE")
                .bind(s.event_id)
                .fetch_one(&mut *tx)
                .await?;
        if matches!(ev_status.as_str(), "draft" | "cancelled") {
            return Err(bad(u.lang, ErrorKey::EventNotOpen));
        }
        let (status,): (String,) =
            sqlx::query_as("SELECT status FROM settlements WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if status != "draft" {
            return Err(conflict(u.lang, ErrorKey::SettlementAlreadyPublished));
        }
        check_receipts_on(&mut tx, &u, &req).await?;
        ensure_users_exist_on(&mut tx, u.lang, &share_uids).await?;
        write_body(&mut tx, id, &req, &shares).await?;
        tx.commit().await?;
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

pub(super) async fn delete_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id || s.status != "draft" {
        return Err(bad(u.lang, ErrorKey::OnlyDraftsCanBeDeleted));
    }
    require(&u, Page::Settlements, Act::Delete)?;
    let deleted = sqlx::query("DELETE FROM settlements WHERE id=$1 AND status='draft'")
        .bind(id)
        .execute(&st.db)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::SettlementAlreadyPublished));
    }
    audit(
        &st.db,
        Some(u.id),
        "settlement.delete",
        "settlement",
        Some(id),
        format!("draft: {}", s.title),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn publish(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    // Hold the bill while checking and publishing it, so a draft edit saved at the same
    // moment either lands first (and is what gets checked) or waits and is refused, and
    // two presses of publish send the notices once.
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM settlements WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let s = load_on(&mut *tx, u.lang, id).await?;
    let ev_status: String = sqlx::query_scalar("SELECT status FROM events WHERE id=$1 FOR SHARE")
        .bind(s.event_id)
        .fetch_one(&mut *tx)
        .await?;
    if matches!(ev_status.as_str(), "draft" | "cancelled") {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    if s.created_by != u.id || !u.can(Page::Settlements, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanPublish));
    }
    if s.status != "draft" {
        return Err(bad(u.lang, ErrorKey::SettlementAlreadyPublished));
    }
    if s.payee_account.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::PayeeDetailsRequired));
    }
    // 分摊前检查出席: only members whose attendance is confirmed may be charged.
    let shares: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, amount_cents FROM settlement_shares WHERE settlement_id=$1",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    ensure_all_attended_on(
        &mut tx,
        u.lang,
        s.event_id,
        shares.iter().map(|s| s.0).collect(),
    )
    .await?;
    sqlx::query("UPDATE settlements SET status='published', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
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

pub(super) async fn close(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id || !u.can(Page::Settlements, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::OnlyOrganiserCanCloseBill));
    }
    // Hold the bill, as a revision, proof or verification does, before counting what is
    // still open. Otherwise a revision landing between the count and the close leaves a
    // closed bill with a share that can no longer be resolved.
    let mut tx = st.db.begin().await?;
    let (status,): (String,) =
        sqlx::query_as("SELECT status FROM settlements WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if status == "closed" {
        return Err(bad(u.lang, ErrorKey::BillClosedCannotEdit));
    }
    if status != "published" {
        return Err(bad(u.lang, ErrorKey::ProofNotNeededNow));
    }
    let (open,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=$1 AND status != 'confirmed'",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if open > 0 {
        return Err(bad(u.lang, ErrorKey::OpenPaymentsRemain(open)));
    }
    sqlx::query("UPDATE settlements SET status='closed', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
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

pub(super) async fn revisions(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let s = load(&st, u.lang, id).await?;
    if s.created_by != u.id && !u.can(Page::AdminBills, Act::View) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    let rows: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT r.version, r.snapshot::text, r.reason, u.name, r.created_at FROM settlement_revisions r JOIN users u ON u.id=r.created_by WHERE r.settlement_id=$1 ORDER BY r.version DESC",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(v, snap, reason, by, at)| json!({
        "version": v, "snapshot": serde_json::from_str::<Value>(&snap).unwrap_or(Value::Null), "reason": reason, "by": by, "created_at": at,
    })).collect::<Vec<_>>())))
}
