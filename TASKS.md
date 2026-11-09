# Grace Ride — TASKS.md (task inventory & priority)

Source of truth: docs/Grace_Ride_Chinese.pdf (44 pages). Traceability: docs/REQUIREMENTS_TRACEABILITY.md.

## Phase mapping (per PDF pp. 26, 42-43)
- Phase 1 — Core rides (P0): invite/login & security, passenger signup, driver choice, passenger confirm, transport approval, notifications, ride details/reminders, driver status updates, live map during trip (max 10), approximate area before approval.
- Phase 2 — Coordination (P1/P2): targeted activity events & signups, waitlist, Grab reimbursement flow, monthly report, volunteer badges, driver areas/seats/carpool.
- Phase 3 — Settlement (P2/P3): cost split (equal/custom), receipt upload + manual payee confirmation, revisions, disputes; TNG auto-payout studied only, NOT implemented (PDF p.20, p.41).

## Backlog (ranked; highest-priority safe unfinished slice first)
| ID | Task | Phase | Priority | Acceptance | Status |
|----|------|-------|----------|------------|--------|
| T-1 | Baseline test suite green: `cargo test` + `tsc -b`, no unacknowledged skips | P0 | P0 | All tests pass | done |
| T-2 | docs/REQUIREMENTS_TRACEABILITY.md: every PDF requirement → page, phase, code, tests | P0 | P0 | Complete inventory | done |
| T-3 | docs/TEST_MATRIX.md: per page/button/role/scenario test matrix incl. E2E | P0 | P0 | Complete matrix | done |
| T-4 | Ride state machine E2E coverage: signup→claim→respond→approve→start→pickup→complete incl. RBAC/IDOR | P0 | P0 | Browser E2E or documented blocked case | pending |
| T-5 | Privacy: exact pickup_address hidden until staff approval; driver sees area only | P0 | P0 | Unit + E2E | pending |
| T-6 | Live-location cap: MAX_LIVE_DRIVERS=10 enforced; location stops after trip complete/cancel | P1 | P1 | Unit + E2E | pending |
| T-7 | Temp ride group chat: opens on approval, read-only/closed after completion | P1 | P1 | Unit + E2E | pending |
| T-8 | Reimbursement flow: claim → transport approve/return/reject → finance payee confirm | P2 | P2 | Unit + E2E | pending |
| T-9 | Settlement split: equal/custom, cent-exact remainder, only attended members charged | P2 | P2 | Unit + E2E | pending |
| T-10 | Monthly report per member (rides driven/received, attendance, costs) | P2 | P2 | Unit + E2E | done (backend impl in community.rs; test gap) |
| T-11 | Volunteer badges with public criteria + opt-out | P2 | P2 | Unit + E2E | done (backend impl compute_badges; test gap) |
| T-12 | Targeted activity events + group-scoped visibility + signups/waitlist | P1 | P1 | Unit + E2E | pending |
| T-13 | Disaster recovery test in test env (DB backup/restore drill) | P0 | P0 | Documented + run | pending |
| T-14 | Accessibility automated audit pass | P1 | P1 | Automated checks pass | pending |
| T-15 | Performance/capacity baseline on dev hardware (1,000 members, 10 drivers) | P1 | P1 | Reproducible script + report | done |

## Done
None yet.

## Blocked
None yet.
