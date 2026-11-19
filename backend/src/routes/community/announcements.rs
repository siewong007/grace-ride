//! Church announcements listing and admin management endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audience::{announcement_users_sql, visible_announcements_sql};
use crate::auth::CurrentUser;
use crate::error::{bad, not_found, AppResult};
use crate::i18n::{ErrorKey, Lang};
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, notify_many, now, require};

// ---------------- Announcements ----------------

#[derive(Deserialize)]
pub(super) struct AnnQuery {
    /// The admin page lists every announcement, not only the ones for the admin's groups.
    manage: Option<bool>,
}

type AnnRow = (i64, String, String, i64, String, String, i64);

pub(super) async fn list_announcements(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AnnQuery>,
) -> AppResult<Json<Value>> {
    let rows: Vec<AnnRow> = if q.manage.unwrap_or(false)
        && u.can(Page::AdminAnnouncements, Act::View)
    {
        sqlx::query_as(
                "SELECT a.id, a.title, a.body, a.pinned, u.name, a.created_at, a.targeted FROM announcements a JOIN users u ON u.id=a.created_by ORDER BY a.pinned DESC, a.id DESC LIMIT 30",
            )
            .fetch_all(&st.db)
            .await?
    } else {
        sqlx::query_as(crate::dyn_sql!(
                "SELECT a.id, a.title, a.body, a.pinned, u.name, a.created_at, a.targeted FROM announcements a JOIN users u ON u.id=a.created_by WHERE {} ORDER BY a.pinned DESC, a.id DESC LIMIT 30",
                visible_announcements_sql("$1", "$2")
            ))
            .bind(u.id)
            .bind(&u.group_scope)
            .fetch_all(&st.db)
            .await?
    };
    let ids: Vec<i64> = rows.iter().map(|r| r.0).collect();
    let groups: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT ag.announcement_id, g.id, g.name FROM announcement_groups ag JOIN groups g ON g.id = ag.group_id WHERE ag.announcement_id = ANY($1) ORDER BY g.is_all DESC, g.name",
    )
    .bind(&ids)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, t, b, p, by, c, targeted)| {
            let gs: Vec<Value> = groups
                .iter()
                .filter(|(a, _, _)| *a == id)
                .map(|(_, gid, name)| json!({"id": gid, "name": name}))
                .collect();
            json!({"id": id, "title": t, "body": b, "pinned": p == 1, "by": by, "created_at": c, "targeted": targeted == 1, "groups": gs})
        })
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub(super) struct AnnIn {
    title: String,
    body: String,
    pinned: Option<bool>,
    /// Notify the members the announcement is for (everyone when it has no groups).
    notify_all: Option<bool>,
    /// The groups it is for. Empty or missing: every member.
    group_ids: Option<Vec<i64>>,
}

#[derive(Deserialize)]
pub(super) struct AnnEdit {
    title: String,
    body: String,
    pinned: bool,
    /// Missing: keep the audience as it is (pinning sends only title, body and pinned).
    group_ids: Option<Vec<i64>>,
}

/// Set the groups an announcement is for, and return their names for the audit log.
/// Every group must still exist and not be the visitor group.
async fn set_groups(
    tx: &mut sqlx::PgConnection,
    id: i64,
    group_ids: &[i64],
    lang: Lang,
) -> AppResult<Vec<String>> {
    let mut wanted = group_ids.to_vec();
    wanted.sort_unstable();
    wanted.dedup();
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM groups WHERE id = ANY($1) AND is_visitor = 0 ORDER BY name FOR SHARE",
    )
    .bind(&wanted)
    .fetch_all(&mut *tx)
    .await?;
    if names.len() != wanted.len() {
        return Err(bad(lang, ErrorKey::SelectVisibleGroup));
    }
    sqlx::query("DELETE FROM announcement_groups WHERE announcement_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO announcement_groups (announcement_id, group_id) SELECT $1, unnest($2::BIGINT[])")
        .bind(id)
        .bind(&wanted)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE announcements SET targeted=$1 WHERE id=$2")
        .bind(!wanted.is_empty() as i64)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    Ok(names)
}

/// "Title → Group A, Group B" for the audit log, or just the title when it is for everyone.
fn audit_detail(title: &str, groups: &[String]) -> String {
    if groups.is_empty() {
        title.to_string()
    } else {
        format!("{title} → {}", groups.join(", "))
    }
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
    let mut tx = st.db.begin().await?;
    let (id,): (i64,) = sqlx::query_as("INSERT INTO announcements (title, body, pinned, created_by, created_at) VALUES ($1,$2,$3,$4,$5) RETURNING id")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(req.pinned.unwrap_or(false) as i64)
        .bind(u.id)
        .bind(now())
        .fetch_one(&mut *tx)
        .await?;
    let groups = set_groups(&mut tx, id, req.group_ids.as_deref().unwrap_or(&[]), u.lang).await?;
    tx.commit().await?;
    if req.notify_all.unwrap_or(false) {
        let ids: Vec<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(announcement_users_sql("$1")))
            .bind(id)
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
        audit_detail(req.title.trim(), &groups),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

/// Fix a typo, pin and unpin, or change who it is for, without publishing again: nobody is
/// notified a second time.
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
    let mut tx = st.db.begin().await?;
    let changed = sqlx::query("UPDATE announcements SET title=$1, body=$2, pinned=$3 WHERE id=$4")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(req.pinned as i64)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(not_found(u.lang));
    }
    let detail = match &req.group_ids {
        Some(g) => audit_detail(req.title.trim(), &set_groups(&mut tx, id, g, u.lang).await?),
        None => req.title.trim().to_string(),
    };
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "announcement.update",
        "announcement",
        Some(id),
        detail,
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
