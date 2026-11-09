//! HTTP-layer security controls:
//! - security headers (CSP, frame/sniff/referrer/permissions policies, HSTS)
//! - Origin check on state-changing requests (CSRF defence in depth on top of SameSite cookies)
//! - per-IP rate limiting on authentication endpoints
//! - cache headers for the built frontend

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use axum_extra::extract::CookieJar;

use crate::auth::{hash_token, SESSION_COOKIE, SESSION_COOKIE_SECURE};
use crate::error::forbidden;
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{now, t, Lang};

/// Fixed-window counter keyed by "bucket:ip". Single-process is fine: the app runs
/// as one instance of this app. Move to Redis if it is ever scaled horizontally.
pub struct RateLimiter {
    windows: Mutex<HashMap<String, (Instant, u32)>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
        }
    }
}

impl RateLimiter {
    /// Returns Err(seconds_to_wait) when `limit` hits within `window` are exceeded.
    pub fn check(&self, key: &str, limit: u32, window: Duration) -> Result<(), u64> {
        let now = Instant::now();
        let mut map = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() > 50_000 {
            map.retain(|_, (start, _)| now.duration_since(*start) < window);
        }
        let entry = map.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) >= window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        if entry.1 > limit {
            Err(window
                .saturating_sub(now.duration_since(entry.0))
                .as_secs()
                .max(1))
        } else {
            Ok(())
        }
    }
}

pub fn client_ip(headers: &HeaderMap, req_ip: Option<SocketAddr>, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return ip.to_string();
        }
    }
    req_ip
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn too_many(retry_after: u64, lang: Lang) -> Response {
    let mut res = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "error": t(lang, &format!("尝试次数过多，请 {retry_after} 秒后再试"), &format!("Too many attempts, try again in {retry_after} seconds")) })),
    )
        .into_response();
    if let Ok(v) = HeaderValue::from_str(&retry_after.to_string()) {
        res.headers_mut().insert(header::RETRY_AFTER, v);
    }
    res
}

/// Unauthenticated paths use Accept-Language (default 中文).
fn visitor_lang(headers: &HeaderMap) -> Lang {
    headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .map(Lang::from_accept_language)
        .unwrap_or(Lang::Zh)
}

/// Rate limits for unauthenticated / credential endpoints.
pub async fn rate_limit(State(st): State<AppState>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let rule: Option<(&str, u32, u64)> =
        if req.method() == Method::GET && !path.starts_with("/api/invites/") {
            None
        } else if path == "/api/auth/login"
            || path == "/api/auth/dev-login"
            || path.starts_with("/api/auth/passkey/login")
        {
            Some(("login", 10, 60))
        } else if path == "/api/auth/recovery-request" {
            Some(("recovery", 3, 3600))
        } else if path.starts_with("/api/invites/") {
            Some(("invite", 20, 600))
        } else if path == "/api/files" {
            Some(("upload", 30, 600))
        } else {
            None
        };
    if let Some((bucket, limit, secs)) = rule {
        let peer = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0);
        let ip = client_ip(req.headers(), peer, st.cfg.trust_proxy);
        if let Err(wait) =
            st.limiter
                .check(&format!("{bucket}:{ip}"), limit, Duration::from_secs(secs))
        {
            tracing::warn!(bucket, ip, "rate limited");
            let lang = visitor_lang(req.headers());
            return too_many(wait, lang);
        }
    }
    next.run(req).await
}

fn pending_profile_only(path: &str) -> bool {
    let path = path.strip_prefix("/api").unwrap_or(path);
    path == "/health"
        || path == "/me"
        || path == "/auth"
        || path.starts_with("/auth/")
        || path == "/files"
        || path.starts_with("/files/")
        || path.starts_with("/invites/")
}

/// A member who has accepted an invite but is not yet approved may use their
/// profile, passkey, and photo. Every other API route stays closed.
pub async fn pending_guard(State(st): State<AppState>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    if pending_profile_only(&path) {
        return next.run(req).await;
    }
    let jar = CookieJar::from_headers(req.headers());
    let token = jar
        .get(SESSION_COOKIE_SECURE)
        .or_else(|| jar.get(SESSION_COOKIE))
        .map(|c| c.value().to_string());
    let Some(token) = token.filter(|t| t.len() == 48) else {
        return next.run(req).await;
    };
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT u.status, u.lang, s.expires_at FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token = $1",
    )
    .bind(hash_token(&token))
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten();
    if let Some((status, lang, expires)) = row {
        if status == "pending" && expires >= now() {
            return forbidden(Lang::from_code(&lang), ErrorKey::AccountPending).into_response();
        }
    }
    next.run(req).await
}

/// Reject cross-site state-changing requests. Browsers always send Origin on
/// cross-origin POST/PUT/DELETE; non-browser clients (curl, tests) send none.
pub async fn origin_guard(State(st): State<AppState>, req: Request, next: Next) -> Response {
    let safe = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if !safe {
        let h = req.headers();
        let origin = h.get(header::ORIGIN).and_then(|v| v.to_str().ok());
        let fetch_site = h.get("sec-fetch-site").and_then(|v| v.to_str().ok());
        let ok = match origin {
            Some(o) => o == st.cfg.origin(),
            None => fetch_site != Some("cross-site"),
        };
        if !ok {
            tracing::warn!(?origin, ?fetch_site, path = %req.uri().path(), "blocked cross-origin request");
            let accept = crate::util::t(
                visitor_lang(req.headers()),
                "请求来源不被允许",
                "Request origin not allowed",
            );
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": accept })),
            )
                .into_response();
        }
    }
    next.run(req).await
}

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob: https://*.tile.openstreetmap.org; connect-src 'self'; font-src 'self'; \
object-src 'none'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; manifest-src 'self'; worker-src 'self'";

pub async fn security_headers(State(st): State<AppState>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    let set = |h: &mut HeaderMap, k: &'static str, v: &'static str| {
        h.entry(k).or_insert(HeaderValue::from_static(v));
    };
    set(h, "x-content-type-options", "nosniff");
    set(h, "x-frame-options", "DENY");
    set(h, "referrer-policy", "strict-origin-when-cross-origin");
    // Drivers share GPS from this origin only; nothing else may use sensors.
    set(
        h,
        "permissions-policy",
        "geolocation=(self), camera=(), microphone=(), payment=(), usb=(), interest-cohort=()",
    );
    set(h, "cross-origin-opener-policy", "same-origin");
    set(h, "cross-origin-resource-policy", "same-origin");
    set(h, "content-security-policy", CSP);
    if st.cfg.secure_cookies() {
        set(
            h,
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        );
    }
    if path.starts_with("/api/") {
        set(h, "cache-control", "no-store");
    } else if path.starts_with("/assets/") {
        set(h, "cache-control", "public, max-age=31536000, immutable");
    } else {
        set(h, "cache-control", "no-cache");
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_blocks_after_limit_and_resets() {
        let l = RateLimiter::default();
        for _ in 0..3 {
            assert!(l.check("k", 3, Duration::from_millis(50)).is_ok());
        }
        assert!(l.check("k", 3, Duration::from_millis(50)).is_err());
        assert!(l.check("other", 3, Duration::from_millis(50)).is_ok());
        std::thread::sleep(Duration::from_millis(60));
        assert!(l.check("k", 3, Duration::from_millis(50)).is_ok());
    }

    #[test]
    fn client_ip_respects_trust_proxy() {
        let mut h = HeaderMap::new();
        h.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.9, 10.0.0.1"),
        );
        let peer: SocketAddr = "10.0.0.2:5555".parse().unwrap();
        assert_eq!(client_ip(&h, Some(peer), true), "203.0.113.9");
        assert_eq!(client_ip(&h, Some(peer), false), "10.0.0.2");
    }
}
