#!/bin/sh
set -eu

out="$1"
tag="${2:-marker}"

mkdir -p "$(dirname "$out")"
printf '%s\n' "$tag" >"$out"
printf 'marker:%s\n' "$tag"
