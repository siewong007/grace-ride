//! Server support for the admin UX plan: ride search, member status filter, announcement editing.
mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::{json, Value};

fn ids(body: &Value) -> Vec<i64> {
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn admin_rides_search_matches_passenger_driver_and_destination_in_any_case() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;

    sqlx::query("UPDATE users SET name = 'Zephyrine Quill' WHERE id = (SELECT passenger_id FROM rides WHERE id = 20004)")
        .execute(&st.db)
        .await
        .unwrap();
    sqlx::query("UPDATE rides SET destination = 'Quokka Hall' WHERE id = 20005")
        .execute(&st.db)
        .await
        .unwrap();

    let all = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(all.status, StatusCode::OK, "{}", all.json);
    let everything = all.json["total"].as_i64().unwrap();

    let by_passenger = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=50&q=zephyrine",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(by_passenger.status, StatusCode::OK, "{}", by_passenger.json);
    // The same passenger may have several rides: every hit is theirs, and 20004 is among them.
    assert!(ids(&by_passenger.json).contains(&20004));
    let items = by_passenger.json["items"].as_array().unwrap();
    assert!(items
        .iter()
        .all(|r| r["passenger"]["name"] == "Zephyrine Quill"));
    assert_eq!(
        by_passenger.json["total"].as_u64().unwrap() as usize,
        items.len()
    );
    assert!((items.len() as i64) < everything);

    let by_destination = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=50&q=QUOKKA",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(ids(&by_destination.json), [20005]);

    // A search combines with the other filters, and a blank one filters nothing.
    let blank = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&q=%20%20",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(blank.json["total"].as_i64().unwrap(), everything);
    let none = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=50&q=zephyrine&status=no_such_status",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(none.json["total"], 0);

    // The unpaged list (used by the dashboard) searches too.
    let unpaged = call(
        &app,
        Method::GET,
        "/api/admin/rides?q=quokka",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(unpaged.json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn admin_users_can_be_filtered_by_status() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;

    for (name, email, status) in [
        ("Pend One", "pend1@example.org", "pending"),
        ("Pend Two", "pend2@example.org", "pending"),
        ("Off One", "off1@example.org", "disabled"),
    ] {
        sqlx::query("INSERT INTO users (name, email, webauthn_uuid, status) VALUES ($1,$2,$3,$4)")
            .bind(name)
            .bind(email)
            .bind(format!("uuid-{email}"))
            .bind(status)
            .execute(&st.db)
            .await
            .unwrap();
    }

    let pending = call(
        &app,
        Method::GET,
        "/api/admin/users?page=1&limit=50&status=pending&sort=name&dir=asc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(pending.status, StatusCode::OK, "{}", pending.json);
    let names: Vec<&str> = pending.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Pend One", "Pend Two"]);
    assert_eq!(pending.json["total"], 2);

    // Status and role filters stack.
    let both = call(
        &app,
        Method::GET,
        "/api/admin/users?page=1&limit=50&status=pending&role=driver",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(both.status, StatusCode::OK, "{}", both.json);
    assert_eq!(both.json["total"], 0);

    let disabled = call(
        &app,
        Method::GET,
        "/api/admin/users?page=1&limit=50&status=disabled",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(disabled.json["total"], 1);
}

#[tokio::test]
async fn announcements_can_be_edited_and_pinned_without_notifying_again() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    let member = login(&st, "member@example.org").await;

    let made = call(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&root),
        Some(json!({"title": "Sunday servise", "body": "10am", "pinned": false, "notify_all": false})),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);
    let id = made.json["id"].as_i64().unwrap();

    let before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications")
        .fetch_one(&st.db)
        .await
        .unwrap();

    let edit = call(
        &app,
        Method::PUT,
        &format!("/api/announcements/{id}"),
        Some(&root),
        Some(json!({"title": "Sunday service", "body": "10:30am", "pinned": true})),
    )
    .await;
    assert_eq!(edit.status, StatusCode::OK, "{}", edit.json);

    let list = call(&app, Method::GET, "/api/announcements", Some(&member), None).await;
    let row = list
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
        .unwrap();
    assert_eq!(row["title"], "Sunday service");
    assert_eq!(row["body"], "10:30am");
    assert_eq!(row["pinned"], true);

    // Editing is not publishing: nobody is told again, and the change is on the audit log.
    let after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(before.0, after.0);
    let logged: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_logs WHERE action='announcement.update' AND entity_id=$1",
    )
    .bind(id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(logged.0, 1);

    let blank = call(
        &app,
        Method::PUT,
        &format!("/api/announcements/{id}"),
        Some(&root),
        Some(json!({"title": "  ", "body": "x", "pinned": false})),
    )
    .await;
    assert_eq!(blank.status, StatusCode::BAD_REQUEST, "{}", blank.json);

    let missing = call(
        &app,
        Method::PUT,
        "/api/announcements/9999999",
        Some(&root),
        Some(json!({"title": "x", "body": "y", "pinned": false})),
    )
    .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.json);

    // An ordinary member may not edit.
    let denied = call(
        &app,
        Method::PUT,
        &format!("/api/announcements/{id}"),
        Some(&member),
        Some(json!({"title": "hijack", "body": "x", "pinned": true})),
    )
    .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.json);
}

fn names(body: &Value) -> Vec<String> {
    body.as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].as_str().unwrap().to_string())
        .collect()
}

async fn add_member(
    st: &grace_ride::state::AppState,
    name: &str,
    email: &str,
    phone: &str,
    status: &str,
) {
    sqlx::query(
        "INSERT INTO users (name, email, phone, status, webauthn_uuid) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(name)
    .bind(email)
    .bind(phone)
    .bind(status)
    .bind(format!("uuid-{email}"))
    .execute(&st.db)
    .await
    .unwrap();
}

#[tokio::test]
async fn member_search_ignores_case_and_matches_email_for_staff() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    add_member(
        &st,
        "Xanthe Pickwell",
        "xanthe.p@example.org",
        "0187776655",
        "active",
    )
    .await;

    for q in ["xanthe", "PICKWELL", "XANTHE.P@"] {
        let r = call(
            &app,
            Method::GET,
            &format!("/api/admin/users?q={q}"),
            Some(&root),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        assert_eq!(names(&r.json), ["Xanthe Pickwell"], "q={q}");
    }
}

#[tokio::test]
async fn member_picker_without_member_access_matches_names_of_active_members_only() {
    let (app, st) = setup().await;
    // driver@example.org leads events (Events · update) but cannot open 成员.
    let leader = login(&st, "driver@example.org").await;
    add_member(
        &st,
        "Xanthe Pickwell",
        "xanthe.p@example.org",
        "0187776655",
        "active",
    )
    .await;
    add_member(
        &st,
        "Xanthe Gone",
        "xanthe.g@example.org",
        "0187776600",
        "disabled",
    )
    .await;

    let by_name = call(
        &app,
        Method::GET,
        "/api/admin/users?q=xanthe",
        Some(&leader),
        None,
    )
    .await;
    assert_eq!(by_name.status, StatusCode::OK, "{}", by_name.json);
    assert_eq!(names(&by_name.json), ["Xanthe Pickwell"]);
    assert!(by_name.json[0].get("phone").is_none());

    // Contact details can't be probed through the search.
    for q in ["0187776", "xanthe.p%40"] {
        let r = call(
            &app,
            Method::GET,
            &format!("/api/admin/users?q={q}"),
            Some(&leader),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        assert!(names(&r.json).is_empty(), "q={q}: {}", r.json);
    }

    // Asking for another status doesn't widen it.
    let disabled = call(
        &app,
        Method::GET,
        "/api/admin/users?q=xanthe&status=disabled",
        Some(&leader),
        None,
    )
    .await;
    assert_eq!(names(&disabled.json), ["Xanthe Pickwell"]);

    // Nor can contact details be read off the order: sorting by email falls back to
    // name order. (By email this pair would come out Pickwell, Abbot.)
    add_member(
        &st,
        "Xanthe Abbot",
        "xanthe.z@example.org",
        "0187776611",
        "active",
    )
    .await;
    for sort in ["email", "created_at"] {
        let r = call(
            &app,
            Method::GET,
            &format!("/api/admin/users?q=xanthe&sort={sort}"),
            Some(&leader),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        assert_eq!(
            names(&r.json),
            ["Xanthe Abbot", "Xanthe Pickwell"],
            "sort={sort}"
        );
    }
}

#[tokio::test]
async fn unpaged_member_list_applies_the_status_filter() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    add_member(
        &st,
        "Xanthe Pickwell",
        "xanthe.p@example.org",
        "0187776655",
        "active",
    )
    .await;
    add_member(
        &st,
        "Xanthe Gone",
        "xanthe.g@example.org",
        "0187776600",
        "disabled",
    )
    .await;

    let r = call(
        &app,
        Method::GET,
        "/api/admin/users?q=xanthe&status=disabled",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(names(&r.json), ["Xanthe Gone"]);
}

#[tokio::test]
async fn dashboard_counts_open_invites_and_claims_awaiting_payment() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;

    let before = call(&app, Method::GET, "/api/admin/dashboard", Some(&root), None).await;
    assert_eq!(before.status, StatusCode::OK, "{}", before.json);
    let invites0 = before.json["open_invites"].as_i64().unwrap();
    let pay0 = before.json["awaiting_payment"].as_i64().unwrap();

    let made = call(
        &app,
        Method::POST,
        "/api/admin/invites",
        Some(&root),
        Some(json!({})),
    )
    .await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.json);

    let file = upload_png(&app, &root).await;
    sqlx::query(
        "INSERT INTO reimbursements (user_id, amount_cents, receipt_file_id, status) \
         VALUES ((SELECT id FROM users ORDER BY id LIMIT 1), 500, $1, 'approved')",
    )
    .bind(file)
    .execute(&st.db)
    .await
    .unwrap();

    let after = call(&app, Method::GET, "/api/admin/dashboard", Some(&root), None).await;
    assert_eq!(after.json["open_invites"].as_i64().unwrap(), invites0 + 1);
    let expected: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM reimbursements WHERE status='approved'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(after.json["awaiting_payment"].as_i64().unwrap(), expected.0);
    assert!(expected.0 >= pay0);

    // An invite that has expired stops counting.
    sqlx::query("UPDATE invites SET expires_at = '2000-01-01T00:00:00' WHERE id = (SELECT MAX(id) FROM invites)")
        .execute(&st.db)
        .await
        .unwrap();
    let last = call(&app, Method::GET, "/api/admin/dashboard", Some(&root), None).await;
    assert_eq!(last.json["open_invites"].as_i64().unwrap(), invites0);
}
