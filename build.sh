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

echo "Compiling debug runtime (panic_handler)..."
cc -c std/csources/panic_handler.c -o std/csources/panic_handler.o || exit 1

echo "Building spectre-dev..."
spectre ./src/sxc.sx -o spectre-dev || exit 1

if [ $DO_BOOTSTRAP -eq 1 ]; then
    echo "Emitting bootstrap SSA..."
    spectre ./src/sxc.sx --emit-qbe --quiet > ./bootstrap/sxc.ssa || exit 1
    echo "Bootstrap SSA written to ./bootstrap/sxc.ssa"
fi

echo "Done."
