# Grace Ride — Native Android Audit (Phase 1)

Date: 2026-11-21 · Auditor: Hermes (senior Android engineer pass)
Scope: read-only inspection of `frontend/`, `backend/`, `docs/`. No production changes made.

---

## 0. Executive summary — domain mismatch with the brief

The brief's Phase 3 list (product listing, product details, cart, checkout, payments)
assumes an e-commerce site. **Grace Ride is not a store.** It is a privacy-first
volunteer carpooling, fellowship-event and activity cost-sharing platform for
PJ State CMC:

- Backend: Rust · Axum 0.8 · SQLx · PostgreSQL 18 (184 REST routes under `/api`).
- Frontend: React 19 · TypeScript · Vite PWA, bilingual zh-CN/en.
- There are **no products, no cart, no orders, no payment gateway**. The app
  never moves money (`backend/src/routes/settlements/mod.rs`: "Grace Ride never
  moves money — members pay via TNG / DuitNow / bank"). Cost sharing is done by
  displaying a payee's DuitNow QR and uploading a transfer proof, which staff verify.

The native app therefore mirrors the **real** website. Mapping of the requested
commerce screens onto the actual domain:

| Brief (commerce)        | Grace Ride equivalent                          | Android screen            |
|-------------------------|------------------------------------------------|---------------------------|
| Home                    | Home dashboard (next ride, today's driving, events, announcements, unpaid shares) | `HomeScreen` |
| Product listing         | My rides / Events listing                      | `RidesScreen`, `EventsScreen` |
| Product details         | Ride detail / Event detail                     | `RideDetailScreen`, `EventDetailScreen` |
| Search                  | Place search (`/places`), ride & event filters | search inside Rides/Events/RideNew |
| Cart                    | My cost shares (`/me/shares`) — amounts owed   | `CostsScreen`              |
| Authentication          | Passkey sign-in (WebAuthn), TOTP, session lock | `LoginScreen`, `UnlockScreen` |
| Checkout                | Book a ride (`POST /rides`); submit share payment proof | `RideNewScreen`, `ShareProofScreen` |
| Order history           | Past rides, reimbursements, monthly report     | `HistoryScreen`, `ReimbursementsScreen` |
| Account settings        | Profile, passkeys, 2FA, language, auto-lock    | `ProfileScreen`            |

"Secure payments" on Android = DuitNow QR display + proof upload + staff
verification. **Google Play Billing is not applicable** (no digital goods;
real-world fuel/event cost splitting is exempt from Play billing policy).

---

## 1. Website screens & navigation

Source: `frontend/src/App.tsx`, `frontend/src/bottomNav.ts`, `frontend/src/components/Layout.tsx`.

Public (no session): `/login`, `/terms`, `/licences`, `/invite/:token`,
`/visit/:token` (guest ride pass), `/care/:token` (caregiver safe-arrival view).

Authenticated shell (`RequireAuth` + `AutoLockProvider` + `Layout`):

| Route | Page | Gate (permission) |
|---|---|---|
| `/` | Home | any signed-in member |
| `/rides` | Rides (my rides) | `rides` |
| `/rides/new` | Book a ride | `rides.create` |
| `/rides/:id` | Ride detail (+ chat, location, readiness) | visibility enforced server-side |
| `/drive` | Driver workspace (requests, schedule, route, profile/vehicles) | `drive` |
| `/events` | Events listing | `events` |
| `/events/new`, `/events/:id/edit` | Event form | `events.create` / `events.update` / `admin_events.update` |
| `/events/:id` | Event detail (RSVP, waitlist, QR) | `events` or `admin_events` |
| `/events/:id/checkin` | Check-in (code / QR / member pass scan) | `events` or `admin_events` |
| `/events/:eventId/settle` | Create settlement | `settlements.create` |
| `/settlements`, `/settlements/:id`, `/settlements/:id/edit` | Fuel/event cost splitting | `settlements` / `admin_bills` |
| `/costs` | My shares (pay via DuitNow QR, upload proof) | `costs` |
| `/reimbursements`, `/reimbursements/new`, `/reimbursements/:id` | Grab/mileage claims | `reimbursements` |
| `/help`, `/help/:id`, `/help/guides/*` | Help desk & guides | `help` / `admin_help` |
| `/report` | Monthly activity report & badges | `report` |
| `/notifications` | Notification inbox | any |
| `/me`, `/me/family` | Profile & family/proxy riders | any |
| `/admin/*` (14 pages) | Dashboard, rides, live map, members, settings, reimbursements review, announcements, audit, reports, roles, transport plan, menu | per-page admin perms |

Navigation model:
- Phone (<768px): bottom tab bar — **Home · Events · Admin (staff only) · Profile**.
  Rides/Drive/Costs/Reimbursements/Report live under the Profile section.
- Tablet: icon rail; desktop: full sidebar. Swipe gestures between tabs.
- Staff permissions are hidden until the session is MFA-verified (`mfa_ok`).
- Server-enforced idle lock (`AutoLock` + `/auth/lock` / `/auth/unlock`, HTTP 423).

## 2. Backend API endpoints

Base: `/api` (same origin as the web app; production behind Caddy HTTPS).
Full inventory: **184 routes** — table at the end of this document, extracted
programmatically from every `.route(...)` registration in `backend/src/routes/`.
Cross-cutting routes: `GET /api/health`, `GET /api/stream` (SSE live updates).

Conventions that the Android client must honour:
- JSON bodies ≤128 KB (uploads go through `POST /api/files`, multipart).
- `Accept-Language: zh|en` selects bilingual error messages; errors are
  `{ "error": string, "code": string, "params"?: {...} }`.
- Money is **integer cents** (`amount_cents`, `subsidy_cents`, …). Never floats.
- Timestamps are KL-local text (`YYYY-MM-DD HH:MM:SS`), not ISO-8601 UTC.
- Special codes: `SESSION_LOCKED` (HTTP 423), `RIDE_STATUS_CHANGED` (optimistic
  concurrency on ride actions), `ACCOUNT_PENDING` (403 until staff approval).
- Rate limiting per IP/session; `x-request-id` echoed on every response.
- Optimistic concurrency: ride mutations fail with `RIDE_STATUS_CHANGED` when
  the state moved; client must re-fetch and retry.

## 3. Authentication & session handling

Source: `backend/src/routes/auth/`, `backend/src/auth.rs`, `backend/src/security.rs`.

- **Password login is disabled server-side** (`POST /auth/login` always returns
  `AuthDisabled`). Sign-in is **passkey-only (WebAuthn)**:
  `POST /auth/passkey/login/start` → challenge JSON →
  `POST /auth/passkey/login/finish` → session cookie.
- Registration: `POST /auth/passkey/register/start|finish` (also used for re-auth
  of privileged sessions: `/auth/passkey/reauth/*`).
- TOTP 2FA: `/auth/totp/setup|enable|verify`, backup codes regenerate; used for
  session unlock and privileged re-auth. Argon2 password hashes exist only as a
  legacy/set-password path (`/auth/password`), not for login.
- Invites: single-use `/invites/{token}` preview + accept; account recovery via
  admin-approved requests. Terms acceptance gate (`/auth/terms`, versioned).
- **Session cookie**: `gr_session` (dev/http) or `__Host-gr_session` (prod/https),
  HttpOnly, SameSite=Lax, Secure in prod, Path=/. Token = 48 random chars,
  stored server-side as SHA-256. Lifetime: **30 days** regular members,
  **12 hours** privileged (staff) accounts.
- **Server-enforced idle lock**: `session_lock_guard` locks sessions after the
  user's `auto_lock_mins` / system idle timeout → HTTP **423 SESSION_LOCKED** on
  every route except a bypass list (unlock, passkey reauth, me…). Unlock via
  TOTP/backup code or passkey reauth.
- `pending_guard`: invited-but-unapproved members may only use profile/passkey/photo.
- **CSRF/origin guard**: state-changing requests must carry `Origin` equal to the
  configured public origin **or no Origin at all** (non-browser clients pass).
  → A native OkHttp client that sends no `Origin` header works **without any
  backend change**. The Android app must NOT send a foreign Origin.
- Dev mode only: `GET /auth/dev-users`, `POST /auth/dev-login` (seeded accounts,
  `DEV_MODE=true`; production returns 404). Useful for emulator verification.
- `GET /auth/me` returns the full session profile incl. `permissions`
  (`page.action` strings, staff perms withheld until `mfa_ok`), `locked`,
  `unread`, `groups`, `terms_accepted`, `auto_lock_mins`, `session_timeout_mins`.
- Web push is VAPID (browser-only). SSE `/api/stream` pushes change kinds
  (`notify`, ride/event updates); web polls as fallback (4× relaxed while SSE open).

### Android implications
- Use **Credential Manager** (`androidx.credentials`) for passkey get/create;
  feed it the server's `requestJson` verbatim.
- Passkeys on Android require the RP ID domain to trust the app via
  **Digital Asset Links** (`/.well-known/assetlinks.json` on the production
  host, with the app's package name + signing SHA-256). **This is a production
  change → needs explicit approval + the release signing fingerprint.**
  Until then: dev-login (DEV_MODE) or a WebView-free fallback is not possible
  for passkeys; emulator verification uses dev-login.
- Cookie session → OkHttp `CookieJar` persisted in **EncryptedSharedPreferences**
  (Android Keystore-backed). Never log the token; clear on logout/401.
- Handle 423 → Unlock screen (TOTP/backup code); 401 → Login; 403
  `ACCOUNT_PENDING` → pending-approval screen.

## 4. Domain structures (the "product/cart/order/payment" equivalents)

### Rides (`routes/rides/`, `frontend/src/types/domain.ts`, `components/RideCard`)
`RideRow` (~60 fields): id, passenger/driver ids, event/trip links, `pickup_at`
(KL text), `pickup_area_id` + masked `pickup_address` (privacy: full address only
to passenger, assigned driver after approval, staff), destination, seats,
assistance (wheelchair/low-step/child-seat), notes, status, attention flags,
vehicle info (model/color/plate/type/accessibility), `distance_km`,
`reimbursement_amount_cents`, readiness/delay/gate-arrival/supper-choice fields,
proxy-rider fields (`is_proxy`, rider name/phone/relationship), `leg`
(outbound/return), `paired_ride_id`, group matching fields.

State machine (from SQL guards):
`pending_driver → pending_passenger → pending_approval → pending_driver_confirm
→ confirmed → in_progress → completed`, with exits `cancelled`, `no_show`,
`alt_transport`. **Rides exist only for events**: `POST /rides` requires
`event_id` and a `going` signup on that event (the web opens `/rides/new?event=`
from an event page; without it, members are pointed at the events list).
Passenger actions: create, cancel, respond to offer, readiness,
supper choice. Driver: claim, withdraw, confirm, start, pickup, no-show,
complete, delay, GPS. Staff: approve, return, reassign, alt-transport.
Trip group chat: messages, emoji reactions, read receipts. Live location:
`GET /rides/{id}/location`, passenger/driver location POST/DELETE.

### Events (`routes/events/`)
Listing/detail with recursive group audiences, RSVP + FIFO waitlist, check-in
by code / QR / member-pass scan, ICS calendar feed, visitor passes
(`/visit/{token}`), post-event driver reimbursements.

### Settlements & shares — the "payment" domain (`routes/settlements/`)
Settlement: event-linked bill, `split_mode`, `subsidy_cents`, payee name/account,
**`payee_qr_file_id` (DuitNow QR image served by `/api/files/{id}`)**, due date,
publish/close lifecycle, revision history. Share (per member): `amount_cents`,
status, `txn_ref`, **`proof_file_id`** (member uploads transfer screenshot →
staff `POST /shares/{id}/verify`). `GET /me/shares` = the member's "cart of
amounts owed". Share statuses: `unpaid → pending (proof submitted) → confirmed`.
A withdrawn event signup remains in the payload with `status="cancelled"` and
must be treated as "no signup" by clients. **No card data, no gateway, no Play Billing.**

### Reimbursements (`routes/community/reimbursements.rs`)
Receipt-backed claims: `amount_cents`, `receipt_file_id`, payee name/phone,
review → pay (manual bank transfer, `paid_ref`), monthly payout batches + CSV.

### Files (`routes/files.rs`)
`POST /api/files` multipart upload → `{id, mime, name}`; `GET /api/files/{id}`
(cookie-authenticated download); `extract-ref` for receipt OCR-ish reference pull.

### Notifications / announcements / help / reports
In-app inbox with unread badge (`me.unread`), announcements CRUD, help tickets
with replies/claim/close, monthly report + badges, family/proxy-rider management.

## 5. Features requiring new Android APIs

| Web feature | Android API | Notes |
|---|---|---|
| WebAuthn passkeys | **Credential Manager** (`androidx.credentials:credentials` + `credentials-play-services-auth`) | Needs `assetlinks.json` on prod host (approval-gated). minSdk 28 for passkeys. |
| TOTP unlock / re-auth | plain UI + server verify; optional **BiometricPrompt** to gate the local app lock | Server remains the source of truth for lock. |
| Session cookie persistence | **EncryptedSharedPreferences** (security-crypto, Keystore AES256-GCM) + OkHttp CookieJar | Token is a credential; never plaintext, never logged. |
| SSE live updates | OkHttp SSE (`okhttp-sse`) or polling fallback | Same `/api/stream`; reconnect on resume. |
| Web push (VAPID) | **FCM** would need a backend change (approval-gated). Interim: SSE + local notifications while app is foreground/service. | Do not modify backend without approval. |
| Leaflet/OSM maps | **osmdroid** (OSM tiles, matches web's OSM/OSRM policy) | MapLibre is an alternative; osmdroid keeps the same tile source & attribution. |
| QR check-in scanner | **CameraX + ML Kit barcode scanning** | Web uses camera via browser; native needs CAMERA permission. |
| Driver/passenger GPS | **FusedLocationProviderClient** | ACCESS_FINE_LOCATION; foreground service for active-trip sharing; respect the web's privacy projection. |
| QR display (DuitNow) | Coil image load of `/api/files/{id}` with cookie auth | Optionally generate shareable QR locally — not needed, payee QR is uploaded. |
| PWA install/offline snapshots | **DataStore/Room cache** + WorkManager refresh | Offline: read-only cached lists with stale banner. |
| i18n zh/en | Android resources (`values-zh`) + `Accept-Language` header | Mirror `frontend/src/locales` keys for API error codes. |
| Auto-lock idle | ProcessLifecycleOwner + server 423 handling | Server enforces; client mirrors UX. |
| Text scale / elder-friendly | `sp` units, fontScale respect, 48dp min targets | Web root 17px, adjustable scale. |

## 6. Design tokens (web → Material 3)

From `frontend/src/styles.css` (minimalist: one background, one accent, borders not shadows):

| Token | Light | Dark | M3 role |
|---|---|---|---|
| primary | `#1F5DB4` | `#7AA7EE` | primary |
| primary-ink | `#FFFFFF` | `#0B1730` | onPrimary |
| primary-soft | `#EEF3FB` | (dark soft) | primaryContainer |
| bg/surface | `#FFFFFF` | `#141517` | background/surface |
| surface-2 | `#F4F4F3` | `#1F2124` | surfaceVariant |
| border | `#E6E6E6` | `#2E3135` | outlineVariant |
| text | `#1A1A1A` | `#EDEDED` | onSurface |
| muted | `#6B6B6B` | `#A3A3A3` | onSurfaceVariant |
| success | `#2B7549` | — | custom status |
| warning | `#8A5800` | — | custom status |
| danger | `#B4372F` | — | error |
| brand-red | `#E8462F` | — | logo only |

Spacing 4/8/16/24/32/48/64 · radius 8 (pill for chips) · min tap 48dp ·
type scale 14/17/19/20/24/30 · motion 150ms ease-out · no page elevation.

## 7. Verification environment found on this machine

- JDK 17 (Temurin 17.0.17) ✓ · Android SDK: platforms 36 & 37.0, build-tools 35/36 ✓
- Gradle 8.13 distribution cached ✓ · emulator + android-37.0 system image ✓
- No physical device attached (`adb devices` empty) → device testing outstanding.
- Backend not running locally yet; dev stack via `make db-up` + `make dev-api`
  (DEV_MODE seeds demo accounts, `dev-login` available for emulator tests).

## 8. Risks / approval-gated items

1. `assetlinks.json` on production host — required for passkeys in the release
   app; needs approval + release SHA-256. **Not done.**
2. FCM push — needs backend work. **Not done** (SSE interim).
3. Release signing keystore — must be generated/held by the owner; never committed.
4. Play Store listing — church-internal app; consider internal distribution
   (Play internal testing track) given invite-only membership.
5. No backend/DB changes were made; the React app is untouched.

---

## Appendix A — full API inventory (184 routes)


### admin (20 routes)

| Path | Methods |
|---|---|
| `/admin/dashboard` | GET |
| `/admin/users` | GET |
| `/admin/users/{id}` | PUT |
| `/admin/invites` | GET, POST |
| `/admin/invites/{id}` | DELETE |
| `/admin/groups` | POST |
| `/admin/groups/{id}` | DELETE, PUT |
| `/admin/groups/{id}/members` | GET, PUT |
| `/admin/areas` | POST |
| `/admin/areas/{id}` | PUT |
| `/admin/recovery` | GET |
| `/admin/recovery/{id}/approve` | POST |
| `/admin/recovery/{id}/reject` | POST |
| `/admin/audit` | GET |
| `/admin/audit/export` | GET |
| `/admin/settings/fares` | GET, PUT |
| `/admin/settings/fares/{id}` | DELETE |
| `/admin/settings/system` | GET, PUT |
| `/admin/reports/demand` | GET |
| `/admin/settings/alerts` | GET, PUT |

### area_photos (3 routes)

| Path | Methods |
|---|---|
| `/areas/{id}/photos` | GET, POST |
| `/areas/{id}/photos/order` | PUT |
| `/areas/{id}/photos/{pid}` | DELETE, PUT |

### auth (25 routes)

| Path | Methods |
|---|---|
| `/auth/me` | GET |
| `/auth/login` | POST |
| `/auth/dev-users` | GET |
| `/auth/dev-login` | POST |
| `/auth/logout` | POST |
| `/auth/logout-all` | POST |
| `/auth/lock` | POST |
| `/auth/unlock` | POST |
| `/auth/terms` | POST |
| `/auth/passkey/login/start` | POST |
| `/auth/passkey/login/finish` | POST |
| `/auth/passkey/reauth/start` | POST |
| `/auth/passkey/reauth/finish` | POST |
| `/auth/passkey/register/start` | POST |
| `/auth/passkey/register/finish` | POST |
| `/auth/passkeys` | GET |
| `/auth/passkeys/{id}` | DELETE |
| `/auth/totp/setup` | POST |
| `/auth/totp/enable` | POST |
| `/auth/totp/verify` | POST |
| `/auth/totp/backup-codes/regenerate` | POST |
| `/auth/password` | POST |
| `/auth/recovery-request` | POST |
| `/invites/{token}` | GET |
| `/invites/{token}/accept` | POST |

### care (3 routes)

| Path | Methods |
|---|---|
| `/rides/{id}/care-link` | DELETE, POST |
| `/care/{token}` | GET |
| `/care/{token}/readiness` | POST |

### community (41 routes)

| Path | Methods |
|---|---|
| `/me/family` | GET |
| `/me/family/leave` | POST |
| `/me/family/members/{user_id}` | DELETE |
| `/me/family/people` | POST |
| `/me/family/requests/{id}/accept` | POST |
| `/me/family/requests/{id}/decline` | POST |
| `/me/family/requests/{id}` | DELETE |
| `/me/family/invites/{id}` | DELETE |
| `/me/calendar-feed` | DELETE, GET |
| `/me/calendar-feed/rotate` | POST |
| `/calendar/{token}` | GET |
| `/push/config` | GET |
| `/push/subscribe` | DELETE, POST |
| `/push/test` | POST |
| `/notifications` | GET |
| `/notifications/read-all` | POST |
| `/notifications/{id}/read` | POST |
| `/announcements` | GET, POST |
| `/announcements/{id}` | DELETE, PUT |
| `/help` | GET, POST |
| `/help/{id}` | GET |
| `/help/{id}/replies` | POST |
| `/help/{id}/close` | POST |
| `/help/{id}/reopen` | POST |
| `/help/{id}/claim` | POST |
| `/help/counts` | GET |
| `/help/members` | GET |
| `/help/guides/reorder` | POST |
| `/help/guides/{id}/feedback` | POST |
| `/help/guides` | GET, POST |
| `/help/guides/{key}` | GET |
| `/reimbursements/batches` | GET, POST |
| `/reimbursements/batches/{id}` | DELETE |
| `/reimbursements/batches/{id}/csv` | GET |
| `/reimbursements/batches/{id}/pay` | POST |
| `/reimbursements` | GET, POST |
| `/reimbursements/{id}` | GET, PUT |
| `/reimbursements/{id}/review` | POST |
| `/reimbursements/{id}/pay` | POST |
| `/reports/monthly` | GET |
| `/badges` | GET |

### events (16 routes)

| Path | Methods |
|---|---|
| `/groups` | GET |
| `/events` | GET, POST |
| `/events/{id}` | GET, PUT |
| `/events/{id}/duplicate` | POST |
| `/events/{id}/cancel` | POST |
| `/events/{id}/finish` | POST |
| `/events/{id}/signup` | POST |
| `/events/{id}/withdraw` | POST |
| `/events/{id}/signups` | GET |
| `/events/{id}/attendance` | POST |
| `/events/{id}/checkin` | GET, POST |
| `/events/checkin` | POST |
| `/events/{id}/checkin/scan-member` | POST |
| `/events/{id}/checkin-code/reset` | POST |
| `/events/{id}/post-event-drivers` | GET |
| `/events/{id}/post-event-reimbursements` | POST |

### files (3 routes)

| Path | Methods |
|---|---|
| `/files` | POST |
| `/files/{id}` | GET |
| `/files/{id}/extract-ref` | POST |

### plan (6 routes)

| Path | Methods |
|---|---|
| `/admin/events/{id}/plan` | GET, POST |
| `/admin/events/{id}/plan/draft` | POST |
| `/admin/events/{id}/plan/request-approval` | POST |
| `/admin/events/{id}/plan/return` | POST |
| `/admin/events/{id}/plan/approve` | POST |
| `/admin/events/{id}/plan/arrange` | POST |

### plan_visitors (4 routes)

| Path | Methods |
|---|---|
| `/admin/events/{id}/plan/visitors` | GET, POST |
| `/admin/events/{id}/plan/visitors/{vid}` | DELETE, PUT |
| `/admin/events/{id}/plan/visitors/{vid}/reimbursement` | POST |
| `/admin/events/{id}/plan/visitors/{vid}/reimbursement/{rid}` | PUT |

### rides (44 routes)

| Path | Methods |
|---|---|
| `/areas` | GET |
| `/places` | GET |
| `/proxy-riders` | GET, POST |
| `/proxy-riders/{id}` | DELETE, PUT |
| `/saved-places` | GET, POST |
| `/saved-places/{id}` | DELETE, PUT |
| `/recurring-schedules` | GET, POST |
| `/recurring-schedules/{id}` | DELETE, PUT |
| `/recurring-schedules/{id}/generate` | POST |
| `/rides` | GET, POST |
| `/rides/{id}` | GET |
| `/rides/{id}/cancel` | POST |
| `/rides/{id}/respond` | POST |
| `/rides/{id}/readiness` | POST |
| `/rides/{id}/supper-choice` | POST |
| `/rides/{id}/driver-delay` | POST |
| `/rides/{id}/arrived-gate` | POST |
| `/rides/{id}/claim` | POST |
| `/rides/{id}/withdraw` | POST |
| `/rides/{id}/confirm-assignment` | POST |
| `/rides/{id}/start` | POST |
| `/rides/{id}/pickup` | POST |
| `/rides/{id}/no-show` | POST |
| `/rides/{id}/complete` | POST |
| `/rides/{id}/approve` | POST |
| `/rides/{id}/return` | POST |
| `/rides/{id}/reassign` | POST |
| `/rides/{id}/alt-transport` | POST |
| `/rides/{id}/messages` | GET, POST |
| `/rides/{id}/messages/read` | POST |
| `/rides/{id}/messages/{mid}/reactions` | POST |
| `/rides/{id}/location` | GET |
| `/rides/{id}/passenger-location` | DELETE, POST |
| `/driver/requests` | GET |
| `/driver/schedule` | GET |
| `/driver/route` | GET |
| `/driver/profile` | GET, PUT |
| `/driver/vehicles` | GET, POST |
| `/driver/vehicles/{id}` | PUT |
| `/driver/vehicles/{id}/select` | POST |
| `/driver/location` | DELETE, POST |
| `/admin/rides` | GET |
| `/admin/live` | GET |
| `/admin/drivers` | GET |

### roles (2 routes)

| Path | Methods |
|---|---|
| `/admin/roles` | GET, POST |
| `/admin/roles/{key}` | DELETE, PUT |

### settlements (9 routes)

| Path | Methods |
|---|---|
| `/events/{id}/settlements` | POST |
| `/settlements` | GET |
| `/settlements/{id}` | GET |
| `/settlements/{id}/publish` | POST |
| `/settlements/{id}/close` | POST |
| `/settlements/{id}/revisions` | GET |
| `/me/shares` | GET |
| `/shares/{id}/proof` | POST |
| `/shares/{id}/verify` | POST |

### visitors (8 routes)

| Path | Methods |
|---|---|
| `/events/{id}/visitors` | GET, POST |
| `/visitor-passes/{id}/reissue` | POST |
| `/visitor-passes/{id}/revoke` | POST |
| `/visit/me` | GET |
| `/visit/me/readiness` | POST |
| `/visit/me/location` | GET |
| `/visit/{token}` | GET |
| `/visit/{token}/claim` | POST |
