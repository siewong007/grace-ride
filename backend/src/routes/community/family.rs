//! One named family list per member. The owner creates, renames, deletes, and removes.
//! Rides and church groups are untouched.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, check_name, now};

#[derive(sqlx::FromRow)]
struct FamilyRow {
    id: i64,
    name: String,
    owner_id: i64,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    user_id: i64,
    name: String,
    status: String,
}

#[derive(Deserialize)]
pub(super) struct NameIn {
    name: String,
}

async fn my_family(st: &AppState, user_id: i64) -> AppResult<Option<FamilyRow>> {
    let row = sqlx::query_as(
        "SELECT f.id, f.name, f.owner_id
         FROM families f
         JOIN family_members fm ON fm.family_id = f.id
         WHERE fm.user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&st.db)
    .await?;
    Ok(row)
}

fn family_json(row: &FamilyRow, members: &[MemberRow]) -> Value {
    json!({
        "family": { "id": row.id, "name": row.name, "owner_id": row.owner_id },
        "members": members.iter().map(|m| json!({
            "user_id": m.user_id,
            "name": m.name,
            "status": m.status,
        })).collect::<Vec<_>>(),
        "requests": [],
        "invites": [],
        "incoming": Value::Null,
    })
}

pub(super) async fn get_family(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let Some(row) = my_family(&st, u.id).await? else {
        return Ok(Json(json!({
            "family": null,
            "members": [],
            "requests": [],
            "invites": [],
            "incoming": null,
        })));
    };
    let members: Vec<MemberRow> = sqlx::query_as(
        "SELECT fm.user_id, u.name, u.status
         FROM family_members fm
         JOIN users u ON u.id = fm.user_id
         WHERE fm.family_id = $1
         ORDER BY fm.created_at, fm.user_id",
    )
    .bind(row.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(family_json(&row, &members)))
}

pub(super) async fn create_family(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<NameIn>,
) -> AppResult<Json<Value>> {
    if u.status != "active" {
        return Err(bad(u.lang, ErrorKey::FamilyActiveOnly));
    }
    if my_family(&st, u.id).await?.is_some() {
        return Err(bad(u.lang, ErrorKey::AlreadyInFamily));
    }
    let name = req.name.trim();
    check_name(u.lang, name)?;
    let ts = now();
    let mut tx = st.db.begin().await?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO families (name, owner_id, created_at) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(u.id)
    .bind(&ts)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(u.id)
        .bind(&ts)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "family.create",
        "family",
        Some(id),
        format!("family_id={id}"),
    )
    .await;
    Ok(Json(json!({ "id": id, "name": name })))
}

pub(super) async fn rename_family(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<NameIn>,
) -> AppResult<Json<Value>> {
    let Some(row) = my_family(&st, u.id).await? else {
        return Err(not_found(u.lang));
    };
    if row.owner_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let name = req.name.trim();
    check_name(u.lang, name)?;
    sqlx::query("UPDATE families SET name = $1 WHERE id = $2")
        .bind(name)
        .bind(row.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "family.rename",
        "family",
        Some(row.id),
        format!("family_id={}", row.id),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn delete_family(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let Some(row) = my_family(&st, u.id).await? else {
        return Err(not_found(u.lang));
    };
    if row.owner_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    sqlx::query("DELETE FROM families WHERE id = $1")
        .bind(row.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "family.delete",
        "family",
        Some(row.id),
        format!("family_id={}", row.id),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn leave_family(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let Some(row) = my_family(&st, u.id).await? else {
        return Err(not_found(u.lang));
    };
    if row.owner_id == u.id {
        return Err(bad(u.lang, ErrorKey::FamilyOwnerCannotLeave));
    }
    sqlx::query("DELETE FROM family_members WHERE family_id = $1 AND user_id = $2")
        .bind(row.id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "family.leave",
        "family",
        Some(row.id),
        format!("family_id={} user_id={}", row.id, u.id),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn remove_member(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(user_id): Path<i64>,
) -> AppResult<Json<Value>> {
    let Some(row) = my_family(&st, u.id).await? else {
        return Err(not_found(u.lang));
    };
    if row.owner_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    if user_id == row.owner_id {
        return Err(bad(u.lang, ErrorKey::FamilyOwnerCannotLeave));
    }
    let removed = sqlx::query("DELETE FROM family_members WHERE family_id = $1 AND user_id = $2")
        .bind(row.id)
        .bind(user_id)
        .execute(&st.db)
        .await?
        .rows_affected();
    if removed == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "family.member.remove",
        "family",
        Some(row.id),
        format!("family_id={} user_id={user_id}", row.id),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
