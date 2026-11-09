#!/usr/bin/env bash
# Run the Grace Ride capacity / performance baseline (~1,000 members, 10 drivers).
# Requirement R-32 (Grace_Ride_Chinese.pdf p.15, phase P0): reproducible script + report.
set -Eeuo pipefail

cd "$(dirname "$0")/.."

echo "== Grace Ride capacity baseline (R-32) =="
echo "date: $(date -Is)"
echo "branch: $(git branch --show-current)"
echo
cargo test --test capacity -- --nocapture

echo
echo "== capacity baseline test suite finished =="
