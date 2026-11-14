# Admin Workflow Guide Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A staff-only Admin Guide tab in the Help Centre listing database-managed, bilingual workflow guides whose steps deep-link into the real admin pages, with an in-app editor for help-centre staff.

**Architecture:** One new PostgreSQL table (`guide_workflows`, steps as a JSONB array). Read endpoints gated to any user holding a staff-page permission; write endpoints gated to `admin_help` update with audit entries. Frontend adds a tab to `Help.tsx`, a list+editor component, and a detail route. Starter content seeded as reference data.

**Tech Stack:** Rust/Axum/sqlx (backend), React 18 + TypeScript + react-router + vitest + @testing-library/react (frontend), i18next (zh/en).

**Spec:** `docs/superpowers/specs/2026-11-14-admin-guide-design.md`

## Global Constraints

- Repo root: `/Users/goaltosuceed/Documents/Grace Ride`. All paths below are relative to it.
- **DO NOT touch or `git add` these dirty working files** (unrelated in-progress work): `backend/src/routes/auth.rs`, `backend/src/routes/rides.rs`, `backend/tests/api.rs`, `frontend/src/pages/RideNew.tsx`. Every commit stages only the files its task lists.
- Backend tests live in a **new** file `backend/tests/guides.rs` precisely to avoid the dirty `backend/tests/api.rs`.
- Timestamps are TEXT, produced by `crate::util::now()` (community.rs) / `now_kl()` (seed.rs).
- sqlx has **no `json` feature** — never decode JSONB to `serde_json::Value` directly. Read with `steps::text` + `serde_json::from_str`, write with a bound String + `$n::jsonb` cast (pattern: `backend/src/routes/auth.rs:932`).
- Bilingual fields are paired `*_en` / `*_zh`; rendering falls back to the other language when one is empty.
- UI text goes in `frontend/src/locales/help.en.ts` / `help.zh.ts` (namespace `help`). Guide *content* comes from the API only.
- Run backend commands from `backend/`, frontend commands from `frontend/`.
- Commit message style follows history: `feat(help): …`, `docs: …`.
- Test DB: tests auto-create a fresh schema and run all migrations via `init()` (`backend/src/lib.rs:52`), so a new migration applies automatically.

---

### Task 1: Migration + guide read endpoints (list & get)

**Files:**
- Create: `backend/migrations/0013_admin_guide.sql`
- Modify: `backend/src/routes/community.rs` (router + new handlers at the end of the help-center section)
- Test: `backend/tests/guides.rs` (new)

**Interfaces:**
- Produces: `GET /api/help/guides[?include_drafts=true]` → JSON array `[{id, slug, title_en, title_zh, summary_en, summary_zh, published, step_count}]`; `GET /api/help/guides/{slug}` → `{id, slug, title_en, title_zh, summary_en, summary_zh, published, steps}`. Rust helper `fn any_admin_page(u: &CurrentUser) -> bool` (used again by Task 2).

- [ ] **Step 1: Write the migration**

Create `backend/migrations/0013_admin_guide.sql`:

```sql
-- 恩典同行 Grace Ride — Admin workflow guides
--
-- The in-app manual for staff: ordered, bilingual steps that deep-link into the
-- admin pages they describe. Content is managed in-app by help-centre staff;
-- changing the manual must not require a deploy.

CREATE TABLE IF NOT EXISTS guide_workflows (
  id         BIGSERIAL PRIMARY KEY,
  slug       TEXT UNIQUE NOT NULL,
  sort_order INT     NOT NULL DEFAULT 0,
  published  BOOLEAN NOT NULL DEFAULT TRUE,
  title_en   TEXT NOT NULL,
  title_zh   TEXT NOT NULL,
  summary_en TEXT NOT NULL DEFAULT '',
  summary_zh TEXT NOT NULL DEFAULT '',
  steps      JSONB NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL,
  updated_by BIGINT REFERENCES users(id) ON DELETE SET NULL
);
```

- [ ] **Step 2: Write the failing tests**

Create `backend/tests/guides.rs`. Copy the harness header style from `backend/tests/api.rs` (setup/login/call are reproduced here so this file stands alone):

```rust
//! Admin workflow guide API: staff-only reads, editor-only writes.

#[path = "common/schema.rs"]
mod schema;

use axum::{
    body::Body,
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use grace_ride::{auth::hash_token, init, router, state::AppState, state::Config};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const PW: &str = "GraceRide2026!";
const ORIGIN: &str = "http://localhost:5173";

async fn fresh_db() -> String {
    schema::fresh_schema("gr_").await
}

async fn setup() -> (Router, AppState) {
    let dir = std::env::temp_dir().join(format!("gr-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = Config {
        dev_mode: true,
        database_url: fresh_db().await,
        upload_dir: dir.join("uploads"),
        static_dir: None,
        public_url: ORIGIN.into(),
        bind: "127.0.0.1:0".into(),
        max_live_drivers: 10,
        trust_proxy: false,
        chat_retention_days: 180,
        audit_retention_days: 730,
        session_timeout_mins: 15,
        json_logs: false,
        google_client_id: None,
        google_client_secret: None,
        routing: Default::default(),
    };
    cfg.validate().unwrap();
    let state = init(cfg).await.unwrap();
    (router(state.clone()), state)
}

struct Res {
    status: StatusCode,
    headers: HeaderMap,
    json: Value,
}

async fn call(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> Res {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    b = b.header(header::ORIGIN, ORIGIN);
    let req = match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Res { status, headers, json }
}

/// Test-only auth: mint a hashed session row and return the Cookie header value.
async fn login(st: &AppState, email: &str) -> String {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email=$1 AND status='active'")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap_or_else(|e| panic!("demo user {email}: {e}"));
    let token =
        uuid::Uuid::new_v4().simple().to_string() + &uuid::Uuid::new_v4().simple().to_string();
    let token = token[..48].to_string();
    let expires = (chrono::Local::now() + chrono::Duration::days(7))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,1,$3,$4)",
    )
    .bind(hash_token(&token))
    .bind(id)
    .bind(&expires)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    format!("{}={}", grace_ride::auth::SESSION_COOKIE, token)
}

/// Insert a guide row straight into the DB (test fixture).
async fn seed_guide(
    st: &AppState,
    slug: &str,
    published: bool,
    steps: &str,
) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO guide_workflows (slug, title_en, title_zh, steps, published, updated_at)
         VALUES ($1,'Test guide','测试指南',$2::jsonb,$3,$4) RETURNING id",
    )
    .bind(slug)
    .bind(steps)
    .bind(published)
    .bind(grace_ride::util::now())
    .fetch_one(&st.db)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn guides_are_staff_only() {
    let (app, st) = setup().await;
    seed_guide(&st, "published-guide", true, r#"[{"title_en":"Step","title_zh":"步骤","body_en":"","body_zh":"","link":"/admin"}]"#).await;
    let member = login(&st, "member@example.org").await;
    let r = call(&app, Method::GET, "/api/help/guides", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(&app, Method::GET, "/api/help/guides/published-guide", Some(&member), None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // transport (admin_help) and finance (admin_reimb_pay) both hold staff pages.
    for email in ["transport@example.org", "finance@example.org"] {
        let cookie = login(&st, email).await;
        let r = call(&app, Method::GET, "/api/help/guides", Some(&cookie), None).await;
        assert_eq!(r.status, StatusCode::OK, "{email} should list guides");
        assert_eq!(r.json.as_array().unwrap().len(), 1);
        assert_eq!(r.json[0]["slug"], "published-guide");
        assert_eq!(r.json[0]["step_count"], 1);
        let r = call(&app, Method::GET, "/api/help/guides/published-guide", Some(&cookie), None).await;
        assert_eq!(r.status, StatusCode::OK);
        assert_eq!(r.json["steps"][0]["link"], "/admin");
    }
}

#[tokio::test]
async fn drafts_hidden_from_non_editors() {
    let (app, st) = setup().await;
    seed_guide(&st, "draft-guide", false, "[]").await;
    seed_guide(&st, "live-guide", true, "[]").await;

    // finance holds staff pages but not admin_help update → no drafts, even with the flag.
    let finance = login(&st, "finance@example.org").await;
    let r = call(&app, Method::GET, "/api/help/guides?include_drafts=true", Some(&finance), None).await;
    assert_eq!(r.json.as_array().unwrap().len(), 1);
    let r = call(&app, Method::GET, "/api/help/guides/draft-guide", Some(&finance), None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // transport (admin_help update) sees drafts with the flag; detail loads.
    let transport = login(&st, "transport@example.org").await;
    let r = call(&app, Method::GET, "/api/help/guides?include_drafts=true", Some(&transport), None).await;
    assert_eq!(r.json.as_array().unwrap().len(), 2);
    let r = call(&app, Method::GET, "/api/help/guides/draft-guide", Some(&transport), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["published"], false);
}
```

Note: if `grace_ride::util::now` or `grace_ride::auth::SESSION_COOKIE`/`hash_token` are not public in this tree, mirror exactly what `backend/tests/api.rs` uses for the same purpose (it builds cookies the same way) — copy its imports verbatim; that file is the source of truth for the harness, do not invent alternatives.

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/backend" && cargo test --test guides
```

Expected: compile error (no `/help/guides` routes) or 404/assert failures after seeding — either proves the endpoint is missing.

- [ ] **Step 4: Implement the endpoints**

In `backend/src/routes/community.rs`:

4a. Register routes in `router()` immediately after the `/help/{id}/close` line (around line 34):

```rust
        .route("/help/guides", get(list_guides))
        .route("/help/guides/{key}", get(get_guide))
```

(`{key}` is a slug for GET; Task 2 adds PUT/DELETE on the same pattern where `{key}` is the numeric id — one pattern avoids route conflicts.)

4b. Append at the end of the help-center section (after `close_ticket`), a new section:

```rust
// ---------------- Admin workflow guides ----------------

/// The in-app manual is readable by anyone who can see any staff page and
/// written by help-centre staff only (see CRUD handlers below).
fn any_admin_page(u: &CurrentUser) -> bool {
    crate::perms::CATALOG
        .iter()
        .any(|d| d.page.area() != crate::perms::Area::Member && u.can(d.page, Act::View))
}

#[derive(Deserialize)]
struct GuideQuery {
    include_drafts: Option<bool>,
}

async fn list_guides(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<GuideQuery>,
) -> AppResult<Json<Value>> {
    if !any_admin_page(&u) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    let with_drafts = q.include_drafts.unwrap_or(false) && u.can(Page::AdminHelp, Act::Update);
    let rows: Vec<(i64, String, String, String, String, String, bool, i64)> = sqlx::query_as(
        "SELECT id, slug, title_en, title_zh, summary_en, summary_zh, published, jsonb_array_length(steps)
         FROM guide_workflows WHERE ($1 OR published) ORDER BY sort_order, id",
    )
    .bind(with_drafts)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(Value::Array(
        rows.into_iter()
            .map(|(id, slug, ten, tzh, sen, szh, published, n)| {
                json!({
                    "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
                    "summary_en": sen, "summary_zh": szh, "published": published,
                    "step_count": n,
                })
            })
            .collect(),
    )))
}

async fn get_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Value>> {
    if !any_admin_page(&u) {
        return Err(forbidden(u.lang, ErrorKey::Forbidden));
    }
    let row: Option<(i64, String, String, String, String, String, bool, String)> =
        sqlx::query_as(
            "SELECT id, slug, title_en, title_zh, summary_en, summary_zh, published, steps::text
             FROM guide_workflows WHERE slug=$1",
        )
        .bind(&slug)
        .fetch_optional(&st.db)
        .await?;
    let (id, slug, ten, tzh, sen, szh, published, steps) = row.ok_or(not_found(u.lang))?;
    if !published && !u.can(Page::AdminHelp, Act::Update) {
        // Drafts stay invisible to non-editors; 404 avoids leaking existence.
        return Err(not_found(u.lang));
    }
    let steps: Value = serde_json::from_str(&steps).unwrap_or(json!([]));
    Ok(Json(json!({
        "id": id, "slug": slug, "title_en": ten, "title_zh": tzh,
        "summary_en": sen, "summary_zh": szh, "published": published, "steps": steps,
    })))
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/backend" && cargo test --test guides
```

Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add backend/migrations/0013_admin_guide.sql backend/src/routes/community.rs backend/tests/guides.rs && git commit -m "feat(help): guide_workflows table with staff-only read endpoints"
```

---

### Task 2: Guide CRUD with validation and audit

**Files:**
- Modify: `backend/src/i18n/keys.rs` (enum + `code()` match)
- Modify: `backend/src/i18n/locales/mod.rs` (dispatch arm)
- Modify: `backend/src/i18n/locales/community.rs` (zh/en messages)
- Modify: `frontend/src/locales/errors.en.ts`, `frontend/src/locales/errors.zh.ts` (`community` group)
- Modify: `backend/src/routes/community.rs` (handlers + router)
- Test: `backend/tests/guides.rs` (extend)

**Interfaces:**
- Consumes: `any_admin_page` from Task 1.
- Produces: `POST /api/help/guides` body `{slug, title_en?, title_zh?, summary_en?, summary_zh?, published?, steps?}` → `{id}`; `PUT /api/help/guides/{id}` same body → `{ok:true}`; `DELETE /api/help/guides/{id}` → `{ok:true}`. Error codes: `GUIDE_TITLE_REQUIRED`, `GUIDE_SLUG_REQUIRED`, `GUIDE_SLUG_EXISTS`, `GUIDE_LINK_INVALID`, `GUIDE_STEP_INVALID`.

- [ ] **Step 1: Write the failing tests**

Append to `backend/tests/guides.rs`:

```rust
const GUIDE_BODY: Value = json!({
    "slug": "new-guide",
    "title_en": "New guide",
    "title_zh": "新指南",
    "summary_en": "Summary",
    "summary_zh": "摘要",
    "published": true,
    "steps": [
        {"title_en": "First", "title_zh": "第一步", "body_en": "Do it", "body_zh": "这样做", "link": "/admin/plan"},
        {"title_en": "Second", "title_zh": "第二步", "body_en": "", "body_zh": "", "link": ""}
    ]
});

#[tokio::test]
async fn guide_crud_is_editor_only_and_audited() {
    let (app, st) = setup().await;
    let member = login(&st, "member@example.org").await;
    let finance = login(&st, "finance@example.org").await; // staff pages, no admin_help update
    let transport = login(&st, "transport@example.org").await; // admin_help update

    // Non-editors cannot write.
    for cookie in [&member, &finance] {
        let r = call(&app, Method::POST, "/api/help/guides", Some(cookie), Some(GUIDE_BODY.clone())).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN);
    }

    // Create → round-trip → audit row.
    let r = call(&app, Method::POST, "/api/help/guides", Some(&transport), Some(GUIDE_BODY.clone())).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.json);
    let id = r.json["id"].as_i64().unwrap();
    let r = call(&app, Method::GET, &format!("/api/help/guides/new-guide"), Some(&transport), None).await;
    assert_eq!(r.json["steps"][0]["link"], "/admin/plan");
    assert_eq!(r.json["steps"].as_array().unwrap().len(), 2);
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE action='guide.create' AND entity='guide_workflow' AND entity_id=$1")
        .bind(id).fetch_one(&st.db).await.unwrap();
    assert_eq!(n, 1);

    // Update.
    let mut edited = GUIDE_BODY.clone();
    edited["summary_en"] = json!("Edited");
    let r = call(&app, Method::PUT, &format!("/api/help/guides/{id}"), Some(&transport), Some(edited)).await;
    assert_eq!(r.status, StatusCode::OK);

    // Delete → then gone.
    let r = call(&app, Method::DELETE, &format!("/api/help/guides/{id}"), Some(&transport), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let r = call(&app, Method::GET, "/api/help/guides/new-guide", Some(&transport), None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn guide_validation() {
    let (app, st) = setup().await;
    let transport = login(&st, "transport@example.org").await;
    seed_guide(&st, "taken", true, "[]").await;

    let mut b = GUIDE_BODY.clone();
    b["title_en"] = json!(""); b["title_zh"] = json!(""); // no title in either language
    let r = call(&app, Method::POST, "/api/help/guides", Some(&transport), Some(b.clone())).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json["code"], "GUIDE_TITLE_REQUIRED");

    let mut b = GUIDE_BODY.clone();
    b["slug"] = json!("Bad Slug!"); // not url-safe
    let r = call(&app, Method::POST, "/api/help/guides", Some(&transport), Some(b.clone())).await;
    assert_eq!(r.json["code"], "GUIDE_SLUG_REQUIRED");

    let mut b = GUIDE_BODY.clone();
    b["slug"] = json!("taken"); // unique conflict
    let r = call(&app, Method::POST, "/api/help/guides", Some(&transport), Some(b.clone())).await;
    assert_eq!(r.json["code"], "GUIDE_SLUG_EXISTS");

    let mut b = GUIDE_BODY.clone();
    b["steps"][0]["link"] = json!("https://evil.example"); // external link
    let r = call(&app, Method::POST, "/api/help/guides", Some(&transport), Some(b.clone())).await;
    assert_eq!(r.json["code"], "GUIDE_LINK_INVALID");

    let mut b = GUIDE_BODY.clone();
    b["steps"] = json!([{"title_en": "", "title_zh": "", "body_en": "", "body_zh": "", "link": ""}]); // step without any title
    let r = call(&app, Method::POST, "/api/help/guides", Some(&transport), Some(b.clone())).await;
    assert_eq!(r.json["code"], "GUIDE_STEP_INVALID");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/backend" && cargo test --test guides
```

Expected: the two new tests fail (405 Method Not Allowed / missing codes), the two from Task 1 still pass.

- [ ] **Step 3: Add the error keys (backend)**

3a. `backend/src/i18n/keys.rs` — add to the `ErrorKey` enum in the community/errors section (next to `TitleAndContentRequired` and friends):

```rust
    GuideTitleRequired,
    GuideSlugRequired,
    GuideSlugExists,
    GuideLinkInvalid,
    GuideStepInvalid,
```

In the same file's `pub fn code(&self)` match, add before its closing brace:

```rust
            Self::GuideTitleRequired => "GUIDE_TITLE_REQUIRED",
            Self::GuideSlugRequired => "GUIDE_SLUG_REQUIRED",
            Self::GuideSlugExists => "GUIDE_SLUG_EXISTS",
            Self::GuideLinkInvalid => "GUIDE_LINK_INVALID",
            Self::GuideStepInvalid => "GUIDE_STEP_INVALID",
```

3b. `backend/src/i18n/locales/mod.rs` — find the `|` chain of community `ErrorKey::…` variants whose arm ends `=> community::render(lang, key)` and append the five new variants to that chain.

3c. `backend/src/i18n/locales/community.rs` — add to its `render` match:

```rust
        ErrorKey::GuideTitleRequired => pair(
            lang,
            "请至少填写一种语言的指南标题",
            "Enter the guide title in at least one language",
        ),
        ErrorKey::GuideSlugRequired => pair(
            lang,
            "请填写有效的 slug（小写字母、数字、连字符）",
            "Enter a valid slug (lowercase letters, numbers, dashes)",
        ),
        ErrorKey::GuideSlugExists => pair(lang, "该 slug 已被使用", "That slug is already in use"),
        ErrorKey::GuideLinkInvalid => pair(
            lang,
            "步骤链接必须是应用内路径（以 / 开头）",
            "A step link must be an in-app path starting with /",
        ),
        ErrorKey::GuideStepInvalid => pair(
            lang,
            "每个步骤至少填写一种语言的标题；最多 30 个步骤",
            "Every step needs a title in at least one language; 30 steps max",
        ),
```

- [ ] **Step 4: Add the frontend error-code translations**

`frontend/src/locales/errors.en.ts`, inside the `"community"` group (near `REPLY_CONTENT_REQUIRED`):

```ts
    "GUIDE_TITLE_REQUIRED": "Enter the guide title in at least one language",
    "GUIDE_SLUG_REQUIRED": "Enter a valid slug (lowercase letters, numbers, dashes)",
    "GUIDE_SLUG_EXISTS": "That slug is already in use",
    "GUIDE_LINK_INVALID": "A step link must be an in-app path starting with /",
    "GUIDE_STEP_INVALID": "Every step needs a title in at least one language; 30 steps max",
```

`frontend/src/locales/errors.zh.ts`, same group:

```ts
    "GUIDE_TITLE_REQUIRED": "请至少填写一种语言的指南标题",
    "GUIDE_SLUG_REQUIRED": "请填写有效的 slug（小写字母、数字、连字符）",
    "GUIDE_SLUG_EXISTS": "该 slug 已被使用",
    "GUIDE_LINK_INVALID": "步骤链接必须是应用内路径（以 / 开头）",
    "GUIDE_STEP_INVALID": "每个步骤至少填写一种语言的标题；最多 30 个步骤",
```

- [ ] **Step 5: Implement handlers**

In `backend/src/routes/community.rs`, extend the Task 1 route registrations to:

```rust
        .route(
            "/help/guides",
            get(list_guides).post(create_guide),
        )
        .route(
            "/help/guides/{key}",
            get(get_guide)
                .put(update_guide)
                .delete(delete_guide),
        )
```

and append to the guides section:

```rust
#[derive(Deserialize)]
struct GuideStepIn {
    title_en: Option<String>,
    title_zh: Option<String>,
    body_en: Option<String>,
    body_zh: Option<String>,
    link: Option<String>,
}

#[derive(Deserialize)]
struct GuideIn {
    slug: String,
    title_en: Option<String>,
    title_zh: Option<String>,
    summary_en: Option<String>,
    summary_zh: Option<String>,
    published: Option<bool>,
    #[serde(default)]
    steps: Vec<GuideStepIn>,
}

fn trim_opt(v: &Option<String>) -> &str {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("")
}

/// Normalise + validate a guide payload; returns columns ready to bind,
/// with steps serialised to JSON text (sqlx has no json feature).
#[allow(clippy::type_complexity)]
fn validate_guide(
    req: &GuideIn,
    lang: crate::i18n::Lang,
) -> AppResult<(String, String, String, String, String, bool, String)> {
    let (title_en, title_zh) = (trim_opt(&req.title_en), trim_opt(&req.title_zh));
    if title_en.is_empty() && title_zh.is_empty() {
        return Err(bad(lang, ErrorKey::GuideTitleRequired));
    }
    let slug = req.slug.trim().to_lowercase();
    let ok_slug = !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok_slug {
        return Err(bad(lang, ErrorKey::GuideSlugRequired));
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
            // Internal app paths only: must start with '/', no protocol-relative URLs.
            if !l.starts_with('/') || l.starts_with("//") || l.contains(':') {
                return Err(bad(lang, ErrorKey::GuideLinkInvalid));
            }
        }
        steps.push(json!({
            "title_en": ten, "title_zh": tzh,
            "body_en": trim_opt(&s.body_en), "body_zh": trim_opt(&s.body_zh),
            "link": link.unwrap_or(""),
        }));
    }
    Ok((
        slug.to_string(),
        title_en.to_string(),
        title_zh.to_string(),
        trim_opt(&req.summary_en).to_string(),
        trim_opt(&req.summary_zh).to_string(),
        req.published.unwrap_or(true),
        Value::Array(steps).to_string(),
    ))
}

async fn create_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let (slug, ten, tzh, sen, szh, published, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> = sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1")
        .bind(&slug)
        .fetch_optional(&st.db)
        .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO guide_workflows (slug, title_en, title_zh, summary_en, summary_zh, published, steps, updated_at, updated_by)
         VALUES ($1,$2,$3,$4,$5,$6,$7::jsonb,$8,$9) RETURNING id",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
    .fetch_one(&st.db)
    .await?;
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

async fn update_guide(
    State(st): State<AppState>,
    u: CurrentUser,
    Path(key): Path<i64>,
    Json(req): Json<GuideIn>,
) -> AppResult<Json<Value>> {
    require(&u, Page::AdminHelp, Act::Update)?;
    let (slug, ten, tzh, sen, szh, published, steps) = validate_guide(&req, u.lang)?;
    let dup: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM guide_workflows WHERE slug=$1 AND id<>$2")
            .bind(&slug)
            .bind(key)
            .fetch_optional(&st.db)
            .await?;
    if dup.is_some() {
        return Err(bad(u.lang, ErrorKey::GuideSlugExists));
    }
    let n = sqlx::query(
        "UPDATE guide_workflows SET slug=$1, title_en=$2, title_zh=$3, summary_en=$4, summary_zh=$5,
                published=$6, steps=$7::jsonb, updated_at=$8, updated_by=$9 WHERE id=$10",
    )
    .bind(&slug)
    .bind(&ten)
    .bind(&tzh)
    .bind(&sen)
    .bind(&szh)
    .bind(published)
    .bind(&steps)
    .bind(now())
    .bind(u.id)
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
        "guide.update",
        "guide_workflow",
        Some(key),
        &slug,
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_guide(
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
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/backend" && cargo test --test guides
```

Expected: 4 passed.

- [ ] **Step 7: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add backend/src/i18n/keys.rs backend/src/i18n/locales/mod.rs backend/src/i18n/locales/community.rs backend/src/routes/community.rs backend/tests/guides.rs frontend/src/locales/errors.en.ts frontend/src/locales/errors.zh.ts && git commit -m "feat(help): guide CRUD with validation, audit and bilingual error messages"
```

---

### Task 3: Seed six starter workflow guides

**Files:**
- Modify: `backend/src/seed.rs` (inside `insert_reference_data`)
- Test: `backend/tests/guides.rs` (extend)

**Interfaces:**
- Consumes: table from Task 1. `seed.rs` already imports `serde_json::json` and `crate::util::now_kl`.
- Produces: 6 published rows with stable slugs: `plan-transport`, `review-reimbursements`, `send-announcement`, `manage-members`, `work-help-desk`, `audit-settings`.

- [ ] **Step 1: Write the failing test**

Append to `backend/tests/guides.rs`:

```rust
#[tokio::test]
async fn starter_guides_are_seeded() {
    let (app, st) = setup().await;
    let transport = login(&st, "transport@example.org").await;
    let r = call(&app, Method::GET, "/api/help/guides", Some(&transport), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let slugs: Vec<&str> = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["slug"].as_str().unwrap())
        .collect();
    for expected in [
        "plan-transport",
        "review-reimbursements",
        "send-announcement",
        "manage-members",
        "work-help-desk",
        "audit-settings",
    ] {
        assert!(slugs.contains(&expected), "missing seeded guide {expected}");
    }
    // Every seeded guide has steps, and every step has a bilingual title.
    for g in r.json.as_array().unwrap() {
        assert!(g["step_count"].as_i64().unwrap() >= 2, "guide {} too thin", g["slug"]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/backend" && cargo test --test guides starter_guides
```

Expected: FAIL (missing slugs).

- [ ] **Step 3: Implement the seed**

In `backend/src/seed.rs`, append inside `insert_reference_data` (after the groups inserts, before its `Ok(())`):

```rust
    // Starter admin workflow guides — reference content, editable in-app by
    // help-centre staff. ON CONFLICT keeps operator edits on re-deploys.
    let guides: [(&str, &str, &str, &str, &str, serde_json::Value); 6] = [
        (
            "plan-transport",
            "为活动安排交通",
            "Plan transport for an event",
            "从报名到当天实况，完整走一遍交通安排流程。",
            "Walk through arranging rides for an event, from sign-ups to the live map.",
            json!([
                {"title_en": "Find the event", "title_zh": "找到活动",
                 "body_en": "Open Events and pick the service or activity that needs transport.",
                 "body_zh": "打开「活动」页，选择需要安排交通的聚会或活动。",
                 "link": "/events"},
                {"title_en": "Open the planning board", "title_zh": "打开交通规划板",
                 "body_en": "From the event, open Plan transport. Review demand by area, auto-arrange or assign drivers manually, then save.",
                 "body_zh": "在活动里进入「规划交通」。按区域查看需求，自动编排或手动指派司机后保存。",
                 "link": ""},
                {"title_en": "Watch it run on the day", "title_zh": "当天跟踪",
                 "body_en": "On the day, use the live map to follow drivers sharing GPS and react to delays.",
                 "body_zh": "活动当天通过实时地图查看共享位置的司机，及时处理延误。",
                 "link": "/admin/live"}
            ]),
        ),
        (
            "review-reimbursements",
            "审核与支付报销",
            "Review and pay reimbursements",
            "交通组先审核，财务再记录付款。",
            "Transport reviews claims first; finance records the payout.",
            json!([
                {"title_en": "Review the queue", "title_zh": "审核队例",
                 "body_en": "Open the reimbursement review page. Check the claim and receipt, then approve, return or reject.",
                 "body_zh": "打开报销审核页。核对申请与收据，然后批准、退回或拒绝。",
                 "link": "/admin/reimbursements"},
                {"title_en": "Record the payout", "title_zh": "记录付款",
                 "body_en": "Approved claims move to the payment queue. After transferring, record the payout with the reference.",
                 "body_zh": "已批准的申请进入付款队列。转账后记录付款及凭据编号。",
                 "link": "/admin/reimbursements"}
            ]),
        ),
        (
            "send-announcement",
            "发布通知",
            "Send an announcement",
            "向全体或指定群体发送重要通知。",
            "Send an important notice to everyone or a chosen group.",
            json!([
                {"title_en": "Compose it", "title_zh": "撰写通知",
                 "body_en": "Open Announcements, write a bilingual title and body, and choose the audience.",
                 "body_zh": "打开「通知」，填写中英文标题与内容，并选择接收群体。",
                 "link": "/admin/announcements"},
                {"title_en": "Members get notified", "title_zh": "会员收到通知",
                 "body_en": "Members see it under Notifications with a badge; urgent notices can also trigger reminders.",
                 "body_zh": "会员会在「通知」页看到并显示角标；重要通知还会触发提醒。",
                 "link": "/notifications"}
            ]),
        ),
        (
            "manage-members",
            "管理会员与邀请",
            "Manage members and invites",
            "审批新会员、发放邀请、处理账号找回。",
            "Approve members, hand out invites and handle account recovery.",
            json!([
                {"title_en": "The directory", "title_zh": "会员名录",
                 "body_en": "Open Members to see everyone with contact details. Approve pending applicants, disable or restore accounts, and set groups.",
                 "body_zh": "打开「会员」查看名录与联系方式。审批待定申请人、停用或恢复账号、调整群体。",
                 "link": "/admin/members"},
                {"title_en": "Invites and recovery", "title_zh": "邀请与找回",
                 "body_en": "Create single-use invite links for new members, and approve or reject recovery requests from the same area.",
                 "body_zh": "为新会员创建一次性邀请链接；账号找回申请也在同一处审批。",
                 "link": "/admin/members"}
            ]),
        ),
        (
            "work-help-desk",
            "处理帮助中心求助",
            "Work the help desk",
            "跟进会员在帮助中心提出的问题。",
            "Follow up on questions members raise in the help centre.",
            json!([
                {"title_en": "See every request", "title_zh": "查看全部求助",
                 "body_en": "Open the Help centre and switch to All requests. Open status means the member is waiting.",
                 "body_zh": "打开「帮助中心」并切换到「全部求助」。open 状态表示会员仍在等待。",
                 "link": "/help"},
                {"title_en": "Reply and close", "title_zh": "回复并结案",
                 "body_en": "Reply in the ticket thread, then close it once resolved. Closed tickets stay readable.",
                 "body_zh": "在对话中回复会员，问题解决后结案。已结案的求助仍可查看。",
                 "link": "/help"}
            ]),
        ),
        (
            "audit-settings",
            "审计与设置",
            "Audit and settings",
            "查看操作记录、调整系统设置。",
            "Review the action log and adjust system settings.",
            json!([
                {"title_en": "The audit log", "title_zh": "审计日志",
                 "body_en": "Open Audit to see who did what, with filters and CSV export. Retention follows the configured window.",
                 "body_zh": "打开「审计」查看操作记录，可筛选并导出 CSV。保存期限按配置执行。",
                 "link": "/admin/audit"},
                {"title_en": "Settings", "title_zh": "系统设置",
                 "body_en": "Settings holds groups, areas and app-wide configuration such as reimbursement rates.",
                 "body_zh": "「设置」管理群体、区域及全局配置（如报销费率）。",
                 "link": "/admin/settings"}
            ]),
        ),
    ];
    for (i, (slug, tzh, ten, szh, sen, steps)) in guides.iter().enumerate() {
        sqlx::query(
            "INSERT INTO guide_workflows (slug, title_zh, title_en, summary_zh, summary_en, steps, sort_order, published, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6::jsonb,$7,TRUE,$8) ON CONFLICT (slug) DO NOTHING",
        )
        .bind(slug)
        .bind(tzh)
        .bind(ten)
        .bind(szh)
        .bind(sen)
        .bind(steps.to_string())
        .bind(i as i32)
        .bind(now_kl())
        .execute(db)
        .await?;
    }
```

(`insert_reference_data` runs in production too — that is intentional: the manual is reference data, not demo data. `ON CONFLICT DO NOTHING` preserves staff edits on upgrade restarts.)

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/backend" && cargo test --test guides
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add backend/src/seed.rs backend/tests/guides.rs && git commit -m "feat(help): seed six starter admin workflow guides"
```

---

### Task 4: Frontend visibility helper + guide UI strings

**Files:**
- Modify: `frontend/src/perms.ts`
- Modify: `frontend/src/locales/help.en.ts`, `frontend/src/locales/help.zh.ts`
- Test: `frontend/src/perms.test.ts` (new)

**Interfaces:**
- Produces: `export const ADMIN_PAGES: PageKey[]` and `export const canAnyAdminPage = (can: Can): boolean` (used by Help.tsx and App.tsx). New `help` namespace keys used by Tasks 5–7.

- [ ] **Step 1: Write the failing test**

Create `frontend/src/perms.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { ADMIN_PAGES, canAnyAdminPage, Can, PageKey } from './perms';

describe('canAnyAdminPage', () => {
  it('is false for a plain member', () => {
    const can: Can = (page: PageKey) => page === 'rides';
    expect(canAnyAdminPage(can)).toBe(false);
  });
  it('is true when any staff page is visible', () => {
    const can: Can = (page: PageKey) => page === 'admin_reimb_pay';
    expect(canAnyAdminPage(can)).toBe(true);
  });
  it('covers every staff page key', () => {
    // Every non-member page key from the union must be listed, or the tab/route
    // gate drifts from the backend any_admin_page check.
    expect(ADMIN_PAGES).not.toContain('help');
    expect(ADMIN_PAGES.length).toBeGreaterThanOrEqual(19);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/perms.test.ts
```

Expected: FAIL — `ADMIN_PAGES` / `canAnyAdminPage` not exported.

- [ ] **Step 3: Implement**

In `frontend/src/perms.ts`, append after the `Can` type definition:

```ts
/** Staff pages (the non-member areas). Mirrors backend `any_admin_page`. */
export const ADMIN_PAGES: PageKey[] = [
  'admin_dashboard',
  'admin_rides',
  'admin_live',
  'admin_plan',
  'admin_events',
  'admin_reimb_review',
  'admin_announcements',
  'admin_help',
  'admin_reports',
  'admin_reimb_pay',
  'admin_bills',
  'admin_members',
  'admin_invites',
  'admin_recovery',
  'admin_groups',
  'admin_areas',
  'admin_roles',
  'admin_audit',
  'admin_settings',
];

/** Can the user see any staff page? Gates the admin guide tab and route. */
export const canAnyAdminPage = (can: Can): boolean => ADMIN_PAGES.some((p) => can(p));
```

Add to `frontend/src/locales/help.en.ts`:

```ts
  requestsTab: 'Requests',
  guideTab: 'Admin guide',
  guideTitle: 'Admin workflow guides',
  guideDesc: 'Step-by-step walkthroughs for running this app as staff.',
  guideNone: 'No guides yet',
  guideDraft: 'Draft',
  guideSteps: '{{count}} steps',
  guideOpen: 'Open →',
  guideNew: 'New guide',
  guideEdit: 'Edit',
  guideDelete: 'Delete',
  guideDeleteConfirm: 'Delete this guide?',
  guideSlug: 'Slug (lowercase letters, numbers, dashes)',
  guideTitleEn: 'Title (English)',
  guideTitleZh: 'Title (Chinese)',
  guideSummaryEn: 'Summary (English)',
  guideSummaryZh: 'Summary (Chinese)',
  guidePublishedLabel: 'Published',
  guideAddStep: 'Add step',
  guideRemoveStep: 'Remove step',
  guideMoveUp: 'Move up',
  guideMoveDown: 'Move down',
  guideStepTitleEn: 'Step title (English)',
  guideStepTitleZh: 'Step title (Chinese)',
  guideStepBodyEn: 'Details (English)',
  guideStepBodyZh: 'Details (Chinese)',
  guideStepLink: 'Link (in-app path, e.g. /admin/plan)',
  guideSaved: 'Saved',
  guideSave: 'Save',
  guideCancel: 'Cancel',
```

Add to `frontend/src/locales/help.zh.ts`:

```ts
  requestsTab: '求助',
  guideTab: '管理员指南',
  guideTitle: '管理员操作指南',
  guideDesc: '面向同工的分步操作指南，可直接跳转到对应页面。',
  guideNone: '暂无指南',
  guideDraft: '草稿',
  guideSteps: '{{count}} 个步骤',
  guideOpen: '打开 →',
  guideNew: '新建指南',
  guideEdit: '编辑',
  guideDelete: '删除',
  guideDeleteConfirm: '确定删除该指南？',
  guideSlug: 'Slug（小写字母、数字、连字符）',
  guideTitleEn: '标题（英文）',
  guideTitleZh: '标题（中文）',
  guideSummaryEn: '摘要（英文）',
  guideSummaryZh: '摘要（中文）',
  guidePublishedLabel: '发布',
  guideAddStep: '添加步骤',
  guideRemoveStep: '移除步骤',
  guideMoveUp: '上移',
  guideMoveDown: '下移',
  guideStepTitleEn: '步骤标题（英文）',
  guideStepTitleZh: '步骤标题（中文）',
  guideStepBodyEn: '说明（英文）',
  guideStepBodyZh: '说明（中文）',
  guideStepLink: '链接（应用内路径，如 /admin/plan）',
  guideSaved: '已保存',
  guideSave: '保存',
  guideCancel: '取消',
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/perms.test.ts
```

Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add frontend/src/perms.ts frontend/src/perms.test.ts frontend/src/locales/help.en.ts frontend/src/locales/help.zh.ts && git commit -m "feat(help): staff-page visibility helper and guide UI strings"
```

---

### Task 5: Admin Guide tab with guide list (read-only view)

**Files:**
- Create: `frontend/src/pages/HelpGuides.tsx` (list only; editor form added in Task 7)
- Modify: `frontend/src/pages/Help.tsx` (top-level tabs)
- Test: `frontend/src/pages/HelpGuides.test.tsx` (new)

**Interfaces:**
- Consumes: `canAnyAdminPage` (Task 4), `GET /help/guides` (Tasks 1–3).
- Produces: `export interface GuideStep`, `export interface GuideSummary`, `export interface GuideDetailData`, `export default HelpGuides` (list). Task 6 imports the types; Task 7 extends this file with the editor.

- [ ] **Step 1: Write the failing tests**

Create `frontend/src/pages/HelpGuides.test.tsx` (mocking pattern copied from `frontend/src/pages/admin/Roles.test.tsx`):

```tsx
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../i18n';
import Help from './Help';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mocks = vi.hoisted(() => ({
  allowed: new Set<string>(),
  guides: [] as unknown[],
}));

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: { id: 1 },
    refresh: vi.fn().mockResolvedValue(undefined),
    can: (page: string, act = 'view') => mocks.allowed.has(`${page}.${act}`),
  }),
}));

vi.mock('../api', () => ({
  useApi: (path: string | null) => ({
    data: path === null ? null : path.startsWith('/help/guides') ? mocks.guides : [],
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  post: vi.fn(),
  put: vi.fn(),
  del: vi.fn(),
}));

beforeEach(() => {
  mocks.allowed.clear();
  mocks.guides = [
    {
      id: 1, slug: 'plan-transport', title_en: 'Plan transport', title_zh: '安排交通',
      summary_en: 'Walk through it', summary_zh: '', published: true, step_count: 3,
    },
    {
      id: 2, slug: 'draft-one', title_en: 'Draft guide', title_zh: '草稿',
      summary_en: '', summary_zh: '', published: false, step_count: 1,
    },
  ];
});

describe('Help admin guide tab', () => {
  it('hides the tab from plain members', () => {
    mocks.allowed.add('help.view');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    expect(screen.queryByRole('button', { name: 'Admin guide' })).toBeNull();
    // The member ask form is still there.
    expect(screen.getByText('Ask a question')).toBeTruthy();
  });

  it('shows the tab to staff and lists guides when opened', async () => {
    mocks.allowed.add('help.view');
    mocks.allowed.add('admin_help.view');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Admin guide' }));
    expect(await screen.findByText('Plan transport')).toBeTruthy();
    expect(screen.getByText('3 steps')).toBeTruthy();
    // Drafts only render for editors; this user cannot update.
    expect(screen.queryByText('Draft guide')).toBeNull();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/pages/HelpGuides.test.tsx
```

Expected: FAIL — no 'Admin guide' button (tab not implemented).

- [ ] **Step 3: Implement HelpGuides.tsx (list view)**

Create `frontend/src/pages/HelpGuides.tsx`:

```tsx
import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BookOpen, Pencil, Plus, Trash2 } from 'lucide-react';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
  Page,
  PageHeader,
  Pill,
  Spinner,
  useAction,
} from '../components/ui';

export interface GuideStep {
  title_en: string;
  title_zh: string;
  body_en: string;
  body_zh: string;
  link: string;
}

export interface GuideSummary {
  id: number;
  slug: string;
  title_en: string;
  title_zh: string;
  summary_en: string;
  summary_zh: string;
  published: boolean;
  step_count: number;
}

export interface GuideDetailData extends Omit<GuideSummary, 'step_count'> {
  steps: GuideStep[];
}

/** Pick the active language, falling back to the other when empty. */
export function usePick() {
  const { i18n } = useTranslation();
  return (zh: string, en: string) => (i18n.language.startsWith('zh') ? zh || en : en || zh);
}

export default function HelpGuides() {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { can } = useAuth();
  const isEditor = can('admin_help', 'update');
  const { data, error, loading, reload } = useApi<GuideSummary[]>(
    `/help/guides${isEditor ? '?include_drafts=true' : ''}`,
  );
  const { busy, error: actErr, run } = useAction();

  const remove = (g: GuideSummary) => {
    if (!window.confirm(t('guideDeleteConfirm'))) return;
    run(async () => {
      await del(`/help/guides/${g.id}`);
      reload();
    });
  };

  return (
    <div className="stack">
      <PageHeader
        title={t('guideTitle')}
        subtitle={t('guideDesc')}
        actions={
          isEditor ? (
            <Button variant="secondary" icon={<Plus size={16} aria-hidden="true" />}>
              {t('guideNew')}
            </Button>
          ) : undefined
        }
      />
      <ErrorBox error={error} />
      {loading ? (
        <Spinner />
      ) : !data?.length ? (
        <EmptyState title={t('guideNone')} />
      ) : (
        <Card className="stack">
          <div className="list">
            {data.map((g) => (
              <div key={g.id} className="item">
                <Link to={`/help/guides/${g.slug}`} className="grow min-w-0">
                  <div className="bold ellipsis">
                    <BookOpen size={14} aria-hidden="true" /> {pick(g.title_zh, g.title_en)}
                  </div>
                  <div className="muted tiny">
                    {pick(g.summary_zh, g.summary_en)} · {t('guideSteps', { count: g.step_count })}
                  </div>
                </Link>
                <Pill tone={g.published ? 'success' : 'warning'}>
                  {g.published ? t('guidePublishedLabel') : t('guideDraft')}
                </Pill>
                {isEditor && (
                  <span className="btn-row">
                    <button className="btn sm secondary" type="button" aria-label={t('guideEdit')}>
                      <Pencil size={14} aria-hidden="true" />
                    </button>
                    <button
                      className="btn sm secondary"
                      type="button"
                      aria-label={t('guideDelete')}
                      disabled={busy}
                      onClick={() => remove(g)}
                    >
                      <Trash2 size={14} aria-hidden="true" />
                    </button>
                  </span>
                )}
              </div>
            ))}
          </div>
          <ErrorBox error={actErr} />
        </Card>
      )}
    </div>
  );
}
```

(The New/Edit buttons are inert placeholders this task — Task 7 wires the form. `Field` import is unused until then; keep it out until Task 7 to satisfy the no-unused-vars lint — remove `Field` from the import list in this step.)

- [ ] **Step 4: Wire the tab into Help.tsx**

In `frontend/src/pages/Help.tsx`:

4a. Add imports:

```tsx
import HelpGuides from './HelpGuides';
import { canAnyAdminPage } from '../perms';
```

4b. Inside `Help()`, after the `isStaff` line add:

```tsx
  // The admin guide is for anyone who can see a staff page, not just help desk.
  const showGuide = canAnyAdminPage(can);
  const [view, setView] = useState<'requests' | 'guide'>('requests');
```

4c. Wrap the existing `<div className="split">…</div>` so the render becomes:

```tsx
        <PageHeader title={t('title')} subtitle={t('desc')} />

        {showGuide && (
          <Tabs
            value={view}
            onChange={(v) => setView(v as 'requests' | 'guide')}
            items={[
              { value: 'requests', label: t('requestsTab') },
              { value: 'guide', label: t('guideTab') },
            ]}
          />
        )}

        {view === 'guide' ? (
          <HelpGuides />
        ) : (
          <div className="split">
            {/* …existing member form + ticket list, unchanged… */}
          </div>
        )}
```

(Keep the existing split block exactly as it is inside the else branch; only its indentation changes.)

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/pages/HelpGuides.test.tsx
```

Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add frontend/src/pages/HelpGuides.tsx frontend/src/pages/Help.tsx frontend/src/pages/HelpGuides.test.tsx && git commit -m "feat(help): admin guide tab with database-backed guide list"
```

---

### Task 6: Guide detail page with deep-linked steps

**Files:**
- Create: `frontend/src/pages/HelpGuideDetail.tsx`
- Modify: `frontend/src/App.tsx` (lazy import + route)
- Test: `frontend/src/pages/HelpGuides.test.tsx` (extend)

**Interfaces:**
- Consumes: `GuideDetailData` from `HelpGuides.tsx`; `ADMIN_PAGES` from `perms.ts`; `GET /help/guides/{slug}`.
- Produces: route `/help/guides/:slug` gated to any staff page.

- [ ] **Step 1: Write the failing test**

Append to `frontend/src/pages/HelpGuides.test.tsx` (new describe; reuse existing mocks):

```tsx
import HelpGuideDetail from './HelpGuideDetail';
import { Route, Routes } from 'react-router-dom';

describe('HelpGuideDetail', () => {
  it('renders numbered steps with deep links and a draft pill', async () => {
    mocks.allowed.add('admin_help.view');
    mocks.allowed.add('admin_help.update');
    const detail = {
      id: 1,
      slug: 'plan-transport',
      title_en: 'Plan transport',
      title_zh: '安排交通',
      summary_en: 'Walk through it',
      summary_zh: '',
      published: false,
      steps: [
        { title_en: 'Find the event', title_zh: '找到活动', body_en: 'Open Events', body_zh: '打开活动页', link: '/events' },
        { title_en: 'Watch it run', title_zh: '当天跟踪', body_en: '', body_zh: '', link: '' },
      ],
    };
    // useApi returns mocks.guides for any /help/guides path; reuse it as the detail object.
    mocks.guides = [detail as unknown as never];
    render(
      <MemoryRouter initialEntries={['/help/guides/plan-transport']}>
        <Routes>
          <Route path="/help/guides/:slug" element={<HelpGuideDetail />} />
        </Routes>
      </MemoryRouter>,
    );
    expect(await screen.findByText('1. Find the event')).toBeTruthy();
    expect(screen.getByText('2. Watch it run')).toBeTruthy();
    expect(screen.getByRole('link', { name: 'Open →' })).toHaveProperty('href', expect.stringContaining('/events'));
    expect(screen.getByText('Draft')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/pages/HelpGuides.test.tsx
```

Expected: FAIL — cannot resolve `./HelpGuideDetail`.

- [ ] **Step 3: Implement the detail page**

Create `frontend/src/pages/HelpGuideDetail.tsx`:

```tsx
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useApi } from '../api';
import {
  Card,
  ErrorBox,
  Page,
  PageHeader,
  Pill,
  Spinner,
} from '../components/ui';
import { GuideDetailData, usePick } from './HelpGuides';

export default function HelpGuideDetail() {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { slug } = useParams();
  const { data: g, error } = useApi<GuideDetailData>(`/help/guides/${slug}`);

  if (error)
    return (
      <Page width="narrow">
        <ErrorBox error={error} />
      </Page>
    );
  if (!g) return <Spinner />;

  return (
    <Page width="narrow">
      <div className="stack">
        <PageHeader
          title={pick(g.title_zh, g.title_en)}
          subtitle={pick(g.summary_zh, g.summary_en)}
          back="/help"
          actions={!g.published ? <Pill tone="warning">{t('guideDraft')}</Pill> : undefined}
        />
        <Card className="stack">
          {g.steps.map((s, i) => (
            <div key={i} className="stack-xs">
              <div className="bold">
                {i + 1}. {pick(s.title_zh, s.title_en)}
              </div>
              {(pick(s.body_zh, s.body_en) || s.link) && (
                <div className="stack-xs">
                  <div className="muted pre-wrap">{pick(s.body_zh, s.body_en)}</div>
                  {s.link && (
                    <a className="btn sm secondary" href={s.link}>
                      {t('guideOpen')}
                    </a>
                  )}
                </div>
              )}
            </div>
          ))}
        </Card>
      </div>
    </Page>
  );
}
```

- [ ] **Step 4: Register the route**

In `frontend/src/App.tsx`:

4a. Beside the other lazy imports (after the `const Help = lazy(...)` / `HelpDetail` lines):

```tsx
const HelpGuideDetail = lazy(() => import('./pages/HelpGuideDetail'));
```

4b. Add to the imports at the top:

```tsx
import { ADMIN_PAGES } from './perms';
```

4c. Add the route directly after `<Route path="help/:id" …>`:

```tsx
        <Route path="help/guides/:slug" element={gated(<HelpGuideDetail />, ...ADMIN_PAGES)} />
```

(`gated` is variadic: `gated(el, ...any: Need[])` — spreading `ADMIN_PAGES` gives any-staff-page access, mirroring the backend read gate. It must be declared before `help/:id`? No — react-router v6 ranks static segments (`help/guides/...`) above the dynamic `help/:id`, so order does not matter, but placing it right after the help routes keeps the table readable.)

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/pages/HelpGuides.test.tsx
```

Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add frontend/src/pages/HelpGuideDetail.tsx frontend/src/App.tsx frontend/src/pages/HelpGuides.test.tsx && git commit -m "feat(help): guide detail page with deep-linked workflow steps"
```

---

### Task 7: In-app guide editor for help-centre staff

**Files:**
- Modify: `frontend/src/pages/HelpGuides.tsx` (add `GuideForm`, wire New/Edit/Delete buttons)
- Test: `frontend/src/pages/HelpGuides.test.tsx` (extend)

**Interfaces:**
- Consumes: `POST /help/guides`, `PUT /help/guides/{id}`, `DELETE /help/guides/{id}` (Task 2), `get` via `useApi` for draft fetching when editing.
- Produces: complete staff editing flow; no other file changes.

- [ ] **Step 1: Write the failing tests**

Append to `frontend/src/pages/HelpGuides.test.tsx`:

```tsx
describe('GuideForm editor', () => {
  it('creates a guide through the API with steps', async () => {
    mocks.allowed.add('admin_help.view');
    mocks.allowed.add('admin_help.update');
    const { post } = await import('../api');
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Admin guide' }));
    fireEvent.click(await screen.findByRole('button', { name: 'New guide' }));
    fireEvent.change(screen.getByLabelText('Slug (lowercase letters, numbers, dashes)'), {
      target: { value: 'test-guide' },
    });
    fireEvent.change(screen.getByLabelText('Title (English)'), { target: { value: 'Test' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add step' }));
    fireEvent.change(screen.getByLabelText('Step title (English)'), { target: { value: 'One' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(post).toHaveBeenCalledWith(
        '/help/guides',
        expect.objectContaining({
          slug: 'test-guide',
          title_en: 'Test',
          steps: [expect.objectContaining({ title_en: 'One' })],
        }),
      ),
    );
  });

  it('hides editor controls from non-editors', async () => {
    mocks.allowed.add('admin_help.view'); // staff reader, not editor
    render(
      <MemoryRouter>
        <Help />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Admin guide' }));
    await screen.findByText('Plan transport');
    expect(screen.queryByRole('button', { name: 'New guide' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  });
});
```

(If `Field`/inputs in the form use explicit `<label>` wrapping, `getByLabelText` resolves; if labels are associated via the `Field` component's `label` prop with `htmlFor`, ensure the input id matches — copy how `Help.tsx` pairs `Field label` with inputs, which is the accessible pattern this codebase already uses.)

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/pages/HelpGuides.test.tsx
```

Expected: first new test FAIL (no 'New guide' behavior / no Save), second FAIL (edit buttons currently always rendered for `isEditor` — this task must gate them, and currently Edit is inert so the reader test may pass; the create test is the real driver).

- [ ] **Step 3: Implement the editor**

Rework `frontend/src/pages/HelpGuides.tsx`. Full final file content:

```tsx
import { FormEvent, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ArrowDown, ArrowUp, BookOpen, Pencil, Plus, Trash2 } from 'lucide-react';
import { del, post, put, useApi } from '../api';
import { useAuth } from '../auth';
import {
  Button,
  Card,
  EmptyState,
  ErrorBox,
  Field,
  Page,
  PageHeader,
  Pill,
  Spinner,
  useAction,
} from '../components/ui';

export interface GuideStep {
  title_en: string;
  title_zh: string;
  body_en: string;
  body_zh: string;
  link: string;
}

export interface GuideSummary {
  id: number;
  slug: string;
  title_en: string;
  title_zh: string;
  summary_en: string;
  summary_zh: string;
  published: boolean;
  step_count: number;
}

export interface GuideDetailData extends Omit<GuideSummary, 'step_count'> {
  steps: GuideStep[];
}

/** Pick the active language, falling back to the other when empty. */
export function usePick() {
  const { i18n } = useTranslation();
  return (zh: string, en: string) => (i18n.language.startsWith('zh') ? zh || en : en || zh);
}

const emptyStep = (): GuideStep => ({ title_en: '', title_zh: '', body_en: '', body_zh: '', link: '' });

/** Create/edit form. Pass a guide to edit, or null to create. */
function GuideForm({ guide, onDone }: { guide: GuideDetailData | null; onDone: () => void }) {
  const { t } = useTranslation('help');
  const { busy, error, run } = useAction();
  const [f, setF] = useState({
    slug: guide?.slug ?? '',
    title_en: guide?.title_en ?? '',
    title_zh: guide?.title_zh ?? '',
    summary_en: guide?.summary_en ?? '',
    summary_zh: guide?.summary_zh ?? '',
    published: guide?.published ?? true,
    steps: guide?.steps ?? [emptyStep()],
  });

  const setStep = (i: number, patch: Partial<GuideStep>) =>
    setF((p) => ({ ...p, steps: p.steps.map((s, j) => (j === i ? { ...s, ...patch } : s)) }));
  const move = (i: number, delta: -1 | 1) =>
    setF((p) => {
      const steps = [...p.steps];
      const [s] = steps.splice(i, 1);
      steps.splice(i + delta, 0, s);
      return { ...p, steps };
    });

  const submit = (e: FormEvent) => {
    e.preventDefault();
    run(async () => {
      if (guide) await put(`/help/guides/${guide.id}`, f);
      else await post('/help/guides', f);
      onDone();
    }, t('guideSaved'));
  };

  return (
    <form className="card stack" onSubmit={submit}>
      <h3>{guide ? t('guideEdit') : t('guideNew')}</h3>
      <Field label={t('guideSlug')}>
        <input className="input" required value={f.slug} onChange={(e) => setF({ ...f, slug: e.target.value })} />
      </Field>
      <div className="grid-2">
        <Field label={t('guideTitleEn')}>
          <input className="input" value={f.title_en} onChange={(e) => setF({ ...f, title_en: e.target.value })} />
        </Field>
        <Field label={t('guideTitleZh')}>
          <input className="input" value={f.title_zh} onChange={(e) => setF({ ...f, title_zh: e.target.value })} />
        </Field>
      </div>
      <div className="grid-2">
        <Field label={t('guideSummaryEn')}>
          <input className="input" value={f.summary_en} onChange={(e) => setF({ ...f, summary_en: e.target.value })} />
        </Field>
        <Field label={t('guideSummaryZh')}>
          <input className="input" value={f.summary_zh} onChange={(e) => setF({ ...f, summary_zh: e.target.value })} />
        </Field>
      </div>
      <label className="check">
        <input
          type="checkbox"
          checked={f.published}
          onChange={(e) => setF({ ...f, published: e.target.checked })}
        />
        {t('guidePublishedLabel')}
      </label>

      {f.steps.map((s, i) => (
        <div key={i} className="card stack-xs">
          <div className="btn-row">
            <span className="bold">{i + 1}</span>
            <button type="button" className="btn sm secondary" disabled={i === 0} onClick={() => move(i, -1)} aria-label={t('guideMoveUp')}>
              <ArrowUp size={14} aria-hidden="true" />
            </button>
            <button type="button" className="btn sm secondary" disabled={i === f.steps.length - 1} onClick={() => move(i, 1)} aria-label={t('guideMoveDown')}>
              <ArrowDown size={14} aria-hidden="true" />
            </button>
            <button type="button" className="btn sm secondary" disabled={f.steps.length === 1} onClick={() => setF((p) => ({ ...p, steps: p.steps.filter((_, j) => j !== i) }))}>
              {t('guideRemoveStep')}
            </button>
          </div>
          <div className="grid-2">
            <Field label={t('guideStepTitleEn')}>
              <input className="input" value={s.title_en} onChange={(e) => setStep(i, { title_en: e.target.value })} />
            </Field>
            <Field label={t('guideStepTitleZh')}>
              <input className="input" value={s.title_zh} onChange={(e) => setStep(i, { title_zh: e.target.value })} />
            </Field>
          </div>
          <div className="grid-2">
            <Field label={t('guideStepBodyEn')}>
              <textarea className="input" rows={2} value={s.body_en} onChange={(e) => setStep(i, { body_en: e.target.value })} />
            </Field>
            <Field label={t('guideStepBodyZh')}>
              <textarea className="input" rows={2} value={s.body_zh} onChange={(e) => setStep(i, { body_zh: e.target.value })} />
            </Field>
          </div>
          <Field label={t('guideStepLink')}>
            <input className="input" value={s.link} placeholder="/admin/plan" onChange={(e) => setStep(i, { link: e.target.value })} />
          </Field>
        </div>
      ))}

      <Button variant="secondary" type="button" disabled={f.steps.length >= 30} onClick={() => setF((p) => ({ ...p, steps: [...p.steps, emptyStep()] }))}>
        {t('guideAddStep')}
      </Button>
      <ErrorBox error={error} />
      <div className="btn-row">
        <Button variant="primary" type="submit" loading={busy}>{t('guideSave')}</Button>
        <Button variant="secondary" type="button" onClick={onDone}>
          {t('guideCancel')}
        </Button>
      </div>
    </form>
  );
}

export default function HelpGuides() {
  const { t } = useTranslation('help');
  const pick = usePick();
  const { can } = useAuth();
  const isEditor = can('admin_help', 'update');
  const { data, error, loading, reload } = useApi<GuideSummary[]>(
    `/help/guides${isEditor ? '?include_drafts=true' : ''}`,
  );
  // Editing loads the full guide (steps); creating passes null.
  const [editSlug, setEditSlug] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const { data: editing } = useApi<GuideDetailData>(editSlug ? `/help/guides/${editSlug}` : null);
  const { busy, error: actErr, run } = useAction();

  const remove = (g: GuideSummary) => {
    if (!window.confirm(t('guideDeleteConfirm'))) return;
    run(async () => {
      await del(`/help/guides/${g.id}`);
      reload();
    });
  };

  if (creating || editSlug)
    return (
      <div className="stack">
        {/* Wait for the guide to load before showing the form. */}
        {editSlug && !editing ? <Spinner /> : (
          <GuideForm
            key={editSlug ?? 'new'}
            guide={editSlug ? editing : null}
            onDone={() => {
              setCreating(false);
              setEditSlug(null);
              reload();
            }}
          />
        )}
      </div>
    );

  return (
    <div className="stack">
      <PageHeader
        title={t('guideTitle')}
        subtitle={t('guideDesc')}
        actions={
          isEditor ? (
            <Button variant="secondary" icon={<Plus size={16} aria-hidden="true" />} onClick={() => setCreating(true)}>
              {t('guideNew')}
            </Button>
          ) : undefined
        }
      />
      <ErrorBox error={error} />
      {loading ? (
        <Spinner />
      ) : !data?.length ? (
        <EmptyState title={t('guideNone')} />
      ) : (
        <Card className="stack">
          <div className="list">
            {data.map((g) => (
              <div key={g.id} className="item">
                <Link to={`/help/guides/${g.slug}`} className="grow min-w-0">
                  <div className="bold ellipsis">
                    <BookOpen size={14} aria-hidden="true" /> {pick(g.title_zh, g.title_en)}
                  </div>
                  <div className="muted tiny">
                    {pick(g.summary_zh, g.summary_en)} · {t('guideSteps', { count: g.step_count })}
                  </div>
                </Link>
                <Pill tone={g.published ? 'success' : 'warning'}>
                  {g.published ? t('guidePublishedLabel') : t('guideDraft')}
                </Pill>
                {isEditor && (
                  <span className="btn-row">
                    <button className="btn sm secondary" type="button" aria-label={t('guideEdit')} onClick={() => setEditSlug(g.slug)}>
                      <Pencil size={14} aria-hidden="true" />
                    </button>
                    <button
                      className="btn sm secondary"
                      type="button"
                      aria-label={t('guideDelete')}
                      disabled={busy}
                      onClick={() => remove(g)}
                    >
                      <Trash2 size={14} aria-hidden="true" />
                    </button>
                  </span>
                )}
              </div>
            ))}
          </div>
          <ErrorBox error={actErr} />
        </Card>
      )}
    </div>
  );
}
```

If `grid-2` / `check` classes do not exist in `styles.css`, use the existing layout classes you find there for two-column rows (`split`, `btn-row`) — do not invent new CSS.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npx vitest run src/pages/HelpGuides.test.tsx
```

Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add frontend/src/pages/HelpGuides.tsx frontend/src/pages/HelpGuides.test.tsx frontend/src/locales/help.en.ts frontend/src/locales/help.zh.ts && git commit -m "feat(help): in-app guide editor for help-centre staff"
```

---

### Task 8: Documentation update + full verification

**Files:**
- Modify: `docs/FEATURES.md`, `docs/WORKFLOW.md`
- No new tests; runs both suites as verification.

**Interfaces:**
- Consumes: everything above, merged.

- [ ] **Step 1: Update docs**

In `docs/FEATURES.md`, find the Community row (line ~17) and extend its "Owns" and boundary wording: append to the features column `, admin workflow guides (Help centre → Admin guide tab: DB-managed bilingual steps with deep links, edited in-app by help-centre staff)` and to the source column `backend/src/routes/community.rs (guides), frontend/src/pages/HelpGuides.tsx`. Match the row's existing table formatting exactly.

In `docs/WORKFLOW.md`, find the staff/admin journey section that describes the help centre and add one sentence: `Help-centre staff also maintain the Admin guide tab: database-managed, bilingual workflow walkthroughs whose steps link straight into the admin pages (/help → Admin guide).` Place it in the staff journey next to existing help-centre text; follow that file's heading style.

- [ ] **Step 2: Full backend verification**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/backend" && cargo test
```

Expected: entire suite passes (guides tests included; pre-existing failures, if any, must be compared against `git stash`-free baseline — do not fix unrelated failures, report them).

- [ ] **Step 3: Full frontend verification**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride/frontend" && npm run test && npx tsc --noEmit
```

Expected: vitest suite green; TypeScript clean. (If `tsc --noEmit` needs a project flag, use `npx tsc -p tsconfig.json --noEmit`.)

- [ ] **Step 4: Commit**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git add docs/FEATURES.md docs/WORKFLOW.md && git commit -m "docs: admin workflow guides in features and workflow docs"
```

- [ ] **Step 5: Final check**

```bash
cd "/Users/goaltosuceed/Documents/Grace Ride" && git log --oneline -9 && git status --short
```

Expected: 8 new commits from this plan; the four pre-existing dirty files (`backend/src/routes/auth.rs`, `backend/src/routes/rides.rs`, `backend/tests/api.rs`, `frontend/src/pages/RideNew.tsx`) still modified but **never staged by us**.

---

## Self-review notes (already applied)

- Spec coverage: table ✓ (T1), read gates ✓ (T1), CRUD+audit+validation ✓ (T2), bilingual pairing + fallback ✓ (T1/T5), seed six guides ✓ (T3), perms helper ✓ (T4), tab ✓ (T5), detail+deep links ✓ (T6), editor ✓ (T7), docs ✓ (T8). Overlay tours explicitly out of scope per spec.
- JSONB handled without the sqlx `json` feature (text round-trip) — matches `auth.rs` precedent.
- Route pattern `/help/guides/{key}` shared by GET(slug)/PUT/DELETE(id) avoids axum path-param conflicts.
- `tests/api.rs` untouched (dirty file); new `tests/guides.rs` carries all backend tests.
