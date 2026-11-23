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
use crate::perms::{can_approve_transport_plan, Act, Page, TRAFFIC_MEMBER_ROLE, TRANSPORT_ROLE};
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

struct BadgeDef {
    key: &'static str,
    category: &'static str,
    role_match: bool,
    ch_zh: &'static str,
    ch_en: &'static str,
    name_zh: &'static str,
    name_en: &'static str,
    rule_zh: &'static str,
    rule_en: &'static str,
    progress: i64,
    tiers: [i64; 4],
    default_icon: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedBadgeTier {
    pub tier: &'static str,
    pub tier_level: u8,
    pub next_tier: Option<&'static str>,
    pub next_target: Option<i64>,
    pub target: i64,
    pub earned: bool,
}

/// Resolve a badge's progressive tier state (`none` -> `bronze` -> `silver` -> `gold` -> `platinum`)
/// alongside the next unlock threshold so clients can render tier progress and medallions.
pub(crate) fn resolve_badge_tier(progress: i64, tiers: [i64; 4]) -> ResolvedBadgeTier {
    if progress >= tiers[3] {
        ResolvedBadgeTier {
            tier: "platinum",
            tier_level: 4,
            next_tier: None,
            next_target: None,
            target: tiers[3],
            earned: true,
        }
    } else if progress >= tiers[2] {
        ResolvedBadgeTier {
            tier: "gold",
            tier_level: 3,
            next_tier: Some("platinum"),
            next_target: Some(tiers[3]),
            target: tiers[3],
            earned: true,
        }
    } else if progress >= tiers[1] {
        ResolvedBadgeTier {
            tier: "silver",
            tier_level: 2,
            next_tier: Some("gold"),
            next_target: Some(tiers[2]),
            target: tiers[2],
            earned: true,
        }
    } else if progress >= tiers[0] {
        ResolvedBadgeTier {
            tier: "bronze",
            tier_level: 1,
            next_tier: Some("silver"),
            next_target: Some(tiers[1]),
            target: tiers[1],
            earned: true,
        }
    } else {
        ResolvedBadgeTier {
            tier: "none",
            tier_level: 0,
            next_tier: Some("bronze"),
            next_target: Some(tiers[0]),
            target: tiers[0],
            earned: false,
        }
    }
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

    let roles: Vec<String> =
        sqlx::query_scalar("SELECT role_key FROM member_roles WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&st.db)
            .await?;

    // Normal Member metrics
    let (member_rides,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM rides WHERE passenger_id=$1 AND status='completed'")
            .bind(user_id)
            .fetch_one(&st.db)
            .await?;
    let (member_events,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM event_signups WHERE user_id=$1 AND attended=1")
            .bind(user_id)
            .fetch_one(&st.db)
            .await?;
    let (member_ready,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM rides WHERE passenger_id=$1 AND readiness_status='ready'",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    let (member_care,): (i64,) = sqlx::query_as(
        "SELECT ((SELECT COUNT(*) FROM rides WHERE passenger_id=$1 AND is_proxy=1 AND status <> 'cancelled') + (SELECT COUNT(*) FROM visitor_passes WHERE host_id=$1))::BIGINT",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;

    // Driver metrics
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
            .checked_sub_months(chrono::Months::new(12))
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

    // Traffic Lead metrics
    let (tl_approved,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rides WHERE approved_by=$1")
        .bind(user_id)
        .fetch_one(&st.db)
        .await?;
    let (tl_plans,): (i64,) = sqlx::query_as(
        "SELECT ((SELECT COUNT(*) FROM event_plan_drafts WHERE reviewed_by=$1 AND status='approved') + (SELECT COUNT(*) FROM ride_change_drafts WHERE reviewed_by=$1 AND status='approved'))::BIGINT",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    let (tl_assigns,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE actor_id=$1 AND action='ride.plan_assign'",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;
    let (tl_steward,): (i64,) = sqlx::query_as(
        "SELECT ((SELECT COUNT(*) FROM help_tickets WHERE assignee_id=$1 AND status IN ('answered', 'closed')) + (SELECT COUNT(*) FROM reimbursements WHERE reviewed_by=$1))::BIGINT",
    )
    .bind(user_id)
    .fetch_one(&st.db)
    .await?;

    let is_driver = roles.iter().any(|r| r == "driver") || total > 0;
    let is_traffic_lead = can_approve_transport_plan(&roles)
        || roles
            .iter()
            .any(|r| r == TRAFFIC_MEMBER_ROLE || r == TRANSPORT_ROLE)
        || tl_approved > 0
        || tl_plans > 0
        || tl_assigns > 0;

    let custom_icons: Vec<(String, String)> =
        sqlx::query_as("SELECT badge_key, icon FROM user_badge_icons WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&st.db)
            .await
            .unwrap_or_default();
    let icon_map: std::collections::HashMap<String, String> = custom_icons.into_iter().collect();

    // Criteria are public (徽章获得条件应公开透明).
    let defs = [
        // Normal Member
        BadgeDef {
            key: "member_first_journey",
            category: "member",
            role_match: true,
            ch_zh: "行",
            ch_en: "J",
            name_zh: "喜乐同行",
            name_en: "Joyful Rider",
            rule_zh: "完成乘客接送行程（铜 3 · 银 10 · 金 25 · 铂金 50）",
            rule_en: "Complete rides as a passenger (Bronze 3 · Silver 10 · Gold 25 · Platinum 50)",
            progress: member_rides,
            tiers: [3, 10, 25, 50],
            default_icon: "badge_joy_rider",
        },
        BadgeDef {
            key: "member_fellowship",
            category: "member",
            role_match: true,
            ch_zh: "聚",
            ch_en: "F",
            name_zh: "忠心聚会",
            name_en: "Faithful Attendee",
            rule_zh: "出席教会活动（铜 5 · 银 15 · 金 30 · 铂金 60）",
            rule_en: "Attend church events (Bronze 5 · Silver 15 · Gold 30 · Platinum 60)",
            progress: member_events,
            tiers: [5, 15, 30, 60],
            default_icon: "badge_fellowship_sprout",
        },
        BadgeDef {
            key: "member_punctual",
            category: "member",
            role_match: true,
            ch_zh: "准",
            ch_en: "P",
            name_zh: "准时小达人",
            name_en: "Ready & Punctual",
            rule_zh: "行程前提前确认准时就绪（铜 3 · 银 10 · 金 25 · 铂金 50）",
            rule_en: "Confirm ready before pickup (Bronze 3 · Silver 10 · Gold 25 · Platinum 50)",
            progress: member_ready,
            tiers: [3, 10, 25, 50],
            default_icon: "badge_punctual_clock",
        },
        BadgeDef {
            key: "member_warm_welcome",
            category: "member",
            role_match: true,
            ch_zh: "暖",
            ch_en: "W",
            name_zh: "温暖关怀",
            name_en: "Caring Helper",
            rule_zh: "代家人预约接送或接待访客（铜 2 · 银 5 · 金 12 · 铂金 25）",
            rule_en: "Book proxy rides or host visitors (Bronze 2 · Silver 5 · Gold 12 · Platinum 25)",
            progress: member_care,
            tiers: [2, 5, 12, 25],
            default_icon: "badge_warm_welcome",
        },
        // Driver
        BadgeDef {
            key: "night",
            category: "driver",
            role_match: is_driver,
            ch_zh: "宵",
            ch_en: "N",
            name_zh: "宵夜王",
            name_en: "Night Owl",
            rule_zh: "完成晚上 9 点后的接送（铜 3 · 银 8 · 金 20 · 铂金 40）",
            rule_en: "Complete rides after 9 pm (Bronze 3 · Silver 8 · Gold 20 · Platinum 40)",
            progress: night,
            tiers: [3, 8, 20, 40],
            default_icon: "moon",
        },
        BadgeDef {
            key: "regular",
            category: "driver",
            role_match: is_driver,
            ch_zh: "勤",
            ch_en: "R",
            name_zh: "常驻司机",
            name_en: "Resident Driver",
            rule_zh: "过去 12 个月有完成接送的月份数（铜 3 · 银 6 · 金 9 · 铂金 12）",
            rule_en: "Active driving months in the past year (Bronze 3 · Silver 6 · Gold 9 · Platinum 12)",
            progress: months,
            tiers: [3, 6, 9, 12],
            default_icon: "award",
        },
        BadgeDef {
            key: "carpool",
            category: "driver",
            role_match: is_driver,
            ch_zh: "爱",
            ch_en: "C",
            name_zh: "爱心同行者",
            name_en: "Caring Companion",
            rule_zh: "开启共乘并完成多人同车接送（铜 2 · 银 6 · 金 15 · 铂金 30）",
            rule_en: "Complete multi-passenger carpool trips (Bronze 2 · Silver 6 · Gold 15 · Platinum 30)",
            progress: if carpool == 1 { shared } else { 0 },
            tiers: [2, 6, 15, 30],
            default_icon: "heart",
        },
        BadgeDef {
            key: "driver_helping_hands",
            category: "driver",
            role_match: is_driver,
            ch_zh: "护",
            ch_en: "H",
            name_zh: "恩典护航手",
            name_en: "Helping Wheels",
            rule_zh: "作为志愿司机累计完成接送（铜 10 · 银 30 · 金 75 · 铂金 150）",
            rule_en: "Complete rides as a volunteer driver (Bronze 10 · Silver 30 · Gold 75 · Platinum 150)",
            progress: total,
            tiers: [10, 30, 75, 150],
            default_icon: "badge_helping_wheels",
        },
        // Traffic Lead
        BadgeDef {
            key: "traffic_dispatcher",
            category: "traffic_lead",
            role_match: is_traffic_lead,
            ch_zh: "通",
            ch_en: "G",
            name_zh: "畅通指挥官",
            name_en: "Green-Light Captain",
            rule_zh: "审核批准接送配对（铜 5 · 银 20 · 金 60 · 铂金 150）",
            rule_en: "Approve ride matches (Bronze 5 · Silver 20 · Gold 60 · Platinum 150)",
            progress: tl_approved,
            tiers: [5, 20, 60, 150],
            default_icon: "badge_traffic_light",
        },
        BadgeDef {
            key: "traffic_planner",
            category: "traffic_lead",
            role_match: is_traffic_lead,
            ch_zh: "规",
            ch_en: "P",
            name_zh: "行程规划师",
            name_en: "Master Route Planner",
            rule_zh: "批准活动交通方案或变更草案（铜 2 · 银 6 · 金 15 · 铂金 35）",
            rule_en: "Approve transport plans or change drafts (Bronze 2 · Silver 6 · Gold 15 · Platinum 35)",
            progress: tl_plans,
            tiers: [2, 6, 15, 35],
            default_icon: "badge_route_compass",
        },
        BadgeDef {
            key: "traffic_marshal",
            category: "traffic_lead",
            role_match: is_traffic_lead,
            ch_zh: "调",
            ch_en: "M",
            name_zh: "交通调度星",
            name_en: "Smooth Coordinator",
            rule_zh: "完成活动交通编排派车调度（铜 5 · 银 20 · 金 50 · 铂金 120）",
            rule_en: "Coordinate ride plan assignments (Bronze 5 · Silver 20 · Gold 50 · Platinum 120)",
            progress: tl_assigns,
            tiers: [5, 20, 50, 120],
            default_icon: "badge_traffic_whistle",
        },
        BadgeDef {
            key: "traffic_guardian",
            category: "traffic_lead",
            role_match: is_traffic_lead,
            ch_zh: "管",
            ch_en: "S",
            name_zh: "贴心交通管家",
            name_en: "Care Steward",
            rule_zh: "处理求助工单或报销审核（铜 3 · 银 12 · 金 30 · 铂金 75）",
            rule_en: "Resolve help tickets or reimbursement reviews (Bronze 3 · Silver 12 · Gold 30 · Platinum 75)",
            progress: tl_steward,
            tiers: [3, 12, 30, 75],
            default_icon: "badge_care_shield",
        },
    ];

    Ok(defs
        .iter()
        .map(|d| {
            let ch = t(lang, d.ch_zh, d.ch_en);
            let name = t(lang, d.name_zh, d.name_en);
            let rule = t(lang, d.rule_zh, d.rule_en);
            let icon = icon_map
                .get(d.key)
                .map(|s| s.as_str())
                .unwrap_or(d.default_icon);
            let resolved = resolve_badge_tier(d.progress, d.tiers);
            let tier_steps: Vec<Value> = [
                ("bronze", d.tiers[0]),
                ("silver", d.tiers[1]),
                ("gold", d.tiers[2]),
                ("platinum", d.tiers[3]),
            ]
            .into_iter()
            .map(|(tier_name, threshold)| {
                json!({
                    "tier": tier_name,
                    "target": threshold,
                    "reached": d.progress >= threshold,
                })
            })
            .collect();
            json!({
                "key": d.key,
                "category": d.category,
                "role_match": d.role_match,
                "char": ch,
                "icon": icon,
                "name": name,
                "rule": rule,
                "char_zh": d.ch_zh,
                "char_en": d.ch_en,
                "name_zh": d.name_zh,
                "name_en": d.name_en,
                "rule_zh": d.rule_zh,
                "rule_en": d.rule_en,
                "progress": d.progress,
                "target": resolved.target,
                "earned": resolved.earned,
                "tier": resolved.tier,
                "tier_level": resolved.tier_level,
                "next_tier": resolved.next_tier,
                "next_target": resolved.next_target,
                "tiers": tier_steps,
            })
        })
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

#[cfg(test)]
mod tests {
    use super::resolve_badge_tier;

    #[test]
    fn resolve_badge_tier_handles_all_five_boundaries() {
        let tiers = [3, 10, 25, 50];

        let locked = resolve_badge_tier(2, tiers);
        assert_eq!(locked.tier, "none");
        assert_eq!(locked.tier_level, 0);
        assert_eq!(locked.next_tier, Some("bronze"));
        assert_eq!(locked.next_target, Some(3));
        assert_eq!(locked.target, 3);
        assert!(!locked.earned);

        let bronze = resolve_badge_tier(3, tiers);
        assert_eq!(bronze.tier, "bronze");
        assert_eq!(bronze.tier_level, 1);
        assert_eq!(bronze.next_tier, Some("silver"));
        assert_eq!(bronze.next_target, Some(10));
        assert_eq!(bronze.target, 10);
        assert!(bronze.earned);

        let silver = resolve_badge_tier(10, tiers);
        assert_eq!(silver.tier, "silver");
        assert_eq!(silver.tier_level, 2);
        assert_eq!(silver.next_tier, Some("gold"));
        assert_eq!(silver.next_target, Some(25));
        assert_eq!(silver.target, 25);
        assert!(silver.earned);

        let gold = resolve_badge_tier(25, tiers);
        assert_eq!(gold.tier, "gold");
        assert_eq!(gold.tier_level, 3);
        assert_eq!(gold.next_tier, Some("platinum"));
        assert_eq!(gold.next_target, Some(50));
        assert_eq!(gold.target, 50);
        assert!(gold.earned);

        let platinum = resolve_badge_tier(64, tiers);
        assert_eq!(platinum.tier, "platinum");
        assert_eq!(platinum.tier_level, 4);
        assert_eq!(platinum.next_tier, None);
        assert_eq!(platinum.next_target, None);
        assert_eq!(platinum.target, 50);
        assert!(platinum.earned);
    }
}
