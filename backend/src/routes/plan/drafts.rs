//! Transport planning draft management, single-editor locks, and approval workflow.

use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, conflict, forbidden, locked, not_found, AppResult};
use crate::perms::{self, Act, Page, Perm};
use crate::routes::plan_visitors::{validate_fields as validate_visitor_fields, VisitorIn};
use crate::state::AppState;
use crate::util::{audit, in_minutes, now, require, ErrorKey};

use super::{load_event, DraftEntry};

const DRAFT_LOCK_MINUTES: i64 = 10;
const LABEL_MAX: usize = 60;
const MAX_DRAFT_VISITORS: usize = 50;

#[derive(sqlx::FromRow)]
pub(super) struct DraftRow {
    pub(super) id: i64,
    pub(super) event_id: i64,
    pub(super) version: i64,
    pub(super) label: String,
    pub(super) status: String,
    pub(super) assignments: String,
    pub(super) visitors: String,
    pub(super) created_by: Option<i64>,
    pub(super) created_by_name: Option<String>,
    pub(super) updated_by: Option<i64>,
    pub(super) updated_by_name: Option<String>,
    pub(super) submitted_by: Option<i64>,
    pub(super) submitted_by_name: Option<String>,
    pub(super) submitted_at: Option<String>,
    pub(super) reviewed_by: Option<i64>,
    pub(super) reviewed_by_name: Option<String>,
    pub(super) reviewed_at: Option<String>,
    pub(super) review_notes: String,
    pub(super) locked_by: Option<i64>,
    pub(super) locked_by_name: Option<String>,
    pub(super) locked_at: Option<String>,
    pub(super) lock_expires_at: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
}

pub(super) const DRAFT_SELECT: &str =
    "SELECT d.id, d.event_id, d.version, d.label, d.status, d.assignments, d.visitors,
        d.created_by, u0.name AS created_by_name,
        d.updated_by, u1.name AS updated_by_name,
        d.submitted_by, u2.name AS submitted_by_name, d.submitted_at,
        d.reviewed_by, u3.name AS reviewed_by_name, d.reviewed_at,
        d.review_notes,
        d.locked_by, u4.name AS locked_by_name, d.locked_at, d.lock_expires_at,
        d.created_at, d.updated_at
 FROM event_plan_drafts d
 LEFT JOIN users u1 ON u1.id = d.updated_by
 LEFT JOIN users u0 ON u0.id = d.created_by
 LEFT JOIN users u2 ON u2.id = d.submitted_by
 LEFT JOIN users u3 ON u3.id = d.reviewed_by
 LEFT JOIN users u4 ON u4.id = d.locked_by";

pub(super) fn is_lock_active(
    locked_by: Option<i64>,
    lock_expires_at: Option<&str>,
    now_str: &str,
) -> bool {
    locked_by.is_some() && lock_expires_at.is_some_and(|exp| exp > now_str)
}

pub(super) fn draft_to_json(d: &DraftRow, viewer_id: i64, now_str: &str) -> Value {
    let parsed_assignments: Value =
        serde_json::from_str(&d.assignments).unwrap_or_else(|_| json!([]));
    let parsed_visitors: Value = serde_json::from_str(&d.visitors).unwrap_or_else(|_| json!([]));
    let lock_active = is_lock_active(d.locked_by, d.lock_expires_at.as_deref(), now_str);
    let created_by = d
        .created_by
        .map(|cid| {
            json!({
                "id": cid,
                "name": d.created_by_name.clone().or_else(|| d.updated_by_name.clone()),
            })
        })
        .or_else(|| {
            d.updated_by
                .map(|uid| json!({ "id": uid, "name": d.updated_by_name }))
        });
    json!({
        "id": d.id,
        "event_id": d.event_id,
        "version": d.version,
        "label": d.label,
        "status": d.status,
        "assignments": parsed_assignments,
        "visitors": parsed_visitors,
        "created_by": created_by,
        "updated_by": d.updated_by.map(|uid| json!({ "id": uid, "name": d.updated_by_name })),
        "created_at": d.created_at,
        "updated_at": d.updated_at,
        "submitted_by": d.submitted_by.map(|sid| json!({ "id": sid, "name": d.submitted_by_name })),
        "submitted_at": d.submitted_at,
        "reviewed_by": d.reviewed_by.map(|rid| json!({ "id": rid, "name": d.reviewed_by_name })),
        "reviewed_at": d.reviewed_at,
        "review_notes": d.review_notes,
        "locked_by": if lock_active {
            d.locked_by.map(|lid| json!({ "id": lid, "name": d.locked_by_name }))
        } else {
            None
        },
        "locked_at": if lock_active { d.locked_at.clone() } else { None },
        "lock_expires_at": if lock_active { d.lock_expires_at.clone() } else { None },
        "lock_is_mine": lock_active && d.locked_by == Some(viewer_id),
    })
}

pub(super) async fn load_event_drafts(st: &AppState, event_id: i64) -> AppResult<Vec<DraftRow>> {
    Ok(sqlx::query_as(crate::dyn_sql!(
        "{DRAFT_SELECT} WHERE d.event_id = $1 ORDER BY d.version DESC, d.id DESC"
    ))
    .bind(event_id)
    .fetch_all(&st.db)
    .await?)
}

async fn load_draft_by_id(
    st: &AppState,
    event_id: i64,
    draft_id: i64,
) -> AppResult<Option<DraftRow>> {
    Ok(sqlx::query_as(crate::dyn_sql!(
        "{DRAFT_SELECT} WHERE d.event_id = $1 AND d.id = $2"
    ))
    .bind(event_id)
    .bind(draft_id)
    .fetch_optional(&st.db)
    .await?)
}

fn assert_draft_editable(u: &CurrentUser, status: &str) -> AppResult<()> {
    match status {
        "draft" | "returned" => Ok(()),
        "pending_approval" if u.is_traffic_lead() => Ok(()),
        _ => Err(conflict(u.lang, ErrorKey::DraftNotEditable)),
    }
}

pub(super) fn assert_not_locked_by_other(
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

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct DraftVisitorEntry {
    #[serde(default)]
    pub temp_id: String,
    #[serde(default = "default_visitor_op")]
    pub op: String, // "add" | "update" | "delete"
    pub live_visitor_id: Option<i64>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub notes: String,
    pub area_id: Option<i64>,
    #[serde(default)]
    pub area_name: Option<String>,
    #[serde(default)]
    pub pickup_address: String,
    pub seats: Option<i64>,
    pub leg: Option<String>,
    pub driver_id: Option<i64>,
    #[serde(default)]
    pub driver_name: Option<String>,
}

fn default_visitor_op() -> String {
    "add".to_string()
}

fn clean_label(u: &CurrentUser, raw: Option<&str>) -> AppResult<String> {
    let s = raw.unwrap_or_default().trim();
    if s.chars().count() > LABEL_MAX {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    Ok(s.to_string())
}

async fn clean_draft_visitors(
    st: &AppState,
    u: &CurrentUser,
    event_id: i64,
    entries: &[DraftVisitorEntry],
) -> AppResult<Vec<DraftVisitorEntry>> {
    if entries.len() > MAX_DRAFT_VISITORS {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let mut check_live_ids = Vec::new();
    let mut delete_ids = Vec::new();
    let mut area_ids = Vec::new();
    let mut driver_ids = Vec::new();
    for v in entries {
        let op = v.op.trim();
        if !matches!(op, "add" | "update" | "delete") {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        if op == "update" || op == "delete" {
            let Some(vid) = v.live_visitor_id else {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            };
            check_live_ids.push(vid);
            if op == "delete" {
                delete_ids.push(vid);
                continue;
            }
        }
        if let Some(aid) = v.area_id {
            area_ids.push(aid);
        }
        if let Some(did) = v.driver_id {
            driver_ids.push(did);
        }
    }

    let valid_live_ids: HashSet<i64> = if check_live_ids.is_empty() {
        HashSet::new()
    } else {
        sqlx::query_scalar("SELECT id FROM plan_visitors WHERE event_id = $1 AND id = ANY($2)")
            .bind(event_id)
            .bind(&check_live_ids)
            .fetch_all(&st.db)
            .await?
            .into_iter()
            .collect()
    };
    let live_reimb_ids: HashSet<i64> = if delete_ids.is_empty() {
        HashSet::new()
    } else {
        sqlx::query_scalar(
            "SELECT DISTINCT plan_visitor_id FROM reimbursements
             WHERE plan_visitor_id = ANY($1) AND status <> 'rejected'",
        )
        .bind(&delete_ids)
        .fetch_all(&st.db)
        .await?
        .into_iter()
        .collect()
    };
    let area_names: HashMap<i64, String> = if area_ids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM areas WHERE id = ANY($1)")
            .bind(&area_ids)
            .fetch_all(&st.db)
            .await?
            .into_iter()
            .collect()
    };
    let valid_drivers: HashMap<i64, String> = if driver_ids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as::<_, (i64, String)>(crate::dyn_sql!(
            "SELECT u.id, u.name FROM users u
             JOIN vehicles v ON v.user_id = u.id AND v.is_active = 1
             WHERE u.id = ANY($1) AND {}",
            perms::sql_holds("u", Perm(Page::Drive, Act::Create))
        ))
        .bind(&driver_ids)
        .fetch_all(&st.db)
        .await?
        .into_iter()
        .collect()
    };

    let mut out = Vec::with_capacity(entries.len());
    for (idx, v) in entries.iter().enumerate() {
        let op = v.op.trim().to_string();
        let temp_id = if v.temp_id.trim().is_empty() {
            format!("v-{}", idx + 1)
        } else {
            v.temp_id.trim().to_string()
        };
        if op == "update" || op == "delete" {
            let vid = v.live_visitor_id.expect("checked above");
            if !valid_live_ids.contains(&vid) {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
        }
        if op == "delete" {
            let vid = v.live_visitor_id.expect("checked above");
            if live_reimb_ids.contains(&vid) {
                return Err(conflict(u.lang, ErrorKey::VisitorHasReimbursement));
            }
            out.push(DraftVisitorEntry {
                temp_id,
                op,
                live_visitor_id: v.live_visitor_id,
                name: v.name.trim().to_string(),
                phone: String::new(),
                notes: String::new(),
                area_id: None,
                area_name: None,
                pickup_address: String::new(),
                seats: v.seats,
                leg: v.leg.clone(),
                driver_id: None,
                driver_name: None,
            });
            continue;
        }
        let cleaned = validate_visitor_fields(
            u,
            &VisitorIn {
                name: v.name.clone(),
                phone: v.phone.clone(),
                notes: v.notes.clone(),
                area_id: v.area_id,
                pickup_address: v.pickup_address.clone(),
                seats: v.seats,
                leg: v.leg.clone(),
                driver_id: v.driver_id,
            },
        )?;
        let area_name = match v.area_id {
            Some(aid) => Some(
                area_names
                    .get(&aid)
                    .cloned()
                    .ok_or_else(|| bad(u.lang, ErrorKey::InvalidAction))?,
            ),
            None => None,
        };
        let driver_name = match v.driver_id {
            Some(did) => Some(
                valid_drivers
                    .get(&did)
                    .cloned()
                    .ok_or_else(|| bad(u.lang, ErrorKey::DriverNotFound))?,
            ),
            None => None,
        };
        out.push(DraftVisitorEntry {
            temp_id,
            op,
            live_visitor_id: v.live_visitor_id,
            name: cleaned.name,
            phone: cleaned.phone,
            notes: cleaned.notes,
            area_id: v.area_id,
            area_name,
            pickup_address: cleaned.pickup_address,
            seats: Some(cleaned.seats),
            leg: Some(cleaned.leg),
            driver_id: v.driver_id,
            driver_name,
        });
    }
    Ok(out)
}

#[derive(Deserialize)]
pub(super) struct CreateDraftReq {
    label: Option<String>,
    clone_from_draft_id: Option<i64>,
    assignments: Option<Vec<DraftEntry>>,
    visitors: Option<Vec<DraftVisitorEntry>>,
}

pub(super) async fn create_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    body: Option<Json<CreateDraftReq>>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let req = body.map(|b| b.0).unwrap_or(CreateDraftReq {
        label: None,
        clone_from_draft_id: None,
        assignments: None,
        visitors: None,
    });
    let label = clean_label(&u, req.label.as_deref())?;
    let cleaned_visitors = match req.visitors.as_ref() {
        Some(v) => Some(clean_draft_visitors(&st, &u, id, v).await?),
        None => None,
    };

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let (max_ver,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(version), 0)::BIGINT FROM event_plan_drafts WHERE event_id = $1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let next_version = max_ver + 1;

    let (cloned_assignments, cloned_visitors): (Option<String>, Option<String>) =
        if let Some(src_id) = req.clone_from_draft_id {
            let row: Option<(String, String)> = sqlx::query_as(
            "SELECT assignments, visitors FROM event_plan_drafts WHERE id = $1 AND event_id = $2",
        )
        .bind(src_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
            let (a, v) = row.ok_or_else(|| bad(u.lang, ErrorKey::PlanDraftNotFound))?;
            (Some(a), Some(v))
        } else {
            (None, None)
        };

    let assignments_json = if let Some(a) = &req.assignments {
        serde_json::to_string(a).unwrap_or_else(|_| "[]".to_string())
    } else if let Some(ca) = cloned_assignments {
        ca
    } else {
        let live: Vec<DraftEntry> = super::plan_rides_on(&mut *tx, id)
            .await?
            .into_iter()
            .map(|r| DraftEntry {
                ride_id: r.id,
                driver_id: r.driver_id,
            })
            .collect();
        serde_json::to_string(&live).unwrap_or_else(|_| "[]".to_string())
    };

    let visitors_json = if let Some(v) = &cleaned_visitors {
        serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
    } else if let Some(cv) = cloned_visitors {
        cv
    } else {
        "[]".to_string()
    };

    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);

    let (draft_id,): (i64,) = sqlx::query_as(
        "INSERT INTO event_plan_drafts (
             event_id, version, label, status, assignments, visitors,
             created_by, updated_by, locked_by, locked_at, lock_expires_at,
             created_at, updated_at
         )
         VALUES ($1, $2, $3, 'draft', $4, $5, $6, $6, $6, $7, $8, $7, $7)
         RETURNING id",
    )
    .bind(id)
    .bind(next_version)
    .bind(&label)
    .bind(&assignments_json)
    .bind(&visitors_json)
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.create_draft",
        "event",
        Some(id),
        format!("event {id}: created draft v{next_version} (#{draft_id})"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, draft_id)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
pub(super) struct UpdateDraftReq {
    label: Option<String>,
    assignments: Option<Vec<DraftEntry>>,
    visitors: Option<Vec<DraftVisitorEntry>>,
}

pub(super) async fn update_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
    Json(req): Json<UpdateDraftReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let label_opt = match req.label.as_deref() {
        Some(raw) => Some(clean_label(&u, Some(raw))?),
        None => None,
    };
    let visitors_json = match req.visitors.as_ref() {
        Some(v) => Some(
            serde_json::to_string(&clean_draft_visitors(&st, &u, id, v).await?)
                .unwrap_or_else(|_| "[]".to_string()),
        ),
        None => None,
    };
    let assignments_json = req
        .assignments
        .as_ref()
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| "[]".to_string()));

    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);
    let mut tx = st.db.begin().await?;

    let row: Option<(
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT d.status, d.label, d.assignments, d.visitors,
                    d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((
        cur_status,
        cur_label,
        cur_assignments,
        cur_visitors,
        locked_by,
        locked_by_name,
        lock_expires_at,
    )) = row
    else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    assert_draft_editable(&u, &cur_status)?;
    assert_not_locked_by_other(
        &u,
        locked_by,
        locked_by_name.as_deref(),
        lock_expires_at.as_deref(),
        &stamp,
    )?;

    let next_label = label_opt.unwrap_or(cur_label);
    let next_assignments = assignments_json.unwrap_or(cur_assignments);
    let next_visitors = visitors_json.unwrap_or(cur_visitors);

    sqlx::query(
        "UPDATE event_plan_drafts
         SET label = $1, assignments = $2, visitors = $3,
             updated_by = $4, updated_at = $5,
             locked_by = $4, locked_at = $5, lock_expires_at = $6
         WHERE id = $7 AND event_id = $8",
    )
    .bind(&next_label)
    .bind(&next_assignments)
    .bind(&next_visitors)
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.save_draft",
        "event",
        Some(id),
        format!("event {id}: updated draft #{did}"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

pub(super) async fn delete_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let mut tx = st.db.begin().await?;

    let row: Option<(
        String,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT d.status, d.created_by, d.updated_by, d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, created_by, updated_by, locked_by, locked_by_name, lock_expires_at)) = row
    else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    if status == "approved" {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }
    let is_owner = created_by.or(updated_by) == Some(u.id);
    if !is_owner && !u.is_traffic_lead() {
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

    sqlx::query("DELETE FROM event_plan_drafts WHERE id = $1 AND event_id = $2")
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.delete_draft",
        "event",
        Some(id),
        format!("event {id}: deleted draft #{did}"),
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(super) struct LockReq {
    force: Option<bool>,
}

pub(super) async fn acquire_draft_lock(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
    body: Option<Json<LockReq>>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let force = body.and_then(|b| b.0.force).unwrap_or(false);
    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);
    let mut tx = st.db.begin().await?;

    let row: Option<(String, Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.status, d.locked_by, u.name, d.lock_expires_at
         FROM event_plan_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.event_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    assert_draft_editable(&u, &status)?;

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
        "UPDATE event_plan_drafts
         SET locked_by = $1, locked_at = $2, lock_expires_at = $3
         WHERE id = $4 AND event_id = $5",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(&expires)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

pub(super) async fn release_draft_lock(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((id, did)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let mut tx = st.db.begin().await?;

    let row: Option<(Option<i64>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT d.locked_by, u.name, d.lock_expires_at
         FROM event_plan_drafts d
         LEFT JOIN users u ON u.id = d.locked_by
         WHERE d.id = $1 AND d.event_id = $2
         FOR UPDATE OF d",
    )
    .bind(did)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((locked_by, locked_by_name, lock_expires_at)) = row else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
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
        "UPDATE event_plan_drafts
         SET locked_by = NULL, locked_at = NULL, lock_expires_at = NULL
         WHERE id = $1 AND event_id = $2",
    )
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;
    Ok(Json(json!({
        "ok": true,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
pub(super) struct SaveDraftReq {
    draft_id: Option<i64>,
    label: Option<String>,
    assignments: Vec<DraftEntry>,
    visitors: Option<Vec<DraftVisitorEntry>>,
}

pub(super) async fn save_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<SaveDraftReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let _ev = load_event(&st, &u, id).await?;
    let label_opt = match req.label.as_deref() {
        Some(raw) => Some(clean_label(&u, Some(raw))?),
        None => None,
    };
    let visitors_opt = match req.visitors.as_ref() {
        Some(v) => Some(
            serde_json::to_string(&clean_draft_visitors(&st, &u, id, v).await?)
                .unwrap_or_else(|_| "[]".to_string()),
        ),
        None => None,
    };
    let assignments_json =
        serde_json::to_string(&req.assignments).unwrap_or_else(|_| "[]".to_string());
    let stamp = now();
    let expires = in_minutes(DRAFT_LOCK_MINUTES);

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let existing: Option<(
        i64,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = if let Some(did) = req.draft_id {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.visitors, d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
        )
        .bind(did)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::PlanDraftNotFound))
        .map(Some)?
    } else {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.visitors, d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.event_id = $1 AND d.status IN ('draft', 'returned')
             ORDER BY d.version DESC, d.id DESC
             LIMIT 1
             FOR UPDATE OF d",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    };

    let draft_id = if let Some((
        did,
        cur_status,
        cur_label,
        cur_visitors,
        locked_by,
        locked_by_name,
        lock_expires_at,
    )) = existing
    {
        assert_draft_editable(&u, &cur_status)?;
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
        let next_status = if cur_status == "pending_approval" {
            "pending_approval"
        } else {
            "draft"
        };
        let next_label = label_opt.unwrap_or(cur_label);
        let next_visitors = visitors_opt.unwrap_or(cur_visitors);
        sqlx::query(
            "UPDATE event_plan_drafts
             SET status = $1, label = $2, assignments = $3, visitors = $4,
                 updated_by = $5, updated_at = $6,
                 locked_by = $5, locked_at = $6, lock_expires_at = $7
             WHERE id = $8 AND event_id = $9",
        )
        .bind(next_status)
        .bind(&next_label)
        .bind(&assignments_json)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .bind(&expires)
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        did
    } else {
        let (max_ver,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(version), 0)::BIGINT FROM event_plan_drafts WHERE event_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let next_version = max_ver + 1;
        let next_label = label_opt.unwrap_or_default();
        let next_visitors = visitors_opt.unwrap_or_else(|| "[]".to_string());
        let (did,): (i64,) = sqlx::query_as(
            "INSERT INTO event_plan_drafts (
                 event_id, version, label, status, assignments, visitors,
                 created_by, updated_by, locked_by, locked_at, lock_expires_at,
                 created_at, updated_at
             )
             VALUES ($1, $2, $3, 'draft', $4, $5, $6, $6, $6, $7, $8, $7, $7)
             RETURNING id",
        )
        .bind(id)
        .bind(next_version)
        .bind(&next_label)
        .bind(&assignments_json)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .bind(&expires)
        .fetch_one(&mut *tx)
        .await?;
        did
    };

    tx.commit().await?;

    audit(
        &st.db,
        Some(u.id),
        "plan.save_draft",
        "event",
        Some(id),
        format!(
            "event {id}: saved draft #{draft_id} with {} assignments",
            req.assignments.len()
        ),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, draft_id)
        .await?
        .ok_or_else(|| not_found(u.lang))?;

    Ok(Json(json!({
        "ok": true,
        "status": draft_row.status,
        "assignments": req.assignments.len(),
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
pub(super) struct RequestApprovalReq {
    draft_id: Option<i64>,
    assignments: Option<Vec<DraftEntry>>,
    visitors: Option<Vec<DraftVisitorEntry>>,
    label: Option<String>,
}

pub(super) async fn request_approval(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<RequestApprovalReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    let ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let label_opt = match req.label.as_deref() {
        Some(raw) => Some(clean_label(&u, Some(raw))?),
        None => None,
    };
    let visitors_opt = match req.visitors.as_ref() {
        Some(v) => Some(
            serde_json::to_string(&clean_draft_visitors(&st, &u, id, v).await?)
                .unwrap_or_else(|_| "[]".to_string()),
        ),
        None => None,
    };
    let assignments_opt = req
        .assignments
        .as_ref()
        .map(|a| serde_json::to_string(a).unwrap_or_else(|_| "[]".to_string()));

    let mut tx = st.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('grace_ride_plan_draft', $1))")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let existing: Option<(
        i64,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = if let Some(did) = req.draft_id {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.assignments, d.visitors,
                    d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.id = $1 AND d.event_id = $2
             FOR UPDATE OF d",
        )
        .bind(did)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| bad(u.lang, ErrorKey::PlanDraftNotFound))
        .map(Some)?
    } else {
        sqlx::query_as(
            "SELECT d.id, d.status, d.label, d.assignments, d.visitors,
                    d.locked_by, u.name, d.lock_expires_at
             FROM event_plan_drafts d
             LEFT JOIN users u ON u.id = d.locked_by
             WHERE d.event_id = $1 AND d.status IN ('draft', 'returned')
             ORDER BY d.version DESC, d.id DESC
             LIMIT 1
             FOR UPDATE OF d",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    };

    let draft_id = if let Some((
        did,
        cur_status,
        cur_label,
        cur_assignments,
        cur_visitors,
        locked_by,
        locked_by_name,
        lock_expires_at,
    )) = existing
    {
        if !matches!(cur_status.as_str(), "draft" | "returned") {
            return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
        }
        assert_not_locked_by_other(
            &u,
            locked_by,
            locked_by_name.as_deref(),
            lock_expires_at.as_deref(),
            &stamp,
        )?;
        let next_label = label_opt.unwrap_or(cur_label);
        let next_assignments = assignments_opt.unwrap_or(cur_assignments);
        let next_visitors = visitors_opt.unwrap_or(cur_visitors);
        sqlx::query(
            "UPDATE event_plan_drafts
             SET status = 'pending_approval',
                 label = $1,
                 assignments = $2,
                 visitors = $3,
                 submitted_by = $4,
                 submitted_at = $5,
                 updated_by = $4,
                 updated_at = $5,
                 locked_by = NULL,
                 locked_at = NULL,
                 lock_expires_at = NULL
             WHERE id = $6 AND event_id = $7",
        )
        .bind(&next_label)
        .bind(&next_assignments)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .bind(did)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        did
    } else if let Some(json_str) = assignments_opt {
        let (max_ver,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(version), 0)::BIGINT FROM event_plan_drafts WHERE event_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let next_version = max_ver + 1;
        let next_label = label_opt.unwrap_or_default();
        let next_visitors = visitors_opt.unwrap_or_else(|| "[]".to_string());
        let (did,): (i64,) = sqlx::query_as(
            "INSERT INTO event_plan_drafts (
                 event_id, version, label, status, assignments, visitors,
                 created_by, updated_by, submitted_by, submitted_at,
                 created_at, updated_at
             )
             VALUES ($1, $2, $3, 'pending_approval', $4, $5, $6, $6, $6, $7, $7, $7)
             RETURNING id",
        )
        .bind(id)
        .bind(next_version)
        .bind(&next_label)
        .bind(&json_str)
        .bind(&next_visitors)
        .bind(u.id)
        .bind(&stamp)
        .fetch_one(&mut *tx)
        .await?;
        did
    } else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };

    tx.commit().await?;

    // Notify traffic lead(s) using exact role-token matching.
    let mut lead_rows: Vec<(i64,)> = sqlx::query_as(crate::dyn_sql!(
        "SELECT id FROM users WHERE {}",
        perms::sql_has_role("users", perms::TRAFFIC_HEAD_ROLE)
    ))
    .fetch_all(&st.db)
    .await?;

    if lead_rows.is_empty() {
        lead_rows = sqlx::query_as(crate::dyn_sql!(
            "SELECT id FROM users WHERE {}",
            perms::sql_has_any_role("users", &[perms::ADMIN_ROLE, perms::SUPERADMIN_ROLE])
        ))
        .fetch_all(&st.db)
        .await?;
    }

    let lead_ids: Vec<i64> = lead_rows
        .into_iter()
        .map(|(lid,)| lid)
        .filter(|&lid| lid != u.id)
        .collect();

    crate::util::notify_many(
        &st.db,
        &lead_ids,
        ("接送安排待审批", "Transport arrangement awaiting approval"),
        (
            &format!(
                "{} 提交了「{}」的接送安排草稿，等待您审批。",
                u.name, ev.title
            ),
            &format!(
                "{} submitted a transport arrangement draft for \"{}\" for approval.",
                u.name, ev.title
            ),
        ),
        &format!("/admin/events/{}/plan?draft={}", id, draft_id),
    )
    .await;

    audit(
        &st.db,
        Some(u.id),
        "plan.request_approval",
        "event",
        Some(id),
        format!("event {id}: submitted draft #{draft_id} for traffic lead approval"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, draft_id)
        .await?
        .ok_or_else(|| not_found(u.lang))?;

    Ok(Json(json!({
        "ok": true,
        "status": "pending_approval",
        "draft_id": draft_id,
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}

#[derive(Deserialize)]
pub(super) struct ReturnDraftReq {
    draft_id: Option<i64>,
    notes: Option<String>,
}

pub(super) async fn return_draft(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<ReturnDraftReq>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminPlan, Act::Update)?;
    if !u.is_traffic_lead() {
        return Err(forbidden(u.lang, ErrorKey::OnlyTrafficLeadCanApprove));
    }
    let ev = load_event(&st, &u, id).await?;
    let stamp = now();
    let notes = req.notes.unwrap_or_default().trim().to_string();

    let mut tx = st.db.begin().await?;
    let target: Option<(i64, String, Option<i64>, Option<i64>)> = if let Some(did) = req.draft_id {
        sqlx::query_as(
            "SELECT id, status, submitted_by, created_by
             FROM event_plan_drafts
             WHERE id = $1 AND event_id = $2
             FOR UPDATE",
        )
        .bind(did)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, status, submitted_by, created_by
             FROM event_plan_drafts
             WHERE event_id = $1 AND status = 'pending_approval'
             ORDER BY version DESC, id DESC
             LIMIT 1
             FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
    };

    let Some((did, cur_status, submitted_by, created_by)) = target else {
        return Err(bad(u.lang, ErrorKey::PlanDraftNotFound));
    };
    if !matches!(
        cur_status.as_str(),
        "pending_approval" | "draft" | "returned"
    ) {
        return Err(conflict(u.lang, ErrorKey::DraftNotEditable));
    }

    sqlx::query(
        "UPDATE event_plan_drafts
         SET status = 'returned',
             reviewed_by = $1,
             reviewed_at = $2,
             review_notes = $3,
             updated_at = $2,
             locked_by = NULL,
             locked_at = NULL,
             lock_expires_at = NULL
         WHERE id = $4 AND event_id = $5",
    )
    .bind(u.id)
    .bind(&stamp)
    .bind(&notes)
    .bind(did)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

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
        let reason_zh = if notes.is_empty() {
            String::new()
        } else {
            format!("：{}", notes)
        };
        let reason_en = if notes.is_empty() {
            String::new()
        } else {
            format!(": {}", notes)
        };
        crate::util::notify_many(
            &st.db,
            &notify_targets,
            ("接送安排草稿已退回", "Transport arrangement draft returned"),
            (
                &format!(
                    "交通组组长退回了「{}」的接送安排草稿{}",
                    ev.title, reason_zh
                ),
                &format!(
                    "Traffic lead returned the transport draft for \"{}\"{}",
                    ev.title, reason_en
                ),
            ),
            &format!("/admin/events/{}/plan?draft={}", id, did),
        )
        .await;
    }

    audit(
        &st.db,
        Some(u.id),
        "plan.return_draft",
        "event",
        Some(id),
        format!("event {id}: returned draft #{did} with notes: {notes}"),
    )
    .await;

    let draft_row = load_draft_by_id(&st, id, did)
        .await?
        .ok_or_else(|| not_found(u.lang))?;

    Ok(Json(json!({
        "ok": true,
        "status": "returned",
        "draft": draft_to_json(&draft_row, u.id, &stamp),
    })))
}
