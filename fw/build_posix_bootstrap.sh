#!/usr/bin/env bash
# Build a stage0 Spectre compiler from the checked-in C bootstrap and then
# build spectre-dev from the current source. Avoids package-manager installs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP_ROOT="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
TMP_ROOT="${TMP_ROOT%/}/spectre-bootstrap"
mkdir -p "$TMP_ROOT"

BOOTSTRAP_SRC="bootstrap/sxc.c"
if [[ "$(uname -s)" == "Darwin" ]]; then
	BOOTSTRAP_SRC="bootstrap/sxc_darwin.c"
fi

echo "Building stage0 from $BOOTSTRAP_SRC..."
cc -O2 \
	"$BOOTSTRAP_SRC" \
	-w \
	-Wno-error=int-conversion \
	-o spectre-stage0
chmod +x spectre-stage0

echo "Building spectre-dev with stage0..."
SPECTRE_STAGE0="$ROOT/spectre-stage0" \
	SPECTRE_BUILD_BACKEND="-alt" \
	./fw/build.sh -release
