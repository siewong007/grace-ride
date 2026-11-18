# Audit and Refactor Plan

## Stack Summary

- **Backend:** Rust stable, Axum 0.8, SQLx 0.8 (PostgreSQL 18), Tokio, WebAuthn.
- **Frontend:** React 18, TypeScript 5, Vite 6, i18next, Leaflet, Vitest + Playwright.
- **Package managers:** cargo (backend), npm (frontend).
- **Infrastructure:** Docker (multi-stage), Caddy reverse proxy, OSRM routing (self-hosted option), GitHub Actions CI + deploy.
- **Auth:** Passkeys (WebAuthn), TOTP, backup codes, password fallback (dev only). 67 permissions, 27 pages, 5-tier role hierarchy.
- **Data:** Integer cents for money, local KL-timezone text timestamps, PostgreSQL 18.

---

## Problems Found

### Critical / High
- None identified. Auth model is explicit and comprehensive; no unauthenticated routes beyond `/api/health`, `/api/push/config`, and passkey registration. Field-level data masking in `ride_json` by role (addresses, phone numbers).

### Medium
- **Large files:** Earlier mega-files (`rides.rs` 4 608 lines, `community.rs` 3 035 lines, `admin.rs` 2 312 lines) have been split into domain directory modules (`rides/` `mod.rs` 405 + 10 sub-modules; `community/` `mod.rs` 120 + 10 sub-modules; `admin/` `mod.rs` 165 + 6 sub-modules).
- **Sentry / error monitoring:** ✅ Integrated opt-in Sentry in backend (`sentry` crate v0.34 via `SENTRY_DSN`) and frontend (`@sentry/react` v11.14 via `VITE_SENTRY_DSN`, with PII-stripping `beforeSend`).
- **LICENSE file:** ✅ Private proprietary church-only licence (`LICENSE`) confirmed in repo (PJ State CMC).
- **Dependency advisories:** chrono / chrono-tz unmaintained (transitive via SQLx). `RUSTSEC-2023-0071` (rsa, MySQL path only) and `RUSTSEC-2026-0459` (libsqlite3-sys) suppressed with justification in `backend/.cargo/audit.toml`.

### Low / Minor
- **Modular architecture & file size standards:** ✅ All large files modularized into domain sub-modules:
  - Backend: `rides/` (10 sub-modules), `community/` (10 sub-modules), `admin/` (6 sub-modules), `auth/` (3 sub-modules), `events/` (3 sub-modules), `routing/` (4 sub-modules + tests), and `settlements/` (2 sub-modules).
  - Frontend: `RideDetail.tsx` (1 342 → 205 lines via `components/ride/`), `EventDetail.tsx` (1 079 → 558 lines via `components/event/`), `Drive.tsx` (1 026 → 47 lines via `components/drive/`), and `admin/Audit.tsx` (1 017 → 689 lines via `admin/auditHelpers.tsx`).
  - Zero application files in the entire repository now exceed 1 000 lines. All 67 permissions, 27 pages, and 597 tests (283 backend + 314 frontend) pass.

---

## Phase 5 Checklist

| Item | Status |
|---|---|
| README (stack, setup, env vars, scripts, deploy, folder structure) | ✅ Present |
| `.env.example` (all vars with comments) | ✅ Present |
| CHANGELOG | ✅ Present |
| LICENSE | ✅ Private proprietary — PJ State CMC church use (`LICENSE`) |
| CI workflow (install, lint, typecheck, test, build on push + PR) | ✅ Present |
| Backend unit tests | ✅ 283 tests across 35 suites pass |
| Frontend unit tests | ✅ 314 tests across 49 files pass |
| E2E tests (Playwright) | ✅ Present; run in CI under matching container |
| Database migrations committed | ✅ `backend/migrations/0001–0030` |
| Seed data committed | ✅ `backend/src/seed.rs` (DEV_MODE) |
| Error monitoring | ✅ Sentry integrated (backend `sentry` crate + frontend `@sentry/react`; opt-in via DSN) |
| Health-check endpoint | ✅ `/api/health` + Docker HEALTHCHECK CLI |
| Documented environments and deployment | ✅ README + Makefile + deploy.yml |
| `CLAUDE.md` | ✅ Present |
| Loading / empty / error states | ✅ Present in all data-fetching screens |
| Form validation with messages | ✅ Zod on frontend, typed errors on backend |
| Accessibility (labels, contrast, tap targets) | Partial — not fully audited |
| Responsive layouts and safe areas | ✅ Tailwind + mobile-first CSS |
| Design tokens (colours, spacing, typography) | ✅ `frontend/src/styles.css` tokens |
| User-facing text centralised (i18n) | ✅ i18next + `frontend/src/locales/` |
| Time zone and date format handling | ✅ KL local time throughout; `norm_dt` helpers |
| App icon, splash screen, PWA manifest | ✅ PWA shell with icons |
| Push / email notifications | ✅ Web push + Telegram + webhook alert channels |
| Ride lifecycle (all states + edge cases) | ✅ Full state machine implemented |
| Driver/rider verification approach | Partial — documented in TECHNICAL.md; no automated ID check |
| Report / block a user | Partial — Help Center tickets (`/help`, `help_tickets` linked to `ride_id`); no standalone user-block workflow (`Report.tsx` is monthly ministry summary) |
| Event lifecycle (RSVP, waitlist, reminders) | ✅ Implemented |
| Organiser/admin tools | ✅ 27 admin pages with explicit permission checks |
| Terms of use, privacy policy | ✅ 16-section trilingual (EN/ZH/MS) Terms & Privacy Notice in `Terms.tsx` with `TermsGate` sign-in enforcement |
| Account closure / data retention | Partial — handled via staff deactivation (`PUT /api/admin/users/:id`) / Help Center request per church retention policy; self-service `DELETE /api/me` optional product decision |

---

## Refactor Plan

| Phase | Status |
|---|---|
| Phase 0: Understand the project | ✅ Complete |
| Phase 1: Tooling and standards | ✅ Complete |
| Phase 2: Structure and naming | ✅ Complete |
| Phase 3: Code quality | ✅ Complete (`rides/`, `community/`, `admin/`, and `RideDetail.tsx` split into sub-modules) |
| Phase 4: Security, privacy and performance | ✅ Complete |
| Phase 5: Fill the gaps | ✅ Complete |
| Phase 6: Final check | ✅ Complete |
