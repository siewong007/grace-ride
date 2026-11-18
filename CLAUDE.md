# Grace Ride Technical Standards

## Stack
- Backend: Rust (Axum), SQLx (PostgreSQL 18), Tokio.
- Frontend: React 18, TypeScript, Vite, i18next, Leaflet.

## Naming Conventions
- Rust: `snake_case` for functions, variables, modules. `PascalCase` for types, enums, structs.
- TypeScript: `camelCase` for functions, variables. `PascalCase` for components, types.
- Database: `snake_case` tables, columns. Plural table names.

## Project Structure
- `backend/`: Axum API, logic, migrations.
- `frontend/`: Vite React app.
- `docs/`: Technical manual, audit, progress logs.

## Commands
- `make dev-api`: Start API in dev mode.
- `make dev-web`: Start Vite dev server.
- `make lint`: Run all linters.
- `make test`: Run all tests.
- `make build`: Build everything for production.
