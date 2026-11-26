# Grace Ride iOS App (`恩典同行 · Grace Ride`)

Native Swift / SwiftUI iOS 17+ companion client for **PJ State Chinese Methodist Church — Grace Ride** (`org.pjstatecm.graceride`), alongside the shared `GraceRideKit` Swift Package (`ios/GraceRideKit`) and WidgetKit + AppIntents extension (`ios/GraceRideWidget`).

---

## Requirements

- **macOS + Xcode 16+** (Xcode Swift 5.10+ / iOS 17.0 deployment target)
- Zero external third-party Swift Package Manager or CocoaPods dependencies (pure Apple SDK frameworks: `SwiftUI`, `Observation`, `Foundation`, `Network`, `Security`, `AuthenticationServices`, `CoreLocation`, `MapKit`, `WebKit`, `SafariServices`, `EventKit`, `PhotosUI`, `AVFoundation`, `CoreImage`, `ImageIO`, `BackgroundTasks`, `UserNotifications`, `WidgetKit`, `AppIntents`)
- **Active Xcode developer directory**: If your Mac defaults `xcode-select` to Command Line Tools, prefix commands with:
  ```sh
  export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
  ```

---

## Quick Commands

| Task | Command |
|---|---|
| Run `GraceRideKit` unit tests | `make test-ios` or `DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer" swift test --package-path ios/GraceRideKit` |
| Build iOS Simulator target (unsigned) | `DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer" xcodebuild -project ios/GraceRide.xcodeproj -scheme GraceRide -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build` |
| Build iOS Device target (unsigned) | `DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer" xcodebuild -project ios/GraceRide.xcodeproj -scheme GraceRide -destination 'generic/platform=iOS' CODE_SIGNING_ALLOWED=NO build` |
| Open in Xcode | `open ios/GraceRide.xcodeproj` |

---

## Project Layout

```
ios/
├── GraceRide.xcodeproj/           # App + WidgetKit extension project & shared scheme
├── GraceRideKit/                  # Zero-dependency local Swift Package (macOS 14+ testable + iOS 17+)
│   ├── Package.swift
│   ├── Sources/GraceRideKit/
│   │   ├── API/                   # Codable DTOs, JSONValue, URLSession HTTP + multipart + SSE clients
│   │   ├── DI/                    # AppContainer (@MainActor @Observable), theme/lang/baseURL management
│   │   ├── Domain/                # KlTime (Asia/Kuala_Lumpur), Money, AreaMatch, Routes, NotifLinkRouter,
│   │   │                          # Readiness, RideStatus, QrParsing, WidgetSummary, LocationServicePolicy,
│   │   │                          # GrokBotSpec/GrokBotGeometry, AvatarStyleHelper
│   │   ├── Realtime/              # NetworkMonitor (NWPathMonitor) + RealtimeStreamClient (/api/stream SSE)
│   │   ├── Repositories/          # AuthRepository, RidesRepository, EventsRepository, SharesRepository, CommunityRepository
│   │   ├── Session/               # KeychainSecureStore / InMemorySecureStore, PersistentCookieJar (RFC 6265),
│   │   │                          # OfflineCache (per-user SHA-256), OfflineSnapshotStore
│   │   └── ViewModels/            # 15 @MainActor @Observable ViewModels for every screen
│   └── Tests/GraceRideKitTests/   # 8 unit test suites (25 tests)
├── GraceRide/                     # Native SwiftUI iOS App target (org.pjstatecm.graceride)
│   ├── GraceRideApp.swift         # App lifecycle, APNs registration, interactive notifications, BGAppRefreshTask, Quick Actions
│   ├── GraceRideAppRoot.swift     # Auth state gate, adaptive TabView/NavigationSplitView, Universal Links
│   ├── DesignSystem/              # Color tokens (Light/Dark parity), Typography (14pt floor), L10n, Haptics, WebP loader
│   ├── Platform/                  # PasskeyAuthManager (AuthenticationServices + Turnstile), LiveTripLocationManager,
│   │                              # QrCodeTools (CoreImage + AVFoundation), ImageUploadPrep (EXIF-stripped JPEG), DeviceActions
│   ├── Components/                # GraceRideBrandMark, GrokBotCanvasView, MascotCanvasView, AvatarView,
│   │                              # BadgeAndEventIcons, EventScheduleTimelineView, StateViews
│   ├── Screens/                   # Auth (Login/17-section PDPA v3 Terms/Unlock/Pending), Home, Rides, RideNew,
│   │                              # RideDetail + LiveTrackCard + TripChatSheet, DriveHub, Events + EventDetail + Check-In,
│   │                              # Costs, Notifications, Profile, AvatarEditor + GrokBotStudio, DriverSettings,
│   │                              # ProxyRiders, SavedAddresses
│   └── Resources/                 # Assets.xcassets (1024px AppIcon + AccentColor), 77 bundled WebP icons,
│                                  # Localizable.xcstrings (770+ EN/zh-Hans strings), Info.plist, PrivacyInfo.xcprivacy, Entitlements
└── GraceRideWidget/               # WidgetKit + AppIntents extension (org.pjstatecm.graceride.widget)
    ├── GraceRideWidgetBundle.swift
    ├── GraceRideWidget.swift      # systemSmall / systemMedium Next Trip widget via App Group group.org.pjstatecm.graceride
    ├── MarkRideReadyIntent.swift  # Interactive "I'm Ready" AppIntent
    ├── Info.plist
    └── GraceRideWidget.entitlements
```

---

## Architecture & Feature Parity

1. **Session, Cookies & Security**:
   - Sessions use the backend's `HttpOnly` cookie (`gr_session` / `__Host-gr_session`) persisted in iOS Keychain (`kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, never backed up to iCloud) via `PersistentCookieJar` with RFC 6265 path matching.
   - HTTP requests set `Accept-Language` dynamically and intentionally omit any `Origin` header for native clients.
   - Offline cold-start restores cached `Me` only when valid session cookies remain and auto-lock has not expired. All caches, Keychain entries, and widget snapshots are wiped immediately on sign-out or global `401` / `403`.
   - `PrivacyInfo.xcprivacy` declares `NSPrivacyAccessedAPICategoryUserDefaults` (`CA92.1`, `1C8F.1`).

2. **Authentication, Associated Domains & Backend Support**:
   - Passkey sign-in, step-up re-auth, and registration use `AuthenticationServices` (`ASAuthorizationPlatformPublicKeyCredentialProvider`) with WebAuthn Base64URL payloads and optional Cloudflare Turnstile `WKWebView` verification.
   - The Rust backend serves `GET /.well-known/apple-app-site-association` configured via `IOS_APP_IDS` (`TEAMID.org.pjstatecm.graceride` entries) so both Universal Links (`applinks:`) and Passkey Associated Domains (`webcredentials:`) work seamlessly when deployed.
   - TOTP / backup code unlock (`UnlockScreen`), PDPA v3 17-section consent (`TermsScreen`), and pending-approval hold (`PendingScreen`) match web/Android behaviour.

3. **Real-Time Updates, Push & Background Sync**:
   - Foreground updates stream over `/api/stream` (SSE) with exponential backoff and automatic watermark sync.
   - `GraceRideAppDelegate` supports APNs device registration (`provider: "apns"`, additive migration `0012_ios_apns_push_provider.sql`), `BGAppRefreshTask` (`org.pjstatecm.graceride.sync`), lock-screen privacy (`hideLockDetails`), inline trip chat reply (`UNTextInputNotificationAction`), and one-tap readiness notification actions.

4. **Strict Journey-Only Location Privacy**:
   - `LiveTripLocationManager` enforces `LocationServicePolicy`: background driver GPS updates (`allowsBackgroundLocationUpdates`, `showsBackgroundLocationIndicator = true`, ~10s / ≥25m throttle, 60s watchdog) run **strictly** during an active `live == true` route and stop immediately when the trip completes, cancels, locks, or signs out.

5. **Bilingual EN / 简体中文 & Accessibility**:
   - All 770+ UI strings live in `Localizable.xcstrings` (`en` and `zh-Hans`), resolved dynamically via `L10n` when toggling language without restarting the app, alongside Light/Dark/System `ThemeMode`, ≥44pt touch targets, and 14pt readable body/label hierarchy.
