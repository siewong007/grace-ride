//! One HTTP case for every backend branch the other suites do not already assert.
//!
//! Each test talks to the real handlers on a throwaway schema. Error keys that no
//! handler returns (retired copy, or a race that cannot be forced from one request)
//! are listed in `reachable_error_keys_match_handlers` rather than invented here.

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use common::*;
use grace_ride::{
    auth::hash_token,
    init, router,
    state::{AppState, Config},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

struct Cast {
    admin: String,
    transport: String,
    finance: String,
    driver: String,
    driver2: String,
    member: String,
    amina: String,
    sophie: String,
    leader: String,
    superadmin: String,
    admin_id: i64,
    finance_id: i64,
    driver_id: i64,
    driver2_id: i64,
    member_id: i64,
    sophie_id: i64,
    zhiqiang_id: i64,
    area_id: i64,
    all_group: i64,
    youth_group: i64,
}

async fn cast(_app: &Router, st: &AppState) -> Cast {
    let id = |email: &str| {
        let db = st.db.clone();
        let email = email.to_string();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email=$1")
                .bind(email)
                .fetch_one(&db)
                .await
                .unwrap()
        }
    };
    Cast {
        admin: login(st, "admin@example.org").await,
        transport: login(st, "transport@example.org").await,
        finance: login(st, "finance@example.org").await,
        driver: login(st, "driver@example.org").await,
        driver2: login(st, "driver2@example.org").await,
        member: login(st, "member@example.org").await,
        amina: login(st, "amina@example.org").await,
        sophie: login(st, "sophie@example.org").await,
        leader: login(st, "enci@example.org").await,
        superadmin: login(st, "superadmin@example.org").await,
        admin_id: id("admin@example.org").await,
        finance_id: id("finance@example.org").await,
        driver_id: id("driver@example.org").await,
        driver2_id: id("driver2@example.org").await,
        member_id: id("member@example.org").await,
        sophie_id: id("sophie@example.org").await,
        zhiqiang_id: id("zhiqiang@example.org").await,
        area_id: sqlx::query_scalar("SELECT id FROM areas WHERE active=1 ORDER BY id LIMIT 1")
            .fetch_one(&st.db)
            .await
            .unwrap(),
        all_group: sqlx::query_scalar("SELECT id FROM groups WHERE is_all=1")
            .fetch_one(&st.db)
            .await
            .unwrap(),
        youth_group: sqlx::query_scalar("SELECT id FROM groups WHERE name='青年团契'")
            .fetch_one(&st.db)
            .await
            .unwrap(),
    }
}

async fn must(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Value {
    let r = call(app, method.clone(), path, cookie, body).await;
    assert_eq!(r.status, StatusCode::OK, "{method} {path} {}", r.json);
    r.json
}

async fn deny(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
    status: StatusCode,
    code: &str,
) {
    let r = call(app, method.clone(), path, cookie, body).await;
    assert_eq!(r.status, status, "{method} {path} {}", r.json);
    assert_eq!(r.json["code"], code, "{method} {path} {}", r.json);
}

fn day(offset: i64, hhmm: &str) -> String {
    format!(
        "{}T{hhmm}",
        (grace_ride::util::today_local() + chrono::Duration::days(offset)).format("%Y-%m-%d")
    )
}

fn ride_body(area: i64, seats: i64, event_id: Option<i64>) -> Value {
    json!({
        "pickup_at": day(1, "15:00"),
        "pickup_area_id": area,
        "pickup_address": "Lot 21",
        "destination": "教会",
        "seats": seats,
        "event_id": event_id,
    })
}

async fn book(app: &Router, cookie: &str, area: i64, seats: i64) -> i64 {
    must(
        app,
        Method::POST,
        "/api/rides",
        Some(cookie),
        Some(ride_body(area, seats, None)),
    )
    .await["id"]
        .as_i64()
        .unwrap()
}

fn event_body(
    title: &str,
    kind: &str,
    starts: &str,
    deadline: &str,
    groups: &[i64],
    cap: i64,
) -> Value {
    json!({
        "title": title,
        "kind": kind,
        "description": "",
        "starts_at": starts,
        "location": "Hall",
        "capacity": cap,
        "signup_deadline": deadline,
        "group_ids": groups,
    })
}

async fn make_event(app: &Router, cookie: &str, groups: &[i64], cap: i64) -> i64 {
    must(
        app,
        Method::POST,
        "/api/events",
        Some(cookie),
        Some(event_body(
            "团契聚会",
            "other",
            &day(2, "10:00"),
            "",
            groups,
            cap,
        )),
    )
    .await["id"]
        .as_i64()
        .unwrap()
}

fn bill(
    title: &str,
    items: Value,
    subsidy: i64,
    mode: &str,
    people: Value,
    account: &str,
) -> Value {
    json!({
        "title": title,
        "items": items,
        "subsidy_cents": subsidy,
        "split_mode": mode,
        "participants": people,
        "payee_name": "组织者",
        "payee_account": account,
    })
}

fn people(ids: &[i64]) -> Value {
    Value::Array(ids.iter().map(|id| json!({"user_id": id})).collect())
}

async fn post_bytes(app: &Router, cookie: &str, name: &str, mime: &str, data: &[u8]) -> Res {
    let boundary = "XgrFLOW";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: {mime}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, ORIGIN)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    Res {
        status,
        headers,
        json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

async fn prod_app() -> Router {
    let dir = std::env::temp_dir().join(format!("gr-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        dev_mode: false,
        database_url: fresh_db().await,
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
        db: Default::default(),
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    router(state)
}

#[tokio::test]
async fn auth_session_password_and_totp_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;

    deny(
        &app,
        Method::GET,
        "/api/auth/me",
        None,
        None,
        StatusCode::UNAUTHORIZED,
        "UNAUTHORIZED",
    )
    .await;
    deny(
        &app,
        Method::GET,
        "/api/rides/999999",
        Some(&c.member),
        None,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;

    let blocked = call_with(
        &app,
        Method::POST,
        "/api/auth/logout",
        Some(&c.member),
        None,
        Some("https://evil.example"),
    )
    .await;
    assert_eq!(blocked.status, StatusCode::FORBIDDEN, "{}", blocked.json);
    assert!(blocked.json["error"].as_str().unwrap().contains("来源"));

    let users = must(&app, Method::GET, "/api/auth/dev-users", None, None).await;
    assert!(users
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u["email"] == "member@example.org"));
    let dev = must(
        &app,
        Method::POST,
        "/api/auth/dev-login",
        None,
        Some(json!({"email": "member@example.org"})),
    )
    .await;
    assert_eq!(dev["ok"], true);
    deny(
        &app,
        Method::POST,
        "/api/auth/dev-login",
        None,
        Some(json!({"email": "nobody@example.org"})),
        StatusCode::BAD_REQUEST,
        "INVALID_CREDENTIALS",
    )
    .await;
    sqlx::query("UPDATE users SET status='disabled' WHERE email='yawen@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        "/api/auth/dev-login",
        None,
        Some(json!({"email": "yawen@example.org"})),
        StatusCode::BAD_REQUEST,
        "INVALID_CREDENTIALS",
    )
    .await;

    let prod = prod_app().await;
    deny(
        &prod,
        Method::GET,
        "/api/auth/dev-users",
        None,
        None,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;
    deny(
        &prod,
        Method::POST,
        "/api/auth/dev-login",
        None,
        Some(json!({"email": "member@example.org"})),
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;

    deny(
        &app,
        Method::POST,
        "/api/auth/password",
        Some(&c.member),
        Some(json!({"current": PW, "new_password": "short"})),
        StatusCode::BAD_REQUEST,
        "PASSWORD_TOO_SHORT",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/auth/password",
        Some(&c.member),
        Some(json!({"current": "wrong-password", "new_password": "GraceRide2027!"})),
        StatusCode::BAD_REQUEST,
        "CURRENT_PASSWORD_INCORRECT",
    )
    .await;
    assert_eq!(
        must(
            &app,
            Method::POST,
            "/api/auth/password",
            Some(&c.member),
            Some(json!({"current": PW, "new_password": "GraceRide2027!"})),
        )
        .await["ok"],
        true
    );

    deny(
        &app,
        Method::POST,
        "/api/auth/totp/verify",
        Some(&c.member),
        Some(json!({"code": "123456"})),
        StatusCode::BAD_REQUEST,
        "TWO_FACTOR_NOT_CONFIGURED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/auth/totp/backup-codes/regenerate",
        Some(&c.member),
        None,
        StatusCode::BAD_REQUEST,
        "TWO_FACTOR_NOT_CONFIGURED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&c.member),
        Some(json!({"code": "000000"})),
        StatusCode::BAD_REQUEST,
        "START_SETUP_FIRST",
    )
    .await;

    must(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&c.member),
        None,
    )
    .await;
    sqlx::query("UPDATE users SET totp_secret='!!!' WHERE id=$1")
        .bind(c.member_id)
        .execute(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&c.member),
        Some(json!({"code": "000000"})),
        StatusCode::BAD_REQUEST,
        "INVALID_2FA_SECRET",
    )
    .await;
    let setup_totp = must(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&c.member),
        None,
    )
    .await;
    let secret = setup_totp["secret"].as_str().unwrap().to_string();
    deny(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&c.member),
        Some(json!({"code": "000000"})),
        StatusCode::BAD_REQUEST,
        "INVALID_CODE",
    )
    .await;
    let code = totp_code(&secret, "member@example.org");
    assert_eq!(
        must(
            &app,
            Method::POST,
            "/api/auth/totp/enable",
            Some(&c.member),
            Some(json!({"code": code})),
        )
        .await["ok"],
        true
    );
    // Once enabled, a second enable is refused before the code is checked (no silent re-keying).
    deny(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&c.member),
        Some(json!({"code": code})),
        StatusCode::BAD_REQUEST,
        "TWO_FACTOR_ALREADY_ENABLED",
    )
    .await;

    let keys = must(
        &app,
        Method::GET,
        "/api/auth/passkeys",
        Some(&c.sophie),
        None,
    )
    .await;
    assert!(keys.as_array().unwrap().is_empty());
    assert_eq!(
        must(
            &app,
            Method::DELETE,
            "/api/auth/passkeys/999999",
            Some(&c.sophie),
            None,
        )
        .await["ok"],
        true
    );
    assert_eq!(
        must(
            &app,
            Method::POST,
            "/api/auth/terms",
            Some(&c.member),
            Some(json!({"accepted": true})),
        )
        .await["ok"],
        true
    );
}

#[tokio::test]
async fn invite_recovery_and_login_limit_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;

    let opened = must(
        &app,
        Method::POST,
        "/api/admin/invites",
        Some(&c.admin),
        None,
    )
    .await;
    let token = opened["token"].as_str().unwrap();
    let id = opened["id"].as_i64().unwrap();
    let accept = |body: Value| {
        let path = format!("/api/invites/{token}/accept");
        let app = app.clone();
        async move { call(&app, Method::POST, &path, None, Some(body)).await }
    };
    let bad_name = accept(
        json!({"name": "  ", "email": "flow@example.org", "phone": "012-300 0099", "terms": true}),
    )
    .await;
    assert_eq!(bad_name.json["code"], "NAME_REQUIRED");
    let bad_phone =
        accept(json!({"name": "新人", "email": "flow@example.org", "phone": "123", "terms": true}))
            .await;
    assert_eq!(bad_phone.json["code"], "PHONE_INVALID");
    let bad_mail = accept(
        json!({"name": "新人", "email": "not-an-email", "phone": "012-300 0099", "terms": true}),
    )
    .await;
    assert_eq!(bad_mail.json["code"], "EMAIL_INVALID");
    let taken = accept(json!({"name": "新人", "email": "member@example.org", "phone": "012-300 0099", "terms": true})).await;
    assert_eq!(taken.json["code"], "EMAIL_ALREADY_REGISTERED");
    let joined = accept(json!({"name": "新人", "email": "flow-new@example.org", "phone": "012-300 0099", "terms": true})).await;
    assert_eq!(joined.status, StatusCode::OK, "{}", joined.json);
    let again = accept(json!({"name": "新人", "email": "flow-other@example.org", "phone": "012-300 0099", "terms": true})).await;
    assert_eq!(again.json["code"], "INVITE_EXPIRED");

    let second = must(
        &app,
        Method::POST,
        "/api/admin/invites",
        Some(&c.admin),
        None,
    )
    .await;
    let second_token = second["token"].as_str().unwrap();
    must(
        &app,
        Method::DELETE,
        &format!("/api/admin/invites/{id}"),
        Some(&c.admin),
        None,
    )
    .await;
    // Revoking the already-used invite is a no-op; revoke the open one.
    must(
        &app,
        Method::DELETE,
        &format!("/api/admin/invites/{}", second["id"].as_i64().unwrap()),
        Some(&c.admin),
        None,
    )
    .await;
    // Revoke stores the current second, and preview treats that same second as still open.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    deny(
        &app,
        Method::GET,
        &format!("/api/invites/{second_token}"),
        None,
        None,
        StatusCode::BAD_REQUEST,
        "INVITE_EXPIRED",
    )
    .await;

    let raw = "recovery-link-without-a-user-account";
    sqlx::query(
        "INSERT INTO invites (token, kind, email, name, roles, group_ids, user_id, created_by, expires_at, created_at)
         VALUES ($1, 'recovery', 'member@example.org', '会员', '', '[]', NULL, $2, '2099-01-01T00:00:00', $3)",
    )
    .bind(hash_token(raw))
    .bind(c.admin_id)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/invites/{raw}/accept"),
        None,
        Some(json!({"terms": true})),
        StatusCode::BAD_REQUEST,
        "RECOVERY_LINK_INVALID",
    )
    .await;

    for _ in 0..3 {
        assert_eq!(
            must(
                &app,
                Method::POST,
                "/api/auth/recovery-request",
                None,
                Some(json!({"email": "member@example.org", "reason": "lost phone"})),
            )
            .await["ok"],
            true
        );
    }
    let limited = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({"email": "member@example.org", "reason": "again"})),
    )
    .await;
    assert_eq!(
        limited.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        limited.json
    );
    assert!(limited.headers.get(header::RETRY_AFTER).is_some());

    let rid: i64 = sqlx::query_scalar(
        "SELECT id FROM recovery_requests WHERE email='member@example.org' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    must(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/reject"),
        Some(&c.admin),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/reject"),
        Some(&c.admin),
        None,
        StatusCode::CONFLICT,
        "REQUEST_ALREADY_HANDLED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&c.admin),
        None,
        StatusCode::CONFLICT,
        "REQUEST_ALREADY_HANDLED",
    )
    .await;
}

#[tokio::test]
async fn passkey_challenge_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let origin = Url::parse(ORIGIN).unwrap();

    let start = |cookie: &str| {
        let app = app.clone();
        let cookie = cookie.to_string();
        async move {
            must(
                &app,
                Method::POST,
                "/api/auth/passkey/register/start",
                Some(&cookie),
                None,
            )
            .await
        }
    };
    let first = start(&c.member).await;
    let first_id = first["challenge_id"].as_str().unwrap().to_string();
    let options: CreationChallengeResponse =
        serde_json::from_value(first["options"].clone()).unwrap();
    let mut soft = SoftPasskey::new(true);
    let credential = soft
        .do_registration(origin, options)
        .expect("soft register");
    let cred = serde_json::to_value(&credential).unwrap();

    deny(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&c.sophie),
        Some(json!({"challenge_id": first_id, "credential": cred.clone(), "label": "other"})),
        StatusCode::FORBIDDEN,
        "CHALLENGE_MISMATCH",
    )
    .await;

    let second = start(&c.member).await;
    let second_id = second["challenge_id"].as_str().unwrap().to_string();
    let options: CreationChallengeResponse =
        serde_json::from_value(second["options"].clone()).unwrap();
    let mut soft = SoftPasskey::new(true);
    let credential = soft
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    // A credential built for this challenge, presented against a challenge that was never stored.
    deny(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&c.member),
        Some(json!({
            "challenge_id": "missing-challenge",
            "credential": credential,
        })),
        StatusCode::BAD_REQUEST,
        "VERIFICATION_EXPIRED",
    )
    .await;
    // The credential from the first ceremony cannot finish the second challenge.
    let first_again = start(&c.member).await;
    let options: CreationChallengeResponse =
        serde_json::from_value(first_again["options"].clone()).unwrap();
    let mut soft = SoftPasskey::new(true);
    let foreign = soft
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    deny(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&c.member),
        Some(json!({
            "challenge_id": second_id,
            "credential": foreign,
        })),
        StatusCode::BAD_REQUEST,
        "PASSKEY_REGISTRATION_FAILED",
    )
    .await;

    let enrolled = start(&c.member).await;
    let enrolled_id = enrolled["challenge_id"].as_str().unwrap().to_string();
    let options: CreationChallengeResponse =
        serde_json::from_value(enrolled["options"].clone()).unwrap();
    let mut soft = SoftPasskey::new(true);
    let credential = soft
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    must(
        &app,
        Method::POST,
        "/api/auth/passkey/register/finish",
        Some(&c.member),
        Some(json!({
            "challenge_id": enrolled_id,
            "credential": credential,
            "label": "phone"
        })),
    )
    .await;
    let login = must(
        &app,
        Method::POST,
        "/api/auth/passkey/login/start",
        None,
        Some(json!({"email": "member@example.org"})),
    )
    .await;
    let login_options: RequestChallengeResponse =
        serde_json::from_value(login["options"].clone()).unwrap();
    let assertion = soft
        .do_authentication(Url::parse(ORIGIN).unwrap(), login_options)
        .expect("soft auth");
    deny(
        &app,
        Method::POST,
        "/api/auth/passkey/login/finish",
        None,
        Some(json!({
            "challenge_id": "missing-login",
            "credential": assertion,
        })),
        StatusCode::BAD_REQUEST,
        "PASSKEY_FAILED",
    )
    .await;
}

#[tokio::test]
async fn event_create_signup_and_attendance_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let groups = [c.all_group];

    deny(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.member),
        Some(event_body("x", "other", &day(2, "10:00"), "", &groups, 0)),
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_CREATE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.admin),
        Some(event_body("  ", "other", &day(2, "10:00"), "", &groups, 0)),
        StatusCode::BAD_REQUEST,
        "EVENT_NAME_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.admin),
        Some(event_body(
            "聚会",
            "picnic",
            &day(2, "10:00"),
            "",
            &groups,
            0,
        )),
        StatusCode::BAD_REQUEST,
        "INVALID_EVENT_TYPE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.admin),
        Some(event_body("聚会", "other", &day(2, "10:00"), "", &[], 0)),
        StatusCode::BAD_REQUEST,
        "SELECT_VISIBLE_GROUP",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.admin),
        Some(event_body("聚会", "other", "tomorrow", "", &groups, 0)),
        StatusCode::BAD_REQUEST,
        "INVALID_EVENT_TIME",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.admin),
        Some(event_body(
            "聚会",
            "other",
            &day(2, "10:00"),
            "nope",
            &groups,
            0,
        )),
        StatusCode::BAD_REQUEST,
        "INVALID_SIGNUP_DEADLINE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.admin),
        Some(event_body(
            "聚会",
            "other",
            &day(2, "10:00"),
            &day(2, "11:00"),
            &groups,
            0,
        )),
        StatusCode::BAD_REQUEST,
        "DEADLINE_AFTER_EVENT",
    )
    .await;

    let id = make_event(&app, &c.admin, &groups, 1).await;
    let path = |suffix: &str| format!("/api/events/{id}{suffix}");
    deny(
        &app,
        Method::PUT,
        &path(""),
        Some(&c.member),
        Some(event_body(
            "改名",
            "other",
            &day(2, "10:00"),
            "",
            &groups,
            1,
        )),
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_EDIT",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &path("/cancel"),
        Some(&c.member),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_CANCEL",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &path("/finish"),
        Some(&c.member),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_PERFORM_ACTION",
    )
    .await;
    deny(
        &app,
        Method::GET,
        &path("/signups"),
        Some(&c.member),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_VIEW_SIGNUPS",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &path("/attendance"),
        Some(&c.member),
        Some(json!({"user_id": c.member_id, "attended": true})),
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_RECORD_ATTENDANCE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &path("/signup"),
        Some(&c.member),
        Some(json!({"offer_drive": true})),
        StatusCode::BAD_REQUEST,
        "DRIVER_REGISTRATION_REQUIRED",
    )
    .await;

    let youth = make_event(&app, &c.admin, &[c.youth_group], 0).await;
    deny(
        &app,
        Method::GET,
        &format!("/api/events/{youth}"),
        Some(&c.amina),
        None,
        StatusCode::FORBIDDEN,
        "EVENT_NOT_OPEN_TO_GROUP",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.amina),
        Some(ride_body(c.area_id, 1, Some(youth))),
        StatusCode::FORBIDDEN,
        "CANNOT_VIEW_EVENT",
    )
    .await;

    let closed = must(
        &app,
        Method::POST,
        "/api/events",
        Some(&c.admin),
        Some(event_body(
            "已过",
            "walk",
            &day(-1, "10:00"),
            "",
            &groups,
            0,
        )),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/events/{closed}/signup"),
        Some(&c.member),
        Some(json!({})),
        StatusCode::BAD_REQUEST,
        "SIGNUP_CLOSED",
    )
    .await;

    // A member in two groups must say which one they come as, and only one of theirs.
    deny(
        &app,
        Method::POST,
        &path("/signup"),
        Some(&c.member),
        Some(json!({"need_ride": true})),
        StatusCode::BAD_REQUEST,
        "JOIN_AS_REQUIRED",
    )
    .await;
    let visitor_group: i64 = sqlx::query_scalar("SELECT id FROM groups WHERE is_visitor=1")
        .fetch_one(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        &path("/signup"),
        Some(&c.member),
        Some(json!({"need_ride": true, "as_group_id": visitor_group})),
        StatusCode::BAD_REQUEST,
        "JOIN_AS_NOT_ALLOWED",
    )
    .await;
    assert_eq!(
        must(
            &app,
            Method::POST,
            &path("/signup"),
            Some(&c.member),
            Some(json!({"need_ride": true, "as_group_id": c.youth_group})),
        )
        .await["status"],
        "going"
    );
    assert_eq!(
        must(
            &app,
            Method::POST,
            &path("/signup"),
            Some(&c.sophie),
            Some(json!({})),
        )
        .await["status"],
        "waitlist"
    );
    must(
        &app,
        Method::POST,
        &path("/withdraw"),
        Some(&c.member),
        None,
    )
    .await;
    let signups = must(&app, Method::GET, &path("/signups"), Some(&c.admin), None).await;
    let sophie = signups
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["user_id"] == c.sophie_id)
        .unwrap();
    assert_eq!(sophie["status"], "going");

    // Walk-in attendance for someone who never signed up.
    must(
        &app,
        Method::POST,
        &path("/attendance"),
        Some(&c.admin),
        Some(json!({"user_id": c.driver_id, "attended": true})),
    )
    .await;
    sqlx::query("UPDATE events SET status='cancelled' WHERE id=$1")
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        &path("/signup"),
        Some(&c.amina),
        Some(json!({})),
        StatusCode::BAD_REQUEST,
        "EVENT_NOT_OPEN",
    )
    .await;

    let done = make_event(&app, &c.leader, &groups, 0).await;
    must(
        &app,
        Method::POST,
        &format!("/api/events/{done}/finish"),
        Some(&c.leader),
        None,
    )
    .await;
    // A finished event can no longer be cancelled, and a cancelled one cannot be finished.
    deny(
        &app,
        Method::POST,
        &format!("/api/events/{done}/cancel"),
        Some(&c.leader),
        None,
        StatusCode::BAD_REQUEST,
        "EVENT_NOT_OPEN",
    )
    .await;
    let dropped = make_event(&app, &c.leader, &groups, 0).await;
    must(
        &app,
        Method::POST,
        &format!("/api/events/{dropped}/cancel"),
        Some(&c.leader),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/events/{dropped}/finish"),
        Some(&c.leader),
        None,
        StatusCode::BAD_REQUEST,
        "EVENT_NOT_OPEN",
    )
    .await;
    let listed = must(&app, Method::GET, "/api/groups", Some(&c.leader), None).await;
    assert!(listed.as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn ride_request_claim_and_response_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;

    deny(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.member),
        Some(json!({
            "pickup_at": "yesterday",
            "pickup_area_id": c.area_id,
            "pickup_address": "Lot 21",
            "destination": "教会",
            "seats": 1
        })),
        StatusCode::BAD_REQUEST,
        "INVALID_PICKUP_TIME",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.member),
        Some(json!({
            "pickup_at": day(-1, "09:00"),
            "pickup_area_id": c.area_id,
            "pickup_address": "Lot 21",
            "destination": "教会",
            "seats": 1
        })),
        StatusCode::BAD_REQUEST,
        "PICKUP_TIME_IN_PAST",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.member),
        Some(json!({
            "pickup_at": day(1, "15:00"),
            "pickup_area_id": c.area_id,
            "pickup_address": "Lot 21",
            "destination": "教会",
            "seats": 9
        })),
        StatusCode::BAD_REQUEST,
        "SEATS_RANGE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.member),
        Some(json!({
            "pickup_at": day(1, "15:00"),
            "pickup_area_id": c.area_id,
            "pickup_address": "  ",
            "destination": "",
            "seats": 1
        })),
        StatusCode::BAD_REQUEST,
        "ADDRESS_AND_DESTINATION_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.member),
        Some(json!({
            "pickup_at": day(1, "15:00"),
            "pickup_area_id": 999999,
            "pickup_address": "Lot 21",
            "destination": "教会",
            "seats": 1
        })),
        StatusCode::BAD_REQUEST,
        "PICKUP_AREA_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.member),
        Some(json!({
            "pickup_at": day(1, "15:00"),
            "pickup_area_id": c.area_id,
            "pickup_address": "Lot 21",
            "destination": "教会",
            "seats": 1,
            "is_proxy": true
        })),
        StatusCode::BAD_REQUEST,
        "RIDER_NAME_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/proxy-riders",
        Some(&c.member),
        Some(json!({"name": "  "})),
        StatusCode::BAD_REQUEST,
        "RIDER_NAME_REQUIRED",
    )
    .await;
    let saved = must(
        &app,
        Method::POST,
        "/api/proxy-riders",
        Some(&c.member),
        Some(json!({"name": "妈妈", "phone": "012-300 0088", "relationship": "家人"})),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    let listed = must(
        &app,
        Method::GET,
        "/api/proxy-riders",
        Some(&c.member),
        None,
    )
    .await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    deny(
        &app,
        Method::DELETE,
        "/api/proxy-riders/999999",
        Some(&c.member),
        None,
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;
    must(
        &app,
        Method::DELETE,
        &format!("/api/proxy-riders/{saved}"),
        Some(&c.member),
        None,
    )
    .await;

    let areas = must(&app, Method::GET, "/api/areas", Some(&c.member), None).await;
    assert!(!areas.as_array().unwrap().is_empty());
    let places = must(&app, Method::GET, "/api/places?q=ab", Some(&c.member), None).await;
    assert_eq!(places.as_array().unwrap().len(), 0);

    let own = book(&app, &c.driver, c.area_id, 1).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{own}/claim"),
        Some(&c.driver),
        None,
        StatusCode::BAD_REQUEST,
        "CANNOT_PICK_OWN_RIDE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{own}/claim"),
        Some(&c.member),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_VOLUNTEER_DRIVERS_CAN_CHOOSE",
    )
    .await;
    deny(
        &app,
        Method::GET,
        "/api/driver/requests",
        Some(&c.member),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_VOLUNTEER_DRIVERS_CAN_VIEW",
    )
    .await;

    sqlx::query("DELETE FROM vehicles WHERE user_id=$1")
        .bind(c.zhiqiang_id)
        .execute(&st.db)
        .await
        .unwrap();
    let zhiqiang = login(&st, "zhiqiang@example.org").await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{own}/claim"),
        Some(&zhiqiang),
        None,
        StatusCode::BAD_REQUEST,
        "FILL_VEHICLE_DETAILS_FIRST",
    )
    .await;

    let tight = book(&app, &c.member, c.area_id, 2).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{tight}/claim"),
        Some(&c.driver2),
        None,
        StatusCode::BAD_REQUEST,
        "NOT_ENOUGH_SEATS_SLOT",
    )
    .await;

    let id = book(&app, &c.member, c.area_id, 1).await;
    deny(
        &app,
        Method::GET,
        &format!("/api/rides/{id}"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "CANNOT_VIEW_RIDE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/cancel"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_PASSENGER_OR_STAFF_CAN_CANCEL",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/claim"),
        Some(&c.driver),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/claim"),
        Some(&c.driver2),
        None,
        StatusCode::CONFLICT,
        "RIDE_PICKED_BY_ANOTHER_DRIVER",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/respond"),
        Some(&c.driver),
        Some(json!({"accept": true})),
        StatusCode::FORBIDDEN,
        "ONLY_PASSENGER_CAN_RESPOND",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/respond"),
        Some(&c.member),
        Some(json!({"accept": false})),
    )
    .await;
    // Decline returns the ride to the pool. A second response is no longer pending_passenger.
    let declined = call(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/respond"),
        Some(&c.member),
        Some(json!({"accept": true})),
    )
    .await;
    assert_eq!(declined.status, StatusCode::CONFLICT, "{}", declined.json);
    assert_eq!(declined.json["code"], "RIDE_STATUS_CHANGED");

    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/claim"),
        Some(&c.driver),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/withdraw"),
        Some(&c.driver2),
        Some(json!({"reason": "not mine"})),
        StatusCode::FORBIDDEN,
        "NOT_RIDE_DRIVER",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/withdraw"),
        Some(&c.driver),
        Some(json!({"reason": "车去保养"})),
    )
    .await;

    let schedule = must(
        &app,
        Method::GET,
        "/api/driver/schedule",
        Some(&c.driver),
        None,
    )
    .await;
    assert!(schedule.as_array().is_some());
    let route = must(
        &app,
        Method::GET,
        "/api/driver/route",
        Some(&c.member),
        None,
    )
    .await;
    assert!(route["route"].is_null());
    deny(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&c.member),
        Some(json!({"carpool_enabled": true, "spare_seats": 2, "area_ids": [c.area_id]})),
        StatusCode::FORBIDDEN,
        "ONLY_VOLUNTEER_DRIVERS_CAN_SET",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&c.driver),
        Some(json!({"carpool_enabled": true, "spare_seats": 0, "area_ids": []})),
        StatusCode::BAD_REQUEST,
        "SPARE_SEATS_RANGE",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&c.driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 2,
            "area_ids": [c.area_id],
            "vehicle": {"model": "", "color": "白", "plate": "", "seats": 5}
        })),
        StatusCode::BAD_REQUEST,
        "VEHICLE_MODEL_AND_PLATE_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&c.driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 2,
            "area_ids": [c.area_id],
            "vehicle": {"model": "Myvi", "color": "白", "plate": "ABC 123", "seats": 1}
        })),
        StatusCode::BAD_REQUEST,
        "VEHICLE_SEATS_RANGE",
    )
    .await;
    must(
        &app,
        Method::PUT,
        "/api/driver/profile",
        Some(&c.driver),
        Some(json!({
            "carpool_enabled": true,
            "spare_seats": 2,
            "area_ids": [c.area_id],
            "gate_wait_mins": 8,
            "vehicle": {"model": "Myvi", "color": "白", "plate": "VXX 1234", "seats": 5}
        })),
    )
    .await;
    let profile = must(
        &app,
        Method::GET,
        "/api/driver/profile",
        Some(&c.driver),
        None,
    )
    .await;
    assert_eq!(profile["gate_wait_mins"], 8);

    let drivers = must(
        &app,
        Method::GET,
        "/api/admin/drivers",
        Some(&c.transport),
        None,
    )
    .await;
    assert!(drivers
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"] == c.driver_id));
}

/// Walk one ride to confirmed, then exercise every trip, chat, and staff branch on fresh rides.
#[tokio::test]
async fn ride_trip_and_staff_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;

    async fn to_pending_approval(app: &Router, c: &Cast, seats: i64, driver: &str) -> i64 {
        let id = book(app, &c.member, c.area_id, seats).await;
        must(
            app,
            Method::POST,
            &format!("/api/rides/{id}/claim"),
            Some(driver),
            None,
        )
        .await;
        must(
            app,
            Method::POST,
            &format!("/api/rides/{id}/respond"),
            Some(&c.member),
            Some(json!({"accept": true})),
        )
        .await;
        id
    }

    let bare = book(&app, &c.member, c.area_id, 1).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{bare}/messages"),
        Some(&c.member),
        Some(json!({"body": "hi"})),
        StatusCode::BAD_REQUEST,
        "GROUP_CHAT_NOT_APPROVED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{bare}/approve"),
        Some(&c.transport),
        None,
        StatusCode::BAD_REQUEST,
        "NO_DRIVER_YET",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{bare}/readiness"),
        Some(&c.member),
        Some(json!({"status": "ready"})),
        StatusCode::CONFLICT,
        "TRIP_ALREADY_COMPLETED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{bare}/start"),
        Some(&c.driver),
        None,
        StatusCode::FORBIDDEN,
        "NOT_RIDE_DRIVER",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{bare}/no-show"),
        Some(&c.transport),
        None,
        StatusCode::CONFLICT,
        "RIDE_STATUS_CHANGED",
    )
    .await;

    let over = to_pending_approval(&app, &c, 2, &c.driver).await;
    sqlx::query("UPDATE users SET spare_seats=1 WHERE id=$1")
        .bind(c.driver_id)
        .execute(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{over}/approve"),
        Some(&c.transport),
        None,
        StatusCode::BAD_REQUEST,
        "NOT_ENOUGH_DRIVER_SEATS",
    )
    .await;
    sqlx::query("UPDATE users SET spare_seats=3 WHERE id=$1")
        .bind(c.driver_id)
        .execute(&st.db)
        .await
        .unwrap();

    let id = to_pending_approval(&app, &c, 1, &c.driver).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/reassign"),
        Some(&c.transport),
        Some(json!({"driver_id": 999999})),
        StatusCode::BAD_REQUEST,
        "DRIVER_NOT_FOUND",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/reassign"),
        Some(&c.transport),
        Some(json!({"driver_id": c.member_id})),
        StatusCode::BAD_REQUEST,
        "MEMBER_NOT_VOLUNTEER_DRIVER",
    )
    .await;
    // `id` belongs to the member. This booking belongs to the driver, so the same person cannot be both.
    let own = book(&app, &c.driver, c.area_id, 1).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{own}/reassign"),
        Some(&c.transport),
        Some(json!({"driver_id": c.driver_id})),
        StatusCode::BAD_REQUEST,
        "DRIVER_CANNOT_BE_PASSENGER",
    )
    .await;
    // driver2 offers one spare seat. A second seat on this clock does not fit.
    let wide = book(&app, &c.sophie, c.area_id, 2).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{wide}/reassign"),
        Some(&c.transport),
        Some(json!({"driver_id": c.driver2_id})),
        StatusCode::BAD_REQUEST,
        "NOT_ENOUGH_DRIVER_SEATS_REASSIGN",
    )
    .await;

    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/return"),
        Some(&c.transport),
        Some(json!({"reason": "时间不合"})),
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/claim"),
        Some(&c.driver),
        None,
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/respond"),
        Some(&c.member),
        Some(json!({"accept": true})),
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/approve"),
        Some(&c.transport),
        None,
    )
    .await;

    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/messages"),
        Some(&c.sophie),
        Some(json!({"body": "hello"})),
        StatusCode::FORBIDDEN,
        "ONLY_TRIP_MEMBERS_CAN_VIEW_CHAT",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/messages"),
        Some(&c.member),
        Some(json!({"body": "   "})),
        StatusCode::BAD_REQUEST,
        "INVALID_MESSAGE_LENGTH",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/messages"),
        Some(&c.member),
        Some(json!({"body": "我在门口"})),
    )
    .await;
    let chat = must(
        &app,
        Method::GET,
        &format!("/api/rides/{id}/messages"),
        Some(&c.driver),
        None,
    )
    .await;
    assert_eq!(chat["open"], true);

    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&c.sophie),
        Some(json!({"status": "ready"})),
        StatusCode::FORBIDDEN,
        "ONLY_PASSENGER_CAN_RESPOND",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&c.member),
        Some(json!({"status": "maybe"})),
        StatusCode::BAD_REQUEST,
        "INVALID_STATUS",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/readiness"),
        Some(&c.member),
        Some(json!({"status": "ready"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/driver-delay"),
        Some(&c.member),
        Some(json!({"delay_minutes": 5})),
        StatusCode::FORBIDDEN,
        "NOT_RIDE_DRIVER",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/driver-delay"),
        Some(&c.driver),
        Some(json!({"delay_minutes": 200})),
        StatusCode::BAD_REQUEST,
        "INVALID_ACTION",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/driver-delay"),
        Some(&c.driver),
        Some(json!({"delay_minutes": 10, "reason": "堵车"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/arrived-gate"),
        Some(&c.member),
        None,
        StatusCode::FORBIDDEN,
        "NOT_RIDE_DRIVER",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/arrived-gate"),
        Some(&c.driver),
        None,
    )
    .await;
    deny(
        &app,
        Method::GET,
        &format!("/api/rides/{id}/location"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "CANNOT_VIEW_LOCATION",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&c.driver),
        Some(json!({"lat": 120.0, "lng": 101.0})),
        StatusCode::BAD_REQUEST,
        "INVALID_LOCATION",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/pickup"),
        Some(&c.driver),
        None,
        StatusCode::BAD_REQUEST,
        "START_TRIP_FIRST",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&c.driver),
        Some(json!({"lat": 3.1, "lng": 101.6})),
        StatusCode::FORBIDDEN,
        "NOT_RIDE_PASSENGER",
    )
    .await;

    // Cap live drivers at 1, start this ride, then a second driver's start is refused.
    must(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&c.admin),
        Some(json!({
            "audit_retention_days": 730,
            "chat_retention_days": 180,
            "max_live_drivers": 1,
            "session_timeout_mins": 15
        })),
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/start"),
        Some(&c.driver),
        None,
    )
    .await;
    let second = to_pending_approval(&app, &c, 1, &c.driver2).await;
    // sophie is the passenger of `wide`, not of `second`. `second` belongs to member,
    // who is already on `id`. Use a fresh passenger by moving this booking.
    sqlx::query("UPDATE rides SET passenger_id=$1 WHERE id=$2")
        .bind(c.sophie_id)
        .bind(second)
        .execute(&st.db)
        .await
        .unwrap();
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{second}/approve"),
        Some(&c.transport),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{second}/start"),
        Some(&c.driver2),
        None,
        StatusCode::CONFLICT,
        "MAX_LIVE_DRIVERS_REACHED",
    )
    .await;
    // driver2 has carpool off, so this confirmed booking would block every later assignment in the same hour.
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{second}/cancel"),
        Some(&c.transport),
        Some(json!({"reason": "名额检查结束"})),
    )
    .await;

    must(
        &app,
        Method::POST,
        "/api/driver/location",
        Some(&c.driver),
        Some(json!({"lat": 3.1, "lng": 101.6, "heading": 90.0})),
    )
    .await;
    let sharing = must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&c.member),
        Some(json!({"lat": 3.11, "lng": 101.61})),
    )
    .await;
    assert_eq!(sharing["sharing"], true);
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&c.member),
        Some(json!({"lat": 400.0, "lng": 0.0})),
        StatusCode::BAD_REQUEST,
        "INVALID_LOCATION",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/pickup"),
        Some(&c.driver),
        None,
    )
    .await;
    must(
        &app,
        Method::DELETE,
        &format!("/api/rides/{id}/passenger-location"),
        Some(&c.member),
        None,
    )
    .await;
    // A passenger who has been picked up showed up: neither the driver nor staff can
    // record them as a no-show, and the ride stays in progress.
    for who in [&c.driver, &c.transport] {
        deny(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/no-show"),
            Some(who),
            None,
            StatusCode::CONFLICT,
            "PASSENGER_ALREADY_PICKED_UP",
        )
        .await;
    }
    let (status,): (String,) = sqlx::query_as("SELECT status FROM rides WHERE id=$1")
        .bind(id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(status, "in_progress");
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/complete"),
        Some(&c.driver),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/messages"),
        Some(&c.member),
        Some(json!({"body": "到了"})),
        StatusCode::BAD_REQUEST,
        "GROUP_CHAT_CLOSED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/driver-delay"),
        Some(&c.driver),
        Some(json!({"delay_minutes": 5})),
        StatusCode::CONFLICT,
        "TRIP_ALREADY_COMPLETED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{id}/cancel"),
        Some(&c.member),
        Some(json!({"reason": "too late"})),
        StatusCode::CONFLICT,
        "RIDE_STATUS_CHANGED",
    )
    .await;
    must(
        &app,
        Method::DELETE,
        "/api/driver/location",
        Some(&c.driver),
        None,
    )
    .await;

    let shown = to_pending_approval(&app, &c, 1, &c.driver).await;
    sqlx::query("UPDATE rides SET passenger_id=$1 WHERE id=$2")
        .bind(c.sophie_id)
        .bind(shown)
        .execute(&st.db)
        .await
        .unwrap();
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{shown}/approve"),
        Some(&c.transport),
        None,
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{shown}/no-show"),
        Some(&c.driver),
        None,
    )
    .await;

    let reassigned = book(&app, &c.member, c.area_id, 1).await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{reassigned}/reassign"),
        Some(&c.transport),
        Some(json!({"driver_id": c.driver2_id})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{reassigned}/confirm-assignment"),
        Some(&c.driver),
        None,
        StatusCode::FORBIDDEN,
        "NOT_RIDE_DRIVER",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{reassigned}/confirm-assignment"),
        Some(&c.driver2),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{reassigned}/confirm-assignment"),
        Some(&c.driver2),
        None,
        StatusCode::CONFLICT,
        "RIDE_STATUS_CHANGED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{reassigned}/return"),
        Some(&c.transport),
        Some(json!({"reason": "already confirmed"})),
        StatusCode::CONFLICT,
        "RIDE_STATUS_CHANGED",
    )
    .await;

    let alt = book(&app, &c.sophie, c.area_id, 1).await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{alt}/alt-transport"),
        Some(&c.transport),
        Some(json!({"reason": "没有司机"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{alt}/alt-transport"),
        Some(&c.transport),
        Some(json!({"reason": "已经安排"})),
        StatusCode::CONFLICT,
        "RIDE_STATUS_CHANGED",
    )
    .await;

    let drop = book(&app, &c.member, c.area_id, 1).await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{drop}/cancel"),
        Some(&c.member),
        Some(json!({"reason": "改期"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/rides/{drop}/cancel"),
        Some(&c.member),
        None,
        StatusCode::CONFLICT,
        "RIDE_STATUS_CHANGED",
    )
    .await;

    // Passenger says they cannot join, which cancels a confirmed ride.
    let leave = to_pending_approval(&app, &c, 1, &c.driver).await;
    sqlx::query("UPDATE rides SET passenger_id=$1 WHERE id=$2")
        .bind(c.admin_id)
        .bind(leave)
        .execute(&st.db)
        .await
        .unwrap();
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{leave}/approve"),
        Some(&c.transport),
        None,
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{leave}/readiness"),
        Some(&c.admin),
        Some(json!({"status": "unable_to_join", "notes": "身体不适"})),
    )
    .await;

    let past = must(
        &app,
        Method::GET,
        "/api/rides?scope=past&limit=5",
        Some(&c.member),
        None,
    )
    .await;
    assert!(
        past.get("items").is_some()
            || past.as_array().is_some()
            || past.get("rides").is_some()
            || past.is_object()
    );
    let live = must(
        &app,
        Method::GET,
        "/api/admin/live",
        Some(&c.transport),
        None,
    )
    .await;
    assert!(live.get("drivers").is_some());
}

#[tokio::test]
async fn settlement_bill_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let event = make_event(&app, &c.admin, &[c.all_group], 0).await;
    let create = format!("/api/events/{event}/settlements");
    let item = json!([{"label": "午餐", "amount_cents": 1000}]);
    let both = people(&[c.member_id, c.sophie_id]);

    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.member),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_INITIATE_COST_SPLIT",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "  ",
            item.clone(),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "SETTLEMENT_NAME_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            json!([]),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "AT_LEAST_ONE_EXPENSE_ITEM",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            json!([{"label": "", "amount_cents": 1000}]),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "EXPENSE_ITEM_NEGATIVE_OR_BLANK",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            -1,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "SUBSIDY_CANNOT_BE_NEGATIVE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            5000,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "SUBSIDY_EXCEEDS_TOTAL",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "equal",
            json!([{"user_id": c.member_id}, {"user_id": c.member_id}]),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "DUPLICATE_PARTICIPANT",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "equal",
            json!([]),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "SELECT_PARTICIPANTS",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "thirds",
            both.clone(),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "INVALID_SPLIT_MODE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "custom",
            json!([{"user_id": c.member_id}, {"user_id": c.sophie_id, "amount_cents": 500}]),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "MEMBER_AMOUNT_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "custom",
            json!([{"user_id": c.member_id, "amount_cents": -5}, {"user_id": c.sophie_id, "amount_cents": 1005}]),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "AMOUNT_CANNOT_BE_NEGATIVE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "custom",
            json!([{"user_id": c.member_id, "amount_cents": 100}, {"user_id": c.sophie_id, "amount_cents": 100}]),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "CUSTOM_SPLIT_MISMATCH",
    )
    .await;

    let foreign = upload_png(&app, &c.member).await;
    deny(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            json!([{"label": "午餐", "amount_cents": 1000, "receipt_file_id": foreign}]),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::FORBIDDEN,
        "INVALID_RECEIPT",
    )
    .await;

    let empty_payee = must(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill("午餐", item.clone(), 0, "equal", both.clone(), "")),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/settlements/{empty_payee}/publish"),
        Some(&c.admin),
        None,
        StatusCode::BAD_REQUEST,
        "PAYEE_DETAILS_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::GET,
        &format!("/api/settlements/{empty_payee}"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "NOT_ON_COST_SPLIT_LIST",
    )
    .await;

    let sid = must(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_PUBLISH",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&c.admin),
        None,
        StatusCode::BAD_REQUEST,
        "UNATTENDED_MEMBERS",
    )
    .await;
    for uid in [c.member_id, c.sophie_id] {
        must(
            &app,
            Method::POST,
            &format!("/api/events/{event}/attendance"),
            Some(&c.admin),
            Some(json!({"user_id": uid, "attended": true})),
        )
        .await;
    }
    must(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&c.admin),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&c.admin),
        None,
        StatusCode::BAD_REQUEST,
        "SETTLEMENT_ALREADY_PUBLISHED",
    )
    .await;
    deny(
        &app,
        Method::DELETE,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        None,
        StatusCode::BAD_REQUEST,
        "ONLY_DRAFTS_CAN_BE_DELETED",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.member),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_EDIT_BILL",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(bill(
            "午餐",
            item.clone(),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
        StatusCode::BAD_REQUEST,
        "REVISION_REASON_REQUIRED",
    )
    .await;
    let revised = bill("午餐", item.clone(), 0, "equal", both.clone(), "1234567890");
    let mut revised = revised;
    revised["reason"] = json!("加了饮料");
    revised["items"] = json!([{"label": "午餐", "amount_cents": 2000}]);
    must(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(revised),
    )
    .await;
    deny(
        &app,
        Method::GET,
        &format!("/api/settlements/{sid}/revisions"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "FORBIDDEN",
    )
    .await;
    let revisions = must(
        &app,
        Method::GET,
        &format!("/api/settlements/{sid}/revisions"),
        Some(&c.admin),
        None,
    )
    .await;
    assert!(!revisions.as_array().unwrap().is_empty());

    let view = must(
        &app,
        Method::GET,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        None,
    )
    .await;
    let share_member = view["shares"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["user_id"] == c.member_id)
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let share_sophie = view["shares"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["user_id"] == c.sophie_id)
        .unwrap()["id"]
        .as_i64()
        .unwrap();

    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/proof"),
        Some(&c.sophie),
        Some(json!({"proof_file_id": 1, "txn_ref": "TXN-1001"})),
        StatusCode::FORBIDDEN,
        "PROOF_ONLY_FOR_OWN_SHARE",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/proof"),
        Some(&c.member),
        Some(json!({"proof_file_id": 999999, "txn_ref": "TXN-1001"})),
        StatusCode::BAD_REQUEST,
        "UPLOAD_RECEIPT_FIRST",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/proof"),
        Some(&c.member),
        Some(json!({"proof_file_id": foreign, "txn_ref": "x"})),
        StatusCode::BAD_REQUEST,
        "TXN_REF_REQUIRED",
    )
    .await;
    let admin_file = upload_png(&app, &c.admin).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/proof"),
        Some(&c.member),
        Some(json!({"proof_file_id": admin_file, "txn_ref": "TXN-1001"})),
        StatusCode::FORBIDDEN,
        "INVALID_PROOF_DOCUMENT",
    )
    .await;
    let proof = upload_png(&app, &c.member).await;
    must(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/proof"),
        Some(&c.member),
        Some(json!({"proof_file_id": proof, "txn_ref": "TXN-1001"})),
    )
    .await;
    let sophie_proof = upload_png(&app, &c.sophie).await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_sophie}/proof"),
        Some(&c.sophie),
        Some(json!({"proof_file_id": sophie_proof, "txn_ref": "TXN-1001"})),
        StatusCode::BAD_REQUEST,
        "DUPLICATE_TXN_OR_PROOF",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/verify"),
        Some(&c.sophie),
        Some(json!({"action": "confirm"})),
        StatusCode::FORBIDDEN,
        "ONLY_PAYEE_CAN_VERIFY",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/verify"),
        Some(&c.admin),
        Some(json!({"action": "needs_info"})),
        StatusCode::BAD_REQUEST,
        "NEED_MORE_INFO_REASON_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/verify"),
        Some(&c.admin),
        Some(json!({"action": "drop"})),
        StatusCode::BAD_REQUEST,
        "INVALID_ACTION",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/verify"),
        Some(&c.admin),
        Some(json!({"action": "confirm"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/close"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "ONLY_ORGANISER_CAN_CLOSE_BILL",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/close"),
        Some(&c.admin),
        None,
        StatusCode::BAD_REQUEST,
        "OPEN_PAYMENTS_REMAIN",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/shares/{share_sophie}/proof"),
        Some(&c.sophie),
        Some(json!({"proof_file_id": sophie_proof, "txn_ref": "TXN-2002"})),
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/shares/{share_sophie}/verify"),
        Some(&c.admin),
        Some(json!({"action": "confirm"})),
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/close"),
        Some(&c.admin),
        None,
    )
    .await;
    deny(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some({
            let mut body = bill("午餐", item.clone(), 0, "equal", both.clone(), "1234567890");
            body["reason"] = json!("closed");
            body
        }),
        StatusCode::BAD_REQUEST,
        "BILL_CLOSED_CANNOT_EDIT",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{share_member}/verify"),
        Some(&c.admin),
        Some(json!({"action": "reset"})),
        StatusCode::BAD_REQUEST,
        "BILL_CLOSED_CANNOT_EDIT",
    )
    .await;

    // A draft can be deleted by its organiser. Proof on an unpublished share is refused.
    let draft = must(
        &app,
        Method::POST,
        &create,
        Some(&c.admin),
        Some(bill(
            "草稿",
            item,
            0,
            "equal",
            people(&[c.member_id]),
            "1234567890",
        )),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    let draft_view = must(
        &app,
        Method::GET,
        &format!("/api/settlements/{draft}"),
        Some(&c.admin),
        None,
    )
    .await;
    let draft_share = draft_view["shares"][0]["id"].as_i64().unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/shares/{draft_share}/proof"),
        Some(&c.member),
        Some(json!({"proof_file_id": proof, "txn_ref": "TXN-3003"})),
        StatusCode::BAD_REQUEST,
        "PROOF_NOT_NEEDED_NOW",
    )
    .await;
    must(
        &app,
        Method::DELETE,
        &format!("/api/settlements/{draft}"),
        Some(&c.admin),
        None,
    )
    .await;
    let mine = must(&app, Method::GET, "/api/me/shares", Some(&c.member), None).await;
    assert!(mine
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["settlement_id"] == sid));
}

#[tokio::test]
async fn reimbursement_help_and_profile_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let receipt = upload_png(&app, &c.member).await;
    let other = upload_png(&app, &c.sophie).await;
    let ride = book(&app, &c.sophie, c.area_id, 1).await;

    let reimb = |body: Value, cookie: &str| {
        let app = app.clone();
        let cookie = cookie.to_string();
        async move {
            call(
                &app,
                Method::POST,
                "/api/reimbursements",
                Some(&cookie),
                Some(body),
            )
            .await
        }
    };
    let base = json!({
        "amount_cents": 1500,
        "description": "Grab",
        "receipt_file_id": receipt,
        "payee_name": "陈法拉",
        "payee_phone": "012-300 0006"
    });
    let mut too_small = base.clone();
    too_small["amount_cents"] = json!(0);
    assert_eq!(
        reimb(too_small, &c.member).await.json["code"],
        "AMOUNT_RANGE_INVALID"
    );
    let mut too_big = base.clone();
    too_big["amount_cents"] = json!(50_001);
    assert_eq!(
        reimb(too_big, &c.member).await.json["code"],
        "AMOUNT_RANGE_INVALID"
    );
    let mut no_payee = base.clone();
    no_payee["payee_name"] = json!("  ");
    assert_eq!(
        reimb(no_payee, &c.member).await.json["code"],
        "PAYEE_NAME_AND_PHONE_REQUIRED"
    );
    let mut missing = base.clone();
    missing["receipt_file_id"] = json!(999999);
    assert_eq!(
        reimb(missing, &c.member).await.json["code"],
        "RECEIPT_REQUIRED"
    );
    let mut stolen = base.clone();
    stolen["receipt_file_id"] = json!(other);
    assert_eq!(
        reimb(stolen, &c.member).await.json["code"],
        "INVALID_RECEIPT"
    );
    let mut linked = base.clone();
    linked["ride_id"] = json!(ride);
    assert_eq!(
        reimb(linked, &c.member).await.json["code"],
        "ONLY_LINK_OWN_RIDES"
    );

    let id = reimb(base.clone(), &c.member).await.json["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::PUT,
        &format!("/api/reimbursements/{id}"),
        Some(&c.member),
        Some(base.clone()),
        StatusCode::BAD_REQUEST,
        "ONLY_RETURNED_REIMB_CAN_BE_RESUBMITTED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/pay"),
        Some(&c.finance),
        Some(json!({"payee_verified": true, "paid_ref": "PAY-1001"})),
        StatusCode::BAD_REQUEST,
        "ONLY_APPROVED_REIMB_CAN_BE_PAID",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/review"),
        Some(&c.transport),
        Some(json!({"action": "hold"})),
        StatusCode::BAD_REQUEST,
        "INVALID_ACTION",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/review"),
        Some(&c.transport),
        Some(json!({"action": "return"})),
        StatusCode::BAD_REQUEST,
        "REASON_REQUIRED",
    )
    .await;

    let super_receipt = upload_png(&app, &c.superadmin).await;
    let mut own = base.clone();
    own["receipt_file_id"] = json!(super_receipt);
    let own_id = reimb(own, &c.superadmin).await.json["id"].as_i64().unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{own_id}/review"),
        Some(&c.superadmin),
        Some(json!({"action": "approve"})),
        StatusCode::FORBIDDEN,
        "CANNOT_REVIEW_OWN_REQUEST",
    )
    .await;

    must(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/review"),
        Some(&c.transport),
        Some(json!({"action": "return", "note": "收据模糊"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/review"),
        Some(&c.transport),
        Some(json!({"action": "approve"})),
        StatusCode::BAD_REQUEST,
        "REIMB_ALREADY_REVIEWED",
    )
    .await;
    must(
        &app,
        Method::PUT,
        &format!("/api/reimbursements/{id}"),
        Some(&c.member),
        Some(base.clone()),
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/review"),
        Some(&c.transport),
        Some(json!({"action": "approve"})),
    )
    .await;
    sqlx::query("UPDATE reimbursements SET user_id=$1 WHERE id=$2")
        .bind(c.finance_id)
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/pay"),
        Some(&c.finance),
        Some(json!({"payee_verified": true, "paid_ref": "PAY-1001"})),
        StatusCode::FORBIDDEN,
        "CANNOT_PAY_OWN_REQUEST",
    )
    .await;
    sqlx::query("UPDATE reimbursements SET user_id=$1 WHERE id=$2")
        .bind(c.member_id)
        .bind(id)
        .execute(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/pay"),
        Some(&c.finance),
        Some(json!({"payee_verified": false, "paid_ref": "PAY-1001"})),
        StatusCode::BAD_REQUEST,
        "PAYEE_VERIFICATION_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/pay"),
        Some(&c.finance),
        Some(json!({"payee_verified": true, "paid_ref": "ab"})),
        StatusCode::BAD_REQUEST,
        "TXN_REF_REQUIRED",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/pay"),
        Some(&c.finance),
        Some(json!({"payee_verified": true, "paid_ref": "PAY-1001"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{id}/pay"),
        Some(&c.finance),
        Some(json!({"payee_verified": true, "paid_ref": "PAY-1002"})),
        StatusCode::BAD_REQUEST,
        "ONLY_APPROVED_REIMB_CAN_BE_PAID",
    )
    .await;

    deny(
        &app,
        Method::POST,
        "/api/help",
        Some(&c.member),
        Some(json!({"subject": "  ", "body": ""})),
        StatusCode::BAD_REQUEST,
        "QUESTION_AND_DETAILS_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        "/api/help",
        Some(&c.member),
        Some(json!({"subject": "行程", "body": "不是我的", "ride_id": ride})),
        StatusCode::FORBIDDEN,
        "ONLY_LINK_OWN_RIDES",
    )
    .await;
    let ticket = must(
        &app,
        Method::POST,
        "/api/help",
        Some(&c.member),
        Some(json!({"subject": "如何改接送", "body": "我想改时间"})),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::GET,
        &format!("/api/help/{ticket}"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "FORBIDDEN",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/help/{ticket}/replies"),
        Some(&c.member),
        Some(json!({"body": "  "})),
        StatusCode::BAD_REQUEST,
        "REPLY_CONTENT_REQUIRED",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/help/{ticket}/close"),
        Some(&c.member),
        None,
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/help/{ticket}/replies"),
        Some(&c.member),
        Some(json!({"body": "还有一句"})),
        StatusCode::BAD_REQUEST,
        "REQUEST_ALREADY_CLOSED",
    )
    .await;

    deny(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&c.admin),
        Some(json!({"title": "  ", "body": ""})),
        StatusCode::BAD_REQUEST,
        "TITLE_AND_CONTENT_REQUIRED",
    )
    .await;
    let ann = must(
        &app,
        Method::POST,
        "/api/announcements",
        Some(&c.admin),
        Some(json!({"title": "本周聚会", "body": "周日见", "pinned": true, "notify_all": false})),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    let anns = must(
        &app,
        Method::GET,
        "/api/announcements",
        Some(&c.member),
        None,
    )
    .await;
    assert!(anns.as_array().unwrap().iter().any(|a| a["id"] == ann));
    must(
        &app,
        Method::DELETE,
        &format!("/api/announcements/{ann}"),
        Some(&c.admin),
        None,
    )
    .await;

    deny(
        &app,
        Method::PUT,
        "/api/me",
        Some(&c.member),
        Some(json!({"name": "  "})),
        StatusCode::BAD_REQUEST,
        "NAME_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/me",
        Some(&c.member),
        Some(json!({"lang": "fr"})),
        StatusCode::BAD_REQUEST,
        "LANGUAGE_INVALID",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/me",
        Some(&c.member),
        Some(json!({"auto_lock_mins": 9999})),
        StatusCode::BAD_REQUEST,
        "INVALID_ACTION",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/me",
        Some(&c.member),
        Some(json!({"photo_file_id": other})),
        StatusCode::FORBIDDEN,
        "INVALID_PHOTO",
    )
    .await;
    must(
        &app,
        Method::PUT,
        "/api/me",
        Some(&c.member),
        Some(json!({"lang": "en", "auto_lock_mins": 5, "photo_file_id": receipt})),
    )
    .await;

    must(
        &app,
        Method::GET,
        "/api/notifications",
        Some(&c.member),
        None,
    )
    .await;
    must(
        &app,
        Method::POST,
        "/api/notifications/999999/read",
        Some(&c.member),
        None,
    )
    .await;
    must(
        &app,
        Method::POST,
        "/api/notifications/read-all",
        Some(&c.member),
        None,
    )
    .await;
    let badges = must(&app, Method::GET, "/api/badges", Some(&c.member), None).await;
    assert!(badges.as_array().is_some());
    let report = must(
        &app,
        Method::GET,
        "/api/reports/monthly?month=2026-11&range=monthly&type=all",
        Some(&c.member),
        None,
    )
    .await;
    assert!(report.is_object());
    deny(
        &app,
        Method::GET,
        &format!("/api/reports/monthly?month=2026-11&user_id={}", c.sophie_id),
        Some(&c.member),
        None,
        StatusCode::FORBIDDEN,
        "NO_PERMISSION",
    )
    .await;
}

#[tokio::test]
async fn files_admin_directory_and_plan_branches() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;

    let empty = {
        let boundary = "XgrEMPTY";
        let body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"note\"\r\n\r\nhi\r\n--{boundary}--\r\n");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/files")
            .header(header::COOKIE, &c.member)
            .header(header::ORIGIN, ORIGIN)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null),
        )
    };
    assert_eq!(empty.0, StatusCode::BAD_REQUEST, "{}", empty.1);
    assert_eq!(empty.1["code"], "NO_FILE_RECEIVED");

    let bad = post_bytes(&app, &c.member, "notes.txt", "text/plain", b"hello").await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{}", bad.json);
    assert_eq!(bad.json["code"], "INVALID_FILE_TYPE");

    let mut big = vec![0xFFu8, 0xD8, 0xFF];
    big.resize(8 * 1024 * 1024 + 16, 0);
    let huge = post_bytes(&app, &c.member, "big.jpg", "image/jpeg", &big).await;
    assert_eq!(huge.status, StatusCode::BAD_REQUEST, "{}", huge.json);
    assert_eq!(huge.json["code"], "FILE_TOO_LARGE");

    let mine = upload_png(&app, &c.member).await;
    deny(
        &app,
        Method::GET,
        &format!("/api/files/{mine}"),
        Some(&c.sophie),
        None,
        StatusCode::FORBIDDEN,
        "CANNOT_VIEW_FILE",
    )
    .await;
    let (status, _, body) = call_raw(
        &app,
        Method::GET,
        &format!("/api/files/{mine}"),
        Some(&c.member),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.len() > 8);

    let now = grace_ride::util::now();
    for n in 0..100 {
        sqlx::query(
            "INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at) VALUES ($1,$2,'image/png','r.png',8,$3)",
        )
        .bind(c.sophie_id)
        .bind(format!("quota-{n}.png"))
        .bind(&now)
        .execute(&st.db)
        .await
        .unwrap();
    }
    let capped = post_bytes(&app, &c.sophie, "r.png", "image/png", &{
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0u8; 32]);
        png
    })
    .await;
    assert_eq!(
        capped.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        capped.json
    );
    assert_eq!(capped.json["code"], "UPLOAD_QUOTA_REACHED");

    deny(
        &app,
        Method::POST,
        "/api/admin/groups",
        Some(&c.admin),
        Some(json!({"name": "  "})),
        StatusCode::BAD_REQUEST,
        "GROUP_NAME_REQUIRED",
    )
    .await;
    let gid = must(
        &app,
        Method::POST,
        "/api/admin/groups",
        Some(&c.admin),
        Some(json!({"name": "诗班", "description": "主日"})),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::POST,
        "/api/admin/groups",
        Some(&c.admin),
        Some(json!({"name": "诗班"})),
        StatusCode::BAD_REQUEST,
        "GROUP_NAME_EXISTS",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        &format!("/api/admin/groups/{gid}"),
        Some(&c.admin),
        Some(json!({"name": " "})),
        StatusCode::BAD_REQUEST,
        "GROUP_NAME_REQUIRED",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/admin/groups/999999",
        Some(&c.admin),
        Some(json!({"name": "没有"})),
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;
    deny(
        &app,
        Method::DELETE,
        &format!("/api/admin/groups/{}", c.all_group),
        Some(&c.admin),
        None,
        StatusCode::BAD_REQUEST,
        "CANNOT_DELETE_ALL_MEMBERS_GROUP",
    )
    .await;
    must(
        &app,
        Method::PUT,
        &format!("/api/admin/groups/{gid}/members"),
        Some(&c.admin),
        Some(json!({"user_ids": [c.member_id]})),
    )
    .await;
    let members = must(
        &app,
        Method::GET,
        &format!("/api/admin/groups/{gid}/members"),
        Some(&c.admin),
        None,
    )
    .await;
    assert_eq!(members.as_array().unwrap().len(), 1);
    deny(
        &app,
        Method::PUT,
        "/api/admin/groups/999999/members",
        Some(&c.admin),
        Some(json!({"user_ids": []})),
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;
    must(
        &app,
        Method::DELETE,
        &format!("/api/admin/groups/{gid}"),
        Some(&c.admin),
        None,
    )
    .await;

    deny(
        &app,
        Method::POST,
        "/api/admin/areas",
        Some(&c.admin),
        Some(json!({"name": " "})),
        StatusCode::BAD_REQUEST,
        "AREA_NAME_REQUIRED",
    )
    .await;
    let area_name: String = sqlx::query_scalar("SELECT name FROM areas WHERE id=$1")
        .bind(c.area_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    deny(
        &app,
        Method::POST,
        "/api/admin/areas",
        Some(&c.admin),
        Some(json!({"name": area_name})),
        StatusCode::BAD_REQUEST,
        "AREA_EXISTS",
    )
    .await;
    let area = must(
        &app,
        Method::POST,
        "/api/admin/areas",
        Some(&c.admin),
        Some(json!({"name": "新小区"})),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::PUT,
        &format!("/api/admin/areas/{area}"),
        Some(&c.admin),
        Some(json!({"name": " "})),
        StatusCode::BAD_REQUEST,
        "AREA_NAME_REQUIRED",
    )
    .await;
    must(
        &app,
        Method::PUT,
        &format!("/api/admin/areas/{area}"),
        Some(&c.admin),
        Some(json!({"name": "新小区", "active": false})),
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/admin/areas/999999",
        Some(&c.admin),
        Some(json!({"name": "没有"})),
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
    )
    .await;

    deny(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{}", c.member_id),
        Some(&c.admin),
        Some(json!({"roles": [], "status": "nope", "group_ids": []})),
        StatusCode::BAD_REQUEST,
        "INVALID_STATUS",
    )
    .await;

    deny(
        &app,
        Method::PUT,
        "/api/admin/settings/fares",
        Some(&c.admin),
        Some(json!({"area_a_id": 1, "area_b_id": 1, "amount_cents": -5})),
        StatusCode::BAD_REQUEST,
        "AMOUNT_RANGE_INVALID",
    )
    .await;
    deny(
        &app,
        Method::PUT,
        "/api/admin/settings/system",
        Some(&c.admin),
        Some(json!({
            "audit_retention_days": 1,
            "chat_retention_days": 180,
            "max_live_drivers": 10,
            "session_timeout_mins": 15
        })),
        StatusCode::BAD_REQUEST,
        "INVALID_ACTION",
    )
    .await;
    let settings = must(
        &app,
        Method::GET,
        "/api/admin/settings/system",
        Some(&c.admin),
        None,
    )
    .await;
    assert!(settings["max_live_drivers"].as_i64().unwrap() >= 1);

    let event = make_event(&app, &c.admin, &[c.all_group], 0).await;
    // Event rides need a "going" signup, which records the group each person comes as.
    for who in [&c.driver, &c.member] {
        must(
            &app,
            Method::POST,
            &format!("/api/events/{event}/signup"),
            Some(who),
            Some(json!({"need_ride": true, "as_group_id": c.youth_group})),
        )
        .await;
    }
    let ride = must(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.driver),
        Some(ride_body(c.area_id, 1, Some(event))),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    let plan = format!("/api/admin/events/{event}/plan");
    deny(
        &app,
        Method::POST,
        &plan,
        Some(&c.transport),
        Some(json!({"changes": [{"ride_id": 999999, "from_driver_id": null, "driver_id": c.driver2_id}]})),
        StatusCode::CONFLICT,
        "RIDE_STATUS_CHANGED",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &plan,
        Some(&c.transport),
        Some(json!({"changes": [{"ride_id": ride, "from_driver_id": null, "driver_id": c.member_id}]})),
        StatusCode::BAD_REQUEST,
        "MEMBER_NOT_VOLUNTEER_DRIVER",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &plan,
        Some(&c.transport),
        Some(json!({"changes": [{"ride_id": ride, "from_driver_id": null, "driver_id": c.driver_id}]})),
        StatusCode::BAD_REQUEST,
        "DRIVER_CANNOT_BE_PASSENGER",
    )
    .await;
    let wide = must(
        &app,
        Method::POST,
        "/api/rides",
        Some(&c.member),
        Some(ride_body(c.area_id, 2, Some(event))),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    deny(
        &app,
        Method::POST,
        &plan,
        Some(&c.transport),
        Some(json!({"changes": [{"ride_id": wide, "from_driver_id": null, "driver_id": c.driver2_id}]})),
        StatusCode::BAD_REQUEST,
        "NOT_ENOUGH_DRIVER_SEATS_REASSIGN",
    )
    .await;
    let saved = must(
        &app,
        Method::POST,
        &plan,
        Some(&c.transport),
        Some(json!({"changes": [{"ride_id": wide, "from_driver_id": null, "driver_id": c.driver_id}]})),
    )
    .await;
    assert_eq!(saved["changed"], 1);
    let board = must(&app, Method::GET, &plan, Some(&c.transport), None).await;
    assert!(board["rides"].as_array().unwrap().len() >= 2);
}

#[test]
fn reachable_error_keys_match_handlers() {
    // Keys that exist for old copy or a race, and that no current handler returns.
    // A new `ErrorKey::` in routes/ must be added to flows.rs in the same change.
    let retired = [
        "PICKUP_TIME_REQUIRED",
        "DRIVER_CANNOT_CLAIM_OWN_RIDE",
        "DRIVER_ALREADY_CLAIMED",
        "DRIVER_ALREADY_ASSIGNED",
        "CANNOT_CLAIM_MULTIPLE_DRIVERS",
        "RIDE_NOT_AVAILABLE_FOR_CLAIM",
        "TRIP_ALREADY_APPROVED",
        "TRIP_NOT_APPROVED",
        "TRIP_ALREADY_CANCELLED",
        "DRIVER_CANNOT_DRIVE",
        "DRIVER_SCHEDULE_MISMATCH",
        "DRIVER_CAPACITY_MISMATCH",
        "DRIVER_STATUS_INVALID",
        "DRIVER_ALREADY_ON_TRIP",
        "DRIVER_ASSIGNED_OTHER_RIDE",
        "TRIP_RESPONSE_NOT_ALLOWED",
        "AMOUNT_NEGATIVE",
        "SETTLEMENT_ITEMS_REQUIRED",
        "EMAIL_ALREADY_MEMBER",
        "REIMBURSEMENT_ONLY_PASSENGER",
        "REIMBURSEMENT_ONLY_NON_PENDING",
        "PASSKEY_NOT_FOUND",
        "TOO_MANY_FAILED_LOGINS",
    ];
    let mut src = String::new();
    let mut stack = vec![std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/routes"
    ))];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                src.push_str(&std::fs::read_to_string(&p).unwrap());
                src.push('\n');
            }
        }
    }
    for key in retired {
        let variant: String = key
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        first.to_ascii_uppercase().to_string()
                            + &chars.as_str().to_ascii_lowercase()
                    }
                    None => String::new(),
                }
            })
            .collect();
        let needle = format!("ErrorKey::{variant}");
        assert!(
            !src.contains(&needle),
            "{key} is returned by a handler and needs a test in this file"
        );
    }
}

#[tokio::test]
async fn settlement_revisions_keep_payments_and_settle_zero_shares() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let event = make_event(&app, &c.admin, &[c.all_group], 0).await;
    let both = people(&[c.member_id, c.sophie_id]);
    let sid = must(
        &app,
        Method::POST,
        &format!("/api/events/{event}/settlements"),
        Some(&c.admin),
        Some(bill(
            "午餐",
            json!([{"label": "午餐", "amount_cents": 1000}]),
            0,
            "equal",
            both.clone(),
            "1234567890",
        )),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    for uid in [c.member_id, c.sophie_id] {
        must(
            &app,
            Method::POST,
            &format!("/api/events/{event}/attendance"),
            Some(&c.admin),
            Some(json!({"user_id": uid, "attended": true})),
        )
        .await;
    }
    must(
        &app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&c.admin),
        None,
    )
    .await;
    // The member has paid their RM5.00 and the organiser confirmed it.
    sqlx::query("UPDATE settlement_shares SET status='confirmed', txn_ref='TXN-1' WHERE settlement_id=$1 AND user_id=$2")
        .bind(sid)
        .bind(c.member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let share = |uid: i64| {
        let db = st.db.clone();
        async move {
            sqlx::query_as::<_, (i64, String)>(
                "SELECT amount_cents, status FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2",
            )
            .bind(sid)
            .bind(uid)
            .fetch_optional(&db)
            .await
            .unwrap()
        }
    };
    let revisions = || {
        let db = st.db.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM settlement_revisions WHERE settlement_id=$1",
            )
            .bind(sid)
            .fetch_one(&db)
            .await
            .unwrap()
        }
    };

    // Removing a member who has paid is refused, and nothing is half-written.
    let mut drop_member = bill(
        "午餐",
        json!([{"label": "午餐", "amount_cents": 1000}]),
        0,
        "equal",
        people(&[c.sophie_id]),
        "1234567890",
    );
    drop_member["reason"] = json!("算错人");
    deny(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(drop_member),
        StatusCode::BAD_REQUEST,
        "PAID_MEMBERS_CANNOT_BE_REMOVED",
    )
    .await;
    assert_eq!(share(c.member_id).await, Some((500, "confirmed".into())));
    assert_eq!(revisions().await, 0);

    // Cutting an unpaid share to nothing settles it.
    let custom = |total: i64, sophie: i64| {
        let mut b = bill(
            "午餐",
            json!([{"label": "午餐", "amount_cents": total}]),
            0,
            "custom",
            json!([{"user_id": c.member_id, "amount_cents": 500}, {"user_id": c.sophie_id, "amount_cents": sophie}]),
            "1234567890",
        );
        b["reason"] = json!("调整");
        b
    };
    must(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(custom(500, 0)),
    )
    .await;
    assert_eq!(share(c.sophie_id).await, Some((0, "confirmed".into())));
    assert_eq!(share(c.member_id).await, Some((500, "confirmed".into())));

    // Raising a share of nothing makes it owed, not "needs info".
    must(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(custom(1000, 500)),
    )
    .await;
    assert_eq!(share(c.sophie_id).await, Some((500, "unpaid".into())));

    // Each revision saved its own version.
    let versions: Vec<i64> = sqlx::query_scalar(
        "SELECT version FROM settlement_revisions WHERE settlement_id=$1 ORDER BY version",
    )
    .bind(sid)
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(versions, [1, 2]);
}

#[tokio::test]
async fn reimbursements_pay_out_once_and_need_two_people() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let receipt = upload_png(&app, &c.member).await;
    let body = json!({
        "amount_cents": 1500,
        "description": "Grab",
        "receipt_file_id": receipt,
        "payee_name": "陈法拉",
        "payee_phone": "012-300 0006"
    });
    let first = must(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&c.member),
        Some(body.clone()),
    )
    .await["id"]
        .as_i64()
        .unwrap();

    // The same receipt can't be claimed again while the first request is live.
    deny(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&c.member),
        Some(body.clone()),
        StatusCode::BAD_REQUEST,
        "REIMBURSEMENT_ALREADY_CLAIMED",
    )
    .await;

    // Whoever approves it doesn't also pay it.
    must(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{first}/review"),
        Some(&c.superadmin),
        Some(json!({"action": "approve"})),
    )
    .await;
    deny(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{first}/pay"),
        Some(&c.superadmin),
        Some(json!({"payee_verified": true, "paid_ref": "PAY-2001"})),
        StatusCode::FORBIDDEN,
        "CANNOT_PAY_OWN_APPROVAL",
    )
    .await;
    must(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{first}/pay"),
        Some(&c.finance),
        Some(json!({"payee_verified": true, "paid_ref": "PAY-2001"})),
    )
    .await;

    // A rejected request frees its receipt for a fresh one.
    let receipt2 = upload_png(&app, &c.member).await;
    let mut second = body.clone();
    second["receipt_file_id"] = json!(receipt2);
    let rejected = must(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&c.member),
        Some(second.clone()),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    must(
        &app,
        Method::POST,
        &format!("/api/reimbursements/{rejected}/review"),
        Some(&c.transport),
        Some(json!({"action": "reject", "note": "看不清"})),
    )
    .await;
    must(
        &app,
        Method::POST,
        "/api/reimbursements",
        Some(&c.member),
        Some(second),
    )
    .await;
}

#[tokio::test]
async fn rides_and_visitor_links_follow_a_moved_event() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let event = make_event(&app, &c.admin, &[c.all_group], 0).await;
    let (old_at,): (String,) = sqlx::query_as("SELECT starts_at FROM events WHERE id=$1")
        .bind(event)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let ride = |status: &'static str, destination: &'static str| {
        let db = st.db.clone();
        let (member, driver, area) = (c.member_id, c.driver_id, c.area_id);
        async move {
            sqlx::query_scalar::<_, i64>(
                "INSERT INTO rides (passenger_id, driver_id, event_id, pickup_at, pickup_area_id, pickup_address, destination, seats, status, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,'Lot 21',$6,1,$7,$8,$8) RETURNING id",
            )
            .bind(member)
            .bind(driver)
            .bind(event)
            .bind(day(2, "09:00"))
            .bind(area)
            .bind(destination)
            .bind(status)
            .bind(grace_ride::util::now())
            .fetch_one(&db)
            .await
            .unwrap()
        }
    };
    let waiting = ride("confirmed", "Hall").await;
    let elsewhere = ride("confirmed", "My office").await;
    let done = ride("completed", "Hall").await;
    let pass: i64 = sqlx::query_scalar(
        "INSERT INTO visitor_passes (token_hash, event_id, host_id, host_group_id, ride_id, name, valid_from, valid_until)
         VALUES ('moved-test', $1, $2, $3, $4, 'Guest', $5, $6) RETURNING id",
    )
    .bind(event)
    .bind(c.member_id)
    .bind(c.all_group)
    .bind(waiting)
    .bind(grace_ride::util::now())
    .bind(day(2, "22:00:00"))
    .fetch_one(&st.db)
    .await
    .unwrap();

    let new_at = day(5, "19:30");
    must(
        &app,
        Method::PUT,
        &format!("/api/events/{event}"),
        Some(&c.admin),
        Some(event_body(
            "团契聚会",
            "other",
            &new_at,
            "",
            &[c.all_group],
            0,
        )),
    )
    .await;
    let mut moved = event_body("团契聚会", "other", &new_at, "", &[c.all_group], 0);
    moved["location"] = json!("New Hall");
    must(
        &app,
        Method::PUT,
        &format!("/api/events/{event}"),
        Some(&c.admin),
        Some(moved),
    )
    .await;

    let get = |id: i64| {
        let db = st.db.clone();
        async move {
            sqlx::query_as::<_, (String, i64)>(
                "SELECT destination, needs_attention FROM rides WHERE id=$1",
            )
            .bind(id)
            .fetch_one(&db)
            .await
            .unwrap()
        }
    };
    assert_eq!(get(waiting).await, ("New Hall".into(), 1));
    assert_eq!(get(elsewhere).await, ("My office".into(), 1));
    assert_eq!(get(done).await, ("Hall".into(), 0));
    assert_ne!(old_at, new_at);

    let told: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id=$1 AND link='/drive' AND body LIKE '%New Hall%'",
    )
    .bind(c.driver_id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(told, 1);

    let until: String = sqlx::query_scalar("SELECT valid_until FROM visitor_passes WHERE id=$1")
        .bind(pass)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert!(
        until.as_str() > new_at.as_str(),
        "{until} should be after {new_at}"
    );
}

#[tokio::test]
async fn reissuing_a_visitor_link_leaves_exactly_one_working() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let event = make_event(&app, &c.admin, &[c.all_group], 0).await;
    let ride = book(&app, &c.member, c.area_id, 1).await;
    let pass: i64 = sqlx::query_scalar(
        "INSERT INTO visitor_passes (token_hash, event_id, host_id, host_group_id, ride_id, name, valid_from, valid_until)
         VALUES ('reissue-test', $1, $2, $3, $4, 'Guest', $5, $6) RETURNING id",
    )
    .bind(event)
    .bind(c.member_id)
    .bind(c.all_group)
    .bind(ride)
    .bind(grace_ride::util::now())
    .bind(day(3, "22:00:00"))
    .fetch_one(&st.db)
    .await
    .unwrap();
    let path = format!("/api/visitor-passes/{pass}/reissue");
    let (a, b) = tokio::join!(
        call(&app, Method::POST, &path, Some(&c.member), None),
        call(&app, Method::POST, &path, Some(&c.member), None),
    );
    let ok = [a.status, b.status]
        .iter()
        .filter(|s| **s == StatusCode::OK)
        .count();
    assert_eq!(ok, 1, "{} / {}", a.json, b.json);
    let open: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM visitor_passes WHERE ride_id=$1 AND closed_at IS NULL",
    )
    .bind(ride)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(open, 1);
}

/// Reopening a ride (driver drops out, passenger declines) while another driver claims
/// it must never leave the ride waiting on a passenger with no driver.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reopening_a_ride_never_drops_a_driver_who_just_claimed_it() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    for round in 0..12 {
        let id = book(&app, &c.member, c.area_id, 1).await;
        must(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/claim"),
            Some(&c.driver),
            None,
        )
        .await;
        // Even rounds the driver drops out, odd rounds the passenger declines.
        let (reopen_path, reopen_body, reopener) = if round % 2 == 0 {
            (
                format!("/api/rides/{id}/withdraw"),
                json!({"reason": "车坏了"}),
                &c.driver,
            )
        } else {
            (
                format!("/api/rides/{id}/respond"),
                json!({"accept": false}),
                &c.member,
            )
        };
        let claim_path = format!("/api/rides/{id}/claim");
        let reopen = call(
            &app,
            Method::POST,
            &reopen_path,
            Some(reopener),
            Some(reopen_body),
        );
        let claim = call(&app, Method::POST, &claim_path, Some(&c.driver2), None);
        let (reopened, _) = tokio::join!(reopen, claim);
        assert_eq!(reopened.status, StatusCode::OK, "{}", reopened.json);
        let (status, driver): (String, Option<i64>) =
            sqlx::query_as("SELECT status, driver_id FROM rides WHERE id=$1")
                .bind(id)
                .fetch_one(&st.db)
                .await
                .unwrap();
        match status.as_str() {
            "pending_driver" => assert_eq!(driver, None, "round {round}"),
            "pending_passenger" => assert_eq!(driver, Some(c.driver2_id), "round {round}"),
            other => panic!("round {round}: unexpected status {other}"),
        }
        // Free driver2's seats for the next round.
        must(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/cancel"),
            Some(&c.member),
            None,
        )
        .await;
    }
}

#[tokio::test]
async fn reminders_are_sent_again_after_a_new_time_or_a_new_driver() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;

    // An event whose day-before reminder already went out, then postponed.
    let event = make_event(&app, &c.admin, &[c.all_group], 0).await;
    sqlx::query("UPDATE events SET reminder_sent=1 WHERE id=$1")
        .bind(event)
        .execute(&st.db)
        .await
        .unwrap();
    must(
        &app,
        Method::PUT,
        &format!("/api/events/{event}"),
        Some(&c.admin),
        Some(event_body(
            "团契聚会",
            "other",
            &day(9, "10:00"),
            "",
            &[c.all_group],
            0,
        )),
    )
    .await;
    let event_flag: i64 = sqlx::query_scalar("SELECT reminder_sent FROM events WHERE id=$1")
        .bind(event)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(event_flag, 0, "a postponed event is reminded again");

    // A ride reminded with one driver, who then drops out.
    let ride = book(&app, &c.member, c.area_id, 1).await;
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{ride}/claim"),
        Some(&c.driver),
        None,
    )
    .await;
    let flag = || {
        let db = st.db.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT reminder_sent FROM rides WHERE id=$1")
                .bind(ride)
                .fetch_one(&db)
                .await
                .unwrap()
        }
    };
    sqlx::query("UPDATE rides SET reminder_sent=1 WHERE id=$1")
        .bind(ride)
        .execute(&st.db)
        .await
        .unwrap();
    // Other changes to the ride leave the reminder as sent.
    sqlx::query("UPDATE rides SET notes='带伞' WHERE id=$1")
        .bind(ride)
        .execute(&st.db)
        .await
        .unwrap();
    assert_eq!(flag().await, 1);
    must(
        &app,
        Method::POST,
        &format!("/api/rides/{ride}/withdraw"),
        Some(&c.driver),
        Some(json!({"reason": "车坏了"})),
    )
    .await;
    assert_eq!(flag().await, 0, "the next driver gets the reminder");
}

/// A published bill with `member` and `sophie` on custom shares, both recorded as attended.
async fn published_custom_bill(app: &Router, c: &Cast, member: i64, sophie: i64) -> i64 {
    let event = make_event(app, &c.admin, &[c.all_group], 0).await;
    let sid = must(
        app,
        Method::POST,
        &format!("/api/events/{event}/settlements"),
        Some(&c.admin),
        Some(custom_bill(c, member, sophie)),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    for uid in [c.member_id, c.sophie_id] {
        must(
            app,
            Method::POST,
            &format!("/api/events/{event}/attendance"),
            Some(&c.admin),
            Some(json!({"user_id": uid, "attended": true})),
        )
        .await;
    }
    must(
        app,
        Method::POST,
        &format!("/api/settlements/{sid}/publish"),
        Some(&c.admin),
        None,
    )
    .await;
    sid
}

fn custom_bill(c: &Cast, member: i64, sophie: i64) -> Value {
    let mut b = bill(
        "晚餐",
        json!([{"label": "晚餐", "amount_cents": member + sophie}]),
        0,
        "custom",
        json!([{"user_id": c.member_id, "amount_cents": member}, {"user_id": c.sophie_id, "amount_cents": sophie}]),
        "1234567890",
    );
    b["reason"] = json!("调整金额");
    b
}

#[tokio::test]
async fn repeated_revisions_keep_a_payment_recognised() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    let sid = published_custom_bill(&app, &c, 1000, 1000).await;
    let share_id = |uid: i64| {
        let db = st.db.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT id FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2",
            )
            .bind(sid)
            .bind(uid)
            .fetch_one(&db)
            .await
            .unwrap()
        }
    };
    let share = |uid: i64| {
        let db = st.db.clone();
        async move {
            sqlx::query_as::<_, (i64, String, String)>(
                "SELECT amount_cents, status, note FROM settlement_shares WHERE settlement_id=$1 AND user_id=$2",
            )
            .bind(sid)
            .bind(uid)
            .fetch_one(&db)
            .await
            .unwrap()
        }
    };
    // The member pays RM10 by transfer with proof; sophie pays RM10 in cash, which the
    // payee confirms directly.
    let member_share = share_id(c.member_id).await;
    let proof = upload_png(&app, &c.member).await;
    must(
        &app,
        Method::POST,
        &format!("/api/shares/{member_share}/proof"),
        Some(&c.member),
        Some(json!({"proof_file_id": proof, "txn_ref": "TXN-REV-1"})),
    )
    .await;
    for share in [member_share, share_id(c.sophie_id).await] {
        must(
            &app,
            Method::POST,
            &format!("/api/shares/{share}/verify"),
            Some(&c.admin),
            Some(json!({"action": "confirm"})),
        )
        .await;
    }

    // RM10 → RM20 → RM30: each revision keeps the RM10 already paid on record.
    for (amount, owed) in [(2000, "RM 20.00"), (3000, "RM 30.00")] {
        must(
            &app,
            Method::PUT,
            &format!("/api/settlements/{sid}"),
            Some(&c.admin),
            Some(custom_bill(&c, amount, amount)),
        )
        .await;
        for uid in [c.member_id, c.sophie_id] {
            let (amt, status, note) = share(uid).await;
            assert_eq!((amt, status.as_str()), (amount, "needs_info"), "user {uid}");
            assert!(note.contains("RM 10.00") && note.contains(owed), "{note}");
        }
    }
    // A paid member still can't be dropped while their payment is being reconciled.
    let mut drop_member = custom_bill(&c, 3000, 3000);
    drop_member["participants"] = json!([{"user_id": c.sophie_id, "amount_cents": 3000}]);
    drop_member["items"] = json!([{"label": "晚餐", "amount_cents": 3000}]);
    deny(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(drop_member),
        StatusCode::BAD_REQUEST,
        "PAID_MEMBERS_CANNOT_BE_REMOVED",
    )
    .await;

    // Back to what was paid: nothing to reconcile, the payee checks it again.
    must(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(custom_bill(&c, 1000, 1000)),
    )
    .await;
    assert_eq!(
        share(c.member_id).await,
        (1000, "pending".into(), String::new())
    );

    // A reset says nothing was received, so the next revision makes the share owed.
    let sophie_share = share_id(c.sophie_id).await;
    must(
        &app,
        Method::POST,
        &format!("/api/shares/{sophie_share}/verify"),
        Some(&c.admin),
        Some(json!({"action": "reset"})),
    )
    .await;
    must(
        &app,
        Method::PUT,
        &format!("/api/settlements/{sid}"),
        Some(&c.admin),
        Some(custom_bill(&c, 1500, 1500)),
    )
    .await;
    assert_eq!(
        share(c.sophie_id).await,
        (1500, "unpaid".into(), String::new())
    );
}

#[tokio::test]
async fn closing_races_a_revision_without_trapping_a_share() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;
    for round in 0..8 {
        let sid = published_custom_bill(&app, &c, 1000, 1000).await;
        sqlx::query("UPDATE settlement_shares SET status='confirmed', paid_cents=amount_cents WHERE settlement_id=$1")
            .bind(sid)
            .execute(&st.db)
            .await
            .unwrap();
        let (bill_path, close_path) = (
            format!("/api/settlements/{sid}"),
            format!("/api/settlements/{sid}/close"),
        );
        let revise = call(
            &app,
            Method::PUT,
            &bill_path,
            Some(&c.admin),
            Some(custom_bill(&c, 2000, 2000)),
        );
        let close = call(&app, Method::POST, &close_path, Some(&c.admin), None);
        let (revised, closed) = tokio::join!(revise, close);
        let (status,): (String,) = sqlx::query_as("SELECT status FROM settlements WHERE id=$1")
            .bind(sid)
            .fetch_one(&st.db)
            .await
            .unwrap();
        let (open,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM settlement_shares WHERE settlement_id=$1 AND status != 'confirmed'",
        )
        .bind(sid)
        .fetch_one(&st.db)
        .await
        .unwrap();
        // Exactly one wins: a closed bill has nothing open, and an open bill was revised.
        if status == "closed" {
            assert_eq!(open, 0, "round {round}: closed bill has open shares");
            assert_eq!(closed.status, StatusCode::OK, "round {round}");
            assert_eq!(
                revised.json["code"], "BILL_CLOSED_CANNOT_EDIT",
                "round {round}"
            );
        } else {
            assert_eq!(
                revised.status,
                StatusCode::OK,
                "round {round} {}",
                revised.json
            );
            assert_eq!(closed.json["code"], "OPEN_PAYMENTS_REMAIN", "round {round}");
        }
    }
}

#[tokio::test]
async fn chat_reactions_and_receipts() {
    let (app, st) = setup().await;
    let c = cast(&app, &st).await;

    // An approved trip: member is the passenger, driver drives.
    let id = book(&app, &c.member, c.area_id, 1).await;
    for (path, who, body) in [
        ("claim", &c.driver, None),
        ("respond", &c.member, Some(json!({"accept": true}))),
        ("approve", &c.transport, None),
    ] {
        must(
            &app,
            Method::POST,
            &format!("/api/rides/{id}/{path}"),
            Some(who),
            body,
        )
        .await;
    }
    let msgs = format!("/api/rides/{id}/messages");
    let sent = must(
        &app,
        Method::POST,
        &msgs,
        Some(&c.member),
        Some(json!({"body": "我在门口"})),
    )
    .await;
    let mid = sent["id"].as_i64().unwrap();
    let mine_status = |chat: &Value| -> String {
        chat["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == mid)
            .unwrap()["status"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // Sent: the driver's app has not fetched it yet.
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    assert_eq!(mine_status(&chat), "sent");
    assert!(chat["reactions_allowed"].as_array().unwrap().len() >= 6);
    // A staff viewer fetching the chat does not count as delivery.
    must(&app, Method::GET, &msgs, Some(&c.transport), None).await;
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    assert_eq!(mine_status(&chat), "sent");
    // Delivered once the driver's app fetches it; others' messages carry no status.
    let driver_view = must(&app, Method::GET, &msgs, Some(&c.driver), None).await;
    let theirs = driver_view["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == mid)
        .unwrap()
        .clone();
    assert!(theirs.get("status").is_none());
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    assert_eq!(mine_status(&chat), "delivered");
    // Staff marking read is accepted but not recorded.
    must(
        &app,
        Method::POST,
        &format!("{msgs}/read"),
        Some(&c.transport),
        Some(json!({"up_to": mid})),
    )
    .await;
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    assert_eq!(mine_status(&chat), "delivered");
    // Read once the driver has it on screen; a too-high cursor is clamped.
    must(
        &app,
        Method::POST,
        &format!("{msgs}/read"),
        Some(&c.driver),
        Some(json!({"up_to": mid + 1000})),
    )
    .await;
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    assert_eq!(mine_status(&chat), "read");
    let (cursor,): (i64,) = sqlx::query_as(
        "SELECT last_read_id FROM ride_chat_reads WHERE user_id=$1 AND trip_id=(SELECT trip_id FROM rides WHERE id=$2)",
    )
    .bind(c.driver_id)
    .bind(id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(cursor, mid);
    // An outsider cannot mark read.
    deny(
        &app,
        Method::POST,
        &format!("{msgs}/read"),
        Some(&c.sophie),
        Some(json!({"up_to": mid})),
        StatusCode::FORBIDDEN,
        "ONLY_TRIP_MEMBERS_CAN_VIEW_CHAT",
    )
    .await;

    // Reactions: add, replace, toggle off.
    let react = format!("{msgs}/{mid}/reactions");
    let reactions_on = |chat: &Value| -> Value {
        chat["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == mid)
            .unwrap()["reactions"]
            .clone()
    };
    must(
        &app,
        Method::POST,
        &react,
        Some(&c.driver),
        Some(json!({"emoji": "👍"})),
    )
    .await;
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    let r = reactions_on(&chat);
    assert_eq!(r.as_array().unwrap().len(), 1);
    assert_eq!(r[0]["emoji"], "👍");
    assert_eq!(r[0]["count"], 1);
    assert_eq!(r[0]["mine"], false);
    let chat = must(&app, Method::GET, &msgs, Some(&c.driver), None).await;
    assert_eq!(reactions_on(&chat)[0]["mine"], true);
    must(
        &app,
        Method::POST,
        &react,
        Some(&c.member),
        Some(json!({"emoji": "👍"})),
    )
    .await;
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    assert_eq!(reactions_on(&chat)[0]["count"], 2);
    // A different emoji replaces the driver's reaction.
    must(
        &app,
        Method::POST,
        &react,
        Some(&c.driver),
        Some(json!({"emoji": "❤️"})),
    )
    .await;
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    let r = reactions_on(&chat);
    assert_eq!(r.as_array().unwrap().len(), 2);
    assert_eq!(r[0]["count"], 1);
    // The same emoji again removes it.
    let off = must(
        &app,
        Method::POST,
        &react,
        Some(&c.driver),
        Some(json!({"emoji": "❤️"})),
    )
    .await;
    assert!(off["emoji"].is_null());
    let chat = must(&app, Method::GET, &msgs, Some(&c.member), None).await;
    assert_eq!(reactions_on(&chat).as_array().unwrap().len(), 1);

    deny(
        &app,
        Method::POST,
        &react,
        Some(&c.driver),
        Some(json!({"emoji": "🔥"})),
        StatusCode::BAD_REQUEST,
        "INVALID_REACTION",
    )
    .await;
    deny(
        &app,
        Method::POST,
        &react,
        Some(&c.sophie),
        Some(json!({"emoji": "👍"})),
        StatusCode::FORBIDDEN,
        "ONLY_TRIP_MEMBERS_CAN_VIEW_CHAT",
    )
    .await;
    // System messages take no reactions.
    let system: i64 = sqlx::query_scalar(
        "SELECT MIN(id) FROM ride_messages WHERE user_id IS NULL AND trip_id=(SELECT trip_id FROM rides WHERE id=$1)",
    )
    .bind(id)
    .fetch_one(&st.db)
    .await
    .unwrap();
    deny(
        &app,
        Method::POST,
        &format!("{msgs}/{system}/reactions"),
        Some(&c.driver),
        Some(json!({"emoji": "👍"})),
        StatusCode::BAD_REQUEST,
        "INVALID_REACTION",
    )
    .await;

    // A message from another trip cannot be reached through this ride.
    let other = book(&app, &c.sophie, c.area_id, 1).await;
    for (path, who, body) in [
        ("claim", &c.driver2, None),
        ("respond", &c.sophie, Some(json!({"accept": true}))),
        ("approve", &c.transport, None),
    ] {
        must(
            &app,
            Method::POST,
            &format!("/api/rides/{other}/{path}"),
            Some(who),
            body,
        )
        .await;
    }
    let foreign = must(
        &app,
        Method::POST,
        &format!("/api/rides/{other}/messages"),
        Some(&c.sophie),
        Some(json!({"body": "other car"})),
    )
    .await["id"]
        .as_i64()
        .unwrap();
    let r = call(
        &app,
        Method::POST,
        &format!("{msgs}/{foreign}/reactions"),
        Some(&c.member),
        Some(json!({"emoji": "👍"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}
