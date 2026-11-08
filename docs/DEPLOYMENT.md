# Deployment runbook

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

Migrations run automatically at startup and are additive. The app shuts down gracefully on SIGTERM; PostgreSQL checkpoints automatically.

## Backups

- **Nightly snapshot** of the database plus receipts (cron on the host). `DATABASE_URL` comes from the environment; on the server, pass the Docker volume's path as the first argument, or run it where the volume is mounted:
  ```bash
  scripts/backup.sh /var/lib/docker/volumes/graceride_app-data/_data /srv/backups 30
  ```
  Or restore in-place consistency by exec'ing into the stack's Postgres instead.
  Encrypt the output (e.g. `age` or `gpg`) before it leaves the server.
- **Restore drill**, at least once before launch:
  ```bash
  docker compose stop app
  scripts/restore.sh backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz <volume path>
  docker compose start app
  ```

## Monitoring

- `GET /api/health` returns `200 {"status":"ok"}`. Point an uptime checker at it (UptimeRobot, Better Stack, etc.).
- Logs are JSON on stdout (`LOG_FORMAT=json`), rotated by Docker (10 MB × 5). Watch for:
  - `rate limited`, `blocked cross-origin request` (attack noise)
  - `background job failed`
  - HTTP 5xx

## Local development

```bash
make dev-api     # API with demo data on :8080 (DEV_MODE=true, localhost only)
make dev-web     # Vite on :5173, proxies /api
make lint test audit
```
