# Grace Ride (恩典同行): UI/UX enhancement plan, written for Gemini to execute

## Context (for the owner)

Grace Ride works, but its frontend looks like a monochrome prototype: black-and-white palette, success and warning pills that look the same grey, hand-written tab bars, `confirm()` dialogs, emoji used as icons, about 55 inline styles, and Chinese strings hardcoded in English mode. Theme, text-size and language preferences exist in code (`src/prefs.ts`, `DisplayPreferences` in `src/components/AppChrome.tsx`, `setLanguage` in `src/i18n.ts`), but nothing in the UI uses them. Several real bugs are also mixed in: a hooks-rule crash risk, form buttons that submit by accident, raw translation keys showing on screen, and a wrong link.

You chose a **full visual refresh** of every area (rider, driver, admin, design system), split into **small phased PRs**. The brief below is self-contained so it can go to Gemini as-is. On approval I'll also save it in the repo as `docs/UI_UX_PLAN.md`, so Gemini can read it from the checkout.

---

# BRIEF FOR THE EXECUTING AGENT

## 0. Read first: global rules (apply to every phase)

**Product.** Grace Ride is an invite-only ride and fellowship web app for a church in Petaling Jaya, Malaysia. Many users are **older adults on phones**, so readability and large tap targets come before density. The UI is bilingual: Chinese first (zh), with English (en).

**Stack (do not change it).** React 18 + TypeScript (strict) + Vite, react-router 7, plain CSS in a single file (`frontend/src/styles.css`), lucide-react icons, i18next, Leaflet. Shared UI lives in `frontend/src/components/ui.tsx`. All paths below are relative to `frontend/`.

**Hard rules**
1. **Frontend only.** Do not touch `backend/`, API paths, request payloads or permission logic. Server behaviour must stay identical.
2. **No new UI or CSS libraries.** No Tailwind, MUI, shadcn, styled-components or CSS-in-JS. Extend `src/styles.css` and `src/components/ui.tsx`. You may add new files under `src/components/`.
3. **No hex colours or inline `style={{}}` in TSX.** Use CSS variables and classes. The one exception is a truly dynamic value such as a progress width.
4. **Every user-visible string goes through i18n.** Add each key to *both* `src/locales/<ns>.en.ts` and `src/locales/<ns>.zh.ts`, with identical `{placeholder}` names (single braces). `src/i18n.test.ts` enforces parity.
5. **Icons come from lucide-react only.** No emoji used as icons. Icon-only buttons need an `aria-label`.
6. **Accessibility baseline:** interactive targets at least 48×48px on touch screens, WCAG AA contrast (4.5:1 for text, 3:1 for large text and UI), visible `:focus-visible`, and every control usable from the keyboard.
7. **Privacy UI must keep working.** The lock icon and hidden pickup address before approval (commit `4b0504e`), and the live map shown only during in-progress rides, must look and behave the same as before.
8. **Check every page you change** at 375px, 768px and 1280px, in light and dark mode, in both languages, and at the "xlarge" text size.
9. **One PR per phase.** Branch from the latest `master` as `ui/<phase-number>-<slug>`, open a PR against `master`, and **do not merge it yourself**. Keep each PR focused on its phase; do not refactor unrelated code.
10. **Before opening each PR,** run `npm run test`, `npm run build`, `npm run lint` (once Phase 1 adds it) and `npm run test:e2e` from `frontend/`. All must pass. Never delete or skip a failing test to get green.

**PR description template**
```
## Phase N: <name>
### What changed
### Screenshots (375 / 1280, light + dark) for each page touched
### How I verified (commands run + manual checks)
### Out of scope / follow-ups noticed
```

**Manual testing:** start the backend and frontend as described in the root `README.md` ("Run locally"). Sign in with the README's demo accounts to see each role: member, driver, transport, finance, admin.

---

## Phase 1: Guardrails and bug fixes (PR `ui/01-foundations`)

**Goal:** a safe base before any visual work.

1. **Add ESLint 9** with a flat config (`frontend/eslint.config.js`) using `typescript-eslint`, `eslint-plugin-react-hooks` (`rules-of-hooks` = error, `exhaustive-deps` = warn) and `eslint-plugin-jsx-a11y` (recommended rules as **warn**). Add the script `"lint": "eslint src"` and add `npm run lint` to the frontend job in `.github/workflows/ci.yml`, after `npm ci`.
2. **Fix these bugs:**
   - `src/pages/EventDetail.tsx`: `useApi(.../signups)` is called after early returns. Move every hook above the returns.
   - `src/pages/Settlements.tsx` `SettlementForm`: `id ? useApi(...) : {data:null}` is a conditional hook. Always call the hook and skip the fetch when there's no id (extend `useApi` in `src/api.ts` to accept `null` as the path, meaning "don't fetch").
   - `SettlementForm`: give the add-item and delete-item buttons `type="button"`.
   - `SettlementForm` custom split: make it work. Show one amount input per attendee when the mode is `custom`, keep the values in state, and validate that they are non-negative and add up exactly to the total (integer cents).
   - `SettlementDetail`: the `t('settlementDetail.*')` keys are missing. Add them to `src/locales/settlements.{en,zh}.ts`.
   - `SettlementDetail`: replace the hardcoded `width: '60%'` bar with the real paid/total ratio. Show the Edit button only to the settlement's organiser or an admin, using the same check the page already uses elsewhere (or data from the API).
   - `src/pages/admin/Dashboard.tsx`: `t('all')` should be `t('rides.all')`.
   - `src/pages/Home.tsx`: the unpaid-share link uses `/settlements/${s.title}`. Change it to the id.
   - `src/pages/admin/Members.tsx`: disabled users show `members.statusActive`. Use a correct "disabled" label and add the key if it's missing.
   - `src/App.tsx`: move the `*` → `NotFound` route outside `RequireAuth`, so signed-out visitors see the 404 page. Remove the `<ErrorBox error={null}/>` in `src/pages/NotFound.tsx`.
   - Swallowed errors: replace every `.catch(() => {})` on a user action with the existing `useAction()` from `ui.tsx`, so failures show a toast.

**Done when:** lint reports zero errors (warnings are OK), all listed bugs are fixed, and tests and build pass.

---

## Phase 2: i18n sweep and status consolidation (PR `ui/02-i18n`)

1. Move every hardcoded string into the locale files. Known offenders:
   - `components/ui.tsx`: 载入中, 关闭, upload messages, 查看收据
   - `components/Layout.tsx`: nav aria-labels
   - `components/Chat.tsx`: input aria-label
   - `components/AppChrome.tsx`: its whole inline zh/en table. Move it into a new `chrome` namespace and register the namespace in `src/i18n.ts`.
   - `pages/Home.tsx`: 余 {n}
   - `pages/Settlements.tsx`: 加载中…, 保存中…, 交易编号, 核对, all of VerifyModal
   - `pages/EventDetail.tsx`: 需要接送, 提供 N 座
   - `pages/admin/Dashboard.tsx`: 24 小时内仍未有司机
   - `pages/admin/Live.tsx`: 项行程 · 首站
   - `pages/Reimbursements.tsx`: 接送
   - `pages/Help.tsx`: 帮助中心
   - `pages/Profile.tsx`: 无
   - `pages/admin/Members.tsx`: （非会员）
   - `pages/admin/Reimbursements.tsx`: alt 收据
   - `pages/Drive.tsx`: placeholder 白色
   - `src/webauthn.ts`: 已取消

   Then search for any remaining CJK characters in `.tsx`/`.ts` files outside `src/locales/` with `grep -rnP '[\x{4e00}-\x{9fff}]' src --include='*.ts*' | grep -v locales`, and fix every hit.
2. Remove the copy-pasted `useTranslationHooks()` helpers and the `t`/`ft` props typed as `any`. Call `useTranslation()` directly in each component.
3. **Single source of truth for statuses.** Status → label key and tone mappings live only in `src/format.ts`. Delete `STATUS_MAP`/`getStatusMap` in `Settlements.tsx` and `STATUS()` in `Help.tsx` and import from `format.ts`. Every `fmtAgo`/`fmtDateTime` call must pass the translator.

**Done when:** the grep returns nothing, there is no Chinese text in English mode on any page, and `i18n.test.ts` passes.

---

## Phase 3: Design tokens and the new visual language (PR `ui/03-tokens`)

**Goal:** replace the monochrome palette with a warm, calm, high-contrast system, without changing layouts yet. Every edit in this phase is in `src/styles.css`, plus the brand files in step 7.

**1. Palette.** Replace `:root` and the dark block. There are currently *two* dark blocks (around lines 33–56 and 354–361); merge them into one `@media (prefers-color-scheme: dark) { :root:not([data-theme='light']) {...} }` block plus one `:root[data-theme='dark'] {...}` block with the same values. Keep every existing variable name working (alias old names to new ones) so nothing breaks.

| Token | Light | Dark | Use |
|---|---|---|---|
| `--bg` | `#F6F4EF` | `#121412` | page |
| `--surface` | `#FFFFFF` | `#1A1D1B` | cards |
| `--surface-2` | `#EFECE5` | `#232725` | inputs, subtle fills |
| `--border` | `#E2DDD3` | `#323835` | |
| `--text` | `#1D1B18` | `#F1EFEA` | |
| `--muted` | `#5E5A52` | `#A8A49B` | secondary text |
| `--primary` | `#1E6B5C` | `#5FBFA8` | brand teal-green: buttons, active nav, links |
| `--primary-hover` | `#17574A` | `#7CCDB9` | |
| `--primary-ink` | `#FFFFFF` | `#0F1F1A` | text on primary |
| `--primary-soft` | `#E3F0EC` | `#1D3530` | active nav bg, selected chips |
| `--accent` | `#C2841F` | `#E0A644` | warm amber, **fills only** (badges, highlights) |
| `--accent-ink` | `#1D1B18` | `#1D1B18` | text on accent |
| `--accent-soft` | `#FBF0DC` | `#3A2E17` | |
| `--success` / `--success-soft` | `#2F7D4F` / `#E3F2E8` | `#6CC18D` / `#1C3326` | completed, paid, approved |
| `--warning` / `--warning-soft` | `#9A6200` / `#FCF0D6` | `#E5B454` / `#3A2E14` | pending, needs attention |
| `--danger` / `--danger-soft` | `#B4372F` / `#FBE6E3` | `#EF7B70` / `#3D1F1C` | |
| `--info` / `--info-soft` | `#2C5D9C` / `#E4ECF8` | `#86AEE6` / `#1C2A3D` | in-progress, informational |

Alias the old names: `--blue`→`--info`, `--teal`→`--muted`, `--gray-soft`→`--surface-2`. Check every text/background pair with a contrast checker; if a pair fails AA, darken the light value or lighten the dark value, and note the change in the PR.

**2. Spacing scale.** `--space-1: 4px; --space-2: 8px; --space-3: 12px; --space-4: 16px; --space-5: 20px; --space-6: 24px; --space-8: 32px; --space-10: 40px`. Replace the literal gaps, paddings and margins in `styles.css` with the nearest token.

**3. Radius.** `--radius-sm: 10px; --radius: 14px; --radius-lg: 20px; --radius-pill: 999px`. Replace the hardcoded 12/14/20/999 values.

**4. Elevation.** Add `--shadow-sm: 0 1px 2px rgb(0 0 0 / .05)`; keep the current shadow as `--shadow`; add `--shadow-lg: 0 8px 32px rgb(0 0 0 / .12)`. In dark mode, shadows become `none` and cards rely on `--border`.

**5. Typography.**
- Set `html { font-size: 106.25%; }` (17px) and change `body` to `font-size: 1rem`, so that `prefs.ts` text scaling multiplies everything. Check how `applyPrefs` sets the size and make it scale the `html` font-size.
- Convert **every** `px` font-size in `styles.css` to `rem` (for example `.small` → `.875rem`, `.tiny` → `.75rem`, table 15.5px → `.92rem`).
- Type scale: `--fs-xs .75rem; --fs-sm .875rem; --fs-md 1rem; --fs-lg 1.125rem; --fs-xl 1.375rem; --fs-2xl 1.75rem; --fs-3xl 2.125rem`. Use `h1` = 2xl, `h2` = xl, `h3` = lg.
- Use only weights 400, 600 and 700. Replace 650 → 600 and 800 → 700.
- Keep the system font stack (CJK fonts are already in it; no web fonts, for speed on older phones). Add `font-variant-numeric: tabular-nums` to times, prices and stats.

**6. Motion and tap size.** `--dur-fast: 120ms; --dur: 200ms; --ease: cubic-bezier(.2,.8,.2,1); --tap: 48px`. Keep the existing `prefers-reduced-motion` handling. Set `.btn` `min-height: var(--tap)`, and `.btn.sm` to at least 40px.

**7. Brand.** Update `public/icon.svg` to the `--primary` teal background with the same white car/arc mark. Set `theme_color` in `index.html` and `public/manifest.webmanifest` to `#1E6B5C`, and `background_color` to `#F6F4EF`.

**8. Remaining hardcoded colours:**
- `Login.tsx`: replace `#3b82f6` with `var(--primary)`, and define or remove `.other-methods`.
- `Home.tsx`: replace the hero's `rgba(255,255,255,.18/.1)` with tokens that work in both themes.
- `Notifications.tsx`: move the inline unread-dot styles into a CSS class.
- `.dot`: replace `#fff` with `var(--primary-ink)`.
- `LiveMap.tsx`: read marker colours from CSS variables via `getComputedStyle(document.documentElement)`.

**Done when:** `grep -rnE '#[0-9a-fA-F]{3,8}\b' src --include='*.tsx'` returns nothing, the app looks correct in light and dark, and the text-size preference scales every piece of text.

---

## Phase 4: UI primitives and accessibility (PR `ui/04-components`)

Build these in `src/components/ui.tsx`, or in new files under `src/components/` re-exported from `ui.tsx`. Style them in `styles.css`. Each one should be small, typed and documented with a one-line JSDoc comment.

| Component | API | Notes |
|---|---|---|
| `Button` | `variant: 'primary'\|'secondary'\|'ghost'\|'danger'`, `size: 'sm'\|'md'\|'lg'`, `block?`, `loading?`, `icon?: ReactNode`, `as Link` via `to?` | defaults to `type="button"`; while `loading`, shows a spinner and sets `aria-busy`; renders the existing `.btn` classes |
| `IconButton` | `label` (required, becomes `aria-label`), `icon`, `variant` | 48px square |
| `Card` | `variant: 'default'\|'flat'\|'hero'\|'link'`, `padding: 'sm'\|'md'\|'lg'` | wraps the `.card` classes |
| `PageHeader` | `title`, `subtitle?`, `back?: string`, `actions?: ReactNode` | standard top of every page; back arrow on mobile |
| `Page` | `width: 'narrow'\|'medium'\|'wide'` (720 / 900 / full) | replaces every inline `maxWidth` |
| `Tabs` | `value`, `onChange`, `items: {value,label,count?,icon?}[]` | `role="tablist"`, `role="tab"`, `aria-selected`, roving tabindex with ←/→ keys; horizontally scrollable on phones |
| `Modal` (upgrade) | add `description?`, `footer?`, `size?` | focus trap, Esc closes, focus returns to the trigger, `aria-labelledby`, body scroll lock; keep the bottom-sheet look on phones, with a drag-handle visual |
| `ConfirmDialog` + `useConfirm()` | `const ok = await confirm({title, body, confirmLabel, tone:'danger'})` | provided by a context next to `ToastProvider` |
| `EmptyState` | `icon`, `title`, `description?`, `action?` | replaces `Empty`; keep `Empty` as a thin alias |
| `Skeleton`, `ListSkeleton`, `CardSkeleton` | `lines?`, `count?` | shimmer animation, disabled under reduced motion |
| `Pill` / `StatusPill` (upgrade) | tones `neutral\|success\|warning\|danger\|info\|primary` | map the old `green`→`success` and `amber`→`warning`; optional leading icon |
| `Toast` (upgrade) | success/error/info variants with an icon | queue up to 3; errors stay 6s, others 3s; keep `role="status"`; add a close button |
| `Stat` | `label`, `value`, `icon?`, `tone?`, `to?` | for dashboard tiles |
| `KeyValue` | `items: {label, value}[]` | replaces the ad-hoc `.kv` |

Then:
- Replace every native `confirm()` (EventDetail, Profile, admin Settings, admin Announcements) with `useConfirm()`.
- Replace every hand-written tab bar (Rides, Events, Drive, Help, admin Members, admin Reimbursements) with `Tabs`.
- Replace emoji icons (⚠ ♿ 🔒 📌 📎 📡 🙋 🚗) with lucide equivalents: `AlertTriangle`, `Accessibility`, `Lock`, `Pin`, `Paperclip`, `Radio`, `Hand`, `Car`.
- In `admin/Dashboard.tsx` `RideTable`, make rows keyboard-accessible. The first cell becomes a `<Link>` that covers the row with a pseudo-element, so there's no `onClick` on the `<tr>`.
- Add unit tests in `src/components/ui.test.tsx` for `Tabs` (keyboard navigation), `Modal` (Esc closes, focus returns), `useConfirm` (resolves true/false) and `Button` (default `type="button"`).

**Done when:** the components exist with tests, the replacements above are complete, and there are no `confirm(` calls left in `src/`.

---

## Phase 5: App shell, navigation and preferences (PR `ui/05-shell`)

Files: `src/components/Layout.tsx`, `src/components/AppChrome.tsx`, `src/pages/Profile.tsx`, `styles.css`.

1. **Sidebar (desktop and tablet rail):**
   - Brand block at the top: icon plus 恩典同行 / Grace Ride.
   - Section labels 会员 / 服事 / 管理, kept as they are.
   - Active item gets a `--primary-soft` background, `--primary` text and a 3px left indicator.
   - Hover uses `--surface-2`.
   - On the tablet rail, show a tooltip with the label on hover and focus.
2. **Top bar:**
   - Show the current page title on mobile. The brand appears only on Home.
   - The notification bell becomes an `IconButton` with an unread count badge (cap the display at "9+").
   - Avatar on the right linking to `/me`.
   - Add a subtle bottom border on scroll (use an `IntersectionObserver` sentinel, no scroll listeners).
3. **Bottom tab bar (phone):**
   - 64px plus the safe-area inset.
   - Icon above a label, with labels always visible at a minimum of `.75rem`.
   - The active tab shows `--primary` and a filled pill behind the icon.
   - Keep the per-role selection of 5 items.
4. **Preferences.** Render `DisplayPreferences` in `Profile.tsx` under a new "Display / 显示" card with three controls:
   - Theme: system / light / dark, as a segmented control.
   - Text size: normal / large / xlarge, with a live "Aa" preview.
   - Language: 中文 / English, which calls `setLanguage()` from `src/i18n.ts`.

   Also add a compact language toggle on the Login page.
5. **Notices.** Restyle the pending-account and MFA notices as `.notice` banners with an icon, using the warning and info tones.
6. **Profile layout.** Remove the cards nested inside cards. Use one card per section: Personal info, Display, Security (passkeys and TOTP), Account.

**Done when:** theme, text size and language can be changed in the UI and persist across reloads, and navigation looks right at all three breakpoints.

---

## Phase 6: Sign-in and public pages (PR `ui/06-auth`)

Files: `pages/Login.tsx`, `pages/Invite.tsx`, `components/Terms.tsx`, `pages/NotFound.tsx`.

- **Login:**
  - On ≥900px, a two-column layout: on the left, a brand panel with a `--primary` background, the app name, a one-line welcome in both languages and a simple SVG road/arc illustration in CSS; on the right, the form.
  - On phones, the brand header sits above the form.
  - Large "Sign in with passkey" primary button, the email field above it, then "Email recovery" as a ghost button.
  - The dev account picker gets a dashed "DEV" frame so nobody mistakes it for a real feature.
  - Give `.auth-card` the `card` class like the other auth pages.
- **Invite:**
  - Turn it into a 3-step stepper: Your details → Terms → Create passkey. Show "Step n of 3" plus a progress bar.
  - Inline field validation, with errors shown under each field.
  - Explain what a passkey is in one sentence, with an icon.
- **Terms:** readable measure (max 68ch), headings and a sticky accept bar at the bottom in the `TermsGate` state.
- **NotFound:** friendly illustration (a lucide `MapPinOff` at large size), a title, and two buttons: Go home, and Back.

---

## Phase 7: Rider core flows (PR `ui/07-rider`)

Files: `pages/Home.tsx`, `pages/Rides.tsx`, `pages/RideNew.tsx`, `pages/RideDetail.tsx`, `components/RideCard.tsx`.

- **Home**
  - Greeting with the date. If there is a next ride, show a **Next ride hero card**:
    - Large date/time and a status pill.
    - Pickup → destination with route dots.
    - Driver avatar, name and car with a plate badge (or "Waiting for a driver" with a warning tone).
    - A full-width "View ride" button.
  - Below the hero, a quick-action row of 3 tiles: Request a ride, Events, Help.
  - Then the sections in this order: Today's driving (drivers only), Upcoming rides, Unpaid shares (warning tone), Events, Announcements.
  - Each section uses a `PageHeader`-style section title with a "See all" link, and an `EmptyState` when empty.
  - Use skeletons while loading.
- **RideCard**
  - Hierarchy: time (lg, tabular) and date on the left; destination (600 weight) and pickup area (muted) in the middle; status pill at the top right; driver avatar at the bottom right.
  - Keep the lock icon on a hidden address.
  - The whole card is one link with a hover lift.
- **Rides:** `Tabs` for upcoming and past; group the list under sticky date headers (Today, Tomorrow, weekday + date); use `ListSkeleton` while loading.
- **RideNew**
  - Split the form into labelled sections, each a `Card`: When (date and time), Where (pickup area and destination), Who (passenger count stepper with −/+ buttons at 48px), Needs (assistance chips with icons such as wheelchair and walking aid, plus a notes field).
  - When opened with `?event=`, show the event prefill as a dismissible info notice.
  - Sticky submit bar on phones.
  - Validate inline on blur, and scroll to and focus the first error on submit.
- **RideDetail**
  - Replace the flat progress bar with a **vertical status timeline**: done, current and upcoming states with icons and timestamps.
  - The phone layout, in order:
    1. Status header
    2. Driver/vehicle card with call and chat buttons when permitted
    3. Live map, only while in progress, in a collapsible card showing the "last updated Xs ago" timestamp
    4. Trip details as `KeyValue`
    5. Chat
  - On desktop, use two columns: details and timeline on the left, map and chat on the right.
  - Put the primary action for the viewer's role in a **sticky bottom action bar** on phones, with secondary actions in a "More" menu.
  - Use `ConfirmDialog` and `ReasonModal` for destructive actions.
  - Keep the Google Maps / Waze buttons, styled as a labelled button group.

---

## Phase 8: Member secondary pages (PR `ui/08-member`)

Files: `pages/Events.tsx`, `EventDetail.tsx`, `EventForm.tsx`, `Costs.tsx`, `Settlements.tsx`, `Reimbursements.tsx`, `Help.tsx`, `Notifications.tsx`, `Report.tsx`.

Apply `Page`, `PageHeader`, `Tabs`, `Card`, `EmptyState`, `Skeleton`, `Button` and `StatusPill` consistently on every page. Then make these page-specific changes:

- **Events:** each card gets a large date block (day number and month), title, location and time, a seats-left meter, and a "Signed up" pill. **EventDetail:** a hero with the date and location, a signup card with a single clear CTA (Sign up / Join waitlist / Withdraw), "Need a ride?" linking to RideNew prefilled, and the organiser tools in a separate "Manage" card.
- **Costs and Settlements:**
  - Show the amount owed large, and the payee account with a copy button that gives toast feedback.
  - Use a clear pending / needs info / confirmed status.
  - Use a real progress meter for paid/total.
  - In `SettlementForm`, line items become rows with a remove `IconButton`, and the running total updates live.
- **Reimbursements:**
  - A form with a receipt upload drop zone showing a thumbnail preview (using the existing `FileInput`).
  - The detail page shows a status timeline (submitted → reviewed → paid).
- **Help:** a ticket list with status pills. The ticket detail is a chat-style thread (my messages on the right, staff on the left).
- **Notifications:**
  - Group by Today, Earlier.
  - Unread items get a `--primary-soft` background and a dot.
  - Each item has an icon by type.
  - "Mark all read" is a ghost button in the header.
- **Report:** month picker, stat tiles (rides taken, rides driven, events attended), and badges as cards with an icon and their criteria. Use no ranking language.

---

## Phase 9: Driver experience (PR `ui/09-driver`)

File: `pages/Drive.tsx`, plus the shared `StartTripButton`/`ShareLocation` exports in `RideDetail.tsx`. Move those shared pieces into `src/components/TripActions.tsx` and update both imports.

- **Schedule tab:**
  - Day/week segmented control.
  - Today is a timeline of trip cards ordered by time. Each card shows the passenger, pickup area, destination, seats and assistance-needs icons.
  - Each card has **one large primary next-step button** that follows the state: Start trip → Picked up → Complete.
  - "No-show" and "Withdraw" sit in a secondary menu and need confirmation.
  - Navigation buttons (Google Maps / Waze) appear as soon as the address is visible.
  - If location permission is denied, show a clear notice with retry instructions.
- **Open requests tab:**
  - Filter chips for area and date.
  - Each request card shows the time, approximate area (keep the lock icon), seats needed and needs icons, plus an "Offer to drive" button.
  - If the claim conflicts, show a friendly toast and refresh the list.
- **Settings tab:** three cards (Service areas as toggle chips, Carpool seats as a stepper, Vehicle with model, colour and plate and a live plate-badge preview). There is one Save button, sticky on phones, and it shows a dirty-state indicator when there are unsaved changes.

---

## Phase 10: Admin and staff (PR `ui/10-admin`)

Files: `pages/admin/*.tsx`.

- **Dashboard:**
  - 8 `Stat` tiles with icons. Tone them by meaning: items needing attention use warning, failures use danger. Each tile links to the pre-filtered list.
  - Below the tiles, a two-column layout on desktop: Today's rides on the left, Pending approvals and Follow-ups on the right, each with its own `EmptyState`.
- **RideTable (shared):** sticky header, row hover, status pills, keyboard-accessible rows (done in Phase 4), stacked card layout on phones (keep the `data-label` approach), and a result count above the table.
- **Rides:** a filter bar with status chips, a date input, an "Needs attention" toggle and a clear-filters link. Keep the filters in the URL query string.
- **Live:**
  - Theme-aware markers with the driver's initials.
  - A side list of active drivers; clicking one pans the map to them.
  - A stale-location badge (warning) with "last seen Xm ago".
  - A legend.
- **Members:**
  - Merge the near-duplicate Edit and Approve modals into one `MemberFormModal` with a `mode` prop.
  - Table columns: Name/avatar, Roles as pills, Groups, Status, and Actions in an overflow menu.
  - Invites as a card list, with an `EmptyState` and copy-link buttons.
- **Settings, Announcements, Reimbursements, Audit:**
  - Every list and table gets an `EmptyState`.
  - Announcements get a preview of how members will see them.
  - Reimbursements use `Tabs` (Review / Payout) and a clear finance-only section.
  - Audit uses monospace timestamps, actor/action/target columns and a search box.

---

## Phase 11: PWA polish and visual regression (PR `ui/11-pwa-qa`)

1. **PWA icons.**
   - Generate PNG icons from the new `icon.svg`: `icon-192.png`, `icon-512.png`, `icon-maskable-512.png` (with a safe-zone margin) and `apple-touch-icon.png` (180px).
   - Add them to `manifest.webmanifest` and `index.html`.
   - Do **not** add a service worker in this phase.
2. **Visual regression.** Add `e2e/visual.spec.ts` using the existing mocked-API approach in `e2e/smoke.spec.ts`. It takes `toHaveScreenshot` baselines of Login, Home, RideDetail, Drive and admin Dashboard at 375px and 1280px, in light and dark mode.
3. **Accessibility checks.** Add `@axe-core/playwright` and assert zero serious or critical violations on those same pages.
4. **Report.** Write `reports/ui-refresh-summary.md` listing each phase, its PR link and the before/after screenshots.

---

## Verification (for each phase and at the end)

- From `frontend/`, run `npm run lint && npm run test && npm run build && npm run test:e2e`. All must pass.
- Start the backend and frontend as in the README, then sign in as each demo role and walk the flows. For each case, confirm the expected result:

  | Flow | Expected |
  |---|---|
  | Member requests a ride | The hidden address shows the lock icon |
  | Driver offers to drive | The request moves to the driver's schedule |
  | Member accepts the driver | The ride moves forward |
  | Transport approves the ride | The address becomes visible to the driver |
  | Driver starts the trip, then marks picked up, then complete | The live map shows only while the trip is in progress |
  | Event signup | The member is signed up, or waitlisted when full |
  | Settlement: create it, then pay, then verify | The paid/total meter updates |
  | Reimbursement: submit, then review, then pay out | The status timeline advances at each step |
- Toggle dark mode, xlarge text and English on every page touched, at 375px, 768px and 1280px. Check for overflow, clipped text and contrast problems.
