//! Transactional application and approval of event transport plans.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, AppResult};
use crate::perms::{self, Act, Page};
use crate::routes::rides::seat_capacity;
use crate::state::AppState;
use crate::util::{audit, notify, now, require, system_message, ErrorKey};

use super::{
    drafts::DraftVisitorEntry, load_event, other_load_on, plan_drivers_on, plan_rides_on,
    DraftEntry, PlanDriver, PlanRide, PLANNABLE,
};

#[derive(Deserialize)]
pub(super) struct Change {
    ride_id: i64,
    /// The driver the board showed for this ride; must still be current.
    from_driver_id: Option<i64>,
    /// New driver, or none to put the ride back in the open pool.
    driver_id: Option<i64>,
}

#[derive(Deserialize)]
pub(super) struct SavePlan {
    changes: Vec<Change>,
    draft_id: Option<i64>,
}

pub(super) async fn save_plan(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SavePlan>,
) -> AppResult<Json<Value>> {
    apply_plan_impl(&st, &u, id, req, false).await
}

pub(super) async fn approve_plan(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SavePlan>,
) -> AppResult<Json<Value>> {
    apply_plan_impl(&st, &u, id, req, true).await
}

async fn apply_plan_impl(
    st: &AppState,
    u: &CurrentUser,
    id: i64,
    req: SavePlan,
    is_approve_route: bool,
) -> AppResult<Json<Value>> {
    require(u, Page::AdminPlan, Act::Update)?;
    let ev = load_event(st, u, id).await?;

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let ev_status: String = sqlx::query_scalar("SELECT status FROM events WHERE id = $1 FOR SHARE")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    if ev_status != "published" {
        return Err(bad(u.lang, ErrorKey::EventNotOpen));
    }

    let target_draft: Option<(i64, String, String, Option<i64>, Option<i64>, i64)> =
        if let Some(did) = req.draft_id {
            let row: Option<(i64, String, String, Option<i64>, Option<i64>, i64)> = sqlx::query_as(
                "SELECT id, status, visitors, submitted_by, created_by, updated_by
                 FROM event_plan_drafts
                 WHERE id = $1 AND event_id = $2
                 FOR UPDATE",
            )
            .bind(did)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(r) = row else {
                return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
            };
            if !matches!(r.1.as_str(), "draft" | "pending_approval" | "returned") {
                return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
            }
            Some(r)
        } else if is_approve_route {
            sqlx::query_as(
                "SELECT id, status, visitors, submitted_by, created_by, updated_by
                 FROM event_plan_drafts
                 WHERE event_id = $1 AND status = 'pending_approval'
                 ORDER BY version DESC, id DESC
                 LIMIT 1
                 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
        } else {
            None
        };

    // Permission check:
    // - Approving a draft (or calling /plan/approve) always requires traffic lead authority.
    // - Direct live edits without a draft require traffic lead, legacy `transport` role, or
    //   `cell_leader` on an event they created. Non-lead `traffic_member` and other roles
    //   must save drafts and submit for traffic lead approval.
    let can_direct_apply = u.is_traffic_lead()
        || u.roles.iter().any(|r| r == perms::TRANSPORT_ROLE)
        || (u.roles.iter().any(|r| r == perms::CELL_LEADER_ROLE) && ev.created_by == u.id);
    if is_approve_route || target_draft.is_some() {
        if !u.is_traffic_lead() {
            return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
        }
    } else if !can_direct_apply {
        return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
    }

    let draft_visitors_parsed: Vec<DraftVisitorEntry> = target_draft
        .as_ref()
        .map(|(_, _, visitors_raw, _, _, _)| serde_json::from_str(visitors_raw).unwrap_or_default())
        .unwrap_or_default();

    // Lock all potentially touched drivers in sorted order BEFORE reading rides/drivers/load
    // so concurrent driver_claim / staff_reassign transactions cannot race capacity checks.
    let mut prelock: HashSet<i64> = req.changes.iter().filter_map(|c| c.driver_id).collect();
    for dv in &draft_visitors_parsed {
        if let Some(d) = dv.driver_id {
            prelock.insert(d);
        }
    }
    let existing_assigned: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT driver_id FROM rides WHERE event_id = $1 AND driver_id IS NOT NULL",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    prelock.extend(existing_assigned);
    let mut lock_order: Vec<i64> = prelock.into_iter().collect();
    lock_order.sort_unstable();
    for d in &lock_order {
        sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
            .bind(d)
            .fetch_optional(&mut *tx)
            .await?;
    }

    let rides = plan_rides_on(&mut *tx, id).await?;
    let by_id: HashMap<i64, &PlanRide> = rides.iter().map(|r| (r.id, r)).collect();

    // Only real changes, each ride once.
    let mut changes: BTreeMap<i64, &Change> = BTreeMap::new();
    for c in &req.changes {
        let r = by_id
            .get(&c.ride_id)
            .ok_or_else(|| conflict(u.lang, ErrorKey::RideStatusChanged))?;
        if r.driver_id != c.from_driver_id {
            return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
        }
        if c.driver_id != r.driver_id {
            changes.insert(c.ride_id, c);
        }
    }

    if changes.is_empty() && target_draft.is_none() {
        return Ok(Json(json!({ "ok": true, "changed": 0 })));
    }

    // Validate every new driver and the seats each driver ends up with.
    let drivers = plan_drivers_on(&mut *tx).await?;
    let scopes = crate::audience::drive_scopes_on(&mut *tx).await?;
    let driver: HashMap<i64, &PlanDriver> = drivers.iter().map(|d| (d.id, d)).collect();
    let final_driver = |r: &PlanRide| match changes.get(&r.id) {
        Some(c) => c.driver_id,
        None => r.driver_id,
    };
    for c in changes.values() {
        if let Some(d) = c.driver_id {
            if !driver.contains_key(&d) {
                return Err(bad(u.lang, ErrorKey::MemberNotVolunteerDriver));
            }
            if by_id[&c.ride_id].passenger_id == d {
                return Err(bad(u.lang, ErrorKey::DriverCannotBePassenger));
            }
        }
    }
    let mut touched: HashSet<i64> = changes.values().filter_map(|c| c.driver_id).collect();
    for dv in &draft_visitors_parsed {
        if dv.op != "delete" {
            if let Some(d) = dv.driver_id {
                touched.insert(d);
            }
        }
    }
    if !touched.is_empty() {
        let other = other_load_on(&mut tx, id, &rides, &ev.starts_at).await?;
        let in_prog_rows: Vec<(i64, String, i64)> = sqlx::query_as(
            "SELECT driver_id, leg, seats FROM rides
             WHERE event_id = $1 AND driver_id IS NOT NULL AND status = 'in_progress'",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let live_visitors: Vec<(i64, Option<i64>, String, i64)> = sqlx::query_as(
            "SELECT id, driver_id, leg, seats FROM plan_visitors WHERE event_id = $1",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let mut eff_visitors: HashMap<i64, (Option<i64>, String, i64)> = live_visitors
            .into_iter()
            .map(|(vid, did, leg, seats)| (vid, (did, leg, seats)))
            .collect();
        let mut added_visitors: Vec<(Option<i64>, String, i64)> = Vec::new();
        for dv in &draft_visitors_parsed {
            match dv.op.as_str() {
                "add" => added_visitors.push((
                    dv.driver_id,
                    dv.leg.clone().unwrap_or_else(|| "outbound".into()),
                    dv.seats.unwrap_or(1),
                )),
                "update" => {
                    if let Some(vid) = dv.live_visitor_id {
                        eff_visitors.insert(
                            vid,
                            (
                                dv.driver_id,
                                dv.leg.clone().unwrap_or_else(|| "outbound".into()),
                                dv.seats.unwrap_or(1),
                            ),
                        );
                    }
                }
                "delete" => {
                    if let Some(vid) = dv.live_visitor_id {
                        eff_visitors.remove(&vid);
                    }
                }
                _ => {}
            }
        }
        for d in &touched {
            let Some(&dr) = driver.get(d) else {
                return Err(bad(u.lang, ErrorKey::DriverNotFound));
            };
            let cap = seat_capacity(dr.spare_seats, dr.v_seats);
            for (is_ret, other_map) in [(false, &other.outbound), (true, &other.ret)] {
                let assigned_on_leg = changes.values().any(|c| {
                    c.driver_id == Some(*d) && (by_id[&c.ride_id].leg == "return") == is_ret
                }) || draft_visitors_parsed.iter().any(|dv| {
                    dv.op != "delete"
                        && dv.driver_id == Some(*d)
                        && (dv.leg.as_deref() == Some("return")) == is_ret
                });
                if !assigned_on_leg {
                    continue;
                }
                let mine: Vec<&PlanRide> = rides
                    .iter()
                    .filter(|r| (r.leg == "return") == is_ret && final_driver(r) == Some(*d))
                    .collect();
                let in_prog_seats: i64 = in_prog_rows
                    .iter()
                    .filter(|(did, leg, _)| *did == *d && (leg == "return") == is_ret)
                    .map(|(_, _, s)| *s)
                    .sum();
                let in_prog_count: i64 = in_prog_rows
                    .iter()
                    .filter(|(did, leg, _)| *did == *d && (leg == "return") == is_ret)
                    .count() as i64;
                let visitor_seats: i64 = eff_visitors
                    .values()
                    .chain(added_visitors.iter())
                    .filter(|(did, leg, _)| *did == Some(*d) && (leg == "return") == is_ret)
                    .map(|(_, _, s)| *s)
                    .sum();
                let (o_seats, o_rides) = other_map.get(d).copied().unwrap_or((0, 0));
                let load = o_seats
                    + in_prog_seats
                    + visitor_seats
                    + mine.iter().map(|r| r.seats).sum::<i64>();
                // Without 共乘模式 a driver takes one booking per leg/time slot.
                let one_booking =
                    dr.carpool_enabled == 1 || o_rides + in_prog_count + mine.len() as i64 <= 1;
                if load > cap || !one_booking {
                    return Err(bad(
                        u.lang,
                        ErrorKey::NotEnoughDriverSeatsReassign { load, cap },
                    ));
                }
            }
        }
    }

    // Apply all or nothing; each update re-checks the state the board saw.
    let stamp = now();
    for c in changes.values() {
        let res = match c.driver_id {
            Some(d) => sqlx::query(
                "UPDATE rides SET driver_id=$1, status='pending_driver_confirm', trip_id=NULL, approved_by=$2,
                        needs_attention=1, attention_reason='等待新司机确认', updated_at=$3
                 WHERE id=$4 AND event_id=$5 AND status = ANY($6) AND driver_id IS NOT DISTINCT FROM $7",
            )
            .bind(d)
            .bind(u.id),
            None => sqlx::query(
                "UPDATE rides SET driver_id=NULL, status='pending_driver', trip_id=NULL, approved_by=NULL,
                        needs_attention=0, attention_reason='', updated_at=$1
                 WHERE id=$2 AND event_id=$3 AND status = ANY($4) AND driver_id IS NOT DISTINCT FROM $5",
            ),
        };
        let res = res
            .bind(&stamp)
            .bind(c.ride_id)
            .bind(id)
            .bind(&PLANNABLE[..])
            .bind(c.from_driver_id)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
        }
    }

    // Commit draft visitors and sync the approved draft snapshot if approving a draft.
    let mut visitor_audits: Vec<(&'static str, String)> = Vec::new();
    let approved_draft_info: Option<(i64, Option<i64>, Option<i64>)> = if let Some((
        did,
        _status,
        visitors_raw,
        submitted_by,
        created_by,
        updated_by,
    )) = target_draft
    {
        let draft_visitors: Vec<DraftVisitorEntry> =
            serde_json::from_str(&visitors_raw).unwrap_or_default();
        let visitor_author = submitted_by.or(created_by).unwrap_or(updated_by);

        for dv in &draft_visitors {
            match dv.op.as_str() {
                "add" => {
                    if let Some(d) = dv.driver_id {
                        if !driver.contains_key(&d) {
                            return Err(bad(u.lang, ErrorKey::DriverNotFound));
                        }
                    }
                    let vid: i64 = sqlx::query_scalar(
                        "INSERT INTO plan_visitors (
                                 event_id, name, phone, notes, area_id, pickup_address,
                                 seats, leg, driver_id, created_by, created_at, updated_at
                             )
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
                             RETURNING id",
                    )
                    .bind(id)
                    .bind(dv.name.trim())
                    .bind(dv.phone.trim())
                    .bind(dv.notes.trim())
                    .bind(dv.area_id)
                    .bind(dv.pickup_address.trim())
                    .bind(dv.seats.unwrap_or(1))
                    .bind(dv.leg.as_deref().unwrap_or("outbound"))
                    .bind(dv.driver_id)
                    .bind(visitor_author)
                    .bind(&stamp)
                    .fetch_one(&mut *tx)
                    .await?;
                    visitor_audits.push((
                        "plan.visitor_add",
                        format!(
                            "event {id}: visitor #{vid} ({} seat(s)) via draft #{did}",
                            dv.seats.unwrap_or(1)
                        ),
                    ));
                }
                "update" => {
                    let Some(vid) = dv.live_visitor_id else {
                        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
                    };
                    if let Some(d) = dv.driver_id {
                        if !driver.contains_key(&d) {
                            return Err(bad(u.lang, ErrorKey::DriverNotFound));
                        }
                    }
                    let res = sqlx::query(
                        "UPDATE plan_visitors
                             SET name=$3, phone=$4, notes=$5, area_id=$6, pickup_address=$7,
                                 seats=$8, leg=$9, driver_id=$10, updated_at=$11
                             WHERE id=$1 AND event_id=$2",
                    )
                    .bind(vid)
                    .bind(id)
                    .bind(dv.name.trim())
                    .bind(dv.phone.trim())
                    .bind(dv.notes.trim())
                    .bind(dv.area_id)
                    .bind(dv.pickup_address.trim())
                    .bind(dv.seats.unwrap_or(1))
                    .bind(dv.leg.as_deref().unwrap_or("outbound"))
                    .bind(dv.driver_id)
                    .bind(&stamp)
                    .execute(&mut *tx)
                    .await?;
                    if res.rows_affected() == 0 {
                        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
                    }
                    visitor_audits.push((
                        "plan.visitor_update",
                        format!("event {id}: visitor #{vid} via draft #{did}"),
                    ));
                }
                "delete" => {
                    let Some(vid) = dv.live_visitor_id else {
                        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
                    };
                    let locked_v: Option<(i64,)> = sqlx::query_as(
                        "SELECT id FROM plan_visitors WHERE id = $1 AND event_id = $2 FOR UPDATE",
                    )
                    .bind(vid)
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    if locked_v.is_none() {
                        return Err(conflict(u.lang, ErrorKey::RideStatusChanged));
                    }
                    let live_reimb: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM reimbursements WHERE plan_visitor_id = $1 AND status <> 'rejected')",
                    )
                    .bind(vid)
                    .fetch_one(&mut *tx)
                    .await?;
                    if live_reimb {
                        return Err(conflict(u.lang, ErrorKey::VisitorHasReimbursement));
                    }
                    sqlx::query("DELETE FROM plan_visitors WHERE id = $1 AND event_id = $2")
                        .bind(vid)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                    visitor_audits.push((
                        "plan.visitor_delete",
                        format!("event {id}: visitor #{vid} via draft #{did}"),
                    ));
                }
                _ => {}
            }
        }

        // Store the exact approved live snapshot on the approved draft so reloading never replays stale changes.
        let final_snapshot: Vec<DraftEntry> = rides
            .iter()
            .map(|r| DraftEntry {
                ride_id: r.id,
                driver_id: final_driver(r),
            })
            .collect();
        let final_snapshot_json =
            serde_json::to_string(&final_snapshot).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "UPDATE event_plan_drafts
                 SET status = 'approved',
                     assignments = $1,
                     reviewed_by = $2,
                     reviewed_at = $3,
                     updated_by = $2,
                     updated_at = $3,
                     locked_by = NULL,
                     locked_at = NULL,
                     lock_expires_at = NULL
                 WHERE id = $4 AND event_id = $5",
        )
        .bind(&final_snapshot_json)
        .bind(u.id)
        .bind(&stamp)
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        // Supersede any other open drafts on this event.
        sqlx::query(
                "UPDATE event_plan_drafts
                 SET status = 'superseded',
                     updated_at = $1,
                     locked_by = NULL,
                     locked_at = NULL,
                     lock_expires_at = NULL
                 WHERE event_id = $2 AND id <> $3 AND status IN ('draft', 'pending_approval', 'returned')",
            )
            .bind(&stamp)
            .bind(id)
            .bind(did)
            .execute(&mut *tx)
            .await?;

        Some((did, submitted_by, created_by))
    } else {
        None
    };

    tx.commit().await?;

    for (action, detail) in visitor_audits {
        audit(&st.db, Some(u.id), action, "event", Some(id), detail).await;
    }

    // One summary per driver, after the batch is in.
    let when = ev.starts_at.replace('T', " ");
    let mut gained: BTreeMap<i64, usize> = BTreeMap::new();
    let mut lost: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
    for c in changes.values() {
        let r = by_id[&c.ride_id];
        if let Some(d) = c.driver_id {
            *gained.entry(d).or_default() += 1;
        }
        if let Some(old) = r.driver_id {
            st.routing.invalidate(old, r.trip_id.unwrap_or(r.id)).await;
            lost.entry(old).or_default().push(r.p_name.as_str());
            if let Some(t) = r.trip_id {
                system_message(
                    &st.db,
                    t,
                    &format!("交通组已重新安排 {} 的接送。", r.p_name),
                    &format!("Transport staff have rearranged {}'s ride.", r.p_name),
                )
                .await;
            }
        }
        // A passenger who already had a confirmed driver hears that it's changing.
        if r.status == "confirmed" {
            notify(
                &st.db,
                r.passenger_id,
                (
                    "交通组正在调整您的司机",
                    "Transport staff are changing your driver",
                ),
                (
                    &format!("{} · {}，确认后会再通知您。", ev.title, when),
                    &format!(
                        "{} · {}. We'll let you know once it's confirmed.",
                        ev.title, when
                    ),
                ),
                &format!("/rides/{}", r.id),
            )
            .await;
        }
        audit(
            &st.db,
            Some(u.id),
            "ride.plan_assign",
            "ride",
            Some(r.id),
            format!("event {id}: driver {:?} → {:?}", r.driver_id, c.driver_id),
        )
        .await;
        // Staff may go outside the rider's group; that is recorded, as on a reassign.
        if let (Some(d), Some(g)) = (c.driver_id, r.match_group_id) {
            if !scopes.get(&d).is_some_and(|s| s.contains(&g)) {
                audit(
                    &st.db,
                    Some(u.id),
                    "ride.assign_outside_group",
                    "ride",
                    Some(r.id),
                    format!("driver {d} is outside group {g}"),
                )
                .await;
            }
        }
    }
    // A driver from outside the event's groups learns only that it is a church event.
    let title_for = |sees: bool| -> (String, String) {
        if sees {
            (ev.title.clone(), ev.title.clone())
        } else {
            ("教会活动".to_string(), "Church event".to_string())
        }
    };
    let mut plan_notifs = Vec::with_capacity(gained.len() + lost.len());
    for (d, n) in &gained {
        let (zh, en) = title_for(crate::audience::member_sees_event(&st.db, *d, id).await?);
        plan_notifs.push(crate::notifications::BatchNotifItem {
            user_id: *d,
            title_zh: format!("交通组为您安排了 {n} 位乘客"),
            title_en: format!(
                "Transport staff arranged {n} passenger{} for you",
                if *n == 1 { "" } else { "s" }
            ),
            body_zh: format!("{zh} · {when} · 请在「开车」确认"),
            body_en: format!("{en} · {when} · please confirm in Driving"),
            link: "/drive".into(),
            meta: crate::notifications::NotifMeta {
                category: "ride",
                event_type: "plan_assigned".into(),
                priority: "normal",
                entity_type: "event",
                entity_id: Some(id),
                tag: format!("event-{id}-drive"),
            },
        });
    }
    for (d, names) in &lost {
        let (zh, en) = title_for(crate::audience::member_sees_event(&st.db, *d, id).await?);
        plan_notifs.push(crate::notifications::BatchNotifItem {
            user_id: *d,
            title_zh: "接送已改派".into(),
            title_en: "Ride reassigned".into(),
            body_zh: format!("{zh} · {}", names.join(", ")),
            body_en: format!("{en} · {}", names.join(", ")),
            link: "/drive".into(),
            meta: crate::notifications::NotifMeta {
                category: "ride",
                event_type: "plan_reassigned".into(),
                priority: "urgent",
                entity_type: "event",
                entity_id: Some(id),
                tag: format!("event-{id}-drive"),
            },
        });
    }
    crate::notifications::notify_batch(&st.db, plan_notifs).await;

    if let Some((did, submitted_by, created_by)) = approved_draft_info {
        audit(
            &st.db,
            Some(u.id),
            "plan.approve_draft",
            "event",
            Some(id),
            format!(
                "event {id}: approved draft #{did} ({} changes)",
                changes.len()
            ),
        )
        .await;
        let mut notify_targets = Vec::new();
        if let Some(sub) = submitted_by {
            if sub != u.id {
                notify_targets.push(sub);
            }
        }
        if let Some(creator) = created_by {
            if creator != u.id && !notify_targets.contains(&creator) {
                notify_targets.push(creator);
            }
        }
        if !notify_targets.is_empty() {
            crate::util::notify_many(
                &st.db,
                &notify_targets,
                ("接送安排已审批通过", "Transport arrangement approved"),
                (
                    &format!(
                        "{} 审批通过了「{}」的接送安排，已通知司机确认。",
                        u.name, ev.title
                    ),
                    &format!(
                        "{} approved the transport arrangement for \"{}\", drivers notified.",
                        u.name, ev.title
                    ),
                ),
                &format!("/admin/events/{}/plan?draft={}", id, did),
            )
            .await;
        }
    }

    Ok(Json(json!({
        "ok": true,
        "changed": changes.len(),
        "approved_draft_id": approved_draft_info.map(|(did, _, _)| did),
    })))
}
