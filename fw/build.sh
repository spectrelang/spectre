#!/usr/bin/env bash

set -u

DO_BOOTSTRAP=0
RELEASE=0
if [ "${SPECTRE_BUILD_RELEASE:-0}" = "1" ]; then
	RELEASE=1
fi

for arg in "$@"; do
	case "$arg" in
	-bs)
		DO_BOOTSTRAP=1
		;;
	-release)
		RELEASE=1
		;;
	esac
done

cc -c spectrelib/csources/panic_handler.c -o spectrelib/csources/panic_handler.o || exit 1
cc -c spectrelib/csources/qbe_runtime.c -o spectrelib/csources/qbe_runtime.o || exit 1

GIT_HASH="unknown"
if command -v git >/dev/null 2>&1; then
	GIT_HASH="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
fi

PROJECT_VERSION="$(sed -n 's/^[[:space:]]*version[[:space:]]*"\([^"]*\)".*/\1/p' sx.mod | head -n 1)"
DEFINES=(-d:GIT_HASH="$GIT_HASH")
if [ -n "$PROJECT_VERSION" ]; then
	DEFINES+=(-d:SPECTRE_MOD_VERSION="$PROJECT_VERSION")
fi

STAGE0="${SPECTRE_STAGE0:-}"
if [ -z "$STAGE0" ]; then
	if [ -x ./spectre-dev ]; then
		STAGE0=./spectre-dev
	else
		STAGE0=spectre
	fi
fi

# Optional backend flag, e.g. "-alt" for bootstrap environments without QBE.
BACKEND_FLAG="${SPECTRE_BUILD_BACKEND:-}"
RELEASE_FLAG=""
if [ "$RELEASE" -eq 1 ]; then
	RELEASE_FLAG="-release"
fi

echo "Building spectre stage 1..."
"$STAGE0" ./src/sxc.sx -runtime:hosted $RELEASE_FLAG $BACKEND_FLAG -o ./spectre-stage1 || exit 1

echo "Building spectre-dev..."
./spectre-stage1 "${DEFINES[@]}" ./src/sxc.sx -runtime:hosted $RELEASE_FLAG $BACKEND_FLAG -o spectre-dev || exit 1
rm -f ./spectre-stage1

if [ $DO_BOOTSTRAP -eq 1 ]; then
	echo "Emitting bootstrap SSA..."
	./spectre-dev "${DEFINES[@]}" ./src/sxc.sx -b -emit-ssa >./bootstrap/sxc.ssa || exit 1
	echo "Bootstrap SSA written to ./bootstrap/sxc.ssa"
fi

echo "Done."
