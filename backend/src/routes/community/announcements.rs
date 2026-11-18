//! Church announcements listing and admin management endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, notify_many, now, require};

// ---------------- Announcements ----------------

pub(super) async fn list_announcements(
    State(st): State<AppState>,
    _u: CurrentUser,
) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT a.id, a.title, a.body, a.pinned, u.name, a.created_at FROM announcements a JOIN users u ON u.id=a.created_by ORDER BY a.pinned DESC, a.id DESC LIMIT 30",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, b, p, by, c)| json!({"id": id, "title": t, "body": b, "pinned": p == 1, "by": by, "created_at": c})).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub(super) struct AnnIn {
    title: String,
    body: String,
    pinned: Option<bool>,
    notify_all: Option<bool>,
}

#[derive(Deserialize)]
pub(super) struct AnnEdit {
    title: String,
    body: String,
    pinned: bool,
}

pub(super) async fn create_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<AnnIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Create)?;
    if req.title.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::TitleAndContentRequired));
    }
    let (id,): (i64,) = sqlx::query_as("INSERT INTO announcements (title, body, pinned, created_by, created_at) VALUES ($1,$2,$3,$4,$5) RETURNING id")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(req.pinned.unwrap_or(false) as i64)
        .bind(u.id)
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    if req.notify_all.unwrap_or(false) {
        let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM users WHERE status='active'")
            .fetch_all(&st.db)
            .await?;
        let title = req.title.trim();
        notify_many(
            &st.db,
            &ids,
            ("新公告", "New announcement"),
            (title, title),
            "/",
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "announcement.create",
        "announcement",
        Some(id),
        req.title.trim(),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

/// Fix a typo, or pin and unpin, without publishing again: nobody is notified a second time.
/// Editing is part of publishing, so it needs the same permission as creating.
pub(super) async fn update_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<AnnEdit>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Create)?;
    if req.title.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::TitleAndContentRequired));
    }
    let changed = sqlx::query("UPDATE announcements SET title=$1, body=$2, pinned=$3 WHERE id=$4")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(req.pinned as i64)
        .bind(id)
        .execute(&st.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "announcement.update",
        "announcement",
        Some(id),
        req.title.trim(),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn delete_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Delete)?;
    let gone: Option<(String,)> =
        sqlx::query_as("DELETE FROM announcements WHERE id=$1 RETURNING title")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    if let Some((title,)) = gone {
        audit(
            &st.db,
            Some(u.id),
            "announcement.delete",
            "announcement",
            Some(id),
            title,
        )
        .await;
    }
    Ok(Json(json!({"ok": true})))
}
