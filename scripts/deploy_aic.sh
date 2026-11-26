#!/usr/bin/env bash
# Build Grace Ride (linux/amd64) locally and roll it directly onto the AIC VPS
# using the shared local SSH key (no GitHub Actions required).
# Set EXISTING_TAG=<tag> to roll out an image already loaded on the VPS without rebuilding.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

VPS_HOST="${VPS_HOST:-162.19.81.122}"
VPS_PORT="${VPS_PORT:-20049}"
VPS_USER="${VPS_USER:-root}"
VPS_SSH_KEY="${VPS_SSH_KEY:-/Volumes/APPLE EXTERNAL SSD /Personal Projects/hotel-app/deploy/credentials/aic-vps-ed25519}"
VPS_HOST_KEY='[162.19.81.122]:20049 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIxenH95jfg3hxfkumJpZJyV7phwCNDrXrAL0wUXnta9'
VPS_HOST_FP='SHA256:HV6OszvuYmwmWRLEY7z7jm8GojWZJppJlDBHIhJVUxw'
PUBLIC_URL="${PUBLIC_URL:-https://graceride.saliminn.my}"

[[ -f "$VPS_SSH_KEY" ]] || { echo "Missing SSH private key: $VPS_SSH_KEY" >&2; exit 1; }

USE_EXISTING=0
if [[ -n "${EXISTING_TAG:-}" ]]; then
  TAG="$EXISTING_TAG"
  USE_EXISTING=1
else
  command -v docker >/dev/null 2>&1 || { echo "docker CLI is required" >&2; exit 1; }
  docker info >/dev/null 2>&1 || { echo "Docker daemon is not running" >&2; exit 1; }
  head_sha="$(git rev-parse --short=12 HEAD)"
  if ! git diff --quiet || ! git diff --cached --quiet; then
    TAG="${head_sha}-$(date -u +%Y%m%d%H%M%S)"
  else
    TAG="$(git rev-parse HEAD)"
  fi
fi
[[ "$TAG" =~ ^[a-z0-9-]+$ ]] || { echo "Invalid image tag: $TAG" >&2; exit 1; }

TMP_DIR="$(mktemp -d /tmp/grace-ride-deploy.XXXXXX)"
SSH_READY=0

install -m 0600 /dev/null "$TMP_DIR/known_hosts"
printf '%s\n' "$VPS_HOST_KEY" > "$TMP_DIR/known_hosts"
ssh-keygen -lf "$TMP_DIR/known_hosts" | grep -Fq "$VPS_HOST_FP" \
  || { echo "Pinned host key fingerprint mismatch" >&2; exit 1; }

ssh_opts=(
  -i "$VPS_SSH_KEY"
  -p "$VPS_PORT"
  -o BatchMode=yes
  -o ConnectTimeout=20
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=yes
  -o "UserKnownHostsFile=$TMP_DIR/known_hosts"
)
scp_opts=(
  -i "$VPS_SSH_KEY"
  -P "$VPS_PORT"
  -o BatchMode=yes
  -o ConnectTimeout=20
  -o IdentitiesOnly=yes
  -o StrictHostKeyChecking=yes
  -o "UserKnownHostsFile=$TMP_DIR/known_hosts"
)

cleanup() {
  if [[ "$SSH_READY" == "1" ]]; then
    ssh "${ssh_opts[@]}" "$VPS_USER@$VPS_HOST" \
      "rm -f -- /tmp/grace-ride-$TAG.tar.gz.part /tmp/grace-ride-$TAG.tar.gz /tmp/docker-compose.aic.$TAG.yml /tmp/docker-compose.osrm.$TAG.yml" \
      >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

if [[ "$USE_EXISTING" == "0" ]]; then
  echo "[deploy] Building linux/amd64 image grace-ride:$TAG ..."
  docker buildx build \
    --platform linux/amd64 \
    --load \
    -t "grace-ride:$TAG" \
    --label "org.opencontainers.image.revision=$TAG" \
    .

  test "$(docker image inspect --format '{{.Os}}/{{.Architecture}}' "grace-ride:$TAG")" = "linux/amd64" \
    || { echo "Built image is not linux/amd64" >&2; exit 1; }

  echo "[deploy] Exporting and verifying compressed image tarball..."
  docker save "grace-ride:$TAG" | gzip -1 > "$TMP_DIR/grace-ride-$TAG.tar.gz"
  gzip -t "$TMP_DIR/grace-ride-$TAG.tar.gz"
  ls -lh "$TMP_DIR/grace-ride-$TAG.tar.gz"
else
  echo "[deploy] Reusing existing image grace-ride:$TAG on VPS..."
fi

echo "[deploy] Checking existing production container on $VPS_USER@$VPS_HOST:$VPS_PORT ..."
SSH_READY=1
if [[ "$USE_EXISTING" == "1" ]]; then
  ssh "${ssh_opts[@]}" "$VPS_USER@$VPS_HOST" "docker image inspect 'grace-ride:$TAG' >/dev/null" \
    || { echo "Image grace-ride:$TAG not found on VPS" >&2; exit 1; }
fi
prev_image=$(ssh "${ssh_opts[@]}" "$VPS_USER@$VPS_HOST" \
  "docker inspect -f '{{.Config.Image}}' grace-ride 2>/dev/null || true")
old_tag="${prev_image#grace-ride:}"
if [[ -z "$prev_image" || ! "$old_tag" =~ ^[A-Za-z0-9._-]+$ ]]; then
  old_tag="none"
fi
echo "[deploy] Current production image: ${prev_image:-none} (rollback tag: $old_tag)"

if [[ "$USE_EXISTING" == "1" ]]; then
  echo "[deploy] Uploading compose manifests (reusing loaded image)..."
else
  echo "[deploy] Uploading compose manifests and image tarball..."
fi
scp "${scp_opts[@]}" \
  deploy/docker-compose.aic.yml \
  "$VPS_USER@$VPS_HOST:/tmp/docker-compose.aic.$TAG.yml"
if [[ -f deploy/docker-compose.osrm.yml ]]; then
  scp "${scp_opts[@]}" \
    deploy/docker-compose.osrm.yml \
    "$VPS_USER@$VPS_HOST:/tmp/docker-compose.osrm.$TAG.yml"
fi
if [[ "$USE_EXISTING" == "0" ]]; then
  scp "${scp_opts[@]}" \
    "$TMP_DIR/grace-ride-$TAG.tar.gz" \
    "$VPS_USER@$VPS_HOST:/tmp/grace-ride-$TAG.tar.gz.part"
fi

echo "[deploy] Rolling out grace-ride:$TAG on VPS..."
ssh "${ssh_opts[@]}" \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=8 \
  "$VPS_USER@$VPS_HOST" \
  "TAG='$TAG' USE_EXISTING='$USE_EXISTING' bash -s" <<'REMOTE'
set -euo pipefail
[[ "$TAG" =~ ^[a-z0-9-]+$ ]]

docker image prune -f >/dev/null 2>&1 || true
avail=$(df --output=avail -B1 / | tail -n 1 | tr -d ' ')
if [[ "$avail" -lt 1500000000 ]]; then
  echo "Not enough free disk for a Grace Ride roll" >&2
  df -h / >&2
  exit 1
fi

install -d -m 0750 /opt/grace-ride
if [[ -f /opt/grace-ride/docker-compose.aic.yml ]]; then
  cp -a /opt/grace-ride/docker-compose.aic.yml /opt/grace-ride/docker-compose.aic.yml.prev
fi
install -m 0644 "/tmp/docker-compose.aic.$TAG.yml" /opt/grace-ride/docker-compose.aic.yml
rm -f "/tmp/docker-compose.aic.$TAG.yml"

if [[ -f "/tmp/docker-compose.osrm.$TAG.yml" ]]; then
  if [[ -f /opt/grace-ride/docker-compose.osrm.yml ]]; then
    cp -a /opt/grace-ride/docker-compose.osrm.yml /opt/grace-ride/docker-compose.osrm.yml.prev
  fi
  install -m 0644 "/tmp/docker-compose.osrm.$TAG.yml" /opt/grace-ride/docker-compose.osrm.yml
  rm -f "/tmp/docker-compose.osrm.$TAG.yml"
fi

old_image=$(docker inspect -f '{{.Config.Image}}' grace-ride 2>/dev/null || echo '')
if [[ -z "$old_image" ]]; then
  echo "No existing app container; will start fresh."
  old_image="grace-ride:none"
fi
old_tag=${old_image#grace-ride:}

set_image_tag() {
  local tag="$1"
  if grep -q '^IMAGE_TAG=' /opt/grace-ride/secrets.env; then
    sed -i "s/^IMAGE_TAG=.*/IMAGE_TAG=${tag}/" /opt/grace-ride/secrets.env
  else
    [[ -s /opt/grace-ride/secrets.env && -n "$(tail -c 1 /opt/grace-ride/secrets.env)" ]] \
      && printf '\n' >> /opt/grace-ride/secrets.env
    printf 'IMAGE_TAG=%s\n' "$tag" >> /opt/grace-ride/secrets.env
  fi
}

mkdir -p /opt/grace-ride/pre-roll-backups
chmod 700 /opt/grace-ride/pre-roll-backups

ensure_secrets_encryption_key() {
  chmod 600 /opt/grace-ride/secrets.env 2>/dev/null || true
  if ! grep -qE '^SECRETS_ENCRYPTION_KEY=[^[:space:]]+' /opt/grace-ride/secrets.env; then
    local new_key
    new_key="$(openssl rand -hex 32)"
    [[ -s /opt/grace-ride/secrets.env && -n "$(tail -c 1 /opt/grace-ride/secrets.env)" ]] \
      && printf '\n' >> /opt/grace-ride/secrets.env
    printf 'SECRETS_ENCRYPTION_KEY=%s\n' "$new_key" >> /opt/grace-ride/secrets.env
    echo "Initialized SECRETS_ENCRYPTION_KEY in /opt/grace-ride/secrets.env"
  fi
}

write_pre_roll_dump() {
  local dump_base="/opt/grace-ride/pre-roll-backups/pre-${TAG}.dump"
  local age_pub
  age_pub=$(grep -E '^BACKUP_AGE_RECIPIENT=' /opt/grace-ride/secrets.env 2>/dev/null | head -n 1 | cut -d= -f2- | tr -d "\"'\r\n" || true)
  if [[ -n "$age_pub" ]] && command -v age >/dev/null 2>&1; then
    docker exec grace-ride-db pg_dump -U grace -d grace_ride -Fc | age -r "$age_pub" -o "${dump_base}.age"
    chmod 600 "${dump_base}.age"
  else
    docker exec grace-ride-db pg_dump -U grace -d grace_ride -Fc > "$dump_base"
    chmod 600 "$dump_base"
  fi
  ls -1t /opt/grace-ride/pre-roll-backups/pre-*.dump* 2>/dev/null | tail -n +3 | xargs -r rm -f
}

initial_users=0
pre_dump_done=0
if [[ "$(docker inspect -f '{{.State.Running}}' grace-ride-db 2>/dev/null || echo false)" == "true" ]]; then
  initial_users=$(docker exec grace-ride-db psql -U grace -d grace_ride -tAc 'SELECT COUNT(*) FROM users' 2>/dev/null || echo 0)
  initial_users=${initial_users//[$'\t\r\n ']/}
  initial_users=${initial_users:-0}
  if [[ "$initial_users" =~ ^[0-9]+$ ]] && (( initial_users > 0 )); then
    write_pre_roll_dump
    pre_dump_done=1
  fi
fi

if [[ "$USE_EXISTING" == "0" || -f "/tmp/grace-ride-$TAG.tar.gz.part" ]]; then
  mv "/tmp/grace-ride-$TAG.tar.gz.part" "/tmp/grace-ride-$TAG.tar.gz"
  gzip -t "/tmp/grace-ride-$TAG.tar.gz"
  gunzip -c "/tmp/grace-ride-$TAG.tar.gz" | docker load
  rm -f "/tmp/grace-ride-$TAG.tar.gz"
fi

net_internal=$(docker network inspect -f '{{.Internal}}' grace-ride_database 2>/dev/null || echo missing)
if [[ "$net_internal" == "false" ]] && grep -qE '^[[:space:]]+internal:[[:space:]]+true' /opt/grace-ride/docker-compose.aic.yml; then
  echo "Migrating grace-ride_database network to internal: true (preserving volumes)..."
  IMAGE_TAG="${TAG}" docker compose \
    --env-file /opt/grace-ride/secrets.env \
    -f /opt/grace-ride/docker-compose.aic.yml \
    down
fi

if [[ "$(docker inspect -f '{{.State.Running}}' grace-ride-db 2>/dev/null || echo false)" != "true" ]]; then
  echo "grace-ride-db is not running; booting database container first..."
  IMAGE_TAG="${TAG}" docker compose \
    --env-file /opt/grace-ride/secrets.env \
    -f /opt/grace-ride/docker-compose.aic.yml \
    up -d db
fi
db_healthy=0
for _ in $(seq 1 30); do
  if [[ "$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' grace-ride-db 2>/dev/null || echo none)" == "healthy" ]]; then
    db_healthy=1
    break
  fi
  sleep 2
done
if [[ "$db_healthy" != 1 ]]; then
  echo "grace-ride-db failed to reach healthy state" >&2
  docker logs grace-ride-db --tail 30 >&2 || true
  exit 1
fi

users_before=$(docker exec grace-ride-db psql -U grace -d grace_ride -tAc 'SELECT COUNT(*) FROM users' 2>/dev/null || echo 0)
users_before=${users_before//[$'\t\r\n ']/}
users_before=${users_before:-0}
if [[ "$initial_users" =~ ^[0-9]+$ ]] && (( initial_users > 0 )); then
  users_before="$initial_users"
fi
db_before=$(docker inspect -f '{{.Id}}' grace-ride-db)
echo "users_before=$users_before"
echo "old_image=$old_image"

if [[ "$pre_dump_done" == "0" ]] && (( users_before > 0 )); then
  write_pre_roll_dump
fi

rollback() {
  trap - ERR
  if [[ "$old_tag" == "none" || -z "$old_tag" ]]; then
    echo "No previous image to restore" >&2
    return 0
  fi
  if [[ -f /opt/grace-ride/docker-compose.aic.yml.prev ]]; then
    mv -f /opt/grace-ride/docker-compose.aic.yml.prev /opt/grace-ride/docker-compose.aic.yml
  fi
  if [[ -f /opt/grace-ride/docker-compose.osrm.yml.prev ]]; then
    mv -f /opt/grace-ride/docker-compose.osrm.yml.prev /opt/grace-ride/docker-compose.osrm.yml
  fi
  set_image_tag "$old_tag"
  if ! docker compose --env-file /opt/grace-ride/secrets.env -f /opt/grace-ride/docker-compose.aic.yml up -d --no-deps --force-recreate app; then
    IMAGE_TAG="$old_tag" docker compose --env-file /opt/grace-ride/secrets.env -f /opt/grace-ride/docker-compose.aic.yml down
    IMAGE_TAG="$old_tag" docker compose --env-file /opt/grace-ride/secrets.env -f /opt/grace-ride/docker-compose.aic.yml up -d
  fi
}
trap rollback ERR

ensure_secrets_encryption_key
set_image_tag "$TAG"
grep -q "^IMAGE_TAG=${TAG}$" /opt/grace-ride/secrets.env
docker compose --env-file /opt/grace-ride/secrets.env -f /opt/grace-ride/docker-compose.aic.yml up -d --no-deps --force-recreate app

ok=0
for i in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:8090/api/health; then
    echo
    echo "health_try=$i"
    ok=1
    break
  fi
  sleep 2
done
db_after=$(docker inspect -f '{{.Id}}' grace-ride-db)
users_after=$(docker exec grace-ride-db psql -U grace -d grace_ride -tAc 'SELECT COUNT(*) FROM users' 2>/dev/null || echo 0)
users_after=${users_after//[$'\t\r\n ']/}
users_after=${users_after:-0}
echo "users_after=$users_after"

if [[ "$ok" != 1 || "$db_before" != "$db_after" ]]; then
  echo "roll failed (health/db); restoring $old_image" >&2
  docker logs grace-ride 2>&1 | grep -v -e 'one-time' -e '/invite/' | tail -60 >&2 || true
  rollback
  exit 1
fi
if ! [[ "$users_after" =~ ^[0-9]+$ && "$users_before" =~ ^[0-9]+$ ]]; then
  echo "roll failed (user count unreadable); restoring $old_image" >&2
  rollback
  exit 1
fi
if (( users_after < users_before )); then
  echo "roll failed (users shrank $users_before → $users_after); restoring $old_image" >&2
  docker logs grace-ride 2>&1 | grep -v -e 'one-time' -e '/invite/' | tail -60 >&2 || true
  rollback
  exit 1
fi

docker images grace-ride --format '{{.Repository}}:{{.Tag}}' | while read -r img; do
  if [[ "$img" != "grace-ride:$TAG" && "$img" != "$old_image" ]]; then
    docker image rm "$img" >/dev/null 2>&1 || true
  fi
done
echo "app_image=$(docker inspect -f '{{.Config.Image}}' grace-ride)"
echo "db_net_internal=$(docker network inspect -f '{{.Internal}}' grace-ride_database)"
REMOTE

echo "[deploy] Checking public HTTPS health endpoint $PUBLIC_URL/api/health ..."
if ! curl --fail --silent --show-error \
  --retry 10 --retry-delay 3 --retry-all-errors \
  --max-time 20 \
  "$PUBLIC_URL/api/health"; then
  echo
  if [[ -n "${old_tag:-}" && "$old_tag" != "none" ]]; then
    echo "[deploy] Public health check failed; rolling back VPS to grace-ride:$old_tag..." >&2
    ssh "${ssh_opts[@]}" "$VPS_USER@$VPS_HOST" "OLD_TAG='$old_tag' bash -s" <<'ROLLBACK'
set -euo pipefail
[[ "$OLD_TAG" =~ ^[A-Za-z0-9._-]+$ ]]
if [[ -f /opt/grace-ride/docker-compose.aic.yml.prev ]]; then
  mv -f /opt/grace-ride/docker-compose.aic.yml.prev /opt/grace-ride/docker-compose.aic.yml
fi
if [[ -f /opt/grace-ride/docker-compose.osrm.yml.prev ]]; then
  mv -f /opt/grace-ride/docker-compose.osrm.yml.prev /opt/grace-ride/docker-compose.osrm.yml
fi
if grep -q '^IMAGE_TAG=' /opt/grace-ride/secrets.env; then
  sed -i "s/^IMAGE_TAG=.*/IMAGE_TAG=${OLD_TAG}/" /opt/grace-ride/secrets.env
else
  printf 'IMAGE_TAG=%s\n' "$OLD_TAG" >> /opt/grace-ride/secrets.env
fi
if ! docker compose --env-file /opt/grace-ride/secrets.env -f /opt/grace-ride/docker-compose.aic.yml up -d --no-deps --force-recreate app; then
  IMAGE_TAG="$OLD_TAG" docker compose --env-file /opt/grace-ride/secrets.env -f /opt/grace-ride/docker-compose.aic.yml down
  IMAGE_TAG="$OLD_TAG" docker compose --env-file /opt/grace-ride/secrets.env -f /opt/grace-ride/docker-compose.aic.yml up -d
fi
echo "Rolled back app_image=$(docker inspect -f '{{.Config.Image}}' grace-ride)"
ROLLBACK
  fi
  exit 1
fi
ssh "${ssh_opts[@]}" "$VPS_USER@$VPS_HOST" \
  "rm -f /opt/grace-ride/docker-compose.aic.yml.prev /opt/grace-ride/docker-compose.osrm.yml.prev" \
  >/dev/null 2>&1 || true
echo
echo "[deploy] Deployment complete: grace-ride:$TAG is live at $PUBLIC_URL"
