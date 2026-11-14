# Resolved issues and regression notes

Consolidated 14 November 2026 from Git history and the superseded run/UI reports. These are recorded fixes, not results of a fresh application test run. Commit IDs identify historical evidence; current tests must be rerun using [Testing](TESTING.md).

| Previous issue | Recorded resolution | Evidence / regression check |
|---|---|---|
| Frontend baseline failed with 44 TypeScript errors | Removed duplicate/unused declarations; repaired translation plumbing, nullable data access and payment state wiring | Historical `d72c6be`; run frontend build/type-check |
| Backend errors ignored member language | Routed localized errors through member language or request language; generic DB/internal errors retain default-language fallback when context is absent | `a102353`; `backend/src/error.rs`, `backend/src/i18n/`, locale tests |
| Monthly report and retention jobs incorrectly reported missing | Corrected documentation: implementations already existed in community and jobs modules | `backend/src/routes/community.rs`, `backend/src/jobs.rs`; this was an inventory error, not a new feature |
| No capacity baseline existed | Added synthetic thousand-member/ten-driver fixture and reminder timing runner | `backend/tests/capacity.rs`; old report recorded about 1.14 ms on its machine, not a current benchmark or production SLA |
| Browser/unit test tooling reported absent | Vitest, Playwright smoke/visual tests and CI wiring were added | `762fa60` and subsequent UI commits; `frontend/package.json`, `frontend/e2e/` |
| Settlement conditional hook, custom-share form and UI navigation defects | UI foundation work repaired hook usage, form controls/validation, missing strings, links and public 404 handling | `762fa60`; settlement/routing tests and build |
| Admin live map query rejected by PostgreSQL | Grouped by the driver-location key so latitude/longitude selection is valid | `617141c`; `admin_live_map_with_driver_location` in `backend/tests/gaps.rs` |
| Abandoned per-test schemas caused backup lock pressure | Test helper labels and removes stale matching schemas in the test database | `09d0d15`; `backend/tests/common/schema.rs` |
| Home invalid date/crash baseline, missing Phone label, undefined CSS tokens, crowded driver actions and date-dependent test | Minimalist phase-zero corrected rendering, locale/CSS issues and the affected test/baselines | `da23c54`; frontend page/token tests and visual suite |
| Time/date presentation, unsupported admin status filters, raw translation keys and duplicate headings | Minimalist/UI follow-ups corrected formatting, filters, labels and headings | Minimalist series starting `f569310`; `frontend/src/format.ts`, page tests |
| Untranslated toast-close and Help thread labels | Added localized labels | `95c8fdb`; i18n tests |
| Live-map legend disagreed with stale-driver marker | Stale drivers render in amber with matching legend | `6b8b51e`; `frontend/src/components/LiveMap.tsx`, admin Live page |
| Pending-account restrictions bypassable via alternate token channel | Cookie-only authentication aligned extractor and pending guard | `4d79880`; `pending_member_is_kept_out_however_the_token_is_sent` |
| Unauthorized settlement receipt attachment and recovery-link exposure | Enforced organizer ownership; tightened recovery token handling | `4d79880`, `f50b62a`; `settlement_receipt_must_be_the_organisers_own_upload`, `recovery_links_are_stored_hashed` |
| Sign-in method changes and public request surfaces needed hardening | Recent-auth checks, bounded bodies/uploads, profile validation and related security fixes | `f50b62a`; named regression cases in `backend/tests/gaps.rs` |
| Production deployment could precede passing CI | Automatic deployment now follows successful CI for a master push; explicit manual dispatch remains available | `f50b62a`; `.github/workflows/deploy.yml` |
| Decoy passkey response could reveal account state | Always supplies a credential in decoy challenges | `c3ebbb7`; `passkey_login_start_does_not_reveal_who_is_a_member` |
| Older documentation described one migration, no browser tests, SQLite indexes, future PostgreSQL and obsolete UI colors | Consolidated docs now describe numbered PostgreSQL migrations, actual tooling and current blue-accent UI | Documentation audit of 14 November 2026; source/path/link checks |

## Do not confuse with resolved defects

The old missing-Cline-CLI blocker concerned retired automation tooling, not application behavior. Current product and verification gaps remain in [Features](FEATURES.md#boundaries-and-outstanding-work) and [Testing](TESTING.md#still-needs-current-evidence). The audit did not resolve those gaps.
