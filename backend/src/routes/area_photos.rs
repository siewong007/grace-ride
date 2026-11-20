//! Photos of a pickup area, so passengers can recognise where to wait for their ride.
//!
//! Who may do what (`admin_area_photos`, editable on the Roles page):
//! - **view**: open the area-photos manager in Settings. Every active member already
//!   sees an area's photos wherever its pickup area is shown (`GET /areas/{id}/photos`).
//! - **create**: add photos (up to [`MAX_PER_AREA`] per area) and change the caption of,
//!   or remove, photos they added themselves. Traffic team members hold this by default.
//! - **update**: change any caption and reorder an area's photos.
//! - **delete**: remove any photo.
//!
//! The Head of Traffic and everyone who manages areas (admin, President, Vice-President,
//! secretary, transport, super admin) hold all four by default.
//!
//! A photo is an image the adder uploaded through `POST /files` (8 MB cap, sniffed type,
//! daily quota). Only JPEG, PNG and WebP are accepted so every phone can show it. The app
//! shrinks and re-encodes photos before upload, which drops camera metadata; as a fallback
//! the server strips Exif / XMP / IPTC (location included) from JPEGs when attached.

use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, forbidden, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, now, require};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/areas/{id}/photos", get(list_photos).post(add_photo))
        .route("/areas/{id}/photos/order", put(reorder_photos))
        .route(
            "/areas/{id}/photos/{pid}",
            put(update_photo).delete(delete_photo),
        )
}

/// Photos one area may have.
pub const MAX_PER_AREA: i64 = 10;
/// Longest caption, in characters.
pub const CAPTION_MAX: usize = 120;
/// Image types every phone browser can show.
const PHOTO_MIMES: &[&str] = &["image/jpeg", "image/png", "image/webp"];

#[derive(sqlx::FromRow)]
struct PhotoRow {
    id: i64,
    file_id: i64,
    caption: String,
    sort_order: i64,
    created_by: Option<i64>,
    created_by_name: Option<String>,
    created_at: String,
}

/// May change this photo's caption.
fn may_edit(u: &CurrentUser, created_by: Option<i64>) -> bool {
    u.can(Page::AdminAreaPhotos, Act::Update)
        || (u.can(Page::AdminAreaPhotos, Act::Create) && created_by == Some(u.id))
}

/// May remove this photo.
fn may_delete(u: &CurrentUser, created_by: Option<i64>) -> bool {
    u.can(Page::AdminAreaPhotos, Act::Delete)
        || (u.can(Page::AdminAreaPhotos, Act::Create) && created_by == Some(u.id))
}

/// Anyone who books or gives rides, and photo managers.
fn may_view(u: &CurrentUser) -> bool {
    u.can(Page::Rides, Act::View)
        || u.can(Page::Drive, Act::View)
        || u.can(Page::AdminAreaPhotos, Act::View)
}

fn clean_caption(u: &CurrentUser, caption: Option<&str>) -> AppResult<String> {
    let c = caption.unwrap_or_default().trim();
    if c.chars().count() > CAPTION_MAX {
        return Err(bad(u.lang, ErrorKey::AreaPhotoCaptionTooLong));
    }
    Ok(c.to_string())
}

async fn area_name(st: &AppState, u: &CurrentUser, area: i64) -> AppResult<String> {
    sqlx::query_scalar("SELECT name FROM areas WHERE id=$1")
        .bind(area)
        .fetch_optional(&st.db)
        .await?
        .ok_or(not_found(u.lang))
}

async fn list_photos(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(area): Path<i64>,
) -> AppResult<Json<Value>> {
    if !may_view(&u) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let name = area_name(&st, &u, area).await?;
    let rows: Vec<PhotoRow> = sqlx::query_as(
        "SELECT p.id, p.file_id, p.caption, p.sort_order, p.created_by,
                c.name AS created_by_name, p.created_at
         FROM area_photos p LEFT JOIN users c ON c.id = p.created_by
         WHERE p.area_id = $1
         ORDER BY p.sort_order, p.id",
    )
    .bind(area)
    .fetch_all(&st.db)
    .await?;
    // Who added a photo is for the people who look after them, not every rider.
    let manager = u.can(Page::AdminAreaPhotos, Act::View);
    let photos: Vec<Value> = rows
        .into_iter()
        .map(|p| {
            let mut v = json!({
                "id": p.id,
                "file_id": p.file_id,
                "url": format!("/api/files/{}", p.file_id),
                "caption": p.caption,
                "sort_order": p.sort_order,
                "can_edit": may_edit(&u, p.created_by),
                "can_delete": may_delete(&u, p.created_by),
            });
            if manager {
                v["mine"] = json!(p.created_by == Some(u.id));
                v["created_by"] = p
                    .created_by
                    .map(|id| json!({"id": id, "name": p.created_by_name}))
                    .unwrap_or(Value::Null);
                v["created_at"] = json!(p.created_at);
            }
            v
        })
        .collect();
    Ok(Json(json!({
        "area": {"id": area, "name": name},
        "photos": photos,
        "max": MAX_PER_AREA,
        "can_add": u.can(Page::AdminAreaPhotos, Act::Create),
        "can_reorder": u.can(Page::AdminAreaPhotos, Act::Update),
    })))
}

#[derive(Deserialize)]
struct PhotoIn {
    file_id: i64,
    caption: Option<String>,
}

async fn add_photo(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(area): Path<i64>,
    Json(req): Json<PhotoIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAreaPhotos, Act::Create)?;
    let caption = clean_caption(&u, req.caption.as_deref())?;
    let mut tx = st.db.begin().await?;
    // Locking the area serialises adds, so two at once cannot pass the limit together.
    let name: String = sqlx::query_scalar("SELECT name FROM areas WHERE id=$1 FOR UPDATE")
        .bind(area)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(not_found(u.lang))?;
    // Only the adder's own upload, and only an image every phone can show.
    let file: Option<(String, String)> =
        sqlx::query_as("SELECT mime, stored_name FROM files WHERE id=$1 AND owner_id=$2")
            .bind(req.file_id)
            .bind(u.id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((mime, stored)) = file else {
        return Err(bad(u.lang, ErrorKey::InvalidPhoto));
    };
    if !PHOTO_MIMES.contains(&mime.as_str()) {
        return Err(bad(u.lang, ErrorKey::InvalidPhoto));
    }
    let (count, next): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX(sort_order) + 1, 0) FROM area_photos WHERE area_id=$1",
    )
    .bind(area)
    .fetch_one(&mut *tx)
    .await?;
    if count >= MAX_PER_AREA {
        return Err(bad(u.lang, ErrorKey::AreaPhotoLimit));
    }
    let ts = now();
    let inserted = sqlx::query_scalar(
        "INSERT INTO area_photos (area_id, file_id, caption, sort_order, created_by, created_at, updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$6) RETURNING id",
    )
    .bind(area)
    .bind(req.file_id)
    .bind(&caption)
    .bind(next)
    .bind(u.id)
    .bind(&ts)
    .fetch_one(&mut *tx)
    .await;
    let id: i64 = match inserted {
        Ok(id) => id,
        // The same upload is already one of the photos.
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::InvalidPhoto));
        }
        Err(e) => return Err(e.into()),
    };
    tx.commit().await?;
    if mime == "image/jpeg" {
        strip_jpeg_file(&st, &stored).await;
    }
    audit(
        &st.db,
        Some(u.id),
        "area.photo.add",
        "area",
        Some(area),
        format!("{name}: photo {id}{}", caption_note(&caption)),
    )
    .await;
    Ok(Json(json!({"id": id})))
}

fn caption_note(caption: &str) -> String {
    if caption.is_empty() {
        String::new()
    } else {
        format!(" \"{caption}\"")
    }
}

/// The photo, if it belongs to this area.
async fn load_photo(
    st: &AppState,
    u: &CurrentUser,
    area: i64,
    pid: i64,
) -> AppResult<(Option<i64>, i64, String, String)> {
    sqlx::query_as(
        "SELECT p.created_by, p.file_id, p.caption, a.name FROM area_photos p
         JOIN areas a ON a.id = p.area_id WHERE p.id=$1 AND p.area_id=$2",
    )
    .bind(pid)
    .bind(area)
    .fetch_optional(&st.db)
    .await?
    .ok_or(not_found(u.lang))
}

#[derive(Deserialize)]
struct CaptionIn {
    caption: Option<String>,
}

async fn update_photo(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((area, pid)): Path<(i64, i64)>,
    Json(req): Json<CaptionIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAreaPhotos, Act::View)?;
    let (created_by, _, _, name) = load_photo(&st, &u, area, pid).await?;
    if !may_edit(&u, created_by) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let caption = clean_caption(&u, req.caption.as_deref())?;
    sqlx::query("UPDATE area_photos SET caption=$1, updated_at=$2 WHERE id=$3")
        .bind(&caption)
        .bind(now())
        .bind(pid)
        .execute(&st.db)
        .await?;
    audit(
        &st.db,
        Some(u.id),
        "area.photo.update",
        "area",
        Some(area),
        format!("{name}: photo {pid}{}", caption_note(&caption)),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

async fn delete_photo(
    State(st): State<AppState>,
    u: CurrentUser,
    Path((area, pid)): Path<(i64, i64)>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAreaPhotos, Act::View)?;
    let (created_by, file_id, caption, name) = load_photo(&st, &u, area, pid).await?;
    if !may_delete(&u, created_by) {
        return Err(forbidden(u.lang, ErrorKey::NoPermission));
    }
    let mut tx = st.db.begin().await?;
    sqlx::query("DELETE FROM area_photos WHERE id=$1")
        .bind(pid)
        .execute(&mut *tx)
        .await?;
    // The upload goes too, unless something else still shows it.
    let stored: Option<String> = sqlx::query_scalar(
        "DELETE FROM files f WHERE f.id=$1
           AND NOT EXISTS (SELECT 1 FROM users WHERE photo_file_id=f.id)
           AND NOT EXISTS (SELECT 1 FROM events WHERE photo_file_id=f.id)
           AND NOT EXISTS (SELECT 1 FROM help_replies WHERE file_id=f.id)
           AND NOT EXISTS (SELECT 1 FROM settlements WHERE payee_qr_file_id=f.id)
           AND NOT EXISTS (SELECT 1 FROM settlement_items WHERE receipt_file_id=f.id)
           AND NOT EXISTS (SELECT 1 FROM settlement_shares WHERE proof_file_id=f.id)
           AND NOT EXISTS (SELECT 1 FROM reimbursements WHERE receipt_file_id=f.id)
           AND NOT EXISTS (SELECT 1 FROM guide_workflows
                           WHERE steps @> jsonb_build_array(jsonb_build_object('image_file_id', f.id)))
         RETURNING stored_name",
    )
    .bind(file_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    if let Some(stored) = stored {
        let _ = tokio::fs::remove_file(st.cfg.upload_dir.join(stored)).await;
    }
    audit(
        &st.db,
        Some(u.id),
        "area.photo.delete",
        "area",
        Some(area),
        format!("{name}: photo {pid}{}", caption_note(&caption)),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct OrderIn {
    photo_ids: Vec<i64>,
}

async fn reorder_photos(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(area): Path<i64>,
    Json(req): Json<OrderIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminAreaPhotos, Act::Update)?;
    let mut tx = st.db.begin().await?;
    let name: String = sqlx::query_scalar("SELECT name FROM areas WHERE id=$1 FOR UPDATE")
        .bind(area)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(not_found(u.lang))?;
    let mut current: Vec<i64> = sqlx::query_scalar("SELECT id FROM area_photos WHERE area_id=$1")
        .bind(area)
        .fetch_all(&mut *tx)
        .await?;
    // The new order must name every photo of the area exactly once.
    let mut wanted = req.photo_ids.clone();
    wanted.sort_unstable();
    wanted.dedup();
    current.sort_unstable();
    if wanted != current || wanted.len() != req.photo_ids.len() {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let ts = now();
    for (i, pid) in req.photo_ids.iter().enumerate() {
        sqlx::query("UPDATE area_photos SET sort_order=$1, updated_at=$2 WHERE id=$3")
            .bind(i as i64)
            .bind(&ts)
            .bind(pid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "area.photo.reorder",
        "area",
        Some(area),
        format!("{name}: {:?}", req.photo_ids),
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

/// Rewrite a stored JPEG without its metadata segments. Leaves the file as it is when it
/// cannot be parsed or has nothing to strip.
async fn strip_jpeg_file(st: &AppState, stored: &str) {
    let path = st.cfg.upload_dir.join(stored);
    let Ok(data) = tokio::fs::read(&path).await else {
        return;
    };
    if let Some(clean) = strip_jpeg_metadata(&data) {
        if let Err(e) = tokio::fs::write(&path, &clean).await {
            tracing::warn!("could not strip photo metadata: {e}");
        } else {
            let _ = sqlx::query("UPDATE files SET size=$1 WHERE stored_name=$2")
                .bind(clean.len() as i64)
                .bind(stored)
                .execute(&st.db)
                .await;
        }
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
