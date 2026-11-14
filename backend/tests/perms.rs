//! Roles and permissions: page × action checks, editable roles, and the guard rules
//! that hold whatever an administrator configures.

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

fn has(me: &Value, perm: &str) -> bool {
    me["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p == perm)
}

#[tokio::test]
async fn me_lists_permissions_from_roles_and_hides_staff_ones_without_mfa() {
    let (app, st) = setup().await;

    let member = login(&st, "member@example.org").await;
    let me = call(&app, Method::GET, "/api/auth/me", Some(&member), None).await;
    assert!(has(&me.json, "rides.create"), "{}", me.json);
    assert!(has(&me.json, "events.view"));
    assert!(!has(&me.json, "events.create"));
    assert!(!has(&me.json, "drive.view"));
    assert!(!has(&me.json, "admin_rides.view"));
    assert_eq!(me.json["privileged"], false);

    // 管理员 acts as 交通组 and 活动负责人, never as finance or driver.
    let admin = login(&st, "admin@example.org").await;
    let me = call(&app, Method::GET, "/api/auth/me", Some(&admin), None).await;
    for p in [
        "admin_roles.update",
        "admin_members.update",
        "admin_rides.update",
        "events.create",
        "rides.create",
    ] {
        assert!(has(&me.json, p), "admin should hold {p}");
    }
    for p in ["admin_reimb_pay.view", "admin_bills.view", "drive.create"] {
        assert!(!has(&me.json, p), "admin should not hold {p}");
    }

    // Without a passkey / 2FA session the staff permissions are not usable.
    let weak = login_mfa(&st, "admin@example.org", false).await;
    let me = call(&app, Method::GET, "/api/auth/me", Some(&weak), None).await;
    assert_eq!(me.json["privileged"], true);
    assert!(!has(&me.json, "admin_roles.update"), "{}", me.json);
    assert!(has(&me.json, "events.create"));
}

#[tokio::test]
async fn role_list_carries_the_catalog_and_built_in_roles() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&admin), None).await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.json);

    let catalog = list.json["catalog"].as_array().unwrap();
    let rides = catalog.iter().find(|p| p["page"] == "admin_rides").unwrap();
    assert_eq!(rides["area"], "transport");
    assert_eq!(rides["actions"], json!(["view", "update", "delete"]));

    for key in [
        "member",
        "admin",
        "transport",
        "finance",
        "leader",
        "driver",
    ] {
        assert_eq!(role(&list.json, key)["system"], true, "{key}");
    }
    assert_eq!(role(&list.json, "member")["implicit"], true);
    assert!(perms_of(role(&list.json, "driver")).contains(&"drive.create".to_string()));
    assert!(role(&list.json, "admin")["locked"]
        .as_array()
        .unwrap()
        .contains(&json!("admin_roles.update")));

    // Members cannot read the role list; neither can staff without the permission.
    let member = login(&st, "member@example.org").await;
    let r = call(&app, Method::GET, "/api/admin/roles", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    let finance = login(&st, "finance@example.org").await;
    let r = call(&app, Method::GET, "/api/admin/roles", Some(&finance), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
}

#[tokio::test]
async fn custom_role_grants_exactly_the_actions_ticked() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let member_id = user_id(&st, "member@example.org").await;

    // Before: a plain member cannot see or publish announcements from the admin side.
    let post_ann = json!({"title": "Notice", "body": "Hello"});
    let r = call(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&member),
        Some(post_ann.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);

    // A role that may create announcements but not delete them.
    let made = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Welcome team",
            "description": "Posts notices",
            "permissions": ["admin_announcements.view", "admin_announcements.create"],
        })),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);
    let key = made.json["key"].as_str().unwrap().to_string();

    let give = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}"),
        Some(&admin),
        Some(json!({"roles": [key], "status": "active"})),
    )
    .await;
    assert_eq!(give.status, StatusCode::OK, "{}", give.json);

    // The role reaches a staff page, so the member's existing session applies at once
    // (dev-mode sessions are MFA-verified through `login`).
    let me = call(&app, Method::GET, "/api/auth/me", Some(&member), None).await;
    assert_eq!(me.json["privileged"], true);
    assert_eq!(me.json["role_names"][&key], "Welcome team");

    let created = call(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&member),
        Some(post_ann),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    let ann = created.json["id"].as_i64().unwrap();
    let del = call(
        &app,
        Method::DELETE,
        &format!("/api/announcements/{ann}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(del.status, StatusCode::FORBIDDEN, "{}", del.json);
    assert_eq!(del.json["code"], "NO_PERMISSION");

    // Ticking delete takes effect on the next request, no new sign-in needed.
    let upd = call(
        &app,
        Method::PUT,
        &format!("/api/admin/roles/{key}"),
        Some(&admin),
        Some(json!({
            "name": "Welcome team",
            "permissions": [
                "admin_announcements.view",
                "admin_announcements.create",
                "admin_announcements.delete",
            ],
        })),
    )
    .await;
    assert_eq!(upd.status, StatusCode::OK, "{}", upd.json);
    let del = call(
        &app,
        Method::DELETE,
        &format!("/api/announcements/{ann}"),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(del.status, StatusCode::OK, "{}", del.json);

    // Deleting the role takes it away from its holders.
    let gone = call(
        &app,
        Method::DELETE,
        &format!("/api/admin/roles/{key}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(gone.status, StatusCode::OK, "{}", gone.json);
    let (roles,): (String,) = sqlx::query_as("SELECT roles FROM users WHERE id=$1")
        .bind(member_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(roles, "");
    let r = call(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&member),
        Some(json!({"title": "Again", "body": "x"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
}

#[tokio::test]
async fn editing_the_member_role_changes_what_everyone_may_do() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;

    let list = call(&app, Method::GET, "/api/admin/roles", Some(&admin), None).await;
    let mut keep = perms_of(role(&list.json, "member"));
    assert!(keep.contains(&"help.create".to_string()));
    keep.retain(|p| p != "help.create");

    let upd = call(
        &app,
        Method::PUT,
        "/api/admin/roles/member",
        Some(&admin),
        Some(json!({ "permissions": keep })),
    )
    .await;
    assert_eq!(upd.status, StatusCode::OK, "{}", upd.json);

    let ask = call(
        &app,
        Method::POST,
        "/api/help",
        Some(&member),
        Some(json!({"subject": "Question", "body": "Hello"})),
    )
    .await;
    assert_eq!(ask.status, StatusCode::FORBIDDEN, "{}", ask.json);
    // Viewing was left ticked.
    let mine = call(&app, Method::GET, "/api/help", Some(&member), None).await;
    assert_eq!(mine.status, StatusCode::OK, "{}", mine.json);
}

#[tokio::test]
async fn role_edits_are_validated_and_built_ins_are_protected() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let put = |key: &'static str, body: Value| {
        let (app, admin) = (app.clone(), admin.clone());
        async move {
            call(
                &app,
                Method::PUT,
                &format!("/api/admin/roles/{key}"),
                Some(&admin),
                Some(body),
            )
            .await
        }
    };

    // An action the page does not offer, and a page that does not exist.
    for bad in ["report.delete", "nowhere.view", "admin_roles"] {
        let r = put("leader", json!({ "permissions": [bad] })).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}: {}", r.json);
        assert_eq!(r.json["code"], "INVALID_PERMISSION");
    }

    // The admin role keeps member and role management.
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&admin), None).await;
    let mut without = perms_of(role(&list.json, "admin"));
    without.retain(|p| p != "admin_roles.update");
    let r = put("admin", json!({ "permissions": without })).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "ROLE_PERMISSION_LOCKED");

    // Built-in roles cannot be deleted; custom ones need a unique name.
    let r = call(
        &app,
        Method::DELETE,
        "/api/admin/roles/driver",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "SYSTEM_ROLE_CANNOT_BE_DELETED");

    let new = |name: &'static str| {
        let (app, admin) = (app.clone(), admin.clone());
        async move {
            call(
                &app,
                Method::POST,
                "/api/admin/roles",
                Some(&admin),
                Some(json!({ "name": name, "permissions": [] })),
            )
            .await
        }
    };
    assert_eq!(new("  ").await.json["code"], "ROLE_NAME_REQUIRED");
    assert_eq!(new("Ushers").await.status, StatusCode::OK);
    assert_eq!(new("ushers").await.json["code"], "ROLE_NAME_EXISTS");

    // A role that does not exist cannot be given to a member.
    let member_id = user_id(&st, "member@example.org").await;
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}"),
        Some(&admin),
        Some(json!({"roles": ["no_such_role"], "status": "active"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "UNKNOWN_ROLE");
}

#[tokio::test]
async fn nobody_grants_themselves_finance_or_drops_their_own_role_access() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&admin), None).await;

    // Adding a finance permission to a role the caller holds is a self-grant…
    let mut with_pay = perms_of(role(&list.json, "admin"));
    with_pay.push("admin_reimb_pay.view".into());
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/roles/admin",
        Some(&admin),
        Some(json!({ "permissions": with_pay })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "CANNOT_GRANT_SELF_FINANCE");

    // …and so is adding it to the role every member holds.
    let mut everyone = perms_of(role(&list.json, "member"));
    everyone.push("admin_bills.view".into());
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/roles/member",
        Some(&admin),
        Some(json!({ "permissions": everyone })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "CANNOT_GRANT_SELF_FINANCE");

    // A custom finance role is fine to create, but not to take for oneself.
    let made = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({ "name": "Treasurer", "permissions": ["admin_bills.view"] })),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);
    let key = made.json["key"].as_str().unwrap().to_string();
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{admin_id}"),
        Some(&admin),
        Some(json!({"roles": ["admin", key], "status": "active"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(r.json["code"], "CANNOT_GRANT_SELF_FINANCE");

    // Dropping the admin role would leave the caller unable to manage roles.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{admin_id}"),
        Some(&admin),
        Some(json!({"roles": ["leader"], "status": "active"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.json);
    assert_eq!(r.json["code"], "CANNOT_DISABLE_SELF");
}

#[tokio::test]
async fn approving_members_does_not_carry_the_right_to_hand_out_roles() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let secretary = login(&st, "member@example.org").await;
    let secretary_id = user_id(&st, "member@example.org").await;
    let leader_id = user_id(&st, "enci@example.org").await;

    // 成员 view + update, without 角色与权限.
    let made = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Membership secretary",
            "permissions": ["admin_members.view", "admin_members.update"],
        })),
    )
    .await;
    let key = made.json["key"].as_str().unwrap().to_string();
    let give = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{secretary_id}"),
        Some(&admin),
        Some(json!({"roles": [key], "status": "active"})),
    )
    .await;
    assert_eq!(give.status, StatusCode::OK, "{}", give.json);

    // They can keep a member's roles as they are (e.g. to change groups)…
    let same = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{leader_id}"),
        Some(&secretary),
        Some(json!({"roles": ["leader"], "status": "active"})),
    )
    .await;
    assert_eq!(same.status, StatusCode::OK, "{}", same.json);

    // …but cannot make anyone, themselves included, an administrator.
    for target in [leader_id, secretary_id] {
        let r = call(
            &app,
            Method::PUT,
            &format!("/api/admin/users/{target}"),
            Some(&secretary),
            Some(json!({"roles": ["admin"], "status": "active"})),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
        assert_eq!(r.json["code"], "NO_PERMISSION");
    }
    // Nor edit roles.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/roles/{key}"),
        Some(&secretary),
        Some(json!({ "name": "x", "permissions": ["admin_roles.update"] })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
}

#[tokio::test]
async fn startup_sync_keeps_an_administrators_changes() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&admin), None).await;
    let mut keep = perms_of(role(&list.json, "driver"));
    keep.retain(|p| p != "drive.delete");
    let upd = call(
        &app,
        Method::PUT,
        "/api/admin/roles/driver",
        Some(&admin),
        Some(json!({ "permissions": keep.clone() })),
    )
    .await;
    assert_eq!(upd.status, StatusCode::OK, "{}", upd.json);

    // A restart must not hand the removed permission back.
    grace_ride::perms::sync(&st.db).await.unwrap();
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&admin), None).await;
    assert_eq!(perms_of(role(&list.json, "driver")), keep);
}

#[tokio::test]
async fn admin_settings_rates_crud_permissions_and_audit() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let admin = login(&st, "admin@example.org").await;

    // Member cannot view or update rates
    let get_forbidden = call(
        &app,
        Method::GET,
        "/api/admin/settings/rates",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(get_forbidden.status, StatusCode::FORBIDDEN);

    let put_forbidden = call(
        &app,
        Method::PUT,
        "/api/admin/settings/rates",
        Some(&member),
        Some(json!({
            "rates": [
                { "vehicle_type": "standard", "rate_cents_per_km": 40 }
            ]
        })),
    )
    .await;
    assert_eq!(put_forbidden.status, StatusCode::FORBIDDEN);

    // Admin can view rates
    let get_ok = call(
        &app,
        Method::GET,
        "/api/admin/settings/rates",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(get_ok.status, StatusCode::OK);
    assert!(get_ok.json["rates"].is_array());

    // Admin updates rates
    let put_ok = call(
        &app,
        Method::PUT,
        "/api/admin/settings/rates",
        Some(&admin),
        Some(json!({
            "rates": [
                { "vehicle_type": "standard", "rate_cents_per_km": 45 },
                { "vehicle_type": "other", "rate_cents_per_km": 65 }
            ]
        })),
    )
    .await;
    assert_eq!(put_ok.status, StatusCode::OK);

    // Verify rates were updated
    let get_after = call(
        &app,
        Method::GET,
        "/api/admin/settings/rates",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(get_after.status, StatusCode::OK);
    let rates = get_after.json["rates"].as_array().unwrap();
    let std_rate = rates
        .iter()
        .find(|r| r["vehicle_type"] == "standard")
        .unwrap();
    assert_eq!(std_rate["rate_cents_per_km"], 45);

    // Negative rates rejected
    let bad_rate = call(
        &app,
        Method::PUT,
        "/api/admin/settings/rates",
        Some(&admin),
        Some(json!({
            "rates": [
                { "vehicle_type": "standard", "rate_cents_per_km": -10 }
            ]
        })),
    )
    .await;
    assert_eq!(bad_rate.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn admin_settings_system_retention_crud_permissions_and_audit() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let admin = login(&st, "admin@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;

    // 1. Regular member cannot view or update system settings
    let get_forbidden = call(
        &app,
        Method::GET,
        "/api/admin/settings/system",
        Some(&member),
        None,
    )
    .await;
    assert_eq!(get_forbidden.status, StatusCode::FORBIDDEN);

    let put_forbidden = call(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&member),
        Some(json!({
            "audit_retention_days": 365,
            "chat_retention_days": 90,
            "max_live_drivers": 15
        })),
    )
    .await;
    assert_eq!(put_forbidden.status, StatusCode::FORBIDDEN);

    // 2. Admin views initial defaults
    let get_init = call(
        &app,
        Method::GET,
        "/api/admin/settings/system",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(get_init.status, StatusCode::OK);
    assert_eq!(get_init.json["audit_retention_days"], 730);
    assert_eq!(get_init.json["chat_retention_days"], 180);
    assert_eq!(get_init.json["max_live_drivers"], 10);
    assert_eq!(get_init.json["session_timeout_mins"], 15);

    // 3. Validation bounds rejected
    let bad_audit = call(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&admin),
        Some(json!({
            "audit_retention_days": 10, // too small (< 30)
            "chat_retention_days": 90,
            "max_live_drivers": 15,
            "session_timeout_mins": 15
        })),
    )
    .await;
    assert_eq!(bad_audit.status, StatusCode::BAD_REQUEST);

    let bad_timeout = call(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&admin),
        Some(json!({
            "audit_retention_days": 365,
            "chat_retention_days": 90,
            "max_live_drivers": 15,
            "session_timeout_mins": -5 // negative
        })),
    )
    .await;
    assert_eq!(bad_timeout.status, StatusCode::BAD_REQUEST);

    let bad_chat = call(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&admin),
        Some(json!({
            "audit_retention_days": 365,
            "chat_retention_days": 2, // too small (< 7)
            "max_live_drivers": 15
        })),
    )
    .await;
    assert_eq!(bad_chat.status, StatusCode::BAD_REQUEST);

    let bad_drivers = call(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&admin),
        Some(json!({
            "audit_retention_days": 365,
            "chat_retention_days": 90,
            "max_live_drivers": 0 // too small (< 1)
        })),
    )
    .await;
    assert_eq!(bad_drivers.status, StatusCode::BAD_REQUEST);

    // 4. Admin updates settings successfully
    let put_ok = call(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&admin),
        Some(json!({
            "audit_retention_days": 365,
            "chat_retention_days": 90,
            "max_live_drivers": 15,
            "session_timeout_mins": 30
        })),
    )
    .await;
    assert_eq!(put_ok.status, StatusCode::OK);

    // 5. Verify updated values
    let get_updated = call(
        &app,
        Method::GET,
        "/api/admin/settings/system",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(get_updated.status, StatusCode::OK);
    assert_eq!(get_updated.json["audit_retention_days"], 365);
    assert_eq!(get_updated.json["chat_retention_days"], 90);
    assert_eq!(get_updated.json["max_live_drivers"], 15);
    assert_eq!(get_updated.json["session_timeout_mins"], 30);

    // 6. Verify audit log entry
    let audit_row: (Option<i64>, String, String) = sqlx::query_as(
        "SELECT actor_id, action, detail FROM audit_logs WHERE action='settings.system' ORDER BY id DESC LIMIT 1"
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(audit_row.0, Some(admin_id));
    assert_eq!(audit_row.1, "settings.system");
    assert!(audit_row.2.contains("audit_retention_days: 365d"));
    assert!(audit_row.2.contains("max_live_drivers: 15"));
    assert!(audit_row.2.contains("session_timeout_mins: 30m"));

    // 7. Verify housekeeping retention uses the dynamic 365 days
    let old_ts = (chrono::Local::now() - chrono::Duration::days(366))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    let new_ts = (chrono::Local::now() - chrono::Duration::days(300))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let (old_id,): (i64,) = sqlx::query_as(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at)
         VALUES (NULL, 'test.old_ret', 'test', NULL, 'past new 365d limit', $1) RETURNING id",
    )
    .bind(&old_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let (new_id,): (i64,) = sqlx::query_as(
        "INSERT INTO audit_logs (actor_id, action, entity, entity_id, detail, created_at)
         VALUES (NULL, 'test.new_ret', 'test', NULL, 'within 365d limit', $1) RETURNING id",
    )
    .bind(&new_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    grace_ride::jobs::run_once(&st.db, &st.cfg).await.unwrap();

    let remaining_old: Option<i64> = sqlx::query_scalar("SELECT id FROM audit_logs WHERE id=$1")
        .bind(old_id)
        .fetch_optional(&st.db)
        .await
        .unwrap();
    assert!(
        remaining_old.is_none(),
        "Entry older than 365 days must be purged by dynamic retention"
    );

    let remaining_new: Option<i64> = sqlx::query_scalar("SELECT id FROM audit_logs WHERE id=$1")
        .bind(new_id)
        .fetch_optional(&st.db)
        .await
        .unwrap();
    assert_eq!(
        remaining_new,
        Some(new_id),
        "Entry within 365 days must be preserved"
    );
}
