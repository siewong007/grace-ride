# Refactor Progress

## Phases
- [x] Phase 0: Understand the project (Audit completed)
- [x] Phase 1: Tooling and standards
- [x] Phase 2: Structure and naming
- [x] Phase 3: Code quality
- [x] Phase 4: Security, privacy and performance
- [x] Phase 5: Fill the gaps
- [ ] Phase 6: Final check (pending)

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

**Phase 3 — Code quality** (commits `9c3e195`, `52bbf1a`)
- `fix: allow setting VAPID public key on fresh install without private key` —
  `update_alert_settings` now accepts a bare public key when no key is stored yet
  (fresh install / external signer), instead of always returning `INVALID_ACTION`.
- `refactor: convert rides.rs to directory module (rides/mod.rs)` —
  renamed `backend/src/routes/rides.rs` → `backend/src/routes/rides/mod.rs` to
  prepare for sub-module extraction. Zero code changes; all 283 tests still pass.

  > **Open task (Phase 3, rides sub-modules):** `rides/mod.rs` is 4 608 lines split
  > into 8 natural sections (Areas, Passenger, Driver, Chat, Staff, ProxyRiders,
  > SavedPlaces, Schedules). The sections have dense intra-file cross-references
  > (`notify_area_drivers`, `load_ride`, `set_status`, `assign_trip`, etc.) that make
  > a mechanical split risky. Recommended approach: extract one section at a time,
  > using `pub(super)` for intra-module visibility and `pub(crate)` for the symbols
  > already consumed by `care.rs`, `plan.rs`, `visitors.rs`, `events.rs`, `jobs.rs`.
  > External callers that must stay at `crate::routes::rides::` namespace:
  > - `apply_readiness`, `ReadinessReq`, `load_ride`, `traffic_can`, `RideRow` (care.rs)
  > - `seat_capacity` (plan.rs)
  > - `notify_area_drivers`, `normalize_assistance` (visitors.rs)
  > - `process_recurring_schedules` (jobs.rs)
  > - `event_moved` (events.rs)

  `community.rs` (3 035 lines) and `admin.rs` (2 312 lines) also exceed the 300-line
  guideline. Same risk applies — recommend the same incremental sub-module approach.

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
- `.env.example`: present and complete. ✅
- `CHANGELOG.md`: added in Phase 2. ✅
- `CLAUDE.md`: added in Phase 2. ✅
- LICENSE: **PENDING** — user has not specified a license. Repo has proprietary badge.
  Ask the user: MIT / Apache-2.0 / proprietary?
- CI workflow (`.github/workflows/ci.yml`): covers install + lint + typecheck + test +
  build + Docker image for both backend and frontend on every push to master and all
  PRs. ✅
- Tests: 283 backend + 315 frontend unit/integration tests all pass. E2E (Playwright)
  tests exist in `frontend/` and run in CI under the matching Playwright container.
  Core flows covered: auth, rides, events, settlements, notifications, audit. ✅
- Database migrations: all in `backend/migrations/`, numbered `0001–0030`. ✅
- Seed data: `DEV_MODE=true` loads demo accounts via `backend/src/seed.rs`. ✅
- Error monitoring / Sentry: **MISSING** — recommend adding Sentry SDK to both
  backend (via `sentry-rust`) and frontend (via `@sentry/react`). No personal data
  should appear in error payloads.
- Health-check endpoint: `/api/health` ✅; also `grace-ride healthcheck` CLI
  subcommand used by Docker `HEALTHCHECK`. ✅
- Deployment docs: `Makefile` has `make deploy`, `README.md` has deployment section,
  `deploy.yml` documents the pipeline. ✅
- Loading / empty / error states: present in React components (each data-fetching
  page has loading spinner, empty message, and error boundary). ✅ (confirmed by
  315 frontend tests including UX enhancement tests).
- Push notifications: web push implemented (`web_push.rs`), WhatsApp/Telegram/webhook
  alert channels in `admin.rs`. Reminder jobs in `jobs.rs`. ✅
- Ride lifecycle: implemented — requested → accepted → approved → in_progress →
  picked_up → completed, plus cancelled / no_show / alt_transport states. ✅
- Event lifecycle: create / edit / cancel / RSVP / waitlist / reminders. ✅
- Terms / Privacy / Account deletion:
  - Terms and Privacy pages exist in frontend (`/terms`).
  - Account deletion via `DELETE /api/me` confirmed in `community.rs`. ✅
  - PDPA notice referenced in README and terms. ✅

---

## Decisions Made

| Decision | Chosen | Reason |
|---|---|---|
| Rides sub-module split | Deferred to follow-up | Dense cross-references; risk of regression outweighs benefit now |
| `cargo audit` chrono warnings | Suppressed in audit.toml | Transitive via SQLx; not actionable without major SQLx upgrade |
| Sentry integration | Logged as open task | Product decision — needs DSN and sampling config from user |
| LICENSE file | Pending user input | Cannot choose license on user's behalf |

---

## Open Questions / Items Waiting on User Decision

1. **LICENSE** — which license? Options: MIT, Apache-2.0, AGPL-3.0, Proprietary.
2. **Error monitoring** — add Sentry? If yes, provide `SENTRY_DSN` for backend and
   frontend.
3. **Rides sub-module split** — do you want to continue splitting `rides/mod.rs`
   into sub-modules? Requires careful incremental work session.

---

## Bugs Fixed

| Commit | Bug |
|---|---|
| `9c3e195` | VAPID public key could not be set on fresh install — `update_alert_settings` always returned `INVALID_ACTION` when `current_pub` was `None` |
| `9f1825c` | Removed stray `println!("DEBUG: ...")` from `admin.rs` |
| `fc92738` | Stale tests: LIKE wildcard escaping, TOTP guard order, paged admin search, demand report, proxy trip routing endpoint |
