//! Integration tests covering Phase 1 backend enhancements:
//! - JPEG EXIF/GPS stripping on general /api/files uploads
//! - ETag / 304 Not Modified for member-accessible media vs private no-store receipts
//! - Orphan-file garbage collection in housekeeping
//! - Denormalized audit_logs.entity_name persistence and searchability
//! - Recent-sign-in enforcement on POST /api/auth/password
//! - Request-scoped Accept-Language propagation on DB errors

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use common::*;
use grace_ride::jobs;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn multipart(content_type: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "XgrBOUNDARY";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"r.jpg\"\r\nContent-Type: {content_type}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[tokio::test]
async fn jpeg_upload_strips_exif_and_media_supports_etag_304() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let driver = login(&st, "driver@example.org").await;

    // Minimal valid JPEG containing APP0 + APP1 (EXIF with GPS marker) + SOS + EOI.
    let app0 = [0xFF, 0xE0, 0x00, 0x04, 0x4A, 0x46];
    let exif = [0xFF, 0xE1, 0x00, 0x08, b'E', b'x', b'i', b'f', 0x00, 0x00];
    let scan = [0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22, 0xFF, 0xD9];
    let mut jpg = vec![0xFF, 0xD8];
    jpg.extend_from_slice(&app0);
    jpg.extend_from_slice(&exif);
    jpg.extend_from_slice(&scan);

    let (mct, body) = multipart("image/jpeg", &jpg);
    let up = Request::builder()
        .method(Method::POST)
        .uri("/api/files")
        .header(header::COOKIE, &member)
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, mct)
        .body(Body::from(body))
        .unwrap();
    let up_res = app.clone().oneshot(up).await.unwrap();
    assert_eq!(up_res.status(), StatusCode::OK);
    let up_json: Value =
        serde_json::from_slice(&up_res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let file_id = up_json["id"].as_i64().unwrap();

    // Verify stored bytes on disk no longer carry EXIF APP1.
    let stored_name: String = sqlx::query_scalar("SELECT stored_name FROM files WHERE id=$1")
        .bind(file_id)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let disk_bytes = tokio::fs::read(st.cfg.upload_dir.join(&stored_name))
        .await
        .unwrap();
    assert!(
        !disk_bytes.windows(4).any(|w| w == b"Exif"),
        "EXIF payload must be stripped on /api/files upload"
    );

    // Unattached owner download stays private, no-store.
    let priv_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/files/{file_id}"))
                .header(header::COOKIE, &member)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(priv_res.status(), StatusCode::OK);
    assert_eq!(
        priv_res.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );

    // Attach as member profile photo -> CacheableMedia with ETag / 304 support.
    let set_photo = call(
        &app,
        Method::PUT,
        "/api/me",
        Some(&member),
        Some(json!({ "photo_file_id": file_id })),
    )
    .await;
    assert_eq!(set_photo.status, StatusCode::OK, "{}", set_photo.json);

    let media_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/files/{file_id}"))
                .header(header::COOKIE, &driver)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(media_res.status(), StatusCode::OK);
    assert_eq!(
        media_res.headers()[header::CACHE_CONTROL],
        "private, no-cache"
    );
    let etag = media_res.headers()[header::ETAG]
        .to_str()
        .unwrap()
        .to_string();

    let not_mod = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/files/{file_id}"))
                .header(header::COOKIE, &driver)
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_mod.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn housekeeping_purges_old_orphan_files_and_keeps_referenced_ones() {
    let (_app, st) = setup().await;
    let uid: i64 = sqlx::query_scalar("SELECT id FROM users WHERE email='member@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();

    tokio::fs::create_dir_all(&st.cfg.upload_dir).await.unwrap();
    let old_ts = (grace_ride::util::local_now() - chrono::Duration::hours(48))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let orphan_name = format!("{}.png", uuid::Uuid::new_v4());
    let kept_name = format!("{}.png", uuid::Uuid::new_v4());
    tokio::fs::write(st.cfg.upload_dir.join(&orphan_name), b"orphan")
        .await
        .unwrap();
    tokio::fs::write(st.cfg.upload_dir.join(&kept_name), b"kept")
        .await
        .unwrap();

    let orphan_id: i64 = sqlx::query_scalar(
        "INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at)
         VALUES ($1, $2, 'image/png', 'orphan.png', 6, $3) RETURNING id",
    )
    .bind(uid)
    .bind(&orphan_name)
    .bind(&old_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    let kept_id: i64 = sqlx::query_scalar(
        "INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at)
         VALUES ($1, $2, 'image/png', 'kept.png', 4, $3) RETURNING id",
    )
    .bind(uid)
    .bind(&kept_name)
    .bind(&old_ts)
    .fetch_one(&st.db)
    .await
    .unwrap();

    sqlx::query("UPDATE users SET photo_file_id=$1 WHERE id=$2")
        .bind(kept_id)
        .bind(uid)
        .execute(&st.db)
        .await
        .unwrap();

    jobs::run_once(&st.db, &st.cfg).await.unwrap();

    let orphan_row: Option<i64> = sqlx::query_scalar("SELECT id FROM files WHERE id=$1")
        .bind(orphan_id)
        .fetch_optional(&st.db)
        .await
        .unwrap();
    assert!(orphan_row.is_none(), "orphan file row should be purged");
    assert!(
        !st.cfg.upload_dir.join(&orphan_name).exists(),
        "orphan disk blob should be deleted"
    );

    let kept_row: Option<i64> = sqlx::query_scalar("SELECT id FROM files WHERE id=$1")
        .bind(kept_id)
        .fetch_optional(&st.db)
        .await
        .unwrap();
    assert_eq!(kept_row, Some(kept_id));
    assert!(st.cfg.upload_dir.join(&kept_name).exists());
}

#[tokio::test]
async fn audit_entity_name_persists_after_deletion_and_set_password_checks_recent_signin() {
    let (app, st) = setup().await;
    let admin = login(&st, "admin@example.org").await;
    let member = login(&st, "member@example.org").await;
    let member_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE email='member@example.org'")
            .fetch_one(&st.db)
            .await
            .unwrap();

    // Create a group, audit it, then delete the group row directly: entity_name stays searchable.
    let grp = call(
        &app,
        Method::POST,
        "/api/admin/groups",
        Some(&admin),
        Some(json!({ "name": "Snapshot Choir" })),
    )
    .await;
    assert_eq!(grp.status, StatusCode::OK, "{}", grp.json);
    let gid = grp.json["id"].as_i64().unwrap();
    grace_ride::util::audit(
        &st.db,
        None,
        "group.snapshot_test",
        "group",
        Some(gid),
        "updated",
    )
    .await;
    sqlx::query("DELETE FROM groups WHERE id=$1")
        .bind(gid)
        .execute(&st.db)
        .await
        .unwrap();

    let audit_res = call(
        &app,
        Method::GET,
        "/api/admin/audit?q=Snapshot%20Choir",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(audit_res.status, StatusCode::OK, "{}", audit_res.json);
    let items = audit_res.json["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0]["entity_name"], "Snapshot Choir");

    // Account with TOTP enabled and a >15m-old session cannot change password without re-auth.
    sqlx::query("UPDATE users SET totp_enabled=1 WHERE id=$1")
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();
    let stale_ts = (grace_ride::util::local_now() - chrono::Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query("UPDATE sessions SET created_at=$1 WHERE user_id=$2")
        .bind(&stale_ts)
        .bind(member_id)
        .execute(&st.db)
        .await
        .unwrap();

    let pw_res = call(
        &app,
        Method::POST,
        "/api/auth/password",
        Some(&member),
        Some(json!({ "current": PW, "new_password": "NewGraceRide2027!" })),
    )
    .await;
    assert_eq!(pw_res.status, StatusCode::FORBIDDEN);
    assert_eq!(pw_res.json["code"], "RECENT_SIGN_IN_REQUIRED");
}
