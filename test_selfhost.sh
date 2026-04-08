#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples"
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
echo "Summary:"
echo "Total tests : $total"
echo "Passed      : $passed"
echo "Failed      : $failed"
echo "Skipped     : $skipped"
echo


echo "Extra tests:"
"$COMPILER" ./std/collections.sx --test

if [ $failed -ne 0 ]; then
    exit 1
fi

exit 0