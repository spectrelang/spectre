#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ ! -x ./spectre-dev ]; then
    echo "error: ./spectre-dev is missing or not executable" >&2
    exit 1
fi

GIT_HASH="unknown"
if command -v git >/dev/null 2>&1; then
    GIT_HASH="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
fi

PROJECT_VERSION="$(sed -n 's/^[[:space:]]*version[[:space:]]*"\([^"]*\)".*/\1/p' sx.mod | head -n 1)"
DEFINES=(-d:GIT_HASH="$GIT_HASH")
if [ -n "$PROJECT_VERSION" ]; then
    DEFINES+=(-d:SPECTRE_MOD_VERSION="$PROJECT_VERSION")
fi

mkdir -p bootstrap

# Write through a temporary file so a failed generation never replaces a
# checked-in bootstrap file with a partial artifact or compiler diagnostics.
emit() {
    local target="$1"
    shift
    local tmp="${target}.tmp.$$"
    if "$@" >"$tmp"; then
        mv "$tmp" "$target"
    else
        rm -f "$tmp"
        return 1
    fi
}

emit bootstrap/sxc.ssa \
    ./spectre-dev "${DEFINES[@]}" ./src/sxc.sx \
    -emit-ssa -emit-shim:spectrelib/csources/qbe_runtime.c -src-target:linux
emit bootstrap/sxc.c \
    ./spectre-dev "${DEFINES[@]}" ./src/sxc.sx \
    -b -emit-alt -src-target:linux
emit bootstrap/sxc_darwin.ssa \
    ./spectre-dev "${DEFINES[@]}" ./src/sxc.sx \
    -emit-ssa -src-target:darwin
emit bootstrap/sxc_darwin.c \
    ./spectre-dev "${DEFINES[@]}" ./src/sxc.sx \
    -b -emit-alt -src-target:darwin
emit bootstrap/sxcw.c \
    ./spectre-dev "${DEFINES[@]}" ./src/sxc.sx \
    -b -emit-alt -src-target:windows
