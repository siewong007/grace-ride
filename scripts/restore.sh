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
trap 'rm -rf "$TMP"' EXIT

TARBALL="$ARCHIVE"
if [[ "$ARCHIVE" == *.age ]]; then
  : "${BACKUP_AGE_KEY_FILE:?set BACKUP_AGE_KEY_FILE to decrypt .age backup}"
  TARBALL="$TMP/decrypted.tar.gz"
  age --decrypt -i "$BACKUP_AGE_KEY_FILE" -o "$TARBALL" "$ARCHIVE"
elif [[ "$ARCHIVE" == *.gpg ]]; then
  TARBALL="$TMP/decrypted.tar.gz"
  gpg --batch --yes --decrypt -o "$TARBALL" "$ARCHIVE"
fi

tar -C "$TMP" -xzf "$TARBALL"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'grace-ride-*' | head -1)"
if [[ -z "$SRC" || ! -s "$SRC/db.dump" || ! -f "$SRC/uploads.tar.gz" ]]; then
  echo "Invalid backup archive '$ARCHIVE': missing db.dump or uploads.tar.gz" >&2
  exit 1
fi

# The target database must already exist; --clean drops objects before recreating them.
if [[ -n "${DATABASE_URL:-}" ]]; then
  pg_restore --clean --if-exists --no-owner --dbname "$DATABASE_URL" "$SRC/db.dump"
else
  PG_USER="${POSTGRES_USER:-grace}"
  PG_DB="${POSTGRES_DB:-grace_ride}"
  docker exec -i "$PG_CONTAINER" pg_restore --clean --if-exists --no-owner -U "$PG_USER" -d "$PG_DB" < "$SRC/db.dump"
fi

mkdir -p "$DATA_DIR"
# Remove stale uploads first so post-backup orphan files do not remain on disk.
rm -rf "$DATA_DIR/uploads"
tar -C "$DATA_DIR" -xzf "$SRC/uploads.tar.gz"
echo "Restored archive ($ARCHIVE) and uploads ($DATA_DIR). Start the app and check /api/health."
