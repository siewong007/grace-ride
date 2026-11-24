//! Notifications, announcements, help center, Grab reimbursements, monthly reports & badges, profile.

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::state::AppState;

mod announcements;
mod calendar;
mod family;
mod guides;
mod help;
mod notifications;
mod payout_batches;
mod profile;
mod push;
mod reimbursements;
mod reports;

pub(crate) use family::FAMILY_CAP;
pub(crate) use guides::any_admin_page;
pub(crate) use push::telegram_webhook_secret;
pub use reports::compute_badges;
pub(crate) use reports::parse_report_month;

/// Escape LIKE wildcards so a search term is matched literally.
pub(super) fn like_pattern(q: &str) -> String {
    let esc: String = q
        .trim()
        .chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect();
    format!("%{esc}%")
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", axum::routing::put(profile::update_profile))
        .route(
            "/me/family",
            get(family::get_family)
                .post(family::create_family)
                .patch(family::rename_family)
                .delete(family::delete_family),
        )
        .route("/me/family/leave", post(family::leave_family))
        .route(
            "/me/family/members/{user_id}",
            delete(family::remove_member),
        )
        .route("/me/family/people", post(family::add_person))
        .route(
            "/me/family/requests/{id}/accept",
            post(family::accept_request),
        )
        .route(
            "/me/family/requests/{id}/decline",
            post(family::decline_request),
        )
        .route("/me/family/requests/{id}", delete(family::cancel_request))
        .route("/me/family/invites/{id}", delete(family::revoke_invite))
        .route(
            "/me/calendar-feed",
            get(calendar::get_calendar_feed).delete(calendar::revoke_calendar_feed),
        )
        .route(
            "/me/calendar-feed/rotate",
            post(calendar::rotate_calendar_feed),
        )
        .route("/calendar/{token}", get(calendar::serve_calendar_ics))
        .route("/push/config", get(push::push_config))
        .route(
            "/push/subscribe",
            post(push::push_subscribe).delete(push::push_unsubscribe),
        )
        .route("/push/subscriptions", get(push::list_subscriptions))
        .route(
            "/push/subscriptions/{id}",
            delete(push::delete_subscription),
        )
        .route(
            "/push/mobile-token",
            post(push::register_mobile_token).delete(push::unregister_mobile_token),
        )
        .route(
            "/push/telegram/link-token",
            post(push::create_telegram_link_token),
        )
        .route(
            "/push/telegram/webhook/{secret}",
            post(push::telegram_webhook),
        )
        .route("/push/telegram/test", post(push::telegram_test))
        .route("/push/test", post(push::push_test))
        .route("/notifications", get(notifications::list_notifications))
        .route(
            "/notifications/preferences",
            get(notifications::get_preferences).put(notifications::update_preferences),
        )
        .route("/notifications/read-all", post(notifications::read_all))
        .route("/notifications/{id}/read", post(notifications::read_one))
        .route(
            "/notifications/{id}/unread",
            post(notifications::unread_one),
        )
        .route("/notifications/{id}", delete(notifications::delete_one))
        .route(
            "/announcements",
            get(announcements::list_announcements).post(announcements::create_announcement),
        )
        .route(
            "/announcements/{id}",
            delete(announcements::delete_announcement).put(announcements::update_announcement),
        )
        .route("/help", get(help::list_tickets).post(help::create_ticket))
        .route("/help/{id}", get(help::get_ticket))
        .route("/help/{id}/replies", post(help::reply_ticket))
        .route("/help/{id}/close", post(help::close_ticket))
        .route("/help/{id}/reopen", post(help::reopen_ticket))
        .route("/help/{id}/claim", post(help::claim_ticket))
        .route("/help/counts", get(help::ticket_counts))
        .route("/help/members", get(help::help_members))
        .route("/help/guides/reorder", post(guides::reorder_guides))
        .route("/help/guides/{id}/feedback", post(guides::guide_feedback))
        .route(
            "/help/guides",
            get(guides::list_guides).post(guides::create_guide),
        )
        .route(
            "/help/guides/{key}",
            get(guides::get_guide)
                .put(guides::update_guide)
                .delete(guides::delete_guide),
        )
        .route(
            "/reimbursements/batches",
            get(payout_batches::list_payout_batches).post(payout_batches::create_payout_batch),
        )
        .route(
            "/reimbursements/batches/{id}",
            delete(payout_batches::cancel_payout_batch),
        )
        .route(
            "/reimbursements/batches/{id}/csv",
            get(payout_batches::export_payout_batch_csv),
        )
        .route(
            "/reimbursements/batches/{id}/pay",
            post(payout_batches::pay_payout_batch),
        )
        .route(
            "/reimbursements",
            get(reimbursements::list_reimbursements).post(reimbursements::create_reimbursement),
        )
        .route(
            "/reimbursements/{id}",
            get(reimbursements::get_reimbursement).put(reimbursements::resubmit_reimbursement),
        )
        .route(
            "/reimbursements/{id}/review",
            post(reimbursements::review_reimbursement),
        )
        .route(
            "/reimbursements/{id}/pay",
            post(reimbursements::pay_reimbursement),
        )
        .route("/reports/monthly", get(reports::monthly_report))
        .route("/badges", get(reports::my_badges))
}
