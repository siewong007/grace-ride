//! Durable Postgres notification delivery outbox (`notification_deliveries`) with bounded retries.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{
    i18n::Lang,
    util::{in_minutes, is_dev_mode, now, t},
    web_push::{PushMessage, PushOutcome, VapidSigner},
};

use super::meta::{absolute_url, is_in_quiet_hours};
use super::ssrf::{build_pinned_client_with_url, sign_webhook_headers};

const RETRY_BACKOFF_SECS: &[i64] = &[30, 120, 600, 1800, 7200];
const MAX_ATTEMPTS: i64 = 5;

#[derive(Clone, Debug)]
pub(super) struct EnqueueItem {
    pub(super) notification_id: Option<i64>,
    pub(super) user_id: i64,
    pub(super) title: String,
    pub(super) body: String,
    pub(super) link: String,
    pub(super) category: String,
    pub(super) priority: String,
    pub(super) tag: String,
}

#[derive(Clone, Debug)]
struct UserAlertPref {
    name: String,
    phone: String,
    lang: Lang,
    telegram_chat_id: String,
    whatsapp_opt_in: bool,
    push_opt_in: bool,
    muted_categories: HashSet<String>,
    quiet_hours_start: String,
    quiet_hours_end: String,
    hide_lock_details: bool,
    telegram_blocked: bool,
}

#[derive(Clone, Default)]
struct AlertSecrets {
    telegram_bot_token: String,
    alert_webhook_url: String,
    alert_webhook_secret: String,
    mobile_push_webhook_url: String,
    vapid_signer: Option<Arc<VapidSigner>>,
}

#[derive(sqlx::FromRow)]
struct ClaimedDelivery {
    id: i64,
    user_id: i64,
    channel: String,
    target_id: Option<i64>,
    target_ref: String,
    title: String,
    body: String,
    link: String,
    category: String,
    priority: String,
    tag: String,
    badge_count: Option<i64>,
    attempts: i64,
}

fn should_deliver_external(pref: &UserAlertPref, category: &str, priority: &str) -> bool {
    if priority == "urgent" {
        return true;
    }
    if pref.muted_categories.contains(category) {
        return false;
    }
    let now_hhmm = crate::util::local_now().format("%H:%M").to_string();
    !is_in_quiet_hours(&pref.quiet_hours_start, &pref.quiet_hours_end, &now_hhmm)
}

fn redact_if_needed(lang: Lang, hide_lock: bool, title: &str, body: &str) -> (String, String) {
    if !hide_lock {
        return (title.to_string(), body.to_string());
    }
    let t_str = t(lang, "恩典同行通知", "Grace Ride Alert").to_string();
    let b_str = t(
        lang,
        "您有一条新通知，请打开应用查看详情。",
        "You have a new notification. Open the app to view details.",
    )
    .to_string();
    (t_str, b_str)
}

async fn load_alert_secrets(db: &PgPool) -> AlertSecrets {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets
         WHERE name IN (
             'telegram_bot_token', 'alert_webhook_url', 'alert_webhook_secret',
             'mobile_push_webhook_url', 'vapid_public_key', 'vapid_private_key'
         )",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut s = AlertSecrets::default();
    let mut vapid_pub = String::new();
    let mut vapid_priv = String::new();
    for (k, v) in rows {
        let plain =
            crate::crypto::decrypt_secret(&format!("app_secret:{k}"), &v).unwrap_or_default();
        match k.as_str() {
            "telegram_bot_token" => s.telegram_bot_token = plain.trim().to_string(),
            "alert_webhook_url" => s.alert_webhook_url = plain.trim().to_string(),
            "alert_webhook_secret" => s.alert_webhook_secret = plain.trim().to_string(),
            "mobile_push_webhook_url" => s.mobile_push_webhook_url = plain.trim().to_string(),
            "vapid_public_key" => vapid_pub = plain,
            "vapid_private_key" => vapid_priv = plain,
            _ => {}
        }
    }
    s.vapid_signer = crate::web_push::cached_vapid_signer(&vapid_pub, &vapid_priv);
    s
}

pub(super) async fn enqueue_external_deliveries(db: &PgPool, items: Vec<EnqueueItem>) {
    if items.is_empty() {
        return;
    }

    let uids: Vec<i64> = items
        .iter()
        .map(|i| i.user_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let unread_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, COUNT(*) FROM notifications
         WHERE user_id = ANY($1) AND read_at IS NULL
         GROUP BY user_id",
    )
    .bind(&uids)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let unread_map: HashMap<i64, i64> = unread_rows.into_iter().collect();

    for uid in &uids {
        if items
            .iter()
            .any(|i| i.user_id == *uid && i.notification_id.is_some())
        {
            crate::stream::publish(*uid, "notify");
            crate::stream::publish_unread(*uid, unread_map.get(uid).copied().unwrap_or(0));
        }
    }

    let secrets = load_alert_secrets(db).await;
    let dev_mode = is_dev_mode();
    let webhook_ok = !secrets.alert_webhook_url.is_empty()
        && crate::util::validate_webhook_target(&secrets.alert_webhook_url, dev_mode).await;
    if !secrets.alert_webhook_url.is_empty() && !webhook_ok {
        tracing::warn!("blocked alert webhook URL failing SSRF policy");
    }
    let mobile_hook_ok = !secrets.mobile_push_webhook_url.is_empty()
        && crate::util::validate_webhook_target(&secrets.mobile_push_webhook_url, dev_mode).await;

    if secrets.telegram_bot_token.is_empty()
        && !webhook_ok
        && !mobile_hook_ok
        && secrets.vapid_signer.is_none()
    {
        return;
    }

    #[allow(clippy::type_complexity)]
    let pref_rows: Vec<(
        i64,
        String,
        String,
        String,
        String,
        i64,
        i64,
        String,
        String,
        String,
        i64,
        i64,
    )> = sqlx::query_as(
        "SELECT u.id, u.name, u.phone, u.lang, COALESCE(u.telegram_chat_id, ''),
                u.whatsapp_opt_in, u.push_opt_in,
                COALESCE(p.muted_categories, '[]'),
                COALESCE(p.quiet_hours_start, ''),
                COALESCE(p.quiet_hours_end, ''),
                COALESCE(p.hide_lock_details, 0),
                COALESCE(p.telegram_blocked, 0)
         FROM users u
         LEFT JOIN user_notification_prefs p ON p.user_id = u.id
         WHERE u.id = ANY($1) AND u.status = 'active'",
    )
    .bind(&uids)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let pref_map: HashMap<i64, UserAlertPref> = pref_rows
        .into_iter()
        .map(
            |(id, name, phone, lang_s, tg, wa, push, muted_json, q_start, q_end, hide, tg_blk)| {
                let muted: HashSet<String> = serde_json::from_str::<Vec<String>>(&muted_json)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                (
                    id,
                    UserAlertPref {
                        name,
                        phone,
                        lang: Lang::from_code(&lang_s),
                        telegram_chat_id: tg,
                        whatsapp_opt_in: wa == 1,
                        push_opt_in: push == 1,
                        muted_categories: muted,
                        quiet_hours_start: q_start,
                        quiet_hours_end: q_end,
                        hide_lock_details: hide == 1,
                        telegram_blocked: tg_blk == 1,
                    },
                )
            },
        )
        .collect();

    let mut web_push_map: HashMap<i64, Vec<(i64, String)>> = HashMap::new();
    if secrets.vapid_signer.is_some() {
        let push_ids: Vec<i64> = pref_map
            .iter()
            .filter_map(|(uid, p)| p.push_opt_in.then_some(*uid))
            .collect();
        if !push_ids.is_empty() {
            let subs: Vec<(i64, i64, String)> = sqlx::query_as(
                "SELECT id, user_id, endpoint FROM push_subscriptions WHERE user_id = ANY($1)",
            )
            .bind(&push_ids)
            .fetch_all(db)
            .await
            .unwrap_or_default();
            for (sub_id, uid, endpoint) in subs {
                web_push_map
                    .entry(uid)
                    .or_default()
                    .push((sub_id, endpoint));
            }
        }
    }

    let mut mobile_push_map: HashMap<i64, Vec<(i64, String)>> = HashMap::new();
    if mobile_hook_ok {
        let push_ids: Vec<i64> = pref_map
            .iter()
            .filter_map(|(uid, p)| p.push_opt_in.then_some(*uid))
            .collect();
        if !push_ids.is_empty() {
            let toks: Vec<(i64, i64, String)> = sqlx::query_as(
                "SELECT id, user_id, token FROM mobile_push_tokens WHERE user_id = ANY($1)",
            )
            .bind(&push_ids)
            .fetch_all(db)
            .await
            .unwrap_or_default();
            for (tok_id, uid, token) in toks {
                mobile_push_map
                    .entry(uid)
                    .or_default()
                    .push((tok_id, token));
            }
        }
    }

    let mut o_notif_id: Vec<Option<i64>> = Vec::new();
    let mut o_uid: Vec<i64> = Vec::new();
    let mut o_channel: Vec<String> = Vec::new();
    let mut o_target_id: Vec<Option<i64>> = Vec::new();
    let mut o_target_ref: Vec<String> = Vec::new();
    let mut o_title: Vec<String> = Vec::new();
    let mut o_body: Vec<String> = Vec::new();
    let mut o_link: Vec<String> = Vec::new();
    let mut o_cat: Vec<String> = Vec::new();
    let mut o_prio: Vec<String> = Vec::new();
    let mut o_tag: Vec<String> = Vec::new();
    let mut o_badge: Vec<Option<i64>> = Vec::new();
    let mut o_ts: Vec<String> = Vec::new();
    let ts_now = now();

    let mut push_row = |item: &EnqueueItem,
                        channel: &str,
                        target_id: Option<i64>,
                        target_ref: String,
                        title: String,
                        body: String,
                        badge: Option<i64>| {
        o_notif_id.push(item.notification_id);
        o_uid.push(item.user_id);
        o_channel.push(channel.to_string());
        o_target_id.push(target_id);
        o_target_ref.push(target_ref);
        o_title.push(title);
        o_body.push(body);
        o_link.push(item.link.clone());
        o_cat.push(item.category.clone());
        o_prio.push(item.priority.clone());
        o_tag.push(item.tag.clone());
        o_badge.push(badge);
        o_ts.push(ts_now.clone());
    };

    for item in &items {
        let Some(pref) = pref_map.get(&item.user_id) else {
            continue;
        };
        if !should_deliver_external(pref, &item.category, &item.priority) {
            continue;
        }
        let badge = unread_map.get(&item.user_id).copied();
        let (ext_title, ext_body) =
            redact_if_needed(pref.lang, pref.hide_lock_details, &item.title, &item.body);

        if !secrets.telegram_bot_token.is_empty()
            && !pref.telegram_blocked
            && !pref.telegram_chat_id.trim().is_empty()
        {
            push_row(
                item,
                "telegram",
                None,
                pref.telegram_chat_id.trim().to_string(),
                ext_title.clone(),
                ext_body.clone(),
                badge,
            );
        }

        if webhook_ok && pref.whatsapp_opt_in {
            let meta_ref = serde_json::json!({
                "name": pref.name,
                "phone": pref.phone,
            })
            .to_string();
            push_row(
                item,
                "whatsapp_webhook",
                None,
                meta_ref,
                ext_title.clone(),
                ext_body.clone(),
                badge,
            );
        }

        if pref.push_opt_in && secrets.vapid_signer.is_some() {
            if let Some(subs) = web_push_map.get(&item.user_id) {
                for (sub_id, endpoint) in subs {
                    push_row(
                        item,
                        "web_push",
                        Some(*sub_id),
                        endpoint.clone(),
                        ext_title.clone(),
                        ext_body.clone(),
                        badge,
                    );
                }
            }
        }

        if pref.push_opt_in && mobile_hook_ok {
            if let Some(toks) = mobile_push_map.get(&item.user_id) {
                for (tok_id, token) in toks {
                    push_row(
                        item,
                        "mobile_push",
                        Some(*tok_id),
                        token.clone(),
                        ext_title.clone(),
                        ext_body.clone(),
                        badge,
                    );
                }
            }
        }
    }

    if o_uid.is_empty() {
        return;
    }

    let inserted: sqlx::Result<Vec<i64>> = sqlx::query_scalar(
        "INSERT INTO notification_deliveries
            (notification_id, user_id, channel, target_id, target_ref,
             title, body, link, category, priority, tag, badge_count,
             status, attempts, next_attempt_at, created_at)
         SELECT nid, uid, ch, tid, tref, ttl, bdy, lnk, cat, prio, tg, bdg, 'pending', 0, ts, ts
         FROM UNNEST(
             $1::BIGINT[], $2::BIGINT[], $3::TEXT[], $4::BIGINT[], $5::TEXT[],
             $6::TEXT[], $7::TEXT[], $8::TEXT[], $9::TEXT[], $10::TEXT[], $11::TEXT[], $12::BIGINT[], $13::TEXT[]
         ) AS t(nid, uid, ch, tid, tref, ttl, bdy, lnk, cat, prio, tg, bdg, ts)
         RETURNING id",
    )
    .bind(&o_notif_id)
    .bind(&o_uid)
    .bind(&o_channel)
    .bind(&o_target_id)
    .bind(&o_target_ref)
    .bind(&o_title)
    .bind(&o_body)
    .bind(&o_link)
    .bind(&o_cat)
    .bind(&o_prio)
    .bind(&o_tag)
    .bind(&o_badge)
    .bind(&o_ts)
    .fetch_all(db)
    .await;

    match inserted {
        Ok(ids) if !ids.is_empty() => {
            let db_clone = db.clone();
            tokio::spawn(async move {
                if let Err(e) = process_delivery_ids(&db_clone, &ids).await {
                    tracing::warn!("immediate notification delivery pass failed: {e:?}");
                }
            });
        }
        Ok(_) => {}
        Err(e) => tracing::error!("enqueue notification_deliveries failed: {e}"),
    }
}

pub async fn process_due_deliveries(db: &PgPool, limit: i64) -> anyhow::Result<usize> {
    let lease_until = in_minutes(5);
    let claimed: Vec<ClaimedDelivery> = sqlx::query_as(
        "WITH due AS (
             SELECT id FROM notification_deliveries
             WHERE status IN ('pending', 'retry') AND next_attempt_at <= $1
             ORDER BY next_attempt_at ASC, id ASC
             LIMIT $2
             FOR UPDATE SKIP LOCKED
         )
         UPDATE notification_deliveries d
         SET next_attempt_at = $3
         FROM due
         WHERE d.id = due.id
         RETURNING d.id, d.user_id, d.channel, d.target_id, d.target_ref,
                   d.title, d.body, d.link, d.category, d.priority, d.tag,
                   d.badge_count, d.attempts",
    )
    .bind(now())
    .bind(limit.clamp(1, 200))
    .bind(&lease_until)
    .fetch_all(db)
    .await?;

    let count = claimed.len();
    if count > 0 {
        dispatch_claimed_rows(db, claimed).await;
    }
    Ok(count)
}

pub async fn process_delivery_ids(db: &PgPool, ids: &[i64]) -> anyhow::Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    let ts_now = now();
    let lease_until = in_minutes(5);
    let claimed: Vec<ClaimedDelivery> = sqlx::query_as(
        "WITH target AS (
             SELECT id FROM notification_deliveries
             WHERE id = ANY($1) AND status IN ('pending', 'retry') AND next_attempt_at <= $3
             FOR UPDATE SKIP LOCKED
         )
         UPDATE notification_deliveries d
         SET next_attempt_at = $2
         FROM target
         WHERE d.id = target.id
         RETURNING d.id, d.user_id, d.channel, d.target_id, d.target_ref,
                   d.title, d.body, d.link, d.category, d.priority, d.tag,
                   d.badge_count, d.attempts",
    )
    .bind(ids)
    .bind(&lease_until)
    .bind(&ts_now)
    .fetch_all(db)
    .await?;

    let count = claimed.len();
    if count > 0 {
        dispatch_claimed_rows(db, claimed).await;
    }
    Ok(count)
}

async fn dispatch_claimed_rows(db: &PgPool, rows: Vec<ClaimedDelivery>) {
    let secrets = Arc::new(load_alert_secrets(db).await);
    let dev_mode = is_dev_mode();

    let web_sub_ids: Vec<i64> = rows
        .iter()
        .filter(|r| r.channel == "web_push")
        .filter_map(|r| r.target_id)
        .collect();
    let web_keys: HashMap<i64, (String, String, String)> = if web_sub_ids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as::<_, (i64, String, String, String)>(
            "SELECT id, endpoint, p256dh, auth FROM push_subscriptions WHERE id = ANY($1)",
        )
        .bind(&web_sub_ids)
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(id, ep, p256, auth)| (id, (ep, p256, auth)))
        .collect()
    };
    let web_keys = Arc::new(web_keys);

    let mob_ids: Vec<i64> = rows
        .iter()
        .filter(|r| r.channel == "mobile_push")
        .filter_map(|r| r.target_id)
        .collect();
    let mob_tokens: HashMap<i64, (String, String, String)> = if mob_ids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as::<_, (i64, String, String, String)>(
            "SELECT id, platform, provider, token FROM mobile_push_tokens WHERE id = ANY($1)",
        )
        .bind(&mob_ids)
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(id, plat, prov, tok)| (id, (plat, prov, tok)))
        .collect()
    };
    let mob_tokens = Arc::new(mob_tokens);

    let uids: Vec<i64> = rows
        .iter()
        .map(|r| r.user_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let user_langs: HashMap<i64, Lang> = if uids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as::<_, (i64, String)>("SELECT id, lang FROM users WHERE id = ANY($1)")
            .bind(&uids)
            .fetch_all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(id, l)| (id, Lang::from_code(&l)))
            .collect()
    };
    let user_langs = Arc::new(user_langs);

    let pinned_webhook = if !secrets.alert_webhook_url.is_empty() {
        build_pinned_client_with_url(&secrets.alert_webhook_url, dev_mode).await
    } else {
        None
    };
    let pinned_mobile = if !secrets.mobile_push_webhook_url.is_empty() {
        build_pinned_client_with_url(&secrets.mobile_push_webhook_url, dev_mode).await
    } else {
        None
    };

    let client = crate::util::alert_client().clone();
    let subject = crate::util::vapid_subject();
    let sem = Arc::new(tokio::sync::Semaphore::new(8));
    let mut join_set = tokio::task::JoinSet::new();

    for row in rows {
        let db = db.clone();
        let secrets = secrets.clone();
        let web_keys = web_keys.clone();
        let mob_tokens = mob_tokens.clone();
        let user_langs = user_langs.clone();
        let client = client.clone();
        let pinned_webhook = pinned_webhook.clone();
        let pinned_mobile = pinned_mobile.clone();
        let subject = subject.clone();
        let sem = sem.clone();

        join_set.spawn(async move {
            let Ok(_permit) = sem.acquire_owned().await else {
                return;
            };
            deliver_one(
                &db,
                &row,
                &secrets,
                &web_keys,
                &mob_tokens,
                &user_langs,
                &client,
                pinned_webhook.as_ref(),
                pinned_mobile.as_ref(),
                &subject,
                dev_mode,
            )
            .await;
        });
    }
    while join_set.join_next().await.is_some() {}
}

fn next_backoff_ts(attempts_done: i64, retry_after_secs: Option<u64>) -> String {
    let idx = (attempts_done.saturating_sub(1) as usize).min(RETRY_BACKOFF_SECS.len() - 1);
    let secs = retry_after_secs
        .map(|s| s as i64)
        .unwrap_or(RETRY_BACKOFF_SECS[idx]);
    crate::util::ts(crate::util::local_now() + chrono::Duration::seconds(secs))
}

#[allow(clippy::too_many_arguments)]
async fn deliver_one(
    db: &PgPool,
    row: &ClaimedDelivery,
    secrets: &AlertSecrets,
    web_keys: &HashMap<i64, (String, String, String)>,
    mob_tokens: &HashMap<i64, (String, String, String)>,
    user_langs: &HashMap<i64, Lang>,
    client: &reqwest::Client,
    pinned_webhook: Option<&(String, reqwest::Client)>,
    pinned_mobile: Option<&(String, reqwest::Client)>,
    subject: &str,
    dev_mode: bool,
) {
    let next_attempt = row.attempts + 1;
    match row.channel.as_str() {
        "web_push" => {
            let Some(signer) = secrets.vapid_signer.as_ref() else {
                mark_delivery_terminal(db, row.id, next_attempt, "failed", "missing VAPID key")
                    .await;
                return;
            };
            let Some(sub_id) = row.target_id else {
                mark_delivery_terminal(db, row.id, next_attempt, "gone", "missing sub_id").await;
                return;
            };
            let Some((endpoint, p256dh, auth)) = web_keys.get(&sub_id) else {
                mark_delivery_terminal(db, row.id, next_attempt, "gone", "subscription deleted")
                    .await;
                return;
            };
            if !crate::util::validate_push_endpoint(endpoint, dev_mode).await {
                mark_delivery_terminal(db, row.id, next_attempt, "failed", "endpoint blocked")
                    .await;
                return;
            }
            let Some((pinned_ep_url, pinned_ep_client)) =
                build_pinned_client_with_url(endpoint, dev_mode).await
            else {
                mark_delivery_terminal(db, row.id, next_attempt, "failed", "endpoint blocked")
                    .await;
                return;
            };
            let lang = user_langs.get(&row.user_id).copied().unwrap_or(Lang::Zh);
            let urgency = match row.priority.as_str() {
                "urgent" => "high",
                "low" => "low",
                _ => "normal",
            };
            let ttl_secs = if row.priority == "urgent" {
                1800
            } else if row.category == "ride" || row.category == "chat" {
                7200
            } else {
                86400
            };
            let actions_zh = [("ready", "已就绪"), ("delayed_5m", "稍等5分钟")];
            let actions_en = [("ready", "Ready"), ("delayed_5m", "+5 min")];
            let actions = if row.link.contains("action=readiness") {
                Some(if lang == Lang::En {
                    &actions_en[..]
                } else {
                    &actions_zh[..]
                })
            } else {
                None
            };
            let outcome = crate::web_push::send_push_message_rich(
                &pinned_ep_client,
                signer,
                subject,
                &pinned_ep_url,
                p256dh,
                auth,
                &PushMessage {
                    title: &row.title,
                    body: &row.body,
                    url_path: &row.link,
                    badge_count: row.badge_count,
                    tag: (!row.tag.is_empty()).then_some(row.tag.as_str()),
                    category: &row.category,
                    priority: &row.priority,
                    urgency,
                    ttl_secs,
                    actions,
                },
            )
            .await;
            match outcome {
                PushOutcome::Delivered => {
                    let ts_now = now();
                    let _ = sqlx::query(
                        "UPDATE push_subscriptions SET last_delivered_at=$1 WHERE id=$2",
                    )
                    .bind(&ts_now)
                    .bind(sub_id)
                    .execute(db)
                    .await;
                    mark_delivery_terminal(db, row.id, next_attempt, "delivered", "").await;
                }
                PushOutcome::Gone => {
                    let _ = sqlx::query("DELETE FROM push_subscriptions WHERE id=$1")
                        .bind(sub_id)
                        .execute(db)
                        .await;
                    mark_delivery_terminal(db, row.id, next_attempt, "gone", "HTTP 404/410").await;
                }
                PushOutcome::RateLimited(ra) => {
                    schedule_retry(db, row.id, next_attempt, "HTTP 429", ra).await;
                }
                PushOutcome::Retryable(err) => {
                    schedule_retry(db, row.id, next_attempt, &err, None).await;
                }
                PushOutcome::Skipped => {
                    mark_delivery_terminal(db, row.id, next_attempt, "failed", "payload skipped")
                        .await;
                }
                PushOutcome::Failed(err) => {
                    mark_delivery_terminal(db, row.id, next_attempt, "failed", &err).await;
                }
            }
        }
        "telegram" => {
            if secrets.telegram_bot_token.is_empty() || row.target_ref.trim().is_empty() {
                mark_delivery_terminal(db, row.id, next_attempt, "failed", "telegram unconfigured")
                    .await;
                return;
            }
            let abs = absolute_url(&row.link);
            let text = if abs.is_empty() {
                format!("{}\n{}", row.title, row.body)
            } else {
                format!("{}\n{}\n{}", row.title, row.body, abs)
            };
            let url = format!(
                "https://api.telegram.org/bot{}/sendMessage",
                secrets.telegram_bot_token
            );
            match client
                .post(&url)
                .json(&json!({
                    "chat_id": row.target_ref.trim(),
                    "text": text,
                }))
                .send()
                .await
            {
                Ok(res) => {
                    let status = res.status().as_u16();
                    if (200..=299).contains(&status) {
                        let _ = sqlx::query(
                            "UPDATE user_notification_prefs SET telegram_blocked=0, updated_at=$2 WHERE user_id=$1 AND telegram_blocked=1",
                        )
                        .bind(row.user_id)
                        .bind(now())
                        .execute(db)
                        .await;
                        mark_delivery_terminal(db, row.id, next_attempt, "delivered", "").await;
                    } else if status == 403 || status == 400 {
                        let body_bytes = crate::util::read_response_bytes_bounded(res, 64 * 1024)
                            .await
                            .unwrap_or_default();
                        let body_txt = String::from_utf8_lossy(&body_bytes);
                        let blocked = status == 403
                            || body_txt.contains("chat not found")
                            || body_txt.contains("blocked");
                        if blocked {
                            let _ = sqlx::query(
                                "INSERT INTO user_notification_prefs (user_id, telegram_blocked, updated_at)
                                 VALUES ($1, 1, $2)
                                 ON CONFLICT (user_id) DO UPDATE SET telegram_blocked=1, updated_at=EXCLUDED.updated_at",
                            )
                            .bind(row.user_id)
                            .bind(now())
                            .execute(db)
                            .await;
                            mark_delivery_terminal(
                                db,
                                row.id,
                                next_attempt,
                                "gone",
                                &format!(
                                    "HTTP {status}: {}",
                                    body_txt.chars().take(160).collect::<String>()
                                ),
                            )
                            .await;
                        } else {
                            mark_delivery_terminal(
                                db,
                                row.id,
                                next_attempt,
                                "failed",
                                &format!(
                                    "HTTP {status}: {}",
                                    body_txt.chars().take(160).collect::<String>()
                                ),
                            )
                            .await;
                        }
                    } else if status == 429 {
                        let body_bytes = crate::util::read_response_bytes_bounded(res, 64 * 1024)
                            .await
                            .unwrap_or_default();
                        let body_json: Value =
                            serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);
                        let ra = body_json["parameters"]["retry_after"].as_u64();
                        schedule_retry(db, row.id, next_attempt, "telegram 429", ra).await;
                    } else if status >= 500 {
                        schedule_retry(db, row.id, next_attempt, &format!("HTTP {status}"), None)
                            .await;
                    } else {
                        mark_delivery_terminal(
                            db,
                            row.id,
                            next_attempt,
                            "failed",
                            &format!("HTTP {status}"),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    let err = e.without_url().to_string();
                    tracing::warn!(delivery_id = row.id, error = %err, "telegram delivery error");
                    schedule_retry(db, row.id, next_attempt, &err, None).await;
                }
            }
        }
        "whatsapp_webhook" => {
            let Some((target_url, pinned)) = pinned_webhook else {
                mark_delivery_terminal(db, row.id, next_attempt, "failed", "webhook blocked").await;
                return;
            };
            let meta_val: Value = serde_json::from_str(&row.target_ref).unwrap_or(Value::Null);
            let name = meta_val["name"].as_str().unwrap_or("");
            let phone = meta_val["phone"].as_str().unwrap_or("");
            let abs = absolute_url(&row.link);
            let payload_bytes = serde_json::to_vec(&json!({
                "delivery_id": row.id,
                "channel": "whatsapp",
                "user_id": row.user_id,
                "name": name,
                "phone": phone,
                "title": row.title,
                "body": row.body,
                "link": abs,
                "link_path": row.link,
                "category": row.category,
                "priority": row.priority,
            }))
            .unwrap_or_default();
            let (ts, sig) = sign_webhook_headers(&secrets.alert_webhook_secret, &payload_bytes);
            let mut req = pinned
                .post(target_url)
                .header("Content-Type", "application/json")
                .header("X-GraceRide-Timestamp", ts)
                .header("X-GraceRide-Delivery-Id", row.id.to_string());
            if let Some(s) = sig {
                req = req.header("X-GraceRide-Signature", s);
            }
            match req.body(payload_bytes).send().await {
                Ok(res) => {
                    let status = res.status().as_u16();
                    if (200..=299).contains(&status) {
                        mark_delivery_terminal(db, row.id, next_attempt, "delivered", "").await;
                    } else if status == 429 || status >= 500 || status == 408 {
                        schedule_retry(db, row.id, next_attempt, &format!("HTTP {status}"), None)
                            .await;
                    } else {
                        mark_delivery_terminal(
                            db,
                            row.id,
                            next_attempt,
                            "failed",
                            &format!("HTTP {status}"),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    let err = e.without_url().to_string();
                    tracing::warn!(delivery_id = row.id, error = %err, "webhook delivery error");
                    schedule_retry(db, row.id, next_attempt, &err, None).await;
                }
            }
        }
        "mobile_push" => {
            let Some((target_url, pinned)) = pinned_mobile else {
                mark_delivery_terminal(
                    db,
                    row.id,
                    next_attempt,
                    "failed",
                    "mobile push webhook unconfigured",
                )
                .await;
                return;
            };
            let Some(tok_id) = row.target_id else {
                mark_delivery_terminal(db, row.id, next_attempt, "gone", "missing token id").await;
                return;
            };
            let Some((platform, provider, token)) = mob_tokens.get(&tok_id) else {
                mark_delivery_terminal(db, row.id, next_attempt, "gone", "token removed").await;
                return;
            };
            let payload_bytes = serde_json::to_vec(&json!({
                "delivery_id": row.id,
                "channel": "mobile_push",
                "user_id": row.user_id,
                "platform": platform,
                "provider": provider,
                "token": token,
                "title": row.title,
                "body": row.body,
                "url": row.link,
                "category": row.category,
                "priority": row.priority,
                "badge_count": row.badge_count,
                "tag": row.tag,
            }))
            .unwrap_or_default();
            let (ts, sig) = sign_webhook_headers(&secrets.alert_webhook_secret, &payload_bytes);
            let mut req = pinned
                .post(target_url)
                .header("Content-Type", "application/json")
                .header("X-GraceRide-Timestamp", ts)
                .header("X-GraceRide-Delivery-Id", row.id.to_string());
            if let Some(s) = sig {
                req = req.header("X-GraceRide-Signature", s);
            }
            match req.body(payload_bytes).send().await {
                Ok(res) => {
                    let status = res.status().as_u16();
                    if (200..=299).contains(&status) {
                        let _ = sqlx::query(
                            "UPDATE mobile_push_tokens SET last_delivered_at=$1 WHERE id=$2",
                        )
                        .bind(now())
                        .bind(tok_id)
                        .execute(db)
                        .await;
                        mark_delivery_terminal(db, row.id, next_attempt, "delivered", "").await;
                    } else if status == 404 || status == 410 {
                        let _ = sqlx::query("DELETE FROM mobile_push_tokens WHERE id=$1")
                            .bind(tok_id)
                            .execute(db)
                            .await;
                        mark_delivery_terminal(db, row.id, next_attempt, "gone", "HTTP 404/410")
                            .await;
                    } else if status == 429 || status >= 500 || status == 408 {
                        schedule_retry(db, row.id, next_attempt, &format!("HTTP {status}"), None)
                            .await;
                    } else {
                        mark_delivery_terminal(
                            db,
                            row.id,
                            next_attempt,
                            "failed",
                            &format!("HTTP {status}"),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    let err = e.without_url().to_string();
                    schedule_retry(db, row.id, next_attempt, &err, None).await;
                }
            }
        }
        other => {
            mark_delivery_terminal(
                db,
                row.id,
                next_attempt,
                "failed",
                &format!("unknown channel {other}"),
            )
            .await;
        }
    }
}

async fn schedule_retry(
    db: &PgPool,
    id: i64,
    attempts: i64,
    err: &str,
    retry_after_secs: Option<u64>,
) {
    let err_short: String = err.chars().take(240).collect();
    if attempts >= MAX_ATTEMPTS {
        mark_delivery_terminal(db, id, attempts, "failed", &err_short).await;
        return;
    }
    let next_at = next_backoff_ts(attempts, retry_after_secs);
    let expected_attempts = attempts.saturating_sub(1);
    let _ = sqlx::query(
        "UPDATE notification_deliveries
         SET status='retry', attempts=$2, next_attempt_at=$3, last_error=$4
         WHERE id=$1 AND status IN ('pending', 'retry') AND attempts=$5",
    )
    .bind(id)
    .bind(attempts)
    .bind(next_at)
    .bind(err_short)
    .bind(expected_attempts)
    .execute(db)
    .await;
}

async fn mark_delivery_terminal(db: &PgPool, id: i64, attempts: i64, status: &str, err: &str) {
    let delivered_at = (status == "delivered").then(now);
    let err_short: String = err.chars().take(240).collect();
    let expected_attempts = attempts.saturating_sub(1);
    let _ = sqlx::query(
        "UPDATE notification_deliveries
         SET status=$2, attempts=$3, last_error=$4, delivered_at=$5
         WHERE id=$1 AND status IN ('pending', 'retry') AND attempts=$6",
    )
    .bind(id)
    .bind(status)
    .bind(attempts)
    .bind(err_short)
    .bind(delivered_at)
    .bind(expected_attempts)
    .execute(db)
    .await;
}
