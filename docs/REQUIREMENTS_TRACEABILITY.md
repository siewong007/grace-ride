# Grace Ride — REQUIREMENTS_TRACEABILITY.md

Source of truth: `docs/Grace_Ride_Chinese.pdf` (44 pages, Chinese).
Per MASTER_TARGET.md §1: every product requirement is recorded here with PDF page reference, desired phase, acceptance criteria, corresponding code/module, and tests. Visual mockups (pp. 1, 5-10, 34-40) are concepts only — never claimed as implemented features.

Status scale: implemented (code exists) / partial (coverage incomplete) / gap (not mapped to code or tests) / verified (passing test evidence exists).

| ID | PDF page(s) | Requirement (summary) | Phase | Acceptance criteria | Code / module | Tests | Status |
|----|-------------|----------------------|-------|---------------------|---------------|-------|--------|
| R-01 | 1, 15, 26, 42 | Invite-only membership: no public self-registration; admin invites approved members | P1 | Only invite tokens create accounts; no public register endpoint | `backend/src/auth.rs` (invites, invite_accept) | tests/api.rs invite flows; RBAC E2E | implemented |
| R-02 | 12, 27 | Passkey (WebAuthn) login; no facial template stored by church | P1 | WebAuthn register/authenticate routes; RP id from PUBLIC_URL | `backend/src/auth.rs` passkey routes; `state::build_webauthn` | passkey unit coverage partial | implemented |
| R-03 | 12, 27 | 2FA for admins and sensitive account recovery | P1 | `require_any` enforces mfa_ok for transport/admin/finance | `backend/src/util.rs require_any`; `auth.rs CurrentUser.mfa_ok` | MFA middleware unit | implemented |
| R-04 | 12 | Account recovery via admin-reviewed one-time recovery link | P1 | recovery_requests table, invite kind 'recovery', single-use | `backend/src/auth.rs` recovery routes | recovery E2E | implemented |
| R-05 | 3, 4, 13, 15 | Three roles: admin, driver, passenger; least privilege server-side | P1 | Role checks in every mutating route; never UI-hiding only | `auth.rs ROLES/PRIVILEGED`; `require_any`; per-route require_any | RBAC/IDOR E2E per route | implemented |
| R-06 | 4, 16, 28 | Ride lifecycle: signup → driver chooses → passenger accepts → transport approves → completed | P1 | State machine transitions enforced atomically; no double-claim | `backend/src/routes/rides.rs` | ride lifecycle E2E | implemented |
| R-07 | 16, 28 | Passenger fills date/time, pickup area, seats, assistance needs | P1 | Validation of required fields | `rides.rs create_ride` | create_ride tests | implemented |
| R-08 | 16, 28 | Driver picks by area/time/seats; carpool & spare seats | P1 | driver_areas, spare_seats, carpool_enabled; driver_load capacity check | `rides.rs driver_load`; `seed.rs` areas | driver_load tests | implemented |
| R-09 | 16 | Passenger accepts or declines driver invitation | P1 | respond endpoint accepts/rejects | `rides.rs passenger_respond` | respond tests | implemented |
| R-10 | 4, 16, 28 | Transport staff approves; checks seat capacity, conflicts, driver consent | P1 | staff_approve enforces load+seats<=cap; audit log entry | `rides.rs staff_approve`; `util::audit` | approve tests incl. capacity | implemented |
| R-11 | 28 | Before approval: exact home address NOT exposed; only approximate pickup area | P1 | pickup_address projected only after approval to authorized parties | `rides.rs` privacy projection | privacy/IDOR E2E | implemented |
| R-12 | 9, 10, 37 | Live map: location shared only while in_progress; stops after complete/cancel; max 10 concurrent drivers | P1 | post_location gated by in_progress; MAX_LIVE_DRIVERS=10; stop_location revokes | `rides.rs post_location/stop_location`; `Config.max_live_drivers` | location E2E; max-10 boundary test | implemented |
| R-13 | 9, 10 | Location updates every 15-30 s; OpenStreetMap tiles (free), no Google billing | P1 | Frontend poll interval; Leaflet OSM tiles | frontend rides pages | frontend type-check; no auto-payment | implemented |
| R-14 | 8, 29 | Temp changes: driver withdraws → transport reassigns → new driver confirms → passenger notified | P1 | withdraw/reassign/confirm-assignment transitions; notifications | `rides.rs driver_withdraw/staff_reassign/driver_confirm_assignment`; `util::notify` | change-flow E2E | implemented |
| R-15 | 8, 29 | Cancel, no-show, 返回 (return) states handled | P1 | cancel_ride, no_show, complete transitions | `rides.rs` | status-transition tests | implemented |
| R-16 | 29 | No driver available → transport volunteers / Grab alternative per policy (manual, not platform-arranged) | P2 | needs_attention queue; alt_transport records | `rides.rs staff_alt_transport`; admin_rides | alt-transport E2E | implemented |
| R-17 | 18, 37 | Per-ride temporary group chat: opens on approval, closes/read-only after completion | P1 | list_messages/post_message gated by confirmed or in_progress | `rides.rs chat_access/list_messages/post_message`; `util::system_message` | chat access RBAC E2E | implemented |
| R-18 | 11, 31 | Notifications: reminders before rides; idempotent reminder job | P1 | notifications table; reminder job idempotent | `jobs.rs`; `util::notify` | reminder_job_notifies_passenger_and_driver | implemented |
| R-19 | 22-24 | Targeted activities: leader creates event with group-scoped visibility; signup/withdraw/waitlist | P1 | events visibility via groups; waitlist auto-promote | `backend/src/routes/events.rs` | events E2E | implemented |
| R-20 | 23-24 | Event signup can request ride or offer driving; ride formal only after confirm + approval | P1 | event_signups linked to rides; separate statuses | `events.rs`; `rides.rs` | linked signup/ride tests | implemented |
| R-21 | 25 | Personal monthly report: attendance by type, rides driven/received, settled costs, badges | P2 | Aggregates per user; completed rides only counted | `routes/community.rs` monthly_report + `GET /reports/monthly` | report E2E | implemented (test needed) |
| R-22 | 19-20 | Grab reimbursement: claim with receipt → transport approve/return/reject → finance verifies payee + records transaction (manual) | P2 | reimbursements statuses; receipt file required | `routes/settlements.rs` reimbursements | reimbursement E2E | implemented |
| R-23 | 20 | TNG/DuitNow manual payout: finance records transaction ID; no automatic TNG payout | P2 | payee account/phone; paid_ref recorded; no auto-payout code | `settlements.rs`; no TNG integration | no auto-payout (by design) | documented |
| R-24 | 20 | Future TNG auto-payout only if business API confirmed — NOT implemented without approval | — | No TNG integration code | — | — | excluded by policy |
| R-25 | 31-33 | Cost split: equal or custom; subsidy first; no negative amounts; cent-level remainder; total equals sum | P2 | settlement custom split validation; settlement_shares | `routes/settlements.rs` | settlement_custom_split_must_match_total | implemented |
| R-26 | 33 | Only attended members charged; proof needs payee confirmation ("confirmed") | P2 | shares only for attended; status confirmed by payee | `settlements.rs` | share-attendance rules | implemented |
| R-27 | 33 | Receipt visible only to payer, payee, authorized finance | P2 | files/receipts RBAC | `routes/files.rs`; `settlements.rs` | file RBAC E2E | implemented |
| R-28 | 33 | Revisions kept on changes; affected members notified; disputes stay pending | P2 | settlement_revisions table; notifications on change | `settlements.rs` | revision tests | implemented |
| R-29 | 21 | Volunteer badges: non-monetary, public criteria, opt-out | P2 | badges_opt_out; public criteria | `routes/community.rs` compute_badges + `GET /badges` | badge tests | implemented (test needed) |
| R-30 | 14 | Data retention: chat retention days, audit retention days | P1 | Config CHAT_RETENTION_DAYS/AUDIT_RETENTION_DAYS | `state.rs Config` + `jobs.rs` housekeeping | `tests/audit.rs` housekeeping retention purge test | implemented |
| R-31 | 14 | Audit log of admin operations; data correction & recovery channels | P1 | audit_logs table; audit() calls in mutating routes; /admin/audit API, /admin/audit/export stream, & page | `util::audit`, `routes/admin.rs`, `Audit.tsx` | `tests/audit.rs` (page, filter, retention, stream export), `admin.test.tsx` | implemented |
| R-32 | 15 | Phase 1 serves ~1,000 PJ State CMC members | P0 | DB tested at that scale | — | capacity baseline script | gap — not measured |
| R-33 | 15 | No ride fares, commissions, driver earnings, automatic rider-driver assignment | P0 | No payment for rides; no auto-assignment | code review | policy gate | by design |
| R-34 | 41 | Risks: operations rules, privacy control, third-party payment integration | — | Documented open questions | PROGRESS.md/BLOCKERS.md | — | tracked |
| R-35 | 14, 15, 25 | Scalable list pagination & infinite scroll: prevents silent data truncation across admin tables (rides, members, reimbursements, settlements) and member activity streams (notifications, past rides, past events, audit, costs); keyset composite indexes & client CSV export | P1 | Pagination metadata envelope ({ items, total, page, page_size, total_pages }) + cursor pagination ({ items, next_before }); backward-compatible with legacy arrays; URL query sync; accessible live announcements; zero scroll listeners | `backend/src/routes/{rides,admin,community,events,settlements}.rs`; `backend/migrations/0007_keyset_pagination_indexes.sql`; `frontend/src/components/ui.tsx` (<Pagination>, <InfiniteSentinel>, <BackToTop>, <VirtualList>) | `tests/api.rs`, `tests/audit.rs`, `admin.test.tsx`, `ui.test.tsx`, `format.test.ts` | implemented |

## Test coverage evidence required (MASTER_TARGET.md §7)
- Every inventoried actionable control in every implemented route/role/state: passing browser E2E check or documented blocked environment-only/manual case.
- Security-critical RBAC/IDOR/GPS/privacy and booking capacity/concurrency tests: PASS.
- Full unit/integration/E2E suites PASS with no unacknowledged skips; flake rate measured.

## Next step
Run baseline `cargo test` + `tsc -b` (see PROGRESS.md), then convert "implemented" rows above into passing test evidence, starting with T-1/T-4/T-5.


