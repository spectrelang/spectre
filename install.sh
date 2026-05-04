#!/usr/bin/env sh
set -eu

PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"
SRC_DIR="${PREFIX}/src"

QBE_OK=1
QBE_MIRROR_REPO="https://github.com/8l/qbe.git"
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

detect_pkg_manager() {
    if   command -v zypper  >/dev/null 2>&1; then echo "zypper"
    elif command -v apt-get >/dev/null 2>&1; then echo "apt"
    elif command -v dnf     >/dev/null 2>&1; then echo "dnf"
    elif command -v pacman  >/dev/null 2>&1; then echo "pacman"
    elif command -v pkg     >/dev/null 2>&1; then echo "pkg"
    else echo "unknown"
    fi
}

install_pkg() {
    PKG_MGR="$1"
    shift
    case "$PKG_MGR" in
        zypper)  sudo zypper install --non-interactive "$@" ;;
        apt)     sudo apt-get install -y "$@" ;;
        dnf)     sudo dnf install -y "$@" ;;
        pacman)  sudo pacman -S --noconfirm "$@" ;;
        pkg)     sudo pkg install -y "$@" ;;
        *)       err "No supported package manager found. Install $* manually." ;;
    esac
}

log "Detecting package manager..."
PKG_MGR="$(detect_pkg_manager)"
log "Package manager: ${PKG_MGR}"

case "$PKG_MGR" in
    apt) sudo apt-get update -y ;;
    dnf) sudo dnf check-update -y || true ;;
esac

log "Checking for clang..."
if ! command -v clang >/dev/null 2>&1; then
    log "clang not found — installing..."
    install_pkg "$PKG_MGR" clang
    command -v clang >/dev/null 2>&1 || err "clang installation failed or not in PATH."
    log "clang installed successfully."
else
    log "clang already installed: $(command -v clang)"
fi

log "Checking for tcc..."
if ! command -v tcc >/dev/null 2>&1; then
    log "tcc not found — attempting install (optional)..."
    install_pkg "$PKG_MGR" tcc 2>/dev/null || true
    if command -v tcc >/dev/null 2>&1; then
        log "tcc installed successfully."
    else
        log "tcc unavailable on this platform, clang will be used as alt backend instead."
    fi
else
    log "tcc already installed: $(command -v tcc)"
fi

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
        git -C "$QBE_DIR" pull --ff-only || {
            log "Primary remote failed, trying mirror..."
            git -C "$QBE_DIR" remote set-url origin "$QBE_MIRROR_REPO"
            git -C "$QBE_DIR" pull --ff-only || QBE_OK=0
        }
    else
        git_clone_with_fallback "$QBE_REPO" "$QBE_MIRROR_REPO" "$QBE_DIR" || QBE_OK=0
    fi

    if [ "$QBE_OK" -eq 1 ]; then
        log "Building QBE..."
        (cd "$QBE_DIR" && make) || QBE_OK=0
    fi

    if [ "$QBE_OK" -eq 1 ] && [ -x "${QBE_DIR}/qbe" ]; then
        log "Installing QBE to ${BIN_DIR}..."
        /usr/bin/install -m 0755 "${QBE_DIR}/qbe" "$BIN_DIR/qbe"
    else
        log "QBE setup failed — will use C bootstrap fallback."
        QBE_OK=0
    fi
fi

log "Installing Spectre (self-hosted)..."

SPECTRE_DIR="${SRC_DIR}/spectre"

if [ -d "$SPECTRE_DIR" ]; then
    log "Spectre source already exists, updating..."
    git -C "$SPECTRE_DIR" pull --ff-only
else
    git clone "$SPECTRE_REPO" "$SPECTRE_DIR"
fi

CSOURCES_DIR="${SPECTRE_DIR}/std/csources"
PANIC_HANDLER_SRC="${CSOURCES_DIR}/panic_handler.c"
PANIC_HANDLER_OBJ="${CSOURCES_DIR}/panic_handler.o"
YYJSON_SHIM_SRC="${CSOURCES_DIR}/yyjson_shim.c"
YYJSON_SHIM_OBJ="${CSOURCES_DIR}/yyjson_shim.o"
OTHER_PREFIX="/usr/local"
YYJSON_DIR="${SRC_DIR}/yyjson"
OS="$(uname -s)"

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

if [ "$QBE_OK" -eq 1 ]; then
    BOOTSTRAP_SSA="${SPECTRE_DIR}/bootstrap/sxc.ssa"
    BOOTSTRAP_OUT="${SPECTRE_DIR}/bootstrap/sxc_bootstrap"

    [ -f "$BOOTSTRAP_SSA" ] || err "Missing bootstrap SSA at ${BOOTSTRAP_SSA}"

    log "Bootstrapping Spectre with QBE..."

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
else
    POSIX_BOOTSTRAP_SRC="${SPECTRE_DIR}/bootstrap/sxc_posix.c"
    POSIX_BOOTSTRAP_OUT="${SPECTRE_DIR}/bootstrap/sxc_posix_bootstrap"
    OTHER_PREFIX="/usr/local"

    [ -f "$POSIX_BOOTSTRAP_SRC" ] || err "Missing C bootstrap at ${POSIX_BOOTSTRAP_SRC}"

    log "Bootstrapping Spectre from C source (sxc_posix.c)..."

    log "CC Stage I (C sources)..."
    cc -O2 -c "${PANIC_HANDLER_SRC}" -o "${PANIC_HANDLER_OBJ}"
    cc -O2 -I"${OTHER_PREFIX}/include" -c "${YYJSON_SHIM_SRC}" -o "${YYJSON_SHIM_OBJ}"

    log "CC Stage II (C bootstrap)..."
    (cd "$SPECTRE_DIR" && cc -O2 \
        bootstrap/sxc_posix.c \
        "${PANIC_HANDLER_OBJ}" \
        "${YYJSON_SHIM_OBJ}" \
        -I"${OTHER_PREFIX}/include" \
        -L"${OTHER_PREFIX}/lib" \
        -lyyjson \
        -o "${POSIX_BOOTSTRAP_OUT}")

    log "Installing Spectre binary (C bootstrap)..."
    /usr/bin/install -m 0755 \
        "${POSIX_BOOTSTRAP_OUT}" \
        "${BIN_DIR}/spectre"
fi

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

OTHER_PREFIX="/usr/local"

case "$OS" in
    Darwin)
        if [ -d "/opt/homebrew" ]; then
            OTHER_PREFIX="/opt/homebrew"
        fi
        ;;
esac

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
if [ "$QBE_OK" -eq 1 ]; then
    echo "  - qbe     -> ${BIN_DIR}/qbe"
else
    echo "  - qbe     -> (skipped, used fallback)"
fi
if command -v tcc >/dev/null 2>&1; then
    echo "  - tcc     -> $(command -v tcc)"
else
    echo "  - tcc     -> (unavailable, used fallback)"
fi
echo "  - clang   -> $(command -v clang)"
echo "  - yyjson  -> ${OTHER_PREFIX}/lib/libyyjson.a"
echo
echo "Verify with:"
echo "  spectre -v"
