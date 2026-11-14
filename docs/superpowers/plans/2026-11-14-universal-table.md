# Universal Table Sort & Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reusable sorting and filter-toolbar support to `UniversalTable` and wire allow-listed server-side sorting/filtering into the Members, Rides, and Reimbursements admin tables.

**Architecture:** `UniversalTable` gains a controlled sort/filter API plus a local-sort fallback. A new `useTableParams` hook owns URL state and page resets. Backend list endpoints accept allow-listed `sort`/`dir` params and reuse existing filter params; a shared `order_by` helper keeps SQL identifiers out of client control. Audit keeps its keyset newest-first behavior unchanged.

**Tech Stack:** React 18, TypeScript, React Router `useSearchParams`, Vitest/Testing Library, Axum, SQLx/PostgreSQL.

## Global Constraints

- No new runtime dependencies.
- Never interpolate client-provided `sort` or `dir` values into SQL; only allow-listed SQL expressions and `ASC`/`DESC`.
- Preserve existing page filters and behavior unless this plan explicitly moves them into the shared toolbar.
- Do not add sorting to Audit; it keeps `before=id` keyset infinite scroll.
- Do not change the Roles permission matrix.
- Keep URL writes on `{ replace: true }`.
- Run commands from the repository root unless a step says otherwise.
- Backend tests require the same PostgreSQL environment as the existing `backend/tests/*` suite.
- **Start from a clean working tree.** Commit or stash unrelated changes before Task 1. Each task's `git add` stages whole files, so unrelated local edits in the same files would be committed with the feature. Never use `git add -A`.

---

### Task 1: Backend allow-listed `order_by` helper

**Files:**
- Modify: `backend/src/util.rs` (append at the bottom)

**Interfaces:**
- Produces: `pub fn order_by(sort: Option<&str>, dir: Option<&str>, allowed: &[(&str, &str)], default: &str, tiebreak: &str) -> String`
- The returned string is an SQL `ORDER BY` body, not a full clause. `default` must already include its tie-breaker; a custom sort gets `NULLS LAST, {tiebreak}` appended.

- [ ] **Step 1: Write the failing unit tests**

Append this test module to `backend/src/util.rs`:

```rust
#[cfg(test)]
mod order_by_tests {
    use super::order_by;

    const ALLOWED: &[(&str, &str)] = &[("name", "u.name"), ("created", "u.created_at")];

    #[test]
    fn uses_allow_listed_column_and_direction() {
        assert_eq!(
            order_by(Some("name"), Some("desc"), ALLOWED, "u.name ASC", "u.id ASC"),
            "u.name DESC NULLS LAST, u.id ASC"
        );
        assert_eq!(
            order_by(Some("name"), None, ALLOWED, "u.name ASC", "u.id ASC"),
            "u.name ASC NULLS LAST, u.id ASC"
        );
    }

    #[test]
    fn falls_back_for_unknown_sort_and_direction() {
        assert_eq!(
            order_by(
                Some("name; DROP TABLE users"),
                Some("desc"),
                ALLOWED,
                "u.name ASC, u.id ASC",
                "u.id ASC"
            ),
            "u.name ASC, u.id ASC"
        );
        assert_eq!(
            order_by(Some("created"), Some("sideways"), ALLOWED, "u.name ASC", "u.id ASC"),
            "u.created_at ASC NULLS LAST, u.id ASC"
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cd backend && cargo test order_by
```

Expected: FAIL with `cannot find function order_by` or `unresolved import`.

- [ ] **Step 3: Implement the helper**

Append this function above the test module in `backend/src/util.rs`:

```rust
/// Build an `ORDER BY` body from an allow-list of `(client_key, SQL expression)`.
/// Unknown keys use `default`; any direction other than `desc` sorts ascending.
/// `tiebreak` keeps paginated results stable when the sort value repeats.
pub fn order_by(
    sort: Option<&str>,
    dir: Option<&str>,
    allowed: &[(&str, &str)],
    default: &str,
    tiebreak: &str,
) -> String {
    let Some((_, column)) = sort
        .map(str::trim)
        .and_then(|key| allowed.iter().find(|(name, _)| *name == key))
    else {
        return default.to_string();
    };
    let direction = if dir
        .map(str::trim)
        .is_some_and(|d| d.eq_ignore_ascii_case("desc"))
    {
        "DESC"
    } else {
        "ASC"
    };
    format!("{column} {direction} NULLS LAST, {tiebreak}")
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
cd backend && cargo test order_by
```

Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add backend/src/util.rs
git commit -m "feat(backend): add allow-listed order_by helper"
```

---

### Task 2: Allow-listed sorting for `GET /admin/users`

**Files:**
- Modify: `backend/src/routes/admin.rs:114-242`
- Test: `backend/tests/users_sort.rs` (new)

**Interfaces:**
- Consumes: `crate::util::order_by(...)` from Task 1.
- Produces: `GET /api/admin/users?...&sort=name|email|status|created_at&dir=asc|desc`.
- Default order remains `u.name ASC, u.id ASC`.

- [ ] **Step 1: Write the failing integration test**

Create `backend/tests/users_sort.rs`:

```rust
mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::Value;

fn names(body: &Value) -> Vec<String> {
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn admin_users_sort_uses_allow_listed_fields() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;

    for (name, email) in [
        ("AAA Sort", "aaa-sort@example.org"),
        ("BBB Sort", "bbb-sort@example.org"),
        ("CCC Sort", "ccc-sort@example.org"),
    ] {
        sqlx::query("INSERT INTO users (name, email, webauthn_uuid) VALUES ($1,$2,$3)")
            .bind(name)
            .bind(email)
            .bind(format!("uuid-{email}"))
            .execute(&st.db)
            .await
            .unwrap();
    }

    let asc = call(
        &app,
        Method::GET,
        "/api/admin/users?q=Sort&page=1&limit=50&sort=name&dir=asc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(asc.status, StatusCode::OK, "{}", asc.json);
    assert_eq!(names(&asc.json), ["AAA Sort", "BBB Sort", "CCC Sort"]);

    let desc = call(
        &app,
        Method::GET,
        "/api/admin/users?q=Sort&page=1&limit=50&sort=name&dir=desc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(desc.status, StatusCode::OK, "{}", desc.json);
    assert_eq!(names(&desc.json), ["CCC Sort", "BBB Sort", "AAA Sort"]);

    let injected = call(
        &app,
        Method::GET,
        "/api/admin/users?q=Sort&page=1&limit=50&sort=name%3B+DROP+TABLE+users&dir=sideways",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(injected.status, StatusCode::OK, "{}", injected.json);
    assert_eq!(names(&injected.json), ["AAA Sort", "BBB Sort", "CCC Sort"]);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cd backend && cargo test --test users_sort
```

Expected: FAIL — the `desc` response is still ascending because `sort`/`dir` are ignored.

- [ ] **Step 3: Add the query fields and order selection**

In `backend/src/routes/admin.rs`, extend `UserQuery`:

```rust
#[derive(Deserialize)]
struct UserQuery {
    q: Option<String>,
    role: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}
```

In `list_users`, immediately after `let like = ...;`, add:

```rust
    let order = crate::util::order_by(
        q.sort.as_deref(),
        q.dir.as_deref(),
        &[
            ("name", "u.name"),
            ("email", "u.email"),
            ("status", "u.status"),
            ("created_at", "u.created_at"),
        ],
        "u.name ASC, u.id ASC",
        "u.id ASC",
    );
```

Replace the paginated SQL's fixed order:

```rust
        let sql = format!(
            "SELECT u.id, u.name, u.email, u.phone, u.roles, u.status, u.created_at, (SELECT COUNT(*) FROM passkeys p WHERE p.user_id=u.id),
                (SELECT string_agg(g.name, '、') FROM group_members gm JOIN groups g ON g.id=gm.group_id WHERE gm.user_id=u.id)
             FROM users u WHERE {} ORDER BY {order} LIMIT ${n} OFFSET ${}",
            conds.join(" AND "),
            n + 1
        );
```

Replace the legacy SQL's fixed order:

```rust
    let rows: Vec<(i64, String, String, String, String, String, String, i64, Option<String>)> = sqlx::query_as(
        &format!(
            "SELECT u.id, u.name, u.email, u.phone, u.roles, u.status, u.created_at, (SELECT COUNT(*) FROM passkeys p WHERE p.user_id=u.id),
                (SELECT string_agg(g.name, '、') FROM group_members gm JOIN groups g ON g.id=gm.group_id WHERE gm.user_id=u.id)
             FROM users u WHERE (u.name ILIKE $1 OR u.email ILIKE $2 OR u.phone ILIKE $3) ORDER BY {order} LIMIT 500"
        ),
    )
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .fetch_all(&st.db)
    .await?;
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cd backend && cargo test --test users_sort
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/admin.rs backend/tests/users_sort.rs
git commit -m "feat(backend): allow-listed member sorting"
```

---

### Task 3: Allow-listed sorting for `GET /admin/rides`

**Files:**
- Modify: `backend/src/routes/rides.rs:2418-2509`
- Test: `backend/tests/rides_sort.rs` (new)

**Interfaces:**
- Consumes: `crate::util::order_by(...)` from Task 1.
- Produces: `GET /api/admin/rides?...&sort=pickup_at|passenger|area|driver|status&dir=asc|desc`.
- Default order remains `r.pickup_at ASC, r.id ASC`.

- [ ] **Step 1: Write the failing integration test**

Create `backend/tests/rides_sort.rs`:

```rust
mod common;

use axum::http::{Method, StatusCode};
use common::*;

fn pickup_times(body: &serde_json::Value) -> Vec<String> {
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["pickup_at"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn admin_rides_sort_uses_allow_listed_fields() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;

    let desc = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=pickup_at&dir=desc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(desc.status, StatusCode::OK, "{}", desc.json);
    let times = pickup_times(&desc.json);
    assert!(times.windows(2).all(|w| w[0] >= w[1]), "{times:?}");

    let asc = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=pickup_at&dir=asc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(asc.status, StatusCode::OK, "{}", asc.json);
    let times = pickup_times(&asc.json);
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{times:?}");

    let injected = call(
        &app,
        Method::GET,
        "/api/admin/rides?page=1&limit=200&sort=passenger%3B+DROP+TABLE+rides&dir=sideways",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(injected.status, StatusCode::OK, "{}", injected.json);
    let times = pickup_times(&injected.json);
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{times:?}");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cd backend && cargo test --test rides_sort
```

Expected: FAIL — the `desc` response is still ascending because `sort`/`dir` are ignored.

- [ ] **Step 3: Add the query fields and order selection**

Extend `AdminRidesQuery`:

```rust
#[derive(Deserialize)]
struct AdminRidesQuery {
    date: Option<String>,
    status: Option<String>,
    attention: Option<bool>,
    sort: Option<String>,
    dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}
```

In `admin_rides`, after `require(&u, Page::AdminRides, Act::View)?;`, add:

```rust
    let order = crate::util::order_by(
        q.sort.as_deref(),
        q.dir.as_deref(),
        &[
            ("pickup_at", "r.pickup_at"),
            ("passenger", "p.name"),
            ("area", "a.name"),
            ("driver", "d.name"),
            ("status", "r.status"),
        ],
        "r.pickup_at ASC, r.id ASC",
        "r.id ASC",
    );
```

Replace the paginated SQL order:

```rust
        let sql = format!(
            "{RIDE_SELECT} WHERE {} ORDER BY {order} LIMIT ${n} OFFSET ${}",
            conds.join(" AND "),
            n + 1
        );
```

Replace the legacy SQL order:

```rust
    let sql = format!(
        "{RIDE_SELECT} WHERE {} ORDER BY {order} LIMIT 300",
        conds.join(" AND ")
    );
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cd backend && cargo test --test rides_sort
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/rides.rs backend/tests/rides_sort.rs
git commit -m "feat(backend): allow-listed ride sorting"
```

### Task 4: Sorting and filters for `GET /reimbursements`

**Files:**
- Modify: `backend/src/routes/community.rs:855-975`
- Test: `backend/tests/reimbursements_sort.rs` (new)

**Interfaces:**
- Consumes: `crate::util::order_by(...)` from Task 1.
- Produces: `GET /api/reimbursements?scope=review|payout&q=...&status=...&sort=created_at|amount_cents|applicant|status|id&dir=asc|desc`.
- Review default remains pending-first then `r.id DESC`; payout default remains approved-first then `r.id DESC`; own-claims default remains `r.id DESC`.

- [ ] **Step 1: Write the failing integration test**

Create `backend/tests/reimbursements_sort.rs`:

```rust
mod common;

use axum::http::{Method, StatusCode};
use common::*;

#[tokio::test]
async fn reimbursement_review_sorts_and_filters() {
    let (app, st) = setup().await;
    let root = login(&st, "superadmin@example.org").await;
    let receipt = upload_png(&app, &root).await;
    let member: i64 = sqlx::query_scalar("SELECT id FROM users WHERE email='member@example.org'")
        .fetch_one(&st.db)
        .await
        .unwrap();

    for (amount, description, status, created) in [
        (1000, "Needle low", "pending", "2026-01-01T10:00:00"),
        (3000, "Needle high", "approved", "2026-01-02T10:00:00"),
        (2000, "Other", "pending", "2026-01-03T10:00:00"),
    ] {
        sqlx::query(
            "INSERT INTO reimbursements
             (user_id, amount_cents, description, receipt_file_id, payee_name, status, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$7)",
        )
        .bind(member)
        .bind(amount)
        .bind(description)
        .bind(receipt)
        .bind("Payee")
        .bind(status)
        .bind(created)
        .execute(&st.db)
        .await
        .unwrap();
    }

    let sorted = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100&sort=amount_cents&dir=desc",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(sorted.status, StatusCode::OK, "{}", sorted.json);
    let amounts: Vec<i64> = sorted.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["amount_cents"].as_i64().unwrap())
        .collect();
    assert!(amounts.windows(2).all(|w| w[0] >= w[1]), "{amounts:?}");

    let searched = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100&q=Needle",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(searched.status, StatusCode::OK, "{}", searched.json);
    assert_eq!(searched.json["total"].as_i64(), Some(2));
    assert!(searched.json["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["description"].as_str().unwrap().contains("Needle")));

    let filtered = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100&status=approved",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(filtered.status, StatusCode::OK, "{}", filtered.json);
    assert_eq!(filtered.json["total"].as_i64(), Some(1));
    assert_eq!(filtered.json["items"][0]["status"], "approved");

    let default_order = call(
        &app,
        Method::GET,
        "/api/reimbursements?scope=review&page=1&limit=100",
        Some(&root),
        None,
    )
    .await;
    assert_eq!(default_order.status, StatusCode::OK, "{}", default_order.json);
    assert_eq!(default_order.json["items"][0]["status"], "pending");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cd backend && cargo test --test reimbursements_sort
```

Expected: FAIL — `q` and `status` are ignored, and `sort`/`dir` do not reorder the rows.

- [ ] **Step 3: Extend the query and add allow-listed filters/sorts**

Replace the existing `ReimbQuery` with:

```rust
#[derive(Deserialize)]
struct ReimbQuery {
    scope: Option<String>, // mine | review | payout
    q: Option<String>,
    status: Option<String>,
    sort: Option<String>,
    dir: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}

const REIMB_SORTS: &[(&str, &str)] = &[
    ("created_at", "r.created_at"),
    ("amount_cents", "r.amount_cents"),
    ("applicant", "u.name"),
    (
        "status",
        "CASE r.status WHEN 'pending' THEN 0 WHEN 'returned' THEN 1 WHEN 'approved' THEN 2 WHEN 'paid' THEN 3 ELSE 4 END",
    ),
    ("id", "r.id"),
];

fn reimb_like(q: &Option<String>) -> Option<String> {
    q.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            format!(
                "%{}%",
                s.replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        })
}

fn reimb_filter_parts(
    search: Option<&str>,
    status: Option<&str>,
    start: usize,
) -> (Vec<String>, Vec<String>, usize) {
    let mut parts = Vec::new();
    let mut binds = Vec::new();
    let mut n = start;
    if let Some(search) = search {
        parts.push(format!(
            "(u.name ILIKE ${n} OR r.payee_name ILIKE ${n} OR r.description ILIKE ${n})"
        ));
        binds.push(search.to_string());
        n += 1;
    }
    if let Some(status) = status {
        parts.push(format!("r.status = ${n}"));
        binds.push(status.to_string());
        n += 1;
    }
    (parts, binds, n)
}
```

Replace `list_reimbursements` with:

```rust
async fn list_reimbursements(
    State(st): State<AppState>,
    u: CurrentUser,
    Query(q): Query<ReimbQuery>,
) -> AppResult<Json<Value>> {
    let ReimbQuery {
        scope,
        q,
        status,
        sort,
        dir,
        page,
        limit,
    } = q;
    let is_paginated = page.is_some() || limit.is_some();
    let page_val = page.unwrap_or(1).max(1);
    let limit_val = limit.unwrap_or(25).clamp(1, 100);
    let offset = (page_val - 1) * limit_val;
    let search = reimb_like(&q);
    let status = status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    match scope.as_deref() {
        Some("review") => {
            require(&u, Page::AdminReimbReview, Act::View)?;
            let order = crate::util::order_by(
                sort.as_deref(),
                dir.as_deref(),
                REIMB_SORTS,
                "CASE r.status WHEN 'pending' THEN 0 ELSE 1 END, r.id DESC",
                "r.id DESC",
            );
            let (parts, binds, n) =
                reimb_filter_parts(search.as_deref(), status.as_deref(), 1);
            let where_sql = if parts.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", parts.join(" AND "))
            };
            if is_paginated {
                let count_sql = format!(
                    "SELECT COUNT(*) FROM reimbursements r JOIN users u ON u.id=r.user_id{where_sql}"
                );
                let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
                for b in &binds {
                    count_query = count_query.bind(b.as_str());
                }
                let (total,) = count_query.fetch_one(&st.db).await?;
                let sql = format!(
                    "{REIMB_SELECT}{where_sql} ORDER BY {order} LIMIT ${n} OFFSET ${}",
                    n + 1
                );
                let mut query = sqlx::query_as::<_, ReimbRow>(&sql);
                for b in &binds {
                    query = query.bind(b.as_str());
                }
                let rows = query.bind(limit_val).bind(offset).fetch_all(&st.db).await?;
                let total_pages = ((total as f64) / (limit_val as f64)).ceil() as i64;
                Ok(Json(json!({
                    "items": rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>(),
                    "total": total,
                    "page": page_val,
                    "page_size": limit_val,
                    "total_pages": total_pages.max(1),
                })))
            } else {
                let sql = format!("{REIMB_SELECT}{where_sql} ORDER BY {order} LIMIT 200");
                let mut query = sqlx::query_as::<_, ReimbRow>(&sql);
                for b in &binds {
                    query = query.bind(b.as_str());
                }
                let rows = query.fetch_all(&st.db).await?;
                Ok(Json(json!(rows
                    .iter()
                    .map(|r| reimb_json(r, &u))
                    .collect::<Vec<_>>())))
            }
        }
        Some("payout") => {
            require(&u, Page::AdminReimbPay, Act::View)?;
            let order = crate::util::order_by(
                sort.as_deref(),
                dir.as_deref(),
                REIMB_SORTS,
                "CASE r.status WHEN 'approved' THEN 0 ELSE 1 END, r.id DESC",
                "r.id DESC",
            );
            let (parts, binds, n) =
                reimb_filter_parts(search.as_deref(), status.as_deref(), 1);
            let mut where_parts = vec!["r.status IN ('approved','paid')".to_string()];
            where_parts.extend(parts);
            let where_sql = format!(" WHERE {}", where_parts.join(" AND "));
            if is_paginated {
                let count_sql = format!(
                    "SELECT COUNT(*) FROM reimbursements r JOIN users u ON u.id=r.user_id{where_sql}"
                );
                let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
                for b in &binds {
                    count_query = count_query.bind(b.as_str());
                }
                let (total,) = count_query.fetch_one(&st.db).await?;
                let sql = format!(
                    "{REIMB_SELECT}{where_sql} ORDER BY {order} LIMIT ${n} OFFSET ${}",
                    n + 1
                );
                let mut query = sqlx::query_as::<_, ReimbRow>(&sql);
                for b in &binds {
                    query = query.bind(b.as_str());
                }
                let rows = query.bind(limit_val).bind(offset).fetch_all(&st.db).await?;
                let total_pages = ((total as f64) / (limit_val as f64)).ceil() as i64;
                Ok(Json(json!({
                    "items": rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>(),
                    "total": total,
                    "page": page_val,
                    "page_size": limit_val,
                    "total_pages": total_pages.max(1),
                })))
            } else {
                let sql = format!("{REIMB_SELECT}{where_sql} ORDER BY {order} LIMIT 200");
                let mut query = sqlx::query_as::<_, ReimbRow>(&sql);
                for b in &binds {
                    query = query.bind(b.as_str());
                }
                let rows = query.fetch_all(&st.db).await?;
                Ok(Json(json!(rows
                    .iter()
                    .map(|r| reimb_json(r, &u))
                    .collect::<Vec<_>>())))
            }
        }
        _ => {
            require(&u, Page::Reimbursements, Act::View)?;
            let order = crate::util::order_by(
                sort.as_deref(),
                dir.as_deref(),
                REIMB_SORTS,
                "r.id DESC",
                "r.id DESC",
            );
            let (parts, binds, n) =
                reimb_filter_parts(search.as_deref(), status.as_deref(), 2);
            let mut where_parts = vec!["r.user_id = $1".to_string()];
            where_parts.extend(parts);
            let where_sql = format!(" WHERE {}", where_parts.join(" AND "));
            if is_paginated {
                let count_sql = format!(
                    "SELECT COUNT(*) FROM reimbursements r JOIN users u ON u.id=r.user_id{where_sql}"
                );
                let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql).bind(u.id);
                for b in &binds {
                    count_query = count_query.bind(b.as_str());
                }
                let (total,) = count_query.fetch_one(&st.db).await?;
                let sql = format!(
                    "{REIMB_SELECT}{where_sql} ORDER BY {order} LIMIT ${n} OFFSET ${}",
                    n + 1
                );
                let mut query = sqlx::query_as::<_, ReimbRow>(&sql).bind(u.id);
                for b in &binds {
                    query = query.bind(b.as_str());
                }
                let rows = query.bind(limit_val).bind(offset).fetch_all(&st.db).await?;
                let total_pages = ((total as f64) / (limit_val as f64)).ceil() as i64;
                Ok(Json(json!({
                    "items": rows.iter().map(|r| reimb_json(r, &u)).collect::<Vec<_>>(),
                    "total": total,
                    "page": page_val,
                    "page_size": limit_val,
                    "total_pages": total_pages.max(1),
                })))
            } else {
                let sql = format!("{REIMB_SELECT}{where_sql} ORDER BY {order} LIMIT 100");
                let mut query = sqlx::query_as::<_, ReimbRow>(&sql).bind(u.id);
                for b in &binds {
                    query = query.bind(b.as_str());
                }
                let rows = query.fetch_all(&st.db).await?;
                Ok(Json(json!(rows
                    .iter()
                    .map(|r| reimb_json(r, &u))
                    .collect::<Vec<_>>())))
            }
        }
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cd backend && cargo test --test reimbursements_sort
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/community.rs backend/tests/reimbursements_sort.rs
git commit -m "feat(backend): sort and filter reimbursements"
```

### Task 5: `UniversalTable` sorting, toolbar, and mobile sort

**Files:**
- Modify: `frontend/src/components/ui.tsx:1-31` and `1272-1379`
- Modify: `frontend/src/locales/common.en.ts`
- Modify: `frontend/src/locales/common.zh.ts`
- Test: `frontend/src/components/ui.test.tsx:6-27` and `416-487`

**Interfaces:**
- Produces: `SortDirection`, `TableSort`, `TableFilter`, extended `TableColumn<T>`, and extended `UniversalTableProps<T>`.
- `onSortChange` accepts `TableSort | null` so the mobile “Default order” option can clear a sort.

- [ ] **Step 1: Write the failing tests**

In `frontend/src/components/ui.test.tsx`, add `TableFilter` to the import from `./ui`:

```tsx
  TableColumn,
  TableFilter,
  UniversalTable,
```

Append these tests inside the existing `describe('UniversalTable component', ...)` block:

```tsx
  interface SortableUser {
    id: number;
    name: string;
    age: number | null;
  }

  const sortableUsers: SortableUser[] = [
    { id: 1, name: 'Bo', age: 2 },
    { id: 2, name: 'Ana', age: null },
    { id: 3, name: 'Cy', age: 1 },
  ];

  const sortableColumns: TableColumn<SortableUser>[] = [
    {
      key: 'name',
      header: 'Name',
      sortable: true,
      sortValue: (u) => u.name,
      render: (u) => <span data-testid="sort-name">{u.name}</span>,
    },
    {
      key: 'age',
      header: 'Age',
      sortable: true,
      sortValue: (u) => u.age,
      render: (u) => <span data-testid="sort-age">{u.age ?? 'none'}</span>,
    },
  ];

  it('sorts locally and toggles direction', () => {
    render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        defaultSort={{ key: 'name', dir: 'asc' }}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Ana',
      'Bo',
      'Cy',
    ]);
    const header = screen.getAllByRole('columnheader')[0];
    expect(header).toHaveAttribute('aria-sort', 'ascending');
    fireEvent.click(screen.getByRole('button', { name: /name/i }));
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Cy',
      'Bo',
      'Ana',
    ]);
    expect(header).toHaveAttribute('aria-sort', 'descending');
  });

  it('keeps null values last in both directions', () => {
    render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        defaultSort={{ key: 'age', dir: 'desc' }}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getAllByTestId('sort-age').map((n) => n.textContent)).toEqual([
      '2',
      '1',
      'none',
    ]);
    fireEvent.click(screen.getByRole('button', { name: /age/i }));
    expect(screen.getAllByTestId('sort-age').map((n) => n.textContent)).toEqual([
      '1',
      '2',
      'none',
    ]);
  });

  it('does not reorder controlled rows and emits the next sort', () => {
    const onSortChange = vi.fn();
    render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        sort={{ key: 'name', dir: 'asc' }}
        onSortChange={onSortChange}
        itemKey={(u) => u.id}
      />,
    );
    expect(screen.getAllByTestId('sort-name').map((n) => n.textContent)).toEqual([
      'Bo',
      'Ana',
      'Cy',
    ]);
    fireEvent.click(screen.getByRole('button', { name: /name/i }));
    expect(onSortChange).toHaveBeenCalledWith({ key: 'name', dir: 'desc' });
  });

  it('renders a declarative filter toolbar and clear action', () => {
    const onFilterChange = vi.fn();
    const onClearFilters = vi.fn();
    const filters: TableFilter[] = [
      { type: 'text', key: 'q', label: 'Search' },
      {
        type: 'select',
        key: 'status',
        label: 'Status',
        allLabel: 'All statuses',
        options: [{ value: 'active', label: 'Active' }],
      },
      { type: 'date', key: 'date', label: 'Date' },
      { type: 'checkbox', key: 'attention', label: 'Only attention' },
    ];
    render(
      <UniversalTable
        columns={columns}
        items={users}
        itemKey={(u) => u.id}
        filters={filters}
        filterValues={{ q: 'ana' }}
        onFilterChange={onFilterChange}
        onClearFilters={onClearFilters}
        clearFiltersLabel="Clear filters"
      />,
    );
    expect(screen.getByRole('textbox', { name: 'Search' })).toHaveValue('ana');
    fireEvent.change(screen.getByRole('combobox', { name: 'Status' }), {
      target: { value: 'active' },
    });
    expect(onFilterChange).toHaveBeenCalledWith('status', 'active');
    fireEvent.click(screen.getByLabelText('Only attention'));
    expect(onFilterChange).toHaveBeenCalledWith('attention', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
    expect(onClearFilters).toHaveBeenCalled();
  });

  it('keeps the toolbar visible when there are no rows', () => {
    render(
      <UniversalTable
        columns={columns}
        items={[]}
        itemKey={(u) => u.id}
        filters={[{ type: 'text', key: 'q', label: 'Search' }]}
        filterValues={{}}
        emptyContent={<p data-testid="empty">No rows</p>}
      />,
    );
    expect(screen.getByRole('textbox', { name: 'Search' })).toBeInTheDocument();
    expect(screen.getByTestId('empty')).toBeInTheDocument();
  });

  it('does not fire row clicks from interactive children', () => {
    const onRowClick = vi.fn();
    const actionColumns: TableColumn<User>[] = [
      {
        key: 'action',
        header: 'Action',
        render: () => <button type="button">Edit</button>,
      },
    ];
    render(
      <UniversalTable
        columns={actionColumns}
        items={users}
        onRowClick={onRowClick}
        itemKey={(u) => u.id}
      />,
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'Edit' })[0]);
    expect(onRowClick).not.toHaveBeenCalled();
  });

  it('renders a mobile sort control for sortable columns', () => {
    const { container } = render(
      <UniversalTable
        columns={sortableColumns}
        items={sortableUsers}
        defaultSort={{ key: 'name', dir: 'asc' }}
        itemKey={(u) => u.id}
      />,
    );
    expect(container.querySelector('.table-mobile-sort select')).toBeInTheDocument();
  });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cd frontend && npx vitest run src/components/ui.test.tsx
```

Expected: FAIL — `TableFilter` is not exported, and sorting/toolbar props are not implemented.

- [ ] **Step 3: Implement the component and styles**

Add `ArrowUpDown`, `ChevronDown`, and `ChevronUp` to the existing `lucide-react` import in `frontend/src/components/ui.tsx`:

```tsx
import {
  AlertCircle,
  ArrowLeft,
  ArrowUp,
  ArrowUpDown,
  CheckCircle2,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  Info,
  Paperclip,
  X,
} from 'lucide-react';
```

Replace the whole `UniversalTable` section from `export interface TableColumn<T = unknown> {` through `export default UniversalTable;` with:

```tsx
export type SortDirection = 'asc' | 'desc';

export interface TableSort {
  key: string;
  dir: SortDirection;
}

export type TableFilter =
  | { type: 'text'; key: string; label: string; placeholder?: string }
  | {
      type: 'select';
      key: string;
      label: string;
      allLabel?: string;
      options: { value: string; label: string }[];
    }
  | { type: 'date'; key: string; label: string }
  | { type: 'checkbox'; key: string; label: string };

export interface TableColumn<T = unknown> {
  /** Stable key for the column, used as the cell key. */
  key: string;
  /** Header label for the column. */
  header: ReactNode;
  /** Renders the cell contents for a single row. */
  render: (item: T, rowIndex: number) => ReactNode;
  /** Label shown for this column on narrow (stacked) screens; falls back to `header`. */
  dataLabel?: string;
  /** Extra class name applied to the header cell. */
  headerClassName?: string;
  /** Extra class name applied to the cell. */
  cellClassName?: string;
  /** Enables the header sort control. */
  sortable?: boolean;
  /** Field used in sort state / server query; defaults to `key`. */
  sortKey?: string;
  /** Required for local sorting when the table is uncontrolled. */
  sortValue?: (item: T) => string | number | null | undefined;
  /** Accessible/mobile-sort label when `header` is not a string. */
  sortLabel?: string;
}

export interface UniversalTableProps<T> {
  /** Column definitions. */
  columns: TableColumn<T>[];
  /** Row data to render. */
  items: T[];
  /** When provided, rows become clickable and this is called with the row data. */
  onRowClick?: (item: T, rowIndex: number) => void;
  /** Stable key selector for each row. Defaults to the array index. */
  itemKey?: (item: T, index: number) => string | number;
  /** Content rendered when `items` is empty. */
  emptyContent?: ReactNode;
  /** Optional callback; returns true for a row that spans all columns (e.g. a group header). */
  isSectionRow?: (item: T, index: number) => boolean;
  /** Extra class name applied to the wrapping element. */
  className?: string;
  /** Controlled sort. When set, the table does not reorder `items`. */
  sort?: TableSort | null;
  /** Initial local sort when uncontrolled. */
  defaultSort?: TableSort | null;
  onSortChange?: (sort: TableSort | null) => void;
  /** Declarative filter toolbar. */
  filters?: TableFilter[];
  filterValues?: Record<string, string>;
  onFilterChange?: (key: string, value: string) => void;
  onClearFilters?: () => void;
  clearFiltersLabel?: string;
  toolbarActions?: ReactNode;
}

function columnSortKey<T>(col: TableColumn<T>): string {
  return col.sortKey ?? col.key;
}

function defaultDataLabel<T>(col: TableColumn<T>): string {
  return typeof col.header === 'string' ? (col.dataLabel ?? col.header) : '';
}

function sortLabel<T>(col: TableColumn<T>): string {
  return col.sortLabel ?? (typeof col.header === 'string' ? col.header : col.key);
}

function compareValues(a: unknown, b: unknown): number {
  if (typeof a === 'number' && typeof b === 'number') return a - b;
  return String(a).localeCompare(String(b), undefined, {
    numeric: true,
    sensitivity: 'base',
  });
}

function isInteractive(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    Boolean(
      target.closest(
        'a,button,input,select,textarea,[role="button"],[data-row-click-ignore]',
      ),
    )
  );
}

export function UniversalTable<T>({
  columns,
  items,
  onRowClick,
  itemKey,
  emptyContent,
  isSectionRow,
  className = '',
  sort,
  defaultSort = null,
  onSortChange,
  filters,
  filterValues,
  onFilterChange,
  onClearFilters,
  clearFiltersLabel,
  toolbarActions,
}: UniversalTableProps<T>) {
  const { t } = useTranslation('common');
  const [localSort, setLocalSort] = useState<TableSort | null>(defaultSort);
  const controlled = sort !== undefined;
  const activeSort = controlled ? sort : localSort;
  const sortableColumns = columns.filter((col) => col.sortable);

  const rows = useMemo(() => {
    if (controlled || !activeSort) return items;
    const col = columns.find((c) => columnSortKey(c) === activeSort.key);
    if (!col?.sortValue) return items;
    const direction = activeSort.dir === 'asc' ? 1 : -1;
    return items
      .map((item, index) => ({ item, index }))
      .sort((a, b) => {
        const av = col.sortValue?.(a.item);
        const bv = col.sortValue?.(b.item);
        // Nulls always sort last, whichever direction is active.
        if (av == null && bv == null) return a.index - b.index;
        if (av == null) return 1;
        if (bv == null) return -1;
        const cmp = compareValues(av, bv);
        if (cmp === 0) return a.index - b.index;
        return cmp * direction;
      })
      .map(({ item }) => item);
  }, [activeSort, columns, controlled, items]);

  const changeSort = (col: TableColumn<T>) => {
    const key = columnSortKey(col);
    const next: TableSort = {
      key,
      dir: activeSort?.key === key && activeSort.dir === 'asc' ? 'desc' : 'asc',
    };
    if (!controlled) setLocalSort(next);
    onSortChange?.(next);
  };

  const filtersList = filters ?? [];
  const values = filterValues ?? {};
  const hasActiveFilters = filtersList.some((f) => (values[f.key] ?? '') !== '');

  const renderFilter = (f: TableFilter) => {
    const value = values[f.key] ?? '';
    switch (f.type) {
      case 'text':
        return (
          <div key={f.key} className="table-filter">
            <input
              className="input admin-input-compact"
              type="text"
              value={value}
              placeholder={f.placeholder ?? f.label}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            />
          </div>
        );
      case 'select':
        return (
          <div key={f.key} className="table-filter">
            <select
              className="input admin-input-compact"
              value={value}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            >
              {f.allLabel != null && <option value="">{f.allLabel}</option>}
              {f.options.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
          </div>
        );
      case 'date':
        return (
          <div key={f.key} className="table-filter">
            <input
              className="input admin-input-compact"
              type="date"
              value={value}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            />
          </div>
        );
      case 'checkbox':
        return (
          <div key={f.key} className="table-filter">
            <label className={`check ${value === 'true' ? 'on' : ''}`}>
              <input
                type="checkbox"
                checked={value === 'true'}
                aria-label={f.label}
                onChange={(e) => onFilterChange?.(f.key, e.target.checked ? 'true' : '')}
              />
              {f.label}
            </label>
          </div>
        );
    }
  };

  const tableClassName = onRowClick
    ? 'table responsive sticky-header row-clickable'
    : 'table responsive sticky-header';

  return (
    <div className={`table-block${className ? ` ${className}` : ''}`}>
      {sortableColumns.length > 0 && (
        <label className="table-mobile-sort">
          <span>{t('table.sortBy')}</span>
          <select
            className="input admin-input-compact"
            value={activeSort ? `${activeSort.key}:${activeSort.dir}` : ''}
            aria-label={t('table.sortBy')}
            onChange={(e) => {
              const raw = e.target.value;
              if (!raw) {
                if (!controlled) setLocalSort(defaultSort);
                onSortChange?.(null);
                return;
              }
              const [key, dir] = raw.split(':');
              const next: TableSort = { key, dir: dir === 'desc' ? 'desc' : 'asc' };
              if (!controlled) setLocalSort(next);
              onSortChange?.(next);
            }}
          >
            <option value="">{t('table.defaultOrder')}</option>
            {sortableColumns.map((col) => {
              const key = columnSortKey(col);
              const label = sortLabel(col);
              return (
                <optgroup key={key} label={label}>
                  <option value={`${key}:asc`}>{label} ↑</option>
                  <option value={`${key}:desc`}>{label} ↓</option>
                </optgroup>
              );
            })}
          </select>
        </label>
      )}

      {(filtersList.length > 0 || toolbarActions) && (
        <div className="table-toolbar">
          {filtersList.map(renderFilter)}
          {hasActiveFilters && onClearFilters && (
            <Button variant="ghost" size="sm" onClick={onClearFilters}>
              {clearFiltersLabel ?? t('table.clearFilters')}
            </Button>
          )}
          {toolbarActions && <div className="table-toolbar-actions">{toolbarActions}</div>}
        </div>
      )}

      <div className="table-wrap">
        <table className={tableClassName}>
          <thead>
            <tr>
              {columns.map((col) => {
                const sorted = col.sortable && activeSort?.key === columnSortKey(col);
                const direction = sorted ? activeSort?.dir : undefined;
                return (
                  <th
                    key={col.key}
                    scope="col"
                    className={col.headerClassName}
                    aria-sort={
                      col.sortable
                        ? sorted
                          ? direction === 'asc'
                            ? 'ascending'
                            : 'descending'
                          : 'none'
                        : undefined
                    }
                  >
                    {col.sortable ? (
                      <button
                        type="button"
                        className="th-sort"
                        onClick={() => changeSort(col)}
                        aria-label={`${t('table.sortBy')} ${sortLabel(col)}`}
                      >
                        <span>{col.header}</span>
                        {direction === 'asc' ? (
                          <ChevronUp size={14} aria-hidden="true" />
                        ) : direction === 'desc' ? (
                          <ChevronDown size={14} aria-hidden="true" />
                        ) : (
                          <ArrowUpDown size={14} aria-hidden="true" className="sort-indicator" />
                        )}
                      </button>
                    ) : (
                      col.header
                    )}
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {rows.map((item, rowIndex) => {
              const key = itemKey ? itemKey(item, rowIndex) : rowIndex;
              const isSection = isSectionRow?.(item, rowIndex) ?? false;
              return (
                <tr
                  key={key}
                  className={onRowClick ? 'clickable' : ''}
                  tabIndex={onRowClick ? 0 : undefined}
                  onClick={(e) => {
                    if (isInteractive(e.target)) return;
                    onRowClick?.(item, rowIndex);
                  }}
                  onKeyDown={(e) => {
                    if (e.target !== e.currentTarget) return;
                    if (onRowClick && (e.key === 'Enter' || e.key === ' ')) {
                      e.preventDefault();
                      onRowClick?.(item, rowIndex);
                    }
                  }}
                >
                  {isSection ? (
                    <td key={`section-${key}`} colSpan={columns.length}>
                      {columns[0]?.render?.(item, rowIndex)}
                    </td>
                  ) : (
                    columns.map((col) => (
                      <td
                        key={col.key}
                        data-label={defaultDataLabel(col) || undefined}
                        className={col.cellClassName}
                      >
                        {col.render(item, rowIndex)}
                      </td>
                    ))
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {items.length === 0 && emptyContent != null && <div>{emptyContent}</div>}
    </div>
  );
}

export default UniversalTable;
```

Add the new CSS immediately after the existing `.admin-input-compact` rule in `frontend/src/styles.css`:

```css
.table-block { display: flex; flex-direction: column; gap: var(--space-3); }
.table-toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
.table-filter { display: flex; align-items: center; min-width: 0; }
.table-toolbar-actions { margin-left: auto; display: flex; align-items: center; gap: var(--space-2); }
.th-sort {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: 0;
  border: 0;
  background: none;
  color: inherit;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.th-sort:hover { color: var(--text); }
.th-sort:focus-visible { outline: 3px solid var(--primary); outline-offset: 2px; border-radius: var(--radius-sm); }
.sort-indicator { opacity: 0.45; }
.table-mobile-sort { display: none; }

@media (max-width: 767px) {
  .table-mobile-sort { display: flex; align-items: center; gap: var(--space-2); }
  .table-mobile-sort > span { color: var(--muted); font-size: var(--fs-sm); }
  .table-mobile-sort .input { flex: 1; max-width: none; }
}
```

Add these keys to `frontend/src/locales/common.en.ts` after `exportCsv`:

```ts
  table: {
    clearFilters: 'Clear filters',
    sortBy: 'Sort by',
    defaultOrder: 'Default order',
  },
```

Add the matching keys to `frontend/src/locales/common.zh.ts` after `exportCsv`:

```ts
  table: {
    clearFilters: '清除筛选',
    sortBy: '排序',
    defaultOrder: '默认排序',
  },
```

- [ ] **Step 4: Run the tests to verify they pass**

Run:

```bash
cd frontend && npx vitest run src/components/ui.test.tsx
```

Expected: PASS, including the pre-existing table tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/ui.tsx frontend/src/components/ui.test.tsx frontend/src/locales/common.en.ts frontend/src/locales/common.zh.ts frontend/src/styles.css
git commit -m "feat(ui): add table sorting and filter toolbar"
```

### Task 6: Shared table URL state hook

**Files:**
- Create: `frontend/src/components/tableState.ts`
- Test: `frontend/src/components/tableState.test.tsx`

**Interfaces:**
- Consumes: `TableSort` from `./ui` (Task 5).
- Produces: `useTableParams(options)` returning `page`, `pageSize`, `sort`, `filters`, `setPage`, `setPageSize`, `setSort`, `setFilter`, `clearFilters`, `hasFilters`, and `queryString`.

- [ ] **Step 1: Write the failing test**

Create `frontend/src/components/tableState.test.tsx`:

```tsx
import { describe, expect, it } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { useTableParams } from './tableState';

function Harness() {
  const location = useLocation();
  const table = useTableParams({
    defaultPageSize: 25,
    defaultSort: { key: 'name', dir: 'asc' },
    filterKeys: ['q', 'status'],
  });
  return (
    <div>
      <output data-testid="search">{location.search}</output>
      <output data-testid="page">{table.page}</output>
      <output data-testid="sort">
        {table.sort ? `${table.sort.key}:${table.sort.dir}` : 'none'}
      </output>
      <output data-testid="query">{table.queryString}</output>
      <output data-testid="filters">{JSON.stringify(table.filters)}</output>
      <button onClick={() => table.setPage(3)}>page3</button>
      <button onClick={() => table.setPageSize(50)}>size50</button>
      <button onClick={() => table.setFilter('q', 'ana')}>search</button>
      <button onClick={() => table.setSort({ key: 'name', dir: 'desc' })}>sort</button>
      <button onClick={table.clearFilters}>clear</button>
    </div>
  );
}

describe('useTableParams', () => {
  it('serializes filters/sort, resets the page, and preserves unrelated params', () => {
    render(
      <MemoryRouter
        initialEntries={['/members?scope=keep&q=bo&page=4&sort=name&dir=desc']}
      >
        <Harness />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('page')).toHaveTextContent('4');
    expect(screen.getByTestId('sort')).toHaveTextContent('name:desc');
    expect(screen.getByTestId('query')).toHaveTextContent(
      'page=4&sort=name&dir=desc&q=bo',
    );
    expect(screen.getByTestId('filters')).toHaveTextContent('{"q":"bo","status":""}');

    fireEvent.click(screen.getByRole('button', { name: 'search' }));
    expect(screen.getByTestId('search')).toHaveTextContent(
      '?scope=keep&q=ana&sort=name&dir=desc',
    );
    expect(screen.getByTestId('page')).toHaveTextContent('1');
    expect(screen.getByTestId('query')).toHaveTextContent('sort=name&dir=desc&q=ana');

    fireEvent.click(screen.getByRole('button', { name: 'clear' }));
    expect(screen.getByTestId('search')).toHaveTextContent('?scope=keep');
    expect(screen.getByTestId('sort')).toHaveTextContent('name:asc');
    expect(screen.getByTestId('query').textContent).toBe('');
  });

  it('omits a sort that equals the default', () => {
    render(
      <MemoryRouter initialEntries={['/members?page=2&sort=name&dir=asc']}>
        <Harness />
      </MemoryRouter>,
    );

    expect(screen.getByTestId('query')).toHaveTextContent('page=2');
  });

  it('changes page size and resets the page', () => {
    render(
      <MemoryRouter initialEntries={['/members?page=4']}>
        <Harness />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'size50' }));
    expect(screen.getByTestId('search')).toHaveTextContent('?limit=50');
    expect(screen.getByTestId('page')).toHaveTextContent('1');
    expect(screen.getByTestId('query')).toHaveTextContent('limit=50');
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cd frontend && npx vitest run src/components/tableState.test.tsx
```

Expected: FAIL — cannot resolve `./tableState`.

- [ ] **Step 3: Implement the hook**

Create `frontend/src/components/tableState.ts`:

```ts
import { useSearchParams } from 'react-router-dom';
import type { TableSort } from './ui';

export interface UseTableParamsOptions {
  defaultPageSize?: number;
  defaultSort?: TableSort | null;
  filterKeys?: string[];
}

export interface TableParams {
  page: number;
  pageSize: number;
  sort: TableSort | null;
  filters: Record<string, string>;
  hasFilters: boolean;
  queryString: string;
  setPage: (page: number) => void;
  setPageSize: (pageSize: number) => void;
  setSort: (sort: TableSort | null) => void;
  setFilter: (key: string, value: string) => void;
  clearFilters: () => void;
}

export function useTableParams({
  defaultPageSize = 25,
  defaultSort = null,
  filterKeys = [],
}: UseTableParamsOptions = {}): TableParams {
  const [params, setParams] = useSearchParams();

  const page = Math.max(1, Number(params.get('page') ?? '1') || 1);
  const pageSize = Math.max(
    1,
    Number(params.get('limit') ?? String(defaultPageSize)) || defaultPageSize,
  );
  const sortKey = params.get('sort');
  const sort: TableSort | null = sortKey
    ? { key: sortKey, dir: params.get('dir') === 'desc' ? 'desc' : 'asc' }
    : defaultSort;

  const filters: Record<string, string> = {};
  for (const key of filterKeys) filters[key] = params.get(key) ?? '';
  const hasFilters = filterKeys.some((key) => (params.get(key) ?? '') !== '');

  const update = (mutate: (next: URLSearchParams) => void) => {
    const next = new URLSearchParams(params);
    mutate(next);
    setParams(next, { replace: true });
  };

  const setPage = (value: number) => {
    update((next) => {
      if (value > 1) next.set('page', String(value));
      else next.delete('page');
    });
  };

  const setPageSize = (value: number) => {
    update((next) => {
      if (value === defaultPageSize) next.delete('limit');
      else next.set('limit', String(value));
      next.delete('page');
    });
  };

  const setSort = (value: TableSort | null) => {
    update((next) => {
      next.delete('page');
      if (
        !value ||
        (defaultSort &&
          value.key === defaultSort.key &&
          value.dir === defaultSort.dir)
      ) {
        next.delete('sort');
        next.delete('dir');
      } else {
        next.set('sort', value.key);
        next.set('dir', value.dir);
      }
    });
  };

  const setFilter = (key: string, value: string) => {
    update((next) => {
      if (value) next.set(key, value);
      else next.delete(key);
      next.delete('page');
    });
  };

  const clearFilters = () => {
    update((next) => {
      for (const key of filterKeys) next.delete(key);
      next.delete('sort');
      next.delete('dir');
      next.delete('page');
    });
  };

  const query = new URLSearchParams();
  if (page > 1) query.set('page', String(page));
  if (pageSize !== defaultPageSize) query.set('limit', String(pageSize));
  if (
    sort &&
    (!defaultSort ||
      sort.key !== defaultSort.key ||
      sort.dir !== defaultSort.dir)
  ) {
    query.set('sort', sort.key);
    query.set('dir', sort.dir);
  }
  for (const key of filterKeys) {
    const value = params.get(key);
    if (value) query.set(key, value);
  }

  return {
    page,
    pageSize,
    sort,
    filters,
    hasFilters,
    queryString: query.toString(),
    setPage,
    setPageSize,
    setSort,
    setFilter,
    clearFilters,
  };
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
cd frontend && npx vitest run src/components/tableState.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/tableState.ts frontend/src/components/tableState.test.tsx
git commit -m "feat(ui): add URL-backed table state hook"
```

### Task 7: Wire Members to server-side sort and the toolbar

**Files:**
- Modify: `frontend/src/pages/admin/Members.tsx:1-30`, `122-318`
- Modify: `frontend/src/admin.test.tsx:164-175`

**Interfaces:**
- Consumes: `useTableParams` (Task 6), `TableFilter` / extended `UniversalTable` (Task 5).
- Produces: Members list requests `/admin/users?q=...&page=...&limit=...&sort=name|email&dir=...`.

- [ ] **Step 1: Run the existing Members test to establish a baseline**

Run:

```bash
cd frontend && npx vitest run src/admin.test.tsx -t "Admin Members"
```

Expected: PASS before changes.

- [ ] **Step 2: Replace the page search/pagination state**

Keep the existing `FormEvent`, `useState`, and `useSearchParams` imports: the parent `Members` component still uses `useSearchParams` for the tab. Add `TableFilter` to the `../../components/ui` import list, and import the hook:

```tsx
  TableColumn,
  TableFilter,
  UniversalTable,
```

```tsx
import { useTableParams } from '../../components/tableState';
```

Also update the parent `handleTabChange` in the same file to clear table state:

```tsx
    p.delete('page');
    p.delete('limit');
    p.delete('q');
    p.delete('sort');
    p.delete('dir');
```

Replace the start of `MemberList` through the `handleSearch` definition with:

```tsx
function MemberList() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can, me } = useAuth();
  const canEdit = can('admin_members', 'update');
  // A super admin's account is changed only by another super admin (the server enforces it).
  const isSuper = !!me?.roles?.includes('superadmin');
  const editable = (u: User) => isSuper || !u.roles.includes('superadmin');
  const { names } = useRoleOptions();
  const {
    page,
    pageSize,
    sort,
    filters,
    setPage,
    setPageSize,
    setSort,
    setFilter,
    clearFilters,
    queryString,
  } = useTableParams({
    defaultPageSize: 50,
    defaultSort: { key: 'name', dir: 'asc' },
    filterKeys: ['q'],
  });

  const { data, loading, reload } = useApi<PaginatedUsers | User[]>(
    `/admin/users?${queryString}`,
  );
  const [modalUser, setModalUser] = useState<{
    user: User;
    mode: 'edit' | 'approve';
  } | null>(null);

  const users = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const filterDefs: TableFilter[] = [
    {
      type: 'text',
      key: 'q',
      label: t('members.search'),
      placeholder: t('members.search'),
    },
  ];
```

Delete the old `setParam` and `handleSearch` definitions. The `handleExport` function stays unchanged.

- [ ] **Step 3: Make the name and email columns sortable**

Change the first two entries of `columns`:

```tsx
    {
      key: 'name',
      header: t('members.name'),
      sortable: true,
      sortKey: 'name',
      render: (u) => (
        <div className="row nowrap">
          <Avatar name={u.name} size="sm" />
          <span>{u.name}</span>
        </div>
      ),
    },
    {
      key: 'email',
      header: t('members.email'),
      sortable: true,
      sortKey: 'email',
      render: (u) => u.email,
    },
```

- [ ] **Step 4: Replace the render block**

Replace the returned JSX from `<div className="stack">` through the component's final `);` with:

```tsx
  return (
    <div className="stack">
      <Card>
        {loading ? (
          <CardSkeleton count={4} />
        ) : (
          <>
            <UniversalTable
              columns={columns}
              items={users}
              itemKey={(u) => u.id}
              sort={sort}
              onSortChange={setSort}
              filters={filterDefs}
              filterValues={filters}
              onFilterChange={setFilter}
              onClearFilters={clearFilters}
              toolbarActions={
                users.length > 0 ? (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={handleExport}
                    icon={<Download size={16} />}
                  >
                    {t('common:exportCsv', 'Export CSV')}
                  </Button>
                ) : undefined
              }
              emptyContent={<EmptyState title={t('members.none')} />}
            />
            {users.length > 0 && (
              <Pagination
                page={page}
                totalPages={totalPages}
                total={total}
                pageSize={pageSize}
                onPageChange={setPage}
                onPageSizeChange={setPageSize}
              />
            )}
          </>
        )}
      </Card>

      {modalUser && (
        <MemberFormModal
          user={modalUser.user}
          mode={modalUser.mode}
          onClose={() => setModalUser(null)}
          onSuccess={() => {
            setModalUser(null);
            reload();
          }}
        />
      )}
    </div>
  );
```

- [ ] **Step 5: Update the admin test mock**

In `frontend/src/admin.test.tsx`, change:

```tsx
  '/admin/users?q=': mockUsers,
```

to:

```tsx
  '/admin/users?': mockUsers,
```

- [ ] **Step 6: Run the tests to verify they pass**

Run:

```bash
cd frontend && npx vitest run src/admin.test.tsx -t "Admin Members"
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/pages/admin/Members.tsx frontend/src/admin.test.tsx
git commit -m "feat(admin): wire member table sorting and filters"
```

### Task 8: Wire Rides to server-side sort and the toolbar

**Files:**
- Modify: `frontend/src/pages/admin/Rides.tsx:1-212`

**Interfaces:**
- Consumes: `useTableParams` (Task 6), `TableFilter` / extended `UniversalTable` (Task 5).
- Produces: Rides requests `/admin/rides?status=...&date=...&attention=true&page=...&limit=...&sort=pickup_at|passenger|area|driver|status&dir=...`.

- [ ] **Step 1: Run the existing Rides test to establish a baseline**

Run:

```bash
cd frontend && npx vitest run src/admin.test.tsx -t "Admin Rides"
```

Expected: PASS before changes.

- [ ] **Step 2: Replace the page state with the hook**

Change the first import in `frontend/src/pages/admin/Rides.tsx`:

```tsx
import { useNavigate } from 'react-router-dom';
```

Add `TableFilter` to the `../../components/ui` import list, and import the hook:

```tsx
  Pagination,
  UniversalTable,
  TableColumn,
  TableFilter,
  StatusPill,
```

```tsx
import { useTableParams } from '../../components/tableState';
```

Replace the state block from `const [params, setParams] = useSearchParams();` through the `hasFilters` declaration with:

```tsx
  const {
    page,
    pageSize,
    sort,
    filters,
    setPage,
    setPageSize,
    setSort,
    setFilter,
    clearFilters,
    queryString,
  } = useTableParams({
    defaultPageSize: 50,
    defaultSort: { key: 'pickup_at', dir: 'asc' },
    filterKeys: ['status', 'date', 'attention'],
  });

  const { data, error, loading } = useApi<AdminRidesData | Ride[]>(
    `/admin/rides?${queryString}`,
    20000,
  );

  const rides = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const filterDefs: TableFilter[] = [
    {
      type: 'select',
      key: 'status',
      label: t('rides.status'),
      allLabel: t('rides.allStatus'),
      options: FILTER_STATUSES.map((k) => ({
        value: k,
        label: ft(RIDE_STATUS[k].label),
      })),
    },
    { type: 'date', key: 'date', label: t('rides.date') },
    { type: 'checkbox', key: 'attention', label: t('rides.onlyAttention') },
  ];
```

Delete the old `set`, `clearFilters`, and `hasFilters` definitions. `handleExport` stays unchanged.

- [ ] **Step 3: Make the ride columns sortable**

Replace the `columns` array with:

```tsx
  const columns: TableColumn<Ride>[] = [
    {
      key: 'pickup_at',
      header: t('dashboard.pickupTime'),
      sortable: true,
      sortKey: 'pickup_at',
      render: (r) => (
        <>
          {fmtDay(r.pickup_at, ft).split(' ')[0]} {r.pickup_at.slice(11, 16)}
        </>
      ),
    },
    {
      key: 'passenger',
      header: t('dashboard.passenger'),
      sortable: true,
      sortKey: 'passenger',
      render: (r) => `${r.passenger.name}${r.seats > 1 ? ` (${r.seats})` : ''}`,
    },
    {
      key: 'area',
      header: t('dashboard.area'),
      sortable: true,
      sortKey: 'area',
      render: (r) => r.area.name,
    },
    {
      key: 'driver',
      header: t('dashboard.driver'),
      sortable: true,
      sortKey: 'driver',
      render: (r) => r.driver?.name ?? <span className="muted">—</span>,
    },
    {
      key: 'status',
      header: t('dashboard.status'),
      sortable: true,
      sortKey: 'status',
      render: (r) => (
        <>
          <StatusPill map={RIDE_STATUS} status={r.status} />
          {r.needs_attention && (
            <AlertTriangle size={14} aria-hidden="true" className="pill-icon warn" />
          )}
        </>
      ),
    },
  ];
```

- [ ] **Step 4: Replace the filter bar and table render**

Delete the `<div className="admin-filters-bar">...</div>` block entirely.

Replace the `<Card>...</Card>` block with:

```tsx
      <Card>
        {loading ? (
          <CardSkeleton count={4} />
        ) : (
          <>
            <UniversalTable
              columns={columns}
              items={rides}
              onRowClick={(r) => nav(`/admin/rides/${r.id}`)}
              itemKey={(r) => r.id}
              sort={sort}
              onSortChange={setSort}
              filters={filterDefs}
              filterValues={filters}
              onFilterChange={setFilter}
              onClearFilters={clearFilters}
              emptyContent={<EmptyState title={t('rides.none')} />}
            />
            {rides.length > 0 && (
              <Pagination
                page={page}
                totalPages={totalPages}
                total={total}
                pageSize={pageSize}
                onPageChange={setPage}
                onPageSizeChange={setPageSize}
              />
            )}
          </>
        )}
      </Card>
```

- [ ] **Step 5: Run the Rides test and add a sortable-header assertion**

Add this assertion to the existing Admin Rides test, after the attention checkbox assertion:

```tsx
    expect(
      screen.getByRole('button', { name: /sort by pickup time/i }),
    ).toBeInTheDocument();
```

Run:

```bash
cd frontend && npx vitest run src/admin.test.tsx -t "Admin Rides"
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/pages/admin/Rides.tsx frontend/src/admin.test.tsx
git commit -m "feat(admin): wire ride table sorting and filters"
```

### Task 9: Wire Reimbursements to server-side sort and filters

**Files:**
- Modify: `frontend/src/pages/admin/Reimbursements.tsx:1-267`
- Modify: `frontend/src/locales/admin.en.ts:163-202`
- Modify: `frontend/src/locales/admin.zh.ts:160-205`
- Modify: `frontend/src/admin.test.tsx:336-350`

**Interfaces:**
- Consumes: `useTableParams` (Task 6), `TableFilter` / extended `UniversalTable` (Task 5), and the backend `q`/`status`/`sort`/`dir` contract (Task 4).
- Produces: Reimbursement requests `/reimbursements?scope=review|payout&q=...&status=...&page=...&limit=...&sort=applicant|amount_cents|created_at|status&dir=...`.

- [ ] **Step 1: Run the existing Reimbursements test to establish a baseline**

Run:

```bash
cd frontend && npx vitest run src/admin.test.tsx -t "Admin Reimbursements"
```

Expected: PASS before changes.

- [ ] **Step 2: Add the locale keys**

In `frontend/src/locales/admin.en.ts`, add these keys inside `reimb` after `status`:

```ts
    search: 'Search applicant, payee or description',
    allStatus: 'All statuses',
```

In `frontend/src/locales/admin.zh.ts`, add these keys inside `reimb` after `status`:

```ts
    search: '搜索申请人、收款人或说明',
    allStatus: '全部状态',
```

- [ ] **Step 3: Replace the list state and filters**

In `frontend/src/pages/admin/Reimbursements.tsx`, keep the existing `Link, useSearchParams` import because the parent tabs still use it. Add `TableFilter` to the `../../components/ui` import list and import the hook:

```tsx
  TableColumn,
  TableFilter,
  Tabs,
  UniversalTable,
  useAction,
```

```tsx
import { useTableParams } from '../../components/tableState';
```

Replace the start of `List` through the `const totalPages` line with:

```tsx
function List({ scope }: { scope: 'review' | 'payout' }) {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const { can } = useAuth();
  const canAct = can(scope === 'review' ? 'admin_reimb_review' : 'admin_reimb_pay', 'update');
  const {
    page,
    pageSize,
    sort,
    filters,
    setPage,
    setPageSize,
    setSort,
    setFilter,
    clearFilters,
    queryString,
  } = useTableParams({
    defaultPageSize: 25,
    filterKeys: ['q', 'status'],
  });
  const scopeQs = `scope=${scope}${queryString ? `&${queryString}` : ''}`;
  const { data, error, loading, reload } = useApi<PaginatedReimbursements | Reimb[]>(
    `/reimbursements?${scopeQs}`,
  );
  const [open, setOpen] = useState<Reimb | null>(null);

  const items = Array.isArray(data) ? data : data?.items ?? [];
  const total = Array.isArray(data) ? data.length : data?.total ?? 0;
  const totalPages = Array.isArray(data) ? 1 : data?.total_pages ?? 1;

  const statusOptions = Object.keys(REIMB_STATUS)
    .filter((k) => scope === 'review' || k === 'approved' || k === 'paid')
    .map((k) => ({ value: k, label: ft(REIMB_STATUS[k].label) }));

  const filterDefs: TableFilter[] = [
    {
      type: 'text',
      key: 'q',
      label: t('reimb.search'),
      placeholder: t('reimb.search'),
    },
    {
      type: 'select',
      key: 'status',
      label: t('reimb.status'),
      allLabel: t('reimb.allStatus'),
      options: statusOptions,
    },
  ];
```

- [ ] **Step 4: Make the reimbursement columns sortable**

Replace the `applicant`, `amount`, `submitted`, and `status` entries in `columns` with:

```tsx
    {
      key: 'applicant',
      header: t('reimb.applicant'),
      sortable: true,
      sortKey: 'applicant',
      render: (r) => <div className="bold">{r.user.name}</div>,
    },
    {
      key: 'amount',
      header: t('reimb.amount'),
      sortable: true,
      sortKey: 'amount_cents',
      render: (r) => <div className="mono bold">{rm(r.amount_cents)}</div>,
    },
```

```tsx
    {
      key: 'created_at',
      header: t('reimb.submitted'),
      sortable: true,
      sortKey: 'created_at',
      render: (r) => fmtDay(r.created_at, ft),
    },
    {
      key: 'status',
      header: t('reimb.status'),
      sortable: true,
      sortKey: 'status',
      render: (r) => <StatusPill map={REIMB_STATUS} status={r.status} />,
    },
```

- [ ] **Step 5: Replace the table and pagination render**

Replace the existing `loading ? ... : !items.length ? ... : (<Card>...</Card>)` block with:

```tsx
      {loading ? (
        <CardSkeleton count={3} />
      ) : (
        <Card>
          <UniversalTable
            columns={columns}
            items={items}
            itemKey={(r) => r.id}
            sort={sort}
            onSortChange={setSort}
            filters={filterDefs}
            filterValues={filters}
            onFilterChange={setFilter}
            onClearFilters={clearFilters}
            emptyContent={<EmptyState title={t('reimb.none')} />}
          />
          {items.length > 0 && (
            <Pagination
              page={page}
              totalPages={totalPages}
              total={total}
              pageSize={pageSize}
              onPageChange={setPage}
              onPageSizeChange={setPageSize}
            />
          )}
        </Card>
      )}
```

- [ ] **Step 6: Reset table state when the tab changes**

In `AdminReimbursements`'s `handleTabChange`, after `p.delete('page');` add:

```tsx
    p.delete('q');
    p.delete('status');
    p.delete('sort');
    p.delete('dir');
```

- [ ] **Step 7: Assert the new toolbar in the existing test**

Add this assertion to the Admin Reimbursements test, after the notice assertion:

```tsx
    expect(
      screen.getByPlaceholderText('Search applicant, payee or description'),
    ).toBeInTheDocument();
```

Run:

```bash
cd frontend && npx vitest run src/admin.test.tsx -t "Admin Reimbursements"
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/pages/admin/Reimbursements.tsx frontend/src/locales/admin.en.ts frontend/src/locales/admin.zh.ts frontend/src/admin.test.tsx
git commit -m "feat(admin): wire reimbursement sorting and filters"
```

### Task 10: Full verification and cleanup

**Files:**
- Review only; fix any regression found in the files above.

**Interfaces:**
- Confirms Audit is unchanged and all existing tests/builds pass.

- [ ] **Step 1: Run the full frontend unit suite**

```bash
cd frontend && npm test
```

Expected: all test files pass, including `ui.test.tsx`, `tableState.test.tsx`, and `admin.test.tsx`.

- [ ] **Step 2: Run the production frontend build**

```bash
cd frontend && npm run build
```

Expected: `tsc -b` and `vite build` complete without errors.

- [ ] **Step 3: Run frontend lint**

```bash
cd frontend && npm run lint
```

Expected: no errors. Fix any new warning introduced by the changed files.

- [ ] **Step 4: Run backend formatting and the full backend suite**

```bash
cd backend && cargo fmt --check
cd backend && cargo test
```

Expected: formatting clean and all backend tests pass, including the three new sort/filter test files.

- [ ] **Step 5: Confirm Audit was not changed**

Record the file hash before Task 1 and compare it at the end, so pre-existing local edits do not confuse the check:

```bash
git hash-object frontend/src/pages/admin/Audit.tsx
```

Expected: the hash is identical to the one recorded before Task 1. No task in this plan should modify `Audit.tsx`.

- [ ] **Step 6: Commit any verification fixes**

If a step required a fix:

```bash
git add frontend/src backend/src backend/tests frontend/src/components frontend/src/pages/admin frontend/src/locales frontend/src/admin.test.tsx
git commit -m "fix: address universal table verification findings"
```

If no fixes were required, skip the commit and report the verification results.
