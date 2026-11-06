#!/usr/bin/env bash
set -euo pipefail

COMPILER="${SPECTRE:-./spectre-dev}"
if [[ ! -x "$COMPILER" ]]; then
	if [[ -x "./spectre-dev.exe" ]]; then
		COMPILER="./spectre-dev.exe"
	else
		echo "error: compiler not found: $COMPILER" >&2
		exit 1
	fi
fi

KERNEL="${SPECTRE_SIMD_KERNEL:-./fw/simd_kernel.sx}"
EXPECTED="2314892023878"
TMP=".simd-test-$$"
rm -rf "$TMP"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

has_cmd() {
	command -v "$1" >/dev/null 2>&1
}

run_mode() {
	local label="$1"
	shift
	local output="$TMP/$label.exe"
	local generated="$TMP/$label.c"

	echo "==> SIMD mode: $label ${*:-auto}"
	"$COMPILER" "$KERNEL" -release -alt "$@" -native -o "$output"
	local actual
	actual="$("$output")"
	echo "$actual"

	if [[ "$actual" != *"Checksum: $EXPECTED"* ]]; then
		echo "error: unexpected checksum in $label" >&2
		exit 1
	fi

	"$COMPILER" "$KERNEL" -release -alt "$@" -native -emit-alt >"$generated"
	if ! grep -q "int64_t sx__ve" "$generated"; then
		echo "error: no vectorized loop emitted for $label" >&2
		exit 1
	fi
}

run_scalar() {
	local label="$1"
	shift
	local output="$TMP/scalar-$label.exe"
	local generated="$TMP/scalar-$label.c"

	echo "==> scalar fallback: $label ${*:-auto}"
	"$COMPILER" "$KERNEL" -release -alt "$@" -native -no-simd -o "$output"
	local actual
	actual="$("$output")"
	echo "$actual"

	if [[ "$actual" != *"Checksum: $EXPECTED"* ]]; then
		echo "error: unexpected checksum in scalar $label" >&2
		exit 1
	fi

	"$COMPILER" "$KERNEL" -release -alt "$@" -native -no-simd -emit-alt >"$generated"
	if grep -q "int64_t sx__ve" "$generated"; then
		echo "error: vectorized loop emitted under -no-simd for $label" >&2
		exit 1
	fi
}

run_mode "auto"
run_scalar "auto"

echo "==> fallback: unsupported platform"
"$COMPILER" "$KERNEL" -release -alt -src-target:freebsd -emit-alt >"$TMP/fallback.c"
if grep -q "int64_t sx__ve" "$TMP/fallback.c"; then
	echo "error: vectorized loop emitted for an unsupported platform" >&2
	exit 1
fi

if has_cmd clang || has_cmd clang.exe; then
	run_mode "clang" -alt:clang
fi

if has_cmd clang-cl || has_cmd clang-cl.exe; then
	run_mode "clang-cl" -alt:clang-cl
fi

if has_cmd cl || has_cmd cl.exe; then
	run_mode "msvc" -alt:cl
	run_scalar "msvc" -alt:cl
fi

echo "SIMD checks passed."
