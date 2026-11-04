GIT_HASH="unknown"
if command -v git >/dev/null 2>&1; then
    GIT_HASH="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
fi

./spectre-dev -dGIT_HASH="$GIT_HASH" ./src/sxc.sx --emit-ssa --emit-shim=spectrelib/csources/qbe_runtime.c --src-target=linux > bootstrap/sxc.ssa
./spectre-dev -dGIT_HASH="$GIT_HASH" ./src/sxc.sx -b --emit-alt --src-target=linux > bootstrap/sxc.c
./spectre-dev -dGIT_HASH="$GIT_HASH" ./src/sxc.sx --emit-ssa --src-target=darwin > bootstrap/sxc_darwin.ssa
./spectre-dev -dGIT_HASH="$GIT_HASH" ./src/sxc.sx -b --emit-alt --src-target=darwin > bootstrap/sxc_darwin.c
./spectre-dev -dGIT_HASH="$GIT_HASH" ./src/sxc.sx -b --emit-alt --src-target=windows > bootstrap/sxcw.c
