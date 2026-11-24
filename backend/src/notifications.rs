//! Centralized notification delivery engine:
//! - Structured notification metadata (category, event_type, priority, entity)
//! - Per-user preferences (muted categories, quiet hours, lock-screen privacy)
//! - Durable Postgres outbox (`notification_deliveries`) with bounded retries & backoff
//! - DNS-pinned SSRF-safe HTTP dispatch + HMAC-SHA256 signed webhooks
//! - Throttled offline alerts for trip group chat

use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::{Arc, OnceLock, RwLock},
    time::{Duration, Instant},
};

use ring::hmac;
use serde_json::{json, Value};
use sqlx::PgPool;
use url::Url;

use crate::{
    i18n::Lang,
    perms::{Act, Page, Perm},
    util::{in_minutes, is_dev_mode, minutes_ago, now, t},
    web_push::{PushMessage, PushOutcome, VapidSigner},
};

pub const VALID_CATEGORIES: &[&str] = &[
    "ride",
    "event",
    "cost",
    "help",
    "announcement",
    "family",
    "admin",
    "chat",
    "general",
];

const RETRY_BACKOFF_SECS: &[i64] = &[30, 120, 600, 1800, 7200];
const MAX_ATTEMPTS: i64 = 5;

#[derive(Clone, Debug, Default)]
pub struct NotifMeta {
    pub category: &'static str,
    pub event_type: String,
    pub priority: &'static str,
    pub entity_type: &'static str,
    pub entity_id: Option<i64>,
    pub tag: String,
}

#[derive(Clone, Debug)]
pub struct BatchNotifItem {
    pub user_id: i64,
    pub title_zh: String,
    pub title_en: String,
    pub body_zh: String,
    pub body_en: String,
    pub link: String,
    pub meta: NotifMeta,
}

#[derive(Clone, Debug)]
struct EnqueueItem {
    notification_id: Option<i64>,
    user_id: i64,
    title: String,
    body: String,
    link: String,
    category: String,
    priority: String,
    tag: String,
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

static PINNED_CLIENTS: OnceLock<RwLock<HashMap<(String, bool), (Instant, reqwest::Client)>>> =
    OnceLock::new();

pub fn validate_telegram_chat_id(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }
    let digits = trimmed.strip_prefix('-').unwrap_or(trimmed);
    (5..=20).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
}

pub fn absolute_url(link: &str) -> String {
    let trimmed = link.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return trimmed.to_string();
    }
    let base = crate::util::public_url();
    if base.is_empty() {
        return trimmed.to_string();
    }
    if trimmed.starts_with('/') {
        format!("{base}{trimmed}")
    } else {
        format!("{base}/{trimmed}")
    }
}

pub fn infer_meta(link: &str, title_en: &str) -> NotifMeta {
    let clean_path = link.split('?').next().unwrap_or(link).trim();
    let mut category = "general";
    let mut entity_type = "";
    let mut entity_id: Option<i64> = None;
    let mut tag = String::new();

    if let Some(id) = parse_trailing_id(clean_path, "/rides/") {
        category = "ride";
        entity_type = "ride";
        entity_id = Some(id);
        tag = format!("ride-{id}");
    } else if let Some(id) = parse_trailing_id(clean_path, "/admin/rides/") {
        category = "ride";
        entity_type = "ride";
        entity_id = Some(id);
        tag = format!("ride-{id}");
    } else if clean_path == "/drive"
        || clean_path == "/rides"
        || clean_path.starts_with("/admin/rides")
    {
        category = "ride";
    } else if let Some(id) = parse_trailing_id(clean_path, "/events/") {
        category = "event";
        entity_type = "event";
        entity_id = Some(id);
        tag = format!("event-{id}");
    } else if clean_path.starts_with("/events") {
        category = "event";
    } else if let Some(id) = parse_trailing_id(clean_path, "/reimbursements/") {
        category = "cost";
        entity_type = "reimbursement";
        entity_id = Some(id);
        tag = format!("reimb-{id}");
    } else if let Some(id) = parse_trailing_id(clean_path, "/settlements/") {
        category = "cost";
        entity_type = "settlement";
        entity_id = Some(id);
        tag = format!("settlement-{id}");
    } else if clean_path.starts_with("/costs")
        || clean_path.starts_with("/settlements")
        || clean_path.starts_with("/reimbursements")
        || clean_path.starts_with("/admin/reimbursements")
    {
        category = "cost";
    } else if let Some(id) = parse_trailing_id(clean_path, "/help/") {
        category = "help";
        entity_type = "help";
        entity_id = Some(id);
        tag = format!("help-{id}");
    } else if clean_path.starts_with("/help") {
        category = "help";
    } else if clean_path.starts_with("/me/family") {
        category = "family";
    } else if clean_path.starts_with("/admin/") {
        category = "admin";
    } else if title_en.to_ascii_lowercase().contains("announcement") {
        category = "announcement";
    }

    let lower = title_en.to_ascii_lowercase();
    let priority = if link.contains("action=readiness")
        || lower.contains("arrived")
        || lower.contains("delayed")
        || lower.contains("unable to attend")
        || lower.contains("running late")
        || lower.contains("departed")
        || lower.contains("no driver")
        || lower.contains("ride reminder")
        || lower.contains("reassignment")
        || lower.contains("cannot drive")
    {
        "urgent"
    } else if category == "announcement" || lower.contains("report") || lower.contains("mileage") {
        "low"
    } else {
        "normal"
    };

    NotifMeta {
        category,
        event_type: String::new(),
        priority,
        entity_type,
        entity_id,
        tag,
    }
}

fn parse_trailing_id(path: &str, prefix: &str) -> Option<i64> {
    let rest = path.strip_prefix(prefix)?;
    let num_part = rest.split('/').next()?;
    num_part.parse::<i64>().ok()
}

pub fn is_valid_hhmm(raw: &str) -> bool {
    let Some((h, m)) = raw.split_once(':') else {
        return false;
    };
    if h.len() != 2 || m.len() != 2 {
        return false;
    }
    matches!((h.parse::<u8>(), m.parse::<u8>()), (Ok(0..=23), Ok(0..=59)))
}

pub fn is_in_quiet_hours(start: &str, end: &str, now_hhmm: &str) -> bool {
    if start.is_empty() || end.is_empty() || start == end {
        return false;
    }
    if start < end {
        now_hhmm >= start && now_hhmm < end
    } else {
        // Overnight window (e.g. 22:00 to 07:00)
        now_hhmm >= start || now_hhmm < end
    }
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

pub fn sign_webhook_headers(secret: &str, body: &[u8]) -> (String, Option<String>) {
    let ts = chrono::Utc::now().timestamp().to_string();
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return (ts, None);
    }
    let key = hmac::Key::new(hmac::HMAC_SHA256, trimmed.as_bytes());
    let mut ctx = hmac::Context::with_key(&key);
    ctx.update(ts.as_bytes());
    ctx.update(b".");
    ctx.update(body);
    let sig = ctx.sign();
    (ts, Some(format!("sha256={}", hex::encode(sig.as_ref()))))
}

fn is_metadata_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4 == std::net::Ipv4Addr::new(169, 254, 169, 254),
        IpAddr::V6(v6) => v6.to_ipv4_mapped() == Some(std::net::Ipv4Addr::new(169, 254, 169, 254)),
    }
}

fn is_non_global_v4(v4: std::net::Ipv4Addr) -> bool {
    let o = v4.octets();
    let cgnat = o[0] == 100 && (o[1] & 0b1100_0000) == 64;
    let bench = o[0] == 198 && (o[1] == 18 || o[1] == 19);
    let this_net = o[0] == 0;
    this_net
        || v4.is_private()
        || v4.is_loopback()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_documentation()
        || v4.is_multicast()
        || v4.is_unspecified()
        || cgnat
        || bench
        || o[0] >= 240
}

fn is_non_global_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_non_global_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_non_global_v4(mapped);
            }
            let segs = v6.segments();
            let is_ipv4_compat =
                segs[0..6] == [0, 0, 0, 0, 0, 0] && !v6.is_loopback() && !v6.is_unspecified();
            if is_ipv4_compat {
                let mapped = std::net::Ipv4Addr::new(
                    (segs[6] >> 8) as u8,
                    segs[6] as u8,
                    (segs[7] >> 8) as u8,
                    segs[7] as u8,
                );
                return is_non_global_v4(mapped);
            }
            let unicast_link_local = (segs[0] & 0xffc0) == 0xfe80;
            let unique_local = (segs[0] & 0xfe00) == 0xfc00;
            let doc = segs[0] == 0x2001 && segs[1] == 0x0db8;
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || unicast_link_local
                || unique_local
                || doc
        }
    }
}

pub async fn resolve_and_validate_http_target(
    raw: &str,
    max_len: usize,
    dev_mode: bool,
) -> Option<(Url, Option<(String, Vec<SocketAddr>)>)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > max_len {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    let scheme = parsed.scheme();
    if dev_mode {
        if scheme != "https" && scheme != "http" {
            return None;
        }
    } else if scheme != "https" {
        return None;
    }
    let host = parsed.host()?;
    let host_lower = parsed
        .host_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host_lower.is_empty()
        || host_lower == "metadata.google.internal"
        || host_lower == "169.254.169.254"
    {
        return None;
    }
    match host {
        url::Host::Ipv4(v4) => {
            let ip = IpAddr::V4(v4);
            if is_metadata_ip(ip) || (!dev_mode && is_non_global_v4(v4)) {
                return None;
            }
            Some((parsed, None))
        }
        url::Host::Ipv6(v6) => {
            let ip = IpAddr::V6(v6);
            if is_metadata_ip(ip) || (!dev_mode && is_non_global_ip(ip)) {
                return None;
            }
            Some((parsed, None))
        }
        url::Host::Domain(domain) => {
            let d = domain.trim_end_matches('.').to_ascii_lowercase();
            if d == "metadata.google.internal" {
                return None;
            }
            if dev_mode {
                return Some((parsed, None));
            }
            if d == "localhost"
                || d.ends_with(".localhost")
                || d.ends_with(".local")
                || d.ends_with(".internal")
                || !d.contains('.')
            {
                return None;
            }
            let port = parsed.port_or_known_default().unwrap_or(443);
            let addrs = tokio::net::lookup_host((d.as_str(), port)).await.ok()?;
            let mut valid = Vec::new();
            for addr in addrs {
                if is_non_global_ip(addr.ip()) {
                    return None;
                }
                valid.push(addr);
            }
            if valid.is_empty() {
                return None;
            }
            Some((parsed, Some((d, valid))))
        }
    }
}

pub async fn build_pinned_client(raw_url: &str, dev_mode: bool) -> Option<reqwest::Client> {
    let key = (raw_url.trim().to_string(), dev_mode);
    let cache = PINNED_CLIENTS.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(guard) = cache.read() {
        if let Some((at, client)) = guard.get(&key) {
            if at.elapsed() < Duration::from_secs(60) {
                return Some(client.clone());
            }
        }
    }

    let (_, pin) = resolve_and_validate_http_target(raw_url, 1000, dev_mode).await?;
    let client = match pin {
        Some((domain, addrs)) => reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .resolve_to_addrs(&domain, &addrs)
            .build()
            .ok()?,
        None => crate::util::alert_client().clone(),
    };
    if let Ok(mut guard) = cache.write() {
        if guard.len() >= 32 {
            guard.clear();
        }
        guard.insert(key, (Instant::now(), client.clone()));
    }
    Some(client)
}

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
        match k.as_str() {
            "telegram_bot_token" => s.telegram_bot_token = v.trim().to_string(),
            "alert_webhook_url" => s.alert_webhook_url = v.trim().to_string(),
            "alert_webhook_secret" => s.alert_webhook_secret = v.trim().to_string(),
            "mobile_push_webhook_url" => s.mobile_push_webhook_url = v.trim().to_string(),
            "vapid_public_key" => vapid_pub = v,
            "vapid_private_key" => vapid_priv = v,
            _ => {}
        }
    }
    s.vapid_signer = crate::web_push::cached_vapid_signer(&vapid_pub, &vapid_priv);
    s
}

async fn enqueue_external_deliveries(db: &PgPool, items: Vec<EnqueueItem>) {
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
                item.title.clone(),
                item.body.clone(),
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
    let lease_until = in_minutes(5);
    let claimed: Vec<ClaimedDelivery> = sqlx::query_as(
        "WITH target AS (
             SELECT id FROM notification_deliveries
             WHERE id = ANY($1) AND status IN ('pending', 'retry')
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
        build_pinned_client(&secrets.alert_webhook_url, dev_mode).await
    } else {
        None
    };
    let pinned_mobile = if !secrets.mobile_push_webhook_url.is_empty() {
        build_pinned_client(&secrets.mobile_push_webhook_url, dev_mode).await
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
    pinned_webhook: Option<&reqwest::Client>,
    pinned_mobile: Option<&reqwest::Client>,
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
                client,
                signer,
                subject,
                endpoint,
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
                        let body_txt = res.text().await.unwrap_or_default();
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
                        let body_json: Value = res.json().await.unwrap_or(Value::Null);
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
                    tracing::warn!(delivery_id = row.id, error = %e, "telegram delivery error");
                    schedule_retry(db, row.id, next_attempt, &e.to_string(), None).await;
                }
            }
        }
        "whatsapp_webhook" => {
            let Some(pinned) = pinned_webhook else {
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
                .post(&secrets.alert_webhook_url)
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
                    tracing::warn!(delivery_id = row.id, error = %e, "webhook delivery error");
                    schedule_retry(db, row.id, next_attempt, &e.to_string(), None).await;
                }
            }
        }
        "mobile_push" => {
            let Some(pinned) = pinned_mobile else {
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
                .post(&secrets.mobile_push_webhook_url)
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
                    schedule_retry(db, row.id, next_attempt, &e.to_string(), None).await;
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
    let _ = sqlx::query(
        "UPDATE notification_deliveries
         SET status='retry', attempts=$2, next_attempt_at=$3, last_error=$4
         WHERE id=$1",
    )
    .bind(id)
    .bind(attempts)
    .bind(next_at)
    .bind(err_short)
    .execute(db)
    .await;
}

async fn mark_delivery_terminal(db: &PgPool, id: i64, attempts: i64, status: &str, err: &str) {
    let delivered_at = (status == "delivered").then(now);
    let err_short: String = err.chars().take(240).collect();
    let _ = sqlx::query(
        "UPDATE notification_deliveries
         SET status=$2, attempts=$3, last_error=$4, delivered_at=$5
         WHERE id=$1",
    )
    .bind(id)
    .bind(status)
    .bind(attempts)
    .bind(err_short)
    .bind(delivered_at)
    .execute(db)
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_handles_overnight_and_daytime_windows() {
        assert!(is_in_quiet_hours("22:00", "07:00", "23:30"));
        assert!(is_in_quiet_hours("22:00", "07:00", "06:59"));
        assert!(!is_in_quiet_hours("22:00", "07:00", "07:00"));
        assert!(!is_in_quiet_hours("22:00", "07:00", "14:00"));

        assert!(is_in_quiet_hours("13:00", "15:00", "13:45"));
        assert!(!is_in_quiet_hours("13:00", "15:00", "15:00"));
        assert!(!is_in_quiet_hours("", "", "23:00"));
    }

    #[test]
    fn telegram_chat_id_validation() {
        assert!(validate_telegram_chat_id(""));
        assert!(validate_telegram_chat_id("123456789"));
        assert!(validate_telegram_chat_id("-1001234567890"));
        assert!(!validate_telegram_chat_id("@username"));
        assert!(!validate_telegram_chat_id("123"));
        assert!(!validate_telegram_chat_id("abc12345"));
    }

    #[test]
    fn category_and_priority_inference() {
        let m1 = infer_meta("/rides/42?action=readiness", "Ride reminder");
        assert_eq!(m1.category, "ride");
        assert_eq!(m1.priority, "urgent");
        assert_eq!(m1.entity_id, Some(42));
        assert_eq!(m1.tag, "ride-42");

        let m2 = infer_meta("/", "New announcement");
        assert_eq!(m2.category, "announcement");
        assert_eq!(m2.priority, "low");

        let m3 = infer_meta("/help/15", "Help center has replied");
        assert_eq!(m3.category, "help");
        assert_eq!(m3.priority, "normal");
        assert_eq!(m3.entity_id, Some(15));
    }
}
