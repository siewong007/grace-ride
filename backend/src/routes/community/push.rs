//! Web Push VAPID configuration and device subscription endpoints.

use axum::{
    extract::State,
    http::{header, HeaderMap},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::state::AppState;
use crate::util::{now, t};

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
    let secrets: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, value FROM app_secrets WHERE name IN ('telegram_bot_token', 'telegram_bot_username', 'alert_webhook_url')",
    )
    .fetch_all(&st.db)
    .await?;
    let mut telegram_configured = false;
    let mut telegram_bot_username = String::new();
    let mut whatsapp_configured = false;
    for (name, value) in secrets {
        match name.as_str() {
            "telegram_bot_token" => telegram_configured = !value.trim().is_empty(),
            "telegram_bot_username" => {
                telegram_bot_username = value.trim().trim_start_matches('@').to_string();
            }
            "alert_webhook_url" => whatsapp_configured = !value.trim().is_empty(),
            _ => {}
        }
    }
    Ok(Json(json!({
        "enabled": true,
        "configured": configured,
        "vapid_public_key": pub_key,
        "subscription_count": sub_count,
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
    sqlx::query(
        "INSERT INTO push_subscriptions (user_id, endpoint, p256dh, auth, user_agent, created_at)
         VALUES ($1,$2,$3,$4,$5,$6)
         ON CONFLICT (endpoint) DO UPDATE
         SET user_id=excluded.user_id, p256dh=excluded.p256dh, auth=excluded.auth,
             user_agent=excluded.user_agent, created_at=excluded.created_at",
    )
    .bind(u.id)
    .bind(endpoint)
    .bind(p256dh)
    .bind(auth)
    .bind(ua)
    .bind(now())
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
    let client = crate::util::alert_client();
    let mut delivered = 0usize;
    for (sub_id, endpoint, p256dh, auth) in subs {
        if !crate::util::validate_push_endpoint(&endpoint, st.cfg.dev_mode).await {
            continue;
        }
        match crate::web_push::send_push_message(
            client, &signer, &subject, &endpoint, &p256dh, &auth, title, body, "/me",
        )
        .await
        {
            crate::web_push::PushOutcome::Delivered => delivered += 1,
            crate::web_push::PushOutcome::Gone => {
                let _ = sqlx::query("DELETE FROM push_subscriptions WHERE id=$1")
                    .bind(sub_id)
                    .execute(&st.db)
                    .await;
            }
            crate::web_push::PushOutcome::Skipped | crate::web_push::PushOutcome::Failed => {}
        }
    }
    Ok(Json(
        json!({ "ok": true, "delivered": delivered, "total": total }),
    ))
}
