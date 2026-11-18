#!/usr/bin/env bash
# Download Geofabrik's Malaysia/Singapore/Brunei OSM extract, verify its MD5 checksum,
# and pre-process an MLD car-routing graph for deploy/docker-compose.osrm.yml.
#
# Usage: scripts/setup_osrm_malaysia.sh [OSRM_DATA_DIR]
set -Eeuo pipefail

OSRM_DATA_DIR="${1:-./osrm-data}"
OSRM_IMAGE="${OSRM_IMAGE:-ghcr.io/project-osrm/osrm-backend:v5.27.1}"
PBF_BASE="malaysia-singapore-brunei-latest"
PBF_URL="https://download.geofabrik.de/asia/${PBF_BASE}.osm.pbf"
MD5_URL="${PBF_URL}.md5"

mkdir -p "$OSRM_DATA_DIR"
ABS_DIR="$(cd "$OSRM_DATA_DIR" && pwd)"
STAGING="$(mktemp -d "${ABS_DIR}/.staging.XXXXXX")"
chmod 0755 "$STAGING"
trap 'rm -rf "$STAGING"' EXIT

echo "== Downloading ${PBF_BASE}.osm.pbf and checksum =="
curl -fL --retry 3 --output "$STAGING/${PBF_BASE}.osm.pbf" "$PBF_URL"
curl -fL --retry 3 --output "$STAGING/${PBF_BASE}.osm.pbf.md5" "$MD5_URL"

(
  cd "$STAGING"
  if command -v md5sum >/dev/null 2>&1; then
    md5sum -c "${PBF_BASE}.osm.pbf.md5"
  elif command -v md5 >/dev/null 2>&1; then
    expected="$(awk '{print $1}' "${PBF_BASE}.osm.pbf.md5")"
    actual="$(md5 -q "${PBF_BASE}.osm.pbf")"
    [[ "$expected" == "$actual" ]] || { echo "MD5 mismatch: expected $expected, got $actual" >&2; exit 1; }
  fi
)

echo "== Building OSRM MLD graph with ${OSRM_IMAGE} =="
docker run --rm -v "$STAGING:/data" "$OSRM_IMAGE" osrm-extract -p /opt/car.lua "/data/${PBF_BASE}.osm.pbf"
docker run --rm -v "$STAGING:/data" "$OSRM_IMAGE" osrm-partition "/data/${PBF_BASE}.osrm"
docker run --rm -v "$STAGING:/data" "$OSRM_IMAGE" osrm-customize "/data/${PBF_BASE}.osrm"

# Remove raw PBF before swapping into place to save disk space.
rm -f "$STAGING/${PBF_BASE}.osm.pbf" "$STAGING/${PBF_BASE}.osm.pbf.md5"
find "$ABS_DIR" -maxdepth 1 -name "${PBF_BASE}.osrm*" -delete
mv "$STAGING"/${PBF_BASE}.osrm* "$ABS_DIR/"

echo "== OSRM graph ready in ${ABS_DIR} =="
echo "Set OSRM_URL=http://grace-ride-osrm:5000 in /opt/grace-ride/secrets.env and start deploy/docker-compose.osrm.yml."
