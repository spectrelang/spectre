#!/usr/bin/env sh
set -eu

PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"
SRC_DIR="${PREFIX}/src"
QBE_REPO="https://c9x.me/git/qbe.git"
SPECTRE_REPO="https://github.com/spectrelang/spectre.git"

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

if ! command -v cargo >/dev/null 2>&1; then
    err "Rust (cargo) is required. Install it from https://rustup.rs/"
fi

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
    (
        cd "$QBE_DIR"
        make
    )

    log "Installing QBE to ${BIN_DIR}..."
    install -m 0755 "$QBE_DIR/qbe" "$BIN_DIR/qbe"
fi

log "Installing Spectre..."

SPECTRE_DIR="${SRC_DIR}/spectre"

if [ -d "$SPECTRE_DIR" ]; then
    log "Spectre source already exists, updating..."
    git -C "$SPECTRE_DIR" pull --ff-only
else
    git clone "$SPECTRE_REPO" "$SPECTRE_DIR"
fi

log "Building Spectre (release)..."
(
    cd "$SPECTRE_DIR"
    cargo build --release
)

log "Installing Spectre binary..."
install -m 0755 \
    "$SPECTRE_DIR/target/release/spectre" \
    "$BIN_DIR/spectre"

STDLIB_SRC="${SPECTRE_DIR}/std"
STDLIB_DEST="${BIN_DIR}/std"

if [ ! -d "$STDLIB_SRC" ]; then
    err "Spectre std library not found at ${STDLIB_SRC}"
fi

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
echo
echo "Verify with:"
echo "  spectre --help"
