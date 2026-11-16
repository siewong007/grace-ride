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

SQLx applies `backend/migrations/0001_init.sql` at startup (`lib.rs::connect_db`). That file is the whole schema: tables, constraints, indexes, reimbursement rates, and system settings. Demo members stay in `seed::run`.

A database that still has the old numbered history (`0002` through `0016`), or a checksum that does not match this file, has `_sqlx_migrations` cleared and then this script runs. The script is idempotent, so existing rows stay. Do not put `DROP COLUMN`, `TRUNCATE`, or an unguarded `DELETE` in it: a checksum change re-runs the whole file. Uploads and PostgreSQL data are separate persistent stores and both must be included in recovery planning.

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
