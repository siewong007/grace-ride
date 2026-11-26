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

    // The care link opens 24 hours before pickup: bring the ride into that window.
    sqlx::query("UPDATE rides SET pickup_at=$1 WHERE id=$2")
        .bind(grace_ride::util::in_hours(2))
        .bind(out_id)
        .execute(&st.db)
        .await
        .unwrap();

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

    // Rides exist only for events, so a standing weekly passenger ride can't be set up.
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
    assert_eq!(sched.status, StatusCode::BAD_REQUEST, "{}", sched.json);
    assert_eq!(sched.json["code"], "EVENT_RIDE_NEEDS_SIGNUP");
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

    let push_cfg_before = call(&app, Method::GET, "/api/push/config", Some(&member), None).await;
    assert_eq!(push_cfg_before.status, StatusCode::OK);
    assert_eq!(push_cfg_before.json["whatsapp_configured"], false);
    assert_eq!(push_cfg_before.json["telegram_configured"], false);
    assert_eq!(push_cfg_before.json["telegram_bot_username"], "");

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
        })),
    )
    .await;
    assert_eq!(put_alerts.status, StatusCode::OK, "{}", put_alerts.json);

    let push_cfg = call(&app, Method::GET, "/api/push/config", Some(&member), None).await;
    assert_eq!(push_cfg.status, StatusCode::OK);
    // A bare public key without its private half is rejected, so the server-generated key stays.
    assert!(push_cfg.json["vapid_public_key"]
        .as_str()
        .is_some_and(|k| !k.is_empty()));
    assert_eq!(push_cfg.json["whatsapp_configured"], true);
    assert_eq!(push_cfg.json["telegram_configured"], false);
    assert_eq!(push_cfg.json["telegram_bot_username"], "GraceTestBot");

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

    // Reviewer (`transport`) and batch creator (`finance`) cannot pay the batch
    let rev_pay = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&transport),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-FORBID" })),
    )
    .await;
    assert_eq!(rev_pay.status, StatusCode::FORBIDDEN);

    let self_batch_pay = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-SELF-BATCH" })),
    )
    .await;
    assert_eq!(self_batch_pay.status, StatusCode::FORBIDDEN);
    assert_eq!(self_batch_pay.json["code"], "CANNOT_PAY_OWN_BATCH");

    let finance2 = login(&st, "superadmin@example.org").await;
    let fin_pay = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance2),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-BATCH-01" })),
    )
    .await;
    assert_eq!(fin_pay.status, StatusCode::OK, "{}", fin_pay.json);

    let pay_twice = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{batch_id}/pay"),
        Some(&finance2),
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

struct TestHkdfLen(usize);

impl ring::hkdf::KeyType for TestHkdfLen {
    fn len(&self) -> usize {
        self.0
    }
}

fn verify_vapid_and_decrypt(
    headers: &[(String, String)],
    body: &[u8],
    expected_pub_b64: &str,
    sub_priv: ring::agreement::EphemeralPrivateKey,
    sub_pub: &[u8],
    sub_auth: &[u8],
    expected_ttl: &str,
) -> Value {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use ring::{
        aead::{self, LessSafeKey, UnboundKey},
        agreement::{self, UnparsedPublicKey, ECDH_P256},
        hkdf::{Salt, HKDF_SHA256},
        signature::{UnparsedPublicKey as SigPublicKey, ECDSA_P256_SHA256_FIXED},
    };

    let hdr = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(hdr("content-encoding"), Some("aes128gcm"));
    assert_eq!(hdr("ttl"), Some(expected_ttl));

    let authz = hdr("authorization").expect("vapid Authorization header");
    let rest = authz.strip_prefix("vapid ").expect("vapid prefix");
    let (t_part, k_part) = rest.split_once(", ").expect("vapid t and k parts");
    assert_eq!(k_part.strip_prefix("k="), Some(expected_pub_b64));
    let jwt = t_part.strip_prefix("t=").expect("jwt token");
    let jwt_parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(jwt_parts.len(), 3);
    let signing_input = format!("{}.{}", jwt_parts[0], jwt_parts[1]);
    let sig = URL_SAFE_NO_PAD.decode(jwt_parts[2]).unwrap();
    let pub_bytes = URL_SAFE_NO_PAD.decode(expected_pub_b64).unwrap();
    SigPublicKey::new(&ECDSA_P256_SHA256_FIXED, &pub_bytes)
        .verify(signing_input.as_bytes(), &sig)
        .expect("VAPID ES256 signature must verify");

    assert!(body.len() > 86);
    let salt = &body[..16];
    assert_eq!(u32::from_be_bytes(body[16..20].try_into().unwrap()), 4096);
    let idlen = body[20] as usize;
    assert_eq!(idlen, 65);
    let as_pub = &body[21..21 + idlen];
    let mut record = body[21 + idlen..].to_vec();

    let ecdh =
        agreement::agree_ephemeral(sub_priv, &UnparsedPublicKey::new(&ECDH_P256, as_pub), |s| {
            s.to_vec()
        })
        .unwrap();
    let ikm_prk = Salt::new(HKDF_SHA256, sub_auth).extract(&ecdh);
    let mut ikm = [0u8; 32];
    ikm_prk
        .expand(&[b"WebPush: info\0", sub_pub, as_pub], TestHkdfLen(32))
        .unwrap()
        .fill(&mut ikm)
        .unwrap();
    let content_prk = Salt::new(HKDF_SHA256, salt).extract(&ikm);
    let mut cek = [0u8; 16];
    content_prk
        .expand(&[b"Content-Encoding: aes128gcm\0"], TestHkdfLen(16))
        .unwrap()
        .fill(&mut cek)
        .unwrap();
    let mut nonce = [0u8; 12];
    content_prk
        .expand(&[b"Content-Encoding: nonce\0"], TestHkdfLen(12))
        .unwrap()
        .fill(&mut nonce)
        .unwrap();

    let dec_key = LessSafeKey::new(UnboundKey::new(&aead::AES_128_GCM, &cek).unwrap());
    let plain_padded = dec_key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::empty(),
            &mut record,
        )
        .unwrap();
    assert_eq!(plain_padded.last().copied(), Some(0x02));
    serde_json::from_slice(&plain_padded[..plain_padded.len() - 1]).unwrap()
}

#[tokio::test]
async fn web_push_end_to_end_encryption_test_endpoint_and_410_pruning() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use ring::{
        agreement::{EphemeralPrivateKey, ECDH_P256},
        rand::{SecureRandom, SystemRandom},
    };

    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    let me = call(&app, Method::GET, "/api/auth/me", Some(&member), None).await;
    let is_en = me.json["lang"].as_str() == Some("en");

    // 1. Auto-generated VAPID keypair is active right out of setup()
    let cfg = call(&app, Method::GET, "/api/push/config", Some(&member), None).await;
    assert_eq!(cfg.status, StatusCode::OK, "{}", cfg.json);
    assert_eq!(cfg.json["configured"], true);
    let orig_pub = cfg.json["vapid_public_key"].as_str().unwrap().to_string();
    assert_eq!(orig_pub.len(), 87);

    let admin_alerts = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(admin_alerts.status, StatusCode::OK);
    assert_eq!(admin_alerts.json["vapid_private_configured"], true);

    // 2. Local Web Push receiver with /push/ok1, /push/ok2, and /push/gone (410)
    type CapturedPush = Vec<(Vec<(String, String)>, Vec<u8>)>;
    let ok1_reqs: Arc<Mutex<CapturedPush>> = Arc::new(Mutex::new(Vec::new()));
    let ok2_reqs: Arc<Mutex<CapturedPush>> = Arc::new(Mutex::new(Vec::new()));
    let gone_hits: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    let ok1_store = ok1_reqs.clone();
    let ok2_store = ok2_reqs.clone();
    let gone_store = gone_hits.clone();
    let push_app = Router::new()
        .route(
            "/push/ok1",
            post(
                move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                    let store = ok1_store.clone();
                    async move {
                        let hdrs = headers
                            .iter()
                            .map(|(k, v)| {
                                (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())
                            })
                            .collect();
                        store.lock().unwrap().push((hdrs, body.to_vec()));
                        StatusCode::CREATED
                    }
                },
            ),
        )
        .route(
            "/push/ok2",
            post(
                move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                    let store = ok2_store.clone();
                    async move {
                        let hdrs = headers
                            .iter()
                            .map(|(k, v)| {
                                (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())
                            })
                            .collect();
                        store.lock().unwrap().push((hdrs, body.to_vec()));
                        StatusCode::CREATED
                    }
                },
            ),
        )
        .route(
            "/push/gone",
            post(move || {
                let store = gone_store.clone();
                async move {
                    *store.lock().unwrap() += 1;
                    StatusCode::GONE
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, push_app).await;
    });

    // Generate subscriber P-256 ECDH keypairs
    let rng = SystemRandom::new();
    let gen_sub = || {
        let priv_k = EphemeralPrivateKey::generate(&ECDH_P256, &rng).unwrap();
        let pub_k = priv_k.compute_public_key().unwrap();
        let mut auth = [0u8; 16];
        rng.fill(&mut auth).unwrap();
        (priv_k, pub_k, auth)
    };
    let (sub1_priv, sub1_pub, sub1_auth) = gen_sub();
    let (sub2_priv, sub2_pub, sub2_auth) = gen_sub();
    let (_gone_priv, gone_pub, gone_auth) = gen_sub();

    for (path, p256dh, auth) in [
        (
            "/push/ok1",
            URL_SAFE_NO_PAD.encode(sub1_pub.as_ref()),
            URL_SAFE_NO_PAD.encode(sub1_auth),
        ),
        (
            "/push/gone",
            URL_SAFE_NO_PAD.encode(gone_pub.as_ref()),
            URL_SAFE_NO_PAD.encode(gone_auth),
        ),
    ] {
        assert_eq!(
            call(
                &app,
                Method::POST,
                "/api/push/subscribe",
                Some(&member),
                Some(json!({
                    "endpoint": format!("http://{addr}{path}"),
                    "p256dh": p256dh,
                    "auth": auth,
                })),
            )
            .await
            .status,
            StatusCode::OK
        );
    }

    // 3. Trigger notify() and verify RFC 8291 ciphertext, VAPID ES256 signature, and 410 pruning
    grace_ride::util::notify(
        &st.db,
        member_id,
        ("接送提醒", "Ride reminder"),
        ("司机已出发", "Driver departed"),
        "/rides/7?action=readiness",
    )
    .await;

    for _ in 0..40 {
        if !ok1_reqs.lock().unwrap().is_empty() && *gone_hits.lock().unwrap() > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(ok1_reqs.lock().unwrap().len(), 1);
    assert_eq!(*gone_hits.lock().unwrap(), 1);

    // Wait briefly for 410 deletion inside async dispatch task
    let mut sub_count = 2;
    for _ in 0..20 {
        let c = call(&app, Method::GET, "/api/push/config", Some(&member), None).await;
        sub_count = c.json["subscription_count"].as_i64().unwrap();
        if sub_count == 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert_eq!(sub_count, 1, "410 Gone endpoint should be pruned");

    let (hdrs1, body1) = ok1_reqs.lock().unwrap()[0].clone();
    let payload1 = verify_vapid_and_decrypt(
        &hdrs1,
        &body1,
        &orig_pub,
        sub1_priv,
        sub1_pub.as_ref(),
        &sub1_auth,
        "1800",
    );
    assert_eq!(
        payload1["title"],
        if is_en {
            "Ride reminder"
        } else {
            "接送提醒"
        }
    );
    assert_eq!(
        payload1["body"],
        if is_en {
            "Driver departed"
        } else {
            "司机已出发"
        }
    );
    assert_eq!(payload1["url"], "/rides/7?action=readiness");

    // 4. Subscribe second device and exercise POST /api/push/test
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/push/subscribe",
            Some(&member),
            Some(json!({
                "endpoint": format!("http://{addr}/push/ok2"),
                "p256dh": URL_SAFE_NO_PAD.encode(sub2_pub.as_ref()),
                "auth": URL_SAFE_NO_PAD.encode(sub2_auth),
            })),
        )
        .await
        .status,
        StatusCode::OK
    );

    let test_res = call(
        &app,
        Method::POST,
        "/api/push/test",
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(test_res.status, StatusCode::OK, "{}", test_res.json);
    assert_eq!(test_res.json["ok"], true);
    assert_eq!(test_res.json["delivered"], 2);
    assert_eq!(test_res.json["total"], 2);

    assert_eq!(ok2_reqs.lock().unwrap().len(), 1);
    let (hdrs2, body2) = ok2_reqs.lock().unwrap()[0].clone();
    let payload2 = verify_vapid_and_decrypt(
        &hdrs2,
        &body2,
        &orig_pub,
        sub2_priv,
        sub2_pub.as_ref(),
        &sub2_auth,
        "86400",
    );
    assert_eq!(payload2["url"], "/me");
    assert!(!payload2["title"].as_str().unwrap_or("").is_empty());

    // 5. Unsubscribe one endpoint and then rotate VAPID keys as admin
    assert_eq!(
        call(
            &app,
            Method::DELETE,
            "/api/push/subscribe",
            Some(&member),
            Some(json!({ "endpoint": format!("http://{addr}/push/ok1") })),
        )
        .await
        .status,
        StatusCode::OK
    );
    let cfg_mid = call(&app, Method::GET, "/api/push/config", Some(&member), None).await;
    assert_eq!(cfg_mid.json["subscription_count"], 1);

    let rot = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "rotate_vapid_keys": true })),
    )
    .await;
    assert_eq!(rot.status, StatusCode::OK, "{}", rot.json);

    let cfg_after = call(&app, Method::GET, "/api/push/config", Some(&member), None).await;
    assert_eq!(cfg_after.json["configured"], true);
    assert_eq!(cfg_after.json["subscription_count"], 0);
    let new_pub = cfg_after.json["vapid_public_key"].as_str().unwrap();
    assert_eq!(new_pub.len(), 87);
    assert_ne!(new_pub, orig_pub);

    let admin_after = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(admin_after.json["vapid_private_configured"], true);
    assert_eq!(admin_after.json["push_subscriptions_count"], 0);
}

#[tokio::test]
async fn driver_can_configure_multiple_vehicles_and_choose_active() {
    let (app, st) = setup().await;
    let driver = login(&st, "driver@example.org").await;

    // Driver fetches vehicles
    let res = call(
        &app,
        Method::GET,
        "/api/driver/vehicles",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::OK);
    let initial_vehicles = res.json.as_array().unwrap();
    assert!(!initial_vehicles.is_empty());

    // Add a second vehicle (7-seater MPV with wheelchair access)
    let add_res = call(
        &app,
        Method::POST,
        "/api/driver/vehicles",
        Some(&driver),
        Some(json!({
            "model": "Toyota Vellfire",
            "color": "Black",
            "plate": "VLL 7788",
            "seats": 7,
            "vehicle_type": "standard",
            "wheelchair_ok": true,
            "low_step": true,
            "child_seat": true,
            "is_active": false
        })),
    )
    .await;
    assert_eq!(add_res.status, StatusCode::OK);
    let new_vehicle_id = add_res.json["id"].as_i64().unwrap();
    assert_eq!(add_res.json["is_active"], false);

    // Profile check
    let prof = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(prof.status, StatusCode::OK);
    assert!(prof.json["vehicles"].as_array().unwrap().len() >= 2);
    assert_ne!(prof.json["active_vehicle_id"], new_vehicle_id);

    // Choose the new vehicle as active
    let sel_res = call(
        &app,
        Method::POST,
        &format!("/api/driver/vehicles/{new_vehicle_id}/select"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(sel_res.status, StatusCode::OK);
    assert_eq!(sel_res.json["active_vehicle_id"], new_vehicle_id);

    // Profile check: active vehicle is now Vellfire
    let prof_after = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(prof_after.json["active_vehicle_id"], new_vehicle_id);
    assert_eq!(prof_after.json["vehicle"]["plate"], "VLL 7788");
    assert_eq!(prof_after.json["vehicle"]["seats"], 7);
    assert_eq!(prof_after.json["vehicle"]["wheelchair_ok"], true);

    // Update the vehicle
    let upd_res = call(
        &app,
        Method::PUT,
        &format!("/api/driver/vehicles/{new_vehicle_id}"),
        Some(&driver),
        Some(json!({
            "model": "Toyota Alphard",
            "color": "White",
            "plate": "VLL 7788",
            "seats": 8,
            "vehicle_type": "standard",
            "wheelchair_ok": true,
            "low_step": true,
            "child_seat": true,
        })),
    )
    .await;
    assert_eq!(upd_res.status, StatusCode::OK);
    assert_eq!(upd_res.json["model"], "Toyota Alphard");
    assert_eq!(upd_res.json["seats"], 8);

    // Switch active vehicle back using PUT /api/driver/profile
    let original_id = initial_vehicles[0]["id"].as_i64().unwrap();
    let prof_put = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 2,
            "area_ids": [1],
            "active_vehicle_id": original_id
        })),
    )
    .await;
    assert_eq!(prof_put.status, StatusCode::OK);

    let prof_check2 = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(prof_check2.json["active_vehicle_id"], original_id);

    // Delete the new vehicle
    let del_res = call(
        &app,
        Method::DELETE,
        &format!("/api/driver/vehicles/{new_vehicle_id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(del_res.status, StatusCode::OK);

    let list_after = call(
        &app,
        Method::GET,
        "/api/driver/vehicles",
        Some(&driver),
        None,
    )
    .await;
    assert!(!list_after
        .json
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["id"] == new_vehicle_id));

    // Auto-clamp spare_seats to active vehicle capacity (seats - 1) on PUT /api/driver/profile
    let compact = call(
        &app,
        Method::POST,
        "/api/driver/vehicles",
        Some(&driver),
        Some(json!({
            "model": "Smart Fortwo",
            "color": "Yellow",
            "plate": "SMT 2002",
            "seats": 2,
            "is_active": false
        })),
    )
    .await;
    assert_eq!(compact.status, StatusCode::OK, "{}", compact.json);
    let compact_id = compact.json["id"].as_i64().unwrap();

    // Active original vehicle has 5 seats; spare_seats=5 clamps to 4
    let clamp_prof = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 5,
            "area_ids": [1]
        })),
    )
    .await;
    assert_eq!(clamp_prof.status, StatusCode::OK, "{}", clamp_prof.json);

    let prof_clamped_orig = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(prof_clamped_orig.json["spare_seats"], 4);

    // Raise spare_seats to 3 on the 5-seat vehicle, then switch to 2-seat compact vehicle -> clamps spare_seats to 1
    let set3 = call(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 3,
            "area_ids": [1]
        })),
    )
    .await;
    assert_eq!(set3.status, StatusCode::OK, "{}", set3.json);

    let sel_compact = call(
        &app,
        Method::POST,
        &format!("/api/driver/vehicles/{compact_id}/select"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(sel_compact.status, StatusCode::OK, "{}", sel_compact.json);

    let prof_clamped = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(prof_clamped.json["active_vehicle_id"], compact_id);
    assert_eq!(prof_clamped.json["spare_seats"], 1);
}

#[tokio::test]
async fn multiple_saved_places_defaults_isolation_and_preserve() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let area = first_area_id(&st).await;

    let home = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({
            "label": "Home",
            "area_id": area,
            "address": "10 Jalan SS2",
            "gate_notes": "Gate 2",
            "is_default": true
        })),
    )
    .await;
    assert_eq!(home.status, StatusCode::OK, "{}", home.json);
    let home_id = home.json["id"].as_i64().unwrap();

    let office = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({
            "label": "Office",
            "area_id": area,
            "address": "88 Tower B",
            "gate_notes": "Lobby",
            "is_default": false
        })),
    )
    .await;
    assert_eq!(office.status, StatusCode::OK, "{}", office.json);
    let office_id = office.json["id"].as_i64().unwrap();

    let parents = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({
            "label": "Parents",
            "area_id": area,
            "address": "22 Jalan Aman"
        })),
    )
    .await;
    assert_eq!(parents.status, StatusCode::OK, "{}", parents.json);
    let parents_id = parents.json["id"].as_i64().unwrap();

    // Ordered by default DESC, then label ASC
    let list1 = call(&app, Method::GET, "/api/saved-places", Some(&member), None).await;
    assert_eq!(list1.status, StatusCode::OK);
    let arr1 = list1.json.as_array().unwrap();
    assert_eq!(arr1.len(), 3);
    assert_eq!(arr1[0]["label"], "Home");
    assert_eq!(arr1[0]["is_default"], 1);
    assert_eq!(arr1[1]["label"], "Office");
    assert_eq!(arr1[1]["is_default"], 0);
    assert_eq!(arr1[2]["label"], "Parents");
    assert_eq!(arr1[2]["is_default"], 0);

    // Promote Office to default
    let promote = call(
        &app,
        Method::PUT,
        &format!("/api/saved-places/{office_id}"),
        Some(&member),
        Some(json!({
            "label": "Office",
            "area_id": area,
            "address": "88 Tower B",
            "gate_notes": "Lobby",
            "is_default": true
        })),
    )
    .await;
    assert_eq!(promote.status, StatusCode::OK, "{}", promote.json);

    let list2 = call(&app, Method::GET, "/api/saved-places", Some(&member), None).await;
    let arr2 = list2.json.as_array().unwrap();
    assert_eq!(arr2[0]["id"], office_id);
    assert_eq!(arr2[0]["is_default"], 1);
    let home_row = arr2.iter().find(|r| r["id"] == home_id).unwrap();
    assert_eq!(home_row["is_default"], 0);

    // Omitting is_default preserves existing default flag (stays 1)
    let upd_omit = call(
        &app,
        Method::PUT,
        &format!("/api/saved-places/{office_id}"),
        Some(&member),
        Some(json!({
            "label": "Office",
            "area_id": area,
            "address": "88 Tower B",
            "gate_notes": "Side entrance"
        })),
    )
    .await;
    assert_eq!(upd_omit.status, StatusCode::OK, "{}", upd_omit.json);

    let list3 = call(&app, Method::GET, "/api/saved-places", Some(&member), None).await;
    let arr3 = list3.json.as_array().unwrap();
    assert_eq!(arr3[0]["id"], office_id);
    assert_eq!(arr3[0]["is_default"], 1);
    assert_eq!(arr3[0]["gate_notes"], "Side entrance");

    // Per-user isolation: another member cannot view, edit, or delete member's places
    let sophie_list = call(&app, Method::GET, "/api/saved-places", Some(&sophie), None).await;
    assert_eq!(sophie_list.status, StatusCode::OK);
    assert_eq!(sophie_list.json.as_array().unwrap().len(), 0);

    let sophie_put = call(
        &app,
        Method::PUT,
        &format!("/api/saved-places/{office_id}"),
        Some(&sophie),
        Some(json!({
            "label": "Hijack",
            "area_id": area,
            "address": "1 Bad St"
        })),
    )
    .await;
    assert_eq!(sophie_put.status, StatusCode::NOT_FOUND);

    let sophie_del = call(
        &app,
        Method::DELETE,
        &format!("/api/saved-places/{home_id}"),
        Some(&sophie),
        None,
    )
    .await;
    assert_eq!(sophie_del.status, StatusCode::NOT_FOUND);

    let sophie_create = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&sophie),
        Some(json!({
            "label": "Home",
            "area_id": area,
            "address": "5 Jalan Damansara",
            "is_default": true
        })),
    )
    .await;
    assert_eq!(sophie_create.status, StatusCode::OK);

    // Member deletes Parents, leaving 2 places
    let del_parents = call(
        &app,
        Method::DELETE,
        &format!("/api/saved-places/{parents_id}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(del_parents.status, StatusCode::OK);

    let list4 = call(&app, Method::GET, "/api/saved-places", Some(&member), None).await;
    assert_eq!(list4.json.as_array().unwrap().len(), 2);

    // Explicit coordinates are stored on POST, preserved when unchanged address is PUT without coords,
    // and out-of-range coordinates are rejected.
    let bad_coord = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({
            "label": "Invalid",
            "area_id": area,
            "address": "99 Nowheresville",
            "lat": 120.0,
            "lng": 101.6
        })),
    )
    .await;
    assert_eq!(bad_coord.status, StatusCode::BAD_REQUEST);

    let pinned = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({
            "label": "Clinic",
            "area_id": area,
            "address": "15 Jalan Gasing",
            "lat": 3.1025,
            "lng": 101.6481
        })),
    )
    .await;
    assert_eq!(pinned.status, StatusCode::OK, "{}", pinned.json);
    let pinned_id = pinned.json["id"].as_i64().unwrap();
    assert_eq!(pinned.json["lat"], 3.1025);
    assert_eq!(pinned.json["lng"], 101.6481);

    // Update label without resending coords while keeping address unchanged -> coords preserved
    let upd_pin = call(
        &app,
        Method::PUT,
        &format!("/api/saved-places/{pinned_id}"),
        Some(&member),
        Some(json!({
            "label": "GP Clinic",
            "area_id": area,
            "address": "15 Jalan Gasing",
            "is_default": true
        })),
    )
    .await;
    assert_eq!(upd_pin.status, StatusCode::OK, "{}", upd_pin.json);

    let list5 = call(&app, Method::GET, "/api/saved-places", Some(&member), None).await;
    let pinned_row = list5
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == pinned_id)
        .unwrap();
    assert_eq!(pinned_row["label"], "GP Clinic");
    assert_eq!(pinned_row["lat"], 3.1025);
    assert_eq!(pinned_row["lng"], 101.6481);
}
