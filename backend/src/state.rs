use std::{path::PathBuf, sync::Arc};

use chrono_tz::Tz;

use sqlx::PgPool;
use tokio::sync::broadcast;
use url::Url;
use webauthn_rs::prelude::*;

use crate::{
    routing::{RoutingConfig, RoutingState},
    security::RateLimiter,
    stream::Push,
};

#[derive(Clone, Debug)]
pub struct Config {
    /// When true, demo data is loaded and password sessions count as 2FA-verified.
    /// Refused unless PUBLIC_URL points at localhost.
    pub dev_mode: bool,
    pub database_url: String,
    pub upload_dir: PathBuf,
    pub static_dir: Option<PathBuf>,
    pub public_url: String,
    pub bind: String,
    /// Maximum number of drivers sharing live location at the same time.
    pub max_live_drivers: i64,
    /// Trust the first X-Forwarded-For hop for client IPs (only behind our own reverse proxy).
    pub trust_proxy: bool,
    /// Days to keep trip chat messages after the trip ends (资料保存期限).
    pub chat_retention_days: i64,
    /// Days to keep audit log entries.
    pub audit_retention_days: i64,
    /// Default inactivity timeout in minutes before auto-locking member session (0 = disabled).
    pub session_timeout_mins: i64,
    /// Emit JSON logs (for log shippers).
    pub json_logs: bool,
    /// Google OAuth client id (empty ⇒ Google sign-in is disabled).
    pub google_client_id: Option<String>,
    /// Google OAuth client secret (empty ⇒ Google sign-in is disabled).
    pub google_client_secret: Option<String>,
    /// Pickup routing and ETAs (OpenStreetMap geocoder + router).
    pub routing: RoutingConfig,
    /// Database connection pool and session settings, including the time zone.
    pub db: DbConfig,
    /// Minutes between clean-up runs (expired sessions, old chats and audit rows).
    /// Reminders still run every minute.
    pub housekeeping_interval_mins: i64,
    /// Days to keep notifications once read. Unread ones are kept for a year, or this
    /// long if it is longer.
    pub notification_retention_days: i64,
}

/// How the app talks to Postgres.
#[derive(Clone, Debug)]
pub struct DbConfig {
    /// IANA zone (`APP_TIMEZONE`) that stored times are local to. It is also the
    /// session `TimeZone`, so `LOCALTIMESTAMP` and `CURRENT_DATE` agree with the app.
    pub timezone: Tz,
    /// Most connections the pool opens (`DB_MAX_CONNECTIONS`). Keep it under the
    /// server's `max_connections` (20 on the production host).
    pub max_connections: u32,
    /// Seconds a request waits for a free connection before failing (`DB_ACQUIRE_TIMEOUT_SECS`).
    pub acquire_timeout_secs: u64,
    /// Longest any single statement may run, in seconds (`DB_STATEMENT_TIMEOUT_SECS`, 0 = no limit).
    /// Migrations run on their own connection without this limit.
    pub statement_timeout_secs: u64,
}

impl Default for DbConfig {
    fn default() -> Self {
        DbConfig {
            timezone: crate::util::DEFAULT_TIMEZONE,
            max_connections: 10,
            acquire_timeout_secs: 5,
            statement_timeout_secs: 15,
        }
    }
}

impl DbConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let d = DbConfig::default();
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let num = |k: &str, d: u64| -> anyhow::Result<u64> {
            var(k).map_or(Ok(d), |v| {
                v.trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("{k} must be a whole number"))
            })
        };
        let timezone = match var("APP_TIMEZONE") {
            Some(v) => v.trim().parse::<Tz>().map_err(|_| {
                anyhow::anyhow!(
                    "APP_TIMEZONE must be an IANA zone such as Asia/Kuala_Lumpur (got {v})"
                )
            })?,
            None => d.timezone,
        };
        Ok(DbConfig {
            timezone,
            max_connections: u32::try_from(num("DB_MAX_CONNECTIONS", d.max_connections.into())?)
                .map_err(|_| anyhow::anyhow!("DB_MAX_CONNECTIONS is too large"))?,
            acquire_timeout_secs: num("DB_ACQUIRE_TIMEOUT_SECS", d.acquire_timeout_secs)?,
            statement_timeout_secs: num("DB_STATEMENT_TIMEOUT_SECS", d.statement_timeout_secs)?,
        })
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if !(1..=100).contains(&self.max_connections) {
            anyhow::bail!("DB_MAX_CONNECTIONS must be between 1 and 100");
        }
        if !(1..=120).contains(&self.acquire_timeout_secs) {
            anyhow::bail!("DB_ACQUIRE_TIMEOUT_SECS must be between 1 and 120");
        }
        if self.statement_timeout_secs > 3600 {
            anyhow::bail!("DB_STATEMENT_TIMEOUT_SECS must be at most 3600 (0 = no limit)");
        }
        Ok(())
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        let num = |k: &str, d: i64| -> anyhow::Result<i64> {
            env(k, &d.to_string())
                .parse()
                .map_err(|_| anyhow::anyhow!("{k} must be a number"))
        };
        let cfg = Config {
            dev_mode: env("DEV_MODE", "false") == "true",
            database_url: env("DATABASE_URL", "postgres://localhost/grace_ride_dev"),
            upload_dir: PathBuf::from(env("UPLOAD_DIR", "./data/uploads")),
            static_dir: std::env::var("STATIC_DIR")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
            public_url: env("PUBLIC_URL", "http://localhost:5173")
                .trim_end_matches('/')
                .to_string(),
            bind: env("BIND", "127.0.0.1:8080"),
            max_live_drivers: num("MAX_LIVE_DRIVERS", 10)?,
            trust_proxy: env("TRUST_PROXY", "false") == "true",
            chat_retention_days: num("CHAT_RETENTION_DAYS", 180)?,
            audit_retention_days: num("AUDIT_RETENTION_DAYS", 730)?,
            session_timeout_mins: num("SESSION_TIMEOUT_MINS", 15)?,
            json_logs: env("LOG_FORMAT", "text") == "json",
            google_client_id: {
                let v = env("GOOGLE_CLIENT_ID", "");
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            google_client_secret: {
                let v = env("GOOGLE_CLIENT_SECRET", "");
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            routing: RoutingConfig::from_env(),
            db: DbConfig::from_env()?,
            housekeeping_interval_mins: num("HOUSEKEEPING_INTERVAL_MINS", 60)?,
            notification_retention_days: num("NOTIFICATION_RETENTION_DAYS", 90)?,
        };
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let url = Url::parse(&self.public_url)
            .map_err(|_| anyhow::anyhow!("PUBLIC_URL is not a valid URL"))?;
        let host = url.host_str().unwrap_or_default();
        let local =
            matches!(host, "localhost" | "127.0.0.1" | "[::1]") || host.ends_with(".localhost");
        if self.dev_mode && !local {
            anyhow::bail!("DEV_MODE=true is only allowed when PUBLIC_URL is localhost (got {host}). Demo accounts must never reach production.");
        }
        if !local && url.scheme() != "https" {
            anyhow::bail!("PUBLIC_URL must use https:// outside localhost (passkeys and secure cookies require it).");
        }
        if !(1..=100).contains(&self.max_live_drivers) {
            anyhow::bail!("MAX_LIVE_DRIVERS must be between 1 and 100");
        }
        self.db.validate()?;
        if !(1..=1440).contains(&self.housekeeping_interval_mins) {
            anyhow::bail!("HOUSEKEEPING_INTERVAL_MINS must be between 1 and 1440");
        }
        if !(7..=3650).contains(&self.notification_retention_days) {
            anyhow::bail!("NOTIFICATION_RETENTION_DAYS must be between 7 and 3650");
        }
        if self.routing.enabled {
            for (k, v) in [
                ("OSRM_URL", &self.routing.osrm_url),
                ("NOMINATIM_URL", &self.routing.nominatim_url),
                ("PHOTON_URL", &self.routing.photon_url),
            ] {
                Url::parse(v).map_err(|_| anyhow::anyhow!("{k} is not a valid URL"))?;
            }
        }
        Ok(())
    }

    pub fn secure_cookies(&self) -> bool {
        self.public_url.starts_with("https://")
    }

    /// Origin (scheme://host[:port]) that browsers send in the Origin header.
    pub fn origin(&self) -> String {
        Url::parse(&self.public_url)
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_else(|_| self.public_url.clone())
    }

    /// Callback URL that Google redirects to after the user signs in.
    pub fn google_redirect_uri(&self) -> String {
        format!("{}/api/auth/google/callback", self.origin())
    }

    /// True when Google sign-in is configured (both client id and secret set).
    pub fn google_configured(&self) -> bool {
        self.google_client_id.is_some() && self.google_client_secret.is_some()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub webauthn: Arc<Webauthn>,
    pub cfg: Arc<Config>,
    pub limiter: Arc<RateLimiter>,
    pub http_client: reqwest::Client,
    /// Route and geocoder caches for pickup routing.
    pub routing: Arc<RoutingState>,
    /// Live updates for open tabs (`/api/stream`).
    pub events: broadcast::Sender<Push>,
}

pub fn build_webauthn(public_url: &str) -> anyhow::Result<Webauthn> {
    let origin = Url::parse(public_url)?;
    let rp_id = origin
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("PUBLIC_URL has no host"))?
        .to_string();
    let w = WebauthnBuilder::new(&rp_id, &origin)?
        .rp_name("恩典同行 Grace Ride")
        .build()?;
    Ok(w)
}
