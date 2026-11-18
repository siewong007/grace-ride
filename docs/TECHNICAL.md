# Grace Ride (恩典同行) — Technical Manual

Complete engineering, operational, security, legal/compliance, licensing, and brand reference for **Grace Ride**, the invite-only ride and activity coordination platform built for **PJ State CMC**.

> [!IMPORTANT]
> **Single-Manual Policy:** `docs/TECHNICAL.md` owns 100% of detailed technical, operational, security, compliance, and licensing documentation, paired with the concise overview in [`../README.md`](../README.md). Update this manual in the same change whenever behavior, configuration, schema, or dependencies change. Do not create separate audit logs, ADR files, phase plans, or resolved-issue documents—rely on Git history for historical context.

---

## Table of Contents

1. [Local Development & Tooling](#1-local-development--tooling)
2. [Features & Module Inventory](#2-features--module-inventory)
3. [User & Operational Workflows](#3-user--operational-workflows)
4. [System Architecture & Database](#4-system-architecture--database)
5. [Configuration & Deployment Runbook](#5-configuration--deployment-runbook)
6. [Testing & Verification](#6-testing--verification)
7. [Security Model](#7-security-model)
8. [Compliance, PDPA & Malaysian Law](#8-compliance-pdpa--malaysian-law)
9. [Copyright, Third-Party Notices & Brand Kit](#9-copyright-third-party-notices--brand-kit)

---

## 1. Local Development & Tooling

### Prerequisites

- **PostgreSQL 18** (native install, or containerized via `docker-compose.dev.yml` / `make db-up`)
- **Rust** stable toolchain (`cargo`, `rustfmt`, `clippy`, plus `cargo-audit` for advisory scans)
- **Node.js 22+** (`.nvmrc` pins `22`) and `npm`

### Local Environment Setup

From the repository root, create or boot the development database and start the API server:

```bash
# Option A: Native local PostgreSQL
createdb grace_ride_dev                 # once, if it does not exist
cp backend/.env.example backend/.env

# Option B: Docker PostgreSQL 18 (creates both grace_ride_dev & grace_ride_test on volume grace-ride-dev-pg)
make db-up
cp backend/.env.example backend/.env
# When using docker-compose.dev.yml, point backend/.env at:
#   DATABASE_URL=postgres://grace:grace@127.0.0.1:5432/grace_ride_dev

cd backend
cargo run                              # API on http://127.0.0.1:8080 (or `make dev-api` from root)
```

> [!NOTE]
> `./dev-env.sh` wraps Rust commands (`Makefile` defaults `CARGO ?= ./dev-env.sh cargo`): if the maintainer's external SSD toolchain (`/Volumes/APPLE EXTERNAL SSD /Rust`) is mounted, it uses that `CARGO_HOME`/`RUSTUP_HOME`; otherwise it falls back transparently to the standard host Rust toolchain on macOS/Linux.

In a second terminal, start the Vite frontend server (`tsconfig.json` uses TypeScript project references across `tsconfig.app.json`, `tsconfig.test.json`, and `tsconfig.node.json`):

```bash
cd frontend
npm ci
npm run dev                            # http://localhost:5173 (proxies /api to :8080)
```

Startup automatically applies numbered SQLx migrations (`backend/migrations/`) and, when `DEV_MODE=true` on `localhost`, seeds deterministic synthetic demo fixtures (`seed::run`). Never drop a database as a routine update procedure, and **never** set `DEV_MODE=true` in production (startup aborts if `DEV_MODE=true` on a non-localhost `PUBLIC_URL`).

### Demo Accounts (`DEV_MODE=true` only)

- **Shared demo password:** `GraceRide2026!` (in `DEV_MODE`, password logins also satisfy the staff MFA gate)
- **Simulated TOTP secret:** `MFRGGZDF73DGXYP3`
- **Seeded synthetic accounts (`@example.org` — RFC 2606 reserved):**
  - **Superadmin:** `superadmin@example.org` (holds every permission, including `/admin/settings` and `superadmin` role management)
  - **Leadership & Admin:** `admin@example.org`, `president@example.org`, `vicepresident@example.org`, `secretary@example.org`
  - **Operations & Finance:** `traffichead@example.org`, `cellleader@example.org`, `transport@example.org`, `finance@example.org`, `auditor@example.org`
  - **Drivers & Members:** `driver@example.org`, `driver2@example.org`, `member@example.org`, `amina@example.org`, `sophie@example.org`, `yawen@example.org`, `zhiqiang@example.org`, `meiling@example.org`, `enci@example.org`, `guorong@example.org`

### `Makefile` Shortcuts

Run `make` (or `make help`) to list all documented targets:

| Target | Action |
|---|---|
| `make dev-api` | Run the Rust API in dev mode (`./dev-env.sh cargo run`) |
| `make dev-web` | Run Vite dev server on `:5173` |
| `make db-up` / `make db-down` | Start / stop local PostgreSQL 18 (`grace_ride_dev` + `grace_ride_test`) via `docker-compose.dev.yml` |
| `make fmt` | Format backend Rust code (`cargo fmt`) |
| `make lint` | Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `npm run lint` |
| `make test` | Run `cargo test --locked`, `npm run typecheck` (`tsc -b`), and `npm test` (Vitest) |
| `make test-api` / `make test-web` | Run backend Cargo tests or frontend typecheck + Vitest suites individually |
| `make test-e2e` | Run Playwright browser end-to-end suites (`npm run test:e2e`) |
| `make audit` | Run `cargo audit` (`backend/.cargo/audit.toml`) and `npm audit --omit=dev --audit-level=high` |
| `make build` | Build production frontend bundle (including `/third-party-licenses.txt`) + `cargo build --release --locked` |
| `make image` | Build `grace-ride:latest` Docker image |
| `make up` | Start generic Docker Compose production stack with `.env.production` |
| `make backup` | Snapshot database + uploads into `./backups` via `scripts/backup.sh` (auto-sources `.env.production` or `backend/.env` if needed) |
| `make restore` | Restore an archive (`ARCHIVE=backups/grace-ride-….tar.gz[.age]`) via `scripts/restore.sh` |
| `make restore-drill` | Non-destructive restore + upload reconciliation drill (`ARCHIVE=…`) via `scripts/restore_drill.sh` |
| `make osrm-setup` | Download & build Malaysia/Singapore/Brunei OSRM MLD graph via `scripts/setup_osrm_malaysia.sh` |
| `make capacity` | Run the 1,000-member / 10-driver capacity baseline (`scripts/capacity_baseline.sh`) |

---

## 2. Features & Module Inventory

Paths below are repository-relative. "Implemented" means present in source code; see [Boundaries & Outstanding Work](#boundaries--outstanding-work) and [Section 6](#6-testing--verification) for operational and verification scope.

| Module & Routes | Implemented Behavior | Primary Source |
|---|---|---|
| **Identity & Onboarding**<br>`/login`, `/invite/:token`, `/terms`, `/me` | Invite-only onboarding (14-day single-use token), Terms & PDPA v2 consent gate (`TERMS_VERSION`, English/Chinese/Malay), member approval, profile & language (`Accept-Language` fallback), WebAuthn passkeys, password + TOTP MFA, single-use hashed backup codes, passkey in-place re-auth, server-enforced idle/manual session lock (`session_lock_guard`, `/api/auth/lock` & `/api/auth/unlock`), alert channel preferences (`push_subscriptions`, Telegram/WhatsApp opt-ins), admin-reviewed account recovery (dual-admin for privileged accounts) | `backend/src/routes/auth.rs`, `backend/src/auth.rs`, `backend/src/security.rs`, `frontend/src/pages/Login.tsx`, `Invite.tsx`, `Profile.tsx`, `components/Terms.tsx` |
| **Roles, Permissions & Members**<br>`/admin/members`, `/admin/roles` | 27 page keys across 4 areas (`8 Member + 9 Transport + 2 Finance + 8 Admin = 27 pages`) totaling 67 page/action permissions; 13 built-in roles + custom roles; 5-tier ranked chain of command (`superadmin > president > vice_president > finance > auditor`); hierarchical groups & sub-groups (`groups.parent_id`, max depth 3, cycle-checked, built-in `Visitor` group); member approval/disable, invites, and recovery | `backend/src/perms.rs`, `backend/src/routes/roles`, `backend/src/routes/admin`, `frontend/src/pages/admin/Members.tsx`, `Roles.tsx` |
| **Passenger Rides & Caregiver Links**<br>`/rides`, `/rides/new`, `/rides/:id`, `/care/:token` | Outbound & paired return-leg round-trip requests (`leg`, `paired_ride_id`), saved places (`saved_places`) & proxy rider presets (`gate_notes`, `caregiver_user_id`), standing recurring schedules (`recurring_ride_schedules`), driver offer accept/decline, cancellation, seat count & assistance needs, approximate-area pickup privacy prior to approval, pickup-day readiness updates, single-own-ride-per-event guard (`DUPLICATE_EVENT_RIDE`), scoped zero-login Caregiver "Safe Arrival" links (`caregiver_passes`, `routes/care.rs`, `CarePass.tsx` — hides street address while showing status/driver/ETA and readiness toggle), local read-only snapshot of today's own active rides (`offlineSnapshot.ts`) | `backend/src/routes/rides.rs`, `backend/src/routes/care.rs`, `frontend/src/pages/Rides.tsx`, `RideNew.tsx`, `RideDetail.tsx`, `CarePass.tsx`, `offlineSnapshot.ts` |
| **Volunteer Driving**<br>`/drive` | Driver profile (vehicle, type, seats, accessibility capabilities `wheelchair_ok` / `low_step` / `child_seat`, service areas), standing driver schedules, carpool trip grouping, open-request claiming, withdrawal, staff reassignment confirmation, trip lifecycle (`start` → `pickup` / `no_show` → `complete`), driver delay & arrival-at-gate alerts; event rides restrict visibility and claiming to drivers in the rider's group or an ancestor group (`audience::MATCH_GROUP_SQL`), while non-event rides match by service area | `backend/src/routes/rides.rs`, `frontend/src/pages/Drive.tsx`, `frontend/src/components/TripActions.tsx` |
| **Maps, Routing & Chat**<br>Embedded on ride & admin views | Leaflet + OpenStreetMap tiles, server-proxied Photon address autocomplete (signed-in only), cached Nominatim geocoding, OSRM pickup ordering & ETA calculation, active-trip driver GPS + opt-in passenger GPS, concurrent live-driver cap (`MAX_LIVE_DRIVERS`), amber stale-location indicator, Google Maps / Waze handoff links, trip-scoped chat (`ride_messages` linked to `trips`) | `backend/src/routing.rs`, `backend/src/routes/rides.rs`, `frontend/src/components/LiveMap.tsx`, `Chat.tsx` |
| **Transport Staff & Planning**<br>`/admin/rides`, `/admin/live`, `/admin/events/:id/plan` | Full ride queue review, approve, return to pool, reassign, alternative transport, no-show, live fleet map; interactive event transport planning board with desktop drag-and-drop + mobile/accessible picker modal, filters, undo history (`Ctrl/Cmd+Z`), accessibility-aware in-group auto-arrange suggestions with explanations, return-leg driver matching (`applyMatchReturnDrivers`), audited out-of-group admin override (`ride.assign_outside_group`), and atomic batch save under driver row locks | `backend/src/routes/plan.rs`, `backend/src/arrange.rs`, `frontend/src/pages/admin/Rides.tsx`, `Live.tsx`, `PlanTransport.tsx` |
| **Events, Audiences & Check-In**<br>`/events`, `/events/new`, `/events/:id`, `/events/:id/checkin` | Group and sub-group event audiences (`audience::visible_events_sql`; non-eligible viewers see neither the event nor its title on rides, masked as `"Church event"` / `教会活动`), recurring event series (`series_id`), event duplication (`POST /events/{id}/duplicate`, `?duplicate=`), required `"joining as"` attribution group (`event_attribution_groups`, `as_group_id`), optional event cover photo (`photo_file_id`), capacity & FIFO waitlist auto-promotion, driver offers, ride requests gated on `"going"` signup, attendance module toggle (`attendance_enabled`), organiser walk-in/attendance sheet, member personal pass codes (`pass_code`) with organiser pass scanning (`POST /events/{id}/checkin/scan-member`), self-service QR check-in (`checkin_code`), and live ICS calendar feed (`calendar_token_hash`) | `backend/src/audience.rs`, `backend/src/routes/events.rs`, `frontend/src/pages/Events.tsx`, `EventDetail.tsx`, `EventForm.tsx`, `EventCheckin.tsx` |
| **Visitor Passes**<br>`/visit/:token` | Going members and organisers can invite a guest without an account, creating a proxy ride recorded under the `Visitor` group (driver-matched via the host's group) and a one-time link; opens 24 h before pickup, claimed by a single browser via explicit `POST` (`gr_visit` cookie scoped to `/api/visit`), displays live ride status & readiness buttons (`ready`, `running late`, `can't make it`), and closes permanently on completion, cancellation, revocation, reissue, or `valid_until` (event start + 12 h) | `backend/src/routes/visitors.rs`, `frontend/src/pages/Visit.tsx`, `frontend/src/components/VisitorInvites.tsx` |
| **Reimbursements**<br>`/reimbursements`, `/admin/reimbursements` | Receipt-backed substitute-transport claims, resubmit on return, transport review queue (approve/return/reject), finance payout queue & monthly payout batches (`payout_batches`, `batch_id`, CSV export, enforced 2-person `created_by != paid_by` separation) with payee verification & payment reference recording; vehicle-type per-km rates and completion-generated volunteer driver reimbursement records (cell-leader traffic scope never includes claim approval) | `backend/src/routes/settlements.rs`, `backend/src/routes/rides.rs`, `backend/src/routes/community.rs`, `frontend/src/pages/Reimbursements.tsx`, `admin/Reimbursements.tsx` |
| **Activity Cost Splitting**<br>`/events/:eventId/settle`, `/settlements`, `/costs` | Equal or custom integer-cent participant shares, church subsidies, optional payee DuitNow QR image (`payee_qr_file_id`), one-tap exact RM copy, receipt transaction reference extraction (`POST /files/{id}/extract-ref`), attended-member eligibility, draft/publish/revise lifecycle (`paid_cents` preserved across revisions so prior payments move to `needs_info` for reconciliation rather than resetting to `unpaid`), external payment proof upload, payee verification, duplicate-reference detection, share disputes, and settlement closure | `backend/src/routes/settlements.rs`, `frontend/src/pages/Settlements.tsx`, `Costs.tsx` |
| **File Storage**<br>`/api/files/*` | Magic-byte validated uploads (JPEG, PNG, WebP, HEIC, PDF), 8 MB file limit (9 MB body limit), 100 uploads/member/24 h, random storage filenames, `CSP: sandbox`, forced download for PDFs, receipt transaction reference extraction (`POST /api/files/{id}/extract-ref`), and per-use access authorization (uploader, visible event cover photo, settlement organiser/payee/QR, reimbursement claimant/reviewer/finance, and authorized help-guide screenshots) | `backend/src/routes/files.rs` |
| **Community, Help & Guides**<br>`/`, `/help`, `/help/guides/*`, `/notifications`, `/admin/announcements` | Announcements, answer-first member Help centre (prioritizing today's active ride and awaiting replies before ticket creation), support threads with claim/reply/close, SSE live updates (`/api/stream`) with polling fallback, encrypted native Web Push delivery (RFC 8291 `aes128gcm` + RFC 8292 `ES256` VAPID with self-test `/api/push/test` and automatic `404`/`410` stale-endpoint pruning), per-passenger `readiness_prompt_mins` pickup reminders (30/45/60 min or passenger-muted, auto-rearmed when `driver_id` changes) & escalation jobs, volunteer badges with opt-out, and bilingual step-by-step guides (member & staff audiences, formatted step text, optional step screenshots, stable step IDs, and persistent floating walkthrough card) | `backend/src/routes/community.rs`, `backend/src/web_push.rs`, `backend/src/jobs.rs`, `backend/src/stream.rs`, `frontend/src/pages/Help.tsx`, `HelpGuides.tsx`, `HelpGuideDetail.tsx`, `HelpGuideEditor.tsx`, `components/GuideRunner.tsx`, `components/GuideSteps.tsx`, `guideProgress.ts` |
| **Reports & Exports**<br>`/report`, `/admin/reports` | Monthly member activity/driving/ride summaries, staff attendance/ride/cost reports, unmet-demand & driver shortage analytics (`GET /admin/reports/demand`, Demand & Shortages tab), CSV exports, and client-side CJK-capable PDF generation (`pdfWriter.ts`) for Chinese and English reports and audit trails | `backend/src/routes/community.rs`, `backend/src/routes/admin.rs`, `frontend/src/pages/Report.tsx`, `admin/Reports.tsx`, `pdfWriter.ts` |
| **Administration & Settings**<br>`/admin`, `/admin/menu`, `/admin/audit`, `/admin/settings` | Operational dashboard & tab counts, filterable/exportable audit log, hierarchical groups & service areas, vehicle reimbursement rates, external alert channel settings (`GET/PUT /admin/settings/alerts` storing auto-generated/rotatable P-256 VAPID keys, Telegram bot, and WhatsApp webhook secrets in `app_secrets` for async `util::dispatch_external_alerts`), data-retention settings, and organization auto-lock timeout defaults (`admin_settings` restricted to `superadmin`) | `backend/src/routes/admin.rs`, `backend/src/web_push.rs`, `backend/src/jobs.rs`, `frontend/src/pages/admin/Dashboard.tsx`, `Audit.tsx`, `Settings.tsx`, `Menu.tsx` |
| **Shared UI, PWA Shell & Large Lists**<br>App-wide | Minimalist white/neutral surfaces with logo-blue (`#1F5DB4`) primary actions, dark mode, adjustable text size, responsive mobile bottom-nav / tablet icon-rail / desktop sidebar, route-level lazy code splitting with animated boot splash handoff, numbered & keyset pagination, `@tanstack/react-virtual` list primitive, PWA manifest/icons, and privacy-safe Service Worker (`frontend/public/sw.js` — caches static app shell & handles push/notification clicks while never caching `/api/*` in `CacheStorage`) | `frontend/src/App.tsx`, `styles.css`, `components/Layout.tsx`, `components/ui/` (`components/ui.tsx`), `public/sw.js`, `locales/` |
| **Capacity & Operations**<br>Backend & `scripts/` | PostgreSQL connection pool with timezone & statement timeouts, 60 s reminder/escalation/schedule loop, periodic housekeeping job, `/api/health` + binary `healthcheck` subcommand, `age`/`gpg`-capable backup/restore/drill scripts, self-hosted OSRM setup script, synthetic 1,000-member capacity test harness | `backend/src/lib.rs`, `jobs.rs`, `backend/tests/capacity.rs`, `scripts/backup.sh`, `scripts/restore.sh`, `scripts/restore_drill.sh`, `scripts/setup_osrm_malaysia.sh`, `scripts/capacity_baseline.sh` |

### Boundaries & Outstanding Work

- **No commercial fares or automated bank payouts:** Rides are voluntary and free of fares or commissions. Direct payment-gateway transfers and Grab booking API integrations are not built; settlements optionally display a static DuitNow QR uploaded by the payee and record external payment proofs, while volunteer/substitute reimbursement records and monthly payout batches are settled manually by finance.
- **Privacy-scoped offline storage:** `frontend/public/sw.js` caches static SPA shell assets (`/` and `/assets/*`) and handles push/notification display, intentionally never caching `/api/*` responses in `CacheStorage`. Offline ride data is strictly limited to `frontend/src/offlineSnapshot.ts` (read-only cache of the signed-in user's own active rides for today, wiped immediately on logout, HTTP `401`, or Auto-Lock); offline write queues are not built.
- **Deferred analytics & alerts:** `as_group_id` is stored on signups and event rides, but per-group headcount breakdowns in reports and a 24 h pre-pickup dashboard alert for unmatched in-group event rides are not yet built. Visitor pages link out to external maps rather than embedding `LiveMap`.
- **Single-instance runtime assumptions:** Per-IP rate buckets, geocoding/route memory caches, SSE broadcast channels, and the 60 s background job loop run in-process. Running multiple API replicas requires moving SSE to PostgreSQL `LISTEN/NOTIFY` and guarding jobs with advisory locks.
- **Pending church governance sign-off:** Legal review of PDPA notices/Malay translation, final retention periods for profiles/rides/receipts, motor insurance guidance, and written PJ State CMC logo permission remain pre-launch church actions (see [Section 8](#8-compliance-pdpa--malaysian-law)).

---

## 3. User & Operational Workflows

### Membership & Account Access

1. An authorized administrator creates a single-use invitation (`/admin/members`). The invitee opens `/invite/:token`, enters their details, and accepts the current Terms & Privacy Notice (`TERMS_VERSION`).
2. Staff approves the pending member and assigns church groups and roles. Pending accounts are blocked from member and staff endpoints by server-side extractors regardless of how credentials are sent.
3. Members sign in via passkey (WebAuthn) or password (+ TOTP if configured). Accessing any non-Member page (`Transport`, `Finance`, `Admin`) requires a passkey or 2FA-verified session (`util::require`). Members manage language, passkeys, TOTP, single-use backup codes, alert channels, saved places, and auto-lock timeout in `/me`.
4. Account recovery is verified offline by staff, who issue a hashed 24 h one-time link. Recovering any account that holds staff permissions requires two distinct administrators to initiate and confirm; only the confirming administrator receives the link, and completion wipes prior sessions, passkeys, TOTP, and backup codes.

### Requesting & Delivering a Ride

```text
pending_driver -> pending_passenger -> pending_approval -> confirmed -> in_progress -> completed
staff reassignment / transport plan -> pending_driver_confirm -> confirmed
terminal alternatives: cancelled, no_show, alt_transport
```

1. **Request:** A member submits pickup time, area, full address (or saved place), destination, seats, optional return leg, and optional assistance needs (`/rides/new`), or sets up a standing weekly schedule. Proxy rides can name a family member or elder while keeping booking management under the signed-in member, and bookers/caregivers can share a zero-login `/care/:token` Safe Arrival link.
2. **Match & Approve:** A volunteer driver claims an open request (`pending_passenger`), the passenger accepts (`pending_approval`), and transport staff approves after capacity verification (`confirmed`). Before approval, drivers see only the approximate pickup area; approval reveals contact/address details and opens trip chat.
3. **Active Trip:** Passenger readiness (`ready`, `late`, `unable to join`) and driver updates (`delay`, `arrived at gate`, `start`, `pickup`, `no_show`, `complete`) coordinate the pickup. Live GPS is accepted only during `in_progress` rides and deleted upon completion. Chat closes when the trip ends (including when the last passenger is marked `no_show`).
4. **Concurrency & Safety Guards:**
   - One active own ride per member per event (`DUPLICATE_EVENT_RIDE`); proxy rides for others are unaffected.
   - Drivers cannot claim rides whose pickup time has passed; unmatched/unconfirmed requests expire automatically 6 hours after pickup time.
   - Every state transition checks the expected `driver_id` and source status so stale tabs cannot act on a reassigned ride.
   - Staff approval, driver confirmation, and transport-plan saves acquire row locks on receiving drivers, verify seat capacity, attach the carpool `trip_id` atomically, and re-arm the 1-hour pickup reminder (`trg_rides_rearm_reminder`).
   - `"Unable to join"` is rejected once a passenger has been picked up. Cancelling, withdrawing, no-show, or switching to alternative transport invalidates the cached trip route.
   - Disabling a driver's account automatically returns their unstarted rides to the pool, flags confirmed rides (`needs_attention = 1`) for staff, and notifies affected passengers. Reassigning always notifies the outgoing driver.
5. **Event Transport Planning Board (`/admin/events/:id/plan`):** Staff review event riders and offered drivers, filter and assign via drag-and-drop or picker modal (with multi-step undo), run accessibility-aware in-group auto-arrange suggestions (which explain placements and flag riders who do not fit without silently saving), and commit the reviewed batch in a single transaction.

### Events, Attendance, Visitors & Money

- **Events & Group Audiences:** Organisers target one or more groups (single events or recurring series); sub-groups automatically inherit visibility (`groups.parent_id`). Signups require a `"joining as"` group (`as_group_id`, auto-selected when unambiguous), copied onto linked event rides so historical reports remain stable if a member later changes groups. Events can be cancelled or finished only while `published`, and cancelled events are immutable.
- **Attendance & QR Check-In:** Events enable attendance tracking by default (`attendance_enabled = 1`). Organisers can record attendance manually (only `"attended"` marks create walk-in signups; capacity increases promote waitlisted members in FIFO order, while withdrawing and rejoining resets waitlist position to the back). Active members can participate without booking a ride, scan the event QR code (`/events/:id/checkin?code=<checkin_code>`), or present their personal signup `pass_code`.
- **Visitor Passes:** Going members and organisers can invite a guest from the event page, booking a proxy ride and generating `/visit/:token` scoped strictly to that journey.
- **Activity Settlements:** Organisers record expenses, optional church subsidies, optional DuitNow QR image, and equal/custom integer-cent shares. Drafts can be deleted only while still `draft` (guarded atomically against concurrent publish), and only `published` bills can be closed. Revising a published bill preserves `paid_cents` and moves previously paid/pending shares to `needs_info` so reconciliations never lose payment history.
- **Reimbursements:** Substitute-transport claims require receipt uploads, transport review, and manual finance payout recording (individually or grouped into monthly `payout_batches`). Completed volunteer rides create mileage-based reimbursement records according to configured vehicle rates without moving funds automatically.
- **Help Centre & Guides:** Closed help tickets reject concurrent replies rather than silently reopening. Help editors (`admin_help.update`) author bilingual member and staff guides with step links, formatted notes, and uploaded step screenshots; users can follow guides either as a checklist or as a floating cross-page walkthrough card.

### Development & Maintenance Workflow

1. Inspect `git status` / `git diff` first and preserve unrelated working-tree changes.
2. Check server-side permissions (`perms.rs`), audience visibility (`audience.rs`), and privacy projections whenever touching endpoints or exports.
3. Run relevant backend, frontend, and browser checks ([Section 6](#6-testing--verification)) and report actual command outcomes—never extrapolate pass counts from historical runs.
4. Keep documentation consolidated in `README.md` (concise overview) and `docs/TECHNICAL.md` (complete technical reference).

---

## 4. System Architecture & Database

### Runtime Topology

```text
Browser: React 18 + TypeScript + Vite SPA, Leaflet, Service Worker (sw.js), i18next (zh/en + ms terms)
  -> HTTPS / Caddy (or Cloudflare -> Host Caddy -> loopback on AIC VPS)
     -> Single Axum Rust binary (/app/grace-ride)
        ├── /api/*            JSON REST routes + cookie auth + session_lock_guard + CSRF/rate limits
        ├── /api/stream       Authenticated Server-Sent Events (SSE) push channel
        ├── /api/visit/*      Isolated visitor-pass endpoints (gr_visit cookie)
        ├── /api/care/*       Scoped caregiver Safe Arrival endpoints (hashed token)
        └── /*                Compiled SPA static assets + sw.js + /third-party-licenses.txt
     -> PostgreSQL 18 (SQLx pool, startup migrations, timezone & statement timeouts)
     -> Persistent upload directory (/data/uploads)
     -> Photon / Nominatim / OSRM (proxied autocomplete, cached geocoding & routing)
     -> In-process Tokio tasks (60s reminders/escalations/stale-rides/schedules + housekeeping)
```

### Code Ownership

| Path | Responsibility |
|---|---|
| `backend/src/main.rs`, `state.rs` | CLI entry (`healthcheck` mode or server), env validation, tracing logs, WebAuthn setup, graceful `SIGTERM`/`SIGINT` shutdown |
| `backend/src/lib.rs` | PostgreSQL connection & migration history classifier, startup initialization, Axum router & middleware stack |
| `backend/src/auth.rs`, `perms.rs`, `security.rs` | Session & visitor extractors, 27-page / 67-permission catalog & 5-tier rank rules, `session_lock_guard`, CSRF/CSP headers, IP rate limiters |
| `backend/src/routes/` | Modularized domain HTTP handlers (`auth`, `rides`, `care`, `events`, `visitors`, `plan`, `settlements`, `files`, `community`, `admin`, `roles` — organized into domain submodules under `backend/src/routes/*/`) |
| `backend/src/audience.rs` | Shared recursive group-tree visibility SQL (`visible_events_sql`), joining-as validation, driver group matching (`MATCH_GROUP_SQL`) |
| `backend/src/arrange.rs`, `routing.rs` | Accessibility-aware event transport auto-arrange heuristics; Photon autocomplete proxy, Nominatim geocoding, and OSRM route caching |
| `backend/src/jobs.rs`, `stream.rs`, `web_push.rs` | 1-minute reminder/escalation/stale-ride/visitor-expiry/recurring-schedule tick, periodic retention cleanup, SSE fan-out, and RFC 8291/8292 Web Push encryption |
| `backend/src/i18n/`, `error.rs`, `util.rs`, `seed.rs` | Bilingual backend error/notification catalogs, `Accept-Language` negotiation, audit helpers, KL-time helpers, dev demo seeding |
| `backend/migrations/` | Frozen baseline `0001_init.sql` + additive numbered migrations `0002`–`0009` |
| `frontend/src/App.tsx`, `auth.tsx`, `perms.ts`, `staffNav.tsx` | Lazy client routing, session context, permission gates, boot splash handoff, staff navigation |
| `frontend/src/pages/`, `components/`, `components/ui/` | Member, visitor, caregiver, and admin screens; `LiveMap`, `Chat`, `Terms`, `AutoLock`, `BackNavigation`, `GuideRunner`, `GuideSteps`, and modularized shared UI primitives (`components/ui/` re-exported via `components/ui.tsx`) |
| `frontend/src/styles.css`, `prefs.ts`, `locales/`, `pdfWriter.ts`, `offlineSnapshot.ts` | CSS design tokens, theme/font-size preferences, `en`/`zh`/`ms` locale bundles, CJK PDF writer, today's active ride cache |
| `frontend/public/sw.js`, `frontend/scripts/third-party-licenses.mjs` | Privacy-safe app-shell/push service worker; prebuild licence generator producing `/third-party-licenses.txt` |
| `deploy/`, `Dockerfile`, `docker-compose*.yml`, `scripts/`, `SECURITY.md` | Multi-stage container build, generic/dev/AIC/OSRM Compose configs, Caddyfiles, `backup.sh`, `restore.sh`, `restore_drill.sh`, `setup_osrm_malaysia.sh`, `capacity_baseline.sh`, and vulnerability reporting policy |

### Permission Catalog (`backend/src/perms.rs`)

Every handler checks `Page` + `Act` (`view`, `create`, `update`, `delete`), never hardcoded role names (except fixed structural invariants in [Section 7](#7-security-model)):

- **Member Area (`Area::Member` — standard member session):**
  `rides` (`V,C,U,D`), `drive` (`V,C,U,D`), `events` (`V,C,U,D`), `settlements` (`V,C,U,D`), `costs` (`V,U`), `reimbursements` (`V,C,U`), `help` (`V,C,U`), `report` (`V`).
- **Transport Area (`Area::Transport` — requires passkey/2FA session):**
  `admin_dashboard` (`V`), `admin_rides` (`V,U,D`), `admin_live` (`V`), `admin_plan` (`V,U`), `admin_events` (`V,U,D`), `admin_reimb_review` (`V,U`), `admin_announcements` (`V,C,D`), `admin_help` (`V,U`), `admin_reports` (`V`).
- **Finance Area (`Area::Finance` — requires passkey/2FA; never self-granted):**
  `admin_reimb_pay` (`V,U`), `admin_bills` (`V`).
- **Admin Area (`Area::Admin` — requires passkey/2FA):**
  `admin_members` (`V,U`), `admin_invites` (`V,C,D`), `admin_recovery` (`V,U`), `admin_groups` (`V,C,U,D`), `admin_areas` (`V,C,U`), `admin_roles` (`V,C,U,D`), `admin_audit` (`V`), `admin_settings` (`V,U`).

### Storage, Schema & Migrations

SQLx runs `backend/migrations/` at startup (`lib.rs::connect_db_with`) on a dedicated connection without a statement timeout before opening the request pool. Every pooled connection sets `TimeZone = <APP_TIMEZONE>`, `statement_timeout`, and `idle_in_transaction_session_timeout`.

| Migration | Purpose |
|---|---|
| `0001_init.sql` | **Frozen** squashed baseline schema (idempotent `IF NOT EXISTS` DDL, `CHECK` constraints, composite indexes, `trips` & `member_roles` triggers, `JSONB` columns, `app_now()`). Guarded by `schema_baseline.rs`—**never edit this file**. |
| `0002_guide_step_links.sql` | Repairs legacy seeded help-guide step route patterns (e.g. `/rides/:id` → `/rides`) without touching editor-modified guides. |
| `0003_event_photo.sql` | Adds optional `events.photo_file_id` cover image column. |
| `0004_rearm_ride_reminder.sql` | Adds `BEFORE UPDATE OF driver_id` trigger `trg_rides_rearm_reminder` on `rides` to reset `reminder_sent = 0` whenever a driver changes. |
| `0005_share_paid_cents.sql` | Adds `settlement_shares.paid_cents` and backfills confirmed/pending shares so bill revisions preserve paid amounts. |
| `0006_attendance_module.sql` | Adds `events.attendance_enabled` (default `1`) and backfilled 10-char `events.checkin_code` for QR self-check-in. |
| `0007_terms_version.sql` | Adds `users.terms_version` (default `2`) and marks prior acceptances as `1` to trigger PDPA v2 re-consent. |
| `0008_extended_features.sql` | Adds server-enforced session lock (`sessions.locked_at`, `last_seen_at`), round-trip ride pairing (`leg`, `paired_ride_id`), `saved_places` & caregiver/proxy defaults, vehicle accessibility flags (`wheelchair_ok`, `low_step`, `child_seat`), `recurring_ride_schedules`, `push_subscriptions` & alert opt-ins, `caregiver_passes`, recurring `events.series_id` & `event_signups.pass_code`, `users.calendar_token_hash`, `settlements.payee_qr_file_id`, and `payout_batches` / `reimbursements.batch_id`. |
| `0009_schema_hardening.sql` | Additive schema hardening: partial B-tree indexes backing `files::can_access_file`, GIN index on `guide_workflows.steps`, event `checkin_code` / `pass_code` lookup indexes, and guarded `NOT VALID` + validated `ON DELETE SET NULL` foreign keys on `users.photo_file_id` and `events.photo_file_id`. |

**Migration & Data Conventions:**
- **Upgrade safety (`lib.rs::classify_history`):** Every schema change must be a new numbered `000N_….sql` file. Databases from pre-squash histories whose version 1 checksum differs automatically clear `_sqlx_migrations` and replay idempotent `0001_init.sql` + subsequent migrations without losing rows. If version 1 matches but a later applied file was modified or is missing from the binary, startup halts with an error.
- **Constraint validation:** Add new `CHECK` constraints `NOT VALID` and validate in a guarded block that logs a warning on legacy offending rows rather than aborting deployment. Inspect unvalidated constraints via `SELECT conrelid::regclass, conname FROM pg_constraint WHERE NOT convalidated;`.
- **Timestamps & Money:** Times are stored as local `YYYY-MM-DDTHH:MM[:SS]` text in `APP_TIMEZONE` (default `Asia/Kuala_Lumpur`), read in Rust via `util::local_now()` / `util::now()` and in SQL defaults via `app_now()`. Pass query cutoffs from Rust parameters rather than `LOCALTIMESTAMP`. Monetary amounts are integer cents (`BIGINT`); boolean flags on legacy tables use checked `BIGINT` `0`/`1`; emails are enforced lowercase.
- **Roles & JSONB:** Application writes comma-separated role names to `users.roles`, which database triggers synchronize into `member_roles` (the table read by authorization checks). Structured payloads (`guide_workflows.steps`, `settlement_revisions.snapshot`, `invites.group_ids`) use `JSONB`.
- **Lists & Real-Time Updates:** Endpoints use either numbered pagination (`page`/`limit` envelope) or keyset cursors (`items` + `next_before`) backed by composite `(timestamp, id)` indexes. Authenticated SSE (`/api/stream`) nudges active tabs to refetch without requiring WebSockets.

---

## 5. Configuration & Deployment Runbook

### Environment Variables (`backend/src/state.rs`, `routing.rs`)

| Variable | Default | Notes / Validation |
|---|---|---|
| `DATABASE_URL` | `postgres://localhost/grace_ride_dev` | PostgreSQL connection URL |
| `PUBLIC_URL` | `http://localhost:5173` | Canonical browser URL; WebAuthn RP ID binds to its host. Must be `https://` outside localhost |
| `BIND` | `127.0.0.1:8080` | Socket address for the Axum HTTP listener (`0.0.0.0:8080` in Docker) |
| `DEV_MODE` | `false` | Seeds demo data and relaxes MFA for password logins; **refused unless `PUBLIC_URL` is localhost** |
| `ADMIN_EMAIL` | *(unset)* | On first production boot, creates initial `superadmin` + `admin` and logs a one-time passkey setup link |
| `UPLOAD_DIR` | `./data/uploads` | Persistent directory for validated receipt and photo uploads (`/data/uploads` in Docker) |
| `STATIC_DIR` | *(unset)* | Path to built SPA (`frontend/dist` or `/app/web`) served by Axum in production |
| `TRUST_PROXY` | `false` | Trust first `X-Forwarded-For` hop for client IP rate-limiting/audit (enable only behind Caddy) |
| `LOG_FORMAT` | `text` | Set to `json` in production for structured JSON stdout logging |
| `RUST_LOG` | `grace_ride=info,tower_http=info` | Tracing filter (`grace_ride=info,tower_http=warn` recommended in production) |
| `APP_TIMEZONE` | `Asia/Kuala_Lumpur` | IANA timezone; **choose once before real data exists** (changing later does not convert stored text times) |
| `DB_MAX_CONNECTIONS` | `10` | Pool size (`1..=100`); keep below PostgreSQL `max_connections` (`20` on AIC VPS) |
| `DB_ACQUIRE_TIMEOUT_SECS` | `5` | Pool checkout timeout (`1..=120`) |
| `DB_STATEMENT_TIMEOUT_SECS` | `15` | Per-statement limit in seconds (`0..=3600`, `0` = unlimited; migrations are exempt) |
| `MAX_LIVE_DRIVERS` | `10` | Concurrent active GPS-sharing drivers cap (`1..=100`) |
| `SESSION_TIMEOUT_MINS` | `15` | Initial default org auto-lock timeout in minutes (`0` = disabled; editable in `/admin/settings`) |
| `CHAT_RETENTION_DAYS` | `180` | Days after trip completion before trip chat messages are purged |
| `NOTIFICATION_RETENTION_DAYS` | `90` | Days read notifications are kept (`7..=3650`; unread kept `max(365, value)` days) |
| `AUDIT_RETENTION_DAYS` | `730` | Days audit log entries are retained |
| `HOUSEKEEPING_INTERVAL_MINS` | `60` | Minutes between retention/session/empty-trip cleanup runs (`1..=1440`; reminders always tick every 60 s) |
| `ROUTING_ENABLED` | `true` | Set `false` to keep all addresses in-house (uses straight-line GPS estimates and disables autocomplete) |
| `OSRM_URL` | `https://router.project-osrm.org` | OSRM routing base URL (**self-host before production launch**) |
| `NOMINATIM_URL` | `https://nominatim.openstreetmap.org` | Nominatim geocoding base URL |
| `PHOTON_URL` | `https://photon.komoot.io` | Photon address-autocomplete base URL |
| `GEOCODER_USER_AGENT` | `GraceRide/0.1` | Set contact URL/email for Nominatim policy compliance (e.g. `GraceRide/0.1 (+https://…)`) |
| `GEOCODER_COUNTRY` | `my` | ISO country bias/filter for geocoding and autocomplete |
| `PUSH_ALLOWED_HOSTS` | *(built-in browser push hosts)* | Optional comma-separated host or `.suffix` override for Web Push subscription endpoints outside `DEV_MODE` |
| `TEST_DATABASE_URL` | `postgres://localhost/grace_ride_test` | Test-only PostgreSQL URL used by Cargo integration tests and `scripts/capacity_baseline.sh` (**never** point at production) |
| `PG_CONTAINER` / `DOCKER_DB_CONTAINER` | *(unset)* | Optional running PostgreSQL container name (e.g. `grace-ride-db`) used by `scripts/backup.sh`, `scripts/restore.sh`, and `scripts/restore_drill.sh` via `docker exec` |
| `BACKUP_SCHEMA` | *(all schemas)* | Optional PostgreSQL `--schema` filter for `scripts/backup.sh` (used by isolated integration test schemas) |
| `REQUIRE_ENCRYPTION` | `false` | When `true`, `scripts/backup.sh` refuses to run unless `BACKUP_AGE_RECIPIENT` or `BACKUP_GPG_RECIPIENT` is set |
| `BACKUP_AGE_RECIPIENT` / `BACKUP_GPG_RECIPIENT` | *(unset)* | Public `age` recipient (`age1…`) or GPG recipient used by `scripts/backup.sh` to encrypt archives at rest |
| `BACKUP_AGE_KEY_FILE` | *(unset)* | Path to private `age` identity key used by `scripts/restore.sh` and `scripts/restore_drill.sh` to decrypt `.tar.gz.age` archives |
| `ADMIN_DATABASE_URL` | *(unset)* | Maintenance PostgreSQL URL (e.g. `postgres://localhost/postgres`) used by `scripts/restore_drill.sh` to create/drop disposable drill databases |
| `OSRM_DATA_DIR` | `./osrm-data` (`/opt/grace-ride/osrm-data` in Compose) | Host directory storing pre-processed Malaysia/Singapore/Brunei OSRM MLD graph files |

### Option A: Standard Single-VM Deployment (`docker-compose.yml`)

Suitable for a standalone Linux VM (e.g. 1 vCPU / 1–2 GB RAM) where Docker Compose runs PostgreSQL 18 (`database` internal network only), the Grace Ride container (`init: true`, `512m` memory / `512` PID caps), and Caddy (`frontend` network only, automatic Let's Encrypt HTTPS with `X-Forwarded-For` overwritten from `{client_ip}`):

1. Point DNS `A`/`AAAA` records (e.g. `ride.pjstatecmc.org`) at the server and open TCP `80`/`443` (plus UDP `443` for HTTP/3).
2. Install Docker + Compose plugin, copy the repository, and configure `.env.production`:
   ```bash
   cp deploy/.env.production.example .env.production
   # Edit DOMAIN, ACME_EMAIL, PUBLIC_URL, ADMIN_EMAIL, POSTGRES_PASSWORD (keep DEV_MODE=false)
   ```
3. Build and start the stack:
   ```bash
   docker compose --env-file .env.production up -d --build
   ```
4. Retrieve the initial administrator's one-time passkey setup link and open it on their device:
   ```bash
   docker compose logs app | grep "one-time link"
   ```
5. Verify health:
   ```bash
   curl -fsS https://ride.pjstatecmc.org/api/health
   ```
6. **Updates:** `git pull && docker compose --env-file .env.production up -d --build`.

### Option B: AIC Cloud VPS Automated Deployment (`.github/workflows/deploy.yml`)

Production deploys to `https://graceride.saliminn.my` on the shared AIC VPS (`deploy/docker-compose.aic.yml` + `deploy/graceride.Caddyfile`):

- **Architecture:** Host Caddy owns ports `80`/`443` and reverse-proxies `graceride.saliminn.my` to `127.0.0.1:8090` using a Cloudflare Origin CA certificate (`/etc/caddy/certs/graceride-origin.{pem,key}` under Cloudflare **Full (strict)** mode, with SSE buffering disabled on `/api/stream` and `X-Forwarded-For` overwritten from `{client_ip}`). PostgreSQL (`grace-ride-db`, tuned to `max_connections=20`) stays internal on the Docker `database` network. Secrets live in `/opt/grace-ride/secrets.env`.
- **Self-Hosted Malaysia Routing (`deploy/docker-compose.osrm.yml`):** Run `scripts/setup_osrm_malaysia.sh /opt/grace-ride/osrm-data` once to download and checksum-verify Geofabrik's Malaysia/Singapore/Brunei extract and build an MLD routing graph, set `OSRM_URL=http://grace-ride-osrm:5000` in `/opt/grace-ride/secrets.env`, and launch with `-f /opt/grace-ride/docker-compose.aic.yml -f /opt/grace-ride/docker-compose.osrm.yml`. Alternatively, set `ROUTING_ENABLED=false` in `secrets.env` to keep all addresses in-house using straight-line GPS estimates.
- **CI-Gated Rollout:** Pushing to `master` runs `.github/workflows/ci.yml`. Only when CI succeeds (or via manual `workflow_dispatch` on `master`) does `deploy.yml` build a `linux/amd64` image, verify ≥1.5 GB free disk, sync `deploy/docker-compose.aic.yml` and `deploy/docker-compose.osrm.yml` into `/opt/grace-ride/`, ensure `grace-ride-db` is running and healthy (supporting first boot on a fresh VPS), upload and load the image tarball over SSH (default port `20049`, pinned host key `SHA256:HV6OszvuYmwmWRLEY7z7jm8GojWZJppJlDBHIhJVUxw`), swap the `grace-ride` app container, and verify both local `127.0.0.1:8090/api/health` and public `$PUBLIC_URL/api/health` (automatically rolling back to `old_tag` if either check fails).
- **GitHub Environment Configuration (`production`):** Secrets `VPS_SSH_PRIVATE_KEY` and `VPS_HOST_KEY`; variables `VPS_HOST` (plus optional `VPS_PORT`, `VPS_USER`).
- **Skipping Deploy:** Include `[skip ci]` in a commit message to skip CI and deployment (note: `ci.yml` has no Markdown path filter, so regular doc commits to `master` trigger CI and deploy).
- **Rollback Caveat:** Failed rollout health checks automatically restore the previous container image tag, which does **not** undo additive SQL migrations. Always write backward-compatible additive migrations.

### Backups, Restore Drills & Monitoring

- **Nightly Encrypted Backups (`scripts/backup.sh`):** Dumps PostgreSQL (`pg_dump -Fc` via `DATABASE_URL` or `PG_CONTAINER=grace-ride-db`), tars the upload volume, optionally encrypts with `age` (`BACKUP_AGE_RECIPIENT=age1…`, enforceable via `REQUIRE_ENCRYPTION=true`) or `gpg` (`BACKUP_GPG_RECIPIENT`), and prunes archives older than `KEEP_DAYS` (default `30`):
  ```bash
  PG_CONTAINER=grace-ride-db REQUIRE_ENCRYPTION=true BACKUP_AGE_RECIPIENT="age1..." \
    scripts/backup.sh /var/lib/docker/volumes/grace-ride_app-data/_data /srv/backups 30
  ```
  Store the `age` private key offline (the host only needs the public recipient key) and sync `/srv/backups/*.age` off-site.
- **Non-Destructive Restore Drill (`scripts/restore_drill.sh`) — run before launch and quarterly:**
  ```bash
  PG_CONTAINER=grace-ride-db BACKUP_AGE_KEY_FILE=/root/.config/grace-ride/backup-age.key \
    scripts/restore_drill.sh /srv/backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz.age
  ```
  `restore_drill.sh` decrypts and restores into a disposable `grace_ride_drill_*` database and temporary directory without touching production, reconciles every `files.stored_name` row against extracted files on disk, prints table counts, and drops the temporary database on exit.
- **Full Production Restore (`scripts/restore.sh`):**
  ```bash
  docker compose stop app
  PG_CONTAINER=grace-ride-db BACKUP_AGE_KEY_FILE=/root/.config/grace-ride/backup-age.key \
    scripts/restore.sh /srv/backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz.age /var/lib/docker/volumes/grace-ride_app-data/_data
  docker compose start app
  ```
  Always take a fresh backup before restoring over production.
- **Monitoring:** Point an external uptime monitor at `GET /api/health` (returns `200` with `{ status, db, version }` or `503` on DB failure; container `HEALTHCHECK` runs `/app/grace-ride healthcheck`). Monitor Docker JSON logs for `background job failed`, HTTP `5xx`, `rate limited`, and `blocked cross-origin request`.

---

## 6. Testing & Verification

`.github/workflows/ci.yml` is the authoritative verification pipeline (PostgreSQL 18 service container, Rust stable fmt/clippy/test/audit, Playwright `v1.65.0-noble` frontend lint/unit/build/e2e/audit, and Docker image build).

### Local Verification Commands

**Backend** (uses disposable per-test schemas inside `TEST_DATABASE_URL` and cleans up stale test schemas automatically—**never** point `TEST_DATABASE_URL` at a real database):

```bash
createdb grace_ride_test                 # once (or `make db-up` via docker-compose.dev.yml)
cd backend
export TEST_DATABASE_URL=postgres://localhost/grace_ride_test
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo audit                              # advisory ignores live in backend/.cargo/audit.toml

# Optional helper: print the default permission matrix for all 13 built-in roles
cargo run --example role_matrix
```

**Frontend & Browser E2E:**

```bash
cd frontend
npm ci
npm run lint                             # ESLint + jsx-a11y + react-hooks (--max-warnings 0)
npm run typecheck                        # tsc -b across app, test, and node project references
npm run test                             # Vitest + Testing Library suites
npm run build                            # prebuild license bundle + tsc -b + vite build
npm run test:e2e                         # Playwright suites against vite preview (:4173)
npm audit --omit=dev --audit-level=high
```

> [!WARNING]
> **Visual Baselines:** Playwright screenshot baselines (`frontend/e2e/visual.spec.ts-snapshots/*-linux.png`) are rendered inside `mcr.microsoft.com/playwright:v1.65.0-noble`. Keep the container tag in `.github/workflows/ci.yml` synced with `@playwright/test` in `frontend/package-lock.json`, and regenerate visual snapshots in that Linux container rather than macOS.

### Test Suite Inventory

| Layer | Suites | Coverage Scope |
|---|---|---|
| **Core API & Flows** | `backend/tests/api.rs`, `extended_features.rs`, `security_audit_fixes.rs`, `scenario_matrix.rs`, `flows.rs`, `gaps.rs`, `regressions.rs`, `edge_data.rs` | End-to-end HTTP handler integration against PostgreSQL: sessions, passkeys (software WebAuthn), TOTP replay/rate-limits, invites, dual-admin recovery, rides, round-trips, saved places, recurring schedules, care links, events & series, QR passes, calendar feeds, settlements, payout batches, demand analytics, files, session lock/auto-lock timeouts, request-size limits, security regression checks, and handler error branches |
| **RBAC, Rank & Scope** | `backend/tests/perms.rs`, `rank.rs`, `leadership.rs`, `billing_scope.rs`, `audit.rs` | Dynamic role editing, 5-tier ranked chain of command, President/VP admin appointment rules, cell-leader scoping, finance separation, audit log filtering/retention/export |
| **Events, Attendance & Visitors** | `backend/tests/audience.rs`, `attendance.rs`, `event_photo.rs`, `event_update_atomic.rs`, `visitors.rs` | Recursive sub-group visibility, single-source `audience.rs` SQL guard, masked event titles, QR check-in & walk-in attendance, cover photo access, transactional audience updates, one-time visitor pass lifecycle & cookie isolation |
| **Rides, Planning & Concurrency** | `backend/tests/plan.rs`, `routing.rs`, `trip_integrity.rs`, `workflow_fixes.rs`, `workflow_review_probe.rs` | Transport planning board batch locks, geocoding/OSRM routing & privacy, carpool trip triggers, stale-assignment guards, duplicate event ride block, driver-disable ride release, readiness reminder timing & care-pass guards |
| **Admin, Help & Sorting** | `backend/tests/admin_ux.rs`, `help_desk.rs`, `guides.rs`, `rides_sort.rs`, `users_sort.rs`, `reimbursements_sort.rs` | Admin UX endpoints, support ticket claiming/closing races, step-by-step guide CRUD & step validation, allowlisted sort columns |
| **Schema & Capacity** | `backend/tests/schema_baseline.rs`, `db_audit_fixes.rs`, `capacity.rs`, `scripts/capacity_baseline.sh` | Frozen `0001_init.sql` checksum, pre-squash upgrade replay without row loss, `0002`–`0009` migrations, constraint/index integrity, synthetic 1,000-member / 10-driver fixture & reminder timing |
| **Frontend Unit (Vitest)** | `frontend/src/**/*.test.{ts,tsx}` (including `ux-enhancements.test.tsx`, `extended-features.test.tsx`, `bugfixes.test.tsx`, `scenario-matrix.test.tsx`) | Auth/public gates, permissions, member/driver/rider/visit/care/attendance/report flows, admin pages/consistency/tab-counts, modal focus retention, Service Worker privacy/redirect guards, transport plan logic, help guides & `guideProgress` step-ID persistence, i18n key parity (`en`/`zh`), date formatting, calendar links, CSS tokens, WebAuthn helpers, CJK `pdfWriter` |
| **Browser E2E (Playwright)** | `frontend/e2e/smoke.spec.ts`, `visual.spec.ts`, `admin-a11y.spec.ts`, `help-guides.spec.ts`, `full-scenarios.spec.ts` | Mocked-API routing/login smoke tests, public `/care/:token` & `/visit/:token` & `/invite/:token` journeys, Terms v2 & AutoLock unlock flows, mobile & desktop light/dark visual regression snapshots, axe accessibility scans across member and admin screens, and step-by-step guide / walkthrough navigation |

### Verification Limits (Pre-Launch Evidence Still Required)

- Playwright E2E suites run against mocked API responses; full multi-role browser journeys against a live test PostgreSQL + Axum backend should be executed before launch.
- Real-device mobile verification (iOS Safari / Android Chrome passkeys, GPS permission denial/staleness, QR camera scanning, weak cellular handoffs, and trip-end location clearing).
- Production-like backup & restore drill reconciling PostgreSQL rows with `/data/uploads` files, plus sustained concurrent load testing against self-hosted OSRM.

---

## 7. Security Model

### Protected Assets & Threat Model

Grace Ride holds home/pickup addresses, phone numbers, live trip GPS, health-related assistance flags, chat messages, and payment receipts/payee details for ~1,000 church members (many elderly). Primary threats are account takeover, unauthorized insider browsing, cross-site attacks against signed-in browsers, link preview/scanner consumption of one-time tokens, and leaked database/upload backups.

### Technical Controls

| Domain | Enforced Control | Source |
|---|---|---|
| **Invite-Only Access** | Single-use 14-day invitation tokens; no public self-registration. Pending accounts are blocked at the extractor layer until staff approval. | `routes/auth.rs`, `routes/admin.rs`, `auth.rs` |
| **Passkeys & Staff MFA** | WebAuthn passkeys primary; password + TOTP supported. Every non-Member page (`Transport`, `Finance`, `Admin`) rejects sessions lacking passkey/2FA verification (`util::require`). | `routes/auth.rs`, `util.rs`, `perms.rs` |
| **Session Cookies** | 48-char random token in `HttpOnly`, `SameSite=Lax`, `__Host-`-prefixed `Secure` cookie (cookie-only; `Authorization` headers are ignored). Stored solely as SHA-256 hash. TTL: **12 h** for accounts holding any staff permission, **30 days** for regular members. Revoked on logout, disable, or recovery (along with calendar feed tokens and Web Push subscriptions). | `auth.rs`, `routes/admin.rs` |
| **Rate Limits & Enumeration Defense** | Bounded in-memory per-IP buckets (with TTL expiry & 50k cap): login `10/min`, TOTP/unlock `10/5 min` (plus per-member TOTP & backup-code `10/5 min` limits), passkey reauth `20/5 min`, recovery `3/h`, invites `20/10 min`, visitor lookup/claim `30/10 min`, caregiver links `60/10 min`, calendar feeds `30/10 min`, uploads `30/10 min`, receipt ref extraction `30/10 min`, push test `/api/push/test` `10/10 min`; plus `100 uploads / member / 24 h`. Unknown emails and passkey-less accounts receive deterministic decoy WebAuthn challenges keyed by `app_secrets` to prevent member enumeration. | `security.rs`, `routes/auth.rs` |
| **2FA & Credential Integrity** | TOTP steps are single-use (atomic DB conditional update blocks replay). Existing authenticators cannot be silently overwritten from a session. Adding/removing passkeys or 2FA requires authentication within the last **15 minutes** (renewable in place via `/api/auth/passkey/reauth/*` using the member's own passkey). | `routes/auth.rs` |
| **Account Recovery** | Offline identity check → 24 h one-time SHA-256 hashed link shown once. Privileged accounts require **two distinct administrators** (initiator ≠ approver; link shown only to approver). Finalizing wipes all old sessions, passkeys, TOTP, backup codes, calendar feed tokens, and push subscriptions. | `routes/admin.rs` |
| **Visitor & Caregiver Pass Isolation** | Visitor links (`visitor_passes`) store SHA-256 hashes; `GET` previews never consume the link, while explicit `POST` claims a `gr_visit` `HttpOnly` `SameSite=Lax` cookie scoped to `/api/visit` (`VisitorPass` never yields `CurrentUser`) and closes on terminal ride state, `valid_until`, reissue, or revocation. Caregiver "Safe Arrival" links (`caregiver_passes`, `/care/:token` → `/api/care/{token}`) also store SHA-256 hashes, omit street addresses and raw coordinates, mask return-leg destinations prior to approval, hide driver/vehicle details once completed, expose only pickup/driver status and pre-pickup `ready`/`delayed` actions for a single active ride, and close automatically on expiration or terminal ride states. | `routes/visitors.rs`, `routes/care.rs`, `security.rs`, `jobs.rs` |
| **CSRF & Browser Hardening** | `SameSite=Lax` cookies + mandatory `Origin` / `Sec-Fetch-Site` validation on all non-`GET` requests; strict CSP (`script-src 'self'`, `frame-ancestors 'none'`), `Permissions-Policy` (`geolocation=(self), camera=(self)` only), `X-Content-Type-Options: nosniff`, COOP/CORP, HSTS. | `security.rs` |
| **RBAC & Outbound Alert Safety** | Permissions stored in `roles` / `role_permissions` (`member_roles` trigger-synced). Unremovable invariants: `superadmin` always holds all 67 permissions and cannot be edited; only `superadmin` grants/revokes `superadmin`, edits a `superadmin`, or grants `admin_settings`; last active `superadmin` cannot be demoted/disabled; nobody can self-grant `Finance` permissions or strip their own `admin_roles` management right; `cell_leader` traffic views and demand/monthly reports are scoped to own created events (never global) and never include claim approval. Outbound alert webhooks validate scheme/credentials/metadata & private/loopback/reserved IPs with redirects disabled, mask saved webhook URLs in admin responses, and neutralize spreadsheet formula prefixes (`=`, `+`, `-`, `@`, `\t`, `\r`) in payout-batch and UI CSV exports. | `perms.rs`, `routes/roles.rs`, `routes/admin.rs`, `routes/community.rs`, `util.rs` |
| **Chain of Command** | 5 ranked roles: `superadmin > president > vice_president > finance > auditor`. Each tier appoints only the tier immediately below it and may command (demote/disable/enable/edit role within own permissions) strictly lower tiers—never self, peers, or superiors. Unranked roles (`admin`, `secretary`, `traffic_head`, custom) cannot touch ranked members. Appointing/demoting/disabling `admin` or granting `admin_roles.update` requires `superadmin`, `president`, or `vice_president` (VP promoting an ordinary member to `admin` must record the President's acknowledgement). Serialized under the roles-mutation database lock. | `perms.rs`, `routes/admin.rs`, `routes/roles.rs` |
| **Least-Privilege Projections** | Viewer-specific JSON projections: drivers see only pickup area (and only destination area on return legs) until staff approval, across `/rides`, `/plan`, and `/calendar/{token}.ics` feeds; passengers see driver phone only after approval; receipts/files and `/files/*/extract-ref` check per-use ownership/role (`files::can_access_file`) with bounded 512 KiB reads; `admin` is not implicitly `finance`; signup lists are organiser/admin-events only; hidden events mask their title as `"Church event"` on rides and calendar feeds; passenger route polylines omit segments containing other riders' pickups. | `routes/*.rs`, `audience.rs`, `routing.rs` |
| **Uploads & Body Caps** | JSON/API bodies capped at `128 KB` (uploads `9 MB` body / `8 MB` file); names ≤60 chars, phones 10–20 chars. Magic-byte MIME verification (`JPG`/`PNG`/`WEBP`/`HEIC`/`PDF`), random disk filenames, `CSP: sandbox`, attachment ownership verification on events, settlements, and help guides. | `lib.rs`, `util.rs`, `routes/files.rs` |
| **Live GPS & Routing Privacy** | GPS accepted only on `in_progress` rides, deleted on completion, swept when stale, capped at `MAX_LIVE_DRIVERS`. Geocoding/routing sends only address text and coordinates (never member names, phones, or client IPs; Photon is server-proxied). Geocode cache expires after 30 days; addresses are omitted from server logs. | `routes/rides.rs`, `routing.rs`, `jobs.rs` |
| **Audit Trail** | Logs every state mutation (logins, failed logins with IP, sign-outs, MFA/passkey changes, role/member updates, ride/event/bill/help actions, uploads, and actor-less system escalations/expiries) naming changed fields rather than sensitive payloads. Retained for `AUDIT_RETENTION_DAYS`. | `util::audit`, `routes/admin.rs` |
| **Container & Supply Chain** | Non-root user (`uid 10001`), `read_only: true` rootfs, `cap_drop: [ALL]`, `no-new-privileges`, memory/PID limits; lockfiles enforced (`--locked`, `npm ci`); CI runs `cargo audit` and `npm audit` + weekly Dependabot. | `Dockerfile`, `docker-compose*.yml`, `.github/` |

### Auto-Lock, Offline Cache & Backup Codes

- **TOTP Backup Codes:** Single-use codes generated on `/me`, stored hashed (SHA-256 salted by user ID), and wiped on account recovery.
- **Server-Enforced Session Lock & Local Snapshot Hygiene:** Organization `session_timeout_mins` and per-member `auto_lock_mins` (`>0` override, `0` org default, `-1` off) drive both the client `AutoLockProvider` overlay (`inert` background + QuickJump blocking) and server-side `security::evaluate_session_lock` (`sessions.locked_at` / `sessions.last_seen_at`). Locked or idle-expired sessions are blocked (`423`) across `/api/stream`, `/api/auth/logout-all`, and protected API endpoints until re-authenticated via `/api/auth/unlock` or `/api/auth/passkey/reauth/*`. Locking, logging out, or receiving HTTP `401` immediately wipes `gr_today_rides_v1` (`offlineSnapshot.ts`) from `localStorage` (and `logout` clears `gr_autolock_mins` / `gr_locked_user`), while `public/sw.js` never writes `/api/*` responses or capability-token navigation URLs (`/care/*`, `/visit/*`, `/invite/*`, `/recover/*`, `?code=`) to `CacheStorage`.

### Accepted Risks & Known Technical Gaps

1. **Ignored Rust Advisories (`backend/.cargo/audit.toml`):** `RUSTSEC-2023-0071` (`rsa` via unused `sqlx-mysql`) and `RUSTSEC-2026-0459` (`libsqlite3-sys` via unused `sqlx-sqlite` in `Cargo.lock` only) are ignored in `backend/.cargo/audit.toml` because neither crate is compiled into the PostgreSQL-only binary; re-evaluate on every SQLx upgrade.
2. **Inline Styles in CSP:** `style-src 'unsafe-inline'` is required by Leaflet marker positioning; script execution remains locked to `'self'`.
3. **In-Memory Rate Limiter:** Per-IP rate buckets live in memory and reset on container restart (per-account auth lockouts are persisted in PostgreSQL).
4. **Encryption at Rest & Secret Storage:** `scripts/backup.sh` supports `age` (`BACKUP_AGE_RECIPIENT`) and `gpg` (`BACKUP_GPG_RECIPIENT`) archive encryption enforced via `REQUIRE_ENCRYPTION=true`, while live PostgreSQL volumes rely on host disk encryption, TOTP secrets are stored in plaintext, and 40-bit backup codes are salted only with `user_id`. Host disk encryption + `REQUIRE_ENCRYPTION=true` are mandatory in production; full column-level hardening requires an external KMS/env key to encrypt TOTP secrets and HMAC backup codes.
5. **Multi-Account Admin Collusion:** Self-finance restrictions block single-account escalation (`id == u.id`), but an admin with invite + member + role permissions could theoretically create a sockpuppet account. Mitigate organizationally via multiple active admins and periodic review of `user.update`, `role.update`, and `recovery.approve` audit logs.
6. **Cloudflare-to-Origin TLS (`deploy/graceride.Caddyfile`):** Configured to load a Cloudflare Origin CA certificate (`/etc/caddy/certs/graceride-origin.{pem,key}`, overridable via `ORIGIN_CERT`/`ORIGIN_KEY`). Before production launch with real member data, provision that certificate on the host Caddy, switch Cloudflare SSL/TLS to **Full (strict)**, and optionally enable Authenticated Origin Pulls (`client_auth` block in `deploy/graceride.Caddyfile`).
7. **API-Level Terms Gate:** `TermsGate` blocks the React UI when `terms_version < TERMS_VERSION`, but authenticated non-`/auth/*` API handlers do not yet reject direct HTTP calls from an active member who has not yet accepted the latest version.

### Security Pre-Launch Checklist & Vulnerability Reporting

- [ ] Host volume encryption enabled and off-site encrypted backup (`REQUIRE_ENCRYPTION=true`) + `scripts/restore_drill.sh` drill completed
- [ ] Cloudflare Origin CA certificate installed and SSL/TLS set to **Full (strict)**, verified with an external TLS scan (e.g. SSL Labs)
- [ ] At least two distinct administrators enrolled with hardware/phone passkeys
- [ ] Full backend, frontend, E2E, and dependency audit suites green ([Section 6](#6-testing--verification))

**Reporting a Vulnerability:** Follow [`../SECURITY.md`](../SECURITY.md) and email the PJ State CMC IT contact configured in `.env.production` (`ACME_EMAIL`). Do not open public GitHub issues for security vulnerabilities.

---

## 8. Compliance, PDPA & Malaysian Law

> [!CAUTION]
> **Draft Legal & Compliance Review (prepared from codebase and public Malaysian legal sources — NOT legal advice).** Before production launch, PJ State CMC must have this review, the in-app Terms & Privacy Notice (`TERMS_VERSION = 2`), and its Bahasa Malaysia translation reviewed by Malaysian legal counsel and formally approved by church leadership.

### Compliance Summary & Action Owners

| Area | Current Implementation | Next Step & Owner |
|---|---|---|
| **PDPA s.7 Notice (English & Malay)** | Terms v2 (`frontend/src/locales/terms.{en,zh,ms}.ts`) with `"Baca dalam Bahasa Malaysia"` toggle on `/terms` | **Church:** Have a qualified translator verify `terms.ms.ts` and insert a named DPO/office contact email/phone |
| **Sensitive Data Consent (PDPA s.40)** | Explicit "Sensitive information" section in Terms v2; opt-in records `terms_accepted_at`, `terms_version`, and `terms.accept` audit log; `rides.assistance` is optional | **Counsel:** Confirm whether one combined opt-in tick suffices or a separate sensitive-data checkbox is needed |
| **Re-Consent on Material Changes** | `TERMS_VERSION = 2` (`routes/auth.rs` + migration `0007_terms_version.sql`) prompts existing v1 members via `TermsGate` | **Developer:** Increment `TERMS_VERSION`, update all 3 `terms.*.ts` files + migration default on future policy changes; optionally enforce in `CurrentUser` |
| **Visitors (No Account)** | Host invite sheet prompts host to obtain guest's consent first (`bringConsent`); `/visit/:token` displays `privacyNote` | **Counsel:** Confirm host-intermediated consent (or require a recorded host confirmation checkbox) |
| **Minors (<18)** | Terms instruct a parent/guardian to consent on behalf of minors; app does not verify age | **Church:** Collect guardian consent on ministry registration forms where youth/children ride |
| **Data Subject Rights (Access, Correction, Portability, Erasure)** | Self-service profile correction (`/me`); access, portability (in force 1 June 2025), and erasure are currently manual admin procedures (21-day statutory response window) | **Developer / Church:** Add self-service JSON data export and account anonymisation/closure, or document manual SOP |
| **Data Retention (PDPA s.10)** | Automated purge for GPS (trip end), chats (180 d), notifications (90 d read / 1 yr unread), audit logs (730 d), backups (30 d); no automated purge yet for profiles, rides, settlements, or disabled accounts | **Church:** Set retention periods (e.g. ~7 yrs financial records, shorter for inactive profiles). **Developer:** Add purge/anonymisation job |
| **DPO & Breach Notification** | 5-step breach runbook documented below; mandatory DPO & breach rules in force since 1 June 2025 (subject to JPDP guidelines/thresholds) | **Church:** Appoint a church officer (not the developer) as DPO / responsible privacy contact |
| **Cross-Border Transfers (PDPA s.129)** | Addresses/coordinates (never names/phones) sent to UK/EU OpenStreetMap services; disclosed in Terms v2 | **Church / Ops:** Accept GDPR-jurisdiction adequacy disclosure, or self-host Nominatim & OSRM in Malaysia |
| **Third-Party Map Servers** | Nominatim geocodes cached; Photon proxied; OSRM defaults to public demo server | **Ops:** **Self-host OSRM before launch** and set `GEOCODER_USER_AGENT` contact |
| **Volunteer Transport & Insurance** | Zero-fare model stated in Terms v2; church-rate reimbursements & cost-sharing only | **Counsel & Church:** Confirm Land Public Transport Act 2010 position; verify driver licence, road tax, and motor insurance cover |
| **Software & Brand Licensing** | Proprietary [`LICENSE`](../LICENSE) + [Third-Party Notices](#third-party-notices) below | **Developer & Church:** Sign written licence + PDPA data-processor agreement (if developer hosts); obtain written church permission for PJ State CMC logo arc |

### Personal & Sensitive Data Inventory

| Data Category | Storage | PDPA Classification |
|---|---|---|
| Name, email, phone, profile photo | `users`, `files` | Personal data |
| Church membership & fellowship group | `users`, `group_members`, `event_signups` | **Sensitive personal data (religious belief)** |
| Mobility / health assistance (`wheelchair`, `oxygen`, `stroller`, etc.) | `rides.assistance` | **Sensitive personal data (physical health)** |
| Home/pickup addresses & ride schedules | `rides`, `saved_places`, `recurring_ride_schedules`, `geocode_cache` (30 d) | Personal data (high physical safety sensitivity for elderly members) |
| Active trip GPS coordinates | Memory & `rides` (deleted on trip completion) | Personal data (high safety sensitivity) |
| Payee name/phone/account, DuitNow QR & receipt uploads | `settlements`, `reimbursements`, `payout_batches`, `files` | Financial personal data |
| Trip chats, notifications, help threads, sign-in IPs & audit logs | `ride_messages`, `notifications`, `help_*`, `audit_log` | Operational personal data |
| Visitor & elder proxy details | `visitor_passes`, `caregiver_passes`, `proxy_riders`, proxy `rides` | Personal data of non-members / represented riders |

### Personal Data Breach Runbook

1. **Contain immediately:** Disable compromised accounts in `/admin/members` (or run `DELETE FROM sessions;` in PostgreSQL to sign out all users), rotate `.env` / `/opt/grace-ride/secrets.env` credentials, and patch the vulnerability.
2. **Assess within 24 hours:** Determine affected tables, record counts, and risk of significant harm (addresses, live location, financial details, and elderly health/assistance data carry high risk).
3. **Notify JPDP (Personal Data Protection Commissioner):** Submit notification as soon as practicable within the statutory window (72 hours under the JPDP breach notification framework—verify current JPDP form and portal).
4. **Notify affected members:** Where significant harm is likely, notify affected individuals without undue delay in plain Chinese, English, and Bahasa Malaysia with actionable protective steps.
5. **Preserve audit evidence:** Export relevant entries from `/admin/audit` and retain a permanent incident log of findings and decisions, even if notification thresholds were not triggered.

### Third-Party Map & Routing Service Policies

- **OSRM (`router.project-osrm.org`):** Community demo server capped at ~1 req/s with zero SLA. **Self-host an OSRM Malaysia extract (`OSRM_URL`) before launch** so Sunday transport planning never depends on a volunteer demo server.
- **Nominatim (`nominatim.openstreetmap.org`):** Max 1 req/s, mandatory identifying `GEOCODER_USER_AGENT` with contact details, result caching enabled (`geocode_cache`), and never used for keystroke autocomplete.
- **Photon (`photon.komoot.io`):** Fair-use server-proxied autocomplete (`≥3` chars, debounced); client IPs are never exposed.
- **OpenStreetMap Standard Tiles:** Displays mandatory `"© OpenStreetMap contributors"` ODbL attribution link in `LiveMap.tsx`. Switch to a self-hosted or commercial tile server (and update `security.rs` CSP `img-src`) if traffic outgrows community tile fair-use limits.

### Transport Law, Insurance & Liability Notes

- **Land Public Transport Act 2010:** Carrying passengers for "hire or reward" without an e-hailing/public-service licence is unlawful. Counsel must confirm that church-set per-km fuel/toll reimbursements and event expense sharing do not constitute hire or reward.
- **Motor Insurance & Driver Vetting:** Private motor policies often exclude reward-based carriage; drivers should confirm volunteer carpooling coverage with their insurer, transport staff should verify valid driving licences and road tax manually upon driver approval, and the church should consider group volunteer personal-accident/liability cover (Malaysian law generally bars excluding liability for death or personal injury caused by negligence).
- **Electronic Consent:** Opt-in timestamps and `terms_version` records satisfy electronic agreement formation under the Electronic Commerce Act 2006.

### Legal References

- Personal Data Protection Act 2010 (Act 709) & Personal Data Protection (Amendment) Act 2024 (Act A1727) — phased commencement on 1 January 2025, 1 April 2025 (cross-border transfers, data processor security duties, enhanced penalties), and 1 June 2025 (mandatory DPO, data breach notification, data portability).
- OpenStreetMap [Copyright & ODbL](https://www.openstreetmap.org/copyright), [Nominatim Usage Policy](https://operations.osmfoundation.org/policies/nominatim/), and [OSRM Demo Policy](https://github.com/Project-OSRM/osrm-backend/wiki/Demo-server).

---

## 9. Copyright, Third-Party Notices & Brand Kit

### Proprietary Software Copyright

Grace Ride (恩典同行), including its source code, UI design, documentation, and brand assets in `docs/brand/`, is proprietary software (`Copyright (c) 2026 siewong007`, all rights reserved; see [`LICENSE`](../LICENSE)). PJ State CMC is granted a non-exclusive, non-transferable, royalty-free licence to host, operate, and modify the software solely for its own ministry. Both `frontend/package.json` (`"license": "UNLICENSED"`, `"private": true`) and `backend/Cargo.toml` (`publish = false`) prevent accidental public registry publication. Before launch, replace the GitHub handle in `LICENSE` with the developer's legal name and execute a short written licence and PDPA data-processing agreement between the developer and PJ State CMC.

### Third-Party Notices

#### Web Application Bundle (Distributed to Browsers)

`npm run build` automatically runs `frontend/scripts/third-party-licenses.mjs` (`prebuild`), generating `/third-party-licenses.txt` (linked from `/terms`) with full licence texts for all production npm packages.

Production dependency licence summary (77 resolved packages):

| Licence | Count | Notes |
|---|---|---|
| `MIT`, `ISC`, `BSD-2-Clause`, `Apache-2.0`, `MIT AND Zlib` | 72 | Permissive; notices bundled in `/third-party-licenses.txt`. |
| `MPL-2.0 OR Apache-2.0` (`dompurify`) | 1 | Used under `Apache-2.0`. |
| `OFL-1.1` (`@fontsource/jost`) | 1 | SIL Open Font License; may be bundled with the web app, never sold standalone. Noto Sans SC is used only as a system font fallback and is not bundled. |
| `Hippocratic-2.1` (`react-leaflet`, `@react-leaflet/core`) | 2 | **Non-OSI ethical source licence** (human-rights compliance, indemnity & arbitration clauses). Compatible with a church ministry app; if legal counsel prefers pure OSI terms, replace the thin `react-leaflet` wrapper with direct `leaflet` (`BSD-2-Clause`). |

#### Rust Backend Crates (Server-Only)

The Rust server runs on the church's infrastructure and is not distributed to end users. Across `backend/Cargo.lock`, all crates use permissive licences (`MIT`, `Apache-2.0`, `BSD`, `ISC`, `Zlib`, `Unicode`) except:
- `webauthn-rs` and sub-crates (`MPL-2.0` file-level copyleft — unmodified use creates no source-disclosure obligation; if modified and distributed externally, publish modified crate files).
- `r-efi` (`MIT OR Apache-2.0 OR LGPL-2.1+`, used under `MIT`, UEFI-only target not compiled on Linux).
- **Zero `GPL`, `AGPL`, or `SSPL` dependencies** are present in either stack.

**Whenever dependencies change**, regenerate and inspect third-party notices:

```bash
cd frontend && node scripts/third-party-licenses.mjs
```

### Brand Asset Guide (`docs/brand/grace-ride-logo-kit/`)

The **Pilgrim Pin** mark pairs a pointed pilgrim cross (*cross fitchée*, evoking a map pin) inside the blue arc derived from the PJ State CMC church logo (obtain written church/denomination permission before public launch).

| Colour Role | Hex | Typography & Asset Rules |
|---|---|---|
| **Grace Red** (cross, `"Grace"`) | `#E8462F` | **English Wordmark:** `Jost` — `"Grace"` Bold (`700`), `"Ride"` Light (`300`). |
| **Ride Blue** (arc, `"Ride"`, UI `--primary`) | `#1F5DB4` | **Chinese Line (新市堂):** `Noto Sans SC` Medium (`500`) & Bold (`700`). |
| **Church Grey** (small text) | `#474A4F` | **Clear space:** Maintain margin ≥ width of the cross's horizontal bar. |
| **One-Colour Charcoal** | `#2B2E33` | **Minimum size:** Symbol ≥ `24 px`; below `64 px`, use the thicker-stroke `app-icon` artwork. |
| **Light Blue** (on blue surfaces) | `#CFE0F7` | **Dark / photo backgrounds:** Always use the white mark variant (`--logo-*` CSS tokens switch automatically in dark mode). |

**Asset Inventory (`docs/brand/grace-ride-logo-kit/`):**
- `svg/grace-ride-mark*.svg` & `png/grace-ride-mark*-1024.png` — standalone symbol (full-colour, white, one-colour; pure vector paths).
- `svg/grace-ride-app-icon*.svg` & `png/grace-ride-app-icon*-1024.png` — `1024 × 1024` square app icons (white and blue backgrounds; PWA icons in `frontend/public/` derive from these).
- `svg/grace-ride-logo-horizontal.svg` & `svg/grace-ride-logo-stacked.svg` — full lockups with live editable `Jost` / `Noto Sans SC` text.
- `animation/grace-ride-loading-mark.svg` & `animation/grace-ride-loading-screen.html` — animated boot splash symbol and interactive preview.
