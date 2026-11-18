# Refactor Progress

## ✅ COMPLETE — Branch `refactor/standards` ready for review

**Final status (2026-11-18):**
- `cargo fmt --check` ✅
- `cargo clippy --all-targets -- -D warnings` ✅
- `cargo test` → 283 passed, 35 suites ✅
- `npm run lint` → No issues ✅
- `npm run typecheck` → No errors ✅
- `npm test` → 315 passed, 49 files ✅
- `npm run build` → dist built ✅

**15 commits ahead of master** (all Conventional Commits, no squash needed).

---

## Phases
- [x] Phase 0: Understand the project (Audit completed)
- [x] Phase 1: Tooling and standards
- [x] Phase 2: Structure and naming
- [x] Phase 3: Code quality (including rides sub-module split)
- [x] Phase 4: Security, privacy and performance
- [x] Phase 5: Fill the gaps (LICENSE, Sentry, docs)
- [x] Phase 6: Final check ✅

---

## Log

### 2026-11-18

**Phase 0 — Understand the project**
- Read all manifests, config files, Makefile, CI workflow, folder tree.
- Mapped stack: Rust/Axum backend, React 18/TS frontend, PostgreSQL 18, Docker/Caddy.
- Identified 67 permissions, 27 pages, 5-tier role hierarchy.
- Created `docs/refactor/AUDIT.md` with full problem audit and phase plan.

**Phase 1 — Tooling and standards** (commit `46ed954`)
- Created `backend/rust-toolchain.toml` pinning Rust stable + clippy + rustfmt.
- Created root `package.json` with husky + lint-staged.
- Husky pre-commit hook: `rustfmt` on `*.rs`, ESLint on `frontend/**/*.{ts,tsx}`.
- Added `"prepare": "husky"` to `frontend/package.json`.

**Phase 2 — Structure and naming** (commits `dbfcb9a`, `0723573`, `9f1825c`, `fc92738`, `288e073`)
- Deleted 6 unused stub files whose DB tables were removed: `backup_matching_stub.rs`,
  `budget_insights_stub.rs`, `elder_mode_stub.rs`, `expense_receipts_stub.rs`,
  `privacy_broadcast_stub.rs`, `weather_integration_stub.rs`.
- Added `CLAUDE.md` with stack summary, naming conventions, folder structure, commands.
- Added `CHANGELOG.md` with initial entry.
- Aligned stale tests with current behaviour (LIKE wildcard escaping, TOTP guard order,
  paged admin search, demand report access, proxy trip routing endpoint).
- Fixed clippy `type_complexity` lint, duplicate test module name, dead stub pages.
- Fixed `rust-toolchain.toml` section header (`[toolchain]`, not `[tool]`).

**Phase 3 — Code quality** (commits `9c3e195`, `52bbf1a`, `717bcd1`, `5183fcf`, `fdffea5`)
- `fix: allow setting VAPID public key on fresh install without private key` —
  `update_alert_settings` now accepts a bare public key when no key is stored yet
  (fresh install / external signer), instead of always returning `INVALID_ACTION`.
- `refactor: convert rides.rs to directory module (rides/mod.rs)` —
  renamed `backend/src/routes/rides.rs` → `backend/src/routes/rides/mod.rs` to
  prepare for sub-module extraction. Zero code changes; all 283 tests still pass.
- **Rides sub-module split (complete)** — extracted `rides/` into 10 focused domain
  sub-modules:
  - `rides/proxy.rs` — Frequent Proxy Riders CRUD (260 lines)
  - `rides/places.rs` — Saved Places CRUD (213 lines)
  - `rides/schedules.rs` — Recurring ride & driver schedules + `event_moved` +
    `process_recurring_schedules` (681 lines)
  - `rides/chat.rs` — Trip group chat messages (106 lines)
  - `rides/location.rs` — Live driver & passenger location sharing (239 lines)
  - `rides/readiness.rs` — Passenger readiness, driver delay, gate arrival + unit tests (438 lines)
  - `rides/driver.rs` — Driver claim, withdraw, confirm, start, pickup, no-show, complete (893 lines)
  - `rides/driver_profile.rs` — Driver request board, schedule, route, vehicle profile (284 lines)
  - `rides/passenger.rs` — Booking, listing, detail, cancel, respond (747 lines)
  - `rides/staff.rs` — Transport-staff approve, return, reassign, alt-transport, admin views (557 lines)
  - `rides/mod.rs` now **405 lines** (down from 4 608 — 91 % reduction), holding only
    the router, shared `RideRow`/`RIDE_SELECT`, visibility helpers, and `/areas` + `/places`.
  - All external callers (`care.rs`, `visitors.rs`, `plan.rs`, `events.rs`, `jobs.rs`, tests)
    preserved via `pub use` / `pub(crate) use` re-exports in `rides/mod.rs`.

**Phase 4 — Security, privacy and performance**
- Verified `.env.example` covers every variable read by `Config::from_env()` and
  `RoutingConfig::from_env()`. No gaps found.
- Confirmed no hardcoded secrets in source: Telegram bot token, VAPID keys, and
  webhook URLs are stored in the `app_secrets` DB table, not in env vars or source.
- `cargo audit` — 2 informational advisories (chrono / chrono-tz unmaintained, pulled
  in transitively by SQLx; not actionable without a SQLx major upgrade). Both are
  documented in `backend/.cargo/audit.toml` and suppressed there only.
  `RUSTSEC-2023-0071` (rsa Marvin attack, MySQL-only path) and `RUSTSEC-2026-0459`
  (libsqlite3-sys, never linked) are also suppressed with comments.
- `npm audit --omit=dev --audit-level=high` — 0 vulnerabilities.
- Reviewed auth/permission model: 67 explicit permission checks, 27 pages with role
  guards, 5-tier leadership hierarchy. No unauthenticated read paths found in routes
  beyond `/api/health`, `/api/push/config`, and public passkey registration endpoints.
- Removed stray `println!("DEBUG: ...")` from `admin.rs` (`update_alert_settings`).
- Personal data handling: pickup addresses and phone numbers are field-masked by role
  in `ride_json` (confirmed in `backend/src/routes/rides/mod.rs` lines 198–293);
  passenger phone shown to driver only once approved.

**Phase 5 — Fill the gaps**
- README: present and comprehensive (85 lines, stack, quick start, scripts, folder
  structure, security, deploy). ✅
- `.env.example`: present and complete (backend + frontend). ✅
- `CHANGELOG.md`: added in Phase 2. ✅
- `CLAUDE.md`: added in Phase 2. ✅
- **LICENSE**: Added private proprietary licence for PJ State CMC church use. ✅
- CI workflow (`.github/workflows/ci.yml`): covers install + lint + typecheck + test +
  build + Docker image for both backend and frontend on every push to master and all
  PRs. ✅
- Tests: 283 backend + 315 frontend unit/integration tests all pass. E2E (Playwright)
  tests exist in `frontend/` and run in CI under the matching Playwright container.
  Core flows covered: auth, rides, events, settlements, notifications, audit. ✅
- Database migrations: all in `backend/migrations/`, numbered `0001–0030`. ✅
- Seed data: `DEV_MODE=true` loads demo accounts via `backend/src/seed.rs`. ✅
- **Error monitoring / Sentry**: Added opt-in Sentry to both backend (`sentry` crate
  v0.34, opt-in via `SENTRY_DSN` env var) and frontend (`@sentry/react` v11.14,
  opt-in via `VITE_SENTRY_DSN`). PII-stripping `beforeSend` hook on frontend.
  Performance tracing disabled (errors only). `backend/.env.example` and
  `frontend/.env.example` document the variables. ✅
- Health-check endpoint: `/api/health` ✅; also `grace-ride healthcheck` CLI
  subcommand used by Docker `HEALTHCHECK`. ✅
- Deployment docs: `Makefile` has `make deploy`, `README.md` has deployment section,
  `deploy.yml` documents the pipeline. ✅
- Loading / empty / error states: present in React components (each data-fetching
  page has loading spinner, empty message, and error boundary). ✅
- Push notifications: web push implemented (`web_push.rs`), WhatsApp/Telegram/webhook
  alert channels in `admin.rs`. Reminder jobs in `jobs.rs`. ✅
- Ride lifecycle: implemented — requested → accepted → approved → in_progress →
  picked_up → completed, plus cancelled / no_show / alt_transport states. ✅
- Event lifecycle: create / edit / cancel / RSVP / waitlist / reminders. ✅
- Terms / Privacy / Account deletion:
  - Terms and Privacy pages exist in frontend (`/terms`).
  - Account deletion via `DELETE /api/me` confirmed in `community.rs`. ✅
  - PDPA notice referenced in README and terms. ✅

**Phase 6 — Final check** ✅
- `cargo fmt --check` ✅
- `cargo clippy --all-targets -- -D warnings` ✅ (fixed two stale imports after schedules extraction)
- `DEV_MODE=true cargo test` → 283 passed ✅
- `npm run lint` → No issues ✅
- `npm run typecheck` ✅
- `npm test -- --run` → 315 passed ✅
- `npm run build` ✅

---

## Decisions Made

| Decision | Chosen | Reason |
|---|---|---|
| Rides sub-module split | Completed (proxy, places, schedules) | Done incrementally; driver/passenger/staff deferred (heavy cross-refs) |
| `cargo audit` chrono warnings | Suppressed in audit.toml | Transitive via SQLx; not actionable without major SQLx upgrade |
| Sentry integration | Completed | Opt-in via env var; no PII in payloads |
| LICENSE file | Private proprietary — PJ State CMC church only | User confirmed |

---

## Open Questions / Items Waiting on User Decision

1. **Further rides splits** — `rides/mod.rs` is still 3 512 lines. The remaining hard splits:
   - `driver.rs` (~1 357 lines) — calls `assign_trip`/`announce_confirmed` shared with `staff.rs`
   - `passenger.rs` (~1 127 lines)
   - `staff.rs` (~533 lines) — calls `assign_trip` from driver section
   - `chat.rs` (~90 lines) — cleanest to extract, low risk
   Recommendation: extract `chat.rs` first if further splitting is desired.

2. **community.rs / admin.rs** — both > 2 000 lines. Same incremental split approach applies.

3. **Sentry DSN** — provide real DSNs for staging and production via `.env`.

---

## Bugs Fixed

| Commit | Bug |
|---|---|
| `9c3e195` | VAPID public key could not be set on fresh install — `update_alert_settings` always returned `INVALID_ACTION` when `current_pub` was `None` |
| `9f1825c` | Removed stray `println!("DEBUG: ...")` from `admin.rs` |
| `fc92738` | Stale tests: LIKE wildcard escaping, TOTP guard order, paged admin search, demand report, proxy trip routing endpoint |
| `fdffea5` | Stale `use chrono::Datelike` and `use serde::Serialize` in `rides/mod.rs` after sub-module extraction |

