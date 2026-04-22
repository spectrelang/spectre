#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples"
STD_DIR="./std"
COMPILER="./spectre-dev"

BOOTSTRAP_ONLY=0
LLVM_ONLY=0

total=0
passed=0
failed=0
skipped=0

for arg in "$@"; do
    case "$arg" in
        -bs)
            BOOTSTRAP_ONLY=1
            ;;
        -ll)
            LLVM_ONLY=1
            ;;
    esac
done

if [ ! -x "$COMPILER" ]; then
    echo "ERROR: compiler not found or not executable at $COMPILER"
    exit 1
fi

run_std_tests() {
    for file in "$STD_DIR"/*.sx; do
        [ -e "$file" ] || continue

        filename=$(basename "$file")

        ((total++))

        if [[ "$filename" == std.sx ]]; then
            echo "[SKIP] $filename (this is the std facade)"
            ((skipped++))
            continue
        fi

        if [ "$LLVM_ONLY" -eq 1 ]; then
            "$COMPILER" "$file" --llvm --test > /dev/null 2>&1
        else
            "$COMPILER" "$file" --test > /dev/null 2>&1
        fi

        status=$?

        if [ $status -eq 0 ]; then
            echo "[PASS] $filename"
            ((passed++))
        else
            echo "[FAIL] $filename"
            ((failed++))
        fi
    done
}

run_samples() {
    for file in "$SAMPLES_DIR"/*.sx; do
        [ -e "$file" ] || continue

        filename=$(basename "$file")

        if [[ "$filename" == *_rt_error.sx ]]; then
            echo "[SKIP] $filename (runtime error test)"
            ((skipped++))
            continue
        fi

        ((total++))

        if [ "$LLVM_ONLY" -eq 1 ]; then
            "$COMPILER" "$file" --llvm > /dev/null 2>&1
        else
            "$COMPILER" "$file" > /dev/null 2>&1
        fi

        status=$?

        if [[ "$filename" == *_error.sx ]]; then
            if [ $status -ne 0 ]; then
                echo "[PASS] $filename (failed as expected)"
                ((passed++))
            else
                echo "[FAIL] $filename (expected failure, got success)"
                ((failed++))
            fi
        else
            if [ $status -eq 0 ]; then
                echo "[PASS] $filename"
                ((passed++))
            else
                echo "[FAIL] $filename"
                ((failed++))
            fi
        fi
    done
}

if [ $LLVM_ONLY -eq 1 ]; then
    echo "Running LLVM backend tests only..."
    run_samples
    echo
    echo "LLVM std tests:"
    run_std_tests
    echo
    echo "Self compilation tests:"
    "$COMPILER" ./src/ast/lexer.sx --test --llvm
    "$COMPILER" ./src/ast/parser.sx --test --llvm
    "$COMPILER" ./src/ast/ast_printer.sx --test --llvm
    "$COMPILER" ./src/codegen/llvm_codegen.sx --test --llvm
    "$COMPILER" ./src/codegen/codegen.sx --test --llvm
    "$COMPILER" ./src/module/module.sx --test --llvm
    echo
    echo "LLVM Test Summary:"
    echo "Total tests : $total"
    echo "Passed      : $passed"
    echo "Failed      : $failed"
    echo "Skipped     : $skipped"

    if [ $failed -ne 0 ]; then
        exit 1
    fi

    exit 0
fi

if [ $BOOTSTRAP_ONLY -eq 0 ]; then

    run_samples

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

    "$COMPILER" ./src/ast/lexer.sx -l --test
    "$COMPILER" ./src/ast/parser.sx -l --test
    "$COMPILER" ./src/sema/sema.sx -l --test
    "$COMPILER" ./src/module/module.sx -l --test
    "$COMPILER" ./src/codegen/codegen.sx -l --test
    "$COMPILER" ./src/sxc.sx -l --test

fi

echo "Bootstrap test:"
"$COMPILER" ./src/sxc.sx -l -o sxc2 || exit 1
./sxc2 ./src/sxc.sx -l -o sxc3 || exit 1
./sxc3 ./src/sxc.sx -l -o sxc4 || exit 1

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
