#!/usr/bin/env bash
# Build a stage0 Spectre compiler from the checked-in C bootstrap and then
# build spectre-dev from the current source. Avoids package-manager installs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
TMP_ROOT="${TMP_ROOT%/}/spectre-bootstrap"
mkdir -p "$TMP_ROOT"

YYJSON_DIR="${YYJSON_DIR:-$TMP_ROOT/yyjson}"
if [[ ! -f "$YYJSON_DIR/build/libyyjson.a" ]]; then
	rm -rf "$YYJSON_DIR"
	git clone --depth 1 https://github.com/ibireme/yyjson.git "$YYJSON_DIR"
	cmake -S "$YYJSON_DIR" -B "$YYJSON_DIR/build" \
		-DCMAKE_BUILD_TYPE=Release \
		-DBUILD_SHARED_LIBS=OFF \
		-DYYJSON_BUILD_TESTS=OFF \
		-DYYJSON_BUILD_FUZZER=OFF \
		-DYYJSON_BUILD_MISC=OFF \
		-DYYJSON_BUILD_DOC=OFF
	cmake --build "$YYJSON_DIR/build" --config Release
fi

if [[ ! -f "$YYJSON_DIR/build/libyyjson.a" ]]; then
	echo "error: libyyjson.a not found under $YYJSON_DIR" >&2
	exit 1
fi

BOOTSTRAP_SRC="bootstrap/sxc.c"
if [[ "$(uname -s)" == "Darwin" ]]; then
	BOOTSTRAP_SRC="bootstrap/sxc_darwin.c"
fi

echo "Building stage0 from $BOOTSTRAP_SRC..."
cc -O2 \
	"$BOOTSTRAP_SRC" \
	spectrelib/csources/panic_handler.c \
	spectrelib/csources/yyjson_shim.c \
	-I "$YYJSON_DIR/src" \
	"$YYJSON_DIR/build/libyyjson.a" \
	-w \
	-Wno-error=int-conversion \
	-o spectre-stage0
chmod +x spectre-stage0

echo "Building spectre-dev with stage0..."
YYJSON_INC="-I$YYJSON_DIR/src" \
	SPECTRE_STAGE0="$ROOT/spectre-stage0" \
	SPECTRE_BUILD_BACKEND="-alt" \
	./fw/build.sh
