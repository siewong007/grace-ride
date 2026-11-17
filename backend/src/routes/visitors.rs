//! Visitor passes: a one-time link that lets a member's visitor follow their ride to an
//! event, and closes when the journey ends
//! (docs/superpowers/specs/2026-11-16-event-audience-rides-design.md, section 4).
//!
//! - A member who is going (or the organiser) invites a visitor. That books a proxy ride
//!   in the member's name for the visitor, recorded as the visitor group and matched to
//!   drivers on the host's group (`audience::MATCH_GROUP_SQL`).
//! - Opening the link shows its state. The visitor **claims** it with an explicit POST (a
//!   link preview or scanner must not use it up); that browser gets a `gr_visit` cookie,
//!   scoped to `/api/visit`, which no member endpoint accepts.
//! - Visitor endpoints read the ride from the pass, never from the request.
//! - The pass closes for good when the ride is completed, cancelled, or otherwise ended,
//!   when `valid_until` passes, or when the host or staff turn it off. It is checked on
//!   every visitor request and swept by the background job.

use axum::{
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::audience::join_choices;
use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, conflict, forbidden, not_found, unauthorized, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::routes::events::{can_manage, can_see_event};
use crate::routes::rides::{apply_readiness, load_ride, notify_area_drivers, ReadinessReq};
use crate::state::AppState;
use crate::util::{audit, check_phone, norm_dt, now, parse_dt, random_token, require, Lang};

const VISIT_COOKIE: &str = "gr_visit";
const VISIT_PATH: &str = "/api/visit";
const TOKEN_LEN: usize = 48;
/// The link opens this long before pickup.
const OPENS_BEFORE_HOURS: i64 = 24;
/// Backstop: the link stops this long after the event (or pickup, if later) even if the
/// driver never marks the ride complete.
const CLOSES_AFTER_HOURS: i64 = 12;
/// Ride states after which the journey is over.
const ENDED: [&str; 4] = ["completed", "cancelled", "no_show", "alt_transport"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events/{id}/visitors", get(list_passes).post(create_pass))
        .route("/visitor-passes/{id}/reissue", post(reissue))
        .route("/visitor-passes/{id}/revoke", post(revoke))
        .route("/visit/me", get(visit_me))
        .route("/visit/me/readiness", post(visit_readiness))
        .route("/visit/me/location", get(visit_location))
        .route("/visit/{token}", get(preview))
        .route("/visit/{token}/claim", post(claim))
}

#[derive(sqlx::FromRow, Clone)]
struct Pass {
    id: i64,
    event_id: i64,
    host_id: i64,
    host_group_id: i64,
    ride_id: Option<i64>,
    name: String,
    phone: String,
    valid_from: String,
    valid_until: String,
    claimed_at: Option<String>,
    closed_at: Option<String>,
    closed_reason: Option<String>,
}

const PASS_COLS: &str =
    "p.id, p.event_id, p.host_id, p.host_group_id, p.ride_id, p.name, p.phone, \
    p.valid_from, p.valid_until, p.claimed_at, p.closed_at, p.closed_reason";

fn shift(at: &str, hours: i64) -> String {
    parse_dt(at)
        .map(|d| {
            (d + chrono::Duration::hours(hours))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| at.to_string())
}

fn lang_of(parts: &Parts) -> Lang {
    Lang::from_accept_language(
        parts
            .headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    )
}

/// Close a pass for good and sign its browser out.
async fn close(db: &PgPool, pass: i64, reason: &str) -> AppResult<()> {
    sqlx::query("UPDATE visitor_passes SET closed_at=$1, closed_reason=$2 WHERE id=$3 AND closed_at IS NULL")
        .bind(now())
        .bind(reason)
        .bind(pass)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM visitor_sessions WHERE pass_id=$1")
        .bind(pass)
        .execute(db)
        .await?;
    Ok(())
}

/// Close the pass now if its journey has ended, whichever way the ride ended.
async fn close_if_done(db: &PgPool, p: &mut Pass) -> AppResult<()> {
    if p.closed_at.is_some() {
        return Ok(());
    }
    let status: Option<String> = match p.ride_id {
        Some(r) => {
            sqlx::query_scalar("SELECT status FROM rides WHERE id=$1")
                .bind(r)
                .fetch_optional(db)
                .await?
        }
        None => None,
    };
    let reason = match status.as_deref() {
        Some("completed") => Some("completed"),
        Some(s) if ENDED.contains(&s) => Some("cancelled"),
        None => Some("cancelled"),
        _ if now() > p.valid_until => Some("expired"),
        _ => None,
    };
    if let Some(r) = reason {
        close(db, p.id, r).await?;
        p.closed_at = Some(now());
        p.closed_reason = Some(r.into());
    }
    Ok(())
}

/// Background sweep (`jobs::run_once`): close every pass whose journey has ended and drop
/// sessions that are closed or out of date.
pub async fn close_finished(db: &PgPool) -> Result<(), sqlx::Error> {
    let n = now();
    sqlx::query(
        "UPDATE visitor_passes vp SET closed_at = $1, closed_reason = CASE
             WHEN (SELECT status FROM rides WHERE id = vp.ride_id) = 'completed' THEN 'completed'
             WHEN vp.ride_id IS NULL OR (SELECT status FROM rides WHERE id = vp.ride_id) = ANY($2) THEN 'cancelled'
             ELSE 'expired' END
         WHERE vp.closed_at IS NULL
           AND (vp.valid_until < $1 OR vp.ride_id IS NULL OR (SELECT status FROM rides WHERE id = vp.ride_id) = ANY($2))",
    )
    .bind(&n)
    .bind(&ENDED[..])
    .execute(db)
    .await?;
    sqlx::query(
        "DELETE FROM visitor_sessions s WHERE s.expires_at < $1
             OR EXISTS (SELECT 1 FROM visitor_passes vp WHERE vp.id = s.pass_id AND vp.closed_at IS NOT NULL)",
    )
    .bind(&n)
    .execute(db)
    .await?;
    Ok(())
}

/// The event moved: open links close relative to the new time, as if issued now.
pub async fn event_moved(db: &PgPool, event_id: i64, starts_at: &str) -> AppResult<()> {
    let open: Vec<(i64, String)> = sqlx::query_as(
        "SELECT vp.id, r.pickup_at FROM visitor_passes vp JOIN rides r ON r.id = vp.ride_id
         WHERE vp.event_id = $1 AND vp.closed_at IS NULL",
    )
    .bind(event_id)
    .fetch_all(db)
    .await?;
    for (pass, pickup) in open {
        let latest = if pickup.as_str() > starts_at {
            pickup
        } else {
            starts_at.to_string()
        };
        let until = shift(&latest, CLOSES_AFTER_HOURS);
        sqlx::query("UPDATE visitor_passes SET valid_until=$1 WHERE id=$2")
            .bind(&until)
            .bind(pass)
            .execute(db)
            .await?;
        sqlx::query("UPDATE visitor_sessions SET expires_at=$1 WHERE pass_id=$2")
            .bind(&until)
            .bind(pass)
            .execute(db)
            .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------- visitor side

/// The claimed browser's pass. Only `gr_visit` is read; a member session is never a pass,
/// and a pass is never a member.
struct VisitorPass(Pass, Lang);

impl FromRequestParts<AppState> for VisitorPass {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &AppState) -> Result<Self, Self::Rejection> {
        let lang = lang_of(parts);
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(VISIT_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(unauthorized(lang))?;
        if token.len() != TOKEN_LEN {
            return Err(unauthorized(lang));
        }
        let pass: Option<Pass> = sqlx::query_as(&format!(
            "SELECT {PASS_COLS} FROM visitor_sessions s JOIN visitor_passes p ON p.id = s.pass_id
             WHERE s.token_hash = $1 AND s.expires_at > $2"
        ))
        .bind(hash_token(&token))
        .bind(now())
        .fetch_optional(&st.db)
        .await?;
        let mut pass = pass.ok_or(unauthorized(lang))?;
        close_if_done(&st.db, &mut pass).await?;
        if pass.closed_at.is_some() {
            return Err(conflict(lang, ErrorKey::VisitorJourneyEnded));
        }
        Ok(VisitorPass(pass, lang))
    }
}

async fn pass_by_token(st: &AppState, lang: Lang, token: &str) -> AppResult<Pass> {
    if token.len() != TOKEN_LEN {
        return Err(bad(lang, ErrorKey::VisitorLinkInvalid));
    }
    let pass: Option<Pass> = sqlx::query_as(&format!(
        "SELECT {PASS_COLS} FROM visitor_passes p WHERE p.token_hash = $1"
    ))
    .bind(hash_token(token))
    .fetch_optional(&st.db)
    .await?;
    let mut pass = pass.ok_or_else(|| bad(lang, ErrorKey::VisitorLinkInvalid))?;
    close_if_done(&st.db, &mut pass).await?;
    Ok(pass)
}

/// Does this browser already hold the pass?
async fn holds(st: &AppState, jar: &CookieJar, pass: i64) -> AppResult<bool> {
    let Some(c) = jar.get(VISIT_COOKIE) else {
        return Ok(false);
    };
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM visitor_sessions WHERE token_hash=$1 AND pass_id=$2 AND expires_at > $3")
        .bind(hash_token(c.value()))
        .bind(pass)
        .bind(now())
        .fetch_one(&st.db)
        .await?;
    Ok(n > 0)
}

fn state_of(p: &Pass, mine: bool) -> &'static str {
    if p.closed_at.is_some() {
        "closed"
    } else if now() < p.valid_from {
        "not_yet"
    } else if p.claimed_at.is_some() {
        if mine {
            "mine"
        } else {
            "used"
        }
    } else {
        "open"
    }
}

async fn event_brief(st: &AppState, event: i64) -> AppResult<Value> {
    let (title, starts_at, location): (String, String, String) =
        sqlx::query_as("SELECT title, starts_at, location FROM events WHERE id=$1")
            .bind(event)
            .fetch_one(&st.db)
            .await?;
    Ok(json!({"title": title, "starts_at": starts_at, "location": location}))
}

async fn host_name(st: &AppState, host: i64) -> AppResult<String> {
    Ok(sqlx::query_scalar("SELECT name FROM users WHERE id=$1")
        .bind(host)
        .fetch_one(&st.db)
        .await?)
}

/// What the link shows before anything is used up.
async fn preview(
    State(st): State<AppState>,
    jar: CookieJar,
    parts: Parts,
    Path(token): Path<String>,
) -> AppResult<Json<Value>> {
    let lang = lang_of(&parts);
    let p = pass_by_token(&st, lang, &token).await?;
    let mine = holds(&st, &jar, p.id).await?;
    Ok(Json(json!({
        "state": state_of(&p, mine),
        "closed_reason": p.closed_reason,
        "opens_at": p.valid_from,
        "name": p.name,
        "host": host_name(&st, p.host_id).await?,
        "event": event_brief(&st, p.event_id).await?,
    })))
}

/// Use the link up: this browser becomes the only one that can follow the ride.
async fn claim(
    State(st): State<AppState>,
    jar: CookieJar,
    parts: Parts,
    Path(token): Path<String>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = lang_of(&parts);
    let p = pass_by_token(&st, lang, &token).await?;
    let mine = holds(&st, &jar, p.id).await?;
    match state_of(&p, mine) {
        "mine" => return Ok((jar, Json(json!({"ok": true})))),
        "closed" => return Err(conflict(lang, ErrorKey::VisitorJourneyEnded)),
        "not_yet" => return Err(conflict(lang, ErrorKey::VisitorNotOpenYet)),
        "used" => return Err(conflict(lang, ErrorKey::VisitorLinkUsed)),
        _ => {}
    }
    // Claim and session commit together: a failed session insert must not burn the one-use link.
    let mut tx = st.db.begin().await?;
    let claimed = sqlx::query("UPDATE visitor_passes SET claimed_at=$1 WHERE id=$2 AND claimed_at IS NULL AND closed_at IS NULL")
        .bind(now())
        .bind(p.id)
        .execute(&mut *tx)
        .await?;
    if claimed.rows_affected() == 0 {
        return Err(conflict(lang, ErrorKey::VisitorLinkUsed));
    }
    let session = random_token(TOKEN_LEN);
    sqlx::query("INSERT INTO visitor_sessions (token_hash, pass_id, expires_at) VALUES ($1,$2,$3)")
        .bind(hash_token(&session))
        .bind(p.id)
        .bind(&p.valid_until)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    audit(
        &st.db,
        None,
        "visitor.claim",
        "visitor_pass",
        Some(p.id),
        "",
    )
    .await;
    // The server ends the session at `valid_until`, which moves if the event does, so the
    // cookie itself is kept a week longer and never runs out first.
    let secs = parse_dt(&p.valid_until)
        .and_then(|until| parse_dt(&now()).map(|n| (until - n).num_seconds()))
        .unwrap_or(3600)
        .max(60)
        + 7 * 24 * 3600;
    let cookie = Cookie::build((VISIT_COOKIE, session))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(st.cfg.secure_cookies())
        .path(VISIT_PATH)
        .max_age(time::Duration::seconds(secs))
        .build();
    Ok((jar.add(cookie), Json(json!({"ok": true}))))
}

/// The visitor's one page: their ride, and nothing else.
async fn visit_me(
    State(st): State<AppState>,
    VisitorPass(p, lang): VisitorPass,
) -> AppResult<Json<Value>> {
    let ride_id = p
        .ride_id
        .ok_or_else(|| conflict(lang, ErrorKey::VisitorJourneyEnded))?;
    let r = load_ride(&st, lang, ride_id).await?;
    // Driver details once staff have approved the ride, as for any passenger.
    let approved = matches!(r.status.as_str(), "confirmed" | "in_progress");
    Ok(Json(json!({
        "name": p.name,
        "host": { "name": r.p_name, "phone": r.p_phone },
        "event": event_brief(&st, p.event_id).await?,
        "ride": {
            "status": r.status,
            "pickup_at": r.pickup_at,
            "pickup_address": r.pickup_address,
            "area": r.area_name,
            "destination": r.destination,
            "seats": r.seats,
            "readiness_status": r.readiness_status.as_deref().unwrap_or("unconfirmed"),
            "delay_minutes": r.delay_minutes,
            "arrived_at_gate_at": r.arrived_at_gate_at,
            "picked_up_at": r.picked_up_at,
            "driver": if approved { r.d_name.as_ref().map(|n| json!({
                "name": n,
                "phone": r.d_phone,
                "vehicle": { "model": r.v_model, "color": r.v_color, "plate": r.v_plate },
            })) } else { None },
        },
    })))
}

async fn visit_readiness(
    State(st): State<AppState>,
    VisitorPass(p, lang): VisitorPass,
    Json(req): Json<ReadinessReq>,
) -> AppResult<Json<Value>> {
    let ride_id = p
        .ride_id
        .ok_or_else(|| conflict(lang, ErrorKey::VisitorJourneyEnded))?;
    let r = load_ride(&st, lang, ride_id).await?;
    let out = apply_readiness(&st, lang, &r, None, &req).await?;
    if req.status == "unable_to_join" {
        close(&st.db, p.id, "cancelled").await?;
    }
    Ok(out)
}

/// The driver's position while the ride is under way.
async fn visit_location(
    State(st): State<AppState>,
    VisitorPass(p, lang): VisitorPass,
) -> AppResult<Json<Value>> {
    let ride_id = p
        .ride_id
        .ok_or_else(|| conflict(lang, ErrorKey::VisitorJourneyEnded))?;
    let r = load_ride(&st, lang, ride_id).await?;
    if r.status != "in_progress" {
        return Ok(Json(json!({"sharing": false})));
    }
    let loc: Option<(f64, f64, Option<f64>, String)> = sqlx::query_as(
        "SELECT lat, lng, heading, updated_at FROM driver_locations WHERE driver_id=$1",
    )
    .bind(r.driver_id)
    .fetch_optional(&st.db)
    .await?;
    Ok(Json(match loc {
        Some((lat, lng, h, at)) => {
            json!({"sharing": true, "lat": lat, "lng": lng, "heading": h, "updated_at": at})
        }
        None => json!({"sharing": false}),
    }))
}

// ---------------------------------------------------------------- host side

#[derive(Deserialize)]
struct InviteIn {
    name: String,
    phone: Option<String>,
    pickup_at: String,
    pickup_area_id: i64,
    pickup_address: String,
    seats: Option<i64>,
    assistance: Option<String>,
    notes: Option<String>,
}

fn link(st: &AppState, token: &str) -> String {
    format!("{}/visit/{}", st.cfg.public_url, token)
}

async fn insert_pass(db: &mut sqlx::PgConnection, p: &Pass) -> AppResult<(i64, String)> {
    let token = random_token(TOKEN_LEN);
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO visitor_passes (token_hash, event_id, host_id, host_group_id, ride_id, name, phone, valid_from, valid_until, created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
    )
    .bind(hash_token(&token))
    .bind(p.event_id)
    .bind(p.host_id)
    .bind(p.host_group_id)
    .bind(p.ride_id)
    .bind(&p.name)
    .bind(&p.phone)
    .bind(&p.valid_from)
    .bind(&p.valid_until)
    .bind(now())
    .fetch_one(db)
    .await?;
    Ok((id, token))
}

/// Invite a visitor to an event: books their ride (as a proxy ride in the host's name)
/// and returns the one-time link to send them.
async fn create_pass(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(event_id): Path<i64>,
    Json(req): Json<InviteIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Rides, Act::Create)?;
    if !can_see_event(&st, &u, event_id).await? {
        return Err(forbidden(u.lang, ErrorKey::CannotViewEvent));
    }
    let (status, starts_at, location, created_by): (String, String, String, i64) =
        sqlx::query_as("SELECT status, starts_at, location, created_by FROM events WHERE id=$1")
            .bind(event_id)
            .fetch_optional(&st.db)
            .await?
            .ok_or_else(|| not_found(u.lang))?;
    if status != "published" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    // The visitor's drivers come from the host's group: the one they joined as, or, for an
    // organiser who isn't going, their best fit among the event's choices.
    let signup: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT as_group_id FROM event_signups WHERE event_id=$1 AND user_id=$2 AND status='going'",
    )
    .bind(event_id)
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;
    let organiser = can_manage(&u, created_by, Act::Update);
    if signup.is_none() && !organiser {
        return Err(forbidden(u.lang, ErrorKey::OnlyGoingCanInviteVisitor));
    }
    let host_group = match signup.and_then(|(g,)| g) {
        Some(g) => g,
        None => join_choices(&st.db, &u, event_id)
            .await?
            .first()
            .map(|(g, _)| *g)
            .ok_or_else(|| bad(u.lang, ErrorKey::JoinAsRequired))?,
    };

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(bad(u.lang, ErrorKey::VisitorNameRequired));
    }
    let phone = req.phone.unwrap_or_default().trim().to_string();
    check_phone(u.lang, &phone)?;
    let at = norm_dt(&req.pickup_at).ok_or_else(|| bad(u.lang, ErrorKey::InvalidPickupTime))?;
    if at.as_str() < &now()[..16] {
        return Err(bad(u.lang, ErrorKey::PickupTimeInPast));
    }
    let seats = req.seats.unwrap_or(1);
    if !(1..=6).contains(&seats) {
        return Err(bad(u.lang, ErrorKey::SeatsRange));
    }
    if req.pickup_address.trim().is_empty() {
        return Err(bad(u.lang, ErrorKey::AddressAndDestinationRequired));
    }
    let area: Option<i64> = sqlx::query_scalar("SELECT id FROM areas WHERE id=$1 AND active=1")
        .bind(req.pickup_area_id)
        .fetch_optional(&st.db)
        .await?;
    area.ok_or_else(|| bad(u.lang, ErrorKey::PickupAreaRequired))?;
    let visitor_group: Option<i64> =
        sqlx::query_scalar("SELECT id FROM groups WHERE is_visitor=1 ORDER BY id LIMIT 1")
            .fetch_optional(&st.db)
            .await?;

    let n = now();
    let opens = shift(&at, -OPENS_BEFORE_HOURS);
    let valid_from = if opens < n { n.clone() } else { opens };
    let latest = if at.as_str() > starts_at.as_str() {
        at.clone()
    } else {
        starts_at.clone()
    };
    let valid_until = shift(&latest, CLOSES_AFTER_HOURS);

    let mut tx = st.db.begin().await?;
    let (ride_id,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, assistance, notes,
                            is_proxy, rider_name, rider_phone, rider_relationship, created_at, updated_at, as_group_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,1,$10,$11,'visitor',$12,$12,$13) RETURNING id",
    )
    .bind(u.id)
    .bind(event_id)
    .bind(&at)
    .bind(req.pickup_area_id)
    .bind(req.pickup_address.trim())
    .bind(&location)
    .bind(seats)
    .bind(req.assistance.unwrap_or_default())
    .bind(req.notes.unwrap_or_default())
    .bind(&name)
    .bind(&phone)
    .bind(&n)
    .bind(visitor_group)
    .fetch_one(&mut *tx)
    .await?;
    let pass = Pass {
        id: 0,
        event_id,
        host_id: u.id,
        host_group_id: host_group,
        ride_id: Some(ride_id),
        name: name.clone(),
        phone,
        valid_from: valid_from.clone(),
        valid_until: valid_until.clone(),
        claimed_at: None,
        closed_at: None,
        closed_reason: None,
    };
    let (pass_id, token) = insert_pass(&mut tx, &pass).await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "ride.create",
        "ride",
        Some(ride_id),
        &at,
    )
    .await;
    audit(
        &st.db,
        Some(u.id),
        "visitor.invite",
        "event",
        Some(event_id),
        format!("pass {pass_id}: {name}"),
    )
    .await;
    notify_area_drivers(
        &st,
        req.pickup_area_id,
        Some(host_group),
        u.id,
        &at,
        &format!("{name} ({} 代订)", u.name),
        seats,
    )
    .await?;
    Ok(Json(json!({
        "id": pass_id, "url": link(&st, &token), "ride_id": ride_id,
        "valid_from": valid_from, "valid_until": valid_until,
    })))
}

/// The host sees the visitors they invited; the organiser and event staff see them all.
async fn list_passes(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(event_id): Path<i64>,
) -> AppResult<Json<Value>> {
    if !can_see_event(&st, &u, event_id).await? {
        return Err(forbidden(u.lang, ErrorKey::CannotViewEvent));
    }
    let created_by: i64 = sqlx::query_scalar("SELECT created_by FROM events WHERE id=$1")
        .bind(event_id)
        .fetch_one(&st.db)
        .await?;
    let all = can_manage(&u, created_by, Act::View) || u.can(Page::AdminRides, Act::View);
    let mut passes: Vec<Pass> = sqlx::query_as(&format!(
        "SELECT {PASS_COLS} FROM visitor_passes p
         WHERE p.event_id = $1 AND ($2 OR p.host_id = $3) AND p.closed_reason IS DISTINCT FROM 'reissued'
         ORDER BY p.id DESC"
    ))
    .bind(event_id)
    .bind(all)
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    let mut out = Vec::new();
    for p in passes.iter_mut() {
        close_if_done(&st.db, p).await?;
        let ride_status: Option<String> = match p.ride_id {
            Some(r) => {
                sqlx::query_scalar("SELECT status FROM rides WHERE id=$1")
                    .bind(r)
                    .fetch_optional(&st.db)
                    .await?
            }
            None => None,
        };
        out.push(json!({
            "id": p.id,
            "name": p.name,
            "phone": p.phone,
            "ride_id": p.ride_id,
            "ride_status": ride_status,
            "host": { "id": p.host_id, "name": host_name(&st, p.host_id).await? },
            // For the host, "used" means the visitor has opened it.
            "state": match state_of(p, false) { "used" => "claimed", s => s },
            "closed_reason": p.closed_reason,
            "valid_from": p.valid_from,
            "valid_until": p.valid_until,
        }));
    }
    Ok(Json(json!(out)))
}

/// A pass the caller may manage: its host, the event's organiser, or transport staff.
async fn managed_pass(st: &AppState, u: &CurrentUser, id: i64) -> AppResult<Pass> {
    let mut p: Pass = sqlx::query_as(&format!(
        "SELECT {PASS_COLS} FROM visitor_passes p WHERE p.id = $1"
    ))
    .bind(id)
    .fetch_optional(&st.db)
    .await?
    .ok_or_else(|| not_found(u.lang))?;
    let created_by: i64 = sqlx::query_scalar("SELECT created_by FROM events WHERE id=$1")
        .bind(p.event_id)
        .fetch_one(&st.db)
        .await?;
    if p.host_id != u.id
        && !can_manage(u, created_by, Act::Update)
        && !u.can(Page::AdminRides, Act::Update)
    {
        return Err(forbidden(u.lang, ErrorKey::OnlyGoingCanInviteVisitor));
    }
    close_if_done(&st.db, &mut p).await?;
    if p.closed_at.is_some() {
        return Err(conflict(u.lang, ErrorKey::VisitorJourneyEnded));
    }
    Ok(p)
}

/// A new link for the same journey (say, the visitor changed phone). The old one stops.
async fn reissue(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let old = managed_pass(&st, &u, id).await?;
    // Closing the old link and making the new one is one step, and only the first of two
    // quick presses gets to close it, so there is never more than one working link.
    let mut tx = st.db.begin().await?;
    let closed = sqlx::query(
        "UPDATE visitor_passes SET closed_at=$1, closed_reason='reissued' WHERE id=$2 AND closed_at IS NULL",
    )
    .bind(now())
    .bind(old.id)
    .execute(&mut *tx)
    .await?;
    if closed.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::VisitorJourneyEnded));
    }
    sqlx::query("DELETE FROM visitor_sessions WHERE pass_id=$1")
        .bind(old.id)
        .execute(&mut *tx)
        .await?;
    let (new_id, token) = insert_pass(&mut tx, &old).await?;
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "visitor.reissue",
        "visitor_pass",
        Some(new_id),
        format!("replaces {}", old.id),
    )
    .await;
    Ok(Json(json!({"id": new_id, "url": link(&st, &token)})))
}

/// Turn the link off. The ride stays; the host manages it as any ride they booked.
async fn revoke(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let p = managed_pass(&st, &u, id).await?;
    close(&st.db, p.id, "revoked").await?;
    audit(
        &st.db,
        Some(u.id),
        "visitor.revoke",
        "visitor_pass",
        Some(p.id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}
