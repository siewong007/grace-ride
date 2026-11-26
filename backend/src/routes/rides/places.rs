//! Saved places management (frequently used pickup addresses).

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, not_found, AppResult};
use crate::i18n::Lang;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{now, require, ErrorKey};

// ---------------- Saved Places ----------------

#[derive(sqlx::FromRow, Serialize)]
struct SavedPlaceRow {
    id: i64,
    user_id: i64,
    label: String,
    area_id: Option<i64>,
    area_name: Option<String>,
    address: String,
    gate_notes: String,
    lat: Option<f64>,
    lng: Option<f64>,
    is_default: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
pub(super) struct SavedPlaceIn {
    label: String,
    area_id: Option<i64>,
    address: String,
    #[serde(default)]
    gate_notes: Option<String>,
    #[serde(default)]
    is_default: Option<bool>,
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lng: Option<f64>,
    #[serde(default)]
    clear_coords: bool,
}

fn validate_coords(
    lat: Option<f64>,
    lng: Option<f64>,
    lang: Lang,
) -> AppResult<Option<(f64, f64)>> {
    match (lat, lng) {
        (None, None) => Ok(None),
        (Some(la), Some(ln))
            if la.is_finite()
                && ln.is_finite()
                && (-90.0..=90.0).contains(&la)
                && (-180.0..=180.0).contains(&ln) =>
        {
            Ok(Some((la, ln)))
        }
        _ => Err(bad(lang, ErrorKey::InvalidAction)),
    }
}

pub(super) async fn list_saved_places(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let rows: Vec<SavedPlaceRow> = sqlx::query_as(
        "SELECT sp.id, sp.user_id, sp.label, sp.area_id, a.name AS area_name, sp.address, sp.gate_notes, sp.lat, sp.lng, sp.is_default, sp.created_at, sp.updated_at
         FROM saved_places sp LEFT JOIN areas a ON a.id = sp.area_id
         WHERE sp.user_id=$1 ORDER BY sp.is_default DESC, sp.label ASC, sp.id ASC",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows)))
}

pub(super) async fn create_saved_place(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<SavedPlaceIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::Create)?;
    let label = req.label.trim();
    let address = req.address.trim();
    let gate_notes = req.gate_notes.unwrap_or_default().trim().to_string();
    if label.is_empty()
        || label.chars().count() > 40
        || address.is_empty()
        || address.chars().count() > 200
        || gate_notes.chars().count() > 300
    {
        return Err(bad(u.lang, ErrorKey::AddressAndDestinationRequired));
    }
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM saved_places WHERE user_id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if count.0 >= 50 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if let Some(aid) = req.area_id {
        let ok: Option<(i64,)> = sqlx::query_as("SELECT id FROM areas WHERE id=$1 AND active=1")
            .bind(aid)
            .fetch_optional(&st.db)
            .await?;
        if ok.is_none() {
            return Err(bad(u.lang, ErrorKey::PickupAreaRequired));
        }
    }
    let explicit_coords = validate_coords(req.lat, req.lng, u.lang)?;
    let (lat, lng) = if let Some((la, ln)) = explicit_coords {
        (Some(la), Some(ln))
    } else if req.clear_coords {
        (None, None)
    } else {
        let cached: Option<(Option<f64>, Option<f64>)> =
            sqlx::query_as("SELECT lat, lng FROM geocode_cache WHERE query=$1")
                .bind(crate::routing::cache_key(address))
                .fetch_optional(&st.db)
                .await?;
        cached.unwrap_or((None, None))
    };
    let is_def = if req.is_default.unwrap_or(false) {
        1i64
    } else {
        0i64
    };
    let mut tx = st.db.begin().await?;
    if is_def == 1 {
        sqlx::query("UPDATE saved_places SET is_default=0 WHERE user_id=$1")
            .bind(u.id)
            .execute(&mut *tx)
            .await?;
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO saved_places (user_id, label, area_id, address, gate_notes, lat, lng, is_default, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
    )
    .bind(u.id)
    .bind(label)
    .bind(req.area_id)
    .bind(address)
    .bind(&gate_notes)
    .bind(lat)
    .bind(lng)
    .bind(is_def)
    .bind(now())
    .bind(now())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        json!({ "id": id, "ok": true, "lat": lat, "lng": lng }),
    ))
}

pub(super) async fn update_saved_place(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SavedPlaceIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::Update)?;
    let label = req.label.trim();
    let address = req.address.trim();
    let gate_notes = req.gate_notes.unwrap_or_default().trim().to_string();
    if label.is_empty()
        || label.chars().count() > 40
        || address.is_empty()
        || address.chars().count() > 200
        || gate_notes.chars().count() > 300
    {
        return Err(bad(u.lang, ErrorKey::AddressAndDestinationRequired));
    }
    if let Some(aid) = req.area_id {
        let ok: Option<(i64,)> = sqlx::query_as("SELECT id FROM areas WHERE id=$1 AND active=1")
            .bind(aid)
            .fetch_optional(&st.db)
            .await?;
        if ok.is_none() {
            return Err(bad(u.lang, ErrorKey::PickupAreaRequired));
        }
    }
    let explicit_coords = validate_coords(req.lat, req.lng, u.lang)?;
    let existing: Option<(String, Option<f64>, Option<f64>)> =
        sqlx::query_as("SELECT address, lat, lng FROM saved_places WHERE id=$1 AND user_id=$2")
            .bind(id)
            .bind(u.id)
            .fetch_optional(&st.db)
            .await?;
    let Some((existing_addr, existing_lat, existing_lng)) = existing else {
        return Err(not_found(u.lang));
    };

    let (lat, lng) = if let Some((la, ln)) = explicit_coords {
        (Some(la), Some(ln))
    } else if req.clear_coords {
        (None, None)
    } else {
        let cached: Option<(Option<f64>, Option<f64>)> =
            sqlx::query_as("SELECT lat, lng FROM geocode_cache WHERE query=$1")
                .bind(crate::routing::cache_key(address))
                .fetch_optional(&st.db)
                .await?;
        match cached {
            Some((Some(la), Some(ln))) => (Some(la), Some(ln)),
            _ if crate::routing::cache_key(&existing_addr)
                == crate::routing::cache_key(address) =>
            {
                (existing_lat, existing_lng)
            }
            _ => (None, None),
        }
    };
    let is_def: Option<i64> = req.is_default.map(|b| if b { 1 } else { 0 });
    let mut tx = st.db.begin().await?;
    if is_def == Some(1) {
        sqlx::query("UPDATE saved_places SET is_default=0 WHERE user_id=$1 AND id<>$2")
            .bind(u.id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    let res = sqlx::query(
        "UPDATE saved_places SET label=$1, area_id=$2, address=$3, gate_notes=$4, lat=$5, lng=$6, is_default=COALESCE($7, is_default), updated_at=$8
         WHERE id=$9 AND user_id=$10",
    )
    .bind(label)
    .bind(req.area_id)
    .bind(address)
    .bind(&gate_notes)
    .bind(lat)
    .bind(lng)
    .bind(is_def)
    .bind(now())
    .bind(id)
    .bind(u.id)
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    tx.commit().await?;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn delete_saved_place(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let res = sqlx::query("DELETE FROM saved_places WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    Ok(Json(json!({ "ok": true })))
}
