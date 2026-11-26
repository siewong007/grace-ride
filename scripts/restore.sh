#!/usr/bin/env bash
# Restore a backup made by scripts/backup.sh (.tar.gz, .tar.gz.age, or .tar.gz.gpg).
# STOP THE APP FIRST when restoring over production.
#
# Usage: scripts/restore.sh <ARCHIVE_PATH> [UPLOAD_DATA_DIR]
#
# Database connection (set one):
#   DATABASE_URL=postgres://grace:pw@localhost/grace_ride
#   PG_CONTAINER=grace-ride-db   (alias: DOCKER_DB_CONTAINER; uses `docker exec` with
#                                 POSTGRES_USER=grace, POSTGRES_DB=grace_ride)
#
# Decryption (when restoring .age or .gpg archives):
#   BACKUP_AGE_KEY_FILE=/root/.config/grace-ride/backup-age.key
set -Eeuo pipefail

ARCHIVE="${1:-}"
if [[ -z "$ARCHIVE" || ! -f "$ARCHIVE" ]]; then
  echo "Usage: $0 <ARCHIVE_PATH> [UPLOAD_DATA_DIR]" >&2
  exit 1
fi

DATA_DIR="${2:-./backend/data}"
PG_CONTAINER="${PG_CONTAINER:-${DOCKER_DB_CONTAINER:-}}"

if [[ -z "${DATABASE_URL:-}" && -z "${PG_CONTAINER:-}" ]]; then
  echo "set DATABASE_URL (postgres://user:pass@host/dbname) or PG_CONTAINER / DOCKER_DB_CONTAINER (e.g. grace-ride-db)" >&2
  exit 1
fi

TMP="$(mktemp -d)"
chmod 0700 "$TMP"
PGPASS_TMP=""
SAFE_PG_URL=""
cleanup() {
  rm -rf "$TMP"
  if [[ -n "$PGPASS_TMP" ]]; then
    rm -f "$PGPASS_TMP"
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

validate_tar_archive() {
  local tgz="$1"
  local entry mode
  while IFS= read -r entry; do
    if [[ "$entry" == /* || "$entry" == ".." || "$entry" == ../* || "$entry" == */.. || "$entry" == */../* ]]; then
      echo "Unsafe path inside archive '$tgz': $entry" >&2
      exit 1
    fi
  done < <(tar -tzf "$tgz")
  while IFS= read -r mode; do
    case "${mode:0:1}" in
      -|d) ;;
      *)
        echo "Unsafe non-regular/symlink entry inside archive '$tgz': $mode" >&2
        exit 1
        ;;
    esac
  done < <(tar -tvzf "$tgz")
}

TARBALL="$ARCHIVE"
if [[ "$ARCHIVE" == *.age ]]; then
  : "${BACKUP_AGE_KEY_FILE:?set BACKUP_AGE_KEY_FILE to decrypt .age backup}"
  TARBALL="$TMP/decrypted.tar.gz"
  age --decrypt -i "$BACKUP_AGE_KEY_FILE" -o "$TARBALL" "$ARCHIVE"
elif [[ "$ARCHIVE" == *.gpg ]]; then
  TARBALL="$TMP/decrypted.tar.gz"
  gpg --batch --yes --decrypt -o "$TARBALL" "$ARCHIVE"
fi

validate_tar_archive "$TARBALL"
tar --no-same-owner --no-same-permissions -C "$TMP" -xzf "$TARBALL"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'grace-ride-*' | head -1)"
if [[ -z "$SRC" || ! -s "$SRC/db.dump" || ! -f "$SRC/uploads.tar.gz" ]]; then
  echo "Invalid backup archive '$ARCHIVE': missing db.dump or uploads.tar.gz" >&2
  exit 1
fi

# The target database must already exist; --clean drops objects before recreating them.
if [[ -n "${DATABASE_URL:-}" ]]; then
  prepare_pgpass_url "$DATABASE_URL"
  pg_restore --clean --if-exists --no-owner --dbname "$SAFE_PG_URL" "$SRC/db.dump"
else
  PG_USER="${POSTGRES_USER:-grace}"
  PG_DB="${POSTGRES_DB:-grace_ride}"
  docker exec -i "$PG_CONTAINER" pg_restore --clean --if-exists --no-owner -U "$PG_USER" -d "$PG_DB" < "$SRC/db.dump"
fi

validate_tar_archive "$SRC/uploads.tar.gz"
mkdir -p "$DATA_DIR"
# Remove stale uploads first so post-backup orphan files do not remain on disk.
rm -rf "$DATA_DIR/uploads"
tar --no-same-owner --no-same-permissions -C "$DATA_DIR" -xzf "$SRC/uploads.tar.gz"
echo "Restored archive ($ARCHIVE) and uploads ($DATA_DIR). Start the app and check /api/health."
