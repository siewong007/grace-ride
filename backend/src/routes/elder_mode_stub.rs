
use axum::{extract::{State}, Json};
use serde_json::json;
use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;

pub async fn toggle_elder_mode(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    sqlx::query("UPDATE users SET elder_mode = NOT elder_mode WHERE id=$1")
        .bind(u.id)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({"ok": true})))
}
