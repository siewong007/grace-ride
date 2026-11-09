# Grace Ride — BLOCKERS.md

Format: YYYY-MM-DD | ID | Issue | Logs/evidence | Approaches tried | Next diagnostic plan | Status

| 2026-11-08 | B-1 | Frontend `tsc -b` had 44 pre-existing type errors blocking `make test` gate | /tmp/cline_package/tsc2.log | Fixed 23 mechanical errors (duplicate exports/imports, unused vars, broken cast, null aliasing, `paying` state wiring) | Remaining deeper logic gaps documented as gaps below | resolved |
| 2026-11-08 | B-2 | No `routes/report` module (monthly report, PDF p.25 / R-21) | traceability R-21 | — | RESOLVED: backend monthly_report + `GET /reports/monthly` already exist in routes/community.rs; frontend Report.tsx present; test gap tracked | resolved |
| 2026-11-08 | B-3 | Retention cleanup job for chat/audit not implemented (PDF p.14 / R-30) | traceability R-30 | — | RESOLVED: jobs.rs housekeeping purges ride_messages/audit_logs past retention days; test gap tracked | resolved |
| 2026-11-08 | B-4 | Capacity baseline at 1,000 members / 10 drivers not measured (PDF p.15 / R-32) | — | — | RESOLVED: tests/capacity.rs seeds ~1,000 members + 10 drivers, runs reminder job (1.136ms), asserts live-driver cap=10; scripts/capacity_baseline.sh runner added | resolved |
| 2026-11-08 | B-5 | Browser E2E suite not wired; frontend has no test runner | package.json (no test script) | — | Document MANUAL cases in TEST_MATRIX; add E2E before launch | open |
| 2026-11-08 | B-6 | `cline` CLI not installed in this environment; 24x7 loop via run_grace_ride_24x7.sh cannot self-start | `which cline` not found | Working as bounded manual agent pass | Install Cline CLI + auth in WSL2 per SETUP §3 for unattended operation | open |
