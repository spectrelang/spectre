#!/usr/bin/env bash

set -u

COMPILER="./spectre-dev"
FIXTURES="./samplesmisc/criticsamples"

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
	"variable name 'bad_const' should be SCREAMING_SNAKE_CASE"
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

if grep -Fq "GOOD_CONST" <<<"$output"; then
	fail "unexpected diagnostic for SCREAMING_SNAKE_CASE constant"
fi

if [ $failed -eq 0 ]; then
	pass "diagnostics"
fi

# Directory targets with a trailing slash must not produce doubled separators.
trailing_output=$("$COMPILER" critic "$FIXTURES/" 2>&1)
if grep -Fq "criticsamples//" <<<"$trailing_output"; then
	fail "trailing slash (double slash in reported paths)"
else
	pass "trailing slash"
fi

actual="$tmpdir/fix_bad.sx"
cp "$FIXTURES/fix_bad.sx" "$actual"

if ! "$COMPILER" critic -fix "$actual" >/dev/null 2>&1; then
	fail "fix mode (expected zero exit code after fixes)"
elif ! cmp -s "$actual" "$FIXTURES/fix_good.sx"; then
	echo "[FAIL] fix mode (output differs from fix_good.sx)"
	diff -u "$FIXTURES/fix_good.sx" "$actual" | head -40 | sed 's/^/    /'
	failed=$((failed + 1))
else
	pass "fix mode"
fi

# Mutable bindings that are never mutated must lose the 'mut' qualifier,
# while bindings that are assigned or incremented keep it.
mut_actual="$tmpdir/mut_bad.sx"
cp "$FIXTURES/mut_bad.sx" "$mut_actual"

if ! "$COMPILER" critic -fix "$mut_actual" >/dev/null 2>&1; then
	fail "mut fix mode (expected zero exit code after fixes)"
elif ! cmp -s "$mut_actual" "$FIXTURES/mut_good.sx"; then
	echo "[FAIL] mut fix mode (output differs from mut_good.sx)"
	diff -u "$FIXTURES/mut_good.sx" "$mut_actual" | head -40 | sed 's/^/    /'
	failed=$((failed + 1))
elif ! "$COMPILER" "$mut_actual" -syntax-check >/dev/null 2>&1; then
	fail "mut fix mode (fixed file does not compile)"
else
	pass "mut fix mode"
fi

mut_calls_actual="$tmpdir/mut_calls_bad.sx"
cp "$FIXTURES/mut_calls_bad.sx" "$mut_calls_actual"

if ! "$COMPILER" critic -fix "$mut_calls_actual" >/dev/null 2>&1; then
	fail "mut calls fix mode (expected zero exit code after fixes)"
elif ! cmp -s "$mut_calls_actual" "$FIXTURES/mut_calls_good.sx"; then
	echo "[FAIL] mut calls fix mode (output differs from mut_calls_good.sx)"
	diff -u "$FIXTURES/mut_calls_good.sx" "$mut_calls_actual" | head -40 | sed 's/^/    /'
	failed=$((failed + 1))
elif ! "$COMPILER" "$mut_calls_actual" -syntax-check >/dev/null 2>&1; then
	fail "mut calls fix mode (fixed file does not compile)"
else
	pass "mut calls fix mode"
fi

mut_regression="$tmpdir/mut_regression"
mkdir -p "$mut_regression"
cp ./spectrelib/collections/core.sx "$mut_regression/collections.sx"
cp ./spectrelib/net/core.sx "$mut_regression/net.sx"
cp ./spectrelib/algorithms/core.sx "$mut_regression/algorithms.sx"
"$COMPILER" critic -fix "$mut_regression" >/dev/null 2>&1
if ! grep -Fq "val m: mut Map" "$mut_regression/collections.sx" ||
! grep -Fq "val big: mut Map" "$mut_regression/collections.sx" ||
! grep -Fq "val list3: mut ref void" "$mut_regression/collections.sx" ||
! grep -Fq "val free_loop: mut EventLoop" "$mut_regression/net.sx" ||
! grep -Fq "val should_break: mut i32" "$mut_regression/algorithms.sx"; then
	echo "[FAIL] mut regression (critic removed a required mut qualifier)"
	failed=$((failed + 1))
else
	pass "mut regression"
fi

# Nested trust statements must be removed, including same-line nested trust.
trust_actual="$tmpdir/trust_bad.sx"
cp "$FIXTURES/trust_bad.sx" "$trust_actual"

if ! "$COMPILER" critic -fix "$trust_actual" >/dev/null 2>&1; then
	fail "trust fix mode (expected zero exit code after fixes)"
elif ! cmp -s "$trust_actual" "$FIXTURES/trust_good.sx"; then
	echo "[FAIL] trust fix mode (output differs from trust_good.sx)"
	diff -u "$FIXTURES/trust_good.sx" "$trust_actual" | head -40 | sed 's/^/    /'
	failed=$((failed + 1))
else
	pass "trust fix mode"
fi

# Regression: a named type inferred for a 'ref void' edit used to leave a
# dangling pointer that made apply_edits call strlen on a freed string.
crash_dir="$tmpdir/ref_void_crash"
mkdir -p "$crash_dir"
cat >"$crash_dir/crash.sx" <<'EOF'
type Thing = {
    value: mut i32
}

fn f(t: Thing) void = {
    val p: ref void = t
}
EOF

if ! "$COMPILER" critic -fix "$crash_dir" >/dev/null 2>&1; then
	fail "ref void fix mode (critic crashed or failed)"
elif ! grep -Fq "val p: Thing" "$crash_dir/crash.sx"; then
	fail "ref void fix mode (expected inferred type)"
else
	pass "ref void fix mode"
fi

# Regression: critic -fix used to segfault on the compiler source tree.
src_copy="$tmpdir/src"
cp -a ./src "$src_copy"
"$COMPILER" critic -fix "$src_copy" >/dev/null 2>&1
src_status=$?
if [ $src_status -ge 128 ]; then
	fail "critic -fix src (crashed with status $src_status)"
else
	pass "critic -fix src"
fi

tree="$tmpdir/tree"
mkdir -p "$tree/sub"
cp "$FIXTURES/fix_bad.sx" "$tree/sub/fix_bad.sx"

if ! "$COMPILER" critic -fix "$tree" >/dev/null 2>&1; then
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
