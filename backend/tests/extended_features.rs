//! Integration tests covering all 14 extended features:
//! 1. Round-trip / return-leg pairing
//! 2. Saved places & extended proxy rider defaults
//! 3. Accessibility-aware auto-arrange & board data
//! 4. Standing / recurring ride & driver schedules + job generation
//! 5. Alert channels (settings, Web Push config/subscribe, Telegram/WhatsApp webhook)
//! 6. Caregiver safe-arrival scoped links & nominated caregiver pickup/arrival alerts
//! 7. Offline snapshot support is tested in frontend; backend exposes paired leg/vehicle metadata
//! 8. Recurring event series & event duplication
//! 9. Member personal QR check-in pass & organiser pass scanner
//! 10. Live calendar subscription (.ics) & rotation/revocation
//! 11. Settlement payee DuitNow QR image + receipt ref extraction
//! 12. Monthly finance payout batches (CSV export, 2-person separation of duties, batch pay)
//! 13. Unmet-demand & driver shortage analytics
//! 14. Server-enforced session lock/unlock & rich guide step metadata

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    routing::post,
    Json, Router,
};
use common::*;
use grace_ride::state::AppState;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

fn future_dt(days: i64, hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (grace_ride::util::today_local() + chrono::Duration::days(days)).format("%Y-%m-%d")
    )
}

async fn first_area_id(st: &AppState) -> i64 {
    sqlx::query_scalar("SELECT id FROM areas WHERE active=1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn all_group_id(st: &AppState) -> i64 {
    sqlx::query_scalar("SELECT id FROM groups WHERE is_all=1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn join_as_group(app: &Router, cookie: &str, ev_id: i64) -> i64 {
    let r = call(
        app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(cookie),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    r.json["join_as"][0]["id"].as_i64().expect("join_as choice")
}

async fn user_id(st: &AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn upload_png_with_text(app: &Router, cookie: &str, text: &[u8]) -> i64 {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&[0u8; 16]);
    png.extend_from_slice(text);
    let boundary = "XgrEXTENDED";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"receipt.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(&png);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let mct = format!("multipart/form-data; boundary={boundary}");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, &mct)
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let j: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "{j}");
    j["id"].as_i64().unwrap()
}

#[tokio::test]
async fn round_trip_saved_places_proxy_defaults_and_care_links() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let area = first_area_id(&st).await;
    let all_group = all_group_id(&st).await;

    // 1. Saved Places CRUD
    let sp = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({
            "label": "Parents House",
            "area_id": area,
            "address": "88 Jalan Damai, PJ",
            "gate_notes": "Ring bell #4",
            "is_default": true
        })),
    )
    .await;
    assert_eq!(sp.status, StatusCode::OK, "{}", sp.json);
    let sp_id = sp.json["id"].as_i64().unwrap();

    let list_sp = call(&app, Method::GET, "/api/saved-places", Some(&member), None).await;
    assert_eq!(list_sp.status, StatusCode::OK);
    assert_eq!(list_sp.json.as_array().unwrap().len(), 1);
    assert_eq!(list_sp.json[0]["gate_notes"], "Ring bell #4");

    let upd_sp = call(
        &app,
        Method::PUT,
        &format!("/api/saved-places/{sp_id}"),
        Some(&member),
        Some(json!({
            "label": "Mom Home",
            "area_id": area,
            "address": "88 Jalan Damai, PJ",
            "gate_notes": "Side gate open",
            "is_default": true
        })),
    )
    .await;
    assert_eq!(upd_sp.status, StatusCode::OK, "{}", upd_sp.json);

    // 2. Proxy rider extended defaults + PUT
    let pr = call(
        &app,
        Method::POST,
        "/api/proxy-riders",
        Some(&member),
        Some(json!({
            "name": "Uncle Tan",
            "phone": "012-1112222",
            "relationship": "parent",
            "notes": "Front Porch",
            "area_id": area,
            "pickup_address": "12 Jalan Merdu",
            "gate_notes": "Press buzzer 2"
        })),
    )
    .await;
    assert_eq!(pr.status, StatusCode::OK, "{}", pr.json);
    let pr_id = pr.json["id"].as_i64().unwrap();
    let pr_upd = call(
        &app,
        Method::PUT,
        &format!("/api/proxy-riders/{pr_id}"),
        Some(&member),
        Some(json!({
            "name": "Uncle Tan",
            "phone": "012-1112222",
            "relationship": "parent",
            "notes": "Walker",
            "area_id": area,
            "pickup_address": "14 Jalan Merdu",
            "gate_notes": "Use side gate"
        })),
    )
    .await;
    assert_eq!(pr_upd.status, StatusCode::OK, "{}", pr_upd.json);
    assert_eq!(pr_upd.json["pickup_address"], "14 Jalan Merdu");

    // 3. Round-trip event ride + per-leg duplicate protection
    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Sunday Service",
            "kind": "other",
            "starts_at": future_dt(2, "09:30"),
            "location": "Main Sanctuary",
            "capacity": 20,
            "group_ids": [all_group]
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev_id = ev.json["id"].as_i64().unwrap();

    let join_group = join_as_group(&app, &member, ev_id).await;
    let su = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/signup"),
        Some(&member),
        Some(json!({ "as_group_id": join_group })),
    )
    .await;
    assert_eq!(su.status, StatusCode::OK, "{}", su.json);

    let rt = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(2, "08:30"),
            "pickup_area_id": area,
            "pickup_address": "88 Jalan Damai",
            "destination": "Main Sanctuary",
            "seats": 1,
            "return_pickup_at": future_dt(2, "12:00"),
            "return_pickup_area_id": area,
            "return_pickup_address": "Main Sanctuary",
            "return_destination": "88 Jalan Damai"
        })),
    )
    .await;
    assert_eq!(rt.status, StatusCode::OK, "{}", rt.json);
    let out_id = rt.json["id"].as_i64().unwrap();
    let ret_id = rt.json["return_ride_id"].as_i64().unwrap();

    let out_get = call(
        &app,
        Method::GET,
        &format!("/api/rides/{out_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(out_get.json["leg"], "outbound");
    assert_eq!(out_get.json["paired_ride_id"], ret_id);
    let ret_get = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ret_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(ret_get.json["leg"], "return");
    assert_eq!(ret_get.json["paired_ride_id"], out_id);

    // Duplicate outbound ride on same event is rejected
    let dup = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(2, "08:45"),
            "pickup_area_id": area,
            "pickup_address": "88 Jalan Damai",
            "destination": "Main Sanctuary",
            "seats": 1,
            "leg": "outbound"
        })),
    )
    .await;
    assert_eq!(dup.status, StatusCode::BAD_REQUEST, "{}", dup.json);

    // 4. Nominate caregiver + care-link scoped view (hides pickup street address) & pickup alert
    let sophie_id = user_id(&st, "sophie@example.org").await;
    let set_cg = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({ "caregiver_user_id": sophie_id })),
    )
    .await;
    assert_eq!(set_cg.status, StatusCode::OK, "{}", set_cg.json);

    let cl = call(
        &app,
        Method::POST,
        &format!("/api/rides/{out_id}/care-link"),
        Some(&member),
        Some(json!({ "label": "Family Care" })),
    )
    .await;
    assert_eq!(cl.status, StatusCode::OK, "{}", cl.json);
    let token = cl.json["token"].as_str().unwrap().to_string();

    // Unauthenticated care link omits pickup_address for privacy
    let care_pub = call(&app, Method::GET, &format!("/api/care/{token}"), None, None).await;
    assert_eq!(care_pub.status, StatusCode::OK, "{}", care_pub.json);
    assert!(care_pub.json.get("pickup_address").is_none());
    assert_eq!(care_pub.json["label"], "Family Care");

    // Advance ride to confirmed -> in_progress -> pickup -> notifies caregiver (Sophie)
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/claim"),
            Some(&driver),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/respond"),
            Some(&member),
            Some(json!({ "accept": true }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/approve"),
            Some(&transport),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Scoped readiness via care link
    let rd = call(
        &app,
        Method::POST,
        &format!("/api/care/{token}/readiness"),
        None,
        Some(json!({ "status": "ready" })),
    )
    .await;
    assert_eq!(rd.status, StatusCode::OK, "{}", rd.json);

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/start"),
            Some(&driver),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/pickup"),
            Some(&driver),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );
    let sophie_notifs = call(&app, Method::GET, "/api/notifications", Some(&sophie), None).await;
    let body_text = sophie_notifs.json.to_string();
    assert!(
        body_text.contains(&format!("/rides/{out_id}")),
        "{body_text}"
    );

    // Revoking care link closes access
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/rides/{out_id}/care-link"),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let care_after = call(&app, Method::GET, &format!("/api/care/{token}"), None, None).await;
    assert_ne!(care_after.status, StatusCode::OK);

    // Cleanup saved place
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            &format!("/api/saved-places/{sp_id}"),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn accessibility_arrange_and_recurring_schedules() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let driver1 = login(&st, "driver@example.org").await;
    let driver2 = login(&st, "driver2@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let area = first_area_id(&st).await;
    let all_group = all_group_id(&st).await;
    let d1_id = user_id(&st, "driver@example.org").await;
    let d2_id = user_id(&st, "driver2@example.org").await;

    // Driver 1 gets wheelchair_ok = true; Driver 2 gets wheelchair_ok = false
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/driver/profile",
            Some(&driver1),
            Some(json!({
                "carpool_enabled": true,
                "spare_seats": 4,
                "area_ids": [area],
                "vehicle": {
                    "model": "Toyota Innova",
                    "color": "Silver",
                    "plate": "WCC 111",
                    "seats": 6,
                    "vehicle_type": "standard",
                    "wheelchair_ok": true,
                    "low_step": true,
                    "child_seat": false
                }
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
            "/api/driver/profile",
            Some(&driver2),
            Some(json!({
                "carpool_enabled": true,
                "spare_seats": 4,
                "area_ids": [area],
                "vehicle": {
                    "model": "Perodua Axia",
                    "color": "Blue",
                    "plate": "WXX 222",
                    "seats": 4,
                    "vehicle_type": "standard",
                    "wheelchair_ok": false,
                    "low_step": false,
                    "child_seat": false
                }
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Accessibility Plan Event",
            "kind": "other",
            "starts_at": future_dt(3, "10:00"),
            "location": "Church Hall",
            "capacity": 30,
            "group_ids": [all_group]
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev_id = ev.json["id"].as_i64().unwrap();
    let join_group = join_as_group(&app, &member, ev_id).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{ev_id}/signup"),
            Some(&member),
            Some(json!({ "as_group_id": join_group }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(3, "09:15"),
            "pickup_area_id": area,
            "pickup_address": "10 Jalan Kemajuan",
            "destination": "Church Hall",
            "seats": 1,
            "assistance": "wheelchair"
        })),
    )
    .await;
    assert_eq!(ride.status, StatusCode::OK, "{}", ride.json);
    let ride_id = ride.json["id"].as_i64().unwrap();

    // Auto-arrange must choose d1 (wheelchair_ok=true) over d2
    let arr = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{ev_id}/plan/arrange"),
        Some(&transport),
        Some(json!({ "draft": [] })),
    )
    .await;
    assert_eq!(arr.status, StatusCode::OK, "{}", arr.json);
    let placed = arr.json["placed"].as_array().unwrap();
    assert!(
        placed
            .iter()
            .any(|p| p["ride_id"] == ride_id && p["driver_id"] == d1_id),
        "expected wheelchair ride placed on d1 ({d1_id}), not d2 ({d2_id}): {}",
        arr.json
    );

    // Standing passenger recurring schedule + generate
    let sched = call(
        &app,
        Method::POST,
        "/api/recurring-schedules",
        Some(&member),
        Some(json!({
            "kind": "passenger",
            "label": "Sunday Worship",
            "weekday": 0,
            "pickup_time": "08:30",
            "area_id": area,
            "pickup_address": "10 Jalan Kemajuan",
            "destination": "Church Hall",
            "seats": 1,
            "lead_days": 7
        })),
    )
    .await;
    assert_eq!(sched.status, StatusCode::OK, "{}", sched.json);
    let sched_id = sched.json["id"].as_i64().unwrap();

    let gen1 = call(
        &app,
        Method::POST,
        &format!("/api/recurring-schedules/{sched_id}/generate"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(gen1.status, StatusCode::OK, "{}", gen1.json);
    assert!(gen1.json["ride_id"].as_i64().is_some(), "{}", gen1.json);
    assert_eq!(gen1.json["already_generated"], false, "{}", gen1.json);

    // Second generate in same week is idempotent (creates nothing)
    let gen2 = call(
        &app,
        Method::POST,
        &format!("/api/recurring-schedules/{sched_id}/generate"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(gen2.status, StatusCode::OK);
    assert!(gen2.json["ride_id"].is_null(), "{}", gen2.json);
    assert_eq!(gen2.json["already_generated"], true, "{}", gen2.json);
}

#[tokio::test]
async fn event_series_duplication_and_member_qr_pass() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let all_group = all_group_id(&st).await;

    // Weekly repeat series of 3 events
    let series = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Friday Youth",
            "kind": "other",
            "starts_at": future_dt(4, "19:30"),
            "location": "Youth Room",
            "capacity": 40,
            "group_ids": [all_group],
            "attendance_enabled": true,
            "repeat_frequency": "weekly",
            "repeat_count": 3
        })),
    )
    .await;
    assert_eq!(series.status, StatusCode::OK, "{}", series.json);
    let created_ids = series.json["created_ids"].as_array().unwrap();
    assert_eq!(created_ids.len(), 3);
    assert!(series.json["series_id"].as_str().is_some());
    let first_id = series.json["id"].as_i64().unwrap();

    // Duplicate event
    let dup = call(
        &app,
        Method::POST,
        &format!("/api/events/{first_id}/duplicate"),
        Some(&admin),
        Some(json!({
            "starts_at": future_dt(14, "20:00"),
            "title": "Friday Youth Special"
        })),
    )
    .await;
    assert_eq!(dup.status, StatusCode::OK, "{}", dup.json);
    let dup_id = dup.json["id"].as_i64().unwrap();
    assert_ne!(dup_id, first_id);

    // Member signs up -> gets personal 10-char pass_code -> organiser scans it
    let join_group = join_as_group(&app, &member, dup_id).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{dup_id}/signup"),
            Some(&member),
            Some(json!({ "as_group_id": join_group }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let ev_detail = call(
        &app,
        Method::GET,
        &format!("/api/events/{dup_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(ev_detail.status, StatusCode::OK);
    let pass_code = ev_detail.json["my_signup"]["pass_code"]
        .as_str()
        .expect("signup includes pass_code")
        .to_string();
    assert_eq!(pass_code.len(), 10);

    let scan = call(
        &app,
        Method::POST,
        &format!("/api/events/{dup_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": pass_code })),
    )
    .await;
    assert_eq!(scan.status, StatusCode::OK, "{}", scan.json);
    assert_eq!(scan.json["ok"], true);
    assert_eq!(scan.json["already_attended"], false);
    assert!(
        scan.json["user"]["name"].as_str().is_some(),
        "{}",
        scan.json
    );

    // Invalid pass code rejects cleanly
    let bad_scan = call(
        &app,
        Method::POST,
        &format!("/api/events/{dup_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": "INVALID000" })),
    )
    .await;
    assert_eq!(
        bad_scan.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_scan.json
    );
}

#[tokio::test]
async fn live_calendar_feed_and_alert_channels() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    // Calendar feed initial state -> rotate -> fetch .ics -> revoke
    let init_cal = call(
        &app,
        Method::GET,
        "/api/me/calendar-feed",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(init_cal.status, StatusCode::OK);
    assert_eq!(init_cal.json["enabled"], false);

    let rot = call(
        &app,
        Method::POST,
        "/api/me/calendar-feed/rotate",
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(rot.status, StatusCode::OK, "{}", rot.json);
    let ics_path = rot.json["ics_path"].as_str().unwrap().to_string();

    let (status, headers, ics_body) = call_raw(&app, Method::GET, &ics_path, None).await;
    assert_eq!(status, StatusCode::OK, "{ics_body}");
    assert!(headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .contains("text/calendar"));
    assert!(ics_body.contains("BEGIN:VCALENDAR"));

    assert_eq!(
        call(
            &app,
            Method::DELETE,
            "/api/me/calendar-feed",
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let (revoked_status, _, _) = call_raw(&app, Method::GET, &ics_path, None).await;
    assert_eq!(revoked_status, StatusCode::NOT_FOUND);

    // Ephemeral local webhook receiver for external alerts
    let received: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let rx_store = received.clone();
    let hook_app = Router::new().route(
        "/hook",
        post(move |Json(payload): Json<Value>| {
            let store = rx_store.clone();
            async move {
                store.lock().unwrap().push(payload);
                StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, hook_app).await;
    });

    // Admin updates alert channels
    let put_alerts = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({
            "telegram_bot_username": "@GraceTestBot",
            "telegram_bot_token": "",
            "alert_webhook_url": format!("http://{addr}/hook"),
            "vapid_public_key": "BTestVapidKey"
        })),
    )
    .await;
    assert_eq!(put_alerts.status, StatusCode::OK, "{}", put_alerts.json);

    let push_cfg = call(&app, Method::GET, "/api/push/config", Some(&member), None).await;
    assert_eq!(push_cfg.status, StatusCode::OK);
    assert_eq!(push_cfg.json["vapid_public_key"], "BTestVapidKey");

    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/push/subscribe",
            Some(&member),
            Some(json!({ "endpoint": "https://push.example.org/sub/123", "p256dh": "k", "auth": "a" })),
        )
        .await
        .status,
        StatusCode::OK
    );

    // Member opts into WhatsApp alerts & triggers notification
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&member),
            Some(json!({ "whatsapp_opt_in": true, "push_opt_in": true })),
        )
        .await
        .status,
        StatusCode::OK
    );
    grace_ride::util::notify(
        &st.db,
        member_id,
        ("测试通知", "Test alert"),
        ("内容", "Alert body"),
        "/rides",
    )
    .await;
    for _ in 0..20 {
        if !received.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let hooks = received.lock().unwrap();
    assert!(!hooks.is_empty(), "expected webhook alert dispatch");
    assert_eq!(hooks[0]["user_id"], member_id);
}

#[tokio::test]
async fn settlement_qr_receipt_extraction_payout_batches_and_demand() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let area = first_area_id(&st).await;
    let all_group = all_group_id(&st).await;

    // 1. Receipt transaction ref extraction + Settlement Payee DuitNow QR file access
    let receipt_id =
        upload_png_with_text(&app, &member, b"DuitNow Ref: DN20261118007 RM15.00").await;
    let ext = call(
        &app,
        Method::POST,
        &format!("/api/files/{receipt_id}/extract-ref"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(ext.status, StatusCode::OK, "{}", ext.json);
    assert_eq!(ext.json["ref"], "DN20261118007");

    let qr_id = upload_png_with_text(&app, &admin, b"DUITNOW-QR-PNG").await;
    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Badminton Night",
            "kind": "badminton",
            "starts_at": future_dt(1, "20:00"),
            "location": "Sports Complex",
            "capacity": 16,
            "group_ids": [all_group],
            "attendance_enabled": false
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev_id = ev.json["id"].as_i64().unwrap();
    let bill = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/settlements"),
        Some(&admin),
        Some(json!({
            "title": "Court Rental",
            "items": [{ "label": "Courts", "amount_cents": 2000 }],
            "subsidy_cents": 0,
            "split_mode": "equal",
            "participants": [{ "user_id": member_id }],
            "payee_name": "Transport Lead",
            "payee_account": "TNG 012-3456789",
            "payee_qr_file_id": qr_id
        })),
    )
    .await;
    assert_eq!(bill.status, StatusCode::OK, "{}", bill.json);
    let sid = bill.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/settlements/{sid}/publish"),
            Some(&admin),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Participant can view the payee QR; non-participant sophie gets 403
    let (p_status, _, _) = call_raw(
        &app,
        Method::GET,
        &format!("/api/files/{qr_id}"),
        Some(&member),
    )
    .await;
    assert_eq!(p_status, StatusCode::OK);
    let (s_status, _, _) = call_raw(
        &app,
        Method::GET,
        &format!("/api/files/{qr_id}"),
        Some(&sophie),
    )
    .await;
    assert_eq!(s_status, StatusCode::FORBIDDEN);

    // 2. Monthly Payout Batches (2-person separation of duties)
    let claim_file = upload_png_with_text(&app, &driver, b"GRAB-RECEIPT").await;
    let claim = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&driver),
        Some(json!({
            "amount_cents": 2500,
            "description": "Grab for elder",
            "receipt_file_id": claim_file,
            "payee_name": "Driver One",
            "payee_phone": "019-9998888"
        })),
    )
    .await;
    assert_eq!(claim.status, StatusCode::OK, "{}", claim.json);
    let claim_id = claim.json["id"].as_i64().unwrap();

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/reimbursements/{claim_id}/review"),
            Some(&transport),
            Some(json!({ "action": "approve", "note": "" })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let month = grace_ride::util::now()[..7].to_string();
    let batch = call(
        &app,
        Method::POST,
        "/api/reimbursements/batches",
        Some(&finance),
        Some(json!({ "month": month, "title": "November Grab Batch" })),
    )
    .await;
    assert_eq!(batch.status, StatusCode::OK, "{}", batch.json);
    let batch_id = batch.json["id"].as_i64().unwrap();
    assert_eq!(batch.json["total_cents"], 2500);
    assert_eq!(batch.json["claim_count"], 1);

    let (csv_status, csv_headers, csv_body) = call_raw(
        &app,
        Method::GET,
        &format!("/api/reimbursements/batches/{batch_id}/csv"),
        Some(&finance),
    )
    .await;
    assert_eq!(csv_status, StatusCode::OK);
    assert!(csv_headers
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/csv"));
    assert!(csv_body.contains("Driver One") && csv_body.contains("25.00"));

    // Reviewer (`transport`) cannot pay the batch they approved
    let rev_pay = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&transport),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-FORBID" })),
    )
    .await;
    assert_eq!(rev_pay.status, StatusCode::FORBIDDEN);

    let fin_pay = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-BATCH-01" })),
    )
    .await;
    assert_eq!(fin_pay.status, StatusCode::OK, "{}", fin_pay.json);

    let pay_twice = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-BATCH-02" })),
    )
    .await;
    assert_eq!(pay_twice.status, StatusCode::BAD_REQUEST);
    assert_eq!(pay_twice.json["code"], "BATCH_ALREADY_PAID");

    // 3. Demand analytics endpoint
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/rides",
            Some(&member),
            Some(json!({
                "pickup_at": future_dt(1, "09:00"),
                "pickup_area_id": area,
                "pickup_address": "5 Jalan Dua",
                "destination": "Church",
                "seats": 2
            })),
        )
        .await
        .status,
        StatusCode::OK
    );
    let demand_month = future_dt(1, "09:00")[..7].to_string();
    let demand = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/demand?month={demand_month}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(demand.status, StatusCode::OK, "{}", demand.json);
    assert!(demand.json["totals"]["unfilled"].as_i64().unwrap() >= 1);
    assert!(demand.json["by_area"].as_array().is_some());
    assert!(demand.json["by_slot"].as_array().is_some());
}

#[tokio::test]
async fn server_session_lock_and_rich_help_guides() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let transport = login(&st, "transport@example.org").await;

    // Lock session -> protected routes return 423 SESSION_LOCKED -> /auth/me remains 200 (locked: true)
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/lock",
            Some(&member),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );
    let blocked = call(&app, Method::GET, "/api/rides", Some(&member), None).await;
    assert_eq!(blocked.status, StatusCode::LOCKED, "{}", blocked.json);
    assert_eq!(blocked.json["code"], "SESSION_LOCKED");

    let me_locked = call(&app, Method::GET, "/api/auth/me", Some(&member), None).await;
    assert_eq!(me_locked.status, StatusCode::OK);
    assert_eq!(me_locked.json["locked"], true);

    // Ensure test user unlocks cleanly via dev_unlock (clear passkey/totp if any seeded) or test both
    sqlx::query("UPDATE users SET totp_enabled=0, totp_secret=NULL WHERE id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM passkeys WHERE user_id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();

    let unlocked = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&member),
        Some(json!({ "dev_unlock": true })),
    )
    .await;
    assert_eq!(unlocked.status, StatusCode::OK, "{}", unlocked.json);

    let after = call(&app, Method::GET, "/api/rides", Some(&member), None).await;
    assert_eq!(after.status, StatusCode::OK, "{}", after.json);

    // Help guide step stable IDs and callout note fields
    let g = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&transport),
        Some(json!({
            "slug": "extended-rich-guide",
            "title_en": "Rich Guide",
            "title_zh": "丰富指南",
            "summary_en": "Summary",
            "summary_zh": "摘要",
            "published": true,
            "audience": "member",
            "steps": [{
                "id": "custom_step_1",
                "title_en": "Step One",
                "title_zh": "第一步",
                "body_en": "- Bullet\n`code`",
                "body_zh": "说明",
                "link": "/rides",
                "note_kind": "tip",
                "note_en": "Helpful tip",
                "note_zh": "实用提示"
            }]
        })),
    )
    .await;
    assert_eq!(g.status, StatusCode::OK, "{}", g.json);
    assert!(g.json["id"].as_i64().is_some(), "{}", g.json);

    let fetched = call(
        &app,
        Method::GET,
        "/api/help/guides/extended-rich-guide",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(fetched.status, StatusCode::OK, "{}", fetched.json);
    assert_eq!(fetched.json["steps"][0]["id"], "custom_step_1");
    assert_eq!(fetched.json["steps"][0]["note_kind"], "tip");
    assert_eq!(fetched.json["steps"][0]["note_en"], "Helpful tip");
}
