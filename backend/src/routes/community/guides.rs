//! Admin and member workflow guides CRUD, feedback, and ordering endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::CurrentUser;
use crate::error::{bad, not_found, AppResult};
use crate::i18n::ErrorKey;
use crate::perms::{Act, Page};
use crate::state::AppState;
use crate::util::{audit, now, require};

// ---------------- Admin workflow guides ----------------

/// The in-app manual is readable by anyone who can see any staff page and
/// written by help-centre staff only (see CRUD handlers below).
pub(crate) fn any_admin_page(u: &CurrentUser) -> bool {
    crate::perms::CATALOG
        .iter()
        .any(|d| d.page.area() != crate::perms::Area::Member && u.can(d.page, Act::View))
}

#[derive(Deserialize)]
pub(super) struct GuideQuery {
    include_drafts: Option<bool>,
    /// Also return each guide's steps (for in-browser search).
    full: Option<bool>,
}

pub(super) async fn list_guides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<GuideQuery>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let with_drafts = q.include_drafts.unwrap_or(false) && u.can(Page::AdminHelp, Act::Update);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let full = q.full.unwrap_or(false);
    let rows: Vec<(i64, String, String, String, String, String, bool, String, i64, i32, String, i64, i64)> = sqlx::query_as(
        "SELECT g.id, g.slug, g.title_en, g.title_zh, g.summary_en, g.summary_zh, g.published, g.audience,
                jsonb_array_length(g.steps)::bigint, g.sort_order, g.steps::text,
                COUNT(f.user_id) FILTER (WHERE f.helpful)::bigint,
                COUNT(f.user_id) FILTER (WHERE NOT f.helpful)::bigint
         FROM guide_workflows g LEFT JOIN guide_feedback f ON f.guide_id=g.id
         WHERE ($1 OR g.published) AND ($2 OR g.audience='member')
         GROUP BY g.id ORDER BY g.sort_order, g.id",
    )
    .bind(with_drafts)
    .bind(staff)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(Value::Array(
        rows.into_iter()
            .map(|(id, slug, ten, tzh, sen, szh, published, audience, n, sort, steps, yes, no)| {
                let mut v = json!({
                    "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
                    "summary_en": sen, "summary_zh": szh, "published": published, "audience": audience,
                    "step_count": n, "sort_order": sort,
                });
                if full {
                    v["steps"] = serde_json::from_str(&steps).unwrap_or(json!([]));
                }
                if can_edit {
                    v["helpful_yes"] = json!(yes);
                    v["helpful_no"] = json!(no);
                }
                v
            })
            .collect(),
    )))
}

#[derive(Deserialize)]
pub(super) struct FeedbackIn {
    helpful: bool,
}

/// One vote per user per guide; voting again replaces the earlier answer.
pub(super) async fn guide_feedback(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<FeedbackIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let visible: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM guide_workflows WHERE id=$1 AND ($2 OR published) AND ($3 OR audience='member')",
    )
    .bind(id)
    .bind(can_edit)
    .bind(staff)
    .fetch_optional(&st.db)
    .await?;
    visible.ok_or(not_found(u.lang))?;
    sqlx::query(
        "INSERT INTO guide_feedback (guide_id, user_id, helpful, created_at) VALUES ($1,$2,$3,$4)
         ON CONFLICT (guide_id, user_id) DO UPDATE SET helpful=EXCLUDED.helpful, created_at=EXCLUDED.created_at",
    )
    .bind(id)
    .bind(u.id)
    .bind(req.helpful)
    .bind(now())
    .execute(&st.db)
    .await?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
pub(super) struct ReorderIn {
    ids: Vec<i64>,
}

pub(super) async fn reorder_guides(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<ReorderIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    if req.ids.len() > 200 {
        return Err(bad(u.lang, ErrorKey::InvalidAction));
    }
    let mut seen = std::collections::HashSet::new();
    let mut ids = Vec::with_capacity(req.ids.len());
    for &id in &req.ids {
        if seen.insert(id) {
            ids.push(id);
        }
    }
    let mut tx = st.db.begin().await?;
    if !ids.is_empty() {
        let positions: Vec<i32> = (0..ids.len()).map(|i| i as i32).collect();
        sqlx::query(
            "UPDATE guide_workflows g SET sort_order = v.pos
             FROM UNNEST($1::BIGINT[], $2::INT[]) AS v(id, pos)
             WHERE g.id = v.id",
        )
        .bind(&ids)
        .bind(&positions)
        .execute(&mut *tx)
        .await?;
        let tail_start = ids.len() as i32;
        sqlx::query(
            "UPDATE guide_workflows SET sort_order = sort_order + $1
             WHERE NOT (id = ANY($2))",
        )
        .bind(tail_start)
        .bind(&ids)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    audit(
        &st.db,
        Some(u.id),
        "guide.reorder",
        "guide_workflow",
        None,
        "",
    )
    .await;
    Ok(Json(json!({"ok": true})))
}

pub(super) async fn get_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Value>> {
    require(&u, Page::Help, Act::View)?;
    let staff = any_admin_page(&u);
    let can_edit = u.can(Page::AdminHelp, Act::Update);
    let row: Option<(i64, String, String, String, String, String, bool, String, String)> = sqlx::query_as(
        "SELECT id, slug, title_en, title_zh, summary_en, summary_zh, published, audience, steps::text
             FROM guide_workflows WHERE slug=$1 AND ($2 OR published) AND ($3 OR audience='member')",
    )
    .bind(&slug)
    .bind(can_edit)
    .bind(staff)
    .fetch_optional(&st.db)
    .await?;
    let (id, slug, ten, tzh, sen, szh, published, audience, steps) =
        row.ok_or(not_found(u.lang))?;
    let steps: Value = serde_json::from_str(&steps).unwrap_or(json!([]));
    Ok(Json(json!({
        "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
        "summary_en": sen, "summary_zh": szh, "published": published, "audience": audience, "steps": steps,
    })))
}

#[derive(Deserialize)]
struct GuideStepIn {
    id: Option<String>,
    title_en: Option<String>,
    title_zh: Option<String>,
    body_en: Option<String>,
    body_zh: Option<String>,
    link: Option<String>,
    image_file_id: Option<i64>,
    note_kind: Option<String>,
    note_en: Option<String>,
    note_zh: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct GuideIn {
    slug: String,
    title_en: Option<String>,
    title_zh: Option<String>,
    summary_en: Option<String>,
    summary_zh: Option<String>,
    published: Option<bool>,
    #[serde(default = "default_guide_audience")]
    audience: String,
    #[serde(default)]
    steps: Vec<GuideStepIn>,
}

fn default_guide_audience() -> String {
    "staff".into()
}

fn trim_opt(v: &Option<String>) -> &str {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
}

/// Normalise + validate a guide payload; returns columns ready to bind,
/// with steps serialised to JSON text (sqlx has no json feature).
fn validate_guide(
    req: &GuideIn,
    lang: crate::i18n::Lang,
) -> AppResult<(String, String, String, String, String, bool, String, String)> {
    let (title_en, title_zh) = (trim_opt(&req.title_en), trim_opt(&req.title_zh));
    if title_en.is_empty() && title_zh.is_empty() {
        return Err(bad(lang, ErrorKey::GuideTitleRequired));
    }
    let slug = req.slug.trim().to_lowercase();
    let ok_slug = !slug.is_empty()
        && slug != "reorder"
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok_slug {
        return Err(bad(lang, ErrorKey::GuideSlugRequired));
    }
    if !matches!(req.audience.as_str(), "member" | "staff") {
        return Err(bad(lang, ErrorKey::GuideAudienceInvalid));
    }
    if req.steps.len() > 30 {
        return Err(bad(lang, ErrorKey::GuideStepInvalid));
    }
    let mut steps: Vec<Value> = Vec::new();
    for s in &req.steps {
        let (ten, tzh) = (trim_opt(&s.title_en), trim_opt(&s.title_zh));
        if ten.is_empty() && tzh.is_empty() {
            return Err(bad(lang, ErrorKey::GuideStepInvalid));
        }
        let link = s.link.as_deref().map(str::trim).filter(|l| !l.is_empty());
        if let Some(l) = link {
            // Internal app paths only: must start with '/', no protocol-relative, backslashes,
            // control/whitespace chars, traversal, colons, or /api URLs.
            if !l.starts_with('/')
                || l.starts_with("//")
                || l == "/api"
                || l.starts_with("/api/")
                || l.starts_with("/api?")
                || l.contains('\\')
                || l.contains("..")
                || l.contains(':')
                || l.bytes()
                    .any(|b| b <= 0x1F || b == 0x7F || b.is_ascii_whitespace())
            {
                return Err(bad(lang, ErrorKey::GuideLinkInvalid));
            }
        }
        let step_id =
            s.id.as_deref()
                .map(str::trim)
                .filter(|id| {
                    !id.is_empty()
                        && id.len() <= 36
                        && id
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                })
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string()[..12].to_string());
        let note_en = trim_opt(&s.note_en);
        let note_zh = trim_opt(&s.note_zh);
        let note_kind = trim_opt(&s.note_kind).to_lowercase();
        if !note_kind.is_empty()
            && !matches!(note_kind.as_str(), "tip" | "warning" | "important" | "note")
        {
            return Err(bad(lang, ErrorKey::GuideStepInvalid));
        }
        let mut obj = json!({
            "id": step_id,
            "title_en": ten, "title_zh": tzh,
            "body_en": trim_opt(&s.body_en), "body_zh": trim_opt(&s.body_zh),
            "link": link.unwrap_or(""),
        });
        if let Some(fid) = s.image_file_id.filter(|&id| id > 0) {
            obj["image_file_id"] = json!(fid);
        }
        if !note_en.is_empty() || !note_zh.is_empty() {
            obj["note_kind"] = json!(if note_kind.is_empty() {
                "tip"
            } else {
                note_kind.as_str()
            });
            obj["note_en"] = json!(note_en);
            obj["note_zh"] = json!(note_zh);
        }
        steps.push(obj);
    }
    Ok((
        slug.to_string(),
        title_en.to_string(),
        title_zh.to_string(),
        trim_opt(&req.summary_en).to_string(),
        trim_opt(&req.summary_zh).to_string(),
        req.published.unwrap_or(true),
        req.audience.clone(),
        Value::Array(steps).to_string(),
    ))
}

async fn verify_guide_step_files(
    st: &AppState,
    u: &CurrentUser,
    req: &GuideIn,
    existing_guide_id: Option<i64>,
) -> AppResult<()> {
    let fids: Vec<i64> = req
        .steps
        .iter()
        .filter_map(|s| s.image_file_id.filter(|&id| id > 0))
        .collect();
    if fids.is_empty() {
        return Ok(());
    }
    let mut already_attached = std::collections::HashSet::new();
    if let Some(gid) = existing_guide_id {
        if let Some((raw,)) =
            sqlx::query_as::<_, (String,)>("SELECT steps::text FROM guide_workflows WHERE id=$1")
                .bind(gid)
                .fetch_optional(&st.db)
                .await?
        {
            if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(&raw) {
                for item in arr {
                    if let Some(fid) = item.get("image_file_id").and_then(Value::as_i64) {
                        already_attached.insert(fid);
                    }
                }
            }
        }
    }
    for fid in fids {
        if already_attached.contains(&fid) {
            continue;
        }
        let (ok,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM files WHERE id=$1 AND owner_id=$2 AND mime LIKE 'image/%'",
        )
        .bind(fid)
        .bind(u.id)
        .fetch_one(&st.db)
        .await?;
        if ok == 0 {
            return Err(bad(u.lang, ErrorKey::InvalidPhoto));
        }
    }
    Ok(())
}

pub(super) async fn create_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    verify_guide_step_files(&st, &u, &req, None).await?;
    let (slug, ten, tzh, sen, szh, published, audience, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> = sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1")
        .bind(&slug)
        .fetch_optional(&st.db)
        .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let id: i64 = match sqlx::query_scalar(
        "INSERT INTO guide_workflows (slug, title_en, title_zh, summary_en, summary_zh, published, audience, steps, sort_order, updated_at, updated_by)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8::jsonb,COALESCE((SELECT MAX(sort_order)+1 FROM guide_workflows), 0),$9,$10) RETURNING id",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&audience)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
    .fetch_one(&st.db)
    .await
    {
        Ok(id) => id,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::GuideSlugExists));
        }
        Err(e) => return Err(e.into()),
    };
    audit(
        &st.db,
        Some(u.id),
        "guide.create",
        "guide_workflow",
        Some(id),
        &slug,
    )
    .await;
    Ok(Json(json!({ "id": id })))
}

pub(super) async fn update_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<i64>,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    verify_guide_step_files(&st, &u, &req, Some(key)).await?;
    let (slug, ten, tzh, sen, szh, published, audience, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1 AND id<>$2")
            .bind(&slug)
            .bind(key)
            .fetch_optional(&st.db)
            .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let n = match sqlx::query(
        "UPDATE guide_workflows SET slug=$1, title_en=$2, title_zh=$3, summary_en=$4, summary_zh=$5,
                published=$6, audience=$7, steps=$8::jsonb, updated_at=$9, updated_by=$10 WHERE id=$11",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&audience)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
    .bind(key)
    .execute(&st.db)
    .await
    {
        Ok(res) => res.rows_affected(),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(bad(u.lang, ErrorKey::GuideSlugExists));
        }
        Err(e) => return Err(e.into()),
    };
    if n == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "guide.update",
        "guide_workflow",
        Some(key),
        &slug,
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub(super) async fn delete_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<i64>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let n = sqlx::query("DELETE FROM guide_workflows WHERE id=$1")
        .bind(key)
        .execute(&st.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(not_found(u.lang));
    }
    audit(
        &st.db,
        Some(u.id),
        "guide.delete",
        "guide_workflow",
        Some(key),
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
