#!/usr/bin/env bash

set -u

SAMPLES_DIR="./samples/csamples"
COMPILER="./spectre-dev"

if [ ! -d "$SAMPLES_DIR" ]; then
	echo "Error: directory not found: $SAMPLES_DIR"
	exit 1
fi

for cfile in "$SAMPLES_DIR"/*.c; do
	[ -e "$cfile" ] || continue

	name="$(basename "$cfile" .c)"
	sxfile="$name.sx"

	echo "== Testing $name =="

	"$COMPILER" port-c "$cfile"
	if [ $? -ne 0 ]; then
		echo "[FAIL] translation failed: $name"
		echo
		continue
	fi

	"$COMPILER" "./src-out/$sxfile"
	if [ $? -ne 0 ]; then
		echo "[FAIL] compile failed: $name"
		echo
		continue
	fi

	echo "[PASS] $name"
	echo
done

bind_header="./samples/csamples/bind_demo.h"
if [ -f "$bind_header" ]; then
	echo "== Testing bind-c =="
	rm -rf "./s-out/bind-c-test"
	"$COMPILER" bind-c "$bind_header" -o "./s-out/bind-c-test" -normalize
	if [ $? -ne 0 ]; then
		echo "[FAIL] bind-c generation failed"
		exit 1
	fi
	"$COMPILER" "./s-out/bind-c-test/bind_demo.sx" -c -o "./s-out/bind-c-test/bind_demo.o"
	if [ $? -ne 0 ]; then
		echo "[FAIL] generated bindings failed to compile"
		exit 1
	fi
	echo "[PASS] bind-c"
	echo
fi
