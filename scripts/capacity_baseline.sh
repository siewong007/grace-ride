#!/usr/bin/env bash
# Run the Grace Ride capacity / performance baseline (~1,000 members, 10 drivers).
# Requirement R-32 (see docs/TECHNICAL.md): reproducible script + report.
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export TEST_DATABASE_URL="${TEST_DATABASE_URL:-postgres://localhost/grace_ride_test}"

cd "$ROOT_DIR/backend"

echo "== Grace Ride capacity baseline (R-32) =="
echo "date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo "branch: $(git branch --show-current)"
echo
../dev-env.sh cargo test --test capacity -- --nocapture

echo
echo "== capacity baseline test suite finished =="
