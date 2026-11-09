//! Background maintenance (runs once a minute):
//! - reminders: confirmed rides ~1h away, published events ~24h away
//! - escalation: unmatched rides still driverless ~12h before pickup
//! - housekeeping: expired sessions/challenges, stale GPS, old login failures
//! - retention (资料保存期限): purge chat messages and audit entries past the configured age

use std::time::Duration;

use chrono::Local;
use sqlx::PgPool;

use crate::state::Config;
use crate::util::{notify, notify_roles, now};

pub fn spawn(db: PgPool, cfg: Config) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if let Err(e) = run_once(&db, &cfg).await {
                tracing::error!("background job failed: {e:?}");
            }
        }
    });
}

/// Run a single tick of the background jobs.
pub async fn run_once(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    ride_reminders(db).await?;
    event_reminders(db).await?;
    escalate_unmatched(db).await?;
    housekeeping(db, cfg).await?;
    Ok(())
}

fn ts_in(minutes: i64) -> String {
    (Local::now() + chrono::Duration::minutes(minutes))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

async fn ride_reminders(db: &PgPool) -> anyhow::Result<()> {
    let due: Vec<(i64, i64, Option<i64>, String, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.passenger_id, r.driver_id, r.pickup_at, a.name, p.name, COALESCE(d.name,'')
         FROM rides r JOIN areas a ON a.id=r.pickup_area_id JOIN users p ON p.id=r.passenger_id LEFT JOIN users d ON d.id=r.driver_id
         WHERE r.status='confirmed' AND r.reminder_sent=0 AND r.pickup_at <= $1 AND r.pickup_at >= $2",
    )
    .bind(ts_in(60))
    .bind(ts_in(-10))
    .fetch_all(db)
    .await?;
    for (id, passenger, driver, at, area, pname, dname) in due {
        let time = at.get(11..16).unwrap_or(&at);
        notify(
            db,
            passenger,
            ("接送提醒", "Ride reminder"),
            (
                &format!("司机 {dname} 将于 {time} 到 {area} 接您。"),
                &format!("Driver {dname} will pick you up at {area} at {time}."),
            ),
            &format!("/rides/{id}"),
        )
        .await;
        if let Some(d) = driver {
            notify(
                db,
                d,
                ("接送提醒", "Ride reminder"),
                (
                    &format!("{time} 接载 {pname}（{area}）。出发时请按「开始行程」。"),
                    &format!("{time} pick up {pname} ({area}). Tap \"Start trip\" when departing."),
                ),
                &format!("/rides/{id}"),
            )
            .await;
        }
        sqlx::query("UPDATE rides SET reminder_sent=1 WHERE id=$1")
            .bind(id)
            .execute(db)
            .await?;
    }
    Ok(())
}

async fn event_reminders(db: &PgPool) -> anyhow::Result<()> {
    let due: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, title, starts_at, location FROM events
         WHERE status='published' AND reminder_sent=0 AND starts_at <= $1 AND starts_at >= $2",
    )
    .bind(ts_in(24 * 60))
    .bind(ts_in(0))
    .fetch_all(db)
    .await?;
    for (id, title, at, loc) in due {
        let users: Vec<(i64,)> = sqlx::query_as(
            "SELECT user_id FROM event_signups WHERE event_id=$1 AND status='going'",
        )
        .bind(id)
        .fetch_all(db)
        .await?;
        for (u,) in users {
            notify(
                db,
                u,
                ("活动提醒", "Event reminder"),
                (
                    &format!("{title} · {} · {loc}", at.replace('T', " ")),
                    &format!("{title} · {} · {loc}", at.replace('T', " ")),
                ),
                &format!("/events/{id}"),
            )
            .await;
        }
        sqlx::query("UPDATE events SET reminder_sent=1 WHERE id=$1")
            .bind(id)
            .execute(db)
            .await?;
    }
    Ok(())
}

async fn escalate_unmatched(db: &PgPool) -> anyhow::Result<()> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT r.id, p.name, r.pickup_at FROM rides r JOIN users p ON p.id=r.passenger_id
         WHERE r.status='pending_driver' AND r.needs_attention=0 AND r.pickup_at <= $1 AND r.pickup_at >= $2",
    )
    .bind(ts_in(12 * 60))
    .bind(ts_in(0))
    .fetch_all(db)
    .await?;
    for (id, name, at) in rows {
        sqlx::query("UPDATE rides SET needs_attention=1, attention_reason='接送前 12 小时仍未有司机 / No driver confirmed 12h before pickup', updated_at=$1 WHERE id=$2")
            .bind(now())
            .bind(id)
            .execute(db)
            .await?;
        notify_roles(
            db,
            &["transport"],
            ("没人接单", "No driver"),
            (
                &format!("{name} · {}", at.replace('T', " ")),
                &format!("{name} · {}", at.replace('T', " ")),
            ),
            &format!("/admin/rides/{id}"),
        )
        .await;
    }
    Ok(())
}

async fn housekeeping(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    let t = |minutes: i64| ts_in(minutes);
    sqlx::query("DELETE FROM sessions WHERE expires_at < $1")
        .bind(now())
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE created_at < $1")
        .bind(ts_in(-10))
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM login_failures WHERE created_at < $1")
        .bind(t(-24 * 60))
        .execute(db)
        .await?;
    // GPS is only meaningful while a trip is underway.
    sqlx::query("DELETE FROM driver_locations WHERE driver_id NOT IN (SELECT driver_id FROM rides WHERE status='in_progress' AND driver_id IS NOT NULL)")
        .execute(db)
        .await?;
    sqlx::query(
        "DELETE FROM ride_messages WHERE created_at < $1
         AND trip_id NOT IN (SELECT trip_id FROM rides WHERE trip_id IS NOT NULL AND status IN ('confirmed','in_progress'))",
    )
    .bind(ts_in(-cfg.chat_retention_days * 24 * 60))
    .execute(db)
    .await?;
    sqlx::query("DELETE FROM audit_logs WHERE created_at < $1")
        .bind(ts_in(-cfg.audit_retention_days * 24 * 60))
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM notifications WHERE read_at IS NOT NULL AND created_at < $1")
        .bind(ts_in(-90 * 24 * 60))
        .execute(db)
        .await?;
    Ok(())
}
