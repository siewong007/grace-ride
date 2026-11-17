mod common;
use axum::{
    http::{Method, StatusCode},
    Router,
};
use common::*;
use grace_ride::state::AppState;
use serde_json::{json, Value};
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};
async fn ok(app: &Router, path: &str, cookie: Option<&str>, body: Option<Value>) -> Value {
    let r = call(app, Method::POST, path, cookie, body).await;
    assert_eq!(r.status, StatusCode::OK, "{path}: {}", r.json);
    r.json
}
async fn pending_login() -> (Router, AppState, String, Value) {
    let (app, st) = setup().await;
    let cookie = login(&st, "member@example.org").await;
    let start = ok(
        &app,
        "/api/auth/passkey/register/start",
        Some(&cookie),
        None,
    )
    .await;
    let opts: CreationChallengeResponse = serde_json::from_value(start["options"].clone()).unwrap();
    let mut soft = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = Url::parse(ORIGIN).unwrap();
    let cred = soft.do_registration(origin.clone(), opts).unwrap();
    ok(
        &app,
        "/api/auth/passkey/register/finish",
        Some(&cookie),
        Some(json!({"challenge_id":start["challenge_id"],"credential":cred})),
    )
    .await;
    let start = ok(
        &app,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({"email":"member@example.org"})),
    )
    .await;
    let opts: RequestChallengeResponse = serde_json::from_value(start["options"].clone()).unwrap();
    let cred = soft.do_authentication(origin, opts).unwrap();
    (
        app,
        st,
        cookie,
        json!({"challenge_id":start["challenge_id"],"credential":cred}),
    )
}
#[tokio::test]
async fn revoked_key_cannot_finish_login_after_recovery() {
    let (app, st, old_cookie, assertion) = pending_login().await;
    let admin = login(&st, "admin@example.org").await;
    ok(
        &app,
        "/api/auth/recovery-request",
        None,
        Some(json!({"email":"member@example.org","reason":"Lost phone"})),
    )
    .await;
    let rid:i64=sqlx::query_scalar("SELECT id FROM recovery_requests WHERE email='member@example.org' ORDER BY id DESC LIMIT 1").fetch_one(&st.db).await.unwrap();
    ok(
        &app,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&admin),
        None,
    )
    .await;
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM passkeys WHERE user_id=(SELECT id FROM users WHERE email='member@example.org')").fetch_one(&st.db).await.unwrap();
    assert_eq!(n, 0);
    assert_eq!(
        call(&app, Method::GET, "/api/auth/me", Some(&old_cookie), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let result = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(assertion),
    )
    .await;
    assert_eq!(
        result.status,
        StatusCode::BAD_REQUEST,
        "revoked credential must be rejected"
    );
}

#[tokio::test]
async fn expired_challenge_is_rejected() {
    let (app, st, _, assertion) = pending_login().await;
    sqlx::query("UPDATE auth_challenges SET created_at=$1 WHERE id=$2")
        .bind(grace_ride::util::minutes_ago(20))
        .bind(assertion["challenge_id"].as_str().unwrap())
        .execute(&st.db)
        .await
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(assertion),
    )
    .await;
    assert_eq!(
        r.status,
        StatusCode::BAD_REQUEST,
        "expired challenge must be rejected"
    );
}

#[tokio::test]
async fn challenge_cannot_be_replayed_concurrently() {
    let (app, st, _, assertion) = pending_login().await;
    let mut lock = st.db.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM auth_challenges WHERE id=$1 FOR UPDATE")
        .bind(assertion["challenge_id"].as_str().unwrap())
        .execute(&mut *lock)
        .await
        .unwrap();
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let (a, b) = (app.clone(), assertion.clone());
        tasks.push(tokio::spawn(async move {
            call(
                &a,
                Method::POST,
                "/api/auth/passkey/login/finish",
                None,
                Some(b),
            )
            .await
        }));
    }
    tokio::time::timeout(std::time::Duration::from_secs(10),async{loop{let n:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE cardinality(pg_blocking_pids(pid))>0 AND query LIKE 'DELETE FROM auth_challenges WHERE id=%'").fetch_one(&st.db).await.unwrap();if n>=2{break;}tokio::time::sleep(std::time::Duration::from_millis(20)).await;}}).await.unwrap_or_else(|_|panic!("wait for blocker {pid}"));
    lock.commit().await.unwrap();
    let mut statuses = Vec::new();
    for t in tasks {
        let r = t.await.unwrap();
        statuses.push(r.status);
    }
    assert!(
        statuses.contains(&StatusCode::OK),
        "one request should succeed"
    );
    assert!(
        statuses.contains(&StatusCode::BAD_REQUEST),
        "duplicate replay should be rejected"
    );
}
