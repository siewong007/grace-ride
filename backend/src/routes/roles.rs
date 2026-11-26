//! 角色与权限: roles are editable sets of page × action permissions (see `crate::perms`).
//!
//! Built-in roles can be edited but not deleted; administrators add their own beside
//! them. Members are given roles on the 成员 page (`routes::admin::update_user`).

use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{
    self, Act, Page, Perm, PermSet, ADMIN_ROLE, CATALOG, MEMBER_ROLE, SUPERADMIN_ROLE, SYSTEM_ROLES,
};
use crate::state::AppState;
use crate::util::{audit, require, require_one, Lang};

/// Longest role name accepted; it shows as a chip on member lists.
const MAX_ROLE_NAME_CHARS: usize = 40;
const MAX_ROLE_DESCRIPTION_CHARS: usize = 200;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/roles", get(list).post(create))
        .route("/admin/roles/{key}", put(update).delete(remove))
}

/// The pages and actions that exist, so the matrix editor needs no copy of the catalog.
fn catalog_json() -> Value {
    json!(CATALOG
        .iter()
        .map(|d| json!({
            "page": d.key,
            "area": d.area.key(),
            "actions": d.actions.iter().map(|a| a.key()).collect::<Vec<_>>(),
        }))
        .collect::<Vec<_>>())
}

/// Roles with their permissions. Also read by the 成员 page to offer and label roles.
async fn list(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require_one(
        &u,
        &[
            (Page::AdminRoles, Act::View),
            (Page::AdminMembers, Act::View),
        ],
    )?;
    let mut roles: Vec<(String, String, String, i64, String)> =
        sqlx::query_as("SELECT key, name, description, is_system, created_at FROM roles")
            .fetch_all(&st.db)
            .await?;
    // Built-in roles first, in their fixed order; custom ones oldest first.
    let rank = |key: &str| {
        SYSTEM_ROLES
            .iter()
            .position(|r| *r == key)
            .unwrap_or(SYSTEM_ROLES.len())
    };
    roles.sort_by(|a, b| (rank(&a.0), &a.4, &a.0).cmp(&(rank(&b.0), &b.4, &b.0)));

    let granted: Vec<(String, String)> =
        sqlx::query_as("SELECT role_key, permission FROM role_permissions")
            .fetch_all(&st.db)
            .await?;
    let holders: Vec<(String, i64)> = sqlx::query_as(
        "SELECT mr.role_key, COUNT(*) FROM member_roles mr JOIN users u ON u.id = mr.user_id
         WHERE u.status = 'active' GROUP BY mr.role_key",
    )
    .fetch_all(&st.db)
    .await?;
    let (active,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE status = 'active'")
        .fetch_one(&st.db)
        .await?;

    let caller_super = perms::is_superadmin(&u.roles);
    let out: Vec<Value> = roles
        .into_iter()
        .map(|(key, name, description, is_system, _)| {
            let set = PermSet::from_keys(
                granted
                    .iter()
                    .filter(|(r, _)| *r == key)
                    .map(|(_, p)| p.as_str()),
            );
            let members = if key == MEMBER_ROLE {
                active
            } else {
                holders
                    .iter()
                    .find(|(r, _)| *r == key)
                    .map(|(_, n)| *n)
                    .unwrap_or(0)
            };
            json!({
                "key": key, "name": name, "description": description,
                "system": is_system == 1,
                // Held by every active member; it is not handed out one by one.
                "implicit": key == MEMBER_ROLE,
                // Always holds every permission; nothing about it can be changed.
                "fixed": key == SUPERADMIN_ROLE,
                // Can the caller give this role to a member? Only a super admin hands
                // out the super admin role.
                "assignable": key != MEMBER_ROLE && (key != SUPERADMIN_ROLE || caller_super),
                "permissions": set.keys(),
                "locked": perms::locked(&key).into_iter().map(Perm::key).collect::<Vec<_>>(),
                "members": members,
            })
        })
        .collect();
    Ok(Json(json!({ "catalog": catalog_json(), "roles": out })))
}

#[derive(Deserialize)]
struct RoleIn {
    name: Option<String>,
    description: Option<String>,
    permissions: Vec<String>,
}

fn parse_permissions(lang: Lang, raw: &[String]) -> AppResult<PermSet> {
    raw.iter()
        .map(|k| Perm::parse(k).ok_or_else(|| bad(lang, ErrorKey::InvalidPermission)))
        .collect()
}

async fn check_name<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    lang: Lang,
    name: &str,
    own_key: &str,
) -> AppResult<()> {
    let n = name.chars().count();
    if n == 0 || n > MAX_ROLE_NAME_CHARS {
        return Err(bad(lang, ErrorKey::RoleNameRequired));
    }
    let taken: Option<(String,)> =
        sqlx::query_as("SELECT key FROM roles WHERE lower(name) = lower($1) AND key <> $2")
            .bind(name)
            .bind(own_key)
            .fetch_optional(db)
            .await?;
    if taken.is_some() {
        return Err(bad(lang, ErrorKey::RoleNameExists));
    }
    Ok(())
}

fn description(req: &RoleIn) -> String {
    req.description
        .as_deref()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(MAX_ROLE_DESCRIPTION_CHARS)
        .collect()
}

async fn role_perms_on<'e, E: sqlx::PgExecutor<'e>>(db: E, key: &str) -> AppResult<PermSet> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT permission FROM role_permissions WHERE role_key = $1")
            .bind(key)
            .fetch_all(db)
            .await?;
    Ok(PermSet::from_keys(rows.iter().map(|r| r.0.as_str())))
}

fn holds_settings(set: &PermSet) -> bool {
    set.has(Page::AdminSettings, Act::View) || set.has(Page::AdminSettings, Act::Update)
}

fn adds_settings(set: &PermSet, before: &PermSet) -> bool {
    [Act::View, Act::Update]
        .iter()
        .any(|a| set.has(Page::AdminSettings, *a) && !before.has(Page::AdminSettings, *a))
}

/// Rank rules for creating or editing a role, called with the roles-mutation lock held.
/// The caller's roles are re-read here rather than trusting `u`, which was loaded earlier.
///  - a ranked role is edited only by somebody strictly above it;
///  - a ranked role only gains permissions its editor holds themselves (the super admin
///    holds all);
///  - giving a role the power to manage members or roles is reserved for the super admin,
///    President and Vice-President, like appointing an administrator.
async fn guard_rank(
    conn: &mut sqlx::PgConnection,
    u: &CurrentUser,
    key: &str,
    set: &PermSet,
    before: &PermSet,
) -> AppResult<()> {
    let (raw,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id = $1")
        .bind(u.id)
        .fetch_one(&mut *conn)
        .await?;
    let roles = perms::parse_roles(&raw);
    if !perms::outranks_role(&roles, key) {
        return Err(forbidden(u.lang, ErrorKey::RankTooLow));
    }
    let added: Vec<Perm> = set
        .keys()
        .iter()
        .filter_map(|k| Perm::parse(k))
        .filter(|p| !before.contains(*p))
        .collect();
    if !perms::is_superadmin(&roles) && !added.is_empty() {
        let caller_holds_role = key == MEMBER_ROLE || roles.iter().any(|r| r == key);
        let needs_held_check = perms::role_rank(key).is_some()
            || caller_holds_role
            || added.iter().any(|p| p.0 == Page::AdminAudit);
        if needs_held_check {
            let held = perms::for_roles(&mut *conn, &roles).await?;
            if (perms::role_rank(key).is_some() || caller_holds_role)
                && added.iter().any(|p| !held.contains(*p))
            {
                return Err(forbidden(u.lang, ErrorKey::CannotGrantUnheldPermission));
            }
            if added
                .iter()
                .any(|p| p.0 == Page::AdminAudit && !held.contains(*p))
            {
                return Err(forbidden(u.lang, ErrorKey::CannotGrantUnheldPermission));
            }
        }
    }
    let power = perms::admin_power_keys();
    let old_keys = before.keys();
    let new_keys = set.keys();
    let touches_power = added.iter().any(|p| power.contains(&p.key()))
        || old_keys
            .iter()
            .any(|k| !new_keys.contains(k) && power.contains(k));
    if !perms::can_manage_admins(&roles) && touches_power {
        return Err(forbidden(u.lang, ErrorKey::AdminAppointmentRestricted));
    }
    Ok(())
}

/// What the caller would hold if role `key` granted `replacement` (`None`: role gone).
async fn caller_after<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    roles: &[String],
    key: &str,
    replacement: Option<&PermSet>,
) -> AppResult<PermSet> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT permission FROM role_permissions
         WHERE (role_key = $1 OR role_key = ANY($2)) AND role_key <> $3",
    )
    .bind(MEMBER_ROLE)
    .bind(roles)
    .bind(key)
    .fetch_all(db)
    .await?;
    let mut after = PermSet::from_keys(rows.iter().map(|r| r.0.as_str()));
    if let Some(set) = replacement {
        after.union(set);
    }
    Ok(after)
}

/// A change to a role the caller holds must not take away their own right to manage
/// roles (someone must always hold it) nor hand them finance permissions.
///
/// Call it while holding the roles-mutation advisory lock. It re-reads the caller's
/// roles rather than trusting `u`, which was loaded before the lock: a concurrent
/// "give myself role X" may have committed in between.
async fn guard_own_access(
    conn: &mut sqlx::PgConnection,
    u: &CurrentUser,
    key: &str,
    replacement: Option<&PermSet>,
) -> AppResult<()> {
    let (raw,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id = $1")
        .bind(u.id)
        .fetch_one(&mut *conn)
        .await?;
    let roles = crate::perms::parse_roles(&raw);
    if key != MEMBER_ROLE && !roles.iter().any(|r| r == key) {
        return Ok(());
    }
    let current = crate::perms::for_roles(&mut *conn, &roles).await?;
    let after = caller_after(&mut *conn, &roles, key, replacement).await?;
    if current.has(Page::AdminRoles, Act::Update) && !after.has(Page::AdminRoles, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::CannotRemoveOwnRoleAccess));
    }
    if after.new_finance(&current) {
        return Err(forbidden(u.lang, ErrorKey::CannotGrantSelfFinance));
    }
    Ok(())
}

async fn write_permissions(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    set: &PermSet,
) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM role_permissions WHERE role_key = $1")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "INSERT INTO role_permissions (role_key, permission) SELECT $1, p FROM UNNEST($2::text[]) AS p",
    )
    .bind(key)
    .bind(set.keys())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn create(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<RoleIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRoles, Act::Create)?;
    let name = req.name.as_deref().unwrap_or_default().trim().to_string();
    let set = parse_permissions(u.lang, &req.permissions)?;
    // Random key: role names may be 中文 and may be renamed, the key never changes.
    let key = format!("c_{}", hex::encode(rand::random::<[u8; 5]>()));
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_roles_mutation', 0))")
        .execute(&mut *tx)
        .await?;
    check_name(&mut *tx, u.lang, &name, "").await?;
    let (raw_caller,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id = $1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let caller_roles = perms::parse_roles(&raw_caller);
    // System configuration stays with the super admin, even if another role may
    // create or edit roles.
    if !perms::is_superadmin(&caller_roles) && holds_settings(&set) {
        return Err(forbidden(u.lang, ErrorKey::SettingsSuperadminOnly));
    }
    guard_rank(&mut tx, &u, &key, &set, &PermSet::default()).await?;
    sqlx::query("INSERT INTO roles (key, name, description, is_system) VALUES ($1, $2, $3, 0)")
        .bind(&key)
        .bind(&name)
        .bind(description(&req))
        .execute(&mut *tx)
        .await?;
    write_permissions(&mut tx, &key, &set).await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "role.create",
        "role",
        None,
        format!("{name} ({key}): {}", set.keys().join(", ")),
    )
    .await;
    Ok(Json(json!({ "key": key })))
}

async fn update(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<String>,
    Json(req): Json<RoleIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRoles, Act::Update)?;
    let set = parse_permissions(u.lang, &req.permissions)?;
    // The super admin role is fixed: it holds every permission and nothing else is stored.
    if key == SUPERADMIN_ROLE || perms::locked(&key).into_iter().any(|p| !set.contains(p)) {
        return Err(bad(u.lang, ErrorKey::RolePermissionLocked));
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_roles_mutation', 0))")
        .execute(&mut *tx)
        .await?;
    let (old_name, is_system): (String, i64) =
        sqlx::query_as("SELECT name, is_system FROM roles WHERE key = $1")
            .bind(&key)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(not_found(u.lang))?;
    let before = role_perms_on(&mut *tx, &key).await?;
    let (raw_caller,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id = $1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let caller_roles = perms::parse_roles(&raw_caller);
    // Only the super admin adds system-configuration access to a role.
    if !perms::is_superadmin(&caller_roles) && adds_settings(&set, &before) {
        return Err(forbidden(u.lang, ErrorKey::SettingsSuperadminOnly));
    }
    guard_own_access(&mut tx, &u, &key, Some(&set)).await?;
    guard_rank(&mut tx, &u, &key, &set, &before).await?;
    // Built-in roles are named by the app in each member's language.
    let name = if is_system == 1 {
        old_name
    } else {
        let name = req.name.as_deref().unwrap_or_default().trim().to_string();
        check_name(&mut *tx, u.lang, &name, &key).await?;
        name
    };

    sqlx::query("UPDATE roles SET name = $1, description = $2 WHERE key = $3")
        .bind(&name)
        .bind(description(&req))
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    write_permissions(&mut tx, &key, &set).await?;
    if set.privileged() && !before.privileged() {
        if key == MEMBER_ROLE {
            sqlx::query(
                "UPDATE sessions SET expires_at = $1
                 WHERE expires_at > $1
                   AND user_id IN (SELECT id FROM users WHERE status = 'active')",
            )
            .bind(crate::util::in_hours(12))
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "UPDATE sessions SET expires_at = $1
                 WHERE expires_at > $1
                   AND user_id IN (SELECT user_id FROM member_roles WHERE role_key = $2)",
            )
            .bind(crate::util::in_hours(12))
            .bind(&key)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;

    let (old, new) = (before.keys(), set.keys());
    let added: Vec<&str> = new
        .iter()
        .filter(|k| !old.contains(k))
        .map(String::as_str)
        .collect();
    let removed: Vec<&str> = old
        .iter()
        .filter(|k| !new.contains(k))
        .map(String::as_str)
        .collect();
    audit(
        &st.db,
        Some(u.id),
        "role.update",
        "role",
        None,
        format!("{key}: +[{}] -[{}]", added.join(", "), removed.join(", ")),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn remove(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<String>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRoles, Act::Delete)?;
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_roles_mutation', 0))")
        .execute(&mut *tx)
        .await?;
    let (name, is_system): (String, i64) =
        sqlx::query_as("SELECT name, is_system FROM roles WHERE key = $1")
            .bind(&key)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(not_found(u.lang))?;
    if is_system == 1 || key == ADMIN_ROLE {
        return Err(bad(u.lang, ErrorKey::SystemRoleCannotBeDeleted));
    }
    let before = role_perms_on(&mut *tx, &key).await?;
    let (raw_caller,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id = $1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    let caller_roles = perms::parse_roles(&raw_caller);
    let power = perms::admin_power_keys();
    if !perms::can_manage_admins(&caller_roles) && before.keys().iter().any(|k| power.contains(k)) {
        return Err(forbidden(u.lang, ErrorKey::AdminAppointmentRestricted));
    }
    guard_own_access(&mut tx, &u, &key, None).await?;
    // Members who held the role simply lose it.
    let held = sqlx::query(
        "UPDATE users SET roles = array_to_string(array_remove(string_to_array(roles, ','), $1), ',')
         WHERE $1 = ANY(string_to_array(roles, ','))",
    )
    .bind(&key)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    sqlx::query("DELETE FROM roles WHERE key = $1")
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "role.delete",
        "role",
        None,
        format!("{name} ({key}); taken from {held} members"),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
