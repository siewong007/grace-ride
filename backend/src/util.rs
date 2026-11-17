//! Shared helpers: time, audit log, notifications, permission checks.

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
         FROM users u WHERE u.id = ANY($1)
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

fn alert_client() -> &'static reqwest::Client {
    ALERT_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default()
    })
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
    if tg_token.is_empty() && webhook_url.is_empty() {
        return;
    }

    let ids: Vec<i64> = recipients.iter().map(|(id, _, _, _)| *id).collect();
    let prefs: Vec<(i64, String, String, String, i64)> = sqlx::query_as(
        "SELECT id, name, phone, COALESCE(telegram_chat_id, ''), whatsapp_opt_in
         FROM users WHERE id = ANY($1)",
    )
    .bind(&ids)
    .fetch_all(&db)
    .await
    .unwrap_or_default();

    let pref_map: std::collections::HashMap<i64, (String, String, String, i64)> = prefs
        .into_iter()
        .map(|(id, name, phone, tg, wa)| (id, (name, phone, tg, wa)))
        .collect();

    let client = alert_client();
    for (uid, title, body, link) in recipients {
        let Some((name, phone, tg_chat_id, wa_opt_in)) = pref_map.get(&uid) else {
            continue;
        };
        if !tg_token.is_empty() && !tg_chat_id.trim().is_empty() {
            let text = format!("{title}\n{body}\n{link}");
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
        if !webhook_url.is_empty() && *wa_opt_in == 1 {
            let _ = client
                .post(&webhook_url)
                .json(&serde_json::json!({
                    "channel": "whatsapp",
                    "user_id": uid,
                    "name": name,
                    "phone": phone,
                    "title": title,
                    "body": body,
                    "link": link,
                }))
                .send()
                .await;
        }
    }
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
