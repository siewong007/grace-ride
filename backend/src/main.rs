//! 恩典同行 Grace Ride — API server entry point.

use std::net::SocketAddr;

use grace_ride::{init, jobs, router, state::Config};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    // `grace-ride healthcheck` is used by the container HEALTHCHECK (no curl in the image).
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return healthcheck().await;
    }

    // Sentry: opt-in error and panic monitoring. Set SENTRY_DSN to enable.
    // Never log personal data (names, phone numbers, addresses) in Sentry events.
    // The guard must stay alive for the duration of main to flush events on exit.
    // ClientOptions is `#[non_exhaustive]`, so set the release on a default value instead
    // of building the struct literally. Performance tracing stays off (the default).
    let mut sentry_options = sentry::ClientOptions::default();
    sentry_options.release = sentry::release_name!();
    let _sentry = sentry::init((
        std::env::var("SENTRY_DSN").unwrap_or_default(),
        sentry_options,
    ));

    let cfg = Config::from_env()?;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "grace_ride=info,tower_http=info".into());
    if cfg.json_logs {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
    if cfg.dev_mode {
        tracing::warn!("DEV_MODE is on: demo accounts are on the login page and do not need a password. Never use in production.");
    }

    let state = init(cfg.clone()).await?;
    let shutdown_tx = state.shutdown.clone();
    jobs::spawn_with_shutdown(state.db.clone(), cfg.clone(), shutdown_tx.subscribe());
    let app = router(state);

    let addr: SocketAddr = cfg.bind.parse()?;
    tracing::info!(
        "Grace Ride API listening on http://{addr} (public url {}, dev_mode={})",
        cfg.public_url,
        cfg.dev_mode
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown(shutdown_tx))
    .await?;

    tracing::info!("shut down cleanly");
    Ok(())
}

async fn healthcheck() -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let port = bind.rsplit(':').next().unwrap_or("8080");
    let mut s = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")),
    )
    .await??;
    s.write_all(b"GET /api/health HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .await?;
    let mut buf = Vec::new();
    tokio::time::timeout(std::time::Duration::from_secs(3), s.read_to_end(&mut buf)).await??;
    let head = String::from_utf8_lossy(&buf);
    if head.starts_with("HTTP/1.0 200") || head.starts_with("HTTP/1.1 200") {
        Ok(())
    } else {
        anyhow::bail!("unhealthy: {}", head.lines().next().unwrap_or(""))
    }
}

async fn shutdown(shutdown_tx: tokio::sync::watch::Sender<bool>) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
    tracing::info!("shutdown signal received");
    let _ = shutdown_tx.send(true);
}
