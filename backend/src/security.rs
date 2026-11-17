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
use crate::error::{forbidden, locked};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{now, t, Lang};

/// Fixed-window counter keyed by "bucket:ip". Single-process is fine: the app runs
/// as one instance of this app. Move to Redis if it is ever scaled horizontally.
pub struct RateLimiter {
    windows: Mutex<HashMap<String, (Instant, Duration, u32)>>,
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
            map.retain(|_, (start, w, _)| now.duration_since(*start) < *w);
            if !map.contains_key(key) && map.len() >= 50_000 {
                return Err(window.as_secs().max(1));
            }
        }
        let entry = map.entry(key.to_string()).or_insert((now, window, 0));
        if now.duration_since(entry.0) >= window || entry.1 != window {
            *entry = (now, window, 0);
        }
        entry.2 += 1;
        if entry.2 > limit {
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
    // Visitor links (preview and claim), but not the claimed browser's page, which polls.
    let visit_link = path.starts_with("/api/visit/") && !path.starts_with("/api/visit/me");
    let care_link = path.starts_with("/api/care/");
    let calendar_link = path.starts_with("/api/calendar/");
    let rule: Option<(&str, u32, u64)> = if req.method() == Method::GET
        && !path.starts_with("/api/invites/")
        && !visit_link
        && !care_link
        && !calendar_link
    {
        None
    } else if path == "/api/auth/login"
        || path == "/api/auth/dev-login"
        || path.starts_with("/api/auth/passkey/login")
    {
        Some(("login", 10, 60))
    } else if path.starts_with("/api/auth/passkey/reauth/") {
        Some(("reauth", 20, 300))
    } else if path == "/api/auth/totp/verify"
        || path == "/api/auth/totp/enable"
        || path == "/api/auth/unlock"
    {
        // A 6-digit code has a million values: guesses are capped per member
        // (below), so neither a new address nor a new session buys more tries.
        Some(("totp", 10, 300))
    } else if path == "/api/auth/recovery-request" {
        Some(("recovery", 3, 3600))
    } else if path.starts_with("/api/invites/") {
        Some(("invite", 20, 600))
    } else if visit_link {
        Some(("visit", 30, 600))
    } else if care_link {
        Some(("care", 60, 600))
    } else if calendar_link {
        Some(("calendar", 30, 600))
    } else if path == "/api/files" {
        Some(("upload", 30, 600))
    } else if path.starts_with("/api/files/") && path.ends_with("/extract-ref") {
        Some(("extract_ref", 30, 600))
    } else {
        None
    };
    if let Some((bucket, limit, secs)) = rule {
        let peer = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0);
        let ip = client_ip(req.headers(), peer, st.cfg.trust_proxy);
        // Code checks count per member and per address; everything else per address.
        if bucket == "totp" {
            if let Some(owner) = totp_owner(&st, req.headers()).await {
                if let Err(wait) =
                    st.limiter
                        .check(&format!("totp:{owner}"), limit, Duration::from_secs(secs))
                {
                    tracing::warn!(bucket, ip, "rate limited");
                    let lang = visitor_lang(req.headers());
                    return too_many(wait, lang);
                }
                if let Err(wait) = st.limiter.check(
                    &format!("totp-ip:{ip}"),
                    limit.saturating_mul(3),
                    Duration::from_secs(secs),
                ) {
                    tracing::warn!(bucket = "totp-ip", ip, "rate limited");
                    let lang = visitor_lang(req.headers());
                    return too_many(wait, lang);
                }
            } else if let Err(wait) =
                st.limiter
                    .check(&format!("totp:{ip}"), limit, Duration::from_secs(secs))
            {
                tracing::warn!(bucket, ip, "rate limited");
                let lang = visitor_lang(req.headers());
                return too_many(wait, lang);
            }
        } else if let Err(wait) =
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

/// Whose two-step code is being guessed: the member behind the session cookie. Keyed by
/// member, not session, because anyone holding the password can sign in again for a
/// fresh session, and per-session counting would hand them a new allowance each time.
/// `None` (no cookie, or not a live session) falls back to the address; such requests
/// are refused as unauthenticated anyway.
async fn totp_owner(st: &AppState, headers: &HeaderMap) -> Option<String> {
    let jar = CookieJar::from_headers(headers);
    let token = jar
        .get(SESSION_COOKIE_SECURE)
        .or_else(|| jar.get(SESSION_COOKIE))?
        .value()
        .to_string();
    if token.len() != 48 {
        return None;
    }
    let uid: Option<i64> =
        sqlx::query_scalar("SELECT user_id FROM sessions WHERE token=$1 AND expires_at > $2")
            .bind(hash_token(&token))
            .bind(crate::util::now())
            .fetch_optional(&st.db)
            .await
            .ok()
            .flatten();
    uid.map(|id| format!("user:{id}"))
}

fn pending_profile_only(path: &str) -> bool {
    let path = path.strip_prefix("/api").unwrap_or(path);
    path == "/health"
        || path == "/stream"
        || path == "/me"
        || path == "/auth"
        || path.starts_with("/auth/")
        || path == "/files"
        || path.starts_with("/files/")
        || path.starts_with("/invites/")
        || path.starts_with("/visit/")
        || path.starts_with("/care/")
        || path.starts_with("/calendar/")
}

fn lock_bypass_path(path: &str) -> bool {
    let path = path.strip_prefix("/api").unwrap_or(path);
    path == "/health"
        || path == "/auth/me"
        || path == "/auth/lock"
        || path == "/auth/unlock"
        || path == "/auth/logout"
        || path == "/auth/login"
        || path == "/auth/dev-users"
        || path == "/auth/dev-login"
        || path == "/auth/recovery-request"
        || path == "/auth/passkey/reauth/start"
        || path == "/auth/passkey/reauth/finish"
        || path.starts_with("/auth/passkey/login")
        || path.starts_with("/invites/")
        || path.starts_with("/visit/")
        || path.starts_with("/care/")
        || path.starts_with("/calendar/")
}

/// Evaluates whether a session is currently locked (or has exceeded its idle timeout),
/// engaging `sessions.locked_at` when idle and updating `last_seen_at` if `touch` is true.
/// Returns `Some((lang, is_locked_without_mfa))` when the session is locked.
pub async fn evaluate_session_lock(
    st: &AppState,
    token_hash: &str,
    touch: bool,
) -> Option<(Lang, bool)> {
    type LockRow = (
        Option<String>,
        String,
        i64,
        String,
        String,
        String,
        i64,
        i64,
        Option<String>,
    );
    let row: Option<LockRow> = sqlx::query_as(
        "SELECT s.locked_at, s.last_seen_at, u.auto_lock_mins, u.lang, s.expires_at,
                u.status, u.totp_enabled,
                (SELECT COUNT(*) FROM passkeys p WHERE p.user_id = u.id) AS passkey_count,
                (SELECT value FROM system_settings WHERE key = 'session_timeout_mins') AS org_timeout
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = $1",
    )
    .bind(token_hash)
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten();
    let (
        locked_at,
        last_seen_at,
        auto_lock_mins,
        lang,
        expires,
        status,
        totp_enabled,
        passkey_count,
        org_timeout_str,
    ) = row?;
    let now_str = now();
    if expires < now_str {
        return None;
    }
    let has_unlock_method = passkey_count > 0 || totp_enabled == 1 || st.cfg.dev_mode;
    if locked_at.is_some() {
        return Some((Lang::from_code(&lang), !has_unlock_method));
    }
    let org_timeout = org_timeout_str
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(st.cfg.session_timeout_mins);
    let effective_mins = if auto_lock_mins > 0 {
        auto_lock_mins
    } else if auto_lock_mins == 0 {
        org_timeout
    } else {
        0
    };
    if effective_mins > 0
        && status == "active"
        && has_unlock_method
        && last_seen_at < crate::util::minutes_ago(effective_mins)
    {
        let _ = sqlx::query(
            "UPDATE sessions SET locked_at = $1 WHERE token = $2 AND locked_at IS NULL",
        )
        .bind(&now_str)
        .bind(token_hash)
        .execute(&st.db)
        .await;
        return Some((Lang::from_code(&lang), false));
    }
    if touch && last_seen_at < crate::util::minutes_ago(1) {
        let _ = sqlx::query("UPDATE sessions SET last_seen_at = $1 WHERE token = $2")
            .bind(&now_str)
            .bind(token_hash)
            .execute(&st.db)
            .await;
    }
    None
}

/// Server-enforced screen lock: blocks API access when the current session is locked
/// or has been inactive longer than the user's configured auto-lock timeout.
pub async fn session_lock_guard(State(st): State<AppState>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    if lock_bypass_path(&path) {
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
    let token_hash = hash_token(&token);
    if let Some((lang, no_mfa)) = evaluate_session_lock(&st, &token_hash, true).await {
        let sub = path.strip_prefix("/api").unwrap_or(&path);
        if no_mfa
            && (sub == "/auth/passkey/register/start" || sub == "/auth/passkey/register/finish")
        {
            return next.run(req).await;
        }
        return locked(lang, ErrorKey::SessionLocked).into_response();
    }
    next.run(req).await
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
            return (StatusCode::FORBIDDEN, Json(json!({ "error": accept }))).into_response();
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
    // Drivers share GPS and QR check-in scanner may use camera on this origin; nothing else may use sensors.
    set(
        h,
        "permissions-policy",
        "geolocation=(self), camera=(self), microphone=(), payment=(), usb=(), interest-cohort=()",
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
