#!/usr/bin/env bash

set -u

COMPILER="./spectre-dev"
FIXTURES="./samples/fmt_samples"

if [ ! -x "$COMPILER" ]; then
    echo "ERROR: compiler not found or not executable at $COMPILER"
    exit 1
fi

if [ ! -d "$FIXTURES" ]; then
    echo "ERROR: formatter fixtures not found at $FIXTURES"
    exit 1
fi

tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/spectre-fmt-test.XXXXXX") || exit 1
trap 'rm -rf "$tmpdir"' EXIT

total=0
passed=0
failed=0

fail() {
    echo "[FAIL] $1"
    failed=$((failed + 1))
}

pass() {
    echo "[PASS] $1"
    passed=$((passed + 1))
}

format_file() {
    "$COMPILER" fmt "$1" > /dev/null 2>&1
}

for bad in "$FIXTURES"/bad*.sx; do
    [ -e "$bad" ] || continue

    filename=$(basename "$bad")
    suffix=${filename#bad}
    good="$FIXTURES/good$suffix"
    total=$((total + 1))

    if [ ! -f "$good" ]; then
        fail "$filename (missing good$suffix)"
        continue
    fi

    actual="$tmpdir/$filename"
    cp "$bad" "$actual"

    if ! format_file "$actual"; then
        fail "$filename (formatter exited non-zero)"
        continue
    fi

    if ! cmp -s "$actual" "$good"; then
        echo "[FAIL] $filename (output differs from good$suffix)"
        diff -u "$good" "$actual" | head -40 | sed 's/^/    /'
        failed=$((failed + 1))
        continue
    fi

    if ! format_file "$actual" || ! cmp -s "$actual" "$good"; then
        fail "$filename (formatter is not idempotent)"
        continue
    fi

    pass "$filename"
done

# Formatting a directory must process the .sx files and ignore other files.
total=$((total + 1))
tree="$tmpdir/tree"
mkdir -p "$tree/sub"

for bad in "$FIXTURES"/bad*.sx; do
    [ -e "$bad" ] || continue
    cp "$bad" "$tree/"
done

printf 'this is not a spectre source file\n' > "$tree/readme.txt"
printf 'pub fn nested() void = {}\n' > "$tree/sub/nested.txt"

if ! "$COMPILER" fmt "$tree" > /dev/null 2>&1; then
    fail "directory mode (formatter exited non-zero)"
else
    dir_ok=1

    for bad in "$FIXTURES"/bad*.sx; do
        [ -e "$bad" ] || continue

        filename=$(basename "$bad")
        suffix=${filename#bad}
        good="$FIXTURES/good$suffix"

        if ! cmp -s "$tree/$filename" "$good"; then
            echo "[FAIL] directory mode ($filename differs from good$suffix)"
            diff -u "$good" "$tree/$filename" | head -40 | sed 's/^/    /'
            dir_ok=0
            break
        fi
    done

    if [ $dir_ok -eq 1 ]; then
        pass "directory mode"
    else
        failed=$((failed + 1))
    fi
fi

echo
echo "Formatter Test Summary:"
echo "Total tests : $total"
echo "Passed      : $passed"
echo "Failed      : $failed"
echo

if [ $failed -ne 0 ]; then
    exit 1
fi

exit 0
