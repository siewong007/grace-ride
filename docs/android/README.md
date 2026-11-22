# Grace Ride — Native Android App

Kotlin · Jetpack Compose · Material 3 · MVVM. Reuses the existing Rust/Axum
backend and REST API unchanged; the React website is untouched.

Package: `org.pjstatecm.graceride` · minSdk 28 · target/compileSdk 36 ·
AGP 8.13.0 · Gradle 8.13 · Kotlin 2.2.20 · Compose BOM 2025.10.00.

## What is implemented

| Area | Screens |
|---|---|
| Session | Login (passkey via Credential Manager + dev accounts on debug), Unlock (TOTP/backup code, server 423 idle-lock), Pending approval |
| Home | Next ride, unpaid shares, driving today, upcoming events, announcements; pull-to-refresh; per-section error degradation |
| Rides | Upcoming/Past lists, ride detail (privacy-projected fields, readiness, accept/decline, cancel with optimistic-concurrency retry), booking form opened from an event (place search + saved places, seats, assistance, round trip) |
| Events | Upcoming/Past lists, detail with RSVP (need ride / offer drive / join-as group), withdraw, check-in code display, "Request a ride" CTA |
| Costs | My shares (integer cents → RM), DuitNow payee QR display, proof photo upload (multipart) + transaction reference submission |
| History | Past rides |
| Profile | Roles, passkey list/add/delete, auto-lock setting, language (zh/en, per-app locale + Accept-Language), sign out / sign out all |
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

## Verification status (2026-11-21)

Verified on the API-37 emulator against the live dev backend:
dev sign-in, encrypted session persistence across restarts, server idle-lock →
Unlock → restore, home with live data, ride list/detail, readiness mutation,
cancel mutation, event RSVP + withdraw + re-signup (server-verified each time),
ride booking from an event (server-verified), cost-share proof upload
(multipart, server-verified status/proof/txn_ref), history, notifications.
44 JVM unit tests + 6 Compose UI tests green; release AAB signature verified.

NOT yet tested / outstanding:
- Physical device (none attached).
- Real passkey sign-in/registration: needs `/.well-known/assetlinks.json` on the
  production host with the release package + SHA-256 (approval-gated production
  change). Credential Manager code paths are compile-verified only.
- Push notifications: web uses VAPID; FCM needs backend work (approval-gated).
  Interim: in-app inbox + SSE-ready architecture (SSE client not yet wired).
- Driver workspace, admin tools, help desk, reimbursements, monthly report,
  family/proxy riders, live maps (osmdroid), QR check-in scanner (CameraX/ML Kit)
  — planned future increments.
- Tablet/rail adaptive layouts; dark mode verified by token design only.
- Play Store listing/upload (needs the owner's Play account).

See `docs/android/DESIGN.md` for design system token mappings and UI guidelines.
