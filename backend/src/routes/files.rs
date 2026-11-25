//! Receipt / photo uploads with per-use access control
//! (收据仅让本人、收款者与获授权财务查看).

use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;

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
        .route("/files/{id}/extract-ref", post(extract_ref))
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
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
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

/// A JPEG without its APP1 (Exif, incl. GPS location; XMP) and APP13 (IPTC) segments.
/// `None` when the data is not a well-formed JPEG header or nothing was removed.
pub fn strip_jpeg_metadata(data: &[u8]) -> Option<Vec<u8>> {
    if !data.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..2]);
    let mut pos = 2;
    let mut removed = false;
    loop {
        if pos + 1 >= data.len() || data[pos] != 0xFF {
            return None;
        }
        let marker = data[pos + 1];
        // Fill bytes before a marker.
        if marker == 0xFF {
            pos += 1;
            continue;
        }
        // Start of scan or end of image: the rest is image data, copied as it is.
        if marker == 0xDA || marker == 0xD9 {
            out.extend_from_slice(&data[pos..]);
            break;
        }
        // Markers without a length.
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            out.extend_from_slice(&data[pos..pos + 2]);
            pos += 2;
            continue;
        }
        if pos + 4 > data.len() {
            return None;
        }
        let len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        if len < 2 || pos + 2 + len > data.len() {
            return None;
        }
        let end = pos + 2 + len;
        if marker == 0xE1 || marker == 0xED {
            removed = true;
        } else {
            out.extend_from_slice(&data[pos..end]);
        }
        pos = end;
    }
    removed.then_some(out)
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
        // Strip EXIF/XMP/IPTC (including GPS tags) on all JPEG uploads regardless of usage.
        let stripped = if mime == "image/jpeg" {
            strip_jpeg_metadata(&data)
        } else {
            None
        };
        let payload = stripped.as_deref().unwrap_or(&data);
        let mime = mime.to_string();
        let stored = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        let path = st.cfg.upload_dir.join(&stored);
        tokio::fs::create_dir_all(&st.cfg.upload_dir)
            .await
            .map_err(anyhow::Error::from)?;
        tokio::fs::write(&path, payload)
            .await
            .map_err(anyhow::Error::from)?;
        let inserted: Result<(i64,), sqlx::Error> = sqlx::query_as(
            "INSERT INTO files (owner_id, stored_name, mime, original_name, size, created_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
        )
        .bind(u.id)
        .bind(&stored)
        .bind(&mime)
        .bind(original.chars().take(120).collect::<String>())
        .bind(payload.len() as i64)
        .bind(now())
        .fetch_one(&st.db)
        .await;
        let (id,) = match inserted {
            Ok(row) => row,
            Err(e) => {
                let _ = tokio::fs::remove_file(&path).await;
                return Err(e.into());
            }
        };
        audit(
            &st.db,
            Some(u.id),
            "file.upload",
            "file",
            Some(id),
            format!("{mime}, {} bytes", payload.len()),
        )
        .await;
        return Ok(Json(json!({ "id": id, "mime": mime, "name": original })));
    }
    Err(bad(u.lang, ErrorKey::NoFileReceived))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileAccess {
    Denied,
    Private,
    CacheableMedia,
}

async fn file_access(st: &AppState, u: &CurrentUser, file_id: i64) -> AppResult<FileAccess> {
    let (owner,): (i64,) = sqlx::query_as("SELECT owner_id FROM files WHERE id=$1")
        .bind(file_id)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))?;
    // Pending (unapproved) accounts can only access files they uploaded themselves.
    if u.status != "active" {
        return Ok(if owner == u.id {
            FileAccess::Private
        } else {
            FileAccess::Denied
        });
    }
    // Profile photos are visible to members (passengers recognise their driver).
    let (photo,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE photo_file_id=$1")
        .bind(file_id)
        .fetch_one(&st.db)
        .await?;
    if photo > 0 {
        return Ok(FileAccess::CacheableMedia);
    }
    // Pickup-area photos: every active member (passengers recognise where to wait).
    let (area_photo,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM area_photos WHERE file_id=$1")
        .bind(file_id)
        .fetch_one(&st.db)
        .await?;
    if area_photo > 0 {
        return Ok(FileAccess::CacheableMedia);
    }
    // Event cover photos: everyone who can see the event.
    let events: Vec<i64> = sqlx::query_scalar("SELECT id FROM events WHERE photo_file_id=$1")
        .bind(file_id)
        .fetch_all(&st.db)
        .await?;
    for ev in events {
        if crate::routes::events::can_see_event(st, u, ev).await? {
            return Ok(FileAccess::CacheableMedia);
        }
    }
    // Guide step screenshots: members who may view Help and can see that guide.
    if u.can(Page::Help, Act::View) {
        let staff = crate::routes::community::any_admin_page(u);
        let can_edit = u.can(Page::AdminHelp, Act::Update);
        let needle = json!([{"image_file_id": file_id}]).to_string();
        let (guide,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM guide_workflows
             WHERE steps @> $1::jsonb AND ($2 OR published) AND ($3 OR audience='member')",
        )
        .bind(needle)
        .bind(can_edit)
        .bind(staff)
        .fetch_one(&st.db)
        .await?;
        if guide > 0 {
            return Ok(FileAccess::CacheableMedia);
        }
    }
    if owner == u.id {
        return Ok(FileAccess::Private);
    }
    let bills = u.can(Page::AdminBills, Act::View);
    if bills {
        let (item,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM settlement_items WHERE receipt_file_id=$1")
                .bind(file_id)
                .fetch_one(&st.db)
                .await?;
        if item > 0 {
            return Ok(FileAccess::Private);
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
        return Ok(FileAccess::Private);
    }
    // Payee DuitNow / TNG QR codes attached to bills: visible to participants once published and to finance.
    let (qr,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM settlements s LEFT JOIN settlement_shares sh ON sh.settlement_id=s.id
         WHERE s.payee_qr_file_id=$1 AND ((sh.user_id=$2 AND s.status <> 'draft') OR $3)",
    )
    .bind(file_id)
    .bind(u.id)
    .bind(bills)
    .fetch_one(&st.db)
    .await?;
    if qr > 0 {
        return Ok(FileAccess::Private);
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
        return Ok(FileAccess::Private);
    }
    // Grab reimbursement receipts: claimant, 报销审核 (交通组), 报销付款 (finance).
    let (reimb,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reimbursements WHERE receipt_file_id=$1 AND (user_id=$2 OR $3 OR $4)",
    )
    .bind(file_id)
    .bind(u.id)
    .bind(u.can(Page::AdminReimbReview, Act::View))
    .bind(u.can(Page::AdminReimbPay, Act::View))
    .fetch_one(&st.db)
    .await?;
    if reimb > 0 {
        return Ok(FileAccess::Private);
    }
    Ok(FileAccess::Denied)
}

pub async fn can_access_file(st: &AppState, u: &CurrentUser, file_id: i64) -> AppResult<bool> {
    Ok(file_access(st, u, file_id).await? != FileAccess::Denied)
}

fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
    let Some(raw) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    raw.split(',')
        .map(str::trim)
        .any(|tag| tag == "*" || tag == etag || tag.strip_prefix("W/") == Some(etag))
}

async fn download(
    State(st): State<AppState>,
    u: CurrentUser,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    let access = file_access(&st, &u, id).await?;
    if access == FileAccess::Denied {
        return Err(forbidden(u.lang, ErrorKey::CannotViewFile));
    }
    let (stored, mime, name, size): (String, String, String, i64) =
        sqlx::query_as("SELECT stored_name, mime, original_name, size FROM files WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await?;
    let etag = format!("\"{stored}-{size}\"");
    let cache_control = if access == FileAccess::CacheableMedia {
        "private, no-cache"
    } else {
        "private, no-store"
    };
    if access == FileAccess::CacheableMedia && etag_matches(&headers, &etag) {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, &etag)
            .header(header::CACHE_CONTROL, cache_control)
            .header("X-Content-Type-Options", "nosniff")
            .header(
                "Content-Security-Policy",
                "default-src 'none'; img-src 'self'; sandbox",
            )
            .body(Body::empty())
            .map_err(|e| AppError::Other(anyhow::anyhow!(e)));
    }
    let mut file = tokio::fs::File::open(st.cfg.upload_dir.join(&stored))
        .await
        .map_err(|_| not_found(u.lang))?;
    let content_len = file
        .metadata()
        .await
        .map(|m| m.len())
        .unwrap_or_else(|_| size.max(0) as u64);
    let stream = async_stream::stream! {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match file.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => yield Ok::<Bytes, std::io::Error>(Bytes::copy_from_slice(&buf[..n])),
                Err(e) => {
                    yield Err(e);
                    break;
                }
            }
        }
    };
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
        .header(header::CONTENT_LENGTH, content_len.to_string())
        .header(header::ETAG, &etag)
        .header(header::CACHE_CONTROL, cache_control)
        .header("X-Content-Type-Options", "nosniff")
        .header(
            "Content-Security-Policy",
            "default-src 'none'; img-src 'self'; sandbox",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("{disposition}; filename=\"{safe_name}\""),
        )
        .body(Body::from_stream(stream))
        .map_err(|e| AppError::Other(anyhow::anyhow!(e)))
}

#[derive(Deserialize)]
struct ExtractRefIn {
    text: Option<String>,
}

fn is_ref_token(tok: &str) -> bool {
    let clean = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_');
    if clean.len() < 6 || clean.len() > 36 {
        return false;
    }
    let upper = clean.to_ascii_uppercase();
    const NOISE: &[&str] = &[
        "APPLICATION",
        "CHROME",
        "COMMENT",
        "CONTENT",
        "CREATE",
        "DEFAULT",
        "DEFLATE",
        "DUITNOW",
        "ENDOF",
        "FILTER",
        "HEIGHT",
        "HEADER",
        "IMAGE",
        "JFIF",
        "LENGTH",
        "LINEARIZED",
        "MODDATE",
        "OBJECT",
        "PREVIEW",
        "PRODUCER",
        "RECEIPT",
        "REFERENCE",
        "RGB",
        "SRGB",
        "STREAM",
        "TRANSFER",
        "WIDTH",
    ];
    if NOISE.contains(&upper.as_str()) {
        return false;
    }
    let digits = clean.bytes().filter(|b| b.is_ascii_digit()).count();
    let alpha = clean.bytes().filter(|b| b.is_ascii_alphabetic()).count();
    if digits < 3 {
        return false;
    }
    // High-confidence prefixes (MY banks / wallets / ref codes) or >= 8 digits
    let prefixed = [
        "DN", "TNG", "M2U", "MAE", "CIMB", "RHB", "HLB", "PBE", "FPX", "IBG", "REF", "TXN", "TRX",
        "MYCN", "GRAB", "DUIT", "APP",
    ]
    .iter()
    .any(|p| upper.starts_with(p));
    prefixed || digits >= 8 || (alpha >= 2 && digits >= 4)
}

fn extract_ascii_runs(data: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for &b in data.iter().take(512 * 1024) {
        if (0x20..=0x7E).contains(&b) {
            cur.push(b);
        } else if cur.len() >= 6 {
            if let Ok(s) = std::str::from_utf8(&cur) {
                out.push(s.to_string());
            }
            cur.clear();
        } else {
            cur.clear();
        }
    }
    if cur.len() >= 6 {
        if let Ok(s) = std::str::from_utf8(&cur) {
            out.push(s.to_string());
        }
    }
    out
}

pub fn extract_receipt_ref(
    data: &[u8],
    original_name: &str,
    extra_text: Option<&str>,
) -> Vec<String> {
    let mut sources: Vec<String> = Vec::new();
    if let Some(t) = extra_text.map(str::trim).filter(|s| !s.is_empty()) {
        sources.push(t.to_string());
    }
    sources.extend(extract_ascii_runs(data));
    if !original_name.trim().is_empty() {
        sources.push(original_name.to_string());
    }

    let mut labelled = Vec::new();
    let mut unlabelled = Vec::new();
    let labels = [
        "reference no",
        "ref no",
        "reference",
        "txn ref",
        "trx id",
        "transaction id",
        "trace no",
        "approval code",
        "duitnow",
        "tng",
        "fpx",
        "ref",
        "txn",
    ];

    for line in &sources {
        let lower = line.to_ascii_lowercase();
        for lbl in labels {
            if let Some(pos) = lower.find(lbl) {
                let tail = &line[pos + lbl.len()..];
                for word in tail.split(|c: char| {
                    c.is_whitespace() || matches!(c, ':' | '#' | '=' | '(' | ')' | ',' | ';' | '"')
                }) {
                    let clean = word
                        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_');
                    if is_ref_token(clean) && !labelled.iter().any(|x: &String| x == clean) {
                        labelled.push(clean.to_string());
                        break;
                    }
                }
            }
        }
        for word in line.split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    ':' | '#' | '=' | '(' | ')' | ',' | ';' | '"' | '/' | '\\'
                )
        }) {
            let clean =
                word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_');
            if is_ref_token(clean)
                && !labelled.iter().any(|x| x == clean)
                && !unlabelled.iter().any(|x: &String| x == clean)
            {
                unlabelled.push(clean.to_string());
            }
        }
    }

    labelled.extend(unlabelled);
    labelled.truncate(5);
    labelled
}

async fn extract_ref(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    body: Bytes,
) -> AppResult<Json<Value>> {
    use tokio::io::AsyncReadExt;

    let file_row: Option<(i64, String, String)> =
        sqlx::query_as("SELECT owner_id, stored_name, original_name FROM files WHERE id=$1")
            .bind(id)
            .fetch_optional(&st.db)
            .await?;
    let Some((owner_id, stored, name)) = file_row else {
        return Err(not_found(u.lang));
    };

    let mut allowed = owner_id == u.id;
    if !allowed
        && (u.can(Page::AdminBills, Act::View)
            || u.can(Page::AdminReimbReview, Act::View)
            || u.can(Page::AdminReimbPay, Act::View))
        && can_access_file(&st, &u, id).await?
    {
        let finance_doc: i64 = sqlx::query_scalar(
            "SELECT (SELECT COUNT(*) FROM reimbursements WHERE receipt_file_id=$1)
                  + (SELECT COUNT(*) FROM settlement_items WHERE receipt_file_id=$1)
                  + (SELECT COUNT(*) FROM settlement_shares WHERE proof_file_id=$1)",
        )
        .bind(id)
        .fetch_one(&st.db)
        .await?;
        allowed = finance_doc > 0;
    }
    if !allowed {
        let organizer_doc: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM settlement_shares sh JOIN settlements s ON s.id=sh.settlement_id
             WHERE sh.proof_file_id=$1 AND s.created_by=$2",
        )
        .bind(id)
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
        allowed = organizer_doc > 0;
    }
    if !allowed {
        return Err(forbidden(u.lang, ErrorKey::CannotViewFile));
    }

    let extra = serde_json::from_slice::<ExtractRefIn>(&body)
        .ok()
        .and_then(|v| v.text);
    let mut data = Vec::new();
    if let Ok(f) = tokio::fs::File::open(st.cfg.upload_dir.join(&stored)).await {
        let _ = f.take(512 * 1024).read_to_end(&mut data).await;
    }
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let candidates = extract_receipt_ref(&data, &safe_name, extra.as_deref());
    let best = candidates.first().cloned().unwrap_or_default();
    Ok(Json(json!({
        "ref": best,
        "candidates": candidates,
    })))
}

#[cfg(test)]
mod tests {
    use super::strip_jpeg_metadata;

    #[test]
    fn strips_exif_and_keeps_the_image() {
        let app0 = [0xFF, 0xE0, 0x00, 0x04, 0x4A, 0x46];
        let exif = [0xFF, 0xE1, 0x00, 0x06, b'E', b'x', b'i', b'f'];
        let scan = [0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22, 0xFF, 0xD9];
        let mut jpg = vec![0xFF, 0xD8];
        jpg.extend_from_slice(&app0);
        jpg.extend_from_slice(&exif);
        jpg.extend_from_slice(&scan);
        let clean = strip_jpeg_metadata(&jpg).expect("exif removed");
        let mut want = vec![0xFF, 0xD8];
        want.extend_from_slice(&app0);
        want.extend_from_slice(&scan);
        assert_eq!(clean, want);
    }

    #[test]
    fn leaves_clean_or_broken_files_alone() {
        let clean = [0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9];
        assert_eq!(strip_jpeg_metadata(&clean), None);
        assert_eq!(strip_jpeg_metadata(&[0xFF, 0xD8, 0xFF, 0xE1, 0xFF]), None);
        assert_eq!(strip_jpeg_metadata(b"\x89PNG"), None);
    }
}
