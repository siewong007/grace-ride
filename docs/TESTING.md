# Testing and verification

Inventory audited 14 November 2026. This documentation audit did not run application suites, measure performance or inspect live deployment. Historical pass counts have deliberately not been carried forward as current evidence.

## Local commands

Install dependencies and configure local PostgreSQL using [Deployment](DEPLOYMENT.md). Use a disposable test database: tests create schemas and remove matching stale test schemas. Never point `TEST_DATABASE_URL` at production.

```bash
createdb grace_ride_test                 # once, if it does not exist
cd backend
export TEST_DATABASE_URL=postgres://localhost/grace_ride_test
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2026-0459
```

Requires Rust/Cargo on PATH and `cargo-audit` installed. The repository's `dev-env.sh` instead selects the owner's external-SSD Rust toolchain. Use that wrapper only when that toolchain is available.

```bash
cd frontend                            # from repository root
npm ci
npm run lint
npm run test
npm run build
npm run test:e2e
npm audit --audit-level=high
```

Install the matching Playwright Chromium browser if missing (`npx playwright install chromium`). Browser tests use the built Vite preview server on port 4173. CI uses the Playwright container pinned in `.github/workflows/ci.yml` so Linux screenshot fonts match; keep it aligned with the lockfile. Inspect changed baselines before accepting them.

`make test` runs backend tests and frontend TypeScript only; `make lint` runs Rust checks only. Neither replaces the complete CI frontend suite. `.github/workflows/ci.yml` is the authoritative CI command list, including advisory exceptions and image build.

## Coverage inventory

| Area | Existing evidence location | Scope / limits |
|---|---|---|
| Sessions, rides, privacy, roles, jobs, uploads, settlements | `backend/tests/api.rs` | HTTP handlers against test PostgreSQL |
| Passkeys, TOTP/replay, invitations, recovery, concurrency, reimbursement, pending accounts, receipts and request limits | `backend/tests/gaps.rs` | Includes software WebAuthn authenticator and security regression cases |
| Permission changes and settings | `backend/tests/perms.rs` | Backend authorization/settings tests |
| Audit behavior, filtering, retention and export | `backend/tests/audit.rs` | Audit-specific integration coverage |
| Planning and routing | `backend/tests/plan.rs`, `backend/tests/routing.rs` | Plan validation and routing behavior |
| Capacity | `backend/tests/capacity.rs`, `scripts/capacity_baseline.sh` | Synthetic thousand-member fixture, configured live-driver cap and reminder timing; not proof of sustained production throughput or rejection of an actual eleventh live driver |
| UI components and pages | `frontend/src/**/*.test.ts*` | Vitest/Testing Library, including permissions, auto-lock, locales and formatting |
| Browser smoke | `frontend/e2e/smoke.spec.ts` | Mocked API, public/login/routing checks |
| Visual/accessibility | `frontend/e2e/visual.spec.ts` and snapshots | Mocked API, selected pages at mobile/desktop sizes, themes and axe checks; not whole-product accessibility certification |

## Still needs current evidence

- Full browser journeys against a real test API for member, driver, transport, organizer and finance roles, including failure/recovery paths.
- Real-device passkey, GPS denial/staleness, weak-network and trip-end privacy checks.
- Restore drill including database and uploaded-file reconciliation, and measured load/concurrency limits.
- New-feature-specific coverage review for proxy booking, readiness/delay and backup codes; source presence alone does not establish comprehensive tests.
- Release-level checks across phone/tablet/desktop, English/Chinese, light/dark and large text. Automated axe checks cover only sampled pages and rules.

For each verification record, identify commit/working-tree changes, commands, environment, outcomes and skipped/blocked checks. Summarize relevant evidence in the canonical topic or change review; use CI/Git history for old runs rather than another maintained documentation copy.
