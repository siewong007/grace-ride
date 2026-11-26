//! End-to-end integration tests for notification enhancements:
//! - Webhook masking guard, HMAC-SHA256 signatures, and /admin/settings/alerts/test-webhook
//! - Decoupled passenger/driver reminders & carpool grouping (+ trigger re-arm on pickup_at change)
//! - Inbox cursor pagination, filters (unread_only, category), mark-unread, delete, category read-all
//! - Single-device logout push_endpoint cleanup
//! - Durable outbox retry (5xx -> 200) & Web Push 410 subscription pruning
//! - User notification preferences: numeric Telegram Chat ID, muted categories, quiet hours + urgent bypass, hide_lock_details & Telegram 1-tap pairing webhook
//! - Per-user SSE EventHub fan-out >300 users without lagging
//! - Mobile push token register/list/unregister lifecycle
//! - Offline trip chat throttled alerts

mod common;

use axum::{
    body::Bytes,
    http::{HeaderMap, Method, StatusCode},
    routing::post,
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use common::*;
use grace_ride::{
    jobs,
    notifications::{notify_trip_chat, notify_with, process_due_deliveries, NotifMeta},
    state::AppState,
    stream::{EventHub, Push},
    util::{in_minutes, today_local},
};
use ring::{
    agreement::{EphemeralPrivateKey, ECDH_P256},
    hmac,
    rand::SystemRandom,
    signature::{EcdsaKeyPair, KeyPair as _, ECDSA_P256_SHA256_FIXED_SIGNING},
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

async fn user_id(st: &AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn first_area_id(st: &AppState) -> i64 {
    sqlx::query_scalar("SELECT id FROM areas WHERE active=1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

fn future_pickup_iso(days: i64, hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (today_local() + chrono::Duration::days(days)).format("%Y-%m-%d")
    )
}

type CapturedReq = (Vec<(String, String)>, Vec<u8>);

#[tokio::test]
async fn masked_webhook_rejection_hmac_signature_and_test_webhook() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;

    let captured: Arc<Mutex<Vec<CapturedReq>>> = Arc::new(Mutex::new(Vec::new()));
    let store = captured.clone();
    let mock = Router::new().route(
        "/hook",
        post(move |hdrs: HeaderMap, body: Bytes| {
            let s = store.clone();
            async move {
                let pairs = hdrs
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                s.lock().unwrap().push((pairs, body.to_vec()));
                StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, mock).await;
    });

    // Masked URL from UI must never be accepted back.
    let bad_masked = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "alert_webhook_url": format!("http://{addr}/***") })),
    )
    .await;
    assert_eq!(bad_masked.status, StatusCode::BAD_REQUEST);

    // Configure real webhook + HMAC secret.
    let secret = "grace-webhook-signing-secret";
    let ok = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({
            "alert_webhook_url": format!("http://{addr}/hook"),
            "alert_webhook_secret": secret,
        })),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);

    let cfg = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(cfg.status, StatusCode::OK);
    assert_eq!(cfg.json["alert_webhook_configured"], true);
    assert_eq!(cfg.json["alert_webhook_secret_configured"], true);
    assert!(cfg.json["alert_webhook_url"]
        .as_str()
        .unwrap()
        .ends_with("/***"));

    // Editing another field with omitted alert_webhook_url must preserve saved URL.
    let ok2 = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "telegram_bot_username": "GraceRideTestBot" })),
    )
    .await;
    assert_eq!(ok2.status, StatusCode::OK, "{}", ok2.json);
    let stored_url: String =
        sqlx::query_scalar("SELECT value FROM app_secrets WHERE name='alert_webhook_url'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(stored_url, format!("http://{addr}/hook"));

    // Test webhook button sends signed JSON payload.
    let test_res = call(
        &app,
        Method::POST,
        "/api/admin/settings/alerts/test-webhook",
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(test_res.status, StatusCode::OK, "{}", test_res.json);
    assert_eq!(test_res.json["ok"], true);
    assert_eq!(test_res.json["status_code"], 200);

    let reqs = captured.lock().unwrap();
    assert_eq!(reqs.len(), 1);
    let (hdrs, body) = &reqs[0];
    let get_hdr = |n: &str| {
        hdrs.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(n))
            .map(|(_, v)| v.as_str())
    };
    let ts = get_hdr("x-graceride-timestamp").expect("timestamp header");
    let sig = get_hdr("x-graceride-signature").expect("signature header");
    assert!(get_hdr("x-graceride-delivery-id").is_some());

    let hex_sig = sig.strip_prefix("sha256=").expect("sha256= prefix");
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let mut ctx = hmac::Context::with_key(&key);
    ctx.update(ts.as_bytes());
    ctx.update(b".");
    ctx.update(body);
    let expected: String = ctx
        .sign()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hex_sig, expected);

    let parsed: Value = serde_json::from_slice(body).unwrap();
    assert_eq!(parsed["event"], "webhook.test");
}

#[tokio::test]
async fn decoupled_driver_and_passenger_reminders_and_carpool_grouping() {
    let (app, st) = setup().await;
    let p1 = login(&st, "member@example.org").await;
    let p2 = login(&st, "amina@example.org").await;
    let drv = login(&st, "driver@example.org").await;
    let p1_id = user_id(&st, "member@example.org").await;
    let p2_id = user_id(&st, "amina@example.org").await;
    let drv_id = user_id(&st, "driver@example.org").await;
    let area = first_area_id(&st).await;

    // P1 opts out of passenger readiness prompts; P2 keeps 45m; Driver configures 90m lead time.
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/notifications/preferences",
            Some(&p1),
            Some(json!({ "readiness_prompt_mins": 0 })),
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/notifications/preferences",
            Some(&drv),
            Some(json!({ "driver_remind_mins": 90 })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let r1 = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&p1),
        Some(json!({
            "pickup_area_id": area,
            "pickup_address": "1 Jalan Satu",
            "destination": "Church Main",
            "pickup_at": future_pickup_iso(2, "09:00"),
            "direction": "to_church",
        })),
    )
    .await;
    assert_eq!(r1.status, StatusCode::OK, "{}", r1.json);
    let r1_id = r1.json["id"].as_i64().unwrap();

    let r2 = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&p2),
        Some(json!({
            "pickup_area_id": area,
            "pickup_address": "2 Jalan Dua",
            "destination": "Church Main",
            "pickup_at": future_pickup_iso(2, "09:05"),
            "direction": "to_church",
        })),
    )
    .await;
    assert_eq!(r2.status, StatusCode::OK, "{}", r2.json);
    let r2_id = r2.json["id"].as_i64().unwrap();

    // Claim r1 so a valid trip_id row exists, then place both confirmed rides inside 40m window on the same trip.
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{r1_id}/claim"),
            Some(&drv),
            Some(json!({})),
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{r1_id}/respond"),
            Some(&p1),
            Some(json!({ "accept": true })),
        )
        .await
        .status,
        StatusCode::OK
    );
    let admin = login(&st, "admin@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{r1_id}/approve"),
            Some(&admin),
            None,
        )
        .await
        .status,
        StatusCode::OK
    );
    let trip_id: i64 = sqlx::query_scalar("SELECT trip_id FROM rides WHERE id=$1")
        .bind(r1_id)
        .fetch_one(&st.db)
        .await
        .unwrap();

    let soon = in_minutes(35);
    sqlx::query(
        "UPDATE rides
         SET status='confirmed', driver_id=$1, trip_id=$2, pickup_at=$3,
             reminder_sent=0, driver_reminder_sent=0
         WHERE id = ANY($4)",
    )
    .bind(drv_id)
    .bind(trip_id)
    .bind(&soon)
    .bind(vec![r1_id, r2_id])
    .execute(&st.db)
    .await
    .unwrap();
    sqlx::query("DELETE FROM notifications WHERE user_id = ANY($1)")
        .bind(vec![p1_id, p2_id, drv_id])
        .execute(&st.db)
        .await
        .unwrap();

    jobs::run_reminders(&st.db).await.unwrap();

    // P1 had readiness_prompt_mins=0: still gets NO readiness notification even though driver got reminded.
    let p1_notifs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id=$1 AND category='ride'",
    )
    .bind(p1_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(p1_notifs, 0);

    // P2 gets readiness prompt.
    let p2_links: Vec<String> =
        sqlx::query_scalar("SELECT link FROM notifications WHERE user_id=$1 AND category='ride'")
            .bind(p2_id)
            .fetch_all(&st.db)
            .await
            .unwrap();
    assert_eq!(p2_links, vec![format!("/rides/{r2_id}?action=readiness")]);

    // Driver gets ONE grouped carpool reminder for the whole trip rather than N separate pings.
    let drv_notifs: Vec<(String, String)> = sqlx::query_as(
        "SELECT title, body FROM notifications WHERE user_id=$1 AND event_type='ride_reminder' AND entity_id = ANY($2)",
    )
    .bind(drv_id)
    .bind(vec![r1_id, r2_id])
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(drv_notifs.len(), 1);
    assert!(
        drv_notifs[0].1.contains("2") || drv_notifs[0].1.contains(","),
        "expected grouped summary in driver body: {}",
        drv_notifs[0].1
    );

    // Rescheduling pickup_at must re-arm both reminder_sent and driver_reminder_sent via DB trigger.
    let later = in_minutes(42);
    sqlx::query("UPDATE rides SET pickup_at=$1 WHERE id=$2")
        .bind(&later)
        .bind(r2_id)
        .execute(&st.db)
        .await
        .unwrap();
    let (rem_p, rem_d): (i64, i64) =
        sqlx::query_as("SELECT reminder_sent, driver_reminder_sent FROM rides WHERE id=$1")
            .bind(r2_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!((rem_p, rem_d), (0, 0));
}

#[tokio::test]
async fn inbox_pagination_filters_unread_delete_and_category_read_all() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let uid = user_id(&st, "member@example.org").await;
    sqlx::query("DELETE FROM notifications WHERE user_id=$1")
        .bind(uid)
        .execute(&st.db)
        .await
        .unwrap();

    notify_with(
        &st.db,
        uid,
        ("行程A", "Ride A"),
        ("内容", "Body"),
        "/rides/10",
        NotifMeta {
            category: "ride",
            event_type: "ride_confirmed".into(),
            priority: "normal",
            entity_type: "ride",
            entity_id: Some(10),
            tag: "ride-10".into(),
        },
    )
    .await;
    notify_with(
        &st.db,
        uid,
        ("活动B", "Event B"),
        ("内容", "Body"),
        "/events/20",
        NotifMeta {
            category: "event",
            event_type: "event_new".into(),
            priority: "normal",
            entity_type: "event",
            entity_id: Some(20),
            tag: "event-20".into(),
        },
    )
    .await;
    notify_with(
        &st.db,
        uid,
        ("行程C", "Ride C"),
        ("内容", "Body"),
        "/rides/11",
        NotifMeta {
            category: "ride",
            event_type: "ride_confirmed".into(),
            priority: "normal",
            entity_type: "ride",
            entity_id: Some(11),
            tag: "ride-11".into(),
        },
    )
    .await;

    // Invalid category returns 400.
    let bad_cat = call(
        &app,
        Method::GET,
        "/api/notifications?category=bogus",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(bad_cat.status, StatusCode::BAD_REQUEST);

    // Page 1 filtered to category=ride & limit=1 returns Ride C + cursor.
    let p1 = call(
        &app,
        Method::GET,
        "/api/notifications?category=ride&unread_only=true&limit=1",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(p1.status, StatusCode::OK, "{}", p1.json);
    assert_eq!(p1.json["items"].as_array().unwrap().len(), 1);
    assert_eq!(p1.json["unread"], 3);
    let c_id = p1.json["items"][0]["id"].as_i64().unwrap();
    let cursor = p1.json["next_before"].as_i64().unwrap();

    // Cursor page 2 returns Ride A and null next_before.
    let p2 = call(
        &app,
        Method::GET,
        &format!("/api/notifications?category=ride&unread_only=true&limit=1&before={cursor}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(p2.json["items"].as_array().unwrap().len(), 1);
    assert!(p2.json["next_before"].is_null());

    // Mark Ride C read, then mark unread again.
    let r = call(
        &app,
        Method::POST,
        &format!("/api/notifications/{c_id}/read"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(r.json["unread"], 2);
    let u = call(
        &app,
        Method::POST,
        &format!("/api/notifications/{c_id}/unread"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(u.status, StatusCode::OK);
    assert_eq!(u.json["unread"], 3);

    // Mark only ride notifications read -> event remains unread (unread == 1).
    let ra = call(
        &app,
        Method::POST,
        "/api/notifications/read-all",
        Some(&member),
        Some(json!({ "category": "ride" })),
    )
    .await;
    assert_eq!(ra.status, StatusCode::OK);
    assert_eq!(ra.json["unread"], 1);

    // Delete the unread event notification -> unread becomes 0.
    let ev_page = call(
        &app,
        Method::GET,
        "/api/notifications?unread_only=true&limit=10",
        Some(&member),
        None,
    )
    .await;
    let ev_id = ev_page.json["items"][0]["id"].as_i64().unwrap();
    let del_res = call(
        &app,
        Method::DELETE,
        &format!("/api/notifications/{ev_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(del_res.status, StatusCode::OK);
    assert_eq!(del_res.json["unread"], 0);
}

#[tokio::test]
async fn single_device_logout_cleans_only_matching_push_endpoint() {
    let (app, st) = setup().await;
    let session_a = login(&st, "member@example.org").await;
    let session_b = login(&st, "member@example.org").await;
    let uid = user_id(&st, "member@example.org").await;

    let rng = SystemRandom::new();
    let p1 = EphemeralPrivateKey::generate(&ECDH_P256, &rng)
        .unwrap()
        .compute_public_key()
        .unwrap();
    let p2 = EphemeralPrivateKey::generate(&ECDH_P256, &rng)
        .unwrap()
        .compute_public_key()
        .unwrap();

    for (sess, ep, pubk) in [
        (&session_a, "https://push.example/dev-a", p1.as_ref()),
        (&session_b, "https://push.example/dev-b", p2.as_ref()),
    ] {
        let res = call(
            &app,
            Method::POST,
            "/api/push/subscribe",
            Some(sess),
            Some(json!({
                "endpoint": ep,
                "p256dh": URL_SAFE_NO_PAD.encode(pubk),
                "auth": URL_SAFE_NO_PAD.encode([7u8; 16]),
                "device_label": "Chrome",
            })),
        )
        .await;
        assert_eq!(res.status, StatusCode::OK, "{}", res.json);
    }

    let logout = call(
        &app,
        Method::POST,
        "/api/auth/logout",
        Some(&session_a),
        Some(json!({ "push_endpoint": "https://push.example/dev-a" })),
    )
    .await;
    assert_eq!(logout.status, StatusCode::OK);

    let remaining: Vec<String> =
        sqlx::query_scalar("SELECT endpoint FROM push_subscriptions WHERE user_id=$1")
            .bind(uid)
            .fetch_all(&st.db)
            .await
            .unwrap();
    assert_eq!(remaining, vec!["https://push.example/dev-b".to_string()]);
}

#[tokio::test]
async fn outbox_webhook_retry_and_web_push_410_pruning() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let uid = user_id(&st, "member@example.org").await;

    // Mock webhook returns 500 on first call, 200 on second call; push returns 410 Gone.
    let hook_calls = Arc::new(AtomicUsize::new(0));
    let hook_counter = hook_calls.clone();
    let gone_calls = Arc::new(AtomicUsize::new(0));
    let gone_counter = gone_calls.clone();

    let mock = Router::new()
        .route(
            "/wh",
            post(move || {
                let c = hook_counter.clone();
                async move {
                    let prev = c.fetch_add(1, Ordering::SeqCst);
                    if prev == 0 {
                        StatusCode::INTERNAL_SERVER_ERROR
                    } else {
                        StatusCode::OK
                    }
                }
            }),
        )
        .route(
            "/gone",
            post(move || {
                let c = gone_counter.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    StatusCode::GONE
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, mock).await;
    });

    // Generate valid VAPID keys + webhook config.
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    let kp =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/admin/settings/alerts",
            Some(&admin),
            Some(json!({
                "alert_webhook_url": format!("http://{addr}/wh"),
                "vapid_public_key": URL_SAFE_NO_PAD.encode(kp.public_key().as_ref()),
                "vapid_private_key": URL_SAFE_NO_PAD.encode(pkcs8.as_ref()),
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/notifications/preferences",
            Some(&member),
            Some(json!({ "whatsapp_opt_in": true, "push_opt_in": true })),
        )
        .await
        .status,
        StatusCode::OK
    );
    let sub_pub = EphemeralPrivateKey::generate(&ECDH_P256, &rng)
        .unwrap()
        .compute_public_key()
        .unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/push/subscribe",
            Some(&member),
            Some(json!({
                "endpoint": format!("http://{addr}/gone"),
                "p256dh": URL_SAFE_NO_PAD.encode(sub_pub.as_ref()),
                "auth": URL_SAFE_NO_PAD.encode([9u8; 16]),
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    grace_ride::util::notify(
        &st.db,
        uid,
        ("测试", "Test alert"),
        ("内容", "Outbox test"),
        "/rides/1",
    )
    .await;

    // Wait briefly for initial spawn task to attempt both channels once.
    for _ in 0..40 {
        if hook_calls.load(Ordering::SeqCst) >= 1 && gone_calls.load(Ordering::SeqCst) >= 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // 410 Gone subscription must be pruned and outbox marked 'gone'.
    for _ in 0..20 {
        let left: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions WHERE user_id=$1")
                .bind(uid)
                .fetch_one(&st.db)
                .await
                .unwrap();
        if left == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let left_subs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions WHERE user_id=$1")
            .bind(uid)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(left_subs, 0);

    // Wait until webhook row reaches status='retry' with attempts=1, then force next_attempt_at due now and process_due_deliveries.
    for _ in 0..20 {
        let st_str: Option<String> = sqlx::query_scalar(
            "SELECT status FROM notification_deliveries WHERE user_id=$1 AND channel='whatsapp_webhook' ORDER BY id DESC LIMIT 1",
        )
        .bind(uid)
        .fetch_optional(&st.db)
        .await
        .unwrap();
        if st_str.as_deref() == Some("retry") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    sqlx::query(
        "UPDATE notification_deliveries SET next_attempt_at='2020-01-01T00:00:00'
         WHERE user_id=$1 AND channel='whatsapp_webhook'",
    )
    .bind(uid)
    .execute(&st.db)
    .await
    .unwrap();

    process_due_deliveries(&st.db, 20).await.unwrap();

    let (status, attempts): (String, i64) = sqlx::query_as(
        "SELECT status, attempts FROM notification_deliveries
         WHERE user_id=$1 AND channel='whatsapp_webhook' ORDER BY id DESC LIMIT 1",
    )
    .bind(uid)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(status, "delivered");
    assert_eq!(attempts, 2);
}

#[tokio::test]
async fn preferences_quiet_hours_hide_lock_details_and_telegram_pairing() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let uid = user_id(&st, "member@example.org").await;

    // Manual Telegram Chat ID entry must be rejected (requires verified /start webhook pairing).
    let bad_tg = call(
        &app,
        Method::PUT,
        "/api/notifications/preferences",
        Some(&member),
        Some(json!({ "telegram_chat_id": "123456789" })),
    )
    .await;
    assert_eq!(bad_tg.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_tg.json["code"], "TELEGRAM_PAIRING_REQUIRED");

    // Configure Telegram bot credentials + webhook secret path.
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/admin/settings/alerts",
            Some(&admin),
            Some(json!({
                "telegram_bot_username": "GraceTestBot",
                "telegram_bot_token": "123456:ABCDEF_secret_token_value",
                "alert_webhook_url": "https://example.org/hook",
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    // Enable 24h quiet hours (00:00 -> 23:59), mute 'cost', enable hide_lock_details, opt in webhook.
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/notifications/preferences",
            Some(&member),
            Some(json!({
                "whatsapp_opt_in": true,
                "muted_categories": ["cost"],
                "quiet_hours_start": "00:00",
                "quiet_hours_end": "23:59",
                "hide_lock_details": true,
                "driver_remind_mins": 90,
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    // Muted + normal priority during quiet hours: in-app row saved, external delivery skipped.
    notify_with(
        &st.db,
        uid,
        ("费用通知", "Cost update"),
        ("请支付 RM10", "Pay RM 10 to John"),
        "/costs",
        NotifMeta {
            category: "cost",
            event_type: "share_due".into(),
            priority: "normal",
            entity_type: "settlement",
            entity_id: Some(1),
            tag: "cost-1".into(),
        },
    )
    .await;
    let cost_ext: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notification_deliveries WHERE user_id=$1 AND category='cost'",
    )
    .bind(uid)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(cost_ext, 0);

    // Urgent readiness reminder MUST bypass quiet hours AND honor hide_lock_details!
    grace_ride::util::notify(
        &st.db,
        uid,
        ("接送提醒", "Ride reminder"),
        (
            "司机 Alice 将接 123 Jalan Secret",
            "Driver Alice pickup at 123 Jalan Secret",
        ),
        "/rides/99?action=readiness",
    )
    .await;
    let (ext_title, ext_body): (String, String) = sqlx::query_as(
        "SELECT title, body FROM notification_deliveries
         WHERE user_id=$1 AND channel='whatsapp_webhook' ORDER BY id DESC LIMIT 1",
    )
    .bind(uid)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(!ext_body.contains("Secret") && !ext_body.contains("Alice"));
    assert!(!ext_title.is_empty());

    // Simulate telegram_blocked=1, then pair via 1-tap /start webhook -> resets blocked=0 and binds chat id.
    sqlx::query("UPDATE user_notification_prefs SET telegram_blocked=1 WHERE user_id=$1")
        .bind(uid)
        .execute(&st.db)
        .await
        .unwrap();

    let link_res = call(
        &app,
        Method::POST,
        "/api/push/telegram/link-token",
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(link_res.status, StatusCode::OK, "{}", link_res.json);
    let url_str = link_res.json["url"].as_str().unwrap();
    let tok = url_str.split("?start=").nth(1).expect("start query param");

    let admin_cfg = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&admin),
        None,
    )
    .await;
    let wh_path = admin_cfg.json["telegram_webhook_path"].as_str().unwrap();

    // Even from unauthenticated Telegram webhook without Origin header (exempt by security layer).
    let wh_res = call_exact(
        &app,
        Method::POST,
        wh_path,
        None,
        Some(json!({
            "message": {
                "chat": { "id": 987654321i64 },
                "text": format!("/start {tok}")
            }
        })),
        None,
    )
    .await;
    assert_eq!(wh_res.status, StatusCode::OK, "{}", wh_res.json);

    let prefs = call(
        &app,
        Method::GET,
        "/api/notifications/preferences",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(prefs.json["telegram_chat_id"], "987654321");
    assert_eq!(prefs.json["telegram_blocked"], false);
    assert_eq!(prefs.json["hide_lock_details"], true);
}

#[tokio::test]
async fn per_user_sse_hub_scales_past_300_users_without_dropping() {
    let hub = EventHub::new();
    let mut rxs = Vec::with_capacity(350);
    for uid in 1..=350i64 {
        rxs.push((uid, hub.subscribe(uid)));
    }
    for uid in 1..=350i64 {
        hub.send(Push {
            user_id: uid,
            kind: "notify",
        })
        .unwrap();
    }
    for (uid, mut rx) in rxs {
        let ev = rx
            .try_recv()
            .unwrap_or_else(|e| panic!("user {uid} missed SSE push: {e}"));
        assert_eq!(ev.kind, "notify");
    }
}

#[tokio::test]
async fn mobile_push_token_lifecycle() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;

    let bad = call(
        &app,
        Method::POST,
        "/api/push/mobile-token",
        Some(&member),
        Some(json!({
            "platform": "android",
            "provider": "unknown_provider",
            "token": "https://ntfy.sh/grace-ride-token-123456",
        })),
    )
    .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);

    let reg = call(
        &app,
        Method::POST,
        "/api/push/mobile-token",
        Some(&member),
        Some(json!({
            "platform": "android",
            "provider": "fcm",
            "token": "fcm-device-token-1234567890",
            "device_label": "Pixel 9",
        })),
    )
    .await;
    assert_eq!(reg.status, StatusCode::OK, "{}", reg.json);

    let subs = call(
        &app,
        Method::GET,
        "/api/push/subscriptions",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(subs.status, StatusCode::OK);
    let mobs = subs.json["mobile"].as_array().unwrap();
    assert_eq!(mobs.len(), 1);
    assert_eq!(mobs[0]["device_label"], "Pixel 9");

    let unreg = call(
        &app,
        Method::DELETE,
        "/api/push/mobile-token",
        Some(&member),
        Some(json!({ "token": "fcm-device-token-1234567890" })),
    )
    .await;
    assert_eq!(unreg.status, StatusCode::OK);

    let subs2 = call(
        &app,
        Method::GET,
        "/api/push/subscriptions",
        Some(&member),
        None,
    )
    .await;
    assert!(subs2.json["mobile"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn throttled_offline_trip_chat_alerts() {
    let (app, st) = setup().await;
    let p1 = login(&st, "member@example.org").await;
    let drv = login(&st, "driver@example.org").await;
    let drv_id = user_id(&st, "driver@example.org").await;
    let p1_id = user_id(&st, "member@example.org").await;
    let area = first_area_id(&st).await;

    let r = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&p1),
        Some(json!({
            "pickup_area_id": area,
            "pickup_address": "8 Jalan Chat",
            "destination": "Church Main",
            "pickup_at": future_pickup_iso(2, "09:30"),
            "direction": "to_church",
        })),
    )
    .await;
    let ride_id = r.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{ride_id}/claim"),
            Some(&drv),
            Some(json!({})),
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{ride_id}/respond"),
            Some(&p1),
            Some(json!({ "accept": true })),
        )
        .await
        .status,
        StatusCode::OK
    );
    let admin = login(&st, "admin@example.org").await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{ride_id}/approve"),
            Some(&admin),
            None,
        )
        .await
        .status,
        StatusCode::OK
    );
    let trip_id: i64 = sqlx::query_scalar("SELECT trip_id FROM rides WHERE id=$1")
        .bind(ride_id)
        .fetch_one(&st.db)
        .await
        .unwrap();

    // Give passenger an opted-in webhook channel so enqueue_external_deliveries records outbox rows.
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/notifications/preferences",
            None,
            Some(json!({})),
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/admin/settings/alerts",
            Some(&admin),
            Some(json!({ "alert_webhook_url": "https://example.org/hook" })),
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/notifications/preferences",
            Some(&p1),
            Some(json!({ "whatsapp_opt_in": true })),
        )
        .await
        .status,
        StatusCode::OK
    );

    // First message from offline driver queues 1 external alert to passenger.
    notify_trip_chat(
        &st.db,
        trip_id,
        ride_id,
        drv_id,
        "Driver Dan",
        "Leaving now",
    )
    .await;
    // Rapid follow-up within cooldown is throttled (still 1 outbox row for this trip tag).
    notify_trip_chat(
        &st.db,
        trip_id,
        ride_id,
        drv_id,
        "Driver Dan",
        "At the junction",
    )
    .await;

    let cnt: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notification_deliveries WHERE user_id=$1 AND tag=$2",
    )
    .bind(p1_id)
    .bind(format!("trip-chat-{trip_id}"))
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(cnt, 1);
}
