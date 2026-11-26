//! End-to-end regression tests for all 8 security audit remediations (F-01 through F-08).

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use base64::Engine as _;
use common::*;
use grace_ride::state::Config;
use http_body_util::BodyExt;
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};
use serde_json::json;
use tower::ServiceExt;

const P256_SPKI_PREFIX: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

fn test_ec_key() -> (EcdsaKeyPair, SystemRandom, String) {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
    let pair =
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng).unwrap();
    let mut spki = Vec::with_capacity(91);
    spki.extend_from_slice(&P256_SPKI_PREFIX);
    spki.extend_from_slice(pair.public_key().as_ref());
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(spki);
    (pair, rng, b64)
}

#[tokio::test]
async fn f01_rbac_unheld_sensitive_and_finance_roles_cannot_be_created_edited_or_assigned() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let president = login(&st, "president@example.org").await;
    let vice = login(&st, "vicepresident@example.org").await;
    let root = login(&st, "superadmin@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;

    // 1. Admin cannot create custom unranked roles carrying Finance or unheld sensitive perms.
    for perm in [
        "admin_reimb_pay.view",
        "admin_reimb_pay.update",
        "admin_bills.view",
    ] {
        let r = call(
            &app,
            Method::POST,
            "/api/admin/roles",
            Some(&admin),
            Some(json!({ "name": format!("Shadow {perm}"), "permissions": [perm] })),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{perm}: {}", r.json);
        assert_eq!(r.json["code"], "CANNOT_GRANT_UNHELD_PERMISSION");
    }

    // 2. Admin cannot edit an existing unranked role (`driver` or custom) to add Finance perms.
    let edit_driver_fin = call(
        &app,
        Method::PUT,
        "/api/admin/roles/driver",
        Some(&admin),
        Some(json!({ "permissions": ["drive.view", "admin_reimb_pay.update"] })),
    )
    .await;
    assert_eq!(edit_driver_fin.status, StatusCode::FORBIDDEN);
    assert_eq!(
        edit_driver_fin.json["code"],
        "CANNOT_GRANT_UNHELD_PERMISSION"
    );

    // Regression check: admin can still edit non-sensitive member perms (`drive.*`) on `driver` and assign `driver`.
    let edit_driver_ok = call(
        &app,
        Method::PUT,
        "/api/admin/roles/driver",
        Some(&admin),
        Some(json!({ "permissions": ["drive.view", "drive.create"] })),
    )
    .await;
    assert_eq!(
        edit_driver_ok.status,
        StatusCode::OK,
        "{}",
        edit_driver_ok.json
    );
    let assign_driver_ok = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}"),
        Some(&admin),
        Some(json!({ "roles": ["driver"], "status": "active" })),
    )
    .await;
    assert_eq!(
        assign_driver_ok.status,
        StatusCode::OK,
        "{}",
        assign_driver_ok.json
    );

    // 3. President & Vice-President cannot mint custom Finance roles or unheld `admin_fares.update` / `admin_audit.view`.
    for (who, cookie) in [("president", &president), ("vice_president", &vice)] {
        for perm in [
            "admin_reimb_pay.view",
            "admin_reimb_pay.update",
            "admin_bills.view",
            "admin_fares.update",
            "admin_audit.view",
        ] {
            let r = call(
                &app,
                Method::POST,
                "/api/admin/roles",
                Some(cookie),
                Some(json!({ "name": format!("{who}-{perm}"), "permissions": [perm] })),
            )
            .await;
            assert_eq!(r.status, StatusCode::FORBIDDEN, "{who} {perm}: {}", r.json);
            assert_eq!(r.json["code"], "CANNOT_GRANT_UNHELD_PERMISSION");
        }
    }

    // 4. Superadmin creates a custom Finance role; admin cannot assign it to another user OR themselves.
    let custom_fin = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&root),
        Some(json!({
            "name": "Special Treasurer",
            "permissions": ["admin_reimb_pay.view", "admin_reimb_pay.update", "admin_bills.view"]
        })),
    )
    .await;
    assert_eq!(custom_fin.status, StatusCode::OK, "{}", custom_fin.json);
    let fin_key = custom_fin.json["key"].as_str().unwrap();

    let give_other = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}"),
        Some(&admin),
        Some(json!({ "roles": [fin_key], "status": "active" })),
    )
    .await;
    assert_eq!(
        give_other.status,
        StatusCode::FORBIDDEN,
        "{}",
        give_other.json
    );
    assert_eq!(give_other.json["code"], "CANNOT_GRANT_UNHELD_PERMISSION");

    let give_self = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{admin_id}"),
        Some(&admin),
        Some(json!({ "roles": ["admin", fin_key], "status": "active" })),
    )
    .await;
    assert_eq!(
        give_self.status,
        StatusCode::FORBIDDEN,
        "{}",
        give_self.json
    );
    assert_eq!(give_self.json["code"], "CANNOT_GRANT_SELF_FINANCE");
}

#[tokio::test]
async fn f02_recovery_respects_rank_admin_power_and_self_approval_across_approve_and_reject() {
    let (app, st) = setup().await;
    let secretary = login(&st, "secretary@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let president = login(&st, "president@example.org").await;
    let vice = login(&st, "vicepresident@example.org").await;

    async fn file_recovery(st: &grace_ride::state::AppState, email: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO recovery_requests (email, reason, status, created_at)
             VALUES ($1, 'lost phone', 'pending', app_now()) RETURNING id",
        )
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
    }

    let rid_pres = file_recovery(&st, "president@example.org").await;
    let rid_fin = file_recovery(&st, "finance@example.org").await;
    let rid_aud = file_recovery(&st, "auditor@example.org").await;
    let rid_root = file_recovery(&st, "superadmin@example.org").await;
    let rid_adm = file_recovery(&st, "admin@example.org").await;
    let rid_mem = file_recovery(&st, "member@example.org").await;

    // 1. Secretary cannot approve OR reject ranked leadership/finance/superadmin/admin accounts.
    for (rid, expected_code) in [
        (rid_pres, "RANK_TOO_LOW"),
        (rid_fin, "RANK_TOO_LOW"),
        (rid_aud, "RANK_TOO_LOW"),
        (rid_root, "SUPERADMIN_ONLY"),
        (rid_adm, "ADMIN_APPOINTMENT_RESTRICTED"),
    ] {
        for action in ["approve", "reject"] {
            let r = call(
                &app,
                Method::POST,
                &format!("/api/admin/recovery/{rid}/{action}"),
                Some(&secretary),
                None,
            )
            .await;
            assert_eq!(
                r.status,
                StatusCode::FORBIDDEN,
                "sec {action} {rid}: {}",
                r.json
            );
            assert_eq!(r.json["code"], expected_code);
        }
    }

    // Secretary CAN reject an unranked non-admin member's recovery request.
    let mem_reject = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid_mem}/reject"),
        Some(&secretary),
        None,
    )
    .await;
    assert_eq!(mem_reject.status, StatusCode::OK, "{}", mem_reject.json);

    // 2. Unranked admin cannot approve OR reject ranked leadership/finance/superadmin accounts.
    for (rid, expected_code) in [
        (rid_pres, "RANK_TOO_LOW"),
        (rid_fin, "RANK_TOO_LOW"),
        (rid_aud, "RANK_TOO_LOW"),
        (rid_root, "SUPERADMIN_ONLY"),
    ] {
        for action in ["approve", "reject"] {
            let r = call(
                &app,
                Method::POST,
                &format!("/api/admin/recovery/{rid}/{action}"),
                Some(&admin),
                None,
            )
            .await;
            assert_eq!(
                r.status,
                StatusCode::FORBIDDEN,
                "admin {action} {rid}: {}",
                r.json
            );
            assert_eq!(r.json["code"], expected_code);
        }
    }

    // 3. Target cannot self-approve their own recovery request.
    let self_app = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid_pres}/approve"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(self_app.status, StatusCode::FORBIDDEN, "{}", self_app.json);
    assert_eq!(self_app.json["code"], "SECOND_ADMIN_REQUIRED");

    // 4. President + Vice-President CAN two-person recover Finance.
    let s1 = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid_fin}/approve"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(s1.status, StatusCode::OK, "{}", s1.json);
    assert_eq!(s1.json["awaiting_second"], true);

    let s2 = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid_fin}/approve"),
        Some(&vice),
        None,
    )
    .await;
    assert_eq!(s2.status, StatusCode::OK, "{}", s2.json);
    assert!(s2.json["url"].is_string());
}

#[tokio::test]
async fn f03_post_event_mileage_enforces_claimable_participation_fare_review_and_dedup() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let root = login(&st, "superadmin@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let driver1 = login(&st, "driver@example.org").await;
    let driver1_id = user_id(&st, "driver@example.org").await;
    let driver2_id = user_id(&st, "driver2@example.org").await;
    let signup_only_id = user_id(&st, "amina@example.org").await;
    let passenger_id = user_id(&st, "member@example.org").await;

    // Ensure area_fares(1, 2) = 1500 cents.
    sqlx::query(
        "INSERT INTO area_fares (area_a_id, area_b_id, amount_cents, updated_at)
         VALUES (1, 2, 1500, app_now())
         ON CONFLICT (area_a_id, area_b_id) DO UPDATE SET amount_cents = 1500",
    )
    .execute(&st.db)
    .await
    .unwrap();

    // Create a non-claimable event first.
    let ev_no = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&admin),
        Some(json!({
            "title": "Non-claimable Service",
            "kind": "other",
            "starts_at": tomorrow("18:00"),
            "location": "Hall",
            "capacity": 20,
            "group_ids": [1],
            "area_id": 1,
            "mileage_claimable": false
        })),
    )
    .await;
    assert_eq!(ev_no.status, StatusCode::OK, "{}", ev_no.json);
    let ev_no_id = ev_no.json["id"].as_i64().unwrap();

    // Attempting post-event reimbursement on non-claimable event must fail with EVENT_MILEAGE_NOT_CLAIMABLE.
    let denied_non_claimable = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_no_id}/post-event-reimbursements"),
        Some(&admin),
        Some(json!({
            "claims": [{
                "driver_id": driver1_id,
                "trip_type": "direct_return",
                "destination_area_id": 2,
                "amount_cents": 1500
            }]
        })),
    )
    .await;
    assert_eq!(denied_non_claimable.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        denied_non_claimable.json["code"],
        "EVENT_MILEAGE_NOT_CLAIMABLE"
    );

    // Authorise mileage_claimable = true as superadmin.
    let ev_ok = call(
        &app,
        Method::POST,
        "/api/events",
        Some(&root),
        Some(json!({
            "title": "Claimable Service",
            "kind": "other",
            "starts_at": tomorrow("19:00"),
            "location": "Hall",
            "capacity": 20,
            "group_ids": [1],
            "area_id": 1,
            "mileage_claimable": true
        })),
    )
    .await;
    let ev_id = ev_ok.json["id"].as_i64().unwrap();

    // `signup_only_id` only offered to drive in event_signups, no assigned rides/visitors -> rejected.
    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, need_ride, offer_drive, offer_seats)
         VALUES ($1, $2, 'going', 0, 1, 3)
         ON CONFLICT (event_id, user_id) DO UPDATE SET status='going', offer_drive=1",
    )
    .bind(ev_id)
    .bind(signup_only_id)
    .execute(&st.db)
    .await
    .unwrap();
    let denied_offer_only = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/post-event-reimbursements"),
        Some(&admin),
        Some(json!({
            "claims": [{
                "driver_id": signup_only_id,
                "trip_type": "direct_return",
                "destination_area_id": 2,
                "amount_cents": 1500
            }]
        })),
    )
    .await;
    assert_eq!(denied_offer_only.status, StatusCode::BAD_REQUEST);
    assert_eq!(denied_offer_only.json["code"], "INVALID_ACTION");

    // Driver 1 drove an un-fared completed ride (mileage_receipt_needed = 1); Driver 2 also drove a ride.
    let (ride_d1,): (i64,) = sqlx::query_as(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, leg, status, mileage_receipt_needed, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 3, 'No Fare Area', 'Hall', 'outbound', 'completed', 1, app_now(), app_now())
         RETURNING id",
    )
    .bind(passenger_id)
    .bind(driver1_id)
    .bind(ev_id)
    .bind(tomorrow("18:30"))
    .fetch_one(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, leg, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 2, 'Area 2', 'Hall', 'outbound', 'completed', app_now(), app_now())",
    )
    .bind(passenger_id)
    .bind(driver2_id)
    .bind(ev_id)
    .bind(tomorrow("18:30"))
    .execute(&st.db)
    .await
    .unwrap();

    // Driver 1 gets exact fare (1500) -> auto-approved; Driver 2 gets custom override (2800 != 1500) -> pending review with requested_by = admin.
    let created = call(
        &app,
        Method::POST,
        &format!("/api/events/{ev_id}/post-event-reimbursements"),
        Some(&admin),
        Some(json!({
            "claims": [
                {
                    "driver_id": driver1_id,
                    "trip_type": "direct_return",
                    "destination_area_id": 2,
                    "amount_cents": 1500
                },
                {
                    "driver_id": driver2_id,
                    "trip_type": "supper",
                    "destination_area_id": 2,
                    "amount_cents": 2800,
                    "note": "Extra detour"
                }
            ]
        })),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    assert_eq!(created.json["auto_approved_count"], 1);
    assert_eq!(created.json["pending_review_count"], 1);

    let (d2_reimb_id, d2_status): (i64, String) =
        sqlx::query_as("SELECT id, status FROM reimbursements WHERE event_id=$1 AND user_id=$2")
            .bind(ev_id)
            .bind(driver2_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(d2_status, "pending");

    // Admin who submitted the custom amount cannot self-approve it.
    let self_rev = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{d2_reimb_id}/review"),
        Some(&admin),
        Some(json!({ "action": "approve", "note": "" })),
    )
    .await;
    assert_eq!(self_rev.status, StatusCode::FORBIDDEN, "{}", self_rev.json);
    assert_eq!(self_rev.json["code"], "CANNOT_REVIEW_OWN_REQUEST");

    // An independent reviewer (`transport`) can approve it.
    let other_rev = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{d2_reimb_id}/review"),
        Some(&transport),
        Some(json!({ "action": "approve", "note": "" })),
    )
    .await;
    assert_eq!(other_rev.status, StatusCode::OK, "{}", other_rev.json);

    // Double-claim check: Driver 1 already has a post-event mileage claim for this event;
    // trying to submit a second ride-based mileage claim via POST /api/reimbursements must fail.
    let receipt = upload_png(&app, &driver1).await;
    let double_claim = call_exact(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&driver1),
        Some(json!({
            "kind": "mileage",
            "ride_id": ride_d1,
            "amount_cents": 1200,
            "description": "Duplicate ride claim",
            "receipt_file_id": receipt,
            "payee_name": "Driver One",
            "payee_phone": "0123456789"
        })),
        Some(ORIGIN),
    )
    .await;
    assert_eq!(
        double_claim.status,
        StatusCode::BAD_REQUEST,
        "{}",
        double_claim.json
    );
    assert_eq!(double_claim.json["code"], "REIMBURSEMENT_ALREADY_CLAIMED");
}

#[tokio::test]
async fn f04_payout_batch_and_claim_enforce_creator_and_requester_separation_of_duties() {
    let (app, st) = setup().await;
    let finance = login(&st, "finance@example.org").await;
    let finance_id = user_id(&st, "finance@example.org").await;
    let transport = login(&st, "transport@example.org").await;
    let root = login(&st, "superadmin@example.org").await;
    let driver_id = user_id(&st, "driver@example.org").await;
    let area_id: i64 =
        sqlx::query_scalar("SELECT id FROM areas WHERE active=1 ORDER BY id LIMIT 1")
            .fetch_one(&st.db)
            .await
            .unwrap();

    // Give finance@ dual role with traffic_head for test fixture of requested_by == finance_id.
    sqlx::query("UPDATE users SET roles='finance,traffic_head' WHERE id=$1")
        .bind(finance_id)
        .execute(&st.db)
        .await
        .unwrap();

    let ev_id: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, starts_at, location, status, created_by, area_id)
         VALUES ('SoD Event', $1, 'Hall', 'published', $2, $3) RETURNING id",
    )
    .bind(tomorrow("19:00"))
    .bind(finance_id)
    .bind(area_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    let vis_id: i64 = sqlx::query_scalar(
        "INSERT INTO plan_visitors (event_id, name, seats, driver_id, created_by, created_at)
         VALUES ($1, 'Guest SoD', 1, $2, $3, app_now()) RETURNING id",
    )
    .bind(ev_id)
    .bind(driver_id)
    .bind(finance_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // `finance_id` requests special reimbursement (`requested_by = finance_id`), `transport` approves it (`reviewed_by = transport`).
    let req_res = call(
        &app,
        Method::POST,
        &format!("/api/admin/events/{ev_id}/plan/visitors/{vis_id}/reimbursement"),
        Some(&finance),
        Some(json!({ "amount_cents": 2000, "reason": "Far pickup" })),
    )
    .await;
    assert_eq!(req_res.status, StatusCode::OK, "{}", req_res.json);
    let claim_id = req_res.json["id"].as_i64().unwrap();

    let rev = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{claim_id}/review"),
        Some(&transport),
        Some(json!({ "action": "approve", "note": "" })),
    )
    .await;
    assert_eq!(rev.status, StatusCode::OK, "{}", rev.json);

    // 1. Requester (`finance_id`) cannot pay the individual claim they requested (`CANNOT_PAY_OWN_REQUEST`).
    let pay_own_req = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{claim_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-OWN-REQ" })),
    )
    .await;
    assert_eq!(
        pay_own_req.status,
        StatusCode::FORBIDDEN,
        "{}",
        pay_own_req.json
    );
    assert_eq!(pay_own_req.json["code"], "CANNOT_PAY_OWN_REQUEST");

    // 2. Superadmin creates a payout batch containing that claim; requester (`finance_id`) cannot pay that batch.
    let b1 = call(
        &app,
        Method::POST,
        "/api/reimbursements/batches",
        Some(&root),
        Some(json!({ "claim_ids": [claim_id] })),
    )
    .await;
    assert_eq!(b1.status, StatusCode::OK, "{}", b1.json);
    let b1_id = b1.json["id"].as_i64().unwrap();

    let batch_own_req = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{b1_id}/pay"),
        Some(&finance),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-B-REQ" })),
    )
    .await;
    assert_eq!(
        batch_own_req.status,
        StatusCode::FORBIDDEN,
        "{}",
        batch_own_req.json
    );
    assert_eq!(batch_own_req.json["code"], "CANNOT_PAY_OWN_REQUEST");

    // 3. Batch creator (`root`) cannot pay their own batch (`CANNOT_PAY_OWN_BATCH`).
    let batch_own_creator = call(
        &app,
        Method::POST,
        &format!("/api/reimbursements/batches/{b1_id}/pay"),
        Some(&root),
        Some(json!({ "payee_verified": true, "paid_ref": "FT-B-CREATOR" })),
    )
    .await;
    assert_eq!(
        batch_own_creator.status,
        StatusCode::FORBIDDEN,
        "{}",
        batch_own_creator.json
    );
    assert_eq!(batch_own_creator.json["code"], "CANNOT_PAY_OWN_BATCH");

    // 4. DB CHECK constraint rejects `paid_by = created_by` on `payout_batches`.
    let db_chk = sqlx::query(
        "UPDATE payout_batches SET status='paid', paid_by=created_by, paid_ref='DB-BYPASS' WHERE id=$1",
    )
    .bind(b1_id)
    .execute(&st.db)
    .await;
    assert!(
        db_chk.is_err(),
        "DB constraint must block paid_by == created_by"
    );

    // 5. DB CHECK constraint also blocks `paid_by = requested_by` on `reimbursements`.
    let db_req_chk = sqlx::query(
        "UPDATE reimbursements SET status='paid', paid_ref='REF-VIOL', paid_by=$1, paid_at=app_now() WHERE id=$2",
    )
    .bind(finance_id)
    .bind(claim_id)
    .execute(&st.db)
    .await;
    assert!(
        db_req_chk.is_err(),
        "DB constraint must block paid_by == requested_by"
    );

    // 6. Finance report flags any historical SoD violation where COALESCE(r.paid_by, b.paid_by) == r.requested_by as maker_checker_ok: false.
    let viol_batch: i64 = sqlx::query_scalar(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, paid_ref, created_by, paid_by, paid_at, created_at)
         VALUES ($1, 'Req=Payer Batch', 1000, 1, 'paid', 'REF-VIOL', $2, $3, app_now(), app_now()) RETURNING id",
    )
    .bind(&grace_ride::util::now()[..7])
    .bind(user_id(&st, "superadmin@example.org").await)
    .bind(finance_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query("UPDATE reimbursements SET batch_id=$1, paid_by=NULL WHERE id=$2")
        .bind(viol_batch)
        .bind(claim_id)
        .execute(&st.db)
        .await
        .unwrap();

    let fin_rep = call(
        &app,
        Method::GET,
        "/api/admin/reports/finance",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(fin_rep.status, StatusCode::OK, "{}", fin_rep.json);
    let batch_entry = fin_rep.json["reimbursement_payouts"]["batches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"].as_i64() == Some(viol_batch))
        .expect("viol_batch in finance report");
    assert_eq!(batch_entry["maker_checker_ok"], false, "{}", batch_entry);
}

#[tokio::test]
async fn f05_f06_biometric_attestation_strict_prod_and_locked_session_closes_zero_mfa_enrollment() {
    let dir = std::env::temp_dir().join(format!("gr-sec-prod-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_url = fresh_db().await;

    // 1. F-05: Config::validate() refuses REQUIRE_BIOMETRIC_ATTESTATION=true without Android cert fingerprints in prod.
    let mut prod_cfg = Config {
        dev_mode: false,
        database_url: db_url,
        upload_dir: dir.join("uploads"),
        static_dir: None,
        public_url: ORIGIN.into(),
        bind: "127.0.0.1:0".into(),
        max_live_drivers: 10,
        trust_proxy: false,
        chat_retention_days: 180,
        audit_retention_days: 730,
        session_timeout_mins: 15,
        json_logs: false,
        routing: Default::default(),
        db: grace_ride::state::DbConfig {
            max_connections: 5,
            acquire_timeout_secs: 60,
            ..Default::default()
        },
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
        turnstile: Default::default(),
        android_cert_sha256_fingerprints: Vec::new(),
        ios_app_ids: Vec::new(),
        require_biometric_attestation: true,
    };
    assert!(
        prod_cfg.validate().is_err(),
        "REQUIRE_BIOMETRIC_ATTESTATION=true with no Android cert fingerprints must fail validation"
    );

    prod_cfg.android_cert_sha256_fingerprints = vec![
        "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99".into(),
    ];
    prod_cfg.validate().unwrap();

    let prod_st = grace_ride::init(prod_cfg).await.unwrap();
    register_test_db(&prod_st);
    let prod_app = grace_ride::router(prod_st.clone());

    let uid: i64 = sqlx::query_scalar(
        "INSERT INTO users (name, email, phone, roles, status, webauthn_uuid, terms_accepted_at, terms_version)
         VALUES ('Prod Member', 'prod@example.org', '0129990001', '', 'active', $1, app_now(), 3)
         RETURNING id",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .fetch_one(&prod_st.db)
    .await
    .unwrap();
    let cookie = login(&prod_st, "prod@example.org").await;

    // 2. Unattested biometric registration fails in strict prod mode.
    let (pair, rng, pub_b64) = test_ec_key();
    let rs = call(
        &prod_app,
        Method::POST,
        "/api/auth/biometric/register/start",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(rs.status, StatusCode::OK, "{}", rs.json);
    let chal = rs.json["challenge"].as_str().unwrap();
    let msg = format!("graceride-biometric-v1:register:{chal}");
    let sig = pair.sign(&rng, msg.as_bytes()).unwrap();
    let reg_fail = call(
        &prod_app,
        Method::POST,
        "/api/auth/biometric/register/finish",
        Some(&cookie),
        Some(json!({
            "challenge_id": rs.json["challenge_id"],
            "public_key": pub_b64,
            "signature": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.as_ref()),
            "label": "Unattested Key",
        })),
    )
    .await;
    assert_eq!(
        reg_fail.status,
        StatusCode::BAD_REQUEST,
        "{}",
        reg_fail.json
    );
    assert_eq!(reg_fail.json["code"], "BIOMETRIC_REGISTRATION_FAILED");

    // 3. Even if a legacy `attested = false` row exists, login and reauth refuse it in strict prod mode.
    let mut spki = Vec::with_capacity(91);
    spki.extend_from_slice(&P256_SPKI_PREFIX);
    spki.extend_from_slice(pair.public_key().as_ref());
    let unattested_cred_id = "legacy_unattested_cred_0123456789";
    sqlx::query(
        "INSERT INTO biometric_credentials (user_id, cred_id, public_key, label, created_at, updated_at, attested)
         VALUES ($1, $2, $3, 'Legacy Unattested', app_now(), app_now(), FALSE)",
    )
    .bind(uid)
    .bind(unattested_cred_id)
    .bind(pair.public_key().as_ref())
    .execute(&prod_st.db)
    .await
    .unwrap();

    let login_start = call(
        &prod_app,
        Method::POST,
        "/api/auth/biometric/login/start",
        None,
        Some(json!({ "credential_id": unattested_cred_id })),
    )
    .await;
    assert_eq!(login_start.status, StatusCode::OK);
    let lchal = login_start.json["challenge"].as_str().unwrap();
    let lsig = pair
        .sign(
            &rng,
            format!("graceride-biometric-v1:login:{lchal}").as_bytes(),
        )
        .unwrap();
    let login_finish = call(
        &prod_app,
        Method::POST,
        "/api/auth/biometric/login/finish",
        None,
        Some(json!({
            "challenge_id": login_start.json["challenge_id"],
            "credential_id": unattested_cred_id,
            "signature": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(lsig.as_ref()),
        })),
    )
    .await;
    assert_eq!(login_finish.status, StatusCode::BAD_REQUEST);
    assert_eq!(login_finish.json["code"], "BIOMETRIC_FAILED");

    let reauth_start = call(
        &prod_app,
        Method::POST,
        "/api/auth/biometric/reauth/start",
        Some(&cookie),
        Some(json!({ "credential_id": unattested_cred_id })),
    )
    .await;
    assert_eq!(reauth_start.status, StatusCode::BAD_REQUEST);
    assert_eq!(reauth_start.json["code"], "BIOMETRIC_FAILED");

    // Remove the unattested biometric row so the prod user has ZERO MFA factors.
    sqlx::query("DELETE FROM biometric_credentials WHERE user_id=$1")
        .bind(uid)
        .execute(&prod_st.db)
        .await
        .unwrap();

    // 4. F-06: If a zero-MFA session is locked in DB, registering a passkey or biometric key must return 423 SESSION_LOCKED.
    sqlx::query("UPDATE sessions SET locked_at = app_now() WHERE user_id = $1")
        .bind(uid)
        .execute(&prod_st.db)
        .await
        .unwrap();
    for path in [
        "/api/auth/passkey/register/start",
        "/api/auth/passkey/register/finish",
        "/api/auth/biometric/register/start",
        "/api/auth/biometric/register/finish",
    ] {
        let locked_res = call(
            &prod_app,
            Method::POST,
            path,
            Some(&cookie),
            Some(json!({})),
        )
        .await;
        assert_eq!(
            locked_res.status,
            StatusCode::LOCKED,
            "{path}: {}",
            locked_res.json
        );
        assert_eq!(locked_res.json["code"], "SESSION_LOCKED");
    }

    // 5. F-06: Locking a zero-MFA session in prod via POST /api/auth/lock logs out & deletes the session row.
    sqlx::query("UPDATE sessions SET locked_at = NULL WHERE user_id = $1")
        .bind(uid)
        .execute(&prod_st.db)
        .await
        .unwrap();
    let lock_res = call(
        &prod_app,
        Method::POST,
        "/api/auth/lock",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(lock_res.status, StatusCode::OK, "{}", lock_res.json);
    assert_eq!(lock_res.json["logged_out"], true);

    let after_lock = call(&prod_app, Method::GET, "/api/rides", Some(&cookie), None).await;
    assert_eq!(after_lock.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn f07_telegram_chat_id_cannot_be_bound_without_webhook_pairing() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    // 1. Reject arbitrary Telegram chat IDs on both /api/me and /api/notifications/preferences.
    for path in ["/api/me", "/api/notifications/preferences"] {
        let bad_req = call(
            &app,
            Method::PUT,
            path,
            Some(&member),
            Some(json!({ "telegram_chat_id": "999888777" })),
        )
        .await;
        assert_eq!(
            bad_req.status,
            StatusCode::BAD_REQUEST,
            "{path}: {}",
            bad_req.json
        );
        assert_eq!(bad_req.json["code"], "TELEGRAM_PAIRING_REQUIRED");
    }

    // 2. Simulate verified pairing + blocked state.
    sqlx::query("UPDATE users SET telegram_chat_id='987654321' WHERE id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO user_notification_prefs (user_id, telegram_blocked, updated_at)
         VALUES ($1, 1, app_now())
         ON CONFLICT (user_id) DO UPDATE SET telegram_blocked=1",
    )
    .bind(member_id)
    .execute(&st.db)
    .await
    .unwrap();

    // Re-saving unchanged verified ID succeeds and preserves `telegram_blocked = 1`.
    let same_me = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({ "telegram_chat_id": "987654321", "readiness_prompt_mins": 30 })),
    )
    .await;
    assert_eq!(same_me.status, StatusCode::OK, "{}", same_me.json);
    let same_prefs = call(
        &app,
        Method::PUT,
        "/api/notifications/preferences",
        Some(&member),
        Some(json!({ "telegram_chat_id": "987654321", "driver_remind_mins": 60 })),
    )
    .await;
    assert_eq!(same_prefs.status, StatusCode::OK, "{}", same_prefs.json);
    let blocked_still: i64 =
        sqlx::query_scalar("SELECT telegram_blocked FROM user_notification_prefs WHERE user_id=$1")
            .bind(member_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(blocked_still, 1);

    // Changing to another user's chat ID is rejected.
    let hijack = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({ "telegram_chat_id": "111222333" })),
    )
    .await;
    assert_eq!(hijack.status, StatusCode::BAD_REQUEST);
    assert_eq!(hijack.json["code"], "TELEGRAM_PAIRING_REQUIRED");

    // Explicit disconnect (`""`) clears the binding and resets `telegram_blocked = 0`.
    let disconnect = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({ "telegram_chat_id": "" })),
    )
    .await;
    assert_eq!(disconnect.status, StatusCode::OK, "{}", disconnect.json);
    let stored_tg: String =
        sqlx::query_scalar("SELECT COALESCE(telegram_chat_id, '') FROM users WHERE id=$1")
            .bind(member_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(stored_tg, "");
    let blocked_after: i64 =
        sqlx::query_scalar("SELECT telegram_blocked FROM user_notification_prefs WHERE user_id=$1")
            .bind(member_id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(blocked_after, 0);
}

#[tokio::test]
async fn f08_totp_backup_code_uses_user_scoped_rate_limit_key_across_ips() {
    // Enable trust_proxy to simulate two distinct X-Forwarded-For client IPs against the same user account.
    let dir = std::env::temp_dir().join(format!("gr-f08-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        dev_mode: true,
        database_url: fresh_db().await,
        upload_dir: dir.join("uploads"),
        static_dir: None,
        public_url: ORIGIN.into(),
        bind: "127.0.0.1:0".into(),
        max_live_drivers: 10,
        trust_proxy: true,
        chat_retention_days: 180,
        audit_retention_days: 730,
        session_timeout_mins: 15,
        json_logs: false,
        routing: Default::default(),
        db: grace_ride::state::DbConfig {
            max_connections: 5,
            acquire_timeout_secs: 60,
            ..Default::default()
        },
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
        turnstile: Default::default(),
        android_cert_sha256_fingerprints: Vec::new(),
        ios_app_ids: Vec::new(),
        require_biometric_attestation: false,
    };
    cfg.validate().unwrap();
    let st = grace_ride::init(cfg).await.unwrap();
    let app = grace_ride::router(st.clone());

    let member = login(&st, "member@example.org").await;
    let member2 = login(&st, "member@example.org").await;
    let mid = user_id(&st, "member@example.org").await;
    sqlx::query("UPDATE users SET totp_enabled=1, totp_secret='JBSWY3DPEHPK3PXP' WHERE id=$1")
        .bind(mid)
        .execute(&st.db)
        .await
        .unwrap();

    // Send 10 failed attempts (5 TOTP + 5 backup codes) across 10 distinct X-Forwarded-For IPs.
    for i in 0..10 {
        let code = if i < 5 {
            format!("{i:06}")
        } else {
            format!("BAD{i:05}")
        };
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/auth/totp/verify")
            .header(header::COOKIE, &member)
            .header(header::ORIGIN, ORIGIN)
            .header("x-forwarded-for", format!("198.51.100.{}", i + 1))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "code": code }).to_string()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&bytes);
        assert_eq!(status, StatusCode::BAD_REQUEST, "attempt {i}: {body_str}");
    }

    // Attempt #11 from another IP AND a fresh second session must hit user-scoped rate limit (429 TOO_MANY_REQUESTS).
    let req11 = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/totp/verify")
        .header(header::COOKIE, &member2)
        .header(header::ORIGIN, ORIGIN)
        .header("x-forwarded-for", "198.51.100.99")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": "BADFINAL" }).to_string()))
        .unwrap();
    let res11 = app.clone().oneshot(req11).await.unwrap();
    assert_eq!(res11.status(), StatusCode::TOO_MANY_REQUESTS);
}
