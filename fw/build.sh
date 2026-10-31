#!/usr/bin/env bash

set -u

DO_BOOTSTRAP=0

for arg in "$@"; do
	case "$arg" in
	-bs)
		DO_BOOTSTRAP=1
		;;
	esac
done

cc -c spectrelib/csources/panic_handler.c -o spectrelib/csources/panic_handler.o || exit 1
cc -c spectrelib/csources/yyjson_shim.c -o spectrelib/csources/yyjson_shim.o || exit 1

GIT_HASH="unknown"
if command -v git >/dev/null 2>&1; then
	GIT_HASH="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
fi

echo "Building spectre stage 1..."
spectre ./src/sxc.sx -o ./spectre-stage1 || exit 1

echo "Building spectre-dev..."
./spectre-stage1 -dGIT_HASH="$GIT_HASH" ./src/sxc.sx -o spectre-dev || exit 1
rm -f ./spectre-stage1

if [ $DO_BOOTSTRAP -eq 1 ]; then
	echo "Emitting bootstrap SSA..."
	./spectre-dev -dGIT_HASH="$GIT_HASH" ./src/sxc.sx --emit-ssa >./bootstrap/sxc.ssa || exit 1
	echo "Bootstrap SSA written to ./bootstrap/sxc.ssa"
fi

echo "Done."
