//! Receipt / photo uploads with per-use access control
//! (收据仅让本人、收款者与获授权财务查看).

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, too_many_requests, AppError, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, in_hours, now};

pub fn router() -> Router<AppState> {
    Router::new()
        // Uploads are the one route allowed a large body; every other API request is
        // capped at `crate::MAX_JSON_BODY` by the app-wide limit.
        .route(
            "/files",
            post(upload).layer(DefaultBodyLimit::max(MAX_BYTES + 1024 * 1024)),
        )
        .route("/files/{id}", get(download))
}

const MAX_BYTES: usize = 8 * 1024 * 1024;
/// Files one member may upload in 24 hours. Receipts and photos are a handful a week;
/// this only stops one account filling the disk (the per-IP limit resets every 10 min).
pub const MAX_UPLOADS_PER_DAY: i64 = 100;

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
    let (today,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM files WHERE owner_id=$1 AND created_at >= $2")
            .bind(u.id)
            .bind(in_hours(-24))
            .fetch_one(&st.db)
            .await?;
    if today >= MAX_UPLOADS_PER_DAY {
        return Err(too_many_requests(u.lang, ErrorKey::UploadQuotaReached));
    }
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|_| bad(u.lang, ErrorKey::UploadFailed))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let original = field.file_name().unwrap_or("upload").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|_| bad(u.lang, ErrorKey::UploadFailed))?;
        if data.len() > MAX_BYTES {
            return Err(bad(u.lang, ErrorKey::FileTooLarge));
        }
        // Trust the bytes, not the browser-supplied Content-Type.
        let (mime, ext) = sniff(&data).ok_or_else(|| bad(u.lang, ErrorKey::InvalidFileType))?;
        let mime = mime.to_string();
        let stored = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        tokio::fs::create_dir_all(&st.cfg.upload_dir)
            .await
            .map_err(anyhow::Error::from)?;
        tokio::fs::write(st.cfg.upload_dir.join(&stored), &data)
            .await
            .map_err(anyhow::Error::from)?;
        let (id,): (i64,) = sqlx::query_as("INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id")
            .bind(u.id)
            .bind(&stored)
            .bind(&mime)
            .bind(original.chars().take(120).collect::<String>())
            .bind(data.len() as i64)
            .bind(now())
            .fetch_one(&st.db)
            .await?;
        audit(
            &st.db,
            Some(u.id),
            "file.upload",
            "file",
            Some(id),
            format!("{mime}, {} bytes", data.len()),
        )
        .await;
        return Ok(Json(json!({ "id": id, "mime": mime, "name": original })));
    }
    Err(bad(u.lang, ErrorKey::NoFileReceived))
}

pub async fn can_access_file(st: &AppState, u: &CurrentUser, file_id: i64) -> AppResult<bool> {
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
        .bind(file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;
    if owner == u.id {
        return Ok(true);
    }
    let bills = u.can(Page::AdminBills, Act::View);
    // Profile photos are visible to members (passengers recognise their driver).
    let (photo,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE photo_file_id=$1")
        .bind(file_id)
        .fetch_one(&st.db)
        .await?;
    if photo > 0 {
        return Ok(true);
    }
    // Settlement expense receipts: 账单查阅 (finance). The organiser attaches only their own
    // uploads (checked when the bill is saved) and reaches them through the owner
    // check above, so naming another member's file id on a bill opens nothing.
    // Participants see the bill lines but not the receipts.
    if bills {
        let (item,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM settlement_items WHERE receipt_file_id=$1")
                .bind(file_id)
                .fetch_one(&st.db)
                .await?;
        if item > 0 {
            return Ok(true);
        }
    }
    // Payment proofs: payer (owner), organiser/payee, 账单查阅 (finance).
    let (proof,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id
         WHERE sh.proof_file_id=$1 AND (s.created_by=$2 OR $3)",
    )
    .bind(file_id)
    .bind(u.id)
    .bind(bills)
    .fetch_one(&st.db)
    .await?;
    if proof > 0 {
        return Ok(true);
    }
    // Help request photos: the request's member and 帮助台 staff.
    let (help,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM help_replies r JOIN help_tickets t ON t.id=r.ticket_id
         WHERE r.file_id=$1 AND (t.user_id=$2 OR $3)",
    )
    .bind(file_id)
    .bind(u.id)
    .bind(u.can(Page::AdminHelp, Act::View))
    .fetch_one(&st.db)
    .await?;
    if help > 0 {
        return Ok(true);
    }
    // Grab reimbursement receipts: claimant, 报销审核 (交通组), 报销付款 (finance).
    let (reimb,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reimbursements WHERE receipt_file_id=$1 AND ($2 OR $3)",
    )
    .bind(file_id)
    .bind(u.can(Page::AdminReimbReview, Act::View))
    .bind(u.can(Page::AdminReimbPay, Act::View))
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
        return Err(forbidden(u.lang, ErrorKey::CannotViewFile));
    }
    let (stored, mime, name): (String, String, String) =
        sqlx::query_as("SELECT stored_name, mime, original_name FROM files WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await?;
    let data = tokio::fs::read(st.cfg.upload_dir.join(&stored))
        .await
        .map_err(|_| not_found(u.lang))?;
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
