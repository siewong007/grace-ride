//! Member in-app notifications inbox endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;
use crate::util::now;

// ---------------- Notifications ----------------

#[derive(Deserialize)]
pub(super) struct NotifQuery {
    before: Option<i64>,
    limit: Option<i64>,
}

pub(super) async fn list_notifications(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<NotifQuery>,
) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    if q.before.is_some() || q.limit.is_some() {
        let sql = if q.before.is_some() {
            "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 AND id < $2 ORDER BY id DESC LIMIT $3"
        } else {
            "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 ORDER BY id DESC LIMIT $2"
        };
        let mut rows: Vec<(i64, String, String, String, Option<String>, String)> =
            if let Some(b) = q.before {
                sqlx::query_as(sql)
                    .bind(u.id)
                    .bind(b)
                    .bind(limit + 1)
                    .fetch_all(&st.db)
                    .await?
            } else {
                sqlx::query_as(sql)
                    .bind(u.id)
                    .bind(limit + 1)
                    .fetch_all(&st.db)
                    .await?
            };
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = if more { rows.last().map(|r| r.0) } else { None };
        return Ok(Json(json!({
            "items": rows.into_iter().map(|(id, t, b, l, r, c)| json!({"id": id, "title": t, "body": b, "link": l, "read": r.is_some(), "created_at": c})).collect::<Vec<_>>(),
            "next_before": next_before
        })));
    }

    let rows: Vec<(i64, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 ORDER BY id DESC LIMIT 100",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, b, l, r, c)| json!({"id": id, "title": t, "body": b, "link": l, "read": r.is_some(), "created_at": c})).collect::<Vec<_>>())))
}

pub(super) async fn read_all(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE notifications SET read_at=$1 WHERE user_id=$2 AND read_at IS NULL")
        .bind(now())
        .bind(u.id)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn read_one(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        "UPDATE notifications SET read_at=$1 WHERE id=$2 AND user_id=$3 AND read_at IS NULL",
    )
    .bind(now())
    .bind(id)
    .bind(u.id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true})))
}
