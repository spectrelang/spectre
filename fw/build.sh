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

GIT_HASH="unknown"
if command -v git >/dev/null 2>&1; then
	GIT_HASH="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
fi

STAGE0="./spectre-stage0"
if ! command -v spectre >/dev/null 2>&1; then
	echo "error: could not find a spectre bootstrap compiler in PATH" >&2
	exit 1
fi
cp "$(command -v spectre)" "$STAGE0" || exit 1
chmod +x "$STAGE0"

echo "Building spectre stage 1..."
"$STAGE0" ./src/sxc.sx -o ./spectre-stage1 || {
	rm -f "$STAGE0"
	exit 1
}
rm -f "$STAGE0"

echo "Building spectre-dev..."
./spectre-stage1 -dGIT_HASH="$GIT_HASH" ./src/sxc.sx -o spectre-dev || exit 1
rm -f ./spectre-stage1

if [ $DO_BOOTSTRAP -eq 1 ]; then
	echo "Emitting bootstrap SSA..."
	./spectre-dev -dGIT_HASH="$GIT_HASH" ./src/sxc.sx --emit-ssa >./bootstrap/sxc.ssa || exit 1
	echo "Bootstrap SSA written to ./bootstrap/sxc.ssa"
fi

echo "Done."
