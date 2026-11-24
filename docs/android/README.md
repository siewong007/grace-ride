# Grace Ride — Native Android App

Kotlin · Jetpack Compose · Material 3 · MVVM. Reuses the existing Rust/Axum
backend and REST API unchanged; the React website is untouched.

Package: `org.pjstatecm.graceride` · minSdk 28 · target/compileSdk 36 ·
AGP 8.13.0 · Gradle 8.13 · Kotlin 2.2.20 · Compose BOM 2025.10.00 · Coil 2.7.0 (`coil-gif`).

## What is implemented

| Area | Screens |
|---|---|
| Session | Login (passkey via Credential Manager + dev accounts on debug), Terms (`TermsScreen`, PDPA v2 consent), Unlock (TOTP/backup code, server 423 idle-lock), Pending approval |
| Home | Next ride, unpaid shares, driving today + 7-day next-drive preview, offline ride snapshot fallback, upcoming events, announcements; pull-to-refresh; per-section error degradation |
| Rides | Upcoming/Past lists, ride detail (privacy-projected fields, driver avatar, proxy details, readiness, accept/decline, cancel with optimistic-concurrency retry, native driver trip execution controls: `confirm-assignment`, `start`, `arrived-gate`, `pickup`, `complete`, `driver-delay`, `withdraw` + Web App handoff links), booking form opened from an event (signup gate, trip direction outbound/round/return, family/proxy riders + saved riders, `"Use my current location"` GPS capture, place search + saved places with area inference, lifecycle-refreshed navigation to Saved Addresses & Proxy Riders, seats, assistance) |
| Events | Upcoming/Past/Managing lists (driver open-requests badge, draft filtering via `hide_drafts`, `publish`/`unpublish`), detail with `EventScheduleTimeline`, RSVP (need ride / offer drive / join-as group), withdraw, check-in code display, existing ride status card, `"Request a ride"` & `"Book ride for family / proxy"` CTAs, animated GIF event icons (`coil-gif`) |
| Costs | My shares (integer cents → RM), DuitNow payee QR display, proof photo upload (multipart) + transaction reference submission |
| History | Past rides |
| Profile | Roles, editable personal details (name, phone, Telegram chat ID), `DriverSettingsScreen` (carpool toggle, spare-seats clamping, multi-vehicle CRUD & selection, service areas), `ProxyRidersScreen` (frequent proxy riders CRUD), `SavedAddressesScreen` (saved places CRUD with GPS capture & area inference), `GrokBotStudioScreen` (interactive clay bot designer: 8 shapes × 16 faces × 12 colours + animated moves, Save as Avatar), avatar editor (photo/animated GIF, native animated mascot shapes in `Avatar.kt`, bundled Bloub GIF assets in `assets/icons/`, Grok/Muse animated bot avatars, static icons, initials, colours), 4-tier badge medallions (`BadgeIcon.kt`: `bronze` → `platinum`), passkey list/add/delete, auto-lock setting, language (zh/en, per-app locale + Accept-Language), sign out / sign out all |
| Notifications | Inbox, mark all read, deep links to ride/event/costs |

Architecture: `data/api` (Retrofit + kotlinx.serialization DTOs, error mapping,
auth event bus) · `data/session` (EncryptedSharedPreferences-backed CookieJar) ·
`data/repo` (repositories) · `di/AppContainer` (manual DI) · `domain`
(money/time/status helpers) · `ui` (M3 theme from web tokens, shared state views,
screens + ViewModels with StateFlow).

## Build & run

```bash
cd android
echo "sdk.dir=$HOME/Library/Android/sdk" > local.properties   # if missing
./gradlew :app:assembleDebug            # debug APK (API base: http://10.0.2.2:8080/)
./gradlew :app:testDebugUnitTest        # JVM unit tests (MockWebServer)
./gradlew :app:connectedDebugAndroidTest # Compose UI tests (emulator/device needed)
./gradlew :app:bundleRelease            # signed AAB if keystore.properties exists
```

Dev backend (from the repo root): `make db-up`, then run the API with
`DEV_MODE=true` (see `backend/.env.example`). The debug login screen lists the
seeded demo accounts and lets you point at a LAN server. Release builds talk to
`https://graceride.saliminn.my/` (override: `-Pgraceride.apiBaseUrl=…`).

## Signing

`keystore.properties` (gitignored) points at a keystore OUTSIDE the repo.
A local upload key was generated at `~/.graceride/upload-keystore.jks` for build
verification only — replace it with the ministry's own upload key before any
Play Store upload, and enroll in Play App Signing. Never commit keys.

## Verification status & test suites

Historical emulator verification (2026-11-21 on the API-37 emulator against the live dev backend) exercised:
dev sign-in, encrypted session persistence across restarts, server idle-lock →
Unlock → restore, home with live data, ride list/detail, readiness mutation,
cancel mutation, event RSVP + withdraw + re-signup (server-verified each time),
ride booking from an event (server-verified), cost-share proof upload
(multipart, server-verified status/proof/txn_ref), history, and notifications,
plus release AAB signature verification.

Current automated test suites in `android/app/`:
- **74 JVM unit tests** across 6 test files (`ApiSessionTest`, `BugfixRegressionTest`, `DomainTest`, `PersistentCookieJarTest`, `RegressionTest`, `ViewModelTest`) using MockWebServer.
- **7 Compose UI tests** (`ComposeUiTest.kt`).

NOT yet tested / outstanding:
- Physical device (none attached) and post-2026-11-21 live emulator re-verification of newer screens (`DriverSettingsScreen`, `ProxyRidersScreen`, `SavedAddressesScreen`, `GrokBotStudioScreen`, `TermsScreen`, native driver trip execution).
- Real passkey sign-in/registration: needs `/.well-known/assetlinks.json` on the
  production host with the release package + SHA-256 (approval-gated production
  change). Credential Manager code paths are compile-verified only.
- Push notifications: web uses VAPID; FCM needs backend work (approval-gated).
  Interim: in-app inbox + SSE-ready architecture (SSE client not yet wired).
- Full admin & transport planning board, help desk, reimbursements, monthly report,
  family list management, trip chat, live maps (`osmdroid`), and QR check-in scanner
  (`CameraX`/`ML Kit`) — extended workflows currently hand off to the Web App via
  browser links.
- Tablet/rail adaptive layouts; dark mode verified by token design only.
- Play Store listing/upload (needs the owner's Play account).

See `docs/android/DESIGN.md` for design system token mappings and UI guidelines.
