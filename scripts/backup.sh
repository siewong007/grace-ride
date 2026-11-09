#!/usr/bin/env bash
# Consistent snapshot of the database + uploaded receipts, with rotation.
# Usage: scripts/backup.sh [UPLOAD_DATA_DIR] [BACKUP_DIR] [KEEP_DAYS]
#   DATABASE_URL comes from the environment (same as the app uses), e.g.
#   DATABASE_URL=postgres://grace:pw@localhost/grace_ride
# Uploads still live in DATA_DIR/uploads; the database is dumped with pg_dump
# (consistent snapshot, safe to run while the app is serving traffic).
set -euo pipefail
DATA_DIR="${1:-./backend/data}"
BACKUP_DIR="${2:-./backups}"
KEEP_DAYS="${3:-30}"
STAMP="$(date +%Y%m%d-%H%M%S)"
DEST="$BACKUP_DIR/grace-ride-$STAMP"
mkdir -p "$DEST"
umask 077

: "${DATABASE_URL:?set DATABASE_URL (postgres://user:pass@host/dbname)}"

pg_dump --format=custom --no-owner --file "$DEST/db.dump" "$DATABASE_URL"
test -s "$DEST/db.dump" || { echo "pg_dump produced an empty file" >&2; exit 1; }
pg_restore --list "$DEST/db.dump" >/dev/null || { echo "dump sanity check failed" >&2; exit 1; }
tar -C "$DATA_DIR" -czf "$DEST/uploads.tar.gz" uploads
tar -C "$BACKUP_DIR" -czf "$DEST.tar.gz" "grace-ride-$STAMP" && rm -rf "$DEST"
find "$BACKUP_DIR" -name 'grace-ride-*.tar.gz' -mtime +"$KEEP_DAYS" -delete
echo "Backup written: $DEST.tar.gz"
# Contains personal data (addresses, receipts): store encrypted and off-site.
