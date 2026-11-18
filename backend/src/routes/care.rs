//! Caregiver "Safe Arrival" scoped links & zero-login elder ride status.
//!
//! Allows an adult child / booker (or staff) to generate an expiring, revocable link
//! `/care/:token` so an elderly passenger (or family caregiver) can view pickup status,
//! driver & car plate, live arrival ETA, and tap readiness updates without signing in.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::Act;
use crate::routes::rides::{apply_readiness, load_ride, traffic_can, ReadinessReq, RideRow};
use crate::state::AppState;
use crate::util::{audit, now, parse_dt, random_token, Lang};

const TOKEN_LEN: usize = 48;
const CLOSES_AFTER_HOURS: i64 = 12;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/rides/{id}/care-link",
            post(create_care_link).delete(revoke_care_link),
        )
        .route("/care/{token}", get(care_status))
        .route("/care/{token}/readiness", post(care_readiness))
}

#[derive(sqlx::FromRow)]
struct CarePassRow {
    id: i64,
    ride_id: i64,
    label: String,
    valid_until: String,
    closed_at: Option<String>,
}

fn shift(at: &str, hours: i64) -> String {
    parse_dt(at)
        .map(|d| {
            (d + chrono::Duration::hours(hours))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| at.to_string())
}

fn lang_of(headers: &HeaderMap) -> Lang {
    Lang::from_accept_language(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
    )
}

async fn can_manage_care_link(st: &AppState, u: &CurrentUser, r: &RideRow) -> AppResult<bool> {
    if r.passenger_id == u.id || traffic_can(u, r, Act::Update) {
        return Ok(true);
    }
    let user_cg: Option<i64> =
        sqlx::query_scalar("SELECT caregiver_user_id FROM users WHERE id=$1")
            .bind(r.passenger_id)
            .fetch_optional(&st.db)
            .await?
            .flatten();
    if user_cg == Some(u.id) {
        return Ok(true);
    }
    if r.is_proxy == 1 && !r.rider_name.is_empty() {
        let proxy_cg: Option<i64> = sqlx::query_scalar(
            "SELECT caregiver_user_id FROM proxy_riders WHERE user_id=$1 AND name=$2",
        )
        .bind(r.passenger_id)
        .bind(&r.rider_name)
        .fetch_optional(&st.db)
        .await?
        .flatten();
        if proxy_cg == Some(u.id) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Deserialize)]
struct CreateCareLinkIn {
    label: Option<String>,
}

async fn create_care_link(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    req: Option<Json<CreateCareLinkIn>>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if !can_manage_care_link(&st, &u, &r).await? {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    if matches!(
        r.status.as_str(),
        "completed" | "cancelled" | "no_show" | "alt_transport"
    ) {
        return Err(bad(u.lang, ErrorKey::CarePassClosed));
    }

    let ts_now = now();
    let token = random_token(TOKEN_LEN);
    let token_hash = hash_token(&token);
    let valid_until = shift(&r.pickup_at, CLOSES_AFTER_HOURS).max(crate::util::in_hours(2));
    let label = req
        .and_then(|j| j.0.label)
        .map(|s| s.trim().chars().take(60).collect::<String>())
        .unwrap_or_default();

    let mut tx = st.db.begin().await?;
    sqlx::query("UPDATE caregiver_passes SET closed_at=$1 WHERE ride_id=$2 AND closed_at IS NULL")
        .bind(&ts_now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO caregiver_passes (token_hash, ride_id, created_by, label, valid_until, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(&token_hash)
    .bind(id)
    .bind(u.id)
    .bind(&label)
    .bind(&valid_until)
    .bind(&ts_now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    audit(&st.db, Some(u.id), "ride.care_link", "ride", Some(id), "").await;
    Ok(Json(json!({
        "token": token,
        "url": format!("/care/{token}"),
        "valid_until": valid_until,
    })))
}

async fn revoke_care_link(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    if !can_manage_care_link(&st, &u, &r).await? {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    sqlx::query("UPDATE caregiver_passes SET closed_at=$1 WHERE ride_id=$2 AND closed_at IS NULL")
        .bind(now())
        .bind(id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "ride.care_link_revoke",
        "ride",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn load_care_pass(
    st: &AppState,
    token: &str,
    lang: Lang,
) -> AppResult<(CarePassRow, RideRow)> {
    let pass: CarePassRow = sqlx::query_as(
        "SELECT id, ride_id, label, valid_until, closed_at FROM caregiver_passes WHERE token_hash=$1",
    )
    .bind(hash_token(token))
    .fetch_optional(&st.db)
    .await?
    .ok_or_else(|| not_found(lang))?;

    if pass.closed_at.is_some() || pass.valid_until < now() {
        return Err(bad(lang, ErrorKey::CarePassClosed));
    }
    let r = load_ride(st, lang, pass.ride_id).await?;
    if matches!(r.status.as_str(), "cancelled" | "no_show" | "alt_transport") {
        let _ = sqlx::query(
            "UPDATE caregiver_passes SET closed_at=$1 WHERE id=$2 AND closed_at IS NULL",
        )
        .bind(now())
        .bind(pass.id)
        .execute(&st.db)
        .await;
        return Err(bad(lang, ErrorKey::CarePassClosed));
    }
    Ok((pass, r))
}

async fn care_status(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> AppResult<Json<Value>> {
    let lang = lang_of(&headers);
    let (pass, r) = load_care_pass(&st, &token, lang).await?;
    let approved = matches!(r.status.as_str(), "confirmed" | "in_progress");
    let rider_name = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        r.rider_name.clone()
    } else {
        r.p_name.clone()
    };
    let driver = if approved {
        r.driver_id.map(|_| {
            json!({
                "name": r.d_name,
                "vehicle": r.v_model.as_ref().map(|m| json!({
                    "model": m,
                    "color": r.v_color,
                    "plate": r.v_plate,
                })),
            })
        })
    } else {
        None
    };

    let mut location = json!({ "sharing": false });
    if r.status == "in_progress" {
        if let Some(driver_id) = r.driver_id {
            let loc: Option<(f64, f64, Option<f64>, String)> = sqlx::query_as(
                "SELECT lat, lng, heading, updated_at FROM driver_locations WHERE driver_id=$1",
            )
            .bind(driver_id)
            .fetch_optional(&st.db)
            .await?;
            if let Some((_lat, _lng, _h, at)) = loc {
                let mut l = json!({ "sharing": true, "updated_at": at });
                let trip = r.trip_id.unwrap_or(r.id);
                if let Ok(Some(route)) =
                    crate::routing::driver_route(&st, driver_id, Some(trip)).await
                {
                    if let Some(mut eta) =
                        crate::routing::ride_eta(&route, r.id, r.picked_up_at.is_some(), false)
                    {
                        if let Some(obj) = eta.as_object_mut() {
                            obj.remove("target");
                            obj.remove("path");
                            obj.remove("destination");
                        }
                        l["eta"] = eta;
                    }
                }
                location = l;
            }
        }
    }

    Ok(Json(json!({
        "ride_id": r.id,
        "label": pass.label,
        "rider_name": rider_name,
        "booker_name": r.p_name,
        "status": r.status,
        "leg": r.leg,
        "pickup_at": r.pickup_at,
        "area": { "id": r.pickup_area_id, "name": r.area_name },
        "destination": if r.leg == "return" { &r.area_name } else { &r.destination },
        "readiness_status": r.readiness_status.as_deref().unwrap_or("unconfirmed"),
        "delay_minutes": r.delay_minutes.unwrap_or(0),
        "arrived_at_gate_at": r.arrived_at_gate_at,
        "started_at": r.started_at,
        "picked_up_at": r.picked_up_at,
        "completed_at": r.completed_at,
        "driver": driver,
        "location": location,
    })))
}

async fn care_readiness(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
    Json(req): Json<ReadinessReq>,
) -> AppResult<Json<Value>> {
    let lang = lang_of(&headers);
    if !matches!(req.status.as_str(), "ready" | "delayed_5m" | "delayed_10m") {
        return Err(bad(lang, ErrorKey::InvalidStatus));
    }
    let (_pass, r) = load_care_pass(&st, &token, lang).await?;
    apply_readiness(&st, lang, &r, None, &req).await
}
