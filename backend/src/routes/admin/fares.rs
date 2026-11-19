//! Area-to-area mileage fares. One flat price per pair of areas, the same in both directions.

use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, now, require};

pub(super) async fn list_fares(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
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
    require(&u, Page::AdminSettings, Act::Update)?;
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
    require(&u, Page::AdminSettings, Act::Update)?;
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
