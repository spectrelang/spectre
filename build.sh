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

cc -c std/csources/panic_handler.c -o std/csources/panic_handler.o || exit 1
cc -c std/csources/yyjson_shim.c -o std/csources/yyjson_shim.o || exit 1

echo "Building spectre-dev..."
spectre ./src/sxc.sx -o spectre-dev || exit 1

if [ $DO_BOOTSTRAP -eq 1 ]; then
    echo "Emitting bootstrap SSA..."
    spectre ./src/sxc.sx --emit-ssa --quiet > ./bootstrap/sxc.ssa || exit 1
    echo "Bootstrap SSA written to ./bootstrap/sxc.ssa"
fi

echo "Done."
