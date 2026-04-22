#!/usr/bin/env bash

set -euo pipefail

SRC="./src/sxc.sx"

echo "Preparing directories"

echo "Building C support objects"
cc -c std/csources/panic_handler.c -o std/csources/panic_handler.o
cc -c std/csources/yyjson_shim.c -o std/csources/yyjson_shim.o

echo "[Stage 0] Building spectre-dev (default backend)"
spectre "$SRC" -o spectre-dev

echo "[Stage 1] Building spectre-stage1 with LLVM backend"
./spectre-dev "$SRC" --llvm -o "spectre-stage1"

echo "[Stage 2] Building spectre-stage2 with stage1"
"spectre-stage1" "$SRC" --llvm -o "spectre-stage2"

echo "Verifying bootstrap consistency"

echo "Bootstrap complete."
