//! Live iCalendar (.ics) subscription feed management and rendering.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use serde_json::{json, Value};

use crate::auth::{hash_token, CurrentUser};
use crate::error::{not_found, AppError, AppResult};
use crate::state::AppState;
use crate::util::{audit, now, parse_dt, random_token, t, Lang};

// ---------------- Live Calendar Subscription (.ics) ----------------

pub(super) async fn get_calendar_feed(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let (has_hash,): (bool,) =
        sqlx::query_as("SELECT calendar_token_hash IS NOT NULL FROM users WHERE id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    Ok(Json(json!({ "enabled": has_hash })))
}

pub(super) async fn rotate_calendar_feed(
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

pub(super) async fn revoke_calendar_feed(
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
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\r' => {}
            '\n' => out.push_str("\\n"),
            '\u{2028}' | '\u{2029}' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

fn fmt_ics_dt(raw: &str) -> String {
    let Some(dt) = parse_dt(raw) else {
        return raw
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == 'T' || *c == 'Z')
            .take(16)
            .collect();
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

pub(super) async fn serve_calendar_ics(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(raw_token): Path<String>,
) -> AppResult<Response> {
    let accept_lang = Lang::from_accept_language(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    );
    let token = raw_token.strip_suffix(".ics").unwrap_or(&raw_token);
    if token.len() < 16 {
        return Err(not_found(accept_lang));
    }
    let user: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT id, name, lang FROM users WHERE calendar_token_hash=$1 AND status='active'",
    )
    .bind(hash_token(token))
    .fetch_optional(&st.db)
    .await?;
    let Some((uid, _uname, user_lang)) = user else {
        return Err(not_found(accept_lang));
    };
    let lang = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .and_then(Lang::parse_accept_language)
        .unwrap_or_else(|| Lang::from_code(&user_lang));

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

    let (scope, _) = crate::audience::group_scopes(&st.db, uid).await?;
    let events: Vec<(i64, String, String, String, String)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT e.id, e.title, e.starts_at, e.location, e.description
         FROM event_signups es JOIN events e ON e.id = es.event_id
         WHERE es.user_id=$1 AND es.status='going' AND e.status IN ('published','finished') AND e.starts_at >= $2
           AND {}
         ORDER BY e.starts_at",
        crate::audience::visible_events_sql("$1", "$3")
    ))
    .bind(uid)
    .bind(&cutoff)
    .bind(&scope)
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
