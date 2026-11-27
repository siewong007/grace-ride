#!/usr/bin/env bash
# Consistent snapshot of the database + uploaded receipts, with optional age/GPG
# encryption and retention rotation.
#
# Usage: scripts/backup.sh [UPLOAD_DATA_DIR] [BACKUP_DIR] [KEEP_DAYS]
#
# Database connection (set one):
#   DATABASE_URL=postgres://grace:pw@localhost/grace_ride
#   PG_CONTAINER=grace-ride-db        (alias: DOCKER_DB_CONTAINER; uses `docker exec` with
#                                      POSTGRES_USER=grace, POSTGRES_DB=grace_ride)
#
# Optional dump scope:
#   BACKUP_SCHEMA=<schema_name>       (passes `--schema <schema_name>` to pg_dump; used by test suites)
#
# Encryption at rest (strongly recommended in production; set REQUIRE_ENCRYPTION=true to enforce):
#   BACKUP_AGE_RECIPIENT=age1...            -> writes .tar.gz.age via `age`
#   BACKUP_GPG_RECIPIENT=ops@pjstatecmc.org -> writes .tar.gz.gpg via `gpg`
set -Eeuo pipefail

DATA_DIR="${1:-./backend/data}"
BACKUP_DIR="${2:-./backups}"
KEEP_DAYS="${3:-30}"
[[ "$KEEP_DAYS" =~ ^[0-9]+$ ]] || { echo "KEEP_DAYS must be a non-negative integer" >&2; exit 1; }
STAMP="$(date +%Y%m%d-%H%M%S)"
PG_CONTAINER="${PG_CONTAINER:-${DOCKER_DB_CONTAINER:-}}"

if [[ "${REQUIRE_ENCRYPTION:-true}" == "true" && -z "${BACKUP_AGE_RECIPIENT:-}" && -z "${BACKUP_GPG_RECIPIENT:-}" ]]; then
  echo "REQUIRE_ENCRYPTION=true but neither BACKUP_AGE_RECIPIENT nor BACKUP_GPG_RECIPIENT is set" >&2
  exit 1
fi

umask 077
mkdir -p "$BACKUP_DIR" "$DATA_DIR/uploads"
STAGE_DIR="$(mktemp -d)"
chmod 0700 "$STAGE_DIR"
DEST="$STAGE_DIR/grace-ride-$STAMP"
mkdir -p "$DEST"
PGPASS_TMP=""
SAFE_PG_URL=""
ARCHIVE="$BACKUP_DIR/grace-ride-$STAMP.tar.gz"
PLAIN_ARCHIVE="$STAGE_DIR/grace-ride-$STAMP.tar.gz"
SUCCESS=0
cleanup() {
  rm -rf "$STAGE_DIR"
  if [[ -n "$PGPASS_TMP" ]]; then
    rm -f "$PGPASS_TMP"
  fi
  if (( SUCCESS == 0 )); then
    rm -f "$ARCHIVE" "${ARCHIVE}.age" "${ARCHIVE}.gpg"
  fi
}
trap cleanup EXIT

prepare_pgpass_url() {
  local raw="$1"
  if [[ "$raw" =~ ^(postgres(ql)?://)([^:/@?#]+):([^@/?#]+)@(.+)$ ]]; then
    local scheme="${BASH_REMATCH[1]}"
    local user="${BASH_REMATCH[3]}"
    local pass="${BASH_REMATCH[4]}"
    local rest="${BASH_REMATCH[5]}"
    local esc_user="${user//\\/\\\\}"
    esc_user="${esc_user//:/\\:}"
    local esc_pass="${pass//\\/\\\\}"
    esc_pass="${esc_pass//:/\\:}"
    if [[ -z "$PGPASS_TMP" ]]; then
      PGPASS_TMP="$(mktemp)"
      chmod 0600 "$PGPASS_TMP"
    fi
    printf '*:*:*:%s:%s\n' "$esc_user" "$esc_pass" > "$PGPASS_TMP"
    export PGPASSFILE="$PGPASS_TMP"
    SAFE_PG_URL="${scheme}${user}@${rest}"
  else
    SAFE_PG_URL="$raw"
  fi
}

pg_dump_args=(--format=custom --no-owner)
if [[ -n "${BACKUP_SCHEMA:-}" ]]; then
  pg_dump_args+=(--schema "$BACKUP_SCHEMA")
fi

if [[ -n "${DATABASE_URL:-}" ]]; then
  prepare_pgpass_url "$DATABASE_URL"
  pg_dump "${pg_dump_args[@]}" --file "$DEST/db.dump" "$SAFE_PG_URL"
  test -s "$DEST/db.dump" || { echo "pg_dump produced an empty file" >&2; exit 1; }
  pg_restore --list "$DEST/db.dump" >/dev/null || { echo "dump sanity check failed" >&2; exit 1; }
elif [[ -n "${PG_CONTAINER:-}" ]]; then
  PG_USER="${POSTGRES_USER:-grace}"
  PG_DB="${POSTGRES_DB:-grace_ride}"
  docker exec -i "$PG_CONTAINER" pg_dump -U "$PG_USER" "${pg_dump_args[@]}" "$PG_DB" > "$DEST/db.dump"
  test -s "$DEST/db.dump" || { echo "pg_dump produced an empty file" >&2; exit 1; }
  docker exec -i "$PG_CONTAINER" pg_restore --list < "$DEST/db.dump" >/dev/null || { echo "dump sanity check failed" >&2; exit 1; }
else
  echo "set DATABASE_URL (postgres://user:pass@host/dbname) or PG_CONTAINER / DOCKER_DB_CONTAINER (e.g. grace-ride-db)" >&2
  exit 1
fi

tar -C "$DATA_DIR" -czf "$DEST/uploads.tar.gz" uploads
tar -C "$STAGE_DIR" -czf "$PLAIN_ARCHIVE" "grace-ride-$STAMP"
rm -rf "$DEST"

FINAL_ARCHIVE="$ARCHIVE"
if [[ -n "${BACKUP_AGE_RECIPIENT:-}" ]]; then
  age -r "$BACKUP_AGE_RECIPIENT" -o "$STAGE_DIR/backup.age" "$PLAIN_ARCHIVE"
  rm -f "$PLAIN_ARCHIVE"
  mv -f "$STAGE_DIR/backup.age" "${ARCHIVE}.age"
  FINAL_ARCHIVE="${ARCHIVE}.age"
elif [[ -n "${BACKUP_GPG_RECIPIENT:-}" ]]; then
  gpg --batch --yes --encrypt -r "$BACKUP_GPG_RECIPIENT" -o "$STAGE_DIR/backup.gpg" "$PLAIN_ARCHIVE"
  rm -f "$PLAIN_ARCHIVE"
  mv -f "$STAGE_DIR/backup.gpg" "${ARCHIVE}.gpg"
  FINAL_ARCHIVE="${ARCHIVE}.gpg"
else
  mv -f "$PLAIN_ARCHIVE" "$ARCHIVE"
fi

SUCCESS=1
find "$BACKUP_DIR" -maxdepth 1 \( -name 'grace-ride-*.tar.gz' -o -name 'grace-ride-*.tar.gz.age' -o -name 'grace-ride-*.tar.gz.gpg' \) -mtime +"$KEEP_DAYS" -delete
echo "Backup written: $FINAL_ARCHIVE"
# Contains personal data (addresses, receipts): store encrypted and off-site.
