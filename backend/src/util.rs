//! Shared helpers: time, audit log, notifications, permission checks.

use chrono::{Duration, Local, NaiveDateTime};
use rand::{distributions::Alphanumeric, Rng};
use sqlx::SqlitePool;

use crate::auth::CurrentUser;
use crate::error::{forbidden, AppResult};

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
    db: &SqlitePool,
    actor: Option<i64>,
    action: &str,
    entity: &str,
    entity_id: Option<i64>,
    detail: impl AsRef<str>,
) {
    let r = sqlx::query(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at) VALUES (?,?,?,?,?,?)",
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

pub async fn notify(db: &SqlitePool, user_id: i64, title: &str, body: impl AsRef<str>, link: &str) {
    let r = sqlx::query(
        "INSERT INTO notifications (user_id, title, body, link, created_at) VALUES (?,?,?,?,?)",
    )
    .bind(user_id)
    .bind(title)
    .bind(body.as_ref())
    .bind(link)
    .bind(now())
    .execute(db)
    .await;
    if let Err(e) = r {
        tracing::error!("notify failed: {e}");
    }
}

/// Notify every active user holding any of the given roles.
pub async fn notify_roles(db: &SqlitePool, roles: &[&str], title: &str, body: &str, link: &str) {
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

pub async fn system_message(db: &SqlitePool, trip_id: i64, body: &str) {
    let _ = sqlx::query(
        "INSERT INTO ride_messages (trip_id, user_id, body, created_at) VALUES (?,NULL,?,?)",
    )
    .bind(trip_id)
    .bind(body)
    .bind(now())
    .execute(db)
    .await;
}

pub fn require_any(u: &CurrentUser, roles: &[&str]) -> AppResult<()> {
    if roles.iter().any(|r| u.has(r)) {
        if !u.mfa_ok {
            return Err(forbidden("此功能需要通行密钥或双重验证（2FA）登入"));
        }
        Ok(())
    } else {
        Err(forbidden("您没有权限执行此操作"))
    }
}

pub fn fmt_rm(cents: i64) -> String {
    format!("RM {}.{:02}", cents / 100, (cents % 100).abs())
}
