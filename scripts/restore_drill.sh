#!/usr/bin/env bash
# Non-destructive restore drill: restores a backup archive into a disposable database
# and temporary directory, reconciles `files.stored_name` rows against extracted files
# on disk, reports core table counts, and cleans everything up on exit.
#
# Usage: scripts/restore_drill.sh <ARCHIVE_PATH>
#
# Connection (set one, pointing at the Postgres server — production DB is NEVER modified):
#   ADMIN_DATABASE_URL=postgres://localhost/postgres
#   PG_CONTAINER=grace-ride-db   (uses POSTGRES_USER=grace)
set -Eeuo pipefail

ARCHIVE="${1:?Usage: scripts/restore_drill.sh <ARCHIVE_PATH>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DRILL_DB="grace_ride_drill_$(date +%s)_$$"
TMP_DATA="$(mktemp -d)"
chmod 0700 "$TMP_DATA"
PGPASS_TMP=""
SAFE_PG_URL=""

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

run_sql() {
  local db="$1"
  local sql="$2"
  if [[ -n "${ADMIN_DATABASE_URL:-}" ]]; then
    prepare_pgpass_url "$ADMIN_DATABASE_URL"
    local suffix=""
    local base_part="$SAFE_PG_URL"
    if [[ "$SAFE_PG_URL" == *\?* ]]; then
      suffix="?${SAFE_PG_URL#*\?}"
      base_part="${SAFE_PG_URL%%\?*}"
    fi
    local base="${base_part%/*}"
    psql "${base}/${db}${suffix}" -v ON_ERROR_STOP=1 -tAq -c "$sql"
  elif [[ -n "${PG_CONTAINER:-}" ]]; then
    local user="${POSTGRES_USER:-grace}"
    docker exec "$PG_CONTAINER" psql -U "$user" -d "$db" -v ON_ERROR_STOP=1 -tAq -c "$sql"
  else
    echo "set ADMIN_DATABASE_URL (e.g. postgres://localhost/postgres) or PG_CONTAINER (e.g. grace-ride-db)" >&2
    exit 1
  fi
}

cleanup() {
  rm -rf "$TMP_DATA"
  if [[ -n "${ADMIN_DATABASE_URL:-}" ]]; then
    prepare_pgpass_url "$ADMIN_DATABASE_URL"
    psql "$SAFE_PG_URL" -tAq -c "DROP DATABASE IF EXISTS \"${DRILL_DB}\" WITH (FORCE);" >/dev/null 2>&1 || true
  elif [[ -n "${PG_CONTAINER:-}" ]]; then
    docker exec "$PG_CONTAINER" psql -U "${POSTGRES_USER:-grace}" -d postgres -tAq -c "DROP DATABASE IF EXISTS \"${DRILL_DB}\" WITH (FORCE);" >/dev/null 2>&1 || true
  fi
  if [[ -n "$PGPASS_TMP" ]]; then
    rm -f "$PGPASS_TMP"
  fi
}
trap cleanup EXIT

echo "== Creating disposable drill database: ${DRILL_DB} =="
if [[ -n "${ADMIN_DATABASE_URL:-}" ]]; then
  prepare_pgpass_url "$ADMIN_DATABASE_URL"
  psql "$SAFE_PG_URL" -v ON_ERROR_STOP=1 -tAq -c "CREATE DATABASE \"${DRILL_DB}\";"
  URL_SUFFIX=""
  URL_BASE_PART="$ADMIN_DATABASE_URL"
  if [[ "$ADMIN_DATABASE_URL" == *\?* ]]; then
    URL_SUFFIX="?${ADMIN_DATABASE_URL#*\?}"
    URL_BASE_PART="${ADMIN_DATABASE_URL%%\?*}"
  fi
  BASE_URL="${URL_BASE_PART%/*}"
  env -u PG_CONTAINER -u DOCKER_DB_CONTAINER DATABASE_URL="${BASE_URL}/${DRILL_DB}${URL_SUFFIX}" "$SCRIPT_DIR/restore.sh" "$ARCHIVE" "$TMP_DATA"
elif [[ -n "${PG_CONTAINER:-}" ]]; then
  docker exec "$PG_CONTAINER" psql -U "${POSTGRES_USER:-grace}" -d postgres -v ON_ERROR_STOP=1 -tAq -c "CREATE DATABASE \"${DRILL_DB}\";"
  env -u DATABASE_URL PG_CONTAINER="$PG_CONTAINER" POSTGRES_DB="$DRILL_DB" "$SCRIPT_DIR/restore.sh" "$ARCHIVE" "$TMP_DATA"
else
  echo "set ADMIN_DATABASE_URL or PG_CONTAINER" >&2
  exit 1
fi

SCHEMA="${DRILL_SCHEMA:-public}"
[[ "$SCHEMA" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || { echo "Invalid DRILL_SCHEMA: $SCHEMA" >&2; exit 1; }
users_n="$(run_sql "$DRILL_DB" "SELECT COUNT(*) FROM \"${SCHEMA}\".users")"
events_n="$(run_sql "$DRILL_DB" "SELECT COUNT(*) FROM \"${SCHEMA}\".events")"
rides_n="$(run_sql "$DRILL_DB" "SELECT COUNT(*) FROM \"${SCHEMA}\".rides")"
files_n="$(run_sql "$DRILL_DB" "SELECT COUNT(*) FROM \"${SCHEMA}\".files")"

missing=0
while IFS= read -r stored; do
  [[ -z "$stored" ]] && continue
  if [[ "$stored" == /* || "$stored" == *"/"* || "$stored" == *".."* || "$stored" == "." ]]; then
    echo "UNSAFE stored_name IN DB: $stored" >&2
    exit 1
  fi
  if [[ ! -f "$TMP_DATA/uploads/$stored" ]]; then
    echo "MISSING UPLOAD FILE ON DISK: $stored" >&2
    missing=$((missing + 1))
  fi
done < <(run_sql "$DRILL_DB" "SELECT stored_name FROM \"${SCHEMA}\".files")

disk_files="$(find "$TMP_DATA/uploads" -maxdepth 1 -type f | wc -l | tr -d ' ')"

echo "== Restore Drill Summary =="
echo "archive:     $ARCHIVE"
echo "schema:      $SCHEMA"
echo "users:       $users_n"
echo "events:      $events_n"
echo "rides:       $rides_n"
echo "files (DB):  $files_n"
echo "files (disk):$disk_files"
echo "missing:     $missing"

if (( missing > 0 )); then
  echo "DRILL FAILED: $missing DB file row(s) missing from extracted uploads/" >&2
  exit 1
fi

echo "== RESTORE DRILL PASSED =="
