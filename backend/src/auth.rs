//! Session extraction. What the signed-in member may do is in `crate::perms`.
//!
//! Session tokens are 48 random characters given to the browser in an HttpOnly cookie.
//! Only their SHA-256 hash is stored, so a leaked database or backup cannot be replayed
//! as live sessions.

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use axum_extra::extract::CookieJar;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{unauthorized, AppError};
use crate::perms::{self, Act, Page, PermSet};
use crate::state::AppState;
use crate::util::{now, Lang};

/// Cookie name over plain http (localhost dev).
pub const SESSION_COOKIE: &str = "gr_session";
/// Over https the `__Host-` prefix makes browsers refuse the cookie unless it is
/// Secure, Path=/ and has no Domain — it cannot be planted by a sibling subdomain.
pub const SESSION_COOKIE_SECURE: &str = "__Host-gr_session";

pub fn session_cookie_name(secure_cookies: bool) -> &'static str {
    if secure_cookies {
        SESSION_COOKIE_SECURE
    } else {
        SESSION_COOKIE
    }
}

pub fn extract_session_token(
    headers: &axum::http::HeaderMap,
    secure_cookies: bool,
) -> Option<String> {
    let jar = CookieJar::from_headers(headers);
    jar.get(session_cookie_name(secure_cookies))
        .map(|c| c.value().to_string())
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: i64,
    pub name: String,
    pub email: String,
    /// Role keys from `users.roles`. Every active member also holds `member`.
    pub roles: Vec<String>,
    /// Everything those roles grant. Empty until an admin approves the member.
    #[serde(skip)]
    pub perms: PermSet,
    /// `active` members can use the app. `pending` members can only use their profile
    /// until an admin approves them and assigns a role and group.
    pub status: String,
    pub mfa_ok: bool,
    pub lang: Lang,
    /// SHA-256 of the session token (the primary key in `sessions`).
    #[serde(skip)]
    pub session_id: String,
    /// The member's groups plus every group above them. An event for any of these is
    /// one they may see and join (`crate::audience`).
    #[serde(skip)]
    pub group_scope: Vec<i64>,
    /// The member's groups plus every group below them. As a driver they may take event
    /// rides whose group is one of these.
    #[serde(skip)]
    pub drive_scope: Vec<i64>,
}

impl CurrentUser {
    /// May do `act` on `page` in this session. Staff pages need a passkey / 2FA session
    /// on top of the permission; `util::require` tells the two failures apart.
    pub fn can(&self, page: Page, act: Act) -> bool {
        self.perms.has(page, act) && (self.mfa_ok || !page.sensitive())
    }

    /// Traffic staff who run every event. A cell group leader also holds the traffic
    /// tools, but only for events they created (their own group's activity), so the
    /// ride, live-map and plan pages scope them to those events.
    pub fn traffic_is_global(&self) -> bool {
        self.can(Page::AdminDashboard, Act::View)
    }

    /// Whether the user has authority to approve transport arrangement plans as traffic lead.
    pub fn is_traffic_lead(&self) -> bool {
        self.mfa_ok && perms::can_approve_transport_plan(&self.roles)
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: i64,
    name: String,
    email: String,
    roles: String,
    status: String,
    mfa_ok: i64,
    expires_at: String,
    created_at: String,
    lang: String,
    granted: String,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let explicit_lang = parts
            .headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .and_then(Lang::parse_accept_language);
        let unauth_lang = explicit_lang.unwrap_or(Lang::Zh);

        // The session cookie is the only credential. `security::session_guard` reads the
        // same cookie to keep unapproved/locked members out and caches the resolved session
        // in request extensions; a second way to present the token would walk past that guard.
        if let Some(resolved) = parts.extensions.get::<crate::security::ResolvedSession>() {
            match resolved {
                crate::security::ResolvedSession::Invalid => {
                    return Err(unauthorized(unauth_lang));
                }
                crate::security::ResolvedSession::Valid(srow) => {
                    let user_lang = explicit_lang.unwrap_or_else(|| Lang::from_code(&srow.lang));
                    crate::error::set_request_lang(user_lang);
                    if srow.expires_at < now() {
                        return Err(unauthorized(user_lang));
                    }
                    let perms = if srow.status == "active" {
                        PermSet::from_keys(srow.granted.split(','))
                    } else {
                        PermSet::default()
                    };
                    let privileged = perms.privileged();
                    if privileged && srow.created_at < crate::util::minutes_ago(12 * 60) {
                        return Err(unauthorized(user_lang));
                    }
                    let (group_scope, drive_scope) = if srow.status == "active" {
                        crate::audience::group_scopes(&state.db, srow.id).await?
                    } else {
                        (Vec::new(), Vec::new())
                    };
                    return Ok(CurrentUser {
                        id: srow.id,
                        name: srow.name.clone(),
                        email: srow.email.clone(),
                        roles: perms::parse_roles(&srow.roles),
                        perms,
                        status: srow.status.clone(),
                        mfa_ok: if privileged {
                            srow.mfa_ok == 1
                        } else {
                            srow.mfa_ok == 1 || state.cfg.dev_mode
                        },
                        lang: user_lang,
                        session_id: srow.session_id.clone(),
                        group_scope,
                        drive_scope,
                    });
                }
            }
        }

        let token = extract_session_token(&parts.headers, state.cfg.secure_cookies())
            .ok_or(unauthorized(unauth_lang))?;
        if token.len() != 48 {
            return Err(unauthorized(unauth_lang));
        }
        let session_id = hash_token(&token);

        // Fallback path when CurrentUser is extracted without `security::session_guard`.
        let row: Option<SessionRow> = sqlx::query_as(
            "SELECT u.id, u.name, u.email, u.roles, u.status, s.mfa_ok, s.expires_at, s.created_at, u.lang,
                    COALESCE((SELECT string_agg(rp.permission, ',') FROM role_permissions rp
                              WHERE rp.role_key = $2
                                 OR rp.role_key IN (SELECT role_key FROM member_roles WHERE user_id = u.id)), '') AS granted
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token = $1 AND u.status IN ('active', 'pending')",
        )
        .bind(&session_id)
        .bind(perms::MEMBER_ROLE)
        .fetch_optional(&state.db)
        .await?;

        let srow = row.ok_or(unauthorized(unauth_lang))?;
        let user_lang = explicit_lang.unwrap_or_else(|| Lang::from_code(&srow.lang));
        crate::error::set_request_lang(user_lang);
        if srow.expires_at < now() {
            return Err(unauthorized(user_lang));
        }
        let perms = if srow.status == "active" {
            PermSet::from_keys(srow.granted.split(','))
        } else {
            PermSet::default()
        };
        let privileged = perms.privileged();
        // Staff sessions that can access sensitive/other members' data must be capped at 12 hours.
        if privileged && srow.created_at < crate::util::minutes_ago(12 * 60) {
            return Err(unauthorized(user_lang));
        }
        let (group_scope, drive_scope) = if srow.status == "active" {
            crate::audience::group_scopes(&state.db, srow.id).await?
        } else {
            (Vec::new(), Vec::new())
        };
        Ok(CurrentUser {
            id: srow.id,
            name: srow.name,
            email: srow.email,
            roles: perms::parse_roles(&srow.roles),
            perms,
            status: srow.status,
            // Accounts that reach staff pages always honour the session MFA flag (even in
            // DEV_MODE), so demos via /auth/dev-login (mfa_ok=1) still work while tests
            // can assert mfa_ok=0 is denied on staff routes.
            mfa_ok: if privileged {
                srow.mfa_ok == 1
            } else {
                srow.mfa_ok == 1 || state.cfg.dev_mode
            },
            lang: user_lang,
            session_id,
            group_scope,
            drive_scope,
        })
    }
}
