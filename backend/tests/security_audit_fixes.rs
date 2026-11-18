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

async fn upload_pdf(app: &axum::Router, cookie: &str) -> i64 {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut pdf = b"%PDF-1.4\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n".to_vec();
    let boundary = "XgrPdfBoundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"doc.pdf\"\r\nContent-Type: application/pdf\r\n\r\n"
    )
    .into_bytes();
    body.append(&mut pdf);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let mct = format!("multipart/form-data; boundary={boundary}");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(axum::http::header::COOKIE, cookie)
        .header(axum::http::header::ORIGIN, ORIGIN)
        .header(axum::http::header::CONTENT_TYPE, &mct)
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let file_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    assert_eq!(status, StatusCode::OK, "{file_json}");
    file_json["id"].as_i64().unwrap()
}

#[tokio::test]
async fn care_eta_pdpa_return_leg_and_proxy_route_privacy() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let area = first_area_id(&st).await;
    let all_group = all_group_id(&st).await;
    let driver_id = user_id(&st, "driver@example.org").await;

    // Ensure driver has carpool enabled with 4 seats
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/driver/profile",
            Some(&driver),
            Some(json!({
                "carpool_enabled": true,
                "spare_seats": 4,
                "area_ids": [area],
                "vehicle": { "model": "Myvi", "color": "Silver", "plate": "WYY 777", "seats": 4 }
            }))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Seed geocode_cache for Church and two return homes so ETA computes without network calls
    for (q, lat, lng) in [
        ("main sanctuary", 3.1000_f64, 101.6000_f64),
        ("12 jalan secretalpha", 3.1100_f64, 101.6100_f64),
        ("99 jalan secretbeta", 3.1200_f64, 101.6200_f64),
    ] {
        sqlx::query(
            "INSERT INTO geocode_cache (query, lat, lng, approx, updated_at)
             VALUES ($1,$2,$3,FALSE,$4) ON CONFLICT (query) DO NOTHING",
        )
        .bind(q)
        .bind(lat)
        .bind(lng)
        .bind(grace_ride::util::now())
        .execute(&st.db)
        .await
        .unwrap();
    }

    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Return Carpool PDPA",
            "kind": "other",
            "starts_at": future_dt(8, "09:30"),
            "location": "Main Sanctuary",
            "capacity": 20,
            "group_ids": [all_group]
        })),
    )
    .await;
    let ev_id = ev.json["id"].as_i64().unwrap();
    for c in [&member, &sophie] {
        let jg = join_as_group(&app, c, ev_id).await;
        assert_eq!(
            call(
                &app,
                Method::POST,
                &format!("/api/events/{ev_id}/signup"),
                Some(c),
                Some(json!({ "as_group_id": jg }))
            )
            .await
            .status,
            StatusCode::OK
        );
    }

    // Book two return rides with identical pickup_at & different home destinations
    let r1 = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "event_id": ev_id,
            "leg": "return",
            "pickup_at": future_dt(8, "12:00"),
            "pickup_area_id": area,
            "pickup_address": "Main Sanctuary",
            "destination": "12 Jalan SecretAlpha",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(r1.status, StatusCode::OK, "{}", r1.json);
    let r1_id = r1.json["id"].as_i64().unwrap();

    let r2 = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&sophie),
        Some(json!({
            "event_id": ev_id,
            "leg": "return",
            "pickup_at": future_dt(8, "12:00"),
            "pickup_area_id": area,
            "pickup_address": "Main Sanctuary",
            "destination": "99 Jalan SecretBeta",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(r2.status, StatusCode::OK, "{}", r2.json);
    let r2_id = r2.json["id"].as_i64().unwrap();

    // Assign & approve both return rides onto the same driver trip
    for rid in [r1_id, r2_id] {
        let cookie = if rid == r1_id { &member } else { &sophie };
        assert_eq!(
            call(
                &app,
                Method::POST,
                &format!("/api/rides/{rid}/claim"),
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
                &format!("/api/rides/{rid}/respond"),
                Some(cookie),
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
                &format!("/api/rides/{rid}/approve"),
                Some(&transport),
                Some(json!({}))
            )
            .await
            .status,
            StatusCode::OK
        );
    }

    let care = call(
        &app,
        Method::POST,
        &format!("/api/rides/{r1_id}/care-link"),
        Some(&member),
        Some(json!({ "label": "Family" })),
    )
    .await;
    assert_eq!(care.status, StatusCode::OK, "{}", care.json);
    let care_tok = care.json["token"].as_str().unwrap().to_string();

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{r1_id}/start"),
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
            "/api/driver/location",
            Some(&driver),
            Some(json!({ "lat": 3.095, "lng": 101.595 }))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Care pass status before pickup must strip eta.target, eta.path, and eta.destination
    let care_live = call(
        &app,
        Method::GET,
        &format!("/api/care/{care_tok}"),
        None,
        None,
    )
    .await;
    assert_eq!(care_live.status, StatusCode::OK, "{}", care_live.json);
    let care_eta = &care_live.json["location"]["eta"];
    assert!(care_eta.get("duration_s").is_some(), "{}", care_live.json);
    assert!(
        care_eta.get("target").is_none(),
        "care leaked target: {care_eta}"
    );
    assert!(
        care_eta.get("path").is_none(),
        "care leaked path: {care_eta}"
    );
    assert!(
        care_eta.get("destination").is_none(),
        "care leaked destination: {care_eta}"
    );

    // Both riders picked up -> Sophie's location ETA must never leak Rider 1's home address/coords/path
    for rid in [r1_id, r2_id] {
        assert_eq!(
            call(
                &app,
                Method::POST,
                &format!("/api/rides/{rid}/pickup"),
                Some(&driver),
                Some(json!({}))
            )
            .await
            .status,
            StatusCode::OK
        );
    }
    let sophie_loc = call(
        &app,
        Method::GET,
        &format!("/api/rides/{r2_id}/location"),
        Some(&sophie),
        None,
    )
    .await;
    assert_eq!(sophie_loc.status, StatusCode::OK, "{}", sophie_loc.json);
    assert!(
        !sophie_loc.json.to_string().contains("SecretAlpha"),
        "leaked r1 dest: {}",
        sophie_loc.json
    );
    assert_eq!(sophie_loc.json["eta"]["destination"], "99 Jalan SecretBeta");
    assert_eq!(sophie_loc.json["eta"]["located"], false);
    assert!(
        sophie_loc.json["eta"].get("target").is_none(),
        "{}",
        sophie_loc.json
    );
    assert!(
        sophie_loc.json["eta"].get("path").is_none(),
        "{}",
        sophie_loc.json
    );

    // Proxy ride shows actual rider_name in /api/driver/route
    let pr = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": future_dt(9, "08:30"),
            "pickup_area_id": area,
            "pickup_address": "12 Jalan SecretAlpha",
            "destination": "Main Sanctuary",
            "seats": 1,
            "is_proxy": true,
            "rider_name": "Grandma Lim",
            "rider_phone": "012-8887777"
        })),
    )
    .await;
    assert_eq!(pr.status, StatusCode::OK, "{}", pr.json);
    let pr_id = pr.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{pr_id}/claim"),
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
            &format!("/api/rides/{pr_id}/respond"),
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
            &format!("/api/rides/{pr_id}/approve"),
            Some(&transport),
            Some(json!({}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // GET /api/driver/route serves only the current trip, which here is an earlier ride,
    // so build this trip's route directly.
    let route = grace_ride::routing::driver_route(&st, driver_id, Some(pr_id))
        .await
        .unwrap()
        .expect("route for the proxy trip");
    assert_eq!(route.stops[0].passenger, "Grandma Lim");
}

#[tokio::test]
async fn visitor_pass_cell_leader_scope_and_group_deletion_guard() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let cell_leader = login(&st, "cellleader@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let all_group = all_group_id(&st).await;
    let area = first_area_id(&st).await;

    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Admin Global Event",
            "kind": "other",
            "starts_at": future_dt(3, "09:30"),
            "location": "Church",
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
    let vp = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/visitors"),
        Some(&member),
        Some(json!({
            "name": "Uncle Visitor",
            "phone": "012-7776666",
            "need_ride": true,
            "pickup_at": future_dt(3, "08:30"),
            "pickup_area_id": area,
            "pickup_address": "10 Jalan Maju",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(vp.status, StatusCode::OK, "{}", vp.json);
    let pid = vp.json["id"].as_i64().unwrap();

    // Cell leader cannot see or manage visitor passes on events they did not create
    let list_cl = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}/visitors"),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(list_cl.status, StatusCode::OK, "{}", list_cl.json);
    assert_eq!(list_cl.json.as_array().unwrap().len(), 0);
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/visitor-passes/{pid}/reissue"),
            Some(&cell_leader),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/visitor-passes/{pid}/revoke"),
            Some(&cell_leader),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Global transport staff CAN see and reissue
    let list_tr = call(
        &app,
        Method::GET,
        &format!("/api/events/{ev_id}/visitors"),
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(list_tr.status, StatusCode::OK, "{}", list_tr.json);
    assert_eq!(list_tr.json.as_array().unwrap().len(), 1);
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/visitor-passes/{pid}/reissue"),
            Some(&transport),
            None
        )
        .await
        .status,
        StatusCode::OK
    );

    // Built-in visitor group cannot be deleted
    let visitor_group: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE is_visitor=1 LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let del_vg = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/groups/{visitor_group}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(del_vg.status, StatusCode::BAD_REQUEST, "{}", del_vg.json);
    assert_eq!(del_vg.json["code"], "CANNOT_DELETE_ALL_MEMBERS_GROUP");
}

#[tokio::test]
async fn totp_enable_already_enabled_and_disabled_account_auth_blocks() {
    use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;
    use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    // 1. Setup + enable TOTP -> second /totp/enable returns TWO_FACTOR_ALREADY_ENABLED; stale session returns RECENT_SIGN_IN_REQUIRED
    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert_eq!(setup_res.status, StatusCode::OK, "{}", setup_res.json);
    let secret = setup_res.json["secret"].as_str().unwrap().to_string();
    let code = totp_code(&secret, "member@example.org");
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/totp/enable",
            Some(&member),
            Some(json!({ "code": code }))
        )
        .await
        .status,
        StatusCode::OK
    );

    let reenable = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&member),
        Some(json!({ "code": totp_code(&secret, "member@example.org") })),
    )
    .await;
    assert_eq!(
        reenable.status,
        StatusCode::BAD_REQUEST,
        "{}",
        reenable.json
    );
    assert_eq!(reenable.json["code"], "TWO_FACTOR_ALREADY_ENABLED");

    let old_time = (grace_ride::util::local_now() - chrono::Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query("UPDATE sessions SET created_at=$1 WHERE user_id=$2")
        .bind(&old_time)
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let stale_enable = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&member),
        Some(json!({ "code": "000000" })),
    )
    .await;
    assert_eq!(
        stale_enable.status,
        StatusCode::FORBIDDEN,
        "{}",
        stale_enable.json
    );
    assert_eq!(stale_enable.json["code"], "RECENT_SIGN_IN_REQUIRED");

    // Refresh session for passkey registration
    let member = login(&st, "member@example.org").await;
    let pk_start = call(
        &app,
        Method::POST,
        "/api/auth/passkey/register/start",
        Some(&member),
        None,
    )
    .await;
    let reg_opts: CreationChallengeResponse =
        serde_json::from_value(pk_start.json["options"].clone()).unwrap();
    let mut soft = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = Url::parse(ORIGIN).unwrap();
    let reg_cred = soft.do_registration(origin.clone(), reg_opts).unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/passkey/register/finish",
            Some(&member),
            Some(json!({ "challenge_id": pk_start.json["challenge_id"], "credential": reg_cred }))
        )
        .await
        .status,
        StatusCode::OK
    );

    // Prepare in-flight passkey login challenge BEFORE disable (keep recovery in second step because approve_recovery deletes passkeys)
    let ls = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({ "email": "member@example.org" })),
    )
    .await;
    let auth_opts: RequestChallengeResponse =
        serde_json::from_value(ls.json["options"].clone()).unwrap();
    let assertion = soft.do_authentication(origin, auth_opts).unwrap();
    let ch_id = ls.json["challenge_id"].as_str().unwrap().to_string();

    // Disable member -> deletes auth_challenges and rejects passkey login
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
    let ch_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_challenges WHERE user_id=$1")
        .bind(member_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(ch_left, 0);
    let lf = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(json!({ "challenge_id": ch_id, "credential": assertion })),
    )
    .await;
    assert_ne!(
        lf.status,
        StatusCode::OK,
        "disabled user finished passkey login: {}",
        lf.json
    );

    // Re-enable temporarily -> issue recovery invite -> disable -> recovery invite expired AND status guard blocks AUTH_DISABLED
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{member_id}"),
            Some(&admin),
            Some(json!({ "roles": ["member"], "status": "active" }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            "/api/auth/recovery-request",
            None,
            Some(json!({ "email": "member@example.org", "reason": "Lost" }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let rid: i64 = sqlx::query_scalar("SELECT id FROM recovery_requests WHERE email='member@example.org' ORDER BY id DESC LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let app_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&admin),
        None,
    )
    .await;
    let rec_tok = app_res.json["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string();

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
    let rec_use = call(
        &app,
        Method::POST,
        &format!("/api/invites/{rec_tok}/accept"),
        None,
        Some(json!({ "terms": true })),
    )
    .await;
    assert_eq!(rec_use.status, StatusCode::BAD_REQUEST, "{}", rec_use.json);
    assert_eq!(rec_use.json["code"], "INVITE_EXPIRED");

    // Simulate race where recovery expiry wasn't updated before accept -> AuthDisabled
    sqlx::query(
        "UPDATE invites SET expires_at=$1 WHERE kind='recovery' AND user_id=$2 AND used_at IS NULL",
    )
    .bind(grace_ride::util::in_days(1))
    .bind(member_id)
    .execute(&st.db)
    .await
    .unwrap();
    let rec_race = call(
        &app,
        Method::POST,
        &format!("/api/invites/{rec_tok}/accept"),
        None,
        Some(json!({ "terms": true })),
    )
    .await;
    assert_eq!(rec_race.status, StatusCode::FORBIDDEN, "{}", rec_race.json);
    assert_eq!(rec_race.json["code"], "AUTH_DISABLED");
}

#[tokio::test]
async fn profile_photo_mime_readiness_bounds_vapid_settings_and_saved_places_cache() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let area = first_area_id(&st).await;

    // 1. PDF upload rejected as profile photo
    let pdf_id = upload_pdf(&app, &member).await;
    let bad_photo = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({ "photo_file_id": pdf_id })),
    )
    .await;
    assert_eq!(
        bad_photo.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_photo.json
    );
    assert_eq!(bad_photo.json["code"], "INVALID_PHOTO");

    // 2. readiness_prompt_mins bounds and audit logging
    for bad_mins in [-5, 181] {
        let r = call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&member),
            Some(json!({ "readiness_prompt_mins": bad_mins })),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::BAD_REQUEST,
            "mins={bad_mins}: {}",
            r.json
        );
    }
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/me",
            Some(&member),
            Some(json!({ "readiness_prompt_mins": 30 }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let detail: String = sqlx::query_scalar(
        "SELECT detail FROM audit_logs WHERE action='profile.update' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(detail.contains("readiness_prompt"), "{detail}");

    // 3. VAPID public key cannot be replaced without private key
    let push_cfg = call(&app, Method::GET, "/api/push/config", Some(&admin), None).await;
    assert_eq!(push_cfg.status, StatusCode::OK, "{}", push_cfg.json);
    let orig_pub = push_cfg.json["vapid_public_key"]
        .as_str()
        .unwrap()
        .to_string();

    let corrupt = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "vapid_public_key": "BOGUS_NOT_A_P256_KEY" })),
    )
    .await;
    assert_eq!(corrupt.status, StatusCode::BAD_REQUEST, "{}", corrupt.json);

    // Sending unchanged existing vapid_public_key continues to succeed
    let keep = call(
        &app,
        Method::PUT,
        "/api/admin/settings/alerts",
        Some(&admin),
        Some(json!({ "vapid_public_key": orig_pub })),
    )
    .await;
    assert_eq!(keep.status, StatusCode::OK, "{}", keep.json);
    let priv_exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM app_secrets WHERE name='vapid_private_key'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(priv_exists, 1);

    // 4. Saved places geocode cache lookup normalizes mixed-case addresses
    sqlx::query(
        "INSERT INTO geocode_cache (query, lat, lng, approx, updated_at)
         VALUES ('12 jalan ss 2/24, petaling jaya', 3.118, 101.621, FALSE, $1) ON CONFLICT (query) DO NOTHING",
    )
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    let sp = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({ "label": "Home", "area_id": area, "address": "12 Jalan SS 2/24, Petaling Jaya" })),
    )
    .await;
    assert_eq!(sp.status, StatusCode::OK, "{}", sp.json);
    let sp_id = sp.json["id"].as_i64().unwrap();
    let (lat, lng): (Option<f64>, Option<f64>) =
        sqlx::query_as("SELECT lat, lng FROM saved_places WHERE id=$1")
            .bind(sp_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!((lat, lng), (Some(3.118), Some(101.621)));
}

#[tokio::test]
async fn settlement_overflow_reset_proof_and_recurring_schedule_edit() {
    use chrono::Datelike;

    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let all_group = all_group_id(&st).await;
    let area = first_area_id(&st).await;

    let ev = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Badminton Settlement",
            "kind": "badminton",
            "starts_at": future_dt(2, "20:00"),
            "location": "Hall",
            "attendance_enabled": false,
            "capacity": 20,
            "group_ids": [all_group]
        })),
    )
    .await;
    let ev_id = ev.json["id"].as_i64().unwrap();

    // 1. Settlement i64 overflow returns 400 INVALID_ACTION instead of 500 panic
    let overflow = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/settlements"),
        Some(&admin),
        Some(json!({
            "title": "Overflow Bill",
            "split_mode": "equal",
            "payee_name": "Admin",
            "payee_account": "123456",
            "items": [
                { "label": "A", "amount_cents": i64::MAX },
                { "label": "B", "amount_cents": 100 }
            ],
            "participants": [{ "user_id": member_id }]
        })),
    )
    .await;
    assert_eq!(
        overflow.status,
        StatusCode::BAD_REQUEST,
        "{}",
        overflow.json
    );
    assert_eq!(overflow.json["code"], "INVALID_ACTION");

    // 2. Verify action='reset' clears proof_file_id & txn_ref so member can submit on right bill
    let mk_bill = |title: &'static str| {
        let app = app.clone();
        let admin = admin.clone();
        async move {
            let s = call(
                &app,
                Method::POST,
                &format!("/api/events/{ev_id}/settlements"),
                Some(&admin),
                Some(json!({
                    "title": title,
                    "split_mode": "equal",
                    "payee_name": "Admin",
                    "payee_account": "123456",
                    "items": [{ "label": "Court", "amount_cents": 1000 }],
                    "participants": [{ "user_id": admin_id }, { "user_id": member_id }]
                })),
            )
            .await;
            let sid = s.json["id"].as_i64().unwrap();
            assert_eq!(
                call(
                    &app,
                    Method::POST,
                    &format!("/api/settlements/{sid}/publish"),
                    Some(&admin),
                    None
                )
                .await
                .status,
                StatusCode::OK
            );
            sid
        }
    };
    let s1 = mk_bill("Wrong Bill").await;
    let s2 = mk_bill("Right Bill").await;
    let sh1: i64 = sqlx::query_scalar(
        "SELECT id FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2",
    )
    .bind(s1)
    .bind(member_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let sh2: i64 = sqlx::query_scalar(
        "SELECT id FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2",
    )
    .bind(s2)
    .bind(member_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let receipt = upload_png(&app, &member).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/shares/{sh1}/proof"),
            Some(&member),
            Some(json!({ "proof_file_id": receipt, "txn_ref": "REF-RESET-1", "paid_date": &future_dt(0, "00:00")[..10] }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/shares/{sh1}/verify"),
            Some(&admin),
            Some(json!({ "action": "reset", "note": "wrong bill" }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let reuse = call(
        &app,
        Method::POST,
        &format!("/api/shares/{sh2}/proof"),
        Some(&member),
        Some(json!({ "proof_file_id": receipt, "txn_ref": "REF-RESET-1", "paid_date": &future_dt(0, "00:00")[..10] }))
    )
    .await;
    assert_eq!(
        reuse.status,
        StatusCode::OK,
        "reset failed to clear old proof: {}",
        reuse.json
    );

    // 3. Updating a recurring schedule's weekday resets last_generated_date so earlier date generates
    let today = grace_ride::util::today_local();
    let wd5 = (today + chrono::Duration::days(5))
        .weekday()
        .num_days_from_sunday() as i64;
    let wd4 = (today + chrono::Duration::days(4))
        .weekday()
        .num_days_from_sunday() as i64;
    let sched = call(
        &app,
        Method::POST,
        "/api/recurring-schedules",
        Some(&member),
        Some(json!({
            "kind": "passenger",
            "label": "Midweek",
            "weekday": wd5,
            "pickup_time": "07:45",
            "area_id": area,
            "pickup_address": "10 Jalan Maju",
            "destination": "Hall",
            "seats": 1,
            "lead_days": 7
        })),
    )
    .await;
    assert_eq!(sched.status, StatusCode::OK, "{}", sched.json);
    let sched_id = sched.json["id"].as_i64().unwrap();
    let g1 = call(
        &app,
        Method::POST,
        &format!("/api/recurring-schedules/{sched_id}/generate"),
        Some(&member),
        Some(json!({})),
    )
    .await;
    assert!(g1.json["ride_id"].as_i64().is_some(), "{}", g1.json);

    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/recurring-schedules/{sched_id}"),
            Some(&member),
            Some(json!({
                "kind": "passenger",
                "label": "Earlier Day",
                "weekday": wd4,
                "pickup_time": "07:45",
                "area_id": area,
                "pickup_address": "10 Jalan Maju",
                "destination": "Hall",
                "seats": 1,
                "lead_days": 7,
                "active": true
            }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let generated = grace_ride::routes::rides::process_recurring_schedules(&st.db)
        .await
        .unwrap();
    assert!(
        generated >= 1,
        "expected earlier weekday ride generation after edit"
    );
}

#[tokio::test]
async fn ride_lifecycle_cursors_search_escaping_csv_year_and_guides() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let member = login(&st, "member@example.org").await;
    let sophie = login(&st, "sophie@example.org").await;
    let all_group = all_group_id(&st).await;
    let area = first_area_id(&st).await;
    let driver_id = user_id(&st, "driver@example.org").await;

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
                "vehicle": { "model": "Myvi", "color": "White", "plate": "VZZ 101", "seats": 4, "wheelchair_ok": false, "child_seat": false }
            }))
        )
        .await
        .status,
        StatusCode::OK
    );

    // 1. driver_claim rejects wheelchair ride on non-wheelchair vehicle
    let wc_ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": future_dt(7, "09:00"),
            "pickup_area_id": area,
            "pickup_address": "10 Jalan Maju",
            "destination": "100% Literal Hall",
            "seats": 1,
            "assistance": "wheelchair"
        })),
    )
    .await;
    let wc_id = wc_ride.json["id"].as_i64().unwrap();
    let bad_claim = call(
        &app,
        Method::POST,
        &format!("/api/rides/{wc_id}/claim"),
        Some(&driver),
        Some(json!({})),
    )
    .await;
    assert_eq!(
        bad_claim.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_claim.json
    );

    // 2. driver_confirm_assignment checks spare_seats capacity at confirmation time
    let rt = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": future_dt(8, "09:00"),
            "pickup_area_id": area,
            "pickup_address": "10 Jalan Maju",
            "destination": "Church",
            "seats": 3,
            "return_pickup_at": future_dt(8, "12:00"),
            "return_pickup_area_id": area,
            "return_pickup_address": "Church",
            "return_destination": "10 Jalan Maju"
        })),
    )
    .await;
    let out_id = rt.json["id"].as_i64().unwrap();
    let ret_id = rt.json["return_ride_id"].as_i64().unwrap();

    // Reassign outbound to driver while cap=4, then driver drops spare_seats to 1 before confirming
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/reassign"),
            Some(&transport),
            Some(json!({ "driver_id": driver_id, "reason": "assign" }))
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
            Some(&driver),
            Some(json!({ "carpool_enabled": false, "spare_seats": 1, "area_ids": [area] }))
        )
        .await
        .status,
        StatusCode::OK
    );
    let overbook_confirm = call(
        &app,
        Method::POST,
        &format!("/api/rides/{out_id}/confirm-assignment"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(
        overbook_confirm.status,
        StatusCode::BAD_REQUEST,
        "{}",
        overbook_confirm.json
    );
    assert_eq!(overbook_confirm.json["code"], "NOT_ENOUGH_SEATS_SLOT");

    // Raise back to 4 seats, confirm, check staff gate notification name & post-pickup guard & no_show pair alert
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/driver/profile",
            Some(&driver),
            Some(json!({ "carpool_enabled": false, "spare_seats": 4, "area_ids": [area] }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/confirm-assignment"),
            Some(&driver),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    sqlx::query("UPDATE rides SET needs_attention=1 WHERE id=$1")
        .bind(out_id)
        .execute(&st.db)
        .await
        .unwrap();

    // Transport staff marks arrived-gate -> message names driver ("Brother ..." / "Test Driver"), not staff name
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/arrived-gate"),
            Some(&transport),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let (drv_name, tr_name): (String, String) = sqlx::query_as(
        "SELECT (SELECT name FROM users WHERE id=$1), (SELECT name FROM users WHERE email='transport@example.org')",
    )
    .bind(driver_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let gate_body: String = sqlx::query_scalar(
        "SELECT body FROM notifications WHERE user_id=(SELECT passenger_id FROM rides WHERE id=$1) AND link=$2 ORDER BY id DESC LIMIT 1",
    )
    .bind(out_id)
    .bind(format!("/rides/{out_id}"))
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(
        gate_body.contains(&drv_name) && !gate_body.contains(&tr_name),
        "{gate_body}"
    );

    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{out_id}/no-show"),
            Some(&driver),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let out_att: i64 = sqlx::query_scalar("SELECT needs_attention FROM rides WHERE id=$1")
        .bind(out_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let ret_att: i64 = sqlx::query_scalar("SELECT needs_attention FROM rides WHERE id=$1")
        .bind(ret_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(out_att, 0);
    assert_eq!(ret_att, 1);

    // Post-pickup guard on arrived-gate and delay
    let live_ride = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&sophie),
        Some(json!({
            "pickup_at": future_dt(9, "15:00"),
            "pickup_area_id": area,
            "pickup_address": "10 Jalan Maju",
            "destination": "Church",
            "seats": 1
        })),
    )
    .await;
    let live_id = live_ride.json["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{live_id}/claim"),
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
            &format!("/api/rides/{live_id}/respond"),
            Some(&sophie),
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
            &format!("/api/rides/{live_id}/approve"),
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
            &format!("/api/rides/{live_id}/start"),
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
            &format!("/api/rides/{live_id}/pickup"),
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
            &format!("/api/rides/{live_id}/arrived-gate"),
            Some(&driver),
            None
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/rides/{live_id}/driver-delay"),
            Some(&driver),
            Some(json!({ "delay_minutes": 5 }))
        )
        .await
        .status,
        StatusCode::CONFLICT
    );

    // Past rides pagination preserves same-timestamp tied rides across pages
    let sophie_id = user_id(&st, "sophie@example.org").await;
    let mut tied_ride_ids = Vec::new();
    for rider in ["Elder A", "Elder B"] {
        let r = call(
            &app,
            Method::POST,
            "/api/rides",
            Some(&sophie),
            Some(json!({
                "pickup_at": future_dt(10, "10:00"),
                "pickup_area_id": area,
                "pickup_address": "10 Jalan Maju",
                "destination": "Church",
                "seats": 1,
                "is_proxy": true,
                "rider_name": rider,
                "rider_phone": "012-1112222"
            })),
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        let rid = r.json["id"].as_i64().unwrap();
        sqlx::query("UPDATE rides SET status='completed', pickup_at='2026-01-10T09:00', completed_at='2026-01-10T10:00' WHERE id=$1")
            .bind(rid)
            .execute(&st.db)
            .await
            .unwrap();
        tied_ride_ids.push(rid);
    }
    let mut seen_rides = Vec::new();
    let mut r_before: Option<String> = None;
    loop {
        let qs = match &r_before {
            Some(c) => format!(
                "/api/rides?scope=past&limit=1&before={}",
                c.replace('|', "%7C")
            ),
            None => "/api/rides?scope=past&limit=1".to_string(),
        };
        let page = call(&app, Method::GET, &qs, Some(&sophie), None).await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.json);
        for item in page.json["items"].as_array().unwrap() {
            seen_rides.push(item["id"].as_i64().unwrap());
        }
        r_before = page.json["next_before"].as_str().map(str::to_string);
        if r_before.is_none() {
            break;
        }
    }
    for rid in tied_ride_ids {
        assert!(
            seen_rides.contains(&rid),
            "missing tied past ride {rid} for user {sophie_id}: {seen_rides:?}"
        );
    }

    // 3. Past events pagination cursor tie-breaking & future published isolation, plus past rides tie-breaking
    let same_past = format!(
        "{}T10:00",
        (grace_ride::util::today_local() - chrono::Duration::days(3)).format("%Y-%m-%d")
    );
    let mut tied_ids = Vec::new();
    for title in ["Past Tie 1", "Past Tie 2", "Past Tie 3"] {
        let e = call(
            &app,
            Method::POST,
            "/api/events",
            Some(&admin),
            Some(json!({ "title": title, "kind": "other", "starts_at": same_past, "location": "Hall", "group_ids": [all_group] })),
        )
        .await;
        tied_ids.push(e.json["id"].as_i64().unwrap());
    }
    let future_fin = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({ "title": "Early Finished", "kind": "other", "starts_at": future_dt(6, "10:00"), "location": "Hall", "group_ids": [all_group] })),
    )
    .await;
    let future_fin_id = future_fin.json["id"].as_i64().unwrap();
    sqlx::query("UPDATE events SET status='finished' WHERE id=$1")
        .bind(future_fin_id)
        .execute(&st.db)
        .await
        .unwrap();
    let future_pub = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({ "title": "Future Live", "kind": "other", "starts_at": future_dt(5, "10:00"), "location": "Hall", "group_ids": [all_group] })),
    )
    .await;
    let future_pub_id = future_pub.json["id"].as_i64().unwrap();

    // Repeated withdraw only notifies organiser once
    let mjg = join_as_group(&app, &member, future_pub_id).await;
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{future_pub_id}/signup"),
            Some(&member),
            Some(json!({ "as_group_id": mjg }))
        )
        .await
        .status,
        StatusCode::OK
    );
    sqlx::query("DELETE FROM notifications")
        .execute(&st.db)
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{future_pub_id}/withdraw"),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Method::POST,
            &format!("/api/events/{future_pub_id}/withdraw"),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let w_notifs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE link=$1")
        .bind(format!("/events/{future_pub_id}"))
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(w_notifs, 1);

    let mut seen_past = Vec::new();
    let mut before: Option<String> = None;
    loop {
        let qs = match &before {
            Some(c) => format!(
                "/api/events?scope=past&limit=2&before={}",
                c.replace('|', "%7C")
            ),
            None => "/api/events?scope=past&limit=2".to_string(),
        };
        let page = call(&app, Method::GET, &qs, Some(&member), None).await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.json);
        for item in page.json["items"].as_array().unwrap() {
            seen_past.push(item["id"].as_i64().unwrap());
        }
        before = page.json["next_before"].as_str().map(str::to_string);
        if before.is_none() {
            break;
        }
    }
    for tid in tied_ids {
        assert!(
            seen_past.contains(&tid),
            "missing tied past event {tid} in {seen_past:?}"
        );
    }
    assert!(seen_past.contains(&future_fin_id), "{seen_past:?}");
    assert!(
        !seen_past.contains(&future_pub_id),
        "future published event leaked into past: {seen_past:?}"
    );

    // 4. ILIKE literal %, CSV leading tab neutralization, year 9999 bounds, guide /api/ link block
    let literal_rides = call(
        &app,
        Method::GET,
        "/api/admin/rides?q=100%25&page=1",
        Some(&transport),
        None,
    )
    .await;
    assert_eq!(literal_rides.status, StatusCode::OK);
    let r_items = literal_rides.json["items"].as_array().unwrap();
    assert!(
        r_items
            .iter()
            .all(|r| r["destination"].as_str().unwrap_or("").contains("100%")),
        "{r_items:?}"
    );
    assert!(!r_items.is_empty());

    let literal_users = call(
        &app,
        Method::GET,
        "/api/admin/users?q=50%25&page=1",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(literal_users.status, StatusCode::OK);
    assert_eq!(literal_users.json["items"].as_array().unwrap().len(), 0);

    grace_ride::util::audit(&st.db, None, "test.csv_tab", "user", None, "\t=cmd").await;
    let (_, _, csv_body) = call_raw(
        &app,
        Method::GET,
        "/api/admin/audit/export?action=test.csv_tab",
        Some(&admin),
    )
    .await;
    assert!(csv_body.contains("'\t=cmd"), "{csv_body}");

    let y9999_m = call(
        &app,
        Method::GET,
        "/api/reports/monthly?month=9999-12",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(y9999_m.status, StatusCode::BAD_REQUEST);
    let y9999_d = call(
        &app,
        Method::GET,
        "/api/admin/reports/demand?month=9999-12",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(y9999_d.status, StatusCode::BAD_REQUEST);

    let bad_guide = call(
        &app,
        Method::POST,
        "/api/help/guides",
        Some(&admin),
        Some(json!({
            "slug": "bad-api-link",
            "title_en": "Bad",
            "title_zh": "Bad",
            "summary_en": "Bad",
            "summary_zh": "Bad",
            "audience": "member",
            "steps": [{ "title_en": "S1", "title_zh": "S1", "body_en": "B", "body_zh": "B", "link": "/api/admin/audit" }]
        })),
    )
    .await;
    assert_eq!(
        bad_guide.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_guide.json
    );
    assert_eq!(bad_guide.json["code"], "GUIDE_LINK_INVALID");
}
