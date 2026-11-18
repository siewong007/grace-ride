# Audit and Refactor Plan

## Stack Summary
- **Backend:** Rust, Axum, SQLx (PostgreSQL 18), Tokio, WebAuthn.
- **Frontend:** React 18, TypeScript, Vite, Leaflet, i18next.
- **Infrastructure:** Docker, Caddy, OSRM (self-hosted routing), GitHub Actions, scripted encrypted backups.

## Problems Found

### Critical / High
- None immediately identified, but security model and RLS policies need a thorough review in Phase 4.

### Medium
- Dependency management: Need to verify `make audit` covers all needs.
- Testing: Need to ensure complete coverage for core flows (sign up, offer ride, etc.).
- Missing `docs/refactor/PROGRESS.md` and initial `CLAUDE.md`.

### Low / Minor
- Naming conventions: Need to ensure consistency across Rust/TS.
- File sizes: Some files might exceed 300 lines (need inspection).

## Phase 5 Checklist
- [ ] README covering stack, setup, environment, etc.
- [ ] `.env.example`, CHANGELOG, LICENSE.
- [ ] CI workflow (GitHub Actions) for install, lint, typecheck, test, build.
- [ ] Full test suite (unit, integration, E2E).
- [ ] Database migrations and seed data committed.
- [ ] Error monitoring and logging (Sentry example).
- [ ] Health-check endpoint.
- [ ] Documented environments and deployment steps.
- [ ] `CLAUDE.md` documenting standards.
- [ ] App/UX essentials (loading/error/empty states, A11y, translation).
- [ ] Ride/Event lifecycle edge cases.
- [ ] Terms/Privacy/Account deletion.

## Refactor Plan
- **Phase 0:** Understand (Done).
- **Phase 1:** Tooling & Standards (Linter, formatter, types, hooks, scripts).
- **Phase 2:** Structure & Naming (Domain-driven structure, consistent naming).
- **Phase 3:** Code Quality (Split files, extract services/hooks, schema validation, consistent errors).
- **Phase 4:** Security & Performance (Env vars, RLS/auth review, dependency audit, personal data, performance).
- **Phase 5:** Fill Gaps (Project/App/UX/Ride/Event essentials).
- **Phase 6:** Final Check.
