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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use sqlx::PgPool;

use crate::notifications::{notify_batch, BatchNotifItem, NotifMeta};
use crate::perms::{Act, Page};
use crate::state::Config;
use crate::util::{audit_many, notify_many, now};

pub struct BackgroundHandles {
    delivery: tokio::task::JoinHandle<()>,
    scheduler: tokio::task::JoinHandle<()>,
}

impl BackgroundHandles {
    /// Await background workers during graceful shutdown, bounding the wait so a stuck
    /// external push or DB call cannot exceed container stop grace periods.
    pub async fn join_with_timeout(self, timeout: Duration) {
        let _ = tokio::time::timeout(timeout, async {
            let _ = tokio::join!(self.delivery, self.scheduler);
        })
        .await;
    }
}

pub fn spawn(db: PgPool, cfg: Config) {
    let (_tx, rx) = tokio::sync::watch::channel(false);
    let _ = spawn_with_shutdown(db, cfg, rx);
}

pub fn spawn_with_shutdown(
    db: PgPool,
    cfg: Config,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> BackgroundHandles {
    let delivery_db = db.clone();
    let mut delivery_shutdown = shutdown.clone();
    let delivery = tokio::spawn(async move {
        let mut delivery_tick = tokio::time::interval(Duration::from_secs(15));
        delivery_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                biased;
                res = delivery_shutdown.changed() => {
                    if res.is_err() || *delivery_shutdown.borrow() {
                        let _ = tokio::time::timeout(
                            Duration::from_secs(5),
                            crate::notifications::process_due_deliveries(&delivery_db, 50),
                        ).await;
                        break;
                    }
                    continue;
                }
                _ = delivery_tick.tick() => {}
            }
            if *delivery_shutdown.borrow() {
                break;
            }
            if let Err(e) = crate::notifications::process_due_deliveries(&delivery_db, 50).await {
                tracing::warn!("notification outbox retry tick failed: {e:?}");
            }
        }
    });

    let scheduler = tokio::spawn(async move {
        if !cfg.dev_mode {
            let db_init = db.clone();
            tokio::spawn(async move {
                let _ = db_init;
                if let Ok(client) = reqwest::Client::builder()
                    .use_rustls_tls()
                    .redirect(reqwest::redirect::Policy::none())
                    .connect_timeout(Duration::from_secs(5))
                    .timeout(Duration::from_secs(10))
                    .build()
                {
                    if let Err(e) =
                        crate::attestation::refresh_google_attestation_revocation_list(&client)
                            .await
                    {
                        tracing::warn!("initial Google attestation CRL refresh failed: {e}");
                    }
                }
            });
        }
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
                if !cfg.dev_mode {
                    if let Ok(client) = reqwest::Client::builder()
                        .use_rustls_tls()
                        .redirect(reqwest::redirect::Policy::none())
                        .connect_timeout(Duration::from_secs(5))
                        .timeout(Duration::from_secs(10))
                        .build()
                    {
                        if let Err(e) =
                            crate::attestation::refresh_google_attestation_revocation_list(&client)
                                .await
                        {
                            tracing::warn!("Google attestation CRL refresh failed: {e}");
                        }
                    }
                }
                if let Err(e) = housekeeping(&db, &cfg).await {
                    tracing::error!("housekeeping failed: {e:?}");
                }
            }
        }
    });

    BackgroundHandles {
        delivery,
        scheduler,
    }
}

/// Run a single tick of the background jobs.
pub async fn run_once(db: &PgPool, cfg: &Config) -> anyhow::Result<()> {
    run_reminders(db).await?;
    housekeeping(db, cfg).await?;
    Ok(())
}

/// The once-a-minute part: reminders, escalation, closing finished visitor links.
/// Each sub-job runs independently so a failure in one cannot starve the rest.
pub async fn run_reminders(db: &PgPool) -> anyhow::Result<()> {
    let mut first_err: Option<anyhow::Error> = None;
    macro_rules! run_step {
        ($name:literal, $fut:expr) => {
            if let Err(e) = $fut.await {
                tracing::error!(job = $name, "reminder sub-job failed: {e:?}");
                if first_err.is_none() {
                    first_err = Some(e.into());
                }
            }
        };
    }
    run_step!("ride_reminders", ride_reminders(db));
    run_step!("event_reminders", event_reminders(db));
    run_step!("escalate_unmatched", escalate_unmatched(db));
    run_step!("expire_stale_requests", expire_stale_requests(db));
    run_step!(
        "process_recurring_schedules",
        crate::routes::rides::process_recurring_schedules(db)
    );
    run_step!(
        "visitors_close_finished",
        crate::routes::visitors::close_finished(db)
    );
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn ts_in(minutes: i64) -> String {
    crate::util::in_minutes(minutes)
}

#[derive(sqlx::FromRow)]
struct PassengerReminderCandidate {
    id: i64,
    passenger_id: i64,
    pickup_at: String,
    area_name: String,
    driver_name: String,
    readiness_prompt_mins: Option<i64>,
    is_proxy: i64,
    rider_name: String,
}

#[derive(sqlx::FromRow)]
struct DriverReminderCandidate {
    id: i64,
    driver_id: i64,
    trip_id: Option<i64>,
    pickup_at: String,
    area_name: String,
    passenger_name: String,
    is_proxy: i64,
    rider_name: String,
    driver_remind_mins: Option<i64>,
}

async fn ride_reminders(db: &PgPool) -> anyhow::Result<()> {
    // Pass 1: Passenger readiness prompt (governed solely by the passenger's preference).
    let passenger_candidates: Vec<PassengerReminderCandidate> = sqlx::query_as(
        "SELECT r.id, r.passenger_id, r.pickup_at,
                a.name AS area_name,
                COALESCE(d.name, '') AS driver_name,
                p.readiness_prompt_mins,
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

    let mut due = Vec::new();
    for c in passenger_candidates {
        let prompt = c.readiness_prompt_mins.unwrap_or(45);
        let lead = if prompt <= 0 {
            60
        } else {
            prompt.clamp(5, 180)
        };
        if c.pickup_at <= ts_in(lead) {
            due.push((c, prompt));
        }
    }
    let mut batch = Vec::new();
    if !due.is_empty() {
        let due_ids: Vec<i64> = due.iter().map(|(c, _)| c.id).collect();
        let claimed_ids: HashSet<i64> = sqlx::query_scalar(
            "UPDATE rides SET reminder_sent=1 WHERE id = ANY($1) AND status='confirmed' AND reminder_sent=0 RETURNING id",
        )
        .bind(&due_ids)
        .fetch_all(db)
        .await?
        .into_iter()
        .collect();
        for (c, prompt) in due {
            if !claimed_ids.contains(&c.id) || prompt <= 0 {
                continue;
            }
            let time = c.pickup_at.get(11..16).unwrap_or(&c.pickup_at);
            let rider_disp = if c.is_proxy == 1 && !c.rider_name.is_empty() {
                c.rider_name.clone()
            } else {
                "您".to_string()
            };
            let rider_disp_en = if c.is_proxy == 1 && !c.rider_name.is_empty() {
                c.rider_name.clone()
            } else {
                "you".to_string()
            };
            batch.push(BatchNotifItem {
                user_id: c.passenger_id,
                title_zh: "接送提醒".into(),
                title_en: "Ride reminder".into(),
                body_zh: format!(
                    "司机 {} 将于 {time} 到 {} 接 {rider_disp}。请确认长辈/乘车人是否准备就绪或告知需稍候/无法出行。",
                    c.driver_name, c.area_name
                ),
                body_en: format!(
                    "Driver {} will pick up {rider_disp_en} at {} at {time}. Please tap to confirm readiness, delay, or inability to join.",
                    c.driver_name, c.area_name
                ),
                link: format!("/rides/{}?action=readiness", c.id),
                meta: NotifMeta {
                    category: "ride",
                    event_type: "ride_reminder".into(),
                    priority: "urgent",
                    entity_type: "ride",
                    entity_id: Some(c.id),
                    tag: format!("ride-{}", c.id),
                },
            });
        }
    }

    // Pass 2: Driver pre-trip reminder (governed by driver's own lead-time preference and grouped by trip_id).
    let driver_candidates: Vec<DriverReminderCandidate> = sqlx::query_as(
        "SELECT r.id, r.driver_id, r.trip_id, r.pickup_at,
                a.name AS area_name, p.name AS passenger_name,
                r.is_proxy, r.rider_name, up.driver_remind_mins
         FROM rides r
         JOIN areas a ON a.id=r.pickup_area_id
         JOIN users p ON p.id=r.passenger_id
         LEFT JOIN user_notification_prefs up ON up.user_id=r.driver_id
         WHERE r.status='confirmed' AND r.driver_id IS NOT NULL
           AND r.driver_reminder_sent=0 AND r.pickup_at <= $1 AND r.pickup_at >= $2
         ORDER BY r.pickup_at ASC, r.id ASC",
    )
    .bind(ts_in(180))
    .bind(ts_in(-10))
    .fetch_all(db)
    .await?;

    let mut grouped: BTreeMap<(i64, i64), Vec<DriverReminderCandidate>> = BTreeMap::new();
    for c in driver_candidates {
        let lead = c.driver_remind_mins.unwrap_or(60).clamp(15, 180);
        if c.pickup_at > ts_in(lead) {
            continue;
        }
        let trip_key = c.trip_id.unwrap_or(-c.id);
        grouped.entry((c.driver_id, trip_key)).or_default().push(c);
    }

    for ((driver_id, _trip_key), group_rides) in grouped {
        let candidate_ids: Vec<i64> = group_rides.iter().map(|r| r.id).collect();
        let claimed_ids: Vec<i64> = sqlx::query_scalar(
            "UPDATE rides SET driver_reminder_sent=1
             WHERE id = ANY($1) AND status='confirmed' AND driver_reminder_sent=0
             RETURNING id",
        )
        .bind(&candidate_ids)
        .fetch_all(db)
        .await?;
        if claimed_ids.is_empty() {
            continue;
        }
        let claimed: Vec<&DriverReminderCandidate> = group_rides
            .iter()
            .filter(|r| claimed_ids.contains(&r.id))
            .collect();
        let first = claimed[0];
        let time = first.pickup_at.get(11..16).unwrap_or(&first.pickup_at);
        let (body_zh, body_en) = if claimed.len() == 1 {
            let (target_zh, target_en) = if first.is_proxy == 1 && !first.rider_name.is_empty() {
                (
                    format!("{} (由 {} 代订)", first.rider_name, first.passenger_name),
                    format!("{} (booked by {})", first.rider_name, first.passenger_name),
                )
            } else {
                (first.passenger_name.clone(), first.passenger_name.clone())
            };
            (
                format!(
                    "{time} 接载 {target_zh}（{}）。出发时请按「开始行程」。",
                    first.area_name
                ),
                format!(
                    "{time} pick up {target_en} ({}). Tap \"Start trip\" when departing.",
                    first.area_name
                ),
            )
        } else {
            let names: Vec<String> = claimed
                .iter()
                .map(|r| {
                    if r.is_proxy == 1 && !r.rider_name.is_empty() {
                        r.rider_name.clone()
                    } else {
                        r.passenger_name.clone()
                    }
                })
                .collect();
            (
                format!(
                    "{time} 拼车行程需接载 {} 位乘客（{}）。出发时请按「开始行程」。",
                    claimed.len(),
                    names.join("、")
                ),
                format!(
                    "{time} carpool trip for {} passengers ({}). Tap \"Start trip\" when departing.",
                    claimed.len(),
                    names.join(", ")
                ),
            )
        };
        batch.push(BatchNotifItem {
            user_id: driver_id,
            title_zh: "接送提醒".into(),
            title_en: "Ride reminder".into(),
            body_zh,
            body_en,
            link: format!("/rides/{}", first.id),
            meta: NotifMeta {
                category: "ride",
                event_type: "ride_reminder".into(),
                priority: "urgent",
                entity_type: "ride",
                entity_id: Some(first.id),
                tag: format!("ride-{}", first.id),
            },
        });
    }

    notify_batch(db, batch).await;
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
    if !due.is_empty() {
        let due_ids: Vec<i64> = due.iter().map(|(id, ..)| *id).collect();
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT event_id, user_id FROM event_signups WHERE event_id = ANY($1) AND status='going'",
        )
        .bind(&due_ids)
        .fetch_all(db)
        .await?;
        let mut by_event: HashMap<i64, Vec<i64>> = HashMap::new();
        for (ev_id, uid) in rows {
            by_event.entry(ev_id).or_default().push(uid);
        }
        for (id, title, at, loc) in due {
            let ids = by_event.remove(&id).unwrap_or_default();
            if ids.is_empty() {
                continue;
            }
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
    }
    Ok(())
}

async fn escalate_unmatched(db: &PgPool) -> anyhow::Result<()> {
    let rows: Vec<(i64, String, String, Option<i64>)> = sqlx::query_as(
        "WITH due AS (
             UPDATE rides SET needs_attention=1,
                    attention_reason='接送前 12 小时仍未有司机 / No driver confirmed 12h before pickup',
                    updated_at=$3
             WHERE status='pending_driver' AND needs_attention=0 AND pickup_at <= $1 AND pickup_at >= $2
             RETURNING id, passenger_id, pickup_at, event_id
         )
         SELECT due.id, p.name, due.pickup_at, e.created_by
         FROM due
         JOIN users p ON p.id = due.passenger_id
         LEFT JOIN events e ON e.id = due.event_id",
    )
    .bind(ts_in(12 * 60))
    .bind(ts_in(0))
    .bind(now())
    .fetch_all(db)
    .await?;
    let ids: Vec<i64> = rows.iter().map(|(id, _, _, _)| *id).collect();
    audit_many(
        db,
        None,
        "ride.escalate",
        "ride",
        &ids,
        "no driver 12h before pickup",
    )
    .await?;
    if !rows.is_empty() {
        let global_staff_ids: Vec<i64> = sqlx::query_scalar(crate::dyn_sql!(
            "SELECT u.id FROM users u WHERE {} AND {}",
            crate::perms::sql_holds("u", crate::perms::Perm(Page::AdminRides, Act::Update)),
            crate::perms::sql_holds("u", crate::perms::Perm(Page::AdminDashboard, Act::View))
        ))
        .fetch_all(db)
        .await
        .unwrap_or_default();
        let all_ride_updaters: std::collections::HashSet<i64> =
            sqlx::query_scalar::<_, i64>(crate::dyn_sql!(
                "SELECT u.id FROM users u WHERE {}",
                crate::perms::sql_holds("u", crate::perms::Perm(Page::AdminRides, Act::Update))
            ))
            .fetch_all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();
        let mut batch = Vec::new();
        for (id, name, at, event_created_by) in rows {
            let body = format!("{name} · {}", at.replace('T', " "));
            let link = format!("/admin/rides/{id}");
            let mut recipients: Vec<i64> = global_staff_ids.clone();
            if let Some(cid) = event_created_by {
                if all_ride_updaters.contains(&cid) && !recipients.contains(&cid) {
                    recipients.push(cid);
                }
            }
            for sid in recipients {
                batch.push(BatchNotifItem {
                    user_id: sid,
                    title_zh: "没人接单".into(),
                    title_en: "No driver".into(),
                    body_zh: body.clone(),
                    body_en: body.clone(),
                    link: link.clone(),
                    meta: NotifMeta {
                        category: "ride",
                        event_type: "escalate_unmatched".into(),
                        priority: "urgent",
                        entity_type: "ride",
                        entity_id: Some(id),
                        tag: format!("ride-{id}"),
                    },
                });
            }
        }
        if !batch.is_empty() {
            notify_batch(db, batch).await;
        }
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
    // Read notifications go after NOTIFICATION_RETENTION_DAYS from when they were read;
    // unread ones after a year (or that long, if it is longer), so the table cannot grow without end.
    let read_days = cfg.notification_retention_days;
    let unread_days = read_days.max(365);
    sqlx::query(
        "DELETE FROM notifications
         WHERE (read_at IS NOT NULL AND read_at < $1) OR created_at < $2",
    )
    .bind(ts_in(-read_days * 24 * 60))
    .bind(ts_in(-unread_days * 24 * 60))
    .execute(db)
    .await?;
    sqlx::query("DELETE FROM notification_deliveries WHERE created_at < $1")
        .bind(ts_in(-30 * 24 * 60))
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM telegram_link_tokens WHERE expires_at < $1")
        .bind(now())
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM oauth_states WHERE created_at < $1")
        .bind(t(-30))
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM password_reset_tokens WHERE expires_at < $1")
        .bind(now())
        .execute(db)
        .await?;
    purge_orphan_files(db, &cfg.upload_dir).await?;
    Ok(())
}

/// Remove uploaded files that have been unreferenced for more than 24 hours, plus any
/// orphaned UUID-named disk blobs missing from `files` (e.g. failed inserts or interrupted writes).
pub async fn purge_orphan_files(db: &PgPool, upload_dir: &std::path::Path) -> anyhow::Result<()> {
    let cutoff = ts_in(-24 * 60);
    let mut tx = db.begin().await?;
    let candidates: Vec<(i64, String)> = sqlx::query_as(
        "SELECT f.id, f.stored_name FROM files f
         WHERE f.created_at < $1
           AND NOT EXISTS (SELECT 1 FROM users u WHERE u.photo_file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM area_photos ap WHERE ap.file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM events e WHERE e.photo_file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.receipt_file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM settlement_shares ss WHERE ss.proof_file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM settlements s WHERE s.payee_qr_file_id = f.id)
           AND NOT EXISTS (
               SELECT 1 FROM settlement_revisions sr
               WHERE sr.snapshot @> jsonb_build_object('payee_qr_file_id', f.id)
                  OR sr.snapshot->'items' @> jsonb_build_array(jsonb_build_object('receipt_file_id', f.id))
                  OR sr.snapshot->'shares' @> jsonb_build_array(jsonb_build_object('proof_file_id', f.id))
           )
           AND NOT EXISTS (SELECT 1 FROM help_replies hr WHERE hr.file_id = f.id)
           AND NOT EXISTS (SELECT 1 FROM reimbursements r WHERE r.receipt_file_id = f.id)
           AND NOT EXISTS (
               SELECT 1 FROM guide_workflows gw
               WHERE gw.steps @> jsonb_build_array(jsonb_build_object('image_file_id', f.id))
           )
         ORDER BY f.id
         LIMIT 500
         FOR UPDATE OF f SKIP LOCKED",
    )
    .bind(&cutoff)
    .fetch_all(&mut *tx)
    .await?;
    let candidate_ids: Vec<i64> = candidates.iter().map(|(id, _)| *id).collect();
    let stale: Vec<String> = if candidate_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_scalar(
            "DELETE FROM files f
             WHERE f.id = ANY($1)
               AND f.created_at < $2
               AND NOT EXISTS (SELECT 1 FROM users u WHERE u.photo_file_id = f.id)
               AND NOT EXISTS (SELECT 1 FROM area_photos ap WHERE ap.file_id = f.id)
               AND NOT EXISTS (SELECT 1 FROM events e WHERE e.photo_file_id = f.id)
               AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.receipt_file_id = f.id)
               AND NOT EXISTS (SELECT 1 FROM settlement_shares ss WHERE ss.proof_file_id = f.id)
               AND NOT EXISTS (SELECT 1 FROM settlements s WHERE s.payee_qr_file_id = f.id)
               AND NOT EXISTS (
                   SELECT 1 FROM settlement_revisions sr
                   WHERE sr.snapshot @> jsonb_build_object('payee_qr_file_id', f.id)
                      OR sr.snapshot->'items' @> jsonb_build_array(jsonb_build_object('receipt_file_id', f.id))
                      OR sr.snapshot->'shares' @> jsonb_build_array(jsonb_build_object('proof_file_id', f.id))
               )
               AND NOT EXISTS (SELECT 1 FROM help_replies hr WHERE hr.file_id = f.id)
               AND NOT EXISTS (SELECT 1 FROM reimbursements r WHERE r.receipt_file_id = f.id)
               AND NOT EXISTS (
                   SELECT 1 FROM guide_workflows gw
                   WHERE gw.steps @> jsonb_build_array(jsonb_build_object('image_file_id', f.id))
               )
             RETURNING f.stored_name",
        )
        .bind(&candidate_ids)
        .bind(&cutoff)
        .fetch_all(&mut *tx)
        .await?
    };
    tx.commit().await?;

    for stored in stale {
        if let Ok(path) = crate::routes::files::safe_upload_path(upload_dir, &stored) {
            let _ = tokio::fs::remove_file(path).await;
        }
    }

    let Ok(mut entries) = tokio::fs::read_dir(upload_dir).await else {
        return Ok(());
    };
    let mut disk_candidates: Vec<(String, std::path::PathBuf)> = Vec::new();
    let age_cutoff = std::time::SystemTime::now() - Duration::from_secs(24 * 3600);
    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(meta) = tokio::fs::symlink_metadata(entry.path()).await else {
            continue;
        };
        if !meta.is_file() || meta.file_type().is_symlink() {
            continue;
        }
        let is_old = meta.modified().is_ok_and(|m| m < age_cutoff);
        if !is_old {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if let Some((stem, ext)) = name.split_once('.') {
            if uuid::Uuid::parse_str(stem).is_ok()
                && !ext.is_empty()
                && ext.bytes().all(|b| b.is_ascii_alphanumeric())
            {
                if let Ok(safe_path) = crate::routes::files::safe_upload_path(upload_dir, &name) {
                    disk_candidates.push((name, safe_path));
                }
            }
        }
    }
    if !disk_candidates.is_empty() {
        let names: Vec<String> = disk_candidates.iter().map(|(n, _)| n.clone()).collect();
        let live: Vec<String> =
            sqlx::query_scalar("SELECT stored_name FROM files WHERE stored_name = ANY($1)")
                .bind(&names)
                .fetch_all(db)
                .await?;
        let live_set: std::collections::HashSet<String> = live.into_iter().collect();
        for (name, path) in disk_candidates {
            if !live_set.contains(&name) {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
    }
    Ok(())
}
