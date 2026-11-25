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

use crate::auth::{extract_session_token, hash_token};
use crate::error::{forbidden, locked};
use crate::i18n::ErrorKey;
use crate::perms::{self, PermSet};
use crate::state::AppState;
use crate::util::{now, t, Lang};

#[derive(Clone, Debug, sqlx::FromRow)]
pub(crate) struct ResolvedSessionData {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub roles: String,
    pub status: String,
    pub mfa_ok: i64,
    pub expires_at: String,
    pub created_at: String,
    pub lang: String,
    pub granted: String,
    pub session_id: String,
    pub locked_at: Option<String>,
    pub last_seen_at: String,
    pub auto_lock_mins: i64,
    pub totp_enabled: i64,
    pub passkey_count: i64,
    pub org_timeout: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum ResolvedSession {
    Invalid,
    Valid(Box<ResolvedSessionData>),
}

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
                // Full of live windows: evict the oldest batch to make room instead of
                // rejecting every new address (which locked legitimate visitors out).
                // Surviving counters keep their state; evicted keys start fresh, so a
                // flood of new keys can at worst reset the oldest windows.
                let mut oldest: Vec<(String, Instant)> = map
                    .iter()
                    .map(|(k, (start, _, _))| (k.clone(), *start))
                    .collect();
                oldest.sort_by_key(|(_, start)| *start);
                for (k, _) in oldest.into_iter().take(5_000) {
                    map.remove(&k);
                }
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
        Json(json!({
            "error": t(
                lang,
                &format!("尝试次数过多，请 {retry_after} 秒后再试"),
                &format!("Too many attempts, try again in {retry_after} seconds"),
            ),
            "code": "TOO_MANY_REQUESTS",
        })),
    )
        .into_response();
    if let Ok(v) = HeaderValue::from_str(&retry_after.to_string()) {
        res.headers_mut().insert(header::RETRY_AFTER, v);
    }
    res
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
    } else if path == "/api/push/test" || path == "/api/push/telegram/test" {
        Some(("push_test", 10, 600))
    } else if path == "/api/push/subscribe"
        || path == "/api/push/mobile-token"
        || path == "/api/push/telegram/link-token"
    {
        Some(("push_sub", 30, 600))
    } else if path == "/api/auth/password" {
        Some(("password", 5, 300))
    } else if path == "/api/me/family/people" {
        Some(("family_add", 10, 600))
    } else if path == "/api/admin/settings/alerts/test-webhook" {
        Some(("test_webhook", 10, 600))
    } else {
        None
    };
    if let Some((bucket, limit, secs)) = rule {
        let peer = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0);
        let ip = client_ip(req.headers(), peer, st.cfg.trust_proxy);
        // Credential/enumeration checks count per member and per address; everything else per address.
        if matches!(bucket, "totp" | "password" | "family_add") {
            if let Some(owner) = session_user_key(&st, req.headers()).await {
                if let Err(wait) = st.limiter.check(
                    &format!("{bucket}:{owner}"),
                    limit,
                    Duration::from_secs(secs),
                ) {
                    tracing::warn!(bucket, ip, "rate limited");
                    return too_many(wait, Lang::from_headers(req.headers()));
                }
                if let Err(wait) = st.limiter.check(
                    &format!("{bucket}-ip:{ip}"),
                    limit.saturating_mul(3),
                    Duration::from_secs(secs),
                ) {
                    tracing::warn!(bucket = format!("{bucket}-ip").as_str(), ip, "rate limited");
                    return too_many(wait, Lang::from_headers(req.headers()));
                }
            } else if let Err(wait) =
                st.limiter
                    .check(&format!("{bucket}:{ip}"), limit, Duration::from_secs(secs))
            {
                tracing::warn!(bucket, ip, "rate limited");
                return too_many(wait, Lang::from_headers(req.headers()));
            }
        } else if let Err(wait) =
            st.limiter
                .check(&format!("{bucket}:{ip}"), limit, Duration::from_secs(secs))
        {
            tracing::warn!(bucket, ip, "rate limited");
            return too_many(wait, Lang::from_headers(req.headers()));
        }
    }
    next.run(req).await
}

/// Whose session is making this request: the member behind the session cookie. Keyed by
/// member, not session, because per-session counting would hand a fresh allowance per session.
/// `None` (no cookie, or not a live session) falls back to the address.
async fn session_user_key(st: &AppState, headers: &HeaderMap) -> Option<String> {
    let token = extract_session_token(headers, st.cfg.secure_cookies())?;
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
        || path.starts_with("/me/family")
        || path == "/auth"
        || path.starts_with("/auth/")
        || path == "/files"
        || path.starts_with("/files/")
        || path.starts_with("/invites/")
        || path.starts_with("/visit/")
        || path.starts_with("/care/")
        || path.starts_with("/calendar/")
        || path.starts_with("/push/telegram/webhook/")
}

fn lock_bypass_path(path: &str) -> bool {
    let path = path.strip_prefix("/api").unwrap_or(path);
    path == "/health"
        || path == "/auth/me"
        || path == "/auth/turnstile"
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
        || path.starts_with("/push/telegram/webhook/")
}

#[derive(sqlx::FromRow)]
struct LockRow {
    locked_at: Option<String>,
    last_seen_at: String,
    auto_lock_mins: i64,
    lang: String,
    expires_at: String,
    status: String,
    totp_enabled: i64,
    passkey_count: i64,
    org_timeout: Option<String>,
}

struct LockFields<'a> {
    locked_at: Option<&'a str>,
    last_seen_at: &'a str,
    auto_lock_mins: i64,
    lang: &'a str,
    expires_at: &'a str,
    status: &'a str,
    totp_enabled: i64,
    passkey_count: i64,
    org_timeout: Option<&'a str>,
}

async fn eval_lock_fields(
    st: &AppState,
    token_hash: &str,
    fields: LockFields<'_>,
    touch: bool,
) -> Option<(Lang, bool)> {
    let now_str = now();
    if fields.expires_at < now_str.as_str() {
        return None;
    }
    let has_unlock_method = fields.passkey_count > 0 || fields.totp_enabled == 1 || st.cfg.dev_mode;
    if fields.locked_at.is_some() {
        return Some((Lang::from_code(fields.lang), !has_unlock_method));
    }
    let org_timeout = fields
        .org_timeout
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(st.cfg.session_timeout_mins);
    let effective_mins = if fields.auto_lock_mins > 0 {
        fields.auto_lock_mins
    } else if fields.auto_lock_mins == 0 {
        org_timeout
    } else {
        0
    };
    if effective_mins > 0
        && fields.status == "active"
        && has_unlock_method
        && fields.last_seen_at < crate::util::minutes_ago(effective_mins).as_str()
    {
        if let Err(e) =
            sqlx::query("UPDATE sessions SET locked_at = $1 WHERE token = $2 AND locked_at IS NULL")
                .bind(&now_str)
                .bind(token_hash)
                .execute(&st.db)
                .await
        {
            tracing::warn!("failed to set session locked_at: {e}");
        }
        return Some((Lang::from_code(fields.lang), false));
    }
    if touch && fields.last_seen_at < crate::util::minutes_ago(1).as_str() {
        if let Err(e) = sqlx::query("UPDATE sessions SET last_seen_at = $1 WHERE token = $2")
            .bind(&now_str)
            .bind(token_hash)
            .execute(&st.db)
            .await
        {
            tracing::warn!("failed to touch session last_seen_at: {e}");
        }
    }
    None
}

/// Evaluates whether a session is currently locked (or has exceeded its idle timeout),
/// engaging `sessions.locked_at` when idle and updating `last_seen_at` if `touch` is true.
/// Returns `Some((lang, is_locked_without_mfa))` when the session is locked.
pub async fn evaluate_session_lock(
    st: &AppState,
    token_hash: &str,
    touch: bool,
) -> Option<(Lang, bool)> {
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
    let row = row?;
    eval_lock_fields(
        st,
        token_hash,
        LockFields {
            locked_at: row.locked_at.as_deref(),
            last_seen_at: &row.last_seen_at,
            auto_lock_mins: row.auto_lock_mins,
            lang: &row.lang,
            expires_at: &row.expires_at,
            status: &row.status,
            totp_enabled: row.totp_enabled,
            passkey_count: row.passkey_count,
            org_timeout: row.org_timeout.as_deref(),
        },
        touch,
    )
    .await
}

#[derive(sqlx::FromRow)]
struct LockAliveRow {
    locked_at: Option<String>,
    last_seen_at: String,
    created_at: String,
    auto_lock_mins: i64,
    lang: String,
    expires_at: String,
    status: String,
    totp_enabled: i64,
    passkey_count: i64,
    org_timeout: Option<String>,
    granted: String,
}

/// Checks session existence, status, expiration, 12h privileged cap, and idle auto-lock in one query for SSE.
/// A database hiccup keeps the stream alive (`true`); only a clear expired/locked/disabled
/// state returns `false`.
pub(crate) async fn evaluate_session_alive_and_lock(st: &AppState, token_hash: &str) -> bool {
    let now_str = now();
    let res: Result<Option<LockAliveRow>, _> = sqlx::query_as(
        "SELECT s.locked_at, s.last_seen_at, s.created_at, u.auto_lock_mins, u.lang, s.expires_at,
                u.status, u.totp_enabled,
                (SELECT COUNT(*) FROM passkeys p WHERE p.user_id = u.id) AS passkey_count,
                (SELECT value FROM system_settings WHERE key = 'session_timeout_mins') AS org_timeout,
                COALESCE((SELECT string_agg(rp.permission, ',') FROM role_permissions rp
                          WHERE rp.role_key = $3
                             OR rp.role_key IN (SELECT role_key FROM member_roles WHERE user_id = u.id)), '') AS granted
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = $1 AND s.expires_at > $2 AND s.locked_at IS NULL AND u.status IN ('active', 'pending')",
    )
    .bind(token_hash)
    .bind(&now_str)
    .bind(perms::MEMBER_ROLE)
    .fetch_optional(&st.db)
    .await;
    match res {
        Err(_) => true,
        Ok(None) => false,
        Ok(Some(row)) => {
            if row.status == "active" {
                let privileged = PermSet::from_keys(row.granted.split(',')).privileged();
                if privileged && row.created_at < crate::util::minutes_ago(12 * 60) {
                    return false;
                }
            }
            eval_lock_fields(
                st,
                token_hash,
                LockFields {
                    locked_at: row.locked_at.as_deref(),
                    last_seen_at: &row.last_seen_at,
                    auto_lock_mins: row.auto_lock_mins,
                    lang: &row.lang,
                    expires_at: &row.expires_at,
                    status: &row.status,
                    totp_enabled: row.totp_enabled,
                    passkey_count: row.passkey_count,
                    org_timeout: row.org_timeout.as_deref(),
                },
                false,
            )
            .await
            .is_none()
        }
    }
}

/// Unified per-request guard: resolves the session + user + permissions + lock/pending state
/// in a single database query and attaches `ResolvedSession` to request extensions so
/// `CurrentUser` does not query `sessions` / `users` again.
pub async fn session_guard(State(st): State<AppState>, mut req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let token = extract_session_token(req.headers(), st.cfg.secure_cookies());
    let Some(token) = token.filter(|t| t.len() == 48) else {
        return next.run(req).await;
    };
    let token_hash = hash_token(&token);
    let row: Option<ResolvedSessionData> = sqlx::query_as(
        "SELECT u.id, u.name, u.email, u.roles, u.status, s.mfa_ok, s.expires_at, s.created_at, u.lang,
                COALESCE((SELECT string_agg(rp.permission, ',') FROM role_permissions rp
                          WHERE rp.role_key = $2
                             OR rp.role_key IN (SELECT role_key FROM member_roles WHERE user_id = u.id)), '') AS granted,
                s.token AS session_id,
                s.locked_at, s.last_seen_at, u.auto_lock_mins, u.totp_enabled,
                (SELECT COUNT(*) FROM passkeys p WHERE p.user_id = u.id) AS passkey_count,
                (SELECT value FROM system_settings WHERE key = 'session_timeout_mins') AS org_timeout
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = $1 AND u.status IN ('active', 'pending')",
    )
    .bind(&token_hash)
    .bind(perms::MEMBER_ROLE)
    .fetch_optional(&st.db)
    .await
    .ok()
    .flatten();

    let Some(data) = row else {
        req.extensions_mut().insert(ResolvedSession::Invalid);
        return next.run(req).await;
    };

    let now_str = now();
    if data.expires_at < now_str {
        req.extensions_mut().insert(ResolvedSession::Invalid);
        return next.run(req).await;
    }

    let explicit_lang = req
        .headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .and_then(Lang::parse_accept_language);
    let user_lang = explicit_lang.unwrap_or_else(|| Lang::from_code(&data.lang));
    crate::error::set_request_lang(user_lang);

    if data.status == "pending" && !pending_profile_only(&path) {
        return forbidden(Lang::from_code(&data.lang), ErrorKey::AccountPending).into_response();
    }

    if !lock_bypass_path(&path) {
        if let Some((lang, no_mfa)) = eval_lock_fields(
            &st,
            &token_hash,
            LockFields {
                locked_at: data.locked_at.as_deref(),
                last_seen_at: &data.last_seen_at,
                auto_lock_mins: data.auto_lock_mins,
                lang: &data.lang,
                expires_at: &data.expires_at,
                status: &data.status,
                totp_enabled: data.totp_enabled,
                passkey_count: data.passkey_count,
                org_timeout: data.org_timeout.as_deref(),
            },
            true,
        )
        .await
        {
            let sub = path.strip_prefix("/api").unwrap_or(&path);
            if !(no_mfa
                && (sub == "/auth/passkey/register/start"
                    || sub == "/auth/passkey/register/finish"))
            {
                return locked(lang, ErrorKey::SessionLocked).into_response();
            }
        }
    }

    req.extensions_mut()
        .insert(ResolvedSession::Valid(Box::new(data)));
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
                Lang::from_headers(req.headers()),
                "请求来源不被允许",
                "Request origin not allowed",
            );
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": accept, "code": "FORBIDDEN" })),
            )
                .into_response();
        }
    }
    next.run(req).await
}

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob: https://*.tile.openstreetmap.org; connect-src 'self'; font-src 'self'; \
object-src 'none'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; manifest-src 'self'; worker-src 'self'";

const CSP_TURNSTILE: &str = "default-src 'self'; script-src 'self' https://challenges.cloudflare.com; style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob: https://*.tile.openstreetmap.org; connect-src 'self' https://challenges.cloudflare.com; font-src 'self'; \
frame-src 'self' https://challenges.cloudflare.com; \
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
    set(
        h,
        "content-security-policy",
        if st.cfg.turnstile.is_enabled() {
            CSP_TURNSTILE
        } else {
            CSP
        },
    );
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
    fn limiter_evicts_oldest_windows_instead_of_locking_out_new_keys() {
        let l = RateLimiter::default();
        let w = Duration::from_secs(3600);
        for i in 0..50_001 {
            assert!(l.check(&format!("b:{i}"), 10, w).is_ok());
        }
        // The map is full of live windows; brand-new keys must still be served.
        assert!(l.check("fresh:1", 10, w).is_ok());
        assert!(l.check("fresh:2", 10, w).is_ok());
        // A surviving key keeps its counter (the newest windows are not evicted).
        for _ in 0..9 {
            assert!(l.check("b:50000", 10, w).is_ok());
        }
        assert!(l.check("b:50000", 10, w).is_err());
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
