//! Frequent proxy-rider management (riders a member books on behalf of).

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, not_found, AppResult};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{now, require, ErrorKey};

// ---------------- Frequent Proxy Riders ----------------

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct ProxyRiderRow {
    id: i64,
    user_id: i64,
    name: String,
    phone: String,
    relationship: String,
    notes: String,
    area_id: Option<i64>,
    pickup_address: String,
    gate_notes: String,
    caregiver_user_id: Option<i64>,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub(super) struct CreateProxyRiderReq {
    name: String,
    #[serde(default)]
    phone: Option<String>,
    #[serde(default)]
    relationship: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    area_id: Option<i64>,
    #[serde(default)]
    pickup_address: Option<String>,
    #[serde(default)]
    gate_notes: Option<String>,
    #[serde(default)]
    caregiver_user_id: Option<i64>,
}

pub(super) async fn list_proxy_riders(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    crate::util::require(&u, crate::perms::Page::Rides, crate::perms::Act::View)?;
    let rows: Vec<ProxyRiderRow> = sqlx::query_as(
        "SELECT id, user_id, name, phone, relationship, notes, area_id, pickup_address, gate_notes, caregiver_user_id, created_at, updated_at
         FROM proxy_riders WHERE user_id=$1 ORDER BY name",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows)))
}

struct ProxyFields<'a> {
    name: &'a str,
    phone: &'a str,
    relationship: &'a str,
    notes: &'a str,
    pickup_address: &'a str,
    gate_notes: &'a str,
    area_id: Option<i64>,
    caregiver_user_id: Option<i64>,
}

async fn validate_proxy_fields(db: &PgPool, u: &CurrentUser, f: &ProxyFields<'_>) -> AppResult<()> {
    crate::util::check_name(u.lang, f.name)?;
    crate::util::check_phone(u.lang, f.phone)?;
    if f.relationship.chars().count() > 60
        || f.notes.chars().count() > 500
        || f.pickup_address.chars().count() > 200
        || f.gate_notes.chars().count() > 300
    {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if let Some(aid) = f.area_id {
        let ok: Option<(i64,)> = sqlx::query_as("SELECT id FROM areas WHERE id=$1 AND active=1")
            .bind(aid)
            .fetch_optional(db)
            .await?;
        if ok.is_none() {
            return Err(bad(u.lang, ErrorKey::PickupAreaRequired));
        }
    }
    if let Some(cg) = f.caregiver_user_id {
        if cg == u.id {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        let ok: Option<(i64,)> = sqlx::query_as(
            "SELECT cg.id FROM users cg
             WHERE cg.id=$1 AND cg.status='active'
               AND EXISTS (
                   SELECT 1 FROM group_members g1
                   JOIN group_members g2 ON g2.group_id = g1.group_id
                   WHERE g1.user_id = $1 AND g2.user_id = $2
               )",
        )
        .bind(cg)
        .bind(u.id)
        .fetch_optional(db)
        .await?;
        if ok.is_none() {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    Ok(())
}

pub(super) async fn create_proxy_rider(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CreateProxyRiderReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::Create)?;
    let name = req.name.trim();
    if name.is_empty() {
        return Err(bad(u.lang, ErrorKey::RiderNameRequired));
    }
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM proxy_riders WHERE user_id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if count.0 >= 50 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let phone = req.phone.map(|s| s.trim().to_string()).unwrap_or_default();
    let relationship = req
        .relationship
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let notes = req.notes.map(|s| s.trim().to_string()).unwrap_or_default();
    let pickup_address = req
        .pickup_address
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let gate_notes = req
        .gate_notes
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    validate_proxy_fields(
        &st.db,
        &u,
        &ProxyFields {
            name,
            phone: &phone,
            relationship: &relationship,
            notes: &notes,
            pickup_address: &pickup_address,
            gate_notes: &gate_notes,
            area_id: req.area_id,
            caregiver_user_id: req.caregiver_user_id,
        },
    )
    .await?;
    let dup: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM proxy_riders
             WHERE user_id = $1 AND LOWER(BTRIM(name)) = LOWER(BTRIM($2))
         )",
    )
    .bind(u.id)
    .bind(name)
    .fetch_one(&st.db)
    .await?;
    if dup {
        return Err(conflict(u.lang, ErrorKey::InvalidAction));
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO proxy_riders (user_id, name, phone, relationship, notes, area_id, pickup_address, gate_notes, caregiver_user_id, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
    )
    .bind(u.id)
    .bind(name)
    .bind(phone)
    .bind(relationship)
    .bind(notes)
    .bind(req.area_id)
    .bind(pickup_address)
    .bind(gate_notes)
    .bind(req.caregiver_user_id)
    .bind(now())
    .bind(now())
    .fetch_one(&st.db)
    .await
    .map_err(|e| {
        if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
            conflict(u.lang, ErrorKey::InvalidAction)
        } else {
            e.into()
        }
    })?;
    Ok(Json(json!({ "id": id, "ok": true })))
}

pub(super) async fn update_proxy_rider(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<CreateProxyRiderReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::Update)?;
    let name = req.name.trim();
    if name.is_empty() {
        return Err(bad(u.lang, ErrorKey::RiderNameRequired));
    }
    let phone = req.phone.unwrap_or_default().trim().to_string();
    let relationship = req.relationship.unwrap_or_default().trim().to_string();
    let notes = req.notes.unwrap_or_default().trim().to_string();
    let pickup_address = req.pickup_address.unwrap_or_default().trim().to_string();
    let gate_notes = req.gate_notes.unwrap_or_default().trim().to_string();
    validate_proxy_fields(
        &st.db,
        &u,
        &ProxyFields {
            name,
            phone: &phone,
            relationship: &relationship,
            notes: &notes,
            pickup_address: &pickup_address,
            gate_notes: &gate_notes,
            area_id: req.area_id,
            caregiver_user_id: req.caregiver_user_id,
        },
    )
    .await?;
    let dup: bool = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM proxy_riders
             WHERE user_id = $1 AND id <> $2 AND LOWER(BTRIM(name)) = LOWER(BTRIM($3))
         )",
    )
    .bind(u.id)
    .bind(id)
    .bind(name)
    .fetch_one(&st.db)
    .await?;
    if dup {
        return Err(conflict(u.lang, ErrorKey::InvalidAction));
    }
    let row: Option<ProxyRiderRow> = sqlx::query_as(
        "UPDATE proxy_riders
         SET name=$1, phone=$2, relationship=$3, notes=$4, area_id=$5, pickup_address=$6, gate_notes=$7, caregiver_user_id=$8, updated_at=$9
         WHERE id=$10 AND user_id=$11
         RETURNING id, user_id, name, phone, relationship, notes, area_id, pickup_address, gate_notes, caregiver_user_id, created_at, updated_at",
    )
    .bind(name)
    .bind(phone)
    .bind(relationship)
    .bind(notes)
    .bind(req.area_id)
    .bind(pickup_address)
    .bind(gate_notes)
    .bind(req.caregiver_user_id)
    .bind(now())
    .bind(id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await
    .map_err(|e| {
        if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
            conflict(u.lang, ErrorKey::InvalidAction)
        } else {
            e.into()
        }
    })?;
    let Some(row) = row else {
        return Err(not_found(u.lang));
    };
    let mut out = json!(row);
    out["ok"] = json!(true);
    Ok(Json(out))
}

pub(super) async fn delete_proxy_rider(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    crate::util::require(&u, crate::perms::Page::Rides, crate::perms::Act::Delete)?;
    let res = sqlx::query("DELETE FROM proxy_riders WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    Ok(Json(json!({ "ok": true })))
}
