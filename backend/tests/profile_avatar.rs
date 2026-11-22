//! Custom avatars: a member picks a colour and icon from fixed lists, can remove their
//! photo, and anything outside the lists is refused.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;

#[tokio::test]
async fn avatar_style_and_photo_removal() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let me = || call(&app, Method::GET, "/api/auth/me", Some(&member), None);
    let put =
        |body: serde_json::Value| call(&app, Method::PUT, "/api/me", Some(&member), Some(body));

    // Colour and icon are saved and come back on /me.
    let r = put(json!({"avatar_color": "teal", "avatar_icon": "car"})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let m = me().await;
    assert_eq!(m.json["avatar_color"], "teal", "{}", m.json);
    assert_eq!(m.json["avatar_icon"], "car");

    // Fields not sent are left alone.
    let r = put(json!({"name": "Grace Member"})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(me().await.json["avatar_icon"], "car");

    // Keys outside the fixed lists, including raw colours, are refused.
    for body in [
        json!({"avatar_color": "#ff0000"}),
        json!({"avatar_color": "red; background:url(x)"}),
        json!({"avatar_icon": "skull"}),
    ] {
        let r = put(body.clone()).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body} {}", r.json);
    }
    assert_eq!(me().await.json["avatar_color"], "teal");

    // An empty string resets the icon to the initial.
    let r = put(json!({"avatar_icon": ""})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let m = me().await;
    assert!(m.json["avatar_icon"].is_null(), "{}", m.json);
    assert_eq!(m.json["avatar_color"], "teal");

    // A photo can be set and then removed.
    let photo = upload_png(&app, &member).await;
    let r = put(json!({"photo_file_id": photo})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(me().await.json["photo_file_id"], photo);
    let r = put(json!({"clear_photo": true})).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert!(me().await.json["photo_file_id"].is_null());

    // Animated avatars grok and muse are supported.
    for anim in ["grok", "muse"] {
        let r = put(json!({"avatar_icon": anim, "avatar_color": "purple"})).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        assert_eq!(me().await.json["avatar_icon"], anim);
    }
}

#[tokio::test]
async fn admin_can_change_user_achievement_icon() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;

    // Member gets their badges. Initially defaults are used (e.g. night => moon).
    let r = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let badges = r.json.as_array().expect("array of badges");
    let night = badges
        .iter()
        .find(|b| b["key"] == "night")
        .expect("night badge");
    assert_eq!(night["icon"], "moon");

    // Regular member cannot change achievement icons.
    let r = call(
        &app,
        Method::PUT,
        "/api/admin/users/1/badges/night/icon",
        Some(&member),
        Some(json!({ "icon": "owl" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Admin updates the achievement icon for the member (user id 2 in seed is member).
    let member_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}/badges/night/icon"),
        Some(&admin),
        Some(json!({ "icon": "trophy" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    assert_eq!(r.json["icon"], "trophy");

    // Member checks their badges again; night badge now has custom icon 'trophy'.
    let r = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let badges = r.json.as_array().expect("array of badges");
    let night = badges
        .iter()
        .find(|b| b["key"] == "night")
        .expect("night badge");
    assert_eq!(night["icon"], "trophy");

    // Resetting to empty restores default icon.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/admin/users/{member_id}/badges/night/icon"),
        Some(&admin),
        Some(json!({ "icon": "" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    let r = call(&app, Method::GET, "/api/badges", Some(&member), None).await;
    let badges = r.json.as_array().expect("array of badges");
    let night = badges
        .iter()
        .find(|b| b["key"] == "night")
        .expect("night badge");
    assert_eq!(night["icon"], "moon");
}
