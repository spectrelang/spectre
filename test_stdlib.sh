#!/usr/bin/env bash

set -u

SPECTRE="./target/release/spectre"
STD_DIR="./std"

failures=0

for file in "$STD_DIR"/*.sx; do
echo "Testing $file..."

"$SPECTRE" "$file" --test > /dev/null 2>&1
exit_code=$?

if [ $exit_code -ne 0 ]; then
    echo "FAILED: $file (exit code $exit_code)"
    failures=$((failures + 1))
else
    echo "OK: $file"
fi

done

echo

if [ $failures -ne 0 ]; then
echo "Total failures: $failures"
exit 1
else
echo "All tests passed"
exit 0
fi
