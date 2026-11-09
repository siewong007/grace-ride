pub mod admin;
pub mod auth;
pub mod community;
pub mod events;
pub mod files;
pub mod rides;
pub mod settlements;

use axum::Router;

use crate::state::AppState;

pub fn api() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(rides::router())
        .merge(events::router())
        .merge(settlements::router())
        .merge(files::router())
        .merge(community::router())
        .merge(admin::router())
}
