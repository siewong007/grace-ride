# Deployment runbook

Reviewed against repository configuration on 14 November 2026; no live server or backup was inspected during the documentation audit.

## First deploy (one small VM, e.g. 1 vCPU / 1 GB)

1. Point a DNS A/AAAA record (e.g. `ride.pjstatecmc.org`) at the server. Open ports 80 and 443 only.
2. Install Docker and the Compose plugin, then copy this repository to the server.
3. Create the environment file:
   ```bash
   cp deploy/.env.production.example .env.production
   ```
   Then set `DOMAIN`, `ACME_EMAIL`, `PUBLIC_URL`, `ADMIN_EMAIL`. Never set `DEV_MODE=true`; the app refuses to start with it on a real domain.
4. Start the stack:
   ```bash
   docker compose --env-file .env.production up -d --build
   ```
5. Get the first administrator's one-time setup link and open it on the admin's phone to register a passkey:
   ```bash
   docker compose logs app | grep "one-time link"
   ```
6. Check health:
   ```bash
   curl -s https://ride.pjstatecmc.org/api/health
   ```

## Updates

```bash
git pull   # or copy the new release
docker compose --env-file .env.production up -d --build
```

## AIC cloud (GitHub)

The configured public URL is https://graceride.saliminn.my. A successful CI run for a push to `master` triggers `.github/workflows/deploy.yml`; manual workflow dispatch is also supported. That workflow builds the linux/amd64 image and replaces only the app container on the AIC VPS. The Postgres container and its volume are left as they are. Host Caddy keeps ports 80 and 443.

The production environment needs one secret, `VPS_SSH_PRIVATE_KEY`, the same key hotel-app uses for `root@162.19.81.122` port `20049`. A commit message that contains `[skip ci]` does not deploy. The current CI workflow has no Markdown path exclusion: a Markdown-only push to `master` can run CI and then deploy.

Migrations run automatically at startup and are additive. The app shuts down gracefully on SIGTERM; PostgreSQL checkpoints automatically.

## Backups

- **Nightly snapshot** of the database plus receipts (cron on the host). `DATABASE_URL` comes from the environment; on the server, pass the Docker volume's path as the first argument, or run it where the volume is mounted:
  ```bash
  scripts/backup.sh /var/lib/docker/volumes/graceride_app-data/_data /srv/backups 30
  ```
  The host needs `pg_dump`/`pg_restore` and a database URL reachable from the host. The AIC database is internal to its Compose network; run backup tooling within that network or arrange an authorized connection. Do not assume a container-only hostname resolves from the host. Quiesce app writes if a consistent database-and-upload snapshot is required: the script dumps PostgreSQL and archives uploads sequentially.
  Encrypt the output (e.g. `age` or `gpg`) before it leaves the server.
- **Restore drill**, at least once before launch:
  ```bash
  docker compose stop app
  scripts/restore.sh backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz <volume path>
  docker compose start app
  ```

## Monitoring

- `GET /api/health` queries the database and returns HTTP 200 with `status`, `db` and `version` when healthy, or HTTP 503 with degraded status when the query fails. Point an uptime checker at it (UptimeRobot, Better Stack, etc.).
- Logs are JSON on stdout (`LOG_FORMAT=json`), rotated by Docker (10 MB × 5). Watch for:
  - `rate limited`, `blocked cross-origin request` (attack noise)
  - `background job failed`
  - HTTP 5xx

## Local development

Requires PostgreSQL, Node/npm and Rust. From repository root:

```bash
createdb grace_ride_dev                 # once, if it does not exist
cp backend/.env.example backend/.env
cd backend
cargo run                              # API on 127.0.0.1:8080
```

If using the owner's external-SSD toolchain, replace `cargo run` with `../dev-env.sh cargo run`. In a second terminal, from repository root:

```bash
cd frontend
npm ci
npm run dev                            # localhost:5173, proxies /api
```

Startup runs the numbered migrations and seeds demo accounts only in dev mode. Do not drop a database as a routine update procedure. See [Testing](TESTING.md) for all checks; the Makefile covers only a subset of frontend checks.

## Recovery cautions

The restore script uses `pg_restore --clean --if-exists` against `DATABASE_URL`: select the intended recovery database and stop application writes first. It extracts archived uploads into the supplied directory and does not remove extra existing files. For a clean restore drill, use an empty upload destination and a disposable database, then check records, file access and health before reopening traffic. Preserve a current backup before replacing an existing database.

The deployment workflow can restore the previous app image after failed rollout checks; that does not reverse database migrations. Design additive migrations for compatibility and keep a separate database recovery plan.
