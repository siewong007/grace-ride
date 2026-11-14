//! 恩典同行 Grace Ride — library crate. `main.rs` only reads config and serves;
//! everything here is reusable by the integration tests in `tests/`.

// SQL rows are read into tuples next to the query that defines them; naming every
// tuple as a type alias would hide the column list from the reader.
#![allow(clippy::type_complexity)]

pub mod arrange;
pub mod auth;
pub mod error;
pub mod i18n;
pub mod jobs;
pub mod perms;
pub mod routes;
pub mod routing;
pub mod security;
pub mod seed;
pub mod state;
pub mod stream;
pub mod util;

use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::postgres::PgConnectOptions;
use sqlx::PgPool;
use std::str::FromStr;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use security::RateLimiter;
use state::{build_webauthn, AppState, Config};

/// Largest request body for every route except uploads (`routes::files` raises it).
/// The biggest real request, a transport plan or a bill, is a few kilobytes; the cap
/// keeps free-text fields from being used to store megabytes per row.
pub const MAX_JSON_BODY: usize = 128 * 1024;

pub async fn connect_db(url: &str) -> anyhow::Result<PgPool> {
    let opts = PgConnectOptions::from_str(url)?;
    let db = PgPool::connect_with(opts).await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    Ok(db)
}

/// Build shared state: database, migrations, first-run data, WebAuthn.
pub async fn init(cfg: Config) -> anyhow::Result<AppState> {
    std::fs::create_dir_all(&cfg.upload_dir)?;
    let db = connect_db(&cfg.database_url).await?;
    // Built-in roles and their default permissions must exist before anyone signs in.
    perms::sync(&db).await?;
    seed::run(&db, &cfg).await?;
    // There is always one super admin: on a first run, or on a database from before
    // the role existed, the original administrator becomes it.
    let first_admin = std::env::var("ADMIN_EMAIL").ok();
    if let Some((id, email)) = perms::ensure_superadmin(&db, first_admin.as_deref()).await? {
        tracing::warn!("No super admin existed; {email} now holds the role.");
        util::audit(
            &db,
            None,
            "role.bootstrap",
            "user",
            Some(id),
            "superadmin given to the original administrator at start-up",
        )
        .await;
    }
    let webauthn = Arc::new(build_webauthn(&cfg.public_url)?);
    Ok(AppState {
        db,
        webauthn,
        cfg: Arc::new(cfg),
        limiter: Arc::new(RateLimiter::default()),
        http_client: reqwest::Client::new(),
        routing: Arc::new(routing::RoutingState::default()),
        events: stream::channel(),
    })
}

async fn health(State(st): State<AppState>) -> (StatusCode, Json<Value>) {
    let db_ok = sqlx::query("SELECT 1").execute(&st.db).await.is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(
            json!({ "status": if db_ok { "ok" } else { "degraded" }, "db": db_ok, "version": env!("CARGO_PKG_VERSION") }),
        ),
    )
}

async fn api_not_found(headers: axum::http::HeaderMap) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": crate::util::t(
            crate::util::Lang::from_accept_language(
                headers
                    .get(axum::http::header::ACCEPT_LANGUAGE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
            ),
            "找不到此功能",
            "Not found",
        ) })),
    )
}

pub fn router(state: AppState) -> Router {
    let api = routes::api()
        .route("/stream", get(stream::handler))
        .route("/health", get(health))
        .fallback(api_not_found)
        .layer(from_fn_with_state(state.clone(), security::pending_guard));
    let mut app = Router::new().nest("/api", api);

    // In production the compiled React app is served from the same origin.
    if let Some(dir) = &state.cfg.static_dir {
        app = app
            .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html"))));
    }

    // Layers run bottom-up on the way in: trace → headers → compression → body limit → origin → rate limit.
    app.layer(from_fn_with_state(state.clone(), security::rate_limit))
        .layer(from_fn_with_state(state.clone(), security::origin_guard))
        .layer(DefaultBodyLimit::max(MAX_JSON_BODY))
        .layer(CompressionLayer::new())
        .layer(from_fn_with_state(
            state.clone(),
            security::security_headers,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
