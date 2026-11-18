#!/usr/bin/env bash
# Runs a command with the external-SSD Rust toolchain when mounted,
# falling back transparently to the host toolchain otherwise.
set -euo pipefail
SSD_RUST="/Volumes/APPLE EXTERNAL SSD /Rust"
if [[ -d "$SSD_RUST/cargo" && -d "$SSD_RUST/rustup" ]]; then
  export CARGO_HOME="$SSD_RUST/cargo"
  export RUSTUP_HOME="$SSD_RUST/rustup"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
exec "$@"
