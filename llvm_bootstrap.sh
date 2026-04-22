#!/usr/bin/env bash

set -u

echo "Preparing bootstrap directory..."
mkdir -p ./bootstrap || exit 1

echo "Compiling C support objects..."
cc -c std/csources/panic_handler.c -o std/csources/panic_handler.o || exit 1
cc -c std/csources/yyjson_shim.c -o std/csources/yyjson_shim.o || exit 1

echo "Emitting LLVM IR bootstrap (sxc.ll)..."
./spectre-dev ./src/sxc.sx --llvm --emit-ll > ./bootstrap/sxc.ll || exit 1

echo "LLVM bootstrap written to ./bootstrap/sxc.ll"
echo "Done."
