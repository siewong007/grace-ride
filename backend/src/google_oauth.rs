//! Google OAuth 2.0 / OpenID Connect client (Authorization Code + PKCE).
//!
//! Opt-in via `GOOGLE_CLIENT_ID` + `GOOGLE_CLIENT_SECRET`. When unset, Google Sign-In
//! and Google-verified password reset are disabled cleanly.

use std::{fmt, time::Duration};

use base64::Engine as _;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;

use crate::error::{bad, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::util::Lang;

pub const DEFAULT_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const DEFAULT_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

#[derive(Clone)]
pub struct GoogleOAuthConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: String,
}

impl fmt::Debug for GoogleOAuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GoogleOAuthConfig")
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "[redacted]"),
            )
            .field("auth_url", &self.auth_url)
            .field("token_url", &self.token_url)
            .field("userinfo_url", &self.userinfo_url)
            .finish()
    }
}

impl Default for GoogleOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: None,
            client_secret: None,
            auth_url: DEFAULT_AUTH_URL.to_string(),
            token_url: DEFAULT_TOKEN_URL.to_string(),
            userinfo_url: DEFAULT_USERINFO_URL.to_string(),
        }
    }
}

impl GoogleOAuthConfig {
    pub fn from_env() -> Self {
        let opt = |k: &str| {
            std::env::var(k)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Self {
            client_id: opt("GOOGLE_CLIENT_ID"),
            client_secret: opt("GOOGLE_CLIENT_SECRET"),
            auth_url: opt("GOOGLE_AUTH_URL").unwrap_or_else(|| DEFAULT_AUTH_URL.to_string()),
            token_url: opt("GOOGLE_TOKEN_URL").unwrap_or_else(|| DEFAULT_TOKEN_URL.to_string()),
            userinfo_url: opt("GOOGLE_USERINFO_URL")
                .unwrap_or_else(|| DEFAULT_USERINFO_URL.to_string()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }

    pub fn validate(&self, public_url_is_local: bool) -> anyhow::Result<()> {
        if self.client_id.is_some() != self.client_secret.is_some() {
            anyhow::bail!(
                "GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET must both be set or both be unset"
            );
        }
        if self.is_enabled() {
            for (k, raw) in [
                ("GOOGLE_AUTH_URL", &self.auth_url),
                ("GOOGLE_TOKEN_URL", &self.token_url),
                ("GOOGLE_USERINFO_URL", &self.userinfo_url),
            ] {
                let parsed =
                    Url::parse(raw).map_err(|_| anyhow::anyhow!("{k} is not a valid URL"))?;
                if !public_url_is_local && parsed.scheme() != "https" {
                    anyhow::bail!("{k} must use https:// outside localhost");
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleIdentity {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
}

#[derive(Deserialize)]
struct UserInfoResponse {
    sub: Option<String>,
    email: Option<String>,
    #[serde(default)]
    email_verified: bool,
}

pub struct GoogleOAuthClient {
    cfg: GoogleOAuthConfig,
    redirect_uri: String,
    http: reqwest::Client,
}

impl GoogleOAuthClient {
    pub fn new(cfg: GoogleOAuthConfig, public_url: &str) -> Self {
        let redirect_uri = format!(
            "{}/api/auth/google/callback",
            public_url.trim_end_matches('/')
        );
        let http = reqwest::Client::builder()
            .use_rustls_tls()
            .redirect(reqwest::redirect::Policy::none())
            .pool_idle_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10))
            .build()
            .expect("HTTP client build failed");
        Self {
            cfg,
            redirect_uri,
            http,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.cfg.is_enabled()
    }

    pub fn pkce_challenge(verifier: &str) -> String {
        let digest = Sha256::digest(verifier.as_bytes());
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
    }

    pub fn authorization_url(
        &self,
        state: &str,
        code_verifier: &str,
        login_hint: Option<&str>,
    ) -> AppResult<String> {
        let client_id = self
            .cfg
            .client_id
            .as_deref()
            .ok_or_else(|| AppError::Other(anyhow::anyhow!("google client_id missing")))?;
        let mut url = Url::parse(&self.cfg.auth_url)
            .map_err(|e| AppError::Other(anyhow::anyhow!("invalid google auth_url: {e}")))?;
        let challenge = Self::pkce_challenge(code_verifier);
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("client_id", client_id);
            q.append_pair("redirect_uri", &self.redirect_uri);
            q.append_pair("response_type", "code");
            q.append_pair("scope", "openid email");
            q.append_pair("state", state);
            q.append_pair("code_challenge", &challenge);
            q.append_pair("code_challenge_method", "S256");
            q.append_pair("prompt", "select_account");
            if let Some(hint) = login_hint.map(str::trim).filter(|s| !s.is_empty()) {
                q.append_pair("login_hint", hint);
            }
        }
        Ok(url.into())
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        lang: Lang,
    ) -> AppResult<GoogleIdentity> {
        let (Some(client_id), Some(client_secret)) = (
            self.cfg.client_id.as_deref(),
            self.cfg.client_secret.as_deref(),
        ) else {
            return Err(bad(lang, ErrorKey::GoogleNotConfigured));
        };

        let form_body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("code", code),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
                ("code_verifier", code_verifier),
            ])
            .finish();
        let token_res = self
            .http
            .post(&self.cfg.token_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(form_body)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "google token exchange request failed");
                bad(lang, ErrorKey::GoogleAuthFailed)
            })?;

        if !token_res.status().is_success() {
            tracing::warn!(
                status = token_res.status().as_u16(),
                "google token exchange returned non-success status"
            );
            return Err(bad(lang, ErrorKey::GoogleAuthFailed));
        }

        let token_body: TokenResponse = token_res.json().await.map_err(|e| {
            tracing::warn!(error = %e, "google token response invalid json");
            bad(lang, ErrorKey::GoogleAuthFailed)
        })?;
        let access_token = token_body
            .access_token
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .ok_or_else(|| bad(lang, ErrorKey::GoogleAuthFailed))?;

        let userinfo_res = self
            .http
            .get(&self.cfg.userinfo_url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "google userinfo request failed");
                bad(lang, ErrorKey::GoogleAuthFailed)
            })?;

        if !userinfo_res.status().is_success() {
            tracing::warn!(
                status = userinfo_res.status().as_u16(),
                "google userinfo returned non-success status"
            );
            return Err(bad(lang, ErrorKey::GoogleAuthFailed));
        }

        let info: UserInfoResponse = userinfo_res.json().await.map_err(|e| {
            tracing::warn!(error = %e, "google userinfo invalid json");
            bad(lang, ErrorKey::GoogleAuthFailed)
        })?;

        let sub = info
            .sub
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| bad(lang, ErrorKey::GoogleAuthFailed))?;
        let email = info
            .email
            .map(|e| e.trim().to_lowercase())
            .filter(|e| !e.is_empty() && e.contains('@'))
            .ok_or_else(|| bad(lang, ErrorKey::GoogleAuthFailed))?;

        Ok(GoogleIdentity {
            sub,
            email,
            email_verified: info.email_verified,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::{header, HeaderMap, StatusCode},
        routing::{get, post},
        Json, Router,
    };
    use serde_json::{json, Value};
    use std::collections::HashMap;

    #[test]
    fn config_debug_redacts_secret() {
        let cfg = GoogleOAuthConfig {
            client_id: Some("cid-123".into()),
            client_secret: Some("super-secret-google".into()),
            ..Default::default()
        };
        let dbg = format!("{cfg:?}");
        assert!(dbg.contains("cid-123"));
        assert!(dbg.contains("[redacted]"));
        assert!(!dbg.contains("super-secret-google"));
    }

    #[test]
    fn config_validate_enforces_pairs_and_https_in_prod() {
        assert!(GoogleOAuthConfig::default().validate(false).is_ok());

        let half = GoogleOAuthConfig {
            client_id: Some("cid".into()),
            client_secret: None,
            ..Default::default()
        };
        assert!(half.validate(true).is_err());

        let http_cfg = GoogleOAuthConfig {
            client_id: Some("cid".into()),
            client_secret: Some("sec".into()),
            auth_url: "http://127.0.0.1:9000/auth".into(),
            token_url: "http://127.0.0.1:9000/token".into(),
            userinfo_url: "http://127.0.0.1:9000/userinfo".into(),
        };
        assert!(http_cfg.validate(true).is_ok());
        assert!(http_cfg.validate(false).is_err());

        let prod_ok = GoogleOAuthConfig {
            client_id: Some("cid".into()),
            client_secret: Some("sec".into()),
            ..Default::default()
        };
        assert!(prod_ok.validate(false).is_ok());
    }

    #[test]
    fn authorization_url_includes_pkce_and_optional_hint() {
        let client = GoogleOAuthClient::new(
            GoogleOAuthConfig {
                client_id: Some("my-client-id".into()),
                client_secret: Some("my-secret".into()),
                ..Default::default()
            },
            "https://ride.example.org/",
        );
        let url_str = client
            .authorization_url("st-1", "verifier-abc", Some("Member@Example.org"))
            .unwrap();
        let parsed = Url::parse(&url_str).unwrap();
        let params: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
        assert_eq!(
            params.get("client_id").map(String::as_str),
            Some("my-client-id")
        );
        assert_eq!(
            params.get("redirect_uri").map(String::as_str),
            Some("https://ride.example.org/api/auth/google/callback")
        );
        assert_eq!(
            params.get("response_type").map(String::as_str),
            Some("code")
        );
        assert_eq!(
            params.get("scope").map(String::as_str),
            Some("openid email")
        );
        assert_eq!(params.get("state").map(String::as_str), Some("st-1"));
        assert_eq!(
            params.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert_eq!(
            params.get("code_challenge").map(String::as_str),
            Some(GoogleOAuthClient::pkce_challenge("verifier-abc").as_str())
        );
        assert_eq!(
            params.get("login_hint").map(String::as_str),
            Some("Member@Example.org")
        );
    }

    async fn mock_google_server(
        token_status: StatusCode,
        token_body: Value,
        userinfo_status: StatusCode,
        userinfo_body: Value,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route(
                "/token",
                post(move |body: String| {
                    let token_body = token_body.clone();
                    let params: HashMap<String, String> =
                        url::form_urlencoded::parse(body.as_bytes())
                            .into_owned()
                            .collect();
                    async move {
                        if params.get("code").map(String::as_str) != Some("good-code") {
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(json!({ "error": "invalid_grant" })),
                            );
                        }
                        (token_status, Json(token_body))
                    }
                }),
            )
            .route(
                "/userinfo",
                get(move |headers: HeaderMap| {
                    let userinfo_body = userinfo_body.clone();
                    async move {
                        let auth = headers
                            .get(header::AUTHORIZATION)
                            .and_then(|v| v.to_str().ok());
                        if auth != Some("Bearer good-token") {
                            return (
                                StatusCode::UNAUTHORIZED,
                                Json(json!({ "error": "invalid_token" })),
                            );
                        }
                        (userinfo_status, Json(userinfo_body))
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn exchange_code_parses_verified_identity_and_fails_closed() {
        let (base, h) = mock_google_server(
            StatusCode::OK,
            json!({ "access_token": "good-token" }),
            StatusCode::OK,
            json!({
                "sub": "google-sub-42",
                "email": " Member@Example.ORG ",
                "email_verified": true
            }),
        )
        .await;
        let client = GoogleOAuthClient::new(
            GoogleOAuthConfig {
                client_id: Some("cid".into()),
                client_secret: Some("csec".into()),
                auth_url: format!("{base}/auth"),
                token_url: format!("{base}/token"),
                userinfo_url: format!("{base}/userinfo"),
            },
            "http://localhost:5173",
        );

        let id = client
            .exchange_code("good-code", "ver", Lang::En)
            .await
            .unwrap();
        assert_eq!(
            id,
            GoogleIdentity {
                sub: "google-sub-42".into(),
                email: "member@example.org".into(),
                email_verified: true,
            }
        );

        let err = client
            .exchange_code("bad-code", "ver", Lang::En)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::BadRequest {
                key: ErrorKey::GoogleAuthFailed,
                ..
            }
        ));
        h.abort();
    }
}
