# Grace Ride — Technical Standards

> **For AI sessions:** Read this file first. It tells you the stack, conventions, and commands
> so you don't have to rediscover them. Update it whenever the structure changes.

---

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust stable · Axum 0.8 · SQLx 0.9 · Tokio |
| Database | PostgreSQL 18 · integer cents · KL local-time text timestamps |
| Frontend | React 19 · TypeScript 6 · Vite 8 · i18next · Leaflet |
| Auth | WebAuthn passkeys · TOTP · backup codes · Argon2 password hash |
| Infra | Docker (multi-stage) · Caddy · GitHub Actions CI/CD |
| Package managers | cargo (backend) · npm (frontend) |

---

## Naming Conventions

### Rust (backend)
- `snake_case` — functions, variables, modules, file names
- `PascalCase` — types, structs, enums, traits
- `UPPER_SNAKE_CASE` — global constants
- Module files follow the route domain: `rides/mod.rs`, `events.rs`, `admin.rs`, etc.

### TypeScript / React (frontend)
- `camelCase` — variables, functions, hooks
- `PascalCase` — React components, types, interfaces
- `UPPER_SNAKE_CASE` — true compile-time constants
- Component files: `PascalCase.tsx`; utility/lib files: `camelCase.ts`

### Database
- `snake_case` tables and columns; plural table names (`rides`, `events`, `users`)
- Every table: `id BIGSERIAL PRIMARY KEY`, `created_at TEXT`, `updated_at TEXT`
- Booleans stored as `INTEGER` (0/1) for SQLite compatibility pattern; SQLx maps them

---

## Folder Structure

```
/
├── backend/
│   ├── src/
│   │   ├── routes/          # Axum route handlers (one file or dir per domain)
│   │   │   ├── rides/               # directory module for all ride routes
│   │   │   │   ├── mod.rs           # router, RideRow, visibility helpers, areas/places (405 lines)
│   │   │   │   ├── chat.rs          # trip group chat messages, emoji reactions, read receipts
│   │   │   │   ├── driver.rs        # claim, withdraw, confirm, start, pickup, no-show, complete
│   │   │   │   ├── driver_profile.rs # requests board, schedule, route, driver/vehicle profile
│   │   │   │   ├── location.rs      # live driver & passenger location sharing
│   │   │   │   ├── passenger.rs     # booking, listing, detail, cancel, offer response
│   │   │   │   ├── places.rs        # saved-places CRUD
│   │   │   │   ├── proxy.rs         # frequent proxy-riders CRUD
│   │   │   │   ├── readiness.rs     # readiness, driver delay, gate arrival + unit tests
│   │   │   │   ├── schedules.rs     # recurring schedules, event_moved, process_recurring_schedules
│   │   │   │   └── staff.rs         # staff approve, return, reassign, alt-transport, admin views
│   │   │   ├── admin/               # directory module for admin panel routes
│   │   │   │   ├── mod.rs           # admin router & dashboard overview (165 lines)
│   │   │   │   ├── audit.rs         # audit log query & streaming CSV export
│   │   │   │   ├── groups_areas.rs  # groups, group members & pickup areas CRUD
│   │   │   │   ├── invites_recovery.rs # invite links & account recovery approvals
│   │   │   │   ├── members.rs       # member directory, roles, status & group assignments
│   │   │   │   ├── reports.rs       # demand breakdown report & scope helpers
│   │   │   │   └── settings.rs      # reimbursement rates, system & alert settings
│   │   │   ├── auth/                # directory module for authentication routes
│   │   │   │   ├── mod.rs           # auth router, login, session, password, terms
│   │   │   │   ├── passkeys.rs      # WebAuthn registration and authentication
│   │   │   │   ├── totp.rs          # TOTP setup, verify, backup codes
│   │   │   │   └── invites_recovery.rs # invitation accept & account recovery request
│   │   │   ├── care.rs              # /care/:token safe-arrival links
│   │   │   ├── community/           # directory module for community & finance routes
│   │   │   │   ├── mod.rs           # router & LIKE escaping helper (120 lines)
│   │   │   │   ├── announcements.rs # announcements CRUD
│   │   │   │   ├── calendar.rs      # live .ics calendar feed
│   │   │   │   ├── guides.rs        # workflow guides CRUD, feedback & ordering
│   │   │   │   ├── help.rs          # help center tickets, replies & claiming
│   │   │   │   ├── notifications.rs # in-app notifications inbox
│   │   │   │   ├── payout_batches.rs # monthly finance payout batches & CSV export
│   │   │   │   ├── profile.rs       # /me profile update
│   │   │   │   ├── push.rs          # VAPID config & web-push subscriptions
│   │   │   │   ├── reimbursements.rs # Grab reimbursement claims, review & single payout
│   │   │   │   └── reports.rs       # monthly activity reports & badges
│   │   │   ├── events/              # directory module for events routes
│   │   │   │   ├── mod.rs           # event listing, detail, visibility, router
│   │   │   │   ├── manage.rs        # event CRUD, duplication, cancel, export
│   │   │   │   ├── signups.rs       # RSVP signups, waitlist promotion, withdraw
│   │   │   │   └── checkin.rs       # code, QR, and member pass scan check-in
│   │   │   ├── settlements/         # directory module for fuel settlements
│   │   │   │   ├── mod.rs           # settlement listing, detail, summary, router
│   │   │   │   ├── manage.rs        # settlement CRUD, close, revise, publish
│   │   │   │   └── shares.rs        # passenger share proof & confirmation
│   │   │   ├── plan.rs              # transport planning board
│   │   │   └── visitors.rs          # one-time guest ride pass
│   │   ├── auth.rs          # session / credential helpers
│   │   ├── audience.rs      # recursive group audience logic
│   │   ├── error.rs         # AppError, AppResult, ErrorKey
│   │   ├── i18n/            # bilingual error messages (zh-CN / en)
│   │   ├── jobs.rs          # housekeeping and reminder background jobs
│   │   ├── lib.rs           # AppState wiring, top-level router
│   │   ├── main.rs          # entry point
│   │   ├── perms.rs         # 67 permissions, 27 pages, 5-tier role hierarchy
│   │   ├── routing/         # OSRM geocoder / router integration directory module
│   │   │   ├── mod.rs       # config, routing state, Haversine geometry
│   │   │   ├── order.rs     # pickup/dropoff trip stop sequencing
│   │   │   ├── geocode.rs   # geocoding client & Nominatim queries
│   │   │   ├── trip.rs      # trip driving distance/duration calculations
│   │   │   └── tests.rs     # geometry and ordering unit tests
│   │   ├── seed.rs          # demo data for DEV_MODE
│   │   ├── state.rs         # Config, AppState, DbConfig
│   │   ├── util.rs          # audit log, notify, fmt helpers, ErrorKey
│   │   └── web_push.rs      # VAPID web-push implementation
│   ├── migrations/          # numbered SQL migrations (0001–0014)
│   ├── tests/               # integration tests (DEV_MODE=true required)
│   ├── .cargo/audit.toml    # suppressed cargo-audit advisories with justification
│   ├── rust-toolchain.toml  # pins Rust stable + clippy + rustfmt
│   └── .env.example         # all env vars with comments
├── frontend/
│   ├── src/
│   │   ├── pages/           # one file per route (PascalCase, e.g. EventDetail, Drive, admin/Audit)
│   │   ├── components/      # shared React components
│   │   │   ├── ride/        # sub-components for RideDetail
│   │   │   ├── event/       # QR & scanner modals and types for EventDetail
│   │   │   └── drive/       # schedule, requests, and settings tabs for Drive
│   │   ├── locales/         # i18next namespace JSON files (en, zh)
│   │   ├── hooks/           # custom React hooks
│   │   ├── types/           # TypeScript type declarations
│   │   ├── api.ts           # typed fetch wrappers
│   │   ├── auth.tsx         # auth context and session management
│   │   ├── perms.ts         # permission helpers (mirrors backend perms.rs)
│   │   └── styles.css       # design tokens (colour, spacing, typography)
│   └── .nvmrc               # pins Node 22
├── docs/
│   ├── TECHNICAL.md         # authoritative technical reference (single-manual policy)
│   ├── refactor/
│   │   ├── AUDIT.md         # problem audit + Phase 5 checklist
│   │   └── PROGRESS.md      # phase log, decisions, open questions
│   └── brand/               # logo kit, assets
├── .github/workflows/
│   ├── ci.yml               # lint + typecheck + test + build on push/PR
│   └── deploy.yml           # auto-deploy to VPS on passing CI on master
├── CHANGELOG.md
├── CLAUDE.md                # (this file)
├── LICENSE                  # Private proprietary — PJ State CMC church use
├── Makefile                 # dev, test, build, deploy, audit targets
└── docker-compose.yml       # local dev stack (Postgres + app)
```

---

## Commands

| Goal | Command |
|---|---|
| Start API (dev) | `make dev-api` |
| Start frontend (dev) | `make dev-web` |
| Lint everything | `make lint` |
| Format everything | `make fmt` |
| Type-check | `cd frontend && npm run typecheck` |
| Run all tests | `make test` |
| Run backend tests only | `cd backend && DEV_MODE=true cargo test --locked` |
| Run frontend tests only | `cd frontend && npm test` |
| Production build | `make build` |
| Dependency audit | `make audit` |
| DB migrate (dev) | `make migrate` |
| DB up (Docker) | `make db-up` |

---

## Key Conventions

- **Env vars:** validated at startup in `Config::from_env()` / `DbConfig::from_env()`. Every var is in `backend/.env.example` with a comment.
- **Money:** integer cents (`i64`). Never floats.
- **Timestamps:** local KL text (`LOCALTIMESTAMP AT TIME ZONE ...`). Use `norm_dt()` helper to format.
- **Errors:** `AppResult<T>` = `Result<T, AppError>`. Use `bad()`, `forbidden()`, `not_found()`, `conflict()` helpers. Never panic in handlers.
- **Permissions:** check via `crate::perms::Perm::require(&u, Page::X, Act::Y)?` in every handler. Never bypass.
- **Personal data:** pickup address and phone visible only to the passenger, their assigned driver (once approved), and staff. `ride_json()` enforces this.
- **Secrets:** Telegram token, VAPID keys stored in `app_secrets` DB table, never in env or source.
- **i18n:** all user-facing text in `frontend/src/locales/`. Backend error messages in `backend/src/i18n/`.
- **Booleans in SQL:** stored as `INTEGER 0/1`.
- **Dynamic SQL (sqlx 0.9):** `sqlx::query*` only accepts `&'static str`. Build SQL that interpolates fixed fragments (column lists, placeholders, visibility clauses) with `crate::dyn_sql!(...)`; keep every user value in `.bind()`. Use `sqlx::AssertSqlSafe(...)` for an already-built `String`.
- **Stubs:** all stub files have been deleted. Do not add new stub-only files.

---

## Open Items (as of 2026-11-18)

1. **LICENSE** ✅ — private proprietary, church-only use (PJ State CMC). Already in repo.
2. **Sentry** ✅ — `sentry` crate (backend) + `@sentry/react` (frontend) integrated. Activate by setting `SENTRY_DSN` / `VITE_SENTRY_DSN` env vars.
3. **Mega-module splits** ✅ — `rides/` (`mod.rs` 405 lines + 10 sub-modules), `community/` (`mod.rs` 120 lines + 10 sub-modules), `admin/` (`mod.rs` 165 lines + 6 sub-modules), and `RideDetail.tsx` (205 lines + 4 components under `components/ride/`) are all split.
4. **Driver/rider verification** — documented approach only; no automated ID check implemented yet.
