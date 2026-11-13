//! 管理员: dashboard, members & roles, invitations, groups, areas, recovery approvals, audit log.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{is_privileged, parse_roles, CurrentUser};
use crate::error::{bad, forbidden, not_found, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, in_days, now, random_token, require_any};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/dashboard", get(dashboard))
        .route("/admin/users", get(list_users))
        .route("/admin/users/{id}", put(update_user))
        .route("/admin/invites", get(list_invites).post(create_invite))
        .route("/admin/invites/{id}", delete(revoke_invite))
        .route("/admin/groups", post(create_group))
        .route("/admin/groups/{id}", put(update_group).delete(delete_group))
        .route(
            "/admin/groups/{id}/members",
            get(group_members).put(set_group_members),
        )
        .route("/admin/areas", post(create_area))
        .route("/admin/areas/{id}", put(update_area))
        .route("/admin/recovery", get(list_recovery))
        .route("/admin/recovery/{id}/approve", post(approve_recovery))
        .route("/admin/recovery/{id}/reject", post(reject_recovery))
        .route("/admin/audit", get(audit_log))
}

async fn dashboard(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require_any(&u, &["transport"])?;
    let today = now()[..10].to_string();
    let count = |sql: &'static str| {
        let db = st.db.clone();
        let today = today.clone();
        async move {
            let (n,): (i64,) = sqlx::query_as(sql).bind(today).fetch_one(&db).await?;
            Ok::<i64, AppError>(n)
        }
    };
    let today_total = count(
        "SELECT COUNT(*) FROM rides WHERE substr(pickup_at,1,10)=$1 AND status NOT IN ('cancelled')",
    )
    .await?;
    let today_confirmed = count("SELECT COUNT(*) FROM rides WHERE substr(pickup_at,1,10)=$1 AND status IN ('confirmed','in_progress','completed')").await?;
    let today_changes = count("SELECT COUNT(*) FROM rides WHERE substr(updated_at,1,10)=$1 AND (needs_attention=1 OR status IN ('cancelled','no_show','alt_transport'))").await?;
    let pending_approval =
        count("SELECT COUNT(*) FROM rides WHERE status='pending_approval' AND $1 != ''").await?;
    let no_driver =
        count("SELECT COUNT(*) FROM rides WHERE status='pending_driver' AND pickup_at >= $1")
            .await?;
    let attention = count("SELECT COUNT(*) FROM rides WHERE needs_attention=1 AND status NOT IN ('cancelled','completed') AND $1 != ''").await?;
    let (live,): (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress'")
            .fetch_one(&st.db)
            .await?;
    let (open_help,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM help_tickets WHERE status='open'")
            .fetch_one(&st.db)
            .await?;
    let (pending_reimb,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM reimbursements WHERE status='pending'")
            .fetch_one(&st.db)
            .await?;
    let (members,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE status='active'")
        .fetch_one(&st.db)
        .await?;
    let (recovery,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM recovery_requests WHERE status IN ('pending','awaiting_second')",
    )
    .fetch_one(&st.db)
    .await?;
    Ok(Json(json!({
        "today": today, "today_total": today_total, "today_confirmed": today_confirmed, "today_changes": today_changes,
        "pending_approval": pending_approval, "no_driver": no_driver, "needs_attention": attention,
        "live_drivers": live, "max_live_drivers": st.cfg.max_live_drivers, "open_help": open_help,
        "pending_reimbursements": pending_reimb, "members": members, "pending_recovery": recovery,
    })))
}

#[derive(Deserialize)]
struct UserQuery {
    q: Option<String>,
    role: Option<String>,
}

/// Member directory for staff only (会员资料不会成为任何人都能浏览的公开通讯录).
/// Leaders may search names (no contact details) to record walk-in attendance.
async fn list_users(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<UserQuery>,
) -> AppResult<Json<Value>> {
    let staff = require_any(&u, &["admin", "transport"]).is_ok();
    if !staff && !u.has("leader") {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let like = format!("%{}%", q.q.unwrap_or_default().trim());
    let rows: Vec<(i64, String, String, String, String, String, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT u.id, u.name, u.email, u.phone, u.roles, u.status, u.created_at, (SELECT COUNT(*) FROM passkeys p WHERE p.user_id=u.id),
            (SELECT string_agg(g.name, '、') FROM group_members gm JOIN groups g ON g.id=gm.group_id WHERE gm.user_id=u.id)
         FROM users u WHERE (u.name LIKE $1 OR u.email LIKE $2 OR u.phone LIKE $3) ORDER BY u.name LIMIT 500",
    )
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .fetch_all(&st.db)
    .await?;
    let out: Vec<Value> = rows
        .into_iter()
        .filter(|r| q.role.as_deref().map(|role| r.4.split(',').any(|x| x == role)).unwrap_or(true))
        .map(|(id, name, email, phone, roles, status, created, pk, groups)| {
            if staff {
                json!({"id": id, "name": name, "email": email, "phone": phone, "roles": parse_roles(&roles), "status": status,
                       "created_at": created, "passkeys": pk, "groups": groups.unwrap_or_default()})
            } else {
                json!({"id": id, "name": name})
            }
        })
        .collect();
    Ok(Json(json!(out)))
}

#[derive(Deserialize)]
struct UserUpdate {
    roles: Vec<String>,
    status: String,
    group_ids: Option<Vec<i64>>,
}

async fn update_user(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<UserUpdate>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    if !matches!(req.status.as_str(), "active" | "disabled") {
        return Err(bad(u.lang, ErrorKey::InvalidStatus));
    }
    let (old_roles, old_status): (String, String) =
        sqlx::query_as("SELECT roles, status FROM users WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    let new_roles = parse_roles(&req.roles.join(","));
    if id == u.id {
        if req.status != "active" || !new_roles.iter().any(|r| r == "admin") {
            return Err(bad(u.lang, ErrorKey::CannotDisableSelf));
        }
        // Finance sees every receipt and payment record, so an admin cannot
        // grant it to themselves; another admin must decide.
        let already_finance = parse_roles(&old_roles).iter().any(|r| r == "finance");
        if !already_finance && new_roles.iter().any(|r| r == "finance") {
            return Err(forbidden(u.lang, ErrorKey::CannotGrantSelfFinance));
        }
    }
    let roles = new_roles.join(",");
    sqlx::query("UPDATE users SET roles=$1, status=$2 WHERE id=$3")
        .bind(&roles)
        .bind(&req.status)
        .bind(id)
        .execute(&st.db)
        .await?;
    if req.status == "disabled" {
        sqlx::query("DELETE FROM sessions WHERE user_id=$1")
            .bind(id)
            .execute(&st.db)
            .await?;
    }
    if let Some(gids) = req.group_ids {
        sqlx::query("DELETE FROM group_members WHERE user_id=$1")
            .bind(id)
            .execute(&st.db)
            .await?;
        for g in gids {
            sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
                .bind(g)
                .bind(id)
                .execute(&st.db)
                .await?;
        }
    }
    audit(
        &st.db,
        Some(u.id),
        "user.update",
        "user",
        Some(id),
        format!(
            "roles {old_roles} → {roles}; status {old_status} → {}",
            req.status
        ),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn create_invite(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    // The invited person types their own name and email. The link carries neither.
    let token = random_token(32);
    let (id,): (i64,) = sqlx::query_as("INSERT INTO invites (token, kind, email, name, roles, group_ids, created_by, expires_at, created_at) VALUES ($1, 'invite', '', '', '', '[]', $2,$3,$4) RETURNING id")
        .bind(&token)
        .bind(u.id)
        .bind(in_days(3))
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "invite.create",
        "invite",
        Some(id),
        "open",
    )
    .await;
    Ok(Json(
        json!({"id": id, "token": token, "url": format!("{}/invite/{}", st.cfg.public_url, token)}),
    ))
}

async fn list_invites(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    let rows: Vec<(i64, String, String, String, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, token, kind, email, name, roles, expires_at, used_at, created_at FROM invites ORDER BY id DESC LIMIT 200",
    )
    .fetch_all(&st.db)
    .await?;
    let n = now();
    // A recovery link signs its holder in as that member, so it is shown once, to the
    // admin who approved it (`approve_recovery`), and never listed again: otherwise any
    // other admin, including the one who only flagged a privileged recovery, could use it.
    Ok(Json(json!(rows.into_iter().map(|(id, token, kind, email, name, roles, exp, used, c)| json!({
        "id": id, "kind": kind, "email": email, "name": name, "roles": parse_roles(&roles), "expires_at": exp, "used_at": used, "created_at": c,
        "status": if used.is_some() { "used" } else if exp < n { "expired" } else { "open" },
        "url": if kind != "recovery" && used.is_none() && exp >= n { Some(format!("{}/invite/{}", st.cfg.public_url, token)) } else { None },
    })).collect::<Vec<_>>())))
}

async fn revoke_invite(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    sqlx::query("UPDATE invites SET expires_at=$1 WHERE id=$2 AND used_at IS NULL")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "invite.revoke", "invite", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct GroupIn {
    name: String,
    description: Option<String>,
}

async fn create_group(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<GroupIn>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    if req.name.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::GroupNameRequired));
    }
    let (id,): (i64,) =
        sqlx::query_as("INSERT INTO groups (name, description) VALUES ($1,$2) RETURNING id")
            .bind(req.name.trim())
            .bind(req.description.unwrap_or_default())
            .fetch_one(&st.db)
            .await
            .map_err(|_| bad(u.lang, ErrorKey::GroupNameExists))?;
    audit(
        &st.db,
        Some(u.id),
        "group.create",
        "group",
        Some(id),
        req.name.trim(),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

async fn update_group(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<GroupIn>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    sqlx::query("UPDATE groups SET name=$1, description=$2 WHERE id=$3")
        .bind(req.name.trim())
        .bind(req.description.unwrap_or_default())
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "group.update",
        "group",
        Some(id),
        req.name.trim(),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn delete_group(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    let (is_all,): (i64,) = sqlx::query_as("SELECT is_all FROM groups WHERE id=$1")
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;
    if is_all == 1 {
        return Err(bad(u.lang, ErrorKey::CannotDeleteAllMembersGroup));
    }
    sqlx::query("DELETE FROM groups WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "group.delete", "group", Some(id), "").await;
    Ok(Json(json!({"ok": true})))
}

async fn group_members(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT u.id, u.name FROM group_members m JOIN users u ON u.id=m.user_id WHERE m.group_id=$1 ORDER BY u.name")
        .bind(id)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, n)| json!({"id": id, "name": n}))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct MembersIn {
    user_ids: Vec<i64>,
}

async fn set_group_members(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<MembersIn>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM group_members WHERE group_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    for uid in &req.user_ids {
        sqlx::query(
            "INSERT INTO group_members (group_id, user_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "group.members",
        "group",
        Some(id),
        format!("{} members", req.user_ids.len()),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct AreaIn {
    name: String,
    active: Option<bool>,
}

async fn create_area(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<AreaIn>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["transport"])?;
    if req.name.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::AreaNameRequired));
    }
    let (id,): (i64,) =
        sqlx::query_as("INSERT INTO areas (name, active) VALUES ($1, 1) RETURNING id")
            .bind(req.name.trim())
            .fetch_one(&st.db)
            .await
            .map_err(|_| bad(u.lang, ErrorKey::AreaExists))?;
    audit(
        &st.db,
        Some(u.id),
        "area.create",
        "area",
        Some(id),
        req.name.trim(),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

async fn update_area(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<AreaIn>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["transport"])?;
    sqlx::query("UPDATE areas SET name=$1, active=$2 WHERE id=$3")
        .bind(req.name.trim())
        .bind(req.active.unwrap_or(true) as i64)
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "area.update",
        "area",
        Some(id),
        req.name.trim(),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn list_recovery(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    let rows: Vec<(i64, String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT r.id, r.email, r.reason, r.status, r.created_at, u.name FROM recovery_requests r LEFT JOIN users u ON u.email = r.email ORDER BY r.status IN ('pending','awaiting_second') DESC, r.id DESC LIMIT 100",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, e, reason, s, c, name)| json!({"id": id, "email": e, "reason": reason, "status": s, "created_at": c, "name": name})).collect::<Vec<_>>())))
}

/// 已审核的程序恢复账号: admin verifies identity offline, then issues a one-time 24h recovery link.
async fn approve_recovery(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    let (email, status, resolved_by): (String, String, Option<i64>) =
        sqlx::query_as("SELECT email, status, resolved_by FROM recovery_requests WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    if !matches!(status.as_str(), "pending" | "awaiting_second") {
        return Err(bad(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    let (uid, roles): (i64, String) = sqlx::query_as("SELECT id, roles FROM users WHERE email=$1")
        .bind(&email)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;
    // Recovering a privileged account needs two different admins: the first
    // only flags the request, the second confirms and receives the link.
    if status == "pending" && is_privileged(&roles) {
        sqlx::query(
            "UPDATE recovery_requests SET status='awaiting_second', resolved_by=$1 WHERE id=$2",
        )
        .bind(u.id)
        .bind(id)
        .execute(&st.db)
        .await?;
        audit(
            &st.db,
            Some(u.id),
            "recovery.await_second",
            "user",
            Some(uid),
            &email,
        )
        .await;
        return Ok(Json(json!({"awaiting_second": true})));
    }
    if status == "awaiting_second" && resolved_by == Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::SecondAdminRequired));
    }
    let token = random_token(32);
    sqlx::query("INSERT INTO invites (token, kind, email, user_id, created_by, expires_at, created_at) VALUES ($1, 'recovery', $2, $3, $4, $5, $6)")
        .bind(&token)
        .bind(&email)
        .bind(uid)
        .bind(u.id)
        .bind(in_days(1))
        .bind(now())
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE recovery_requests SET status='approved', resolved_by=$1 WHERE id=$2")
        .bind(u.id)
        .bind(id)
        .execute(&st.db)
        .await?;
    // The old phone is presumed lost: revoke its sessions, passkeys and authenticator.
    // The member registers a new passkey with the one-time link.
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(uid)
        .execute(&st.db)
        .await?;
    sqlx::query("DELETE FROM passkeys WHERE user_id=$1")
        .bind(uid)
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE users SET totp_enabled=0, totp_secret=NULL, totp_last_step=0 WHERE id=$1")
        .bind(uid)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "recovery.approve",
        "user",
        Some(uid),
        &email,
    )
    .await;
    Ok(Json(
        json!({"url": format!("{}/invite/{}", st.cfg.public_url, token)}),
    ))
}

async fn reject_recovery(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin"])?;
    sqlx::query("UPDATE recovery_requests SET status='rejected', resolved_by=$1 WHERE id=$2 AND status IN ('pending','awaiting_second')").bind(u.id).bind(id).execute(&st.db).await?;
    audit(
        &st.db,
        Some(u.id),
        "recovery.reject",
        "recovery",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct AuditQuery {
    entity: Option<String>,
    entity_id: Option<i64>,
}

/// 管理员可查看谁在何时修改了安排。
async fn audit_log(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AuditQuery>,
) -> AppResult<Json<Value>> {
    require_any(&u, &["admin", "transport"])?;
    let rows: Vec<(i64, Option<String>, String, String, Option<i64>, String, String)> = sqlx::query_as(
        "SELECT a.id, u.name, a.action, a.entity, a.entity_id, a.detail, a.created_at FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id
         WHERE ($1 IS NULL OR a.entity = $2) AND ($3 IS NULL OR a.entity_id = $4) ORDER BY a.id DESC LIMIT 300",
    )
    .bind(&q.entity)
    .bind(&q.entity)
    .bind(q.entity_id)
    .bind(q.entity_id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, actor, action, entity, eid, detail, at)| json!({
        "id": id, "actor": actor, "action": action, "entity": entity, "entity_id": eid, "detail": detail, "created_at": at,
    })).collect::<Vec<_>>())))
}
