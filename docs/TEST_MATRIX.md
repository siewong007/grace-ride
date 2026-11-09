# Grace Ride — TEST_MATRIX.md

Per MASTER_TARGET.md §7 and SETUP §6: every page / button / role / scenario has a test type assigned. Browser E2E against a live frontend + API is not wired in this repository (no Playwright/Cypress in package.json); those cases are marked MANUAL and must be exercised by a human reviewer before launch. Unit/integration = `backend/tests/api.rs` + lib unit tests.

Legend: INT = backend integration test, UNIT = backend unit test, TS = frontend `tsc -b` type-check, MANUAL = human-verified in test environment only.

## Ride lifecycle (PDF pp. 4, 16, 28)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Passenger submits ride request (date, area, seats, assistance) | passenger | POST /api/rides | INT (ride_matching_flow...) | pass |
| Exact address hidden before staff approval | all | pickup_address projection | INT (ride_matching_flow_protects_address) | pass |
| Driver browses requests by area/time/seats | driver | GET /api/driver/requests | INT | pass |
| Driver claims a request; double-claim prevented | driver | POST /api/rides/{id}/claim | INT (atomic claim) | pass |
| Passenger accepts/declines driver invitation | passenger | POST /api/rides/{id}/respond | INT | pass |
| Staff approves; seat-capacity & conflict check | transport | POST /api/rides/{id}/approve | INT | pass |
| Ride start → pickup → complete transitions | driver | POST /start, /pickup, /complete | INT | pass |
| Cancel / no-show / return | passenger/driver/staff | POST /cancel, /no-show, /return | INT | pass |

## Access control / RBAC / IDOR (PDF pp. 3, 13, 15)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Invite-only registration; no public signup | guest | POST /api/invites/{token}/accept | INT | pass |
| Session cookie HttpOnly; token hashed in DB | all | sessions table | INT (session_tokens_are_stored_hashed) | pass |
| Privileged roles require passkey/2FA (mfa_ok) | transport/admin/finance | require_any | INT | pass |
| Cross-origin state-changing writes blocked | all | origin_guard | INT (cross_origin_writes_are_blocked) | pass |
| Per-IP rate limit on login | guest | POST /api/auth/login | INT (per_ip_rate_limit_on_login) | pass |
| Account lockout after repeated failures | guest | login failures | INT (account_lockout_after_repeated_failures) | pass |
| Dev mode refused on public domain; HTTPS required | config | Config.validate | UNIT (production_config_rejects...) | pass |
| Member cannot view another member's address/details | passenger | ride/reimb/file projections | INT (uploads_are_sniffed_and_access_controlled, role_based_access) | pass |
| Finance receipts visible only to payer/payee/finance | finance | GET /api/files/{id} | INT | pass |
| Non-organiser cannot create event settlement | member | POST /api/events/{id}/settlements | INT (settlement_custom_split...) | pass |

## Live location (PDF pp. 9, 10)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Location shared only while trip in_progress | driver | POST /api/rides/{id}/location gated | INT | pass |
| Location stops after complete/cancel | driver | DELETE /api/driver/location | INT | pass |
| Cap: max 10 concurrent live drivers | config | MAX_LIVE_DRIVERS=10 | UNIT (Config.validate 1..=100) | pass |
| E2E boundary test: 11th driver rejected | — | — | MANUAL (no automated harness) | gap |

## Temp changes (PDF pp. 8, 29)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Driver withdraws → staff reassigns → new driver confirms → passenger notified | transport/driver | POST /withdraw, /reassign, /confirm-assignment | INT | pass |
| Passenger cancels; seats released | passenger | POST /cancel | INT | pass |
| No driver available → needs_attention queue; Grab alternative is manual off-platform | transport | GET /api/admin/rides | INT | pass |

## Group chat (PDF pp. 18, 37)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Chat opens on approval; driver+passengers joined | confirmed ride | GET/POST /api/rides/{id}/messages | INT | pass |
| Chat read-only/closed after completion | completed ride | post_message rejected | INT | pass |
| Stranger cannot read ride chat | other member | chat_access forbidden | INT | pass |

## Activities & signups (PDF pp. 22-24)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Leader creates event with group-scoped visibility | leader | POST /api/events | INT | pass |
| Members see only their groups' events | member | GET /api/events | INT | pass |
| Signup / waitlist / withdraw; waitlist auto-promotes | member | POST /api/events/{id}/signups | INT | pass |
| Event signup can request ride / offer driving; ride formal only after confirm + approval | member | linked event_signups + rides | INT | pass |
| Organiser records attendance after event | leader | signups attended | INT | pass |

## Settlements & reimbursements (PDF pp. 19-20, 31-33)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Grab reimbursement claim with receipt | member | POST /api/reimbursements | INT | pass |
| Transport approves / returns / rejects claim | transport | POST /api/reimbursements/{id}/review | INT | pass |
| Finance verifies payee, records transaction ID | finance | POST /api/reimbursements/{id}/pay | INT | pass |
| Cost split equal / custom; cent-exact; total must equal sum | leader | POST /api/events/{id}/settlements | INT (settlement_custom_split_must_match_total) | pass |
| Only attended members charged | settlement shares | attendance-gated shares | INT | pass |
| Revisions kept; members notified on change | settlement | settlement_revisions + notifications | INT | pass |
| TNG/DuitNow manual payout recorded, NO automatic payout | finance | payee_account + paid_ref fields | code review (no TNG integration) | documented |
| Public self-payment for rides prohibited | — | no fare/commission code | code review | by design |

## Notifications & reminders (PDF pp. 11, 31)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Ride reminder before pickup; idempotent re-run | system | jobs::run_once | INT (reminder_job_notifies_passenger_and_driver) | pass |

## Auth & security (PDF pp. 12, 27)
| Scenario | Role | Control | Test | Status |
|----------|------|---------|------|--------|
| Passkey register/login (WebAuthn) | all | /api/auth/passkey/* | INT (partial) | pass |
| TOTP 2FA setup/enable/verify with replay protection | all | /api/auth/totp/* | INT (partial) | pass |
| Account recovery via one-time link | member | /api/recovery-* | INT | pass |
| Security headers: CSP, HSTS, frame-ancestors DENY | all | middleware | TS + INT headers | pass |

## Frontend (PDF pp. 34-40 concepts)
| Area | Control | Test | Status |
|------|---------|------|--------|
| All pages type-check under strict TS | React pages/components | `tsc -b` | pass |
| No unused imports/broken casts block build | Login.tsx, Members.tsx, Settlements.tsx, etc. | `tsc -b` | pass (fixed in this session) |
| UI screens are design concepts (mockups only) | pp. 34-40 | — | not claimed as implemented |

## Gaps (recorded in BLOCKERS.md / TASKS.md)
- R-21 monthly report page: no `routes/report` module; no tests — gap
- R-29 badges: `compute_badges` in routes/community.rs exists; badge page/tests unconfirmed — partial
- R-30 retention cleanup job (chat/audit expiration) — not implemented
- R-32 capacity baseline at 1,000 members / 10 drivers — not measured
- Browser E2E suite — not wired; MANUAL only
