//! Receipt / photo uploads with per-use access control
//! (收据仅让本人、收款者与获授权财务查看).

use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
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
    // Pending (unapproved) accounts can only access files they uploaded themselves.
    if u.status != "active" {
        return Ok(false);
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
    // Event cover photos: everyone who can see the event.
    let events: Vec<i64> = sqlx::query_scalar("SELECT id FROM events WHERE photo_file_id=$1")
        .bind(file_id)
        .fetch_all(&st.db)
        .await?;
    for ev in events {
        if crate::routes::events::can_see_event(st, u, ev).await? {
            return Ok(true);
        }
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
    if reimb > 0 {
        return Ok(true);
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
            return Ok(true);
        }
    }
    Ok(false)
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
    if !can_access_file(&st, &u, id).await? {
        return Err(forbidden(u.lang, ErrorKey::CannotViewFile));
    }
    let (stored, _mime, name): (String, String, String) =
        sqlx::query_as("SELECT stored_name, mime, original_name FROM files WHERE id=$1")
            .bind(id)
            .fetch_one(&st.db)
            .await?;
    let extra = serde_json::from_slice::<ExtractRefIn>(&body)
        .ok()
        .and_then(|v| v.text);
    let data = tokio::fs::read(st.cfg.upload_dir.join(&stored))
        .await
        .unwrap_or_default();
    let candidates = extract_receipt_ref(&data, &name, extra.as_deref());
    let best = candidates.first().cloned().unwrap_or_default();
    Ok(Json(json!({
        "ref": best,
        "candidates": candidates,
    })))
}
