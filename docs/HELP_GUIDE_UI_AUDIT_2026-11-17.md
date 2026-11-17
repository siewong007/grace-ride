# Help centre step-by-step guides: UI audit and enhancement plan

Date: 2026-11-17. No source files were changed for this audit.

## What was audited

| Surface | File | How it was checked |
| --- | --- | --- |
| Guide page (the checklist of steps) | `frontend/src/pages/HelpGuideDetail.tsx`, `frontend/src/components/GuideSteps.tsx` | Code + browser, 390px and 1280px, light and dark |
| Walkthrough bar (follows you across pages) | `frontend/src/components/GuideRunner.tsx` | Code + browser, 390px and 1280px, light and dark |
| Guide lists on the help home | `frontend/src/components/HelpHome.tsx`, `frontend/src/pages/Help.tsx` | Code + browser, member and staff |
| Guide styles | `frontend/src/styles.css` (lines 2251-2270, 2347-2410, 2543) | Code |

Not audited: the guide editor (`HelpGuideEditor.tsx`), the arranger (`GuideArranger.tsx`), and the Chinese rendering (the browser run stayed in English). The browser check used a production build with a mocked API, not live data.

## Findings

### Broken

1. **The walkthrough bar's buttons stack into a tall column.** Expand, Back, Next and Close each take a full row, so the bar is about 210px tall collapsed and 340px expanded, on phone and desktop alike. On a 844px-tall phone the expanded bar covers 40% of the page it is meant to guide you through.
   Cause: the buttons sit in `.btn-row`, and `.btn-row > .btn { flex: 1 1 140px }` with wrapping (`styles.css:466-467`) makes each one grow and wrap. `GuideRunner.tsx:185` asks for `btn-row nowrap align-center`, but neither `.btn-row.nowrap` nor `.align-center` exists in the stylesheet.
2. **On a phone the bar does not say which guide or step you are on.** The guide title and step title are squeezed to nothing; only "· Step 1 / 3 ·" is left.
3. **The step instruction is hidden by default.** The bar opens collapsed, so the user sees Back/Next but not what to do. The text only appears after tapping an unlabelled chevron.
4. **The bar covers page content and the desktop sidebar.** It is `position: fixed; left: 0; right: 0` and nothing pads the page beneath it: the `has-active-walkthrough` and `guide-runner-container` classes it sets have no CSS rule.
5. **Toasts land on top of the bar** (and on top of the "Contact us" button on the guide page), because both are pinned to the bottom of the screen.

### Confusing flow

6. **Two competing ways to follow a guide.** The guide page is a checklist with "Go to Events" links; "Start walkthrough" is a second mode with its own bar. They share progress but behave differently, and nothing explains which to use.
7. **"Go to Events" leaves the guide and marks the step done before the user has done anything** (`GuideSteps.tsx:173`). The user lands on Events with no instruction and no way back except the browser.
8. **Steps without a link leave the walkthrough on the previous page.** Step 2 of "Plan transport" ("Open the planning board") is shown while the user is still on Events, with nothing pointing at where to go.
9. **Links with a query string never match.** `GuideRunner.tsx` compares `loc.pathname` with the whole link, so a step linking to `/help?tab=guides` re-navigates every time, and the "Guide" page-header button cannot find it.
10. **The expanded bar offers "Open this step page" when you are already on that page.**
11. **The "Guide" button in a page header starts a walkthrough part-way through a guide** (at whichever step links to that page), with no context about the earlier steps.

### Weak design on the guide page

12. **No sense of "you are here".** Every step has the same weight; done steps are not quietened and the next step is not highlighted.
13. **The tick control is hard to find and small.** The number circle is the button (32px; the rest of the app uses 44px). Its label ("Mark as completed") does not say which step.
14. **Progress reads "1 of 3" with no noun**, and the bar has no `progressbar` role for screen readers.
15. **No finish.** Completing every step changes nothing: no "done" state, no next guide, no way to start over.
16. **"Completed" is shown twice** per done step (filled circle and a "Completed" label).
17. **The feedback box is heavy.** Two half-width buttons, then a full-width primary "Contact us"; the answer is forgotten on reload, so the question comes back.
18. **Step text is plain text only.** No emphasis, no notes or warnings, no picture of the screen being described.
19. **Search results that deep-link to a step (`#step-2`) scroll it under the sticky header**; there is no scroll offset.

### Guide lists

20. **Two different looks for the same thing.** Members get rows; staff get cards. Cards for a finished guide look identical to an unstarted one.

### Code issues behind the UI

21. **Progress is stored as step positions** in `localStorage`. If an editor reorders or removes steps, the wrong steps show as done and the count can exceed the total.
22. **The same `localStorage` read/write is written three times** (`GuideSteps.tsx`, `GuideRunner.tsx`, `Help.tsx:61`).
23. **`back` is missing from `help.en.ts` and `help.zh.ts`**, so the Back button falls back to hard-coded English. Several other strings carry English fallbacks inline.
24. **Layout is done with inline `style=` props** in `GuideSteps.tsx` and `GuideRunner.tsx`, and with classes that do not exist (`align-center`, `tone-green`).

## Plan

One model instead of two: the guide page is where you read; the walkthrough is the same guide, shrunk to a small card that travels with you. Frontend only.

### Phase 1: repair the walkthrough card

Files: `GuideRunner.tsx`, `styles.css`, `help.en.ts`, `help.zh.ts`.

- Replace the bar with a compact card: one line "Step 2 of 5 · Open the planning board", the instruction text shown by default, then Back / Next and a close button on one row.
- Phone: sits above the bottom nav, full width, capped at about a third of the screen with the text scrolling inside. Desktop: 380px card in the bottom-right of the content area, clear of the sidebar.
- Pad the page bottom while the card is open so nothing is hidden; move toasts above it.
- A collapse control that leaves a small "Step 2 of 5" pill, for when the card is in the way.
- Remove "Open this step page" when already there; fix the query-string link comparison.
- Add the missing `back` string in both languages; drop inline styles and undefined classes.

Fixes findings 1-5, 9, 10, 23, 24 (for this file).

### Phase 2: rebuild the guide page

Files: `GuideSteps.tsx`, `HelpGuideDetail.tsx`, `styles.css`, locales.

- Highlight the next step to do; quieten done steps (muted text, tick in the circle, no second "Completed" label).
- A proper 44px tick control labelled "Mark step 2 done"; progress as "1 of 3 steps done" with a `progressbar` role.
- "Go to Events" becomes the step's main button and starts the walkthrough at that step, so the instruction comes along. A step is marked done by Next or by ticking it, not by following a link.
- A finish state when all steps are done: confirmation, "Start again", and the next guide for that audience.
- Feedback as one quiet line ("Did this guide help? Yes / No"), remembered per guide; "Contact us" as a secondary button.
- Scroll offset for `#step-N` links.

Fixes findings 6, 7, 12-17, 19.

### Phase 3: walkthrough behaviour and shared progress

Files: new `frontend/src/guideProgress.ts`, `GuideRunner.tsx`, `GuideSteps.tsx`, `Help.tsx`, `HelpHome.tsx`.

- One `useGuideProgress(slug)` hook replacing the three copies; clamp to the current step count so edits cannot produce "4 of 3".
- Steps without a link say "On this page:" and stay put on purpose, instead of looking stuck.
- The page-header "Guide" button opens the card at the matching step with the guide title and a "See all steps" link, instead of silently starting mid-guide.

Fixes findings 8, 11, 21 (partly), 22.

### Phase 4: guide lists

Files: `Help.tsx`, `HelpHome.tsx`.

- Same row/card content for members and staff: title, summary, and one status (not started / 2 of 5 / done).

Fixes finding 20.

### Later, needs a decision

- **Richer step content** (finding 18): bold text, a note/warning line, an optional screenshot per step. This changes the stored step shape and the editor, so it touches the backend.
- **Progress tied to steps, not positions** (rest of finding 21): needs a stable id on each step, also a backend change.

## Tests each phase must keep green

- `frontend/src/pages/HelpGuides.test.tsx`
- `frontend/e2e/help-guides.spec.ts`: it clicks the link named "Go to Events" and expects to land on `/events`. Phase 2 must keep that name and destination.
- `frontend/src/i18n.test.ts`: English/Chinese key parity for every new string.
- `frontend/e2e/visual.spec.ts`: baselines are Linux-only; do not regenerate them on macOS.

## Sequencing note

`backend/src/seed.rs` and `backend/tests/guides.rs` have uncommitted changes from another session. Phases 1-4 do not touch the backend. The two "later" items do and should wait until that work lands.
