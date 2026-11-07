#!/usr/bin/env bash
# Restore a backup made by scripts/backup.sh. STOP THE APP FIRST.
# Usage: scripts/restore.sh backups/grace-ride-YYYYMMDD-HHMMSS.tar.gz [DATA_DIR]
set -euo pipefail
ARCHIVE="$1"
DATA_DIR="${2:-./backend/data}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
tar -C "$TMP" -xzf "$ARCHIVE"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'grace-ride-*' | head -1)"
sqlite3 "$SRC/grace_ride.db" "PRAGMA integrity_check;" | grep -qx ok
mkdir -p "$DATA_DIR"
[ -f "$DATA_DIR/grace_ride.db" ] && mv "$DATA_DIR/grace_ride.db" "$DATA_DIR/grace_ride.db.before-restore.$(date +%s)"
rm -f "$DATA_DIR/grace_ride.db-wal" "$DATA_DIR/grace_ride.db-shm"
cp "$SRC/grace_ride.db" "$DATA_DIR/grace_ride.db"
tar -C "$DATA_DIR" -xzf "$SRC/uploads.tar.gz"
echo "Restored into $DATA_DIR. Start the app and check /api/health."
