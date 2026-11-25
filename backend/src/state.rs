use std::{path::PathBuf, sync::Arc};

use chrono_tz::Tz;
use sqlx::PgPool;
use tokio::sync::watch;
use url::Url;
use webauthn_rs::prelude::*;

use crate::{
    routing::{RoutingConfig, RoutingState},
    security::RateLimiter,
    stream::EventHub,
    turnstile::{TurnstileConfig, TurnstileVerifier},
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
    /// Optional Cloudflare Turnstile verification on public web auth/onboarding/visitor flows.
    pub turnstile: TurnstileConfig,
    /// Optional comma-separated SHA-256 signing certificate fingerprints (colon-hex)
    /// for native Android passkeys (`android:apk-key-hash:` + `/.well-known/assetlinks.json`).
    pub android_cert_sha256_fingerprints: Vec<String>,
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
        let env = |k: &str, d: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| d.to_string())
        };
        let num = |k: &str, d: i64| -> anyhow::Result<i64> {
            env(k, &d.to_string())
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("{k} must be a number"))
        };
        let cfg = Config {
            dev_mode: env("DEV_MODE", "false") == "true",
            database_url: std::env::var("DATABASE_URL")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| "postgres://localhost/grace_ride_dev".to_string()),
            upload_dir: PathBuf::from(env("UPLOAD_DIR", "./data/uploads")),
            static_dir: std::env::var("STATIC_DIR")
                .ok()
                .filter(|s| !s.trim().is_empty())
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
            routing: RoutingConfig::from_env(),
            db: DbConfig::from_env()?,
            housekeeping_interval_mins: num("HOUSEKEEPING_INTERVAL_MINS", 60)?,
            notification_retention_days: num("NOTIFICATION_RETENTION_DAYS", 90)?,
            turnstile: TurnstileConfig::from_env(),
            android_cert_sha256_fingerprints: parse_android_fingerprints(&env(
                "ANDROID_CERT_SHA256_FINGERPRINTS",
                "",
            ))?,
        };
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.database_url.trim().is_empty() {
            anyhow::bail!("DATABASE_URL must not be empty");
        }
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
        if !(7..=3650).contains(&self.chat_retention_days) {
            anyhow::bail!("CHAT_RETENTION_DAYS must be between 7 and 3650");
        }
        if !(30..=3650).contains(&self.audit_retention_days) {
            anyhow::bail!("AUDIT_RETENTION_DAYS must be between 30 and 3650");
        }
        if !(0..=1440).contains(&self.session_timeout_mins) {
            anyhow::bail!("SESSION_TIMEOUT_MINS must be between 0 and 1440");
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
        self.turnstile.validate(local)?;
        for fp in &self.android_cert_sha256_fingerprints {
            android_apk_key_hash_origin(fp)?;
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
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub webauthn: Arc<Webauthn>,
    pub cfg: Arc<Config>,
    pub limiter: Arc<RateLimiter>,
    /// Route and geocoder caches for pickup routing.
    pub routing: Arc<RoutingState>,
    /// Cloudflare Turnstile verifier for public web forms.
    pub turnstile: Arc<TurnstileVerifier>,
    /// Live updates for open tabs (`/api/stream`).
    pub events: EventHub,
    /// Signals open SSE streams and background tasks to drain on `SIGTERM`/`SIGINT`.
    pub shutdown: watch::Sender<bool>,
}

pub fn normalize_android_cert_fingerprint(raw: &str) -> anyhow::Result<String> {
    let hex_only: String = raw.trim().chars().filter(|c| *c != ':').collect();
    if hex_only.len() != 64 || !hex_only.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!(
            "ANDROID_CERT_SHA256_FINGERPRINTS entries must be 32-byte hex SHA-256 fingerprints"
        );
    }
    let upper = hex_only.to_ascii_uppercase();
    let pairs: Vec<&str> = (0..upper.len())
        .step_by(2)
        .map(|i| &upper[i..i + 2])
        .collect();
    Ok(pairs.join(":"))
}

fn parse_android_fingerprints(raw: &str) -> anyhow::Result<Vec<String>> {
    let mut out = Vec::new();
    for part in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let norm = normalize_android_cert_fingerprint(part)?;
        if !out.contains(&norm) {
            out.push(norm);
        }
    }
    Ok(out)
}

pub fn android_apk_key_hash_origin(fingerprint: &str) -> anyhow::Result<Url> {
    use base64::Engine as _;
    let hex_only: String = fingerprint.chars().filter(|c| *c != ':').collect();
    let bytes = hex::decode(hex_only)?;
    if bytes.len() != 32 {
        anyhow::bail!("Android SHA-256 fingerprint must be 32 bytes");
    }
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes);
    Ok(Url::parse(&format!("android:apk-key-hash:{b64}"))?)
}

pub fn build_webauthn_with_android_certs(
    public_url: &str,
    android_certs: &[String],
) -> anyhow::Result<Webauthn> {
    let origin = Url::parse(public_url)?;
    let rp_id = origin
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("PUBLIC_URL has no host"))?
        .to_string();
    let mut builder = WebauthnBuilder::new(&rp_id, &origin)?.rp_name("恩典同行 Grace Ride");
    for fp in android_certs {
        let apk_origin = android_apk_key_hash_origin(fp)?;
        builder = builder.append_allowed_origin(&apk_origin);
    }
    Ok(builder.build()?)
}

pub fn build_webauthn(public_url: &str) -> anyhow::Result<Webauthn> {
    build_webauthn_with_android_certs(public_url, &[])
}
