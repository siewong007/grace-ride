//! Session extraction. What the signed-in member may do is in `crate::perms`.
//!
//! Session tokens are 48 random characters given to the browser in an HttpOnly cookie.
//! Only their SHA-256 hash is stored, so a leaked database or backup cannot be replayed
//! as live sessions.

use axum::{extract::FromRequestParts, http::request::Parts};
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
}

impl CurrentUser {
    /// May do `act` on `page` in this session. Staff pages need a passkey / 2FA session
    /// on top of the permission; `util::require` tells the two failures apart.
    pub fn can(&self, page: Page, act: Act) -> bool {
        self.perms.has(page, act) && (self.mfa_ok || !page.sensitive())
    }
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
        // The session cookie is the only credential. `security::pending_guard` reads the
        // same cookie to keep unapproved members out; a second way to present the token
        // (it used to be accepted as a Bearer header) would walk straight past that guard.
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE_SECURE)
            .or_else(|| jar.get(SESSION_COOKIE))
            .map(|c| c.value().to_string())
            .ok_or(unauthorized(accept_lang))?;
        if token.len() != 48 {
            return Err(unauthorized(accept_lang));
        }
        let session_id = hash_token(&token);

        // Permissions are read with the session on every request, so a change to a role
        // or to a member's roles applies to their very next call.
        type Row = (
            i64,
            String,
            String,
            String,
            String,
            i64,
            String,
            String,
            String,
        );
        let row: Option<Row> = sqlx::query_as(
            "SELECT u.id, u.name, u.email, u.roles, u.status, s.mfa_ok, s.expires_at, u.lang,
                    COALESCE((SELECT string_agg(rp.permission, ',') FROM role_permissions rp
                              WHERE rp.role_key = $2 OR rp.role_key = ANY(string_to_array(u.roles, ','))), '')
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token = $1 AND u.status IN ('active', 'pending')",
        )
        .bind(&session_id)
        .bind(perms::MEMBER_ROLE)
        .fetch_optional(&state.db)
        .await?;

        let (id, name, email, roles, status, mfa_ok, expires, lang, granted) =
            row.ok_or(unauthorized(accept_lang))?;
        if expires < now() {
            return Err(unauthorized(accept_lang));
        }
        let perms = if status == "active" {
            PermSet::from_keys(granted.split(','))
        } else {
            PermSet::default()
        };
        let privileged = perms.privileged();
        Ok(CurrentUser {
            id,
            name,
            email,
            roles: perms::parse_roles(&roles),
            perms,
            status,
            // Accounts that reach staff pages always honour the session MFA flag (even in
            // DEV_MODE), so demos via /auth/dev-login (mfa_ok=1) still work while tests
            // can assert mfa_ok=0 is denied on staff routes.
            mfa_ok: if privileged {
                mfa_ok == 1
            } else {
                mfa_ok == 1 || state.cfg.dev_mode
            },
            lang: Lang::from_code(&lang),
            session_id,
        })
    }
}
