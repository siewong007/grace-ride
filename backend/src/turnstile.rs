//! Cloudflare Turnstile server-side verification (`siteverify`).
//!
//! Opt-in via `TURNSTILE_SITE_KEY` + `TURNSTILE_SECRET_KEY`. When unset, checks are a no-op
//! so local development, CI, and self-hosted setups without Turnstile work unchanged.

use std::{fmt, time::Duration};

use serde::Deserialize;
use serde_json::json;
use url::Url;

use crate::error::{bad, AppResult};
use crate::i18n::ErrorKey;
use crate::util::Lang;

pub const DEFAULT_VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const MAX_TOKEN_LEN: usize = 2048;

/// Official Cloudflare Turnstile dummy test keys (never allowed in non-local production).
const DUMMY_SITE_KEYS: &[&str] = &[
    "1x00000000000000000000AA",
    "2x00000000000000000000AB",
    "1x00000000000000000000BB",
    "2x00000000000000000000BB",
    "3x00000000000000000000FF",
    "3x0000000000000000000FF",
];

const DUMMY_SECRET_KEYS: &[&str] = &[
    "1x0000000000000000000000000000000AA",
    "2x0000000000000000000000000000000AA",
    "3x0000000000000000000000000000000AA",
];

#[derive(Clone)]
pub struct TurnstileConfig {
    pub site_key: Option<String>,
    pub secret_key: Option<String>,
    pub verify_url: String,
}

impl fmt::Debug for TurnstileConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TurnstileConfig")
            .field("site_key", &self.site_key)
            .field(
                "secret_key",
                &self.secret_key.as_ref().map(|_| "[redacted]"),
            )
            .field("verify_url", &self.verify_url)
            .finish()
    }
}

impl Default for TurnstileConfig {
    fn default() -> Self {
        Self {
            site_key: None,
            secret_key: None,
            verify_url: DEFAULT_VERIFY_URL.to_string(),
        }
    }
}

impl TurnstileConfig {
    pub fn from_env() -> Self {
        let opt = |k: &str| {
            std::env::var(k)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Self {
            site_key: opt("TURNSTILE_SITE_KEY"),
            secret_key: opt("TURNSTILE_SECRET_KEY"),
            verify_url: opt("TURNSTILE_VERIFY_URL")
                .unwrap_or_else(|| DEFAULT_VERIFY_URL.to_string()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.site_key.is_some() && self.secret_key.is_some()
    }

    pub fn is_dummy_secret(&self) -> bool {
        self.secret_key
            .as_deref()
            .is_some_and(|s| DUMMY_SECRET_KEYS.contains(&s))
    }

    pub fn validate(&self, public_url_is_local: bool) -> anyhow::Result<()> {
        if self.site_key.is_some() != self.secret_key.is_some() {
            anyhow::bail!(
                "TURNSTILE_SITE_KEY and TURNSTILE_SECRET_KEY must both be set or both be unset"
            );
        }
        if self.is_enabled() {
            let parsed = Url::parse(&self.verify_url)
                .map_err(|_| anyhow::anyhow!("TURNSTILE_VERIFY_URL is not a valid URL"))?;
            if !public_url_is_local && parsed.scheme() != "https" {
                anyhow::bail!("TURNSTILE_VERIFY_URL must use https:// outside localhost");
            }
            let dummy_site = self
                .site_key
                .as_deref()
                .is_some_and(|k| DUMMY_SITE_KEYS.contains(&k));
            if !public_url_is_local && (dummy_site || self.is_dummy_secret()) {
                anyhow::bail!(
                    "Cloudflare Turnstile dummy test keys are not allowed outside localhost"
                );
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct SiteVerifyResponse {
    success: bool,
    #[serde(default, rename = "error-codes")]
    error_codes: Vec<String>,
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    action: Option<String>,
}

pub struct TurnstileVerifier {
    cfg: TurnstileConfig,
    expected_hostname: Option<String>,
    http: reqwest::Client,
}

impl TurnstileVerifier {
    pub fn new(cfg: TurnstileConfig, public_url: &str) -> Self {
        let expected_hostname = Url::parse(public_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_lowercase));
        let http = reqwest::Client::builder()
            .use_rustls_tls()
            .redirect(reqwest::redirect::Policy::none())
            .pool_idle_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(5))
            .build()
            .expect("HTTP client build failed");
        Self {
            cfg,
            expected_hostname,
            http,
        }
    }

    pub async fn verify(
        &self,
        token: Option<&str>,
        expected_action: &str,
        remote_ip: Option<&str>,
        lang: Lang,
    ) -> AppResult<()> {
        if !self.cfg.is_enabled() {
            return Ok(());
        }
        let Some(secret) = self.cfg.secret_key.as_deref() else {
            return Ok(());
        };
        let Some(token) = token.map(str::trim).filter(|t| !t.is_empty()) else {
            return Err(bad(lang, ErrorKey::TurnstileRequired));
        };
        if token.len() > MAX_TOKEN_LEN {
            return Err(bad(lang, ErrorKey::TurnstileFailed));
        }

        let mut body = json!({
            "secret": secret,
            "response": token,
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
        });
        if let Some(ip) = remote_ip
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "unknown")
        {
            body["remoteip"] = json!(ip);
        }

        let res = match self
            .http
            .post(&self.cfg.verify_url)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(action = expected_action, error = %e, "turnstile siteverify request failed");
                return Err(bad(lang, ErrorKey::TurnstileFailed));
            }
        };

        if !res.status().is_success() {
            tracing::warn!(
                action = expected_action,
                status = res.status().as_u16(),
                "turnstile siteverify returned non-success status"
            );
            return Err(bad(lang, ErrorKey::TurnstileFailed));
        }

        let raw = match crate::util::read_response_bytes_bounded(res, 64 * 1024).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(action = expected_action, error = %e, "turnstile siteverify response read failed");
                return Err(bad(lang, ErrorKey::TurnstileFailed));
            }
        };
        let parsed: SiteVerifyResponse = match serde_json::from_slice(&raw) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(action = expected_action, error = %e, "turnstile siteverify invalid json");
                return Err(bad(lang, ErrorKey::TurnstileFailed));
            }
        };

        if !parsed.success {
            tracing::warn!(
                action = expected_action,
                error_codes = ?parsed.error_codes,
                "turnstile verification failed"
            );
            return Err(bad(lang, ErrorKey::TurnstileFailed));
        }

        // Cloudflare's dummy test secret keys omit or return generic hostname/action fields,
        // so strict hostname + action binding applies to real secret keys.
        if !self.cfg.is_dummy_secret() {
            if let Some(expected_host) = &self.expected_hostname {
                let got_host = parsed.hostname.as_deref().map(str::to_lowercase);
                if got_host.as_deref() != Some(expected_host.as_str()) {
                    tracing::warn!(
                        action = expected_action,
                        got_hostname = ?parsed.hostname,
                        expected_hostname = %expected_host,
                        "turnstile hostname mismatch"
                    );
                    return Err(bad(lang, ErrorKey::TurnstileFailed));
                }
            }
            if parsed.action.as_deref() != Some(expected_action) {
                tracing::warn!(
                    expected_action,
                    got_action = ?parsed.action,
                    "turnstile action mismatch"
                );
                return Err(bad(lang, ErrorKey::TurnstileFailed));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use axum::{http::StatusCode, routing::post, Json, Router};
    use serde_json::Value;

    fn error_key(res: AppResult<()>) -> ErrorKey {
        match res.unwrap_err() {
            AppError::BadRequest { key, .. } => key,
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    async fn mock_verify_server(
        status: StatusCode,
        response: Value,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/siteverify",
            post(move || {
                let response = response.clone();
                async move { (status, Json(response)) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (format!("http://{addr}/siteverify"), handle)
    }

    #[test]
    fn config_debug_redacts_secret() {
        let cfg = TurnstileConfig {
            site_key: Some("site-pub".into()),
            secret_key: Some("super-secret-value".into()),
            verify_url: DEFAULT_VERIFY_URL.into(),
        };
        let dbg = format!("{cfg:?}");
        assert!(dbg.contains("site-pub"));
        assert!(dbg.contains("[redacted]"));
        assert!(!dbg.contains("super-secret-value"));
    }

    #[test]
    fn config_validate_enforces_pairs_https_and_no_dummy_keys_in_prod() {
        // Both unset is valid
        assert!(TurnstileConfig::default().validate(false).is_ok());

        // Half-configured fails
        let site_only = TurnstileConfig {
            site_key: Some("pub".into()),
            secret_key: None,
            ..Default::default()
        };
        assert!(site_only.validate(true).is_err());

        let secret_only = TurnstileConfig {
            site_key: None,
            secret_key: Some("sec".into()),
            ..Default::default()
        };
        assert!(secret_only.validate(true).is_err());

        // Dummy keys allowed locally, refused outside localhost
        let dummy = TurnstileConfig {
            site_key: Some("1x00000000000000000000AA".into()),
            secret_key: Some("1x0000000000000000000000000000000AA".into()),
            verify_url: DEFAULT_VERIFY_URL.into(),
        };
        assert!(dummy.validate(true).is_ok());
        assert!(dummy.validate(false).is_err());

        // HTTP verify URL allowed locally, refused outside localhost
        let prod_http = TurnstileConfig {
            site_key: Some("0x4AAAAAArealSiteKey".into()),
            secret_key: Some("0x4AAAAAArealSecretKey".into()),
            verify_url: "http://127.0.0.1:9999/siteverify".into(),
        };
        assert!(prod_http.validate(true).is_ok());
        assert!(prod_http.validate(false).is_err());

        // Real HTTPS config valid in prod
        let prod_ok = TurnstileConfig {
            site_key: Some("0x4AAAAAArealSiteKey".into()),
            secret_key: Some("0x4AAAAAArealSecretKey".into()),
            verify_url: DEFAULT_VERIFY_URL.into(),
        };
        assert!(prod_ok.validate(false).is_ok());
    }

    #[tokio::test]
    async fn disabled_verifier_allows_requests_without_token() {
        let v = TurnstileVerifier::new(TurnstileConfig::default(), "http://localhost:5173");
        assert!(v
            .verify(None, "passkey_login", Some("1.2.3.4"), Lang::En)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn rejects_missing_empty_and_oversized_tokens_locally() {
        let cfg = TurnstileConfig {
            site_key: Some("0x4AAAAAArealSiteKey".into()),
            secret_key: Some("0x4AAAAAArealSecretKey".into()),
            verify_url: "http://127.0.0.1:9/unused".into(),
        };
        let v = TurnstileVerifier::new(cfg, "https://ride.example.org");

        assert_eq!(
            error_key(v.verify(None, "passkey_login", None, Lang::En).await),
            ErrorKey::TurnstileRequired
        );
        assert_eq!(
            error_key(v.verify(Some("   "), "passkey_login", None, Lang::En).await),
            ErrorKey::TurnstileRequired
        );
        let huge = "a".repeat(MAX_TOKEN_LEN + 1);
        assert_eq!(
            error_key(v.verify(Some(&huge), "passkey_login", None, Lang::En).await),
            ErrorKey::TurnstileFailed
        );
    }

    #[tokio::test]
    async fn verifies_success_hostname_action_failure_and_upstream_errors() {
        // 1. Valid response with matching hostname and action
        let (url, h) = mock_verify_server(
            StatusCode::OK,
            json!({
                "success": true,
                "hostname": "ride.example.org",
                "action": "invite_accept",
            }),
        )
        .await;
        let cfg = TurnstileConfig {
            site_key: Some("0x4AAAAAArealSiteKey".into()),
            secret_key: Some("0x4AAAAAArealSecretKey".into()),
            verify_url: url,
        };
        let v = TurnstileVerifier::new(cfg.clone(), "https://ride.example.org");
        assert!(v
            .verify(Some("tok"), "invite_accept", Some("203.0.113.5"), Lang::En)
            .await
            .is_ok());
        // Action mismatch fails
        assert_eq!(
            error_key(v.verify(Some("tok"), "passkey_login", None, Lang::En).await),
            ErrorKey::TurnstileFailed
        );
        // Hostname mismatch fails
        let v_other_host = TurnstileVerifier::new(cfg, "https://other.example.org");
        assert_eq!(
            error_key(
                v_other_host
                    .verify(Some("tok"), "invite_accept", None, Lang::En)
                    .await
            ),
            ErrorKey::TurnstileFailed
        );
        h.abort();

        // 2. success: false fails
        let (url, h) = mock_verify_server(
            StatusCode::OK,
            json!({ "success": false, "error-codes": ["timeout-or-duplicate"] }),
        )
        .await;
        let v_fail = TurnstileVerifier::new(
            TurnstileConfig {
                site_key: Some("0x4AAAAAArealSiteKey".into()),
                secret_key: Some("0x4AAAAAArealSecretKey".into()),
                verify_url: url,
            },
            "https://ride.example.org",
        );
        assert_eq!(
            error_key(
                v_fail
                    .verify(Some("tok"), "invite_accept", None, Lang::En)
                    .await
            ),
            ErrorKey::TurnstileFailed
        );
        h.abort();

        // 3. Dummy secret allows generic response on localhost
        let (url, h) = mock_verify_server(StatusCode::OK, json!({ "success": true })).await;
        let v_dummy = TurnstileVerifier::new(
            TurnstileConfig {
                site_key: Some("1x00000000000000000000AA".into()),
                secret_key: Some("1x0000000000000000000000000000000AA".into()),
                verify_url: url,
            },
            "http://localhost:5173",
        );
        assert!(v_dummy
            .verify(
                Some("XXXX.DUMMY.TOKEN.XXXX"),
                "invite_accept",
                None,
                Lang::En
            )
            .await
            .is_ok());
        h.abort();

        // 4. Upstream 500 fails closed
        let (url, h) = mock_verify_server(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": "down" }),
        )
        .await;
        let v_500 = TurnstileVerifier::new(
            TurnstileConfig {
                site_key: Some("0x4AAAAAArealSiteKey".into()),
                secret_key: Some("0x4AAAAAArealSecretKey".into()),
                verify_url: url,
            },
            "https://ride.example.org",
        );
        assert_eq!(
            error_key(
                v_500
                    .verify(Some("tok"), "invite_accept", None, Lang::En)
                    .await
            ),
            ErrorKey::TurnstileFailed
        );
        h.abort();
    }
}
