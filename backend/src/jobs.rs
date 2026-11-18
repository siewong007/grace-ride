//! Background maintenance.
//! Every minute:
//! - reminders: confirmed rides ~1h away, published events ~24h away
//! - escalation: unmatched rides still driverless ~12h before pickup
//!
//! Every HOUSEKEEPING_INTERVAL_MINS (default hourly):
//! - housekeeping: expired sessions/challenges, stale GPS, old login failures, old geocodes
//! - retention (资料保存期限): purge chat messages, audit entries and notifications past
//!   the configured age
//!
//! Each reminder and escalation claims its rows with `UPDATE … RETURNING` before
//! notifying, so a row is handled once even if two ticks (or two instances) overlap.

use std::time::{Duration, Instant};

use sqlx::PgPool;

use crate::perms::{Act, Page};
use crate::state::Config;
use crate::util::{audit_many, notify, notify_many, notify_perm, now};

pub fn spawn(db: PgPool, cfg: Config) {
    let (_tx, rx) = tokio::sync::watch::channel(false);
    spawn_with_shutdown(db, cfg, rx);
}

pub fn spawn_with_shutdown(
    db: PgPool,
    cfg: Config,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let every = Duration::from_secs(cfg.housekeeping_interval_mins.max(1) as u64 * 60);
        let mut last_housekeeping: Option<Instant> = None;
        loop {
            tokio::select! {
                biased;
                res = shutdown.changed() => {
                    if res.is_err() || *shutdown.borrow() {
                        tracing::info!("background jobs stopping for shutdown");
                        break;
                    }
                    continue;
                }
                _ = tick.tick() => {}
            }
            if *shutdown.borrow() {
                break;
            }
            if let Err(e) = run_reminders(&db).await {
                tracing::error!("background job failed: {e:?}");
            }
            if last_housekeeping.is_none_or(|t| t.elapsed() >= every) {
                last_housekeeping = Some(Instant::now());
                if let Err(e) = housekeeping(&db, &cfg).await {
                    tracing::error!("housekeeping failed: {e:?}");
                }
            }
        }
    });
}

/// Run a single tick of the background jobs.
pub async fn run_once(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    run_reminders(db).await?;
    housekeeping(db, cfg).await?;
    Ok(())
}

/// The once-a-minute part: reminders, escalation, closing finished visitor links.
pub async fn run_reminders(db: &PgPool) -> anyhow::Result<()> {
    ride_reminders(db).await?;
    event_reminders(db).await?;
    escalate_unmatched(db).await?;
    expire_stale_requests(db).await?;
    // Standing weekly ride & driver schedules.
    crate::routes::rides::process_recurring_schedules(db).await?;
    // Visitor links close for good once their journey is over.
    crate::routes::visitors::close_finished(db).await?;
    Ok(())
}

fn ts_in(minutes: i64) -> String {
    crate::util::in_minutes(minutes)
}

async fn ride_reminders(db: &PgPool) -> anyhow::Result<()> {
    let candidates: Vec<(
        i64,
        i64,
        Option<i64>,
        String,
        String,
        String,
        String,
        Option<i64>,
        i64,
        String,
    )> = sqlx::query_as(
        "SELECT r.id, r.passenger_id, r.driver_id, r.pickup_at, a.name, p.name, COALESCE(d.name,''), p.readiness_prompt_mins,
                r.is_proxy, r.rider_name
         FROM rides r
         JOIN areas a ON a.id=r.pickup_area_id
         JOIN users p ON p.id=r.passenger_id
         LEFT JOIN users d ON d.id=r.driver_id
         WHERE r.status='confirmed' AND r.reminder_sent=0 AND r.pickup_at <= $1 AND r.pickup_at >= $2",
    )
    .bind(ts_in(180))
    .bind(ts_in(-10))
    .fetch_all(db)
    .await?;
    for (id, passenger, driver, at, area, pname, dname, prompt_mins, is_proxy, rider_name) in
        candidates
    {
        let prompt = prompt_mins.unwrap_or(45);
        let lead = if prompt <= 0 {
            60
        } else {
            prompt.clamp(5, 180)
        };
        if at > ts_in(lead) {
            continue;
        }
        let claimed = sqlx::query(
            "UPDATE rides SET reminder_sent=1 WHERE id=$1 AND status='confirmed' AND reminder_sent=0",
        )
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
        if claimed == 0 {
            continue;
        }
        let time = at.get(11..16).unwrap_or(&at);
        if prompt > 0 {
            let rider_disp = if is_proxy == 1 && !rider_name.is_empty() {
                rider_name.clone()
            } else {
                "您".to_string()
            };
            let rider_disp_en = if is_proxy == 1 && !rider_name.is_empty() {
                rider_name.clone()
            } else {
                "you".to_string()
            };
            notify(
                db,
                passenger,
                ("接送提醒", "Ride reminder"),
                (
                    &format!("司机 {dname} 将于 {time} 到 {area} 接 {rider_disp}。请确认长辈/乘车人是否准备就绪或告知需稍候/无法出行。"),
                    &format!("Driver {dname} will pick up {rider_disp_en} at {area} at {time}. Please tap to confirm readiness, delay, or inability to join."),
                ),
                &format!("/rides/{id}?action=readiness"),
            )
            .await;
        }
        if let Some(d) = driver {
            let driver_target = if is_proxy == 1 && !rider_name.is_empty() {
                format!("{rider_name} (由 {pname} 代订)")
            } else {
                pname.clone()
            };
            notify(
                db,
                d,
                ("接送提醒", "Ride reminder"),
                (
                    &format!("{time} 接载 {driver_target}（{area}）。出发时请按「开始行程」。"),
                    &format!("{time} pick up {driver_target} ({area}). Tap \"Start trip\" when departing."),
                ),
                &format!("/rides/{id}"),
            )
            .await;
        }
    }
    Ok(())
}

async fn event_reminders(db: &PgPool) -> anyhow::Result<()> {
    let due: Vec<(i64, String, String, String)> = sqlx::query_as(
        "UPDATE events SET reminder_sent=1
         WHERE status='published' AND reminder_sent=0 AND starts_at <= $1 AND starts_at >= $2
         RETURNING id, title, starts_at, location",
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
        let ids: Vec<i64> = users.into_iter().map(|(u,)| u).collect();
        let body = format!("{title} · {} · {loc}", at.replace('T', " "));
        notify_many(
            db,
            &ids,
            ("活动提醒", "Event reminder"),
            (&body, &body),
            &format!("/events/{id}"),
        )
        .await;
    }
    Ok(())
}

async fn escalate_unmatched(db: &PgPool) -> anyhow::Result<()> {
    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "WITH due AS (
             UPDATE rides SET needs_attention=1,
                    attention_reason='接送前 12 小时仍未有司机 / No driver confirmed 12h before pickup',
                    updated_at=$3
             WHERE status='pending_driver' AND needs_attention=0 AND pickup_at <= $1 AND pickup_at >= $2
             RETURNING id, passenger_id, pickup_at
         )
         SELECT due.id, p.name, due.pickup_at FROM due JOIN users p ON p.id = due.passenger_id",
    )
    .bind(ts_in(12 * 60))
    .bind(ts_in(0))
    .bind(now())
    .fetch_all(db)
    .await?;
    let ids: Vec<i64> = rows.iter().map(|(id, _, _)| *id).collect();
    audit_many(
        db,
        None,
        "ride.escalate",
        "ride",
        &ids,
        "no driver 12h before pickup",
    )
    .await?;
    for (id, name, at) in rows {
        notify_perm(
            db,
            Page::AdminRides,
            Act::Update,
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

/// A request that was never matched, accepted or approved and whose pickup time passed
/// hours ago will never happen: close it so it stops showing as upcoming.
async fn expire_stale_requests(db: &PgPool) -> anyhow::Result<()> {
    let expired: Vec<(i64,)> = sqlx::query_as(
        "UPDATE rides SET status='cancelled', needs_attention=0,
                attention_reason='已过接送时间，请求自动关闭 / Pickup time passed; request closed', updated_at=$2
         WHERE status IN ('pending_driver','pending_passenger','pending_approval','pending_driver_confirm')
           AND pickup_at < $1
         RETURNING id",
    )
    .bind(ts_in(-6 * 60))
    .bind(now())
    .fetch_all(db)
    .await?;
    let ids: Vec<i64> = expired.into_iter().map(|(id,)| id).collect();
    audit_many(db, None, "ride.expire", "ride", &ids, "pickup time passed").await?;
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
    sqlx::query("DELETE FROM passenger_locations WHERE ride_id NOT IN (SELECT id FROM rides WHERE status='in_progress' AND picked_up_at IS NULL)")
        .execute(db)
        .await?;
    let close_at = now();
    sqlx::query(
        "UPDATE caregiver_passes
         SET closed_at = $1
         WHERE closed_at IS NULL
           AND (
               valid_until < $1
               OR ride_id IN (SELECT id FROM rides WHERE status IN ('cancelled', 'no_show', 'alt_transport'))
           )",
    )
    .bind(&close_at)
    .execute(db)
    .await?;

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM system_settings WHERE key IN ('audit_retention_days', 'chat_retention_days')",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut audit_days = cfg.audit_retention_days;
    let mut chat_days = cfg.chat_retention_days;
    for (k, v) in rows {
        if let Ok(n) = v.parse::<i64>() {
            if k == "audit_retention_days" && (30..=3650).contains(&n) {
                audit_days = n;
            } else if k == "chat_retention_days" && (7..=3650).contains(&n) {
                chat_days = n;
            }
        }
    }

    sqlx::query(
        "DELETE FROM ride_messages WHERE created_at < $1
         AND trip_id NOT IN (SELECT trip_id FROM rides WHERE trip_id IS NOT NULL AND status IN ('confirmed','in_progress'))",
    )
    .bind(ts_in(-chat_days * 24 * 60))
    .execute(db)
    .await?;
    // A trip no ride belongs to any more and with no chat left is just a number.
    sqlx::query(
        "DELETE FROM trips t WHERE t.created_at < $1
         AND NOT EXISTS (SELECT 1 FROM rides r WHERE r.trip_id = t.id)
         AND NOT EXISTS (SELECT 1 FROM ride_messages m WHERE m.trip_id = t.id)",
    )
    .bind(ts_in(-24 * 60))
    .execute(db)
    .await?;
    sqlx::query("DELETE FROM audit_logs WHERE created_at < $1")
        .bind(ts_in(-audit_days * 24 * 60))
        .execute(db)
        .await?;
    // Geocoded addresses: re-looked-up monthly; "not found" retried after a day.
    sqlx::query(
        "DELETE FROM geocode_cache WHERE updated_at < $1 OR (lat IS NULL AND updated_at < $2)",
    )
    .bind(ts_in(-30 * 24 * 60))
    .bind(t(-24 * 60))
    .execute(db)
    .await?;
    // Read notifications go after NOTIFICATION_RETENTION_DAYS; unread ones after a year
    // (or that long, if it is longer), so the table cannot grow without end.
    let read_days = cfg.notification_retention_days;
    let unread_days = read_days.max(365);
    sqlx::query(
        "DELETE FROM notifications
         WHERE (read_at IS NOT NULL AND created_at < $1) OR created_at < $2",
    )
    .bind(ts_in(-read_days * 24 * 60))
    .bind(ts_in(-unread_days * 24 * 60))
    .execute(db)
    .await?;
    Ok(())
}
