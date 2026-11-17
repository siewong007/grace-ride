//! Church leadership roles: President, Vice-President, Secretary, Head of Traffic
//! and Cell Group Leader. They hold admin tools but not system configuration; only
//! the super admin, President and Vice-President may appoint or demote administrators.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::{json, Value};

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap();
    id
}

fn role<'a>(list: &'a Value, key: &str) -> &'a Value {
    list["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap_or_else(|| panic!("role {key} missing"))
}

fn perms_of(role: &Value) -> Vec<String> {
    role["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_string())
        .collect()
}

async fn set_roles(app: &axum::Router, cookie: &str, id: i64, roles: Value, ack: bool) -> Res {
    let mut body = json!({"roles": roles, "status": "active"});
    if ack {
        body["president_acknowledged"] = json!(true);
    }
    call(
        app,
        Method::PUT,
        &format!("/api/admin/users/{id}"),
        Some(cookie),
        Some(body),
    )
    .await
}

#[tokio::test]
async fn leadership_roles_have_admin_tools_but_not_system_configuration() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&root), None).await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.json);

    // President and Vice-President: run the office, approve people, manage roles,
    // see finance read-only and the settings page read-only, but never the audit log.
    for key in ["president", "vice_president"] {
        let def = role(&list.json, key);
        assert_eq!(def["system"], true, "{key} should be a built-in role");
        assert_eq!(def["assignable"], true, "{key} should be assignable");
        let perms = perms_of(def);
        for p in [
            "admin_members.update",
            "admin_invites.create",
            "admin_roles.update",
            "admin_settings.view",
            "admin_reimb_pay.view",
            "admin_bills.view",
            "admin_dashboard.view",
            "admin_rides.update",
            "admin_live.view",
            "admin_announcements.create",
        ] {
            assert!(
                perms.contains(&p.to_string()),
                "{key} should hold {p}: {perms:?}"
            );
        }
        for p in ["admin_settings.update", "admin_audit.view"] {
            assert!(
                !perms.contains(&p.to_string()),
                "{key} must not hold {p}: {perms:?}"
            );
        }
    }

    // Secretary keeps the office running (recovery, groups, reports, read-only roles)
    // but cannot invite or approve members, hand out roles, or reach settings.
    let secretary = perms_of(role(&list.json, "secretary"));
    for p in [
        "admin_recovery.update",
        "admin_groups.update",
        "admin_reports.view",
        "admin_roles.view",
    ] {
        assert!(
            secretary.contains(&p.to_string()),
            "secretary should hold {p}: {secretary:?}"
        );
    }
    for p in [
        "admin_invites.view",
        "admin_invites.create",
        "admin_members.update",
        "admin_roles.update",
        "admin_settings.view",
    ] {
        assert!(
            !secretary.contains(&p.to_string()),
            "secretary must not hold {p}: {secretary:?}"
        );
    }

    // Head of Traffic Management: dashboard, ride management, live map, plus events
    // and member oversight for coordination.
    let head = perms_of(role(&list.json, "traffic_head"));
    for p in [
        "admin_dashboard.view",
        "admin_rides.view",
        "admin_rides.update",
        "admin_live.view",
        "admin_events.update",
        "admin_reports.view",
        "admin_members.update",
    ] {
        assert!(
            head.contains(&p.to_string()),
            "traffic head should hold {p}: {head:?}"
        );
    }
    for p in ["admin_settings.view", "admin_roles.update"] {
        assert!(
            !head.contains(&p.to_string()),
            "traffic head must not hold {p}: {head:?}"
        );
    }

    // Cell Group Leader runs its own cell activity and its traffic, but only for events
    // it created, and never claims payments.
    let cell = perms_of(role(&list.json, "cell_leader"));
    for p in [
        "admin_plan.view",
        "admin_plan.update",
        "admin_rides.view",
        "admin_rides.update",
        "admin_rides.delete",
        "admin_live.view",
        "admin_reports.view",
        "admin_members.view",
    ] {
        assert!(
            cell.contains(&p.to_string()),
            "cell leader should hold {p}: {cell:?}"
        );
    }
    for p in [
        "admin_events.view",
        "admin_events.update",
        "admin_reimb_review.view",
        "admin_reimb_pay.view",
        "admin_bills.view",
        "admin_settings.view",
        "admin_roles.update",
    ] {
        assert!(
            !cell.contains(&p.to_string()),
            "cell leader must not hold {p}: {cell:?}"
        );
    }
}

#[tokio::test]
async fn cell_leader_manages_traffic_only_for_their_own_events() {
    let (app, st) = setup().await;
    let leader = user_id(&st, "cellleader@example.org").await;
    let admin = user_id(&st, "admin@example.org").await;
    let passenger = user_id(&st, "member@example.org").await;
    let driver_own = user_id(&st, "driver@example.org").await;
    let driver_other = user_id(&st, "driver2@example.org").await;
    let (area,): (i64,) = sqlx::query_as("SELECT id FROM areas ORDER BY id LIMIT 1")
        .fetch_one(&st.db)
        .await
        .unwrap();

    // One event the cell leader created, one someone else created, each with a ride.
    let mut events = Vec::new();
    let mut rides = Vec::new();
    for (i, (owner, title)) in [(leader, "Cell night"), (admin, "Other night")]
        .into_iter()
        .enumerate()
    {
        let (event,): (i64,) = sqlx::query_as(
            "INSERT INTO events (title, starts_at, location, created_by)
             VALUES ($1,$2,'Church hall',$3) RETURNING id",
        )
        .bind(title)
        .bind(tomorrow("19:30"))
        .bind(owner)
        .fetch_one(&st.db)
        .await
        .unwrap();
        let driver = if i == 0 { driver_own } else { driver_other };
        let (ride,): (i64,) = sqlx::query_as(
            "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id,
                                pickup_address, destination, seats, status)
             VALUES ($1,$2,$3,$4,$5,'1 Jalan Test','Church hall',1,'in_progress') RETURNING id",
        )
        .bind(passenger)
        .bind(driver)
        .bind(event)
        .bind(tomorrow("19:00"))
        .bind(area)
        .fetch_one(&st.db)
        .await
        .unwrap();
        events.push(event);
        rides.push(ride);
    }

    let cell = login(&st, "cellleader@example.org").await;

    // Their own event: the transport plan is available.
    let r = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", events[0]),
        Some(&cell),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // Another organiser's event: refused, and its ride is not visible.
    let r = call(
        &app,
        Method::GET,
        &format!("/api/admin/events/{}/plan", events[1]),
        Some(&cell),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    let r = call(
        &app,
        Method::GET,
        &format!("/api/rides/{}", rides[1]),
        Some(&cell),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);

    // The ride list and live map contain only their own event's traffic.
    let r = call(&app, Method::GET, "/api/admin/rides", Some(&cell), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let rows = r.json.as_array().unwrap();
    assert!(rows.iter().any(|row| row["id"] == rides[0]));
    assert!(
        rows.iter().all(|row| row["event"]["id"] == events[0]),
        "{}",
        r.json
    );

    let r = call(&app, Method::GET, "/api/admin/live", Some(&cell), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let drivers: Vec<i64> = r.json["drivers"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d["driver_id"].as_i64())
        .collect();
    assert!(drivers.contains(&driver_own), "{}", r.json);
    assert!(!drivers.contains(&driver_other), "{}", r.json);

    // A separate finance/claims role is what unlocks claims; a Cell Group Leader
    // cannot approve or pay reimbursements by itself.
    let r = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review",
        Some(&cell),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
}

#[tokio::test]
async fn treasurer_and_auditor_only_read_finance_and_member_reports() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&root), None).await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.json);

    // 财政 (Treasurer) verifies and pays, and reads the reports.
    let treasurer = role(&list.json, "finance");
    assert_eq!(
        treasurer["system"], true,
        "finance should be a built-in role"
    );
    let finance = perms_of(treasurer);
    for p in [
        "admin_reimb_pay.view",
        "admin_reimb_pay.update",
        "admin_bills.view",
        "admin_reports.view",
    ] {
        assert!(
            finance.contains(&p.to_string()),
            "treasurer should hold {p}: {finance:?}"
        );
    }
    for p in [
        "admin_members.view",
        "admin_rides.view",
        "admin_settings.view",
        "admin_audit.view",
        "admin_roles.view",
    ] {
        assert!(
            !finance.contains(&p.to_string()),
            "treasurer must not hold {p}: {finance:?}"
        );
    }

    // 查账 (Financial Auditor) reads the same records but may not pay or edit.
    let auditor_role = role(&list.json, "auditor");
    assert_eq!(
        auditor_role["system"], true,
        "auditor should be a built-in role"
    );
    let auditor = perms_of(auditor_role);
    for p in [
        "admin_reimb_pay.view",
        "admin_bills.view",
        "admin_reports.view",
    ] {
        assert!(
            auditor.contains(&p.to_string()),
            "auditor should hold {p}: {auditor:?}"
        );
    }
    for p in [
        "admin_reimb_pay.update",
        "admin_members.view",
        "admin_rides.view",
        "admin_settings.view",
        "admin_audit.view",
    ] {
        assert!(
            !auditor.contains(&p.to_string()),
            "auditor must not hold {p}: {auditor:?}"
        );
    }
}

#[tokio::test]
async fn startup_sync_gives_a_new_built_in_role_its_default_permissions() {
    let (_app, st) = setup().await;
    // Simulate a database from before the leadership roles existed: no role row at all,
    // while every permission is already in the catalog.
    sqlx::query("DELETE FROM roles WHERE key = 'president'")
        .execute(&st.db)
        .await
        .unwrap();
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM role_permissions WHERE role_key = 'president'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(n, 0);

    grace_ride::perms::sync(&st.db).await.unwrap();

    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM role_permissions WHERE role_key = 'president'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(
        n > 0,
        "a newly added built-in role must receive its defaults"
    );
    let (settings,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM role_permissions WHERE role_key = 'president' AND permission = 'admin_settings.view')",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(
        settings,
        "president should regain its default settings view"
    );
}

#[tokio::test]
async fn only_superadmin_president_and_vice_appoint_or_demote_admins() {
    let (app, st) = setup().await;
    let member_id = user_id(&st, "member@example.org").await;

    // A plain administrator holds the roles page's update permission, but may not
    // hand out the admin role.
    let admin = login(&st, "admin@example.org").await;
    let r = set_roles(&app, &admin, member_id, json!(["admin"]), false).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "ADMIN_APPOINTMENT_RESTRICTED");

    for email in [
        "secretary@example.org",
        "traffichead@example.org",
        "cellleader@example.org",
    ] {
        let cookie = login(&st, email).await;
        let r = set_roles(&app, &cookie, member_id, json!(["admin"]), false).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{email}: {}", r.json);
        assert!(
            matches!(
                r.json["code"].as_str(),
                Some("ADMIN_APPOINTMENT_RESTRICTED") | Some("NO_PERMISSION")
            ),
            "{email}: {}",
            r.json
        );
    }

    // The Vice-President must explicitly confirm the President's acknowledgement.
    let vice = login(&st, "vicepresident@example.org").await;
    let r = set_roles(&app, &vice, member_id, json!(["admin"]), false).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "PRESIDENT_ACKNOWLEDGEMENT_REQUIRED");
    let r = set_roles(&app, &vice, member_id, json!(["admin"]), true).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // Once appointed, only the three leaders may demote the administrator.
    let r = set_roles(&app, &admin, member_id, json!([]), false).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "ADMIN_APPOINTMENT_RESTRICTED");
    let r = set_roles(&app, &vice, member_id, json!([]), false).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // The President and super admin need no acknowledgement and keep ultimate power.
    let president = login(&st, "president@example.org").await;
    let r = set_roles(&app, &president, member_id, json!(["admin"]), false).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let root = login(&st, "superadmin@example.org").await;
    let r = set_roles(&app, &root, member_id, json!([]), false).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
}

#[tokio::test]
async fn only_the_superadmin_can_grant_system_configuration_access() {
    let (app, st) = setup().await;
    let president = login(&st, "president@example.org").await;

    // Creating a new role that reaches the settings page is refused.
    let r = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&president),
        Some(json!({
            "name": "Settings sneaky",
            "permissions": ["admin_settings.view", "admin_settings.update"],
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "SETTINGS_SUPERADMIN_ONLY");

    // Adding settings access to an existing role is refused too.
    let list = call(
        &app,
        Method::GET,
        "/api/admin/roles",
        Some(&president),
        None,
    )
    .await;
    let mut perms = perms_of(role(&list.json, "secretary"));
    perms.push("admin_settings.view".into());
    perms.push("admin_settings.update".into());
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/roles/secretary",
        Some(&president),
        Some(json!({ "permissions": perms })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "SETTINGS_SUPERADMIN_ONLY");

    // The super admin retains ultimate power to configure the system.
    let root = login(&st, "superadmin@example.org").await;
    let r = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&root),
        Some(json!({
            "name": "Settings keeper",
            "permissions": ["admin_settings.view", "admin_settings.update"],
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
}
