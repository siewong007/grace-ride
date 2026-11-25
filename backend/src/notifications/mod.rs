//! Centralized notification delivery engine:
//! - Structured notification metadata (category, event_type, priority, entity)
//! - Per-user preferences (muted categories, quiet hours, lock-screen privacy)
//! - Durable Postgres outbox (`notification_deliveries`) with bounded retries & backoff
//! - DNS-pinned SSRF-safe HTTP dispatch + HMAC-SHA256 signed webhooks
//! - Throttled offline alerts for trip group chat

mod meta;
mod outbox;
mod ssrf;

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::{
    i18n::Lang,
    perms::{Act, Page, Perm},
    util::{minutes_ago, now, t},
};

pub use meta::{
    absolute_url, infer_meta, is_in_quiet_hours, is_valid_hhmm, validate_telegram_chat_id,
    BatchNotifItem, NotifMeta, VALID_CATEGORIES,
};
pub use outbox::{process_delivery_ids, process_due_deliveries};
pub use ssrf::{
    build_pinned_client, build_pinned_client_with_url, resolve_and_validate_http_target,
    sign_webhook_headers,
};

use outbox::{enqueue_external_deliveries, EnqueueItem};

pub async fn notify_with(
    db: &PgPool,
    user_id: i64,
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
    meta: NotifMeta,
) {
    let lang = user_lang(db, user_id).await;
    let title_s = t(lang, title.0, title.1).to_string();
    let body_s = t(lang, body.0, body.1).to_string();
    let row: sqlx::Result<(i64,)> = sqlx::query_as(
        "INSERT INTO notifications
            (user_id, title, body, link, category, event_type, priority, entity_type, entity_id, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
         RETURNING id",
    )
    .bind(user_id)
    .bind(&title_s)
    .bind(&body_s)
    .bind(link)
    .bind(meta.category)
    .bind(&meta.event_type)
    .bind(meta.priority)
    .bind(meta.entity_type)
    .bind(meta.entity_id)
    .bind(now())
    .fetch_one(db)
    .await;

    match row {
        Ok((notif_id,)) => {
            enqueue_external_deliveries(
                db,
                vec![EnqueueItem {
                    notification_id: Some(notif_id),
                    user_id,
                    title: title_s,
                    body: body_s,
                    link: link.to_string(),
                    category: meta.category.to_string(),
                    priority: meta.priority.to_string(),
                    tag: meta.tag,
                }],
            )
            .await;
        }
        Err(e) => tracing::error!("notify failed: {e}"),
    }
}

pub async fn notify_many_with(
    db: &PgPool,
    user_ids: &[i64],
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
    meta: NotifMeta,
) {
    if user_ids.is_empty() {
        return;
    }
    let rows: sqlx::Result<Vec<(i64, i64, String, String)>> = sqlx::query_as(
        "INSERT INTO notifications
            (user_id, title, body, link, category, event_type, priority, entity_type, entity_id, created_at)
         SELECT u.id,
                CASE WHEN u.lang = 'en' THEN $3 ELSE $2 END,
                CASE WHEN u.lang = 'en' THEN $5 ELSE $4 END,
                $6, $7, $8, $9, $10, $11, $12
         FROM users u WHERE u.id = ANY($1) AND u.status = 'active'
         RETURNING id, user_id, title, body",
    )
    .bind(user_ids)
    .bind(title.0)
    .bind(title.1)
    .bind(body.0)
    .bind(body.1)
    .bind(link)
    .bind(meta.category)
    .bind(&meta.event_type)
    .bind(meta.priority)
    .bind(meta.entity_type)
    .bind(meta.entity_id)
    .bind(now())
    .fetch_all(db)
    .await;

    match rows {
        Ok(inserted) => {
            let items = inserted
                .into_iter()
                .map(|(nid, uid, title_s, body_s)| EnqueueItem {
                    notification_id: Some(nid),
                    user_id: uid,
                    title: title_s,
                    body: body_s,
                    link: link.to_string(),
                    category: meta.category.to_string(),
                    priority: meta.priority.to_string(),
                    tag: meta.tag.clone(),
                })
                .collect();
            enqueue_external_deliveries(db, items).await;
        }
        Err(e) => tracing::error!("notify_many failed: {e}"),
    }
}

pub async fn notify_batch(db: &PgPool, items: Vec<BatchNotifItem>) {
    if items.is_empty() {
        return;
    }
    let uids: Vec<i64> = items.iter().map(|i| i.user_id).collect();
    let lang_rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, lang FROM users WHERE id = ANY($1)")
            .bind(&uids)
            .fetch_all(db)
            .await
            .unwrap_or_default();
    let lang_map: HashMap<i64, Lang> = lang_rows
        .into_iter()
        .map(|(id, l)| (id, Lang::from_code(&l)))
        .collect();

    let ts_now = now();
    let mut b_uid = Vec::with_capacity(items.len());
    let mut b_title = Vec::with_capacity(items.len());
    let mut b_body = Vec::with_capacity(items.len());
    let mut b_link = Vec::with_capacity(items.len());
    let mut b_cat = Vec::with_capacity(items.len());
    let mut b_ev = Vec::with_capacity(items.len());
    let mut b_prio = Vec::with_capacity(items.len());
    let mut b_ent_type = Vec::with_capacity(items.len());
    let mut b_ent_id: Vec<Option<i64>> = Vec::with_capacity(items.len());
    let mut b_ts = Vec::with_capacity(items.len());
    let mut tags = Vec::with_capacity(items.len());

    for item in items {
        let lang = lang_map.get(&item.user_id).copied().unwrap_or(Lang::Zh);
        let title_s = t(lang, &item.title_zh, &item.title_en).to_string();
        let body_s = t(lang, &item.body_zh, &item.body_en).to_string();
        b_uid.push(item.user_id);
        b_title.push(title_s);
        b_body.push(body_s);
        b_link.push(item.link);
        b_cat.push(item.meta.category.to_string());
        b_ev.push(item.meta.event_type);
        b_prio.push(item.meta.priority.to_string());
        b_ent_type.push(item.meta.entity_type.to_string());
        b_ent_id.push(item.meta.entity_id);
        b_ts.push(ts_now.clone());
        tags.push(item.meta.tag);
    }

    let inserted: sqlx::Result<Vec<(i64, i64, String, String, String, String, String)>> =
        sqlx::query_as(
            "INSERT INTO notifications
                (user_id, title, body, link, category, event_type, priority, entity_type, entity_id, created_at)
             SELECT * FROM UNNEST(
                $1::BIGINT[], $2::TEXT[], $3::TEXT[], $4::TEXT[], $5::TEXT[],
                $6::TEXT[], $7::TEXT[], $8::TEXT[], $9::BIGINT[], $10::TEXT[]
             )
             RETURNING id, user_id, title, body, link, category, priority",
        )
        .bind(&b_uid)
        .bind(&b_title)
        .bind(&b_body)
        .bind(&b_link)
        .bind(&b_cat)
        .bind(&b_ev)
        .bind(&b_prio)
        .bind(&b_ent_type)
        .bind(&b_ent_id)
        .bind(&b_ts)
        .fetch_all(db)
        .await;

    match inserted {
        Ok(rows) => {
            let mut enq = Vec::with_capacity(rows.len());
            for ((nid, uid, title, body, link, category, priority), tag) in
                rows.into_iter().zip(tags)
            {
                enq.push(EnqueueItem {
                    notification_id: Some(nid),
                    user_id: uid,
                    title,
                    body,
                    link,
                    category,
                    priority,
                    tag,
                });
            }
            enqueue_external_deliveries(db, enq).await;
        }
        Err(e) => tracing::error!("notify_batch failed: {e}"),
    }
}

pub async fn notify_perm_except(
    db: &PgPool,
    page: Page,
    act: Act,
    exclude_user_id: Option<i64>,
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    let users: Vec<(i64,)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT u.id FROM users u WHERE {}",
        crate::perms::sql_holds("u", Perm(page, act))
    ))
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let ids: Vec<i64> = users
        .into_iter()
        .map(|(id,)| id)
        .filter(|id| exclude_user_id != Some(*id))
        .collect();
    let meta = infer_meta(link, title.1);
    notify_many_with(db, &ids, title, body, link, meta).await;
}

/// Send live SSE `"chat"` ticks to every participant on the trip and queue throttled
/// external push/Telegram/webhook alerts for participants who have no open SSE stream.
pub async fn notify_trip_chat(
    db: &PgPool,
    trip_id: i64,
    ride_id: i64,
    sender_id: i64,
    sender_name: &str,
    message_body: &str,
) {
    let ids: Vec<(i64,)> = sqlx::query_as(
        "SELECT passenger_id FROM rides WHERE trip_id=$1 AND status <> 'cancelled'
         UNION SELECT driver_id FROM rides WHERE trip_id=$1 AND driver_id IS NOT NULL",
    )
    .bind(trip_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut offline_ids = Vec::new();
    for (uid,) in ids {
        crate::stream::publish(uid, "chat");
        if uid != sender_id && !crate::stream::has_active_stream(uid) {
            offline_ids.push(uid);
        }
    }
    if offline_ids.is_empty() {
        return;
    }

    let tag = format!("trip-chat-{trip_id}");
    let recently_sent: Vec<(i64,)> = sqlx::query_as(
        "SELECT DISTINCT user_id FROM notification_deliveries
         WHERE user_id = ANY($1) AND tag = $2 AND created_at >= $3",
    )
    .bind(&offline_ids)
    .bind(&tag)
    .bind(minutes_ago(1))
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let recent_set: HashSet<i64> = recently_sent.into_iter().map(|(u,)| u).collect();
    let target_ids: Vec<i64> = offline_ids
        .into_iter()
        .filter(|uid| !recent_set.contains(uid))
        .collect();
    if target_ids.is_empty() {
        return;
    }

    let lang_rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, lang FROM users WHERE id = ANY($1) AND status = 'active'")
            .bind(&target_ids)
            .fetch_all(db)
            .await
            .unwrap_or_default();
    let preview: String = message_body.chars().take(120).collect();
    let body_text = format!("{sender_name}: {preview}");
    let link = format!("/rides/{ride_id}");

    let mut enq = Vec::with_capacity(lang_rows.len());
    for (uid, lang_s) in lang_rows {
        let lang = Lang::from_code(&lang_s);
        enq.push(EnqueueItem {
            notification_id: None,
            user_id: uid,
            title: t(lang, "行程群聊新消息", "New trip chat message").to_string(),
            body: body_text.clone(),
            link: link.clone(),
            category: "chat".to_string(),
            priority: "normal".to_string(),
            tag: tag.clone(),
        });
    }
    enqueue_external_deliveries(db, enq).await;
}

async fn user_lang(db: &PgPool, user_id: i64) -> Lang {
    let row: Option<(String,)> = sqlx::query_as("SELECT lang FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    row.map(|(l,)| Lang::from_code(&l)).unwrap_or(Lang::Zh)
}
