
use axum::{extract::{State}, Json};
use serde::{Deserialize};
use serde_json::json;
use crate::auth::CurrentUser;
use crate::error::{bad, AppResult};
use crate::state::AppState;
use crate::util::{now, require, ErrorKey};
use crate::perms::{Act, Page};

#[derive(Deserialize)]
pub struct BroadcastIn {
    title: String,
    body: String,
}

pub async fn list_broadcasts(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<serde_json::Value>> {
    let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, title, body, created_at FROM broadcasts ORDER BY id DESC LIMIT 50"
    )
    .fetch_all(&st.db)
    .await?;
    
    Ok(Json(json!(rows.into_iter().map(|(id, t, b, c)| json!({
        "id": id, 
        "title": t, 
        "body": b,
        "created_at": c
    })).collect::<Vec<_>>())))
}

pub async fn create_broadcast(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<BroadcastIn>,
) -> AppResult<Json<serde_json::Value>> {
    require(&u, Page::AdminAnnouncements, Act::Create)?;
    
    sqlx::query("INSERT INTO broadcasts (title, body, created_at) VALUES ($1,$2,$3)")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(now())
        .execute(&st.db)
        .await?;
        
    Ok(Json(json!({"ok": true})))
}
