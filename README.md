# 恩典同行 Grace Ride

Invite-only, non-commercial ride and activity coordination for PJ State CMC members. Rust/Axum API, PostgreSQL, and a React/TypeScript web application.

## Current documentation

Audited 14 November 2026 against repository HEAD `5659a63` and the local working tree. “Implemented” means present in source; it does not mean deployed or freshly tested.

| Canonical document | Owns |
|---|---|
| [Workflow](docs/WORKFLOW.md) | Member and staff journeys, development process, documentation maintenance |
| [Features by module](docs/FEATURES.md) | Built functionality, original requirement IDs, boundaries and outstanding work |
| [Architecture](docs/ARCHITECTURE.md) | Runtime, code structure, data, migrations and design decisions |
| [Resolved issues](docs/RESOLVED_ISSUES.md) | Previous failures, fixes and regression evidence |
| [Testing](docs/TESTING.md) | Current commands, coverage inventory and verification limits |
| [Deployment](docs/DEPLOYMENT.md) | Local setup, deployment, backups, recovery and monitoring |
| [Security](docs/SECURITY.md) | Access controls, privacy, accepted risks and launch checks |

The [original Chinese proposal](docs/Grace_Ride_Chinese.pdf) is retained as a requirements reference, not a description of everything currently built. The [brand asset guide](docs/brand/grace-ride-logo-kit/README.md) owns logo usage.

## Start locally

Follow [local setup](docs/DEPLOYMENT.md#local-development). Demo data is enabled only with `DEV_MODE=true`; never use this in production. Demo password: `GraceRide2026!`. Accounts include `admin@example.org`, `transport@example.org`, `finance@example.org`, `driver@example.org`, `driver2@example.org`, `member@example.org`, `amina@example.org`, and `sophie@example.org`.

## Documentation policy

Keep one current file per topic above. Update it when behavior changes; use Git history for prior versions instead of timestamped reports, duplicate plans or progress files. Keep proposed features separate from implemented behavior and link claims to source or tests. Never copy old test counts forward as current results.

The November 2026 consolidation replaced root task/progress/blocker/decision files, Cline execution instructions, UI plans, run reports, and the old traceability/test matrix with these canonical documents. Historical tracked content remains in Git. The ignored Cline ZIP is a legacy tooling archive, not active project instructions.
