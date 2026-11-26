//! 管理员: dashboard, members, invitations, groups, areas, recovery approvals, audit log.
//! Roles themselves are edited in `routes::roles`.

use axum::{
    extract::{DefaultBodyLimit, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
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
    let today = now()[..10].to_string();
    let count = |sql: &'static str| {
        let db = st.db.clone();
        let today = today.clone();
        async move {
            let (n,): (i64,) = sqlx::query_as(sql).bind(today).fetch_one(&db).await?;
            Ok::<i64, AppError>(n)
        }
    };
    let today_total = count(
        "SELECT COUNT(*) FROM rides WHERE substr(pickup_at,1,10)=$1 AND status NOT IN ('cancelled')",
    )
    .await?;
    let today_confirmed = count("SELECT COUNT(*) FROM rides WHERE substr(pickup_at,1,10)=$1 AND status IN ('confirmed','in_progress','completed')").await?;
    let today_changes = count("SELECT COUNT(*) FROM rides WHERE substr(updated_at,1,10)=$1 AND (needs_attention=1 OR status IN ('cancelled','no_show','alt_transport'))").await?;
    let pending_approval =
        count("SELECT COUNT(*) FROM rides WHERE status='pending_approval' AND $1 != ''").await?;
    let no_driver =
        count("SELECT COUNT(*) FROM rides WHERE status='pending_driver' AND pickup_at >= $1")
            .await?;
    let attention = count("SELECT COUNT(*) FROM rides WHERE needs_attention=1 AND status NOT IN ('cancelled','completed','no_show','alt_transport') AND $1 != ''").await?;
    let (live,): (i64,) =
        sqlx::query_as("SELECT COUNT(DISTINCT driver_id) FROM rides WHERE status='in_progress'")
            .fetch_one(&st.db)
            .await?;
    let (open_help,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM help_tickets WHERE status='open'")
            .fetch_one(&st.db)
            .await?;
    let (pending_reimb,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM reimbursements WHERE status='pending'")
            .fetch_one(&st.db)
            .await?;
    let (members,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE status='active'")
        .fetch_one(&st.db)
        .await?;
    // Members who joined from an invite and are waiting for an admin to assign a role and group.
    let (pending_members,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE status='pending'")
            .fetch_one(&st.db)
            .await?;
    let (recovery,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM recovery_requests WHERE status IN ('pending','awaiting_second')",
    )
    .fetch_one(&st.db)
    .await?;
    // Invites still waiting to be used, and approved claims waiting for finance to pay them:
    // the counts on the Members "Invites" and Reimbursements "Finance payout" tabs.
    let (open_invites,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM invites WHERE kind <> 'family' AND used_at IS NULL AND expires_at >= $1",
    )
    .bind(now())
    .fetch_one(&st.db)
    .await?;
    let (awaiting_payment,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM reimbursements WHERE status='approved'")
            .fetch_one(&st.db)
            .await?;
    // How long the oldest item in each queue has waited, for the work queue on the admin home.
    let (oldest_approval_at, next_no_driver_at): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT (SELECT MIN(created_at) FROM rides WHERE status='pending_approval'),
                (SELECT MIN(pickup_at) FROM rides WHERE status='pending_driver' AND pickup_at >= $1)",
    )
    .bind(&today)
    .fetch_one(&st.db)
    .await?;
    let (help_claimed_by_me,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM help_tickets WHERE status='open' AND assignee_id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let (oldest_help_at,): (Option<String>,) =
        sqlx::query_as("SELECT MIN(updated_at) FROM help_tickets WHERE status='open'")
            .fetch_one(&st.db)
            .await?;
    let max_live_drivers: i64 = sqlx::query_scalar::<_, String>(
        "SELECT value FROM system_settings WHERE key='max_live_drivers'",
    )
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten()
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or(st.cfg.max_live_drivers);
    let (pending_plan_drafts,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM event_plan_drafts WHERE status='pending_approval'")
            .fetch_one(&st.db)
            .await?;
    let (pending_ride_drafts,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ride_change_drafts WHERE status='pending_approval'")
            .fetch_one(&st.db)
            .await?;
    let (my_returned_plan_drafts,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM event_plan_drafts WHERE status='returned' AND (submitted_by=$1 OR created_by=$1)",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    let (my_returned_ride_drafts,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM ride_change_drafts WHERE status='returned' AND (submitted_by=$1 OR created_by=$1)",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    Ok(Json(json!({
        "today": today, "today_total": today_total, "today_confirmed": today_confirmed, "today_changes": today_changes,
        "pending_approval": pending_approval, "no_driver": no_driver, "needs_attention": attention,
        "live_drivers": live, "max_live_drivers": max_live_drivers, "open_help": open_help,
        "pending_reimbursements": pending_reimb, "members": members, "pending_members": pending_members, "pending_recovery": recovery,
        "open_invites": open_invites, "awaiting_payment": awaiting_payment,
        "oldest_approval_at": oldest_approval_at, "next_no_driver_at": next_no_driver_at,
        "oldest_help_at": oldest_help_at, "help_claimed_by_me": help_claimed_by_me,
        "pending_plan_drafts": pending_plan_drafts,
        "pending_ride_drafts": pending_ride_drafts,
        "my_returned_plan_drafts": my_returned_plan_drafts,
        "my_returned_ride_drafts": my_returned_ride_drafts,
    })))
}
