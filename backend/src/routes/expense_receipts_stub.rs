
use axum::{extract::{State, Path}, Json};
use serde::{Deserialize};
use serde_json::json;
use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;
use crate::util::{now};

#[derive(Deserialize)]
pub struct ReceiptIn {
    ride_id: i64,
    amount: f64,
    image_path: String,
}

pub async fn upload_receipt(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReceiptIn>,
) -> AppResult<Json<serde_json::Value>> {
    sqlx::query("INSERT INTO receipts (ride_id, amount, image_path, created_at) VALUES ($1,$2,$3,$4)")
        .bind(req.ride_id)
        .bind(req.amount)
        .bind(req.image_path)
        .bind(now())
        .execute(&st.db)
        .await?;
        
    Ok(Json(json!({"ok": true})))
}
