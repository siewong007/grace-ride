//! Verification suite for PostgreSQL 18 performance indexes, bug regressions, single-connection
//! pool deadlock elimination, scenario seed coverage/idempotency, and scale query plans.

mod common;

use std::time::Duration;

use axum::http::{header, Method, StatusCode};
use grace_ride::seed::{
    seed_edge, seed_reset, seed_scale, seed_scenarios, seed_scenarios_with_uploads, seed_verify,
    EDGE_BATCH, SCENARIO_BATCH,
};
use serde_json::{json, Value};

fn plan_uses_index(node: &Value, expected_index: &str) -> bool {
    let is_idx = node
        .get("Node Type")
        .and_then(|v| v.as_str())
        .is_some_and(|nt| matches!(nt, "Index Scan" | "Index Only Scan" | "Bitmap Index Scan"));
    let name_ok = node
        .get("Index Name")
        .and_then(|v| v.as_str())
        .is_some_and(|name| name == expected_index);
    if is_idx && name_ok {
        return true;
    }
    node.get("Plans")
        .and_then(|v| v.as_array())
        .is_some_and(|children| children.iter().any(|c| plan_uses_index(c, expected_index)))
}

fn plan_has_seq_scan(node: &Value) -> bool {
    if node
        .get("Node Type")
        .and_then(|v| v.as_str())
        .is_some_and(|nt| nt == "Seq Scan")
    {
        return true;
    }
    node.get("Plans")
        .and_then(|v| v.as_array())
        .is_some_and(|children| children.iter().any(plan_has_seq_scan))
}

async fn explain_plan(db: &sqlx::PgPool, query: String) -> Value {
    let mut tx = db.begin().await.expect("begin explain tx");
    for setting in [
        "SET LOCAL random_page_cost = 1.1",
        "SET LOCAL effective_io_concurrency = 200",
        "SET LOCAL work_mem = '16MB'",
        "SET LOCAL jit = off",
    ] {
        sqlx::query(setting)
            .execute(&mut *tx)
            .await
            .expect("set planner parameter");
    }
    let rows: Vec<(Value,)> = sqlx::query_as(sqlx::AssertSqlSafe(query))
        .fetch_all(&mut *tx)
        .await
        .expect("explain query");
    tx.rollback().await.expect("rollback explain tx");
    rows[0].0[0]["Plan"].clone()
}

#[tokio::test]
async fn migration_0016_indexes_and_constraints() {
    let (_app, st) = common::setup().await;

    let dropped: Vec<String> = sqlx::query_scalar(
        "SELECT indexname FROM pg_indexes
         WHERE schemaname = current_schema()
           AND indexname = ANY($1)",
    )
    .bind(
        &[
            "idx_notifications_unread",
            "idx_user_badge_icons_user",
            "idx_vehicles_user_id",
            "idx_audit_logs_actor",
        ][..],
    )
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert!(
        dropped.is_empty(),
        "redundant indexes should be dropped, still present: {dropped:?}"
    );

    let expected = [
        "idx_rides_driver_pickup",
        "idx_rides_passenger_pickup",
        "idx_rides_pending_driver_pickup",
        "idx_rides_stale_pending",
        "idx_rides_attention_open",
        "idx_events_checkin_code_norm",
        "idx_event_signups_pass_code_lower",
        "idx_reimbursements_effective_paid_at",
        "idx_users_status_name_id",
        "idx_areas_lower_name",
        "idx_audit_logs_action_id",
        "idx_oauth_states_created",
    ];
    let present: Vec<String> = sqlx::query_scalar(
        "SELECT indexname FROM pg_indexes
         WHERE schemaname = current_schema()
           AND indexname = ANY($1)",
    )
    .bind(&expected[..])
    .fetch_all(&st.db)
    .await
    .unwrap();
    for idx in expected {
        assert!(
            present.iter().any(|p| p == idx),
            "expected index {idx} to exist in current schema"
        );
    }

    let has_trgm: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm')")
            .fetch_one(&st.db)
            .await
            .unwrap();
    if has_trgm {
        let trgm_expected = [
            "idx_users_name_trgm",
            "idx_users_email_trgm",
            "idx_users_phone_trgm",
            "idx_rides_destination_trgm",
            "idx_audit_logs_detail_trgm",
        ];
        let trgm_present: Vec<String> = sqlx::query_scalar(
            "SELECT indexname FROM pg_indexes
             WHERE schemaname = current_schema()
               AND indexname = ANY($1)",
        )
        .bind(&trgm_expected[..])
        .fetch_all(&st.db)
        .await
        .unwrap();
        for idx in trgm_expected {
            assert!(
                trgm_present.iter().any(|p| p == idx),
                "expected pg_trgm index {idx} to exist"
            );
        }
    }

    let unvalidated: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_constraint
         WHERE connamespace = current_schema()::regnamespace AND NOT convalidated",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(unvalidated, 0, "all constraints must be validated");
}

#[tokio::test]
async fn bug_regressions_vehicles_null_fks_visitors_readiness_checkin_housekeeping() {
    let (app, st) = common::setup().await;
    seed_scenarios_with_uploads(&st.db, Some(&st.cfg.upload_dir))
        .await
        .unwrap();

    // 1. Vehicle active-switch must deactivate the previous active vehicle without 23505.
    let driver_cookie = common::login(&st, "scen.carpool@example.org").await;
    let create_res = common::call(
        &app,
        Method::POST,
        "/api/driver/vehicles",
        Some(&driver_cookie),
        Some(json!({
            "model": "Kia Carnival",
            "color": "银色",
            "plate": "REG 2026",
            "seats": 7,
            "vehicle_type": "standard",
            "wheelchair_ok": true,
            "is_active": true,
        })),
    )
    .await;
    assert_eq!(create_res.status, StatusCode::OK, "{}", create_res.json);
    assert_eq!(create_res.json["is_active"], true);

    let list_res = common::call(
        &app,
        Method::GET,
        "/api/driver/vehicles",
        Some(&driver_cookie),
        None,
    )
    .await;
    assert_eq!(list_res.status, StatusCode::OK);
    let vehicles = list_res.json.as_array().unwrap();
    let active_count = vehicles.iter().filter(|v| v["is_active"] == true).count();
    assert_eq!(active_count, 1);

    let old_id = vehicles
        .iter()
        .find(|v| v["plate"] == "SCN 7001")
        .expect("seeded vehicle SCN 7001")["id"]
        .as_i64()
        .unwrap();
    let reactivate_res = common::call(
        &app,
        Method::PUT,
        &format!("/api/driver/vehicles/{old_id}"),
        Some(&driver_cookie),
        Some(json!({
            "model": "Toyota Vellfire",
            "color": "白色",
            "plate": "SCN 7001",
            "seats": 7,
            "vehicle_type": "standard",
            "is_active": true,
        })),
    )
    .await;
    assert_eq!(
        reactivate_res.status,
        StatusCode::OK,
        "{}",
        reactivate_res.json
    );
    assert_eq!(reactivate_res.json["is_active"], true);

    let active_db: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM vehicles WHERE user_id = 91007 AND is_active = 1")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(active_db, 1);

    // 2. Nullable FKs after user/group deletion (event_plan_drafts, ride_change_drafts, visitor_passes).
    let (disposable_uid,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, roles, status, webauthn_uuid)
         VALUES ('Temp Editor', 'tempeditor@example.org', 'transport', 'active', $1) RETURNING id",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .fetch_one(&st.db)
    .await
    .unwrap();
    let (disposable_gid,): (i64,) = sqlx::query_as(
        "INSERT INTO groups (name, description, is_all) VALUES ('Temp Group Null FK', 'temp', 0) RETURNING id",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_plan_drafts (event_id, version, label, status, assignments, visitors, created_by, updated_by)
         VALUES (9102, 99, 'Temp Null Draft', 'draft', '[]', '[]', $1, $1)",
    )
    .bind(disposable_uid)
    .execute(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ride_change_drafts (ride_id, version, action, reason, status, created_by, updated_by)
         VALUES (91001, 99, 'return', 'Temp null author', 'draft', $1, $1)",
    )
    .bind(disposable_uid)
    .execute(&st.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO visitor_passes (token_hash, event_id, host_id, host_group_id, ride_id, name, valid_from, valid_until)
         VALUES ('temp-null-group-token-hash', 9102, 91002, $1, 91001, 'Null Group Visitor', '2026-01-01T00:00:00', '2099-01-01T00:00:00')",
    )
    .bind(disposable_gid)
    .execute(&st.db)
    .await
    .unwrap();
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(disposable_uid)
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM groups WHERE id = $1")
        .bind(disposable_gid)
        .execute(&st.db)
        .await
        .unwrap();

    let super_cookie = common::login_mfa(&st, "scen.superadmin@example.org", true).await;
    for path in [
        "/api/admin/events/9102/plan",
        "/api/rides/91001/drafts",
        "/api/admin/ride-drafts",
        "/api/events/9102/visitors",
    ] {
        let res = common::call(&app, Method::GET, path, Some(&super_cookie), None).await;
        assert_eq!(res.status, StatusCode::OK, "GET {path}: {}", res.json);
    }

    // 3. visitors::close_finished closes open passes tied to cancelled events.
    let cancelled_pass_tok = "temp-cancelled-event-pass-hash";
    sqlx::query(
        "INSERT INTO visitor_passes (token_hash, event_id, host_id, host_group_id, ride_id, name, valid_from, valid_until)
         VALUES ($1, 9105, 91002, 2, 91001, 'Cancelled Event Visitor', '2026-01-01T00:00:00', '2099-01-01T00:00:00')",
    )
    .bind(cancelled_pass_tok)
    .execute(&st.db)
    .await
    .unwrap();
    grace_ride::routes::visitors::close_finished(&st.db)
        .await
        .unwrap();
    let (closed_at, closed_reason): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT closed_at, closed_reason FROM visitor_passes WHERE token_hash = $1")
            .bind(cancelled_pass_tok)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(closed_at.is_some());
    assert_eq!(closed_reason.as_deref(), Some("cancelled"));

    // 4. Readiness updates reject rides already picked up and preserve readiness_status.
    sqlx::query("UPDATE rides SET picked_up_at = '2026-11-26T08:35:00' WHERE id = 91005")
        .execute(&st.db)
        .await
        .unwrap();
    let elder_cookie = common::login(&st, "scen.elder@example.org").await;
    for bad_status in ["delayed_5m", "unable_to_join", "ready"] {
        let res = common::call(
            &app,
            Method::POST,
            "/api/rides/91005/readiness",
            Some(&elder_cookie),
            Some(json!({ "status": bad_status })),
        )
        .await;
        assert_eq!(
            res.status,
            StatusCode::CONFLICT,
            "{bad_status}: {}",
            res.json
        );
    }

    // 5. Checkin re-verifies attendance_enabled inside FOR UPDATE tx.
    sqlx::query("UPDATE events SET attendance_enabled = 0 WHERE id = 9102")
        .execute(&st.db)
        .await
        .unwrap();
    let checkin_res = common::call(
        &app,
        Method::POST,
        "/api/events/9102/checkin",
        Some(&elder_cookie),
        Some(json!({ "code": "SCEN26" })),
    )
    .await;
    assert_eq!(
        checkin_res.status,
        StatusCode::BAD_REQUEST,
        "{}",
        checkin_res.json
    );

    // 6. jobs::run_once cleans expired oauth_states (> 1h old) while keeping fresh ones.
    let stale_ts = grace_ride::util::ts(grace_ride::util::local_now() - chrono::Duration::hours(2));
    let fresh_ts = grace_ride::util::now();
    sqlx::query("INSERT INTO oauth_states (state, created_at) VALUES ('stale-oauth-state', $1), ('fresh-oauth-state', $2)")
        .bind(&stale_ts)
        .bind(&fresh_ts)
        .execute(&st.db)
        .await
        .unwrap();
    grace_ride::jobs::run_once(&st.db, &st.cfg).await.unwrap();
    let remaining: Vec<String> = sqlx::query_scalar(
        "SELECT state FROM oauth_states WHERE state IN ('stale-oauth-state', 'fresh-oauth-state') ORDER BY state",
    )
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(remaining, vec!["fresh-oauth-state"]);
}

#[tokio::test]
async fn single_connection_pool_no_deadlocks() {
    let (_base_app, base_st) = common::setup().await;
    let mut cfg = (*base_st.cfg).clone();
    base_st.db.close().await;
    cfg.db.max_connections = 1;
    cfg.db.acquire_timeout_secs = 5;

    let st = grace_ride::init(cfg).await.unwrap();
    common::register_test_db(&st);
    seed_scenarios_with_uploads(&st.db, Some(&st.cfg.upload_dir))
        .await
        .unwrap();
    seed_edge(&st.db).await.unwrap();
    let app = grace_ride::router(st.clone());

    // exec_staff_reassign_tx must run group_scopes_on(&mut **tx) on single connection without deadlocking.
    let super_cookie = common::login_mfa(&st, "scen.superadmin@example.org", true).await;
    let reassign_res = tokio::time::timeout(
        Duration::from_secs(5),
        common::call(
            &app,
            Method::POST,
            "/api/rides/91003/reassign",
            Some(&super_cookie),
            Some(json!({ "driver_id": 91007 })),
        ),
    )
    .await
    .expect("reassign must not deadlock on single-connection pool");
    assert_eq!(reassign_res.status, StatusCode::OK, "{}", reassign_res.json);

    // family::add_person drops tx before audit on disabled target phone (90009 phone: 012-300 9009).
    let caregiver_cookie = common::login(&st, "scen.caregiver@example.org").await;
    let family_res = tokio::time::timeout(
        Duration::from_secs(5),
        common::call(
            &app,
            Method::POST,
            "/api/me/family/people",
            Some(&caregiver_cookie),
            Some(json!({ "phone": "012-300 9009" })),
        ),
    )
    .await
    .expect("family add_person (disabled audit) must not deadlock on single-connection pool");
    assert_eq!(
        family_res.status,
        StatusCode::BAD_REQUEST,
        "{}",
        family_res.json
    );

    // Normal add_person path (usher phone: 012-310 9113) must also succeed on 1-conn pool.
    let family_ok = tokio::time::timeout(
        Duration::from_secs(5),
        common::call(
            &app,
            Method::POST,
            "/api/me/family/people",
            Some(&caregiver_cookie),
            Some(json!({ "phone": "012-310 9113" })),
        ),
    )
    .await
    .expect("family add_person (active) must not deadlock on single-connection pool");
    assert_eq!(family_ok.status, StatusCode::OK, "{}", family_ok.json);
}

#[tokio::test]
async fn seed_scenarios_idempotency_api_smoke_reset_verify() {
    let (app, st) = common::setup().await;
    seed_scenarios_with_uploads(&st.db, Some(&st.cfg.upload_dir))
        .await
        .unwrap();
    // Second run must be completely idempotent.
    seed_scenarios_with_uploads(&st.db, Some(&st.cfg.upload_dir))
        .await
        .unwrap();
    let report = seed_verify(&st.db).await.unwrap();
    assert_eq!(report.unvalidated_constraints, 0);
    assert_eq!(report.role_sync_drift, 0);
    assert_eq!(report.orphan_trips, 0);
    assert_eq!(report.reimbursement_sod_violations, 0);
    assert_eq!(report.payout_batch_dual_control_violations, 0);
    assert_eq!(report.multiple_active_vehicles, 0);
    assert!(
        report.sequence_lags.is_empty(),
        "{:?}",
        report.sequence_lags
    );

    let super_cookie = common::login_mfa(&st, "scen.superadmin@example.org", true).await;
    let usher_cookie = common::login(&st, "scen.usher@example.org").await;
    let driver_cookie = common::login(&st, "scen.carpool@example.org").await;
    let elder_cookie = common::login(&st, "scen.elder@example.org").await;

    // /api/auth/me (superadmin + custom role + passkey/biometric + hierarchical groups)
    let me_super = common::call(&app, Method::GET, "/api/auth/me", Some(&super_cookie), None).await;
    assert_eq!(me_super.status, StatusCode::OK);
    assert!(me_super.json["passkey_count"].as_i64().unwrap() >= 1);
    assert!(me_super.json["biometric_count"].as_i64().unwrap() >= 1);

    let me_usher = common::call(&app, Method::GET, "/api/auth/me", Some(&usher_cookie), None).await;
    assert_eq!(me_usher.status, StatusCode::OK);
    assert_eq!(me_usher.json["role_names"]["c_scen_usher"], "司事接待组");

    // Staff vs non-staff member directory / picker
    let staff_users = common::call(
        &app,
        Method::GET,
        "/api/admin/users?page=1&limit=20",
        Some(&super_cookie),
        None,
    )
    .await;
    assert_eq!(staff_users.status, StatusCode::OK, "{}", staff_users.json);
    assert!(staff_users.json["items"][0].get("email").is_some());

    // Event organiser/leader or report-permitted user without AdminMembers View sees picker shape ({id, name})
    let picker_cookie = common::login(&st, "scen.usher@example.org").await;
    let picker_users = common::call(
        &app,
        Method::GET,
        "/api/admin/users?page=1&limit=20",
        Some(&picker_cookie),
        None,
    )
    .await;
    assert_eq!(picker_users.status, StatusCode::OK, "{}", picker_users.json);
    assert!(picker_users.json["items"][0].get("email").is_none());

    for (path, cookie) in [
        ("/api/admin/dashboard", &super_cookie),
        ("/api/events", &elder_cookie),
        ("/api/rides", &elder_cookie),
        ("/api/driver/requests", &driver_cookie),
        ("/api/driver/schedule", &driver_cookie),
        ("/api/events/9102/post-event-drivers", &super_cookie),
        ("/api/admin/reports/overview", &super_cookie),
        ("/api/admin/reports/finance", &super_cookie),
        ("/api/events/9102/visitors", &super_cookie),
    ] {
        let res = common::call(&app, Method::GET, path, Some(cookie), None).await;
        assert_eq!(res.status, StatusCode::OK, "GET {path}: {}", res.json);
    }

    // /api/files/91001 serves real PNG bytes + Cache-Control header from load_file_and_access
    let (f_status, f_headers, f_body) =
        common::call_raw(&app, Method::GET, "/api/files/91001", Some(&elder_cookie)).await;
    assert_eq!(f_status, StatusCode::OK);
    assert_eq!(
        f_headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("image/png")
    );
    assert_eq!(
        f_headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok()),
        Some("67")
    );
    assert_eq!(
        f_headers
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok()),
        Some("private, no-cache")
    );
    assert!(f_body.contains("PNG"));

    // Reset non-core batches -> keeps core demo fixtures intact, drops scenario/edge rows
    seed_edge(&st.db).await.unwrap();
    seed_reset(&st.db, false).await.unwrap();
    let scen_left: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE demo_batch = ANY($1)")
            .bind(&[SCENARIO_BATCH, EDGE_BATCH][..])
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(scen_left, 0);
    let core_admin: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = 'superadmin@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(core_admin, 1);

    seed_scenarios_with_uploads(&st.db, Some(&st.cfg.upload_dir))
        .await
        .unwrap();
    seed_verify(&st.db).await.unwrap();
}

#[tokio::test]
async fn explain_json_index_usage_at_scale() {
    let (_app, st) = common::setup().await;
    seed_scenarios(&st.db).await.unwrap();
    seed_scale(&st.db, 2000).await.unwrap();

    let (driver_id, pickup_at): (i64, String) = sqlx::query_as(
        "SELECT driver_id, pickup_at FROM rides WHERE demo_batch = 'scale-2026' AND driver_id IS NOT NULL LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let passenger_id: i64 = sqlx::query_scalar(
        "SELECT passenger_id FROM rides WHERE demo_batch = 'scale-2026' LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let (signup_event_id, signup_pass_code): (i64, String) = sqlx::query_as(
        "SELECT event_id, pass_code FROM event_signups WHERE demo_batch = 'scale-2026' AND pass_code IS NOT NULL LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let (paid_sample,): (String,) = sqlx::query_as(
        "SELECT COALESCE(NULLIF(paid_at, ''), updated_at, created_at) FROM reimbursements WHERE demo_batch = 'scale-2026' AND status = 'paid' LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let paid_day = &paid_sample[..10];
    let today_str = grace_ride::util::today_local()
        .format("%Y-%m-%d")
        .to_string();

    // Force realistic planner selectivity and disable tiny-table seq-scan bias if needed only after checking natural plans:
    let checks: Vec<(&str, String)> = vec![
        (
            "idx_rides_driver_pickup",
            format!(
                "EXPLAIN (FORMAT JSON) SELECT id, pickup_at FROM rides WHERE driver_id = {driver_id} AND pickup_at >= '{}T00:00:00' AND pickup_at <= '{}T23:59:59'",
                &pickup_at[..10], &pickup_at[..10]
            ),
        ),
        (
            "idx_rides_passenger_pickup",
            format!(
                "EXPLAIN (FORMAT JSON) SELECT id FROM rides WHERE passenger_id = {passenger_id} ORDER BY pickup_at DESC, id DESC LIMIT 20"
            ),
        ),
        (
            "idx_rides_pending_driver_pickup",
            format!(
                "EXPLAIN (FORMAT JSON) SELECT id FROM rides WHERE status = 'pending_driver' AND pickup_at >= '{today_str}T00:00' ORDER BY pickup_at LIMIT 50"
            ),
        ),
        (
            "idx_users_status_name_id",
            "EXPLAIN (FORMAT JSON) SELECT id, name FROM users WHERE status = 'active' ORDER BY name ASC, id ASC LIMIT 50".to_string(),
        ),
        (
            "idx_reimbursements_effective_paid_at",
            format!(
                "EXPLAIN (FORMAT JSON) SELECT id, amount_cents FROM reimbursements WHERE status = 'paid' AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) >= '{paid_day}T00:00:00' AND COALESCE(NULLIF(paid_at, ''), updated_at, created_at) <= '{paid_day}T23:59:59'"
            ),
        ),
        (
            "idx_events_checkin_code_norm",
            "EXPLAIN (FORMAT JSON) SELECT id FROM events WHERE LOWER(TRIM(checkin_code)) = 'scl1005'".to_string(),
        ),
        (
            "idx_event_signups_pass_code_lower",
            format!(
                "EXPLAIN (FORMAT JSON) SELECT id FROM event_signups WHERE event_id = {signup_event_id} AND LOWER(pass_code) = LOWER('{signup_pass_code}')"
            ),
        ),
    ];

    for (expected_idx, sql) in checks {
        let plan = explain_plan(&st.db, sql.clone()).await;
        assert!(
            plan_uses_index(&plan, expected_idx),
            "expected {expected_idx} for `{sql}`, got plan:\n{}",
            serde_json::to_string_pretty(&plan).unwrap()
        );
        assert!(
            !plan_has_seq_scan(&plan),
            "unexpected Seq Scan for `{sql}`, got plan:\n{}",
            serde_json::to_string_pretty(&plan).unwrap()
        );
    }

    let has_trgm: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm')")
            .fetch_one(&st.db)
            .await
            .unwrap();
    if has_trgm {
        let trgm_sql =
            "EXPLAIN (FORMAT JSON) SELECT id FROM users WHERE name ILIKE '%Daniel Sample Nair%' LIMIT 20".to_string();
        let plan = explain_plan(&st.db, trgm_sql.clone()).await;
        assert!(
            plan_uses_index(&plan, "idx_users_name_trgm"),
            "expected idx_users_name_trgm for `{trgm_sql}`, got plan:\n{}",
            serde_json::to_string_pretty(&plan).unwrap()
        );
    }
}
