
use axum::{extract::State, Json};
use serde_json::json;
use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;

pub async fn list_broadcasts(
    State(st): State<AppState>,
    _u: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as("SELECT id, title, body FROM broadcasts ORDER BY id DESC LIMIT 20")
        .fetch_all(&st.db)
        .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, b)| json!({"id": id, "title": t, "body": b})).collect::<Vec<_>>())))
}
