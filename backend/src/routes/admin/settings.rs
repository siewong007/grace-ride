//! Admin settings: reimbursement rates, system retention/concurrency settings, and alert channels.

use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, now, require};

#[derive(Deserialize)]
pub(super) struct UpdateRatesIn {
    rates: Vec<RateEntryIn>,
}

#[derive(Deserialize)]
struct RateEntryIn {
    vehicle_type: String,
    rate_cents_per_km: i64,
}

pub(super) async fn get_reimbursement_rates(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let rows: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT vehicle_type, rate_cents_per_km, updated_at FROM reimbursement_rates ORDER BY vehicle_type ASC",
    )
    .fetch_all(&st.db)
    .await?;

    let items: Vec<Value> = rows
        .into_iter()
        .map(|(vehicle_type, rate_cents_per_km, updated_at)| {
            json!({
                "vehicle_type": vehicle_type,
                "rate_cents_per_km": rate_cents_per_km,
                "updated_at": updated_at,
            })
        })
        .collect();

    Ok(Json(json!({ "rates": items })))
}

pub(super) async fn update_reimbursement_rates(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UpdateRatesIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;
    if req.rates.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let mut tx = st.db.begin().await?;
    let mut details = Vec::new();
    let now_str = now();
    for entry in &req.rates {
        if entry.rate_cents_per_km < 0 {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        if entry.vehicle_type != "standard" && entry.vehicle_type != "other" {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
        sqlx::query(
            "INSERT INTO reimbursement_rates (vehicle_type, rate_cents_per_km, updated_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (vehicle_type) DO UPDATE
             SET rate_cents_per_km = EXCLUDED.rate_cents_per_km, updated_at = EXCLUDED.updated_at",
        )
        .bind(&entry.vehicle_type)
        .bind(entry.rate_cents_per_km)
        .bind(&now_str)
        .execute(&mut *tx)
        .await?;
        details.push(format!(
            "{}:{}c",
            entry.vehicle_type, entry.rate_cents_per_km
        ));
    }
    tx.commit().await?;

    let detail_str = details.join(", ");
    audit(
        &st.db,
        Some(u.id),
        "settings.rates",
        "reimbursement_rates",
        None,
        &detail_str,
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(super) struct UpdateSystemSettingsIn {
    audit_retention_days: i64,
    chat_retention_days: i64,
    max_live_drivers: i64,
    session_timeout_mins: Option<i64>,
    auto_lock_mins: Option<i64>,
}

pub(super) async fn get_system_settings(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM system_settings WHERE key IN ('audit_retention_days', 'chat_retention_days', 'max_live_drivers', 'session_timeout_mins')",
    )
    .fetch_all(&st.db)
    .await?;

    let mut audit_days = st.cfg.audit_retention_days;
    let mut chat_days = st.cfg.chat_retention_days;
    let mut max_drivers = st.cfg.max_live_drivers;
    let mut session_timeout = st.cfg.session_timeout_mins;

    for (k, v) in rows {
        if let Ok(n) = v.parse::<i64>() {
            match k.as_str() {
                "audit_retention_days" => audit_days = n,
                "chat_retention_days" => chat_days = n,
                "max_live_drivers" => max_drivers = n,
                "session_timeout_mins" => session_timeout = n,
                _ => {}
            }
        }
    }

    Ok(Json(json!({
        "audit_retention_days": audit_days,
        "chat_retention_days": chat_days,
        "max_live_drivers": max_drivers,
        "session_timeout_mins": session_timeout,
        "auto_lock_mins": session_timeout,
    })))
}

pub(super) async fn update_system_settings(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UpdateSystemSettingsIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;

    let session_timeout = req
        .session_timeout_mins
        .or(req.auto_lock_mins)
        .unwrap_or(15);

    if !(30..=3650).contains(&req.audit_retention_days) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(7..=3650).contains(&req.chat_retention_days) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(1..=100).contains(&req.max_live_drivers) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !(1..=1440).contains(&session_timeout) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let now_str = now();
    let mut tx = st.db.begin().await?;

    for (k, v) in [
        ("audit_retention_days", req.audit_retention_days.to_string()),
        ("chat_retention_days", req.chat_retention_days.to_string()),
        ("max_live_drivers", req.max_live_drivers.to_string()),
        ("session_timeout_mins", session_timeout.to_string()),
    ] {
        sqlx::query(
            "INSERT INTO system_settings (key, value, updated_at, updated_by)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (key) DO UPDATE
             SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at, updated_by = EXCLUDED.updated_by",
        )
        .bind(k)
        .bind(&v)
        .bind(&now_str)
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let detail = format!(
        "audit_retention_days: {}d, chat_retention_days: {}d, max_live_drivers: {}, session_timeout_mins: {}m",
        req.audit_retention_days, req.chat_retention_days, req.max_live_drivers, session_timeout
    );
    audit(
        &st.db,
        Some(u.id),
        "settings.system",
        "system_settings",
        None,
        &detail,
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

fn mask_webhook_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(parsed) = url::Url::parse(trimmed) {
        let host = parsed.host_str().unwrap_or("");
        let port = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
        return format!("{}://{host}{port}/***", parsed.scheme());
    }
    "***".to_string()
}

fn validate_telegram_bot_token(tok: &str) -> bool {
    let Some((bot_id, secret)) = tok.split_once(':') else {
        return false;
    };
    !bot_id.is_empty()
        && bot_id.bytes().all(|b| b.is_ascii_digit())
        && secret.len() >= 10
        && secret
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub(super) async fn get_alert_settings(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let _ = crate::web_push::ensure_vapid_keys(&st.db).await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets WHERE name IN ('telegram_bot_token', 'telegram_bot_username', 'alert_webhook_url', 'vapid_public_key', 'vapid_private_key')",
    )
    .fetch_all(&st.db)
    .await?;

    let mut tg_configured = false;
    let mut tg_username = String::new();
    let mut webhook_url = String::new();
    let mut vapid_public_key = String::new();
    let mut vapid_private_key = String::new();
    for (k, v) in rows {
        match k.as_str() {
            "telegram_bot_token" => tg_configured = !v.trim().is_empty(),
            "telegram_bot_username" => tg_username = v,
            "alert_webhook_url" => webhook_url = v,
            "vapid_public_key" => vapid_public_key = v,
            "vapid_private_key" => vapid_private_key = v,
            _ => {}
        }
    }
    let vapid_private_configured =
        crate::web_push::VapidSigner::from_stored(&vapid_public_key, &vapid_private_key).is_ok();
    let push_subscriptions_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions")
            .fetch_one(&st.db)
            .await?;

    Ok(Json(json!({
        "telegram_bot_configured": tg_configured,
        "telegram_bot_username": tg_username,
        "alert_webhook_configured": !webhook_url.trim().is_empty(),
        "alert_webhook_url": mask_webhook_url(&webhook_url),
        "vapid_public_key": vapid_public_key,
        "vapid_private_configured": vapid_private_configured,
        "push_subscriptions_count": push_subscriptions_count,
    })))
}

#[derive(Deserialize)]
pub(super) struct UpdateAlertSettingsIn {
    telegram_bot_token: Option<String>,
    clear_telegram_bot_token: Option<bool>,
    telegram_bot_username: Option<String>,
    alert_webhook_url: Option<String>,
    clear_alert_webhook_url: Option<bool>,
    vapid_public_key: Option<String>,
    vapid_private_key: Option<String>,
    rotate_vapid_keys: Option<bool>,
}

pub(super) async fn update_alert_settings(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<UpdateAlertSettingsIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;

    if let Some(tok) = req
        .telegram_bot_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !req.clear_telegram_bot_token.unwrap_or(false) && !validate_telegram_bot_token(tok) {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }

    let clear_webhook = req.clear_alert_webhook_url.unwrap_or(false);
    let new_webhook = req.alert_webhook_url.as_deref().map(str::trim);
    if !clear_webhook {
        if let Some(cleaned) = new_webhook.filter(|s| !s.is_empty()) {
            if !crate::util::validate_webhook_target(cleaned, st.cfg.dev_mode).await {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
        }
    }

    let rotate_vapid = req.rotate_vapid_keys.unwrap_or(false);
    let new_vapid_pub = req.vapid_public_key.as_deref().map(str::trim);
    let new_vapid_priv = req
        .vapid_private_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let validated_signer = if !rotate_vapid {
        if let Some(priv_str) = new_vapid_priv {
            let pub_hint = new_vapid_pub.unwrap_or("");
            let signer = crate::web_push::VapidSigner::from_stored(pub_hint, priv_str)
                .map_err(|_| bad(u.lang, ErrorKey::InvalidAction))?;
            Some(signer)
        } else {
            None
        }
    } else {
        None
    };

    let mut tx = st.db.begin().await?;
    if req.clear_telegram_bot_token.unwrap_or(false) {
        sqlx::query("DELETE FROM app_secrets WHERE name='telegram_bot_token'")
            .execute(&mut *tx)
            .await?;
    } else if let Some(tok) = req
        .telegram_bot_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('telegram_bot_token', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(tok)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(username) = req.telegram_bot_username.as_deref() {
        let cleaned = username.trim().trim_start_matches('@');
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('telegram_bot_username', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(cleaned)
        .execute(&mut *tx)
        .await?;
    }

    if clear_webhook || new_webhook == Some("") {
        sqlx::query("DELETE FROM app_secrets WHERE name='alert_webhook_url'")
            .execute(&mut *tx)
            .await?;
    } else if let Some(cleaned) = new_webhook.filter(|s| !s.is_empty()) {
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('alert_webhook_url', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(cleaned)
        .execute(&mut *tx)
        .await?;
    }

    let current_pub: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_secrets WHERE name='vapid_public_key'")
            .fetch_optional(&mut *tx)
            .await?;

    if rotate_vapid {
        let (gen_pub, gen_priv) =
            crate::web_push::generate_vapid_keypair().map_err(AppError::Other)?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_public_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&gen_pub)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_private_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&gen_priv)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM push_subscriptions")
            .execute(&mut *tx)
            .await?;
    } else if let Some(signer) = validated_signer {
        let pub_key = signer.public_key_b64();
        let priv_key = new_vapid_priv.unwrap_or_default();
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_public_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(pub_key)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_private_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(priv_key)
        .execute(&mut *tx)
        .await?;
        if current_pub.as_deref().map(str::trim) != Some(pub_key) {
            sqlx::query("DELETE FROM push_subscriptions")
                .execute(&mut *tx)
                .await?;
        }
    } else if let Some(vapid) = new_vapid_pub {
        // Only block changing an already-stored public key without also providing the
        // private key (mismatched keypair). On a fresh install (current_pub is None)
        // the admin may store just the public key for client-side push config lookup
        // while keeping the private key in an external signer.
        if !vapid.is_empty() {
            if let Some(current) = current_pub.as_deref().map(str::trim) {
                if current != vapid {
                    return Err(bad(u.lang, ErrorKey::InvalidAction));
                }
            } else {
                sqlx::query(
                    "INSERT INTO app_secrets (name, value) VALUES ('vapid_public_key', $1)
                     ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
                )
                .bind(vapid)
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "settings.alerts",
        "app_secrets",
        None,
        if rotate_vapid {
            "rotated VAPID push keys"
        } else {
            "updated alert channels"
        },
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
