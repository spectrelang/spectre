#!/usr/bin/env bash

set -u

STRICT=0
for arg in "$@"; do
	case "$arg" in
	--strict)
		STRICT=1
		;;
	-h | --help)
		echo "usage: fw/checkmem.sh [--strict]"
		echo
		echo "  --strict   exit non-zero when any harness reports a leak"
		exit 0
		;;
	*)
		echo "error: unknown option '$arg'"
		echo "usage: fw/checkmem.sh [--strict]"
		exit 1
		;;
	esac
done

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HARNESS_DIR="$ROOT_DIR/fw/harness"
COMPILER="${SPECTRE_CHECKMEM_COMPILER:-$ROOT_DIR/spectre-dev}"
LOG_DIR="${TEST_LOG_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/spectre-checkmem-logs}"

if [ ! -x "$COMPILER" ]; then
	echo "ERROR: compiler not found or not executable at $COMPILER"
	echo "Build it with ./fw/build.sh or set SPECTRE_CHECKMEM_COMPILER"
	exit 1
fi

mkdir -p "$LOG_DIR"
mkdir -p "$ROOT_DIR/s-out"

cd "$ROOT_DIR" || exit 1

total=0
passed=0
failed=0
findings=0
errors=0

echo "Memory leak check:"
echo "  compiler: $COMPILER"
echo "  harness:  $HARNESS_DIR"
echo "  logs:     $LOG_DIR"
echo

for harness in "$HARNESS_DIR"/*.sx; do
	[ -e "$harness" ] || continue

	name="$(basename "$harness" .sx)"
	log_file="$LOG_DIR/$name.log"
	total=$((total + 1))

	if "$COMPILER" "$harness" -test -leak-check >"$log_file" 2>&1 </dev/null; then
		echo "[PASS] $name"
		passed=$((passed + 1))
	else
		echo "[FAIL] $name"
		failed=$((failed + 1))
		if grep -q '^\[FAIL\]' "$log_file"; then
			findings=$((findings + 1))
			grep '^\[FAIL\]' "$log_file" | awk '!seen[$0]++' | sed -n '1,12p' | sed 's/^/  /'
		else
			errors=$((errors + 1))
			sed -n '1,40p' "$log_file" | sed 's/^/  /'
		fi
		echo "  log: $log_file"
	fi
done

echo
echo "Summary:"
echo "  Total  : $total"
echo "  Passed : $passed"
echo "  Failed : $failed"
echo "  Logs   : $LOG_DIR"

if [ "$errors" -ne 0 ]; then
	echo "ERROR: $errors harness(es) failed to build or run"
	exit 1
fi

if [ "$findings" -ne 0 ]; then
	if [ "$STRICT" -eq 1 ]; then
		echo "FAIL: $findings harness(es) reported leaks"
		exit 1
	fi
	echo "NOTE: $findings harness(es) reported leaks; use --strict to fail the command"
fi

exit 0
