#!/usr/bin/env bash
# Build and run the compile-time-evaluation runtime benchmarks.
# Usage: samples/perf/bench.sh [output-dir]
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${1:-/tmp/spectre-perf}"
cc="${SPECTRE:-$root/spectre-dev}"
mkdir -p "$out"

for src in fold_const fold_expr fold_loop; do
    "$cc" "$root/samples/perf/$src.sx" -release -o "$out/${src}_qbe"
    "$cc" "$root/samples/perf/$src.sx" -release -alt:tcc -o "$out/${src}_tcc"
done

for src in fold_const fold_expr fold_loop; do
    echo "== $src qbe =="
    hyperfine --warmup 2 --runs 8 "$out/${src}_qbe"
    echo "== $src alt/tcc =="
    hyperfine --warmup 2 --runs 8 "$out/${src}_tcc"
done
