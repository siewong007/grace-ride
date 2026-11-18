//! Notifications, announcements, help center, Grab reimbursements, monthly reports & badges, profile.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, conflict, forbidden, not_found, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{
    audit, check_name, check_phone, fmt_rm, notify, notify_many, notify_perm, now, parse_dt,
    random_token, require, require_one, t, Lang,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", axum::routing::put(update_profile))
        .route(
            "/me/calendar-feed",
            get(get_calendar_feed).delete(revoke_calendar_feed),
        )
        .route("/me/calendar-feed/rotate", post(rotate_calendar_feed))
        .route("/calendar/{token}", get(serve_calendar_ics))
        .route("/push/config", get(push_config))
        .route(
            "/push/subscribe",
            post(push_subscribe).delete(push_unsubscribe),
        )
        .route("/push/test", post(push_test))
        .route("/notifications", get(list_notifications))
        .route("/notifications/read-all", post(read_all))
        .route("/notifications/{id}/read", post(read_one))
        .route(
            "/announcements",
            get(list_announcements).post(create_announcement),
        )
        .route(
            "/announcements/{id}",
            delete(delete_announcement).put(update_announcement),
        )
        .route("/help", get(list_tickets).post(create_ticket))
        .route("/help/{id}", get(get_ticket))
        .route("/help/{id}/replies", post(reply_ticket))
        .route("/help/{id}/close", post(close_ticket))
        .route("/help/{id}/reopen", post(reopen_ticket))
        .route("/help/{id}/claim", post(claim_ticket))
        .route("/help/counts", get(ticket_counts))
        .route("/help/members", get(help_members))
        .route("/help/guides/reorder", post(reorder_guides))
        .route("/help/guides/{id}/feedback", post(guide_feedback))
        .route("/help/guides", get(list_guides).post(create_guide))
        .route(
            "/help/guides/{key}",
            get(get_guide).put(update_guide).delete(delete_guide),
        )
        .route(
            "/reimbursements/batches",
            get(list_payout_batches).post(create_payout_batch),
        )
        .route("/reimbursements/batches/{id}", delete(cancel_payout_batch))
        .route(
            "/reimbursements/batches/{id}/csv",
            get(export_payout_batch_csv),
        )
        .route("/reimbursements/batches/{id}/pay", post(pay_payout_batch))
        .route(
            "/reimbursements",
            get(list_reimbursements).post(create_reimbursement),
        )
        .route(
            "/reimbursements/{id}",
            get(get_reimbursement).put(resubmit_reimbursement),
        )
        .route("/reimbursements/{id}/review", post(review_reimbursement))
        .route("/reimbursements/{id}/pay", post(pay_reimbursement))
        .route("/reports/monthly", get(monthly_report))
        .route("/badges", get(my_badges))
}

// ---------------- Profile ----------------

#[derive(Deserialize)]
struct ProfileIn {
    /// Roughly every field is optional; callers may only send what changes,
    /// e.g. { "lang": "en" } from the language switcher.
    name: Option<String>,
    phone: Option<String>,
    photo_file_id: Option<i64>,
    badges_opt_out: Option<bool>,
    lang: Option<String>,
    readiness_prompt_mins: Option<i64>,
    auto_lock_mins: Option<i64>,
    telegram_chat_id: Option<String>,
    whatsapp_opt_in: Option<bool>,
    push_opt_in: Option<bool>,
    caregiver_user_id: Option<i64>,
    clear_caregiver: Option<bool>,
}

async fn update_profile(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ProfileIn>,
) -> AppResult<Json<Value>> {
    if let Some(ref name) = req.name {
        check_name(u.lang, name)?;
    }
    if let Some(ref phone) = req.phone {
        check_phone(u.lang, phone)?;
    }
    if let Some(ref lang) = req.lang {
        if lang != "zh" && lang != "en" {
            return Err(bad(u.lang, ErrorKey::LanguageInvalid));
        }
    }
    if let Some(m) = req.readiness_prompt_mins {
        if !(0..=180).contains(&m) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if let Some(m) = req.auto_lock_mins {
        if !(-1..=1440).contains(&m) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    if let Some(f) = req.photo_file_id {
        let (owner, mime): (i64, String) =
            sqlx::query_as("SELECT owner_id, mime FROM files WHERE id=$1")
                .bind(f)
                .fetch_optional(&st.db)
                .await?
                .ok_or(not_found(u.lang))?;
        if owner != u.id {
            return Err(forbidden(u.lang, ErrorKey::InvalidPhoto));
        }
        if !mime.starts_with("image/") {
            return Err(bad(u.lang, ErrorKey::InvalidPhoto));
        }
    }
    if let Some(cg) = req.caregiver_user_id {
        if cg == u.id {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        let ok: Option<(i64,)> = sqlx::query_as(
            "SELECT cg.id FROM users cg
             WHERE cg.id=$1 AND cg.status='active'
               AND EXISTS (
                   SELECT 1 FROM group_members g1
                   JOIN group_members g2 ON g2.group_id = g1.group_id
                   WHERE g1.user_id = $1 AND g2.user_id = $2
               )",
        )
        .bind(cg)
        .bind(u.id)
        .fetch_optional(&st.db)
        .await?;
        if ok.is_none() {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }
    let tg = req
        .telegram_chat_id
        .as_deref()
        .map(|s| s.trim().chars().take(64).collect::<String>());
    sqlx::query(
        "UPDATE users SET
            name=COALESCE($1, name),
            phone=COALESCE($2, phone),
            photo_file_id=COALESCE($3, photo_file_id),
            badges_opt_out=COALESCE($4, badges_opt_out),
            lang=COALESCE($5, lang),
            readiness_prompt_mins=COALESCE($6, readiness_prompt_mins),
            auto_lock_mins=COALESCE($7, auto_lock_mins),
            telegram_chat_id=COALESCE($8, telegram_chat_id),
            whatsapp_opt_in=COALESCE($9, whatsapp_opt_in),
            push_opt_in=COALESCE($10, push_opt_in),
            caregiver_user_id=CASE WHEN $11 THEN NULL ELSE COALESCE($12, caregiver_user_id) END
         WHERE id=$13",
    )
    .bind(req.name.as_deref().map(str::trim))
    .bind(req.phone.as_deref().map(str::trim))
    .bind(req.photo_file_id)
    .bind(req.badges_opt_out.map(|b| b as i64))
    .bind(req.lang.as_deref())
    .bind(req.readiness_prompt_mins)
    .bind(req.auto_lock_mins)
    .bind(tg.as_deref())
    .bind(req.whatsapp_opt_in.map(|b| b as i64))
    .bind(req.push_opt_in.map(|b| b as i64))
    .bind(req.clear_caregiver.unwrap_or(false))
    .bind(req.caregiver_user_id)
    .bind(u.id)
    .execute(&st.db)
    .await?;
    // Which fields were sent, not their values: the log is read by staff.
    let changed: Vec<&str> = [
        ("name", req.name.is_some()),
        ("phone", req.phone.is_some()),
        ("photo", req.photo_file_id.is_some()),
        ("badges_opt_out", req.badges_opt_out.is_some()),
        ("lang", req.lang.is_some()),
        ("readiness_prompt", req.readiness_prompt_mins.is_some()),
        ("auto_lock", req.auto_lock_mins.is_some()),
        (
            "alerts",
            req.telegram_chat_id.is_some()
                || req.whatsapp_opt_in.is_some()
                || req.push_opt_in.is_some(),
        ),
        (
            "caregiver",
            req.caregiver_user_id.is_some() || req.clear_caregiver.unwrap_or(false),
        ),
    ]
    .into_iter()
    .filter_map(|(k, sent)| sent.then_some(k))
    .collect();
    audit(
        &st.db,
        Some(u.id),
        "profile.update",
        "user",
        Some(u.id),
        changed.join(", "),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Live Calendar Subscription (.ics) ----------------

async fn get_calendar_feed(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let (has_hash,): (bool,) =
        sqlx::query_as("SELECT calendar_token_hash IS NOT NULL FROM users WHERE id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    Ok(Json(json!({ "enabled": has_hash })))
}

async fn rotate_calendar_feed(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let token = random_token(48);
    let token_hash = hash_token(&token);
    sqlx::query("UPDATE users SET calendar_token_hash=$1 WHERE id=$2")
        .bind(&token_hash)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "calendar.rotate",
        "user",
        Some(u.id),
        "",
    )
    .await;
    Ok(Json(json!({
        "token": token,
        "ics_path": format!("/api/calendar/{token}.ics"),
    })))
}

async fn revoke_calendar_feed(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE users SET calendar_token_hash=NULL WHERE id=$1")
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "calendar.revoke",
        "user",
        Some(u.id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

fn escape_ics_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\r', "")
        .replace('\n', "\\n")
}

fn fmt_ics_dt(raw: &str) -> String {
    let Some(dt) = parse_dt(raw) else {
        return raw.replace(['-', ':'], "").chars().take(15).collect();
    };
    use chrono::TimeZone;
    crate::util::zone()
        .from_local_datetime(&dt)
        .earliest()
        .map(|zdt| {
            zdt.with_timezone(&chrono::Utc)
                .format("%Y%m%dT%H%M%SZ")
                .to_string()
        })
        .unwrap_or_else(|| dt.format("%Y%m%dT%H%M%S").to_string())
}

fn shift_ics_dt(raw: &str, mins: i64) -> String {
    let Some(dt) = parse_dt(raw) else {
        return fmt_ics_dt(raw);
    };
    let shifted = dt + chrono::Duration::minutes(mins);
    use chrono::TimeZone;
    crate::util::zone()
        .from_local_datetime(&shifted)
        .earliest()
        .map(|zdt| {
            zdt.with_timezone(&chrono::Utc)
                .format("%Y%m%dT%H%M%SZ")
                .to_string()
        })
        .unwrap_or_else(|| shifted.format("%Y%m%dT%H%M%S").to_string())
}

async fn serve_calendar_ics(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(raw_token): Path<String>,
) -> AppResult<Response> {
    let lang = Lang::from_accept_language(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    let token = raw_token.strip_suffix(".ics").unwrap_or(&raw_token);
    if token.len() < 16 {
        return Err(not_found(lang));
    }
    let user: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, name FROM users WHERE calendar_token_hash=$1 AND status='active'",
    )
    .bind(hash_token(token))
    .fetch_optional(&st.db)
    .await?;
    let Some((uid, _uname)) = user else {
        return Err(not_found(lang));
    };

    let cutoff = crate::util::in_days(-30);
    let rides: Vec<(i64, i64, Option<i64>, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT r.id, r.passenger_id, r.driver_id, r.pickup_at, r.destination, r.pickup_address,
                r.status, COALESCE(r.leg, 'outbound'), COALESCE(a.name, '')
         FROM rides r
         LEFT JOIN areas a ON a.id = r.pickup_area_id
         WHERE (r.passenger_id=$1 OR r.driver_id=$1)
           AND r.status IN ('confirmed','in_progress','completed','pending_driver_confirm','pending_approval')
           AND r.pickup_at >= $2
         ORDER BY r.pickup_at",
    )
    .bind(uid)
    .bind(&cutoff)
    .fetch_all(&st.db)
    .await?;

    let events: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT e.id, e.title, e.starts_at, e.location, e.description
         FROM event_signups es JOIN events e ON e.id = es.event_id
         WHERE es.user_id=$1 AND es.status='going' AND e.status IN ('published','finished') AND e.starts_at >= $2
         ORDER BY e.starts_at",
    )
    .bind(uid)
    .bind(&cutoff)
    .fetch_all(&st.db)
    .await?;

    let stamp = fmt_ics_dt(&now());
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//Grace Ride PJ State CMC//EN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "METHOD:PUBLISH".to_string(),
        "X-WR-CALNAME:Grace Ride".to_string(),
    ];

    for (rid, pass_id, _drv_id, pickup_at, dest, addr, status, leg, area_name) in rides {
        let role = if pass_id == uid {
            t(lang, "乘车接送", "Grace Ride")
        } else {
            t(lang, "志愿接送驾驶", "Volunteer Drive")
        };
        let show_address = pass_id == uid || matches!(status.as_str(), "confirmed" | "in_progress");
        let safe_dest = if !show_address && leg == "return" {
            area_name.as_str()
        } else {
            dest.as_str()
        };
        let safe_loc = if !show_address && leg != "return" {
            area_name.as_str()
        } else {
            addr.as_str()
        };
        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:gr-ride-{rid}@graceride.cmc"));
        lines.push(format!("DTSTAMP:{stamp}"));
        lines.push(format!("DTSTART:{}", fmt_ics_dt(&pickup_at)));
        lines.push(format!("DTEND:{}", shift_ics_dt(&pickup_at, 60)));
        lines.push(format!(
            "SUMMARY:{}",
            escape_ics_text(&format!("{role} -> {safe_dest}"))
        ));
        if !safe_loc.is_empty() {
            lines.push(format!("LOCATION:{}", escape_ics_text(safe_loc)));
        }
        lines.push(format!(
            "DESCRIPTION:{}",
            escape_ics_text(&format!("Ride #{rid} ({status})"))
        ));
        lines.push("STATUS:CONFIRMED".to_string());
        lines.push("END:VEVENT".to_string());
    }

    for (eid, title, starts_at, location, desc) in events {
        if !crate::audience::member_sees_event(&st.db, uid, eid).await? {
            continue;
        }
        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:gr-event-{eid}@graceride.cmc"));
        lines.push(format!("DTSTAMP:{stamp}"));
        lines.push(format!("DTSTART:{}", fmt_ics_dt(&starts_at)));
        lines.push(format!("DTEND:{}", shift_ics_dt(&starts_at, 120)));
        lines.push(format!("SUMMARY:{}", escape_ics_text(&title)));
        if !location.is_empty() {
            lines.push(format!("LOCATION:{}", escape_ics_text(&location)));
        }
        if !desc.is_empty() {
            lines.push(format!("DESCRIPTION:{}", escape_ics_text(&desc)));
        }
        lines.push("STATUS:CONFIRMED".to_string());
        lines.push("END:VEVENT".to_string());
    }

    lines.push("END:VCALENDAR".to_string());
    let body = lines.join("\r\n") + "\r\n";

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(body))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

// ---------------- Web Push Subscriptions ----------------

async fn push_config(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let pub_key = crate::web_push::ensure_vapid_keys(&st.db)
        .await
        .map_err(AppError::Other)?;
    let configured = crate::web_push::load_vapid_signer(&st.db).await.is_some();
    let sub_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions WHERE user_id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    Ok(Json(json!({
        "enabled": true,
        "configured": configured,
        "vapid_public_key": pub_key,
        "subscription_count": sub_count,
    })))
}

#[derive(Deserialize)]
struct PushSubIn {
    endpoint: String,
    #[serde(default)]
    p256dh: String,
    #[serde(default)]
    auth: String,
}

async fn push_subscribe(
    State(st): State<AppState>,
    u: CurrentUser,
    headers: HeaderMap,
    Json(req): Json<PushSubIn>,
) -> AppResult<Json<Value>> {
    let endpoint = req.endpoint.trim();
    if endpoint.is_empty()
        || endpoint.len() > 1000
        || !crate::util::validate_push_endpoint(endpoint, st.cfg.dev_mode).await
    {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let p256dh = req.p256dh.trim();
    let auth = req.auth.trim();
    if p256dh.len() > 256 || auth.len() > 256 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !st.cfg.dev_mode && !crate::web_push::validate_subscription_keys(p256dh, auth) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .chars()
        .take(200)
        .collect::<String>();
    sqlx::query(
        "INSERT INTO push_subscriptions (user_id, endpoint, p256dh, auth, user_agent, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)
         ON CONFLICT (endpoint) DO UPDATE
         SET user_id=excluded.user_id, p256dh=excluded.p256dh, auth=excluded.auth,
             user_agent=excluded.user_agent, created_at=excluded.created_at",
    )
    .bind(u.id)
    .bind(endpoint)
    .bind(p256dh)
    .bind(auth)
    .bind(ua)
    .bind(now())
    .execute(&st.db)
    .await?;
    sqlx::query(
        "DELETE FROM push_subscriptions
         WHERE user_id=$1
           AND id NOT IN (
               SELECT id FROM push_subscriptions
               WHERE user_id=$1
               ORDER BY created_at DESC, id DESC
               LIMIT 10
           )",
    )
    .bind(u.id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct PushUnsubIn {
    endpoint: String,
}

async fn push_unsubscribe(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<PushUnsubIn>,
) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1 AND endpoint=$2")
        .bind(u.id)
        .bind(req.endpoint.trim())
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn push_test(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    let signer = crate::web_push::load_vapid_signer(&st.db)
        .await
        .ok_or_else(|| bad(u.lang, ErrorKey::InvalidAction))?;
    let subs: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, endpoint, p256dh, auth FROM push_subscriptions WHERE user_id=$1",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    if subs.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let total = subs.len();
    let title = t(u.lang, "测试推送通知", "Test push notification");
    let body = t(
        u.lang,
        "此设备已成功连接恩典同行通知。",
        "This device is connected to Grace Ride alerts.",
    );
    let subject = crate::util::vapid_subject();
    let client = crate::util::alert_client();
    let mut delivered = 0usize;
    for (sub_id, endpoint, p256dh, auth) in subs {
        if !crate::util::validate_push_endpoint(&endpoint, st.cfg.dev_mode).await {
            continue;
        }
        match crate::web_push::send_push_message(
            client, &signer, &subject, &endpoint, &p256dh, &auth, title, body, "/me",
        )
        .await
        {
            crate::web_push::PushOutcome::Delivered => delivered += 1,
            crate::web_push::PushOutcome::Gone => {
                let _ = sqlx::query("DELETE FROM push_subscriptions WHERE id=$1")
                    .bind(sub_id)
                    .execute(&st.db)
                    .await;
            }
            crate::web_push::PushOutcome::Skipped | crate::web_push::PushOutcome::Failed => {}
        }
    }
    Ok(Json(
        json!({ "ok": true, "delivered": delivered, "total": total }),
    ))
}

// ---------------- Notifications ----------------

#[derive(Deserialize)]
struct NotifQuery {
    before: Option<i64>,
    limit: Option<i64>,
}

async fn list_notifications(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<NotifQuery>,
) -> AppResult<Json<Value>> {
    let limit = q.limit.unwrap_or(30).clamp(1, 100);
    if q.before.is_some() || q.limit.is_some() {
        let sql = if q.before.is_some() {
            "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 AND id < $2 ORDER BY id DESC LIMIT $3"
        } else {
            "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 ORDER BY id DESC LIMIT $2"
        };
        let mut rows: Vec<(i64, String, String, String, Option<String>, String)> =
            if let Some(b) = q.before {
                sqlx::query_as(sql)
                    .bind(u.id)
                    .bind(b)
                    .bind(limit + 1)
                    .fetch_all(&st.db)
                    .await?
            } else {
                sqlx::query_as(sql)
                    .bind(u.id)
                    .bind(limit + 1)
                    .fetch_all(&st.db)
                    .await?
            };
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_before = if more { rows.last().map(|r| r.0) } else { None };
        return Ok(Json(json!({
            "items": rows.into_iter().map(|(id, t, b, l, r, c)| json!({"id": id, "title": t, "body": b, "link": l, "read": r.is_some(), "created_at": c})).collect::<Vec<_>>(),
            "next_before": next_before
        })));
    }

    let rows: Vec<(i64, String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, title, body, link, read_at, created_at FROM notifications WHERE user_id=$1 ORDER BY id DESC LIMIT 100",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, b, l, r, c)| json!({"id": id, "title": t, "body": b, "link": l, "read": r.is_some(), "created_at": c})).collect::<Vec<_>>())))
}

async fn read_all(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE notifications SET read_at=$1 WHERE user_id=$2 AND read_at IS NULL")
        .bind(now())
        .bind(u.id)
        .execute(&st.db)
        .await?;
    Ok(Json(json!({"ok": true})))
}

async fn read_one(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        "UPDATE notifications SET read_at=$1 WHERE id=$2 AND user_id=$3 AND read_at IS NULL",
    )
    .bind(now())
    .bind(id)
    .bind(u.id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Announcements ----------------

async fn list_announcements(State(st): State<AppState>, _u: CurrentUser) -> AppResult<Json<Value>> {
    let rows: Vec<(i64, String, String, i64, String, String)> = sqlx::query_as(
        "SELECT a.id, a.title, a.body, a.pinned, u.name, a.created_at FROM announcements a JOIN users u ON u.id=a.created_by ORDER BY a.pinned DESC, a.id DESC LIMIT 30",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows.into_iter().map(|(id, t, b, p, by, c)| json!({"id": id, "title": t, "body": b, "pinned": p == 1, "by": by, "created_at": c})).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct AnnIn {
    title: String,
    body: String,
    pinned: Option<bool>,
    notify_all: Option<bool>,
}

#[derive(Deserialize)]
struct AnnEdit {
    title: String,
    body: String,
    pinned: bool,
}

async fn create_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<AnnIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Create)?;
    if req.title.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::TitleAndContentRequired));
    }
    let (id,): (i64,) = sqlx::query_as("INSERT INTO announcements (title, body, pinned, created_by, created_at) VALUES ($1,$2,$3,$4,$5) RETURNING id")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(req.pinned.unwrap_or(false) as i64)
        .bind(u.id)
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    if req.notify_all.unwrap_or(false) {
        let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM users WHERE status='active'")
            .fetch_all(&st.db)
            .await?;
        let title = req.title.trim();
        notify_many(&st.db, &ids, ("新公告", "New announcement"), (title, title), "/").await;
    }
    audit(
        &st.db,
        Some(u.id),
        "announcement.create",
        "announcement",
        Some(id),
        req.title.trim(),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

/// Fix a typo, or pin and unpin, without publishing again: nobody is notified a second time.
/// Editing is part of publishing, so it needs the same permission as creating.
async fn update_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<AnnEdit>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Create)?;
    if req.title.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::TitleAndContentRequired));
    }
    let changed = sqlx::query("UPDATE announcements SET title=$1, body=$2, pinned=$3 WHERE id=$4")
        .bind(req.title.trim())
        .bind(req.body.trim())
        .bind(req.pinned as i64)
        .bind(id)
        .execute(&st.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "announcement.update",
        "announcement",
        Some(id),
        req.title.trim(),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn delete_announcement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAnnouncements, Act::Delete)?;
    let gone: Option<(String,)> =
        sqlx::query_as("DELETE FROM announcements WHERE id=$1 RETURNING title")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    if let Some((title,)) = gone {
        audit(
            &st.db,
            Some(u.id),
            "announcement.delete",
            "announcement",
            Some(id),
            title,
        )
        .await;
    }
    Ok(Json(json!({"ok": true})))
}

// ---------------- Help center ----------------

#[derive(Deserialize)]
struct HelpQuery {
    all: Option<bool>,
    status: Option<String>,
    q: Option<String>,
    /// "me": only requests this staff member has claimed.
    assigned: Option<String>,
}

/// A photo on a help message must be an image the sender uploaded themselves,
/// so nobody can surface someone else's file by naming its id.
async fn check_attachment(st: &AppState, u: &CurrentUser, file_id: Option<i64>) -> AppResult<()> {
    let Some(fid) = file_id else { return Ok(()) };
    let ok: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM files WHERE id=$1 AND owner_id=$2 AND mime LIKE 'image/%'")
            .bind(fid)
            .bind(u.id)
            .fetch_optional(&st.db)
            .await?;
    ok.map(|_| ())
        .ok_or(bad(u.lang, ErrorKey::HelpAttachmentInvalid))
}

/// Escape LIKE wildcards so a search term is matched literally.
fn like_pattern(q: &str) -> String {
    let esc: String = q
        .trim()
        .chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect();
    format!("%{esc}%")
}

async fn list_tickets(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<HelpQuery>,
) -> AppResult<Json<Value>> {
    let all = q.all.unwrap_or(false) && u.can(Page::AdminHelp, Act::View);
    if !all {
        require(&u, Page::Help, Act::View)?;
    }
    let status = q
        .status
        .as_deref()
        .filter(|s| matches!(*s, "open" | "answered" | "closed"));
    let search =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(like_pattern);
    let mine_only = all && q.assigned.as_deref() == Some("me");
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        i64,
        String,
        String,
        Option<i64>,
        i64,
        String,
        String,
        String,
        Option<String>,
        Option<bool>,
        Option<String>,
        Option<i64>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT t.id, t.subject, t.status, t.ride_id, t.user_id, u.name, t.created_at, t.updated_at,
                lr.body, lr.staff, lr.created_at, t.assignee_id, a.name
         FROM help_tickets t JOIN users u ON u.id=t.user_id
         LEFT JOIN users a ON a.id=t.assignee_id
         LEFT JOIN LATERAL (
             SELECT LEFT(r.body, 160) AS body, (r.user_id <> t.user_id) AS staff, r.created_at
             FROM help_replies r WHERE r.ticket_id=t.id ORDER BY r.id DESC LIMIT 1
         ) lr ON TRUE
         WHERE ($1 OR t.user_id=$2)
           AND ($3::text IS NULL OR t.status=$3)
           AND ($4::text IS NULL OR t.subject ILIKE $4 ESCAPE '\\' OR u.name ILIKE $4 ESCAPE '\\')
           AND (NOT $5 OR t.assignee_id=$2)
         ORDER BY CASE t.status WHEN 'open' THEN 0 WHEN 'answered' THEN 1 ELSE 2 END, t.updated_at DESC LIMIT 200",
    )
    .bind(all)
    .bind(u.id)
    .bind(status)
    .bind(search)
    .bind(mine_only)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(
            |(id, s, status, r, uid, name, c, upd, body, staff, at, aid, aname)| json!({
                "id": id, "subject": s, "status": status, "ride_id": r, "user_id": uid,
                "user_name": name, "created_at": c, "updated_at": upd,
                "last_body": body, "last_staff": staff, "last_at": at,
                "assignee_id": aid, "assignee_name": aname,
            })
        )
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct CountsQuery {
    all: Option<bool>,
    assigned: Option<String>,
}

async fn ticket_counts(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<CountsQuery>,
) -> AppResult<Json<Value>> {
    let all = q.all.unwrap_or(false) && u.can(Page::AdminHelp, Act::View);
    if !all {
        require(&u, Page::Help, Act::View)?;
    }
    let mine_only = all && q.assigned.as_deref() == Some("me");
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*)::bigint FROM help_tickets
         WHERE ($1 OR user_id=$2) AND (NOT $3 OR assignee_id=$2) GROUP BY status",
    )
    .bind(all)
    .bind(u.id)
    .bind(mine_only)
    .fetch_all(&st.db)
    .await?;
    let get = |k: &str| rows.iter().find(|(s, _)| s == k).map_or(0, |(_, n)| *n);
    Ok(Json(
        json!({"open": get("open"), "answered": get("answered"), "closed": get("closed")}),
    ))
}

#[derive(Deserialize)]
struct MemberQuery {
    q: Option<String>,
}

/// Member picker for staff raising a request on someone's behalf: id and name only.
async fn help_members(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<MemberQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let search = like_pattern(q.q.as_deref().unwrap_or(""));
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, name FROM users WHERE name ILIKE $1 ESCAPE '\\' ORDER BY name LIMIT 20",
    )
    .bind(search)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|(id, name)| json!({"id": id, "name": name}))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct TicketIn {
    subject: String,
    body: String,
    ride_id: Option<i64>,
    /// Staff only: raise the request on behalf of this member.
    for_user_id: Option<i64>,
    /// Optional photo (an image this user uploaded).
    file_id: Option<i64>,
}

async fn create_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<TicketIn>,
) -> AppResult<Json<Value>> {
    let on_behalf = req.for_user_id.filter(|id| *id != u.id);
    if on_behalf.is_some() {
        require(&u, Page::AdminHelp, Act::Update)?;
    } else {
        require(&u, Page::Help, Act::Create)?;
    }
    if req.subject.trim().is_empty() || req.body.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::QuestionAndDetailsRequired));
    }
    let owner_id = on_behalf.unwrap_or(u.id);
    check_attachment(&st, &u, req.file_id).await?;
    if on_behalf.is_some() {
        let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE id=$1")
            .bind(owner_id)
            .fetch_optional(&st.db)
            .await?;
        exists.ok_or(not_found(u.lang))?;
    }
    if let Some(r) = req.ride_id {
        let (p, d): (i64, Option<i64>) =
            sqlx::query_as("SELECT passenger_id, driver_id FROM rides WHERE id=$1")
                .bind(r)
                .fetch_optional(&st.db)
                .await?
                .ok_or(not_found(u.lang))?;
        if p != owner_id && d != Some(owner_id) {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
    }
    let status = if on_behalf.is_some() {
        "answered"
    } else {
        "open"
    };
    let (id,): (i64,) = sqlx::query_as("INSERT INTO help_tickets (user_id, ride_id, subject, status, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id")
        .bind(owner_id)
        .bind(req.ride_id)
        .bind(req.subject.trim())
        .bind(status)
        .bind(now())
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    sqlx::query(
        "INSERT INTO help_replies (ticket_id, user_id, body, file_id, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.body.trim())
    .bind(req.file_id)
    .bind(now())
    .execute(&st.db)
    .await?;
    if on_behalf.is_some() {
        notify(
            &st.db,
            owner_id,
            ("帮助中心已回复", "Help center has replied"),
            (req.subject.trim(), req.subject.trim()),
            &format!("/help/{id}"),
        )
        .await;
    } else {
        notify_perm(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            ("帮助中心：新的求助", "Help center: new request"),
            (
                &format!("{} · {}", u.name, req.subject.trim()),
                &format!("{} · {}", u.name, req.subject.trim()),
            ),
            &format!("/help/{id}"),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "help.create",
        "help_ticket",
        Some(id),
        &if on_behalf.is_some() {
            format!("{} (for user {owner_id})", req.subject.trim())
        } else {
            req.subject.trim().to_string()
        },
    )
    .await;
    Ok(Json(json!({"id": id})))
}

/// Load a help request for its owner, or for 帮助台 staff who may do `staff_act` on it.
async fn ticket_owner(
    st: &AppState,
    u: &CurrentUser,
    id: i64,
    staff_act: Act,
) -> AppResult<(i64, String, String, Option<i64>)> {
    let row: (i64, String, String, Option<i64>) =
        sqlx::query_as("SELECT user_id, subject, status, ride_id FROM help_tickets WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
    if row.0 != u.id && !u.can(Page::AdminHelp, staff_act) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    Ok(row)
}

async fn get_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, ride) = ticket_owner(&st, &u, id, Act::View).await?;
    #[allow(clippy::type_complexity)]
    let replies: Vec<(i64, i64, String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT r.id, r.user_id, u.name, r.body, r.created_at, r.file_id FROM help_replies r JOIN users u ON u.id=r.user_id WHERE r.ticket_id=$1 ORDER BY r.id",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    let (assignee_id, assignee_name): (Option<i64>, Option<String>) = sqlx::query_as(
        "SELECT t.assignee_id, a.name FROM help_tickets t LEFT JOIN users a ON a.id=t.assignee_id WHERE t.id=$1",
    )
    .bind(id)
    .fetch_one(&st.db)
    .await?;
    let member = if u.can(Page::AdminHelp, Act::View) {
        let m: Option<(String, String)> =
            sqlx::query_as("SELECT name, phone FROM users WHERE id=$1")
                .bind(owner)
                .fetch_optional(&st.db)
                .await?;
        let others: Vec<(i64,)> = sqlx::query_as(
            "SELECT id FROM help_tickets WHERE user_id=$1 AND id<>$2 AND status<>'closed' ORDER BY updated_at DESC LIMIT 10",
        )
        .bind(owner)
        .bind(id)
        .fetch_all(&st.db)
        .await?;
        m.map(|(name, phone)| json!({"name": name, "phone": phone, "other_open": others.into_iter().map(|(i,)| i).collect::<Vec<_>>()}))
    } else {
        None
    };
    Ok(Json(json!({
        "id": id, "subject": subject, "status": status, "ride_id": ride, "user_id": owner, "member": member,
        "assignee_id": assignee_id, "assignee_name": assignee_name,
        "replies": replies.into_iter().map(|(rid, uid, name, body, at, fid)| json!({"id": rid, "user_id": uid, "name": name, "body": body, "created_at": at, "staff": uid != owner, "file_id": fid})).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct ReplyIn {
    #[serde(default)]
    body: String,
    file_id: Option<i64>,
}

async fn reply_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReplyIn>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, _) = ticket_owner(&st, &u, id, Act::Update).await?;
    if owner == u.id && !u.can(Page::AdminHelp, Act::Update) {
        require(&u, Page::Help, Act::Update)?;
    }
    if status == "closed" {
        return Err(bad(u.lang, ErrorKey::RequestAlreadyClosed));
    }
    // A photo on its own is a reply too ("this is where I'm waiting").
    if req.body.trim().is_empty() && req.file_id.is_none() {
        return Err(bad(u.lang, ErrorKey::ReplyContentRequired));
    }
    check_attachment(&st, &u, req.file_id).await?;
    // Lock the ticket and look at its status again: a close saved since the read above
    // must not be undone by this reply flipping the ticket back to open/answered.
    let mut tx = st.db.begin().await?;
    let (current,): (String,) =
        sqlx::query_as("SELECT status FROM help_tickets WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if current == "closed" {
        return Err(bad(u.lang, ErrorKey::RequestAlreadyClosed));
    }
    sqlx::query(
        "INSERT INTO help_replies (ticket_id, user_id, body, file_id, created_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.body.trim())
    .bind(req.file_id)
    .bind(now())
    .execute(&mut *tx)
    .await?;
    let new_status = if u.id == owner { "open" } else { "answered" };
    sqlx::query("UPDATE help_tickets SET status=$1, updated_at=$2 WHERE id=$3")
        .bind(new_status)
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if u.id == owner {
        notify_perm(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            ("帮助中心：会员回复", "Help center: member reply"),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    } else {
        notify(
            &st.db,
            owner,
            ("帮助中心已回复", "Help center has replied"),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    }
    // The reply text stays in the ticket; the log records who answered and when.
    audit(
        &st.db,
        Some(u.id),
        "help.reply",
        "help_ticket",
        Some(id),
        if u.id == owner {
            "by member"
        } else {
            "by staff"
        },
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn close_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, ..) = ticket_owner(&st, &u, id, Act::Update).await?;
    if owner == u.id && !u.can(Page::AdminHelp, Act::Update) {
        require(&u, Page::Help, Act::Update)?;
    }
    sqlx::query("UPDATE help_tickets SET status='closed', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    // No notification for a resolve, but the member's open tab should update straight away.
    if owner != u.id {
        crate::stream::publish(owner, "notify");
    }
    audit(
        &st.db,
        Some(u.id),
        "help.close",
        "help_ticket",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn reopen_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let (owner, subject, status, _) = ticket_owner(&st, &u, id, Act::Update).await?;
    if owner == u.id && !u.can(Page::AdminHelp, Act::Update) {
        require(&u, Page::Help, Act::Update)?;
    }
    if status != "closed" {
        return Ok(Json(json!({"ok": true})));
    }
    sqlx::query("UPDATE help_tickets SET status='open', updated_at=$1 WHERE id=$2")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    if u.id == owner {
        notify_perm(
            &st.db,
            Page::AdminHelp,
            Act::Update,
            ("帮助中心：请求已重新打开", "Help center: request reopened"),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    } else {
        notify(
            &st.db,
            owner,
            (
                "帮助中心已重新打开你的请求",
                "Help center reopened your request",
            ),
            (&subject, &subject),
            &format!("/help/{id}"),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "help.reopen",
        "help_ticket",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct ClaimIn {
    /// true: take the request; false: hand it back to the queue.
    claim: bool,
}

/// A help-desk member takes a request so others know it is being handled.
/// Releasing only clears the caller's own claim.
async fn claim_ticket(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ClaimIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM help_tickets WHERE id=$1")
        .bind(id)
        .fetch_optional(&st.db)
        .await?;
    exists.ok_or(not_found(u.lang))?;
    let n = if req.claim {
        sqlx::query("UPDATE help_tickets SET assignee_id=$1 WHERE id=$2")
            .bind(u.id)
            .bind(id)
            .execute(&st.db)
            .await?
            .rows_affected()
    } else {
        sqlx::query("UPDATE help_tickets SET assignee_id=NULL WHERE id=$1 AND assignee_id=$2")
            .bind(id)
            .bind(u.id)
            .execute(&st.db)
            .await?
            .rows_affected()
    };
    if n > 0 {
        audit(
            &st.db,
            Some(u.id),
            if req.claim {
                "help.claim"
            } else {
                "help.release"
            },
            "help_ticket",
            Some(id),
            "",
        )
        .await;
    }
    Ok(Json(json!({"ok": true, "changed": n > 0})))
}

// ---------------- Admin workflow guides ----------------

/// The in-app manual is readable by anyone who can see any staff page and
/// written by help-centre staff only (see CRUD handlers below).
pub(crate) fn any_admin_page(u: &CurrentUser) -> bool {
    crate::perms::CATALOG
        .iter()
        .any(|d| d.page.area() != crate::perms::Area::Member && u.can(d.page, Act::View))
}

#[derive(Deserialize)]
struct GuideQuery {
    include_drafts: Option<bool>,
    /// Also return each guide's steps (for in-browser search).
    full: Option<bool>,
}

async fn list_guides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<GuideQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let with_drafts = q.include_drafts.unwrap_or(false) && u.can(Page::AdminHelp, Act::Update);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let full = q.full.unwrap_or(false);
    #[allow(clippy::type_complexity)]
    let rows: Vec<(i64, String, String, String, String, String, bool, String, i64, i32, String, i64, i64)> = sqlx::query_as(
        "SELECT g.id, g.slug, g.title_en, g.title_zh, g.summary_en, g.summary_zh, g.published, g.audience,
                jsonb_array_length(g.steps)::bigint, g.sort_order, g.steps::text,
                COUNT(f.user_id) FILTER (WHERE f.helpful)::bigint,
                COUNT(f.user_id) FILTER (WHERE NOT f.helpful)::bigint
         FROM guide_workflows g LEFT JOIN guide_feedback f ON f.guide_id=g.id
         WHERE ($1 OR g.published) AND ($2 OR g.audience='member')
         GROUP BY g.id ORDER BY g.sort_order, g.id",
    )
    .bind(with_drafts)
    .bind(staff)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(Value::Array(
        rows.into_iter()
            .map(|(id, slug, ten, tzh, sen, szh, published, audience, n, sort, steps, yes, no)| {
                let mut v = json!({
                    "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
                    "summary_en": sen, "summary_zh": szh, "published": published, "audience": audience,
                    "step_count": n, "sort_order": sort,
                });
                if full {
                    v["steps"] = serde_json::from_str(&steps).unwrap_or(json!([]));
                }
                if can_edit {
                    v["helpful_yes"] = json!(yes);
                    v["helpful_no"] = json!(no);
                }
                v
            })
            .collect(),
    )))
}

#[derive(Deserialize)]
struct FeedbackIn {
    helpful: bool,
}

/// One vote per user per guide; voting again replaces the earlier answer.
async fn guide_feedback(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<FeedbackIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let visible: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM guide_workflows WHERE id=$1 AND ($2 OR published) AND ($3 OR audience='member')",
    )
    .bind(id)
    .bind(can_edit)
    .bind(staff)
    .fetch_optional(&st.db)
    .await?;
    visible.ok_or(not_found(u.lang))?;
    sqlx::query(
        "INSERT INTO guide_feedback (guide_id, user_id, helpful, created_at) VALUES ($1,$2,$3,$4)
         ON CONFLICT (guide_id, user_id) DO UPDATE SET helpful=EXCLUDED.helpful, created_at=EXCLUDED.created_at",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.helpful)
    .bind(now())
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct ReorderIn {
    ids: Vec<i64>,
}

async fn reorder_guides(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReorderIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let mut tx = st.db.begin().await?;
    for (i, id) in req.ids.iter().enumerate() {
        sqlx::query("UPDATE guide_workflows SET sort_order=$1 WHERE id=$2")
            .bind(i as i32)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "guide.reorder",
        "guide_workflow",
        None,
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn get_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let row: Option<(i64, String, String, String, String, String, bool, String, String)> = sqlx::query_as(
        "SELECT id, slug, title_en, title_zh, summary_en, summary_zh, published, audience, steps::text
             FROM guide_workflows WHERE slug=$1 AND ($2 OR published) AND ($3 OR audience='member')",
    )
    .bind(&slug)
    .bind(can_edit)
    .bind(staff)
    .fetch_optional(&st.db)
    .await?;
    let (id, slug, ten, tzh, sen, szh, published, audience, steps) =
        row.ok_or(not_found(u.lang))?;
    let steps: Value = serde_json::from_str(&steps).unwrap_or(json!([]));
    Ok(Json(json!({
        "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
        "summary_en": sen, "summary_zh": szh, "published": published, "audience": audience, "steps": steps,
    })))
}

#[derive(Deserialize)]
struct GuideStepIn {
    id: Option<String>,
    title_en: Option<String>,
    title_zh: Option<String>,
    body_en: Option<String>,
    body_zh: Option<String>,
    link: Option<String>,
    image_file_id: Option<i64>,
    note_kind: Option<String>,
    note_en: Option<String>,
    note_zh: Option<String>,
}

#[derive(Deserialize)]
struct GuideIn {
    slug: String,
    title_en: Option<String>,
    title_zh: Option<String>,
    summary_en: Option<String>,
    summary_zh: Option<String>,
    published: Option<bool>,
    #[serde(default = "default_guide_audience")]
    audience: String,
    #[serde(default)]
    steps: Vec<GuideStepIn>,
}

fn default_guide_audience() -> String {
    "staff".into()
}

fn trim_opt(v: &Option<String>) -> &str {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
}

/// Normalise + validate a guide payload; returns columns ready to bind,
/// with steps serialised to JSON text (sqlx has no json feature).
#[allow(clippy::type_complexity)]
fn validate_guide(
    req: &GuideIn,
    lang: crate::i18n::Lang,
) -> AppResult<(String, String, String, String, String, bool, String, String)> {
    let (title_en, title_zh) = (trim_opt(&req.title_en), trim_opt(&req.title_zh));
    if title_en.is_empty() && title_zh.is_empty() {
        return Err(bad(lang, ErrorKey::GuideTitleRequired));
    }
    let slug = req.slug.trim().to_lowercase();
    let ok_slug = !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok_slug {
        return Err(bad(lang, ErrorKey::GuideSlugRequired));
    }
    if !matches!(req.audience.as_str(), "member" | "staff") {
        return Err(bad(lang, ErrorKey::GuideAudienceInvalid));
    }
    if req.steps.len() > 30 {
        return Err(bad(lang, ErrorKey::GuideStepInvalid));
    }
    let mut steps: Vec<Value> = Vec::new();
    for s in &req.steps {
        let (ten, tzh) = (trim_opt(&s.title_en), trim_opt(&s.title_zh));
        if ten.is_empty() && tzh.is_empty() {
            return Err(bad(lang, ErrorKey::GuideStepInvalid));
        }
        let link = s.link.as_deref().map(str::trim).filter(|l| !l.is_empty());
        if let Some(l) = link {
            // Internal app paths only: must start with '/', no protocol-relative or /api URLs.
            if !l.starts_with('/') || l.starts_with("//") || l.starts_with("/api/") || l.contains(':') {
                return Err(bad(lang, ErrorKey::GuideLinkInvalid));
            }
        }
        let step_id =
            s.id.as_deref()
                .map(str::trim)
                .filter(|id| {
                    !id.is_empty()
                        && id.len() <= 36
                        && id
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                })
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string()[..12].to_string());
        let note_en = trim_opt(&s.note_en);
        let note_zh = trim_opt(&s.note_zh);
        let note_kind = trim_opt(&s.note_kind).to_lowercase();
        if !note_kind.is_empty()
            && !matches!(note_kind.as_str(), "tip" | "warning" | "important" | "note")
        {
            return Err(bad(lang, ErrorKey::GuideStepInvalid));
        }
        let mut obj = json!({
            "id": step_id,
            "title_en": ten, "title_zh": tzh,
            "body_en": trim_opt(&s.body_en), "body_zh": trim_opt(&s.body_zh),
            "link": link.unwrap_or(""),
        });
        if let Some(fid) = s.image_file_id.filter(|&id| id > 0) {
            obj["image_file_id"] = json!(fid);
        }
        if !note_en.is_empty() || !note_zh.is_empty() {
            obj["note_kind"] = json!(if note_kind.is_empty() {
                "tip"
            } else {
                note_kind.as_str()
            });
            obj["note_en"] = json!(note_en);
            obj["note_zh"] = json!(note_zh);
        }
        steps.push(obj);
    }
    Ok((
        slug.to_string(),
        title_en.to_string(),
        title_zh.to_string(),
        trim_opt(&req.summary_en).to_string(),
        trim_opt(&req.summary_zh).to_string(),
        req.published.unwrap_or(true),
        req.audience.clone(),
        Value::Array(steps).to_string(),
    ))
}

async fn verify_guide_step_files(
    st: &AppState,
    u: &CurrentUser,
    req: &GuideIn,
    existing_guide_id: Option<i64>,
) -> AppResult<()> {
    let fids: Vec<i64> = req
        .steps
        .iter()
        .filter_map(|s| s.image_file_id.filter(|&id| id > 0))
        .collect();
    if fids.is_empty() {
        return Ok(());
    }
    let mut already_attached = std::collections::HashSet::new();
    if let Some(gid) = existing_guide_id {
        if let Some((raw,)) =
            sqlx::query_as::<_, (String,)>("SELECT steps::text FROM guide_workflows WHERE id=$1")
                .bind(gid)
                .fetch_optional(&st.db)
                .await?
        {
            if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(&raw) {
                for item in arr {
                    if let Some(fid) = item.get("image_file_id").and_then(Value::as_i64) {
                        already_attached.insert(fid);
                    }
                }
            }
        }
    }
    for fid in fids {
        if already_attached.contains(&fid) {
            continue;
        }
        let (ok,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM files WHERE id=$1 AND owner_id=$2 AND mime LIKE 'image/%'",
        )
        .bind(fid)
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
        if ok == 0 {
            return Err(bad(u.lang, ErrorKey::InvalidPhoto));
        }
    }
    Ok(())
}

async fn create_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    verify_guide_step_files(&st, &u, &req, None).await?;
    let (slug, ten, tzh, sen, szh, published, audience, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> = sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1")
        .bind(&slug)
        .fetch_optional(&st.db)
        .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO guide_workflows (slug, title_en, title_zh, summary_en, summary_zh, published, audience, steps, updated_at, updated_by)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8::jsonb,$9,$10) RETURNING id",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&audience)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    audit(
        &st.db,
        Some(u.id),
        "guide.create",
        "guide_workflow",
        Some(id),
        &slug,
    )
    .await;
    Ok(Json(json!({ "id": id })))
}

async fn update_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<i64>,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    verify_guide_step_files(&st, &u, &req, Some(key)).await?;
    let (slug, ten, tzh, sen, szh, published, audience, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1 AND id<>$2")
            .bind(&slug)
            .bind(key)
            .fetch_optional(&st.db)
            .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let n = sqlx::query(
        "UPDATE guide_workflows SET slug=$1, title_en=$2, title_zh=$3, summary_en=$4, summary_zh=$5,
                published=$6, audience=$7, steps=$8::jsonb, updated_at=$9, updated_by=$10 WHERE id=$11",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&audience)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
    .bind(key)
    .execute(&st.db)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "guide.update",
        "guide_workflow",
        Some(key),
        &slug,
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let n = sqlx::query("DELETE FROM guide_workflows WHERE id=$1")
        .bind(key)
        .execute(&st.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "guide.delete",
        "guide_workflow",
        Some(key),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

// ---------------- Grab reimbursements ----------------
// pending → (交通组) approved | returned | rejected → (财务) paid

#[derive(sqlx::FromRow)]
struct ReimbRow {
    id: i64,
    user_id: i64,
    user_name: String,
    ride_id: Option<i64>,
    ride_at: Option<String>,
    amount_cents: i64,
    description: String,
    receipt_file_id: i64,
    payee_name: String,
    payee_phone: String,
    status: String,
    review_note: String,
    reviewer: Option<String>,
    payee_verified: i64,
    paid_ref: String,
    paid_at: Option<String>,
    created_at: String,
}

const REIMB_SELECT: &str = "SELECT r.id, r.user_id, u.name AS user_name, r.ride_id, rd.pickup_at AS ride_at, r.amount_cents, r.description,
    r.receipt_file_id, r.payee_name, r.payee_phone, r.status, r.review_note, rv.name AS reviewer, r.payee_verified, r.paid_ref, r.paid_at, r.created_at
    FROM reimbursements r JOIN users u ON u.id=r.user_id LEFT JOIN rides rd ON rd.id=r.ride_id LEFT JOIN users rv ON rv.id=r.reviewed_by";

fn reimb_json(r: &ReimbRow, u: &CurrentUser) -> Value {
    let finance = u.can(Page::AdminReimbPay, Act::View);
    let own = r.user_id == u.id;
    json!({
        "id": r.id, "user": {"id": r.user_id, "name": r.user_name}, "ride_id": r.ride_id, "ride_at": r.ride_at,
        "amount_cents": r.amount_cents, "description": r.description, "receipt_file_id": r.receipt_file_id,
        // Payee details only to the claimant and finance.
        "payee_name": if own || finance { Some(r.payee_name.clone()) } else { None },
        "payee_phone": if own || finance { Some(r.payee_phone.clone()) } else { None },
        "status": r.status, "review_note": r.review_note, "reviewer": r.reviewer, "payee_verified": r.payee_verified == 1,
        "paid_ref": if own || finance { Some(r.paid_ref.clone()) } else { None }, "paid_at": r.paid_at, "created_at": r.created_at,
    })
}

#[derive(Deserialize)]
struct ReimbQuery {
    scope: Option<String>, // mine | review | payout
    q: Option<String>,
    status: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}

const REIMB_SORTS: &[(&str, &str)] = &[
    ("created_at", "r.created_at"),
    ("amount_cents", "r.amount_cents"),
    ("applicant", "u.name"),
    (
        "status",
        "CASE r.status WHEN 'pending' THEN 0 WHEN 'returned' THEN 1 WHEN 'approved' THEN 2 WHEN 'paid' THEN 3 ELSE 4 END",
    ),
    ("id", "r.id"),
];

async fn list_reimbursements(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReimbQuery>,
) -> AppResult<Json<Value>> {
    let (permission, scope_filter, default_order, unpaginated_limit) = match q.scope.as_deref() {
        Some("review") => (
            Page::AdminReimbReview,
            "TRUE",
            "CASE r.status WHEN 'pending' THEN 0 ELSE 1 END, r.id DESC",
            200,
        ),
        Some("payout") => (
            Page::AdminReimbPay,
            "r.status IN ('approved','paid')",
            "CASE r.status WHEN 'approved' THEN 0 ELSE 1 END, r.id DESC",
            200,
        ),
        _ => (Page::Reimbursements, "r.user_id = $1", "r.id DESC", 100),
    };
    require(&u, permission, Act::View)?;

    let is_paginated = q.page.is_some() || q.limit.is_some();
    let page = q.page.unwrap_or(1).max(1);
    let limit = if is_paginated {
        q.limit.unwrap_or(25).clamp(1, 100)
    } else {
        unpaginated_limit
    };
    let offset = (page - 1).saturating_mul(limit);
    let order = crate::util::order_by(
        q.sort.as_deref(),
        q.dir.as_deref(),
        REIMB_SORTS,
        default_order,
        "r.id DESC",
    );
    let search =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(like_pattern);
    let status = q.status.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let finance = u.can(Page::AdminReimbPay, Act::View);
    // Payee-name matches follow the same per-claim visibility as reimb_json.
    let where_sql = format!(
        "WHERE {scope_filter}
         AND ($3::text IS NULL OR u.name ILIKE $3 OR r.description ILIKE $3
              OR (($2 OR r.user_id = $1) AND r.payee_name ILIKE $3))
         AND ($4::text IS NULL OR r.status = $4)"
    );
    let total = if is_paginated {
        let count_sql = format!(
            "SELECT COUNT(*) FROM reimbursements r JOIN users u ON u.id=r.user_id {where_sql}"
        );
        Some(
            sqlx::query_scalar::<_, i64>(&count_sql)
                .bind(u.id)
                .bind(finance)
                .bind(search.as_deref())
                .bind(status)
                .fetch_one(&st.db)
                .await?,
        )
    } else {
        None
    };

    let sql = format!("{REIMB_SELECT} {where_sql} ORDER BY {order} LIMIT $5 OFFSET $6");
    let rows = sqlx::query_as::<_, ReimbRow>(&sql)
        .bind(u.id)
        .bind(finance)
        .bind(search.as_deref())
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&st.db)
        .await?;
    let items = rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>();
    if let Some(total) = total {
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i64;
        Ok(Json(json!({
            "items": items,
            "total": total,
            "page": page,
            "page_size": limit,
            "total_pages": total_pages.max(1),
        })))
    } else {
        Ok(Json(json!(items)))
    }
}

async fn load_reimb(st: &AppState, lang: Lang, id: i64) -> AppResult<ReimbRow> {
    sqlx::query_as::<_, ReimbRow>(&format!("{REIMB_SELECT} WHERE r.id=$1"))
        .bind(id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(lang))
}

async fn get_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_reimb(&st, u.lang, id).await?;
    if r.user_id != u.id {
        require_one(
            &u,
            &[
                (Page::AdminReimbReview, Act::View),
                (Page::AdminReimbPay, Act::View),
            ],
        )?;
    }
    Ok(Json(reimb_json(&r, &u)))
}

#[derive(Deserialize)]
struct ReimbIn {
    ride_id: Option<i64>,
    amount_cents: i64,
    description: String,
    receipt_file_id: i64,
    payee_name: String,
    payee_phone: String,
}

async fn validate_reimb(st: &AppState, u: &CurrentUser, req: &ReimbIn) -> AppResult<()> {
    if req.amount_cents <= 0 || req.amount_cents > 50_000 {
        return Err(bad(u.lang, ErrorKey::AmountRangeRM001to500));
    }
    if req.payee_name.trim().is_empty() || req.payee_phone.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::PayeeNameAndPhoneRequired));
    }
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
        .bind(req.receipt_file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::ReceiptRequired))?;
    if owner != u.id {
        return Err(forbidden(u.lang, ErrorKey::InvalidReceipt));
    }
    if let Some(rid) = req.ride_id {
        let (p,): (i64,) = sqlx::query_as("SELECT passenger_id FROM rides WHERE id=$1")
            .bind(rid)
            .fetch_optional(&st.db)
            .await?
            .ok_or(not_found(u.lang))?;
        if p != u.id {
            return Err(forbidden(u.lang, ErrorKey::OnlyLinkOwnRides));
        }
    }
    Ok(())
}

/// A receipt, or a ride, pays out once: refuse it while another request for it is still
/// live (anything but rejected). Runs in the caller's transaction after taking a lock,
/// so two requests sent together can't both pass.
async fn ensure_not_claimed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lang: Lang,
    req: &ReimbIn,
    except: Option<i64>,
) -> AppResult<()> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_reimbursement_claim', 0))",
    )
    .execute(&mut **tx)
    .await?;
    let (taken,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reimbursements
         WHERE status <> 'rejected' AND id IS DISTINCT FROM $1
           AND (receipt_file_id = $2 OR ($3::BIGINT IS NOT NULL AND ride_id = $3))",
    )
    .bind(except)
    .bind(req.receipt_file_id)
    .bind(req.ride_id)
    .fetch_one(&mut **tx)
    .await?;
    if taken > 0 {
        return Err(bad(lang, ErrorKey::ReimbursementAlreadyClaimed));
    }
    Ok(())
}

async fn create_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Reimbursements, Act::Create)?;
    validate_reimb(&st, &u, &req).await?;
    let mut tx = st.db.begin().await?;
    ensure_not_claimed(&mut tx, u.lang, &req, None).await?;
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO reimbursements (user_id, ride_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
    )
    .bind(u.id)
    .bind(req.ride_id)
    .bind(req.amount_cents)
    .bind(req.description.trim())
    .bind(req.receipt_file_id)
    .bind(req.payee_name.trim())
    .bind(req.payee_phone.trim())
    .bind(now())
    .bind(now())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    notify_perm(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        ("新的 Grab 报销申请", "New Grab reimbursement request"),
        (
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
        ),
        "/admin/reimbursements",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.create",
        "reimbursement",
        Some(id),
        fmt_rm(req.amount_cents),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

async fn resubmit_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReimbIn>,
) -> AppResult<Json<Value>> {
    let r = load_reimb(&st, u.lang, id).await?;
    if r.user_id != u.id || r.status != "returned" {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    require(&u, Page::Reimbursements, Act::Update)?;
    validate_reimb(&st, &u, &req).await?;
    let mut tx = st.db.begin().await?;
    ensure_not_claimed(&mut tx, u.lang, &req, Some(id)).await?;
    // Still sent back and still theirs at the moment of writing: a review or another
    // resubmission in between wins, and this one is refused.
    let changed = sqlx::query("UPDATE reimbursements SET ride_id=$1, amount_cents=$2, description=$3, receipt_file_id=$4, payee_name=$5, payee_phone=$6, status='pending', payee_verified=0, updated_at=$7 WHERE id=$8 AND user_id=$9 AND status='returned'")
        .bind(req.ride_id)
        .bind(req.amount_cents)
        .bind(req.description.trim())
        .bind(req.receipt_file_id)
        .bind(req.payee_name.trim())
        .bind(req.payee_phone.trim())
        .bind(now())
        .bind(id)
        .bind(u.id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(bad(u.lang, ErrorKey::OnlyReturnedReimbCanBeResubmitted));
    }
    tx.commit().await?;
    notify_perm(
        &st.db,
        Page::AdminReimbReview,
        Act::Update,
        ("报销申请已补资料", "Reimbursement updated and resubmitted"),
        (
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
            &format!("{} · {}", u.name, fmt_rm(req.amount_cents)),
        ),
        "/admin/reimbursements",
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.resubmit",
        "reimbursement",
        Some(id),
        fmt_rm(req.amount_cents),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct ReviewIn {
    action: String, // approve | return | reject
    note: Option<String>,
}

async fn review_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReviewIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbReview, Act::Update)?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "pending" {
        return Err(bad(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
    if r.user_id == u.id {
        return Err(forbidden(u.lang, ErrorKey::CannotReviewOwnRequest));
    }
    let note = req.note.unwrap_or_default();
    let status = match req.action.as_str() {
        "approve" => "approved",
        "return" => "returned",
        "reject" => "rejected",
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    };
    if status != "approved" && note.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::ReasonRequired));
    }
    let changed = sqlx::query(
        "UPDATE reimbursements SET status=$1, review_note=$2, reviewed_by=$3, updated_at=$4 WHERE id=$5 AND status='pending'",
    )
    .bind(status)
    .bind(&note)
    .bind(u.id)
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbAlreadyReviewed));
    }
    let title = match status {
        "approved" => ("报销申请已批准", "Reimbursement approved"),
        "returned" => ("报销申请需补资料", "Reimbursement needs more info"),
        _ => ("报销申请未获批准", "Reimbursement not approved"),
    };
    notify(
        &st.db,
        r.user_id,
        title,
        (
            &format!("{} {}", fmt_rm(r.amount_cents), note),
            &format!("{} {}", fmt_rm(r.amount_cents), note),
        ),
        &format!("/reimbursements/{id}"),
    )
    .await;
    if status == "approved" {
        notify_perm(
            &st.db,
            Page::AdminReimbPay,
            Act::Update,
            ("有报销待付款", "Reimbursement awaiting payment"),
            (
                &format!("{} · {}", r.user_name, fmt_rm(r.amount_cents)),
                &format!("{} · {}", r.user_name, fmt_rm(r.amount_cents)),
            ),
            "/admin/reimbursements",
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        &format!("reimbursement.{}", req.action),
        "reimbursement",
        Some(id),
        note,
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct PayIn {
    payee_verified: bool,
    paid_ref: String,
}

/// 财务核对收款人 → 转账并记录 (manual payout via church-approved bank / TNG channel).
async fn pay_reimbursement(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<PayIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    let r = load_reimb(&st, u.lang, id).await?;
    if r.status != "approved" {
        return Err(bad(u.lang, ErrorKey::OnlyApprovedReimbCanBePaid));
    }
    if r.user_id == u.id {
        return Err(forbidden(u.lang, ErrorKey::CannotPayOwnRequest));
    }
    // Two people see every payout: whoever approved it doesn't also pay it.
    let (reviewed_by,): (Option<i64>,) =
        sqlx::query_as("SELECT reviewed_by FROM reimbursements WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await?;
    if reviewed_by == Some(u.id) {
        return Err(forbidden(u.lang, ErrorKey::CannotPayOwnApproval));
    }
    if !req.payee_verified {
        return Err(bad(u.lang, ErrorKey::PayeeVerificationRequired));
    }
    if req.paid_ref.trim().len() < 4 {
        return Err(bad(u.lang, ErrorKey::TxnRefRequired));
    }
    // The status check above and this update are separate statements, so a second
    // finance admin can pass the check before the first commit. Transitioning from
    // 'approved' in the update itself makes exactly one payout win.
    let paid = sqlx::query(
        "UPDATE reimbursements
         SET status='paid', payee_verified=1, paid_ref=$1, paid_by=$2, paid_at=$3, updated_at=$4
         WHERE id=$5 AND status='approved' AND batch_id IS NULL
           AND user_id <> $2 AND reviewed_by IS DISTINCT FROM $2",
    )
    .bind(req.paid_ref.trim())
    .bind(u.id)
    .bind(now())
    .bind(now())
    .bind(id)
    .execute(&st.db)
    .await?;
    if paid.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::ReimbursementProcessed));
    }
    notify(
        &st.db,
        r.user_id,
        ("报销款项已转账", "Reimbursement paid"),
        (
            &format!(
                "{} · 交易编号 {}",
                fmt_rm(r.amount_cents),
                req.paid_ref.trim()
            ),
            &format!(
                "{} · Transaction ref {}",
                fmt_rm(r.amount_cents),
                req.paid_ref.trim()
            ),
        ),
        &format!("/reimbursements/{id}"),
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.pay",
        "reimbursement",
        Some(id),
        req.paid_ref.trim(),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

// ---------------- Monthly Finance Payout Batches ----------------

#[derive(sqlx::FromRow)]
struct PayoutBatchRow {
    id: i64,
    month: String,
    title: String,
    total_cents: i64,
    claim_count: i64,
    status: String,
    paid_ref: String,
    created_by: String,
    paid_by: Option<String>,
    paid_at: Option<String>,
    created_at: String,
}

async fn list_payout_batches(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::View)?;
    let rows: Vec<PayoutBatchRow> = sqlx::query_as(
        "SELECT b.id, b.month, b.title, b.total_cents, b.claim_count, b.status, b.paid_ref,
                c.name AS created_by, p.name AS paid_by, b.paid_at, b.created_at
         FROM payout_batches b
         JOIN users c ON c.id = b.created_by
         LEFT JOIN users p ON p.id = b.paid_by
         ORDER BY b.id DESC LIMIT 50",
    )
    .fetch_all(&st.db)
    .await?;
    Ok(Json(json!(rows
        .into_iter()
        .map(|b| json!({
            "id": b.id,
            "month": b.month,
            "title": b.title,
            "total_cents": b.total_cents,
            "claim_count": b.claim_count,
            "status": b.status,
            "paid_ref": b.paid_ref,
            "created_by": b.created_by,
            "paid_by": b.paid_by,
            "paid_at": b.paid_at,
            "created_at": b.created_at,
        }))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct CreateBatchIn {
    month: Option<String>,
    title: Option<String>,
    claim_ids: Option<Vec<i64>>,
}

async fn create_payout_batch(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CreateBatchIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    let month = req
        .month
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| now()[..7].to_string());
    if parse_report_month(&month).is_none() {
        return Err(bad(u.lang, ErrorKey::MonthFormatInvalid));
    }

    let mut tx = st.db.begin().await?;
    let eligible: Vec<(i64, i64)> = if let Some(ref ids) = req.claim_ids {
        let mut unique_ids = ids.clone();
        unique_ids.sort_unstable();
        unique_ids.dedup();
        if unique_ids.is_empty() {
            return Err(bad(u.lang, ErrorKey::BatchEmpty));
        }
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT id, amount_cents FROM reimbursements
             WHERE id = ANY($1) AND status = 'approved' AND batch_id IS NULL
               AND user_id <> $2 AND reviewed_by IS DISTINCT FROM $2
             FOR UPDATE",
        )
        .bind(&unique_ids)
        .bind(u.id)
        .fetch_all(&mut *tx)
        .await?;
        if rows.len() != unique_ids.len() {
            return Err(conflict(u.lang, ErrorKey::ReimbursementProcessed));
        }
        rows
    } else {
        sqlx::query_as(
            "SELECT id, amount_cents FROM reimbursements
             WHERE status = 'approved' AND batch_id IS NULL
               AND substr(created_at, 1, 7) <= $1
               AND user_id <> $2 AND reviewed_by IS DISTINCT FROM $2
             FOR UPDATE",
        )
        .bind(&month)
        .bind(u.id)
        .fetch_all(&mut *tx)
        .await?
    };

    if eligible.is_empty() {
        return Err(bad(u.lang, ErrorKey::BatchEmpty));
    }
    let claim_ids: Vec<i64> = eligible.iter().map(|(id, _)| *id).collect();
    let total_cents: i64 = eligible.iter().map(|(_, amt)| *amt).sum();
    let claim_count = claim_ids.len() as i64;
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Payout {month}"));

    let (batch_id,): (i64,) = sqlx::query_as(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, created_by, created_at)
         VALUES ($1,$2,$3,$4,'open',$5,$6) RETURNING id",
    )
    .bind(&month)
    .bind(&title)
    .bind(total_cents)
    .bind(claim_count)
    .bind(u.id)
    .bind(now())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE reimbursements SET batch_id=$1, updated_at=$2 WHERE id = ANY($3)")
        .bind(batch_id)
        .bind(now())
        .bind(&claim_ids)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "reimbursement.batch_create",
        "payout_batch",
        Some(batch_id),
        format!("{title}: {claim_count} claims, {}", fmt_rm(total_cents)),
    )
    .await;
    Ok(Json(json!({
        "id": batch_id,
        "month": month,
        "title": title,
        "total_cents": total_cents,
        "claim_count": claim_count,
    })))
}

fn csv_cell(v: &str) -> String {
    let needs_neutralize = v.starts_with(['=', '+', '-', '@', '\t', '\r'])
        || v.trim_start().starts_with(['=', '+', '-', '@']);
    let safe = if needs_neutralize {
        format!("'{v}")
    } else {
        v.to_string()
    };
    if safe.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}

async fn export_payout_batch_csv(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    require(&u, Page::AdminReimbPay, Act::View)?;
    let exists: Option<(String,)> = sqlx::query_as("SELECT month FROM payout_batches WHERE id=$1")
        .bind(id)
        .fetch_optional(&st.db)
        .await?;
    let Some((month,)) = exists else {
        return Err(not_found(u.lang));
    };
    let rows: Vec<(i64, String, String, i64, String, Option<i64>, String, String)> = sqlx::query_as(
        "SELECT r.id, r.payee_name, r.payee_phone, r.amount_cents, u.name, r.ride_id, r.description, r.status
         FROM reimbursements r JOIN users u ON u.id = r.user_id
         WHERE r.batch_id = $1 ORDER BY r.id",
    )
    .bind(id)
    .fetch_all(&st.db)
    .await?;

    let mut out = String::from(
        "Claim ID,Payee Name,DuitNow / Phone,Amount (MYR),Applicant,Ride ID,Description,Status\r\n",
    );
    for (cid, pname, pphone, cents, uname, ride_id, desc, status) in rows {
        let sign = if cents < 0 { "-" } else { "" };
        let abs_cents = cents.abs();
        let rm_str = format!("{sign}{}.{:02}", abs_cents / 100, abs_cents % 100);
        let eff_payee = if pname.is_empty() { &uname } else { &pname };
        let phone_cell = csv_cell(&pphone);
        let quoted_phone = if phone_cell.starts_with('"') {
            phone_cell
        } else {
            format!("\"{phone_cell}\"")
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\r\n",
            cid,
            csv_cell(eff_payee),
            quoted_phone,
            rm_str,
            csv_cell(&uname),
            ride_id.map(|x| x.to_string()).unwrap_or_default(),
            csv_cell(&desc),
            csv_cell(&status),
        ));
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"grace-ride-payout-batch-{id}-{month}.csv\""),
        )
        .body(Body::from(out))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

async fn cancel_payout_batch(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    let mut tx = st.db.begin().await?;
    let batch: Option<(String,)> =
        sqlx::query_as("SELECT status FROM payout_batches WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((status,)) = batch else {
        return Err(not_found(u.lang));
    };
    if status != "open" {
        return Err(bad(u.lang, ErrorKey::BatchAlreadyPaid));
    }
    sqlx::query("UPDATE reimbursements SET batch_id=NULL, updated_at=$1 WHERE batch_id=$2")
        .bind(now())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM payout_batches WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.batch_cancel",
        "payout_batch",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn pay_payout_batch(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<PayIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReimbPay, Act::Update)?;
    if !req.payee_verified {
        return Err(bad(u.lang, ErrorKey::PayeeVerificationRequired));
    }
    let paid_ref = req.paid_ref.trim();
    if paid_ref.len() < 4 {
        return Err(bad(u.lang, ErrorKey::TxnRefRequired));
    }

    let mut tx = st.db.begin().await?;
    let batch: Option<(String,)> =
        sqlx::query_as("SELECT status FROM payout_batches WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((status,)) = batch else {
        return Err(not_found(u.lang));
    };
    if status != "open" {
        return Err(bad(u.lang, ErrorKey::BatchAlreadyPaid));
    }
    let claims: Vec<(i64, i64, Option<i64>, i64)> = sqlx::query_as(
        "SELECT id, user_id, reviewed_by, amount_cents FROM reimbursements
         WHERE batch_id=$1 AND status='approved' FOR UPDATE",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    if claims.is_empty() {
        return Err(bad(u.lang, ErrorKey::BatchEmpty));
    }
    for &(_cid, uid, reviewed_by, _amt) in &claims {
        if uid == u.id {
            return Err(forbidden(u.lang, ErrorKey::CannotPayOwnRequest));
        }
        if reviewed_by == Some(u.id) {
            return Err(forbidden(u.lang, ErrorKey::CannotPayOwnApproval));
        }
    }

    let ts_now = now();
    let claim_ids: Vec<i64> = claims.iter().map(|(cid, ..)| *cid).collect();
    sqlx::query(
        "UPDATE reimbursements SET status='paid', payee_verified=1, paid_ref=$1, paid_by=$2, paid_at=$3, updated_at=$3
         WHERE id = ANY($4)",
    )
    .bind(paid_ref)
    .bind(u.id)
    .bind(&ts_now)
    .bind(&claim_ids)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE payout_batches SET status='paid', paid_ref=$1, paid_by=$2, paid_at=$3 WHERE id=$4",
    )
    .bind(paid_ref)
    .bind(u.id)
    .bind(&ts_now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    for (cid, uid, _, amt) in claims {
        notify(
            &st.db,
            uid,
            ("报销款项已转账", "Reimbursement paid"),
            (
                &format!("{} · 交易编号 {}", fmt_rm(amt), paid_ref),
                &format!("{} · Transaction ref {}", fmt_rm(amt), paid_ref),
            ),
            &format!("/reimbursements/{cid}"),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "reimbursement.batch_pay",
        "payout_batch",
        Some(id),
        paid_ref,
    )
    .await;
    Ok(Json(json!({ "ok": true, "paid_count": claim_ids.len() })))
}

// ---------------- Monthly report & badges ----------------

#[derive(Deserialize)]
struct ReportQuery {
    month: Option<String>, // YYYY-MM
    user_id: Option<i64>,
    /// "driver" | "passenger" | "all" — which side of rides to report (default: all)
    r#type: Option<String>,
    /// "monthly" | "quarterly" | "yearly" — report period granularity (default: monthly)
    range: Option<String>,
    lang: Option<String>,
}

#[derive(Deserialize)]
struct BadgeQuery {
    lang: Option<String>,
}

/// Parse `YYYY-MM` without slicing by byte offset: a 7-byte non-ASCII string used
/// to panic the handler when a slice landed inside a multi-byte character.
fn parse_report_month(raw: &str) -> Option<(i32, u32)> {
    let b = raw.as_bytes();
    if b.len() != 7
        || b[4] != b'-'
        || !b[..4].iter().all(u8::is_ascii_digit)
        || !b[5..].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let year: i32 = raw[..4].parse().ok()?;
    let month: u32 = raw[5..7].parse().ok()?;
    if !(1900..=9998).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    Some((year, month))
}

pub async fn compute_badges(st: &AppState, user_id: i64, lang: Lang) -> AppResult<Vec<Value>> {
    let (opt_out, carpool): (i64, i64) =
        sqlx::query_as("SELECT badges_opt_out, carpool_enabled FROM users WHERE id=$1")
            .bind(user_id)
            .fetch_one(&st.db)
            .await?;
    if opt_out == 1 {
        return Ok(vec![]);
    }
    let (night,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rides WHERE driver_id=$1 AND status='completed' AND (substr(pickup_at,12,2) >= '21' OR substr(pickup_at,12,2) < '05')",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    let (months,): (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT substr(pickup_at,1,7)) FROM rides WHERE driver_id=$1 AND status='completed' AND pickup_at >= $2",
    )
    .bind(user_id)
    .bind(
        crate::util::today_local()
            .checked_sub_months(chrono::Months::new(6))
            .unwrap_or_default()
            .format("%Y-%m-%d")
            .to_string(),
    )
    .fetch_one(&st.db)
    .await?;
    let (total,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rides WHERE driver_id=$1 AND status='completed'")
            .bind(user_id)
            .fetch_one(&st.db)
            .await?;
    let (shared,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM (SELECT trip_id FROM rides WHERE driver_id=$1 AND status='completed' AND trip_id IS NOT NULL GROUP BY trip_id HAVING COUNT(*) >= 2)",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    // Criteria are public (徽章获得条件应公开透明).
    let defs = [
        (
            "night",
            "宵",
            "N",
            "宵夜王",
            "Night Owl",
            "完成 3 次或以上晚上 9 点后的接送",
            "Complete 3 or more rides after 9 pm",
            night,
            3,
        ),
        (
            "regular",
            "勤",
            "R",
            "常驻司机",
            "Resident Driver",
            "过去 6 个月中至少 3 个月有完成接送",
            "Completed rides in at least 3 of the past 6 months",
            months,
            3,
        ),
        (
            "carpool",
            "爱",
            "C",
            "爱心同行者",
            "Caring Companion",
            "开启共乘并完成 2 趟或以上多人同车接送",
            "Enable carpool and complete 2 or more rides with multiple passengers",
            if carpool == 1 { shared } else { 0 },
            2,
        ),
    ];
    let _ = total;
    Ok(defs
        .iter()
        .map(
            |(key, ch_zh, ch_en, name_zh, name_en, rule_zh, rule_en, have, need)| {
                let ch = t(lang, ch_zh, ch_en);
                let name = t(lang, name_zh, name_en);
                let rule = t(lang, rule_zh, rule_en);
                json!({
                    "key": key,
                    "char": ch,
                    "name": name,
                    "rule": rule,
                    "char_zh": ch_zh,
                    "char_en": ch_en,
                    "name_zh": name_zh,
                    "name_en": name_en,
                    "rule_zh": rule_zh,
                    "rule_en": rule_en,
                    "progress": have,
                    "target": need,
                    "earned": have >= need,
                })
            },
        )
        .collect())
}

async fn my_badges(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<BadgeQuery>,
) -> AppResult<Json<Value>> {
    let lang = q.lang.as_deref().map(Lang::from_code).unwrap_or(u.lang);
    Ok(Json(json!(compute_badges(&st, u.id, lang).await?)))
}

/// 个人月报 — only the member themself or an admin (获授权人员) may view.
/// Supports driver/passenger filter and monthly/quarterly/yearly ranges.
async fn monthly_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReportQuery>,
) -> AppResult<Json<Value>> {
    let target = q.user_id.unwrap_or(u.id);
    if target != u.id {
        require(&u, Page::AdminReports, Act::View)?;
        if !crate::routes::admin::has_global_report_scope(&u) {
            return Err(forbidden(u.lang, ErrorKey::NoPermission));
        }
    } else {
        require(&u, Page::Report, Act::View)?;
    }
    let report_lang = q.lang.as_deref().map(Lang::from_code).unwrap_or(u.lang);
    let rtype = q.r#type.as_deref().unwrap_or("all");
    let range = q.range.as_deref().unwrap_or("monthly");
    let month = q.month.unwrap_or_else(|| now()[..7].to_string());
    let (y, m) =
        parse_report_month(&month).ok_or_else(|| bad(u.lang, ErrorKey::MonthFormatInvalid))?;

    // Period bounds are half-open: `start <= pickup_at < end`. The end is the first
    // day *after* the period, so the selected month/quarter/year is fully included.
    let (start, end): (String, String) = match range {
        "quarterly" => {
            let quarter = (m - 1) / 3;
            let start_m = quarter * 3 + 1;
            let end_m = start_m + 3;
            let (end_y, end_m) = if end_m > 12 {
                (y + 1, end_m - 12)
            } else {
                (y, end_m)
            };
            (
                format!("{:04}-{:02}-01", y, start_m),
                format!("{:04}-{:02}-01", end_y, end_m),
            )
        }
        "yearly" => (format!("{:04}-01-01", y), format!("{:04}-01-01", y + 1)),
        _ => {
            let (end_y, end_m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            (
                format!("{:04}-{:02}-01", y, m),
                format!("{:04}-{:02}-01", end_y, end_m),
            )
        }
    };

    let (name,): (String,) = sqlx::query_as("SELECT name FROM users WHERE id=$1")
        .bind(target)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;

    // `rides_driven` is the target's own driving, never a ride they merely took as a
    // passenger. The type filter narrows which side of the report is populated.
    let (driven,): (i64,) = if rtype == "passenger" {
        (0,)
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM rides r WHERE r.driver_id=$1 AND r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
        )
        .bind(target)
        .bind(&start)
        .bind(&end)
        .fetch_one(&st.db)
        .await?
    };

    let (received,): (i64,) = if rtype == "driver" {
        (0,)
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM rides r WHERE r.passenger_id=$1 AND r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
        )
        .bind(target)
        .bind(&start)
        .bind(&end)
        .fetch_one(&st.db)
        .await?
    };

    let (dist_total, reimb_total): (Option<i64>, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(r.distance_km),0)::BIGINT, COALESCE(SUM(r.reimbursement_amount_cents),0)::BIGINT FROM rides r WHERE r.driver_id=$1 AND r.status='completed' AND r.pickup_at >= $2 AND r.pickup_at < $3",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_one(&st.db)
    .await?;

    let attended: Vec<(String, i64)> = sqlx::query_as(
        "SELECT e.kind, COUNT(*) FROM event_signups s JOIN events e ON e.id=s.event_id
         WHERE s.user_id=$1 AND s.attended=1 AND e.starts_at >= $2 AND e.starts_at < $3 GROUP BY e.kind",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_all(&st.db)
    .await?;

    let events: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT e.id, e.title, e.kind, e.starts_at FROM event_signups s JOIN events e ON e.id=s.event_id
         WHERE s.user_id=$1 AND s.attended=1 AND e.starts_at >= $2 AND e.starts_at < $3 ORDER BY e.starts_at",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_all(&st.db)
    .await?;

    let (paid,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(sh.amount_cents),0)::BIGINT FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id JOIN events e ON e.id=s.event_id
         WHERE sh.user_id=$1 AND sh.status='confirmed' AND e.starts_at >= $2 AND e.starts_at < $3",
    )
    .bind(target)
    .bind(&start)
    .bind(&end)
    .fetch_one(&st.db)
    .await?;

    let total_attended: i64 = attended.iter().map(|a| a.1).sum();
    Ok(Json(json!({
        "user": {"id": target, "name": name},
        "month": month,
        "range": range,
        "type": rtype,
        "period_start": start,
        "period_end": end,
        "rides_driven": driven,
        "rides_received": received,
        "distance_km": dist_total.unwrap_or(0),
        "reimbursement_cents": reimb_total,
        "events_attended": total_attended,
        "by_kind": attended.into_iter().map(|(k, n)| json!({"kind": k, "count": n})).collect::<Vec<_>>(),
        "events": events.into_iter().map(|(id, t, k, at)| json!({"id": id, "title": t, "kind": k, "starts_at": at})).collect::<Vec<_>>(),
        "costs_paid_cents": paid,
        "badges": compute_badges(&st, target, report_lang).await?,
    })))
}
