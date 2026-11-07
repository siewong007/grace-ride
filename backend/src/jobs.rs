//! Background jobs, run once a minute in-process:
//! - 接送前提醒: remind passenger + driver ~60 minutes before a confirmed pickup
//! - 活动提醒: remind attendees ~24 hours before an event
//! - 没人接单: flag rides still without a driver 12 hours before pickup for 交通组
//! - housekeeping: expired sessions/challenges, stale GPS, old login failures
//! - retention (资料保存期限): purge chat messages and audit entries past the configured age

use std::time::Duration;

use sqlx::SqlitePool;

use crate::state::Config;
use crate::util::{notify, notify_roles, now};

/// Local timestamp `minutes` from now, in the same "YYYY-MM-DDTHH:MM:SS" format the app stores.
const AT: &str = "strftime('%Y-%m-%dT%H:%M:%S','now','localtime', ?)";

pub fn spawn(db: SqlitePool, cfg: Config) {
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

pub async fn run_once(db: &SqlitePool, cfg: &Config) -> anyhow::Result<()> {
    ride_reminders(db).await?;
    event_reminders(db).await?;
    escalate_unmatched(db).await?;
    housekeeping(db, cfg).await?;
    Ok(())
}

async fn ride_reminders(db: &SqlitePool) -> anyhow::Result<()> {
    let due: Vec<(i64, i64, Option<i64>, String, String, String, String)> = sqlx::query_as(&format!(
        "SELECT r.id, r.passenger_id, r.driver_id, r.pickup_at, a.name, p.name, COALESCE(d.name,'')
         FROM rides r JOIN areas a ON a.id=r.pickup_area_id JOIN users p ON p.id=r.passenger_id LEFT JOIN users d ON d.id=r.driver_id
         WHERE r.status='confirmed' AND r.reminder_sent=0 AND r.pickup_at <= {AT} AND r.pickup_at >= {AT}"
    ))
    .bind("+60 minutes")
    .bind("-10 minutes")
    .fetch_all(db)
    .await?;
    for (id, passenger, driver, at, area, pname, dname) in due {
        let time = &at[11..16];
        notify(
            db,
            passenger,
            "接送提醒",
            format!("司机 {dname} 将于 {time} 到 {area} 接您。"),
            &format!("/rides/{id}"),
        )
        .await;
        if let Some(d) = driver {
            notify(
                db,
                d,
                "接送提醒",
                format!("{time} 接载 {pname}（{area}）。出发时请按「开始行程」。"),
                &format!("/rides/{id}"),
            )
            .await;
        }
        sqlx::query("UPDATE rides SET reminder_sent=1 WHERE id=?")
            .bind(id)
            .execute(db)
            .await?;
    }
    Ok(())
}

async fn event_reminders(db: &SqlitePool) -> anyhow::Result<()> {
    let due: Vec<(i64, String, String, String)> = sqlx::query_as(&format!(
        "SELECT id, title, starts_at, location FROM events
         WHERE status='published' AND reminder_sent=0 AND starts_at <= {AT} AND starts_at >= {AT}"
    ))
    .bind("+24 hours")
    .bind("+0 minutes")
    .fetch_all(db)
    .await?;
    for (id, title, at, loc) in due {
        let users: Vec<(i64,)> =
            sqlx::query_as("SELECT user_id FROM event_signups WHERE event_id=? AND status='going'")
                .bind(id)
                .fetch_all(db)
                .await?;
        for (u,) in users {
            notify(
                db,
                u,
                "活动提醒",
                format!("{title} · {} · {loc}", at.replace('T', " ")),
                &format!("/events/{id}"),
            )
            .await;
        }
        sqlx::query("UPDATE events SET reminder_sent=1 WHERE id=?")
            .bind(id)
            .execute(db)
            .await?;
    }
    Ok(())
}

async fn escalate_unmatched(db: &SqlitePool) -> anyhow::Result<()> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(&format!(
        "SELECT r.id, p.name, r.pickup_at FROM rides r JOIN users p ON p.id=r.passenger_id
         WHERE r.status='pending_driver' AND r.needs_attention=0 AND r.pickup_at <= {AT} AND r.pickup_at >= {AT}"
    ))
    .bind("+12 hours")
    .bind("+0 minutes")
    .fetch_all(db)
    .await?;
    for (id, name, at) in rows {
        sqlx::query("UPDATE rides SET needs_attention=1, attention_reason='接送前 12 小时仍未有司机', updated_at=? WHERE id=?")
            .bind(now())
            .bind(id)
            .execute(db)
            .await?;
        notify_roles(
            db,
            &["transport"],
            "没人接单",
            &format!("{name} · {}", at.replace('T', " ")),
            &format!("/admin/rides/{id}"),
        )
        .await;
    }
    Ok(())
}

async fn housekeeping(db: &SqlitePool, cfg: &Config) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
        .bind(now())
        .execute(db)
        .await?;
    sqlx::query(&format!(
        "DELETE FROM auth_challenges WHERE created_at < {AT}"
    ))
    .bind("-10 minutes")
    .execute(db)
    .await?;
    sqlx::query(&format!(
        "DELETE FROM login_failures WHERE created_at < {AT}"
    ))
    .bind("-1 day")
    .execute(db)
    .await?;
    // GPS is only meaningful while a trip is underway.
    sqlx::query("DELETE FROM driver_locations WHERE driver_id NOT IN (SELECT driver_id FROM rides WHERE status='in_progress' AND driver_id IS NOT NULL)")
        .execute(db)
        .await?;
    sqlx::query(&format!(
        "DELETE FROM ride_messages WHERE created_at < {AT}
         AND trip_id NOT IN (SELECT trip_id FROM rides WHERE trip_id IS NOT NULL AND status IN ('confirmed','in_progress'))"
    ))
    .bind(format!("-{} days", cfg.chat_retention_days))
    .execute(db)
    .await?;
    sqlx::query(&format!("DELETE FROM audit_logs WHERE created_at < {AT}"))
        .bind(format!("-{} days", cfg.audit_retention_days))
        .execute(db)
        .await?;
    sqlx::query(&format!(
        "DELETE FROM notifications WHERE read_at IS NOT NULL AND created_at < {AT}"
    ))
    .bind("-90 days")
    .execute(db)
    .await?;
    Ok(())
}
