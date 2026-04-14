#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples"
STD_DIR="./std"
COMPILER="./s-out/sxc"

total=0
passed=0
failed=0
skipped=0

if [ ! -x "$COMPILER" ]; then
    echo "ERROR: compiler not found or not executable at $COMPILER"
    exit 1
fi

for file in "$SAMPLES_DIR"/*.sx; do
    [ -e "$file" ] || continue

    filename=$(basename "$file")

    if [[ "$filename" == *_error.sx ]]; then
        echo "[SKIP] $filename (expected failure)"
        ((skipped++))
        continue
    fi

    ((total++))

    "$COMPILER" "$file" > /dev/null 2>&1
    status=$?

    if [ $status -eq 0 ]; then
        echo "[PASS] $filename"
        ((passed++))
    else
        echo "[FAIL] $filename"
        ((failed++))
    fi
done

echo
echo "Extra tests:"

for file in "$STD_DIR"/*.sx; do
    [ -e "$file" ] || continue

    filename=$(basename "$file")

    ((total++))

    if [[ "$filename" == std.sx ]]; then
        echo "[SKIP] $filename (this is the std facade)"
        ((skipped++))
        continue
    fi

    "$COMPILER" "$file" --test > /dev/null 2>&1
    status=$?

    if [ $status -eq 0 ]; then
        echo "[PASS] $filename"
        ((passed++))
    else
        echo "[FAIL] $filename"
        ((failed++))
    fi
done

"$COMPILER" ./self/lexer.sx --test
"$COMPILER" ./self/parser.sx --test
"$COMPILER" ./self/sema.sx --test
"$COMPILER" ./self/module.sx --test
"$COMPILER" ./self/codegen.sx --test
"$COMPILER" ./self/sxc.sx --test

echo
echo "Final Summary:"
echo "Total tests : $total"
echo "Passed      : $passed"
echo "Failed      : $failed"
echo "Skipped     : $skipped"
echo

if [ $failed -ne 0 ]; then
    exit 1
fi

exit 0
