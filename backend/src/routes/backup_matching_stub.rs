
use axum::{extract::{State, Path}, Json};
use serde_json::json;
use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;

pub async fn find_backup_driver(
    State(st): State<AppState>,
    Path(ride_id): Path<i64>,
    _u: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    // Logic: find drivers in area not currently assigned to ride_id
    Ok(Json(json!({"drivers": []})))
}
