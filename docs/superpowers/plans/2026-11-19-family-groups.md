# Family groups implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An approved member can name one family and add people by phone number, either as an in-app request or as a link they send, without changing rides or church groups.

**Architecture:** New `families`, `family_members`, and `family_requests` tables. A family invite is an `invites` row with `kind = 'family'`. Handlers live in `backend/src/routes/community/family.rs` under `/api/me/family`, which the pending-account allowlist opens. The profile screen and the existing invite page are the only UI.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL, React, TypeScript, Vite, Vitest, i18next.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-11-19-family-groups-design.md`.
- A family holds at most 8 counted people. Members, open requests, and unused unexpired invites all count.
- One person, one family (`UNIQUE (user_id)` on `family_members`). One open request per person (`UNIQUE (user_id)` on `family_requests`).
- Family names are not unique. `check_name` enforces non-empty and at most 60 characters.
- Phone length stays `check_phone`: empty is allowed on the profile, 1–9 and 21+ characters fail. The family add handler rejects an empty phone with `PHONE_INVALID`. Do not change `check_phone` to reject empty.
- Phone key: digits only, and one leading `0` becomes `60`. No other rewriting. Store the owner's typed text.
- Invite lifetime is `in_days(3)`. The raw token is stored, same as an admin invite. URL is `{public_url}/invite/{token}`.
- New accounts from a family link are `status = 'pending'`, `roles = ''`, no church groups. Terms are required. The server writes the phone stored on the invite and ignores the phone in the accept body.
- `pending_profile_only` allows `/me/family` and paths under it. Do not add those paths to the session-lock bypass. Do not allow every `/me/*` path.
- Pending layout redirect must allow `/me/family`. Axum paths use `{param}`, not `:param`.
- Error text is the spec table, both languages. `FAMILY_FULL` carries `max` (`8`). Frontend strings use `{max}`.
- Audit detail text contains ids only, never a phone number.
- `family_name` is returned only when the caller has `Page::AdminMembers` + `Act::View`.
- Church `groups` and `group_members` are not written by any family handler.
- Demo owner for tests: `member@example.org` (陈法拉, phone `012-300 0006`). Non-staff list caller: `enci@example.org` (role `leader`, has Events update, no AdminMembers view). Staff list caller: `admin@example.org`.

---

### Task 1: Phone key and family errors

**Files:**
- Modify: `backend/src/util.rs` (add `phone_key` next to `check_phone`)
- Modify: `backend/src/i18n/keys.rs`
- Create: `backend/src/i18n/locales/family.rs`
- Modify: `backend/src/i18n/locales/mod.rs`
- Modify: `frontend/src/api.ts` (`ERROR_GROUPS`)
- Modify: `frontend/src/locales/errors.en.ts`
- Modify: `frontend/src/locales/errors.zh.ts`
- Test: `backend/src/util.rs` (`mod tests`), `backend/src/i18n/locales/mod.rs` (existing catalog test)

**Interfaces:**
- Consumes: `check_phone`, `ErrorKey::code`, `ErrorKey::params`, `locales::render`, `locales::samples`.
- Produces: `pub fn phone_key(phone: &str) -> String`. `ErrorKey::FamilyActiveOnly`, `AlreadyInFamily`, `FamilyRequestPending`, `FamilyOwnPhone`, `FamilyPhoneAmbiguous`, `FamilyPhoneUnavailable`, `FamilyFull { max: i64 }`, `FamilyOwnerCannotLeave`. Codes: `FAMILY_ACTIVE_ONLY`, `ALREADY_IN_FAMILY`, `FAMILY_REQUEST_PENDING`, `FAMILY_OWN_PHONE`, `FAMILY_PHONE_AMBIGUOUS`, `FAMILY_PHONE_UNAVAILABLE`, `FAMILY_FULL`, `FAMILY_OWNER_CANNOT_LEAVE`. `FamilyFull { max }.params()` is `{"max": max}`.

- [ ] **Step 1: Write the failing phone-key test**

Add this at the bottom of `backend/src/util.rs` (create `mod tests` if the file has none):

```rust
#[cfg(test)]
mod tests {
    use super::{check_phone, phone_key};
    use crate::i18n::Lang;

    #[test]
    fn phone_key_folds_a_single_leading_zero() {
        assert_eq!(phone_key("012-345 6789"), "60123456789");
        assert_eq!(phone_key("+60 12-345 6789"), "60123456789");
        assert_eq!(phone_key("60123456789"), "60123456789");
        assert_eq!(phone_key("1234567890"), "1234567890");
    }

    #[test]
    fn check_phone_still_allows_empty_and_rejects_bad_lengths() {
        assert!(check_phone(Lang::En, "").is_ok());
        assert!(check_phone(Lang::En, "012-345 6789").is_ok());
        assert!(check_phone(Lang::En, "123").is_err());
        assert!(check_phone(Lang::En, &"1".repeat(21)).is_err());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib phone_key_folds_a_single_leading_zero --manifest-path backend/Cargo.toml`

Expected: FAIL compiling, `phone_key` is not found.

- [ ] **Step 3: Implement `phone_key` and the error catalog**

In `util.rs`, directly under `check_phone`:

```rust
/// Digits only. A single leading 0 is rewritten to 60 so 012… and +60 12… match.
pub fn phone_key(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    match digits.strip_prefix('0') {
        Some(rest) => format!("60{rest}"),
        None => digits,
    }
}
```

Create `backend/src/i18n/locales/family.rs`:

```rust
//! Family-list errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum FamilyError {
    ActiveOnly,
    AlreadyInFamily,
    RequestPending,
    OwnPhone,
    PhoneAmbiguous,
    PhoneUnavailable,
    Full(i64),
    OwnerCannotLeave,
}

pub fn render(lang: Lang, key: FamilyError) -> String {
    match key {
        FamilyError::ActiveOnly => pair(
            lang,
            "账号通过审核后才能建立或邀请家人",
            "You can create a family or add someone after an admin approves your account",
        ),
        FamilyError::AlreadyInFamily => pair(lang, "已经在一个家庭里了", "Already in a family"),
        FamilyError::RequestPending => pair(
            lang,
            "这个号码已有另一个家庭的待处理邀请",
            "This number already has an open request from another family",
        ),
        FamilyError::OwnPhone => pair(lang, "这是你自己的号码", "That is your own number"),
        FamilyError::PhoneAmbiguous => pair(
            lang,
            "这个号码对应多位会员，请联系管理员",
            "This number matches more than one member. Ask an admin",
        ),
        FamilyError::PhoneUnavailable => pair(
            lang,
            "这个号码无法加入，请联系管理员",
            "This number cannot be added. Ask an admin",
        ),
        FamilyError::Full(max) => match lang {
            Lang::Zh => format!("一个家庭最多 {max} 人"),
            Lang::En => format!("A family can have at most {max} people"),
        },
        FamilyError::OwnerCannotLeave => pair(
            lang,
            "户主请删除整个家庭，而不能退出",
            "The owner deletes the family instead of leaving",
        ),
    }
}
```

In `keys.rs`, add the variants after `CannotViewFile`:

```rust
    // === Family ===
    FamilyActiveOnly,
    AlreadyInFamily,
    FamilyRequestPending,
    FamilyOwnPhone,
    FamilyPhoneAmbiguous,
    FamilyPhoneUnavailable,
    FamilyFull { max: i64 },
    FamilyOwnerCannotLeave,
```

Add the `code()` arms:

```rust
            Self::FamilyActiveOnly => "FAMILY_ACTIVE_ONLY",
            Self::AlreadyInFamily => "ALREADY_IN_FAMILY",
            Self::FamilyRequestPending => "FAMILY_REQUEST_PENDING",
            Self::FamilyOwnPhone => "FAMILY_OWN_PHONE",
            Self::FamilyPhoneAmbiguous => "FAMILY_PHONE_AMBIGUOUS",
            Self::FamilyPhoneUnavailable => "FAMILY_PHONE_UNAVAILABLE",
            Self::FamilyFull { .. } => "FAMILY_FULL",
            Self::FamilyOwnerCannotLeave => "FAMILY_OWNER_CANNOT_LEAVE",
```

In `params()`, before the wildcard:

```rust
            Self::FamilyFull { max } => Some(json!({ "max": max })),
```

In `locales/mod.rs`: `pub mod family;`, `use family::FamilyError;`, and a `render` arm for each new `ErrorKey` that calls `family::render`. Add the same variants to `samples()` inside `all_error_samples!`. `FamilyFull` is `ErrorKey::FamilyFull { max: 8 }`. Add `"FamilyFull"` to the `dynamic` array in `every_error_has_a_unique_code_and_both_languages`.

In `frontend/src/api.ts` append `'family'` to `ERROR_GROUPS`.

In both `errors.en.ts` and `errors.zh.ts`, add a `"family"` object as a sibling of `"files"` with the eight codes. English `FAMILY_FULL` is `A family can have at most {max} people`. Chinese is `一个家庭最多 {max} 人`. Copy the other seven strings from `family.rs`.

- [ ] **Step 4: Run the tests**

Run:

```bash
cargo test --lib phone_key_folds_a_single_leading_zero --manifest-path backend/Cargo.toml
cargo test --lib check_phone_still_allows_empty_and_rejects_bad_lengths --manifest-path backend/Cargo.toml
cargo test --lib every_error_has_a_unique_code_and_both_languages --manifest-path backend/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/util.rs backend/src/i18n/keys.rs backend/src/i18n/locales/family.rs backend/src/i18n/locales/mod.rs frontend/src/api.ts frontend/src/locales/errors.en.ts frontend/src/locales/errors.zh.ts
git commit -m "feat(family): add phone key and family error catalog"
```

---

### Task 2: Family tables

**Files:**
- Create: `backend/migrations/0015_families.sql`
- Modify: `backend/tests/schema_baseline.rs` (`LATER_RULES`)
- Test: `backend/tests/schema_baseline.rs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: tables `families`, `family_members`, `family_requests`. Columns `invites.family_id` and `invites.phone`. Checks `chk_invites_kind` (now includes `'family'`) and `chk_invites_family`.

- [ ] **Step 1: Write the migration**

`backend/migrations/0015_families.sql`:

```sql
-- Member-owned family lists. Separate from church groups.
CREATE TABLE IF NOT EXISTS families (
    id         BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    name       TEXT NOT NULL,
    owner_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (app_now())
);

CREATE TABLE IF NOT EXISTS family_members (
    family_id  BIGINT NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (app_now()),
    PRIMARY KEY (family_id, user_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS family_members_user_id ON family_members(user_id);

CREATE TABLE IF NOT EXISTS family_requests (
    id         BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    family_id  BIGINT NOT NULL REFERENCES families(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (app_now())
);
CREATE UNIQUE INDEX IF NOT EXISTS family_requests_user_id ON family_requests(user_id);

ALTER TABLE invites ADD COLUMN IF NOT EXISTS family_id BIGINT REFERENCES families(id) ON DELETE CASCADE;
ALTER TABLE invites ADD COLUMN IF NOT EXISTS phone TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_invites_family ON invites(family_id) WHERE family_id IS NOT NULL;

ALTER TABLE invites DROP CONSTRAINT IF EXISTS chk_invites_kind;
ALTER TABLE invites ADD CONSTRAINT chk_invites_kind CHECK (kind IN ('invite', 'recovery', 'family'));

ALTER TABLE invites DROP CONSTRAINT IF EXISTS chk_invites_family;
ALTER TABLE invites ADD CONSTRAINT chk_invites_family CHECK (
    (kind = 'family' AND family_id IS NOT NULL AND length(trim(phone)) >= 10)
    OR (kind <> 'family' AND family_id IS NULL)
) NOT VALID;
ALTER TABLE invites VALIDATE CONSTRAINT chk_invites_family;
```

- [ ] **Step 2: List the new check in the baseline test**

In `LATER_RULES`, immediately after `"chk_invites_kind"`, add `"chk_invites_family"`.

- [ ] **Step 3: Run the baseline test**

Run: `cargo test --test schema_baseline --manifest-path backend/Cargo.toml`

Expected: PASS. The test database is `TEST_DATABASE_URL` or `postgres://localhost/grace_ride_test`.

- [ ] **Step 4: Commit**

```bash
git add backend/migrations/0015_families.sql backend/tests/schema_baseline.rs
git commit -m "feat(family): add family tables and invite kind"
```

---

### Task 3: Create, read, rename, delete, leave, remove

**Files:**
- Create: `backend/src/routes/community/family.rs`
- Modify: `backend/src/routes/community/mod.rs` (module + routes)
- Modify: `backend/src/security.rs` (`pending_profile_only`)
- Create: `backend/tests/family.rs`
- Test: `backend/tests/family.rs`

**Interfaces:**
- Consumes: `phone_key` is not used yet. `ErrorKey` variants from Task 1. Tables from Task 2. `CurrentUser` (`id`, `status`, `lang`, `name`). `audit`, `check_name`, `now`. `common::setup`, `common::login`, `common::call`.
- Produces: routes below. `GET /api/me/family` JSON: `family` `{id, name, owner_id}` or null, `members` `{user_id, name, status}[]`, `requests` `[]`, `invites` `[]`, `incoming` null until Task 4 fills requests, invites, and incoming.

| Action | Route |
|---|---|
| Read | `GET /api/me/family` |
| Create | `POST /api/me/family` `{name}` → `{id, name}` |
| Rename | `PATCH /api/me/family` `{name}` |
| Delete | `DELETE /api/me/family` |
| Leave | `POST /api/me/family/leave` |
| Remove | `DELETE /api/me/family/members/{user_id}` |

- [ ] **Step 1: Write the failing API tests**

Create `backend/tests/family.rs`:

```rust
mod common;

use axum::http::{Method, StatusCode};
use common::{call, login, setup};
use serde_json::json;

async fn session_for(st: &grace_ride::state::AppState, email: &str) -> String {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email=$1")
        .bind(email)
        .fetch_one(&st.db)
        .await
        .unwrap();
    let token = uuid::Uuid::new_v4().simple().to_string() + &uuid::Uuid::new_v4().simple().to_string();
    let token = token[..48].to_string();
    let expires = (grace_ride::util::local_now() + chrono::Duration::days(7))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    sqlx::query(
        "INSERT INTO sessions (token, user_id, mfa_ok, expires_at, created_at) VALUES ($1,$2,1,$3,$4)",
    )
    .bind(grace_ride::auth::hash_token(&token))
    .bind(id)
    .bind(&expires)
    .bind(grace_ride::util::now())
    .execute(&st.db)
    .await
    .unwrap();
    format!("{}={token}", grace_ride::auth::SESSION_COOKIE)
}

async fn groups(st: &grace_ride::state::AppState) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM group_members")
        .fetch_one(&st.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn active_member_creates_one_family_and_pending_cannot() {
    let (app, st) = setup().await;
    let before = groups(&st).await;
    let owner = login(&st, "member@example.org").await;
    let empty = call(&app, Method::GET, "/api/me/family", Some(&owner), None).await;
    assert_eq!(empty.status, StatusCode::OK);
    assert!(empty.json["family"].is_null());

    let created = call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "陈家"})),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.json);
    assert_eq!(created.json["name"], "陈家");

    let view = call(&app, Method::GET, "/api/me/family", Some(&owner), None).await;
    assert_eq!(view.json["family"]["owner_id"], view.json["members"][0]["user_id"]);
    assert_eq!(view.json["members"][0]["name"], "陈法拉");
    assert!(view.json["members"][0].get("phone").is_none());
    assert!(view.json["members"][0].get("email").is_none());

    let again = call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&owner),
        Some(json!({"name": "另一家"})),
    )
    .await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
    assert_eq!(again.json["code"], "ALREADY_IN_FAMILY");

    sqlx::query("INSERT INTO users (name, email, phone, roles, status, webauthn_uuid) VALUES ('待审','kin-pending@example.org','','','pending',$1)")
        .bind(uuid::Uuid::new_v4().to_string())
        .execute(&st.db)
        .await
        .unwrap();
    let pending = session_for(&st, "kin-pending@example.org").await;
    let denied = call(
        &app,
        Method::POST,
        "/api/me/family",
        Some(&pending),
        Some(json!({"name": "不能建"})),
    )
    .await;
    assert_eq!(denied.json["code"], "FAMILY_ACTIVE_ONLY");
    assert_eq!(groups(&st).await, before);
}

#[tokio::test]
async fn owner_renames_and_deletes_member_leaves_and_owner_cannot() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let (fid,): (i64,) = sqlx::query_as("SELECT id FROM families").fetch_one(&st.db).await.unwrap();
    let (amina,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='amina@example.org'")
        .fetch_one(&st.db).await.unwrap();
    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)")
        .bind(fid).bind(amina).bind(grace_ride::util::now())
        .execute(&st.db).await.unwrap();

    let renamed = call(&app, Method::PATCH, "/api/me/family", Some(&owner), Some(json!({"name": "法拉的家"}))).await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.json);

    let amina_cookie = login(&st, "amina@example.org").await;
    let nope = call(&app, Method::PATCH, "/api/me/family", Some(&amina_cookie), Some(json!({"name": "不行"}))).await;
    assert_eq!(nope.json["code"], "NO_PERMISSION");

    let owner_leave = call(&app, Method::POST, "/api/me/family/leave", Some(&owner), None).await;
    assert_eq!(owner_leave.json["code"], "FAMILY_OWNER_CANNOT_LEAVE");

    let left = call(&app, Method::POST, "/api/me/family/leave", Some(&amina_cookie), None).await;
    assert_eq!(left.status, StatusCode::OK, "{}", left.json);
    let (still_user,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email='amina@example.org'")
        .fetch_one(&st.db).await.unwrap();
    assert_eq!(still_user, 1);
    let (amina_rows,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM family_members WHERE user_id=$1")
        .bind(amina).fetch_one(&st.db).await.unwrap();
    assert_eq!(amina_rows, 0);

    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)")
        .bind(fid).bind(amina).bind(grace_ride::util::now())
        .execute(&st.db).await.unwrap();
    let removed = call(&app, Method::DELETE, &format!("/api/me/family/members/{amina}"), Some(&owner), None).await;
    assert_eq!(removed.status, StatusCode::OK, "{}", removed.json);
    let (account,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id=$1")
        .bind(amina).fetch_one(&st.db).await.unwrap();
    assert_eq!(account, 1);

    let (owner_id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email='member@example.org'")
        .fetch_one(&st.db).await.unwrap();
    let self_remove = call(&app, Method::DELETE, &format!("/api/me/family/members/{owner_id}"), Some(&owner), None).await;
    assert_eq!(self_remove.json["code"], "FAMILY_OWNER_CANNOT_LEAVE");

    let gone = call(&app, Method::DELETE, "/api/me/family", Some(&owner), None).await;
    assert_eq!(gone.status, StatusCode::OK, "{}", gone.json);
    let (families,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM families").fetch_one(&st.db).await.unwrap();
    assert_eq!(families, 0);
    let (owner_account,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email='member@example.org'")
        .fetch_one(&st.db).await.unwrap();
    assert_eq!(owner_account, 1);
}

#[tokio::test]
async fn pending_member_can_read_and_leave_but_not_open_rides() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let (fid,): (i64,) = sqlx::query_as("SELECT id FROM families").fetch_one(&st.db).await.unwrap();
    let (kid,): (i64,) = sqlx::query_as(
        "INSERT INTO users (name, email, phone, roles, status, webauthn_uuid) VALUES ('待审','kin-pending@example.org','','','pending',$1) RETURNING id",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .fetch_one(&st.db).await.unwrap();
    sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)")
        .bind(fid).bind(kid).bind(grace_ride::util::now())
        .execute(&st.db).await.unwrap();
    let pending = session_for(&st, "kin-pending@example.org").await;
    let view = call(&app, Method::GET, "/api/me/family", Some(&pending), None).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.json);
    assert_eq!(view.json["members"].as_array().unwrap().len(), 2);
    let rides = call(&app, Method::GET, "/api/rides", Some(&pending), None).await;
    assert_eq!(rides.status, StatusCode::FORBIDDEN);
    assert_eq!(rides.json["code"], "ACCOUNT_PENDING");
    let left = call(&app, Method::POST, "/api/me/family/leave", Some(&pending), None).await;
    assert_eq!(left.status, StatusCode::OK, "{}", left.json);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test family --manifest-path backend/Cargo.toml`

Expected: FAIL. The route is missing (404 or a compile error if the test calls a module that does not exist). The test file itself must compile.

- [ ] **Step 3: Implement the handlers**

`pending_profile_only` in `security.rs`: after `path == "/me"`, add `|| path.starts_with("/me/family")`. Do not change `lock_bypass_path`.

`community/mod.rs`: `mod family;` and these routes inside `router()`:

```rust
.route("/me/family", get(family::get_family).post(family::create_family).patch(family::rename_family).delete(family::delete_family))
.route("/me/family/leave", post(family::leave_family))
.route("/me/family/members/{user_id}", delete(family::remove_member))
```

Add `patch` to the axum routing import.

`family.rs` behavior:

- `get_family`: load `families` joined through `family_members` for `u.id`. Members are `user_id`, `name`, `status` ordered by `family_members.created_at`, then `user_id`. No phone, no email. `requests` and `invites` are empty arrays. `incoming` is null.
- `create_family`: if `u.status != "active"`, `bad(FamilyActiveOnly)`. Else if a `family_members` row exists, `bad(AlreadyInFamily)`. `check_name`. Insert `families` and the owner `family_members` row in one transaction. Audit `family.create` on entity `family` with detail `family_id={id}`.
- `rename_family`: caller must be `owner_id`, else `forbidden(NoPermission)`. `check_name`. Audit `family.rename`.
- `delete_family`: owner only. `DELETE FROM families WHERE id=$1`. Audit `family.delete`. Accounts stay because users are not cascading from this delete except the owner-user FK, which is the other direction.
- `leave_family`: no membership → `not_found`. Owner → `bad(FamilyOwnerCannotLeave)`. Otherwise delete that `family_members` row. Audit `family.leave` with `family_id` and `user_id`.
- `remove_member`: owner only. Target equal to owner → `bad(FamilyOwnerCannotLeave)`. Missing member → `not_found`. Delete the row, keep the user. Audit `family.member.remove` with both ids.

Return `{ok: true}` for rename, delete, leave, and remove. Create returns `{id, name}`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test family --manifest-path backend/Cargo.toml`

Expected: PASS. `cargo test --lib every_error_has_a_unique_code_and_both_languages --manifest-path backend/Cargo.toml` still PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/community/family.rs backend/src/routes/community/mod.rs backend/src/security.rs backend/tests/family.rs
git commit -m "feat(family): create and manage a family list"
```

---

### Task 4: Add a person by phone

**Files:**
- Modify: `backend/src/routes/community/family.rs`
- Modify: `backend/src/routes/community/mod.rs`
- Modify: `backend/tests/family.rs`
- Test: `backend/tests/family.rs`

**Interfaces:**
- Consumes: `phone_key`, `FAMILY` errors, `random_token`, `in_days`, `notify`, `check_phone`, `AppState.cfg.public_url`.
- Produces: `pub const FAMILY_CAP: i64 = 8`. `POST /api/me/family/people` `{phone}` returns `{kind:"request", id, name}` or `{kind:"invite", id, phone, expires_at, url}`. `DELETE /api/me/family/requests/{id}`. `DELETE /api/me/family/invites/{id}` sets `expires_at` to `minutes_ago(1)`. `GET /api/me/family` now includes the owner's `requests` and `invites`, and `incoming` `{id, family_name, owner_name}` for the requested user.

Phone-key SQL, used for both `users.phone` and `invites.phone`:

```sql
CASE WHEN regexp_replace(phone, '[^0-9]', '', 'g') LIKE '0%'
     THEN '60' || substring(regexp_replace(phone, '[^0-9]', '', 'g') from 2)
     ELSE regexp_replace(phone, '[^0-9]', '', 'g')
END
```

- [ ] **Step 1: Write the failing tests**

Append to `backend/tests/family.rs`:

```rust
async fn set_phone(st: &grace_ride::state::AppState, email: &str, phone: &str) {
    sqlx::query("UPDATE users SET phone=$1 WHERE email=$2")
        .bind(phone).bind(email).execute(&st.db).await.unwrap();
}

#[tokio::test]
async fn matching_member_gets_a_request_and_a_second_family_is_blocked() {
    let (app, st) = setup().await;
    set_phone(&st, "member@example.org", "012-300 0006").await;
    set_phone(&st, "amina@example.org", "019-700 1001").await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let added = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "+60 19-700 1001"}))).await;
    assert_eq!(added.status, StatusCode::OK, "{}", added.json);
    assert_eq!(added.json["kind"], "request");
    assert_eq!(added.json["name"], "阿米娜");
    let (users_before,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(&st.db).await.unwrap();
    let again = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "0197001001"}))).await;
    assert_eq!(again.json["id"], added.json["id"]);
    let (notes,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE link='/me/family'").fetch_one(&st.db).await.unwrap();
    assert!(notes >= 1);
    let (users_after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(&st.db).await.unwrap();
    assert_eq!(users_before, users_after);

    let amina = login(&st, "amina@example.org").await;
    let incoming = call(&app, Method::GET, "/api/me/family", Some(&amina), None).await;
    assert!(incoming.json["family"].is_null());
    assert_eq!(incoming.json["incoming"]["family_name"], "陈家");
    let id = added.json["id"].as_i64().unwrap();
    let not_owner = call(&app, Method::POST, "/api/me/family/people", Some(&amina), Some(json!({"phone": "019-700 1099"}))).await;
    assert_eq!(not_owner.json["code"], "NO_PERMISSION");
    let accept = call(&app, Method::POST, &format!("/api/me/family/requests/{id}/accept"), Some(&amina), None).await;
    assert_eq!(accept.status, StatusCode::OK, "{}", accept.json);
    let joined = call(&app, Method::GET, "/api/me/family", Some(&amina), None).await;
    assert!(joined.json["incoming"].is_null());
    assert_eq!(joined.json["family"]["name"], "陈家");

    let owner2 = login(&st, "sophie@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner2), Some(json!({"name": "林家"}))).await;
    let blocked = call(&app, Method::POST, "/api/me/family/people", Some(&owner2), Some(json!({"phone": "019-700 1001"}))).await;
    assert_eq!(blocked.json["code"], "ALREADY_IN_FAMILY");
}

#[tokio::test]
async fn decline_does_not_join_and_other_family_request_conflicts() {
    let (app, st) = setup().await;
    set_phone(&st, "amina@example.org", "019-700 1002").await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let added = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 1002"}))).await;
    let amina = login(&st, "amina@example.org").await;
    let id = added.json["id"].as_i64().unwrap();
    let declined = call(&app, Method::POST, &format!("/api/me/family/requests/{id}/decline"), Some(&amina), None).await;
    assert_eq!(declined.status, StatusCode::OK, "{}", declined.json);
    let (members,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM family_members fm JOIN users u ON u.id=fm.user_id WHERE u.email='amina@example.org'")
        .fetch_one(&st.db).await.unwrap();
    assert_eq!(members, 0);

    let again = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 1002"}))).await;
    assert_eq!(again.status, StatusCode::OK, "{}", again.json);
    let owner2 = login(&st, "sophie@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner2), Some(json!({"name": "林家"}))).await;
    let race = call(&app, Method::POST, "/api/me/family/people", Some(&owner2), Some(json!({"phone": "019-700 1002"}))).await;
    assert_eq!(race.json["code"], "FAMILY_REQUEST_PENDING");
}

#[tokio::test]
async fn unknown_phone_returns_the_same_link_and_bad_numbers_do_not() {
    let (app, st) = setup().await;
    let before_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&st.db).await.unwrap();
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let link = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 2001"}))).await;
    assert_eq!(link.json["kind"], "invite");
    assert!(link.json["url"].as_str().unwrap().contains("/invite/"));
    let again = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "+60 19-700 2001"}))).await;
    assert_eq!(again.json["url"], link.json["url"]);
    let after_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&st.db).await.unwrap();
    assert_eq!(before_users, after_users);

    let own = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "012-300 0006"}))).await;
    assert_eq!(own.json["code"], "FAMILY_OWN_PHONE");
    let empty = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": ""}))).await;
    assert_eq!(empty.json["code"], "PHONE_INVALID");

    set_phone(&st, "amina@example.org", "019-700 1888").await;
    set_phone(&st, "sophie@example.org", "019-700 1888").await;
    let ambiguous = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 1888"}))).await;
    assert_eq!(ambiguous.json["code"], "FAMILY_PHONE_AMBIGUOUS");

    set_phone(&st, "yawen@example.org", "019-700 1999").await;
    sqlx::query("UPDATE users SET status='disabled' WHERE email='yawen@example.org'").execute(&st.db).await.unwrap();
    let disabled = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 1999"}))).await;
    assert_eq!(disabled.json["code"], "FAMILY_PHONE_UNAVAILABLE");
    let (invites,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM invites WHERE kind='family'").fetch_one(&st.db).await.unwrap();
    assert_eq!(invites, 1);
    let (detail,): (String,) = sqlx::query_as(
        "SELECT detail FROM audit_logs WHERE action='family.invite' ORDER BY id DESC LIMIT 1",
    ).fetch_one(&st.db).await.unwrap();
    assert!(!detail.contains("019-700"));
}

#[tokio::test]
async fn ninth_counted_person_is_full_and_groups_stay_put() {
    let (app, st) = setup().await;
    let before = groups(&st).await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let (fid,): (i64,) = sqlx::query_as("SELECT id FROM families").fetch_one(&st.db).await.unwrap();
    for i in 0..7 {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO users (name, email, phone, roles, status, webauthn_uuid) VALUES ($1,$2,'','','pending',$3) RETURNING id",
        )
        .bind(format!("Kin {i}"))
        .bind(format!("kin-full-{i}@example.org"))
        .bind(uuid::Uuid::new_v4().to_string())
        .fetch_one(&st.db).await.unwrap();
        sqlx::query("INSERT INTO family_members (family_id, user_id, created_at) VALUES ($1,$2,$3)")
            .bind(fid).bind(id).bind(grace_ride::util::now()).execute(&st.db).await.unwrap();
    }
    let full = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 3001"}))).await;
    assert_eq!(full.json["code"], "FAMILY_FULL");
    assert_eq!(full.json["params"]["max"], 8);
    assert_eq!(groups(&st).await, before);
}
```

Append this test. Sophie's unused link blocks the same number on another family, and the first owner still gets the original link back:

```rust
#[tokio::test]
async fn another_familys_open_link_blocks_the_number() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let link = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 2100"}))).await;
    assert_eq!(link.json["kind"], "invite");
    let sophie = login(&st, "sophie@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&sophie), Some(json!({"name": "林家"}))).await;
    let blocked = call(&app, Method::POST, "/api/me/family/people", Some(&sophie), Some(json!({"phone": "+60 19-700 2100"}))).await;
    assert_eq!(blocked.json["code"], "FAMILY_REQUEST_PENDING");
    let (invites,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM invites WHERE kind='family' AND used_at IS NULL")
        .fetch_one(&st.db).await.unwrap();
    assert_eq!(invites, 1);
    let again = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 2100"}))).await;
    assert_eq!(again.json["url"], link.json["url"]);
}
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test --test family matching_member_gets_a_request --manifest-path backend/Cargo.toml`

Expected: FAIL with 404 or `PHONE_INVALID` path missing.

- [ ] **Step 3: Implement add, accept, decline, cancel, and revoke**

Check order in `add_person`, first match wins:

1. Empty or `check_phone` fails → `PHONE_INVALID`.
2. Caller is not the owner → `NO_PERMISSION`. Owner is not `active` → `FAMILY_ACTIVE_ONLY`.
3. Key equals `phone_key` of the owner's stored phone, when that phone is non-empty → `FAMILY_OWN_PHONE`.
4. This family has an open `family_requests` row whose user's phone key matches, or an unused unexpired `kind='family'` invite whose phone key matches → return that request or link, even when the count is 8.
5. More than one `active` or `pending` user shares the key → `FAMILY_PHONE_AMBIGUOUS`.
6. The single matching user is already in `family_members` → `ALREADY_IN_FAMILY`.
7. That user has an open request, or another family has an unused unexpired invite for the key → `FAMILY_REQUEST_PENDING`. The other family's invite blocks the number when no account matches.
8. No active or pending user, and at least one `disabled` user has the key → `FAMILY_PHONE_UNAVAILABLE`.
9. `counted >= 8` → `FamilyFull { max: 8 }`. `counted` is members + this family's requests + this family's unused unexpired invites.
10. One active or pending user → insert `family_requests`, `notify` that user with link `/me/family`. Title `("家庭邀请", "Family request")`. Body includes the owner name and family name in both languages. Audit `family.request` with `family_id` and `user_id` only.
11. Nobody matches → `random_token(32)`, insert `invites` with `kind='family'`, empty email and name, `roles=''`, `group_ids='[]'`, `family_id`, the typed phone, `created_by`, `expires_at = in_days(3)`. Audit `family.invite` with `family_id` and `invite_id` only.

`GET` for the owner fills `requests` `{id, user_id, name}` and `invites` `{id, phone, expires_at, url}`. Unused means `used_at IS NULL` and `expires_at > now()`. A non-owner gets empty arrays. `incoming` is set only when the caller has a `family_requests` row and no `family_members` row.

Accept locks the request (`SELECT ... FOR UPDATE`). Caller must be `user_id`, else `NO_PERMISSION`. If they are already in a family, or the count excluding this request plus 1 exceeds 8, roll back and leave the request. Otherwise insert `family_members` and delete the request in that transaction. Audit `family.request.accept`. A missing request is `conflict(RequestAlreadyHandled)`.

Decline deletes the caller's request. Missing → `RequestAlreadyHandled`. Audit `family.request.decline`.

Cancel deletes the owner's request. Revoke expires the owner's unused invite. Audit `family.request.cancel` and `family.invite.revoke`.

Register routes:

```rust
.route("/me/family/people", post(family::add_person))
.route("/me/family/requests/{id}", delete(family::cancel_request))
.route("/me/family/requests/{id}/accept", post(family::accept_request))
.route("/me/family/requests/{id}/decline", post(family::decline_request))
.route("/me/family/invites/{id}", delete(family::revoke_invite))
```

- [ ] **Step 4: Run the family tests**

Run: `cargo test --test family --manifest-path backend/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/community/family.rs backend/src/routes/community/mod.rs backend/tests/family.rs
git commit -m "feat(family): add members by phone number"
```

---

### Task 5: Family invite preview and acceptance

**Files:**
- Modify: `backend/src/routes/auth/invites_recovery.rs` (`invite_preview`, `invite_accept`)
- Modify: `backend/tests/family.rs`
- Test: `backend/tests/family.rs`

**Interfaces:**
- Consumes: `INVITE_BY_TOKEN` (non-recovery kinds match the raw token). Existing accept insert into `users`. `notify_perm(Page::AdminMembers, Act::Update, ..., "/admin/members?status=pending")`. `TERMS_VERSION`.
- Produces: `GET /api/invites/{token}` for `kind=family` adds `family_name` and `phone`. `email` and `name` stay empty. No member list. Accept ignores the body phone and writes `invites.phone`.

- [ ] **Step 1: Write the failing tests**

Append:

```rust
#[tokio::test]
async fn family_link_accept_creates_a_pending_member_and_keeps_the_link_without_terms() {
    let (app, st) = setup().await;
    let before_notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE link='/admin/members?status=pending'")
        .fetch_one(&st.db).await.unwrap();
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let link = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 4001"}))).await;
    let url = link.json["url"].as_str().unwrap();
    let token = url.rsplit('/').next().unwrap();
    let preview = call(&app, Method::GET, &format!("/api/invites/{token}"), None, None).await;
    assert_eq!(preview.status, StatusCode::OK, "{}", preview.json);
    assert_eq!(preview.json["kind"], "family");
    assert_eq!(preview.json["family_name"], "陈家");
    assert_eq!(preview.json["phone"], "019-700 4001");
    assert!(preview.json.get("members").is_none());

    let no_terms = call(&app, Method::POST, &format!("/api/invites/{token}/accept"), None,
        Some(json!({"name": "新人", "email": "new-kin@example.org", "phone": "000", "terms": false}))).await;
    assert_eq!(no_terms.json["code"], "TERMS_REQUIRED");
    let (still,): (Option<String>,) = sqlx::query_as("SELECT used_at FROM invites WHERE kind='family'")
        .fetch_one(&st.db).await.unwrap();
    assert!(still.0.is_none());

    let accepted = call(&app, Method::POST, &format!("/api/invites/{token}/accept"), None,
        Some(json!({"name": "新人", "email": "new-kin@example.org", "phone": "000", "terms": true}))).await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.json);
    let (status, phone, roles): (String, String, String) = sqlx::query_as(
        "SELECT status, phone, roles FROM users WHERE email='new-kin@example.org'",
    ).fetch_one(&st.db).await.unwrap();
    assert_eq!(status, "pending");
    assert_eq!(phone, "019-700 4001");
    assert_eq!(roles, "");
    let (church,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM group_members gm JOIN users u ON u.id=gm.user_id WHERE u.email='new-kin@example.org'",
    ).fetch_one(&st.db).await.unwrap();
    assert_eq!(church, 0);
    let view = call(&app, Method::GET, "/api/me/family", Some(&owner), None).await;
    let names: Vec<_> = view.json["members"].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"新人"));
    let after_notes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE link='/admin/members?status=pending'")
        .fetch_one(&st.db).await.unwrap();
    assert!(after_notes > before_notes);
}

#[tokio::test]
async fn accepting_after_delete_does_not_create_a_user() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let link = call(&app, Method::POST, "/api/me/family/people", Some(&owner), Some(json!({"phone": "019-700 4002"}))).await;
    let token = link.json["url"].as_str().unwrap().rsplit('/').next().unwrap().to_string();
    call(&app, Method::DELETE, "/api/me/family", Some(&owner), None).await;
    let accepted = call(&app, Method::POST, &format!("/api/invites/{token}/accept"), None,
        Some(json!({"name": "新人", "email": "ghost-kin@example.org", "terms": true}))).await;
    assert_eq!(accepted.status, StatusCode::NOT_FOUND);
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email='ghost-kin@example.org'")
        .fetch_one(&st.db).await.unwrap();
    assert_eq!(n, 0);
}
```

`POST` accept from `call` sends `Origin`, which `origin_guard` requires. `GET` preview does not need a cookie.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test family family_link_accept --manifest-path backend/Cargo.toml`

Expected: FAIL. Preview has no `family_name`, or accept stores phone `"000"` / rejects it with `PHONE_INVALID`.

- [ ] **Step 3: Teach preview and accept about `kind = 'family'`**

Preview select adds `i.phone` and `f.name` via `LEFT JOIN families f ON f.id = i.family_id`. When `kind == "family"`, the JSON includes `family_name` and `phone`. Other kinds stay `{kind, email, name}` as they are today.

Accept select also loads `phone` and `family_id`. When `kind == "family"`:

- Do not call `check_phone` on the body phone.
- Still require name, email, and `terms == Some(true)`.
- Inside the existing transaction, after the invite is marked unused-and-updated and before commit: lock the family (`SELECT id FROM families WHERE id=$1 FOR UPDATE`). If it is gone, return `not_found` and roll back. Count members + open requests + unused unexpired invites except this invite, plus 1. If that total is over 8, return `FamilyFull { max: 8 }` and roll back so `used_at` stays null.
- Insert the user with the invite's phone, `roles=''`, `status='pending'`. Insert `family_members`.
- The existing `kind != "recovery"` admin `notify_perm` already covers this. Do not add a second one.
- `invite.accept` audit stays. Do not put the phone in its detail.

A body phone of `"000"` must not be stored and must not burn the link.

- [ ] **Step 4: Run the family tests**

Run: `cargo test --test family --manifest-path backend/Cargo.toml`

Expected: PASS. Then run the existing invite tests:

```bash
cargo test --test gaps invite_preview_accept_expired_used_invalid --manifest-path backend/Cargo.toml
cargo test --test admin_ux invite_signup_notifies_approvers_and_approval_notifies_member --manifest-path backend/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/auth/invites_recovery.rs backend/tests/family.rs
git commit -m "feat(family): open a pending account from a family link"
```

---

### Task 6: Show the family name to member admins

**Files:**
- Modify: `backend/src/routes/admin/members.rs` (`list_users` select and `to_json`)
- Modify: `frontend/src/pages/admin/Members.tsx` (`User` and the approve dialog)
- Modify: `frontend/src/locales/admin.en.ts` and `frontend/src/locales/admin.zh.ts` (`members.family`)
- Modify: `backend/tests/family.rs`
- Test: `backend/tests/family.rs`

**Interfaces:**
- Consumes: `list_users` staff flag (`Page::AdminMembers`, `Act::View`). `families.name` through `family_members`.
- Produces: staff user JSON gains `family_name: string | null`. Non-staff JSON stays `{id, name}` with no `family_name`. The approve dialog shows that name as text. It is not an input and it is not sent in `PUT /admin/users/{id}`.

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn family_name_is_visible_only_to_member_staff() {
    let (app, st) = setup().await;
    let owner = login(&st, "member@example.org").await;
    call(&app, Method::POST, "/api/me/family", Some(&owner), Some(json!({"name": "陈家"}))).await;
    let staff = login(&st, "admin@example.org").await;
    let listed = call(&app, Method::GET, "/api/admin/users?page=1&limit=50", Some(&staff), None).await;
    assert_eq!(listed.status, StatusCode::OK, "{}", listed.json);
    let row = listed.json["items"].as_array().unwrap().iter().find(|u| u["id"] == 6).unwrap();
    assert_eq!(row["email"], "member@example.org");
    assert_eq!(row["family_name"], "陈家");

    let leader = login(&st, "enci@example.org").await;
    let picker = call(&app, Method::GET, "/api/admin/users?page=1&limit=50", Some(&leader), None).await;
    assert_eq!(picker.status, StatusCode::OK, "{}", picker.json);
    let item = picker.json["items"].as_array().unwrap().iter().find(|u| u["id"] == 6).unwrap();
    assert!(item.get("family_name").is_none());
    assert!(item.get("email").is_none());
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --test family family_name_is_visible_only_to_member_staff --manifest-path backend/Cargo.toml`

Expected: FAIL because `family_name` is absent on the staff row.

- [ ] **Step 3: Add the column and the read-only line**

Extend the staff `Row` tuple and both the paged and unpaged selects with:

```sql
(SELECT f.name FROM family_members fm JOIN families f ON f.id = fm.family_id WHERE fm.user_id = u.id)
```

Put `family_name` on the staff JSON only. Leave the non-staff `json!({"id", "name"})` unchanged.

On `User`, add `family_name?: string | null`. In the approve/edit modal, when `user.family_name` is non-empty, render a muted line under the email: the locale string `members.family` plus the name. English `Family`. Chinese `家庭`. Do not add a control and do not include it in the `put` body.

- [ ] **Step 4: Run the test**

Run: `cargo test --test family family_name_is_visible_only_to_member_staff --manifest-path backend/Cargo.toml`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/admin/members.rs backend/tests/family.rs frontend/src/pages/admin/Members.tsx frontend/src/locales/admin.en.ts frontend/src/locales/admin.zh.ts
git commit -m "feat(family): show family name on the member list"
```

---

### Task 7: Profile family screen and invite page

**Files:**
- Create: `frontend/src/pages/family/FamilySection.tsx`
- Modify: `frontend/src/pages/Profile.tsx`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/components/Layout.tsx`
- Modify: `frontend/src/pages/Invite.tsx`
- Modify: `frontend/src/locales/profile.en.ts`, `profile.zh.ts`, `invite.en.ts`, `invite.zh.ts`
- Modify: `frontend/src/pages/Profile.test.tsx`
- Modify: `frontend/src/auth_public.test.tsx`
- Test: those two Vitest files

**Interfaces:**
- Consumes: `GET/POST/PATCH/DELETE /api/me/family` and the people, leave, member, request, and invite routes from Tasks 3–5. Preview fields `kind`, `family_name`, `phone`.
- Produces: `/me/family` opens the family section. The hub row is labelled Family / 家庭. Notification link `/me/family` is a real route. A pending member is not redirected away from it.

- [ ] **Step 1: Write the failing UI tests**

In `Profile.test.tsx`, make the `useApi` mock path-aware. A `let familyData` defaults to `{ family: null, members: [], requests: [], invites: [], incoming: null }`. When the path is `/me/family`, return that object. Add profile strings used by the new section (`family`, `familyName`, `createFamily`, `addPhone`, `leaveFamily`, `waitingApproval`, `copyLink`).

Tests:

- Active member, empty family: click the Family row, see a name field and a create button. No member phone text.
- Set `familyData` to a family owned by `baseMe.id` with members `[{user_id: 1, name: 'Grace Member', status: 'active'}, {user_id: 2, name: 'Kin Pending', status: 'pending'}]` and one invite `{id: 3, phone: '019-700 2001', expires_at: '2026-11-22T00:00:00', url: 'http://localhost/invite/abc'}`. Render at `/me/family`. See both names. See `Waiting for approval` on Kin. See the invite phone. The member rows do not contain `019-700 2001`.
- `currentMe.status = 'pending'` and `familyData.family = null`: the create button is absent.

In `auth_public.test.tsx`, add a test that sets `inviteData` to `{ kind: 'family', email: '', name: '', family_name: '陈家', phone: '019-700 4001' }`. The phone input's value is `019-700 4001` and it is disabled or read-only. Name and email inputs are empty. The terms checkbox is unchecked. The heading or text includes `陈家`.

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
npm test -- src/pages/Profile.test.tsx src/auth_public.test.tsx
```

from `frontend/`. Expected: FAIL. The Family row or the locked phone is missing.

- [ ] **Step 3: Build the screens**

`FamilySection.tsx` loads `/me/family`.

- Active, no family, no incoming request: a name field and Create (`POST /api/me/family`).
- Pending, no family, no request: one sentence that they can join a family after someone invites them, and no create form.
- Incoming request: family name, owner name, Accept and Decline.
- In a family: the name and member names. `status === 'pending'` shows a warning pill `Waiting for approval` / `等待审核`. Owner sees a rename field, an add-phone field, remove on other members, cancel on requests, and copy plus cancel on invites. Copy writes `url` with `navigator.clipboard.writeText`. Delete family uses `useConfirm`. Non-owners see Leave only. The owner does not see Leave.

`Profile.tsx`: add `'family'` handling by pathname. `useLocation().pathname === '/me/family'` opens the family section with `PageHeader` back `/me`. Add a hub `SheetLink` that navigates to `/me/family` for every signed-in member, including pending. Do not put the create form on the hub.

`App.tsx`: `<Route path="me/family" element={page(<Profile />)} />` next to the `me` route.

`Layout.tsx` line that redirects pending users: allow `/me/family` as well as `/me`.

```tsx
const onMe = loc.pathname === '/me' || loc.pathname === '/me/family';
if (pending && !onMe) return <Navigate to="/me" replace />;
```

`Invite.tsx`: extend the preview type with `family_name?: string` and `phone?: string`. When `kind === 'family'`, set the phone state from `data.phone`, render the family name under the title, and set the phone input `readOnly`. Do not change the terms checkbox initial state. Name and email stay empty. Submit still sends `phone` along with name, email, and terms; the server ignores that phone.

Locale keys in both profile languages and both invite languages. No ride-sharing copy.

- [ ] **Step 4: Run the UI tests**

Run: `npm test -- src/pages/Profile.test.tsx src/auth_public.test.tsx` from `frontend/`.

Expected: PASS.

- [ ] **Step 5: Check the path in the browser**

Start the app the way this repo already starts it. Sign in as the demo member `member@example.org`. Create a family, add `019-700 5001`, copy the link, open it in a private window, accept with a new name and email and the terms box, and confirm the pending name is on the family list. Confirm a ride page is still blocked for that new account.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/pages/family/FamilySection.tsx frontend/src/pages/Profile.tsx frontend/src/pages/Profile.test.tsx frontend/src/pages/Invite.tsx frontend/src/auth_public.test.tsx frontend/src/App.tsx frontend/src/components/Layout.tsx frontend/src/locales/profile.en.ts frontend/src/locales/profile.zh.ts frontend/src/locales/invite.en.ts frontend/src/locales/invite.zh.ts
git commit -m "feat(family): add the profile family screen and locked invite phone"
```

---

### Task 8: Manuals

**Files:**
- Modify: `docs/TECHNICAL.md`
- Modify: `docs/USER_MANUAL.md`

**Interfaces:**
- Consumes: the behavior from Tasks 1–7.
- Produces: the three technical notes and the member paragraph required by the spec. No ride-sharing description.

- [ ] **Step 1: Update the manuals**

In `docs/TECHNICAL.md`:

- Add a Family row to the module inventory. Routes `/me/family` and `/invite/:token`. Behavior: one named list per member, phone request or 3-day link, pending account, cap 8, no ride or church-group effect. Source: `backend/src/routes/community/family.rs`, `frontend/src/pages/family/FamilySection.tsx`.
- In Membership & Account Access, add that a family invite is a second way onto the pending list, and that `/api/me/family` is allowed before approval.
- Add migration `0015_families.sql` to the migration table: `families`, `family_members`, `family_requests`, `invites.family_id`, `invites.phone`, `chk_invites_family`.
- The migration list currently stops at `0014_avatar.sql` and the code-ownership line still says `0002`–`0009`. Extend the ownership line's range only far enough to include `0015`. Do not rewrite unrelated rows.

In `docs/USER_MANUAL.md`, after the profile language steps, add a short "我的家庭" paragraph: an approved member creates a family from Profile, types a phone number, copies the link and sends it, and the new person waits for an admin to approve them. Do not say relatives can see or book each other's rides.

- [ ] **Step 2: Commit**

```bash
git add docs/TECHNICAL.md docs/USER_MANUAL.md
git commit -m "docs(family): describe family lists in the manuals"
```

---

## Spec coverage

| Spec section | Task |
|---|---|
| Profile behavior, owner rules, pending leave | Tasks 3 and 7 |
| Phone key and the 11-step check order | Tasks 1 and 4 |
| Invite preview, locked phone, terms, pending user, admin notify, delete kills the link | Task 5 |
| Tables, kind check, cap, cascade | Task 2 |
| API routes and audit names | Tasks 3–5 |
| `family_name` staff-only | Task 6 |
| Error catalog, both languages, `{max}` | Task 1 |
| Privacy (no phone on member rows, audit without phone) | Tasks 3, 4, 5, 7 |
| Manuals | Task 8 |
| API tests listed in the spec | Tasks 3–6 |
| Frontend tests and browser pass | Task 7 |
| Out of scope (SMS, ride sharing, church groups, proxy riders) | Not implemented |
