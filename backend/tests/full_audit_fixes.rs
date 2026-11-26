mod common;

use axum::http::{header, Method, StatusCode};
use common::*;
use grace_ride::i18n::Lang;
use serde_json::json;

#[test]
fn accept_language_q_weight_parsing() {
    assert_eq!(
        Lang::parse_accept_language("en;q=0.5, zh-Hans;q=0.9"),
        Some(Lang::Zh)
    );
    assert_eq!(
        Lang::parse_accept_language("zh;q=0.2, en-US;q=0.8"),
        Some(Lang::En)
    );
    assert_eq!(
        Lang::parse_accept_language("fr, en;q=0.7, zh;q=0.4"),
        Some(Lang::En)
    );
}

#[tokio::test]
async fn audit_fixes_integration() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let driver2 = login(&st, "driver2@example.org").await;

    // 1. Audience `is_all` driving + groupless driver has empty drive_group_ids on plan board.
    let driver2_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email='driver2@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    sqlx::query("DELETE FROM group_members WHERE user_id=$1")
        .bind(driver2_id)
        .execute(&st.db)
        .await
        .unwrap();

    let area_id: i64 = sqlx::query_scalar("SELECT id FROM areas ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let ride_res = call(
        &app,
        Method::POST,
        "/api/rides",
        Some(&member),
        Some(json!({
            "pickup_at": tomorrow("09:00"),
            "pickup_area_id": area_id,
            "pickup_address": "12 Jalan SS2",
            "destination": "Grace Church",
            "seats": 1
        })),
    )
    .await;
    assert_eq!(ride_res.status, StatusCode::OK, "{}", ride_res.json);
    let ride_id = ride_res.json["id"].as_i64().unwrap();
    let ev_id: i64 = sqlx::query_scalar("SELECT event_id FROM rides WHERE id=$1")
        .bind(ride_id)
        .fetch_one(&st.db)
        .await
        .unwrap();

    let plan = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{ev_id}/plan"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(plan.status, StatusCode::OK, "{}", plan.json);
    let d2_plan = plan.json["drivers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == driver2_id)
        .unwrap();
    assert!(
        d2_plan["drive_group_ids"].as_array().unwrap().is_empty(),
        "{d2_plan}"
    );

    let claim = call(
        &app,
        Method::POST,
        &format!("/api/rides/{ride_id}/claim"),
        Some(&driver2),
        None,
    )
    .await;
    assert_eq!(claim.status, StatusCode::OK, "{}", claim.json);

    // 2. Driver vehicles: duplicate plate update conflicts (409); deleting active vehicle promotes next.
    let prof = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(prof.status, StatusCode::OK, "{}", prof.json);
    let v1_id = prof.json["vehicles"][0]["id"].as_i64().unwrap();
    let v1_plate = prof.json["vehicles"][0]["plate"]
        .as_str()
        .unwrap()
        .to_string();

    let add_v2 = call(
        &app,
        Method::POST,
        "/api/driver/vehicles",
        Some(&driver),
        Some(json!({
            "model": "Proton Saga",
            "color": "Silver",
            "plate": "WXY9876",
            "seats": 4,
            "vehicle_type": "standard"
        })),
    )
    .await;
    assert_eq!(add_v2.status, StatusCode::OK, "{}", add_v2.json);
    let v2_id = add_v2.json["id"].as_i64().unwrap();

    let dup_plate = call(
        &app,
        Method::PUT,
        &format!("/api/driver/vehicles/{v2_id}"),
        Some(&driver),
        Some(json!({
            "model": "Proton Saga",
            "color": "Silver",
            "plate": v1_plate,
            "seats": 4,
            "vehicle_type": "standard"
        })),
    )
    .await;
    assert_eq!(dup_plate.status, StatusCode::CONFLICT, "{}", dup_plate.json);

    let del_active = call(
        &app,
        Method::DELETE,
        &format!("/api/driver/vehicles/{v1_id}"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(del_active.status, StatusCode::OK, "{}", del_active.json);
    let prof_after = call(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&driver),
        None,
    )
    .await;
    assert_ne!(
        prof_after.json["active_vehicle_id"], v1_id,
        "{}",
        prof_after.json
    );
    assert!(
        prof_after.json["vehicles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["is_active"] == true && v["id"] != v1_id),
        "{}",
        prof_after.json
    );

    // 3. Saved places: clear_coords wipes lat/lng explicitly.
    let sp_create = call(
        &app,
        Method::POST,
        "/api/saved-places",
        Some(&member),
        Some(json!({
            "label": "Audit Pin",
            "area_id": area_id,
            "address": "88 Jalan Gasing",
            "lat": 3.105,
            "lng": 101.645
        })),
    )
    .await;
    assert_eq!(sp_create.status, StatusCode::OK, "{}", sp_create.json);
    let sp_id = sp_create.json["id"].as_i64().unwrap();

    let sp_clear = call(
        &app,
        Method::PUT,
        &format!("/api/saved-places/{sp_id}"),
        Some(&member),
        Some(json!({
            "label": "Audit Pin",
            "area_id": area_id,
            "address": "88 Jalan Gasing",
            "clear_coords": true
        })),
    )
    .await;
    assert_eq!(sp_clear.status, StatusCode::OK, "{}", sp_clear.json);

    let sp_list = call(&app, Method::GET, "/api/saved-places", Some(&member), None).await;
    let cleared = sp_list
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == sp_id)
        .unwrap();
    assert!(
        cleared["lat"].is_null() && cleared["lng"].is_null(),
        "{cleared}"
    );

    // 4. Invites revoke unknown id returns 404.
    let del_missing = call(
        &app,
        Method::DELETE,
        "/api/admin/invites/999999",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(
        del_missing.status,
        StatusCode::NOT_FOUND,
        "{}",
        del_missing.json
    );

    // 5. Reimbursements kind restriction, self-review block, payout batch CSV BOM + escaping.
    let receipt_id = upload_png(&app, &driver).await;
    let bad_kind = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&driver),
        Some(json!({
            "event_id": ev_id,
            "amount_cents": 2500,
            "description": "Grab ride",
            "receipt_file_id": receipt_id,
            "payee_name": "Driver",
            "payee_phone": "+60123456789",
            "kind": "bogus"
        })),
    )
    .await;
    assert_eq!(
        bad_kind.status,
        StatusCode::BAD_REQUEST,
        "{}",
        bad_kind.json
    );

    let admin_receipt = upload_png(&app, &admin).await;
    let admin_claim = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&admin),
        Some(json!({
            "event_id": ev_id,
            "amount_cents": 1500,
            "description": "|cmd_self",
            "receipt_file_id": admin_receipt,
            "payee_name": "Admin",
            "payee_phone": "+60123456780",
            "kind": "grab"
        })),
    )
    .await;
    assert_eq!(admin_claim.status, StatusCode::OK, "{}", admin_claim.json);
    let admin_claim_id = admin_claim.json["id"].as_i64().unwrap();
    let self_rev = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{admin_claim_id}/review"),
        Some(&admin),
        Some(json!({"action": "approve"})),
    )
    .await;
    assert_eq!(self_rev.status, StatusCode::FORBIDDEN, "{}", self_rev.json);

    let d_claim = call(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&driver),
        Some(json!({
            "event_id": ev_id,
            "amount_cents": 3200,
            "description": "%formula",
            "receipt_file_id": receipt_id,
            "payee_name": "|Driver",
            "payee_phone": "+60123456789",
            "kind": "grab"
        })),
    )
    .await;
    assert_eq!(d_claim.status, StatusCode::OK, "{}", d_claim.json);
    let d_claim_id = d_claim.json["id"].as_i64().unwrap();

    let approve = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{d_claim_id}/review"),
        Some(&admin),
        Some(json!({"action": "approve"})),
    )
    .await;
    assert_eq!(approve.status, StatusCode::OK, "{}", approve.json);

    let batch = call(
        &app,
        Method::POST,
        "/api/reimbursements/batches",
        Some(&finance),
        Some(json!({"claim_ids": [d_claim_id]})),
    )
    .await;
    assert_eq!(batch.status, StatusCode::OK, "{}", batch.json);
    let batch_id = batch.json["id"].as_i64().unwrap();

    let (csv_status, csv_headers, csv_body) = call_raw(
        &app,
        Method::GET,
        &format!("/api/reimbursements/batches/{batch_id}/csv"),
        Some(&finance),
    )
    .await;
    assert_eq!(csv_status, StatusCode::OK);
    assert!(csv_body.starts_with('\u{FEFF}'));
    assert!(csv_body.contains("'|Driver"), "{csv_body}");
    assert!(csv_body.contains("'%formula"), "{csv_body}");
    let cc = csv_headers
        .get(header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(cc.contains("no-store"), "cache-control: {cc}");

    // 6. Help ticket reopen (no-op on open) and claim guard on closed ticket.
    let tk = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject": "Need pickup note", "body": "Please call at gate"})),
    )
    .await;
    assert_eq!(tk.status, StatusCode::OK, "{}", tk.json);
    let tk_id = tk.json["id"].as_i64().unwrap();
    let reopen_open = call(
        &app,
        Method::POST,
        &format!("/api/help/{tk_id}/reopen"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(reopen_open.status, StatusCode::OK, "{}", reopen_open.json);
    let (reopen_audits,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='help.reopen' AND entity_id=$1",
    )
    .bind(tk_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(reopen_audits, 0);

    sqlx::query("UPDATE help_tickets SET status='closed' WHERE id=$1")
        .bind(tk_id)
        .execute(&st.db)
        .await
        .unwrap();
    let claim_closed = call(
        &app,
        Method::POST,
        &format!("/api/help/{tk_id}/claim"),
        Some(&admin),
        Some(json!({"claim": true})),
    )
    .await;
    assert_eq!(
        claim_closed.status,
        StatusCode::CONFLICT,
        "{}",
        claim_closed.json
    );
}
