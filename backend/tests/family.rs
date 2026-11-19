mod common;

use axum::http::{Method, StatusCode};
use common::{call, login, setup};
use serde_json::json;

async fn session_for(st: &grace_ride::state::AppState, email: &str) -> String {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let token =
        uuid::Uuid::new_v4().simple().to_string() + &uuid::Uuid::new_v4().simple().to_string();
    let token = token[..48].to_string();
    let expires = (grace_ride::util::local_now() + chrono::Duration::days(7))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,1,$3,$4)",
    )
    .bind(grace_ride::auth::hash_token(&token))
    .bind(id)
    .bind(&expires)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    format!("{}={token}", grace_ride::auth::SESSION_COOKIE)
}

async fn groups(st: &grace_ride::state::AppState) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM group_members")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn active_member_creates_one_family_and_pending_cannot() {
    let (app, st) = setup().await;
    let before = groups(&st).await;
    let owner = login(&st, "member@example.org").await;
    let empty = call(&app, Method::GET, "/api/me/family", Some(&owner), None).await;
    assert_eq!(empty.status, StatusCode::OK);
    assert!(empty.json["family"].is_null());

    let created = call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    assert_eq!(created.json["name"], "陈家");

    let view = call(&app, Method::GET, "/api/me/family", Some(&owner), None).await;
    assert_eq!(
        view.json["family"]["owner_id"],
        view.json["members"][0]["user_id"]
    );
    assert_eq!(view.json["members"][0]["name"], "陈法拉");
    assert!(view.json["members"][0].get("phone").is_none());
    assert!(view.json["members"][0].get("email").is_none());

    let again = call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "另一家"})),
    )
    .await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
    assert_eq!(again.json["code"], "ALREADY_IN_FAMILY");

    sqlx::query("INSERT INTO users (name, email, phone, roles, status, webauthn_uuid) VALUES ('待审','kin-pending@example.org','','','pending',$1)")
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(&st.db)
        .await
        .unwrap();
    let pending = session_for(&st, "kin-pending@example.org").await;
    let denied = call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&pending),
        Some(json!({"name": "不能建"})),
    )
    .await;
    assert_eq!(denied.json["code"], "FAMILY_ACTIVE_ONLY");
    assert_eq!(groups(&st).await, before);
}

#[tokio::test]
async fn owner_renames_and_deletes_member_leaves_and_owner_cannot() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let (fid,): (i64,) = sqlx::query_as("SELECT id FROM families")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let (amina,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='amina@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)")
        .bind(fid)
        .bind(amina)
        .bind(grace_ride::util::now())
        .execute(&st.db)
        .await
        .unwrap();

    let renamed = call(
        &app,
        Method::PATCH,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "法拉的家"})),
    )
    .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.json);

    let amina_cookie = login(&st, "amina@example.org").await;
    let nope = call(
        &app,
        Method::PATCH,
        "/api/me/family",
        Some(&amina_cookie),
        Some(json!({"name": "不行"})),
    )
    .await;
    assert_eq!(nope.json["code"], "NO_PERMISSION");

    let owner_leave = call(
        &app,
        Method::POST,
        "/api/me/family/leave",
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(owner_leave.json["code"], "FAMILY_OWNER_CANNOT_LEAVE");

    let left = call(
        &app,
        Method::POST,
        "/api/me/family/leave",
        Some(&amina_cookie),
        None,
    )
    .await;
    assert_eq!(left.status, StatusCode::OK, "{}", left.json);
    let (still_user,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE email='amina@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(still_user, 1);
    let (amina_rows,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM family_members WHERE user_id=$1")
            .bind(amina)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(amina_rows, 0);

    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)")
        .bind(fid)
        .bind(amina)
        .bind(grace_ride::util::now())
        .execute(&st.db)
        .await
        .unwrap();
    let removed = call(
        &app,
        Method::DELETE,
        &format!("/api/me/family/members/{amina}"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.json);
    let (account,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id=$1")
        .bind(amina)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(account, 1);

    let (owner_id,): (i64,) =
        sqlx::query_as("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    let self_remove = call(
        &app,
        Method::DELETE,
        &format!("/api/me/family/members/{owner_id}"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(self_remove.json["code"], "FAMILY_OWNER_CANNOT_LEAVE");

    let gone = call(&app, Method::DELETE, "/api/me/family", Some(&owner), None).await;
    assert_eq!(gone.status, StatusCode::OK, "{}", gone.json);
    let (families,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM families")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(families, 0);
    let (owner_account,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(owner_account, 1);
}

#[tokio::test]
async fn pending_member_can_read_and_leave_but_not_open_rides() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let (fid,): (i64,) = sqlx::query_as("SELECT id FROM families")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let (kid,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, status, webauthn_uuid) VALUES ('待审','kin-pending@example.org','','','pending',$1) RETURNING id",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .fetch_one(&st.db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)")
        .bind(fid)
        .bind(kid)
        .bind(grace_ride::util::now())
        .execute(&st.db)
        .await
        .unwrap();
    let pending = session_for(&st, "kin-pending@example.org").await;
    let view = call(&app, Method::GET, "/api/me/family", Some(&pending), None).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.json);
    assert_eq!(view.json["members"].as_array().unwrap().len(), 2);
    let rides = call(&app, Method::GET, "/api/rides", Some(&pending), None).await;
    assert_eq!(rides.status, StatusCode::FORBIDDEN);
    assert_eq!(rides.json["code"], "ACCOUNT_PENDING");
    let left = call(
        &app,
        Method::POST,
        "/api/me/family/leave",
        Some(&pending),
        None,
    )
    .await;
    assert_eq!(left.status, StatusCode::OK, "{}", left.json);
}

async fn set_phone(st: &grace_ride::state::AppState, email: &str, phone: &str) {
    sqlx::query("UPDATE users SET phone=$1 WHERE email=$2")
        .bind(phone)
        .bind(email)
        .execute(&st.db)
        .await
        .unwrap();
}

#[tokio::test]
async fn matching_member_gets_a_request_and_a_second_family_is_blocked() {
    let (app, st) = setup().await;
    set_phone(&st, "member@example.org", "012-300 0006").await;
    set_phone(&st, "amina@example.org", "019-700 1001").await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let added = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "+60 19-700 1001"})),
    )
    .await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.json);
    assert_eq!(added.json["kind"], "request");
    assert_eq!(added.json["name"], "阿米娜");
    let (users_before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let again = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "0197001001"})),
    )
    .await;
    assert_eq!(again.json["id"], added.json["id"]);
    let (notes,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE link='/me/family'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(notes >= 1);
    let (users_after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(users_before, users_after);

    let amina = login(&st, "amina@example.org").await;
    let incoming = call(&app, Method::GET, "/api/me/family", Some(&amina), None).await;
    assert!(incoming.json["family"].is_null());
    assert_eq!(incoming.json["incoming"]["family_name"], "陈家");
    let id = added.json["id"].as_i64().unwrap();
    let not_owner = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&amina),
        Some(json!({"phone": "019-700 1099"})),
    )
    .await;
    assert_eq!(not_owner.json["code"], "NO_PERMISSION");
    let accept = call(
        &app,
        Method::POST,
        &format!("/api/me/family/requests/{id}/accept"),
        Some(&amina),
        None,
    )
    .await;
    assert_eq!(accept.status, StatusCode::OK, "{}", accept.json);
    let joined = call(&app, Method::GET, "/api/me/family", Some(&amina), None).await;
    assert!(joined.json["incoming"].is_null());
    assert_eq!(joined.json["family"]["name"], "陈家");

    let owner2 = login(&st, "sophie@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner2),
        Some(json!({"name": "林家"})),
    )
    .await;
    let blocked = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner2),
        Some(json!({"phone": "019-700 1001"})),
    )
    .await;
    assert_eq!(blocked.json["code"], "ALREADY_IN_FAMILY");
}

#[tokio::test]
async fn decline_does_not_join_and_other_family_request_conflicts() {
    let (app, st) = setup().await;
    set_phone(&st, "amina@example.org", "019-700 1002").await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let added = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 1002"})),
    )
    .await;
    let amina = login(&st, "amina@example.org").await;
    let id = added.json["id"].as_i64().unwrap();
    let declined = call(
        &app,
        Method::POST,
        &format!("/api/me/family/requests/{id}/decline"),
        Some(&amina),
        None,
    )
    .await;
    assert_eq!(declined.status, StatusCode::OK, "{}", declined.json);
    let (members,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM family_members fm JOIN users u ON u.id=fm.user_id WHERE u.email='amina@example.org'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(members, 0);

    let again = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 1002"})),
    )
    .await;
    assert_eq!(again.status, StatusCode::OK, "{}", again.json);
    let owner2 = login(&st, "sophie@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner2),
        Some(json!({"name": "林家"})),
    )
    .await;
    let race = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner2),
        Some(json!({"phone": "019-700 1002"})),
    )
    .await;
    assert_eq!(race.json["code"], "FAMILY_REQUEST_PENDING");
}

#[tokio::test]
async fn unknown_phone_returns_the_same_link_and_bad_numbers_do_not() {
    let (app, st) = setup().await;
    let before_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&st.db)
        .await
        .unwrap();
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let link = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 2001"})),
    )
    .await;
    assert_eq!(link.json["kind"], "invite");
    assert!(link.json["url"].as_str().unwrap().contains("/invite/"));
    let again = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "+60 19-700 2001"})),
    )
    .await;
    assert_eq!(again.json["url"], link.json["url"]);
    let after_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(before_users, after_users);

    let own = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "012-300 0006"})),
    )
    .await;
    assert_eq!(own.json["code"], "FAMILY_OWN_PHONE");
    let empty = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": ""})),
    )
    .await;
    assert_eq!(empty.json["code"], "PHONE_INVALID");

    set_phone(&st, "amina@example.org", "019-700 1888").await;
    set_phone(&st, "sophie@example.org", "019-700 1888").await;
    let ambiguous = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 1888"})),
    )
    .await;
    assert_eq!(ambiguous.json["code"], "FAMILY_PHONE_AMBIGUOUS");

    set_phone(&st, "yawen@example.org", "019-700 1999").await;
    sqlx::query("UPDATE users SET status='disabled' WHERE email='yawen@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let disabled = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 1999"})),
    )
    .await;
    assert_eq!(disabled.json["code"], "FAMILY_PHONE_UNAVAILABLE");
    let (invites,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM invites WHERE kind='family'")
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(invites, 1);
    let (detail,): (String,) = sqlx::query_as(
        "SELECT detail FROM audit_logs WHERE action='family.invite' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(!detail.contains("019-700"));
}

#[tokio::test]
async fn ninth_counted_person_is_full_and_groups_stay_put() {
    let (app, st) = setup().await;
    let before = groups(&st).await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let (fid,): (i64,) = sqlx::query_as("SELECT id FROM families")
        .fetch_one(&st.db)
        .await
        .unwrap();
    for i in 0..7 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO users (name, email, phone, roles, status, webauthn_uuid) VALUES ($1,$2,'','','pending',$3) RETURNING id",
        )
        .bind(format!("Kin {i}"))
        .bind(format!("kin-full-{i}@example.org"))
        .bind(uuid::Uuid::new_v4().to_string())
        .fetch_one(&st.db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)",
        )
        .bind(fid)
        .bind(id)
        .bind(grace_ride::util::now())
        .execute(&st.db)
        .await
        .unwrap();
    }
    let full = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 3001"})),
    )
    .await;
    assert_eq!(full.json["code"], "FAMILY_FULL");
    assert_eq!(full.json["params"]["max"], 8);
    assert_eq!(groups(&st).await, before);
}

#[tokio::test]
async fn another_familys_open_link_blocks_the_number() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let link = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 2100"})),
    )
    .await;
    assert_eq!(link.json["kind"], "invite");
    let sophie = login(&st, "sophie@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&sophie),
        Some(json!({"name": "林家"})),
    )
    .await;
    let blocked = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&sophie),
        Some(json!({"phone": "+60 19-700 2100"})),
    )
    .await;
    assert_eq!(blocked.json["code"], "FAMILY_REQUEST_PENDING");
    let (invites,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM invites WHERE kind='family' AND used_at IS NULL")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(invites, 1);
    let again = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 2100"})),
    )
    .await;
    assert_eq!(again.json["url"], link.json["url"]);
}

#[tokio::test]
async fn family_link_accept_creates_a_pending_member_and_keeps_the_link_without_terms() {
    let (app, st) = setup().await;
    let before_notes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE link='/admin/members?status=pending'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let link = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 4001"})),
    )
    .await;
    let url = link.json["url"].as_str().unwrap();
    let token = url.rsplit('/').next().unwrap();
    let preview = call(
        &app,
        Method::GET,
        &format!("/api/invites/{token}"),
        None,
        None,
    )
    .await;
    assert_eq!(preview.status, StatusCode::OK, "{}", preview.json);
    assert_eq!(preview.json["kind"], "family");
    assert_eq!(preview.json["family_name"], "陈家");
    assert_eq!(preview.json["phone"], "019-700 4001");
    assert!(preview.json.get("members").is_none());

    let no_terms = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "新人",
            "email": "new-kin@example.org",
            "phone": "000",
            "terms": false
        })),
    )
    .await;
    assert_eq!(no_terms.json["code"], "TERMS_REQUIRED");
    let (still,): (Option<String>,) =
        sqlx::query_as("SELECT used_at FROM invites WHERE kind='family'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert!(still.is_none());

    let accepted = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "新人",
            "email": "new-kin@example.org",
            "phone": "000",
            "terms": true
        })),
    )
    .await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.json);
    let (status, phone, roles): (String, String, String) =
        sqlx::query_as("SELECT status, phone, roles FROM users WHERE email='new-kin@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(status, "pending");
    assert_eq!(phone, "019-700 4001");
    assert_eq!(roles, "");
    let (church,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM group_members gm JOIN users u ON u.id=gm.user_id WHERE u.email='new-kin@example.org'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(church, 0);
    let view = call(&app, Method::GET, "/api/me/family", Some(&owner), None).await;
    let names: Vec<_> = view.json["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"新人"));
    let after_notes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE link='/admin/members?status=pending'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert!(after_notes > before_notes);
}

#[tokio::test]
async fn accepting_after_delete_does_not_create_a_user() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let link = call(
        &app,
        Method::POST,
        "/api/me/family/people",
        Some(&owner),
        Some(json!({"phone": "019-700 4002"})),
    )
    .await;
    let token = link.json["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string();
    call(&app, Method::DELETE, "/api/me/family", Some(&owner), None).await;
    let accepted = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "新人",
            "email": "ghost-kin@example.org",
            "terms": true
        })),
    )
    .await;
    assert_eq!(accepted.status, StatusCode::NOT_FOUND);
    let (n,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE email='ghost-kin@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn family_name_is_visible_only_to_member_staff() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    let staff = login(&st, "admin@example.org").await;
    let listed = call(
        &app,
        Method::GET,
        "/api/admin/users?page=1&limit=50",
        Some(&staff),
        None,
    )
    .await;
    assert_eq!(listed.status, StatusCode::OK, "{}", listed.json);
    let row = listed.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == 6)
        .unwrap();
    assert_eq!(row["email"], "member@example.org");
    assert_eq!(row["family_name"], "陈家");

    let leader = login(&st, "enci@example.org").await;
    let picker = call(
        &app,
        Method::GET,
        "/api/admin/users?page=1&limit=50",
        Some(&leader),
        None,
    )
    .await;
    assert_eq!(picker.status, StatusCode::OK, "{}", picker.json);
    let item = picker.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == 6)
        .unwrap();
    assert!(item.get("family_name").is_none());
    assert!(item.get("email").is_none());
}
