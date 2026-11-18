//! 管理员: dashboard, members, invitations, groups, areas, recovery approvals, audit log.
//! Roles themselves are edited in `routes::roles`.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Datelike, NaiveDate};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, conflict, forbidden, not_found, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{self, parse_roles, Act, Page};
use crate::state::AppState;
use crate::util::{
    audit, escape_like, in_days, minutes_ago, now, random_token, require, require_one, Lang,
};

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
        .route("/admin/reports/demand", get(demand_report))
        .route(
            "/admin/settings/alerts",
            get(get_alert_settings).put(update_alert_settings),
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
    let attention = count("SELECT COUNT(*) FROM rides WHERE needs_attention=1 AND status NOT IN ('cancelled','completed','no_show','alt_transport') AND $1 != ''").await?;
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
    // Invites still waiting to be used, and approved claims waiting for finance to pay them:
    // the counts on the Members "Invites" and Reimbursements "Finance payout" tabs.
    let (open_invites,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM invites WHERE used_at IS NULL AND expires_at >= $1")
            .bind(now())
            .fetch_one(&st.db)
            .await?;
    let (awaiting_payment,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM reimbursements WHERE status='approved'")
            .fetch_one(&st.db)
            .await?;
    // How long the oldest item in each queue has waited, for the work queue on the admin home.
    let (oldest_approval_at, next_no_driver_at): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT (SELECT MIN(created_at) FROM rides WHERE status='pending_approval'),
                (SELECT MIN(pickup_at) FROM rides WHERE status='pending_driver' AND pickup_at >= $1)",
    )
    .bind(&today)
    .fetch_one(&st.db)
    .await?;
    let (help_claimed_by_me,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM help_tickets WHERE status='open' AND assignee_id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let (oldest_help_at,): (Option<String>,) =
        sqlx::query_as("SELECT MIN(updated_at) FROM help_tickets WHERE status='open'")
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
        "open_invites": open_invites, "awaiting_payment": awaiting_payment,
        "oldest_approval_at": oldest_approval_at, "next_no_driver_at": next_no_driver_at,
        "oldest_help_at": oldest_help_at, "help_claimed_by_me": help_claimed_by_me,
    })))
}

#[derive(Deserialize)]
struct UserQuery {
    q: Option<String>,
    role: Option<String>,
    /// `active`, `pending` or `disabled`.
    status: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
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
    let like = format!(
        "%{}%",
        escape_like(q.q.as_deref().unwrap_or_default().trim())
    );
    // Without 成员 · view only the name can order the list: sorting by email would line
    // names up by an address the caller can't see, leaking it the same way a search would.
    let sortable: &[(&str, &str)] = if staff {
        &[
            ("name", "u.name"),
            ("email", "u.email"),
            ("status", "u.status"),
            ("created_at", "u.created_at"),
        ]
    } else {
        &[("name", "u.name")]
    };
    let order = crate::util::order_by(
        q.sort.as_deref(),
        q.dir.as_deref(),
        sortable,
        "u.name ASC, u.id ASC",
        "u.id ASC",
    );

    // Search ignores letter case. Without 成员 · view the caller sees names only, so the
    // search matches names only (matching email or phone would reveal them a character
    // at a time) and only active members can be picked.
    let mut conds = vec![if staff {
        "(u.name ILIKE $1 ESCAPE '\\' OR u.email ILIKE $1 ESCAPE '\\' OR u.phone ILIKE $1 ESCAPE '\\')".to_string()
    } else {
        "u.name ILIKE $1 ESCAPE '\\'".to_string()
    }];
    let status = if staff {
        q.status.clone()
    } else {
        Some("active".to_string())
    };
    let mut n = 2;
    if q.role.is_some() {
        conds.push(format!(
            "EXISTS (SELECT 1 FROM member_roles mr WHERE mr.user_id = u.id AND mr.role_key = ${n})"
        ));
        n += 1;
    }
    if status.is_some() {
        conds.push(format!("u.status = ${n}"));
        n += 1;
    }
    let where_sql = conds.join(" AND ");

    type Row = (
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        Option<String>,
    );
    let to_json = |(id, name, email, phone, roles, status, created, pk, groups): Row| {
        if staff {
            json!({"id": id, "name": name, "email": email, "phone": phone, "roles": parse_roles(&roles), "status": status,
                   "created_at": created, "passkeys": pk, "groups": groups.unwrap_or_default()})
        } else {
            json!({"id": id, "name": name})
        }
    };
    let select = "SELECT u.id, u.name, u.email, u.phone, u.roles, u.status, u.created_at, (SELECT COUNT(*) FROM passkeys p WHERE p.user_id=u.id),
                (SELECT string_agg(g.name, '、') FROM group_members gm JOIN groups g ON g.id=gm.group_id WHERE gm.user_id=u.id)
             FROM users u";

    if q.page.is_some() || q.limit.is_some() {
        let count_sql = format!("SELECT COUNT(*) FROM users u WHERE {where_sql}");
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql).bind(&like);
        if let Some(r) = &q.role {
            count_query = count_query.bind(r);
        }
        if let Some(s) = &status {
            count_query = count_query.bind(s);
        }
        let (total,) = count_query.fetch_one(&st.db).await?;

        let page = q.page.unwrap_or(1).max(1);
        let limit = q.limit.unwrap_or(50).clamp(1, 200);
        let offset = (page - 1) * limit;

        let sql = format!(
            "{select} WHERE {where_sql} ORDER BY {order} LIMIT ${n} OFFSET ${}",
            n + 1
        );
        let mut query = sqlx::query_as::<_, Row>(&sql).bind(&like);
        if let Some(r) = &q.role {
            query = query.bind(r);
        }
        if let Some(s) = &status {
            query = query.bind(s);
        }
        query = query.bind(limit).bind(offset);
        let out: Vec<Value> = query
            .fetch_all(&st.db)
            .await?
            .into_iter()
            .map(to_json)
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

    // Unpaged (member pickers): the same filters, applied before the cap.
    let sql = format!("{select} WHERE {where_sql} ORDER BY {order} LIMIT 2000");
    let mut query = sqlx::query_as::<_, Row>(&sql).bind(&like);
    if let Some(r) = &q.role {
        query = query.bind(r);
    }
    if let Some(s) = &status {
        query = query.bind(s);
    }
    let out: Vec<Value> = query
        .fetch_all(&st.db)
        .await?
        .into_iter()
        .map(to_json)
        .collect();
    Ok(Json(json!(out)))
}

#[derive(Deserialize)]
struct UserUpdate {
    roles: Vec<String>,
    status: String,
    group_ids: Option<Vec<i64>>,
    /// Sent by a Vice-President when promoting an ordinary member to administrator,
    /// after the President has acknowledged the appointment.
    #[serde(default)]
    president_acknowledged: bool,
}

async fn update_user(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<UserUpdate>,
) -> AppResult<Json<Value>> {
    // Somebody in the chain of command may appoint the level below without holding the
    // members page (Finance appoints Audit), but only for a change that moves chain
    // roles and nothing else. That is decided below, once the member's roles are known.
    if perms::rank_of(&u.roles).is_none() {
        require(&u, Page::AdminMembers, Act::Update)?;
    }
    if !matches!(req.status.as_str(), "active" | "disabled") {
        return Err(bad(u.lang, ErrorKey::InvalidStatus));
    }
    let mut tx = st.db.begin().await?;
    // Serialize with role edits (routes::roles) so a self-check below never reads a
    // role's permissions while another request is changing them. Taken before any
    // row lock, in the same order as routes::roles, so the two cannot deadlock.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_roles_mutation', 0))")
        .execute(&mut *tx)
        .await?;
    // Lock active superadmins in a stable order so concurrent edits cannot each
    // observe themselves as the last one and demote both accounts.
    let active_superadmins: Vec<i64> = sqlx::query_scalar(
        "SELECT u.id FROM users u JOIN member_roles mr ON mr.user_id = u.id AND mr.role_key = $1
         WHERE u.status='active' ORDER BY u.id FOR UPDATE OF u",
    )
    .bind(perms::SUPERADMIN_ROLE)
    .fetch_all(&mut *tx)
    .await?;
    let (old_roles, old_status): (String, String) =
        sqlx::query_as("SELECT roles, status FROM users WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(not_found(u.lang))?;
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v.dedup();
        v
    };
    let new_roles = sorted(parse_roles(&req.roles.join(",")));
    let old_parsed = parse_roles(&old_roles);
    let roles_changed = new_roles != sorted(old_parsed.clone());
    // The super admin role, and the account of anyone holding it, is changed only by a
    // super admin; and the last active one can be neither demoted nor disabled.
    let was_super = perms::is_superadmin(&old_parsed) && old_status == "active";
    let stays_super = perms::is_superadmin(&new_roles) && req.status == "active";
    if (perms::is_superadmin(&old_parsed) || perms::is_superadmin(&new_roles))
        && !perms::is_superadmin(&u.roles)
    {
        return Err(forbidden(u.lang, ErrorKey::SuperadminOnly));
    }
    if was_super
        && !stays_super
        && active_superadmins
            .iter()
            .all(|superadmin_id| *superadmin_id == id)
    {
        return Err(bad(u.lang, ErrorKey::LastSuperadmin));
    }
    // Chain of command (see `perms::RANKED_ROLES`). Read the caller's roles under the
    // lock rather than trusting `u`, which was loaded before it.
    let (caller_raw,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let caller_roles = parse_roles(&caller_raw);
    let caller_super = perms::is_superadmin(&caller_roles);
    // Permissions. A change that only moves chain-of-command roles, or disables or
    // re-enables somebody the caller outranks, is authorised by rank alone (the checks
    // above and below decide which ones): Finance appoints and commands Audit without
    // holding the members or roles pages. Anything else needs the pages' permissions.
    let chain_only = new_roles
        .iter()
        .filter(|r| !old_parsed.contains(r))
        .chain(old_parsed.iter().filter(|r| !new_roles.contains(r)))
        .all(|r| perms::role_rank(r).is_some() && r != perms::SUPERADMIN_ROLE);
    let commands_ranked = id != u.id
        && perms::rank_of(&old_parsed).is_some()
        && perms::outranks(&caller_roles, &old_parsed);
    let by_rank = perms::rank_of(&caller_roles).is_some()
        && req.group_ids.is_none()
        && (!roles_changed || chain_only)
        && (req.status == old_status || commands_ranked);
    if !by_rank {
        require(&u, Page::AdminMembers, Act::Update)?;
    }
    if roles_changed && !chain_only {
        // Giving or taking any other role decides what a member may do, which is the
        // same authority as editing the roles themselves.
        require(&u, Page::AdminRoles, Act::Update)?;
    }
    if roles_changed {
        let (known,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM roles WHERE key = ANY($1)")
            .bind(&new_roles)
            .fetch_one(&mut *tx)
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
            // Compare with what the caller holds now (read under the lock), not with
            // the permissions loaded when the request arrived.
            let current = perms::for_roles(&st.db, &old_parsed).await?;
            if after.new_finance(&current) {
                return Err(forbidden(u.lang, ErrorKey::CannotGrantSelfFinance));
            }
        }
    }
    // Chain of command (see `perms::RANKED_ROLES`).
    // Whoever is ranked can only be disabled, edited or re-assigned by somebody strictly
    // above them; the super admin may act on any account (peers included, as before).
    if id != u.id && !caller_super && !perms::outranks(&caller_roles, &old_parsed) {
        return Err(forbidden(u.lang, ErrorKey::RankTooLow));
    }
    if roles_changed {
        let ranked = |roles: &[String]| -> Vec<String> {
            roles
                .iter()
                .filter(|r| perms::role_rank(r).is_some())
                .cloned()
                .collect()
        };
        let (old_ranked, new_ranked) = (ranked(&old_parsed), ranked(&new_roles));
        // Nobody changes their own place in the chain.
        if id == u.id && !caller_super && old_ranked != new_ranked {
            return Err(forbidden(u.lang, ErrorKey::RankTooLow));
        }
        // Appointing is one level at a time; taking a role away is a command over
        // everything below. The super admin role itself is handled above.
        for role in new_ranked.iter().filter(|r| !old_ranked.contains(r)) {
            if role != perms::SUPERADMIN_ROLE && !perms::can_appoint(&caller_roles, role) {
                return Err(forbidden(u.lang, ErrorKey::AppointNextLevelOnly));
            }
        }
        for role in old_ranked.iter().filter(|r| !new_ranked.contains(r)) {
            if role != perms::SUPERADMIN_ROLE
                && !caller_super
                && !perms::outranks_role(&caller_roles, role)
            {
                return Err(forbidden(u.lang, ErrorKey::RankTooLow));
            }
        }
        // A role that carries the power to manage members or roles makes its holder an
        // administrator in all but name, so it is handed out like the admin role.
        let added: Vec<String> = new_roles
            .iter()
            .filter(|r| !old_parsed.contains(r))
            .cloned()
            .collect();
        if !added.is_empty() && !perms::can_manage_admins(&caller_roles) {
            let (power,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM role_permissions WHERE role_key = ANY($1) AND permission = ANY($2)",
            )
            .bind(&added)
            .bind(perms::admin_power_keys())
            .fetch_one(&mut *tx)
            .await?;
            if power > 0 {
                return Err(forbidden(u.lang, ErrorKey::AdminAppointmentRestricted));
            }
        }
    }
    // Disabling or re-enabling an administrator is as much a demotion as removing the
    // role, so it needs the same standing.
    if req.status != old_status
        && old_parsed.iter().any(|r| r == perms::ADMIN_ROLE)
        && !perms::can_manage_admins(&caller_roles)
    {
        return Err(forbidden(u.lang, ErrorKey::AdminAppointmentRestricted));
    }
    // Appointing or demoting an administrator is reserved for the super admin,
    // President and Vice-President. Other staff may hold admin tools without that power.
    let was_admin = old_parsed.iter().any(|r| r == perms::ADMIN_ROLE);
    let becomes_admin = new_roles.iter().any(|r| r == perms::ADMIN_ROLE);
    if roles_changed && was_admin != becomes_admin && !perms::can_manage_admins(&u.roles) {
        return Err(forbidden(u.lang, ErrorKey::AdminAppointmentRestricted));
    }
    // A Vice-President promoting an ordinary member to administrator must first
    // confirm that the President has acknowledged the appointment.
    let president_ack =
        roles_changed && !was_admin && becomes_admin && perms::needs_president_ack(&u.roles);
    if president_ack && !req.president_acknowledged {
        return Err(forbidden(
            u.lang,
            ErrorKey::PresidentAcknowledgementRequired,
        ));
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
        .execute(&mut *tx)
        .await?;
    let mut released: Vec<i64> = Vec::new();
    if roles_changed && perms::for_roles(&st.db, &new_roles).await?.privileged() {
        // The session check caps staff sessions at 12 hours from sign-in; this also
        // shortens the stored expiry so open streams (which only check expires_at)
        // end on time.
        sqlx::query("UPDATE sessions SET expires_at = $1 WHERE user_id = $2 AND expires_at > $1")
            .bind(crate::util::in_hours(12))
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if req.status == "disabled" {
        sqlx::query("DELETE FROM sessions WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM auth_challenges WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE invites SET expires_at=$2 WHERE kind='recovery' AND user_id=$1 AND used_at IS NULL",
        )
        .bind(id)
        .bind(minutes_ago(1))
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE users SET calendar_token_hash=NULL WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        // A disabled driver can no longer drive: rides not yet started go back to the
        // pool, flagged for transport staff, instead of waiting on someone who can't act.
        released = sqlx::query_scalar(
            "UPDATE rides SET status='pending_driver', driver_id=NULL, trip_id=NULL,
                    needs_attention = CASE WHEN status IN ('confirmed','pending_driver_confirm') THEN 1 ELSE 0 END,
                    attention_reason='原司机账户已停用，需重新安排', updated_at=$2
             WHERE driver_id=$1
               AND status IN ('pending_passenger','pending_approval','pending_driver_confirm','confirmed')
             RETURNING passenger_id",
        )
        .bind(id)
        .bind(now())
        .fetch_all(&mut *tx)
        .await?;
    }
    let mut groups_note = String::new();
    if let Some(mut gids) = req.group_ids {
        gids.sort_unstable();
        gids.dedup();
        let (known,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM groups WHERE id=ANY($1)")
            .bind(&gids)
            .fetch_one(&mut *tx)
            .await?;
        if known != gids.len() as i64 {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        let old: Vec<i64> = sqlx::query_scalar(
            "SELECT group_id FROM group_members WHERE user_id=$1 ORDER BY group_id",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM group_members WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for g in &gids {
            sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
                .bind(g)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        if gids != old {
            groups_note = format!("; groups {old:?} → {gids:?}");
        }
    }
    tx.commit().await?;
    for passenger in released {
        crate::util::notify(
            &st.db,
            passenger,
            ("司机无法出车", "Driver cannot drive"),
            (
                "交通组正在为您安排其他司机，请留意通知。",
                "Transport staff are arranging another driver for you; please watch for notifications.",
            ),
            "/rides",
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "user.update",
        "user",
        Some(id),
        format!(
            "roles {old_roles} → {roles}; status {old_status} → {}{groups_note}{}",
            req.status,
            if president_ack {
                "; Vice-President confirmed President's acknowledgement"
            } else {
                ""
            }
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
    /// The group this one is part of. Missing: unchanged (create: none). `null`: none.
    #[serde(default, deserialize_with = "present")]
    parent_id: Option<Option<i64>>,
}

/// Tells a JSON `null` (`Some(None)`) apart from a missing field (`None`).
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i64>>, D::Error> {
    Ok(Some(Option::deserialize(d)?))
}

/// A parent must be an ordinary group (not 全体会员 or visitors), keep the tree free of
/// loops, and leave it at most three levels deep.
async fn check_parent(st: &AppState, lang: Lang, group: i64, parent: Option<i64>) -> AppResult<()> {
    let Some(p) = parent else { return Ok(()) };
    let special: Option<(i64, i64)> =
        sqlx::query_as("SELECT is_all, is_visitor FROM groups WHERE id=$1")
            .bind(p)
            .fetch_optional(&st.db)
            .await?;
    let mine: Option<(i64, i64)> =
        sqlx::query_as("SELECT is_all, is_visitor FROM groups WHERE id=$1")
            .bind(group)
            .fetch_optional(&st.db)
            .await?;
    let plain = |g: Option<(i64, i64)>| g.is_none_or(|(a, v)| a == 0 && v == 0);
    if special.is_none()
        || !plain(special)
        || !plain(mine)
        || !crate::audience::parent_ok(&st.db, group, p).await?
    {
        return Err(bad(lang, ErrorKey::InvalidParentGroup));
    }
    Ok(())
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
    let parent = req.parent_id.flatten();
    // 0 is never a group id: a new group has no children and cannot be its own ancestor.
    check_parent(&st, u.lang, 0, parent).await?;
    let result = sqlx::query_as(
        "INSERT INTO groups (name, description, parent_id) VALUES ($1,$2,$3) RETURNING id",
    )
    .bind(req.name.trim())
    .bind(req.description.unwrap_or_default())
    .bind(parent)
    .fetch_one(&st.db)
    .await;
    let (id,): (i64,) = match result {
        Ok(row) => row,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::GroupNameExists));
        }
        Err(e) => return Err(e.into()),
    };
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
    if req.name.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::GroupNameRequired));
    }
    if let Some(parent) = req.parent_id {
        check_parent(&st, u.lang, id, parent).await?;
    }
    let result = sqlx::query(
        "UPDATE groups SET name=$1, description=$2, parent_id = CASE WHEN $4 THEN $5 ELSE parent_id END WHERE id=$3",
    )
        .bind(req.name.trim())
        .bind(req.description.unwrap_or_default())
        .bind(id)
        .bind(req.parent_id.is_some())
        .bind(req.parent_id.flatten())
        .execute(&st.db)
        .await;
    match result {
        Ok(result) if result.rows_affected() == 0 => return Err(not_found(u.lang)),
        Ok(_) => {}
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::GroupNameExists));
        }
        Err(e) => return Err(e.into()),
    }
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
    let (is_all, is_visitor): (i64, i64) =
        sqlx::query_as("SELECT is_all, is_visitor FROM groups WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    if is_all == 1 || is_visitor == 1 {
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
    Json(mut req): Json<MembersIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminGroups, Act::Update)?;
    req.user_ids.sort_unstable();
    req.user_ids.dedup();
    let mut tx = st.db.begin().await?;
    // An unknown member id would otherwise fail the foreign key and come back as a 500.
    let (known,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id=ANY($1)")
        .bind(&req.user_ids)
        .fetch_one(&mut *tx)
        .await?;
    if known != req.user_ids.len() as i64 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM groups WHERE id=$1 FOR KEY SHARE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if exists.is_none() {
        return Err(not_found(u.lang));
    }
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
    let result = sqlx::query_as("INSERT INTO areas (name, active) VALUES ($1, 1) RETURNING id")
        .bind(req.name.trim())
        .fetch_one(&st.db)
        .await;
    let (id,): (i64,) = match result {
        Ok(row) => row,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::AreaExists));
        }
        Err(e) => return Err(e.into()),
    };
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
    if req.name.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::AreaNameRequired));
    }
    let result = sqlx::query("UPDATE areas SET name=$1, active=$2 WHERE id=$3")
        .bind(req.name.trim())
        .bind(req.active.unwrap_or(true) as i64)
        .bind(id)
        .execute(&st.db)
        .await;
    match result {
        Ok(result) if result.rows_affected() == 0 => return Err(not_found(u.lang)),
        Ok(_) => {}
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::AreaExists));
        }
        Err(e) => return Err(e.into()),
    }
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
    let mut tx = st.db.begin().await?;
    let email: String = sqlx::query_scalar("SELECT email FROM recovery_requests WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(not_found(u.lang))?;
    let uid: i64 = sqlx::query_scalar("SELECT id FROM users WHERE email=lower($1) FOR UPDATE")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(not_found(u.lang))?;
    let (locked_email, status, resolved_by): (String, String, Option<i64>) = sqlx::query_as(
        "SELECT email, status, resolved_by FROM recovery_requests WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(not_found(u.lang))?;
    if locked_email != email {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    if !matches!(status.as_str(), "pending" | "awaiting_second") {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    let roles: String = sqlx::query_scalar("SELECT roles FROM users WHERE id=$1")
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;
    let target_roles = parse_roles(&roles);
    if perms::is_superadmin(&target_roles) && !perms::is_superadmin(&u.roles) {
        return Err(forbidden(u.lang, ErrorKey::SuperadminOnly));
    }
    // Recovering a privileged account needs two different admins: the first
    // only flags the request, the second confirms and receives the link.
    if status == "pending" && perms::is_privileged(&st.db, &roles).await? {
        let changed = sqlx::query(
            "UPDATE recovery_requests SET status='awaiting_second', resolved_by=$1 WHERE id=$2 AND status IN ('pending','awaiting_second')",
        )
        .bind(u.id)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if changed == 0 {
            return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
        }
        tx.commit().await?;
        audit(
            &st.db,
            Some(u.id),
            "recovery.await_second",
            "recovery",
            Some(id),
            format!("user_id={uid}; email={email}"),
        )
        .await;
        return Ok(Json(json!({"awaiting_second": true})));
    }
    if status == "awaiting_second" && resolved_by == Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::SecondAdminRequired));
    }
    // Only the hash is stored (see `routes::auth::INVITE_BY_TOKEN`); the link is returned once.
    let token = random_token(32);
    // The account row was locked above, so link replacement is serialized per user.
    sqlx::query(
        "UPDATE invites SET expires_at=$1 WHERE kind='recovery' AND user_id=$2 AND used_at IS NULL",
    )
    .bind(minutes_ago(1))
    .bind(uid)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO invites (token, kind, email, user_id, created_by, expires_at, created_at) VALUES ($1, 'recovery', $2, $3, $4, $5, $6)")
        .bind(hash_token(&token))
        .bind(&email)
        .bind(uid)
        .bind(u.id)
        .bind(in_days(1))
        .bind(now())
        .execute(&mut *tx)
        .await?;
    let changed = sqlx::query("UPDATE recovery_requests SET status='approved', resolved_by=$1 WHERE id=$2 AND status IN ('pending','awaiting_second')")
        .bind(u.id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if changed.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    sqlx::query("UPDATE recovery_requests SET status='superseded' WHERE email=lower($1) AND id<>$2 AND status IN ('pending','awaiting_second')")
        .bind(&email)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // The old phone is presumed lost: revoke its sessions, passkeys and authenticator.
    // The member registers a new passkey with the one-time link.
    sqlx::query("DELETE FROM sessions WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM passkeys WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE users SET totp_enabled=0, totp_secret=NULL, totp_last_step=0, totp_backup_codes=NULL, calendar_token_hash=NULL WHERE id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE user_id=$1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "recovery.approve",
        "recovery",
        Some(id),
        format!("user_id={uid}; email={email}"),
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
    let mut tx = st.db.begin().await?;
    let email: String = sqlx::query_scalar("SELECT email FROM recovery_requests WHERE id=$1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(not_found(u.lang))?;
    let user_id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM users WHERE email=lower($1) FOR UPDATE")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;
    let status: String =
        sqlx::query_scalar("SELECT status FROM recovery_requests WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(not_found(u.lang))?;
    if !matches!(status.as_str(), "pending" | "awaiting_second") {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    let changed = sqlx::query("UPDATE recovery_requests SET status='rejected', resolved_by=$1 WHERE id=$2 AND status IN ('pending','awaiting_second')")
        .bind(u.id)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "recovery.reject",
        "recovery",
        Some(id),
        format!("user_id={user_id:?}; email={email}"),
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

fn audit_attendance(action: &str, detail: &str) -> Option<(i64, &'static str)> {
    if action != "event.attendance" {
        return None;
    }
    let (member, value) = detail.strip_prefix("user ")?.split_once(" = ")?;
    let status = match value {
        "Some(true)" => "attended",
        "Some(false)" => "absent",
        "None" => "cleared",
        _ => return None,
    };
    Some((member.parse().ok()?, status))
}

/// SQL for the name of what an audit entry is about. The live record comes first; once
/// that is gone (or for roles, which have no numeric id) the name the entry itself
/// carried in `detail` stands in, so a deleted item still reads as more than a number.
fn audit_name_sql(lang: Lang) -> String {
    let guide_title = match lang {
        Lang::Zh => "title_zh",
        Lang::En => "title_en",
    };
    format!(
        r"COALESCE(
            CASE a.entity
                WHEN 'user' THEN (SELECT name FROM users WHERE id = a.entity_id)
                WHEN 'ride' THEN (SELECT p.name FROM rides r JOIN users p ON p.id = r.passenger_id WHERE r.id = a.entity_id)
                WHEN 'event' THEN (SELECT title FROM events WHERE id = a.entity_id)
                WHEN 'group' THEN (SELECT name FROM groups WHERE id = a.entity_id)
                WHEN 'area' THEN (SELECT name FROM areas WHERE id = a.entity_id)
                WHEN 'settlement' THEN (SELECT title FROM settlements WHERE id = a.entity_id)
                WHEN 'settlement_share' THEN (SELECT p.name FROM settlement_shares s JOIN users p ON p.id = s.user_id WHERE s.id = a.entity_id)
                WHEN 'reimbursement' THEN (SELECT p.name FROM reimbursements r JOIN users p ON p.id = r.user_id WHERE r.id = a.entity_id)
                WHEN 'announcement' THEN (SELECT title FROM announcements WHERE id = a.entity_id)
                WHEN 'help_ticket' THEN (SELECT subject FROM help_tickets WHERE id = a.entity_id)
                WHEN 'file' THEN (SELECT NULLIF(original_name, '') FROM files WHERE id = a.entity_id)
                WHEN 'invite' THEN (SELECT COALESCE(NULLIF(name, ''), NULLIF(email, '')) FROM invites WHERE id = a.entity_id)
                WHEN 'recovery' THEN (SELECT email FROM recovery_requests WHERE id = a.entity_id)
                WHEN 'visitor_pass' THEN (SELECT name FROM visitor_passes WHERE id = a.entity_id)
                WHEN 'guide_workflow' THEN (SELECT {guide_title} FROM guide_workflows WHERE id = a.entity_id)
                WHEN 'role' THEN CASE
                    WHEN a.action = 'role.update' THEN COALESCE(
                        (SELECT NULLIF(name, '') FROM roles WHERE key = split_part(a.detail, ':', 1)),
                        NULLIF(split_part(a.detail, ':', 1), ''))
                    ELSE substring(a.detail from '^(.*) \([a-z][a-z0-9_]*\)[:;]')
                END
            END,
            CASE
                WHEN a.action IN ('group.create', 'group.update', 'group.delete', 'area.create',
                                  'announcement.create', 'announcement.update', 'announcement.delete',
                                  'settlement.create', 'event.create', 'event.cancel')
                    THEN NULLIF(a.detail, '')
                WHEN a.action = 'settlement.delete' THEN NULLIF(regexp_replace(a.detail, '^draft: ', ''), '')
            END
        )"
    )
}

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
            .map(|s| format!("%{}%", escape_like(s)));
    let action = q.action.filter(|a| !a.is_empty());
    let entity = q.entity.filter(|e| !e.is_empty());
    let limit = q.limit.unwrap_or(AUDIT_PAGE).clamp(1, AUDIT_PAGE_MAX);
    let sql = format!(
        "SELECT a.id, a.actor_id, u.name, a.action, a.entity, a.entity_id,
                {name},
                CASE WHEN a.entity = 'ride' THEN (SELECT pickup_at FROM rides WHERE id = a.entity_id) END,
                a.detail, a.created_at
         FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id
         WHERE ($1::text IS NULL OR a.entity = $1)
           AND ($2::bigint IS NULL OR a.entity_id = $2)
           AND ($3::text IS NULL OR a.action ILIKE $3 ESCAPE '\\' OR a.entity ILIKE $3 ESCAPE '\\' OR a.detail ILIKE $3 ESCAPE '\\' OR u.name ILIKE $3 ESCAPE '\\' OR {name} ILIKE $3 ESCAPE '\\')
           AND ($4::text IS NULL OR a.created_at >= $4)
           AND ($5::text IS NULL OR a.created_at < $5)
           AND ($6::bigint IS NULL OR a.id < $6)
           AND ($7::text IS NULL OR a.action = $7)
           AND ($8::bigint IS NULL OR a.actor_id = $8)
         ORDER BY a.id DESC LIMIT $9",
        name = audit_name_sql(u.lang),
    );
    // One row more than the page shows whether another page follows.
    let mut rows: Vec<(
        i64,
        Option<i64>,
        Option<String>,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        String,
        String,
    )> = sqlx::query_as(&sql)
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
    let attendance: Vec<_> = rows
        .iter()
        .map(|(_, _, _, action, _, _, _, _, detail, _)| audit_attendance(action, detail))
        .collect();
    let member_ids: Vec<_> = attendance.iter().flatten().map(|(id, _)| *id).collect();
    let member_names: HashMap<i64, String> = if member_ids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as("SELECT id, name FROM users WHERE id = ANY($1)")
            .bind(&member_ids)
            .fetch_all(&st.db)
            .await?
            .into_iter()
            .collect()
    };
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
        "items": rows.into_iter().zip(attendance).map(|((id, aid, actor, action, entity, eid, name, entity_at, detail, at), attendance)| {
            let attendance = attendance.map(|(member_id, status)| json!({
                "member_id": member_id, "member_name": member_names.get(&member_id), "status": status,
            }));
            json!({
                "id": id, "actor_id": aid, "actor": actor, "action": action, "entity": entity, "entity_id": eid,
                "entity_name": name, "entity_at": entity_at, "detail": detail, "attendance": attendance, "created_at": at,
            })
        }).collect::<Vec<_>>(),
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
    // Prefix single quote on formula triggers (=, +, -, @, \t, \r, |, %) to prevent spreadsheet injection.
    let trimmed = s.trim_start();
    let sanitized = if s.starts_with('\t')
        || s.starts_with('\r')
        || trimmed.starts_with('=')
        || trimmed.starts_with('+')
        || trimmed.starts_with('-')
        || trimmed.starts_with('@')
        || trimmed.starts_with('|')
        || trimmed.starts_with('%')
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
            .map(|s| format!("%{}%", escape_like(s)));
    let action = q.action.filter(|a| !a.is_empty());
    let entity = q.entity.filter(|e| !e.is_empty());
    let limit = q
        .limit
        .unwrap_or(AUDIT_EXPORT_MAX)
        .clamp(1, AUDIT_EXPORT_MAX);

    let pool = st.db.clone();
    let sql = format!(
        "SELECT a.id, a.actor_id, u.name, a.action, a.entity, a.entity_id, {name}, a.detail, a.created_at \
         FROM audit_logs a LEFT JOIN users u ON u.id=a.actor_id \
         WHERE ($1::text IS NULL OR a.entity = $1) \
           AND ($2::bigint IS NULL OR a.entity_id = $2) \
           AND ($3::text IS NULL OR a.action ILIKE $3 ESCAPE '\\' OR a.entity ILIKE $3 ESCAPE '\\' OR a.detail ILIKE $3 ESCAPE '\\' OR u.name ILIKE $3 ESCAPE '\\' OR {name} ILIKE $3 ESCAPE '\\') \
           AND ($4::text IS NULL OR a.created_at >= $4) \
           AND ($5::text IS NULL OR a.created_at < $5) \
           AND ($6::text IS NULL OR a.action = $6) \
           AND ($7::bigint IS NULL OR a.actor_id = $7) \
         ORDER BY a.id DESC LIMIT $8",
        name = audit_name_sql(u.lang),
    );
    let stream = async_stream::stream! {
        // UTF-8 BOM so Excel and spreadsheet viewers recognize UTF-8 encoding (CJK text).
        yield Ok::<String, std::io::Error>("\u{FEFF}ID,Timestamp,Actor ID,Actor,Action,Entity,Entity ID,Entity Name,Detail\r\n".to_string());

        let mut rows = sqlx::query_as::<_, (i64, Option<i64>, Option<String>, String, String, Option<i64>, Option<String>, String, String)>(&sql)
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
                Ok((id, aid, actor, act, ent, eid, name, detail, at)) => {
                    let line = format!(
                        "{},{},{},{},{},{},{},{},{}\r\n",
                        id,
                        at,
                        aid.map(|v| v.to_string()).unwrap_or_default(),
                        escape_csv_field(&actor.unwrap_or_else(|| "System".to_string())),
                        escape_csv_field(&act),
                        escape_csv_field(&ent),
                        eid.map(|v| v.to_string()).unwrap_or_default(),
                        escape_csv_field(&name.unwrap_or_default()),
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
    session_timeout_mins: Option<i64>,
    auto_lock_mins: Option<i64>,
}

async fn get_system_settings(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM system_settings WHERE key IN ('audit_retention_days', 'chat_retention_days', 'max_live_drivers', 'session_timeout_mins')",
    )
    .fetch_all(&st.db)
    .await?;

    let mut audit_days = st.cfg.audit_retention_days;
    let mut chat_days = st.cfg.chat_retention_days;
    let mut max_drivers = st.cfg.max_live_drivers;
    let mut session_timeout = st.cfg.session_timeout_mins;

    for (k, v) in rows {
        if let Ok(n) = v.parse::<i64>() {
            match k.as_str() {
                "audit_retention_days" => audit_days = n,
                "chat_retention_days" => chat_days = n,
                "max_live_drivers" => max_drivers = n,
                "session_timeout_mins" => session_timeout = n,
                _ => {}
            }
        }
    }

    Ok(Json(json!({
        "audit_retention_days": audit_days,
        "chat_retention_days": chat_days,
        "max_live_drivers": max_drivers,
        "session_timeout_mins": session_timeout,
        "auto_lock_mins": session_timeout,
    })))
}

async fn update_system_settings(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UpdateSystemSettingsIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;

    let session_timeout = req
        .session_timeout_mins
        .or(req.auto_lock_mins)
        .unwrap_or(15);

    if !(30..=3650).contains(&req.audit_retention_days) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(7..=3650).contains(&req.chat_retention_days) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(1..=100).contains(&req.max_live_drivers) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(1..=1440).contains(&session_timeout) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let now_str = now();
    let mut tx = st.db.begin().await?;

    for (k, v) in [
        ("audit_retention_days", req.audit_retention_days.to_string()),
        ("chat_retention_days", req.chat_retention_days.to_string()),
        ("max_live_drivers", req.max_live_drivers.to_string()),
        ("session_timeout_mins", session_timeout.to_string()),
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
        "audit_retention_days: {}d, chat_retention_days: {}d, max_live_drivers: {}, session_timeout_mins: {}m",
        req.audit_retention_days, req.chat_retention_days, req.max_live_drivers, session_timeout
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

#[derive(Deserialize)]
struct DemandQuery {
    month: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

fn month_window(month: &str) -> Option<(String, String)> {
    let b = month.as_bytes();
    if b.len() != 7
        || b[4] != b'-'
        || !b[..4].iter().all(u8::is_ascii_digit)
        || !b[5..].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let y: i32 = month[..4].parse().ok()?;
    let m: u32 = month[5..7].parse().ok()?;
    if !(1900..=9998).contains(&y) || !(1..=12).contains(&m) {
        return None;
    }
    let (ey, em) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    Some((format!("{y:04}-{m:02}-01"), format!("{ey:04}-{em:02}-01")))
}

pub(crate) fn has_global_report_scope(u: &CurrentUser) -> bool {
    u.traffic_is_global()
        || u.can(Page::AdminReimbPay, Act::View)
        || u.can(Page::AdminBills, Act::View)
        || u.can(Page::AdminEvents, Act::View)
}

async fn demand_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<DemandQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReports, Act::View)?;

    let (start, end, display_to, period_label) = match (q.from.as_deref(), q.to.as_deref()) {
        (Some(f), Some(t)) => {
            let fd = NaiveDate::parse_from_str(f.trim(), "%Y-%m-%d")
                .map_err(|_| bad(u.lang, ErrorKey::InvalidAction))?;
            let td = NaiveDate::parse_from_str(t.trim(), "%Y-%m-%d")
                .map_err(|_| bad(u.lang, ErrorKey::InvalidAction))?;
            if td < fd || (td - fd).num_days() > 366 {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
            let end_d = td.succ_opt().unwrap_or(td);
            (
                fd.format("%Y-%m-%d").to_string(),
                end_d.format("%Y-%m-%d").to_string(),
                td.format("%Y-%m-%d").to_string(),
                format!("{}..{}", f.trim(), t.trim()),
            )
        }
        _ => {
            let m = q
                .month
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| now()[..7].to_string());
            let (s, e) =
                month_window(&m).ok_or_else(|| bad(u.lang, ErrorKey::MonthFormatInvalid))?;
            let d_to = e.clone();
            (s, e, d_to, m)
        }
    };

    #[derive(sqlx::FromRow)]
    struct DemandRow {
        area_id: i64,
        area_zh: String,
        area_en: String,
        pickup_at: String,
        status: String,
        driver_id: Option<i64>,
        passenger_id: i64,
        seats_needed: i64,
        assistance: String,
        leg: String,
    }

    let scope_creator: Option<i64> = if has_global_report_scope(&u) {
        None
    } else {
        Some(u.id)
    };

    let rows: Vec<DemandRow> = sqlx::query_as(
        "SELECT r.pickup_area_id AS area_id, COALESCE(a.name, '') AS area_zh, COALESCE(a.name, '') AS area_en,
                r.pickup_at, r.status, r.driver_id, r.passenger_id, r.seats AS seats_needed,
                COALESCE(r.assistance, '') AS assistance, COALESCE(r.leg, 'outbound') AS leg
         FROM rides r
         LEFT JOIN areas a ON a.id = r.pickup_area_id
         WHERE r.pickup_at >= $1 AND r.pickup_at < $2
           AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))",
    )
    .bind(&start)
    .bind(&end)
    .bind(scope_creator)
    .fetch_all(&st.db)
    .await?;

    let mut total_requests = 0_i64;
    let mut fulfilled = 0_i64;
    let mut unfilled = 0_i64;
    let mut completed = 0_i64;
    let mut cancelled_unassigned = 0_i64;
    let mut wheelchair_requests = 0_i64;
    let mut return_requests = 0_i64;
    let mut seats_unfilled = 0_i64;
    let mut drivers_active: HashSet<i64> = HashSet::new();
    let mut passengers_seen: HashSet<i64> = HashSet::new();

    #[derive(Default)]
    struct Bucket {
        area_zh: String,
        area_en: String,
        total: i64,
        fulfilled: i64,
        unfilled: i64,
        cancelled_unassigned: i64,
        seats_needed: i64,
        seats_unfilled: i64,
    }

    let mut by_area: HashMap<i64, Bucket> = HashMap::new();
    let mut by_slot: BTreeMap<(u32, String), Bucket> = BTreeMap::new();

    for r in &rows {
        let is_cancelled = r.status == "cancelled";
        let is_closed_alt = matches!(r.status.as_str(), "no_show" | "alt_transport");
        let is_unfilled = !is_cancelled && !is_closed_alt && r.driver_id.is_none();
        let is_fulfilled = matches!(r.status.as_str(), "confirmed" | "in_progress" | "completed");

        if is_cancelled {
            if r.driver_id.is_none() {
                cancelled_unassigned += 1;
            }
        } else {
            total_requests += 1;
            passengers_seen.insert(r.passenger_id);
            if r.status == "completed" {
                completed += 1;
            }
            if let Some(did) = r.driver_id {
                drivers_active.insert(did);
            }
            if is_fulfilled {
                fulfilled += 1;
            }
            if is_unfilled {
                unfilled += 1;
                seats_unfilled += r.seats_needed;
            }
            if r.assistance.to_ascii_lowercase().contains("wheelchair")
                || r.assistance.contains("轮椅")
            {
                wheelchair_requests += 1;
            }
            if r.leg == "return" {
                return_requests += 1;
            }
        }

        let ab = by_area.entry(r.area_id).or_insert_with(|| Bucket {
            area_zh: r.area_zh.clone(),
            area_en: r.area_en.clone(),
            ..Default::default()
        });
        if is_cancelled {
            if r.driver_id.is_none() {
                ab.cancelled_unassigned += 1;
            }
        } else {
            ab.total += 1;
            ab.seats_needed += r.seats_needed;
            if is_fulfilled {
                ab.fulfilled += 1;
            }
            if is_unfilled {
                ab.unfilled += 1;
                ab.seats_unfilled += r.seats_needed;
            }
        }

        if r.pickup_at.len() >= 13 {
            if let Ok(d) = NaiveDate::parse_from_str(&r.pickup_at[..10], "%Y-%m-%d") {
                let wd = d.weekday().num_days_from_sunday();
                let hour: u32 = r.pickup_at[11..13].parse().unwrap_or(0);
                let bucket_h = (hour / 2) * 2;
                let slot_label = format!("{bucket_h:02}:00-{:02}:00", (bucket_h + 2).min(24));
                let sb = by_slot.entry((wd, slot_label)).or_default();
                if is_cancelled {
                    if r.driver_id.is_none() {
                        sb.cancelled_unassigned += 1;
                    }
                } else {
                    sb.total += 1;
                    sb.seats_needed += r.seats_needed;
                    if is_fulfilled {
                        sb.fulfilled += 1;
                    }
                    if is_unfilled {
                        sb.unfilled += 1;
                        sb.seats_unfilled += r.seats_needed;
                    }
                }
            }
        }
    }

    let mut area_list: Vec<Value> = by_area
        .into_iter()
        .map(|(area_id, b)| {
            json!({
                "area_id": area_id,
                "area_name_zh": b.area_zh,
                "area_name_en": b.area_en,
                "total": b.total,
                "fulfilled": b.fulfilled,
                "unfilled": b.unfilled,
                "cancelled_unassigned": b.cancelled_unassigned,
                "seats_needed": b.seats_needed,
                "seats_unfilled": b.seats_unfilled,
                "fulfillment_rate_pct": if b.total > 0 { (b.fulfilled * 100) / b.total } else { 100 },
            })
        })
        .collect();
    area_list.sort_by(|a, b| {
        b["unfilled"]
            .as_i64()
            .cmp(&a["unfilled"].as_i64())
            .then_with(|| b["total"].as_i64().cmp(&a["total"].as_i64()))
    });

    let mut slot_list: Vec<Value> = by_slot
        .into_iter()
        .filter(|(_, b)| b.total > 0 || b.cancelled_unassigned > 0)
        .map(|((wd, slot), b)| {
            json!({
                "weekday": wd,
                "slot": slot,
                "total": b.total,
                "fulfilled": b.fulfilled,
                "unfilled": b.unfilled,
                "cancelled_unassigned": b.cancelled_unassigned,
                "seats_needed": b.seats_needed,
            })
        })
        .collect();
    slot_list.sort_by(|a, b| {
        b["unfilled"]
            .as_i64()
            .cmp(&a["unfilled"].as_i64())
            .then_with(|| b["total"].as_i64().cmp(&a["total"].as_i64()))
    });

    let fulfillment_rate_pct = if total_requests > 0 {
        (fulfilled * 100) / total_requests
    } else {
        100
    };

    Ok(Json(json!({
        "period": period_label,
        "from": start,
        "to": display_to,
        "totals": {
            "total_requests": total_requests,
            "fulfilled": fulfilled,
            "unfilled": unfilled,
            "completed": completed,
            "cancelled_unassigned": cancelled_unassigned,
            "wheelchair_requests": wheelchair_requests,
            "return_requests": return_requests,
            "seats_unfilled": seats_unfilled,
            "active_drivers": drivers_active.len(),
            "unique_passengers": passengers_seen.len(),
            "fulfillment_rate_pct": fulfillment_rate_pct,
        },
        "by_area": area_list,
        "by_slot": slot_list,
    })))
}

fn mask_webhook_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(parsed) = url::Url::parse(trimmed) {
        let host = parsed.host_str().unwrap_or("");
        let port = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
        return format!("{}://{host}{port}/***", parsed.scheme());
    }
    "***".to_string()
}

fn validate_telegram_bot_token(tok: &str) -> bool {
    let Some((bot_id, secret)) = tok.split_once(':') else {
        return false;
    };
    !bot_id.is_empty()
        && bot_id.bytes().all(|b| b.is_ascii_digit())
        && secret.len() >= 10
        && secret
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

async fn get_alert_settings(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let _ = crate::web_push::ensure_vapid_keys(&st.db).await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets WHERE name IN ('telegram_bot_token', 'telegram_bot_username', 'alert_webhook_url', 'vapid_public_key', 'vapid_private_key')",
    )
    .fetch_all(&st.db)
    .await?;

    let mut tg_configured = false;
    let mut tg_username = String::new();
    let mut webhook_url = String::new();
    let mut vapid_public_key = String::new();
    let mut vapid_private_key = String::new();
    for (k, v) in rows {
        match k.as_str() {
            "telegram_bot_token" => tg_configured = !v.trim().is_empty(),
            "telegram_bot_username" => tg_username = v,
            "alert_webhook_url" => webhook_url = v,
            "vapid_public_key" => vapid_public_key = v,
            "vapid_private_key" => vapid_private_key = v,
            _ => {}
        }
    }
    let vapid_private_configured =
        crate::web_push::VapidSigner::from_stored(&vapid_public_key, &vapid_private_key).is_ok();
    let push_subscriptions_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions")
            .fetch_one(&st.db)
            .await?;

    Ok(Json(json!({
        "telegram_bot_configured": tg_configured,
        "telegram_bot_username": tg_username,
        "alert_webhook_configured": !webhook_url.trim().is_empty(),
        "alert_webhook_url": mask_webhook_url(&webhook_url),
        "vapid_public_key": vapid_public_key,
        "vapid_private_configured": vapid_private_configured,
        "push_subscriptions_count": push_subscriptions_count,
    })))
}

#[derive(Deserialize)]
struct UpdateAlertSettingsIn {
    telegram_bot_token: Option<String>,
    clear_telegram_bot_token: Option<bool>,
    telegram_bot_username: Option<String>,
    alert_webhook_url: Option<String>,
    clear_alert_webhook_url: Option<bool>,
    vapid_public_key: Option<String>,
    vapid_private_key: Option<String>,
    rotate_vapid_keys: Option<bool>,
}

async fn update_alert_settings(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UpdateAlertSettingsIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;

    if let Some(tok) = req
        .telegram_bot_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !req.clear_telegram_bot_token.unwrap_or(false) && !validate_telegram_bot_token(tok) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }

    let clear_webhook = req.clear_alert_webhook_url.unwrap_or(false);
    let new_webhook = req.alert_webhook_url.as_deref().map(str::trim);
    if !clear_webhook {
        if let Some(cleaned) = new_webhook.filter(|s| !s.is_empty()) {
            if !crate::util::validate_webhook_target(cleaned, st.cfg.dev_mode).await {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
        }
    }

    let rotate_vapid = req.rotate_vapid_keys.unwrap_or(false);
    let new_vapid_pub = req.vapid_public_key.as_deref().map(str::trim);
    let new_vapid_priv = req
        .vapid_private_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let validated_signer = if !rotate_vapid {
        if let Some(priv_str) = new_vapid_priv {
            let pub_hint = new_vapid_pub.unwrap_or("");
            let signer = crate::web_push::VapidSigner::from_stored(pub_hint, priv_str)
                .map_err(|_| bad(u.lang, ErrorKey::InvalidAction))?;
            Some(signer)
        } else {
            None
        }
    } else {
        None
    };

    let mut tx = st.db.begin().await?;
    if req.clear_telegram_bot_token.unwrap_or(false) {
        sqlx::query("DELETE FROM app_secrets WHERE name='telegram_bot_token'")
            .execute(&mut *tx)
            .await?;
    } else if let Some(tok) = req
        .telegram_bot_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('telegram_bot_token', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(tok)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(username) = req.telegram_bot_username.as_deref() {
        let cleaned = username.trim().trim_start_matches('@');
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('telegram_bot_username', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(cleaned)
        .execute(&mut *tx)
        .await?;
    }

    if clear_webhook || new_webhook == Some("") {
        sqlx::query("DELETE FROM app_secrets WHERE name='alert_webhook_url'")
            .execute(&mut *tx)
            .await?;
    } else if let Some(cleaned) = new_webhook.filter(|s| !s.is_empty()) {
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('alert_webhook_url', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(cleaned)
        .execute(&mut *tx)
        .await?;
    }

    let current_pub: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_secrets WHERE name='vapid_public_key'")
            .fetch_optional(&mut *tx)
            .await?;

    if rotate_vapid {
        let (gen_pub, gen_priv) =
            crate::web_push::generate_vapid_keypair().map_err(AppError::Other)?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_public_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&gen_pub)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_private_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&gen_priv)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM push_subscriptions")
            .execute(&mut *tx)
            .await?;
    } else if let Some(signer) = validated_signer {
        let pub_key = signer.public_key_b64();
        let priv_key = new_vapid_priv.unwrap_or_default();
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_public_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(pub_key)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_private_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(priv_key)
        .execute(&mut *tx)
        .await?;
        if current_pub.as_deref().map(str::trim) != Some(pub_key) {
            sqlx::query("DELETE FROM push_subscriptions")
                .execute(&mut *tx)
                .await?;
        }
    } else if let Some(vapid) = new_vapid_pub {
        // Only block changing an already-stored public key without also providing the
        // private key (mismatched keypair). On a fresh install (current_pub is None)
        // the admin may store just the public key for client-side push config lookup
        // while keeping the private key in an external signer.
        if !vapid.is_empty() {
            if let Some(current) = current_pub.as_deref().map(str::trim) {
                if current != vapid {
                    return Err(bad(u.lang, ErrorKey::InvalidAction));
                }
            } else {
                sqlx::query(
                    "INSERT INTO app_secrets (name, value) VALUES ('vapid_public_key', $1)
                     ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
                )
                .bind(vapid)
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "settings.alerts",
        "app_secrets",
        None,
        if rotate_vapid {
            "rotated VAPID push keys"
        } else {
            "updated alert channels"
        },
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
