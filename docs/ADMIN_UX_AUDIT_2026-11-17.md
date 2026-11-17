# Admin UI/UX audit and enhancement plan — 2026-11-17

Scope: every staff screen reached from the admin navigation (`frontend/src/pages/admin/*`, the staff shell in `Layout.tsx` / `staffNav.tsx`, and the shared table, modal and pagination components they use). Audit and plan only; no code was changed.

## How this was checked

- **Code read**: the 12 admin pages (10 in full; `Audit.tsx` and `PlanTransport.tsx` in part), the layout shell, `UniversalTable`, `Modal`, `Tabs`, `PageHeader`, and the CSS rules behind each finding.
- **Rendered**: a production build, driven by Playwright with a mocked API, at three sizes — desktop 1440×900, tablet 900×1100, phone (iPhone 13 profile, touch). 20 screens/states per size, plus a finance-only user on phone. Each render recorded horizontal overflow, page height, serious/critical axe violations, console errors, and (on phone) touch targets under 40px.

Limits to keep in mind:

- Data was mocked. Layout findings are reliable; list *contents* are not (the mock returned the same rides for every filter, and audit-log action names and detail text were invented).
- Not checked: dark mode, the Chinese locale, real devices, slow networks, Cost splits (not rendered), and the Help desk beyond an empty-state render.
- "Rendered" below means seen in a screenshot or measurement. "Code" means read in source and not exercised.

## Summary

The admin area is structurally sound: one consistent shell, permission-aware navigation, a work-queue dashboard, URL-backed filters, bottom-sheet modals, and no page-level horizontal overflow at any size. No screen crashed.

The problems cluster in four places:

1. **Phones get the desktop table, stacked.** Three of the busiest lists have no phone layout, producing pages 3,000–9,000px tall.
2. **Tablet widths (768–1199px) fall between layouts.** Tables overflow sideways and hide their action buttons; the navigation is an unlabeled 17-icon rail.
3. **A few workflows are unsafe or dead-end.** Account-recovery approval has no confirmation, its row layout is broken on phones, and "back" sends some staff to a no-access page.
4. **Inconsistent finishing.** Exports that silently cover one page, a reports header that doesn't lay out, mixed naming, and raw data shown where labels exist.

## Findings

Severity: **High** = blocks or endangers a task; **Med** = slows or confuses; **Low** = polish.

### High

| # | Finding | Where | Affects | Evidence |
|---|---|---|---|---|
| H1 | **Account recovery rows are broken on phones.** The Approve/Reject buttons overlap the member's name, email and reason; text is squeezed to a ~3-word column and partly unreadable. The row is a single flex line: `.grow` has no `min-width: 0` and `.btn-row > .btn` insists on 140px. The invite log has the same squeeze, less severely. | `Members.tsx:641-687`, `:572-599`; `styles.css:466-467` | Phone | Rendered |
| H2 | **Recovery decisions fire on one tap.** Approve logs the member out of every device and issues a one-time link; Reject has no undo in the UI. Neither asks for confirmation, although invite revoke and group delete do. The issued link is shown as plain text with no copy button and is lost on reload. | `Members.tsx:626-630`, `:656-686` | Both | Code + rendered |
| H3 | **"Back" leads to a no-access page.** Every admin page sets `back="/admin"` (the dashboard). A finance-only user on the payout screen taps back and lands on "You don't have access to this page". On phones the natural parent is the admin menu (`/admin/menu`), not the dashboard; the dashboard's own back goes to Home. | every page's `PageHeader`; e.g. `Reimbursements.tsx:72`, `Dashboard.tsx:189` | Phone (back arrow is phone-only) | Rendered |
| H4 | **Tablet tables hide their actions.** At 900px the Members table is 232px wider than its card: Group, Passkeys and the Edit/Approve column are off-screen with no scroll cue. The Audit table overflows by 323px. Reimbursements overflows by 41px, clips its Review/Receipt buttons, and wraps each description to one or two words per line. The compact phone layout only starts below 768px. | `ui.tsx:1466-1470` (`useNarrowScreen`), `Members.tsx:196-271` | Tablet / small laptop | Rendered |
| H5 | **Three lists have no phone layout.** Without `compactRow`, each record becomes a stacked card of every column. Measured: Reimbursement review 8 rows = 2,986px; Audit log 25 rows = 8,913px with 96 tap targets under 40px; Dashboard "Today's rides" 7 rows ≈ 1,500px. Rides and Members already have compact rows and are fine. | `Reimbursements.tsx:269`, `Audit.tsx:913`, `Dashboard.tsx:250` | Phone | Rendered |
| H6 | **Member reports header doesn't lay out.** Six controls (month, range, type, member picker, two exports) are passed as `PageHeader` actions. On desktop the selects stretch full-width in a stack with the exports floating beside the last one; on phone they push the report about 480px down. The member picker is a plain `<select>` of every member with no search. | `Reports.tsx:225-289` | Both | Rendered |

### Medium

| # | Finding | Where | Affects | Evidence |
|---|---|---|---|---|
| M1 | **Dashboard repeats itself and runs long.** The work queue already counts approvals and follow-ups; the two cards below list the same items again. Phone height 5,259px. "Recent activity" prints the raw entity key (`ride #100`) and unformatted detail text, while the Audit page has proper label helpers. The status pill is clipped in the desktop "Today's rides" table (29px overflow). | `Dashboard.tsx:233-357`, `:341-345` | Both | Rendered |
| M2 | **Roles editor loses work and hides feedback.** Picking another role remounts the editor and silently discards unsaved ticks; there is no unsaved-changes marker and Save sits at the bottom of a 2,500px (desktop) / 4,600px (phone) page. On phones the role list sits above the editor, so tapping a role should change nothing on screen (inferred from the layout; the tap was not exercised). The four action headers are squeezed into 46px columns and run together ("CreateUpdateDelete"). | `Roles.tsx:96`, `:265-276`; `styles.css:2018-2022` | Both; worst on phone | Rendered + code |
| M3 | **Exports cover only the current page, silently.** The list says "Showing 1–50 of 137" but the CSV has 50 rows. Headers are English-only; reimbursement review exports cents while payout exports ringgit. | `Rides.tsx:94-107`, `Members.tsx:180-194`, `Reimbursements.tsx:146-172` | Both | Code |
| M4 | **Search and filter gaps.** Ride management cannot search by passenger or driver name. Members has no status or role filter, so pending members are found only by scanning for an "Approve" button. Tabs show no waiting counts although `Tabs` supports `count` and the dashboard API already returns the recovery and claim counts (not pending members). | `Rides.tsx:79-92`, `Members.tsx:176-178`, `:108-112` | Both | Code |
| M5 | **Navigation between 768 and 1199px wide** (tablets, and laptop windows that are not full width). The sidebar becomes a rail of icons with the labels removed; they return only as hover tooltips, which touch screens never show. 17 of the rail's links have no accessible name at all (axe `link-name`, serious) — only the badged ones are announced. Separately, at 1440×900 a user with every permission finds the Money, Content and System groups below the fold. | `Layout.tsx:136-148`, `:156-177`; `styles.css:303-327` | Tablet, laptop | Rendered |
| M6 | **Names disagree.** Nav and top bar say "Transport dashboard"; the page heading says "Ride overview". "Groups & areas" also holds reimbursement rates, data retention and session lock, which nobody would look for under that name. The top bar title is blank on ride detail and the transport plan. | `Layout.tsx:26-59`, `Settings.tsx:62-72` | Both | Rendered |
| M7 | **Settings forms hide what they do.** Out-of-range numbers are silently clamped on save. Rates are typed in cents per km. The area edit dialog has no error display or busy state inside it (the error renders behind the dialog). Group-member search is case-sensitive and unlabeled. Each group row carries Edit, Members and Delete side by side. | `Settings.tsx:565-568`, `:493-516`, `:394-430`, `:303-311`, `:130-146` | Both | Code + rendered |
| M8 | **Reimbursement review dialog.** Approve, Return and Reject sit in one row at equal weight; a note is not required to return or reject, so the member may get no reason. | `Reimbursements.tsx:366-396` | Both | Code |
| M9 | **Announcements are write-once.** No edit and no pin/unpin after publishing. "Notify everyone" defaults on and sends without a confirmation. The preview card shows placeholder text to staff who cannot create. | `Announcements.tsx:79-135` | Both | Code |
| M10 | **Transport plan on phones.** Four full-width toolbar buttons stack above the board and "Confirm & notify" scrolls away on a 2,830px page. The per-card driver menu works well. | `PlanTransport.tsx:207-231` | Phone | Rendered |

### Low

| # | Finding | Where | Evidence |
|---|---|---|---|
| L1 | Pagination uses `role="list"` with no list items (axe serious, on every paged list); Previous/Next are 34×36px on touch. | `ui.tsx:1159` | Rendered |
| L2 | Audit log: the time column wraps to four lines on desktop; filter links are 26px tall and the open-object icon 12px; the detail dialog shows the raw action key while the table shows a label. | `Audit.tsx:624-711`, `:958-962` | Rendered |
| L3 | Reports: `aria-label="range"` / `"type"` are untranslated; PDF section titles ("By event kind", "Events", "earned") are hard-coded English. The PDF uses jsPDF's built-in font, which likely cannot print Chinese names — not verified. | `Reports.tsx:238`, `:248`, `:169`, `:184`, `:207` | Code |
| L4 | Links inside text (ride link in a claim description, phone number on ride detail) are distinguished by colour only. | `Reimbursements.tsx:198` | Rendered (axe) |
| L5 | The member edit dialog offers the Visitors group as a chip (only the "everyone" group is filtered out). A failed clipboard copy shows nothing. | `Members.tsx:468`, `:506-513` | Rendered + code |
| L6 | Live map: a driver with no location can be selected but nothing happens; zoom buttons are 30px. | `Live.tsx:51-56` | Code + rendered |
| L8 | On the phone ride list a long status pill ("Alternative transport approved") squeezes the passenger name to a few letters. | `Rides.tsx:185-201` | Rendered |
| L7 | Quick jump is hidden on phones; the bottom-bar Admin badge sums every queue (showed "19"), which reads as one alarming number. | `Layout.tsx:204`, `:291` | Rendered |

## What already works

- No page-level horizontal overflow at any size; no crashes; no console errors besides a mock artifact.
- Work-queue dashboard ordered by urgency with one action per row.
- Compact phone rows on Rides and Members; modals become bottom sheets with focus trapping.
- Filters, sort and page live in the URL, so lists can be bookmarked and shared.
- Navigation and buttons hide per permission; the phone admin menu is clean and badge-aware.
- Transport plan offers a menu alternative to drag-and-drop and warns about unsent changes.

## Enhancement plan

Effort: S ≈ under half a day, M ≈ 1–2 days, L ≈ 3+ days. Each phase is shippable on its own.

### Phase 1 — Safety and dead ends (about 2–3 days)

**Status: implemented 2026-11-17, uncommitted.** Checked with unit tests and a browser render at phone and tablet size. Not yet covered: dark mode, the Chinese wording of the new dialogs (written, not reviewed by a speaker), and a real device.

| Item | Fixes | Effort |
|---|---|---|
| Confirm dialog on recovery Approve and Reject, stating the consequence; copy button and persistent display for the issued link | H2 | S |
| Stack recovery and invite rows on phones (text above, buttons below); add `min-width: 0` to the text block | H1 | S |
| Permission-aware back target: admin menu on phones, dashboard only if the user may open it | H3 | S |
| Roles: unsaved-changes guard when switching role or leaving; sticky Save bar | M2 (part) | M |
| Pagination semantics and 44px touch targets | L1 | S |
| Give every nav-rail link an accessible name at 768–1199px | M5 (part) | S |
| Require a note to return or reject a claim; separate Reject from Approve | M8 | S |

### Phase 2 — Phone and tablet lists (about 3–4 days)

**Status: implemented 2026-11-17.** Checked with unit tests and a browser render at phone, tablet and desktop size (mocked data). Part of this work (the table component, shared ride row, audit/dashboard/members/rides pages) was swept into commit `c8d3fbf` by another session's broad `git add`; the rest is in the Phase 2 commit. Not covered: dark mode, real devices, the Chinese wording of new labels.

| Item | Fixes | Effort |
|---|---|---|
| `compactRow` for Reimbursements, Audit and Dashboard "Today's rides" (tap a row to open the existing detail dialog) | H5 | M |
| Raise the compact-layout switch from viewport width to container width, or to ~1100px, so tablets stop overflowing; until then keep the action column sticky | H4 | M |
| Rebuild the Reports header as a filter bar under the title; searchable member picker | H6 | M |
| Roles on phone: role picker as a select above the editor; wider action columns or two-letter headers with a legend | M2 (rest) | M |
| Sticky "Confirm & notify" bar on the plan board; fold secondary tools into a menu | M10 | S |

### Phase 3 — Finding and finishing work faster (about 1 week)

| Item | Fixes | Effort |
|---|---|---|
| Server-side "export all matching" for Rides, Members, Reimbursements; state the row count on the button | M3 | M (needs backend) |
| Name search on Rides; status and role filters on Members; waiting counts on tabs | M4 | M (name search may need backend) |
| Dashboard: drop or collapse the lists that duplicate the queue; reuse the Audit label helpers for recent activity | M1 | M |
| Tablet navigation: labeled rail or collapsible drawer | M5 (rest) | M |
| Announcements: edit, pin/unpin, confirm before notifying everyone | M9 | M (needs backend) |

### Phase 4 — Consistency (about 2–3 days)

| Item | Fixes | Effort |
|---|---|---|
| One name per screen across nav, top bar and heading; split or rename "Groups & areas"; add missing top-bar titles | M6 | S |
| Settings: inline range validation, rates in RM, errors inside dialogs, case-insensitive member search | M7 | M |
| Audit table density, target sizes and label consistency | L2 | S |
| Reports translations and a PDF font that covers Chinese | L3 | M |
| Underline in-text links; filter Visitors from the group chips; clipboard failure message; live-map empty selection; long status pills on phone rows | L4–L6, L8 | S |
| Add admin screens to the visual/axe suite at phone, tablet and desktop so these do not regress | all | M |

### Suggested order

Phase 1 first: it is small and removes the only items that can cause harm (H2) or strand a user (H1, H3). Phase 2 next, because staff on phones and tablets hit H4/H5 on every visit. Phases 3 and 4 can be reordered freely; the three backend-dependent items in Phase 3 are the natural ones to split out.

### Open questions for the product owner

1. Do staff mostly work on phones, tablets or laptops? This decides whether Phase 2 or Phase 3 comes second.
2. Should recovery approval need a typed confirmation, or is a dialog enough?
3. Is the dashboard meant for transport staff only, or should finance and help-desk staff get their own landing screen?
