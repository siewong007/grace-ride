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
