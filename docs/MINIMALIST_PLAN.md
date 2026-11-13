# Grace Ride: minimalist UI pass

A visual and usability pass over every page. It builds on the 11-phase refresh in
[`UI_UX_PLAN.md`](UI_UX_PLAN.md): the token system and primitives already exist, so this
pass **removes** rather than adds — fewer surfaces, fewer sizes, fewer buttons.

## Rules

- No changes to features, routes, API calls, data logic or the meaning of any copy.
- No new libraries. Work in `frontend/src/styles.css`, `frontend/src/components/ui.tsx` and the pages.
- Privacy UI stays as it is: the lock on a hidden pickup address, and the live map only during a trip in progress.
- Every user-visible string goes through i18n, in both `en` and `zh`.
- One commit per phase. Before each: `npm run lint && npm run test && npm run build && npm run test:e2e` in `frontend/`.
- Check each changed page at 375, 768 and 1280px, light and dark, English and Chinese, and the xlarge text size.

## Audit (13 Nov 2026)

Problems seen across the app:

- Every page shows its title twice: in the top bar and again as a large heading.
- Cards inside cards: the Home hero holds a driver card; Home sections hold boxed items; admin pages nest panels. 21 shadows and 3 gradients in the CSS.
- About 20 font sizes (7 tokens plus loose values) and 7 corner radii.
- Two loud colours: logo blue and logo red both act as UI colours, on a cream background with white cards.
- The main action is not the obvious one: "Request a ride" appears twice on Home, the big button at the very bottom; Drive shows about six buttons per ride.

| Page | Main problem |
|---|---|
| Home | Large blue hero with a card inside, 3 tiles, 5 stacked boxed sections, primary button at the bottom |
| My rides | Repeated title, process explained in arrows, status pill and avatar tucked bottom-right |
| Request a ride | Repeated title, back button squeezed beside the heading, each form section boxed |
| Ride detail | Driver card plus details card, heavy plate box |
| Events | Empty progress bars at 0/10; date block and full date text repeat each other |
| Profile | Avatar and name repeat the top bar |
| Drive | Too many buttons, long warning inside the card, extra help text below |
| Admin dashboard | 8 icon tiles above the content |
| Admin sidebar | About 15 links, unevenly centred |
| Login | Fine; spacing only |

## Design system

- **Colour.** White background, no separate card colour. Text `#1A1A1A`, secondary text `#6B6B6B`, borders `#E6E6E6`. **Blue `#1F5DB4` is the only accent** (links and the main button). Logo red stays in the logo only. Green, amber and red appear only as small status text or dots. Dark mode follows the same rules.
- **Type.** Keep the current font. Body sizes 14px (labels and details only), 17px (default), 19px. Headings 20, 24, 30px. The text-size preference keeps working.
- **Spacing.** 4, 8, 16, 24, 32, 48, 64px only.
- **Shapes.** No shadows; 1px borders or dividers. One 8px radius, plus pill ends for chips. Never a card inside a card.
- **Components.** One style each for buttons (main, secondary, text), inputs, list rows, status pills, top bar, bottom nav and sidebar. One main button per page.
- **Motion.** 150ms fades only. No page entrance animation. Respect reduced motion.

## Phases

0. **Bug fixes** — NaN date on Home, missing Phone label, undefined CSS variables, Drive button overflow and plurals, duplicate "waiting for driver" pills, leftover fixed inline styles, Home visual baseline that captured a crash, a date-dependent Drive unit test.
1. **Design system** — rewrite tokens and shared components in `styles.css` and `ui.tsx`.
2. **App frame** — one title per page, tidier sidebar, cleaner bottom nav.
3. **Member pages** — Home, Rides, Request a ride, Ride detail, Events, Event detail and form, Profile, Notifications, Help, Report.
4. **Money pages** — Costs, Settlements, Reimbursements.
5. **Driver** — Drive, trip actions, route.
6. **Admin** — Dashboard, Ride management, Live map, Members, Settings, Announcements, Audit, transport planning board.
7. **Public pages and final checks** — Login, Invite, Terms, 404; retake visual baselines; full viewport, theme, language and text-size sweep.
