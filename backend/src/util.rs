//! Shared helpers: time, audit log, notifications, permission checks.

use chrono::{Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, Utc};
use rand::{distributions::Alphanumeric, Rng};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppResult};
use crate::perms::{self, Act, Page, Perm};

pub fn now() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn in_days(days: i64) -> String {
    (Local::now() + Duration::days(days))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

pub fn in_hours(hours: i64) -> String {
    (Local::now() + Duration::hours(hours))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

/// Timestamp `minutes` ago, in the same text format as `now()`, for comparisons.
pub fn minutes_ago(minutes: i64) -> String {
    (Local::now() - Duration::minutes(minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
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
    let r = sqlx::query(
        "INSERT INTO notifications (user_id, title, body, link, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(user_id)
    .bind(t(lang, title.0, title.1))
    .bind(t(lang, body.0, body.1))
    .bind(link)
    .bind(now())
    .execute(db)
    .await;
    if let Err(e) = r {
        tracing::error!("notify failed: {e}");
    } else {
        crate::stream::publish(user_id, "notify");
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
    for (id,) in users {
        notify(db, id, title, body, link).await;
    }
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

/// Asia/Kuala Lumpur is UTC+8. All demo timestamps are generated with this
/// fixed offset so seed data is reproducible regardless of the host time zone.
pub const KL_OFFSET_SECS: i32 = 8 * 3600;

pub fn now_kl() -> String {
    let offset = FixedOffset::east_opt(KL_OFFSET_SECS).expect("UTC+8 is a valid offset");
    Utc::now()
        .with_timezone(&offset)
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

/// Demo "today" in KL time; all relative seed dates anchor on this date.
pub fn today_kl() -> NaiveDate {
    let offset = FixedOffset::east_opt(KL_OFFSET_SECS).expect("UTC+8 is a valid offset");
    Utc::now().with_timezone(&offset).date_naive()
}
