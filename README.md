<div align="center">
  <img src="docs/brand/grace-ride-logo-kit/svg/grace-ride-mark.svg" alt="Grace Ride" width="76" />

  # 恩典同行 · Grace Ride

  **Privacy-first volunteer carpooling, fellowship events, and activity cost-sharing for PJ State CMC.**

  [![Rust](https://img.shields.io/badge/Rust-Axum-000000?style=flat-square&logo=rust)](backend/)
  [![Frontend](https://img.shields.io/badge/React_19-TypeScript-1F5DB4?style=flat-square&logo=react)](frontend/)
  [![Android](https://img.shields.io/badge/Android-Compose-3DDC84?style=flat-square&logo=android)](android/)
  [![Database](https://img.shields.io/badge/PostgreSQL-18-336791?style=flat-square&logo=postgresql)](backend/migrations/)
  [![Auth](https://img.shields.io/badge/Auth-Passkeys_%26_2FA-2B7549?style=flat-square)](docs/TECHNICAL.md#7-security-model)
  [![License](https://img.shields.io/badge/License-Proprietary-474A4F?style=flat-square)](LICENSE)

  [**Quick Start**](#quick-start) · [**Highlights**](#highlights) · [**Stack**](#stack) · [**Security**](SECURITY.md) · [**Technical Manual**](docs/TECHNICAL.md)
</div>

---

## Highlights

- **Invite-Only & Passkey-First** — Single-use member invites, family lists (up to 8 people), WebAuthn passkeys, TOTP + backup codes, server-enforced idle lock, and 73 granular permissions across 29 pages and 14 built-in roles (plus custom roles) with a 5-tier leadership chain of command.
- **Elder-Friendly Rides** — Event-driven ride booking with area-masked pickup privacy and pickup area photos, outbound/return legs, proxy riders, saved places with GPS & drag-pin picker, active-trip GPS/ETA, offline today-rides snapshot + trip status action queue, and zero-login `/care/:token` Safe Arrival links (opening 24 h before pickup).
- **Group Events & QR Check-In** — Draft/publish lifecycle, custom event icons, self-transport status, schedule timeline, recursive sub-group audiences, FIFO waitlists, self-service QR attendance (2 h pre-start to 12 h post-start), live ICS feeds, and one-time `/visit/:token` guest ride passes.
- **Traffic Workspace & Planning** — Unified `/admin/traffic` hub with drag-and-drop & mobile event ride planner, multi-version plan and single-ride change drafts with 10-min single-editor lease locks and lead approval, accessibility-aware auto-arrange with explainability drawer, ER-diagram plan image export, plan visitors, and atomic row-locked saves.
- **Transparent Ministry Finance** — Flat area-to-area mileage fares (with XLSX/CSV sheet import/export), post-event driver dispersal & visitor special reimbursements, integer-cent activity cost splitting with DuitNow QR & revision reconciliation, receipt-backed claims, and two-person monthly payout batches.
- **Bilingual & Multi-Client** — Chinese and English UI (with Bahasa Malaysia PDPA notice), dark mode, adjustable text scale, CJK PDF exports, Grok Bot Studio & animated mascot avatars, 4-tier achievement badges, an installable PWA shell, and a native Android app (Kotlin / Jetpack Compose).

## Quick Start

Requires **PostgreSQL 18** (or Docker via `make db-up`), **Rust** (stable), and **Node.js 22+**.

```bash
# 1. Start PostgreSQL & the API (http://127.0.0.1:8080)
createdb grace_ride_dev        # or `make db-up` (docker-compose.dev.yml: boots dev + test DBs)
cp backend/.env.example backend/.env
cd backend && cargo run        # or `make dev-api`
```

```bash
# 2. Start the web app (http://localhost:5173)
cd frontend && npm ci && npm run dev
```

<details>
<summary><strong>Demo accounts</strong> (seeded automatically on <code>localhost</code> when <code>DEV_MODE=true</code>)</summary>

<br />

Password for all demo accounts: `GraceRide2026!` · Simulated TOTP secret: `MFRGGZDF73DGXYP3`

| Role | Account |
|---|---|
| Superadmin (`/admin/settings`) | `superadmin@example.org` |
| Admin / Leadership | `admin@example.org`, `president@example.org`, `vicepresident@example.org`, `secretary@example.org` |
| Transport / Finance | `transport@example.org`, `traffichead@example.org`, `cellleader@example.org`, `finance@example.org`, `auditor@example.org` |
| Drivers / Members | `driver@example.org`, `driver2@example.org`, `member@example.org`, `amina@example.org`, `sophie@example.org` |

</details>

## Stack

| Layer | Technology | Path |
|---|---|---|
| **API & Jobs** | Rust, Axum, SQLx, WebAuthn, Tokio, SSE | [`backend/`](backend/) |
| **Web App** | React 19, TypeScript, Vite, Leaflet, i18next | [`frontend/`](frontend/) |
| **Android App** | Kotlin 2.2, Jetpack Compose, Material 3, MVVM | [`android/`](android/) |
| **Database** | PostgreSQL 18 (squashed `0001` baseline + additive migrations `0002`–`0006`) | [`backend/migrations/`](backend/migrations/) |
| **Ops & Deploy** | Docker, Caddy, OSRM, GitHub Actions, encrypted backup/restore scripts | [`deploy/`](deploy/), [`scripts/`](scripts/) |

## Documentation

- **[`docs/TECHNICAL.md`](docs/TECHNICAL.md)** — Authoritative engineering, architecture, configuration & compliance reference
- **[`docs/USER_MANUAL.md`](docs/USER_MANUAL.md)** — Church member & elder user guide (bilingual context, elder-friendly)
- **[`docs/ADMIN_MANUAL.md`](docs/ADMIN_MANUAL.md)** — Administration, transport planning, and finance operations manual
- **[`docs/BETA_TESTING_PLAN.md`](docs/BETA_TESTING_PLAN.md)** — Closed beta testing rollout & verification plan
- **[`docs/android/README.md`](docs/android/README.md)** & **[`docs/android/DESIGN.md`](docs/android/DESIGN.md)** — Native Android app architecture, setup, design system & test guide
- **[`SECURITY.md`](SECURITY.md)** — Vulnerability reporting policy and responsible disclosure runbook

Detailed sections in **[`docs/TECHNICAL.md`](docs/TECHNICAL.md)**:

- [Local Development & Tooling](docs/TECHNICAL.md#1-local-development--tooling) — setup, `docker-compose.dev.yml`, demo fixtures, and `Makefile` shortcuts
- [Features & Module Inventory](docs/TECHNICAL.md#2-features--module-inventory) — all 16 modules, routes, and explicit boundaries
- [User & Operational Workflows](docs/TECHNICAL.md#3-user--operational-workflows) — ride state machine, concurrency locks, and maintenance rules
- [System Architecture & Database](docs/TECHNICAL.md#4-system-architecture--database) — topology, 29-page / 73-permission catalog, and the consolidated `0001` baseline + `0002`–`0006` migrations
- [Configuration & Deployment](docs/TECHNICAL.md#5-configuration--deployment-runbook) — environment variables, Docker Compose, self-hosted OSRM, AIC VPS CI/CD, and encrypted backups
- [Testing & Verification](docs/TECHNICAL.md#6-testing--verification) — Cargo, Vitest, Android, and Playwright suites
- [Security Model](docs/TECHNICAL.md#7-security-model) — auth, RBAC invariants, pass isolation, privacy projections, and accepted risks
- [Compliance, PDPA & Malaysian Law](docs/TECHNICAL.md#8-compliance-pdpa--malaysian-law) — PDPA review, breach runbook, OSM/OSRM policies, and transport law
- [Copyright, Third-Party Notices & Brand Kit](docs/TECHNICAL.md#9-copyright-third-party-notices--brand-kit) — OSS licences and [`docs/brand/`](docs/brand/grace-ride-logo-kit/) guidelines

## License

Proprietary — Copyright © 2026 siewong007, licensed to PJ State CMC for ministry use. See [`LICENSE`](LICENSE) and [Third-Party Notices](docs/TECHNICAL.md#third-party-notices).
