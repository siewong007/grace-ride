//! Admin member directory, role assignment, status, and group membership management.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{self, parse_roles, Act, Page};
use crate::state::AppState;
use crate::util::{audit, escape_like, minutes_ago, now, require, require_one};

#[derive(Deserialize)]
pub(super) struct UserQuery {
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
pub(super) async fn list_users(
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
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let to_json = |(
        id,
        name,
        email,
        phone,
        roles,
        status,
        created,
        pk,
        groups,
        photo,
        color,
        icon,
        family_name,
    ): Row| {
        if staff {
            json!({"id": id, "name": name, "email": email, "phone": phone, "roles": parse_roles(&roles), "status": status,
                   "created_at": created, "passkeys": pk, "groups": groups.unwrap_or_default(),
                   "photo_file_id": photo, "avatar_color": color, "avatar_icon": icon,
                   "family_name": family_name})
        } else {
            json!({"id": id, "name": name})
        }
    };
    let select = "SELECT u.id, u.name, u.email, u.phone, u.roles, u.status, u.created_at, (SELECT COUNT(*) FROM passkeys p WHERE p.user_id=u.id),
                (SELECT string_agg(g.name, '、') FROM group_members gm JOIN groups g ON g.id=gm.group_id WHERE gm.user_id=u.id),
                u.photo_file_id, u.avatar_color, u.avatar_icon,
                (SELECT f.name FROM family_members fm JOIN families f ON f.id = fm.family_id WHERE fm.user_id = u.id)
             FROM users u";

    if q.page.is_some() || q.limit.is_some() {
        let count_sql = format!("SELECT COUNT(*) FROM users u WHERE {where_sql}");
        let mut count_query =
            sqlx::query_as::<_, (i64,)>(sqlx::AssertSqlSafe(&*count_sql)).bind(&like);
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
        let mut query = sqlx::query_as::<_, Row>(sqlx::AssertSqlSafe(&*sql)).bind(&like);
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
    let mut query = sqlx::query_as::<_, Row>(sqlx::AssertSqlSafe(&*sql)).bind(&like);
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
pub(super) struct UserUpdate {
    roles: Vec<String>,
    status: String,
    group_ids: Option<Vec<i64>>,
    /// Sent by a Vice-President when promoting an ordinary member to administrator,
    /// after the President has acknowledged the appointment.
    #[serde(default)]
    president_acknowledged: bool,
}

pub(super) async fn update_user(
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
        sqlx::query("DELETE FROM mobile_push_tokens WHERE user_id=$1")
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
    if old_status == "pending" && req.status == "active" {
        // The member is waiting on the pending screen; this also refreshes their session live.
        crate::util::notify(
            &st.db,
            id,
            ("账号已通过审核", "Your account is approved"),
            (
                "欢迎加入 Grace Ride！现在你可以预约用车和报名活动了。",
                "Welcome to Grace Ride! You can now book rides and sign up for events.",
            ),
            "/",
        )
        .await;
    }
    crate::util::notify_many(
        &st.db,
        &released,
        ("司机无法出车", "Driver cannot drive"),
        (
            "交通组正在为您安排其他司机，请留意通知。",
            "Transport staff are arranging another driver for you; please watch for notifications.",
        ),
        "/rides",
    )
    .await;
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
