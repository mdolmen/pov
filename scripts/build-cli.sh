#!/usr/bin/env bash
# Builds the release `pov-cli` binary and copies it where Tauri's `externalBin` expects it:
# src-tauri/binaries/pov-cli-<target triple>.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
triple=$(rustc -vV | sed -n 's/^host: //p')

cargo build --release --manifest-path "$root/src-tauri/Cargo.toml" \
  -p pov-core --features cli --bin pov-cli
mkdir -p "$root/src-tauri/binaries"
cp "$root/src-tauri/target/release/pov-cli" "$root/src-tauri/binaries/pov-cli-$triple"
echo "  → src-tauri/binaries/pov-cli-$triple"
