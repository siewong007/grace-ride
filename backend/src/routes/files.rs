//! Receipt / photo uploads with per-use access control
//! (收据仅让本人、收款者与获授权财务查看).

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, AppError, AppResult};
use crate::state::AppState;
use crate::util::now;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/files", post(upload))
        .route("/files/{id}", get(download))
}

const MAX_BYTES: usize = 8 * 1024 * 1024;

/// Identify allowed file types by magic bytes.
pub fn sniff(b: &[u8]) -> Option<(&'static str, &'static str)> {
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("image/jpeg", "jpg"))
    } else if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(("image/png", "png"))
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else if b.len() >= 12
        && &b[4..8] == b"ftyp"
        && matches!(&b[8..12], b"heic" | b"heix" | b"mif1" | b"msf1" | b"hevc")
    {
        Some(("image/heic", "heic"))
    } else if b.starts_with(b"%PDF-") {
        Some(("application/pdf", "pdf"))
    } else {
        None
    }
}

async fn upload(
    State(st): State<AppState>,
    u: CurrentUser,
    mut mp: Multipart,
) -> AppResult<Json<Value>> {
    while let Some(field) = mp.next_field().await.map_err(|_| bad("上传失败"))? {
        if field.name() != Some("file") {
            continue;
        }
        let original = field.file_name().unwrap_or("upload").to_string();
        let data = field.bytes().await.map_err(|_| bad("上传失败"))?;
        if data.len() > MAX_BYTES {
            return Err(bad("文件不能超过 8MB"));
        }
        // Trust the bytes, not the browser-supplied Content-Type.
        let (mime, ext) =
            sniff(&data).ok_or_else(|| bad("只接受图片（JPG/PNG/WEBP/HEIC）或 PDF"))?;
        let mime = mime.to_string();
        let stored = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        tokio::fs::create_dir_all(&st.cfg.upload_dir)
            .await
            .map_err(anyhow::Error::from)?;
        tokio::fs::write(st.cfg.upload_dir.join(&stored), &data)
            .await
            .map_err(anyhow::Error::from)?;
        let id = sqlx::query("INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at) VALUES (?,?,?,?,?,?)")
            .bind(u.id)
            .bind(&stored)
            .bind(&mime)
            .bind(original.chars().take(120).collect::<String>())
            .bind(data.len() as i64)
            .bind(now())
            .execute(&st.db)
            .await?
            .last_insert_rowid();
        return Ok(Json(json!({ "id": id, "mime": mime, "name": original })));
    }
    Err(bad("没有收到文件"))
}

pub async fn can_access_file(st: &AppState, u: &CurrentUser, file_id: i64) -> AppResult<bool> {
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=?")
        .bind(file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(AppError::NotFound)?;
    if owner == u.id {
        return Ok(true);
    }
    let finance = u.has("finance") && u.mfa_ok;
    // Profile photos are visible to members (passengers recognise their driver).
    let (photo,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE photo_file_id=?")
        .bind(file_id)
        .fetch_one(&st.db)
        .await?;
    if photo > 0 {
        return Ok(true);
    }
    // Settlement expense receipts: organiser, participants, finance.
    let (item,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_items i JOIN settlements s ON s.id=i.settlement_id
         WHERE i.receipt_file_id=? AND (s.created_by=? OR ? OR EXISTS (SELECT 1 FROM settlement_shares sh WHERE sh.settlement_id=s.id AND sh.user_id=?))",
    )
    .bind(file_id)
    .bind(u.id)
    .bind(finance)
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
    if item > 0 {
        return Ok(true);
    }
    // Payment proofs: payer (owner), organiser/payee, finance.
    let (proof,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id
         WHERE sh.proof_file_id=? AND (s.created_by=? OR ?)",
    )
    .bind(file_id)
    .bind(u.id)
    .bind(finance)
    .fetch_one(&st.db)
    .await?;
    if proof > 0 {
        return Ok(true);
    }
    // Grab reimbursement receipts: claimant, 交通组, finance.
    let (reimb,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM reimbursements WHERE receipt_file_id=? AND (? OR ?)")
            .bind(file_id)
            .bind(u.is_staff())
            .bind(finance)
            .fetch_one(&st.db)
            .await?;
    Ok(reimb > 0)
}

async fn download(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if !can_access_file(&st, &u, id).await? {
        return Err(forbidden("您无法查看此文件"));
    }
    let (stored, mime, name): (String, String, String) =
        sqlx::query_as("SELECT stored_name, mime, original_name FROM files WHERE id=?")
            .bind(id)
            .fetch_one(&st.db)
            .await?;
    let data = tokio::fs::read(st.cfg.upload_dir.join(&stored))
        .await
        .map_err(|_| AppError::NotFound)?;
    let safe_name: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    let safe_name = if safe_name.is_empty() {
        "file".to_string()
    } else {
        safe_name
    };
    // Images render inline (receipt previews). PDFs are downloaded rather than rendered
    // on our origin, and every file response is sandboxed so it can never run script.
    let disposition = if mime.starts_with("image/") {
        "inline"
    } else {
        "attachment"
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "private, no-store")
        .header("X-Content-Type-Options", "nosniff")
        .header(
            "Content-Security-Policy",
            "default-src 'none'; img-src 'self'; sandbox",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("{disposition}; filename=\"{safe_name}\""),
        )
        .body(Body::from(data))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}
