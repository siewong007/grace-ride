//! Member in-app notifications inbox & preference endpoints.

use std::collections::HashSet;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::notifications::{is_valid_hhmm, validate_telegram_chat_id, VALID_CATEGORIES};
use crate::state::AppState;
use crate::util::now;

// ---------------- Notifications ----------------

#[derive(Deserialize)]
pub(super) struct NotifQuery {
    before: Option<i64>,
    limit: Option<i64>,
    unread_only: Option<bool>,
    category: Option<String>,
}

type NotifRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    Option<i64>,
);

fn notif_to_json(row: NotifRow) -> Value {
    let (
        id,
        title,
        body,
        link,
        read_at,
        created_at,
        category,
        event_type,
        priority,
        entity_type,
        entity_id,
    ) = row;
    json!({
        "id": id,
        "title": title,
        "body": body,
        "link": link,
        "read": read_at.is_some(),
        "created_at": created_at,
        "category": category,
        "event_type": event_type,
        "priority": priority,
        "entity_type": entity_type,
        "entity_id": entity_id,
    })
}

async fn count_unread(db: &sqlx::PgPool, user_id: i64) -> AppResult<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id=$1 AND read_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;
    Ok(n)
}

pub(super) async fn list_notifications(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<NotifQuery>,
) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    let unread_only = q.unread_only.unwrap_or(false);
    let category_filter = q
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all");
    if let Some(cat) = category_filter {
        if !VALID_CATEGORIES.contains(&cat) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }

    let paginated =
        q.before.is_some() || q.limit.is_some() || q.unread_only.is_some() || q.category.is_some();

    if paginated {
        let mut rows: Vec<NotifRow> = sqlx::query_as(
            "SELECT id, title, body, link, read_at, created_at,
                    category, event_type, priority, entity_type, entity_id
             FROM notifications
             WHERE user_id=$1
               AND ($2::BIGINT IS NULL OR id < $2)
               AND (NOT $3 OR read_at IS NULL)
               AND ($4::TEXT IS NULL OR category = $4)
             ORDER BY id DESC
             LIMIT $5",
        )
        .bind(u.id)
        .bind(q.before)
        .bind(unread_only)
        .bind(category_filter)
        .bind(limit + 1)
        .fetch_all(&st.db)
        .await?;

        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = if more { rows.last().map(|r| r.0) } else { None };
        let unread = count_unread(&st.db, u.id).await?;
        return Ok(Json(json!({
            "items": rows.into_iter().map(notif_to_json).collect::<Vec<_>>(),
            "next_before": next_before,
            "unread": unread,
        })));
    }

    let rows: Vec<NotifRow> = sqlx::query_as(
        "SELECT id, title, body, link, read_at, created_at,
                category, event_type, priority, entity_type, entity_id
         FROM notifications
         WHERE user_id=$1
         ORDER BY id DESC
         LIMIT 100",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(notif_to_json)
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct ReadAllIn {
    category: Option<String>,
}

pub(super) async fn read_all(
    State(st): State<AppState>,
    u: CurrentUser,
    body: Bytes,
) -> AppResult<Json<Value>> {
    let cat = if body.is_empty() {
        None
    } else {
        let parsed: ReadAllIn =
            serde_json::from_slice(&body).map_err(|_| bad(u.lang, ErrorKey::InvalidAction))?;
        parsed
            .category
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty() && c != "all")
    };
    if let Some(ref c) = cat {
        if !VALID_CATEGORIES.contains(&c.as_str()) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }

    sqlx::query(
        "UPDATE notifications
         SET read_at=$1
         WHERE user_id=$2 AND read_at IS NULL
           AND ($3::TEXT IS NULL OR category = $3)",
    )
    .bind(now())
    .bind(u.id)
    .bind(cat.as_deref())
    .execute(&st.db)
    .await?;

    let unread = count_unread(&st.db, u.id).await?;
    crate::stream::publish_unread(u.id, unread);
    Ok(Json(json!({ "ok": true, "unread": unread })))
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
    let unread = count_unread(&st.db, u.id).await?;
    crate::stream::publish_unread(u.id, unread);
    Ok(Json(json!({ "ok": true, "unread": unread })))
}

pub(super) async fn unread_one(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let res = sqlx::query("UPDATE notifications SET read_at=NULL WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    let unread = count_unread(&st.db, u.id).await?;
    crate::stream::publish_unread(u.id, unread);
    Ok(Json(json!({ "ok": true, "unread": unread })))
}

pub(super) async fn delete_one(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let res = sqlx::query("DELETE FROM notifications WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    let unread = count_unread(&st.db, u.id).await?;
    crate::stream::publish_unread(u.id, unread);
    Ok(Json(json!({ "ok": true, "unread": unread })))
}

#[derive(sqlx::FromRow)]
struct PrefsRow {
    push_opt_in: i64,
    whatsapp_opt_in: i64,
    telegram_chat_id: String,
    readiness_prompt_mins: Option<i64>,
    muted_categories: String,
    quiet_hours_start: String,
    quiet_hours_end: String,
    hide_lock_details: i64,
    driver_remind_mins: i64,
    telegram_blocked: i64,
}

pub(super) async fn get_preferences(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let row: PrefsRow = sqlx::query_as(
        "SELECT u.push_opt_in, u.whatsapp_opt_in, COALESCE(u.telegram_chat_id, '') AS telegram_chat_id,
                u.readiness_prompt_mins,
                COALESCE(p.muted_categories, '[]') AS muted_categories,
                COALESCE(p.quiet_hours_start, '') AS quiet_hours_start,
                COALESCE(p.quiet_hours_end, '') AS quiet_hours_end,
                COALESCE(p.hide_lock_details, 0) AS hide_lock_details,
                COALESCE(p.driver_remind_mins, 60) AS driver_remind_mins,
                COALESCE(p.telegram_blocked, 0) AS telegram_blocked
         FROM users u
         LEFT JOIN user_notification_prefs p ON p.user_id = u.id
         WHERE u.id = $1",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;

    let muted: Vec<String> = serde_json::from_str(&row.muted_categories).unwrap_or_default();
    Ok(Json(json!({
        "push_opt_in": row.push_opt_in == 1,
        "whatsapp_opt_in": row.whatsapp_opt_in == 1,
        "telegram_chat_id": row.telegram_chat_id,
        "telegram_blocked": row.telegram_blocked == 1,
        "readiness_prompt_mins": row.readiness_prompt_mins.unwrap_or(45),
        "driver_remind_mins": row.driver_remind_mins,
        "muted_categories": muted,
        "quiet_hours_start": row.quiet_hours_start,
        "quiet_hours_end": row.quiet_hours_end,
        "hide_lock_details": row.hide_lock_details == 1,
    })))
}

#[derive(Deserialize)]
pub(super) struct PrefsIn {
    muted_categories: Option<Vec<String>>,
    quiet_hours_start: Option<String>,
    quiet_hours_end: Option<String>,
    hide_lock_details: Option<bool>,
    driver_remind_mins: Option<i64>,
    push_opt_in: Option<bool>,
    whatsapp_opt_in: Option<bool>,
    readiness_prompt_mins: Option<i64>,
    telegram_chat_id: Option<String>,
}

pub(super) async fn update_preferences(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<PrefsIn>,
) -> AppResult<Json<Value>> {
    if let Some(m) = req.readiness_prompt_mins {
        if !(0..=180).contains(&m) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if let Some(m) = req.driver_remind_mins {
        if !(15..=180).contains(&m) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if let Some(ref tg) = req.telegram_chat_id {
        if !validate_telegram_chat_id(tg) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }

    let muted_json = if let Some(ref cats) = req.muted_categories {
        let mut seen = HashSet::new();
        let mut cleaned = Vec::new();
        for c in cats {
            let trimmed = c.trim();
            if !VALID_CATEGORIES.contains(&trimmed) {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
            if seen.insert(trimmed.to_string()) {
                cleaned.push(trimmed.to_string());
            }
        }
        Some(serde_json::to_string(&cleaned).unwrap_or_else(|_| "[]".into()))
    } else {
        None
    };

    let (q_start, q_end) = match (
        req.quiet_hours_start.as_deref(),
        req.quiet_hours_end.as_deref(),
    ) {
        (Some(s), Some(e)) => {
            let st_trim = s.trim();
            let en_trim = e.trim();
            if st_trim.is_empty() && en_trim.is_empty() {
                (Some(String::new()), Some(String::new()))
            } else if is_valid_hhmm(st_trim) && is_valid_hhmm(en_trim) {
                (Some(st_trim.to_string()), Some(en_trim.to_string()))
            } else {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        (None, None) => (None, None),
    };

    let mut tx = st.db.begin().await?;
    let tg_cleaned = req.telegram_chat_id.as_deref().map(str::trim);
    if req.push_opt_in.is_some()
        || req.whatsapp_opt_in.is_some()
        || req.readiness_prompt_mins.is_some()
        || tg_cleaned.is_some()
    {
        sqlx::query(
            "UPDATE users SET
                push_opt_in=COALESCE($1, push_opt_in),
                whatsapp_opt_in=COALESCE($2, whatsapp_opt_in),
                readiness_prompt_mins=COALESCE($3, readiness_prompt_mins),
                telegram_chat_id=COALESCE($4, telegram_chat_id)
             WHERE id=$5",
        )
        .bind(req.push_opt_in.map(|b| b as i64))
        .bind(req.whatsapp_opt_in.map(|b| b as i64))
        .bind(req.readiness_prompt_mins)
        .bind(tg_cleaned)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    }

    let ts_now = now();
    let clear_tg_blocked = tg_cleaned.is_some();
    sqlx::query(
        "INSERT INTO user_notification_prefs
            (user_id, muted_categories, quiet_hours_start, quiet_hours_end,
             hide_lock_details, driver_remind_mins, telegram_blocked, updated_at)
         VALUES ($1, COALESCE($2, '[]'), COALESCE($3, ''), COALESCE($4, ''),
                 COALESCE($5, 0), COALESCE($6, 60), 0, $7)
         ON CONFLICT (user_id) DO UPDATE SET
            muted_categories = COALESCE($2, user_notification_prefs.muted_categories),
            quiet_hours_start = COALESCE($3, user_notification_prefs.quiet_hours_start),
            quiet_hours_end = COALESCE($4, user_notification_prefs.quiet_hours_end),
            hide_lock_details = COALESCE($5, user_notification_prefs.hide_lock_details),
            driver_remind_mins = COALESCE($6, user_notification_prefs.driver_remind_mins),
            telegram_blocked = CASE WHEN $8 THEN 0 ELSE user_notification_prefs.telegram_blocked END,
            updated_at = $7",
    )
    .bind(u.id)
    .bind(muted_json.as_deref())
    .bind(q_start.as_deref())
    .bind(q_end.as_deref())
    .bind(req.hide_lock_details.map(|b| b as i64))
    .bind(req.driver_remind_mins)
    .bind(&ts_now)
    .bind(clear_tg_blocked)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(json!({ "ok": true })))
}
