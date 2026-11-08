#!/usr/bin/env bash
# Restore a backup made by scripts/backup.sh. STOP THE APP FIRST.
# Usage: scripts/restore.sh backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz [UPLOAD_DATA_DIR]
#   DATABASE_URL comes from the environment (same as the app uses).
set -euo pipefail
ARCHIVE="$1"
DATA_DIR="${2:-./backend/data}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

: "${DATABASE_URL:?set DATABASE_URL (postgres://user:pass@host/dbname)}"

tar -C "$TMP" -xzf "$ARCHIVE"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'grace-ride-*' | head -1)"

# The target database must already exist; --clean drops objects before recreating them.
pg_restore --clean --if-exists --no-owner --dbname "$DATABASE_URL" "$SRC/db.dump"

# Replace the uploaded files with the archived copy.
tar -C "$DATA_DIR" -xzf "$SRC/uploads.tar.gz"
echo "Restored into DATABASE_URL. Start the app and check /api/health."
