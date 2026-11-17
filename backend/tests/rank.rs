//! Chain of command: super admin > President > Vice-President > Finance > Audit.
//! Each level appoints only the level directly below it, and commands every level
//! below. Roles outside the chain (admin, secretary, traffic head...) have no rank and
//! cannot touch ranked roles or the members who hold them.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use grace_ride::perms;
use serde_json::{json, Value};

async fn user_id(st: &grace_ride::state::AppState, email: &str) -> i64 {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap();
    id
}

async fn give(st: &grace_ride::state::AppState, email: &str, roles: &str) {
    sqlx::query("UPDATE users SET roles=$1 WHERE email=$2")
        .bind(roles)
        .bind(email)
        .execute(&st.db)
        .await
        .unwrap();
}

async fn roles_of(st: &grace_ride::state::AppState, email: &str) -> String {
    sqlx::query_scalar("SELECT roles FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap()
}

async fn put_user(app: &axum::Router, cookie: &str, id: i64, roles: Value, status: &str) -> Res {
    call(
        app,
        Method::PUT,
        &format!("/api/admin/users/{id}"),
        Some(cookie),
        Some(json!({"roles": roles, "status": status})),
    )
    .await
}

fn code(r: &Res) -> &str {
    r.json["code"].as_str().unwrap_or_default()
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn rank_helpers_follow_the_chain() {
    let chain = [
        "superadmin",
        "president",
        "vice_president",
        "finance",
        "auditor",
    ];
    for (a, actor) in chain.iter().enumerate() {
        for (t, target) in chain.iter().enumerate() {
            // Appoint: only the level directly below.
            assert_eq!(
                perms::can_appoint(&s(&[actor]), target),
                t == a + 1,
                "{actor} appoints {target}"
            );
            // Command: strictly below.
            assert_eq!(
                perms::outranks(&s(&[actor]), &s(&[target])),
                t > a,
                "{actor} commands {target}"
            );
            assert_eq!(
                perms::outranks_role(&s(&[actor]), target),
                t > a,
                "{actor} edits role {target}"
            );
        }
    }
    // Outside the chain: no appointing, no commanding anything ranked...
    for role in ["admin", "secretary", "traffic_head", "cell_leader", "c_abc"] {
        for target in chain {
            assert!(!perms::can_appoint(&s(&[role]), target));
            assert!(!perms::outranks(&s(&[role]), &s(&[target])));
            assert!(!perms::outranks_role(&s(&[role]), target));
        }
        // ...but unranked targets stay with the ordinary permission checks.
        assert!(perms::outranks(&s(&[role]), &s(&["secretary"])));
    }
    // The strongest ranked role decides, whatever else a member holds.
    assert_eq!(
        perms::rank_of(&s(&["admin", "finance", "president"])),
        Some(1)
    );
    assert_eq!(perms::rank_of(&s(&["admin"])), None);
    assert!(perms::can_appoint(
        &s(&["admin", "president"]),
        "vice_president"
    ));
}

#[tokio::test]
async fn each_level_appoints_only_the_level_below() {
    let (app, st) = setup().await;
    let a = user_id(&st, "member@example.org").await;
    let b = user_id(&st, "enci@example.org").await;
    let c = user_id(&st, "amina@example.org").await;
    let d = user_id(&st, "driver@example.org").await;
    // Start with no roles, so a role update only moves the chain role under test.
    for email in [
        "member@example.org",
        "enci@example.org",
        "amina@example.org",
        "driver@example.org",
    ] {
        give(&st, email, "").await;
    }

    let root = login(&st, "superadmin@example.org").await;
    // Super admin appoints the President, and nothing deeper.
    for deeper in ["vice_president", "finance", "auditor"] {
        let r = put_user(&app, &root, a, json!([deeper]), "active").await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "root→{deeper}: {}", r.json);
        assert_eq!(code(&r), "APPOINT_NEXT_LEVEL_ONLY");
    }
    let r = put_user(&app, &root, a, json!(["president"]), "active").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // President appoints the Vice-President only.
    let president = login(&st, "member@example.org").await;
    for other in ["president", "finance", "auditor"] {
        let r = put_user(&app, &president, b, json!([other]), "active").await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "president→{other}: {}",
            r.json
        );
        assert_eq!(code(&r), "APPOINT_NEXT_LEVEL_ONLY");
    }
    let r = put_user(&app, &president, b, json!(["vice_president"]), "active").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // Vice-President appoints Finance only.
    let vice = login(&st, "enci@example.org").await;
    for other in ["president", "vice_president", "auditor"] {
        let r = put_user(&app, &vice, c, json!([other]), "active").await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "vice→{other}: {}", r.json);
        assert_eq!(code(&r), "APPOINT_NEXT_LEVEL_ONLY");
    }
    let r = put_user(&app, &vice, c, json!(["finance"]), "active").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // Finance appoints Audit only, without being able to edit roles.
    let finance = login(&st, "amina@example.org").await;
    for other in ["president", "vice_president", "finance"] {
        let r = put_user(&app, &finance, d, json!([other]), "active").await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "finance→{other}: {}",
            r.json
        );
        assert_eq!(code(&r), "APPOINT_NEXT_LEVEL_ONLY");
    }
    let r = put_user(&app, &finance, d, json!(["auditor"]), "active").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(roles_of(&st, "driver@example.org").await, "auditor");
    let edit_role = call(
        &app,
        Method::PUT,
        "/api/admin/roles/c_nothing",
        Some(&finance),
        Some(json!({"name": "x", "permissions": []})),
    )
    .await;
    assert_eq!(
        edit_role.status,
        StatusCode::FORBIDDEN,
        "{}",
        edit_role.json
    );

    // Audit appoints nobody.
    give(&st, "member@example.org", "").await;
    let auditor = login(&st, "driver@example.org").await;
    let r = put_user(&app, &auditor, a, json!(["auditor"]), "active").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
}

#[tokio::test]
async fn upper_levels_command_every_level_below_and_nobody_above() {
    let (app, st) = setup().await;
    give(&st, "member@example.org", "president").await;
    give(&st, "enci@example.org", "vice_president").await;
    give(&st, "amina@example.org", "finance").await;
    give(&st, "driver@example.org", "auditor").await;
    let (pres, vice, fin, aud) = (
        user_id(&st, "member@example.org").await,
        user_id(&st, "enci@example.org").await,
        user_id(&st, "amina@example.org").await,
        user_id(&st, "driver@example.org").await,
    );
    let ranks = [
        ("member@example.org", pres, "president"),
        ("enci@example.org", vice, "vice_president"),
        ("amina@example.org", fin, "finance"),
        ("driver@example.org", aud, "auditor"),
    ];
    // Disable and re-enable: only strictly upward.
    for (i, (actor_email, _, actor_role)) in ranks.iter().enumerate() {
        // Auditors hold no member permissions, so cannot reach the endpoint at all.
        let cookie = login(&st, actor_email).await;
        for (j, (_, target_id, target_role)) in ranks.iter().enumerate() {
            if j == i {
                continue; // nobody disables themselves; covered elsewhere
            }
            let r = put_user(&app, &cookie, *target_id, json!([target_role]), "disabled").await;
            if j > i {
                assert_eq!(
                    r.status,
                    StatusCode::OK,
                    "{actor_role} disables {target_role}: {}",
                    r.json
                );
                let back =
                    put_user(&app, &cookie, *target_id, json!([target_role]), "active").await;
                assert_eq!(back.status, StatusCode::OK, "{}", back.json);
            } else {
                assert_eq!(
                    r.status,
                    StatusCode::FORBIDDEN,
                    "{actor_role} must not disable {target_role}: {}",
                    r.json
                );
                assert!(
                    matches!(code(&r), "RANK_TOO_LOW" | "NO_PERMISSION"),
                    "{actor_role}→{target_role}: {}",
                    r.json
                );
            }
        }
    }
    // Revoking a ranked role is a command too: Vice-President takes Finance away, Finance
    // cannot take the Vice-President's role.
    let vice_cookie = login(&st, "enci@example.org").await;
    let fin_cookie = login(&st, "amina@example.org").await;
    let r = put_user(&app, &fin_cookie, vice, json!([]), "active").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(code(&r), "RANK_TOO_LOW");
    let r = put_user(&app, &vice_cookie, fin, json!([]), "active").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // The super admin commands everyone.
    let root = login(&st, "superadmin@example.org").await;
    for (_, id, role) in &ranks[..2] {
        let r = put_user(&app, &root, *id, json!([role]), "disabled").await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    }
}

#[tokio::test]
async fn nobody_moves_themselves_up_the_chain() {
    let (app, st) = setup().await;
    give(&st, "enci@example.org", "vice_president").await;
    let vice_id = user_id(&st, "enci@example.org").await;
    let vice = login(&st, "enci@example.org").await;
    for role in ["president", "finance"] {
        let r = put_user(&app, &vice, vice_id, json!([role]), "active").await;
        assert!(r.status.is_client_error(), "{role}: {}", r.json);
    }
    assert_eq!(roles_of(&st, "enci@example.org").await, "vice_president");
}

#[tokio::test]
async fn roles_outside_the_chain_cannot_touch_it() {
    let (app, st) = setup().await;
    let member = user_id(&st, "member@example.org").await;
    give(&st, "enci@example.org", "president").await;
    give(&st, "amina@example.org", "vice_president").await;
    let president = user_id(&st, "enci@example.org").await;
    let vice = user_id(&st, "amina@example.org").await;

    // A plain administrator cannot appoint a ranked role, to anyone or to themselves.
    let admin = login(&st, "admin@example.org").await;
    let admin_id = user_id(&st, "admin@example.org").await;
    for (target, role) in [
        (member, "president"),
        (admin_id, "president"),
        (member, "auditor"),
    ] {
        let r = put_user(&app, &admin, target, json!(["admin", role]), "active").await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "admin→{role}: {}", r.json);
        assert!(
            matches!(
                code(&r),
                "APPOINT_NEXT_LEVEL_ONLY" | "CANNOT_GRANT_SELF_FINANCE"
            ),
            "admin→{role}: {}",
            r.json
        );
    }
    // Nor disable, edit or demote anyone in the chain.
    for target in [president, vice] {
        let r = put_user(&app, &admin, target, json!([]), "active").await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
        assert_eq!(code(&r), "RANK_TOO_LOW");
    }
    // Head of Traffic holds members.update: still cannot disable the President or an
    // administrator.
    let traffic = login(&st, "traffichead@example.org").await;
    let r = put_user(&app, &traffic, president, json!(["president"]), "disabled").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(code(&r), "RANK_TOO_LOW");
    let r = put_user(&app, &traffic, admin_id, json!(["admin"]), "disabled").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(code(&r), "ADMIN_APPOINTMENT_RESTRICTED");
    assert_eq!(roles_of(&st, "enci@example.org").await, "president");
    let active: String = sqlx::query_scalar("SELECT status FROM users WHERE id=$1")
        .bind(admin_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(active, "active");
}

#[tokio::test]
async fn role_edits_follow_rank() {
    let (app, st) = setup().await;
    give(&st, "enci@example.org", "president").await;
    give(&st, "amina@example.org", "vice_president").await;
    let admin = login(&st, "admin@example.org").await;
    let president = login(&st, "enci@example.org").await;
    let vice = login(&st, "amina@example.org").await;
    let root = login(&st, "superadmin@example.org").await;

    let list = call(&app, Method::GET, "/api/admin/roles", Some(&root), None).await;
    let perms_of = |key: &str| -> Vec<String> {
        list.json["roles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == key)
            .unwrap()["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect()
    };
    let put_role = |cookie: String, key: &'static str, perms: Vec<String>| {
        let app = app.clone();
        async move {
            call(
                &app,
                Method::PUT,
                &format!("/api/admin/roles/{key}"),
                Some(&cookie),
                Some(json!({ "permissions": perms })),
            )
            .await
        }
    };

    // An administrator cannot edit any role in the chain, even to remove permissions.
    for key in ["president", "vice_president", "finance", "auditor"] {
        let mut p = perms_of(key);
        p.pop();
        let r = put_role(admin.clone(), key, p).await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "admin edits {key}: {}",
            r.json
        );
        assert_eq!(code(&r), "RANK_TOO_LOW");
    }
    // The President edits the roles below, not their own.
    let mut p = perms_of("vice_president");
    p.pop();
    let r = put_role(president.clone(), "vice_president", p).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let r = put_role(president.clone(), "president", perms_of("president")).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(code(&r), "RANK_TOO_LOW");
    let list = call(&app, Method::GET, "/api/admin/roles", Some(&root), None).await;
    let perms_of = |key: &str| -> Vec<String> {
        list.json["roles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == key)
            .unwrap()["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect()
    };
    // The Vice-President cannot edit the President's role or their own, and cannot hand
    // Finance a permission the Vice-President lacks (the audit log).
    for key in ["president", "vice_president"] {
        let r = put_role(vice.clone(), key, perms_of(key)).await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "vice edits {key}: {}",
            r.json
        );
        assert_eq!(code(&r), "RANK_TOO_LOW");
    }
    let mut p = perms_of("finance");
    p.push("admin_audit.view".into());
    let r = put_role(vice.clone(), "finance", p.clone()).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(code(&r), "CANNOT_GRANT_UNHELD_PERMISSION");
    // The super admin holds every permission, so may.
    let r = put_role(root.clone(), "finance", p).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
}

#[tokio::test]
async fn the_power_to_edit_roles_is_not_handed_out_by_administrators() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let accomplice = user_id(&st, "member@example.org").await;

    // An administrator cannot create a role that carries admin_roles.update…
    let r = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&admin),
        Some(json!({
            "name": "Deputy",
            "permissions": ["admin_roles.update", "admin_members.update"]
        })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(code(&r), "ADMIN_APPOINTMENT_RESTRICTED");

    // …whereas the President can, and then an administrator still cannot assign it.
    give(&st, "enci@example.org", "president").await;
    let president = login(&st, "enci@example.org").await;
    let made = call(
        &app,
        Method::POST,
        "/api/admin/roles",
        Some(&president),
        Some(json!({
            "name": "Deputy",
            "permissions": ["admin_roles.update", "admin_members.update"]
        })),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);
    let key = made.json["key"].as_str().unwrap().to_string();
    let r = put_user(&app, &admin, accomplice, json!([key]), "active").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.json);
    assert_eq!(code(&r), "ADMIN_APPOINTMENT_RESTRICTED");
    let r = put_user(&app, &president, accomplice, json!([key]), "active").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
}
