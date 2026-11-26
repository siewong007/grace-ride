# Grace Ride — Native Android App

Kotlin · Jetpack Compose · Material 3 · MVVM. Reuses the existing Rust/Axum backend and REST API unchanged; the React web app remains untouched.

- **Package:** `org.pjstatecm.graceride`
- **SDK:** `minSdk 28` · `targetSdk 36` · `compileSdk 36` · Java `17`
- **Toolchain & Core Libraries:** AGP `8.13.0` · Gradle `8.13` · Kotlin `2.2.20` · Compose BOM `2025.10.00` · Navigation Compose `2.9.5` · `material3-adaptive-navigation-suite` · Retrofit `2.11.0` · OkHttp `4.12.0` (`okhttp-sse`) · `kotlinx.serialization` `1.9.0` · Coil `2.7.0` (`coil-compose`, `coil-gif`) · AndroidX Biometric `1.1.0` · Credential Manager `1.5.0` · WorkManager `2.10.1` · Glance AppWidget `1.1.1` (`glance-material3`) · ZXing Core `3.5.4` · Play Services Code Scanner `16.1.0` · Core Splashscreen `1.0.1` · AndroidX Browser Custom Tabs `1.9.0`

---

## 1. Implemented Screens & Native Capabilities

| Area | Screens & Native Capabilities |
|---|---|
| **Public & No-Account Flows** | `LoginScreen` + `LoginViewModel` (direct hardware-backed biometric sign-in, WebAuthn passkeys, password/recovery reset flow with `TurnstileDialog`, dev quick-login & LAN server override), `TermsScreen` (PDPA consent gate), `LicensesScreen` (`third-party-licenses.json`), `InviteScreen` (`/invite/{token}`), `VisitScreen` (`/visit/{token}` + claim + readiness + live tracking), `CarePassScreen` (`/care/{token}` + readiness + driver card), `UnlockScreen` (biometric / passkey / TOTP + backup codes; server `423 SESSION_LOCKED` persisted across offline cold starts), `PendingScreen` + `PendingChecklistCard` |
| **Chrome, Links & Widget** | `GraceRideAppRoot` (`NavigationSuiteScaffold` switching automatically between a bottom `NavigationBar` — Home · Events · Traffic (permission-gated) · Profile — on compact phones and a `NavigationRail` on foldables/tablets), type-safe `@Serializable` `AppRoute` destinations (`Routes.kt`), verified App Links (`https://graceride.saliminn.my/...` including `/admin/dashboard`, `/admin/menu`, `/admin/traffic`, `/admin/rides`, `/admin/events/.../plan`, `/admin/live`, `/admin/members`, `/admin/announcements`, `/admin/roles`, `/admin/audit`, `/admin/settings`, `/settlements*`, `/reimbursements*`, `/help*`, `/report*`), `QuickJumpSheet` command palette (`>` action filter), internal shortcut deep links (`graceride://open/...`), static OS App Shortcuts (`shortcuts.xml`), `GraceRideBrandMark` + SplashScreen, and resizable Material 3 Glance home-screen widget (`GraceRideWidget` + `WidgetSnapshotStore`) |
| **Home** | `HomeScreen`: brand mark + wordmark header with `QuickJumpSheet` search button, notifications bell & avatar, `OnboardingChecklistCard`, permission-gated Traffic workspace card & Administration Console card, next ride, unpaid cost shares, driving today + 7-day next-drive preview, offline ride snapshot fallback, upcoming events, announcements (with `View all` → `AnnouncementsScreen`), pull-to-refresh, per-section error degradation |
| **User Management, Admin Consoles & Announcements** | `AdminConsoleScreen` (KPI tiles, Work Queue, today's rides filter, recent audit, module shortcuts), `AdminMembersScreen` + `MemberEditSheets` (`FLAG_SECURE`-protected 3-tab console: `Members` search/status/role/sort/pagination + CSV export via `ExportShare`, single approve/edit sheet with VP acknowledgement, batch approval; `Invites`; `Recovery`), `AdminRolesScreen` (custom role CRUD + permission matrix with `locked`/`fixed` guards), `AdminAuditScreen` (cursor-paginated filterable log, diff modal, object links, visible & streaming CSV exports), `AdminSettingsScreen` (System retention/concurrency, Alert channels/webhooks/VAPID + webhook test, hierarchical Groups CRUD & member editor), and `AnnouncementsScreen` |
| **Rides, Live Tracking & Chat** | `RidesScreen` (upcoming/past), `RideNewScreen` (event-gated, outbound/round/return legs, family/proxy riders + saved riders, GPS capture, interactive `LocationPinPickerSheet` map pin picker, place search & saved places, seats, assistance), `RideDetailScreen` (area pickup photos + zoom, `CareLinkSheet`, readiness, accept/decline, cancel with optimistic-concurrency retry, native driver actions backed by durable `OfflineActionQueue`, Traffic staff controls, single-ride Change Drafts, `"Need help?"`), `TripChatSheet` (SSE chat + inline direct reply), `LiveTripLocationService` + `LiveTrackCard` + `GraceRideLiveMap` |
| **Traffic Admin & Bulk Fare Sheets** | `TrafficHubScreen` (`TrafficHubViewModel`, `TrafficAreasSection`) + `PlanTransportScreen`: native 4-tab hub (`Plans`, `Rides` with CSV export + `Proposed changes`, `Live` map, and permission-gated `Areas` / `Photos` / `Fares` with pure-Kotlin `XlsxCodec` & `FareSheetCodec` XLSX/CSV export, SAF picker & bulk preview/import), full event transport board with multi-version `EventPlanDrafts`, lease locks, diff summary, undo/reset, auto-arrange solver, `Last time` & `Match returns` batch helpers, ranked driver picker, visitors, shareable PNG export, post-event reimbursement claims |
| **Driver Hub** | `DriveHubScreen` + `DriveHubViewModel`: native 3-tab hub (Open Requests + 1-tap Claim, 14-Day Schedule, Live Route map + stops + ETA), `OfflineActionQueue` pending banner & 1-tap sync, and Waze/Google Maps turn-by-turn launch via `DeviceActions.kt` |
| **Events & Native QR Check-In** | `EventsScreen` (Upcoming/Past/Managing, draft filter, publish/unpublish), `EventFormScreen` (create/edit/duplicate/audience picker), `EventDetailScreen` + `EventTimelineSection` (RSVP, withdraw, check-in code display, existing ride card, lazy visitors & area lookup, attendance toggles, waitlist promote, finish/cancel, `"Settle costs"` shortcut), `EventCheckInSheet` + `EventPassQrSheet` + `QrCodeHelper`, zero-permission `CalendarActions.addEventToCalendar` |
| **Costs, Settlements & Reimbursements** | `CostsScreen` (integer cents → RM, DuitNow QR, proof upload + server OCR `/api/files/{id}/extract-ref` auto-fill & candidate reference chips), `HistoryScreen` (cursor-paginated), `SettlementsScreen` / `SettlementDetailScreen` / `SettlementFormScreen` (`Money.equalSplit`, due-date picker, searchable participants, live sticky split footer, segmented verification legend, `ZoomableImageDialog` pinch-to-zoom proof/QR viewer, share verification sheet, revisions), `ReimbursementsScreen` / `ReimbursementDetailScreen` / `ReimbursementFormScreen` (status total strip, timeline stepper, receipt-first OCR upload, `ZoomableImageDialog`, resubmit bar), `AdminReimbursementsScreen` (review sheet with quick reasons, month-stepped payout batches, payee pay sheet + CSV/Mark Paid) |
| **Help Center & Reports** | `HelpScreen` / `HelpDetailScreen` (member & staff ticket queues, directional chat bubbles, claim/reply/close/reopen, attachments + `ZoomableImageDialog`), `HelpGuideDetailScreen` / `HelpGuideEditorScreen` (step timeline cards, EN/ZH segmented bilingual step editor, reorder, helpful feedback), `ReportScreen` (formatted period selector, 2×2 metric tiles, badges & CSV export), `AdminReportsScreen` (`overview` & `demand` donut/threshold cards, `events`, `finance` tabs, badge icon sheet & CSV export) |
| **Profile, Studio & Settings** | `ProfileScreen` (100% native member tools + staff/admin workspaces), `ProfileSections` + `ProfileSubScreens` (`PersonalDetailsScreen`, `AppearanceSettingsScreen` + `textScale`, `NotificationSettingsScreen` with webcal calendar feed + Telegram bot connect + push test/devices, `SecuritySettingsScreen` with passkeys, TOTP 2FA setup/backup codes, sign-out all, recovery code), `FamilyScreen` (family CRUD + leave), `DriverSettingsScreen`, `ProxyRidersScreen`, `SavedAddressesScreen` (GPS + `LocationPinPickerSheet`), `GrokBotStudioScreen` + `AvatarEditorSheet` (8 shapes × 16 faces × 12 clay colours, 77 WebP assets, `BadgeIcon`), per-app EN/ZH toggle |
| **Notifications & Real-Time** | `NotificationsScreen` (inbox, mark all read, deep links via `NotifLinkRouter`), `RealtimeStreamClient` (OkHttp SSE `/api/stream`), `BackgroundNotificationWorker`, `NotificationHelper` (4 localized channels, lock-screen redaction, inline reply), `OfflineActionQueue` |
| **External Fallback Only** | `WebHandoff.kt`: AndroidX Custom Tabs retained solely for external URLs (`https://t.me/...`, `webcal://`, OpenStreetMap search, or unmapped future paths) |

---

## 2. Architecture, Security & Offline Resilience

- **Packages (`android/app/src/main/java/org/pjstatecm/graceride/`):**
  - `data/api/` — Retrofit `ApiService`, `ApiProvider` (thread-safe hot-swap of Retrofit endpoints without recreating repositories), `kotlinx.serialization` `Dtos`, `AdminDtos` & `TrafficDtos`, `ApiClient` error mapping (`423 DRAFT_LOCKED_BY_OTHER` kept distinct from `423 SESSION_LOCKED`), `AuthEventBus`
  - `data/cache/` — `OfflineCache` (per-user atomic JSON stale-while-revalidate store) & `OfflineActionQueue` (durable FIFO offline ride/driver action queue)
  - `data/media/` — `ImageUploadPrep` (resize/strip prep for proof and avatar uploads)
  - `data/repo/` — `AdminRepository`, `AuthRepository`, `CommunityRepository`, `EventsRepository`, `FinanceRepository`, `HelpRepository`, `PublicFlowRepository`, `RidesRepository`, `SharesRepository`, `TrafficRepository`
  - `data/session/` — `SecureStore` (`EncryptedPrefsStore` using hardware/OS-backed `AndroidKeyStore` AES-256-GCM + `AesGcmCodec` with AAD key-binding, replacing deprecated `androidx.security:security-crypto`), lazy-loaded `PersistentCookieJar`, `OfflineSnapshot`
  - `di/` — `AppContainer` manual dependency injection & `Accept-Language` caching
  - `domain/` — `AdminLogic`, `AreaMatch`, `FareSheetCodec`, `Format` (`KlTime`, integer-cent RM formatting, `equalSplit`), `PlanLogic`, `Readiness`, `RideStatus`, `XlsxCodec`
  - `notifications/` — `RealtimeStreamClient`, `BackgroundNotificationWorker`, `NotificationHelper`, `ChatReplyReceiver`, `ReadinessActionReceiver`
  - `platform/` — `BiometricAuthManager`, `CalendarActions`, `ExportShare` (`CsvBuilder` + `FileProvider` export/share), `NetworkMonitor`, `QrCodeHelper`
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
./gradlew :app:testDebugUnitTest         # 184 JVM unit tests across 20 suites (MockWebServer)
./gradlew :app:lintDebug                 # Android Lint check
./gradlew :app:connectedDebugAndroidTest # 45 Compose UI tests across 7 files (emulator/device)
./gradlew :app:bundleRelease             # signed AAB if keystore.properties or GR_STORE_* env vars exist
```

From the repository root, `Makefile` also exposes `make lint-android` and `make test-android` (unit tests + compiling `debugAndroidTest` Kotlin).

- **Dev vs. Release API Target:** Debug builds point at `http://10.0.2.2:8080/` (`make db-up`, then run backend with `DEV_MODE=true`) and expose the dev quick-login + LAN server override. Release builds target `https://graceride.saliminn.my/` (override via `-Pgraceride.apiBaseUrl=…`).
- **Signing:** `keystore.properties` (gitignored) or `GR_STORE_FILE` / `GR_STORE_PASSWORD` / `GR_KEY_ALIAS` / `GR_KEY_PASSWORD` env vars configure release signing outside the repository. Replace any local verification upload key with the ministry's own upload key and enroll in Play App Signing before Play Store upload. Never commit keystores.
- **Passkeys & App Links (`ANDROID_CERT_SHA256_FINGERPRINTS`):** Set `ANDROID_CERT_SHA256_FINGERPRINTS` (comma-separated colon-hex SHA-256 signing fingerprints) on the Rust backend so it serves `/.well-known/assetlinks.json` and registers `android:apk-key-hash:` origins in WebAuthn (`backend/src/lib.rs`, `state.rs`).

---

## 5. Verification Status & Pre-Launch Scope

- **Latest Local JVM, Lint & Build Verification (2026-11-26, JDK 17):**
  - **184 JVM unit tests across 20 suites (`184/184` passed — `0` failures, `0` errors, `0` skipped):** `AdminMembersAndAnnouncementsTest` (`6/6`), `ApiSessionTest` (`10/10`), `ArchitectureAndPlatformTest` (`9/9`), `AreaMatchAndRoutesTest` (`2/2`), `BugfixRegressionTest` (`10/10`), `FinanceHelpAndAdminParityTest` (`13/13`), `GrokBotMotionTest` (`12/12`), `KlTimeTest` (`8/8`), `LocationServicePolicyTest` (`6/6`), `MoneyTest` (`6/6`), `NotificationsTest` (`16/16`), `PerformanceRegressionTest` (`8/8`), `PersistentCookieJarTest` (`8/8`), `PublicAndSecurityParityTest` (`3/3`), `RegressionTest` (`10/10`), `RideStatusTest` (`5/5`), `SecurityAndOfflineTest` (`9/9`), `ThemeAndBadgeTest` (`4/4`), `TrafficAdminAndDraftsTest` (`7/7`), and `ViewModelTest` (`32/32`).
  - **Android Lint, String Parity & Compilation:** `:app:lintDebug` passed (`0` errors, `193` warnings, `1` hint), `:app:assembleDebug` and `:app:compileDebugAndroidTestKotlin` succeeded, and EN/ZH `strings.xml` parity check passed (`2,037` keys matched, `0` duplicates, `0` unescaped apostrophes).
  - **Compose UI Instrumentation Suite (`45` tests across `7` files — `45/45` passed at `1x`, plus `17/17` passed at `0x` reduced motion):** Executed cleanly on API 37 `Medium_Phone_API_37.0` (`-gpu host`) with `0` `AndroidRuntime`/`TestRunner` failures across `AdminMembersAndAnnouncementsAndroidTest` (`4/4`), `TransitionsAndAnimationsAndroidTest` (`7/7`), `TrafficAdminAndroidTest` (`6/6`), `ComposeUiTest` (`15/15`), `RoleAndFeatureMatrixAndroidTest` (`5/5`), `FullAppPagesAndScenariosAndroidTest` (`7/7`), and `FullWebParityAndroidTest` (`1/1`); all `17/17` tests across `AdminMembersAndAnnouncementsAndroidTest`, `TransitionsAndAnimationsAndroidTest`, and `TrafficAdminAndroidTest` also passed with `animator_duration_scale = 0` (`1x` animation scales restored afterward).
- **Emulator & Live Dev Backend Verification (2026-11-26, API 37 `Medium_Phone_API_37.0`, 4 cores / 4 GB / `-gpu host`):**
  - Executed instrumentation at both `1x` animations and `0x` reduced motion plus a live Light/Dark walkthrough against local `DEV_MODE=true` API covering Login (`dev-users`), Home, Events & `EventDetailScreen` push/pop transitions, Traffic Hub, Profile, `AdminMembersScreen` (`Members`/`Invites`/`Recovery` tabs, `MemberEditSheet`, sticky batch selection bar, and `FLAG_SECURE` window protection via `SecureScreenEffect`), and `AnnouncementsScreen` + `AnnouncementEditorSheet`. `dumpsys gfxinfo` measured 3,804 rendered frames (371 janky / 9.75%, p50 22ms, p90 32ms, p95 40ms, Skia OpenGL) with 0 `AndroidRuntime` crashes.
- **Remaining Pre-Launch Items:**
  - Physical-device verification and live-backend walkthrough of the expanded parity screens (`PublicFlowScreens`, `LocationPinPickerSheet`, `Settlements*`, `Reimbursements*`, `AdminReimbursementsScreen`, `Help*`, `ReportScreen`, `AdminReportsScreen`, `AdminConsoleScreen`, `AdminRolesScreen`, `AdminAuditScreen`, `AdminSettingsScreen`, XLSX/CSV fare sheets, `TrafficHubScreen`, `PlanTransportScreen`, `DriveHubScreen`, `TripChatSheet`, `LiveTripLocationService`, `EventCheckInSheet`, `GraceRideWidget`, `DriverSettingsScreen`, `ProxyRidersScreen`, `SavedAddressesScreen`, `GrokBotStudioScreen`, `TermsScreen`).
  - Configuring production `ANDROID_CERT_SHA256_FINGERPRINTS` to verify live WebAuthn passkeys and Digital Asset Links end-to-end against `https://graceride.saliminn.my/.well-known/assetlinks.json`.
  - Native FCM push delivery (the backend `mobile_push_tokens` schema and `/api/push/mobile-token` routes are ready; Android currently delivers notifications via SSE `RealtimeStreamClient` + periodic `WorkManager` `BackgroundNotificationWorker`).
  - Play Store account enrollment and release listing.
