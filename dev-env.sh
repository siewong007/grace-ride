#!/bin/bash
# Runs a command with the Rust toolchain stored on the external SSD.
export CARGO_HOME="/Volumes/APPLE EXTERNAL SSD /Rust/cargo"
export RUSTUP_HOME="/Volumes/APPLE EXTERNAL SSD /Rust/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
exec "$@"
