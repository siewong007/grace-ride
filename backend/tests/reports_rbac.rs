//! Role-privilege & scope tests for member and admin reporting endpoints.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

const TEST_MONTH: &str = "2031-05";

async fn uid(st: &grace_ride::state::AppState, email: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn seed_report_fixture(
    app: &axum::Router,
    st: &grace_ride::state::AppState,
) -> (i64, i64, i64, i64, i64) {
    let admin_id = uid(st, "admin@example.org").await;
    let leader_id = uid(st, "cellleader@example.org").await;
    let driver_id = uid(st, "driver@example.org").await;
    let member_id = uid(st, "member@example.org").await;

    let area_id: i64 = sqlx::query_scalar("SELECT id FROM areas ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let group_id: i64 = sqlx::query_scalar(
        "SELECT id FROM groups WHERE is_all=0 AND is_visitor=0 ORDER BY id LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();

    let starts = "2031-05-10T19:00:00";
    let pickup = "2031-05-10T18:15:00";
    let created_ts = "2031-05-10T12:00:00";

    // 1. Admin-owned published event + attended signup + completed ride + draft settlement + published settlement
    let admin_ev: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, kind, starts_at, location, status, created_by, attendance_enabled)
         VALUES ('Global Worship', 'other', $1, 'Main Hall', 'published', $2, 1) RETURNING id",
    )
    .bind(starts)
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    // Draft event that must be excluded from event reports
    sqlx::query(
        "INSERT INTO events (title, kind, starts_at, location, status, created_by)
         VALUES ('Secret Draft', 'other', $1, 'Main Hall', 'draft', $2)",
    )
    .bind(starts)
    .bind(admin_id)
    .execute(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, need_ride, attended, as_group_id)
         VALUES ($1, $2, 'going', 1, 1, $3)",
    )
    .bind(admin_ev)
    .bind(member_id)
    .bind(group_id)
    .execute(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id,
                            pickup_address, destination, seats, status, distance_km,
                            reimbursement_amount_cents, as_group_id)
         VALUES ($1, $2, $3, $4, $5, '1 Jalan Test', 'Main Hall', 1, 'completed', 12, 1500, $6)",
    )
    .bind(member_id)
    .bind(driver_id)
    .bind(admin_ev)
    .bind(pickup)
    .bind(area_id)
    .bind(group_id)
    .execute(&st.db)
    .await
    .unwrap();

    // 2. Cell-leader-owned published event + unfilled ride + attended signup
    let cell_ev: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, kind, starts_at, location, status, created_by, attendance_enabled)
         VALUES ('Cell Fellowship', 'badminton', $1, 'Court A', 'published', $2, 1) RETURNING id",
    )
    .bind(starts)
    .bind(leader_id)
    .fetch_one(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, need_ride, attended, as_group_id)
         VALUES ($1, $2, 'going', 1, 1, $3)",
    )
    .bind(cell_ev)
    .bind(member_id)
    .bind(group_id)
    .execute(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id,
                            pickup_address, destination, seats, status, as_group_id)
         VALUES ($1, NULL, $2, $3, $4, '2 Jalan Cell', 'Court A', 2, 'pending_driver', $5)",
    )
    .bind(member_id)
    .bind(cell_ev)
    .bind(pickup)
    .bind(area_id)
    .bind(group_id)
    .execute(&st.db)
    .await
    .unwrap();

    // 3. Published settlement for admin_ev with confirmed share for member_id, plus a draft settlement
    let pub_bill: i64 = sqlx::query_scalar(
        "INSERT INTO settlements (event_id, created_by, title, split_mode, subsidy_cents, payee_name, payee_account, status)
         VALUES ($1, $2, 'Worship Snacks', 'equal', 500, 'Admin', '123456', 'published') RETURNING id",
    )
    .bind(admin_ev)
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO settlement_items (settlement_id, label, amount_cents) VALUES ($1, 'Food', 2500)")
        .bind(pub_bill)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO settlement_shares (settlement_id, user_id, amount_cents, status, paid_cents)
         VALUES ($1, $2, 2000, 'confirmed', 2000)",
    )
    .bind(pub_bill)
    .bind(member_id)
    .execute(&st.db)
    .await
    .unwrap();

    let draft_bill: i64 = sqlx::query_scalar(
        "INSERT INTO settlements (event_id, created_by, title, split_mode, subsidy_cents, payee_name, payee_account, status)
         VALUES ($1, $2, 'Draft Bill', 'equal', 0, 'Admin', '123456', 'draft') RETURNING id",
    )
    .bind(admin_ev)
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO settlement_items (settlement_id, label, amount_cents) VALUES ($1, 'Hidden', 9900)")
        .bind(draft_bill)
        .execute(&st.db)
        .await
        .unwrap();

    // 4. Reimbursements for member_id in 2031-05: one pending, one approved, one paid in a payout batch
    let member_cookie = login(st, "member@example.org").await;
    let receipt_id = upload_png(app, &member_cookie).await;
    let finance_id = uid(st, "finance@example.org").await;
    let president_id = uid(st, "president@example.org").await;

    sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, status, created_at, updated_at)
         VALUES ($1, $2, 1800, 'Pending Grab', $3, 'Member', '0123456789', 'pending', $4, $4)",
    )
    .bind(member_id)
    .bind(admin_ev)
    .bind(receipt_id)
    .bind(created_ts)
    .execute(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, status, reviewed_by, payee_verified, created_at, updated_at)
         VALUES ($1, $2, 2200, 'Approved Grab', $3, 'Member', '0123456789', 'approved', $4, 1, $5, $5)",
    )
    .bind(member_id)
    .bind(admin_ev)
    .bind(receipt_id)
    .bind(admin_id)
    .bind(created_ts)
    .execute(&st.db)
    .await
    .unwrap();

    let batch_id: i64 = sqlx::query_scalar(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, paid_ref, created_by, paid_by, paid_at, created_at)
         VALUES ($1, 'Batch May', 3000, 1, 'paid', 'REF-99', $2, $3, $4, $4) RETURNING id",
    )
    .bind(TEST_MONTH)
    .bind(finance_id)
    .bind(president_id)
    .bind(created_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, status, reviewed_by, payee_verified, paid_ref, paid_by, paid_at, batch_id, created_at, updated_at)
         VALUES ($1, $2, 3000, 'Paid Grab', $3, 'Member', '0123456789', 'paid', $4, 1, 'REF-99', $5, $6, $7, $6, $6)",
    )
    .bind(member_id)
    .bind(admin_ev)
    .bind(receipt_id)
    .bind(admin_id)
    .bind(president_id)
    .bind(created_ts)
    .bind(batch_id)
    .execute(&st.db)
    .await
    .unwrap();

    (admin_ev, cell_ev, member_id, driver_id, group_id)
}

#[tokio::test]
async fn reports_enforce_role_privileges_and_mfa_and_scope() {
    let (app, st) = setup().await;
    let (_admin_ev, _cell_ev, member_id, driver_id, group_id) =
        seed_report_fixture(&app, &st).await;
    let month = TEST_MONTH;

    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;
    let leader_cookie = {
        let sid = uid(&st, "sophie@example.org").await;
        sqlx::query("UPDATE users SET roles='leader' WHERE id=$1")
            .bind(sid)
            .execute(&st.db)
            .await
            .unwrap();
        login(&st, "sophie@example.org").await
    };
    let transport = login(&st, "transport@example.org").await;
    let cell_leader = login(&st, "cellleader@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    let auditor = login(&st, "auditor@example.org").await;
    let secretary = login(&st, "secretary@example.org").await;
    let traffic_head = login(&st, "traffichead@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let president = login(&st, "president@example.org").await;
    let vp = login(&st, "vicepresident@example.org").await;
    let superadmin = login(&st, "superadmin@example.org").await;

    // 1. Unprivileged / non-admin_reports roles are forbidden from all /admin/reports/* and other-member reports
    for (label, cookie, other_uid) in [
        ("member", &member, driver_id),
        ("driver", &driver, member_id),
        ("leader", &leader_cookie, member_id),
        ("transport", &transport, member_id),
    ] {
        for path in [
            "/api/admin/reports/capabilities".to_string(),
            format!("/api/admin/reports/overview?month={month}"),
            format!("/api/admin/reports/events?month={month}"),
            format!("/api/admin/reports/demand?month={month}"),
            format!("/api/admin/reports/finance?month={month}"),
            format!("/api/reports/monthly?month={month}&user_id={other_uid}"),
        ] {
            let r = call(&app, Method::GET, &path, Some(cookie), None).await;
            assert_eq!(
                r.status,
                StatusCode::FORBIDDEN,
                "role={label} path={path} body={}",
                r.json
            );
        }
    }

    // 2. Staff MFA gate: admin without MFA cannot access /admin/reports/* or another member's report
    let weak_admin = login_mfa(&st, "admin@example.org", false).await;
    for path in [
        "/api/admin/reports/capabilities".to_string(),
        format!("/api/admin/reports/overview?month={month}"),
        format!("/api/admin/reports/events?month={month}"),
        format!("/api/admin/reports/demand?month={month}"),
        format!("/api/admin/reports/finance?month={month}"),
        format!("/api/reports/monthly?month={month}&user_id={member_id}"),
    ] {
        let r = call(&app, Method::GET, &path, Some(&weak_admin), None).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "weak admin path={path}");
    }

    // 3. Personal report: member gets own reimbursements_summary, no organised_events;
    //    driver with type=passenger gets distance_km=0 and reimbursement_cents=0!
    let self_member = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month={month}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(self_member.status, StatusCode::OK, "{}", self_member.json);
    assert_eq!(self_member.json["costs_paid_cents"], 2000);
    assert_eq!(self_member.json["reimbursements_summary"]["claim_count"], 3);
    assert_eq!(
        self_member.json["reimbursements_summary"]["pending_count"],
        1
    );
    assert!(self_member.json["organised_events"].is_null());

    let driver_pax = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month={month}&type=passenger"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(driver_pax.status, StatusCode::OK);
    assert_eq!(driver_pax.json["rides_driven"], 0);
    assert_eq!(driver_pax.json["distance_km"], 0);
    assert_eq!(driver_pax.json["reimbursement_cents"], 0);

    let driver_all = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month={month}&type=all"),
        Some(&driver),
        None,
    )
    .await;
    assert_eq!(driver_all.status, StatusCode::OK);
    assert_eq!(driver_all.json["rides_driven"], 1);
    assert_eq!(driver_all.json["distance_km"], 12);
    assert_eq!(driver_all.json["reimbursement_cents"], 1500);

    // Organiser sees organised_events + settlements summary on own report
    let self_cell = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month={month}"),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(self_cell.status, StatusCode::OK);
    assert_eq!(self_cell.json["organised_events"]["events_count"], 1);
    assert_eq!(self_cell.json["organised_events"]["total_attended"], 1);
    assert!(self_cell.json["organised_events"]["settlements"].is_object());

    // 4. Cell leader scoping: own_events only on /demand, /events, /overview; 403 on /finance, other member, badge edit
    let cell_caps = call(
        &app,
        Method::GET,
        "/api/admin/reports/capabilities",
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(cell_caps.status, StatusCode::OK);
    assert_eq!(cell_caps.json["member_reports_global"], false);
    assert_eq!(cell_caps.json["member_badge_icon_edit"], false);
    assert_eq!(cell_caps.json["demand_scope"], "own_events");
    assert_eq!(cell_caps.json["events_scope"], "own_events");
    assert_eq!(cell_caps.json["finance"], false);
    assert_eq!(cell_caps.json["membership"], false);

    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/finance?month={month}"),
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
            Method::PUT,
            &format!("/api/admin/users/{driver_id}/badges/night/icon"),
            Some(&cell_leader),
            Some(json!({"icon": "moon"}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    let cell_demand = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/demand?month={month}"),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(cell_demand.status, StatusCode::OK);
    assert_eq!(cell_demand.json["scope"], "own_events");
    assert_eq!(cell_demand.json["totals"]["total_requests"], 1); // only Cell Fellowship ride
    assert_eq!(cell_demand.json["totals"]["unfilled"], 1);
    assert_eq!(cell_demand.json["by_group"][0]["group_id"], group_id);

    let cell_events = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/events?month={month}"),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(cell_events.status, StatusCode::OK);
    assert_eq!(cell_events.json["scope"], "own_events");
    assert_eq!(cell_events.json["totals"]["events_count"], 1);
    assert_eq!(cell_events.json["events"][0]["title"], "Cell Fellowship");
    assert_eq!(cell_events.json["by_group"][0]["group_id"], group_id);

    let cell_overview = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/overview?month={month}"),
        Some(&cell_leader),
        None,
    )
    .await;
    assert_eq!(cell_overview.status, StatusCode::OK);
    assert_eq!(cell_overview.json["transport"]["total_rides"], 1);
    assert_eq!(cell_overview.json["events"]["events_count"], 1);
    assert!(cell_overview.json["finance"].is_null());
    assert!(cell_overview.json["reimb_review"].is_null());
    assert!(cell_overview.json["membership"].is_null());

    // 5. Finance & Auditor: payouts & bills only; 403 on demand/events/badge-edit; member report redacts events & passenger rides & pending claims
    for fin_cookie in [&finance, &auditor] {
        let caps = call(
            &app,
            Method::GET,
            "/api/admin/reports/capabilities",
            Some(fin_cookie),
            None,
        )
        .await;
        assert_eq!(caps.status, StatusCode::OK);
        assert_eq!(caps.json["demand"], false);
        assert_eq!(caps.json["events"], false);
        assert_eq!(caps.json["finance"], true);
        assert_eq!(caps.json["finance_sections"]["reimb_review"], false);
        assert_eq!(caps.json["finance_sections"]["reimb_payouts"], true);
        assert_eq!(caps.json["finance_sections"]["bills"], true);
        assert_eq!(caps.json["member_badge_icon_edit"], false);

        assert_eq!(
            call(
                &app,
                Method::GET,
                &format!("/api/admin/reports/demand?month={month}"),
                Some(fin_cookie),
                None
            )
            .await
            .status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(
                &app,
                Method::GET,
                &format!("/api/admin/reports/events?month={month}"),
                Some(fin_cookie),
                None
            )
            .await
            .status,
            StatusCode::FORBIDDEN
        );

        let fin_rep = call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/finance?month={month}"),
            Some(fin_cookie),
            None,
        )
        .await;
        assert_eq!(fin_rep.status, StatusCode::OK);
        assert!(fin_rep.json["reimbursement_review"].is_null());
        assert_eq!(
            fin_rep.json["reimbursement_payouts"]["approved_unpaid_count"],
            1
        );
        assert_eq!(fin_rep.json["reimbursement_payouts"]["paid_count"], 1);
        assert_eq!(
            fin_rep.json["reimbursement_payouts"]["batches"][0]["maker_checker_ok"],
            true
        );
        assert_eq!(fin_rep.json["bills"]["totals"]["total_settlements"], 1); // Draft bill excluded
        assert_eq!(fin_rep.json["bills"]["totals"]["collected_cents"], 2000);

        let fin_mem = call(
            &app,
            Method::GET,
            &format!("/api/reports/monthly?month={month}&user_id={member_id}"),
            Some(fin_cookie),
            None,
        )
        .await;
        assert_eq!(fin_mem.status, StatusCode::OK);
        assert_eq!(fin_mem.json["costs_paid_cents"], 2000);
        assert!(fin_mem.json["events_attended"].is_null());
        assert!(fin_mem.json["rides_received"].is_null());
        assert_eq!(fin_mem.json["events"].as_array().unwrap().len(), 0);
        assert_eq!(fin_mem.json["reimbursements_summary"]["claim_count"], 2); // approved + paid only
        assert!(fin_mem.json["reimbursements_summary"]["pending_count"].is_null());
    }

    // 6. Admin / Secretary / Traffic Head: global demand & events + reimb_review only (no payouts, no bills, costs_paid_cents=null)
    for ops_cookie in [&admin, &secretary, &traffic_head] {
        let fin_rep = call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/finance?month={month}"),
            Some(ops_cookie),
            None,
        )
        .await;
        assert_eq!(fin_rep.status, StatusCode::OK);
        assert_eq!(
            fin_rep.json["reimbursement_review"]["by_status"]["pending"]["count"],
            1
        );
        assert!(fin_rep.json["reimbursement_payouts"].is_null());
        assert!(fin_rep.json["bills"].is_null());

        let mem_rep = call(
            &app,
            Method::GET,
            &format!("/api/reports/monthly?month={month}&user_id={member_id}"),
            Some(ops_cookie),
            None,
        )
        .await;
        assert_eq!(mem_rep.status, StatusCode::OK);
        assert!(
            mem_rep.json["costs_paid_cents"].is_null(),
            "ops role must not see member settlement payments"
        );
        assert_eq!(mem_rep.json["events_attended"], 2);
        assert_eq!(mem_rep.json["rides_received"], 1);
        assert_eq!(mem_rep.json["reimbursements_summary"]["claim_count"], 3);

        let ev_rep = call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/events?month={month}"),
            Some(ops_cookie),
            None,
        )
        .await;
        assert_eq!(ev_rep.status, StatusCode::OK);
        assert_eq!(ev_rep.json["scope"], "global");
        assert_eq!(ev_rep.json["totals"]["events_count"], 2); // Draft event excluded
        let groups = ev_rep.json["by_group"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["group_id"], group_id);
        assert_eq!(groups[0]["going"], 2);
        assert_eq!(groups[0]["attended"], 2);

        let dem_rep = call(
            &app,
            Method::GET,
            "/api/admin/reports/demand?from=2031-05-01&to=2031-05-31",
            Some(ops_cookie),
            None,
        )
        .await;
        assert_eq!(dem_rep.status, StatusCode::OK);
        assert_eq!(dem_rep.json["range"], "custom");
        assert_eq!(dem_rep.json["totals"]["total_requests"], 2);
        assert_eq!(dem_rep.json["by_group"][0]["group_id"], group_id);
    }

    // Secretary lacks admin_members.update -> 403 on badge icon edit, while admin succeeds
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{driver_id}/badges/night/icon"),
            Some(&secretary),
            Some(json!({"icon": "moon"}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{driver_id}/badges/night/icon"),
            Some(&admin),
            Some(json!({"icon": "moon"}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // 7. President / Vice President / Superadmin: full global access across all sections
    for top_cookie in [&president, &vp, &superadmin] {
        let ov = call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/overview?month={month}"),
            Some(top_cookie),
            None,
        )
        .await;
        assert_eq!(ov.status, StatusCode::OK);
        assert!(ov.json["transport"].is_object());
        assert!(ov.json["events"].is_object());
        assert!(ov.json["reimb_review"].is_object());
        assert!(ov.json["finance"]["reimb_payouts"].is_object());
        assert!(ov.json["finance"]["bills"].is_object());
        assert!(ov.json["membership"].is_object());

        let fin_rep = call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/finance?month={month}"),
            Some(top_cookie),
            None,
        )
        .await;
        assert_eq!(fin_rep.status, StatusCode::OK);
        assert!(fin_rep.json["reimbursement_review"].is_object());
        assert!(fin_rep.json["reimbursement_payouts"].is_object());
        assert!(fin_rep.json["bills"].is_object());

        let mem_rep = call(
            &app,
            Method::GET,
            &format!("/api/reports/monthly?month={month}&user_id={member_id}"),
            Some(top_cookie),
            None,
        )
        .await;
        assert_eq!(mem_rep.status, StatusCode::OK);
        assert_eq!(mem_rep.json["costs_paid_cents"], 2000);
        assert_eq!(mem_rep.json["events_attended"], 2);
    }

    // 8. Custom role with ONLY admin_reports.view fails closed with 403 on domain endpoints and reports minimal capabilities
    let made = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Report Observer",
            "description": "Minimal report role",
            "permissions": ["admin_reports.view"],
        })),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);
    let rkey = made.json["key"].as_str().unwrap().to_string();
    let yawen_id = uid(&st, "yawen@example.org").await;
    sqlx::query("UPDATE users SET roles=$1 WHERE id=$2")
        .bind(&rkey)
        .bind(yawen_id)
        .execute(&st.db)
        .await
        .unwrap();
    let custom_cookie = login(&st, "yawen@example.org").await;
    let custom_caps = call(
        &app,
        Method::GET,
        "/api/admin/reports/capabilities",
        Some(&custom_cookie),
        None,
    )
    .await;
    assert_eq!(custom_caps.status, StatusCode::OK);
    assert_eq!(custom_caps.json["overview"], false);
    assert_eq!(custom_caps.json["demand"], false);
    assert_eq!(custom_caps.json["events"], false);
    assert_eq!(custom_caps.json["finance"], false);
    assert_eq!(custom_caps.json["member_reports_global"], false);

    for path in [
        format!("/api/admin/reports/overview?month={month}"),
        format!("/api/admin/reports/events?month={month}"),
        format!("/api/admin/reports/demand?month={month}"),
        format!("/api/admin/reports/finance?month={month}"),
        format!("/api/reports/monthly?month={month}&user_id={member_id}"),
    ] {
        let res = call(&app, Method::GET, &path, Some(&custom_cookie), None).await;
        assert_eq!(
            res.status,
            StatusCode::FORBIDDEN,
            "custom minimal role path={path}: {}",
            res.json
        );
    }
}

#[tokio::test]
async fn reports_hardening_metrics_and_custom_roles() {
    let (app, st) = setup().await;
    let (admin_ev, _cell_ev, member_id, driver_id, group_id) = seed_report_fixture(&app, &st).await;
    let month = TEST_MONTH;

    let admin_id = uid(&st, "admin@example.org").await;
    let president_id = uid(&st, "president@example.org").await;
    let finance_id = uid(&st, "finance@example.org").await;

    let admin = login(&st, "admin@example.org").await;
    let president = login(&st, "president@example.org").await;
    let superadmin = login(&st, "superadmin@example.org").await;
    let finance = login(&st, "finance@example.org").await;
    let auditor = login(&st, "auditor@example.org").await;
    let cell_leader = login(&st, "cellleader@example.org").await;
    let member_cookie = login(&st, "member@example.org").await;

    // 1. traffic_member lacks admin_reports.view -> 403 everywhere
    let tm_id = uid(&st, "sophie@example.org").await;
    sqlx::query("UPDATE users SET roles='traffic_member' WHERE id=$1")
        .bind(tm_id)
        .execute(&st.db)
        .await
        .unwrap();
    let traffic_member = login(&st, "sophie@example.org").await;
    for path in [
        "/api/admin/reports/capabilities".to_string(),
        format!("/api/admin/reports/overview?month={month}"),
        format!("/api/admin/reports/events?month={month}"),
        format!("/api/admin/reports/demand?month={month}"),
        format!("/api/admin/reports/finance?month={month}"),
        format!("/api/reports/monthly?month={month}&user_id={member_id}"),
    ] {
        assert_eq!(
            call(&app, Method::GET, &path, Some(&traffic_member), None)
                .await
                .status,
            StatusCode::FORBIDDEN,
            "traffic_member path={path}"
        );
    }

    // 2. Cross-domain badge redaction: finance/auditor viewing another member sees ONLY driver badges,
    //    never member_events / passenger / traffic_lead badges.
    for fin in [&finance, &auditor] {
        let rep = call(
            &app,
            Method::GET,
            &format!("/api/reports/monthly?month={month}&user_id={driver_id}"),
            Some(fin),
            None,
        )
        .await;
        assert_eq!(rep.status, StatusCode::OK);
        assert_eq!(rep.json["period_end_inclusive"], "2031-05-31");
        assert_eq!(rep.json["visible_sections"]["badges"], true);
        let cats: Vec<&str> = rep.json["badges"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|b| b["category"].as_str())
            .collect();
        assert!(!cats.is_empty());
        assert!(
            cats.iter().all(|c| *c == "driver"),
            "finance/auditor leaked non-driver badges: {:?}",
            cats
        );
    }

    // 3. Audit trail records report.member_view & report.finance_view
    let fin_audit: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE action='report.member_view' AND entity_id=$1",
    )
    .bind(driver_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(fin_audit >= 2);
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/finance?month={month}"),
            Some(&finance),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    let fin_view_audit: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE actor_id=$1 AND action='report.finance_view'",
    )
    .bind(finance_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(fin_view_audit >= 1);

    // 4. Hardened PUT /admin/users/{id}/badges/{badge_key}/icon: validation, 404, rank check
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{driver_id}/badges/not_a_real_badge/icon"),
            Some(&admin),
            Some(json!({"icon": "moon"}))
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{driver_id}/badges/night/icon"),
            Some(&admin),
            Some(json!({"icon": "x".repeat(40)}))
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            &app,
            Method::PUT,
            "/api/admin/users/99999999/badges/night/icon",
            Some(&admin),
            Some(json!({"icon": "moon"}))
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    // Unranked admin cannot modify ranked president's badge icon; superadmin can edit admin's.
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{president_id}/badges/night/icon"),
            Some(&admin),
            Some(json!({"icon": "moon"}))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{admin_id}/badges/night/icon"),
            Some(&superadmin),
            Some(json!({"icon": "trophy"}))
        )
        .await
        .status,
        StatusCode::OK
    );

    // 5. Preset period inclusive `to` dates & query validation
    let m_rep = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/overview?month={month}&range=monthly"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(m_rep.json["to"], "2031-05-31");
    let q_rep = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/overview?month={month}&range=quarterly"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(q_rep.json["from"], "2031-04-01");
    assert_eq!(q_rep.json["to"], "2031-06-30");
    let y_rep = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/overview?month={month}&range=yearly"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(y_rep.json["from"], "2031-01-01");
    assert_eq!(y_rep.json["to"], "2031-12-31");

    for bad_path in [
        "/api/admin/reports/demand?from=2031-05-10&to=2031-05-01",
        "/api/admin/reports/demand?from=2030-01-01&to=2031-06-01",
        "/api/admin/reports/demand?from=2031-05-01",
        "/api/admin/reports/events?month=2031-05&kind=bogus",
    ] {
        assert_eq!(
            call(&app, Method::GET, bad_path, Some(&president), None)
                .await
                .status,
            StatusCode::BAD_REQUEST,
            "bad_path={bad_path}"
        );
    }

    // 6. attendance_enabled = 0 & cancelled events & group_id reconciliation
    // Add an untracked event (attendance_enabled=0) with 4 going, 0 attended;
    // attendance_rate_pct must stay 100% (not diluted by untracked event!)
    let untracked_ev: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, kind, starts_at, location, status, created_by, attendance_enabled)
         VALUES ('Untracked Social', 'walk', '2031-05-15T10:00:00', 'Park', 'published', $1, 0) RETURNING id",
    )
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, attended, as_group_id)
         VALUES ($1, $2, 'going', 0, NULL)",
    )
    .bind(untracked_ev)
    .bind(member_id)
    .execute(&st.db)
    .await
    .unwrap();

    // Also add a cancelled event with attended=1 signup -> must NOT count in personal report!
    let cancelled_ev: i64 = sqlx::query_scalar(
        "INSERT INTO events (title, kind, starts_at, location, status, created_by, attendance_enabled)
         VALUES ('Cancelled Meet', 'walk', '2031-05-18T10:00:00', 'Park', 'cancelled', $1, 1) RETURNING id",
    )
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_signups (event_id, user_id, status, attended)
         VALUES ($1, $2, 'going', 1)",
    )
    .bind(cancelled_ev)
    .bind(member_id)
    .execute(&st.db)
    .await
    .unwrap();

    let mem_self = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month={month}"),
        Some(&member_cookie),
        None,
    )
    .await;
    assert_eq!(mem_self.json["events_attended"], 2); // excludes cancelled event!

    let ev_all = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/events?month={month}"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(ev_all.status, StatusCode::OK);
    assert_eq!(ev_all.json["totals"]["attendance_rate_pct"], 100);
    assert_eq!(ev_all.json["totals"]["tracked_events_count"], 2);

    let ev_group = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/events?month={month}&group_id={group_id}"),
        Some(&president),
        None,
    )
    .await;
    assert_eq!(ev_group.status, StatusCode::OK);
    assert_eq!(
        ev_group.json["totals"]["total_going"],
        ev_group.json["by_group"][0]["going"]
    );
    assert!(ev_group.json["totals"]["plan_visitors_count"].is_null());

    // 7. Area driver supply in /demand: present for global role, null for cell_leader
    let area_id: i64 = sqlx::query_scalar("SELECT id FROM areas ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO driver_areas (user_id, area_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(driver_id)
    .bind(area_id)
    .execute(&st.db)
    .await
    .unwrap();
    let dem_global = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/demand?month={month}"),
        Some(&admin),
        None,
    )
    .await;
    assert!(dem_global.json["by_area"][0]["registered_drivers"]
        .as_i64()
        .is_some_and(|n| n >= 1));
    let dem_cell = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/demand?month={month}"),
        Some(&cell_leader),
        None,
    )
    .await;
    assert!(dem_cell.json["by_area"][0]["registered_drivers"].is_null());

    // 8. Finance maker-checker accuracy & cross-month payout trend
    // Two finance-capable officers creating & paying a valid batch (applicant=member_id, reviewer=admin_id)
    // must be maker_checker_ok=true! Plus self-created & paid batch -> false, and paid_by == reviewer -> false!
    let receipt_id = upload_png(&app, &member_cookie).await;
    let valid_b: i64 = sqlx::query_scalar(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, paid_ref, created_by, paid_by, paid_at, created_at)
         VALUES ($1, 'Valid Dual-Control Batch', 1100, 1, 'paid', 'REF-OK', $2, $3, '2031-05-20T12:00:00', '2031-05-20T10:00:00') RETURNING id",
    )
    .bind(month)
    .bind(finance_id)
    .bind(president_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    // Claim created in 2031-04 (previous month!) and paid in 2031-05
    sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, status, reviewed_by, payee_verified, paid_ref, paid_by, paid_at, batch_id, created_at, updated_at)
         VALUES ($1, $2, 1100, 'April Cross-Month Paid', $3, 'Member', '0123456789', 'paid', $4, 1, 'REF-OK', $5, '2031-05-20T12:00:00', $6, '2031-04-28T10:00:00', '2031-05-20T12:00:00')",
    )
    .bind(member_id)
    .bind(admin_ev)
    .bind(receipt_id)
    .bind(admin_id)
    .bind(president_id)
    .bind(valid_b)
    .execute(&st.db)
    .await
    .unwrap();

    // Dual-control DB constraint blocks inserting a self-paid batch directly; simulate a historical
    // pre-migration self-paid batch by dropping and re-adding the constraint NOT VALID.
    let direct_self = sqlx::query(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, paid_ref, created_by, paid_by, paid_at, created_at)
         VALUES ($1, 'Self-Executed Batch Violation', 500, 1, 'paid', 'REF-SELF-B', $2, $2, '2031-05-20T13:00:00', '2031-05-20T10:00:00')",
    )
    .bind(month)
    .bind(finance_id)
    .execute(&st.db)
    .await;
    assert!(
        direct_self.is_err(),
        "chk_payout_batches_dual_control must block direct created_by == paid_by insert"
    );
    sqlx::query("ALTER TABLE payout_batches DROP CONSTRAINT chk_payout_batches_dual_control")
        .execute(&st.db)
        .await
        .unwrap();
    let self_b: i64 = sqlx::query_scalar(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, paid_ref, created_by, paid_by, paid_at, created_at)
         VALUES ($1, 'Self-Executed Batch Violation', 500, 1, 'paid', 'REF-SELF-B', $2, $2, '2031-05-20T13:00:00', '2031-05-20T10:00:00') RETURNING id",
    )
    .bind(month)
    .bind(finance_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "ALTER TABLE payout_batches ADD CONSTRAINT chk_payout_batches_dual_control CHECK (paid_by IS NULL OR paid_by <> created_by) NOT VALID",
    )
    .execute(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, status, reviewed_by, payee_verified, paid_ref, paid_by, paid_at, batch_id, created_at, updated_at)
         VALUES ($1, $2, 500, 'Self-Batch Claim', $3, 'Member', '0123456789', 'paid', $4, 1, 'REF-SELF-B', $5, '2031-05-20T13:00:00', $6, '2031-05-20T09:00:00', '2031-05-20T13:00:00')",
    )
    .bind(member_id)
    .bind(admin_ev)
    .bind(receipt_id)
    .bind(admin_id)
    .bind(finance_id)
    .bind(self_b)
    .execute(&st.db)
    .await
    .unwrap();

    let bad_b: i64 = sqlx::query_scalar(
        "INSERT INTO payout_batches (month, title, total_cents, claim_count, status, paid_ref, created_by, paid_by, paid_at, created_at)
         VALUES ($1, 'SoD Violation Batch', 900, 1, 'paid', 'REF-BAD', $2, $3, '2031-05-21T12:00:00', '2031-05-21T10:00:00') RETURNING id",
    )
    .bind(month)
    .bind(finance_id)
    .bind(admin_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE reimbursements DROP CONSTRAINT chk_reimbursements_reviewer_not_payer")
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO reimbursements (user_id, event_id, amount_cents, description, receipt_file_id, payee_name, payee_phone, status, reviewed_by, payee_verified, paid_ref, paid_by, paid_at, batch_id, created_at, updated_at)
         VALUES ($1, $2, 900, 'SoD Violation Claim', $3, 'Member', '0123456789', 'paid', $4, 1, 'REF-BAD', $4, '2031-05-21T12:00:00', $5, '2031-05-21T09:00:00', '2031-05-21T12:00:00')",
    )
    .bind(member_id)
    .bind(admin_ev)
    .bind(receipt_id)
    .bind(admin_id)
    .bind(bad_b)
    .execute(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "ALTER TABLE reimbursements ADD CONSTRAINT chk_reimbursements_reviewer_not_payer CHECK (paid_by IS NULL OR reviewed_by IS NULL OR paid_by <> reviewed_by) NOT VALID",
    )
    .execute(&st.db)
    .await
    .unwrap();

    let fin_q = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/finance?month={month}&range=quarterly"),
        Some(&finance),
        None,
    )
    .await;
    assert_eq!(fin_q.status, StatusCode::OK);
    let batches = fin_q.json["reimbursement_payouts"]["batches"]
        .as_array()
        .unwrap();
    let ok_batch = batches
        .iter()
        .find(|b| b["title"] == "Valid Dual-Control Batch")
        .unwrap();
    let self_batch = batches
        .iter()
        .find(|b| b["title"] == "Self-Executed Batch Violation")
        .unwrap();
    let bad_batch = batches
        .iter()
        .find(|b| b["title"] == "SoD Violation Batch")
        .unwrap();
    assert_eq!(ok_batch["maker_checker_ok"], true);
    assert_eq!(self_batch["maker_checker_ok"], false);
    assert_eq!(bad_batch["maker_checker_ok"], false);
    // Quarterly monthly_trend has zero-filled April, May, June; May has paid_count >= 1
    let trend = fin_q.json["reimbursement_payouts"]["monthly_trend"]
        .as_array()
        .unwrap();
    assert_eq!(trend.len(), 3);
    assert_eq!(trend[0]["month"], "2031-04");
    assert_eq!(trend[1]["month"], "2031-05");
    assert!(trend[1]["paid_count"].as_i64().unwrap() >= 2);
    assert!(trend[1]["claim_count"].as_i64().unwrap() >= 2);

    // 9. Single-domain custom roles: bills-only custom role gets visible_sections.badges=false
    let root = login(&st, "superadmin@example.org").await;
    let bills_role = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&root),
        Some(json!({
            "name": "Bills Viewer Only",
            "description": "Can only see bills and admin reports",
            "permissions": ["admin_reports.view", "admin_bills.view"],
        })),
    )
    .await;
    let bills_key = bills_role.json["key"].as_str().unwrap().to_string();
    let zq_id = uid(&st, "zhiqiang@example.org").await;
    sqlx::query("UPDATE users SET roles=$1 WHERE id=$2")
        .bind(&bills_key)
        .bind(zq_id)
        .execute(&st.db)
        .await
        .unwrap();
    let bills_cookie = login(&st, "zhiqiang@example.org").await;
    let bills_mem = call(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month={month}&user_id={member_id}"),
        Some(&bills_cookie),
        None,
    )
    .await;
    assert_eq!(bills_mem.status, StatusCode::OK);
    assert_eq!(bills_mem.json["costs_paid_cents"], 2000);
    assert_eq!(bills_mem.json["visible_sections"]["badges"], false);
    assert_eq!(bills_mem.json["badges"].as_array().unwrap().len(), 0);

    // 10. Custom role: admin_reports.view + admin_events.view (global events, no rides/finance)
    let ev_role = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Events Report Only",
            "description": "Events analytics without transport or finance",
            "permissions": ["admin_reports.view", "admin_events.view"],
        })),
    )
    .await;
    let ev_key = ev_role.json["key"].as_str().unwrap().to_string();
    let meiling_id = uid(&st, "meiling@example.org").await;
    sqlx::query("UPDATE users SET roles=$1 WHERE id=$2")
        .bind(&ev_key)
        .bind(meiling_id)
        .execute(&st.db)
        .await
        .unwrap();
    let ev_cookie = login(&st, "meiling@example.org").await;
    let ev_only = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/events?month={month}"),
        Some(&ev_cookie),
        None,
    )
    .await;
    assert_eq!(ev_only.status, StatusCode::OK);
    assert_eq!(ev_only.json["scope"], "global");
    assert!(ev_only.json["totals"]["plan_visitors_count"].is_null());
    assert!(ev_only.json["events"][0]["rides_matched"].is_null());
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/demand?month={month}"),
            Some(&ev_cookie),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/finance?month={month}"),
            Some(&ev_cookie),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // 11. Custom role: admin_reports.view + admin_reimb_review.view (review only, no demand/events/payouts/bills)
    let rev_role = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Reimb Reviewer Only",
            "description": "Reimbursement review analytics only",
            "permissions": ["admin_reports.view", "admin_reimb_review.view"],
        })),
    )
    .await;
    let rev_key = rev_role.json["key"].as_str().unwrap().to_string();
    let enci_id = uid(&st, "enci@example.org").await;
    sqlx::query("UPDATE users SET roles=$1 WHERE id=$2")
        .bind(&rev_key)
        .bind(enci_id)
        .execute(&st.db)
        .await
        .unwrap();
    let rev_cookie = login(&st, "enci@example.org").await;
    let rev_ov = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/overview?month={month}"),
        Some(&rev_cookie),
        None,
    )
    .await;
    assert_eq!(rev_ov.status, StatusCode::OK);
    assert!(rev_ov.json["reimb_review"].is_object());
    assert!(rev_ov.json["transport"].is_null());
    assert!(rev_ov.json["events"].is_null());
    assert!(rev_ov.json["finance"].is_null());
    let rev_fin = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/finance?month={month}"),
        Some(&rev_cookie),
        None,
    )
    .await;
    assert_eq!(rev_fin.status, StatusCode::OK);
    assert!(rev_fin.json["reimbursement_review"].is_object());
    assert!(rev_fin.json["reimbursement_payouts"].is_null());
    assert!(rev_fin.json["bills"].is_null());
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/events?month={month}"),
            Some(&rev_cookie),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Method::GET,
            &format!("/api/admin/reports/demand?month={month}"),
            Some(&rev_cookie),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // 12. Custom role: admin_reports.view + admin_members.update (membership overview + badge edit only)
    let mem_role = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Membership Reports Only",
            "description": "Membership analytics and badge icon customization only",
            "permissions": ["admin_reports.view", "admin_members.update"],
        })),
    )
    .await;
    let mem_key = mem_role.json["key"].as_str().unwrap().to_string();
    let gr_id = uid(&st, "guorong@example.org").await;
    sqlx::query("UPDATE users SET roles=$1 WHERE id=$2")
        .bind(&mem_key)
        .bind(gr_id)
        .execute(&st.db)
        .await
        .unwrap();
    let mem_role_cookie = login(&st, "guorong@example.org").await;
    let mem_caps = call(
        &app,
        Method::GET,
        "/api/admin/reports/capabilities",
        Some(&mem_role_cookie),
        None,
    )
    .await;
    assert_eq!(mem_caps.status, StatusCode::OK);
    assert_eq!(mem_caps.json["overview"], true);
    assert_eq!(mem_caps.json["membership"], true);
    assert_eq!(mem_caps.json["member_badge_icon_edit"], true);
    assert_eq!(mem_caps.json["member_reports_global"], false);
    let mem_ov = call(
        &app,
        Method::GET,
        &format!("/api/admin/reports/overview?month={month}"),
        Some(&mem_role_cookie),
        None,
    )
    .await;
    assert_eq!(mem_ov.status, StatusCode::OK);
    assert!(mem_ov.json["membership"].is_object());
    assert!(mem_ov.json["transport"].is_null());
    assert!(mem_ov.json["events"].is_null());
    assert!(mem_ov.json["finance"].is_null());
    for forbidden_path in [
        format!("/api/admin/reports/events?month={month}"),
        format!("/api/admin/reports/demand?month={month}"),
        format!("/api/admin/reports/finance?month={month}"),
        format!("/api/reports/monthly?month={month}&user_id={member_id}"),
    ] {
        assert_eq!(
            call(
                &app,
                Method::GET,
                &forbidden_path,
                Some(&mem_role_cookie),
                None
            )
            .await
            .status,
            StatusCode::FORBIDDEN
        );
    }
}
