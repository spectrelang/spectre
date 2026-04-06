#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "[build] compiling spectre compiler (release)..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

echo "[build] compiling yyjson_shim.c..."
mkdir -p "$SCRIPT_DIR/std/csources"
cc -c -o "$SCRIPT_DIR/std/csources/yyjson_shim.o" "$SCRIPT_DIR/std/csources/yyjson_shim.c"

DEST_DIR="$SCRIPT_DIR/target/release/std/csources"
mkdir -p "$DEST_DIR"
cp "$SCRIPT_DIR/std/csources/yyjson_shim.o" "$DEST_DIR/yyjson_shim.o"

echo "[build] done. binary: $SCRIPT_DIR/target/release/spectre"
echo "[build] shim:   $DEST_DIR/yyjson_shim.o"
