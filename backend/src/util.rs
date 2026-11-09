//! Shared helpers: time, audit log, notifications, permission checks.

use chrono::{Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, Utc, Weekday};
use rand::{distributions::Alphanumeric, Rng};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::error::{forbidden, AppResult};
use serde::Serialize;

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
pub async fn notify(db: &PgPool, user_id: i64, title: (&str, &str), body: (&str, &str), link: &str) {
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
    }
}

/// Notify every active user holding any of the given roles; each recipient
/// gets the message in their own language.
pub async fn notify_roles(
    db: &PgPool,
    roles: &[&str],
    title: (&str, &str),
    body: (&str, &str),
    link: &str,
) {
    let users: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, roles FROM users WHERE status='active'")
            .fetch_all(db)
            .await
            .unwrap_or_default();
    for (id, r) in users {
        if r.split(',').any(|x| roles.contains(&x.trim())) {
            notify(db, id, title, body, link).await;
        }
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
}

pub fn require_any(u: &CurrentUser, roles: &[&str]) -> AppResult<()> {
    if roles.iter().any(|r| u.has(r)) {
        if !u.mfa_ok {
            return Err(forbidden(u.lang, ErrorKey::MfaRequired));
        }
        Ok(())
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
