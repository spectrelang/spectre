#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples"
STD_DIR="./std"
COMPILER="./spectre-dev"
BOOTSTRAP_ONLY=0

total=0
passed=0
failed=0
skipped=0


for arg in "$@"; do
    case "$arg" in
        -bs)
            BOOTSTRAP_ONLY=1
            ;;
    esac
done

if [ ! -x "$COMPILER" ]; then
    echo "ERROR: compiler not found or not executable at $COMPILER"
    exit 1
fi

if [ $BOOTSTRAP_ONLY -eq 0 ]; then

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

"$COMPILER" ./src/ast/lexer.sx -q --test
"$COMPILER" ./src/ast/parser.sx -q --test
"$COMPILER" ./src/sema/sema.sx -q --test
"$COMPILER" ./src/module/module.sx -q --test
"$COMPILER" ./src/codegen/codegen.sx -q --test
"$COMPILER" ./src/sxc.sx -q --test

fi

echo "Bootstrap test:"
"$COMPILER" ./src/sxc.sx -o sxc2 || exit 1
./sxc2 ./src/sxc.sx -o sxc3 || exit 1
./sxc3 ./src/sxc.sx -o sxc4 || exit 1

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
