//! 管理员: dashboard, members, invitations, groups, areas, recovery approvals, audit log.
//! Roles themselves are edited in `routes::roles`.

use axum::{
    extract::{DefaultBodyLimit, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{now, require};

mod audit;
pub(crate) mod fares;
mod groups_areas;
mod invites_recovery;
mod members;
mod reports;
mod settings;

pub(crate) use reports::has_global_report_scope;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/dashboard", get(dashboard))
        .route("/admin/users", get(members::list_users))
        .route("/admin/users/{id}", put(members::update_user))
        .route(
            "/admin/users/{id}/badges/{badge_key}/icon",
            put(reports::update_user_badge_icon),
        )
        .route(
            "/admin/invites",
            get(invites_recovery::list_invites).post(invites_recovery::create_invite),
        )
        .route(
            "/admin/invites/{id}",
            delete(invites_recovery::revoke_invite),
        )
        .route("/admin/groups", post(groups_areas::create_group))
        .route(
            "/admin/groups/{id}",
            put(groups_areas::update_group).delete(groups_areas::delete_group),
        )
        .route(
            "/admin/groups/{id}/members",
            get(groups_areas::group_members).put(groups_areas::set_group_members),
        )
        .route("/admin/areas", post(groups_areas::create_area))
        .route("/admin/areas/{id}", put(groups_areas::update_area))
        .route("/admin/recovery", get(invites_recovery::list_recovery))
        .route(
            "/admin/recovery/{id}/approve",
            post(invites_recovery::approve_recovery),
        )
        .route(
            "/admin/recovery/{id}/reject",
            post(invites_recovery::reject_recovery),
        )
        .route("/admin/audit", get(audit::audit_log))
        .route("/admin/audit/export", get(audit::audit_export))
        .route(
            "/admin/settings/fares",
            get(fares::list_fares).put(fares::put_fare),
        )
        .route(
            "/admin/settings/fares/bulk",
            put(fares::bulk_put_fares).layer(DefaultBodyLimit::max(1024 * 1024)),
        )
        .route("/admin/settings/fares/export", get(fares::export_fares_csv))
        .route("/admin/settings/fares/{id}", delete(fares::delete_fare))
        .route(
            "/admin/settings/system",
            get(settings::get_system_settings).put(settings::update_system_settings),
        )
        .route("/admin/reports/capabilities", get(reports::capabilities))
        .route("/admin/reports/overview", get(reports::overview_report))
        .route("/admin/reports/events", get(reports::events_report))
        .route("/admin/reports/demand", get(reports::demand_report))
        .route("/admin/reports/finance", get(reports::finance_report))
        .route(
            "/admin/settings/alerts",
            get(settings::get_alert_settings).put(settings::update_alert_settings),
        )
        .route(
            "/admin/settings/alerts/test-webhook",
            post(settings::test_alert_webhook),
        )
}

async fn dashboard(State(st): State<AppState>, u: CurrentUser) -> AppResult<Json<Value>> {
    require(&u, Page::AdminDashboard, Act::View)?;
    let today_date = crate::util::today_local();
    let today = today_date.format("%Y-%m-%d").to_string();
    let today_start = format!("{today}T");
    let today_end = format!("{today}U");
    let now_str = now();

    #[derive(sqlx::FromRow)]
    struct DashboardCounts {
        today_total: i64,
        today_confirmed: i64,
        today_changes: i64,
        pending_approval: i64,
        no_driver: i64,
        attention: i64,
        live: i64,
        open_help: i64,
        pending_reimb: i64,
        members: i64,
        pending_members: i64,
        recovery: i64,
        open_invites: i64,
        awaiting_payment: i64,
        oldest_approval_at: Option<String>,
        next_no_driver_at: Option<String>,
        help_claimed_by_me: i64,
        oldest_help_at: Option<String>,
        max_live_drivers_val: Option<String>,
        pending_plan_drafts: i64,
        pending_ride_drafts: i64,
        my_returned_plan_drafts: i64,
        my_returned_ride_drafts: i64,
    }

    let c: DashboardCounts = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM rides WHERE pickup_at >= $1 AND pickup_at < $2 AND status NOT IN ('cancelled'))::BIGINT AS today_total,
            (SELECT COUNT(*) FROM rides WHERE pickup_at >= $1 AND pickup_at < $2 AND status IN ('confirmed','in_progress','completed'))::BIGINT AS today_confirmed,
            (SELECT COUNT(*) FROM rides WHERE updated_at >= $1 AND updated_at < $2 AND (needs_attention=1 OR status IN ('cancelled','no_show','alt_transport')))::BIGINT AS today_changes,
            (SELECT COUNT(*) FROM rides WHERE status='pending_approval')::BIGINT AS pending_approval,
            (SELECT COUNT(*) FROM rides WHERE status='pending_driver' AND pickup_at >= $3)::BIGINT AS no_driver,
            (SELECT COUNT(*) FROM rides WHERE needs_attention=1 AND status NOT IN ('cancelled','completed','no_show','alt_transport'))::BIGINT AS attention,
            (SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress')::BIGINT AS live,
            (SELECT COUNT(*) FROM help_tickets WHERE status='open')::BIGINT AS open_help,
            (SELECT COUNT(*) FROM reimbursements WHERE status='pending')::BIGINT AS pending_reimb,
            (SELECT COUNT(*) FROM users WHERE status='active')::BIGINT AS members,
            (SELECT COUNT(*) FROM users WHERE status='pending')::BIGINT AS pending_members,
            (SELECT COUNT(*) FROM recovery_requests WHERE status IN ('pending','awaiting_second'))::BIGINT AS recovery,
            (SELECT COUNT(*) FROM invites WHERE kind <> 'family' AND used_at IS NULL AND expires_at >= $4)::BIGINT AS open_invites,
            (SELECT COUNT(*) FROM reimbursements WHERE status='approved')::BIGINT AS awaiting_payment,
            (SELECT MIN(created_at) FROM rides WHERE status='pending_approval') AS oldest_approval_at,
            (SELECT MIN(pickup_at) FROM rides WHERE status='pending_driver' AND pickup_at >= $3) AS next_no_driver_at,
            (SELECT COUNT(*) FROM help_tickets WHERE status='open' AND assignee_id=$5)::BIGINT AS help_claimed_by_me,
            (SELECT MIN(updated_at) FROM help_tickets WHERE status='open') AS oldest_help_at,
            (SELECT value FROM system_settings WHERE key='max_live_drivers') AS max_live_drivers_val,
            (SELECT COUNT(*) FROM event_plan_drafts WHERE status='pending_approval')::BIGINT AS pending_plan_drafts,
            (SELECT COUNT(*) FROM ride_change_drafts WHERE status='pending_approval')::BIGINT AS pending_ride_drafts,
            (SELECT COUNT(*) FROM event_plan_drafts WHERE status='returned' AND (submitted_by=$5 OR created_by=$5))::BIGINT AS my_returned_plan_drafts,
            (SELECT COUNT(*) FROM ride_change_drafts WHERE status='returned' AND (submitted_by=$5 OR created_by=$5))::BIGINT AS my_returned_ride_drafts",
    )
    .bind(&today_start)
    .bind(&today_end)
    .bind(&today)
    .bind(&now_str)
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;

    let max_live_drivers = c
        .max_live_drivers_val
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(st.cfg.max_live_drivers);

    Ok(Json(json!({
        "today": today, "today_total": c.today_total, "today_confirmed": c.today_confirmed, "today_changes": c.today_changes,
        "pending_approval": c.pending_approval, "no_driver": c.no_driver, "needs_attention": c.attention,
        "live_drivers": c.live, "max_live_drivers": max_live_drivers, "open_help": c.open_help,
        "pending_reimbursements": c.pending_reimb, "members": c.members, "pending_members": c.pending_members, "pending_recovery": c.recovery,
        "open_invites": c.open_invites, "awaiting_payment": c.awaiting_payment,
        "oldest_approval_at": c.oldest_approval_at, "next_no_driver_at": c.next_no_driver_at,
        "oldest_help_at": c.oldest_help_at, "help_claimed_by_me": c.help_claimed_by_me,
        "pending_plan_drafts": c.pending_plan_drafts,
        "pending_ride_drafts": c.pending_ride_drafts,
        "my_returned_plan_drafts": c.my_returned_plan_drafts,
        "my_returned_ride_drafts": c.my_returned_ride_drafts,
    })))
}
