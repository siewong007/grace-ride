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
| **Polling, not WebSockets** | Chat polls every 4 s, the map every 15 s, notifications every 30 s, and only while the tab is visible. At 10 live trips that is trivial load and survives flaky mobile networks. | Chat volume grows, or push notifications arrive (see roadmap). |
| **In-process job loop** | Reminders and cleanup are idempotent (`reminder_sent` flags) and tolerate a missed minute. | Multiple instances (then use a leader lock or an external scheduler). |
| **OpenStreetMap tiles** | Free, no API key, no billing exposure. | Heavy traffic. OSM's tile policy expects a modest volume; switch to a commercial tile provider then. |

## Code layout

```
backend/
  src/lib.rs        router() + init(): everything tests reuse
  src/main.rs       config, logging, graceful shutdown, `healthcheck` subcommand
  src/state.rs      Config (validated at startup) + AppState
  src/security.rs   headers, Origin guard, rate limiter, client IP
  src/auth.rs       session extractor (hashed tokens), role model
  src/jobs.rs       reminders, unmatched-ride escalation, retention
  src/routes/*.rs   one module per domain
  migrations/       0001_init.sql (single consolidated schema)
  tests/api.rs      end-to-end tests (privacy, RBAC, CSRF, lockout, uploads, jobs)
frontend/
  src/App.tsx       routes; every page except Login/Home is lazy-loaded
  src/components/AppChrome.tsx  error boundary, offline banner, skip link, display prefs
  src/prefs.ts      大字模式 / theme (per device)
  src/i18n.ts, src/locales/     zh / en strings
```

## Domain state machine (rides)

```
pending_driver ──claim──▶ pending_passenger ──accept──▶ pending_approval ──approve──▶ confirmed ──start──▶ in_progress ──complete──▶ completed
      ▲                         │ decline                     │ return                     │                    └──no-show──▶ no_show
      └─────────────────────────┴─────────────────────────────┘                            │
      ▲  driver withdraws (needs_attention)                                                │
      └────────────────────────────────────────────────────────────────────────────────────┘
 staff reassign ──▶ pending_driver_confirm ──driver confirms──▶ confirmed
 staff Grab fallback ──▶ alt_transport          any open state ──cancel──▶ cancelled
```

Every transition is a single conditional `UPDATE … WHERE status IN (…)`. Two drivers claiming the same request cannot both win, and a stale screen gets `409`.

## Roadmap (not built yet)

1. **Web Push notifications** (VAPID). Today, notifications appear only when the app is open.
2. **Postgres option**, once more than one instance or high availability is required.
3. **TNG / DuitNow payout API**, once Touch 'n Go confirms eligibility and terms (spec page 20).
4. **Service worker** for an offline app shell and faster repeat loads.
