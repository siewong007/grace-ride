#!/usr/bin/env bash
# Consistent snapshot of the database + uploaded receipts, with rotation.
# Usage: scripts/backup.sh [DATA_DIR] [BACKUP_DIR] [KEEP_DAYS]
#   Docker: docker compose run --rm -v "$PWD/backups:/backups" --entrypoint sh app \
#           -c 'cp -r /data /backups/data-$(date +%F)'   (or run this script on the host volume)
set -euo pipefail
DATA_DIR="${1:-./backend/data}"
BACKUP_DIR="${2:-./backups}"
KEEP_DAYS="${3:-30}"
STAMP="$(date +%Y%m%d-%H%M%S)"
DEST="$BACKUP_DIR/grace-ride-$STAMP"
mkdir -p "$DEST"
umask 077

# sqlite3 .backup is safe while the app is running (WAL mode).
sqlite3 "$DATA_DIR/grace_ride.db" ".backup '$DEST/grace_ride.db'"
sqlite3 "$DEST/grace_ride.db" "PRAGMA integrity_check;" | grep -qx ok || { echo "integrity check failed" >&2; exit 1; }
tar -C "$DATA_DIR" -czf "$DEST/uploads.tar.gz" uploads
tar -C "$BACKUP_DIR" -czf "$DEST.tar.gz" "grace-ride-$STAMP" && rm -rf "$DEST"
find "$BACKUP_DIR" -name 'grace-ride-*.tar.gz' -mtime +"$KEEP_DAYS" -delete
echo "Backup written: $DEST.tar.gz"
# Contains personal data (addresses, receipts): store encrypted and off-site.
