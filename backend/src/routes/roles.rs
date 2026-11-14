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
        "SELECT r, COUNT(*) FROM users u, unnest(string_to_array(u.roles, ',')) AS r
         WHERE u.status = 'active' GROUP BY r",
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

async fn check_name(st: &AppState, lang: Lang, name: &str, own_key: &str) -> AppResult<()> {
    let n = name.chars().count();
    if n == 0 || n > MAX_ROLE_NAME_CHARS {
        return Err(bad(lang, ErrorKey::RoleNameRequired));
    }
    let taken: Option<(String,)> =
        sqlx::query_as("SELECT key FROM roles WHERE lower(name) = lower($1) AND key <> $2")
            .bind(name)
            .bind(own_key)
            .fetch_optional(&st.db)
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

async fn role_perms(st: &AppState, key: &str) -> AppResult<PermSet> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT permission FROM role_permissions WHERE role_key = $1")
            .bind(key)
            .fetch_all(&st.db)
            .await?;
    Ok(PermSet::from_keys(rows.iter().map(|r| r.0.as_str())))
}

/// What the caller would hold if role `key` granted `replacement` (`None`: role gone).
async fn caller_after(
    st: &AppState,
    u: &CurrentUser,
    key: &str,
    replacement: Option<&PermSet>,
) -> AppResult<PermSet> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT permission FROM role_permissions
         WHERE (role_key = $1 OR role_key = ANY($2)) AND role_key <> $3",
    )
    .bind(MEMBER_ROLE)
    .bind(&u.roles)
    .bind(key)
    .fetch_all(&st.db)
    .await?;
    let mut after = PermSet::from_keys(rows.iter().map(|r| r.0.as_str()));
    if let Some(set) = replacement {
        after.union(set);
    }
    Ok(after)
}

/// A change to a role the caller holds must not take away their own right to manage
/// roles (someone must always hold it) nor hand them finance permissions.
async fn guard_own_access(
    st: &AppState,
    u: &CurrentUser,
    key: &str,
    replacement: Option<&PermSet>,
) -> AppResult<()> {
    if key != MEMBER_ROLE && !u.roles.iter().any(|r| r == key) {
        return Ok(());
    }
    let after = caller_after(st, u, key, replacement).await?;
    if u.perms.has(Page::AdminRoles, Act::Update) && !after.has(Page::AdminRoles, Act::Update) {
        return Err(forbidden(u.lang, ErrorKey::CannotRemoveOwnRoleAccess));
    }
    if after.new_finance(&u.perms) {
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
    check_name(&st, u.lang, &name, "").await?;
    let set = parse_permissions(u.lang, &req.permissions)?;
    // Random key: role names may be 中文 and may be renamed, the key never changes.
    let key = format!("c_{}", hex::encode(rand::random::<[u8; 5]>()));
    let mut tx = st.db.begin().await?;
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
    let (old_name, is_system): (String, i64) =
        sqlx::query_as("SELECT name, is_system FROM roles WHERE key = $1")
            .bind(&key)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    let set = parse_permissions(u.lang, &req.permissions)?;
    // The super admin role is fixed: it holds every permission and nothing else is stored.
    if key == SUPERADMIN_ROLE || perms::locked(&key).into_iter().any(|p| !set.contains(p)) {
        return Err(bad(u.lang, ErrorKey::RolePermissionLocked));
    }
    guard_own_access(&st, &u, &key, Some(&set)).await?;
    // Built-in roles are named by the app in each member's language.
    let name = if is_system == 1 {
        old_name
    } else {
        let name = req.name.as_deref().unwrap_or_default().trim().to_string();
        check_name(&st, u.lang, &name, &key).await?;
        name
    };
    let before = role_perms(&st, &key).await?;

    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE roles SET name = $1, description = $2 WHERE key = $3")
        .bind(&name)
        .bind(description(&req))
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    write_permissions(&mut tx, &key, &set).await?;
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
    let (name, is_system): (String, i64) =
        sqlx::query_as("SELECT name, is_system FROM roles WHERE key = $1")
            .bind(&key)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    if is_system == 1 || key == ADMIN_ROLE {
        return Err(bad(u.lang, ErrorKey::SystemRoleCannotBeDeleted));
    }
    guard_own_access(&st, &u, &key, None).await?;
    let mut tx = st.db.begin().await?;
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
