#!/usr/bin/env bash

set -u

COMPILER="./spectre-dev"
FIXTURES="./criticsamples"

if [ ! -x "$COMPILER" ]; then
	echo "ERROR: compiler not found or not executable at $COMPILER"
	exit 1
fi

if [ ! -d "$FIXTURES" ]; then
	echo "ERROR: critic fixtures not found at $FIXTURES"
	exit 1
fi

tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/spectre-critic-test.XXXXXX") || exit 1
trap 'rm -rf "$tmpdir"' EXIT

failed=0

fail() {
	echo "[FAIL] $1"
	failed=$((failed + 1))
}

pass() {
	echo "[PASS] $1"
}

output=$("$COMPILER" critic "$FIXTURES/checks.sx" 2>&1)
status=$?

if [ $status -eq 0 ]; then
	fail "checks.sx (expected non-zero exit code)"
fi

expected_messages=(
	"unused import 'unused'"
	"function name 'BadName' should be snake_case"
	"variable name 'Bad_var' should be snake_case"
	"avoid 'ref void'"
	"self-assignment has no effect"
	"redundant arithmetic assignment"
	"duplicate condition in boolean expression"
	"empty else body"
	"type name 'bad_type' should be PascalCase"
	"variable name 'Field' should be snake_case"
	"empty function body for 'empty'"
	"empty if body"
	"empty none arm"
)

for message in "${expected_messages[@]}"; do
	if ! grep -Fq "$message" <<<"$output"; then
		fail "missing diagnostic: $message"
	fi
done

if [ $failed -eq 0 ]; then
	pass "diagnostics"
fi

actual="$tmpdir/fix_bad.sx"
cp "$FIXTURES/fix_bad.sx" "$actual"

if ! "$COMPILER" critic --fix "$actual" >/dev/null 2>&1; then
	fail "fix mode (expected zero exit code after fixes)"
elif ! cmp -s "$actual" "$FIXTURES/fix_good.sx"; then
	echo "[FAIL] fix mode (output differs from fix_good.sx)"
	diff -u "$FIXTURES/fix_good.sx" "$actual" | head -40 | sed 's/^/    /'
	failed=$((failed + 1))
else
	pass "fix mode"
fi

tree="$tmpdir/tree"
mkdir -p "$tree/sub"
cp "$FIXTURES/fix_bad.sx" "$tree/sub/fix_bad.sx"

if ! "$COMPILER" critic --fix "$tree" >/dev/null 2>&1; then
	fail "directory fix mode (expected zero exit code after fixes)"
elif ! cmp -s "$tree/sub/fix_bad.sx" "$FIXTURES/fix_good.sx"; then
	echo "[FAIL] directory fix mode (output differs from fix_good.sx)"
	diff -u "$FIXTURES/fix_good.sx" "$tree/sub/fix_bad.sx" | head -40 | sed 's/^/    /'
	failed=$((failed + 1))
else
	pass "directory fix mode"
fi

echo
echo "Critic Test Summary:"
if [ $failed -ne 0 ]; then
	echo "Failed: $failed"
	exit 1
fi
echo "Passed: all"
exit 0
