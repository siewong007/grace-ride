//! Admin role-scoped analytics reports, capabilities discovery, and badge icon customization.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Datelike, Duration, Months, NaiveDate};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{self, Act, Page};
use crate::routes::community::{parse_report_month, BADGE_KEYS};
use crate::state::AppState;
use crate::util::{now, require, Lang};

mod demand;
mod events;
mod finance;
mod overview;

pub(super) use demand::demand_report;
pub(super) use events::events_report;
pub(super) use finance::finance_report;
pub(super) use overview::overview_report;

#[derive(Deserialize)]
pub(super) struct PeriodQuery {
    pub month: Option<String>,
    pub range: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub group_id: Option<i64>,
    pub kind: Option<String>,
}

pub(super) struct PeriodWindow {
    pub start: String,
    pub end: String,
    pub display_to: String,
    pub period_label: String,
    pub range: String,
    pub end_ym_incl: String,
}

pub(super) fn parse_period_window(
    lang: Lang,
    month: Option<&str>,
    range: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
) -> AppResult<PeriodWindow> {
    let from_t = from.map(str::trim).filter(|s| !s.is_empty());
    let to_t = to.map(str::trim).filter(|s| !s.is_empty());
    match (from_t, to_t) {
        (Some(f), Some(t)) => {
            let fd = NaiveDate::parse_from_str(f, "%Y-%m-%d")
                .map_err(|_| bad(lang, ErrorKey::InvalidAction))?;
            let td = NaiveDate::parse_from_str(t, "%Y-%m-%d")
                .map_err(|_| bad(lang, ErrorKey::InvalidAction))?;
            if td < fd
                || (td - fd).num_days() > 366
                || !(1900..=9998).contains(&fd.year())
                || !(1900..=9998).contains(&td.year())
            {
                return Err(bad(lang, ErrorKey::InvalidAction));
            }
            let end_d = td.succ_opt().unwrap_or(td);
            Ok(PeriodWindow {
                start: fd.format("%Y-%m-%d").to_string(),
                end: end_d.format("%Y-%m-%d").to_string(),
                display_to: td.format("%Y-%m-%d").to_string(),
                period_label: format!("{f}..{t}"),
                range: "custom".to_string(),
                end_ym_incl: td.format("%Y-%m").to_string(),
            })
        }
        (Some(_), None) | (None, Some(_)) => Err(bad(lang, ErrorKey::InvalidAction)),
        (None, None) => {
            let m = month
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| now()[..7].to_string());
            let (y, mon) =
                parse_report_month(&m).ok_or_else(|| bad(lang, ErrorKey::MonthFormatInvalid))?;
            let raw_range = range.map(str::trim).unwrap_or("monthly");
            let (start, end, norm_range, period_label) = match raw_range {
                "quarterly" => {
                    let quarter = (mon - 1) / 3;
                    let start_m = quarter * 3 + 1;
                    let end_m = start_m + 3;
                    let (end_y, end_m) = if end_m > 12 {
                        (y + 1, end_m - 12)
                    } else {
                        (y, end_m)
                    };
                    (
                        format!("{y:04}-{start_m:02}-01"),
                        format!("{end_y:04}-{end_m:02}-01"),
                        "quarterly".to_string(),
                        format!("{y:04}-Q{}", quarter + 1),
                    )
                }
                "yearly" => (
                    format!("{y:04}-01-01"),
                    format!("{:04}-01-01", y + 1),
                    "yearly".to_string(),
                    format!("{y:04}"),
                ),
                _ => {
                    let (end_y, end_m) = if mon == 12 { (y + 1, 1) } else { (y, mon + 1) };
                    (
                        format!("{y:04}-{mon:02}-01"),
                        format!("{end_y:04}-{end_m:02}-01"),
                        "monthly".to_string(),
                        m,
                    )
                }
            };
            let end_incl_date =
                NaiveDate::parse_from_str(&end, "%Y-%m-%d").map(|d| d - Duration::days(1));
            let display_to = end_incl_date
                .as_ref()
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|_| start.clone());
            let end_ym_incl = end_incl_date
                .as_ref()
                .map(|d| d.format("%Y-%m").to_string())
                .unwrap_or_else(|_| start[..7].to_string());
            Ok(PeriodWindow {
                start,
                end,
                display_to,
                period_label,
                range: norm_range,
                end_ym_incl,
            })
        }
    }
}

pub(super) fn prev_period_window(w: &PeriodWindow) -> Option<PeriodWindow> {
    let fd = NaiveDate::parse_from_str(&w.start, "%Y-%m-%d").ok()?;
    let td = NaiveDate::parse_from_str(&w.display_to, "%Y-%m-%d").ok()?;
    let (prev_start, prev_disp_to, period_label) = match w.range.as_str() {
        "monthly" => {
            let ps = fd.checked_sub_months(Months::new(1))?;
            let pd = fd - Duration::days(1);
            let lbl = ps.format("%Y-%m").to_string();
            (ps, pd, lbl)
        }
        "quarterly" => {
            let ps = fd.checked_sub_months(Months::new(3))?;
            let pd = fd - Duration::days(1);
            let q = (ps.month() - 1) / 3 + 1;
            let lbl = format!("{:04}-Q{q}", ps.year());
            (ps, pd, lbl)
        }
        "yearly" => {
            let ps = fd.checked_sub_months(Months::new(12))?;
            let pd = fd - Duration::days(1);
            let lbl = format!("{:04}", ps.year());
            (ps, pd, lbl)
        }
        _ => {
            let span = (td - fd).num_days() + 1;
            let pd = fd - Duration::days(1);
            let ps = fd - Duration::days(span);
            let lbl = format!("{}..{}", ps.format("%Y-%m-%d"), pd.format("%Y-%m-%d"));
            (ps, pd, lbl)
        }
    };
    if prev_start.year() < 1900 {
        return None;
    }
    Some(PeriodWindow {
        start: prev_start.format("%Y-%m-%d").to_string(),
        end: w.start.clone(),
        display_to: prev_disp_to.format("%Y-%m-%d").to_string(),
        period_label,
        range: w.range.clone(),
        end_ym_incl: prev_disp_to.format("%Y-%m").to_string(),
    })
}

pub(super) fn month_buckets(w: &PeriodWindow) -> Vec<String> {
    let Ok(mut cur) = NaiveDate::parse_from_str(&format!("{}-01", &w.start[..7]), "%Y-%m-%d")
    else {
        return Vec::new();
    };
    let Ok(last) = NaiveDate::parse_from_str(&format!("{}-01", w.end_ym_incl), "%Y-%m-%d") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while cur <= last && out.len() < 15 {
        out.push(cur.format("%Y-%m").to_string());
        let (y, m) = (cur.year(), cur.month());
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        let Some(next) = NaiveDate::from_ymd_opt(ny, nm, 1) else {
            break;
        };
        cur = next;
    }
    out
}

pub(crate) fn has_global_report_scope(u: &CurrentUser) -> bool {
    u.traffic_is_global()
        || u.can(Page::AdminReimbPay, Act::View)
        || u.can(Page::AdminBills, Act::View)
        || u.can(Page::AdminEvents, Act::View)
}

pub(super) struct ReportCaps {
    pub overview: bool,
    pub member_reports_global: bool,
    pub member_badge_icon_edit: bool,
    pub demand: bool,
    pub demand_global: bool,
    pub events: bool,
    pub events_global: bool,
    pub reimb_review: bool,
    pub reimb_pay: bool,
    pub bills: bool,
    pub finance: bool,
    pub membership: bool,
}

impl ReportCaps {
    pub(super) fn from_user(u: &CurrentUser) -> Self {
        let demand = u.can(Page::AdminRides, Act::View) || u.can(Page::AdminPlan, Act::View);
        let demand_global = demand && u.traffic_is_global();
        let events = u.can(Page::AdminEvents, Act::View) || u.can(Page::Events, Act::Create);
        let events_global = u.can(Page::AdminEvents, Act::View);
        let reimb_review = u.can(Page::AdminReimbReview, Act::View);
        let reimb_pay = u.can(Page::AdminReimbPay, Act::View);
        let bills = u.can(Page::AdminBills, Act::View);
        let finance = reimb_review || reimb_pay || bills;
        let membership =
            u.can(Page::AdminMembers, Act::Update) || u.can(Page::AdminInvites, Act::View);
        let overview = demand || events || finance || membership;
        Self {
            overview,
            member_reports_global: has_global_report_scope(u),
            member_badge_icon_edit: u.can(Page::AdminMembers, Act::Update),
            demand,
            demand_global,
            events,
            events_global,
            reimb_review,
            reimb_pay,
            bills,
            finance,
            membership,
        }
    }
}

pub(super) async fn capabilities(u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminReports, Act::View)?;
    let caps = ReportCaps::from_user(&u);
    Ok(Json(json!({
        "overview": caps.overview,
        "member_reports_global": caps.member_reports_global,
        "member_badge_icon_edit": caps.member_badge_icon_edit,
        "demand": caps.demand,
        "demand_scope": if !caps.demand { "none" } else if caps.demand_global { "global" } else { "own_events" },
        "events": caps.events,
        "events_scope": if !caps.events { "none" } else if caps.events_global { "global" } else { "own_events" },
        "finance": caps.finance,
        "finance_sections": {
            "reimb_review": caps.reimb_review,
            "reimb_payouts": caps.reimb_pay,
            "bills": caps.bills,
        },
        "membership": caps.membership,
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
    if !BADGE_KEYS.contains(&badge_key.as_str()) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let icon = req.icon.trim().to_string();
    if !icon.is_empty() && (icon.chars().count() > 32 || icon.chars().any(|c| c.is_control())) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let target_roles: Option<String> = sqlx::query_scalar("SELECT roles FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&st.db)
        .await?;
    let Some(target_roles) = target_roles else {
        return Err(not_found(u.lang));
    };
    if user_id != u.id
        && !perms::is_superadmin(&u.roles)
        && !perms::outranks(&u.roles, &perms::parse_roles(&target_roles))
    {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }

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
