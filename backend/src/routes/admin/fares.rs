//! Area-to-area mileage fares. One flat price per pair of areas, the same in both directions.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, now, require};

const MAX_BULK_FARES: usize = 10_000;
const MAX_AREA_NAME_CHARS: usize = 80;

pub(super) fn normalize_area_name(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn area_lookup_key(s: &str) -> String {
    normalize_area_name(s).to_lowercase()
}

pub(super) fn valid_area_name(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= MAX_AREA_NAME_CHARS && !s.chars().any(char::is_control)
}

fn escape_csv_cell(s: &str) -> String {
    crate::util::escape_csv_cell(s)
}

pub(super) async fn list_fares(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminFares, Act::View)?;
    let rows: Vec<(i64, i64, String, i64, String, i64, String)> = sqlx::query_as(
        "SELECT f.id, f.area_a_id, a.name, f.area_b_id, b.name, f.amount_cents, f.updated_at
         FROM area_fares f JOIN areas a ON a.id = f.area_a_id JOIN areas b ON b.id = f.area_b_id
         ORDER BY a.name, b.name",
    )
    .fetch_all(&st.db)
    .await?;
    let fares: Vec<Value> = rows
        .into_iter()
        .map(|(id, a, an, b, bn, amount_cents, updated_at)| {
            json!({
                "id": id,
                "area_a": {"id": a, "name": an},
                "area_b": {"id": b, "name": bn},
                "amount_cents": amount_cents,
                "updated_at": updated_at,
            })
        })
        .collect();
    Ok(Json(json!({ "fares": fares })))
}

#[derive(Deserialize)]
pub(super) struct FareIn {
    area_a_id: i64,
    area_b_id: i64,
    amount_cents: i64,
}

/// Add or change the fare for a pair of areas.
pub(super) async fn put_fare(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<FareIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminFares, Act::Update)?;
    if req.amount_cents < 0 || req.amount_cents > 50_000 {
        return Err(bad(u.lang, ErrorKey::AmountRangeInvalid));
    }
    let (lo, hi) = (
        req.area_a_id.min(req.area_b_id),
        req.area_a_id.max(req.area_b_id),
    );
    let (found,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM areas WHERE id = $1 OR id = $2")
        .bind(lo)
        .bind(hi)
        .fetch_one(&st.db)
        .await?;
    if found != if lo == hi { 1 } else { 2 } {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO area_fares (area_a_id, area_b_id, amount_cents, updated_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (area_a_id, area_b_id) DO UPDATE
         SET amount_cents = EXCLUDED.amount_cents, updated_at = EXCLUDED.updated_at
         RETURNING id",
    )
    .bind(lo)
    .bind(hi)
    .bind(req.amount_cents)
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    audit(
        &st.db,
        Some(u.id),
        "settings.fare",
        "area_fares",
        Some(id),
        format!("{lo}-{hi}: {}", fmt_rm(req.amount_cents)),
    )
    .await;
    Ok(Json(json!({ "id": id })))
}

pub(super) async fn delete_fare(
    State(st): State<AppState>,
    u: CurrentUser,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminFares, Act::Update)?;
    sqlx::query("DELETE FROM area_fares WHERE id = $1")
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "settings.fare_delete",
        "area_fares",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(super) struct BulkFareItem {
    area_a: String,
    area_b: String,
    amount_cents: i64,
}

#[derive(Deserialize)]
pub(super) struct BulkFaresIn {
    fares: Vec<BulkFareItem>,
    #[serde(default)]
    create_missing_areas: bool,
}

/// Bulk upsert area-to-area fares by area name inside a single transaction.
pub(super) async fn bulk_put_fares(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<BulkFaresIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminFares, Act::Update)?;
    if req.fares.is_empty() {
        return Err(bad(u.lang, ErrorKey::FareBatchEmpty));
    }
    if req.fares.len() > MAX_BULK_FARES {
        return Err(bad(u.lang, ErrorKey::FareBatchTooLarge));
    }

    let mut validated: Vec<(String, String, i64)> = Vec::with_capacity(req.fares.len());
    for item in &req.fares {
        let a = normalize_area_name(&item.area_a);
        let b = normalize_area_name(&item.area_b);
        if !valid_area_name(&a) || !valid_area_name(&b) {
            return Err(bad(u.lang, ErrorKey::AreaNameRequired));
        }
        if item.amount_cents < 0 || item.amount_cents > 50_000 {
            return Err(bad(u.lang, ErrorKey::AmountRangeInvalid));
        }
        validated.push((a, b, item.amount_cents));
    }

    let mut tx = st.db.begin().await?;
    let existing_areas: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM areas ORDER BY active DESC, id ASC")
            .fetch_all(&mut *tx)
            .await?;
    let mut area_map: HashMap<String, i64> = HashMap::with_capacity(existing_areas.len());
    for (id, name) in existing_areas {
        area_map.entry(area_lookup_key(&name)).or_insert(id);
    }

    let mut seen_missing: HashSet<String> = HashSet::new();
    let mut missing_areas: Vec<(String, String)> = Vec::new();
    for (a, b, _) in &validated {
        for name in [a, b] {
            let key = area_lookup_key(name);
            if !area_map.contains_key(&key) && seen_missing.insert(key.clone()) {
                missing_areas.push((key, name.clone()));
            }
        }
    }

    let mut created_areas: Vec<(i64, String)> = Vec::new();
    if !missing_areas.is_empty() {
        if !req.create_missing_areas {
            return Err(bad(u.lang, ErrorKey::FareUnknownArea));
        }
        require(&u, Page::AdminAreas, Act::Create)?;
        for (key, name) in missing_areas {
            let row: Result<(i64,), sqlx::Error> =
                sqlx::query_as("INSERT INTO areas (name, active) VALUES ($1, 1) RETURNING id")
                    .bind(&name)
                    .fetch_one(&mut *tx)
                    .await;
            let (id,) = match row {
                Ok(r) => r,
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                    return Err(bad(u.lang, ErrorKey::AreaExists));
                }
                Err(e) => return Err(e.into()),
            };
            area_map.insert(key, id);
            created_areas.push((id, name));
        }
    }

    let mut deduped: BTreeMap<(i64, i64), i64> = BTreeMap::new();
    for (a, b, cents) in &validated {
        let id_a = area_map[&area_lookup_key(a)];
        let id_b = area_map[&area_lookup_key(b)];
        let pair = (id_a.min(id_b), id_a.max(id_b));
        if let Some(&prev) = deduped.get(&pair) {
            if prev != *cents {
                return Err(bad(u.lang, ErrorKey::FareDuplicateConflict));
            }
        } else {
            deduped.insert(pair, *cents);
        }
    }

    let existing_fares: Vec<(i64, i64, i64)> =
        sqlx::query_as("SELECT area_a_id, area_b_id, amount_cents FROM area_fares")
            .fetch_all(&mut *tx)
            .await?;
    let existing_map: HashMap<(i64, i64), i64> = existing_fares
        .into_iter()
        .map(|(a, b, c)| ((a, b), c))
        .collect();

    let ts = now();
    let mut created: usize = 0;
    let mut updated: usize = 0;
    let mut unchanged: usize = 0;
    let mut area_a_ids: Vec<i64> = Vec::with_capacity(deduped.len());
    let mut area_b_ids: Vec<i64> = Vec::with_capacity(deduped.len());
    let mut amounts: Vec<i64> = Vec::with_capacity(deduped.len());
    for (&(lo, hi), &cents) in &deduped {
        match existing_map.get(&(lo, hi)) {
            Some(&prev) if prev == cents => {
                unchanged += 1;
            }
            Some(_) => {
                updated += 1;
                area_a_ids.push(lo);
                area_b_ids.push(hi);
                amounts.push(cents);
            }
            None => {
                created += 1;
                area_a_ids.push(lo);
                area_b_ids.push(hi);
                amounts.push(cents);
            }
        }
    }
    if !area_a_ids.is_empty() {
        sqlx::query(
            "INSERT INTO area_fares (area_a_id, area_b_id, amount_cents, updated_at)
             SELECT a, b, c, $4
             FROM UNNEST($1::BIGINT[], $2::BIGINT[], $3::BIGINT[]) AS t(a, b, c)
             ON CONFLICT (area_a_id, area_b_id) DO UPDATE
             SET amount_cents = EXCLUDED.amount_cents, updated_at = EXCLUDED.updated_at",
        )
        .bind(&area_a_ids)
        .bind(&area_b_ids)
        .bind(&amounts)
        .bind(&ts)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let mut created_names: Vec<String> = Vec::with_capacity(created_areas.len());
    for (id, name) in created_areas {
        audit(&st.db, Some(u.id), "area.create", "area", Some(id), &name).await;
        created_names.push(name);
    }
    let total = deduped.len();
    audit(
        &st.db,
        Some(u.id),
        "settings.fare_bulk",
        "area_fares",
        None,
        format!("{total} routes ({created} added, {updated} updated, {unchanged} unchanged)"),
    )
    .await;

    Ok(Json(json!({
        "created": created,
        "updated": updated,
        "unchanged": unchanged,
        "total": total,
        "areas_created": created_names,
    })))
}

/// Export the active area fare matrix as UTF-8 BOM CSV (upper triangle + diagonal).
pub(super) async fn export_fares_csv(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Response> {
    require(&u, Page::AdminFares, Act::View)?;
    let areas: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM areas WHERE active = 1 ORDER BY LOWER(name), name")
            .fetch_all(&st.db)
            .await?;
    let fare_rows: Vec<(i64, i64, i64)> =
        sqlx::query_as("SELECT area_a_id, area_b_id, amount_cents FROM area_fares")
            .fetch_all(&st.db)
            .await?;
    let fare_map: HashMap<(i64, i64), i64> =
        fare_rows.into_iter().map(|(a, b, c)| ((a, b), c)).collect();

    let mut out = String::from("\u{FEFF}");
    out.push_str(&escape_csv_cell("From \\ To"));
    for (_, name) in &areas {
        out.push(',');
        out.push_str(&escape_csv_cell(name));
    }
    out.push_str("\r\n");

    for (i, (id_a, name_a)) in areas.iter().enumerate() {
        out.push_str(&escape_csv_cell(name_a));
        for (j, (id_b, _)) in areas.iter().enumerate() {
            out.push(',');
            if j >= i {
                let pair = ((*id_a).min(*id_b), (*id_a).max(*id_b));
                if let Some(&cents) = fare_map.get(&pair) {
                    out.push_str(&format!("{}.{:02}", cents / 100, cents % 100));
                }
            }
        }
        out.push_str("\r\n");
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"grace-ride-area-fares.csv\"",
        )
        .body(Body::from(out))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}
