# Grace Ride Android — Design System

Native Material 3 translation of the web design tokens (`frontend/src/styles.css`).
The web app is minimalist: one background, one accent, borders instead of shadows.
The Android app keeps that character while using native components and motion.

## Colour

| Role | Light | Dark | Source token |
|---|---|---|---|
| primary | `#1F5DB4` | `#7AA7EE` | `--primary` (logo Ride blue) |
| onPrimary | `#FFFFFF` | `#0B1730` | `--primary-ink` |
| primaryContainer | `#EEF3FB` | `#1B2432` | `--primary-soft` |
| background / surface | `#FFFFFF` | `#141517` | `--bg` |
| surfaceVariant | `#F4F4F3` | `#1F2124` | `--surface-2` |
| onSurface | `#1A1A1A` | `#EDEDED` | `--text` |
| onSurfaceVariant | `#6B6B6B` | `#A3A3A3` | `--muted` (5.3:1 on white) |
| outlineVariant | `#E6E6E6` | `#2E3135` | `--border` |
| error | `#B4372F` | `#F0938B` | `--danger` |

Status colours (text + soft fills only, never large fills — same rule as web):
success `#2B7549`/`#EEF6F1`, warning `#8A5800`/`#FBF4E6`, info `#2C5D9C`/`#EEF3FB`,
danger `#B4372F`/`#FBEEEC`; dark variants lighten the text and darken the fill.
Exposed through `LocalStatusColors`. Brand red `#E8462F` stays in the logo only.

Cards are `OutlinedCard` (1dp border, no elevation) — the web draws cards with
borders on the page background, not shadows.

## Typography

Web root 17px; scale 14 / 17 / 19 / 20 / 24 / 30. Mapping (`ui/theme/Type.kt`):

- headlineMedium 30sp SemiBold (h1)
- headlineSmall 24sp SemiBold (h2)
- titleLarge 20sp SemiBold (h3)
- titleMedium 17sp Medium, titleSmall 14sp Medium
- bodyLarge 17sp (default body), bodyMedium/bodySmall 14sp
- label* 14sp Medium — nothing smaller than 14sp, matching the web's elder-friendly floor.

All sizes in `sp`: the system font scale is respected (elder-friendly text scaling).

## Spacing, shape, touch

- Spacing scale 4 / 8 / 16 / 24 / 32 / 48 / 64 dp (`Spacing` object) — the web's exact scale.
- One corner radius: 8dp for every M3 shape slot; pills/chips use `CircleShape`.
- Minimum touch target 48dp (M3 default `minimumInteractiveComponentSize`), matching `--tap: 48px`.
- Motion: M3 defaults (~150–300ms ease-out family) echo `--dur: 150ms; ease-out`.

## Layout & insets

- `enableEdgeToEdge()` in `MainActivity`; content draws behind system bars.
- `Scaffold` supplies safe insets; screens apply `contentPadding`/`padding(padding)`.
- Bottom navigation (phone): **Home · Events · Profile** (3 primary tabs; Rides, Costs, History, Saved Addresses with GPS capture, Frequent Proxy Riders, Driver Vehicle & Carpool Settings, and Grok Bot Studio live natively under Profile, while Event Detail—via `EventScheduleTimeline`—and Home link straight to Ride Detail and booking; `TermsScreen` presents the PDPA v2 consent gate when needed).
  Volunteer drivers also receive native trip execution controls on Ride Detail (confirm assignment, start trip, delay notice, at-gate arrival, pickup, complete, withdraw); extended workflows (trip chat, live map, QR check-in, reimbursements, family management, and MFA-gated staff tools) hand off directly to the Web App via browser links.
- Detail screens use `TopAppBar` with a back arrow; tab roots do not.
- Adaptive behaviour: single-column lists today; `WindowSizeClass` rail/tablet
  layouts are a later increment (the web switches at 768/1200px).

## Avatars, Iconography & Badges

- **Animated mascots & bundled GIFs (`Avatar.kt`, `EventIcon.kt`):** Native Compose animated mascot shapes pair with bundled Bloub-style transparent GIF assets (`assets/icons/avatars/`, `badges/`, `events/`) rendered via `coil-gif`.
- **Grok Bot Studio (`GrokBotStudio.kt`):** Interactive clay-style bot renderer (`grokbot:<shape>:<face>:<color>`, 8 silhouettes × 16 faces × 12 colours with animated poses) tuned for both light and dark themes.
- **4-tier achievement medallions (`BadgeIcon.kt`):** Progressive badge rings and palettes (`none` → `bronze` → `silver` → `gold` → `platinum`, with subtle animated halo emphasis on `gold`/`platinum`) matching the web's status tokens without relying on colour alone.

## States

Every data screen implements the four states from the web (`useApi` + `ui` components):

- **Loading**: centered `CircularProgressIndicator` (first load only).
- **Refreshing**: M3 `PullToRefreshBox` on lists; stale data stays visible.
- **Empty**: icon + message (`EmptyView`), e.g. "No rides to show. Book a ride…".
- **Error**: message + Retry (`ErrorView`); server messages are shown verbatim
  (bilingual via `Accept-Language`), never raw stack traces.
- **Offline**: `OfflineBanner` ("You are offline. Showing saved data.") when a
  request fails with no HTTP status; section-level failures degrade gracefully
  (Home shows the sections that loaded).

Session states are global: 401 → Login, 423 `SESSION_LOCKED` → Unlock (TOTP /
backup code), 403 `ACCOUNT_PENDING` → Pending approval screen. Ride mutations
that lose the optimistic-concurrency race (`RIDE_STATUS_CHANGED`) re-fetch and
show a snackbar instead of failing hard.

## Accessibility

- Content descriptions on icon buttons (back, delete); decorative icons null.
- Status is never colour-only: pills carry text labels.
- 14sp floor + sp scaling + 48dp targets (elder-friendly, per the web's goals).
- TalkBack: M3 components expose roles/states; tabs announce selection.
- Contrast: muted text 5.3:1 on white (same as web); dark scheme keeps AA.

## i18n

`values/strings.xml` (en) + `values-zh/strings.xml` (zh), mirroring the web's
bilingual UI. Per-app language toggle in Profile writes `lang` to the server
(`PUT /api/me`) and recreates the activity; `Accept-Language` follows it so API
error messages arrive in the same language.
