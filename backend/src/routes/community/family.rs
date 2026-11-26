//! One named family list per member. The owner creates, renames, deletes, and removes.
//! Rides and church groups are untouched.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{
    audit, check_name, check_phone, in_days, minutes_ago, notify, now, phone_key, random_token,
};

/// Members, open requests, and unused unexpired invites all count toward this.
pub const FAMILY_CAP: i64 = 8;

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

const USER_PHONE_KEY: &str = "CASE WHEN regexp_replace(u.phone, '[^0-9]', '', 'g') LIKE '0%' \
     THEN '60' || substring(regexp_replace(u.phone, '[^0-9]', '', 'g') from 2) \
     ELSE regexp_replace(u.phone, '[^0-9]', '', 'g') END";

const PHONE_KEY: &str = "CASE WHEN regexp_replace(phone, '[^0-9]', '', 'g') LIKE '0%' \
     THEN '60' || substring(regexp_replace(phone, '[^0-9]', '', 'g') from 2) \
     ELSE regexp_replace(phone, '[^0-9]', '', 'g') END";

fn member_json(members: &[MemberRow]) -> Vec<Value> {
    members
        .iter()
        .map(|m| {
            json!({
                "user_id": m.user_id,
                "name": m.name,
                "status": m.status,
            })
        })
        .collect()
}

async fn load_members(st: &AppState, family_id: i64) -> AppResult<Vec<MemberRow>> {
    let members = sqlx::query_as(
        "SELECT fm.user_id, u.name, u.status
         FROM family_members fm
         JOIN users u ON u.id = fm.user_id
         WHERE fm.family_id = $1
         ORDER BY fm.created_at, fm.user_id",
    )
    .bind(family_id)
    .fetch_all(&st.db)
    .await?;
    Ok(members)
}

async fn owner_requests(st: &AppState, family_id: i64) -> AppResult<Vec<Value>> {
    let rows: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT r.id, r.user_id, u.name
         FROM family_requests r
         JOIN users u ON u.id = r.user_id
         WHERE r.family_id = $1
         ORDER BY r.created_at, r.id",
    )
    .bind(family_id)
    .fetch_all(&st.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, user_id, name)| json!({ "id": id, "user_id": user_id, "name": name }))
        .collect())
}

async fn owner_invites(st: &AppState, family_id: i64) -> AppResult<Vec<Value>> {
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, phone, expires_at, token
         FROM invites
         WHERE family_id = $1 AND kind = 'family' AND used_at IS NULL AND expires_at > $2
         ORDER BY id",
    )
    .bind(family_id)
    .bind(now())
    .fetch_all(&st.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, phone, expires_at, token)| {
            json!({
                "id": id,
                "phone": phone,
                "expires_at": expires_at,
                "url": format!("{}/invite/{token}", st.cfg.public_url),
            })
        })
        .collect())
}

async fn incoming_for(st: &AppState, user_id: i64) -> AppResult<Value> {
    let row: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT r.id, f.name, owner.name
         FROM family_requests r
         JOIN families f ON f.id = r.family_id
         JOIN users owner ON owner.id = f.owner_id
         WHERE r.user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&st.db)
    .await?;
    Ok(match row {
        Some((id, family_name, owner_name)) => json!({
            "id": id,
            "family_name": family_name,
            "owner_name": owner_name,
        }),
        None => Value::Null,
    })
}

/// Members plus this family's open requests and unused unexpired invites.
/// `except_request` is excluded (pass 0 to exclude nothing).
async fn counted<'e, E>(
    executor: E,
    family_id: i64,
    except_request: i64,
) -> Result<i64, sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_scalar(
        "SELECT
            (SELECT COUNT(*) FROM family_members WHERE family_id = $1)
          + (SELECT COUNT(*) FROM family_requests WHERE family_id = $1 AND id <> $2)
          + (SELECT COUNT(*) FROM invites
             WHERE family_id = $1 AND kind = 'family' AND used_at IS NULL AND expires_at > $3)",
    )
    .bind(family_id)
    .bind(except_request)
    .bind(now())
    .fetch_one(executor)
    .await
}

pub(super) async fn get_family(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let row = my_family(&st, u.id).await?;
    let (members, requests, invites, incoming) = if let Some(row) = &row {
        let members = load_members(&st, row.id).await?;
        let (requests, invites) = if row.owner_id == u.id {
            (
                owner_requests(&st, row.id).await?,
                owner_invites(&st, row.id).await?,
            )
        } else {
            (Vec::new(), Vec::new())
        };
        (members, requests, invites, Value::Null)
    } else {
        (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            incoming_for(&st, u.id).await?,
        )
    };
    let family = row
        .as_ref()
        .map(|row| json!({ "id": row.id, "name": row.name, "owner_id": row.owner_id }));
    Ok(Json(json!({
        "family": family,
        "members": member_json(&members),
        "requests": requests,
        "invites": invites,
        "incoming": incoming,
    })))
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
    match sqlx::query(
        "INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1, $2, $3)",
    )
    .bind(id)
    .bind(u.id)
    .bind(&ts)
    .execute(&mut *tx)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::AlreadyInFamily));
        }
        Err(e) => return Err(e.into()),
    }
    sqlx::query("DELETE FROM family_requests WHERE user_id = $1")
        .bind(u.id)
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

#[derive(Deserialize)]
pub(super) struct PhoneIn {
    phone: String,
}

async fn require_owner(st: &AppState, u: &CurrentUser) -> AppResult<FamilyRow> {
    let Some(row) = my_family(st, u.id).await? else {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    };
    if row.owner_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    if u.status != "active" {
        return Err(bad(u.lang, ErrorKey::FamilyActiveOnly));
    }
    Ok(row)
}

pub(super) async fn add_person(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<PhoneIn>,
) -> AppResult<Json<Value>> {
    let phone = req.phone.trim();
    if phone.is_empty() || check_phone(u.lang, phone).is_err() {
        return Err(bad(u.lang, ErrorKey::PhoneInvalid));
    }
    let key = phone_key(phone);
    if !(8..=15).contains(&key.len()) {
        return Err(bad(u.lang, ErrorKey::PhoneInvalid));
    }
    let family = require_owner(&st, &u).await?;
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT id FROM families WHERE id = $1 FOR UPDATE")
        .bind(family.id)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('grace_ride_family_phone'), hashtext($1))")
        .bind(&key)
        .execute(&mut *tx)
        .await?;
    let stored_phone: String = sqlx::query_scalar("SELECT phone FROM users WHERE id = $1")
        .bind(u.id)
        .fetch_one(&mut *tx)
        .await?;
    if !stored_phone.trim().is_empty() && phone_key(&stored_phone) == key {
        return Err(bad(u.lang, ErrorKey::FamilyOwnPhone));
    }

    let existing_request: Option<(i64, String)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT r.id, u.name
         FROM family_requests r
         JOIN users u ON u.id = r.user_id
         WHERE r.family_id = $1 AND u.phone <> '' AND {USER_PHONE_KEY} = $2"
    ))
    .bind(family.id)
    .bind(&key)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((id, name)) = existing_request {
        tx.commit().await?;
        return Ok(Json(json!({ "kind": "request", "id": id, "name": name })));
    }

    let existing_invite: Option<(i64, String, String, String)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT id, phone, expires_at, token
         FROM invites
         WHERE family_id = $1 AND kind = 'family' AND used_at IS NULL AND expires_at > $2
           AND phone <> '' AND {PHONE_KEY} = $3"
    ))
    .bind(family.id)
    .bind(now())
    .bind(&key)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((id, stored, expires_at, token)) = existing_invite {
        tx.commit().await?;
        return Ok(Json(json!({
            "kind": "invite",
            "id": id,
            "phone": stored,
            "expires_at": expires_at,
            "url": format!("{}/invite/{token}", st.cfg.public_url),
        })));
    }

    let matches: Vec<(i64, String)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT id, name FROM users
         WHERE status IN ('active', 'pending') AND phone <> '' AND {PHONE_KEY} = $1"
    ))
    .bind(&key)
    .fetch_all(&mut *tx)
    .await?;
    if matches.len() > 1 {
        return Err(bad(u.lang, ErrorKey::FamilyPhoneAmbiguous));
    }
    if let Some((user_id, _)) = matches.first() {
        let joined: Option<i64> =
            sqlx::query_scalar("SELECT family_id FROM family_members WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        if joined.is_some() {
            return Err(bad(u.lang, ErrorKey::AlreadyInFamily));
        }
        let open: Option<i64> =
            sqlx::query_scalar("SELECT id FROM family_requests WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        if open.is_some() {
            return Err(bad(u.lang, ErrorKey::FamilyRequestPending));
        }
    }
    let other_link: Option<i64> = sqlx::query_scalar(crate::dyn_sql!(
        "SELECT id FROM invites
         WHERE kind = 'family' AND family_id <> $1 AND used_at IS NULL AND expires_at > $2
           AND phone <> '' AND {PHONE_KEY} = $3
         LIMIT 1"
    ))
    .bind(family.id)
    .bind(now())
    .bind(&key)
    .fetch_optional(&mut *tx)
    .await?;
    if other_link.is_some() {
        return Err(bad(u.lang, ErrorKey::FamilyRequestPending));
    }
    if matches.is_empty() {
        let disabled: i64 = sqlx::query_scalar(crate::dyn_sql!(
            "SELECT COUNT(*) FROM users WHERE status = 'disabled' AND phone <> '' AND {PHONE_KEY} = $1"
        ))
        .bind(&key)
        .fetch_one(&mut *tx)
        .await?;
        if disabled > 0 {
            // Release tx connection before acquiring another connection for audit() on small pools.
            drop(tx);
            audit(
                &st.db,
                Some(u.id),
                "family.phone_unavailable",
                "family",
                Some(family.id),
                "disabled member lookup",
            )
            .await;
            return Err(bad(u.lang, ErrorKey::FamilyPhoneUnavailable));
        }
    }
    let counted = counted(&mut *tx, family.id, 0).await?;
    if counted >= FAMILY_CAP {
        return Err(bad(u.lang, ErrorKey::FamilyFull { max: FAMILY_CAP }));
    }

    if let Some((user_id, name)) = matches.into_iter().next() {
        let id: i64 = match sqlx::query_scalar(
            "INSERT INTO family_requests (family_id, user_id, created_at) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(family.id)
        .bind(user_id)
        .bind(now())
        .fetch_one(&mut *tx)
        .await
        {
            Ok(id) => id,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                return Err(conflict(u.lang, ErrorKey::FamilyRequestPending));
            }
            Err(e) => return Err(e.into()),
        };
        tx.commit().await?;
        let zh = format!("{}邀请你加入家庭「{}」", u.name, family.name);
        let en = format!(
            "{} invited you to join the family \"{}\"",
            u.name, family.name
        );
        notify(
            &st.db,
            user_id,
            ("家庭邀请", "Family request"),
            (&zh, &en),
            "/me/family",
        )
        .await;
        audit(
            &st.db,
            Some(u.id),
            "family.request",
            "family",
            Some(family.id),
            format!("family_id={} user_id={user_id}", family.id),
        )
        .await;
        return Ok(Json(json!({ "kind": "request", "id": id, "name": name })));
    }

    let token = random_token(32);
    let expires_at = in_days(3);
    let id: i64 = match sqlx::query_scalar(
        "INSERT INTO invites
            (token, kind, email, name, roles, group_ids, family_id, phone, created_by, expires_at, created_at)
         VALUES ($1, 'family', '', '', '', '[]', $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(&token)
    .bind(family.id)
    .bind(phone)
    .bind(u.id)
    .bind(&expires_at)
    .bind(now())
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(conflict(u.lang, ErrorKey::FamilyRequestPending));
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "family.invite",
        "family",
        Some(family.id),
        format!("family_id={} invite_id={id}", family.id),
    )
    .await;
    Ok(Json(json!({
        "kind": "invite",
        "id": id,
        "phone": phone,
        "expires_at": expires_at,
        "url": format!("{}/invite/{token}", st.cfg.public_url),
    })))
}

pub(super) async fn accept_request(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let mut tx = st.db.begin().await?;
    let row: Option<(i64, i64)> =
        sqlx::query_as("SELECT family_id, user_id FROM family_requests WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((family_id, user_id)) = row else {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    };
    if user_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    sqlx::query("SELECT id FROM families WHERE id = $1 FOR UPDATE")
        .bind(family_id)
        .fetch_one(&mut *tx)
        .await?;
    let joined: Option<i64> =
        sqlx::query_scalar("SELECT family_id FROM family_members WHERE user_id = $1")
            .bind(u.id)
            .fetch_optional(&mut *tx)
            .await?;
    if joined.is_some() {
        return Err(bad(u.lang, ErrorKey::AlreadyInFamily));
    }
    let total = counted(&mut *tx, family_id, id).await?;
    if total + 1 > FAMILY_CAP {
        return Err(bad(u.lang, ErrorKey::FamilyFull { max: FAMILY_CAP }));
    }
    let ts = now();
    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1, $2, $3)")
        .bind(family_id)
        .bind(u.id)
        .bind(&ts)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM family_requests WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "family.request.accept",
        "family",
        Some(family_id),
        format!("family_id={family_id} user_id={}", u.id),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn decline_request(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let row: Option<(i64, i64)> =
        sqlx::query_as("SELECT family_id, user_id FROM family_requests WHERE id = $1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    let Some((family_id, user_id)) = row else {
        return Err(conflict(u.lang, ErrorKey::RequestAlreadyHandled));
    };
    if user_id != u.id {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    sqlx::query("DELETE FROM family_requests WHERE id = $1")
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "family.request.decline",
        "family",
        Some(family_id),
        format!("family_id={family_id} user_id={user_id}"),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn cancel_request(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let family = require_owner(&st, &u).await?;
    let removed = sqlx::query("DELETE FROM family_requests WHERE id = $1 AND family_id = $2")
        .bind(id)
        .bind(family.id)
        .execute(&st.db)
        .await?
        .rows_affected();
    if removed == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "family.request.cancel",
        "family",
        Some(family.id),
        format!("family_id={} request_id={id}", family.id),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn revoke_invite(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let family = require_owner(&st, &u).await?;
    let updated = sqlx::query(
        "UPDATE invites SET expires_at = $1
         WHERE id = $2 AND family_id = $3 AND kind = 'family' AND used_at IS NULL",
    )
    .bind(minutes_ago(1))
    .bind(id)
    .bind(family.id)
    .execute(&st.db)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "family.invite.revoke",
        "family",
        Some(family.id),
        format!("family_id={} invite_id={id}", family.id),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
