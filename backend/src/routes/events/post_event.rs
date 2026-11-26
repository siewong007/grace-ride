//! Post-event driver dispersal & supper reimbursement handling.
//! Allows Traffic Leads and Event Organisers to log driver destinations and auto-generate
//! approved mileage reimbursements without requiring individual passenger booking records.

use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, fmt_rm, notify, notify_perm, now};

use super::can_manage;

#[derive(Serialize)]
pub struct AreaFareOption {
    pub id: i64,
    pub name: String,
    pub fare_cents: Option<i64>,
}

#[derive(Serialize)]
pub struct DriverReturnPassengerStats {
    pub total: i64,
    pub supper: i64,
    pub direct_home: i64,
    pub unspecified: i64,
}

#[derive(Serialize)]
pub struct PostEventDriverItem {
    pub id: i64,
    pub name: String,
    pub phone: String,
    pub vehicle: Option<String>,
    pub default_area: Option<AreaFareOption>,
    pub passenger_stats: DriverReturnPassengerStats,
    pub suggested_trip_type: String,
    pub already_claimed: bool,
    pub claimed_amount_cents: Option<i64>,
    pub claimed_status: Option<String>,
    pub reimbursement_excluded: bool,
    pub excluded_ride_count: i64,
    pub total_ride_count: i64,
}

#[derive(Serialize)]
pub struct PostEventDriversResponse {
    pub event_id: i64,
    pub event_title: String,
    pub mileage_claimable: bool,
    pub event_area: Option<AreaFareOption>,
    pub available_areas: Vec<AreaFareOption>,
    pub drivers: Vec<PostEventDriverItem>,
}

async fn check_post_event_auth(
    st: &AppState,
    u: &CurrentUser,
    event_id: i64,
) -> AppResult<(i64, String, Option<i64>, bool)> {
    let row: Option<(i64, String, Option<i64>, String, i64)> = sqlx::query_as(
        "SELECT created_by, title, area_id, status, mileage_claimable FROM events WHERE id = $1",
    )
    .bind(event_id)
    .fetch_optional(&st.db)
    .await?;
    let (created_by, title, area_id, status, mileage_claimable) =
        row.ok_or_else(|| not_found(u.lang))?;

    let allowed = can_manage(u, created_by, Act::Update)
        || (u.can(Page::AdminPlan, Act::Update) && (u.traffic_is_global() || created_by == u.id))
        || (u.can(Page::AdminRides, Act::Update) && (u.traffic_is_global() || created_by == u.id))
        || u.can(Page::AdminEvents, Act::Update);

    if !allowed {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    if status != "published" && status != "finished" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }

    Ok((created_by, title, area_id, mileage_claimable == 1))
}

pub(super) async fn get_post_event_drivers(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(event_id): Path<i64>,
) -> AppResult<Json<PostEventDriversResponse>> {
    let (created_by, event_title, event_area_id, mileage_claimable) =
        check_post_event_auth(&st, &u, event_id).await?;
    let show_phone = u.can(Page::AdminMembers, Act::View)
        || (u.can(Page::AdminRides, Act::View) && (u.traffic_is_global() || created_by == u.id));

    // 1. Fetch available active areas with fares relative to event area
    let areas_with_fares: Vec<(i64, String, Option<i64>)> = if let Some(ea_id) = event_area_id {
        sqlx::query_as(
            "SELECT a.id, a.name, f.amount_cents
             FROM areas a
             LEFT JOIN area_fares f ON (
                 (f.area_a_id = LEAST($1, a.id) AND f.area_b_id = GREATEST($1, a.id))
             )
             WHERE a.active = 1
             ORDER BY a.name",
        )
        .bind(ea_id)
        .fetch_all(&st.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT a.id, a.name, NULL::BIGINT FROM areas a WHERE a.active = 1 ORDER BY a.name",
        )
        .fetch_all(&st.db)
        .await?
    };

    let available_areas: Vec<AreaFareOption> = areas_with_fares
        .iter()
        .map(|(id, name, fare)| AreaFareOption {
            id: *id,
            name: name.clone(),
            fare_cents: *fare,
        })
        .collect();

    let event_area = event_area_id.and_then(|ea_id| {
        available_areas
            .iter()
            .find(|a| a.id == ea_id)
            .map(|a| AreaFareOption {
                id: a.id,
                name: a.name.clone(),
                fare_cents: Some(0),
            })
    });

    // 2. Collect candidate drivers who actually drove or were assigned to drive for this event:
    // a) Drivers assigned to live/completed rides for this event (outbound or return)
    let ride_drivers: Vec<(i64,)> = sqlx::query_as(
        "SELECT DISTINCT driver_id FROM rides
         WHERE event_id = $1 AND driver_id IS NOT NULL
           AND status IN ('confirmed', 'in_progress', 'completed')",
    )
    .bind(event_id)
    .fetch_all(&st.db)
    .await?;

    // b) Drivers assigned to event visitors
    let visitor_drivers: Vec<(i64,)> = sqlx::query_as(
        "SELECT DISTINCT driver_id FROM plan_visitors
         WHERE event_id = $1 AND driver_id IS NOT NULL",
    )
    .bind(event_id)
    .fetch_all(&st.db)
    .await?;

    let mut driver_ids = HashSet::new();
    for (d,) in ride_drivers {
        driver_ids.insert(d);
    }
    for (d,) in visitor_drivers {
        driver_ids.insert(d);
    }

    let mut sorted_driver_ids: Vec<i64> = driver_ids.into_iter().collect();
    sorted_driver_ids.sort_unstable();

    // 3. Batch-load profiles, default home areas, return passenger supper stats, ride counts, and existing claims
    let mut driver_items = Vec::with_capacity(sorted_driver_ids.len());
    if !sorted_driver_ids.is_empty() {
        let user_rows: Vec<(
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT u.id, u.name, u.phone, v.model, v.plate, v.color
             FROM users u
             LEFT JOIN vehicles v ON v.user_id = u.id AND v.is_active = 1
             WHERE u.id = ANY($1) AND u.status = 'active'",
        )
        .bind(&sorted_driver_ids)
        .fetch_all(&st.db)
        .await?;
        let user_map: HashMap<i64, (String, String, Option<String>, Option<String>)> = user_rows
            .into_iter()
            .map(|(uid, name, phone, model, plate, _color)| (uid, (name, phone, model, plate)))
            .collect();

        let area_rows: Vec<(i64, i64, String, Option<i64>)> = sqlx::query_as(
            "SELECT DISTINCT ON (da.user_id) da.user_id, a.id, a.name, f.amount_cents
             FROM driver_areas da
             JOIN areas a ON a.id = da.area_id
             LEFT JOIN area_fares f ON $1::BIGINT IS NOT NULL
                 AND f.area_a_id = LEAST($1, a.id)
                 AND f.area_b_id = GREATEST($1, a.id)
             WHERE da.user_id = ANY($2)
             ORDER BY da.user_id, da.area_id",
        )
        .bind(event_area_id)
        .bind(&sorted_driver_ids)
        .fetch_all(&st.db)
        .await?;
        let mut area_map: HashMap<i64, AreaFareOption> = HashMap::with_capacity(area_rows.len());
        for (uid, aid, aname, fare) in area_rows {
            area_map.insert(
                uid,
                AreaFareOption {
                    id: aid,
                    name: aname,
                    fare_cents: fare,
                },
            );
        }

        let ride_stats_rows: Vec<(i64, i64, i64, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT
                driver_id::BIGINT,
                COUNT(*) FILTER (WHERE leg = 'return')::BIGINT,
                COUNT(*) FILTER (WHERE leg = 'return' AND supper_choice = 'supper')::BIGINT,
                COUNT(*) FILTER (WHERE leg = 'return' AND supper_choice = 'direct_home')::BIGINT,
                COUNT(*) FILTER (WHERE leg = 'return' AND (supper_choice = 'unspecified' OR supper_choice IS NULL))::BIGINT,
                COUNT(*)::BIGINT,
                COUNT(*) FILTER (WHERE reimbursement_excluded = 1)::BIGINT
             FROM rides
             WHERE event_id = $1 AND driver_id = ANY($2)
               AND status IN ('confirmed', 'in_progress', 'completed')
             GROUP BY driver_id",
        )
        .bind(event_id)
        .bind(&sorted_driver_ids)
        .fetch_all(&st.db)
        .await?;
        let mut stats_map: HashMap<i64, (i64, i64, i64, i64, i64, i64)> =
            HashMap::with_capacity(ride_stats_rows.len());
        for (uid, t_ret, s_ret, h_ret, u_ret, t_all, ex_all) in ride_stats_rows {
            stats_map.insert(uid, (t_ret, s_ret, h_ret, u_ret, t_all, ex_all));
        }

        let claim_rows: Vec<(i64, i64, String)> = sqlx::query_as(
            "SELECT DISTINCT ON (user_id) user_id, amount_cents, status
             FROM reimbursements
             WHERE event_id = $1 AND user_id = ANY($2) AND status <> 'rejected'
               AND kind <> 'visitor_special'
             ORDER BY user_id, id DESC",
        )
        .bind(event_id)
        .bind(&sorted_driver_ids)
        .fetch_all(&st.db)
        .await?;
        let mut claim_map: HashMap<i64, (i64, String)> = HashMap::with_capacity(claim_rows.len());
        for (uid, amt, st_val) in claim_rows {
            claim_map.insert(uid, (amt, st_val));
        }

        for did in sorted_driver_ids {
            let Some((name, phone, v_model, v_plate)) = user_map.get(&did) else {
                continue;
            };
            let vehicle = match (v_model.as_ref(), v_plate.as_ref()) {
                (Some(m), Some(p)) => Some(format!("{} · {}", m, p)),
                (Some(m), None) => Some(m.clone()),
                _ => None,
            };
            let default_area = area_map.remove(&did);
            let (
                total_ret,
                supper_ret,
                home_ret,
                unspecified_ret,
                total_ride_count,
                excluded_ride_count,
            ) = stats_map.get(&did).copied().unwrap_or((0, 0, 0, 0, 0, 0));
            let passenger_stats = DriverReturnPassengerStats {
                total: total_ret,
                supper: supper_ret,
                direct_home: home_ret,
                unspecified: unspecified_ret,
            };
            let suggested_trip_type = if supper_ret > 0 {
                "supper".to_string()
            } else {
                "direct_return".to_string()
            };
            let (already_claimed, claimed_amount_cents, claimed_status) =
                match claim_map.remove(&did) {
                    Some((amt, st_val)) => (true, Some(amt), Some(st_val)),
                    None => (false, None, None),
                };
            let reimbursement_excluded =
                total_ride_count > 0 && excluded_ride_count == total_ride_count;

            driver_items.push(PostEventDriverItem {
                id: did,
                name: name.clone(),
                phone: if show_phone {
                    phone.clone()
                } else {
                    String::new()
                },
                vehicle,
                default_area,
                passenger_stats,
                suggested_trip_type,
                already_claimed,
                claimed_amount_cents,
                claimed_status,
                reimbursement_excluded,
                excluded_ride_count,
                total_ride_count,
            });
        }
    }

    Ok(Json(PostEventDriversResponse {
        event_id,
        event_title,
        mileage_claimable,
        event_area,
        available_areas,
        drivers: driver_items,
    }))
}

#[derive(Deserialize)]
pub struct PostEventReimbursementClaim {
    pub driver_id: i64,
    pub trip_type: String, // 'supper' | 'direct_return'
    pub destination_area_id: i64,
    pub amount_cents: i64,
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct PostEventReimbursementsIn {
    pub _supper_area_id: Option<i64>,
    pub supper_note: Option<String>,
    pub claims: Vec<PostEventReimbursementClaim>,
}

pub(super) async fn create_post_event_reimbursements(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(event_id): Path<i64>,
    Json(req): Json<PostEventReimbursementsIn>,
) -> AppResult<Json<Value>> {
    let (_created_by, event_title, event_area_id, mileage_claimable) =
        check_post_event_auth(&st, &u, event_id).await?;
    if !mileage_claimable {
        return Err(bad(u.lang, ErrorKey::EventMileageNotClaimable));
    }

    if req.claims.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let mut tx = st.db.begin().await?;

    // Re-check and lock event status/mileage inside the transaction to close concurrent edit/cancel TOCTOU.
    let ev_row: Option<(String, i64)> =
        sqlx::query_as("SELECT status, mileage_claimable FROM events WHERE id = $1 FOR SHARE")
            .bind(event_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((ev_status, ev_claimable)) = ev_row else {
        return Err(not_found(u.lang));
    };
    if !matches!(ev_status.as_str(), "published" | "finished") {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }
    if ev_claimable != 1 {
        return Err(bad(u.lang, ErrorKey::EventMileageNotClaimable));
    }

    // Advisory locks shared with `reimbursements.rs::ensure_not_claimed` to prevent double-claim races.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_post_event_reimb', $1))",
    )
    .bind(event_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_reimbursement_claim', 0))",
    )
    .execute(&mut *tx)
    .await?;

    let mut lock_driver_ids: Vec<i64> = req.claims.iter().map(|c| c.driver_id).collect();
    lock_driver_ids.sort_unstable();
    lock_driver_ids.dedup();
    for did in &lock_driver_ids {
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_reimbursement_claim', $1))",
        )
        .bind(did)
        .execute(&mut *tx)
        .await?;
    }

    let timestamp = now();
    let mut created_ids: Vec<(i64, i64, i64, bool, bool)> = Vec::new();
    let mut total_cents: i64 = 0;
    let mut approved_count: usize = 0;
    let mut approved_cents: i64 = 0;
    let mut pending_count: usize = 0;
    let mut pending_cents: i64 = 0;

    for claim in &req.claims {
        if claim.amount_cents <= 0 || claim.amount_cents > 50_000 {
            return Err(bad(u.lang, ErrorKey::AmountRangeInvalid));
        }
        if !matches!(claim.trip_type.as_str(), "supper" | "direct_return") {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }

        let driver: Option<(String, String)> =
            sqlx::query_as("SELECT name, phone FROM users WHERE id = $1 AND status = 'active'")
                .bind(claim.driver_id)
                .fetch_optional(&mut *tx)
                .await?;
        let (driver_name, driver_phone) = driver.ok_or_else(|| not_found(u.lang))?;

        let participated_as_driver: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM rides
                 WHERE event_id = $1 AND driver_id = $2
                   AND status IN ('confirmed', 'in_progress', 'completed')
             ) OR EXISTS (
                 SELECT 1 FROM plan_visitors
                 WHERE event_id = $1 AND driver_id = $2
             )",
        )
        .bind(event_id)
        .bind(claim.driver_id)
        .fetch_one(&mut *tx)
        .await?;
        if !participated_as_driver {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }

        let dest_area: Option<(String,)> =
            sqlx::query_as("SELECT name FROM areas WHERE id = $1 AND active = 1")
                .bind(claim.destination_area_id)
                .fetch_optional(&mut *tx)
                .await?;
        let (dest_area_name,) = dest_area.ok_or_else(|| bad(u.lang, ErrorKey::InvalidAction))?;

        // Check if already claimed for this event
        let (existing_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reimbursements WHERE event_id = $1 AND user_id = $2 AND status <> 'rejected' AND kind <> 'visitor_special'",
        )
        .bind(event_id)
        .bind(claim.driver_id)
        .fetch_one(&mut *tx)
        .await?;

        if existing_count > 0 {
            return Err(conflict(u.lang, ErrorKey::ReimbursementAlreadyClaimed));
        }

        let (total_rides, excluded_rides): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*)::BIGINT,
                    COUNT(*) FILTER (WHERE reimbursement_excluded = 1)::BIGINT
             FROM rides
             WHERE event_id = $1 AND driver_id = $2
               AND status IN ('confirmed', 'in_progress', 'completed')",
        )
        .bind(event_id)
        .bind(claim.driver_id)
        .fetch_one(&mut *tx)
        .await?;
        let is_excluded = total_rides > 0 && excluded_rides == total_rides;

        let trip_label = if claim.trip_type == "supper" {
            "宵夜"
        } else {
            "散场直返"
        };
        let note_suffix = claim
            .note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!(" ({})", s))
            .unwrap_or_default();

        let desc = format!(
            "[活动车贴/{}] {} ↔ {}{}",
            trip_label, event_title, dest_area_name, note_suffix
        );

        let configured_fare: Option<i64> = match event_area_id {
            Some(ea_id) if ea_id != claim.destination_area_id => {
                sqlx::query_scalar(
                    "SELECT amount_cents FROM area_fares
                     WHERE area_a_id = LEAST($1, $2) AND area_b_id = GREATEST($1, $2)",
                )
                .bind(ea_id)
                .bind(claim.destination_area_id)
                .fetch_optional(&mut *tx)
                .await?
            }
            _ => None,
        };
        let matches_configured_fare =
            configured_fare.is_some_and(|fare| fare > 0 && fare == claim.amount_cents);

        let can_auto_approve = !is_excluded
            && matches_configured_fare
            && u.can(Page::AdminReimbReview, Act::Update)
            && claim.driver_id != u.id;
        let (claim_status, claim_reviewer, claim_requester): (&str, Option<i64>, Option<i64>) =
            if can_auto_approve {
                ("approved", Some(u.id), None)
            } else {
                ("pending", None, Some(u.id))
            };

        let (reimb_id,): (i64,) = sqlx::query_as(
            "INSERT INTO reimbursements (
                user_id, event_id, destination_area_id, trip_type, amount_cents, description,
                receipt_file_id, payee_name, payee_phone, status, kind, reviewed_by, requested_by,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                NULL, $7, $8, $9, 'mileage', $10, $11,
                $12, $12
            ) RETURNING id",
        )
        .bind(claim.driver_id)
        .bind(event_id)
        .bind(claim.destination_area_id)
        .bind(&claim.trip_type)
        .bind(claim.amount_cents)
        .bind(&desc)
        .bind(&driver_name)
        .bind(&driver_phone)
        .bind(claim_status)
        .bind(claim_reviewer)
        .bind(claim_requester)
        .bind(&timestamp)
        .fetch_one(&mut *tx)
        .await?;

        let needs_review = !can_auto_approve;
        created_ids.push((
            reimb_id,
            claim.driver_id,
            claim.amount_cents,
            needs_review,
            is_excluded,
        ));
        total_cents += claim.amount_cents;
        if needs_review {
            pending_count += 1;
            pending_cents += claim.amount_cents;
        } else {
            approved_count += 1;
            approved_cents += claim.amount_cents;
        }
    }

    tx.commit().await?;

    // Disperse notifications
    for (rid, did, amt, needs_review, is_excluded) in &created_ids {
        if *needs_review {
            let (zh, en) = if *is_excluded {
                (
                    format!(
                        "交通组已为您提交活动车贴 {}（因行程标记为不包报销，需等待审核）。",
                        fmt_rm(*amt)
                    ),
                    format!(
                        "Post-event mileage claim of {} was submitted for review (ride marked no reimbursement).",
                        fmt_rm(*amt)
                    ),
                )
            } else {
                (
                    format!("活动车贴 {} 已提交审核。", fmt_rm(*amt)),
                    format!(
                        "Post-event mileage claim of {} was submitted for review.",
                        fmt_rm(*amt)
                    ),
                )
            };
            notify(
                &st.db,
                *did,
                (
                    "活动车贴已提交审核",
                    "Post-event mileage submitted for review",
                ),
                (&zh, &en),
                &format!("/reimbursements/{rid}"),
            )
            .await;
        } else {
            notify(
                &st.db,
                *did,
                ("活动车贴已登记", "Post-event mileage recorded"),
                (
                    &format!("交通组已为您登记活动车贴 {}，财务将安排打款。", fmt_rm(*amt)),
                    &format!(
                        "Post-event mileage reimbursement of {} was recorded; finance will arrange payout.",
                        fmt_rm(*amt)
                    ),
                ),
                &format!("/reimbursements/{rid}"),
            )
            .await;
        }
    }

    let audit_detail = format!(
        "{} claims ({} auto-approved, {} pending review), total {}{}",
        created_ids.len(),
        approved_count,
        pending_count,
        fmt_rm(total_cents),
        req.supper_note
            .as_deref()
            .map(|s| format!(" · 宵夜: {}", s))
            .unwrap_or_default()
    );

    audit(
        &st.db,
        Some(u.id),
        "event.post_event_mileage",
        "event",
        Some(event_id),
        &audit_detail,
    )
    .await;

    if approved_count > 0 {
        notify_perm(
            &st.db,
            Page::AdminReimbPay,
            Act::Update,
            ("新的活动车贴待打款", "New event mileage awaiting payout"),
            (
                &format!(
                    "{} · {} 笔车贴已批准待付（共 {}）",
                    event_title,
                    approved_count,
                    fmt_rm(approved_cents)
                ),
                &format!(
                    "{} · {} claims approved awaiting payout (total {})",
                    event_title,
                    approved_count,
                    fmt_rm(approved_cents)
                ),
            ),
            "/admin/reimbursements?tab=payout",
        )
        .await;
    }

    if pending_count > 0 {
        notify_perm(
            &st.db,
            Page::AdminReimbReview,
            Act::Update,
            (
                "新的例外车贴待审核",
                "New exception mileage awaiting review",
            ),
            (
                &format!(
                    "{} · {} 笔不包报销行程的例外车贴待审核（共 {}）",
                    event_title,
                    pending_count,
                    fmt_rm(pending_cents)
                ),
                &format!(
                    "{} · {} exception claims for excluded rides awaiting review (total {})",
                    event_title,
                    pending_count,
                    fmt_rm(pending_cents)
                ),
            ),
            "/admin/reimbursements",
        )
        .await;
    }

    Ok(Json(json!({
        "ok": true,
        "created_count": created_ids.len(),
        "auto_approved_count": approved_count,
        "pending_review_count": pending_count,
        "total_cents": total_cents,
    })))
}
