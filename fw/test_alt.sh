#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples"
STD_DIR="./spectrelib"
COMPILER="./spectre-dev"
BOOTSTRAP_ONLY=0

for arg in "$@"; do
    case "$arg" in
        -bs)
            BOOTSTRAP_ONLY=1
            ;;
    esac
done

total=0
passed=0
failed=0
skipped=0

if [ ! -x "$COMPILER" ]; then
    echo "ERROR: compiler not found or not executable at $COMPILER"
    exit 1
fi

LOG_DIR="${TEST_LOG_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/spectre-test-logs}"
mkdir -p "$LOG_DIR"

LINK_ARGS=()
if [ "${SPECTRE_TEST_DYNAMIC:-0}" = "1" ]; then
    LINK_ARGS+=(--dynamic)
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

        if [[ "$filename" == *_rv.sx ]]; then
            ((total++))

            "$COMPILER" run "$file" > /dev/null 2>&1
            status=$?

            if [ $status -eq 0 ]; then
                echo "[PASS] $filename (runtime check)"
                ((passed++))
            else
                echo "[FAIL] $filename (runtime check)"
                ((failed++))
            fi

            continue
        fi

        ((total++))

        extra_args=()

        if [[ "$filename" == "sample_fps.sx" ]]; then
            extra_args+=(--inc-path=/usr/include/SDL2/)
        fi

        if [[ "$filename" == "sample100.sx" ]]; then
            extra_args+=(--inc-path=/usr/include/SDL2/)
        fi

        if [[ "$filename" == "sample167.sx" ]]; then
            extra_args+=(--dynamic)
        fi

        if [ ${#extra_args[@]} -gt 0 ]; then
            run_compiler "$file" --alt ${LINK_ARGS[@]+"${LINK_ARGS[@]}"} "${extra_args[@]}"
        else
            run_compiler "$file" --alt ${LINK_ARGS[@]+"${LINK_ARGS[@]}"}
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

run_std_tests() {
    find "$STD_DIR" -type f -name "*.sx" | sort | while IFS= read -r file; do
        filename=$(basename "$file")
        log_name="${file#./}"
        log_name="${log_name//\//_}"
        log_file="$LOG_DIR/std_${log_name}.log"

        ((total++))

        "$COMPILER" "$file" --test --alt ${LINK_ARGS[@]+"${LINK_ARGS[@]}"} >"$log_file" 2>&1 < /dev/null
        status=$?

        if [ $status -eq 0 ]; then
            echo "[PASS] $file"
            ((passed++))
        else
            echo "[FAIL] $file"
            echo "  command: $COMPILER $file --test --alt ${LINK_ARGS[*]+"${LINK_ARGS[*]}"}"
            echo "  log: $log_file"
            sed 's/^/  /' "$log_file" | sed -n '1,240p'
            log_lines=$(wc -l < "$log_file")
            if [ "$log_lines" -gt 240 ]; then
                echo "  ... ($log_lines lines total; see $log_file)"
            fi
            ((failed++))
        fi
    done
}

echo "Running C backend tests (--alt)..."

if [ $BOOTSTRAP_ONLY -eq 0 ]; then
    echo

    run_samples

    echo
    echo "STD tests:"
    run_std_tests

    named_test_src="$LOG_DIR/named_test_output.sx"
    cat >"$named_test_src" <<'EOF'
test "addition works" {
    assert 1 + 1 == 2
}
EOF
    ((total++))
    named_test_out=$("$COMPILER" "$named_test_src" --test --alt 2>&1)
    if printf '%s' "$named_test_out" | grep -q '\[pass\] addition works'; then
        echo "[PASS] named test output"
        ((passed++))
    else
        echo "[FAIL] named test output"
        echo "$named_test_out" | sed 's/^/  /'
        ((failed++))
    fi
fi

echo
echo "Bootstrap test:"

((total++))
"$COMPILER" ./src/sxc.sx -o sxc2 --alt -b -l || { echo "[FAIL] stage1"; ((failed++)); exit 1; }

((total++))
./sxc2 ./src/sxc.sx -o sxc3 --alt -l || { echo "[FAIL] stage2"; ((failed++)); exit 1; }

((total++))
./sxc3 ./src/sxc.sx -o sxc4 --alt -l || { echo "[FAIL] stage3"; ((failed++)); exit 1; }

echo "[PASS] bootstrap"

echo
echo "Final Summary:"
echo "Total tests : $total"
echo "Passed      : $passed"
echo "Failed      : $failed"
echo "Skipped     : $skipped"
echo "Test logs   : $LOG_DIR"

echo

if [ $failed -ne 0 ]; then
    exit 1
fi

exit 0
