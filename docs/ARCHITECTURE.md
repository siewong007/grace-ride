# Architecture

## Shape

```
 Phone / tablet / desktop browser  (React 18 SPA, PWA manifest)
            │  HTTPS (TLS 1.2+/HTTP3, automatic certificates)
            ▼
 ┌──────────────────────┐
 │ Caddy (reverse proxy) │  compression, ACME, sets X-Forwarded-For
 └──────────┬───────────┘
            ▼
 ┌──────────────────────────────────────────────────────────────┐
 │ grace-ride (one Rust binary, Axum)                           │
 │  middleware: trace → security headers → compression →        │
 │              body limit → Origin guard → rate limit          │
 │  /api/*   routes: auth · rides · events · settlements ·      │
 │           files · community · admin · health                  │
 │  /*       built React app (same origin → no CORS)            │
 │  jobs     every 60 s: reminders, escalation, retention       │
 └──────────┬─────────────────────────────┬─────────────────────┘
            ▼                             ▼
   PostgreSQL /data (pg-data volume)  /data/uploads (receipts, photos)
            │
            └── nightly pg_dump → encrypted, off-site storage (scripts/backup.sh)
```

## Why these choices

| Decision | Reason | Revisit when |
|---|---|---|
| **Single binary + PostgreSQL** | About 1,000 members, at most 10 live drivers, low write rate. Transactions and indexes from one mature engine; nothing exotic to operate. | More than one app instance is needed, or writes exceed ~100/s — then add Redis for the rate limiter. |
| **Same-origin SPA served by the API** | No CORS. Session cookies stay `SameSite=Lax` + HttpOnly. One deployable. | — |
| **Polling + SSE, not WebSockets** | Chat, the map and notifications poll (4 s / 15 s / 20 s, only while the tab is visible). `/api/stream` pushes a "something changed for you" tick over SSE, so chat and ride-status changes appear immediately. One-way push is all the app needs; polling remains the fallback, so a dropped stream degrades quietly and flaky mobile networks are not a problem. | Chat volume grows enough that polling load matters, or push notifications arrive (see roadmap). |
| **In-process job loop** | Reminders and cleanup are idempotent (`reminder_sent` flags) and tolerate a missed minute. | Multiple instances (then use a leader lock or an external scheduler). |
| **OpenStreetMap tiles** | Free, no API key, no billing exposure. | Heavy traffic. OSM's tile policy expects a modest volume; switch to a commercial tile provider then. |
| **Pickup routing on OSRM + Nominatim, navigation handed to Google Maps / Waze** | `src/routing.rs` picks the best pickup order (exact up to 8 stops), road distance and ETAs for a driver's trip; the driver taps through to Google Maps (all stops as waypoints) or Waze (one stop at a time; its links take a single destination) for live-traffic turn-by-turn. Free and keyless like the tiles. Routes are computed on the server and cached per trip (30 s while driving), so one computation serves the driver and every passenger; addresses are geocoded once and cached. Address fields autocomplete from Photon (`/api/places`, proxied and cached), since Nominatim's usage policy forbids search-as-you-type; each suggestion's coordinates go into `geocode_cache`, so a picked address routes to that exact point. ETAs ignore live traffic, and some street addresses only match at street level or not at all, in which case the passenger's shared GPS is used. Google's Routes/Geocoding terms forbid showing their results on a non-Google map, so switching to Google means replacing the Leaflet map too. | The public OSRM/Nominatim servers get slow or refuse us (self-host via `OSRM_URL` / `NOMINATIM_URL` / `PHOTON_URL`), or traffic-aware ETAs become a must. |

## Code layout

```
backend/
  src/lib.rs        router() + init(): everything tests reuse
  src/main.rs       config, logging, graceful shutdown, `healthcheck` subcommand
  src/state.rs      Config (validated at startup) + AppState
  src/security.rs   headers, Origin guard, rate limiter, client IP
  src/auth.rs       session extractor (hashed tokens); loads the member's permissions
  src/perms.rs      permission catalog (page × view/create/update/delete), built-in role defaults
  src/jobs.rs       reminders, unmatched-ride escalation, retention
  src/routes/*.rs   one module per domain
  migrations/       0001_init.sql ONLY — single consolidated schema (see note below)
  tests/api.rs      end-to-end tests (privacy, RBAC, CSRF, lockout, uploads, jobs)
frontend/
  src/App.tsx       routes; every page except Login/Home is lazy-loaded
  src/components/AppChrome.tsx  error boundary, offline banner, skip link, display prefs
  src/prefs.ts      大字模式 / theme (per device)
  src/i18n.ts, src/locales/     zh / en strings
```


## Schema / migrations

Grace Ride keeps **one** SQLx migration file: `backend/migrations/0001_init.sql`.

- **While the app is brand-new / no irreplaceable prod data:** edit `0001_init.sql` in place, then wipe/recreate the Postgres volume (or re-baseline `_sqlx_migrations`) before deploy. Rewriting `0001` changes its checksum; sqlx will refuse to start on a DB that already applied an older version of the same file.
- **Once real member data exists:** stop rewriting `0001`. Add forward-only `0002_*.sql` migrations instead, and never edit applied files.

## Domain state machine (rides)

```
pending_driver ──claim──▶ pending_passenger ──accept──▶ pending_approval ──approve──▶ confirmed ──start──▶ in_progress ──complete──▶ completed
      ▲                         │ decline                     │ return                     │                    └──no-show──▶ no_show
      └─────────────────────────┴─────────────────────────────┘                            │
      ▲  driver withdraws (needs_attention)                                                │
      └────────────────────────────────────────────────────────────────────────────────────┘
 staff reassign / event plan board ──▶ pending_driver_confirm ──driver confirms──▶ confirmed
 staff Grab fallback ──▶ alt_transport          any open state ──cancel──▶ cancelled
```

Every transition is a single conditional `UPDATE … WHERE status IN (…)`. The event plan board (`routes/plan.rs`) applies a whole batch in one transaction, each row re-checking the status and driver the board showed, so a stale board changes nothing. Its Auto-arrange (`src/arrange.rs`) only suggests: it fills passengers without a driver (pinned ones stay), within seats and the one-booking rule, preferring nearby carmates (straight-line distance from `geocode_cache`, or the pickup area when an address isn't located yet), drivers who serve the area (specialists first) and previous drivers, then improves with moves and swaps. Two drivers claiming the same request cannot both win, and a stale screen gets `409`.

## Data Pagination & Streams Architecture

To prevent silent data truncation when church records exceed hardcoded SQL limits as congregations grow:

1. **Numbered Table Pagination** (`/admin/rides`, `/admin/members`, `/admin/reimbursements`, `/settlements`):
   - **Backend API**: Endpoints accept `page` (1-based) and `limit` query parameters. When provided, endpoints return a metadata envelope `{ items, total, page, page_size, total_pages }`. When omitted, legacy plain arrays `Vec<Value>` are returned to preserve 100% backward compatibility with legacy consumers and unit test fixtures.
   - **Frontend `<Pagination />`**: Includes dynamic record range counters (`Showing 1–25 of 148 records`), Jump-to-Page quick input for `totalPages > 5`, screen reader announcements via `.sr-only` `aria-live="polite"`, smart ellipses, and clean URL state synchronization (`useSearchParams`) that preserves browser Back/Forward navigation.

2. **Keyset Cursor Infinite Streams** (`/notifications`, `/rides?scope=past`, `/events?scope=past`, `/admin/audit`):
   - **Backend API**: Cursor-based using `before` timestamp or ID, returning `{ items, next_before }` to eliminate `OFFSET` penalties on large datasets and prevent row skipping when new items arrive concurrently.
   - **Frontend `<InfiniteSentinel />` & `<BackToTop />`**: Uses pure `IntersectionObserver` sentinels with zero scroll event listeners, guaranteeing 60fps scrolling and zero idle CPU usage. Includes an accessible `<BackToTop />` floating pill that unmounts when near the top of the viewport.
   - **Virtualization Support**: Heavy or dense feeds can render via `<VirtualList />` leveraging `@tanstack/react-virtual` with dynamic measurement and SSR/test fallback support.

3. **Composite Database Indexes** (`backend/migrations/0007_keyset_pagination_indexes.sql`):
   - Composite B-Tree indexes on `rides(pickup_at DESC, id DESC)`, `notifications(user_id, id DESC)`, `events(starts_at DESC, id DESC)`, `audit_logs(id DESC)`, and `reimbursements(user_id, id DESC)` / `reimbursements(status, id DESC)` for fast range scans in both SQLite and PostgreSQL.

4. **Data Export**:
   - Tabular views include a one-click client-side CSV export (`exportToCsv`) with RFC 4180 escaping for church finance and committee recordkeeping.

## Roadmap (not built yet)

1. **Web Push notifications** (VAPID). Today, notifications appear only when the app is open.
2. **Postgres option**, once more than one instance or high availability is required.
3. **TNG / DuitNow payout API**, once Touch 'n Go confirms eligibility and terms (spec page 20).
4. **Service worker** for an offline app shell and faster repeat loads.
