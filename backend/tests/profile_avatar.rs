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
}
