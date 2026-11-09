//! Session extraction and role model.
//!
//! Session tokens are 48 random characters given to the browser in an HttpOnly cookie.
//! Only their SHA-256 hash is stored, so a leaked database or backup cannot be replayed
//! as live sessions.

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::CookieJar;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{AppError, unauthorized};
use crate::state::AppState;
use crate::util::{now, Lang};

/// Cookie name over plain http (localhost dev).
pub const SESSION_COOKIE: &str = "gr_session";
/// Over https the `__Host-` prefix makes browsers refuse the cookie unless it is
/// Secure, Path=/ and has no Domain — it cannot be planted by a sibling subdomain.
pub const SESSION_COOKIE_SECURE: &str = "__Host-gr_session";

/// Roles: admin (管理员), transport (交通组), finance (财务), leader (活动负责人), driver (志愿司机).
/// Every active member is implicitly a passenger / participant.
pub const ROLES: &[&str] = &["admin", "transport", "finance", "leader", "driver"];
/// Roles that see other members' data and must use passkey or password+2FA.
pub const PRIVILEGED: &[&str] = &["admin", "transport", "finance"];

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub roles: Vec<String>,
    /// `active` members can use the app. `pending` members can only use their profile
    /// until an admin approves them and assigns a role and group.
    pub status: String,
    pub mfa_ok: bool,
    pub lang: Lang,
    /// SHA-256 of the session token (the primary key in `sessions`).
    #[serde(skip)]
    pub session_id: String,
}

impl CurrentUser {
    pub fn has(&self, role: &str) -> bool {
        // Admins also act as 交通组 and 活动负责人, but NOT finance or driver:
        // receipts and payment records stay with finance (按职务需要查看).
        self.roles.iter().any(|r| r == role)
            || (matches!(role, "transport" | "leader") && self.roles.iter().any(|r| r == "admin"))
    }
    pub fn is_staff(&self) -> bool {
        self.mfa_ok && (self.has("transport") || self.has("admin"))
    }
}

pub fn parse_roles(s: &str) -> Vec<String> {
    s.split(',')
        .map(|r| r.trim().to_string())
        .filter(|r| ROLES.contains(&r.as_str()))
        .collect()
}

pub fn is_privileged(roles: &str) -> bool {
    parse_roles(roles)
        .iter()
        .any(|r| PRIVILEGED.contains(&r.as_str()))
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let accept_lang = Lang::from_accept_language(
            parts
                .headers
                .get(axum::http::header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
        );
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE_SECURE)
            .or_else(|| jar.get(SESSION_COOKIE))
            .map(|c| c.value().to_string())
            .or_else(|| {
                parts
                    .headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(|s| s.to_string())
            })
            .ok_or(unauthorized(accept_lang))?;
        if token.len() != 48 {
            return Err(unauthorized(accept_lang));
        }
        let session_id = hash_token(&token);

        let row: Option<(i64, String, String, String, String, i64, String, String)> = sqlx::query_as(
            "SELECT u.id, u.name, u.email, u.roles, u.status, s.mfa_ok, s.expires_at, u.lang
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token = $1 AND u.status IN ('active', 'pending')",
        )
        .bind(&session_id)
        .fetch_optional(&state.db)
        .await?;

        let (id, name, email, roles, status, mfa_ok, expires, lang) =
            row.ok_or(unauthorized(accept_lang))?;
        if expires < now() {
            return Err(unauthorized(accept_lang));
        }
        Ok(CurrentUser {
            id,
            name,
            email,
            roles: parse_roles(&roles),
            status,
            mfa_ok: mfa_ok == 1 || state.cfg.dev_mode,
            lang: Lang::from_code(&lang),
            session_id,
        })
    }
}
