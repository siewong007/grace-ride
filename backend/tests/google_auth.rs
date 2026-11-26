//! Integration tests for Google OAuth sign-in, invite/profile verification,
//! unlinking, and Google-verified password reset.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::{header, HeaderMap, Method, Request, StatusCode},
    routing::{get, post},
    Json, Router,
};
use common::*;
use grace_ride::{
    google_oauth::GoogleOAuthConfig,
    init, router,
    state::{AppState, Config},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

#[derive(Clone)]
struct MockGoogleIdentity {
    sub: String,
    email: String,
    email_verified: bool,
    fail_token: bool,
}

async fn start_mock_google(
    initial: MockGoogleIdentity,
) -> (
    String,
    Arc<Mutex<MockGoogleIdentity>>,
    tokio::task::JoinHandle<()>,
) {
    let state = Arc::new(Mutex::new(initial));
    let s1 = state.clone();
    let s2 = state.clone();
    let app = Router::new()
        .route(
            "/token",
            post(move |body: String| {
                let s = s1.clone();
                let params: HashMap<String, String> = url::form_urlencoded::parse(body.as_bytes())
                    .into_owned()
                    .collect();
                async move {
                    let guard = s.lock().unwrap().clone();
                    if guard.fail_token || params.get("code").is_none_or(String::is_empty) {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({ "error": "invalid_grant" })),
                        );
                    }
                    (
                        StatusCode::OK,
                        Json(json!({ "access_token": "mock-google-access-token" })),
                    )
                }
            }),
        )
        .route(
            "/userinfo",
            get(move |headers: HeaderMap| {
                let s = s2.clone();
                async move {
                    let auth = headers
                        .get(header::AUTHORIZATION)
                        .and_then(|v| v.to_str().ok());
                    if auth != Some("Bearer mock-google-access-token") {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(json!({ "error": "invalid_token" })),
                        );
                    }
                    let guard = s.lock().unwrap().clone();
                    (
                        StatusCode::OK,
                        Json(json!({
                            "sub": guard.sub,
                            "email": guard.email,
                            "email_verified": guard.email_verified,
                        })),
                    )
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}"), state, handle)
}

async fn setup_with_google(base_url: &str, dev_mode: bool) -> (Router, AppState) {
    let dir = std::env::temp_dir().join(format!("gr-google-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        dev_mode,
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
        db: grace_ride::state::DbConfig {
            max_connections: 5,
            acquire_timeout_secs: 60,
            ..Default::default()
        },
        housekeeping_interval_mins: 60,
        notification_retention_days: 90,
        turnstile: Default::default(),
        google_oauth: GoogleOAuthConfig {
            client_id: Some("mock-google-client-id".into()),
            client_secret: Some("mock-google-client-secret".into()),
            auth_url: format!("{base_url}/auth"),
            token_url: format!("{base_url}/token"),
            userinfo_url: format!("{base_url}/userinfo"),
        },
        android_cert_sha256_fingerprints: Vec::new(),
        ios_app_ids: Vec::new(),
        require_biometric_attestation: false,
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    register_test_db(&state);
    (router(state.clone()), state)
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|raw| {
            let pair = raw.split(';').next()?.trim();
            if pair.starts_with(&prefix) && !pair.ends_with('=') {
                Some(pair.to_string())
            } else {
                None
            }
        })
}

fn query_param(url_str: &str, key: &str) -> Option<String> {
    let full = if url_str.starts_with("http://") || url_str.starts_with("https://") {
        url_str.to_string()
    } else {
        format!("http://localhost{url_str}")
    };
    let parsed = url::Url::parse(&full).ok()?;
    parsed
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

async fn get_redirect(
    app: &Router,
    path: &str,
    cookie_header: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let mut b = Request::builder()
        .method(Method::GET)
        .uri(path)
        .header(header::ORIGIN, ORIGIN);
    if let Some(c) = cookie_header {
        b = b.header(header::COOKIE, c);
    }
    let res = app
        .clone()
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let _ = res.into_body().collect().await.unwrap();
    let loc = headers
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    (status, headers, loc)
}

#[tokio::test]
async fn google_config_and_disabled_errors() {
    let (app, _) = setup().await;
    let cfg = call(&app, Method::GET, "/api/auth/google/config", None, None).await;
    assert_eq!(cfg.status, StatusCode::OK);
    assert_eq!(cfg.json["enabled"], false);

    let start = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login" })),
    )
    .await;
    assert_eq!(start.status, StatusCode::BAD_REQUEST);
    assert_eq!(start.json["code"], "GOOGLE_NOT_CONFIGURED");
}

#[tokio::test]
async fn google_login_rejects_non_member_and_unverified_member() {
    let (mock_url, mock_id, h) = start_mock_google(MockGoogleIdentity {
        sub: "google-sub-outsider".into(),
        email: "outsider@example.com".into(),
        email_verified: true,
        fail_token: false,
    })
    .await;
    let (app, st) = setup_with_google(&mock_url, true).await;

    let cfg = call(&app, Method::GET, "/api/auth/google/config", None, None).await;
    assert_eq!(cfg.json["enabled"], true);

    // 1. Non-member trying to sign in with Google gets GOOGLE_NOT_MEMBER and creates no user row.
    let start = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login" })),
    )
    .await;
    assert_eq!(start.status, StatusCode::OK, "{}", start.json);
    let nonce = cookie_value(&start.headers, "gr_oauth_nonce").expect("oauth nonce cookie");
    let state = query_param(start.json["url"].as_str().unwrap(), "state").unwrap();

    let (status, headers, loc) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=c1&state={state}"),
        Some(&nonce),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        query_param(&loc, "google_error").as_deref(),
        Some("GOOGLE_NOT_MEMBER")
    );
    assert!(cookie_value(&headers, "gr_session").is_none());
    let created: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email='outsider@example.com'")
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(created, 0);

    // 2. Existing member without verified Google gets GOOGLE_NOT_VERIFIED and no auto-link.
    {
        let mut g = mock_id.lock().unwrap();
        g.sub = "google-sub-member".into();
        g.email = "member@example.org".into();
    }
    let start2 = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login" })),
    )
    .await;
    let nonce2 = cookie_value(&start2.headers, "gr_oauth_nonce").unwrap();
    let state2 = query_param(start2.json["url"].as_str().unwrap(), "state").unwrap();
    let (status2, headers2, loc2) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=c2&state={state2}"),
        Some(&nonce2),
    )
    .await;
    assert_eq!(status2, StatusCode::SEE_OTHER);
    assert_eq!(
        query_param(&loc2, "google_error").as_deref(),
        Some("GOOGLE_NOT_VERIFIED")
    );
    assert!(cookie_value(&headers2, "gr_session").is_none());
    let linked_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM oauth_accounts oa JOIN users u ON u.id = oa.user_id WHERE u.email='member@example.org'",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    assert_eq!(linked_rows, 0);

    h.abort();
}

#[tokio::test]
async fn google_link_requires_matching_email_and_allows_invited_member_signin() {
    let (mock_url, mock_id, h) = start_mock_google(MockGoogleIdentity {
        sub: "google-sub-invited".into(),
        email: "wrong-email@example.com".into(),
        email_verified: true,
        fail_token: false,
    })
    .await;
    let (app, st) = setup_with_google(&mock_url, false).await;

    let admin_cookie = login(&st, "admin@example.org").await;
    let inv = call(
        &app,
        Method::POST,
        "/api/admin/invites",
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(inv.status, StatusCode::OK, "{}", inv.json);
    let token = inv.json["token"].as_str().unwrap();

    // Invitee accepts the invite (pending member).
    let acc = call(
        &app,
        Method::POST,
        &format!("/api/invites/{token}/accept"),
        None,
        Some(json!({
            "name": "Invited Member",
            "email": "invited@example.com",
            "phone": "012-345 6789",
            "terms": true
        })),
    )
    .await;
    assert_eq!(acc.status, StatusCode::OK, "{}", acc.json);
    let invited_cookie = cookie_from(&acc).expect("session after invite accept");

    // 1. Mismatched Google email is refused.
    let lstart1 = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&invited_cookie),
        Some(json!({ "redirect": "/me?section=security" })),
    )
    .await;
    assert_eq!(lstart1.status, StatusCode::OK, "{}", lstart1.json);
    let nonce1 = cookie_value(&lstart1.headers, "gr_oauth_nonce").unwrap();
    let state1 = query_param(lstart1.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, loc1) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=c1&state={state1}"),
        Some(&format!("{invited_cookie}; {nonce1}")),
    )
    .await;
    assert_eq!(
        query_param(&loc1, "google_error").as_deref(),
        Some("GOOGLE_EMAIL_MISMATCH")
    );

    // 2. Unverified Google email is refused.
    {
        let mut g = mock_id.lock().unwrap();
        g.email = "invited@example.com".into();
        g.email_verified = false;
    }
    let lstart2 = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&invited_cookie),
        None,
    )
    .await;
    let nonce2 = cookie_value(&lstart2.headers, "gr_oauth_nonce").unwrap();
    let state2 = query_param(lstart2.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, loc2) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=c2&state={state2}"),
        Some(&format!("{invited_cookie}; {nonce2}")),
    )
    .await;
    assert_eq!(
        query_param(&loc2, "google_error").as_deref(),
        Some("GOOGLE_EMAIL_UNVERIFIED")
    );

    // 3. Missing nonce cookie is rejected with CHALLENGE_MISMATCH.
    mock_id.lock().unwrap().email_verified = true;
    let lstart3 = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&invited_cookie),
        None,
    )
    .await;
    let state3 = query_param(lstart3.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, loc3) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=c3&state={state3}"),
        Some(&invited_cookie),
    )
    .await;
    assert_eq!(
        query_param(&loc3, "google_error").as_deref(),
        Some("CHALLENGE_MISMATCH")
    );

    // 4. Matching verified Google email links cleanly.
    let lstart4 = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&invited_cookie),
        Some(json!({ "redirect": "/me?section=security" })),
    )
    .await;
    let nonce4 = cookie_value(&lstart4.headers, "gr_oauth_nonce").unwrap();
    let state4 = query_param(lstart4.json["url"].as_str().unwrap(), "state").unwrap();
    let (status4, _, loc4) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=c4&state={state4}"),
        Some(&format!("{invited_cookie}; {nonce4}")),
    )
    .await;
    assert_eq!(status4, StatusCode::SEE_OTHER);
    assert_eq!(query_param(&loc4, "google").as_deref(), Some("linked"));

    let me = call(
        &app,
        Method::GET,
        "/api/auth/me",
        Some(&invited_cookie),
        None,
    )
    .await;
    assert_eq!(me.json["google_verified"], true);
    assert_eq!(me.json["google_email"], "invited@example.com");

    // 5. Same Google `sub` cannot be linked to another member even if email matches.
    {
        let mut g = mock_id.lock().unwrap();
        g.email = "admin@example.org".into();
    }
    let lstart_dup = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&admin_cookie),
        None,
    )
    .await;
    let nonce_dup = cookie_value(&lstart_dup.headers, "gr_oauth_nonce").unwrap();
    let state_dup = query_param(lstart_dup.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, loc_dup) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=c_dup&state={state_dup}"),
        Some(&format!("{admin_cookie}; {nonce_dup}")),
    )
    .await;
    assert_eq!(
        query_param(&loc_dup, "google_error").as_deref(),
        Some("GOOGLE_ALREADY_LINKED")
    );

    // 6. Now the invited member can sign in with Google cleanly.
    {
        let mut g = mock_id.lock().unwrap();
        g.email = "invited@example.com".into();
    }
    let gstart = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login", "redirect": "/events" })),
    )
    .await;
    let gnonce = cookie_value(&gstart.headers, "gr_oauth_nonce").unwrap();
    let gstate = query_param(gstart.json["url"].as_str().unwrap(), "state").unwrap();
    let (gstatus, gheaders, gloc) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=cg&state={gstate}"),
        Some(&gnonce),
    )
    .await;
    assert_eq!(gstatus, StatusCode::SEE_OTHER);
    assert_eq!(gloc, "/events");
    let gcookie = cookie_value(&gheaders, "gr_session").expect("session from Google login");

    // 7. Unlinking disconnects Google and blocks subsequent Google login.
    let unlink = call(
        &app,
        Method::DELETE,
        "/api/auth/google",
        Some(&gcookie),
        None,
    )
    .await;
    assert_eq!(unlink.status, StatusCode::OK, "{}", unlink.json);
    let me_after = call(&app, Method::GET, "/api/auth/me", Some(&gcookie), None).await;
    assert_eq!(me_after.json["google_verified"], false);
    assert!(me_after.json["google_email"].is_null());

    let gstart2 = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login" })),
    )
    .await;
    let gnonce2 = cookie_value(&gstart2.headers, "gr_oauth_nonce").unwrap();
    let gstate2 = query_param(gstart2.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, gloc2) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=cg2&state={gstate2}"),
        Some(&gnonce2),
    )
    .await;
    assert_eq!(
        query_param(&gloc2, "google_error").as_deref(),
        Some("GOOGLE_NOT_VERIFIED")
    );

    h.abort();
}

#[tokio::test]
async fn forgot_password_requires_verified_google_account() {
    let (mock_url, mock_id, h) = start_mock_google(MockGoogleIdentity {
        sub: "google-sub-outsider-reset".into(),
        email: "stranger@example.com".into(),
        email_verified: true,
        fail_token: false,
    })
    .await;
    let (app, st) = setup_with_google(&mock_url, true).await;

    // 1. Non-member -> GOOGLE_NOT_MEMBER.
    let s1 = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "password_reset" })),
    )
    .await;
    let n1 = cookie_value(&s1.headers, "gr_oauth_nonce").unwrap();
    let st1 = query_param(s1.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, h1, l1) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=r1&state={st1}"),
        Some(&n1),
    )
    .await;
    assert_eq!(
        query_param(&l1, "google_error").as_deref(),
        Some("GOOGLE_NOT_MEMBER")
    );
    assert!(cookie_value(&h1, "gr_pw_reset").is_none());

    // 2. Existing member without verified Google -> GOOGLE_NOT_VERIFIED.
    {
        let mut g = mock_id.lock().unwrap();
        g.sub = "google-sub-member-unlinked".into();
        g.email = "member@example.org".into();
    }
    let s2 = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "password_reset", "email": "member@example.org" })),
    )
    .await;
    let n2 = cookie_value(&s2.headers, "gr_oauth_nonce").unwrap();
    let st2 = query_param(s2.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, h2, l2) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=r2&state={st2}"),
        Some(&n2),
    )
    .await;
    assert_eq!(
        query_param(&l2, "google_error").as_deref(),
        Some("GOOGLE_NOT_VERIFIED")
    );
    assert!(cookie_value(&h2, "gr_pw_reset").is_none());

    // 3. Link Google to member@example.org via current session.
    let member_cookie = login(&st, "member@example.org").await;
    let ls = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&member_cookie),
        None,
    )
    .await;
    let ln = cookie_value(&ls.headers, "gr_oauth_nonce").unwrap();
    let lstate = query_param(ls.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, lloc) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=link1&state={lstate}"),
        Some(&format!("{member_cookie}; {ln}")),
    )
    .await;
    assert_eq!(query_param(&lloc, "google").as_deref(), Some("linked"));

    // 4. Disabled verified member -> AUTH_DISABLED.
    sqlx::query("UPDATE users SET status='disabled' WHERE email='member@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let s_dis = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "password_reset" })),
    )
    .await;
    let n_dis = cookie_value(&s_dis.headers, "gr_oauth_nonce").unwrap();
    let st_dis = query_param(s_dis.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, h_dis, l_dis) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=r_dis&state={st_dis}"),
        Some(&n_dis),
    )
    .await;
    assert_eq!(
        query_param(&l_dis, "google_error").as_deref(),
        Some("AUTH_DISABLED")
    );
    assert!(cookie_value(&h_dis, "gr_pw_reset").is_none());

    // Re-enable member and verify wrong email_hint gives GOOGLE_EMAIL_MISMATCH.
    sqlx::query("UPDATE users SET status='active' WHERE email='member@example.org'")
        .execute(&st.db)
        .await
        .unwrap();
    let s_hint = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "password_reset", "email": "other@example.org" })),
    )
    .await;
    let n_hint = cookie_value(&s_hint.headers, "gr_oauth_nonce").unwrap();
    let st_hint = query_param(s_hint.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, h_hint, l_hint) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=r_hint&state={st_hint}"),
        Some(&n_hint),
    )
    .await;
    assert_eq!(
        query_param(&l_hint, "google_error").as_deref(),
        Some("GOOGLE_EMAIL_MISMATCH")
    );
    assert!(cookie_value(&h_hint, "gr_pw_reset").is_none());

    // 5. Verified member completes Google verification for password reset.
    let prior_session = login(&st, "member@example.org").await;
    let s3 = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "password_reset", "email": "member@example.org" })),
    )
    .await;
    let n3 = cookie_value(&s3.headers, "gr_oauth_nonce").unwrap();
    let st3 = query_param(s3.json["url"].as_str().unwrap(), "state").unwrap();
    let (status3, h3, l3) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=r3&state={st3}"),
        Some(&n3),
    )
    .await;
    assert_eq!(status3, StatusCode::SEE_OTHER);
    assert_eq!(l3, "/login?google_reset=1");
    let reset_cookie = cookie_value(&h3, "gr_pw_reset").expect("pw reset cookie");

    let status_no_cookie = call(
        &app,
        Method::GET,
        "/api/auth/password-reset/status",
        None,
        None,
    )
    .await;
    assert_eq!(status_no_cookie.json["valid"], false);

    let status_res = call(
        &app,
        Method::GET,
        "/api/auth/password-reset/status",
        Some(&reset_cookie),
        None,
    )
    .await;
    assert_eq!(status_res.json["valid"], true);
    assert_eq!(status_res.json["email"], "member@example.org");

    // Too-short password fails without consuming the reset cookie.
    let short_res = call(
        &app,
        Method::POST,
        "/api/auth/password-reset/complete",
        Some(&reset_cookie),
        Some(json!({ "new_password": "short" })),
    )
    .await;
    assert_eq!(short_res.status, StatusCode::BAD_REQUEST);
    assert_eq!(short_res.json["code"], "PASSWORD_TOO_SHORT");

    // Valid new password succeeds, revokes prior sessions, and cannot be replayed.
    let new_pw = "NewGraceRide2026!";
    let done_res = call(
        &app,
        Method::POST,
        "/api/auth/password-reset/complete",
        Some(&reset_cookie),
        Some(json!({ "new_password": new_pw })),
    )
    .await;
    assert_eq!(done_res.status, StatusCode::OK, "{}", done_res.json);

    let replay_res = call(
        &app,
        Method::POST,
        "/api/auth/password-reset/complete",
        Some(&reset_cookie),
        Some(json!({ "new_password": "AnotherPassword2026!" })),
    )
    .await;
    assert_eq!(replay_res.status, StatusCode::BAD_REQUEST);
    assert_eq!(replay_res.json["code"], "PASSWORD_RESET_INVALID");

    // Prior session was revoked.
    let old_me = call(
        &app,
        Method::GET,
        "/api/auth/me",
        Some(&prior_session),
        None,
    )
    .await;
    assert_eq!(old_me.status, StatusCode::UNAUTHORIZED);

    // New password signs in cleanly.
    let login_new = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({ "email": "member@example.org", "password": new_pw })),
    )
    .await;
    assert_eq!(login_new.status, StatusCode::OK, "{}", login_new.json);

    h.abort();
}

#[tokio::test]
async fn google_login_enforces_totp_when_enabled() {
    let (mock_url, _mock_id, h) = start_mock_google(MockGoogleIdentity {
        sub: "google-sub-totp-member".into(),
        email: "member@example.org".into(),
        email_verified: true,
        fail_token: false,
    })
    .await;
    let (app, st) = setup_with_google(&mock_url, false).await;

    sqlx::query(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid, terms_accepted_at, terms_version)
         VALUES ('Member TOTP', 'member@example.org', '0123450002', '', 'active', $1, $2, app_now(), 3)",
    )
    .bind(grace_ride::routes::auth::hash_password(PW).unwrap())
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&st.db)
    .await
    .unwrap();

    // Link Google and enable TOTP on member@example.org.
    let member_cookie = login(&st, "member@example.org").await;
    let ls = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&member_cookie),
        None,
    )
    .await;
    assert_eq!(ls.status, StatusCode::OK, "{}", ls.json);
    let ln = cookie_value(&ls.headers, "gr_oauth_nonce").unwrap();
    let lstate = query_param(ls.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, lloc) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=link_totp&state={lstate}"),
        Some(&format!("{member_cookie}; {ln}")),
    )
    .await;
    assert_eq!(query_param(&lloc, "google").as_deref(), Some("linked"));

    let setup_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/setup",
        Some(&member_cookie),
        None,
    )
    .await;
    assert_eq!(setup_res.status, StatusCode::OK, "{}", setup_res.json);
    let secret = setup_res.json["secret"].as_str().unwrap().to_string();
    let enable_code = totp_code(&secret, "member@example.org");
    let enable_res = call(
        &app,
        Method::POST,
        "/api/auth/totp/enable",
        Some(&member_cookie),
        Some(json!({ "code": enable_code })),
    )
    .await;
    assert_eq!(enable_res.status, StatusCode::OK, "{}", enable_res.json);
    let backup_code = enable_res.json["backup_codes"][0]
        .as_str()
        .unwrap()
        .to_string();

    // Google login redirects to 2FA prompt without granting a session yet.
    let gstart = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login", "redirect": "/rides" })),
    )
    .await;
    assert_eq!(gstart.status, StatusCode::OK, "{}", gstart.json);
    let gnonce = cookie_value(&gstart.headers, "gr_oauth_nonce").unwrap();
    let gstate = query_param(gstart.json["url"].as_str().unwrap(), "state").unwrap();
    let (status, headers, loc) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=gtotp1&state={gstate}"),
        Some(&gnonce),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(loc, "/login?google_totp=1");
    assert!(cookie_value(&headers, "gr_session").is_none());
    let totp_cookie = cookie_value(&headers, "gr_google_totp").expect("gr_google_totp cookie");

    let st_missing = call(
        &app,
        Method::GET,
        "/api/auth/google/totp/status",
        None,
        None,
    )
    .await;
    assert_eq!(st_missing.json["valid"], false);
    let st_ok = call(
        &app,
        Method::GET,
        "/api/auth/google/totp/status",
        Some(&totp_cookie),
        None,
    )
    .await;
    assert_eq!(st_ok.json["valid"], true);
    assert_eq!(st_ok.json["email"], "member@example.org");

    // Wrong code is rejected cleanly without issuing a session.
    let wrong = call(
        &app,
        Method::POST,
        "/api/auth/google/totp/complete",
        Some(&totp_cookie),
        Some(json!({ "code": "000000" })),
    )
    .await;
    assert_eq!(wrong.status, StatusCode::BAD_REQUEST, "{}", wrong.json);
    assert_eq!(wrong.json["code"], "INVALID_CODE");
    assert!(cookie_value(&wrong.headers, "gr_session").is_none());

    // Valid backup code completes login with mfa_ok=true and consumes the challenge.
    let ok = call(
        &app,
        Method::POST,
        "/api/auth/google/totp/complete",
        Some(&totp_cookie),
        Some(json!({ "code": backup_code })),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.json);
    assert_eq!(ok.json["redirect"], "/rides");
    let sess_cookie = cookie_value(&ok.headers, "gr_session").expect("session after google 2fa");

    let me = call(&app, Method::GET, "/api/auth/me", Some(&sess_cookie), None).await;
    assert_eq!(me.status, StatusCode::OK, "{}", me.json);
    assert_eq!(me.json["mfa_ok"], true);

    let replay = call(
        &app,
        Method::POST,
        "/api/auth/google/totp/complete",
        Some(&totp_cookie),
        Some(json!({ "code": "000000" })),
    )
    .await;
    assert_eq!(replay.status, StatusCode::BAD_REQUEST, "{}", replay.json);
    assert_eq!(replay.json["code"], "VERIFICATION_EXPIRED");

    // 5 failed attempts on a challenge invalidate that challenge.
    let gstart2 = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login" })),
    )
    .await;
    let gnonce2 = cookie_value(&gstart2.headers, "gr_oauth_nonce").unwrap();
    let gstate2 = query_param(gstart2.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, headers2, _) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=gtotp2&state={gstate2}"),
        Some(&gnonce2),
    )
    .await;
    let totp_cookie2 = cookie_value(&headers2, "gr_google_totp").unwrap();
    for i in 0..5 {
        let bad_try = call(
            &app,
            Method::POST,
            "/api/auth/google/totp/complete",
            Some(&totp_cookie2),
            Some(json!({ "code": "000000" })),
        )
        .await;
        assert_eq!(
            bad_try.status,
            StatusCode::BAD_REQUEST,
            "attempt {i}: {}",
            bad_try.json
        );
        assert_eq!(bad_try.json["code"], "INVALID_CODE");
    }
    let burnt = call(
        &app,
        Method::POST,
        "/api/auth/google/totp/complete",
        Some(&totp_cookie2),
        Some(json!({ "code": "000000" })),
    )
    .await;
    assert_eq!(burnt.status, StatusCode::BAD_REQUEST, "{}", burnt.json);
    assert_eq!(burnt.json["code"], "VERIFICATION_EXPIRED");

    h.abort();
}

#[tokio::test]
async fn security_audit_regressions_recent_signin_locked_link_and_recovery() {
    let (mock_url, mock_id, h) = start_mock_google(MockGoogleIdentity {
        sub: "google-sub-audit".into(),
        email: "pwonly@example.org".into(),
        email_verified: true,
        fail_token: false,
    })
    .await;
    let (app, st) = setup_with_google(&mock_url, false).await;

    let pw_uid: i64 = sqlx::query_scalar(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, webauthn_uuid, terms_accepted_at, terms_version)
         VALUES ('PW Only', 'pwonly@example.org', '0123450003', '', 'active', $1, $2, app_now(), 3)
         RETURNING id",
    )
    .bind(grace_ride::routes::auth::hash_password(PW).unwrap())
    .bind(uuid::Uuid::new_v4().to_string())
    .fetch_one(&st.db)
    .await
    .unwrap();

    // 1. Stale password-only session (>15m) is rejected by require_recent_sign_in.
    let pw_cookie = login(&st, "pwonly@example.org").await;
    sqlx::query("UPDATE sessions SET created_at = $1 WHERE user_id = $2")
        .bind(grace_ride::util::minutes_ago(30))
        .bind(pw_uid)
        .execute(&st.db)
        .await
        .unwrap();
    for path in ["/api/auth/google/link/start", "/api/auth/totp/setup"] {
        let r = call(&app, Method::POST, path, Some(&pw_cookie), None).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{path}: {}", r.json);
        assert_eq!(r.json["code"], "RECENT_SIGN_IN_REQUIRED", "{path}");
    }
    let stale_pw = call(
        &app,
        Method::POST,
        "/api/auth/password",
        Some(&pw_cookie),
        Some(json!({ "current": PW, "new_password": "AnotherSecret2026!" })),
    )
    .await;
    assert_eq!(stale_pw.status, StatusCode::FORBIDDEN, "{}", stale_pw.json);
    assert_eq!(stale_pw.json["code"], "RECENT_SIGN_IN_REQUIRED");

    // 2. Locked session cannot complete an in-flight Google link callback.
    sqlx::query("UPDATE sessions SET created_at = app_now() WHERE user_id = $1")
        .bind(pw_uid)
        .execute(&st.db)
        .await
        .unwrap();
    let ls_lock = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&pw_cookie),
        None,
    )
    .await;
    assert_eq!(ls_lock.status, StatusCode::OK, "{}", ls_lock.json);
    let ln_lock = cookie_value(&ls_lock.headers, "gr_oauth_nonce").unwrap();
    let lstate_lock = query_param(ls_lock.json["url"].as_str().unwrap(), "state").unwrap();
    sqlx::query("UPDATE sessions SET locked_at = app_now() WHERE user_id = $1")
        .bind(pw_uid)
        .execute(&st.db)
        .await
        .unwrap();
    let (_, _, locked_loc) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=locked1&state={lstate_lock}"),
        Some(&format!("{pw_cookie}; {ln_lock}")),
    )
    .await;
    assert_eq!(
        query_param(&locked_loc, "google_error").as_deref(),
        Some("CHALLENGE_MISMATCH")
    );
    let links_after_lock: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1")
            .bind(pw_uid)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(links_after_lock, 0);

    // 3. Fresh session with strong factor (TOTP) and mfa_ok=false gets MFA_REQUIRED on require_recent_sign_in.
    let mfa_uid: i64 = sqlx::query_scalar(
        "INSERT INTO users (name, email, phone, roles, status, password_hash, totp_enabled, webauthn_uuid, terms_accepted_at, terms_version)
         VALUES ('MFA User', 'mfauser@example.org', '0123450004', '', 'active', $1, 1, $2, app_now(), 3)
         RETURNING id",
    )
    .bind(grace_ride::routes::auth::hash_password(PW).unwrap())
    .bind(uuid::Uuid::new_v4().to_string())
    .fetch_one(&st.db)
    .await
    .unwrap();
    let mfa_false_cookie = login_mfa(&st, "mfauser@example.org", false).await;
    let mfa_req = call(
        &app,
        Method::POST,
        "/api/auth/password",
        Some(&mfa_false_cookie),
        Some(json!({ "current": PW, "new_password": "AnotherSecret2026!" })),
    )
    .await;
    assert_eq!(mfa_req.status, StatusCode::FORBIDDEN, "{}", mfa_req.json);
    assert_eq!(mfa_req.json["code"], "MFA_REQUIRED");
    let _ = mfa_uid;

    // 4. Account recovery clears password_hash and oauth_accounts, preventing old login bypass
    //    and allowing password setup without `current` on Recovery Step 3.
    sqlx::query("UPDATE sessions SET locked_at = NULL, created_at = app_now() WHERE user_id = $1")
        .bind(pw_uid)
        .execute(&st.db)
        .await
        .unwrap();
    let ls_ok = call(
        &app,
        Method::POST,
        "/api/auth/google/link/start",
        Some(&pw_cookie),
        None,
    )
    .await;
    let ln_ok = cookie_value(&ls_ok.headers, "gr_oauth_nonce").unwrap();
    let lstate_ok = query_param(ls_ok.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, loc_ok) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=link_pw&state={lstate_ok}"),
        Some(&format!("{pw_cookie}; {ln_ok}")),
    )
    .await;
    assert_eq!(query_param(&loc_ok, "google").as_deref(), Some("linked"));

    let rec_req = call(
        &app,
        Method::POST,
        "/api/auth/recovery-request",
        None,
        Some(json!({ "email": "pwonly@example.org", "reason": "Lost device" })),
    )
    .await;
    assert_eq!(rec_req.status, StatusCode::OK, "{}", rec_req.json);
    let rid: i64 = sqlx::query_scalar(
        "SELECT id FROM recovery_requests WHERE email = 'pwonly@example.org' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&st.db)
    .await
    .unwrap();
    let admin_cookie = login(&st, "admin@example.org").await;
    let approve = call(
        &app,
        Method::POST,
        &format!("/api/admin/recovery/{rid}/approve"),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(approve.status, StatusCode::OK, "{}", approve.json);
    let rec_tok = approve.json["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string();

    // Old password no longer works after recovery approval.
    let old_pw_try = call(
        &app,
        Method::POST,
        "/api/auth/login",
        None,
        Some(json!({ "email": "pwonly@example.org", "password": PW })),
    )
    .await;
    assert_eq!(old_pw_try.status, StatusCode::BAD_REQUEST);
    assert_eq!(old_pw_try.json["code"], "INVALID_CREDENTIALS");

    // Old Google link was removed by recovery approval.
    {
        let mut g = mock_id.lock().unwrap();
        g.sub = "google-sub-audit".into();
        g.email = "pwonly@example.org".into();
    }
    let gs_old = call(
        &app,
        Method::POST,
        "/api/auth/google/start",
        None,
        Some(json!({ "purpose": "login" })),
    )
    .await;
    let gn_old = cookie_value(&gs_old.headers, "gr_oauth_nonce").unwrap();
    let gst_old = query_param(gs_old.json["url"].as_str().unwrap(), "state").unwrap();
    let (_, _, gloc_old) = get_redirect(
        &app,
        &format!("/api/auth/google/callback?code=g_rec&state={gst_old}"),
        Some(&gn_old),
    )
    .await;
    assert_eq!(
        query_param(&gloc_old, "google_error").as_deref(),
        Some("GOOGLE_NOT_VERIFIED")
    );

    // Accepting the recovery link lets the member set a new password without `current`.
    let rec_accept = call(
        &app,
        Method::POST,
        &format!("/api/invites/{rec_tok}/accept"),
        None,
        Some(json!({ "terms": true })),
    )
    .await;
    assert_eq!(rec_accept.status, StatusCode::OK, "{}", rec_accept.json);
    let rec_cookie = cookie_from(&rec_accept).expect("recovery session");
    let rec_set_pw = call(
        &app,
        Method::POST,
        "/api/auth/password",
        Some(&rec_cookie),
        Some(json!({ "new_password": "RecoveredPass2026!" })),
    )
    .await;
    assert_eq!(rec_set_pw.status, StatusCode::OK, "{}", rec_set_pw.json);

    h.abort();
}
