//! Admin church groups, group membership, and pickup areas CRUD endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, require, Lang};

#[derive(Deserialize)]
pub(super) struct GroupIn {
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

pub(super) async fn create_group(
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

pub(super) async fn update_group(
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

pub(super) async fn delete_group(
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

pub(super) async fn group_members(
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
pub(super) struct MembersIn {
    user_ids: Vec<i64>,
}

pub(super) async fn set_group_members(
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
pub(super) struct AreaIn {
    name: String,
    active: Option<bool>,
}

pub(super) async fn create_area(
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

pub(super) async fn update_area(
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
