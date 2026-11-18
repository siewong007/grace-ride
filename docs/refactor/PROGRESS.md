# Refactor Progress

## ✅ COMPLETE — Branch `refactor/modularize-routes` ready for review

**Final status (2026-11-18):**
- `cargo fmt --check` ✅
- `cargo clippy --all-targets -- -D warnings` ✅
- `DEV_MODE=true cargo test` → 283 passed, 35 suites ✅
- `npm run lint` → ESLint: No issues ✅
- `npm run typecheck` → No errors ✅
- `npm test -- --run` → 314 passed, 49 files ✅
- `npm run build` → dist built ✅

**5 commits ahead of `master`** on `refactor/modularize-routes` (prior refactor work `#41` + initial `rides/` split + Sentry were already merged into `origin/master`).

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
- **Community sub-module split (complete)** — converted `community.rs` (3 035 lines) into
  `backend/src/routes/community/` with 10 domain sub-modules (`profile.rs`, `calendar.rs`,
  `push.rs`, `notifications.rs`, `announcements.rs`, `help.rs`, `guides.rs`,
  `reimbursements.rs`, `payout_batches.rs`, `reports.rs`) and `community/mod.rs` (120 lines);
  removed 4 redundant local `#[allow(clippy::type_complexity)]` attributes.
- **Admin sub-module split (complete)** — converted `admin.rs` (2 312 lines) into
  `backend/src/routes/admin/` with 6 domain sub-modules (`members.rs` 526,
  `invites_recovery.rs` 276, `groups_areas.rs` 331, `audit.rs` 358, `reports.rs` 307,
  `settings.rs` 473) and `admin/mod.rs` (165 lines). All 283 backend tests across 35
  suites pass.
- **`RideDetail.tsx` modularization (complete)** — extracted sub-components into
  `frontend/src/components/ride/` (`RideInfoCards.tsx` 342, `RideTracking.tsx` 276,
  `RideActionPanels.tsx` 487, `CareLinkModal.tsx` 98), reducing `pages/RideDetail.tsx`
  from 1 342 lines to 205 lines. Removed `// eslint-disable-line react-hooks/exhaustive-deps`
  in `PassengerShareLocation` using a ref for `onChange`. Frontend lint, typecheck, all
  314 tests (49 files), and production build pass.

**Phase 3b — Secondary Mega-File Splits**
- **Auth sub-module split (complete)** — converted `backend/src/routes/auth.rs`
  (1 644 lines) into `backend/src/routes/auth/` (`mod.rs` core auth/session/router,
  `passkeys.rs`, `totp.rs`, `invites_recovery.rs`) with zero behaviour changes;
  re-exported `TERMS_VERSION`; `cargo fmt`, `cargo clippy`, and `cargo test --lib`
  (43 passed) pass.
- **Events sub-module split (complete)** — converted `backend/src/routes/events.rs`
  (1 601 lines) into `backend/src/routes/events/` (`mod.rs` listing/lookup/visibility/router,
  `manage.rs`, `signups.rs`, `checkin.rs`); `cargo fmt` and `cargo clippy --all-targets`
  pass with zero issues.
- **Routing sub-module split (complete)** — converted `backend/src/routing.rs`
  (1 590 lines) into `backend/src/routing/` (`mod.rs` config/state/geometry, `order.rs`,
  `geocode.rs`, `trip.rs`, `tests.rs`) preserving all `pub` / `pub(crate)` re-exports;
  `cargo fmt`, `cargo clippy`, and `cargo test --lib` (43 passed) pass.
- **Settlements sub-module split (complete)** — converted `backend/src/routes/settlements.rs`
  (1 075 lines) into `backend/src/routes/settlements/` (`mod.rs` router/listing/summary,
  `manage.rs`, `shares.rs`); `cargo fmt`, `cargo clippy --all-targets`, and full backend
  tests (283 passed across 35 suites) pass with zero issues.

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
  in `ride_json` (`backend/src/routes/rides/mod.rs`);
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
- Tests: 283 backend + 314 frontend unit/integration tests all pass. E2E (Playwright)
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
  alert channels in `admin/settings.rs`. Reminder jobs in `jobs.rs`. ✅
- Ride lifecycle: implemented — requested → accepted → approved → in_progress →
  picked_up → completed, plus cancelled / no_show / alt_transport states. ✅
- Event lifecycle: create / edit / cancel / RSVP / waitlist / reminders. ✅
- Terms / Privacy / Account closure:
  - 16-section trilingual (EN/ZH/MS) Terms and Privacy Notice in
    `frontend/src/components/Terms.tsx` (`/terms`), enforced via `TermsGate`. ✅
  - Account closure is handled via staff deactivation (`PUT /api/admin/users/:id`) or
    Help Center request per church record-retention policy in `terms.*.ts`.

**Phase 6 — Final check** ✅
- `cargo fmt --check` ✅
- `cargo clippy --all-targets -- -D warnings` ✅
- `DEV_MODE=true cargo test` → 283 passed (35 suites) ✅
- `npm run lint` → ESLint: No issues ✅
- `npm run typecheck` ✅
- `npm test -- --run` → 314 passed (49 files) ✅
- `npm run build` ✅

---

## Decisions Made

| Decision | Chosen | Reason |
|---|---|---|
| Rides sub-module split | Completed (`mod.rs` 405 lines + 10 sub-modules) | Split across two passes with `pub use` / `pub(crate) use` re-exports |
| Community & Admin splits | Completed (`community/` 10 sub-modules, `admin/` 6 sub-modules) | Broke apart 3 035-line and 2 312-line route files cleanly |
| `RideDetail.tsx` split | Completed (`components/ride/*` + ref fix for `exhaustive-deps`) | Reduced page file from 1 342 to 205 lines; removed lint suppression |
| `cargo audit` chrono warnings | Suppressed in audit.toml | Transitive via SQLx; not actionable without major SQLx upgrade |
| Sentry integration | Completed | Opt-in via env var; no PII in payloads |
| LICENSE file | Private proprietary — PJ State CMC church only | User confirmed |

---

## Open Questions / Items Waiting on User Decision

1. **Sentry DSN** — provide real `SENTRY_DSN` (backend) and `VITE_SENTRY_DSN` (Vite build env) for staging and production via `.env`.
2. **Optional product enhancements** — automated driver/rider ID verification and self-service account deletion (`DELETE /api/me`, currently handled via Help Center / admin deactivation) remain product decisions.
3. **Optional secondary file splits** — remaining larger files (`events.rs`, `auth.rs`, `EventDetail.tsx`, `Drive.tsx`, `Audit.tsx`) can be split incrementally in future work if desired.

---

## Bugs Fixed

| Commit | Bug |
|---|---|
| `9c3e195` | VAPID public key could not be set on fresh install — `update_alert_settings` always returned `INVALID_ACTION` when `current_pub` was `None` |
| `9f1825c` | Removed stray `println!("DEBUG: ...")` from `admin.rs` |
| `fc92738` | Stale tests: LIKE wildcard escaping, TOTP guard order, paged admin search, demand report, proxy trip routing endpoint |
| `fdffea5` | Stale `use chrono::Datelike` and `use serde::Serialize` in `rides/mod.rs` after sub-module extraction |

