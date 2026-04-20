#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples/csamples"
COMPILER="./spectre-dev"

if [ ! -d "$SAMPLES_DIR" ]; then
    echo "Error: directory not found: $SAMPLES_DIR"
    exit 1
fi

for cfile in "$SAMPLES_DIR"/*.c; do
    [ -e "$cfile" ] || continue

    name="$(basename "$cfile" .c)"
    sxfile="$name.sx"

    echo "== Testing $name =="

    "$COMPILER" "$cfile" --translate-c
    if [ $? -ne 0 ]; then
        echo "[FAIL] translation failed: $name"
        echo
        continue
    fi

    "$COMPILER" "./s-source-out/$sxfile"
    if [ $? -ne 0 ]; then
        echo "[FAIL] compile failed: $name"
        echo
        continue
    fi

    echo "[PASS] $name"
    echo
done
