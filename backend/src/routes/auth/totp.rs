//! TOTP 2FA setup/verification, backup codes, session lock/unlock, and password update.

use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use totp_rs::Secret;

use super::passkeys::require_recent_sign_in;
use super::{hash_password_async, totp_for, verify_password_async, verify_totp};
use crate::auth::CurrentUser;
use crate::error::{bad, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{audit, now, Lang};

pub(super) async fn totp_setup(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    // A stolen session must not be able to silently replace an existing second factor.
    let (enabled,): (i64,) = sqlx::query_as("SELECT totp_enabled FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if enabled == 1 {
        return Err(bad(u.lang, ErrorKey::TwoFactorAlreadyEnabled));
    }
    require_recent_sign_in(&st, &u).await?;
    let secret = Secret::generate().to_base32();
    sqlx::query("UPDATE users SET totp_secret=$1, totp_enabled=0 WHERE id=$2")
        .bind(&secret)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    let totp = totp_for(&secret, &u.email, u.lang)?;
    audit(&st.db, Some(u.id), "totp.setup", "user", Some(u.id), "").await;
    let url = totp
        .to_url()
        .map_err(|e| AppError::Other(anyhow::anyhow!("totp url: {e}")))?;
    Ok(Json(json!({ "secret": secret, "url": url })))
}

#[derive(Deserialize)]
pub(super) struct CodeReq {
    code: String,
}

fn hash_backup_code(user_id: i64, norm: &str) -> String {
    crate::auth::hash_token(&format!("backup:{user_id}:{norm}"))
}

fn generate_backup_codes(user_id: i64) -> (Vec<String>, Vec<String>) {
    use rand::RngExt;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    let mut plain = Vec::with_capacity(8);
    let mut hashed = Vec::with_capacity(8);
    for _ in 0..8 {
        let p1: String = (0..4)
            .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
            .collect();
        let p2: String = (0..4)
            .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
            .collect();
        let code = format!("{p1}-{p2}");
        let norm = format!("{p1}{p2}");
        hashed.push(hash_backup_code(user_id, &norm));
        plain.push(code);
    }
    (plain, hashed)
}

pub(super) async fn totp_enable(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CodeReq>,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    let (secret, enabled): (Option<String>, i64) =
        sqlx::query_as("SELECT totp_secret, totp_enabled FROM users WHERE id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    if enabled == 1 {
        return Err(bad(u.lang, ErrorKey::TwoFactorAlreadyEnabled));
    }
    let secret = secret.ok_or_else(|| bad(u.lang, ErrorKey::StartSetupFirst))?;
    if let Err(e) = verify_totp(&st, u.id, &secret, &u.email, &req.code, u.lang).await {
        audit(&st.db, Some(u.id), "totp.failed", "user", Some(u.id), "").await;
        return Err(e);
    }
    let (plain_codes, hashed_codes) = generate_backup_codes(u.id);
    let backup_json =
        serde_json::to_string(&hashed_codes).map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    sqlx::query("UPDATE users SET totp_enabled=1, totp_backup_codes=$1 WHERE id=$2")
        .bind(&backup_json)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "totp.enable", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true, "backup_codes": plain_codes })))
}

async fn consume_backup_code(
    st: &AppState,
    user_id: i64,
    raw: &str,
    lang: Lang,
) -> AppResult<usize> {
    if st
        .limiter
        .check(
            &format!("totp_user:{user_id}"),
            10,
            std::time::Duration::from_secs(300),
        )
        .is_err()
    {
        return Err(crate::error::too_many_requests(lang, ErrorKey::RateLimited));
    }
    let norm = raw.trim().replace(['-', ' '], "").to_uppercase();
    if norm.len() != 8 {
        return Err(bad(lang, ErrorKey::InvalidCode));
    }
    let mut consumed: Option<(String,)> = None;
    for code_hash in [
        hash_backup_code(user_id, &norm),
        crate::auth::hash_token(&norm),
    ] {
        consumed = sqlx::query_as(
            "UPDATE users
             SET totp_backup_codes = (
                 SELECT COALESCE(jsonb_agg(elem ORDER BY seq), '[]'::jsonb)::text
                 FROM jsonb_array_elements_text(totp_backup_codes::jsonb)
                      WITH ORDINALITY AS t(elem, seq)
                 WHERE elem <> $2
             )
             WHERE id = $1 AND totp_backup_codes::jsonb ? $2
             RETURNING totp_backup_codes",
        )
        .bind(user_id)
        .bind(&code_hash)
        .fetch_optional(&st.db)
        .await?;
        if consumed.is_some() {
            break;
        }
    }
    let Some((remaining_json,)) = consumed else {
        audit(
            &st.db,
            Some(user_id),
            "totp.backup_code_used",
            "user",
            Some(user_id),
            "already used",
        )
        .await;
        return Err(bad(lang, ErrorKey::InvalidCode));
    };
    let remaining: usize = serde_json::from_str::<Vec<String>>(&remaining_json)
        .map(|v| v.len())
        .unwrap_or(0);
    Ok(remaining)
}

pub(super) async fn lock_session(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE sessions SET locked_at=$1 WHERE token=$2")
        .bind(now())
        .bind(&u.session_id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "session.lock", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(super) struct UnlockReq {
    totp_code: Option<String>,
    backup_code: Option<String>,
    dev_unlock: Option<bool>,
}

pub(super) async fn unlock_session(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UnlockReq>,
) -> AppResult<Json<Value>> {
    let (totp_enabled, totp_secret, passkey_count): (i64, Option<String>, i64) = sqlx::query_as(
        "SELECT u.totp_enabled, u.totp_secret, (SELECT COUNT(*) FROM passkeys WHERE user_id=u.id)
         FROM users u WHERE u.id=$1",
    )
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;

    let mut mfa_verified = false;
    if let Some(code) = req
        .totp_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if totp_enabled != 1 {
            return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
        }
        let sec = totp_secret.ok_or_else(|| bad(u.lang, ErrorKey::TwoFactorNotConfigured))?;
        if let Err(err) = verify_totp(&st, u.id, &sec, &u.email, code, u.lang).await {
            audit(
                &st.db,
                Some(u.id),
                "totp.failed",
                "user",
                Some(u.id),
                "unlock",
            )
            .await;
            return Err(err);
        }
        mfa_verified = true;
    } else if let Some(bcode) = req
        .backup_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if totp_enabled != 1 {
            return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
        }
        consume_backup_code(&st, u.id, bcode, u.lang).await?;
        mfa_verified = true;
    } else if req.dev_unlock == Some(true) && st.cfg.dev_mode {
        if passkey_count > 0 || totp_enabled == 1 {
            return Err(bad(u.lang, ErrorKey::UnlockMethodRequired));
        }
    } else {
        return Err(bad(u.lang, ErrorKey::UnlockMethodRequired));
    }

    let ts_now = now();
    if mfa_verified {
        sqlx::query("UPDATE sessions SET locked_at=NULL, last_seen_at=$1, mfa_ok=1 WHERE token=$2")
            .bind(&ts_now)
            .bind(&u.session_id)
            .execute(&st.db)
            .await?;
    } else {
        sqlx::query("UPDATE sessions SET locked_at=NULL, last_seen_at=$1 WHERE token=$2")
            .bind(&ts_now)
            .bind(&u.session_id)
            .execute(&st.db)
            .await?;
    }
    audit(
        &st.db,
        Some(u.id),
        "session.unlock",
        "user",
        Some(u.id),
        if mfa_verified { "2fa" } else { "dev" },
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn totp_verify(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<CodeReq>,
) -> AppResult<Json<Value>> {
    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT totp_secret, totp_backup_codes FROM users WHERE id=$1 AND totp_enabled=1",
    )
    .bind(u.id)
    .fetch_optional(&st.db)
    .await?;

    let Some((secret, _backup_codes)) = row else {
        return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
    };

    let clean = req.code.trim();
    // 1. Check if 6 digits TOTP
    if clean.len() == 6 && clean.chars().all(|c| c.is_ascii_digit()) {
        if let Some(sec) = secret {
            if verify_totp(&st, u.id, &sec, &u.email, clean, u.lang)
                .await
                .is_ok()
            {
                sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
                    .bind(&u.session_id)
                    .execute(&st.db)
                    .await?;
                audit(
                    &st.db,
                    Some(u.id),
                    "totp.verify",
                    "user",
                    Some(u.id),
                    "totp",
                )
                .await;
                return Ok(Json(json!({ "ok": true, "method": "totp" })));
            }
        }
    }

    // 2. Check if single-use backup recovery code (8 characters ignoring hyphens/spaces)
    // Consume the code atomically: the UPDATE only takes effect while the hash is still
    // present in the array, so two concurrent requests with the same code cannot both
    // succeed. PostgreSQL serializes row-level UPDATEs; the loser re-reads the row once
    // it gets the lock, finds the code already removed, affects 0 rows and gets an error
    // instead of HTTP 200.
    let norm = clean.replace(['-', ' '], "").to_uppercase();
    if norm.len() == 8 {
        let remaining = consume_backup_code(&st, u.id, &norm, u.lang).await?;

        sqlx::query("UPDATE sessions SET mfa_ok=1 WHERE token=$1")
            .bind(&u.session_id)
            .execute(&st.db)
            .await?;
        audit(
            &st.db,
            Some(u.id),
            "totp.backup_code_used",
            "user",
            Some(u.id),
            format!("remaining: {remaining}"),
        )
        .await;
        return Ok(Json(json!({
            "ok": true,
            "method": "backup_code",
            "remaining_backup_codes": remaining
        })));
    }

    audit(
        &st.db,
        Some(u.id),
        "totp.failed",
        "user",
        Some(u.id),
        "verify",
    )
    .await;
    Err(bad(u.lang, ErrorKey::InvalidCode))
}

pub(super) async fn totp_regenerate_backup_codes(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    let (enabled,): (i64,) = sqlx::query_as("SELECT totp_enabled FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if enabled != 1 {
        return Err(bad(u.lang, ErrorKey::TwoFactorNotConfigured));
    }
    let (plain_codes, hashed_codes) = generate_backup_codes(u.id);
    let backup_json =
        serde_json::to_string(&hashed_codes).map_err(|e| AppError::Other(anyhow::anyhow!(e)))?;
    sqlx::query("UPDATE users SET totp_backup_codes=$1 WHERE id=$2")
        .bind(&backup_json)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "totp.backup_codes_regenerated",
        "user",
        Some(u.id),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true, "backup_codes": plain_codes })))
}

#[derive(Deserialize)]
pub(super) struct PwReq {
    current: Option<String>,
    new_password: String,
}

pub(super) async fn set_password(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<PwReq>,
) -> AppResult<Json<Value>> {
    require_recent_sign_in(&st, &u).await?;
    if req.new_password.chars().count() < 10 {
        return Err(bad(u.lang, ErrorKey::PasswordTooShort));
    }
    let (hash,): (Option<String>,) = sqlx::query_as("SELECT password_hash FROM users WHERE id=$1")
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
    if let Some(h) = hash {
        let current = req.current.unwrap_or_default();
        if !verify_password_async(current, h).await? {
            audit(
                &st.db,
                Some(u.id),
                "password.failed",
                "user",
                Some(u.id),
                "current password incorrect",
            )
            .await;
            return Err(bad(u.lang, ErrorKey::CurrentPasswordIncorrect));
        }
    }
    let new_hash = hash_password_async(req.new_password).await?;
    sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
        .bind(new_hash)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    audit(&st.db, Some(u.id), "password.set", "user", Some(u.id), "").await;
    Ok(Json(json!({ "ok": true })))
}
