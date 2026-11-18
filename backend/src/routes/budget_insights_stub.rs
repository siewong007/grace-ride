
use axum::{extract::{State}, Json};
use serde_json::json;
use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;

pub async fn get_budget_summary(
    State(st): State<AppState>,
    _u: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let rows: Vec<(String, f64)> = sqlx::query_as("SELECT category, SUM(amount) FROM budget_records GROUP BY category")
        .fetch_all(&st.db)
        .await?;
        
    Ok(Json(json!({
        "summary": rows.into_iter().map(|(cat, amt)| json!({"category": cat, "total": amt})).collect::<Vec<_>>()
    })))
}
