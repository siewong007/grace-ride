//! Web Push, native mobile push tokens, connected devices, and Telegram pairing endpoints.

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    Json,
};
use ring::hmac;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{hash_token, CurrentUser};
use crate::error::{bad, not_found, AppError, AppResult};
use crate::i18n::{ErrorKey, Lang};
use crate::state::AppState;
use crate::util::{in_minutes, now, random_token, t};

// ---------------- Device Label Helpers ----------------

pub(super) fn parse_device_label(ua: &str) -> String {
    let lower = ua.to_ascii_lowercase();
    let browser = if lower.contains("edg/") || lower.contains("edge") {
        "Edge"
    } else if lower.contains("firefox/") || lower.contains("fxios") {
        "Firefox"
    } else if lower.contains("chrome/") || lower.contains("crios") {
        "Chrome"
    } else if lower.contains("safari/") {
        "Safari"
    } else {
        "Browser"
    };
    let os = if lower.contains("iphone") {
        "iPhone"
    } else if lower.contains("ipad") {
        "iPad"
    } else if lower.contains("android") {
        "Android"
    } else if lower.contains("mac os x") || lower.contains("macintosh") {
        "macOS"
    } else if lower.contains("windows") {
        "Windows"
    } else if lower.contains("linux") {
        "Linux"
    } else {
        ""
    };
    if os.is_empty() {
        browser.to_string()
    } else {
        format!("{browser} · {os}")
    }
}

fn mask_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim();
    if let Ok(parsed) = url::Url::parse(trimmed) {
        let host = parsed.host_str().unwrap_or("push");
        let tail: String = trimmed
            .chars()
            .rev()
            .take(8)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        return format!("{host}/…{tail}");
    }
    "***".to_string()
}

pub fn telegram_webhook_secret(bot_token: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, bot_token.trim().as_bytes());
    let tag = hmac::sign(&key, b"graceride-tg-webhook");
    hex::encode(tag.as_ref())[..32].to_string()
}

// ---------------- Web Push Subscriptions ----------------

pub(super) async fn push_config(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let pub_key = crate::web_push::ensure_vapid_keys(&st.db)
        .await
        .map_err(AppError::Other)?;
    let configured = crate::web_push::load_vapid_signer(&st.db).await.is_some();
    let sub_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions WHERE user_id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let mobile_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mobile_push_tokens WHERE user_id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let secrets: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets WHERE name IN ('telegram_bot_token', 'telegram_bot_username', 'alert_webhook_url')",
    )
    .fetch_all(&st.db)
    .await?;
    let mut telegram_configured = false;
    let mut telegram_bot_username = String::new();
    let mut whatsapp_configured = false;
    for (name, value) in secrets {
        let plain = crate::crypto::decrypt_secret(&format!("app_secret:{name}"), &value)
            .unwrap_or_default();
        match name.as_str() {
            "telegram_bot_token" => telegram_configured = !plain.trim().is_empty(),
            "telegram_bot_username" => {
                telegram_bot_username = plain.trim().trim_start_matches('@').to_string();
            }
            "alert_webhook_url" => whatsapp_configured = !plain.trim().is_empty(),
            _ => {}
        }
    }
    Ok(Json(json!({
        "enabled": true,
        "configured": configured,
        "vapid_public_key": pub_key,
        "subscription_count": sub_count,
        "mobile_subscription_count": mobile_count,
        "whatsapp_configured": whatsapp_configured,
        "telegram_configured": telegram_configured,
        "telegram_bot_username": telegram_bot_username,
    })))
}

#[derive(Deserialize)]
pub(super) struct PushSubIn {
    endpoint: String,
    #[serde(default)]
    p256dh: String,
    #[serde(default)]
    auth: String,
    #[serde(default)]
    device_label: Option<String>,
}

pub(super) async fn push_subscribe(
    State(st): State<AppState>,
    u: CurrentUser,
    headers: HeaderMap,
    Json(req): Json<PushSubIn>,
) -> AppResult<Json<Value>> {
    let endpoint = req.endpoint.trim();
    if endpoint.is_empty()
        || endpoint.len() > 1000
        || !crate::util::validate_push_endpoint(endpoint, st.cfg.dev_mode).await
    {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let p256dh = req.p256dh.trim();
    let auth = req.auth.trim();
    if p256dh.len() > 256 || auth.len() > 256 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if !st.cfg.dev_mode && !crate::web_push::validate_subscription_keys(p256dh, auth) {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .chars()
        .take(200)
        .collect::<String>();
    let label = req
        .device_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(80).collect::<String>())
        .unwrap_or_else(|| parse_device_label(&ua));
    sqlx::query(
        "INSERT INTO push_subscriptions (user_id, endpoint, p256dh, auth, user_agent, device_label, created_at, session_token)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT (endpoint) DO UPDATE
         SET user_id=excluded.user_id, p256dh=excluded.p256dh, auth=excluded.auth,
             user_agent=excluded.user_agent, device_label=excluded.device_label, created_at=excluded.created_at,
             session_token=excluded.session_token",
    )
    .bind(u.id)
    .bind(endpoint)
    .bind(p256dh)
    .bind(auth)
    .bind(&ua)
    .bind(&label)
    .bind(now())
    .bind(&u.session_id)
    .execute(&st.db)
    .await?;
    sqlx::query(
        "DELETE FROM push_subscriptions
         WHERE user_id=$1
           AND id NOT IN (
               SELECT id FROM push_subscriptions
               WHERE user_id=$1
               ORDER BY created_at DESC, id DESC
               LIMIT 10
           )",
    )
    .bind(u.id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(super) struct PushUnsubIn {
    endpoint: String,
}

pub(super) async fn push_unsubscribe(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<PushUnsubIn>,
) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id=$1 AND endpoint=$2")
        .bind(u.id)
        .bind(req.endpoint.trim())
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn list_subscriptions(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let web_rows: Vec<(i64, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, endpoint, device_label, created_at, last_delivered_at
         FROM push_subscriptions WHERE user_id=$1
         ORDER BY created_at DESC, id DESC",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;

    let mob_rows: Vec<(i64, String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, platform, provider, device_label, created_at, last_delivered_at
         FROM mobile_push_tokens WHERE user_id=$1
         ORDER BY created_at DESC, id DESC",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;

    Ok(Json(json!({
        "web": web_rows.into_iter().map(|(id, ep, label, created, last)| json!({
            "id": id,
            "device_label": if label.is_empty() { "Browser".to_string() } else { label },
            "endpoint_preview": mask_endpoint(&ep),
            "created_at": created,
            "last_delivered_at": last,
        })).collect::<Vec<_>>(),
        "mobile": mob_rows.into_iter().map(|(id, platform, provider, label, created, last)| json!({
            "id": id,
            "platform": platform,
            "provider": provider,
            "device_label": if label.is_empty() { platform.clone() } else { label },
            "created_at": created,
            "last_delivered_at": last,
        })).collect::<Vec<_>>(),
    })))
}

pub(super) async fn delete_subscription(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let res = sqlx::query("DELETE FROM push_subscriptions WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn push_test(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let signer = crate::web_push::load_vapid_signer(&st.db)
        .await
        .ok_or_else(|| bad(u.lang, ErrorKey::InvalidAction))?;
    let subs: Vec<(i64, String, String, String)> = sqlx::query_as(
        "SELECT id, endpoint, p256dh, auth FROM push_subscriptions WHERE user_id=$1",
    )
    .bind(u.id)
    .fetch_all(&st.db)
    .await?;
    if subs.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let total = subs.len();
    let title = t(u.lang, "测试推送通知", "Test push notification");
    let body = t(
        u.lang,
        "此设备已成功连接恩典同行通知。",
        "This device is connected to Grace Ride alerts.",
    );
    let subject = crate::util::vapid_subject();
    let mut delivered = 0usize;
    for (sub_id, endpoint, p256dh, auth) in subs {
        if !crate::util::validate_push_endpoint(&endpoint, st.cfg.dev_mode).await {
            continue;
        }
        let Some((target_endpoint, client)) =
            crate::notifications::build_pinned_client_with_url(&endpoint, st.cfg.dev_mode).await
        else {
            continue;
        };
        match crate::web_push::send_push_message(
            &client,
            &signer,
            &subject,
            &target_endpoint,
            &p256dh,
            &auth,
            title,
            body,
            "/me",
        )
        .await
        {
            crate::web_push::PushOutcome::Delivered => {
                delivered += 1;
                let _ =
                    sqlx::query("UPDATE push_subscriptions SET last_delivered_at=$1 WHERE id=$2")
                        .bind(now())
                        .bind(sub_id)
                        .execute(&st.db)
                        .await;
            }
            crate::web_push::PushOutcome::Gone => {
                let _ = sqlx::query("DELETE FROM push_subscriptions WHERE id=$1")
                    .bind(sub_id)
                    .execute(&st.db)
                    .await;
            }
            crate::web_push::PushOutcome::RateLimited(_)
            | crate::web_push::PushOutcome::Retryable(_)
            | crate::web_push::PushOutcome::Skipped
            | crate::web_push::PushOutcome::Failed(_) => {}
        }
    }
    Ok(Json(
        json!({ "ok": true, "delivered": delivered, "total": total }),
    ))
}

// ---------------- Native Mobile Push Tokens ----------------

#[derive(Deserialize)]
pub(super) struct MobileTokenIn {
    platform: String,
    #[serde(default = "default_provider")]
    provider: String,
    token: String,
    #[serde(default)]
    device_label: String,
}

fn default_provider() -> String {
    "fcm".to_string()
}

pub(super) async fn register_mobile_token(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<MobileTokenIn>,
) -> AppResult<Json<Value>> {
    let platform = req.platform.trim();
    let provider = req.provider.trim();
    let token = req.token.trim();
    if (platform != "android" && platform != "ios")
        || (provider != "fcm" && provider != "unifiedpush" && provider != "apns")
        || !(10..=512).contains(&token.len())
    {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if provider == "apns"
        && (!(64..=256).contains(&token.len())
            || token.len() % 2 != 0
            || !token.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    if provider == "unifiedpush"
        && !crate::util::validate_webhook_target(token, st.cfg.dev_mode).await
    {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let label: String = req.device_label.trim().chars().take(80).collect();
    sqlx::query(
        "INSERT INTO mobile_push_tokens (user_id, platform, provider, token, device_label, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)
         ON CONFLICT (token) DO UPDATE
         SET user_id=excluded.user_id, platform=excluded.platform, provider=excluded.provider,
             device_label=excluded.device_label, created_at=excluded.created_at",
    )
    .bind(u.id)
    .bind(platform)
    .bind(provider)
    .bind(token)
    .bind(label)
    .bind(now())
    .execute(&st.db)
    .await?;
    sqlx::query(
        "DELETE FROM mobile_push_tokens
         WHERE user_id=$1
           AND id NOT IN (
               SELECT id FROM mobile_push_tokens
               WHERE user_id=$1
               ORDER BY created_at DESC, id DESC
               LIMIT 10
           )",
    )
    .bind(u.id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub(super) struct MobileTokenDel {
    token: String,
}

pub(super) async fn unregister_mobile_token(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<MobileTokenDel>,
) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM mobile_push_tokens WHERE user_id=$1 AND token=$2")
        .bind(u.id)
        .bind(req.token.trim())
        .execute(&st.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn delete_mobile_token(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let res = sqlx::query("DELETE FROM mobile_push_tokens WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(u.id)
        .execute(&st.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(not_found(u.lang));
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------------- One-tap Telegram Pairing & Self-Test ----------------

pub(super) async fn create_telegram_link_token(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets WHERE name IN ('telegram_bot_token', 'telegram_bot_username')",
    )
    .fetch_all(&st.db)
    .await?;
    let mut has_token = false;
    let mut bot_username = String::new();
    for (k, v) in rows {
        let plain =
            crate::crypto::decrypt_secret(&format!("app_secret:{k}"), &v).unwrap_or_default();
        match k.as_str() {
            "telegram_bot_token" => has_token = !plain.trim().is_empty(),
            "telegram_bot_username" => {
                bot_username = plain.trim().trim_start_matches('@').to_string();
            }
            _ => {}
        }
    }
    if !has_token || bot_username.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let raw_token = random_token(32);
    let token_hash = hash_token(&raw_token);
    let expires_at = in_minutes(15);

    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM telegram_link_tokens WHERE user_id=$1")
        .bind(u.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO telegram_link_tokens (token_hash, user_id, expires_at, created_at)
         VALUES ($1,$2,$3,$4)",
    )
    .bind(&token_hash)
    .bind(u.id)
    .bind(&expires_at)
    .bind(now())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "url": format!("https://t.me/{bot_username}?start={raw_token}"),
        "expires_at": expires_at,
    })))
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

async fn handle_telegram_webhook_inner(
    st: &AppState,
    presented_secret: Option<&str>,
    update: Value,
) -> AppResult<Json<Value>> {
    let token_opt: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_secrets WHERE name='telegram_bot_token'")
            .fetch_optional(&st.db)
            .await?;
    let Some(bot_token_raw) = token_opt.filter(|s| !s.trim().is_empty()) else {
        return Err(not_found(Lang::En));
    };
    let bot_token = crate::crypto::decrypt_secret("app_secret:telegram_bot_token", &bot_token_raw)
        .map_err(AppError::Other)?;
    let expected = telegram_webhook_secret(&bot_token);
    let Some(secret) = presented_secret.filter(|s| !s.is_empty()) else {
        return Err(not_found(Lang::En));
    };
    if !ct_eq(expected.as_bytes(), secret.as_bytes()) {
        return Err(not_found(Lang::En));
    }

    let Some(msg) = update.get("message") else {
        return Ok(Json(json!({ "ok": true })));
    };
    let Some(chat_id) = msg
        .get("chat")
        .and_then(|c| c.get("id"))
        .and_then(|v| v.as_i64())
    else {
        return Ok(Json(json!({ "ok": true })));
    };
    let text = msg
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim();
    let Some(rest) = text.strip_prefix("/start") else {
        return Ok(Json(json!({ "ok": true })));
    };
    let raw_token = rest.split_whitespace().next().unwrap_or("");
    if raw_token.is_empty() {
        return Ok(Json(json!({ "ok": true })));
    }

    let token_hash = hash_token(raw_token);
    let chat_str = chat_id.to_string();
    let mut tx = st.db.begin().await?;
    let consumed_user_id: Option<i64> = sqlx::query_scalar(
        "DELETE FROM telegram_link_tokens WHERE token_hash=$1 AND expires_at >= $2 RETURNING user_id",
    )
    .bind(&token_hash)
    .bind(now())
    .fetch_optional(&mut *tx)
    .await?;
    let Some(user_id) = consumed_user_id else {
        tx.commit().await?;
        return Ok(Json(json!({ "ok": true })));
    };
    let lang_s: Option<String> = sqlx::query_scalar(
        "UPDATE users SET telegram_chat_id=$1 WHERE id=$2 AND status='active' RETURNING lang",
    )
    .bind(&chat_str)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(lang_code) = lang_s else {
        tx.commit().await?;
        return Ok(Json(json!({ "ok": true })));
    };
    sqlx::query(
        "INSERT INTO user_notification_prefs (user_id, telegram_blocked, updated_at)
         VALUES ($1, 0, $2)
         ON CONFLICT (user_id) DO UPDATE SET telegram_blocked=0, updated_at=EXCLUDED.updated_at",
    )
    .bind(user_id)
    .bind(now())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let lang = Lang::from_code(&lang_code);
    let reply = t(
        lang,
        "已成功绑定您的 Grace Ride 账号！您将会在此收到接送和活动通知。",
        "Your Grace Ride account is now linked! You will receive ride and event alerts here.",
    );
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        bot_token.trim()
    );
    let _ = crate::util::alert_client()
        .post(&url)
        .json(&json!({ "chat_id": chat_str, "text": reply }))
        .send()
        .await;

    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn telegram_webhook_header(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<Value>,
) -> AppResult<Json<Value>> {
    let header_secret = headers
        .get("x-telegram-bot-api-secret-token")
        .and_then(|v| v.to_str().ok())
        .map(str::trim);
    handle_telegram_webhook_inner(&st, header_secret, update).await
}

pub(super) async fn telegram_webhook(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(path_secret): Path<String>,
    Json(update): Json<Value>,
) -> AppResult<Json<Value>> {
    let presented = headers
        .get("x-telegram-bot-api-secret-token")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(path_secret.as_str());
    handle_telegram_webhook_inner(&st, Some(presented), update).await
}

pub(super) async fn telegram_test(
    State(st): State<AppState>,
    u: CurrentUser,
) -> AppResult<Json<Value>> {
    let bot_token_raw: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_secrets WHERE name='telegram_bot_token'")
            .fetch_optional(&st.db)
            .await?;
    let bot_token = bot_token_raw
        .and_then(|raw| crate::crypto::decrypt_secret("app_secret:telegram_bot_token", &raw).ok());
    let Some(bot_token) = bot_token
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    else {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    };
    let chat_id: String =
        sqlx::query_scalar("SELECT COALESCE(telegram_chat_id, '') FROM users WHERE id=$1")
            .bind(u.id)
            .fetch_one(&st.db)
            .await?;
    let chat_id = chat_id.trim().to_string();
    if chat_id.is_empty() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }

    let msg = t(
        u.lang,
        "恩典同行测试消息：您的 Telegram 通知已成功连接。",
        "Grace Ride test alert: your Telegram notifications are connected.",
    );
    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
    match crate::util::alert_client()
        .post(&url)
        .json(&json!({ "chat_id": chat_id, "text": msg }))
        .send()
        .await
    {
        Ok(res) => {
            let status = res.status().as_u16();
            if (200..=299).contains(&status) {
                let _ = sqlx::query(
                    "UPDATE user_notification_prefs SET telegram_blocked=0, updated_at=$2 WHERE user_id=$1 AND telegram_blocked=1",
                )
                .bind(u.id)
                .bind(now())
                .execute(&st.db)
                .await;
                Ok(Json(json!({ "ok": true, "delivered": true })))
            } else {
                let txt = res.text().await.unwrap_or_default();
                if status == 403 || txt.contains("chat not found") || txt.contains("blocked") {
                    let _ = sqlx::query(
                        "INSERT INTO user_notification_prefs (user_id, telegram_blocked, updated_at)
                         VALUES ($1, 1, $2)
                         ON CONFLICT (user_id) DO UPDATE SET telegram_blocked=1, updated_at=EXCLUDED.updated_at",
                    )
                    .bind(u.id)
                    .bind(now())
                    .execute(&st.db)
                    .await;
                }
                Ok(Json(json!({
                    "ok": true,
                    "delivered": false,
                    "error": format!("HTTP {status}"),
                })))
            }
        }
        Err(e) => Ok(Json(json!({
            "ok": true,
            "delivered": false,
            "error": e.without_url().to_string(),
        }))),
    }
}
