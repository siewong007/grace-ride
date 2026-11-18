//! Shared helpers: time, audit log, notifications, permission checks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use rand::{distributions::Alphanumeric, Rng};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::perms::{self, Act, Page, Perm};

/// The zone every stored time is written in when nothing else is configured.
pub const DEFAULT_TIMEZONE: Tz = chrono_tz::Asia::Kuala_Lumpur;

static ZONE: OnceLock<Tz> = OnceLock::new();
static DEV_MODE: AtomicBool = AtomicBool::new(false);
static PUBLIC_URL: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// Record whether the server runs in development mode so background helpers know whether
/// plaintext HTTP / loopback webhook URLs are allowed without threading `Config`.
pub fn set_dev_mode(dev: bool) {
    DEV_MODE.store(dev, Ordering::Relaxed);
}

pub fn is_dev_mode() -> bool {
    DEV_MODE.load(Ordering::Relaxed)
}

/// Record the configured `PUBLIC_URL` so background Web Push tasks can derive a valid VAPID `sub`.
pub fn set_public_url(url: &str) {
    if let Ok(mut guard) = PUBLIC_URL.write() {
        *guard = url.trim().trim_end_matches('/').to_string();
    }
}

pub fn vapid_subject() -> String {
    if let Ok(guard) = PUBLIC_URL.read() {
        if guard.starts_with("https://") {
            return guard.clone();
        }
    }
    "mailto:noreply@example.org".to_string()
}

/// Set the app's time zone (`APP_TIMEZONE`). Stored times are local text in this
/// zone, so it is fixed for the life of the process: a second call with a different
/// zone is ignored with a warning.
pub fn set_timezone(tz: Tz) {
    if ZONE.set(tz).is_err() && zone() != tz {
        tracing::warn!("time zone already set to {}; ignoring {tz}", zone());
    }
}

/// The app's time zone. Never the host's: `TZ` on the server does not matter.
pub fn zone() -> Tz {
    *ZONE.get().unwrap_or(&DEFAULT_TIMEZONE)
}

/// The current time in the app's zone.
pub fn local_now() -> DateTime<Tz> {
    Utc::now().with_timezone(&zone())
}

/// Today's date in the app's zone.
pub fn today_local() -> NaiveDate {
    local_now().date_naive()
}

/// The stored text form of a time: `YYYY-MM-DDTHH:MM:SS`, local to the app's zone.
pub fn ts(at: DateTime<Tz>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn now() -> String {
    ts(local_now())
}

pub fn in_days(days: i64) -> String {
    ts(local_now() + Duration::days(days))
}

pub fn in_hours(hours: i64) -> String {
    ts(local_now() + Duration::hours(hours))
}

/// Timestamp `minutes` from now (negative: ago), in the same text format as `now()`.
pub fn in_minutes(minutes: i64) -> String {
    ts(local_now() + Duration::minutes(minutes))
}

/// Timestamp `minutes` ago, in the same text format as `now()`, for comparisons.
pub fn minutes_ago(minutes: i64) -> String {
    in_minutes(-minutes)
}

/// Longest display name accepted. Names show on every list, chat and notification.
pub const MAX_NAME_CHARS: usize = 60;

pub fn check_name(lang: Lang, name: &str) -> AppResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(bad(lang, ErrorKey::NameRequired));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(bad(lang, ErrorKey::NameTooLong));
    }
    Ok(())
}

/// Phone is optional; when given it must fit the database rule (10–20 characters).
pub fn check_phone(lang: Lang, phone: &str) -> AppResult<()> {
    let n = phone.trim().chars().count();
    if n != 0 && !(10..=20).contains(&n) {
        return Err(bad(lang, ErrorKey::PhoneInvalid));
    }
    Ok(())
}

pub fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Parse "YYYY-MM-DDTHH:MM" or "YYYY-MM-DDTHH:MM:SS" (also accepts a space separator).
pub fn parse_dt(s: &str) -> Option<NaiveDateTime> {
    let s = s.replace(' ', "T");
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M"))
        .ok()
}

pub fn norm_dt(s: &str) -> Option<String> {
    parse_dt(s).map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
}

pub async fn audit(
    db: &PgPool,
    actor: Option<i64>,
    action: &str,
    entity: &str,
    entity_id: Option<i64>,
    detail: impl AsRef<str>,
) {
    let r = sqlx::query(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(actor)
    .bind(action)
    .bind(entity)
    .bind(entity_id)
    .bind(detail.as_ref())
    .bind(now())
    .execute(db)
    .await;
    if let Err(e) = r {
        tracing::error!("audit insert failed: {e}");
    }
}

/// One audit row per id, in a single statement (inside a transaction when given one).
pub async fn audit_many<'e>(
    db: impl sqlx::PgExecutor<'e>,
    actor: Option<i64>,
    action: &str,
    entity: &str,
    entity_ids: &[i64],
    detail: &str,
) -> sqlx::Result<()> {
    if entity_ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at)
         SELECT $1, $2, $3, id, $5, $6 FROM unnest($4::BIGINT[]) AS id",
    )
    .bind(actor)
    .bind(action)
    .bind(entity)
    .bind(entity_ids)
    .bind(detail)
    .bind(now())
    .execute(db)
    .await?;
    Ok(())
}

pub use crate::i18n::{t, ErrorKey, Lang};

async fn user_lang(db: &PgPool, user_id: i64) -> Lang {
    let row: Option<(String,)> = sqlx::query_as("SELECT lang FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    row.map(|(l,)| Lang::from_code(&l)).unwrap_or(Lang::Zh)
}

/// Insert one notification. Title and body are (中文, English) pairs and the
/// recipient's saved language decides which side is rendered.
pub async fn notify(
    db: &PgPool,
    user_id: i64,
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    let lang = user_lang(db, user_id).await;
    let title_s = t(lang, title.0, title.1).to_string();
    let body_s = t(lang, body.0, body.1).to_string();
    let r = sqlx::query(
        "INSERT INTO notifications (user_id, title, body, link, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(user_id)
    .bind(&title_s)
    .bind(&body_s)
    .bind(link)
    .bind(now())
    .execute(db)
    .await;
    if let Err(e) = r {
        tracing::error!("notify failed: {e}");
    } else {
        crate::stream::publish(user_id, "notify");
        tokio::spawn(dispatch_external_alerts(
            db.clone(),
            vec![(user_id, title_s, body_s, link.to_string())],
        ));
    }
}

/// Wake every member's open tab for this trip (new chat message).
pub async fn notify_trip(db: &PgPool, trip_id: i64) {
    let ids: Vec<(i64,)> = sqlx::query_as(
        "SELECT passenger_id FROM rides WHERE trip_id=$1 AND status <> 'cancelled'
         UNION SELECT driver_id FROM rides WHERE trip_id=$1 AND driver_id IS NOT NULL",
    )
    .bind(trip_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for (id,) in ids {
        crate::stream::publish(id, "chat");
    }
}

/// The same notification for many members in one statement; each recipient gets the
/// side of each (中文, English) pair that matches their saved language. Ids that are
/// not members are skipped.
pub async fn notify_many(
    db: &PgPool,
    user_ids: &[i64],
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    if user_ids.is_empty() {
        return;
    }
    let r: sqlx::Result<Vec<(i64, String, String)>> = sqlx::query_as(
        "INSERT INTO notifications (user_id, title, body, link, created_at)
         SELECT u.id,
                CASE WHEN u.lang = 'en' THEN $3 ELSE $2 END,
                CASE WHEN u.lang = 'en' THEN $5 ELSE $4 END,
                $6, $7
         FROM users u WHERE u.id = ANY($1) AND u.status = 'active'
         RETURNING user_id, title, body",
    )
    .bind(user_ids)
    .bind(title.0)
    .bind(title.1)
    .bind(body.0)
    .bind(body.1)
    .bind(link)
    .bind(now())
    .fetch_all(db)
    .await;
    match r {
        Ok(rows) => {
            let mut alerts = Vec::with_capacity(rows.len());
            for (id, t_s, b_s) in rows {
                crate::stream::publish(id, "notify");
                alerts.push((id, t_s, b_s, link.to_string()));
            }
            if !alerts.is_empty() {
                tokio::spawn(dispatch_external_alerts(db.clone(), alerts));
            }
        }
        Err(e) => tracing::error!("notify_many failed: {e}"),
    }
}

static ALERT_HTTP: OnceLock<reqwest::Client> = OnceLock::new();

pub fn alert_client() -> &'static reqwest::Client {
    ALERT_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default()
    })
}

fn is_metadata_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4 == std::net::Ipv4Addr::new(169, 254, 169, 254),
        std::net::IpAddr::V6(v6) => {
            v6.to_ipv4_mapped() == Some(std::net::Ipv4Addr::new(169, 254, 169, 254))
        }
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

fn is_non_global_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => is_non_global_v4(v4),
        std::net::IpAddr::V6(v6) => {
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

async fn validate_http_target(raw: &str, max_len: usize, dev_mode: bool) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > max_len {
        return false;
    }
    let Ok(parsed) = url::Url::parse(trimmed) else {
        return false;
    };
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return false;
    }
    let scheme = parsed.scheme();
    if dev_mode {
        if scheme != "https" && scheme != "http" {
            return false;
        }
    } else if scheme != "https" {
        return false;
    }
    let Some(host) = parsed.host() else {
        return false;
    };
    let host_lower = parsed
        .host_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host_lower.is_empty()
        || host_lower == "metadata.google.internal"
        || host_lower == "169.254.169.254"
    {
        return false;
    }
    match host {
        url::Host::Ipv4(v4) => {
            let ip = std::net::IpAddr::V4(v4);
            if is_metadata_ip(ip) {
                return false;
            }
            dev_mode || !is_non_global_v4(v4)
        }
        url::Host::Ipv6(v6) => {
            let ip = std::net::IpAddr::V6(v6);
            if is_metadata_ip(ip) {
                return false;
            }
            dev_mode || !is_non_global_ip(ip)
        }
        url::Host::Domain(domain) => {
            let d = domain.trim_end_matches('.').to_ascii_lowercase();
            if d == "metadata.google.internal" {
                return false;
            }
            if dev_mode {
                return true;
            }
            if d == "localhost"
                || d.ends_with(".localhost")
                || d.ends_with(".local")
                || d.ends_with(".internal")
                || !d.contains('.')
            {
                return false;
            }
            let port = parsed.port_or_known_default().unwrap_or(443);
            let Ok(addrs) = tokio::net::lookup_host((d.as_str(), port)).await else {
                return false;
            };
            let mut saw_any = false;
            for addr in addrs {
                saw_any = true;
                if is_non_global_ip(addr.ip()) {
                    return false;
                }
            }
            saw_any
        }
    }
}

const DEFAULT_PUSH_HOST_RULES: &[&str] = &[
    "fcm.googleapis.com",
    "push.googleapis.com",
    "android.googleapis.com",
    "updates.push.services.mozilla.com",
    ".push.services.mozilla.com",
    ".push.apple.com",
    ".notify.windows.com",
    ".push.samsungosp.com",
];

fn matches_host_rule(host: &str, rule: &str) -> bool {
    let r = rule.trim().trim_end_matches('.').to_ascii_lowercase();
    if r.is_empty() {
        return false;
    }
    if let Some(suffix) = r.strip_prefix('.') {
        !suffix.is_empty() && host.len() > r.len() && host.ends_with(&r)
    } else {
        host == r
    }
}

/// True when `host` belongs to a known browser Web Push vendor (or `PUSH_ALLOWED_HOSTS` override).
pub fn is_allowed_push_host(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if h.is_empty() || h.parse::<std::net::IpAddr>().is_ok() {
        return false;
    }
    if let Ok(custom) = std::env::var("PUSH_ALLOWED_HOSTS") {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            return trimmed.split(',').any(|rule| matches_host_rule(&h, rule));
        }
    }
    DEFAULT_PUSH_HOST_RULES
        .iter()
        .any(|rule| matches_host_rule(&h, rule))
}

/// Validates an external alert webhook URL against SSRF targets.
pub async fn validate_webhook_target(raw: &str, dev_mode: bool) -> bool {
    validate_http_target(raw, 500, dev_mode).await
}

/// Validates a browser Web Push subscription endpoint against SSRF targets and,
/// outside `DEV_MODE`, restricts destinations to known Web Push service domains.
pub async fn validate_push_endpoint(raw: &str, dev_mode: bool) -> bool {
    if !dev_mode {
        let Ok(parsed) = url::Url::parse(raw.trim()) else {
            return false;
        };
        let Some(url::Host::Domain(domain)) = parsed.host() else {
            return false;
        };
        if !is_allowed_push_host(domain) {
            return false;
        }
    }
    validate_http_target(raw, 1000, dev_mode).await
}

async fn dispatch_external_alerts(db: PgPool, recipients: Vec<(i64, String, String, String)>) {
    if recipients.is_empty() {
        return;
    }
    let secrets: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets WHERE name IN ('telegram_bot_token', 'alert_webhook_url')",
    )
    .fetch_all(&db)
    .await
    .unwrap_or_default();

    let mut tg_token = String::new();
    let mut webhook_url = String::new();
    for (k, v) in secrets {
        match k.as_str() {
            "telegram_bot_token" => tg_token = v.trim().to_string(),
            "alert_webhook_url" => webhook_url = v.trim().to_string(),
            _ => {}
        }
    }
    let dev_mode = is_dev_mode();
    let webhook_ok =
        !webhook_url.is_empty() && validate_webhook_target(&webhook_url, dev_mode).await;
    if !webhook_url.is_empty() && !webhook_ok {
        tracing::warn!("blocked alert webhook URL failing SSRF policy");
    }
    let vapid_signer = crate::web_push::load_vapid_signer(&db)
        .await
        .map(std::sync::Arc::new);

    if tg_token.is_empty() && !webhook_ok && vapid_signer.is_none() {
        return;
    }

    let ids: Vec<i64> = recipients.iter().map(|(id, _, _, _)| *id).collect();
    let prefs: Vec<(i64, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT id, name, phone, COALESCE(telegram_chat_id, ''), whatsapp_opt_in, push_opt_in
         FROM users WHERE id = ANY($1) AND status = 'active'",
    )
    .bind(&ids)
    .fetch_all(&db)
    .await
    .unwrap_or_default();

    let pref_map: std::collections::HashMap<i64, (String, String, String, i64, i64)> = prefs
        .into_iter()
        .map(|(id, name, phone, tg, wa, push)| (id, (name, phone, tg, wa, push)))
        .collect();

    let mut push_map: std::collections::HashMap<i64, Vec<(i64, String, String, String)>> =
        std::collections::HashMap::new();
    if vapid_signer.is_some() {
        let push_ids: Vec<i64> = pref_map
            .iter()
            .filter_map(|(uid, (_, _, _, _, push_opt_in))| (*push_opt_in == 1).then_some(*uid))
            .collect();
        if !push_ids.is_empty() {
            let subs: Vec<(i64, i64, String, String, String)> = sqlx::query_as(
                "SELECT id, user_id, endpoint, p256dh, auth
                 FROM push_subscriptions WHERE user_id = ANY($1)",
            )
            .bind(&push_ids)
            .fetch_all(&db)
            .await
            .unwrap_or_default();
            for (sub_id, uid, endpoint, p256dh, auth) in subs {
                push_map
                    .entry(uid)
                    .or_default()
                    .push((sub_id, endpoint, p256dh, auth));
            }
        }
    }

    let client = alert_client().clone();
    let subject = vapid_subject();
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
    let mut join_set = tokio::task::JoinSet::new();
    for (uid, title, body, link) in recipients {
        let Some((name, phone, tg_chat_id, wa_opt_in, push_opt_in)) = pref_map.get(&uid).cloned()
        else {
            continue;
        };
        let has_tg = !tg_token.is_empty() && !tg_chat_id.trim().is_empty();
        let hook_opt = if webhook_ok && wa_opt_in == 1 {
            Some(webhook_url.clone())
        } else {
            None
        };
        if has_tg || hook_opt.is_some() {
            let tg_token = tg_token.clone();
            let client = client.clone();
            let sem = sem.clone();
            let title_c = title.clone();
            let body_c = body.clone();
            let link_c = link.clone();
            join_set.spawn(async move {
                let Ok(_permit) = sem.acquire_owned().await else {
                    return;
                };
                if !tg_token.is_empty() && !tg_chat_id.trim().is_empty() {
                    let text = format!("{title_c}\n{body_c}\n{link_c}");
                    let url = format!("https://api.telegram.org/bot{tg_token}/sendMessage");
                    let _ = client
                        .post(&url)
                        .json(&serde_json::json!({
                            "chat_id": tg_chat_id.trim(),
                            "text": text,
                        }))
                        .send()
                        .await;
                }
                if let Some(hook) = hook_opt {
                    let _ = client
                        .post(&hook)
                        .json(&serde_json::json!({
                            "channel": "whatsapp",
                            "user_id": uid,
                            "name": name,
                            "phone": phone,
                            "title": title_c,
                            "body": body_c,
                            "link": link_c,
                        }))
                        .send()
                        .await;
                }
            });
        }

        if push_opt_in == 1 {
            if let (Some(signer), Some(subs)) = (vapid_signer.as_ref(), push_map.get(&uid)) {
                for (sub_id, endpoint, p256dh, auth) in subs.iter().cloned() {
                    let signer = signer.clone();
                    let client = client.clone();
                    let db = db.clone();
                    let sem = sem.clone();
                    let subject = subject.clone();
                    let title_c = title.clone();
                    let body_c = body.clone();
                    let link_c = link.clone();
                    join_set.spawn(async move {
                        let Ok(_permit) = sem.acquire_owned().await else {
                            return;
                        };
                        if !validate_push_endpoint(&endpoint, dev_mode).await {
                            return;
                        }
                        if crate::web_push::send_push_message(
                            &client, &signer, &subject, &endpoint, &p256dh, &auth, &title_c,
                            &body_c, &link_c,
                        )
                        .await
                            == crate::web_push::PushOutcome::Gone
                        {
                            let _ = sqlx::query("DELETE FROM push_subscriptions WHERE id=$1")
                                .bind(sub_id)
                                .execute(&db)
                                .await;
                        }
                    });
                }
            }
        }
    }
    while join_set.join_next().await.is_some() {}
}

/// Notify every active member who may do `act` on `page`; each recipient gets the
/// message in their own language.
pub async fn notify_perm(
    db: &PgPool,
    page: Page,
    act: Act,
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    let users: Vec<(i64,)> = sqlx::query_as(&format!(
        "SELECT u.id FROM users u WHERE {}",
        perms::sql_holds("u", Perm(page, act))
    ))
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let ids: Vec<i64> = users.into_iter().map(|(id,)| id).collect();
    notify_many(db, &ids, title, body, link).await;
}

/// Chat system message: one shared row for the whole trip, so both languages
/// ride along (中文 / English) and every reader understands it.
pub async fn system_message(db: &PgPool, trip_id: i64, zh_body: &str, en_body: &str) {
    let _ = sqlx::query(
        "INSERT INTO ride_messages (trip_id, user_id, body, created_at) VALUES ($1,NULL,$2,$3)",
    )
    .bind(trip_id)
    .bind(if zh_body == en_body {
        zh_body.to_string()
    } else {
        format!("{zh_body} / {en_body}")
    })
    .bind(now())
    .execute(db)
    .await;
    notify_trip(db, trip_id).await;
}

/// Refuse unless the member may do `act` on `page`. A member who holds the permission
/// but signed in without a passkey / 2FA is told to sign in properly instead.
pub fn require(u: &CurrentUser, page: Page, act: Act) -> AppResult<()> {
    if u.can(page, act) {
        Ok(())
    } else if u.perms.has(page, act) {
        Err(forbidden(u.lang, ErrorKey::MfaRequired))
    } else {
        Err(forbidden(u.lang, ErrorKey::NoPermission))
    }
}

/// As `require`, for a page reached through more than one permission.
pub fn require_one(u: &CurrentUser, any: &[(Page, Act)]) -> AppResult<()> {
    if any.iter().any(|(p, a)| u.can(*p, *a)) {
        Ok(())
    } else if any.iter().any(|(p, a)| u.perms.has(*p, *a)) {
        Err(forbidden(u.lang, ErrorKey::MfaRequired))
    } else {
        Err(forbidden(u.lang, ErrorKey::NoPermission))
    }
}

pub fn fmt_rm(cents: i64) -> String {
    format!("RM {}.{:02}", cents / 100, (cents % 100).abs())
}

/// Escape `%`, `_`, and `\` for `ILIKE ... ESCAPE '\'`.
pub fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Split an opaque `"timestamp|id"` pagination cursor, falling back to plain `"timestamp"`.
pub fn split_cursor(raw: &str) -> (&str, Option<i64>) {
    match raw.rsplit_once('|') {
        Some((ts, id_str)) => match id_str.parse::<i64>() {
            Ok(id) => (ts, Some(id)),
            Err(_) => (raw, None),
        },
        None => (raw, None),
    }
}

/// Build an `ORDER BY` body from an allow-list of `(client_key, SQL expression)`.
/// Unknown keys use `default`; any direction other than `desc` sorts ascending.
/// `tiebreak` keeps paginated results stable when the sort value repeats.
pub fn order_by(
    sort: Option<&str>,
    dir: Option<&str>,
    allowed: &[(&str, &str)],
    default: &str,
    tiebreak: &str,
) -> String {
    let Some((_, column)) = sort
        .map(str::trim)
        .and_then(|key| allowed.iter().find(|(name, _)| *name == key))
    else {
        return default.to_string();
    };
    let direction = if dir
        .map(str::trim)
        .is_some_and(|d| d.eq_ignore_ascii_case("desc"))
    {
        "DESC"
    } else {
        "ASC"
    };
    format!("{column} {direction} NULLS LAST, {tiebreak}")
}

#[cfg(test)]
mod order_by_tests {
    use super::order_by;

    const ALLOWED: &[(&str, &str)] = &[("name", "u.name"), ("created", "u.created_at")];

    #[test]
    fn uses_allow_listed_column_and_direction() {
        assert_eq!(
            order_by(
                Some("name"),
                Some("desc"),
                ALLOWED,
                "u.name ASC",
                "u.id ASC"
            ),
            "u.name DESC NULLS LAST, u.id ASC"
        );
        assert_eq!(
            order_by(Some("name"), None, ALLOWED, "u.name ASC", "u.id ASC"),
            "u.name ASC NULLS LAST, u.id ASC"
        );
    }

    #[test]
    fn falls_back_for_unknown_sort_and_direction() {
        assert_eq!(
            order_by(
                Some("name; DROP TABLE users"),
                Some("desc"),
                ALLOWED,
                "u.name ASC, u.id ASC",
                "u.id ASC"
            ),
            "u.name ASC, u.id ASC"
        );
        assert_eq!(
            order_by(
                Some("created"),
                Some("sideways"),
                ALLOWED,
                "u.name ASC",
                "u.id ASC"
            ),
            "u.created_at ASC NULLS LAST, u.id ASC"
        );
    }
}
