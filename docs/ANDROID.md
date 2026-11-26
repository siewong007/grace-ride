# Grace Ride — Native Android App

Kotlin · Jetpack Compose · Material 3 · MVVM. Reuses the existing Rust/Axum backend and REST API unchanged; the React web app remains untouched.

- **Package:** `org.pjstatecm.graceride`
- **SDK:** `minSdk 28` · `targetSdk 36` · `compileSdk 36` · Java `17`
- **Toolchain & Core Libraries:** AGP `8.13.0` · Gradle `8.13` · Kotlin `2.2.20` · Compose BOM `2025.10.00` · Navigation Compose `2.9.5` · `material3-adaptive-navigation-suite` · Retrofit `2.11.0` · OkHttp `4.12.0` (`okhttp-sse`) · `kotlinx.serialization` `1.9.0` · Coil `2.7.0` (`coil-compose`, `coil-gif`) · AndroidX Biometric `1.1.0` · Credential Manager `1.5.0` · WorkManager `2.10.1` · Glance AppWidget `1.1.1` (`glance-material3`) · ZXing Core `3.5.4` · Play Services Code Scanner `16.1.0` · Core Splashscreen `1.0.1` · AndroidX Browser Custom Tabs `1.9.0`

---

## 1. Implemented Screens & Native Capabilities

| Area | Screens & Native Capabilities |
|---|---|
| **Session & Auth** | `LoginScreen` (direct hardware-backed biometric sign-in via `BiometricPrompt` + `AndroidKeyStore` ECDSA P-256 challenge-response + WebAuthn passkeys via Android Credential Manager + dev account switcher & LAN server override in debug builds), `BiometricGateScreen` (cold-start & 2-minute idle-resume biometric app lock), post-sign-in biometric opt-in prompt, `TermsScreen` (PDPA v3 consent gate), `UnlockScreen` (biometric / passkey / TOTP / backup code; server `423 SESSION_LOCKED` persisted across offline cold starts), `PendingScreen` |
| **Chrome, Links & Widget** | `GraceRideAppRoot` (`NavigationSuiteScaffold` switching automatically between a bottom `NavigationBar` — Home · Events · Profile — on compact phones and a `NavigationRail` on foldables/tablets), type-safe `@Serializable` `AppRoute` destinations (`Routes.kt`), verified App Links (`https://graceride.saliminn.my/...`), internal shortcut deep links (`graceride://open/...`), static OS App Shortcuts (`shortcuts.xml`), `GraceRideBrandMark` + SplashScreen, and resizable Material 3 Glance home-screen widget (`GraceRideWidget` + `WidgetSnapshotStore`) with a 1-tap *"I'm Ready"* broadcast action (`ReadinessActionReceiver`) |
| **Home** | `HomeScreen`: brand mark + wordmark header with notifications bell & avatar, next ride, unpaid cost shares, driving today + 7-day next-drive preview, offline ride snapshot fallback, upcoming events, announcements, pull-to-refresh, per-section error degradation |
| **Rides, Live Tracking & Chat** | `RidesScreen` (upcoming/past with directional transitions), `RideNewScreen` (event-gated, outbound/round/return legs, family/proxy riders + saved riders, `"Use my current location"` GPS capture, place search & saved places with area auto-inference, lifecycle-refreshed navigation to Saved Addresses & Proxy Riders, seats, assistance), `RideDetailScreen` + `RideDetailControls` (privacy-projected fields, driver avatar, proxy details, readiness, accept/decline, cancel with optimistic-concurrency retry, native driver trip actions: `confirm-assignment`, 1-step GPS-prompted `start`, `arrived-gate`, `pickup`, `complete`, `driver-delay`, `withdraw` + Web App handoff links), `TripChatSheet` + `TripChatViewModel` (real-time SSE chat + `NotificationCompat.MessagingStyle` & `RemoteInput` inline direct reply via `ChatReplyReceiver`), `LiveTripLocationService` + `LiveTrackCard` + `GraceRideLiveMap` (`FOREGROUND_SERVICE_LOCATION` strictly scoped to active `in_progress` journeys with pre-GPS server liveness check, SSE + watchdog + session-lock/sign-out auto-stop, `< 80 m` automatic gate arrival on two accurate fixes for exact pins, destination proximity alert, role-based live map: driver vs waiting rider ETA, auto-off rider location sharing upon pickup, and Waze/Google Maps `rememberSmartNavigator` chooser) |
| **Driver Hub** | `DriveHubScreen` + `DriveHubViewModel`: native 3-tab hub with directional sliding transitions (Open Requests + 1-tap Claim, 14-Day Schedule, Live Route with `GraceRideLiveMap` route polyline & numbered stops, onboard counts, ETA, live GPS sharing toggle, destination proximity banner, and Waze/Google Maps turn-by-turn launch via `DeviceActions.kt`) |
| **Events & Native QR Check-In** | `EventsScreen` (Upcoming/Past/Managing lists with sliding tab transitions, notifications bell button, driver open-requests badge, draft filtering via `hide_drafts`, `publish`/`unpublish`), `EventDetailScreen` + `EventTimelineSection` (`EventScheduleTimeline`, RSVP for need ride / offer drive / join-as group, withdraw, check-in code display, existing ride status card, `"Request a ride"` & `"Book ride for family / proxy"` CTAs, animated event icons), `EventCheckInSheet` + `EventPassQrSheet` + `QrCodeHelper` (pure-Kotlin ZXing vector QR rendering `GRPASS:<eventId>:<passCode>` for member passes and Play Services Code Scanner with manual code fallback for self check-in and organiser member-pass scanning), zero-permission calendar export (`CalendarActions.addEventToCalendar`) |
| **Costs & History** | `CostsScreen` (integer cents → RM, DuitNow payee QR display, multipart proof photo upload via `ImageUploadPrep` + transaction reference submission), `HistoryScreen` (past rides) |
| **Profile, Studio & Settings** | `ProfileScreen` + `ProfileSections` + `ProfileSubScreens` (`PersonalDetailsScreen`, `AppearanceSettingsScreen`, `NotificationSettingsScreen`, `SecuritySettingsScreen`), `DriverSettingsScreen` (carpool toggle, `spare_seats` clamping, multi-vehicle CRUD & selection, service areas, Navigation app preference `Ask`/`Waze`/`Maps`, and `Auto-mark arrived at gate` toggle), `ProxyRidersScreen` (frequent proxy riders CRUD), `SavedAddressesScreen` (saved places CRUD with GPS capture & area inference), `GrokBotStudioScreen` (`GrokBotSpec`, `GrokBotCanvas`, `GrokBotExporter`, `GrokBotStudio` — 8 shapes × 16 faces × 12 clay colours + animated moves, Save as Avatar, SVG/PNG export), `AvatarEditorSheet` (photo/GIF upload, native Compose Canvas mascots in `MascotCanvasAvatars.kt`, 77 lossless WebP icon assets in `assets/icons/{avatars,badges,events}/`), `BadgeIcon` 4-tier achievement medallions (`bronze` → `platinum`), `ThemeMode` switcher (System/Light/Dark persisted in `SecureStore` synced with edge-to-edge `SystemBarStyle`), biometric sign-in & app-lock toggle + enrolled device management, passkey list/add/delete, auto-lock setting, per-app language toggle (`locales_config.xml`, `values/` + `values-zh/`, synced with `PUT /api/me` and `Accept-Language`), permission-gated Staff Reports entry (`admin_reports`), sign out |
| **Notifications & Real-Time** | `NotificationsScreen` (inbox, mark all read, deep links via `NotifLinkRouter`), `RealtimeStreamClient` (OkHttp SSE `/api/stream` with typed `RealtimeEvent`: `Chat`, `Notify`, `Resume`, `Other`, foreground-aware reconnect), `BackgroundNotificationWorker` (WorkManager periodic sync sharing per-user watermark in `SecureStore`), `NotificationHelper` (4 localized channels: `rides_urgent`, `rides_updates`, `chat_messages`, `community`, lock-screen privacy redaction `hideLockDetails`, inline direct reply) |
| **Web App Handoff** | `WebHandoff.kt`: AndroidX Custom Tabs with Grace Ride light/dark chrome pinned to an external browser package (preventing verified App Links loops) for web-only workflows (reimbursements, family management, help desk, monthly reports, admin & transport planning board) |

---

## 2. Architecture, Security & Offline Resilience

- **Packages (`android/app/src/main/java/org/pjstatecm/graceride/`):**
  - `data/api/` — Retrofit `ApiService`, `ApiProvider` (thread-safe hot-swap of Retrofit endpoints without recreating repositories), `kotlinx.serialization` `Dtos`, `ApiClient` error mapping, `AuthEventBus`
  - `data/cache/` — `OfflineCache` (per-user atomic JSON stale-while-revalidate store for Home, Rides, Events, Proxy Riders, and Saved Addresses)
  - `data/media/` — `ImageUploadPrep` (resize/strip prep for proof and avatar uploads)
  - `data/repo/` — `AuthRepository`, `CommunityRepository`, `EventsRepository`, `RidesRepository`, `SharesRepository`
  - `data/session/` — `SecureStore` (`EncryptedPrefsStore` using hardware/OS-backed `AndroidKeyStore` AES-256-GCM + `AesGcmCodec` with AAD key-binding, replacing deprecated `androidx.security:security-crypto`), lazy-loaded `PersistentCookieJar`, `OfflineSnapshot`
  - `di/` — `AppContainer` manual dependency injection & `Accept-Language` caching
  - `domain/` — `AreaMatch`, `Format` (`KlTime`, integer-cent RM formatting), `Readiness`, `RideStatus`
  - `notifications/` — `RealtimeStreamClient`, `BackgroundNotificationWorker`, `NotificationHelper`, `ChatReplyReceiver`, `ReadinessActionReceiver`
  - `platform/` — `BiometricAuthManager`, `CalendarActions`, `NetworkMonitor`, `QrCodeHelper`
  - `service/` — `LiveTripLocationService`
  - `ui/` — `GraceRideAppRoot`, `Routes` (`AppRoute`), `NotifLinkRouter`, `WebHandoff`, `DeviceActions`, `theme/`, `components/`, `screens/`
  - `widget/` — `GraceRideWidget` (Glance M3 widget + `WidgetSnapshotStore`)
- **Security Hygiene:** `android:allowBackup="false"`, `android:fullBackupContent="false"`, `data_extraction_rules.xml`, `network_security_config.xml` (cleartext restricted to debug/localhost), `StrictMode` thread network logging in debug builds, `BIOMETRIC_STRONG` `AndroidKeyStore` EC `secp256r1` keys (`setUserAuthenticationRequired(true)`, `setInvalidatedByBiometricEnrollment(true)`) that automatically wipe enrollment if device biometrics change, and immediate session/cache clearing on logout or `401` (preserving only the public-key enrollment reference for 1-tap biometric re-login).
- **Global Session States:** HTTP `401` → `Login`, `423 SESSION_LOCKED` → `Unlock` (biometric / passkey / TOTP / backup code), `403 ACCOUNT_PENDING` → `Pending`, and active-session biometric app gate on cold start or >= 2-minute background idle when biometric login is enabled. Ride mutations that lose optimistic concurrency (`RIDE_STATUS_CHANGED`) re-fetch and display a snackbar instead of failing hard.

---

## 3. Design System

Native Material 3 translation of the web design tokens (`frontend/src/styles.css`). The web app is minimalist—one background, one accent, borders instead of shadows—and Android preserves that character with native components and motion.

### Colour

| Role | Light | Dark | Source Token |
|---|---|---|---|
| `primary` | `#1F5DB4` | `#7AA7EE` | `--primary` (logo Ride blue) |
| `onPrimary` | `#FFFFFF` | `#0B1730` | `--primary-ink` |
| `primaryContainer` | `#EEF3FB` | `#1A2A44` | `--primary-soft` |
| `background` / `surface` | `#FFFFFF` | `#141517` | `--bg` |
| `surfaceContainerLowest..Highest` | `#FFFFFF`..`#E8E8E6` | `#0E0F11`..`#30343A` | Neutral M3 surface hierarchy |
| `surfaceVariant` | `#F4F4F3` | `#1F2124` | `--surface-2` |
| `onSurface` | `#1A1A1A` | `#EDEDED` | `--text` |
| `onSurfaceVariant` | `#6B6B6B` | `#A3A3A3` | `--muted` (5.3:1 on white) |
| `outlineVariant` | `#E6E6E6` | `#2E3135` | `--border` |
| `error` | `#B4372F` | `#F0938B` | `--danger` |

- Users can switch appearance between **System**, **Light**, and **Dark** in Profile (`ThemeMode`), persisted in `SecureStore` and synced with edge-to-edge `SystemBarStyle` contrast.
- Status colours (text + soft fills only, never large fills): success `#2B7549`/`#EEF6F1`, warning `#8A5800`/`#FBF4E6`, info `#2C5D9C`/`#EEF3FB`, danger `#B4372F`/`#FBEEEC` (dark variants lighten text and darken fill), exposed via `LocalStatusColors`. Brand red `#E8462F` stays in the logo only.
- Cards use `OutlinedCard` (`1dp` border, zero elevation), matching the web's bordered surfaces.

### Typography, Spacing, Shape & Touch

- **Typography (`ui/theme/Type.kt`):** Mirrors the web root `17px` and scale `14 / 17 / 19 / 20 / 24 / 30` in `sp` so system font scaling works for elders: `headlineMedium` `30sp` SemiBold, `headlineSmall` `24sp` SemiBold, `titleLarge` `20sp` SemiBold, `titleMedium` `17sp` Medium, `titleSmall` `14sp` Medium, `bodyLarge` `17sp`, `bodyMedium`/`bodySmall` `14sp`, `label*` `14sp` Medium (strict `14sp` floor). `TextStyle.tabular()` (`tnum`) aligns prices, timestamps, check-in passcodes, and seat steppers.
- **Spacing & Shape:** `Spacing` scale `4 / 8 / 16 / 24 / 32 / 48 / 64 dp`; native M3 shape progression (`8 / 10 / 12 / 16 / 24 dp`); pills/chips use `CircleShape`; minimum `48dp` interactive targets (`--tap: 48px`).
- **Motion, Haptics & Device Actions:** M3 defaults (~150–300 ms ease-out) paired with `UiHaptics` (`CLOCK_TICK` for tabs/filters/steppers, `CONFIRM` for primary trip/form submissions); 1-tap `dialPhone`, `openInMaps` (`geo:` / turn-by-turn `openInNavigation`), `copyToClipboard`, and `CalendarActions.addEventToCalendar`.
- **Chrome:** `enableEdgeToEdge()` in `MainActivity`; detail screens use `TopAppBar` with a back arrow while tab roots do not.

### Avatars, Iconography & Badges

- **Animated Mascots & WebP Assets (`Avatar.kt`, `MascotCanvasAvatars.kt`, `EventIcon.kt`):** Native Compose Canvas mascot renderers (`GrokBotShapeAvatar`, `MuseBotShapeAvatar`) pair with 77 lossless WebP icon assets (`assets/icons/{avatars,badges,events}/`) loaded via Coil.
- **Grok Bot Studio (`GrokBotSpec.kt`, `GrokBotExporter.kt`, `GrokBotCanvas.kt`, `GrokBotStudio.kt`):** Interactive clay-style bot renderer (`grokbot:<shape>:<face>:<color>`, 8 silhouettes × 16 faces × 12 colours with animated poses) tuned for light and dark themes, plus standalone SVG/PNG export.
- **4-Tier Achievement Medallions (`BadgeIcon.kt`):** Progressive badge rings and palettes (`none` → `bronze` → `silver` → `gold` → `platinum`, with subtle animated halo emphasis on `gold`/`platinum`).

### UI States, Accessibility & i18n

- **Loading:** Native `GraceRideBrandMark` (`BrandMarkMode.LOADING` ripple) via `GraceRideLoadingView` (`HomeSkeleton`, `DetailSkeleton`, `SkeletonLoadingList` delegate to it instead of gray shimmer placeholders).
- **Refreshing:** M3 `PullToRefreshBox` on lists and detail screens, plus automatic background refresh on typed `RealtimeEvent` SSE signals and `NetworkMonitor` connectivity restoration.
- **Empty & Error:** `EmptyView` and `ErrorView` (server errors shown verbatim in the member's language, never raw stack traces).
- **Offline:** Reactive `NetworkMonitor` status strip and per-user `OfflineCache` + `OfflineSnapshotStore`; section-level failures degrade gracefully.
- **Accessibility:** Content descriptions on interactive icon buttons, decorative icons `null`, status pills never colour-only, `14sp` floor + `sp` scaling + `48dp` targets, TalkBack roles/states, AA contrast in light and dark schemes.
- **i18n:** `values/strings.xml` (`en`) + `values-zh/strings.xml` (`zh`). Per-app language toggle in Profile writes `lang` via `PUT /api/me` and updates `Accept-Language` so API error messages match.

---

## 4. Build, Signing & Passkey Setup

```bash
cd android
echo "sdk.dir=$HOME/Library/Android/sdk" > local.properties   # if missing
./gradlew :app:assembleDebug             # debug APK (API base: http://10.0.2.2:8080/)
./gradlew :app:testDebugUnitTest         # 132 JVM unit tests across 15 files (MockWebServer)
./gradlew :app:lintDebug                 # Android Lint check
./gradlew :app:connectedDebugAndroidTest # 27 Compose UI tests across 3 files (emulator/device)
./gradlew :app:bundleRelease             # signed AAB if keystore.properties or GR_STORE_* env vars exist
```

From the repository root, `Makefile` also exposes `make lint-android` and `make test-android` (unit tests + compiling `debugAndroidTest` Kotlin).

- **Dev vs. Release API Target:** Debug builds point at `http://10.0.2.2:8080/` (`make db-up`, then run backend with `DEV_MODE=true`) and expose the dev quick-login + LAN server override. Release builds target `https://graceride.saliminn.my/` (override via `-Pgraceride.apiBaseUrl=…`).
- **Signing:** `keystore.properties` (gitignored) or `GR_STORE_FILE` / `GR_STORE_PASSWORD` / `GR_KEY_ALIAS` / `GR_KEY_PASSWORD` env vars configure release signing outside the repository. Replace any local verification upload key with the ministry's own upload key and enroll in Play App Signing before Play Store upload. Never commit keystores.
- **Passkeys & App Links (`ANDROID_CERT_SHA256_FINGERPRINTS`):** Set `ANDROID_CERT_SHA256_FINGERPRINTS` (comma-separated colon-hex SHA-256 signing fingerprints) on the Rust backend so it serves `/.well-known/assetlinks.json` and registers `android:apk-key-hash:` origins in WebAuthn (`backend/src/lib.rs`, `state.rs`).

---

## 5. Verification Status & Pre-Launch Scope

- **Automated Suites (verified in `.github/workflows/ci.yml` `android` job on JDK 17):**
  - **132 JVM unit tests across 15 files:** `ApiSessionTest` (10), `ArchitectureAndPlatformTest` (9), `AreaMatchAndRoutesTest` (2), `BugfixRegressionTest` (3), `KlTimeTest` (8), `LocationServicePolicyTest` (6), `MoneyTest` (6), `NotificationsTest` (12), `PerformanceRegressionTest` (8), `PersistentCookieJarTest` (8), `RegressionTest` (10), `RideStatusTest` (5), `SecurityAndOfflineTest` (9), `ThemeAndBadgeTest` (4), `ViewModelTest` (32).
  - **27 Compose UI instrumentation tests across 3 files:** `ComposeUiTest` (15), `RoleAndFeatureMatrixAndroidTest` (5), and `FullAppPagesAndScenariosAndroidTest` (7).
- **Emulator & Live Dev Backend Verification (2026-11-26, API 37 `Medium_Phone_API_37.0`):** Executed the full 27-test Compose instrumentation suite alongside a live Light/Dark dev-backend walkthrough covering Launch/Login (`dev-users`), Terms opt-in, Home quick actions (rider & driver), Events tabs & EventDetail, Drive Hub (Requests / Schedule / Live Route OpenStreetMap + pins), Vehicle & Driving settings (Waze/Google/Ask & auto-arrive), Rides, Ride History, Costs, Notifications, Appearance & Language, and clean non-duplicated Profile navigation.
- **Remaining Pre-Launch Items:**
  - Physical-device verification and live-emulator re-verification of newer flows (`DriveHubScreen`, `TripChatSheet`, `LiveTripLocationService`, `EventCheckInSheet`, `GraceRideWidget`, `DriverSettingsScreen`, `ProxyRidersScreen`, `SavedAddressesScreen`, `GrokBotStudioScreen`, `TermsScreen`).
  - Configuring production `ANDROID_CERT_SHA256_FINGERPRINTS` to verify live WebAuthn passkeys and Digital Asset Links end-to-end against `https://graceride.saliminn.my/.well-known/assetlinks.json`.
  - Native FCM push delivery (the backend `mobile_push_tokens` schema and `/api/push/mobile-token` routes are ready; Android currently delivers notifications via SSE `RealtimeStreamClient` + periodic `WorkManager` `BackgroundNotificationWorker`).
  - Play Store account enrollment and release listing, plus continuing to serve full admin/transport planning, reimbursements, help desk, family management, and staff report pages via AndroidX Custom Tabs (`WebHandoff.kt`).
