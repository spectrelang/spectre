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

YYJSON_INC="${YYJSON_INC:-}"
YYJSON_FOUND=0

if [ -n "$YYJSON_INC" ]; then
	YYJSON_FOUND=1
elif [ -n "${YYJSON_DIR:-}" ] && [ -f "$YYJSON_DIR/src/yyjson.h" ]; then
	YYJSON_INC="-I$YYJSON_DIR/src"
	YYJSON_FOUND=1
elif [ -f "$HOME/.local/src/yyjson/src/yyjson.h" ]; then
	YYJSON_INC="-I$HOME/.local/src/yyjson/src"
	YYJSON_FOUND=1
fi

if [ "$YYJSON_FOUND" -eq 0 ] && command -v pkg-config >/dev/null 2>&1; then
	if pkg-config --exists yyjson 2>/dev/null; then
		YYJSON_INC="$(pkg-config --cflags yyjson 2>/dev/null || true)"
		YYJSON_FOUND=1
	fi
fi

if [ "$YYJSON_FOUND" -eq 0 ]; then
	for prefix in /opt/homebrew /usr/local; do
		if [ -f "$prefix/include/yyjson.h" ]; then
			YYJSON_INC="-I$prefix/include"
			YYJSON_FOUND=1
			break
		fi
	done
fi

if [ "$YYJSON_FOUND" -eq 0 ]; then
	echo "error: could not find yyjson.h" >&2
	echo "       expected $HOME/.local/src/yyjson/src/yyjson.h" >&2
	echo "       run ./fw/install.sh first, or set YYJSON_DIR / YYJSON_INC" >&2
	exit 1
fi

cc -c spectrelib/csources/panic_handler.c -o spectrelib/csources/panic_handler.o || exit 1
cc $YYJSON_INC -c spectrelib/csources/yyjson_shim.c -o spectrelib/csources/yyjson_shim.o || exit 1
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

echo "Building spectre stage 1..."
"$STAGE0" ./src/sxc.sx $BACKEND_FLAG -o ./spectre-stage1 || exit 1

echo "Building spectre-dev..."
./spectre-stage1 "${DEFINES[@]}" ./src/sxc.sx $BACKEND_FLAG -o spectre-dev || exit 1
rm -f ./spectre-stage1

if [ $DO_BOOTSTRAP -eq 1 ]; then
	echo "Emitting bootstrap SSA..."
	./spectre-dev "${DEFINES[@]}" ./src/sxc.sx -b -emit-ssa >./bootstrap/sxc.ssa || exit 1
	echo "Bootstrap SSA written to ./bootstrap/sxc.ssa"
fi

echo "Done."
