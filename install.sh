#!/usr/bin/env sh
set -eu

PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"
SRC_DIR="${PREFIX}/src"

QBE_REPO="https://c9x.me/git/qbe.git"
SPECTRE_REPO="https://github.com/spectrelang/spectre.git"
YYJSON_REPO="https://github.com/ibireme/yyjson.git"

log() {
    printf "\033[1;34m[INFO]\033[0m %s\n" "$1"
}

err() {
    printf "\033[1;31m[ERROR]\033[0m %s\n" "$1" >&2
    exit 1
}

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || err "Missing required command: $1"
}

log "Checking required tools..."
require_cmd git
require_cmd make
require_cmd cc
require_cmd cmake

log "Preparing directories..."
mkdir -p "$BIN_DIR" "$SRC_DIR"

if command -v qbe >/dev/null 2>&1; then
    log "QBE already installed, skipping."
else
    log "Installing QBE..."

    QBE_DIR="${SRC_DIR}/qbe"

    if [ -d "$QBE_DIR" ]; then
        log "QBE source already exists, updating..."
        git -C "$QBE_DIR" pull --ff-only
    else
        git clone "$QBE_REPO" "$QBE_DIR"
    fi

    log "Building QBE..."
    (cd "$QBE_DIR" && make)

    log "Installing QBE to ${BIN_DIR}..."
    /usr/bin/install -m 0755 "$QBE_DIR/qbe" "$BIN_DIR/qbe"
fi

log "Installing Spectre (self-hosted)..."

SPECTRE_DIR="${SRC_DIR}/spectre"

if [ -d "$SPECTRE_DIR" ]; then
    log "Spectre source already exists, updating..."
    git -C "$SPECTRE_DIR" pull --ff-only
else
    git clone "$SPECTRE_REPO" "$SPECTRE_DIR"
fi

BOOTSTRAP_SSA="${SPECTRE_DIR}/bootstrap/sxc.ssa"
BOOTSTRAP_OUT="${SPECTRE_DIR}/bootstrap/sxc_bootstrap"
OTHER_PREFIX="/usr/local"

[ -f "$BOOTSTRAP_SSA" ] || err "Missing bootstrap SSA at ${BOOTSTRAP_SSA}"

log "Bootstrapping Spectre with QBE..."

CSOURCES_DIR="${SPECTRE_DIR}/std/csources"

PANIC_HANDLER_SRC="${CSOURCES_DIR}/panic_handler.c"
PANIC_HANDLER_OBJ="${CSOURCES_DIR}/panic_handler.o"

YYJSON_SHIM_SRC="${CSOURCES_DIR}/yyjson_shim.c"
YYJSON_SHIM_OBJ="${CSOURCES_DIR}/yyjson_shim.o"

log "QBE Stage..."
qbe -o "${BOOTSTRAP_OUT}.s" "$BOOTSTRAP_SSA"

log "CC Stage I (C sources)..."
cc -O2 -c "${PANIC_HANDLER_SRC}" -o "${PANIC_HANDLER_OBJ}"
cc -O2 -c "${YYJSON_SHIM_SRC}" -o "${YYJSON_SHIM_OBJ}"

log "CC Stage II..."
cc -O2 \
    "${BOOTSTRAP_OUT}.s" \
    "${PANIC_HANDLER_OBJ}" \
    "${YYJSON_SHIM_OBJ}" \
    -L"${OTHER_PREFIX}/lib" \
    -lyyjson \
    -o "${BOOTSTRAP_OUT}"

log "Installing Spectre binary..."
/usr/bin/install -m 0755 \
    "${BOOTSTRAP_OUT}" \
    "${BIN_DIR}/spectre"

STDLIB_SRC="${SPECTRE_DIR}/std"
STDLIB_DEST="${BIN_DIR}/std"

[ -d "$STDLIB_SRC" ] || err "Spectre std library not found at ${STDLIB_SRC}"

log "Installing standard library..."
rm -rf "$STDLIB_DEST"
mkdir -p "$STDLIB_DEST"

(
    cd "$STDLIB_SRC"
    tar cf - .
) | (
    cd "$STDLIB_DEST"
    tar xf -
)

log "Installing yyjson..."

OS="$(uname -s)"
OTHER_PREFIX="/usr/local"

case "$OS" in
    Darwin)
        if [ -d "/opt/homebrew" ]; then
            OTHER_PREFIX="/opt/homebrew"
        fi
        ;;
esac

YYJSON_DIR="${SRC_DIR}/yyjson"

if [ -d "$YYJSON_DIR" ]; then
    log "yyjson source already exists, updating..."
    git -C "$YYJSON_DIR" pull --ff-only
else
    git clone "$YYJSON_REPO" "$YYJSON_DIR"
fi

log "Building yyjson..."
rm -rf "${YYJSON_DIR}/build"
cmake -S "$YYJSON_DIR" -B "${YYJSON_DIR}/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=OFF \
    -DYYJSON_BUILD_TESTS=OFF \
    -DYYJSON_BUILD_FUZZER=OFF \
    -DYYJSON_BUILD_MISC=OFF \
    -DYYJSON_BUILD_DOC=OFF \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
cmake --build "${YYJSON_DIR}/build" --config Release

log "Verifying yyjson symbols..."
nm "${YYJSON_DIR}/build/libyyjson.a" | grep -q ' T yyjson_read' \
    || err "yyjson built but symbols missing — build may have inlined everything"

log "Installing yyjson to ${OTHER_PREFIX}..."
sudo mkdir -p "${OTHER_PREFIX}/lib" "${OTHER_PREFIX}/include"
sudo cp "${YYJSON_DIR}/build/libyyjson.a" "${OTHER_PREFIX}/lib/"
sudo cp "${YYJSON_DIR}/src/yyjson.h" "${OTHER_PREFIX}/include/"

if [ "$OS" = "Linux" ]; then
    log "Refreshing linker cache (ldconfig)..."
    sudo ldconfig
fi

log "yyjson installed successfully."

case ":$PATH:" in
    *":${BIN_DIR}:"*)
        log "PATH already contains ${BIN_DIR}"
        ;;
    *)
        log "IMPORTANT: ${BIN_DIR} is not in your PATH."
        echo
        echo "Add the following line to your shell config:"
        echo
        echo "  export PATH=\"${BIN_DIR}:\$PATH\""
        echo
        ;;
esac

log "Installation complete."
echo
echo "Installed:"
echo "  - spectre -> ${BIN_DIR}/spectre"
echo "  - stdlib  -> ${BIN_DIR}/std"
echo "  - qbe     -> ${BIN_DIR}/qbe"
echo "  - yyjson  -> ${OTHER_PREFIX}/lib/libyyjson.a"
echo
echo "Verify with:"
echo "  spectre -v"
