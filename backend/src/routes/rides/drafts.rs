//! Single-ride change drafts: shared across traffic group members, single-editor
//! lease-locked (10 minutes), and approved/returned only by the traffic lead.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, locked, not_found, AppResult};
use crate::perms::{self, Act, Page, Perm};
use crate::state::AppState;
use crate::util::{audit, in_minutes, notify_many, now, require, ErrorKey};

use super::staff::{
    exec_set_reimbursement_excluded_after, exec_set_reimbursement_excluded_tx,
    exec_staff_alt_transport_after, exec_staff_alt_transport_tx, exec_staff_approve_after,
    exec_staff_approve_tx, exec_staff_reassign_after, exec_staff_reassign_tx,
    exec_staff_return_after, exec_staff_return_tx,
};
use super::{load_ride, traffic_owned_events_sql, traffic_scope_ok};

const DRAFT_LOCK_MINUTES: i64 = 10;
const REASON_MAX: usize = 500;

#[derive(sqlx::FromRow)]
struct RideDraftRow {
    id: i64,
    ride_id: i64,
    version: i64,
    action: String,
    driver_id: Option<i64>,
    driver_name: Option<String>,
    reason: String,
    reimbursement_excluded: Option<i64>,
    status: String,
    created_by: Option<i64>,
    created_by_name: Option<String>,
    updated_by: Option<i64>,
    updated_by_name: Option<String>,
    locked_by: Option<i64>,
    locked_by_name: Option<String>,
    locked_at: Option<String>,
    lock_expires_at: Option<String>,
    submitted_by: Option<i64>,
    submitted_by_name: Option<String>,
    submitted_at: Option<String>,
    reviewed_by: Option<i64>,
    reviewed_by_name: Option<String>,
    reviewed_at: Option<String>,
    review_notes: String,
    created_at: String,
    updated_at: String,
    // Ride summary context for the Traffic Hub queue
    ride_status: String,
    ride_pickup_at: String,
    passenger_name: String,
    area_name: String,
    current_driver_name: Option<String>,
    event_id: Option<i64>,
    event_title: Option<String>,
}

const RIDE_DRAFT_SELECT: &str = "SELECT d.id, d.ride_id, d.version, d.action, d.driver_id, drv.name AS driver_name,
        d.reason, d.reimbursement_excluded, d.status,
        d.created_by, u0.name AS created_by_name,
        d.updated_by, u1.name AS updated_by_name,
        d.locked_by, u4.name AS locked_by_name, d.locked_at, d.lock_expires_at,
        d.submitted_by, u2.name AS submitted_by_name, d.submitted_at,
        d.reviewed_by, u3.name AS reviewed_by_name, d.reviewed_at,
        d.review_notes, d.created_at, d.updated_at,
        r.status AS ride_status, r.pickup_at AS ride_pickup_at,
        CASE WHEN r.is_proxy = 1 AND r.rider_name != '' THEN r.rider_name || ' (' || p.name || ' 代订)' ELSE p.name END AS passenger_name,
        a.name AS area_name, cur_drv.name AS current_driver_name,
        r.event_id, e.title AS event_title
 FROM ride_change_drafts d
 JOIN rides r ON r.id = d.ride_id
 JOIN users p ON p.id = r.passenger_id
 JOIN areas a ON a.id = r.pickup_area_id
 LEFT JOIN events e ON e.id = r.event_id
 LEFT JOIN users drv ON drv.id = d.driver_id
 LEFT JOIN users cur_drv ON cur_drv.id = r.driver_id
 LEFT JOIN users u0 ON u0.id = d.created_by
 LEFT JOIN users u1 ON u1.id = d.updated_by
 LEFT JOIN users u2 ON u2.id = d.submitted_by
 LEFT JOIN users u3 ON u3.id = d.reviewed_by
 LEFT JOIN users u4 ON u4.id = d.locked_by";

fn is_lock_active(locked_by: Option<i64>, lock_expires_at: Option<&str>, now_str: &str) -> bool {
    locked_by.is_some() && lock_expires_at.is_some_and(|exp| exp > now_str)
}

fn ride_draft_json(d: &RideDraftRow, viewer_id: i64, now_str: &str) -> Value {
    let lock_active = is_lock_active(d.locked_by, d.lock_expires_at.as_deref(), now_str);
    json!({
        "id": d.id,
        "ride_id": d.ride_id,
        "version": d.version,
        "action": d.action,
        "driver_id": d.driver_id,
        "driver_name": d.driver_name,
        "reason": d.reason,
        "reimbursement_excluded": d.reimbursement_excluded.map(|v| v == 1),
        "status": d.status,
        "created_by": d.created_by.map(|id| json!({ "id": id, "name": d.created_by_name })),
        "updated_by": d.updated_by.map(|id| json!({ "id": id, "name": d.updated_by_name })),
        "submitted_by": d.submitted_by.map(|id| json!({ "id": id, "name": d.submitted_by_name })),
        "submitted_at": d.submitted_at,
        "reviewed_by": d.reviewed_by.map(|id| json!({ "id": id, "name": d.reviewed_by_name })),
        "reviewed_at": d.reviewed_at,
        "review_notes": d.review_notes,
        "created_at": d.created_at,
        "updated_at": d.updated_at,
        "locked_by": if lock_active {
            d.locked_by.map(|id| json!({ "id": id, "name": d.locked_by_name }))
        } else {
            None
        },
        "locked_at": if lock_active { d.locked_at.clone() } else { None },
        "lock_expires_at": if lock_active { d.lock_expires_at.clone() } else { None },
        "lock_is_mine": lock_active && d.locked_by == Some(viewer_id),
        "ride": {
            "id": d.ride_id,
            "status": d.ride_status,
            "pickup_at": d.ride_pickup_at,
            "passenger_name": d.passenger_name,
            "area_name": d.area_name,
            "current_driver_name": d.current_driver_name,
            "event_id": d.event_id,
            "event_title": d.event_title,
        },
    })
}

fn can_create_or_edit_draft(u: &CurrentUser) -> bool {
    u.can(Page::AdminRides, Act::View)
        && (u.can(Page::AdminPlan, Act::Update) || u.can(Page::AdminRides, Act::Update))
}

fn assert_ride_draft_editable(u: &CurrentUser, status: &str) -> AppResult<()> {
    match status {
        "draft" | "returned" => Ok(()),
        "pending_approval" if u.is_traffic_lead() => Ok(()),
        _ => Err(conflict(u.lang, ErrorKey::DraftNotEditable)),
    }
}

fn assert_not_locked_by_other(
    u: &CurrentUser,
    locked_by: Option<i64>,
    locked_by_name: Option<&str>,
    lock_expires_at: Option<&str>,
    now_str: &str,
) -> AppResult<()> {
    if is_lock_active(locked_by, lock_expires_at, now_str) && locked_by != Some(u.id) {
        return Err(locked(
            u.lang,
            ErrorKey::DraftLockedByOther {
                name: locked_by_name.unwrap_or("Someone").to_string(),
            },
        ));
    }
    Ok(())
}

async fn load_ride_draft_by_id(
    st: &AppState,
    ride_id: i64,
    draft_id: i64,
) -> AppResult<Option<RideDraftRow>> {
    Ok(sqlx::query_as(crate::dyn_sql!(
        "{RIDE_DRAFT_SELECT} WHERE d.ride_id = $1 AND d.id = $2"
    ))
    .bind(ride_id)
    .bind(draft_id)
    .fetch_optional(&st.db)
    .await?)
}

async fn validate_ride_draft_payload(
    st: &AppState,
    u: &CurrentUser,
    passenger_id: i64,
    action: &str,
    driver_id: Option<i64>,
    reason: &str,
    reimbursement_excluded: Option<bool>,
) -> AppResult<(String, Option<i64>, String, Option<i64>)> {
    let act = action.trim().to_string();
    if !matches!(
        act.as_str(),
        "approve" | "reassign" | "return" | "alt_transport" | "reimbursement_excluded"
    ) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let clean_reason = reason.trim().to_string();
    if clean_reason.chars().count() > REASON_MAX {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let clean_driver = match act.as_str() {
        "reassign" => {
            let did = driver_id.ok_or_else(|| bad(u.lang, ErrorKey::DriverNotFound))?;
            let (can_drive,): (bool,) = sqlx::query_as(crate::dyn_sql!(
                "SELECT {} FROM users u WHERE u.id = $1 AND u.status = 'active'",
                perms::sql_holds("u", Perm(Page::Drive, Act::Create))
            ))
            .bind(did)
            .fetch_optional(&st.db)
            .await?
            .ok_or_else(|| bad(u.lang, ErrorKey::DriverNotFound))?;
            if !can_drive {
                return Err(bad(u.lang, ErrorKey::MemberNotVolunteerDriver));
            }
            if did == passenger_id {
                return Err(bad(u.lang, ErrorKey::DriverCannotBePassenger));
            }
            Some(did)
        }
        _ => None,
    };
    let clean_excl = if act == "reimbursement_excluded" {
        let ex = reimbursement_excluded.ok_or_else(|| bad(u.lang, ErrorKey::InvalidAction))?;
        Some(if ex { 1 } else { 0 })
    } else {
        None
    };
    Ok((act, clean_driver, clean_reason, clean_excl))
}

async fn notify_traffic_leads_of_ride_draft(
    st: &AppState,
    u: &CurrentUser,
    ride_id: i64,
    passenger_name: &str,
) {
    let mut lead_rows: Vec<(i64,)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT id FROM users WHERE {}",
        perms::sql_has_role("users", perms::TRAFFIC_HEAD_ROLE)
    ))
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();

    if lead_rows.is_empty() {
        lead_rows = sqlx::query_as(crate::dyn_sql!(
            "SELECT id FROM users WHERE {}",
            perms::sql_has_any_role("users", &[perms::ADMIN_ROLE, perms::SUPERADMIN_ROLE])
        ))
        .fetch_all(&st.db)
        .await
        .unwrap_or_default();
    }

    let lead_ids: Vec<i64> = lead_rows
        .into_iter()
        .map(|(lid,)| lid)
        .filter(|&lid| lid != u.id)
        .collect();

    notify_many(
        &st.db,
        &lead_ids,
        ("接送修改草稿待审批", "Ride change draft awaiting approval"),
        (
            &format!(
                "{} 提交了 {} 的接送修改草稿，等待您审批。",
                u.name, passenger_name
            ),
            &format!(
                "{} submitted a ride change draft for {} for approval.",
                u.name, passenger_name
            ),
        ),
        &format!("/admin/rides/{ride_id}"),
    )
    .await;
}

#[derive(Deserialize)]
pub(super) struct AllRideDraftsQuery {
    status: Option<String>,
    ride_id: Option<i64>,
}

pub(super) async fn list_all_ride_drafts(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<AllRideDraftsQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::View)?;
    let mut conds = vec!["d.status <> 'superseded'".to_string()];
    if !u.traffic_is_global() {
        conds.push(traffic_owned_events_sql(&u));
    }
    let mut n = 1;
    let status = q.status.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if status.is_some() {
        conds.push(format!("d.status = ${n}"));
        n += 1;
    }
    if q.ride_id.is_some() {
        conds.push(format!("d.ride_id = ${n}"));
    }

    let sql = format!(
        "{RIDE_DRAFT_SELECT} WHERE {}
         ORDER BY CASE d.status
             WHEN 'pending_approval' THEN 0
             WHEN 'returned' THEN 1
             WHEN 'draft' THEN 2
             WHEN 'approved' THEN 3
             ELSE 4 END,
             d.updated_at DESC, d.id DESC
         LIMIT 100",
        conds.join(" AND ")
    );
    let mut query = sqlx::query_as::<_, RideDraftRow>(sqlx::AssertSqlSafe(&*sql));
    if let Some(s) = status {
        query = query.bind(s);
    }
    if let Some(rid) = q.ride_id {
        query = query.bind(rid);
    }
    let rows = query.fetch_all(&st.db).await?;
    let now_str = now();
    Ok(Json(json!(rows
        .iter()
        .map(|d| ride_draft_json(d, u.id, &now_str))
        .collect::<Vec<_>>())))
}

pub(super) async fn list_ride_drafts(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::View)?;
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let rows: Vec<RideDraftRow> = sqlx::query_as(crate::dyn_sql!(
        "{RIDE_DRAFT_SELECT} WHERE d.ride_id = $1 ORDER BY d.version DESC, d.id DESC"
    ))
    .bind(id)
    .fetch_all(&st.db)
    .await?;
    let now_str = now();
    Ok(Json(json!(rows
        .iter()
        .map(|d| ride_draft_json(d, u.id, &now_str))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
pub(super) struct RideDraftIn {
    action: String,
    driver_id: Option<i64>,
    #[serde(default)]
    reason: String,
    reimbursement_excluded: Option<bool>,
    submit: Option<bool>,
}

pub(super) async fn create_ride_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<RideDraftIn>,
) -> AppResult<Json<Value>> {
    if !can_create_or_edit_draft(&u) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let (action, driver_id, reason, excl) = validate_ride_draft_payload(
        &st,
        &u,
        r.passenger_id,
        &req.action,
        req.driver_id,
        &req.reason,
        req.reimbursement_excluded,
    )
    .await?;

    let submit_now = req.submit.unwrap_or(false);
    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_ride_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let (max_ver,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(version), 0)::BIGINT FROM ride_change_drafts WHERE ride_id = $1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let next_ver = max_ver + 1;

    let (status, locked_by, locked_at, lock_expires_at, submitted_by, submitted_at) = if submit_now
    {
        (
            "pending_approval",
            None,
            None,
            None,
            Some(u.id),
            Some(stamp.clone()),
        )
    } else {
        (
            "draft",
            Some(u.id),
            Some(stamp.clone()),
            Some(expires),
            None,
            None,
        )
    };

    let (draft_id,): (i64,) = sqlx::query_as(
        "INSERT INTO ride_change_drafts (
             ride_id, version, action, driver_id, reason, reimbursement_excluded,
             status, created_by, updated_by,
             locked_by, locked_at, lock_expires_at,
             submitted_by, submitted_at, created_at, updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $9, $10, $11, $12, $13, $14, $14)
         RETURNING id",
    )
    .bind(id)
    .bind(next_ver)
    .bind(&action)
    .bind(driver_id)
    .bind(&reason)
    .bind(excl)
    .bind(status)
    .bind(u.id)
    .bind(locked_by)
    .bind(locked_at)
    .bind(lock_expires_at)
    .bind(submitted_by)
    .bind(submitted_at)
    .bind(&stamp)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        if submit_now {
            "ride.request_draft_approval"
        } else {
            "ride.create_draft"
        },
        "ride",
        Some(id),
        format!("ride {id}: draft #{draft_id} v{next_ver} action={action}"),
    )
    .await;

    if submit_now {
        notify_traffic_leads_of_ride_draft(&st, &u, id, &r.p_name).await;
    }

    let row = load_ride_draft_by_id(&st, id, draft_id)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": ride_draft_json(&row, u.id, &stamp),
    })))
}

pub(super) async fn update_ride_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
    Json(req): Json<RideDraftIn>,
) -> AppResult<Json<Value>> {
    if !can_create_or_edit_draft(&u) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let (action, driver_id, reason, excl) = validate_ride_draft_payload(
        &st,
        &u,
        r.passenger_id,
        &req.action,
        req.driver_id,
        &req.reason,
        req.reimbursement_excluded,
    )
    .await?;

    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);
    let mut tx = st.db.begin().await?;
    let row: Option<(String, Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.status, d.locked_by, u.name, d.lock_expires_at
         FROM ride_change_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.ride_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((cur_status, locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::RideDraftNotFound));
    };
    assert_ride_draft_editable(&u, &cur_status)?;
    assert_not_locked_by_other(
        &u,
        locked_by,
        locked_by_name.as_deref(),
        lock_expires_at.as_deref(),
        &stamp,
    )?;

    sqlx::query(
        "UPDATE ride_change_drafts
         SET action = $1, driver_id = $2, reason = $3, reimbursement_excluded = $4,
             updated_by = $5, updated_at = $6,
             locked_by = $5, locked_at = $6, lock_expires_at = $7
         WHERE id = $8 AND ride_id = $9",
    )
    .bind(&action)
    .bind(driver_id)
    .bind(&reason)
    .bind(excl)
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let updated = load_ride_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": ride_draft_json(&updated, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
pub(super) struct LockReq {
    force: Option<bool>,
}

pub(super) async fn acquire_ride_draft_lock(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
    body: Option<Json<LockReq>>,
) -> AppResult<Json<Value>> {
    if !can_create_or_edit_draft(&u) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let force = body.and_then(|b| b.0.force).unwrap_or(false);
    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);
    let mut tx = st.db.begin().await?;
    let row: Option<(String, Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.status, d.locked_by, u.name, d.lock_expires_at
         FROM ride_change_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.ride_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::RideDraftNotFound));
    };
    assert_ride_draft_editable(&u, &status)?;
    if !(force && u.is_traffic_lead()) {
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
    }

    sqlx::query(
        "UPDATE ride_change_drafts
         SET locked_by = $1, locked_at = $2, lock_expires_at = $3
         WHERE id = $4 AND ride_id = $5",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let updated = load_ride_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": ride_draft_json(&updated, u.id, &stamp),
    })))
}

pub(super) async fn release_ride_draft_lock(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    if !can_create_or_edit_draft(&u) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let stamp = now();
    let mut tx = st.db.begin().await?;
    let row: Option<(Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.locked_by, u.name, d.lock_expires_at
         FROM ride_change_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.ride_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::RideDraftNotFound));
    };
    if !u.is_traffic_lead() {
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
    }

    sqlx::query(
        "UPDATE ride_change_drafts
         SET locked_by = NULL, locked_at = NULL, lock_expires_at = NULL
         WHERE id = $1 AND ride_id = $2",
    )
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let updated = load_ride_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": ride_draft_json(&updated, u.id, &stamp),
    })))
}

pub(super) async fn request_ride_draft_approval(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    if !can_create_or_edit_draft(&u) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let stamp = now();
    let mut tx = st.db.begin().await?;
    let row: Option<(String, Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.status, d.locked_by, u.name, d.lock_expires_at
         FROM ride_change_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.ride_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::RideDraftNotFound));
    };
    if !matches!(status.as_str(), "draft" | "returned") {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }
    assert_not_locked_by_other(
        &u,
        locked_by,
        locked_by_name.as_deref(),
        lock_expires_at.as_deref(),
        &stamp,
    )?;

    sqlx::query(
        "UPDATE ride_change_drafts
         SET status = 'pending_approval',
             submitted_by = $1,
             submitted_at = $2,
             updated_by = $1,
             updated_at = $2,
             locked_by = NULL,
             locked_at = NULL,
             lock_expires_at = NULL
         WHERE id = $3 AND ride_id = $4",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    notify_traffic_leads_of_ride_draft(&st, &u, id, &r.p_name).await;
    audit(
        &st.db,
        Some(u.id),
        "ride.request_draft_approval",
        "ride",
        Some(id),
        format!("ride {id}: submitted draft #{did}"),
    )
    .await;

    let updated = load_ride_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": ride_draft_json(&updated, u.id, &stamp),
    })))
}

pub(super) async fn approve_ride_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    if !u.is_traffic_lead() {
        return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }

    let stamp = now();
    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_ride_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let draft_info: Option<(
        String,
        Option<i64>,
        String,
        Option<i64>,
        String,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT d.action, d.driver_id, d.reason, d.reimbursement_excluded, d.status,
                d.submitted_by, d.created_by, d.locked_by, u.name, d.lock_expires_at
         FROM ride_change_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.ride_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((
        action,
        driver_id,
        reason,
        excl,
        status,
        submitted_by,
        created_by,
        locked_by,
        locked_by_name,
        lock_expires_at,
    )) = draft_info
    else {
        return Err(bad(u.lang, ErrorKey::RideDraftNotFound));
    };
    if !matches!(status.as_str(), "draft" | "pending_approval" | "returned") {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }
    assert_not_locked_by_other(
        &u,
        locked_by,
        locked_by_name.as_deref(),
        lock_expires_at.as_deref(),
        &stamp,
    )?;

    enum PostApply {
        Approve(i64),
        Reassign(i64, Option<i64>),
        Return,
        AltTransport(String),
        ReimbExcluded(bool),
    }

    let post_apply = match action.as_str() {
        "approve" => {
            let approved_id = exec_staff_approve_tx(&mut tx, &u, &r).await?;
            PostApply::Approve(approved_id)
        }
        "reassign" => {
            let drv = driver_id.ok_or_else(|| bad(u.lang, ErrorKey::DriverNotFound))?;
            let outside = exec_staff_reassign_tx(&mut tx, &u, &r, drv).await?;
            PostApply::Reassign(drv, outside)
        }
        "return" => {
            exec_staff_return_tx(&mut tx, &u, &r, &reason).await?;
            PostApply::Return
        }
        "alt_transport" => {
            let note = exec_staff_alt_transport_tx(
                &mut tx,
                &u,
                &r,
                if reason.is_empty() {
                    None
                } else {
                    Some(&reason)
                },
            )
            .await?;
            PostApply::AltTransport(note)
        }
        "reimbursement_excluded" => {
            let ex = excl.unwrap_or(1) == 1;
            exec_set_reimbursement_excluded_tx(&mut tx, u.lang, &r, ex).await?;
            PostApply::ReimbExcluded(ex)
        }
        _ => return Err(bad(u.lang, ErrorKey::InvalidAction)),
    };

    let updated_rows = sqlx::query(
        "UPDATE ride_change_drafts
         SET status = 'approved',
             reviewed_by = $1,
             reviewed_at = $2,
             updated_by = $1,
             updated_at = $2,
             locked_by = NULL,
             locked_at = NULL,
             lock_expires_at = NULL
         WHERE id = $3 AND ride_id = $4 AND status IN ('draft', 'pending_approval', 'returned')",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if updated_rows == 0 {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }

    sqlx::query(
        "UPDATE ride_change_drafts
         SET status = 'superseded',
             updated_at = $1,
             locked_by = NULL,
             locked_at = NULL,
             lock_expires_at = NULL
         WHERE ride_id = $2 AND id <> $3 AND status IN ('draft', 'pending_approval', 'returned')",
    )
    .bind(&stamp)
    .bind(id)
    .bind(did)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    match post_apply {
        PostApply::Approve(approved_id) => exec_staff_approve_after(&st, &u, approved_id).await?,
        PostApply::Reassign(drv, outside) => {
            exec_staff_reassign_after(&st, &u, &r, drv, outside).await
        }
        PostApply::Return => exec_staff_return_after(&st, &u, &r, &reason).await,
        PostApply::AltTransport(note) => exec_staff_alt_transport_after(&st, &u, &r, note).await,
        PostApply::ReimbExcluded(ex) => {
            exec_set_reimbursement_excluded_after(&st, &u, &r, ex).await
        }
    }

    let mut targets = Vec::new();
    if let Some(sub) = submitted_by {
        if sub != u.id {
            targets.push(sub);
        }
    }
    if let Some(creator) = created_by {
        if creator != u.id && !targets.contains(&creator) {
            targets.push(creator);
        }
    }
    if !targets.is_empty() {
        notify_many(
            &st.db,
            &targets,
            ("接送修改草稿已审批", "Ride change draft approved"),
            (
                &format!("{} 已审批通过 {} 的接送修改草稿。", u.name, r.p_name),
                &format!(
                    "{} approved the ride change draft for {}.",
                    u.name, r.p_name
                ),
            ),
            &format!("/admin/rides/{id}"),
        )
        .await;
    }

    audit(
        &st.db,
        Some(u.id),
        "ride.approve_draft",
        "ride",
        Some(id),
        format!("ride {id}: approved draft #{did} ({action})"),
    )
    .await;

    let updated = load_ride_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": ride_draft_json(&updated, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
pub(super) struct ReturnRideDraftReq {
    notes: Option<String>,
}

pub(super) async fn return_ride_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
    Json(req): Json<ReturnRideDraftReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminRides, Act::Update)?;
    if !u.is_traffic_lead() {
        return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let notes = req.notes.unwrap_or_default().trim().to_string();
    let stamp = now();

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_ride_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let prev: Option<(String, Option<i64>, Option<i64>)> = sqlx::query_as(
        "SELECT status, submitted_by, created_by
         FROM ride_change_drafts
         WHERE id = $1 AND ride_id = $2
         FOR UPDATE",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, submitted_by, created_by)) = prev else {
        return Err(bad(u.lang, ErrorKey::RideDraftNotFound));
    };
    if !matches!(status.as_str(), "pending_approval" | "draft" | "returned") {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }

    let returned_rows = sqlx::query(
        "UPDATE ride_change_drafts
         SET status = 'returned',
             reviewed_by = $1,
             reviewed_at = $2,
             review_notes = $3,
             updated_at = $2,
             locked_by = NULL,
             locked_at = NULL,
             lock_expires_at = NULL
         WHERE id = $4 AND ride_id = $5 AND status IN ('pending_approval', 'draft', 'returned')",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(&notes)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if returned_rows == 0 {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }
    tx.commit().await?;

    let mut targets = Vec::new();
    if let Some(sub) = submitted_by {
        if sub != u.id {
            targets.push(sub);
        }
    }
    if let Some(creator) = created_by {
        if creator != u.id && !targets.contains(&creator) {
            targets.push(creator);
        }
    }
    if !targets.is_empty() {
        let suffix_zh = if notes.is_empty() {
            String::new()
        } else {
            format!("：{notes}")
        };
        let suffix_en = if notes.is_empty() {
            String::new()
        } else {
            format!(": {notes}")
        };
        notify_many(
            &st.db,
            &targets,
            ("接送修改草稿已退回", "Ride change draft returned"),
            (
                &format!("交通组组长退回了 {} 的接送修改草稿{suffix_zh}", r.p_name),
                &format!(
                    "Traffic lead returned the ride change draft for {}{suffix_en}",
                    r.p_name
                ),
            ),
            &format!("/admin/rides/{id}"),
        )
        .await;
    }

    audit(
        &st.db,
        Some(u.id),
        "ride.return_draft",
        "ride",
        Some(id),
        format!("ride {id}: returned draft #{did} with notes: {notes}"),
    )
    .await;

    let updated = load_ride_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": ride_draft_json(&updated, u.id, &stamp),
    })))
}

pub(super) async fn delete_ride_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    if !can_create_or_edit_draft(&u) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let r = load_ride(&st, u.lang, id).await?;
    if !traffic_scope_ok(&u, r.event_created_by) {
        return Err(forbidden(u.lang, ErrorKey::CannotViewRide));
    }
    let stamp = now();
    let mut tx = st.db.begin().await?;
    let row: Option<(
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT d.status, d.created_by, d.locked_by, u.name, d.lock_expires_at
             FROM ride_change_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.ride_id = $2
             FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, created_by, locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::RideDraftNotFound));
    };
    if status == "approved" {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }
    if created_by != Some(u.id) && !u.is_traffic_lead() {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    if !u.is_traffic_lead() {
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
    }

    sqlx::query("DELETE FROM ride_change_drafts WHERE id = $1 AND ride_id = $2")
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "ride.delete_draft",
        "ride",
        Some(id),
        format!("ride {id}: deleted draft #{did}"),
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}
