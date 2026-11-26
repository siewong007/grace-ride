//! Admin settings: system retention/concurrency settings, and alert channels.

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
    if !(0..=1440).contains(&session_timeout) {
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

fn is_masked_url(raw: &str) -> bool {
    let t = raw.trim();
    t == "***" || t.ends_with("/***")
}

pub(super) async fn get_alert_settings(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::View)?;
    let _ = crate::web_push::ensure_vapid_keys(&st.db).await;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets
         WHERE name IN (
             'telegram_bot_token', 'telegram_bot_username',
             'alert_webhook_url', 'alert_webhook_secret', 'mobile_push_webhook_url',
             'vapid_public_key', 'vapid_private_key'
         )",
    )
    .fetch_all(&st.db)
    .await?;

    let mut tg_token = String::new();
    let mut tg_username = String::new();
    let mut webhook_url = String::new();
    let mut webhook_secret_configured = false;
    let mut mobile_webhook_url = String::new();
    let mut vapid_public_key = String::new();
    let mut vapid_private_key = String::new();
    for (k, v) in rows {
        let plain = crate::crypto::decrypt_secret(&format!("app_secret:{k}"), &v)
            .map_err(AppError::Other)?;
        match k.as_str() {
            "telegram_bot_token" => tg_token = plain,
            "telegram_bot_username" => tg_username = plain,
            "alert_webhook_url" => webhook_url = plain,
            "alert_webhook_secret" => webhook_secret_configured = !plain.trim().is_empty(),
            "mobile_push_webhook_url" => mobile_webhook_url = plain,
            "vapid_public_key" => vapid_public_key = plain,
            "vapid_private_key" => vapid_private_key = plain,
            _ => {}
        }
    }
    let can_update = u.can(Page::AdminSettings, Act::Update);
    let tg_configured = !tg_token.trim().is_empty();
    let telegram_webhook_secret = if tg_configured && can_update {
        crate::routes::community::telegram_webhook_secret(&tg_token)
    } else {
        String::new()
    };
    let telegram_webhook_path = if tg_configured {
        if can_update {
            format!("/api/push/telegram/webhook/{telegram_webhook_secret}")
        } else {
            "/api/push/telegram/webhook/<redacted>".to_string()
        }
    } else {
        String::new()
    };
    let telegram_webhook_header_path = if tg_configured {
        "/api/push/telegram/webhook"
    } else {
        ""
    };
    let vapid_private_configured =
        crate::web_push::VapidSigner::from_stored(&vapid_public_key, &vapid_private_key).is_ok();
    let push_subscriptions_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions")
            .fetch_one(&st.db)
            .await?;
    let mobile_push_subscriptions_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mobile_push_tokens")
            .fetch_one(&st.db)
            .await?;

    let stat_rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT channel, status, COUNT(*) FROM notification_deliveries
         WHERE created_at >= $1
         GROUP BY channel, status",
    )
    .bind(crate::util::in_hours(-24))
    .fetch_all(&st.db)
    .await?;
    let mut stats_map: serde_json::Map<String, Value> = serde_json::Map::new();
    for (ch, st_name, cnt) in stat_rows {
        let entry = stats_map.entry(ch).or_insert_with(
            || json!({ "delivered": 0, "pending": 0, "retry": 0, "failed": 0, "gone": 0 }),
        );
        if let Some(obj) = entry.as_object_mut() {
            obj.insert(st_name, json!(cnt));
        }
    }

    Ok(Json(json!({
        "telegram_bot_configured": tg_configured,
        "telegram_bot_username": tg_username,
        "telegram_webhook_path": telegram_webhook_path,
        "telegram_webhook_header_path": telegram_webhook_header_path,
        "telegram_webhook_secret": telegram_webhook_secret,
        "alert_webhook_configured": !webhook_url.trim().is_empty(),
        "alert_webhook_url": mask_webhook_url(&webhook_url),
        "alert_webhook_secret_configured": webhook_secret_configured,
        "mobile_push_webhook_configured": !mobile_webhook_url.trim().is_empty(),
        "mobile_push_webhook_url": mask_webhook_url(&mobile_webhook_url),
        "mobile_push_subscriptions_count": mobile_push_subscriptions_count,
        "vapid_public_key": vapid_public_key,
        "vapid_private_configured": vapid_private_configured,
        "push_subscriptions_count": push_subscriptions_count,
        "delivery_stats_24h": Value::Object(stats_map),
    })))
}

#[derive(Deserialize)]
pub(super) struct UpdateAlertSettingsIn {
    telegram_bot_token: Option<String>,
    clear_telegram_bot_token: Option<bool>,
    telegram_bot_username: Option<String>,
    alert_webhook_url: Option<String>,
    clear_alert_webhook_url: Option<bool>,
    alert_webhook_secret: Option<String>,
    clear_alert_webhook_secret: Option<bool>,
    mobile_push_webhook_url: Option<String>,
    clear_mobile_push_webhook_url: Option<bool>,
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

    if let Some(username) = req.telegram_bot_username.as_deref() {
        let cleaned = username.trim().trim_start_matches('@');
        if !cleaned.is_empty()
            && (!(5..=32).contains(&cleaned.len())
                || !cleaned
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        {
            return Err(bad(u.lang, ErrorKey::InvalidAction));
        }
    }

    let clear_webhook = req.clear_alert_webhook_url.unwrap_or(false);
    let new_webhook = req.alert_webhook_url.as_deref().map(str::trim);
    if !clear_webhook {
        if let Some(cleaned) = new_webhook.filter(|s| !s.is_empty()) {
            if is_masked_url(cleaned)
                || !crate::util::validate_webhook_target(cleaned, st.cfg.dev_mode).await
            {
                return Err(bad(u.lang, ErrorKey::InvalidAction));
            }
        }
    }

    let clear_mobile_hook = req.clear_mobile_push_webhook_url.unwrap_or(false);
    let new_mobile_hook = req.mobile_push_webhook_url.as_deref().map(str::trim);
    if !clear_mobile_hook {
        if let Some(cleaned) = new_mobile_hook.filter(|s| !s.is_empty()) {
            if is_masked_url(cleaned)
                || !crate::util::validate_webhook_target(cleaned, st.cfg.dev_mode).await
            {
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
        let enc = crate::crypto::encrypt_secret("app_secret:telegram_bot_token", tok)
            .map_err(AppError::Other)?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('telegram_bot_token', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&enc)
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
        let enc = crate::crypto::encrypt_secret("app_secret:alert_webhook_url", cleaned)
            .map_err(AppError::Other)?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('alert_webhook_url', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&enc)
        .execute(&mut *tx)
        .await?;
    }

    if req.clear_alert_webhook_secret.unwrap_or(false) {
        sqlx::query("DELETE FROM app_secrets WHERE name='alert_webhook_secret'")
            .execute(&mut *tx)
            .await?;
    } else if let Some(sec) = req
        .alert_webhook_secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let enc = crate::crypto::encrypt_secret("app_secret:alert_webhook_secret", sec)
            .map_err(AppError::Other)?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('alert_webhook_secret', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&enc)
        .execute(&mut *tx)
        .await?;
    }

    if clear_mobile_hook || new_mobile_hook == Some("") {
        sqlx::query("DELETE FROM app_secrets WHERE name='mobile_push_webhook_url'")
            .execute(&mut *tx)
            .await?;
    } else if let Some(cleaned) = new_mobile_hook.filter(|s| !s.is_empty()) {
        let enc = crate::crypto::encrypt_secret("app_secret:mobile_push_webhook_url", cleaned)
            .map_err(AppError::Other)?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('mobile_push_webhook_url', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&enc)
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
        let enc_priv = crate::crypto::encrypt_secret("app_secret:vapid_private_key", &gen_priv)
            .map_err(AppError::Other)?;
        sqlx::query(
            "INSERT INTO app_secrets (name, value) VALUES ('vapid_private_key', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&enc_priv)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM push_subscriptions")
            .execute(&mut *tx)
            .await?;
    } else if let Some(signer) = validated_signer {
        let pub_key = signer.public_key_b64();
        let priv_key = new_vapid_priv.unwrap_or_default();
        let enc_priv = crate::crypto::encrypt_secret("app_secret:vapid_private_key", priv_key)
            .map_err(AppError::Other)?;
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
        .bind(&enc_priv)
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

pub(super) async fn test_alert_webhook(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminSettings, Act::Update)?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets WHERE name IN ('alert_webhook_url', 'alert_webhook_secret')",
    )
    .fetch_all(&st.db)
    .await?;
    let mut url = String::new();
    let mut secret = String::new();
    for (k, v) in rows {
        let plain = crate::crypto::decrypt_secret(&format!("app_secret:{k}"), &v)
            .map_err(AppError::Other)?;
        match k.as_str() {
            "alert_webhook_url" => url = plain.trim().to_string(),
            "alert_webhook_secret" => secret = plain.trim().to_string(),
            _ => {}
        }
    }
    if url.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let Some((target_url, client)) =
        crate::notifications::build_pinned_client_with_url(&url, st.cfg.dev_mode).await
    else {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    };

    let delivery_id = format!("test-{}", uuid::Uuid::new_v4());
    let payload_bytes = serde_json::to_vec(&json!({
        "delivery_id": delivery_id,
        "channel": "test",
        "event": "webhook.test",
        "message": "Grace Ride test alert",
        "tested_by": u.id,
    }))
    .unwrap_or_default();
    let (ts, sig) = crate::notifications::sign_webhook_headers(&secret, &payload_bytes);
    let started = std::time::Instant::now();
    let mut req = client
        .post(&target_url)
        .header("Content-Type", "application/json")
        .header("X-GraceRide-Timestamp", ts)
        .header("X-GraceRide-Delivery-Id", &delivery_id);
    if let Some(s) = sig {
        req = req.header("X-GraceRide-Signature", s);
    }

    let result = req.body(payload_bytes).send().await;
    let latency_ms = started.elapsed().as_millis() as i64;
    match result {
        Ok(res) => {
            let status_code = res.status().as_u16();
            Ok(Json(json!({
                "ok": (200..=299).contains(&status_code),
                "status_code": status_code,
                "latency_ms": latency_ms,
            })))
        }
        Err(e) => Ok(Json(json!({
            "ok": false,
            "status_code": 0,
            "latency_ms": latency_ms,
            "error": e.without_url().to_string(),
        }))),
    }
}
