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
| Android | Kotlin 2.2 · Jetpack Compose · Material 3 · MVVM |
| iOS | Swift 5.10 · SwiftUI (iOS 17+) · `GraceRideKit` SPM · WidgetKit + AppIntents |
| Auth | WebAuthn passkeys · TOTP · backup codes · Argon2 password hash |
| Infra | Docker (multi-stage) · Caddy · GitHub Actions CI/CD |
| Package managers | cargo (backend) · npm (frontend) · gradle (android) · SwiftPM / Xcode (ios) |

---

## Naming Conventions

### Rust (backend)
- `snake_case` — functions, variables, modules, file names
- `PascalCase` — types, structs, enums, traits
- `UPPER_SNAKE_CASE` — global constants
- Module files follow the route domain: `rides/mod.rs`, `events/mod.rs`, `admin/mod.rs`, etc.

### TypeScript / React (frontend)
- `camelCase` — variables, functions, hooks
- `PascalCase` — React components, types, interfaces
- `UPPER_SNAKE_CASE` — true compile-time constants
- Component files: `PascalCase.tsx`; utility/lib files: `camelCase.ts`

### Database
- `snake_case` tables and columns; plural table names (`rides`, `events`, `users`)
- Every table: `id BIGSERIAL PRIMARY KEY`, `created_at TEXT`, `updated_at TEXT`
- Booleans stored as `BIGINT` (`0`/`1`) on legacy/domain tables (`BOOLEAN` on `geocode_cache.approx`, `guide_workflows.published`, `guide_feedback.helpful`); SQLx maps them

---

## Folder Structure

```
/
├── backend/
│   ├── src/
│   │   ├── routes/          # Axum route handlers (one file or dir per domain)
│   │   │   ├── rides/               # directory module for all ride routes
│   │   │   │   ├── mod.rs           # router, RideRow, visibility helpers, areas/places
│   │   │   │   ├── chat.rs          # trip group chat messages, emoji reactions, read receipts
│   │   │   │   ├── drafts.rs        # single-ride change drafts, 10-min lease locks, lead approval
│   │   │   │   ├── driver.rs        # claim, withdraw, confirm, start, pickup, no-show, complete
│   │   │   │   ├── driver_profile.rs # requests board, schedule, route, driver & multi-vehicle profile
│   │   │   │   ├── location.rs      # live driver & passenger location sharing
│   │   │   │   ├── passenger.rs     # booking, listing, detail, cancel, offer response
│   │   │   │   ├── places.rs        # saved-places CRUD (with lat/lng persistence)
│   │   │   │   ├── proxy.rs         # frequent proxy-riders CRUD
│   │   │   │   ├── readiness.rs     # readiness, supper choice, driver delay, gate arrival + unit tests
│   │   │   │   ├── schedules.rs     # recurring schedules, event_moved, process_recurring_schedules
│   │   │   │   └── staff.rs         # staff approve, return, reassign, alt-transport, reimb exclusion, admin views
│   │   │   ├── admin/               # directory module for admin panel routes
│   │   │   │   ├── mod.rs           # admin router & dashboard overview
│   │   │   │   ├── audit.rs         # audit log query & streaming CSV export
│   │   │   │   ├── fares.rs         # fixed area-to-area fares & bulk XLSX/CSV import/export
│   │   │   │   ├── groups_areas.rs  # groups, group members & pickup areas CRUD
│   │   │   │   ├── invites_recovery.rs # invite links & account recovery approvals
│   │   │   │   ├── members.rs       # member directory, roles, status & group assignments
│   │   │   │   ├── reports/         # role-scoped admin reports directory module (mod, overview, events, demand, finance)
│   │   │   │   └── settings.rs      # system retention/concurrency & external alert channel settings
│   │   │   ├── auth/                # directory module for authentication routes
│   │   │   │   ├── mod.rs           # auth router, login, session, password, terms
│   │   │   │   ├── biometrics.rs    # Android hardware-backed ECDSA P-256 biometric login & reauth
│   │   │   │   ├── passkeys.rs      # WebAuthn registration and authentication
│   │   │   │   ├── totp.rs          # TOTP setup, verify, backup codes
│   │   │   │   └── invites_recovery.rs # invitation accept & account recovery request
│   │   │   ├── care.rs              # /care/:token safe-arrival links (24 h pre-pickup window)
│   │   │   ├── community/           # directory module for community & finance routes
│   │   │   │   ├── mod.rs           # router & LIKE escaping helper
│   │   │   │   ├── announcements.rs # announcements CRUD
│   │   │   │   ├── calendar.rs      # live .ics calendar feed
│   │   │   │   ├── family.rs        # member family lists CRUD & invite links
│   │   │   │   ├── guides.rs        # workflow guides CRUD, feedback & ordering
│   │   │   │   ├── help.rs          # help center tickets, replies & claiming
│   │   │   │   ├── notifications.rs # in-app notifications inbox
│   │   │   │   ├── payout_batches.rs # monthly finance payout batches & CSV export
│   │   │   │   ├── profile.rs       # /me profile, avatar & Grok Bot Studio validation
│   │   │   │   ├── push.rs          # VAPID/alert config & web-push subscriptions
│   │   │   │   ├── reimbursements.rs # Grab/mileage/visitor_special claims, review & single payout
│   │   │   │   └── reports.rs       # monthly/quarterly/yearly activity reports & 4-tier badges
│   │   │   ├── area_photos.rs       # pickup area photo management & EXIF stripping
│   │   │   ├── events/              # directory module for events routes
│   │   │   │   ├── mod.rs           # event listing, detail, visibility, router
│   │   │   │   ├── manage.rs        # event CRUD, draft/publish/unpublish, icons, duplication, cancel
│   │   │   │   ├── post_event.rs    # post-event driver dispersal & supper reimbursements
│   │   │   │   ├── signups.rs       # RSVP signups, self_transport, waitlist promotion, withdraw
│   │   │   │   └── checkin.rs       # code, QR, and member pass scan check-in
│   │   │   ├── files.rs             # magic-byte validated file upload/download & receipt ref extraction
│   │   │   ├── mod.rs               # top-level /api router wiring
│   │   │   ├── roles.rs             # role & permission CRUD with 5-tier rank invariants
│   │   │   ├── settlements/         # directory module for fuel/activity settlements
│   │   │   │   ├── mod.rs           # settlement listing, detail, summary, router
│   │   │   │   ├── manage.rs        # settlement CRUD, close, revise, publish
│   │   │   │   └── shares.rs        # passenger share proof & confirmation
│   │   │   ├── plan/                # transport planning board, multi-version drafts, lease locks, Traffic Hub plans
│   │   │   │   ├── mod.rs           # router, board loaders, list_traffic_plans, get_plan, arrange_plan
│   │   │   │   ├── drafts.rs        # draft CRUD, 10-min lease locks, request-approval, return
│   │   │   │   └── apply.rs         # transactional plan apply & approval
│   │   │   ├── plan_visitors.rs     # plan visitors (for the record) & visitor special reimbursements
│   │   │   └── visitors.rs          # one-time guest ride pass
│   │   ├── arrange.rs       # accessibility-aware event transport auto-arrange heuristics
│   │   ├── auth.rs          # session / credential helpers
│   │   ├── audience.rs      # recursive group audience logic
│   │   ├── error.rs         # AppError, AppResult, ErrorKey
│   │   ├── i18n/            # bilingual error messages (zh-CN / en)
│   │   ├── jobs.rs          # housekeeping, orphan-file GC, and reminder background jobs
│   │   ├── lib.rs           # AppState wiring, top-level router
│   │   ├── main.rs          # entry point & Sentry ErrorLayer + PII scrubber
│   │   ├── notifications/   # centralized notification engine directory module
│   │   │   ├── mod.rs       # dispatch & formatting entry points
│   │   │   ├── meta.rs      # categories, priorities, deep-link inference & unit tests
│   │   │   ├── ssrf.rs      # DNS-pinned SSRF guards & HMAC-SHA256 webhook signing
│   │   │   └── outbox.rs    # durable notification_deliveries outbox & channel workers
│   │   ├── perms.rs         # 73 permissions, 29 pages, 14 built-in roles, 5-tier role hierarchy
│   │   ├── routing/         # OSRM geocoder / router integration directory module
│   │   │   ├── mod.rs       # config, routing state, Haversine geometry
│   │   │   ├── order.rs     # pickup/dropoff trip stop sequencing
│   │   │   ├── geocode.rs   # geocoding client & Nominatim queries
│   │   │   ├── trip.rs      # trip driving distance/duration calculations
│   │   │   └── tests.rs     # geometry and ordering unit tests
│   │   ├── security.rs      # rate limiters, session lock, CSRF/CSP headers
│   │   ├── seed.rs          # demo data for DEV_MODE
│   │   ├── state.rs         # Config, AppState, DbConfig
│   │   ├── stream.rs        # authenticated SSE broadcast channel
│   │   ├── turnstile.rs     # opt-in Cloudflare Turnstile siteverify verifier & config
│   │   ├── util.rs          # audit log, notify, require/require_one, fmt helpers, dyn_sql!
│   │   └── web_push.rs      # VAPID web-push implementation
│   ├── migrations/          # squashed 0001_init.sql baseline + additive migrations (0002–0013)
│   ├── tests/               # 49 integration test files (uses TEST_DATABASE_URL; isolated schema per test)
│   ├── .cargo/audit.toml    # suppressed cargo-audit advisories with justification
│   ├── rust-toolchain.toml  # pins Rust stable + clippy + rustfmt
│   └── .env.example         # all env vars with comments
├── frontend/
│   ├── src/
│   │   ├── pages/           # one file per route (PascalCase, e.g. EventDetail, Drive, GrokBotStudioPage, admin/TrafficHub)
│   │   ├── components/      # shared React components
│   │   │   ├── ride/        # sub-components for RideDetail
│   │   │   ├── event/       # timeline, QR & scanner modals and types for EventDetail
│   │   │   ├── events/      # post-event driver dispersal reimbursement modal
│   │   │   ├── drive/       # schedule, requests, and settings tabs for Drive
│   │   │   ├── ui/          # shared UI primitives (re-exported via components/ui.tsx)
│   │   │   └── ...          # AreaPhotos, AvatarEditor, GrokBotSvg/Controls, LiveMap, LocationPinPicker, TurnstileWidget, Chat, etc.
│   │   ├── locales/         # i18next core bundles (core.{en,zh}.ts) + route-lazy namespace loaders
│   │   ├── hooks/           # custom React hooks
│   │   ├── styles/          # isolated feature CSS (grokBotStudio.css, planTransport.css)
│   │   ├── types/           # TypeScript type declarations
│   │   ├── api.ts           # typed fetch wrappers
│   │   ├── auth.tsx         # auth context and session management
│   │   ├── turnstile.ts     # runtime Turnstile config, explicit script loader & hook
│   │   ├── perms.ts         # permission helpers (mirrors backend perms.rs)
│   │   ├── offlineSnapshot.ts / offlineQueue.ts # today's ride cache + offline trip status action queue
│   │   ├── grokBotStudio.ts / fareSheet.ts / xlsxCodec.ts # avatar studio SVG/PNG helpers & fare XLSX/CSV codec
│   │   ├── grokBotMotion.ts # Grok Bot motion engine: living idle, one-shot moves, shared rAF loop
│   │   ├── appUpdate.ts     # SW registration + new-build detection (/version.json) for the update banner
│   │   ├── webShare.ts      # native share sheet wrapper with WhatsApp/copy fallback
│   │   ├── hooks/useWakeLock.ts # Screen Wake Lock during live trips and QR display/scan
│   │   └── styles.css       # design tokens (colour, spacing, typography)
│   └── .nvmrc               # pins Node 22
├── android/
│   ├── app/                 # Kotlin Android app (package org.pjstatecm.graceride)
│   │   ├── src/main/java/   # Jetpack Compose UI, MVVM, Retrofit + ApiProvider, AndroidKeyStore AES-256-GCM SecureStore, BiometricPrompt + ECDSA P-256 direct login & app gate, OkHttp SSE, WorkManager, DriveHub, TripChat, LiveTripLocationService, ZXing/Code Scanner QR, Glance widget, OfflineCache, NetworkMonitor
│   │   ├── src/main/assets/ # 77 bundled lossless WebP icon assets (avatars, badges, events)
│   │   ├── src/test/        # 122 MockWebServer / JVM unit tests across 10 files
│   │   └── src/androidTest/ # 18 Compose UI tests across 2 files
│   ├── build.gradle.kts     # Gradle root configuration
│   └── gradle/              # Gradle wrapper
├── ios/
│   ├── GraceRide.xcodeproj/ # Xcode app + WidgetKit extension project & shared scheme
│   ├── GraceRideKit/        # Zero-dependency local Swift Package (DTOs, URLSession/SSE/cookie/Keychain, domain, repositories, ViewModels + 25 unit tests)
│   ├── GraceRide/           # Native SwiftUI iOS 17+ app (org.pjstatecm.graceride): Passkeys, CoreLocation trip sharing, AVFoundation QR scanner, PhotosUI, EventKit, 77 WebP assets, EN/zh-Hans Localizable.xcstrings
│   └── GraceRideWidget/     # WidgetKit Next-Trip widget + MarkRideReadyIntent AppIntent (group.org.pjstatecm.graceride)
├── scripts/                 # backup.sh, restore.sh, restore_drill.sh, setup_osrm_malaysia.sh, capacity_baseline.sh, generate_animated_icon_gifs.py
├── docs/
│   ├── TECHNICAL.md         # authoritative technical reference (single-manual policy)
│   ├── ANDROID.md           # consolidated native Android architecture, design system & test guide
│   ├── IOS.md               # consolidated native iOS SwiftUI architecture, design system & test guide
│   ├── USER_MANUAL.md       # end-user guide (Chinese, elder-friendly)
│   ├── ADMIN_MANUAL.md      # administration, reports & transport operations manual (Chinese)
│   └── brand/               # logo kit, brand guidelines, SVG/PNG assets
├── .github/workflows/
│   ├── ci.yml               # lint + typecheck + test + e2e + build on push/PR
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
| Start API (dev, auto-applies migrations) | `make dev-api` |
| Start frontend (dev) | `make dev-web` |
| Start Android debug | `cd android && ./gradlew :app:assembleDebug` |
| Lint everything | `make lint` |
| Lint Android | `make lint-android` |
| Format everything | `make fmt` |
| Type-check | `cd frontend && npm run typecheck` |
| Run all unit/integration tests | `make test` |
| Run backend tests only | `make test-api` (or `cd backend && cargo test --locked` with `TEST_DATABASE_URL`) |
| Run frontend unit tests only | `make test-web` (or `cd frontend && npm test`) |
| Run Playwright E2E | `make test-e2e` |
| Run Android unit & compile UI tests | `make test-android` (or `cd android && ./gradlew :app:testDebugUnitTest :app:compileDebugAndroidTestKotlin`) |
| Run iOS package tests & Simulator build | `make test-ios` (or `swift test --package-path ios/GraceRideKit` + `xcodebuild -project ios/GraceRide.xcodeproj -scheme GraceRide -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build`) |
| Production build | `make build` |
| Direct production deploy (AIC VPS, no GitHub Actions) | `make deploy-aic` |
| Dependency audit | `make audit` |
| DB up (Docker) | `make db-up` |

---

## Key Conventions

- **Env vars:** validated at startup in `Config::from_env()` / `DbConfig::from_env()`. Every var is in `backend/.env.example` with a comment.
- **Money:** integer cents (`i64`). Never floats.
- **Timestamps:** local KL text (`LOCALTIMESTAMP AT TIME ZONE ...`). Use `norm_dt()` helper to format.
- **Errors:** `AppResult<T>` = `Result<T, AppError>`. Use `bad()`, `forbidden()`, `not_found()`, `conflict()` helpers. Never panic in handlers.
- **Permissions:** check via `crate::util::require(&u, Page::X, Act::Y)?` (or `require_one` / `u.can(...)`) in every handler. Never bypass.
- **Personal data:** pickup address and phone visible only to the passenger, their assigned driver (once approved), and staff. `ride_json()` enforces this.
- **Secrets:** Telegram token, VAPID keys stored in `app_secrets` DB table, never in env or source.
- **i18n:** all user-facing text in `frontend/src/locales/`. Backend error messages in `backend/src/i18n/`.
- **Booleans in SQL:** stored as `BIGINT 0/1` (`BOOLEAN` on `geocode_cache.approx`, `guide_workflows.published`, `guide_feedback.helpful`).
- **Dynamic SQL (sqlx 0.9):** `sqlx::query*` only accepts `&'static str`. Build SQL that interpolates fixed fragments (column lists, placeholders, visibility clauses) with `crate::dyn_sql!(...)`; keep every user value in `.bind()`. Use `sqlx::AssertSqlSafe(...)` for an already-built `String`.
- **Stubs:** all stub files have been deleted. Do not add new stub-only files.

---

## Active Status & Boundaries

1. **Production Readiness:** Backend modularization complete across all domains; frontend modularized; all 73 permissions, 29 pages, and 14 built-in roles tested.
2. **Multi-Client Support:** React 19 PWA web app + Kotlin Jetpack Compose native Android client + SwiftUI native iOS client.
3. **Single-Manual Policy (One File Per Document Type):** All engineering and operational reference lives in `docs/TECHNICAL.md`; native Android reference in `docs/ANDROID.md`; native iOS reference in `docs/IOS.md`; user operations in `docs/USER_MANUAL.md` and `docs/ADMIN_MANUAL.md`; overview in `README.md`; security policy in `SECURITY.md`; release history in `CHANGELOG.md`. Do not keep duplicate PDFs or separate implemented/stale plans.
4. **Governance & Pre-Launch:** Legal review of PDPA notices, written church licensing confirmation, and Play Store account enrollment before public rollout.
