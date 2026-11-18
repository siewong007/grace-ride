# Grace Ride — Technical Standards

> **For AI sessions:** Read this file first. It tells you the stack, conventions, and commands
> so you don't have to rediscover them. Update it whenever the structure changes.

---

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust stable · Axum 0.8 · SQLx 0.8 · Tokio |
| Database | PostgreSQL 18 · integer cents · KL local-time text timestamps |
| Frontend | React 18 · TypeScript 5 · Vite 6 · i18next · Leaflet |
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
│   │   │   ├── rides/       # rides/mod.rs — 4 608-line directory module
│   │   │   │   └── mod.rs   # (sub-modules planned; see PROGRESS.md)
│   │   │   ├── admin.rs     # admin panel routes (2 312 lines)
│   │   │   ├── auth.rs      # passkeys, TOTP, password, sessions
│   │   │   ├── care.rs      # /care/:token safe-arrival links
│   │   │   ├── community.rs # profile, notifications, push, finance (3 035 lines)
│   │   │   ├── events.rs    # events, RSVP, waitlist, QR check-in
│   │   │   ├── plan.rs      # transport planning board
│   │   │   └── visitors.rs  # one-time guest ride pass
│   │   ├── auth.rs          # session / credential helpers
│   │   ├── audience.rs      # recursive group audience logic
│   │   ├── error.rs         # AppError, AppResult, ErrorKey
│   │   ├── i18n/            # bilingual error messages (zh-CN / en)
│   │   ├── jobs.rs          # housekeeping and reminder background jobs
│   │   ├── lib.rs           # AppState wiring, top-level router
│   │   ├── main.rs          # entry point
│   │   ├── perms.rs         # 67 permissions, 27 pages, 5-tier role hierarchy
│   │   ├── routing.rs       # OSRM geocoder / router integration
│   │   ├── seed.rs          # demo data for DEV_MODE
│   │   ├── state.rs         # Config, AppState, DbConfig
│   │   ├── util.rs          # audit log, notify, fmt helpers, ErrorKey
│   │   └── web_push.rs      # VAPID web-push implementation
│   ├── migrations/          # numbered SQL migrations (0001–0030)
│   ├── tests/               # integration tests (DEV_MODE=true required)
│   ├── .cargo/audit.toml    # suppressed cargo-audit advisories with justification
│   ├── rust-toolchain.toml  # pins Rust stable + clippy + rustfmt
│   └── .env.example         # all env vars with comments
├── frontend/
│   ├── src/
│   │   ├── pages/           # one file per route (PascalCase)
│   │   ├── components/      # shared React components
│   │   ├── locales/         # i18next namespace JSON files (en, ms)
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
├── LICENSE                  # Pending — not yet chosen
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
- **Stubs:** all stub files have been deleted. Do not add new stub-only files.

---

## Open Items (as of 2026-11-18)

1. **LICENSE** — not yet chosen. Ask the user.
2. **Sentry** — no error monitoring. Recommend `sentry-rust` (backend) + `@sentry/react` (frontend).
3. **Rides sub-module split** — `rides/mod.rs` is 4 608 lines. See PROGRESS.md for the split plan and the external callers that need re-exporting.
4. **Driver/rider verification** — documented approach only; no automated ID check implemented yet.
