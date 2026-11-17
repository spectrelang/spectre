#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples"
STD_DIR="./spectrelib"
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

echo "Freestanding runtime audit:"
bash ./fw/audit_freestanding.sh || exit 1
echo

LOG_DIR="${TEST_LOG_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/spectre-test-logs}"
mkdir -p "$LOG_DIR"

LINK_ARGS=()
if [ "${SPECTRE_TEST_DYNAMIC:-0}" = "1" ]; then
	LINK_ARGS+=(-dynamic)
fi

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

			"$COMPILER" run "$file" >/dev/null 2>&1
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

		if [[ "$filename" == "sample167.sx" ]]; then
			extra_args+=(-dynamic)
		fi

		if [ ${#extra_args[@]} -gt 0 ]; then
			"$COMPILER" "$file" ${LINK_ARGS[@]+"${LINK_ARGS[@]}"} "${extra_args[@]}" >/dev/null 2>&1
		else
			"$COMPILER" "$file" ${LINK_ARGS[@]+"${LINK_ARGS[@]}"} >/dev/null 2>&1
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
	while IFS= read -r file; do
		filename=$(basename "$file")
		log_name="${file#./}"
		log_name="${log_name//\//_}"
		log_file="$LOG_DIR/std_${log_name}.log"

		((total++))

		std_link_args=("${LINK_ARGS[@]+"${LINK_ARGS[@]}"}")
		std_extra_args=()
		# runtime/sys.sx implements the freestanding syscall layer and cannot be
		# linked against the hosted C runtime.
		if [[ "$file" == *"runtime/sys.sx" ]]; then
			std_link_args=()
			std_extra_args+=(-runtime:freestanding)
		fi

		"$COMPILER" test "$file" ${std_link_args[@]+"${std_link_args[@]}"} ${std_extra_args[@]+"${std_extra_args[@]}"} >"$log_file" 2>&1 </dev/null
		status=$?

		if [ $status -eq 0 ]; then
			echo "[PASS] $file"
			((passed++))
		else
			echo "[FAIL] $file"
			echo "  command: $COMPILER test $file ${std_link_args[*]+"${std_link_args[*]}"} ${std_extra_args[*]+"${std_extra_args[*]}"}"
			echo "  log: $log_file"
			sed 's/^/  /' "$log_file" | sed -n '1,240p'
			log_lines=$(wc -l <"$log_file")
			if [ "$log_lines" -gt 240 ]; then
				echo "  ... ($log_lines lines total; see $log_file)"
			fi
			((failed++))
		fi
	done < <(find "$STD_DIR" -type f -name "*.sx" | sort)
}

run_sample_app_build() {
	local config="$1"
	local label="$2"
	local sample_dir="./samples/sample_app"
	local log_file="$LOG_DIR/sample_app_${label}.log"
	local compiler_abs
	compiler_abs="$(cd "$(dirname "$COMPILER")" && pwd)/$(basename "$COMPILER")"

	((total++))
	if (cd "$sample_dir" && "$compiler_abs" build $config && ./some_app) >"$log_file" 2>&1; then
		if grep -q 'name: spectre' "$log_file"; then
			echo "[PASS] sample_app ($label)"
			((passed++))
		else
			echo "[FAIL] sample_app ($label) produced unexpected output"
			echo "  log: $log_file"
			sed 's/^/  /' "$log_file" | sed -n '1,240p'
			((failed++))
		fi
	else
		echo "[FAIL] sample_app ($label)"
		echo "  command: cd $sample_dir && $compiler_abs build $config && ./some_app"
		echo "  log: $log_file"
		sed 's/^/  /' "$log_file" | sed -n '1,240p'
		((failed++))
	fi
}

run_multi_actions_app_build() {
	local config="$1"
	local label="$2"
	local binary="$3"
	local sample_dir="./samples/multi_actions_app"
	local log_file="$LOG_DIR/multi_actions_app_${label}.log"
	local compiler_abs
	compiler_abs="$(cd "$(dirname "$COMPILER")" && pwd)/$(basename "$COMPILER")"

	((total++))
	if (cd "$sample_dir" && rm -rf .sxb && SPECTRE_BIN="$compiler_abs" "$compiler_abs" build "$config" && "./$binary") >"$log_file" 2>&1; then
		if grep -q 'multi_actions_app ok' "$log_file" \
			&& grep -q 'dep-first:list-form' "$log_file" \
			&& grep -q 'dep-second:free-form' "$log_file" \
			&& grep -q "prepare:$label" "$log_file"; then
			echo "[PASS] multi_actions_app ($label)"
			((passed++))
		else
			echo "[FAIL] multi_actions_app ($label) produced unexpected output"
			echo "  log: $log_file"
			sed 's/^/  /' "$log_file" | sed -n '1,240p'
			((failed++))
		fi
	else
		echo "[FAIL] multi_actions_app ($label)"
		echo "  command: cd $sample_dir && $compiler_abs build $config && ./$binary"
		echo "  log: $log_file"
		sed 's/^/  /' "$log_file" | sed -n '1,240p'
		((failed++))
	fi
}

run_multi_actions_app_cross_out() {
	local sample_dir="./samples/multi_actions_app"
	local log_file="$LOG_DIR/multi_actions_app_cross_out.log"
	local compiler_abs
	compiler_abs="$(cd "$(dirname "$COMPILER")" && pwd)/$(basename "$COMPILER")"

	((total++))
	if (cd "$sample_dir" && rm -rf .sxb && SPECTRE_BIN="$compiler_abs" "$compiler_abs" build artifacts -target:windows.x86_64 \
		&& grep -q '^cross-target$' .sxb/windows.x86_64/artifact.txt \
		&& grep -q '^cross-target-free$' .sxb/windows.x86_64/artifact_free.txt) >"$log_file" 2>&1; then
		echo "[PASS] multi_actions_app (cross target output)"
		((passed++))
	else
		echo "[FAIL] multi_actions_app (cross target output)"
		echo "  command: cd $sample_dir && $compiler_abs build artifacts -target:windows.x86_64"
		echo "  log: $log_file"
		sed 's/^/  /' "$log_file" | sed -n '1,240p'
		((failed++))
	fi
}

run_sxmod_syntax_tests() {
	local root="./samples/sxmod_syntax"
	local compiler_abs
	compiler_abs="$(cd "$(dirname "$COMPILER")" && pwd)/$(basename "$COMPILER")"
	local dir name log_file status expected out

	for dir in "$root"/*; do
		[ -d "$dir" ] || continue
		name=$(basename "$dir")
		log_file="$LOG_DIR/sxmod_${name}.log"
		((total++))

		out=$(cd "$dir" && "$compiler_abs" build dev 2>&1)
		status=$?
		printf '%s\n' "$out" >"$log_file"

		if [ -f "$dir/valid" ]; then
			if [ $status -eq 0 ]; then
				echo "[PASS] sx.mod $name"
				((passed++))
			else
				echo "[FAIL] sx.mod $name (expected success)"
				sed 's/^/  /' "$log_file" | sed -n '1,120p'
				((failed++))
			fi
			rm -rf "$dir/.sxb" "$dir/app"
			continue
		fi

		expected=$(sed -n '1p' "$dir/expected.txt" 2>/dev/null || true)
		if [ $status -eq 0 ]; then
			echo "[FAIL] sx.mod $name (expected a syntax error)"
			((failed++))
		elif [ -z "$expected" ] || ! printf '%s' "$out" | grep -Fq -- "$expected"; then
			echo "[FAIL] sx.mod $name (missing diagnostic: $expected)"
			sed 's/^/  /' "$log_file" | sed -n '1,120p'
			((failed++))
		elif ! printf '%s' "$out" | grep -Fq -- "--> sx.mod:"; then
			echo "[FAIL] sx.mod $name (diagnostic has no sx.mod source location)"
			sed 's/^/  /' "$log_file" | sed -n '1,120p'
			((failed++))
		else
			echo "[PASS] sx.mod $name"
			((passed++))
		fi
	done
}

if [ $BOOTSTRAP_ONLY -eq 0 ]; then
	run_samples

	echo
	echo "Extra tests:"
	run_std_tests

	named_test_src="./samples/sample226.sx"
	((total++))
	named_test_out=$("$COMPILER" test "$named_test_src" 2>&1)
	if printf '%s' "$named_test_out" | grep -q '\[pass\] hello'; then
		echo "[PASS] named test output"
		((passed++))
	else
		echo "[FAIL] named test output"
		echo "$named_test_out" | sed 's/^/  /'
		((failed++))
	fi

	((total++))
	help_graph_out=$("$COMPILER" help graph 2>&1)
	if printf '%s' "$help_graph_out" | grep -q '^graph - Visualize the project dependency tree'; then
		echo "[PASS] help graph"
		((passed++))
	else
		echo "[FAIL] help graph"
		echo "$help_graph_out" | sed 's/^/  /'
		((failed++))
	fi

	run_sample_app_build "" "dev"
	run_sample_app_build "release" "release"
	run_multi_actions_app_build "release" "release" "multi_actions_app"
	run_multi_actions_app_build "free" "free" "multi_actions_app_free"
	run_multi_actions_app_cross_out

	"$COMPILER" test ./src/ast/lexer.sx -l
	"$COMPILER" test ./src/ast/parser.sx -l
	"$COMPILER" test ./src/sema/sema.sx -l
	"$COMPILER" test ./src/module/module.sx -l
	"$COMPILER" test ./src/codegen/codegen.sx -l
	"$COMPILER" test ./src/sxc.sx -l
fi

echo "Bootstrap test:"
"$COMPILER" ./src/sxc.sx -l -o sxc2 || exit 1
./sxc2 ./src/sxc.sx -l -o sxc3 || exit 1
./sxc3 ./src/sxc.sx -l -o sxc4 || exit 1

echo "Alt-to-QBE bootstrap test:"
"$COMPILER" ./src/sxc.sx -l -alt -o sxc2_alt || exit 1
./sxc2_alt ./src/sxc.sx -l -o sxc3_from_alt || exit 1

echo
echo "sx.mod syntax tests:"
run_sxmod_syntax_tests

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
