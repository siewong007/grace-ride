//! Google OAuth 2.0 login, invite/profile verification, unlink, and Google-verified password reset.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Query, State},
    http::HeaderMap,
    response::Redirect,
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{
    create_session, hash_password_async, verify_totp_or_backup, visitor_lang, SecondFactorOutcome,
};
use crate::auth::{extract_session_token, hash_token, CurrentUser};
use crate::error::{bad, forbidden, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, in_minutes, minutes_ago, now, random_token, Lang};

const OAUTH_NONCE_COOKIE: &str = "gr_oauth_nonce";
const OAUTH_NONCE_COOKIE_SECURE: &str = "__Host-gr_oauth_nonce";
pub(crate) const PW_RESET_COOKIE: &str = "gr_pw_reset";
pub(crate) const PW_RESET_COOKIE_SECURE: &str = "__Host-gr_pw_reset";
pub(crate) const GOOGLE_TOTP_COOKIE: &str = "gr_google_totp";
pub(crate) const GOOGLE_TOTP_COOKIE_SECURE: &str = "__Host-gr_google_totp";

fn build_ephemeral_cookie(
    secure: bool,
    plain_name: &'static str,
    secure_name: &'static str,
    value: String,
    max_age_secs: i64,
) -> Cookie<'static> {
    Cookie::build((if secure { secure_name } else { plain_name }, value))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .path("/")
        .max_age(time::Duration::seconds(max_age_secs))
        .build()
}

fn clear_ephemeral_cookie(
    jar: CookieJar,
    plain_name: &'static str,
    secure_name: &'static str,
) -> CookieJar {
    jar.remove(Cookie::build(plain_name).path("/").build())
        .remove(Cookie::build(secure_name).path("/").secure(true).build())
}

pub(super) fn clear_pending_auth_cookies(jar: CookieJar) -> CookieJar {
    let jar = clear_ephemeral_cookie(jar, PW_RESET_COOKIE, PW_RESET_COOKIE_SECURE);
    clear_ephemeral_cookie(jar, GOOGLE_TOTP_COOKIE, GOOGLE_TOTP_COOKIE_SECURE)
}

pub(super) fn clear_all_ephemeral_cookies(jar: CookieJar) -> CookieJar {
    let jar = clear_ephemeral_cookie(jar, OAUTH_NONCE_COOKIE, OAUTH_NONCE_COOKIE_SECURE);
    clear_pending_auth_cookies(jar)
}

fn extract_ephemeral_cookie(
    jar: &CookieJar,
    secure: bool,
    plain_name: &str,
    secure_name: &str,
) -> Option<String> {
    let name = if secure { secure_name } else { plain_name };
    jar.get(name).map(|c| c.value().to_string())
}

/// Only allow relative in-app paths starting with `/` (never `//`, `#`, or `/api/`).
fn safe_redirect(raw: Option<&str>, fallback: &str) -> String {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return fallback.to_string();
    };
    if s.len() <= 200
        && s.starts_with('/')
        && !s.starts_with("//")
        && !s.starts_with("/api/")
        && !s.contains('\\')
        && !s.contains('#')
        && !s.chars().any(char::is_control)
    {
        s.to_string()
    } else {
        fallback.to_string()
    }
}

fn append_query_param(base: &str, key: &str, value: &str) -> String {
    let sep = if base.contains('?') { '&' } else { '?' };
    let encoded: String = url::form_urlencoded::byte_serialize(value.as_bytes()).collect();
    format!("{base}{sep}{key}={encoded}")
}

fn redirect_err(jar: CookieJar, base: &str, code: &str) -> (CookieJar, Redirect) {
    (
        jar,
        Redirect::to(&append_query_param(base, "google_error", code)),
    )
}

pub(super) async fn google_config(State(st): State<AppState>) -> Json<Value> {
    Json(json!({ "enabled": st.google_oauth.is_enabled() }))
}

#[derive(Deserialize)]
pub(super) struct GoogleStartReq {
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    redirect: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    turnstile_token: Option<String>,
}

pub(super) async fn google_start(
    State(st): State<AppState>,
    peer: Option<axum::Extension<ConnectInfo<SocketAddr>>>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<GoogleStartReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = visitor_lang(&headers);
    if !st.google_oauth.is_enabled() {
        return Err(bad(lang, ErrorKey::GoogleNotConfigured));
    }
    let purpose = req.purpose.as_deref().unwrap_or("login").trim();
    if !matches!(purpose, "login" | "password_reset") {
        return Err(bad(lang, ErrorKey::InvalidAction));
    }
    if purpose == "password_reset" {
        let ip = crate::security::client_ip(&headers, peer.map(|p| p.0 .0), st.cfg.trust_proxy);
        st.turnstile
            .verify(
                req.turnstile_token.as_deref(),
                "password_reset",
                Some(&ip),
                lang,
            )
            .await?;
    }
    let email_hint = req
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if !email_hint.is_empty() && (!email_hint.contains('@') || email_hint.len() > 254) {
        return Err(bad(lang, ErrorKey::EmailInvalid));
    }
    let fallback_redirect = if purpose == "password_reset" {
        "/login"
    } else {
        "/"
    };
    let redirect_to = safe_redirect(req.redirect.as_deref(), fallback_redirect);

    let state_raw = random_token(48);
    let nonce_raw = random_token(48);
    let verifier = random_token(64);
    sqlx::query("DELETE FROM oauth_states WHERE created_at < $1")
        .bind(minutes_ago(15))
        .execute(&st.db)
        .await?;
    sqlx::query(
        "INSERT INTO oauth_states (state, purpose, user_id, code_verifier, redirect_to, lang, email_hint, nonce_hash, created_at)
         VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8)",
    )
    .bind(hash_token(&state_raw))
    .bind(purpose)
    .bind(&verifier)
    .bind(&redirect_to)
    .bind(lang.code())
    .bind(&email_hint)
    .bind(hash_token(&nonce_raw))
    .bind(now())
    .execute(&st.db)
    .await?;

    let hint_opt = (!email_hint.is_empty()).then_some(email_hint.as_str());
    let auth_url = if purpose == "password_reset" {
        st.google_oauth.authorization_url_with_prompt(
            &state_raw,
            &verifier,
            hint_opt,
            "login",
            Some(0),
        )?
    } else {
        st.google_oauth
            .authorization_url(&state_raw, &verifier, hint_opt)?
    };
    let jar = jar.add(build_ephemeral_cookie(
        st.cfg.secure_cookies(),
        OAUTH_NONCE_COOKIE,
        OAUTH_NONCE_COOKIE_SECURE,
        nonce_raw,
        600,
    ));
    Ok((jar, Json(json!({ "url": auth_url }))))
}

#[derive(Default, Deserialize)]
pub(super) struct GoogleLinkStartReq {
    #[serde(default)]
    redirect: Option<String>,
}

pub(super) async fn google_link_start(
    State(st): State<AppState>,
    jar: CookieJar,
    u: CurrentUser,
    body: Option<Json<GoogleLinkStartReq>>,
) -> AppResult<(CookieJar, Json<Value>)> {
    if !st.google_oauth.is_enabled() {
        return Err(bad(u.lang, ErrorKey::GoogleNotConfigured));
    }
    super::passkeys::require_recent_sign_in(&st, &u).await?;
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let redirect_to = safe_redirect(req.redirect.as_deref(), "/me?section=security");
    let state_raw = random_token(48);
    let nonce_raw = random_token(48);
    let verifier = random_token(64);
    sqlx::query("DELETE FROM oauth_states WHERE created_at < $1")
        .bind(minutes_ago(15))
        .execute(&st.db)
        .await?;
    sqlx::query(
        "INSERT INTO oauth_states (state, purpose, user_id, code_verifier, redirect_to, lang, email_hint, nonce_hash, created_at)
         VALUES ($1, 'link', $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(hash_token(&state_raw))
    .bind(u.id)
    .bind(&verifier)
    .bind(&redirect_to)
    .bind(u.lang.code())
    .bind(&u.email)
    .bind(hash_token(&nonce_raw))
    .bind(now())
    .execute(&st.db)
    .await?;

    let auth_url =
        st.google_oauth
            .authorization_url(&state_raw, &verifier, Some(u.email.as_str()))?;
    let jar = jar.add(build_ephemeral_cookie(
        st.cfg.secure_cookies(),
        OAUTH_NONCE_COOKIE,
        OAUTH_NONCE_COOKIE_SECURE,
        nonce_raw,
        600,
    ));
    Ok((jar, Json(json!({ "url": auth_url }))))
}

#[derive(Deserialize)]
pub(super) struct CallbackQuery {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(sqlx::FromRow)]
struct OAuthStateRow {
    purpose: String,
    user_id: Option<i64>,
    code_verifier: String,
    redirect_to: String,
    lang: String,
    email_hint: String,
    nonce_hash: String,
}

pub(super) async fn google_callback(
    State(st): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Query(q): Query<CallbackQuery>,
) -> AppResult<(CookieJar, Redirect)> {
    let cookie_nonce = extract_ephemeral_cookie(
        &jar,
        st.cfg.secure_cookies(),
        OAUTH_NONCE_COOKIE,
        OAUTH_NONCE_COOKIE_SECURE,
    );
    let jar = clear_ephemeral_cookie(jar, OAUTH_NONCE_COOKIE, OAUTH_NONCE_COOKIE_SECURE);
    if !st.google_oauth.is_enabled() {
        return Ok(redirect_err(
            jar,
            "/login",
            ErrorKey::GoogleNotConfigured.code(),
        ));
    }

    let Some(raw_state) = q.state.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(redirect_err(
            jar,
            "/login",
            ErrorKey::GoogleAuthFailed.code(),
        ));
    };

    let row: Option<OAuthStateRow> = sqlx::query_as(
        "DELETE FROM oauth_states
         WHERE state = $1 AND created_at >= $2
         RETURNING purpose, user_id, code_verifier, redirect_to, lang, email_hint, nonce_hash",
    )
    .bind(hash_token(raw_state))
    .bind(minutes_ago(15))
    .fetch_optional(&st.db)
    .await?;

    let Some(row) = row else {
        return Ok(redirect_err(
            jar,
            "/login",
            ErrorKey::GoogleAuthFailed.code(),
        ));
    };

    let err_base = if row.purpose == "link" {
        "/me?section=security"
    } else {
        "/login"
    };
    let state_lang = Lang::from_code(&row.lang);

    if q.error.is_some() {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::GoogleAuthCancelled.code(),
        ));
    }

    let valid_nonce = !row.nonce_hash.is_empty()
        && cookie_nonce
            .as_deref()
            .is_some_and(|n| n.len() == 48 && hash_token(n) == row.nonce_hash);
    if !valid_nonce {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::ChallengeMismatch.code(),
        ));
    }

    let Some(code) = q.code.as_deref().map(str::trim).filter(|c| !c.is_empty()) else {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::GoogleAuthFailed.code(),
        ));
    };

    let Ok(identity) = st
        .google_oauth
        .exchange_code(code, &row.code_verifier, state_lang)
        .await
    else {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::GoogleAuthFailed.code(),
        ));
    };

    if !identity.email_verified {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::GoogleEmailUnverified.code(),
        ));
    }

    match row.purpose.as_str() {
        "link" => handle_link_callback(&st, &headers, jar, &row, &identity, err_base).await,
        "login" => handle_login_callback(&st, jar, &row, &identity).await,
        "password_reset" => handle_password_reset_callback(&st, jar, &row, &identity).await,
        _ => Ok(redirect_err(
            jar,
            "/login",
            ErrorKey::GoogleAuthFailed.code(),
        )),
    }
}

async fn handle_link_callback(
    st: &AppState,
    headers: &HeaderMap,
    jar: CookieJar,
    row: &OAuthStateRow,
    identity: &crate::google_oauth::GoogleIdentity,
    err_base: &str,
) -> AppResult<(CookieJar, Redirect)> {
    let Some(sess_token) =
        extract_session_token(headers, st.cfg.secure_cookies()).filter(|t| t.len() == 48)
    else {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::ChallengeMismatch.code(),
        ));
    };
    let session_hash = hash_token(&sess_token);
    if !crate::security::evaluate_session_alive_and_lock_checked(st, &session_hash).await? {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::ChallengeMismatch.code(),
        ));
    }

    let sess_row: Option<(i64, String)> = sqlx::query_as(
        "SELECT u.id, u.email
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = $1
           AND s.expires_at > $2
           AND s.locked_at IS NULL
           AND u.status IN ('active', 'pending')",
    )
    .bind(&session_hash)
    .bind(now())
    .fetch_optional(&st.db)
    .await?;

    let Some((sess_uid, sess_email)) = sess_row else {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::ChallengeMismatch.code(),
        ));
    };

    if row.user_id != Some(sess_uid) {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::ChallengeMismatch.code(),
        ));
    }

    if !identity.email.eq_ignore_ascii_case(&sess_email) {
        audit(
            &st.db,
            Some(sess_uid),
            "google.link_failed",
            "user",
            Some(sess_uid),
            "email mismatch",
        )
        .await;
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::GoogleEmailMismatch.code(),
        ));
    }

    let mut tx = st.db.begin().await?;
    let user_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM users WHERE id = $1 FOR UPDATE")
            .bind(sess_uid)
            .fetch_optional(&mut *tx)
            .await?;
    if !matches!(user_status.as_deref(), Some("active" | "pending")) {
        return Ok(redirect_err(jar, err_base, ErrorKey::AuthDisabled.code()));
    }

    let existing_owner: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM oauth_accounts WHERE provider = 'google' AND provider_user_id = $1 FOR UPDATE",
    )
    .bind(&identity.sub)
    .fetch_optional(&mut *tx)
    .await?;
    if existing_owner.is_some_and(|other| other != sess_uid) {
        return Ok(redirect_err(
            jar,
            err_base,
            ErrorKey::GoogleAlreadyLinked.code(),
        ));
    }

    if let Err(err) = sqlx::query(
        "INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email, created_at)
         VALUES ($1, 'google', $2, $3, $4)
         ON CONFLICT (user_id, provider) DO UPDATE
           SET provider_user_id = EXCLUDED.provider_user_id,
               email = EXCLUDED.email",
    )
    .bind(sess_uid)
    .bind(&identity.sub)
    .bind(&identity.email)
    .bind(now())
    .execute(&mut *tx)
    .await
    {
        if err.as_database_error().and_then(|e| e.code()).as_deref() == Some("23505") {
            return Ok(redirect_err(
                jar,
                err_base,
                ErrorKey::GoogleAlreadyLinked.code(),
            ));
        }
        return Err(err.into());
    }
    sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
        .bind(sess_uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE user_id = $1 AND kind = 'google_totp'")
        .bind(sess_uid)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    audit(
        &st.db,
        Some(sess_uid),
        "google.link",
        "user",
        Some(sess_uid),
        "",
    )
    .await;

    let target = append_query_param(
        &safe_redirect(Some(&row.redirect_to), "/me?section=security"),
        "google",
        "linked",
    );
    Ok((jar, Redirect::to(&target)))
}

async fn handle_login_callback(
    st: &AppState,
    jar: CookieJar,
    row: &OAuthStateRow,
    identity: &crate::google_oauth::GoogleIdentity,
) -> AppResult<(CookieJar, Redirect)> {
    let linked: Option<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT oa.user_id, u.email, u.status, u.totp_enabled
         FROM oauth_accounts oa
         JOIN users u ON u.id = oa.user_id
         WHERE oa.provider = 'google' AND oa.provider_user_id = $1",
    )
    .bind(&identity.sub)
    .fetch_optional(&st.db)
    .await?;

    if let Some((uid, user_email, status, totp_enabled)) = linked {
        if !matches!(status.as_str(), "active" | "pending") {
            audit(
                &st.db,
                None,
                "login.failed",
                "user",
                Some(uid),
                "google login account disabled",
            )
            .await;
            return Ok(redirect_err(jar, "/login", ErrorKey::AuthDisabled.code()));
        }
        if !user_email.eq_ignore_ascii_case(&identity.email) {
            audit(
                &st.db,
                None,
                "login.failed",
                "user",
                Some(uid),
                "google email mismatch",
            )
            .await;
            return Ok(redirect_err(
                jar,
                "/login",
                ErrorKey::GoogleEmailMismatch.code(),
            ));
        }
        let target = safe_redirect(Some(&row.redirect_to), "/");
        if totp_enabled == 1 {
            let raw_challenge = random_token(48);
            let state_json = json!({ "redirect": target, "attempts": 0 }).to_string();
            let mut tx = st.db.begin().await?;
            sqlx::query(
                "DELETE FROM auth_challenges WHERE (user_id = $1 AND kind = 'google_totp') OR created_at < $2",
            )
            .bind(uid)
            .bind(minutes_ago(10))
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO auth_challenges (id, user_id, kind, state, created_at)
                 VALUES ($1, $2, 'google_totp', $3, $4)",
            )
            .bind(hash_token(&raw_challenge))
            .bind(uid)
            .bind(state_json)
            .bind(now())
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            let jar = jar.add(build_ephemeral_cookie(
                st.cfg.secure_cookies(),
                GOOGLE_TOTP_COOKIE,
                GOOGLE_TOTP_COOKIE_SECURE,
                raw_challenge,
                600,
            ));
            return Ok((jar, Redirect::to("/login?google_totp=1")));
        }
        let jar = create_session(st, jar, uid, false).await?;
        audit(&st.db, Some(uid), "login.google", "user", Some(uid), "").await;
        return Ok((jar, Redirect::to(&target)));
    }

    let member: Option<(i64, String)> =
        sqlx::query_as("SELECT id, status FROM users WHERE email = $1")
            .bind(&identity.email)
            .fetch_optional(&st.db)
            .await?;

    match member {
        Some((uid, status)) if status == "disabled" => {
            audit(
                &st.db,
                None,
                "login.failed",
                "user",
                Some(uid),
                "account disabled",
            )
            .await;
            Ok(redirect_err(jar, "/login", ErrorKey::AuthDisabled.code()))
        }
        Some((uid, _)) => {
            audit(
                &st.db,
                None,
                "login.failed",
                "user",
                Some(uid),
                "google account not verified",
            )
            .await;
            Ok(redirect_err(
                jar,
                "/login",
                ErrorKey::GoogleNotVerified.code(),
            ))
        }
        None => {
            audit(
                &st.db,
                None,
                "login.failed",
                "user",
                None,
                "google login non-member",
            )
            .await;
            Ok(redirect_err(
                jar,
                "/login",
                ErrorKey::GoogleNotMember.code(),
            ))
        }
    }
}

async fn handle_password_reset_callback(
    st: &AppState,
    jar: CookieJar,
    row: &OAuthStateRow,
    identity: &crate::google_oauth::GoogleIdentity,
) -> AppResult<(CookieJar, Redirect)> {
    if !row.email_hint.is_empty() && !row.email_hint.eq_ignore_ascii_case(&identity.email) {
        return Ok(redirect_err(
            jar,
            "/login",
            ErrorKey::GoogleEmailMismatch.code(),
        ));
    }

    let linked: Option<(i64, String, String)> = sqlx::query_as(
        "SELECT oa.user_id, u.email, u.status
         FROM oauth_accounts oa
         JOIN users u ON u.id = oa.user_id
         WHERE oa.provider = 'google' AND oa.provider_user_id = $1",
    )
    .bind(&identity.sub)
    .fetch_optional(&st.db)
    .await?;

    if let Some((uid, user_email, status)) = linked {
        if !matches!(status.as_str(), "active" | "pending") {
            return Ok(redirect_err(jar, "/login", ErrorKey::AuthDisabled.code()));
        }
        if !user_email.eq_ignore_ascii_case(&identity.email) {
            return Ok(redirect_err(
                jar,
                "/login",
                ErrorKey::GoogleEmailMismatch.code(),
            ));
        }

        let reset_token = random_token(48);
        let mut tx = st.db.begin().await?;
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
            .bind(uid)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO password_reset_tokens (token_hash, user_id, expires_at, created_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(hash_token(&reset_token))
        .bind(uid)
        .bind(in_minutes(15))
        .bind(now())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        audit(
            &st.db,
            Some(uid),
            "password.reset_verified",
            "user",
            Some(uid),
            "",
        )
        .await;

        let jar = jar.add(build_ephemeral_cookie(
            st.cfg.secure_cookies(),
            PW_RESET_COOKIE,
            PW_RESET_COOKIE_SECURE,
            reset_token,
            900,
        ));
        return Ok((jar, Redirect::to("/login?google_reset=1")));
    }

    let member: Option<(i64, String)> =
        sqlx::query_as("SELECT id, status FROM users WHERE email = $1")
            .bind(&identity.email)
            .fetch_optional(&st.db)
            .await?;

    match member {
        Some((_, status)) if status == "disabled" => {
            Ok(redirect_err(jar, "/login", ErrorKey::AuthDisabled.code()))
        }
        Some((uid, _)) => {
            audit(
                &st.db,
                None,
                "password.reset_denied",
                "user",
                Some(uid),
                "google account not verified",
            )
            .await;
            Ok(redirect_err(
                jar,
                "/login",
                ErrorKey::GoogleNotVerified.code(),
            ))
        }
        None => {
            audit(
                &st.db,
                None,
                "password.reset_denied",
                "user",
                None,
                "google reset non-member",
            )
            .await;
            Ok(redirect_err(
                jar,
                "/login",
                ErrorKey::GoogleNotMember.code(),
            ))
        }
    }
}

pub(super) async fn google_unlink(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    super::passkeys::require_recent_sign_in(&st, &u).await?;
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM oauth_accounts WHERE user_id = $1 AND provider = 'google'")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE user_id = $1 AND kind = 'google_totp'")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM oauth_states WHERE user_id = $1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token <> $2")
        .bind(u.id)
        .bind(&u.session_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    crate::stream::revoke_other_sessions(u.id, &u.session_id);
    audit(&st.db, Some(u.id), "google.unlink", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct GoogleTotpState {
    #[serde(default)]
    redirect: String,
    #[serde(default)]
    attempts: u32,
}

pub(super) async fn google_totp_status(
    State(st): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<Value>)> {
    let Some(raw) = extract_ephemeral_cookie(
        &jar,
        st.cfg.secure_cookies(),
        GOOGLE_TOTP_COOKIE,
        GOOGLE_TOTP_COOKIE_SECURE,
    )
    .filter(|t| t.len() == 48) else {
        let jar = clear_ephemeral_cookie(jar, GOOGLE_TOTP_COOKIE, GOOGLE_TOTP_COOKIE_SECURE);
        return Ok((jar, Json(json!({ "valid": false }))));
    };
    let email: Option<String> = sqlx::query_scalar(
        "SELECT u.email
         FROM auth_challenges c
         JOIN users u ON u.id = c.user_id
         JOIN oauth_accounts oa ON oa.user_id = u.id AND oa.provider = 'google' AND lower(oa.email) = lower(u.email)
         WHERE c.id = $1 AND c.kind = 'google_totp' AND c.created_at >= $2
           AND COALESCE((c.state::jsonb->>'attempts')::int, 0) < 5
           AND u.status IN ('active', 'pending') AND u.totp_enabled = 1",
    )
    .bind(hash_token(&raw))
    .bind(minutes_ago(10))
    .fetch_optional(&st.db)
    .await?;
    match email {
        Some(email) => Ok((jar, Json(json!({ "valid": true, "email": email })))),
        None => {
            let jar = clear_ephemeral_cookie(jar, GOOGLE_TOTP_COOKIE, GOOGLE_TOTP_COOKIE_SECURE);
            Ok((jar, Json(json!({ "valid": false }))))
        }
    }
}

#[derive(Deserialize)]
pub(super) struct GoogleTotpCompleteReq {
    code: String,
}

pub(super) async fn google_totp_complete(
    State(st): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<GoogleTotpCompleteReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = visitor_lang(&headers);
    let clean = req.code.trim();
    if clean.is_empty() {
        return Err(bad(lang, ErrorKey::TotpRequired));
    }
    let Some(raw) = extract_ephemeral_cookie(
        &jar,
        st.cfg.secure_cookies(),
        GOOGLE_TOTP_COOKIE,
        GOOGLE_TOTP_COOKIE_SECURE,
    )
    .filter(|t| t.len() == 48) else {
        return Err(bad(lang, ErrorKey::VerificationExpired));
    };
    let challenge_id = hash_token(&raw);

    let reserved: Option<(i64, String)> = sqlx::query_as(
        "UPDATE auth_challenges
         SET state = jsonb_set(
             state::jsonb,
             '{attempts}',
             to_jsonb(COALESCE((state::jsonb->>'attempts')::int, 0) + 1)
         )::text
         WHERE id = $1
           AND kind = 'google_totp'
           AND created_at >= $2
           AND COALESCE((state::jsonb->>'attempts')::int, 0) < 5
         RETURNING user_id, state",
    )
    .bind(&challenge_id)
    .bind(minutes_ago(10))
    .fetch_optional(&st.db)
    .await?;

    let Some((uid, state_json)) = reserved else {
        let _ = sqlx::query("DELETE FROM auth_challenges WHERE id = $1 AND kind = 'google_totp'")
            .bind(&challenge_id)
            .execute(&st.db)
            .await;
        return Err(bad(lang, ErrorKey::VerificationExpired));
    };

    let user_row: Option<(String, String, i64, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT u.email, u.status, u.totp_enabled, u.totp_secret, oa.email
         FROM users u
         LEFT JOIN oauth_accounts oa ON oa.user_id = u.id AND oa.provider = 'google'
         WHERE u.id = $1",
    )
    .bind(uid)
    .fetch_optional(&st.db)
    .await?;

    let Some((user_email, user_status, totp_enabled, totp_secret, oa_email)) = user_row else {
        let _ = sqlx::query("DELETE FROM auth_challenges WHERE id = $1")
            .bind(&challenge_id)
            .execute(&st.db)
            .await;
        return Err(bad(lang, ErrorKey::VerificationExpired));
    };
    if !matches!(user_status.as_str(), "active" | "pending") {
        let _ = sqlx::query("DELETE FROM auth_challenges WHERE id = $1")
            .bind(&challenge_id)
            .execute(&st.db)
            .await;
        return Err(forbidden(lang, ErrorKey::AuthDisabled));
    }
    if totp_enabled != 1
        || oa_email
            .as_deref()
            .is_none_or(|e| !e.eq_ignore_ascii_case(&user_email))
    {
        let _ = sqlx::query("DELETE FROM auth_challenges WHERE id = $1")
            .bind(&challenge_id)
            .execute(&st.db)
            .await;
        return Err(bad(lang, ErrorKey::VerificationExpired));
    }

    let parsed_state: GoogleTotpState =
        serde_json::from_str(&state_json).unwrap_or(GoogleTotpState {
            redirect: "/".into(),
            attempts: 1,
        });
    let redirect_target = safe_redirect(Some(&parsed_state.redirect), "/");
    match verify_totp_or_backup(&st, uid, totp_secret.as_deref(), &user_email, clean, lang).await {
        Ok(SecondFactorOutcome::Verified) => {}
        Ok(SecondFactorOutcome::InvalidCode) => {
            if parsed_state.attempts >= 5 {
                let _ = sqlx::query("DELETE FROM auth_challenges WHERE id = $1")
                    .bind(&challenge_id)
                    .execute(&st.db)
                    .await;
            }
            audit(
                &st.db,
                Some(uid),
                "totp.failed",
                "user",
                Some(uid),
                "google_login",
            )
            .await;
            return Err(bad(lang, ErrorKey::InvalidCode));
        }
        Err(err) => {
            let _ = sqlx::query(
                "UPDATE auth_challenges
                 SET state = jsonb_set(
                     state::jsonb,
                     '{attempts}',
                     to_jsonb(GREATEST(0, COALESCE((state::jsonb->>'attempts')::int, 1) - 1))
                 )::text
                 WHERE id = $1 AND kind = 'google_totp'",
            )
            .bind(&challenge_id)
            .execute(&st.db)
            .await;
            return Err(err);
        }
    }

    let consumed: Option<i64> = sqlx::query_scalar(
        "DELETE FROM auth_challenges
         WHERE id = $1 AND kind = 'google_totp' AND created_at >= $2
         RETURNING user_id",
    )
    .bind(&challenge_id)
    .bind(minutes_ago(10))
    .fetch_optional(&st.db)
    .await?;
    if consumed != Some(uid) {
        return Err(bad(lang, ErrorKey::VerificationExpired));
    }

    let jar = create_session(&st, jar, uid, true).await?;
    let jar = clear_ephemeral_cookie(jar, GOOGLE_TOTP_COOKIE, GOOGLE_TOTP_COOKIE_SECURE);
    audit(&st.db, Some(uid), "login.google", "user", Some(uid), "2fa").await;
    Ok((
        jar,
        Json(json!({ "ok": true, "redirect": redirect_target })),
    ))
}

pub(super) async fn password_reset_status(
    State(st): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<Value>)> {
    let Some(raw) = extract_ephemeral_cookie(
        &jar,
        st.cfg.secure_cookies(),
        PW_RESET_COOKIE,
        PW_RESET_COOKIE_SECURE,
    )
    .filter(|t| t.len() == 48) else {
        let jar = clear_ephemeral_cookie(jar, PW_RESET_COOKIE, PW_RESET_COOKIE_SECURE);
        return Ok((jar, Json(json!({ "valid": false }))));
    };
    let email: Option<String> = sqlx::query_scalar(
        "SELECT u.email
         FROM password_reset_tokens t
         JOIN users u ON u.id = t.user_id
         JOIN oauth_accounts oa ON oa.user_id = u.id AND oa.provider = 'google' AND lower(oa.email) = lower(u.email)
         WHERE t.token_hash = $1 AND t.expires_at >= $2 AND u.status IN ('active', 'pending')",
    )
    .bind(hash_token(&raw))
    .bind(now())
    .fetch_optional(&st.db)
    .await?;
    match email {
        Some(email) => Ok((jar, Json(json!({ "valid": true, "email": email })))),
        None => {
            let jar = clear_ephemeral_cookie(jar, PW_RESET_COOKIE, PW_RESET_COOKIE_SECURE);
            Ok((jar, Json(json!({ "valid": false }))))
        }
    }
}

#[derive(Deserialize)]
pub(super) struct PasswordResetCompleteReq {
    new_password: String,
}

pub(super) async fn password_reset_complete(
    State(st): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(req): Json<PasswordResetCompleteReq>,
) -> AppResult<(CookieJar, Json<Value>)> {
    let lang = visitor_lang(&headers);
    if req.new_password.chars().count() < 10 {
        return Err(bad(lang, ErrorKey::PasswordTooShort));
    }
    if req.new_password.len() > super::MAX_PASSWORD_BYTES {
        return Err(bad(lang, ErrorKey::PasswordTooLong));
    }
    let Some(raw) = extract_ephemeral_cookie(
        &jar,
        st.cfg.secure_cookies(),
        PW_RESET_COOKIE,
        PW_RESET_COOKIE_SECURE,
    )
    .filter(|t| t.len() == 48) else {
        return Err(bad(lang, ErrorKey::PasswordResetInvalid));
    };
    let token_hash = hash_token(&raw);

    let precheck: Option<(i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT u.id, u.email, u.status, oa.email
         FROM password_reset_tokens t
         JOIN users u ON u.id = t.user_id
         LEFT JOIN oauth_accounts oa ON oa.user_id = u.id AND oa.provider = 'google'
         WHERE t.token_hash = $1 AND t.expires_at >= $2",
    )
    .bind(&token_hash)
    .bind(now())
    .fetch_optional(&st.db)
    .await?;

    let Some((_uid, pre_email, pre_status, pre_oa_email)) = precheck else {
        return Err(bad(lang, ErrorKey::PasswordResetInvalid));
    };
    if !matches!(pre_status.as_str(), "active" | "pending") {
        return Err(forbidden(lang, ErrorKey::AuthDisabled));
    }
    if pre_oa_email
        .as_deref()
        .is_none_or(|e| !e.eq_ignore_ascii_case(&pre_email))
    {
        return Err(bad(lang, ErrorKey::GoogleNotVerified));
    }

    let new_hash = hash_password_async(req.new_password).await?;
    let mut tx = st.db.begin().await?;
    let consumed: Option<i64> = sqlx::query_scalar(
        "DELETE FROM password_reset_tokens
         WHERE token_hash = $1 AND expires_at >= $2
         RETURNING user_id",
    )
    .bind(&token_hash)
    .bind(now())
    .fetch_optional(&mut *tx)
    .await?;
    let Some(uid) = consumed else {
        return Err(bad(lang, ErrorKey::PasswordResetInvalid));
    };

    let (user_email, user_status, oa_email): (String, String, Option<String>) = sqlx::query_as(
        "SELECT u.email, u.status, oa.email
         FROM users u
         LEFT JOIN oauth_accounts oa ON oa.user_id = u.id AND oa.provider = 'google'
         WHERE u.id = $1
         FOR UPDATE OF u",
    )
    .bind(uid)
    .fetch_one(&mut *tx)
    .await?;
    if !matches!(user_status.as_str(), "active" | "pending") {
        return Err(forbidden(lang, ErrorKey::AuthDisabled));
    }
    if oa_email
        .as_deref()
        .is_none_or(|e| !e.eq_ignore_ascii_case(&user_email))
    {
        return Err(bad(lang, ErrorKey::GoogleNotVerified));
    }

    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(new_hash)
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM auth_challenges WHERE user_id = $1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM oauth_states WHERE user_id = $1")
        .bind(uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM login_failures WHERE lower(email) = lower($1)")
        .bind(&user_email)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    crate::stream::revoke_user(uid);
    st.limiter
        .reset(&format!("pw_attempt:user:{}", user_email.to_lowercase()));

    let jar = clear_pending_auth_cookies(jar);
    audit(&st.db, Some(uid), "password.reset", "user", Some(uid), "").await;
    Ok((jar, Json(json!({ "ok": true }))))
}
