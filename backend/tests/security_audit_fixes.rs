//! Regression tests covering the security, PDPA privacy, concurrency, and workflow fixes.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::state::AppState;
use serde_json::json;

fn future_dt(days: i64, hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (grace_ride::util::today_local() + chrono::Duration::days(days)).format("%Y-%m-%d")
    )
}

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

async fn all_group_id(st: &AppState) -> i64 {
    sqlx::query_scalar("SELECT id FROM groups WHERE is_all=1 ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn join_as_group(app: &axum::Router, cookie: &str, ev_id: i64) -> i64 {
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

#[tokio::test]
async fn session_lock_defaults_sse_logout_all_and_backup_code_rate_limit() {
    let (app, st) = setup().await;
    let member_id = user_id(&st, "member@example.org").await;

    // 1. Default auto_lock_mins == 0 inherits org session_timeout_mins (15 min)
    let session1 = login(&st, "member@example.org").await;
    sqlx::query("UPDATE users SET auto_lock_mins=0 WHERE id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let old_seen = (grace_ride::util::local_now() - chrono::Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query("UPDATE sessions SET last_seen_at=$1 WHERE user_id=$2")
        .bind(&old_seen)
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();

    let me = call(&app, Method::GET, "/api/auth/me", Some(&session1), None).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.json);
    assert_eq!(me.json["locked"], true);

    // Locked sessions must be blocked on /api/stream and /api/auth/logout-all
    let stream_res = call(&app, Method::GET, "/api/stream", Some(&session1), None).await;
    assert_eq!(stream_res.status, StatusCode::LOCKED, "{}", stream_res.json);
    assert_eq!(stream_res.json["code"], "SESSION_LOCKED");

    let logout_all_res = call(
        &app,
        Method::POST,
        "/api/auth/logout-all",
        Some(&session1),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        logout_all_res.status,
        StatusCode::LOCKED,
        "{}",
        logout_all_res.json
    );
    assert_eq!(logout_all_res.json["code"], "SESSION_LOCKED");

    // 2. Per-user backup-code rate limit blocks brute-force on /api/auth/unlock
    let session2 = login(&st, "driver@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    sqlx::query("UPDATE sessions SET locked_at=$1 WHERE user_id=$2")
        .bind(grace_ride::util::now())
        .bind(driver_id)
        .execute(&st.db)
        .await
        .unwrap();

    for i in 0..10 {
        let r = call(
            &app,
            Method::POST,
            "/api/auth/unlock",
            Some(&session2),
            Some(json!({ "backup_code": format!("BADCODE-{i}") })),
        )
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "attempt {i}: {}", r.json);
    }
    let blocked = call(
        &app,
        Method::POST,
        "/api/auth/unlock",
        Some(&session2),
        Some(json!({ "backup_code": "BADCODE-FINAL" })),
    )
    .await;
    assert_eq!(
        blocked.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        blocked.json
    );
    assert!(blocked.json["error"].is_string(), "{}", blocked.json);
    assert!(blocked
        .headers
        .contains_key(axum::http::header::RETRY_AFTER));
}

#[tokio::test]
async fn ssrf_webhook_validation_bot_token_check_and_secret_masking() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;

    // Pure SSRF policy checks for production (`dev_mode = false`) & dev (`dev_mode = true`)
    assert!(!grace_ride::util::validate_webhook_target("http://hooks.example.org/a", false).await);
    assert!(!grace_ride::util::validate_webhook_target("https://127.0.0.1/hook", false).await);
    assert!(!grace_ride::util::validate_webhook_target("https://10.0.0.8/hook", false).await);
    assert!(!grace_ride::util::validate_webhook_target("https://localhost/hook", false).await);
    assert!(!grace_ride::util::validate_webhook_target("https://svc.internal/hook", false).await);
    assert!(
        !grace_ride::util::validate_webhook_target(
            "https://169.254.169.254/latest/meta-data",
            false
        )
        .await
    );
    assert!(
        !grace_ride::util::validate_webhook_target(
            "https://169.254.169.254/latest/meta-data",
            true
        )
        .await
    );
    assert!(
        !grace_ride::util::validate_webhook_target("https://user:pass@hooks.example.org/a", true)
            .await
    );

    // Reject metadata SSRF & invalid Telegram bot token format via HTTP handler
    let meta = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "alert_webhook_url": "https://169.254.169.254/latest/meta-data" })),
    )
    .await;
    assert_eq!(meta.status, StatusCode::BAD_REQUEST, "{}", meta.json);

    let bad_tok = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "telegram_bot_token": "not-a-valid-bot-token" })),
    )
    .await;
    assert_eq!(bad_tok.status, StatusCode::BAD_REQUEST, "{}", bad_tok.json);

    // Valid HTTPS webhook is stored & masked in GET responses, then clearable
    let saved = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "alert_webhook_url": "https://hooks.example.org/secret-path-123" })),
    )
    .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.json);

    let got = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(got.status, StatusCode::OK, "{}", got.json);
    assert_eq!(got.json["alert_webhook_configured"], true);
    let masked = got.json["alert_webhook_url"].as_str().unwrap_or("");
    assert!(
        !masked.contains("secret-path-123"),
        "secret leaked: {masked}"
    );
    assert!(masked.contains("hooks.example.org"), "{masked}");

    let cleared = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "clear_alert_webhook_url": true })),
    )
    .await;
    assert_eq!(cleared.status, StatusCode::OK, "{}", cleared.json);

    let after_clear = call(
        &app,
        Method::GET,
        "/api/admin/settings/alerts",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(after_clear.json["alert_webhook_configured"], false);
    assert_eq!(after_clear.json["alert_webhook_url"], "");
}

#[tokio::test]
async fn pdpa_return_leg_caregiver_ics_masking_and_disable_revocation() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let area = first_area_id(&st).await;
    let all_group = all_group_id(&st).await;
    let sophie_id = user_id(&st, "sophie@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&member),
            Some(json!({ "caregiver_user_id": sophie_id }))
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
            "title": "PDPA Round Trip Service",
            "kind": "other",
            "starts_at": future_dt(2, "09:30"),
            "location": "Main Sanctuary",
            "capacity": 20,
            "group_ids": [all_group]
        })),
    )
    .await;
    let ev_id = ev.json["id"].as_i64().unwrap();
    let jg = join_as_group(&app, &member, ev_id).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{ev_id}/signup"),
            Some(&member),
            Some(json!({ "as_group_id": jg }))
        )
        .await
        .status,
        StatusCode::OK
    );

    let rt = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(2, "08:30"),
            "pickup_area_id": area,
            "pickup_address": "88 Secret Terrace, PJ",
            "destination": "Main Sanctuary",
            "seats": 1,
            "return_pickup_at": future_dt(2, "12:00"),
            "return_pickup_area_id": area,
            "return_pickup_address": "Main Sanctuary",
            "return_destination": "88 Secret Terrace, PJ"
        })),
    )
    .await;
    assert_eq!(rt.status, StatusCode::OK, "{}", rt.json);
    let out_id = rt.json["id"].as_i64().unwrap();
    let ret_id = rt.json["return_ride_id"].as_i64().unwrap();

    // Unapproved driver viewing open pool sees masked pickup & return destination
    let ret_driver_view = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ret_id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(
        ret_driver_view.status,
        StatusCode::OK,
        "{}",
        ret_driver_view.json
    );
    assert_ne!(ret_driver_view.json["destination"], "88 Secret Terrace, PJ");
    assert_eq!(
        ret_driver_view.json["destination"],
        ret_driver_view.json["area"]["name"]
    );

    // Driver claims return leg -> calendar ICS for unapproved driver must also mask destination street address
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{ret_id}/claim"),
            Some(&driver),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );
    let d_cal = call(
        &app,
        Method::POST,
        "/api/me/calendar-feed/rotate",
        Some(&driver),
        Some(json!({})),
    )
    .await;
    assert_eq!(d_cal.status, StatusCode::OK, "{}", d_cal.json);
    let d_ics_path = d_cal.json["ics_path"].as_str().unwrap().to_string();
    let (d_ics_st, _, d_ics_body) = call_raw(&app, Method::GET, &d_ics_path, None).await;
    assert_eq!(d_ics_st, StatusCode::OK);
    assert!(
        d_ics_body.contains("DTSTART:") && d_ics_body.contains('Z'),
        "{d_ics_body}"
    );
    assert!(
        !d_ics_body.contains("88 Secret Terrace"),
        "driver ICS leaked home address: {d_ics_body}"
    );

    // Nominated caregiver (`sophie`) can view masked ride detail & public /care/{token} omits raw GPS
    let cg_ride = call(
        &app,
        Method::GET,
        &format!("/api/rides/{ret_id}"),
        Some(&sophie),
        None,
    )
    .await;
    assert_eq!(cg_ride.status, StatusCode::OK, "{}", cg_ride.json);
    assert_eq!(cg_ride.json["destination"], cg_ride.json["area"]["name"]);

    let cl = call(
        &app,
        Method::POST,
        &format!("/api/rides/{out_id}/care-link"),
        Some(&member),
        Some(json!({ "label": "Care Link" })),
    )
    .await;
    assert_eq!(cl.status, StatusCode::OK, "{}", cl.json);
    let token = cl.json["token"].as_str().unwrap().to_string();

    // Advance outbound ride through completion -> care link hides driver/plate afterwards & drops GPS lat/lng
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
    let care_live = call(&app, Method::GET, &format!("/api/care/{token}"), None, None).await;
    assert_eq!(care_live.status, StatusCode::OK);
    assert!(care_live.json["location"].get("lat").is_none());
    assert!(care_live.json["location"].get("lng").is_none());

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/complete"),
            Some(&driver),
            Some(json!({ "km": 5 }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let care_done = call(&app, Method::GET, &format!("/api/care/{token}"), None, None).await;
    assert_eq!(care_done.status, StatusCode::OK);
    assert_eq!(care_done.json["status"], "completed");
    assert!(care_done.json["driver"].is_null());

    // Disabling user revokes their calendar token AND deletes Web Push subscriptions
    let m_cal = call(
        &app,
        Method::POST,
        "/api/me/calendar-feed/rotate",
        Some(&member),
        Some(json!({})),
    )
    .await;
    let m_ics = m_cal.json["ics_path"].as_str().unwrap().to_string();
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/push/subscribe",
            Some(&member),
            Some(
                json!({ "endpoint": "https://push.example.org/u/m1", "p256dh": "k", "auth": "a" })
            )
        )
        .await
        .status,
        StatusCode::OK
    );
    let dis = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}"),
        Some(&admin),
        Some(json!({ "roles": ["member"], "status": "disabled" })),
    )
    .await;
    assert_eq!(dis.status, StatusCode::OK, "{}", dis.json);

    let (rev_st, _, _) = call_raw(&app, Method::GET, &m_ics, None).await;
    assert_eq!(rev_st, StatusCode::NOT_FOUND);
    let push_left: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM push_subscriptions WHERE user_id=$1")
            .bind(member_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(push_left, 0);
}

#[tokio::test]
async fn plan_per_leg_capacity_duplicate_event_ownership_and_attendance_guards() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let superadmin = login(&st, "superadmin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let area = first_area_id(&st).await;
    let all_group = all_group_id(&st).await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    // Non-carpool driver with 4 seats can be assigned BOTH legs of a 2-seat round trip via plan
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/driver/profile",
            Some(&driver),
            Some(json!({
                "carpool_enabled": false,
                "spare_seats": 4,
                "area_ids": [area],
                "vehicle": {
                    "model": "Proton X70",
                    "color": "Grey",
                    "plate": "VAA 404",
                    "seats": 4,
                    "vehicle_type": "standard"
                }
            }))
        )
        .await
        .status,
        StatusCode::OK
    );

    let photo_id = upload_png(&app, &admin).await;
    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Plan & Capacity Test",
            "kind": "other",
            "starts_at": future_dt(4, "09:30"),
            "location": "Hall",
            "capacity": 1,
            "attendance_enabled": true,
            "photo_file_id": photo_id,
            "group_ids": [all_group]
        })),
    )
    .await;
    assert_eq!(ev.status, StatusCode::OK, "{}", ev.json);
    let ev_id = ev.json["id"].as_i64().unwrap();
    let jg = join_as_group(&app, &member, ev_id).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{ev_id}/signup"),
            Some(&member),
            Some(json!({ "as_group_id": jg }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let rt = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "pickup_at": future_dt(4, "08:30"),
            "pickup_area_id": area,
            "pickup_address": "10 Jalan Maju",
            "destination": "Hall",
            "seats": 2,
            "return_pickup_at": future_dt(4, "12:00"),
            "return_pickup_area_id": area,
            "return_pickup_address": "Hall",
            "return_destination": "10 Jalan Maju"
        })),
    )
    .await;
    let out_id = rt.json["id"].as_i64().unwrap();
    let ret_id = rt.json["return_ride_id"].as_i64().unwrap();

    let save = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{ev_id}/plan"),
        Some(&transport),
        Some(json!({
            "changes": [
                { "ride_id": out_id, "from_driver_id": null, "driver_id": driver_id },
                { "ride_id": ret_id, "from_driver_id": null, "driver_id": driver_id }
            ]
        })),
    )
    .await;
    assert_eq!(save.status, StatusCode::OK, "{}", save.json);

    // Duplicating an event as another organiser drops unowned `photo_file_id` safely
    let dup = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/duplicate"),
        Some(&superadmin),
        Some(json!({ "starts_at": future_dt(11, "09:30"), "title": "Duplicated Event" })),
    )
    .await;
    assert_eq!(dup.status, StatusCode::OK, "{}", dup.json);
    let dup_id = dup.json["id"].as_i64().unwrap();
    let dup_photo: Option<i64> = sqlx::query_scalar("SELECT photo_file_id FROM events WHERE id=$1")
        .bind(dup_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(dup_photo, None);

    // Waitlisted member cannot bypass full capacity via self_checkin or scan-member
    let sjg = join_as_group(&app, &sophie, ev_id).await;
    let ssu = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/signup"),
        Some(&sophie),
        Some(json!({ "as_group_id": sjg })),
    )
    .await;
    assert_eq!(ssu.json["status"], "waitlist");
    let ev_info = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}"),
        Some(&admin),
        None,
    )
    .await;
    let checkin_code = ev_info.json["checkin_code"].as_str().unwrap().to_string();
    let self_ci = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/checkin"),
        Some(&sophie),
        Some(json!({ "code": checkin_code })),
    )
    .await;
    assert_eq!(self_ci.status, StatusCode::BAD_REQUEST, "{}", self_ci.json);
    assert_eq!(self_ci.json["code"], "SIGNUP_CLOSED");

    let sophie_pass: String =
        sqlx::query_scalar("SELECT pass_code FROM event_signups WHERE event_id=$1 AND user_id=$2")
            .bind(ev_id)
            .bind(user_id(&st, "sophie@example.org").await)
            .fetch_one(&st.db)
            .await
            .unwrap();
    let scan_full = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": sophie_pass })),
    )
    .await;
    assert_eq!(
        scan_full.status,
        StatusCode::BAD_REQUEST,
        "{}",
        scan_full.json
    );
    assert_eq!(scan_full.json["code"], "SIGNUP_CLOSED");

    // Disabled member pass cannot be scanned for attendance
    let member_pass: String =
        sqlx::query_scalar("SELECT pass_code FROM event_signups WHERE event_id=$1 AND user_id=$2")
            .bind(ev_id)
            .bind(member_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{member_id}"),
            Some(&admin),
            Some(json!({ "roles": ["member"], "status": "disabled" }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let scan_disabled = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/checkin/scan-member"),
        Some(&admin),
        Some(json!({ "pass_code": member_pass })),
    )
    .await;
    assert_eq!(
        scan_disabled.status,
        StatusCode::FORBIDDEN,
        "{}",
        scan_disabled.json
    );
}

#[tokio::test]
async fn extract_ref_authz_batch_cancel_csv_neutralization_and_report_scoping() {
    let (app, st) = setup().await;
    let driver = login(&st, "driver@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    let cell_leader = login(&st, "cellleader@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let all_group = all_group_id(&st).await;

    // 1. Unrelated user cannot run extract-ref on someone else's uploaded receipt
    let receipt_id = upload_png(&app, &driver).await;
    let unauth_ext = call(
        &app,
        Method::POST,
        &format!("/api/files/{receipt_id}/extract-ref"),
        Some(&sophie),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        unauth_ext.status,
        StatusCode::FORBIDDEN,
        "{}",
        unauth_ext.json
    );

    // 2. Formula injection in payout batch CSV is neutralized, open batch blocks single pay, DELETE cancels batch
    let claim = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&driver),
        Some(json!({
            "amount_cents": 1800,
            "description": "+SUM(A1)",
            "receipt_file_id": receipt_id,
            "payee_name": "=CMD|' /C calc'!A0",
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
            Some(json!({ "action": "approve", "note": "" }))
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
        Some(json!({ "month": month })),
    )
    .await;
    assert_eq!(batch.status, StatusCode::OK, "{}", batch.json);
    let batch_id = batch.json["id"].as_i64().unwrap();

    let (csv_st, _, csv_body) = call_raw(
        &app,
        Method::GET,
        &format!("/api/reimbursements/batches/{batch_id}/csv"),
        Some(&finance),
    )
    .await;
    assert_eq!(csv_st, StatusCode::OK);
    assert!(csv_body.contains("'=CMD"), "{csv_body}");
    assert!(csv_body.contains("'+SUM(A1)"), "{csv_body}");

    // Single claim pay must be blocked while claim belongs to a batch
    let single_pay = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{claim_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-SINGLE-99" })),
    )
    .await;
    assert_eq!(
        single_pay.status,
        StatusCode::CONFLICT,
        "{}",
        single_pay.json
    );
    assert_eq!(single_pay.json["code"], "REIMBURSEMENT_PROCESSED");

    // Cancel open batch -> releases claim
    let cancel_b = call(
        &app,
        Method::DELETE,
        &format!("/api/reimbursements/batches/{batch_id}"),
        Some(&finance),
        None,
    )
    .await;
    assert_eq!(cancel_b.status, StatusCode::OK, "{}", cancel_b.json);

    // 3. Report scoping: cell leader cannot view another user's monthly report and demand_report scopes by_event
    let unauth_report = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?user_id={driver_id}"),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(
        unauth_report.status,
        StatusCode::FORBIDDEN,
        "{}",
        unauth_report.json
    );

    let admin_ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Global Admin Only Event",
            "kind": "other",
            "starts_at": future_dt(5, "09:30"),
            "location": "Church",
            "capacity": 20,
            "group_ids": [all_group]
        })),
    )
    .await;
    let admin_ev_id = admin_ev.json["id"].as_i64().unwrap();

    let leader_demand = call(
        &app,
        Method::GET,
        &format!(
            "/api/admin/reports/demand?from={}&to={}",
            &future_dt(0, "00:00")[..10],
            &future_dt(10, "23:59")[..10]
        ),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(
        leader_demand.status,
        StatusCode::OK,
        "{}",
        leader_demand.json
    );
    let area_id = first_area_id(&st).await;
    let driver_jg = join_as_group(&app, &driver, admin_ev_id).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{admin_ev_id}/signup"),
            Some(&driver),
            Some(json!({ "as_group_id": driver_jg }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/rides",
            Some(&driver),
            Some(json!({
                "event_id": admin_ev_id,
                "pickup_at": future_dt(5, "08:30"),
                "pickup_area_id": area_id,
                "pickup_address": "123 Jalan Other",
                "destination": "Church",
                "seats": 1
            }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let leader_demand_after = call(
        &app,
        Method::GET,
        &format!(
            "/api/admin/reports/demand?from={}&to={}",
            &future_dt(0, "00:00")[..10],
            &future_dt(10, "23:59")[..10]
        ),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(leader_demand_after.status, StatusCode::OK);
    assert_eq!(
        leader_demand_after.json["totals"]["total_requests"], 0,
        "cell leader saw rides from global admin event: {}",
        leader_demand_after.json
    );
}

#[tokio::test]
async fn push_endpoint_allowlist_and_sse_graceful_shutdown_drain() {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    // 1. Production Web Push host allowlist blocks arbitrary third-party relays & IP literals
    assert!(grace_ride::util::is_allowed_push_host("fcm.googleapis.com"));
    assert!(grace_ride::util::is_allowed_push_host(
        "updates.push.services.mozilla.com"
    ));
    assert!(grace_ride::util::is_allowed_push_host(
        "autopush.prod.push.services.mozilla.com"
    ));
    assert!(grace_ride::util::is_allowed_push_host("web.push.apple.com"));
    assert!(grace_ride::util::is_allowed_push_host(
        "sg2p.notify.windows.com"
    ));
    assert!(grace_ride::util::is_allowed_push_host(
        "apn.push.samsungosp.com"
    ));
    assert!(!grace_ride::util::is_allowed_push_host("hooks.example.org"));
    assert!(!grace_ride::util::is_allowed_push_host(
        "fcm.googleapis.com.attacker.org"
    ));
    assert!(!grace_ride::util::is_allowed_push_host("push.apple.com"));
    assert!(!grace_ride::util::is_allowed_push_host("1.1.1.1"));
    assert!(
        !grace_ride::util::validate_push_endpoint("https://hooks.example.org/push", false).await
    );
    assert!(!grace_ride::util::validate_push_endpoint("https://1.1.1.1/push", false).await);

    // 2. Open /api/stream SSE connections drain immediately when shutdown fires
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/stream")
        .header(axum::http::header::COOKIE, &member)
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body();
    let _ = st.shutdown.send(true);
    let drained = tokio::time::timeout(std::time::Duration::from_secs(2), body.collect()).await;
    assert!(
        drained.is_ok(),
        "SSE stream did not close promptly on shutdown"
    );
}
