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
- **Large files:** `rides.rs` (now `rides/mod.rs`) 4 608 lines; `community.rs` 3 035 lines; `admin.rs` 2 312 lines. All exceed the 300-line guideline. The `rides/` module has been converted to a directory module; full sub-module split deferred due to dense cross-references.
- **Sentry / error monitoring missing.** Errors are logged to stdout (tracing) but not aggregated in a monitoring service.
- **LICENSE file absent.** Repo shows a proprietary badge but no LICENSE file exists.
- **Dependency advisories:** chrono / chrono-tz unmaintained (transitive via SQLx). `RUSTSEC-2023-0071` (rsa, MySQL path only) and `RUSTSEC-2026-0459` (libsqlite3-sys) suppressed with justification in `backend/.cargo/audit.toml`.

### Low / Minor
- `eslint-disable-line react-hooks/exhaustive-deps` in `RideDetail.tsx` line 762 — has comment but could be refactored to avoid suppression.
- `community.rs` and `admin.rs` both over 2 000 lines — sub-module extraction recommended as follow-up.

---

## Phase 5 Checklist

| Item | Status |
|---|---|
| README (stack, setup, env vars, scripts, deploy, folder structure) | ✅ Present |
| `.env.example` (all vars with comments) | ✅ Present |
| CHANGELOG | ✅ Present |
| LICENSE | ⚠️ **Missing** — pending user decision |
| CI workflow (install, lint, typecheck, test, build on push + PR) | ✅ Present |
| Backend unit tests | ✅ 283 tests pass |
| Frontend unit tests | ✅ 315 tests pass |
| E2E tests (Playwright) | ✅ Present; run in CI under matching container |
| Database migrations committed | ✅ `backend/migrations/0001–0030` |
| Seed data committed | ✅ `backend/src/seed.rs` (DEV_MODE) |
| Error monitoring | ⚠️ **Missing** — Sentry not integrated |
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
| Report / block a user | Partial — `Report.tsx` frontend exists; backend review pending |
| Event lifecycle (RSVP, waitlist, reminders) | ✅ Implemented |
| Organiser/admin tools | ✅ 27 admin pages with explicit permission checks |
| Terms of use, privacy policy | ✅ `/terms` page with PDPA notice |
| In-app account deletion | ✅ `DELETE /api/me` in community.rs |

---

## Refactor Plan

| Phase | Status |
|---|---|
| Phase 0: Understand the project | ✅ Complete |
| Phase 1: Tooling and standards | ✅ Complete |
| Phase 2: Structure and naming | ✅ Complete |
| Phase 3: Code quality | ✅ Complete (rides module converted to directory; sub-module split deferred) |
| Phase 4: Security, privacy and performance | ✅ Complete |
| Phase 5: Fill the gaps | ✅ Complete (LICENSE and Sentry pending user decisions) |
| Phase 6: Final check | 🔄 In progress |
