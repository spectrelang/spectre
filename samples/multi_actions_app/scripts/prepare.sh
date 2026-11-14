#!/bin/sh
set -eu

mode="${1:-unknown}"
out="${2:-}"

if [ -n "$out" ]; then
	mkdir -p "$(dirname "$out")"
	printf 'prepare:%s\n' "$mode" >"$out"
fi

printf 'prepare:%s\n' "$mode"
