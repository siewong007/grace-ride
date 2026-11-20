//! Pickup-area photos: the traffic lead, area managers and traffic team members add
//! them; team members change or remove only their own; every active member sees them.

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use common::*;
use grace_ride::state::AppState;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn area_id(st: &AppState, name: &str) -> i64 {
    sqlx::query_scalar("SELECT id FROM areas WHERE name=$1")
        .bind(name)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|_| panic!("area {name} missing"))
}

async fn upload(app: &Router, cookie: &str, name: &str, bytes: &[u8]) -> i64 {
    let boundary = "XgrBOUNDARY";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
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
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "{v}");
    v["id"].as_i64().unwrap()
}

/// A JPEG carrying an Exif segment (where a phone keeps the GPS location).
fn jpeg_with_exif() -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x4A, 0x46];
    b.extend_from_slice(&[
        0xFF, 0xE1, 0x00, 0x0A, b'E', b'x', b'i', b'f', 0, 0, b'G', b'P',
    ]);
    b.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02, 0x12, 0x34, 0xFF, 0xD9]);
    b
}

async fn add(app: &Router, who: &str, area: i64, file: i64, caption: &str) -> Res {
    call(
        app,
        Method::POST,
        &format!("/api/areas/{area}/photos"),
        Some(who),
        Some(json!({"file_id": file, "caption": caption})),
    )
    .await
}

async fn photos(app: &Router, who: &str, area: i64) -> Res {
    call(
        app,
        Method::GET,
        &format!("/api/areas/{area}/photos"),
        Some(who),
        None,
    )
    .await
}

#[tokio::test]
async fn who_may_add_change_and_remove_area_photos() {
    let (app, st) = setup().await;
    let ut = area_id(&st, "UT").await;
    sqlx::query("UPDATE users SET roles = 'traffic_member' WHERE email = 'amina@example.org'")
        .execute(&st.db)
        .await
        .unwrap();

    // The Head of Traffic, area managers and traffic team members may add photos.
    let mut ids = vec![];
    for email in [
        "traffichead@example.org",
        "admin@example.org",
        "president@example.org",
        "vicepresident@example.org",
        "amina@example.org",
    ] {
        let who = login(&st, email).await;
        let file = upload_png(&app, &who).await;
        let r = add(&app, &who, ut, file, &format!("by {email}")).await;
        assert_eq!(r.status, StatusCode::OK, "{email}: {}", r.json);
        ids.push(r.json["id"].as_i64().unwrap());
    }

    // Riders, drivers, finance and cell leaders may not.
    for email in [
        "member@example.org",
        "driver@example.org",
        "finance@example.org",
        "cellleader@example.org",
    ] {
        let who = login(&st, email).await;
        let file = upload_png(&app, &who).await;
        let r = add(&app, &who, ut, file, "").await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{email} must not add photos"
        );
    }

    // Staff without a passkey / 2FA session are asked for one.
    let no_mfa = login_mfa(&st, "traffichead@example.org", false).await;
    let file = upload_png(&app, &no_mfa).await;
    let r = add(&app, &no_mfa, ut, file, "").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["code"], "MFA_REQUIRED");

    let member_ids = (ids[0], ids[4]); // head's photo, team member's photo
    let amina = login(&st, "amina@example.org").await;
    let head = login(&st, "traffichead@example.org").await;

    // A team member changes and removes only their own photos, and cannot reorder.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/areas/{ut}/photos/{}", member_ids.1),
        Some(&amina),
        Some(json!({"caption": "Wait at the guardhouse"})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    for method in [Method::PUT, Method::DELETE] {
        let r = call(
            &app,
            method.clone(),
            &format!("/api/areas/{ut}/photos/{}", member_ids.0),
            Some(&amina),
            Some(json!({"caption": "nope"})),
        )
        .await;
        assert_eq!(
            r.status,
            StatusCode::FORBIDDEN,
            "{method} someone else's photo"
        );
    }
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/areas/{ut}/photos/order"),
        Some(&amina),
        Some(json!({"photo_ids": ids.iter().rev().collect::<Vec<_>>()})),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    let list = photos(&app, &amina, ut).await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(list.json["can_add"], true);
    assert_eq!(list.json["can_reorder"], false);
    let mine: Vec<&Value> = list.json["photos"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["can_delete"] == true)
        .collect();
    assert_eq!(mine.len(), 1, "a team member may remove only their own");
    assert_eq!(mine[0]["caption"], "Wait at the guardhouse");

    // The Head of Traffic reorders and removes any photo.
    let reversed: Vec<i64> = ids.iter().rev().copied().collect();
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/areas/{ut}/photos/order"),
        Some(&head),
        Some(json!({"photo_ids": reversed})),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    // An order that leaves a photo out is refused.
    let r = call(
        &app,
        Method::PUT,
        &format!("/api/areas/{ut}/photos/order"),
        Some(&head),
        Some(json!({"photo_ids": [ids[0]]})),
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let list = photos(&app, &head, ut).await;
    let order: Vec<i64> = list.json["photos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_i64().unwrap())
        .collect();
    assert_eq!(order, reversed);

    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/areas/{ut}/photos/{}", member_ids.1),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    // Every change is in the audit log.
    let audited: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT action FROM audit_logs WHERE entity='area' AND entity_id=$1 ORDER BY action",
    )
    .bind(ut)
    .fetch_all(&st.db)
    .await
    .unwrap();
    assert_eq!(
        audited,
        [
            "area.photo.add",
            "area.photo.delete",
            "area.photo.reorder",
            "area.photo.update"
        ]
    );
}

#[tokio::test]
async fn riders_and_drivers_see_area_photos() {
    let (app, st) = setup().await;
    let um = area_id(&st, "UM").await;
    let head = login(&st, "traffichead@example.org").await;
    let file = upload_png(&app, &head).await;
    let r = add(&app, &head, um, file, "Wait at the guardhouse").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);

    for email in ["member@example.org", "driver@example.org"] {
        let who = login(&st, email).await;
        let list = photos(&app, &who, um).await;
        assert_eq!(list.status, StatusCode::OK, "{email}: {}", list.json);
        let p = &list.json["photos"][0];
        assert_eq!(p["caption"], "Wait at the guardhouse");
        assert_eq!(p["can_edit"], false);
        assert_eq!(p["can_delete"], false);
        assert!(
            p.get("created_by").is_none(),
            "riders don't see who added it"
        );
        assert_eq!(list.json["can_add"], false);
        // The image itself opens for them, though they didn't upload it.
        let img = call(
            &app,
            Method::GET,
            &format!("/api/files/{file}"),
            Some(&who),
            None,
        )
        .await;
        assert_eq!(img.status, StatusCode::OK, "{email} opens the photo");
        // The area list says how many photos each area has.
        let areas = call(&app, Method::GET, "/api/areas", Some(&who), None).await;
        let row = areas
            .json
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == um)
            .unwrap()
            .clone();
        assert_eq!(row["photo_count"], 1);
    }

    // Removing the photo removes the upload, so the link stops working.
    let pid = r.json["id"].as_i64().unwrap();
    let r = call(
        &app,
        Method::DELETE,
        &format!("/api/areas/{um}/photos/{pid}"),
        Some(&head),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let gone: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE id=$1")
        .bind(file)
        .fetch_one(&st.db)
        .await
        .unwrap();
    assert_eq!(gone, 0);
}

#[tokio::test]
async fn area_photo_limits() {
    let (app, st) = setup().await;
    let kk = area_id(&st, "KK").await;
    let head = login(&st, "traffichead@example.org").await;
    let admin = login(&st, "admin@example.org").await;

    // Someone else's upload, a PDF, an over-long caption and an unknown area are refused.
    let theirs = upload_png(&app, &admin).await;
    let r = add(&app, &head, kk, theirs, "").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["code"], "INVALID_PHOTO");
    let pdf = upload(&app, &head, "map.pdf", b"%PDF-1.4 hello").await;
    let r = add(&app, &head, kk, pdf, "").await;
    assert_eq!(r.json["code"], "INVALID_PHOTO");
    let png = upload_png(&app, &head).await;
    let r = add(&app, &head, kk, png, &"x".repeat(121)).await;
    assert_eq!(r.json["code"], "AREA_PHOTO_CAPTION_TOO_LONG");
    let r = add(&app, &head, 999_999, png, "").await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // A JPEG loses its Exif (location) when attached.
    let jpg = upload(&app, &head, "gate.jpg", &jpeg_with_exif()).await;
    let r = add(&app, &head, kk, jpg, "Gate").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let stored: String = sqlx::query_scalar("SELECT stored_name FROM files WHERE id=$1")
        .bind(jpg)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let bytes = std::fs::read(st.cfg.upload_dir.join(stored)).unwrap();
    assert!(!bytes.windows(4).any(|w| w == b"Exif"), "Exif stripped");
    assert!(bytes.starts_with(&[0xFF, 0xD8]) && bytes.ends_with(&[0xFF, 0xD9]));

    // The same upload cannot be added twice.
    let r = add(&app, &head, kk, jpg, "").await;
    assert_eq!(r.json["code"], "INVALID_PHOTO");

    // Ten photos per area.
    for _ in 1..10 {
        let f = upload_png(&app, &head).await;
        let r = add(&app, &head, kk, f, "").await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    }
    let f = upload_png(&app, &head).await;
    let r = add(&app, &head, kk, f, "").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["code"], "AREA_PHOTO_LIMIT");
}
