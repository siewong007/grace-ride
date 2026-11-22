//! Member monthly/quarterly/yearly activity reports and achievement badges.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{now, require, t, Lang};

// ---------------- Monthly report & badges ----------------

#[derive(Deserialize)]
pub(super) struct ReportQuery {
    month: Option<String>, // YYYY-MM
    user_id: Option<i64>,
    /// "driver" | "passenger" | "all" — which side of rides to report (default: all)
    r#type: Option<String>,
    /// "monthly" | "quarterly" | "yearly" — report period granularity (default: monthly)
    range: Option<String>,
    lang: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct BadgeQuery {
    lang: Option<String>,
}

/// Parse `YYYY-MM` without slicing by byte offset: a 7-byte non-ASCII string used
/// to panic the handler when a slice landed inside a multi-byte character.
pub(super) fn parse_report_month(raw: &str) -> Option<(i32, u32)> {
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

    let custom_icons: Vec<(String, String)> =
        sqlx::query_as("SELECT badge_key, icon FROM user_badge_icons WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&st.db)
            .await
            .unwrap_or_default();
    let icon_map: std::collections::HashMap<String, String> = custom_icons.into_iter().collect();

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
            "moon",
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
            "award",
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
            "heart",
        ),
    ];
    let _ = total;
    Ok(defs
        .iter()
        .map(
            |(key, ch_zh, ch_en, name_zh, name_en, rule_zh, rule_en, have, need, default_icon)| {
                let ch = t(lang, ch_zh, ch_en);
                let name = t(lang, name_zh, name_en);
                let rule = t(lang, rule_zh, rule_en);
                let icon = icon_map
                    .get(*key)
                    .map(|s| s.as_str())
                    .unwrap_or(*default_icon);
                json!({
                    "key": key,
                    "char": ch,
                    "icon": icon,
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

pub(super) async fn my_badges(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<BadgeQuery>,
) -> AppResult<Json<Value>> {
    let lang = q.lang.as_deref().map(Lang::from_code).unwrap_or(u.lang);
    Ok(Json(json!(compute_badges(&st, u.id, lang).await?)))
}

/// 个人月报 — only the member themself or an admin (获授权人员) may view.
/// Supports driver/passenger filter and monthly/quarterly/yearly ranges.
pub(super) async fn monthly_report(
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
