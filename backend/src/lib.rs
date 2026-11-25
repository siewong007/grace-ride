//! 恩典同行 Grace Ride — library crate. `main.rs` only reads config and serves;
//! everything here is reusable by the integration tests in `tests/`.

// sqlx row tuples (`query_as::<_, (i64, String, ...)>`) trip this lint by design.
#![allow(clippy::type_complexity)]

pub mod arrange;
pub mod audience;
pub mod auth;
pub mod error;
pub mod i18n;
pub mod jobs;
pub mod notifications;
pub mod perms;
pub mod routes;
pub mod routing;
pub mod security;
pub mod seed;
pub mod state;
pub mod stream;
pub mod turnstile;
pub mod util;
pub mod web_push;

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::{from_fn, from_fn_with_state, Next},
    response::Response,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgConnection, PgPoolOptions};
use sqlx::{Connection, Executor, PgPool};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use security::RateLimiter;
use state::{build_webauthn_with_android_certs, AppState, Config, DbConfig};

/// Largest request body for every route except uploads (`routes::files` raises it).
/// The biggest real request, a transport plan or a bill, is a few kilobytes; the cap
/// keeps free-text fields from being used to store megabytes per row.
pub const MAX_JSON_BODY: usize = 128 * 1024;

/// Connect with the default pool settings and time zone (tests, tools).
pub async fn connect_db(url: &str) -> anyhow::Result<PgPool> {
    connect_db_with(url, &DbConfig::default()).await
}

/// Run migrations on a connection of their own (no statement timeout: building an
/// index may take a while), then open the pool every request uses.
pub async fn connect_db_with(url: &str, cfg: &DbConfig) -> anyhow::Result<PgPool> {
    let opts = PgConnectOptions::from_str(url)?;
    let session = session_settings(cfg);

    let mut conn = PgConnection::connect_with(&opts).await?;
    conn.execute(sqlx::AssertSqlSafe(set_time_zone(cfg)))
        .await?;
    clear_squashed_migration_history(&mut conn).await?;
    sqlx::migrate!("./migrations").run(&mut conn).await?;
    sync_timezone(&mut conn, cfg).await?;
    conn.close().await?;

    let db = PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(cfg.acquire_timeout_secs))
        .after_connect(move |conn, _meta| {
            let session = session.clone();
            Box::pin(async move {
                conn.execute(sqlx::AssertSqlSafe(session)).await?;
                Ok(())
            })
        })
        .connect_with(opts)
        .await?;
    Ok(db)
}

/// sqlx opens every connection with `TimeZone=UTC`. Stored times are local text in
/// the app's zone, so the session uses that zone too: `LOCALTIMESTAMP`, `CURRENT_DATE`
/// and `now()::text` then agree with the times the app writes.
fn set_time_zone(cfg: &DbConfig) -> String {
    format!(
        "SET TIME ZONE '{}'",
        cfg.timezone.name().replace('\'', "''")
    )
}

/// Statements run on every new pool connection.
fn session_settings(cfg: &DbConfig) -> String {
    format!(
        "{}; SET statement_timeout = {}; SET idle_in_transaction_session_timeout = 60000",
        set_time_zone(cfg),
        cfg.statement_timeout_secs * 1000
    )
}

/// `system_settings.timezone` is the zone the database's own defaults (`app_now()`)
/// write in. It follows `APP_TIMEZONE`. Changing the zone does not rewrite times
/// already stored, so a change on a database with members is logged loudly.
async fn sync_timezone(conn: &mut PgConnection, cfg: &DbConfig) -> anyhow::Result<()> {
    let zone = cfg.timezone.name();
    let stored: Option<String> =
        sqlx::query_scalar("SELECT value FROM system_settings WHERE key = 'timezone'")
            .fetch_optional(&mut *conn)
            .await?;
    if stored.as_deref() == Some(zone) {
        return Ok(());
    }
    // app_now() runs on every insert, so a zone Postgres doesn't know would break
    // them all. Fail here instead.
    sqlx::query("SELECT now() AT TIME ZONE $1")
        .bind(zone)
        .execute(&mut *conn)
        .await
        .map_err(|e| anyhow::anyhow!("Postgres does not know APP_TIMEZONE {zone}: {e}"))?;
    if let Some(old) = &stored {
        let members: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&mut *conn)
            .await?;
        if members > 0 {
            tracing::warn!(
                "APP_TIMEZONE changed from {old} to {zone}. Times already stored were written in {old} and are not converted."
            );
        }
    }
    sqlx::query(
        "INSERT INTO system_settings (key, value, updated_at) VALUES ('timezone', $1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
    )
    .bind(zone)
    .bind(util::ts(chrono::Utc::now().with_timezone(&cfg.timezone)))
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// What the recorded history says about this build's migration files.
#[derive(Debug, PartialEq, Eq)]
enum History {
    /// Every applied file is the start of this build's files; sqlx runs the rest.
    Current,
    /// Written by a schema that has since been squashed into this build's `0001`
    /// (its version 1 differs). `0001` is repeatable, so the history is cleared and
    /// it runs again over the existing tables, keeping every row.
    Squashed,
    /// Version 1 matches but a later file was edited after it ran, or the database
    /// ran a file this build doesn't ship (a newer build). Refuse to start.
    Diverged(i64),
}

fn classify_history(applied: &[(i64, Vec<u8>)], migrator: &sqlx::migrate::Migrator) -> History {
    let shipped: Vec<_> = migrator
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .collect();
    let first_matches = match (applied.first(), shipped.first()) {
        (None, _) => return History::Current,
        (Some((version, checksum)), Some(first)) => {
            *version == first.version && first.checksum.as_ref() == checksum.as_slice()
        }
        (Some(_), None) => false,
    };
    if !first_matches {
        return History::Squashed;
    }
    // Matched by version, not position: two branches may each add a numbered file, and
    // the later number can reach the database first (e.g. 0023 before 0022). sqlx then
    // runs the missing lower file on the next start. What must never happen is a file
    // that ran being edited, or one this build doesn't ship.
    for (version, checksum) in applied {
        let same = shipped.iter().any(|migration| {
            migration.version == *version && migration.checksum.as_ref() == checksum.as_slice()
        });
        if !same {
            return History::Diverged(*version);
        }
    }
    History::Current
}

async fn clear_squashed_migration_history(conn: &mut PgConnection) -> anyhow::Result<()> {
    let ready: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = current_schema() AND table_name = '_sqlx_migrations'
        )",
    )
    .fetch_one(&mut *conn)
    .await?;
    if !ready {
        return Ok(());
    }
    let applied: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&mut *conn)
            .await?;
    match classify_history(&applied, &sqlx::migrate!("./migrations")) {
        History::Current => Ok(()),
        History::Squashed => {
            tracing::warn!(
                "Migration history ({} row(s)) is from an older schema; replaying 0001_init.sql over the existing tables",
                applied.len()
            );
            sqlx::query("DELETE FROM _sqlx_migrations")
                .execute(&mut *conn)
                .await?;
            Ok(())
        }
        History::Diverged(version) => anyhow::bail!(
            "Migration {version:04} on this database does not match this build: a migration file was edited after it ran, \
             or the database was migrated by a newer build. Put schema changes in a new numbered file."
        ),
    }
}

/// Build shared state: database, migrations, first-run data, WebAuthn.
pub async fn init(cfg: Config) -> anyhow::Result<AppState> {
    std::fs::create_dir_all(&cfg.upload_dir)?;
    util::set_timezone(cfg.db.timezone);
    util::set_dev_mode(cfg.dev_mode);
    util::set_public_url(&cfg.public_url);
    let db = connect_db_with(&cfg.database_url, &cfg.db).await?;
    // Built-in roles and their default permissions must exist before anyone signs in.
    perms::sync(&db).await?;
    seed::run(&db, &cfg).await?;
    web_push::ensure_vapid_keys(&db).await?;
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
    let webauthn = Arc::new(build_webauthn_with_android_certs(
        &cfg.public_url,
        &cfg.android_cert_sha256_fingerprints,
    )?);
    let turnstile = Arc::new(turnstile::TurnstileVerifier::new(
        cfg.turnstile.clone(),
        &cfg.public_url,
    ));
    let (shutdown, _) = tokio::sync::watch::channel(false);
    Ok(AppState {
        db,
        webauthn,
        cfg: Arc::new(cfg),
        limiter: Arc::new(RateLimiter::default()),
        routing: Arc::new(routing::RoutingState::default()),
        turnstile,
        events: stream::channel(),
        shutdown,
    })
}

async fn health(State(st): State<AppState>) -> (StatusCode, Json<Value>) {
    let db_ok = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").execute(&st.db),
    )
    .await
    .is_ok_and(|r| r.is_ok());
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
        Json(json!({
            "error": crate::util::t(
                crate::i18n::Lang::from_headers(&headers),
                "找不到此功能",
                "Not found",
            ),
            "code": "NOT_FOUND",
        })),
    )
}

async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    static REQ_ID: HeaderName = HeaderName::from_static("x-request-id");
    let incoming = req
        .headers()
        .get(&REQ_ID)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
        })
        .and_then(|s| HeaderValue::from_str(s).ok());
    let id_val = incoming.unwrap_or_else(|| {
        HeaderValue::from_str(&uuid::Uuid::new_v4().to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("req"))
    });
    req.headers_mut().insert(REQ_ID.clone(), id_val.clone());
    let req_lang = crate::i18n::Lang::from_headers(req.headers());
    let mut res = crate::error::scope_lang(req_lang, next.run(req)).await;
    res.headers_mut().insert(REQ_ID.clone(), id_val);
    res
}

async fn assetlinks_json(State(st): State<AppState>) -> Json<Value> {
    if st.cfg.android_cert_sha256_fingerprints.is_empty() {
        return Json(json!([]));
    }
    Json(json!([{
        "relation": [
            "delegate_permission/common.handle_all_urls",
            "delegate_permission/common.get_login_creds"
        ],
        "target": {
            "namespace": "android_app",
            "package_name": "org.pjstatecm.graceride",
            "sha256_cert_fingerprints": st.cfg.android_cert_sha256_fingerprints,
        }
    }]))
}

pub fn router(state: AppState) -> Router {
    let api = routes::api()
        .route("/stream", get(stream::handler))
        .route("/health", get(health))
        .fallback(api_not_found)
        .layer(from_fn_with_state(state.clone(), security::session_guard));
    let mut app = Router::new()
        .nest("/api", api)
        .route("/.well-known/assetlinks.json", get(assetlinks_json));

    // In production the compiled React app is served from the same origin.
    if let Some(dir) = &state.cfg.static_dir {
        app = app
            .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html"))));
    }

    // Layers run bottom-up on the way in: request_id → trace → headers → compression → body limit → origin → rate limit.
    app.layer(from_fn_with_state(state.clone(), security::rate_limit))
        .layer(from_fn_with_state(state.clone(), security::origin_guard))
        .layer(DefaultBodyLimit::max(MAX_JSON_BODY))
        .layer(CompressionLayer::new())
        .layer(from_fn_with_state(
            state.clone(),
            security::security_headers,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(from_fn(request_id_middleware))
        .with_state(state)
}

#[cfg(test)]
mod history_tests {
    use super::{classify_history, History};

    fn shipped() -> Vec<(i64, Vec<u8>)> {
        sqlx::migrate!("./migrations")
            .iter()
            .map(|m| (m.version, m.checksum.to_vec()))
            .collect()
    }

    #[test]
    fn current_history_and_empty_history_are_kept() {
        let migrator = sqlx::migrate!("./migrations");
        assert_eq!(classify_history(&[], &migrator), History::Current);
        assert_eq!(classify_history(&shipped(), &migrator), History::Current);
    }

    #[test]
    fn an_older_squashed_schema_is_replayed() {
        let migrator = sqlx::migrate!("./migrations");
        // The five files from before the November 2026 squash.
        let old: Vec<(i64, Vec<u8>)> = (1..=5).map(|v| (v, vec![7u8; 48])).collect();
        assert_eq!(classify_history(&old, &migrator), History::Squashed);
        assert_eq!(
            classify_history(&[(16, vec![1])], &migrator),
            History::Squashed
        );
    }

    #[test]
    fn an_edited_initial_file_replays_as_an_older_schema() {
        let migrator = sqlx::migrate!("./migrations");
        // With everything consolidated into 0001, a database whose 0001 differs
        // (an older freeze, or a sibling branch's numbering) replays the whole file
        // over its existing tables instead of refusing to start.
        let mut edited = shipped();
        edited[0].1 = vec![0u8; 48];
        assert_eq!(classify_history(&edited, &migrator), History::Squashed);
        // A database migrated by a newer build (extra numbered files) still stops.
        let mut newer = shipped();
        newer.push((2, vec![2u8; 48]));
        assert_eq!(classify_history(&newer, &migrator), History::Diverged(2));
    }

    #[test]
    fn an_edited_or_unknown_later_file_is_refused() {
        let migrator = sqlx::migrate!("./migrations");
        let mut applied = shipped();
        applied.push((99, vec![0u8; 48]));
        assert_eq!(classify_history(&applied, &migrator), History::Diverged(99));
    }
}
