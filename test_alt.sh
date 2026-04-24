#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples"
STD_DIR="./std"
COMPILER="./spectre-dev"

total=0
passed=0
failed=0
skipped=0

if [ ! -x "$COMPILER" ]; then
    echo "ERROR: compiler not found or not executable at $COMPILER"
    exit 1
fi

run_compiler() {
    "$COMPILER" "$@" > /dev/null 2>&1
    return $?
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

        run_compiler "$file" --alt
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

run_std_tests() {
    for file in "$STD_DIR"/*.sx; do
        [ -e "$file" ] || continue

        filename=$(basename "$file")

        if [[ "$filename" == std.sx ]]; then
            echo "[SKIP] $filename (std facade)"
            ((skipped++))
            continue
        fi

        ((total++))

        run_compiler "$file" --test --alt
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

echo "Running C backend tests (--alt)..."
echo

run_samples

echo
echo "STD tests:"
run_std_tests

echo
echo "Self compilation tests:"

# self_tests=(
#     "./src/ast/lexer.sx"
#     "./src/ast/parser.sx"
#     "./src/ast/ast_printer.sx"
#     "./src/sema/sema.sx"
#     "./src/module/module.sx"
#     "./src/codegen/codegen.sx"
#     "./src/sxc.sx"
# )
# 
# for file in "${self_tests[@]}"; do
#     ((total++))
# 
#     run_compiler "$file" --test --alt
#     status=$?
# 
#     if [ $status -eq 0 ]; then
#         echo "[PASS] $(basename "$file")"
#         ((passed++))
#     else
#         echo "[FAIL] $(basename "$file")"
#         ((failed++))
#     fi
# done

# echo
# echo "Bootstrap test:"

# run_compiler ./src/sxc.sx -o sxc2 --alt || exit 1
# ./sxc2 ./src/sxc.sx -o sxc3 --alt || exit 1
# ./sxc3 ./src/sxc.sx -o sxc4 --alt || exit 1

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
