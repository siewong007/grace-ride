//! Ride readiness, driver delay, and gate-arrival status updates.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, AppResult};
use crate::perms::Act;
use crate::state::AppState;
use crate::util::{audit, notify, now, system_message, ErrorKey, Lang};

use super::location::{clear_location_if_idle, clear_passenger_location};
use super::{load_ride, traffic_can, RideRow};

#[derive(Deserialize)]
pub(crate) struct ReadinessReq {
    pub status: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// The passenger's optional reason for missing a pickup: trimmed, at most
/// `UNABLE_REASON_MAX` characters, and the standard wording when left blank. The driver
/// and transport staff read it on the ride (`attention_reason`).
const UNABLE_REASON_MAX: usize = 300;
fn unable_reason(notes: Option<&str>) -> String {
    match notes.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => s.chars().take(UNABLE_REASON_MAX).collect(),
        None => "乘客行前告知无法出席".into(),
    }
}

pub(super) async fn post_readiness(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReadinessReq>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    let is_passenger = r.passenger_id == u.id;
    let is_admin = traffic_can(&u, &r, Act::Update);
    if !is_passenger && !is_admin {
        return Err(forbidden(u.lang, ErrorKey::OnlyPassengerCanRespond));
    }

    apply_readiness(&st, u.lang, &r, Some(u.id), &req).await
}

/// Readiness, delay or "can't make it" for a ride, from its passenger, staff, or the
/// visitor whose pass covers it (`actor` None). Callers have checked who may do this.
pub(crate) async fn apply_readiness(
    st: &AppState,
    lang: Lang,
    r: &RideRow,
    actor: Option<i64>,
    req: &ReadinessReq,
) -> AppResult<Json<Value>> {
    let id = r.id;
    // Trip-progress updates (readiness / delay / gate arrival) only apply while the ride
    // is active. A completed or otherwise terminal ride must not accept new updates,
    // otherwise misleading notifications go out after the trip is done.
    if !matches!(r.status.as_str(), "confirmed" | "in_progress") {
        return Err(conflict(lang, ErrorKey::TripAlreadyCompleted));
    }
    if r.picked_up_at.is_some() {
        return Err(conflict(lang, ErrorKey::PassengerAlreadyPickedUp));
    }

    let valid_statuses = ["ready", "delayed_5m", "delayed_10m", "unable_to_join"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(bad(lang, ErrorKey::InvalidStatus));
    }

    let timestamp = now();
    let rider_disp = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        format!("{}（由 {} 代订）", r.rider_name, r.p_name)
    } else {
        r.p_name.clone()
    };

    if req.status == "unable_to_join" {
        if matches!(r.status.as_str(), "confirmed" | "in_progress") {
            if r.picked_up_at.is_some() {
                return Err(conflict(lang, ErrorKey::PassengerAlreadyPickedUp));
            }
            let cancelled = sqlx::query(
                "UPDATE rides SET status='cancelled', needs_attention=0, updated_at=$1
                 WHERE id=$2 AND status IN ('confirmed','in_progress') AND picked_up_at IS NULL",
            )
            .bind(now())
            .bind(id)
            .execute(&st.db)
            .await?;
            if cancelled.rows_affected() == 0 {
                return Err(conflict(lang, ErrorKey::RideStatusChanged));
            }
            let reason = unable_reason(req.notes.as_deref());
            sqlx::query("UPDATE rides SET readiness_status='unable_to_join', readiness_updated_at=$1, attention_reason=$2, completed_at=COALESCE(completed_at, $1) WHERE id=$3")
                .bind(&timestamp)
                .bind(format!("乘客无法出席：{reason}"))
                .bind(id)
                .execute(&st.db)
                .await?;
            clear_passenger_location(st, id).await?;
            if let Some(d) = r.driver_id {
                clear_location_if_idle(st, d).await?;
                notify(
                    &st.db,
                    d,
                    ("乘客告知无法出席", "Passenger unable to attend"),
                    (
                        &format!("{} 告知今天无法出席，已从您的行程中移除该站。", rider_disp),
                        &format!("{} notified they cannot join today; this stop was removed from your route.", rider_disp),
                    ),
                    "/drive",
                )
                .await;
                if let Some(t) = r.trip_id {
                    st.routing.invalidate(d, t).await;
                    system_message(
                        &st.db,
                        t,
                        &format!("系统：{} 告知无法出席，已取消接送安排。", rider_disp),
                        &format!(
                            "System: {} notified unable to attend; pickup cancelled.",
                            rider_disp
                        ),
                    )
                    .await;
                }
            }
            if r.leg == "outbound" {
                if let Some(paired_id) = r.paired_ride_id {
                    let _ = sqlx::query(
                        "UPDATE rides
                         SET needs_attention=1,
                             attention_reason=COALESCE(NULLIF(attention_reason,''), '去程乘客无法出席，请确认返程接送 / Outbound passenger unable to join; confirm return leg'),
                             updated_at=$1
                         WHERE id=$2 AND leg='return' AND driver_id IS NOT NULL
                           AND status NOT IN ('completed','cancelled','no_show','alt_transport')",
                    )
                    .bind(now())
                    .bind(paired_id)
                    .execute(&st.db)
                    .await;
                }
            }
            audit(
                &st.db,
                actor,
                "ride.unable_to_join",
                "ride",
                Some(id),
                &reason,
            )
            .await;
        }
    } else if req.status == "delayed_5m" || req.status == "delayed_10m" {
        let mins = if req.status == "delayed_5m" { 5 } else { 10 };
        sqlx::query("UPDATE rides SET readiness_status=$1, readiness_updated_at=$2 WHERE id=$3")
            .bind(&req.status)
            .bind(&timestamp)
            .bind(id)
            .execute(&st.db)
            .await?;
        if let Some(d) = r.driver_id {
            notify(
                &st.db,
                d,
                ("乘客需稍等", "Passenger running late"),
                (
                    &format!("{} 告知需稍等约 {} 分钟，请留意。", rider_disp, mins),
                    &format!(
                        "{} notified they need about {} more minutes.",
                        rider_disp, mins
                    ),
                ),
                "/drive",
            )
            .await;
            if let Some(t) = r.trip_id {
                system_message(
                    &st.db,
                    t,
                    &format!("系统：{} 告知需稍候约 {} 分钟。", rider_disp, mins),
                    &format!(
                        "System: {} notified they need ~{} more minutes.",
                        rider_disp, mins
                    ),
                )
                .await;
            }
        }
        audit(
            &st.db,
            actor,
            "ride.delayed",
            "ride",
            Some(id),
            &format!("{mins}m"),
        )
        .await;
    } else if req.status == "ready" {
        sqlx::query(
            "UPDATE rides SET readiness_status='ready', readiness_updated_at=$1 WHERE id=$2",
        )
        .bind(&timestamp)
        .bind(id)
        .execute(&st.db)
        .await?;
        if let Some(d) = r.driver_id {
            notify(
                &st.db,
                d,
                ("乘客已准备就绪", "Passenger is ready"),
                (
                    &format!("{} 已准备就绪！", rider_disp),
                    &format!("{} is ready for pickup!", rider_disp),
                ),
                "/drive",
            )
            .await;
            if let Some(t) = r.trip_id {
                system_message(
                    &st.db,
                    t,
                    &format!("系统：{} 已准备好出发。", rider_disp),
                    &format!("System: {} is ready to depart.", rider_disp),
                )
                .await;
            }
        }
        audit(&st.db, actor, "ride.ready", "ride", Some(id), "").await;
    }

    Ok(Json(json!({"ok": true, "status": req.status})))
}

#[derive(Deserialize)]
pub(super) struct DriverDelayReq {
    delay_minutes: i64,
    #[serde(default)]
    reason: Option<String>,
}

pub(super) async fn post_driver_delay(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<DriverDelayReq>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    let is_driver = r.driver_id == Some(u.id);
    let is_admin = traffic_can(&u, &r, Act::Update);
    if !is_driver && !is_admin {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }

    // Trip-progress updates (readiness / delay / gate arrival) only apply while the ride
    // is active. A completed or otherwise terminal ride must not accept new updates,
    // otherwise misleading notifications go out after the trip is done.
    if !matches!(r.status.as_str(), "confirmed" | "in_progress") {
        return Err(conflict(u.lang, ErrorKey::TripAlreadyCompleted));
    }
    if r.picked_up_at.is_some() {
        return Err(conflict(u.lang, ErrorKey::PassengerAlreadyPickedUp));
    }

    if req.delay_minutes < 0 || req.delay_minutes > 120 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    sqlx::query("UPDATE rides SET delay_minutes=$1 WHERE id=$2")
        .bind(req.delay_minutes)
        .bind(id)
        .execute(&st.db)
        .await?;

    let mins = req.delay_minutes;
    let drv_name = r.d_name.as_deref().unwrap_or(&u.name);
    let note = req
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(300).collect::<String>())
        .unwrap_or_default();
    if mins > 0 {
        notify(
            &st.db,
            r.passenger_id,
            ("司机稍有延误", "Driver slightly delayed"),
            (
                &format!("司机 {drv_name} 因路况稍有延误，预计推迟约 {mins} 分钟到达。车到门前时会再次通知您，请在屋内稍候。"),
                &format!("Driver {drv_name} is delayed by ~{mins} mins due to road conditions. We will notify you when outside, please wait indoors."),
            ),
            &format!("/rides/{id}"),
        )
        .await;

        if let Some(t) = r.trip_id {
            system_message(
                &st.db,
                t,
                &format!("系统：司机预计推迟约 {mins} 分钟到达。"),
                &format!("System: Driver estimated to arrive ~{mins} mins later than planned."),
            )
            .await;
        }
    }
    audit(
        &st.db,
        Some(u.id),
        "ride.driver_delay",
        "ride",
        Some(id),
        &format!("{mins}m {note}"),
    )
    .await;
    Ok(Json(json!({"ok": true, "delay_minutes": mins})))
}

pub(super) async fn post_arrived_gate(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    let is_driver = r.driver_id == Some(u.id);
    let is_admin = traffic_can(&u, &r, Act::Update);
    if !is_driver && !is_admin {
        return Err(forbidden(u.lang, ErrorKey::NotRideDriver));
    }

    // Trip-progress updates (readiness / delay / gate arrival) only apply while the ride
    // is active. A completed or otherwise terminal ride must not accept new updates,
    // otherwise misleading notifications go out after the trip is done.
    if !matches!(r.status.as_str(), "confirmed" | "in_progress") {
        return Err(conflict(u.lang, ErrorKey::TripAlreadyCompleted));
    }
    if r.picked_up_at.is_some() {
        return Err(conflict(u.lang, ErrorKey::PassengerAlreadyPickedUp));
    }

    let timestamp = now();
    sqlx::query("UPDATE rides SET arrived_at_gate_at=$1 WHERE id=$2")
        .bind(&timestamp)
        .bind(id)
        .execute(&st.db)
        .await?;

    let drv_name = r.d_name.as_deref().unwrap_or(&u.name);
    let veh = format!(
        "{} {} · {}",
        r.v_color.clone().unwrap_or_default(),
        r.v_model.clone().unwrap_or_default(),
        r.v_plate.clone().unwrap_or_default()
    );
    let (msg_zh, msg_en) = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        (
            format!(
                "司机 {drv_name} 已到达大门外（{veh}），请通知 {} 出门上车。",
                r.rider_name
            ),
            format!(
                "Driver {drv_name} has arrived outside ({veh}). Please notify {} to come out to board.",
                r.rider_name
            ),
        )
    } else {
        (
            format!("司机 {drv_name} 已到达您的大门外（{veh}），请出门上车。"),
            format!(
                "Driver {drv_name} has arrived outside your gate ({veh}). Please come out to board."
            ),
        )
    };
    notify(
        &st.db,
        r.passenger_id,
        ("司机已到达门前", "Driver arrived at your gate"),
        (&msg_zh, &msg_en),
        &format!("/rides/{id}"),
    )
    .await;

    if let Some(t) = r.trip_id {
        let rider_disp = if r.is_proxy == 1 && !r.rider_name.is_empty() {
            format!("{}（由 {} 代订）", r.rider_name, r.p_name)
        } else {
            r.p_name.clone()
        };
        system_message(
            &st.db,
            t,
            &format!("系统：司机已到达 {} 的上车地点门前。", rider_disp),
            &format!(
                "System: Driver has arrived outside {}'s pickup location.",
                rider_disp
            ),
        )
        .await;
    }
    audit(
        &st.db,
        Some(u.id),
        "ride.arrived_gate",
        "ride",
        Some(id),
        "",
    )
    .await;
    Ok(Json(json!({"ok": true, "arrived_at_gate_at": timestamp})))
}

#[derive(Deserialize)]
pub(crate) struct SupperChoiceReq {
    pub choice: String, // 'supper' | 'direct_home' | 'unspecified'
}

pub(super) async fn post_supper_choice(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SupperChoiceReq>,
) -> AppResult<Json<Value>> {
    let r = load_ride(&st, u.lang, id).await?;
    let is_passenger = r.passenger_id == u.id;
    let is_admin = traffic_can(&u, &r, Act::Update);
    if !is_passenger && !is_admin {
        return Err(forbidden(u.lang, ErrorKey::OnlyPassengerCanRespond));
    }
    if r.leg != "return"
        || matches!(
            r.status.as_str(),
            "completed" | "cancelled" | "no_show" | "alt_transport"
        )
    {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let choice = req.choice.trim();
    if !matches!(choice, "supper" | "direct_home" | "unspecified") {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let timestamp = now();
    let res = sqlx::query(
        "UPDATE rides SET supper_choice = $1, updated_at = $2
         WHERE id = $3 AND status NOT IN ('completed','cancelled','no_show','alt_transport')",
    )
    .bind(choice)
    .bind(&timestamp)
    .bind(id)
    .execute(&st.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
    }

    let rider_disp = if r.is_proxy == 1 && !r.rider_name.is_empty() {
        format!("{}（由 {} 代订）", r.rider_name, r.p_name)
    } else {
        r.p_name.clone()
    };

    if let Some(d) = r.driver_id {
        let (title_zh, title_en, msg_zh, msg_en) = match choice {
            "supper" => (
                "乘客参加宵夜",
                "Passenger joins supper",
                format!("{} 确认聚会后【参加宵夜】。", rider_disp),
                format!(
                    "{} confirmed they will join for supper after the event.",
                    rider_disp
                ),
            ),
            "direct_home" => (
                "乘客直接回家",
                "Passenger heads directly home",
                format!("{} 确认聚会后【直接回家】（不吃宵夜）。", rider_disp),
                format!(
                    "{} confirmed they will head directly home (no supper).",
                    rider_disp
                ),
            ),
            _ => (
                "乘客更新宵夜意向",
                "Passenger updated supper plan",
                format!("{} 重置了宵夜安排意向。", rider_disp),
                format!("{} reset their supper preference.", rider_disp),
            ),
        };
        notify(
            &st.db,
            d,
            (title_zh, title_en),
            (&msg_zh, &msg_en),
            &format!("/rides/{id}"),
        )
        .await;
    }

    audit(
        &st.db,
        Some(u.id),
        "ride.supper_choice",
        "ride",
        Some(id),
        choice,
    )
    .await;

    Ok(Json(json!({"ok": true, "supper_choice": choice})))
}

#[cfg(test)]
mod unable_reason_tests {
    use super::{unable_reason, UNABLE_REASON_MAX};

    #[test]
    fn blank_reason_falls_back_to_standard_wording() {
        assert_eq!(unable_reason(None), "乘客行前告知无法出席");
        assert_eq!(unable_reason(Some("   ")), "乘客行前告知无法出席");
    }

    #[test]
    fn reason_is_trimmed_and_capped() {
        assert_eq!(unable_reason(Some("  fever  ")), "fever");
        let long = "a".repeat(UNABLE_REASON_MAX + 50);
        assert_eq!(
            unable_reason(Some(&long)).chars().count(),
            UNABLE_REASON_MAX
        );
    }
}
