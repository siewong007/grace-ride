//! Admin ride-demand breakdown reports by area and time slot.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{now, require};

#[derive(Deserialize)]
pub(super) struct DemandQuery {
    month: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

fn month_window(month: &str) -> Option<(String, String)> {
    let b = month.as_bytes();
    if b.len() != 7
        || b[4] != b'-'
        || !b[..4].iter().all(u8::is_ascii_digit)
        || !b[5..].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let y: i32 = month[..4].parse().ok()?;
    let m: u32 = month[5..7].parse().ok()?;
    if !(1900..=9998).contains(&y) || !(1..=12).contains(&m) {
        return None;
    }
    let (ey, em) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    Some((format!("{y:04}-{m:02}-01"), format!("{ey:04}-{em:02}-01")))
}

pub(crate) fn has_global_report_scope(u: &CurrentUser) -> bool {
    u.traffic_is_global()
        || u.can(Page::AdminReimbPay, Act::View)
        || u.can(Page::AdminBills, Act::View)
        || u.can(Page::AdminEvents, Act::View)
}

pub(super) async fn demand_report(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<DemandQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReports, Act::View)?;

    let (start, end, display_to, period_label) = match (q.from.as_deref(), q.to.as_deref()) {
        (Some(f), Some(t)) => {
            let fd = NaiveDate::parse_from_str(f.trim(), "%Y-%m-%d")
                .map_err(|_| bad(u.lang, ErrorKey::InvalidAction))?;
            let td = NaiveDate::parse_from_str(t.trim(), "%Y-%m-%d")
                .map_err(|_| bad(u.lang, ErrorKey::InvalidAction))?;
            if td < fd || (td - fd).num_days() > 366 {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
            let end_d = td.succ_opt().unwrap_or(td);
            (
                fd.format("%Y-%m-%d").to_string(),
                end_d.format("%Y-%m-%d").to_string(),
                td.format("%Y-%m-%d").to_string(),
                format!("{}..{}", f.trim(), t.trim()),
            )
        }
        _ => {
            let m = q
                .month
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| now()[..7].to_string());
            let (s, e) =
                month_window(&m).ok_or_else(|| bad(u.lang, ErrorKey::MonthFormatInvalid))?;
            let d_to = e.clone();
            (s, e, d_to, m)
        }
    };

    #[derive(sqlx::FromRow)]
    struct DemandRow {
        area_id: i64,
        area_zh: String,
        area_en: String,
        pickup_at: String,
        status: String,
        driver_id: Option<i64>,
        passenger_id: i64,
        seats_needed: i64,
        assistance: String,
        leg: String,
    }

    let scope_creator: Option<i64> = if has_global_report_scope(&u) {
        None
    } else {
        Some(u.id)
    };

    let rows: Vec<DemandRow> = sqlx::query_as(
        "SELECT r.pickup_area_id AS area_id, COALESCE(a.name, '') AS area_zh, COALESCE(a.name, '') AS area_en,
                r.pickup_at, r.status, r.driver_id, r.passenger_id, r.seats AS seats_needed,
                COALESCE(r.assistance, '') AS assistance, COALESCE(r.leg, 'outbound') AS leg
         FROM rides r
         LEFT JOIN areas a ON a.id = r.pickup_area_id
         WHERE r.pickup_at >= $1 AND r.pickup_at < $2
           AND ($3::BIGINT IS NULL OR r.event_id IN (SELECT id FROM events WHERE created_by = $3))",
    )
    .bind(&start)
    .bind(&end)
    .bind(scope_creator)
    .fetch_all(&st.db)
    .await?;

    let mut total_requests = 0_i64;
    let mut fulfilled = 0_i64;
    let mut unfilled = 0_i64;
    let mut completed = 0_i64;
    let mut cancelled_unassigned = 0_i64;
    let mut wheelchair_requests = 0_i64;
    let mut return_requests = 0_i64;
    let mut seats_unfilled = 0_i64;
    let mut drivers_active: HashSet<i64> = HashSet::new();
    let mut passengers_seen: HashSet<i64> = HashSet::new();

    #[derive(Default)]
    struct Bucket {
        area_zh: String,
        area_en: String,
        total: i64,
        fulfilled: i64,
        unfilled: i64,
        cancelled_unassigned: i64,
        seats_needed: i64,
        seats_unfilled: i64,
    }

    let mut by_area: HashMap<i64, Bucket> = HashMap::new();
    let mut by_slot: BTreeMap<(u32, String), Bucket> = BTreeMap::new();

    for r in &rows {
        let is_cancelled = r.status == "cancelled";
        let is_closed_alt = matches!(r.status.as_str(), "no_show" | "alt_transport");
        let is_unfilled = !is_cancelled && !is_closed_alt && r.driver_id.is_none();
        let is_fulfilled = matches!(r.status.as_str(), "confirmed" | "in_progress" | "completed");

        if is_cancelled {
            if r.driver_id.is_none() {
                cancelled_unassigned += 1;
            }
        } else {
            total_requests += 1;
            passengers_seen.insert(r.passenger_id);
            if r.status == "completed" {
                completed += 1;
            }
            if let Some(did) = r.driver_id {
                drivers_active.insert(did);
            }
            if is_fulfilled {
                fulfilled += 1;
            }
            if is_unfilled {
                unfilled += 1;
                seats_unfilled += r.seats_needed;
            }
            if r.assistance.to_ascii_lowercase().contains("wheelchair")
                || r.assistance.contains("轮椅")
            {
                wheelchair_requests += 1;
            }
            if r.leg == "return" {
                return_requests += 1;
            }
        }

        let ab = by_area.entry(r.area_id).or_insert_with(|| Bucket {
            area_zh: r.area_zh.clone(),
            area_en: r.area_en.clone(),
            ..Default::default()
        });
        if is_cancelled {
            if r.driver_id.is_none() {
                ab.cancelled_unassigned += 1;
            }
        } else {
            ab.total += 1;
            ab.seats_needed += r.seats_needed;
            if is_fulfilled {
                ab.fulfilled += 1;
            }
            if is_unfilled {
                ab.unfilled += 1;
                ab.seats_unfilled += r.seats_needed;
            }
        }

        if r.pickup_at.len() >= 13 {
            if let Ok(d) = NaiveDate::parse_from_str(&r.pickup_at[..10], "%Y-%m-%d") {
                let wd = d.weekday().num_days_from_sunday();
                let hour: u32 = r.pickup_at[11..13].parse().unwrap_or(0);
                let bucket_h = (hour / 2) * 2;
                let slot_label = format!("{bucket_h:02}:00-{:02}:00", (bucket_h + 2).min(24));
                let sb = by_slot.entry((wd, slot_label)).or_default();
                if is_cancelled {
                    if r.driver_id.is_none() {
                        sb.cancelled_unassigned += 1;
                    }
                } else {
                    sb.total += 1;
                    sb.seats_needed += r.seats_needed;
                    if is_fulfilled {
                        sb.fulfilled += 1;
                    }
                    if is_unfilled {
                        sb.unfilled += 1;
                        sb.seats_unfilled += r.seats_needed;
                    }
                }
            }
        }
    }

    let mut area_list: Vec<Value> = by_area
        .into_iter()
        .map(|(area_id, b)| {
            json!({
                "area_id": area_id,
                "area_name_zh": b.area_zh,
                "area_name_en": b.area_en,
                "total": b.total,
                "fulfilled": b.fulfilled,
                "unfilled": b.unfilled,
                "cancelled_unassigned": b.cancelled_unassigned,
                "seats_needed": b.seats_needed,
                "seats_unfilled": b.seats_unfilled,
                "fulfillment_rate_pct": if b.total > 0 { (b.fulfilled * 100) / b.total } else { 100 },
            })
        })
        .collect();
    area_list.sort_by(|a, b| {
        b["unfilled"]
            .as_i64()
            .cmp(&a["unfilled"].as_i64())
            .then_with(|| b["total"].as_i64().cmp(&a["total"].as_i64()))
    });

    let mut slot_list: Vec<Value> = by_slot
        .into_iter()
        .filter(|(_, b)| b.total > 0 || b.cancelled_unassigned > 0)
        .map(|((wd, slot), b)| {
            json!({
                "weekday": wd,
                "slot": slot,
                "total": b.total,
                "fulfilled": b.fulfilled,
                "unfilled": b.unfilled,
                "cancelled_unassigned": b.cancelled_unassigned,
                "seats_needed": b.seats_needed,
            })
        })
        .collect();
    slot_list.sort_by(|a, b| {
        b["unfilled"]
            .as_i64()
            .cmp(&a["unfilled"].as_i64())
            .then_with(|| b["total"].as_i64().cmp(&a["total"].as_i64()))
    });

    let fulfillment_rate_pct = if total_requests > 0 {
        (fulfilled * 100) / total_requests
    } else {
        100
    };

    Ok(Json(json!({
        "period": period_label,
        "from": start,
        "to": display_to,
        "totals": {
            "total_requests": total_requests,
            "fulfilled": fulfilled,
            "unfilled": unfilled,
            "completed": completed,
            "cancelled_unassigned": cancelled_unassigned,
            "wheelchair_requests": wheelchair_requests,
            "return_requests": return_requests,
            "seats_unfilled": seats_unfilled,
            "active_drivers": drivers_active.len(),
            "unique_passengers": passengers_seen.len(),
            "fulfillment_rate_pct": fulfillment_rate_pct,
        },
        "by_area": area_list,
        "by_slot": slot_list,
    })))
}

#[derive(Deserialize)]
pub(super) struct UpdateBadgeIconIn {
    pub icon: String,
}

pub(super) async fn update_user_badge_icon(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((user_id, badge_key)): Path<(i64, String)>,
    Json(req): Json<UpdateBadgeIconIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminMembers, Act::Update)?;
    let icon = req.icon.trim().to_string();
    if icon.is_empty() {
        sqlx::query("DELETE FROM user_badge_icons WHERE user_id = $1 AND badge_key = $2")
            .bind(user_id)
            .bind(&badge_key)
            .execute(&st.db)
            .await?;
    } else {
        sqlx::query(
            "INSERT INTO user_badge_icons (user_id, badge_key, icon, updated_at, updated_by)
             VALUES ($1, $2, $3, app_now(), $4)
             ON CONFLICT (user_id, badge_key)
             DO UPDATE SET icon = EXCLUDED.icon, updated_at = EXCLUDED.updated_at, updated_by = EXCLUDED.updated_by",
        )
        .bind(user_id)
        .bind(&badge_key)
        .bind(&icon)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    }
    crate::util::audit(
        &st.db,
        Some(u.id),
        "admin.user_badge_icon",
        "user",
        Some(user_id),
        &format!("badge {badge_key} icon set to {icon}"),
    )
    .await;
    Ok(Json(
        json!({ "ok": true, "user_id": user_id, "badge_key": badge_key, "icon": icon }),
    ))
}
