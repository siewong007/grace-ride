# Architecture

Audited 14 November 2026. Source, rather than old implementation plans, defines the current system.

## Runtime

```text
Browser: React 18 + TypeScript + Vite, Leaflet, bilingual UI
  -> HTTPS / Caddy
     -> Axum Rust application
        /api/* domain routes + authenticated SSE stream
        /* compiled SPA in production
        -> PostgreSQL (SQLx pool and startup migrations)
        -> uploaded files on persistent storage
        -> Photon / Nominatim / OSRM for address and route assistance
        -> in-process recurring jobs for reminders, escalation and retention
```

The generic Compose stack includes Caddy. The AIC deployment uses host Caddy and an app bound to a loopback port. See [Deployment](DEPLOYMENT.md). Backups are operator-scheduled; the app does not itself encrypt or upload them off-site.

## Code ownership

| Path | Responsibility |
|---|---|
| `backend/src/main.rs`, `state.rs` | Startup, configuration validation, logging, graceful shutdown, WebAuthn setup |
| `backend/src/lib.rs` | Database connection/migrations, initialization, router and middleware |
| `backend/src/auth.rs`, `perms.rs`, `security.rs` | Cookie sessions, permission catalog, request guards, rate limits and headers |
| `backend/src/routes/` | Auth, rides, events, plan, settlements, files, community, admin and roles |
| `backend/src/audience.rs`, `routes/visitors.rs` | Who an event is for (group tree), joining-as choices, driver matching; visitor passes |
| `backend/src/arrange.rs`, `routing.rs` | Staff planning suggestions and geocoding/route computation |
| `backend/src/jobs.rs`, `stream.rs` | Recurring work and SSE notifications |
| `backend/src/i18n/`, `error.rs`, `util.rs` | Localized messages, errors, shared audit/notification helpers |
| `backend/migrations/0001_init.sql` | The PostgreSQL schema (one script) |
| `frontend/src/App.tsx`, `auth.tsx`, `perms.ts` | Routes, authentication and client permission gates |
| `frontend/src/pages/`, `components/` | Member/staff pages and shared UI, maps/chat/auto-lock |
| `frontend/src/styles.css`, `prefs.ts`, `locales/` | Design tokens, display preferences and Chinese/English strings |
| `backend/tests/`, `frontend/src/**/*.test.*`, `frontend/e2e/` | API/integration, frontend unit and browser checks |

## Design decisions

- One API process with PostgreSQL keeps transactions and deployment straightforward for the proposed thousand-member community. PostgreSQL is already required, not a future option.
- Production serves SPA and API on one origin. Vite proxies API requests in development. Session cookies are the authentication channel; server handlers enforce access regardless of UI visibility.
- SSE signals changes and polling refreshes data. This avoids a bidirectional WebSocket service. Browser visibility and connection state affect refresh behavior; no fixed latency guarantee is made.
- Rate limits, route caches and SSE fan-out are process-local. Multiple API instances would require shared coordination, including a strategy for recurring jobs.
- Leaflet displays OSM maps. Address autocomplete is proxied through Photon; geocoding uses Nominatim and road routing uses OSRM by default. Results are cached. Google Maps/Waze links hand off navigation; computed ETAs are not live-traffic estimates. Providers can be configured/self-hosted; disabling routing disables address autocomplete.
- Event audiences extend `event_groups` with a group tree (`groups.parent_id`): an event for a group reaches it and every group below it, and every visibility check goes through `audience::visible_events_sql`. Signups and event rides record `as_group_id`; drivers match on it (or the host's group for a visitor's ride, `audience::MATCH_GROUP_SQL`). Visitors get a scoped, expiring pass rather than an account. See `docs/superpowers/specs/2026-11-16-event-audience-rides-design.md`.
- The UI uses white/neutral surfaces, logo-blue primary actions, restrained status colors, dark mode and adjustable text. CSS tokens and shared primitives are authoritative. The earlier teal UI plan is obsolete.

## Storage and migrations

SQLx applies the files in `backend/migrations/` at startup (`lib.rs::connect_db_with`), on a connection of its own with no statement timeout. The request pool is opened afterwards with the `DB_*` settings, and every pool connection runs `SET TIME ZONE <APP_TIMEZONE>`, a `statement_timeout` and an idle-in-transaction timeout.

- `0001_init.sql` is the whole schema, squashed on 2026-11-17 from the earlier 0001–0005, and is **frozen**: a test (`the_initial_schema_is_frozen`) fails if it changes.
- Every schema change from now on is a new numbered file (`0002_…`) that runs once.
- A database whose recorded version 1 is a different file (any older schema: the pre-November numbered files, the first frozen 0001, or 0001–0005) has its history cleared and `0001_init.sql` runs again over the existing tables. That is why 0001 is fully repeatable (`IF NOT EXISTS`, guarded changes, no `DROP COLUMN`, `TRUNCATE` or unguarded `DELETE`) and keeps every row.
- If version 1 matches but a later file differs (edited after it ran, or shipped by a newer build), start-up stops with an error rather than replaying anything (`lib.rs::classify_history`).
- New `CHECK` rules go in `NOT VALID` and are then validated; if existing rows break one, the migration logs a warning and leaves it unvalidated (new rows are still checked) instead of failing the deploy. List those with `SELECT conrelid::regclass, conname FROM pg_constraint WHERE NOT convalidated;`.

Conventions (see `0001_init.sql`): times are local text `YYYY-MM-DDTHH:MM[:SS]` in the app's zone (`APP_TIMEZONE`, default Asia/Kuala_Lumpur). Rust reads the clock through `util::local_now()`/`util::now()`, never the host's `TZ`. The app copies the zone into `system_settings.timezone`, and the database's own defaults call `app_now()`, which writes in that zone. Pass time cut-offs from Rust as parameters rather than using `LOCALTIMESTAMP` in SQL. Emails are stored lowercase (a `CHECK` enforces it), so lookups lowercase what was typed. Money is integer cents, most flags are `BIGINT` 0/1 (checked), and role lists are comma-separated on `users.roles`, which is still the column the app writes. The database copies the names that are real roles into `member_roles` (one row per member per role), and permission checks read that table: a name that isn't a role grants nothing, deleting a role removes it from every member, and a role created later attaches to members who already list it. The column stays because migrations must not `DROP COLUMN`. JSON lives in `JSONB` columns (`guide_workflows.steps`, `settlement_revisions.snapshot` and `invites.group_ids`): bind `$n::jsonb` on write and read `col::text`. A trip is a row in `trips` (its id is the ride that started it): `rides.trip_id` and `ride_messages.trip_id` point to it, a trigger adds the row when a ride is given a trip, deleting a trip removes its chat, and the clean-up (every `HOUSEKEEPING_INTERVAL_MINS`, default hourly) drops trips with no rides and no chat left. Reminders and escalations run every minute and claim their rows with `UPDATE … RETURNING`, so each is sent once. Demo members stay in `seed::run`. Uploads and PostgreSQL data are separate persistent stores and both must be included in recovery planning.

## Ride states

```text
pending_driver -> pending_passenger -> pending_approval -> confirmed
confirmed -> in_progress -> completed
staff reassignment / planning -> pending_driver_confirm -> confirmed
other outcomes: cancelled, no_show, alt_transport
```

Declines/returns can send requests back to driver matching. Withdrawal clears assignment and may flag attention. `needs_attention` is a flag, not a separate status; readiness and delay are additional fields. Legal source states are checked by each action in `routes/rides.rs`. Conditional updates prevent stale claims; planning applies a whole reviewed batch transactionally. See [Workflow](WORKFLOW.md) for the user journey.

## Lists, streams and exports

Numbered endpoints accept `page`/`limit` and return an envelope with items and totals; some retain array responses when pagination is omitted. Keyset feeds return `items` and `next_before`. Endpoint-specific timestamp/ID cursors and composite indexes support growing lists. Frontend primitives provide pagination, intersection-observer loading, back-to-top and optional virtualization; these are implementation choices, not guarantees of zero CPU or a particular frame rate.

Client CSV/PDF export and server audit export serve different views. Preserve each endpoint's permission filtering when adding an export. The database is PostgreSQL; old SQLite portability claims are not current architecture.

## Limits and future work

See [Features](FEATURES.md#boundaries-and-outstanding-work) for product gaps and [Security](SECURITY.md) for privacy/trust boundaries. Infrastructure changes such as shared caches, distributed scheduling or a different map provider should follow measured needs, not be described as already implemented.
