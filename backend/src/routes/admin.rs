//! 管理员: dashboard, members, invitations, groups, areas, recovery approvals, audit log.
//! Roles themselves are edited in `routes::roles`.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, forbidden, not_found, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{self, parse_roles, Act, Page};
use crate::state::AppState;
use crate::util::{audit, in_days, now, random_token, require, require_one};

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
        .route("/admin/audit/export", get(audit_export))
        .route(
            "/admin/settings/rates",
            get(get_reimbursement_rates).put(update_reimbursement_rates),
        )
        .route(
            "/admin/settings/system",
            get(get_system_settings).put(update_system_settings),
        )
}

async fn dashboard(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminDashboard, Act::View)?;
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
    let max_live_drivers: i64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM system_settings WHERE key='max_live_drivers'",
    )
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten()
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or(st.cfg.max_live_drivers);
    Ok(Json(json!({
        "today": today, "today_total": today_total, "today_confirmed": today_confirmed, "today_changes": today_changes,
        "pending_approval": pending_approval, "no_driver": no_driver, "needs_attention": attention,
        "live_drivers": live, "max_live_drivers": max_live_drivers, "open_help": open_help,
        "pending_reimbursements": pending_reimb, "members": members, "pending_recovery": recovery,
    })))
}

#[derive(Deserialize)]
struct UserQuery {
    q: Option<String>,
    role: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}

/// Member directory for 成员 · view only (会员资料不会成为任何人都能浏览的公开通讯录).
/// Whoever records attendance, fills a group or opens member reports may search names
/// (no contact details) to pick a member.
async fn list_users(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<UserQuery>,
) -> AppResult<Json<Value>> {
    let staff = u.can(Page::AdminMembers, Act::View);
    if !staff {
        require_one(
            &u,
            &[
                (Page::AdminMembers, Act::View),
                (Page::Events, Act::Update),
                (Page::AdminEvents, Act::Update),
                (Page::AdminGroups, Act::Update),
                (Page::AdminReports, Act::View),
            ],
        )?;
    }
    let like = format!("%{}%", q.q.unwrap_or_default().trim());

    if q.page.is_some() || q.limit.is_some() {
        let mut conds = vec!["(u.name LIKE $1 OR u.email LIKE $2 OR u.phone LIKE $3)".to_string()];
        if q.role.is_some() {
            conds.push("(',' || u.roles || ',') LIKE ('%,' || $4 || ',%')".to_string());
        }
        let count_sql = format!("SELECT COUNT(*) FROM users u WHERE {}", conds.join(" AND "));
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql)
            .bind(&like)
            .bind(&like)
            .bind(&like);
        if let Some(r) = &q.role {
            count_query = count_query.bind(r);
        }
        let (total,) = count_query.fetch_one(&st.db).await?;

        let page = q.page.unwrap_or(1).max(1);
        let limit = q.limit.unwrap_or(50).clamp(1, 200);
        let offset = (page - 1) * limit;

        let n = if q.role.is_some() { 5 } else { 4 };
        let sql = format!(
            "SELECT u.id, u.name, u.email, u.phone, u.roles, u.status, u.created_at, (SELECT COUNT(*) FROM passkeys p WHERE p.user_id=u.id),
                (SELECT string_agg(g.name, '、') FROM group_members gm JOIN groups g ON g.id=gm.group_id WHERE gm.user_id=u.id)
             FROM users u WHERE {} ORDER BY u.name LIMIT ${n} OFFSET ${}",
            conds.join(" AND "),
            n + 1
        );
        let mut query = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                String,
                String,
                String,
                String,
                i64,
                Option<String>,
            ),
        >(&sql)
        .bind(&like)
        .bind(&like)
        .bind(&like);
        if let Some(r) = &q.role {
            query = query.bind(r);
        }
        query = query.bind(limit).bind(offset);
        let rows = query.fetch_all(&st.db).await?;
        let out: Vec<Value> = rows
            .into_iter()
            .map(|(id, name, email, phone, roles, status, created, pk, groups)| {
                if staff {
                    json!({"id": id, "name": name, "email": email, "phone": phone, "roles": parse_roles(&roles), "status": status,
                           "created_at": created, "passkeys": pk, "groups": groups.unwrap_or_default()})
                } else {
                    json!({"id": id, "name": name})
                }
            })
            .collect();
        let total_pages = if total == 0 {
            1
        } else {
            (total + limit - 1) / limit
        };
        return Ok(Json(json!({
            "items": out,
            "total": total,
            "page": page,
            "page_size": limit,
            "total_pages": total_pages,
        })));
    }

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
    require(&u, Page::AdminMembers, Act::Update)?;
    if !matches!(req.status.as_str(), "active" | "disabled") {
        return Err(bad(u.lang, ErrorKey::InvalidStatus));
    }
    let (old_roles, old_status): (String, String) =
        sqlx::query_as("SELECT roles, status FROM users WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v.dedup();
        v
    };
    let new_roles = sorted(parse_roles(&req.roles.join(",")));
    let roles_changed = new_roles != sorted(parse_roles(&old_roles));
    if roles_changed {
        // Giving or taking a role decides what a member may do, which is the same
        // authority as editing the roles themselves.
        require(&u, Page::AdminRoles, Act::Update)?;
        let (known,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roles WHERE key = ANY($1)")
            .bind(&new_roles)
            .fetch_one(&st.db)
            .await?;
        if known != new_roles.len() as i64 {
            return Err(bad(u.lang, ErrorKey::UnknownRole));
        }
    }
    if id == u.id {
        if req.status != "active" {
            return Err(bad(u.lang, ErrorKey::CannotDisableSelf));
        }
        if roles_changed {
            let after = perms::for_roles(&st.db, &new_roles).await?;
            // Nobody removes their own right to manage roles, so someone always holds it.
            if !after.has(Page::AdminRoles, Act::Update) {
                return Err(bad(u.lang, ErrorKey::CannotDisableSelf));
            }
            // Finance sees every receipt and payment record, so nobody grants it to
            // themselves; another administrator must decide.
            if after.new_finance(&u.perms) {
                return Err(forbidden(u.lang, ErrorKey::CannotGrantSelfFinance));
            }
        }
    }
    let roles = if roles_changed {
        new_roles.join(",")
    } else {
        old_roles.clone()
    };
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
    let mut groups_note = String::new();
    if let Some(mut gids) = req.group_ids {
        let old: Vec<i64> = sqlx::query_scalar(
            "SELECT group_id FROM group_members WHERE user_id=$1 ORDER BY group_id",
        )
        .bind(id)
        .fetch_all(&st.db)
        .await?;
        sqlx::query("DELETE FROM group_members WHERE user_id=$1")
            .bind(id)
            .execute(&st.db)
            .await?;
        for g in &gids {
            sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
                .bind(g)
                .bind(id)
                .execute(&st.db)
                .await?;
        }
        gids.sort();
        gids.dedup();
        if gids != old {
            groups_note = format!("; groups {old:?} → {gids:?}");
        }
    }
    audit(
        &st.db,
        Some(u.id),
        "user.update",
        "user",
        Some(id),
        format!(
            "roles {old_roles} → {roles}; status {old_status} → {}{groups_note}",
            req.status
        ),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn create_invite(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminInvites, Act::Create)?;
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
    require(&u, Page::AdminInvites, Act::View)?;
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
    require(&u, Page::AdminInvites, Act::Delete)?;
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
    require(&u, Page::AdminGroups, Act::Create)?;
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
    require(&u, Page::AdminGroups, Act::Update)?;
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
    require(&u, Page::AdminGroups, Act::Delete)?;
    let (is_all,): (i64,) = sqlx::query_as("SELECT is_all FROM groups WHERE id=$1")
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;
    if is_all == 1 {
        return Err(bad(u.lang, ErrorKey::CannotDeleteAllMembersGroup));
    }
    let (name,): (String,) = sqlx::query_as("DELETE FROM groups WHERE id=$1 RETURNING name")
        .bind(id)
        .fetch_one(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "group.delete", "group", Some(id), name).await;
    Ok(Json(json!({"ok": true})))
}

async fn group_members(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminGroups, Act::View)?;
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
    require(&u, Page::AdminGroups, Act::Update)?;
    let mut tx = st.db.begin().await?;
    let old: Vec<i64> = sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id=$1")
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
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
    let added: Vec<i64> = req
        .user_ids
        .iter()
        .copied()
        .filter(|x| !old.contains(x))
        .collect();
    let removed: Vec<i64> = old
        .iter()
        .copied()
        .filter(|x| !req.user_ids.contains(x))
        .collect();
    audit(
        &st.db,
        Some(u.id),
        "group.members",
        "group",
        Some(id),
        format!(
            "{} members; added {added:?}; removed {removed:?}",
            req.user_ids.len()
        ),
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
    require(&u, Page::AdminAreas, Act::Create)?;
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
    require(&u, Page::AdminAreas, Act::Update)?;
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
        format!(
            "{}; {}",
            req.name.trim(),
            if req.active.unwrap_or(true) {
                "active"
            } else {
                "inactive"
            }
        ),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn list_recovery(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRecovery, Act::View)?;
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
    require(&u, Page::AdminRecovery, Act::Update)?;
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
    if status == "pending" && perms::is_privileged(&st.db, &roles).await? {
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
    // Only the hash is stored (see `routes::auth::INVITE_BY_TOKEN`); the link is returned once.
    let token = random_token(32);
    sqlx::query("INSERT INTO invites (token, kind, email, user_id, created_by, expires_at, created_at) VALUES ($1, 'recovery', $2, $3, $4, $5, $6)")
        .bind(hash_token(&token))
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
    sqlx::query("UPDATE users SET totp_enabled=0, totp_secret=NULL, totp_last_step=0, totp_backup_codes=NULL WHERE id=$1")
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
    require(&u, Page::AdminRecovery, Act::Update)?;
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
    action: Option<String>,
    actor_id: Option<i64>,
    /// Text to find in the actor's name, the action, the object or the detail.
    q: Option<String>,
    /// First and last day to include, `YYYY-MM-DD`.
    from: Option<String>,
    to: Option<String>,
    /// Entries older than this id: the `next_before` of the previous page.
    before: Option<i64>,
    limit: Option<i64>,
}

/// Entries per page of the audit log unless the caller asks for fewer or more.
const AUDIT_PAGE: i64 = 100;
const AUDIT_PAGE_MAX: i64 = 200;

/// 管理员可查看谁在何时修改了安排。 Newest first, one page at a time: `next_before` is
/// the cursor for the next page and is null on the last one, so the whole retained log
/// can be read, not just its newest entries.
async fn audit_log(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AuditQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAudit, Act::View)?;
    let day = |s: &Option<String>| -> AppResult<Option<NaiveDate>> {
        match s.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(Some)
                .map_err(|_| bad(u.lang, ErrorKey::InvalidDate)),
        }
    };
    // `created_at` is ISO text, so a date bounds it as a plain string comparison.
    let from = day(&q.from)?.map(|d| d.to_string());
    let until = day(&q.to)?
        .and_then(|d| d.succ_opt())
        .map(|d| d.to_string());
    // The search text is matched literally: `%` and `_` are not wildcards.
    let like =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                format!(
                    "%{}%",
                    s.replace('\\', "\\\\")
                        .replace('%', "\\%")
                        .replace('_', "\\_")
                )
            });
    let action = q.action.filter(|a| !a.is_empty());
    let entity = q.entity.filter(|e| !e.is_empty());
    let limit = q.limit.unwrap_or(AUDIT_PAGE).clamp(1, AUDIT_PAGE_MAX);
    // One row more than the page shows whether another page follows.
    let mut rows: Vec<(i64, Option<i64>, Option<String>, String, String, Option<i64>, String, String)> = sqlx::query_as(
        "SELECT a.id, a.actor_id, u.name, a.action, a.entity, a.entity_id, a.detail, a.created_at FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id
         WHERE ($1::text IS NULL OR a.entity = $1)
           AND ($2::bigint IS NULL OR a.entity_id = $2)
           AND ($3::text IS NULL OR a.action ILIKE $3 OR a.entity ILIKE $3 OR a.detail ILIKE $3 OR u.name ILIKE $3)
           AND ($4::text IS NULL OR a.created_at >= $4)
           AND ($5::text IS NULL OR a.created_at < $5)
           AND ($6::bigint IS NULL OR a.id < $6)
           AND ($7::text IS NULL OR a.action = $7)
           AND ($8::bigint IS NULL OR a.actor_id = $8)
         ORDER BY a.id DESC LIMIT $9",
    )
    .bind(&entity)
    .bind(q.entity_id)
    .bind(&like)
    .bind(&from)
    .bind(&until)
    .bind(q.before)
    .bind(&action)
    .bind(q.actor_id)
    .bind(limit + 1)
    .fetch_all(&st.db)
    .await?;
    let more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    let next_before = rows.last().filter(|_| more).map(|r| r.0);
    let retention_days: i64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM system_settings WHERE key='audit_retention_days'",
    )
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten()
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or(st.cfg.audit_retention_days);

    Ok(Json(json!({
        "items": rows.into_iter().map(|(id, aid, actor, action, entity, eid, detail, at)| json!({
            "id": id, "actor_id": aid, "actor": actor, "action": action, "entity": entity, "entity_id": eid, "detail": detail, "created_at": at,
        })).collect::<Vec<_>>(),
        "next_before": next_before,
        "retention_days": retention_days,
    })))
}

#[derive(Deserialize)]
struct AuditExportQuery {
    entity: Option<String>,
    entity_id: Option<i64>,
    action: Option<String>,
    actor_id: Option<i64>,
    q: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<i64>,
}

fn escape_csv_field(s: &str) -> String {
    // Prefix single quote on formula triggers (=, +, -, @, \t, \r) to prevent spreadsheet injection.
    let sanitized = if s.starts_with('=')
        || s.starts_with('+')
        || s.starts_with('-')
        || s.starts_with('@')
        || s.starts_with('\t')
        || s.starts_with('\r')
    {
        format!("'{s}")
    } else {
        s.to_string()
    };

    if sanitized.contains('"')
        || sanitized.contains(',')
        || sanitized.contains('\n')
        || sanitized.contains('\r')
    {
        format!("\"{}\"", sanitized.replace('"', "\"\""))
    } else {
        sanitized
    }
}

const AUDIT_EXPORT_MAX: i64 = 50_000;

/// Stream audit logs directly to client as CSV without buffering the full dataset in memory.
async fn audit_export(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AuditExportQuery>,
) -> AppResult<Response> {
    require(&u, Page::AdminAudit, Act::View)?;
    let day = |s: &Option<String>| -> AppResult<Option<NaiveDate>> {
        match s.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(Some)
                .map_err(|_| bad(u.lang, ErrorKey::InvalidDate)),
        }
    };
    let from = day(&q.from)?.map(|d| d.to_string());
    let until = day(&q.to)?
        .and_then(|d| d.succ_opt())
        .map(|d| d.to_string());
    let like =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                format!(
                    "%{}%",
                    s.replace('\\', "\\\\")
                        .replace('%', "\\%")
                        .replace('_', "\\_")
                )
            });
    let action = q.action.filter(|a| !a.is_empty());
    let entity = q.entity.filter(|e| !e.is_empty());
    let limit = q
        .limit
        .unwrap_or(AUDIT_EXPORT_MAX)
        .clamp(1, AUDIT_EXPORT_MAX);

    let pool = st.db.clone();
    let stream = async_stream::stream! {
        // UTF-8 BOM so Excel and spreadsheet viewers recognize UTF-8 encoding (CJK text).
        yield Ok::<String, std::io::Error>("\u{FEFF}ID,Timestamp,Actor ID,Actor,Action,Entity,Entity ID,Detail\r\n".to_string());

        let mut rows = sqlx::query_as::<_, (i64, Option<i64>, Option<String>, String, String, Option<i64>, String, String)>(
            "SELECT a.id, a.actor_id, u.name, a.action, a.entity, a.entity_id, a.detail, a.created_at \
             FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id \
             WHERE ($1::text IS NULL OR a.entity = $1) \
               AND ($2::bigint IS NULL OR a.entity_id = $2) \
               AND ($3::text IS NULL OR a.action ILIKE $3 OR a.entity ILIKE $3 OR a.detail ILIKE $3 OR u.name ILIKE $3) \
               AND ($4::text IS NULL OR a.created_at >= $4) \
               AND ($5::text IS NULL OR a.created_at < $5) \
               AND ($6::text IS NULL OR a.action = $6) \
               AND ($7::bigint IS NULL OR a.actor_id = $7) \
             ORDER BY a.id DESC LIMIT $8",
        )
        .bind(&entity)
        .bind(q.entity_id)
        .bind(&like)
        .bind(&from)
        .bind(&until)
        .bind(&action)
        .bind(q.actor_id)
        .bind(limit)
        .fetch(&pool);

        while let Some(row) = rows.next().await {
            match row {
                Ok((id, aid, actor, act, ent, eid, detail, at)) => {
                    let line = format!(
                        "{},{},{},{},{},{},{},{}\r\n",
                        id,
                        at,
                        aid.map(|v| v.to_string()).unwrap_or_default(),
                        escape_csv_field(&actor.unwrap_or_else(|| "System".to_string())),
                        escape_csv_field(&act),
                        escape_csv_field(&ent),
                        eid.map(|v| v.to_string()).unwrap_or_default(),
                        escape_csv_field(&detail),
                    );
                    yield Ok(line);
                }
                Err(e) => {
                    yield Err(std::io::Error::other(e));
                    break;
                }
            }
        }
    };

    let filename = format!(
        "audit_export_{}.csv",
        &now().replace([':', '-'], "").replace('T', "_")[..15]
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
        .body(Body::from_stream(stream))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

#[derive(Deserialize)]
struct UpdateRatesIn {
    rates: Vec<RateEntryIn>,
}

#[derive(Deserialize)]
struct RateEntryIn {
    vehicle_type: String,
    rate_cents_per_km: i64,
}

async fn get_reimbursement_rates(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let rows: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT vehicle_type, rate_cents_per_km, updated_at FROM reimbursement_rates ORDER BY vehicle_type ASC",
    )
    .fetch_all(&st.db)
    .await?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(|(vehicle_type, rate_cents_per_km, updated_at)| {
            json!({
                "vehicle_type": vehicle_type,
                "rate_cents_per_km": rate_cents_per_km,
                "updated_at": updated_at,
            })
        })
        .collect();

    Ok(Json(json!({ "rates": items })))
}

async fn update_reimbursement_rates(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UpdateRatesIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;
    if req.rates.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let mut tx = st.db.begin().await?;
    let mut details = Vec::new();
    let now_str = now();
    for entry in &req.rates {
        if entry.rate_cents_per_km < 0 {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        if entry.vehicle_type != "standard" && entry.vehicle_type != "other" {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        sqlx::query(
            "INSERT INTO reimbursement_rates (vehicle_type, rate_cents_per_km, updated_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (vehicle_type) DO UPDATE
             SET rate_cents_per_km = EXCLUDED.rate_cents_per_km, updated_at = EXCLUDED.updated_at",
        )
        .bind(&entry.vehicle_type)
        .bind(entry.rate_cents_per_km)
        .bind(&now_str)
        .execute(&mut *tx)
        .await?;
        details.push(format!(
            "{}:{}c",
            entry.vehicle_type, entry.rate_cents_per_km
        ));
    }
    tx.commit().await?;

    let detail_str = details.join(", ");
    audit(
        &st.db,
        Some(u.id),
        "settings.rates",
        "reimbursement_rates",
        None,
        &detail_str,
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct UpdateSystemSettingsIn {
    audit_retention_days: i64,
    chat_retention_days: i64,
    max_live_drivers: i64,
}

async fn get_system_settings(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM system_settings WHERE key IN ('audit_retention_days', 'chat_retention_days', 'max_live_drivers')",
    )
    .fetch_all(&st.db)
    .await?;

    let mut audit_days = st.cfg.audit_retention_days;
    let mut chat_days = st.cfg.chat_retention_days;
    let mut max_drivers = st.cfg.max_live_drivers;

    for (k, v) in rows {
        if let Ok(n) = v.parse::<i64>() {
            match k.as_str() {
                "audit_retention_days" => audit_days = n,
                "chat_retention_days" => chat_days = n,
                "max_live_drivers" => max_drivers = n,
                _ => {}
            }
        }
    }

    Ok(Json(json!({
        "audit_retention_days": audit_days,
        "chat_retention_days": chat_days,
        "max_live_drivers": max_drivers,
    })))
}

async fn update_system_settings(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UpdateSystemSettingsIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;

    if !(30..=3650).contains(&req.audit_retention_days) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(7..=3650).contains(&req.chat_retention_days) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(1..=100).contains(&req.max_live_drivers) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let now_str = now();
    let mut tx = st.db.begin().await?;

    for (k, v) in [
        ("audit_retention_days", req.audit_retention_days.to_string()),
        ("chat_retention_days", req.chat_retention_days.to_string()),
        ("max_live_drivers", req.max_live_drivers.to_string()),
    ] {
        sqlx::query(
            "INSERT INTO system_settings (key, value, updated_at, updated_by)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (key) DO UPDATE
             SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at, updated_by = EXCLUDED.updated_by",
        )
        .bind(k)
        .bind(&v)
        .bind(&now_str)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let detail = format!(
        "audit_retention_days: {}d, chat_retention_days: {}d, max_live_drivers: {}",
        req.audit_retention_days, req.chat_retention_days, req.max_live_drivers
    );
    audit(
        &st.db,
        Some(u.id),
        "settings.system",
        "system_settings",
        None,
        &detail,
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}
