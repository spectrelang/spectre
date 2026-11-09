#!/usr/bin/env sh
set -eu

PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"
SRC_DIR="${PREFIX}/src"

QBE_OK=1
QBE_BIN=""
FORCE_FALLBACK=0
QBE_MIRROR_REPO="https://github.com/spectrelang/qbe-mirror.git"
QBE_REPO="https://c9x.me/git/qbe.git"
SPECTRE_REPO="https://github.com/spectrelang/spectre.git"
SPECTRE_LS_REPO="https://github.com/spectrelang/spectre-ls.git"
YYJSON_REPO="https://github.com/ibireme/yyjson.git"
TUI_STTY=""

tui_cleanup() {
	if [ -n "$TUI_STTY" ]; then
		stty "$TUI_STTY" </dev/tty 2>/dev/null || true
		TUI_STTY=""
	fi
	printf '\033[?25h' >/dev/tty 2>/dev/null || true
}

tui_readbyte() {
	dd bs=1 count=1 2>/dev/null </dev/tty | od -An -tx1 | tr -d ' \n'
}

tui_draw() {
	_sel="$1"
	_top="$2"
	_vis="$3"
	_total="$4"
	_i="$_top"
	_end=$((_top + _vis - 1))
	{
		while [ "$_i" -le "$_end" ]; do
			_label="$(printf '%s\n' "$TUI_ITEMS" | sed -n "${_i}p")"
			if [ "$_i" -eq "$_sel" ]; then
				printf '\r\033[2K  \033[1;36m> %s\033[0m\n' "$_label"
			else
				printf '\r\033[2K    %s\n' "$_label"
			fi
			_i=$((_i + 1))
		done
		printf '\r\033[2K  \033[2mUp/Down or j/k: move   Enter: select   q: quit   (%s/%s)\033[0m\n' \
			"$_sel" "$_total"
	} >/dev/tty
}

select_version_tui() {
	[ -z "$SPECTRE_VERSION" ] || return 0
	[ "$FORCE_LATEST" -eq 0 ] || return 0

	command -v git >/dev/null 2>&1 || return 0
	command -v stty >/dev/null 2>&1 || return 0
	(: </dev/tty >/dev/tty) 2>/dev/null || {
		log "No interactive terminal, installing latest Spectre."
		return 0
	}

	log "Fetching available Spectre versions..."
	_tags="$(git ls-remote --tags --refs "$SPECTRE_REPO" 'v*' 2>/dev/null |
		sed 's|.*refs/tags/||' |
		grep -E '^v[0-9]+(\.[0-9]+)*$' |
		sort -t. -k1.2,1nr -k2,2nr -k3,3nr -k4,4nr)" || _tags=""

	if [ -z "$_tags" ]; then
		log "Could not fetch version list, installing latest Spectre."
		return 0
	fi

	TUI_ITEMS="$(printf 'latest (default branch)\n%s' "$_tags")"
	_total="$(printf '%s\n' "$TUI_ITEMS" | wc -l | tr -d ' ')"
	_vis=10
	[ "$_total" -ge "$_vis" ] || _vis="$_total"

	TUI_STTY="$(stty -g </dev/tty)"
	trap 'tui_cleanup; exit 130' INT TERM
	trap tui_cleanup EXIT
	stty -icanon -echo min 1 time 0 </dev/tty
	printf '\033[?25l' >/dev/tty

	printf '\n  \033[1mSelect a Spectre version to install\033[0m\n\n' >/dev/tty

	_sel=1
	_top=1
	_first=1
	_cancel=0
	while :; do
		if [ "$_first" -eq 0 ]; then
			printf '\033[%dA' $((_vis + 1)) >/dev/tty
		fi
		_first=0
		tui_draw "$_sel" "$_top" "$_vis" "$_total"

		_key="$(tui_readbyte)"
		case "$_key" in
		1b)
			stty min 0 time 1 </dev/tty
			_b1="$(tui_readbyte)"
			_b2="$(tui_readbyte)"
			stty min 1 time 0 </dev/tty
			case "${_b1}${_b2}" in
			5b41 | 4f41) _key="up" ;;
			5b42 | 4f42) _key="down" ;;
			*) _key="" ;;
			esac
			;;
		6b | 77) _key="up" ;;
		6a | 73) _key="down" ;;
		0a | 0d) break ;;
		71 | 51)
			_cancel=1
			break
			;;
		03 | 04)
			_cancel=1
			break
			;;
		*) _key="" ;;
		esac

		case "$_key" in
		up) [ "$_sel" -le 1 ] || _sel=$((_sel - 1)) ;;
		down) [ "$_sel" -ge "$_total" ] || _sel=$((_sel + 1)) ;;
		esac

		if [ "$_sel" -lt "$_top" ]; then
			_top="$_sel"
		elif [ "$_sel" -gt $((_top + _vis - 1)) ]; then
			_top=$((_sel - _vis + 1))
		fi
	done

	tui_cleanup
	trap - EXIT INT TERM
	printf '\n' >/dev/tty

	if [ "$_cancel" -eq 1 ]; then
		log "Installation cancelled."
		exit 0
	fi

	if [ "$_sel" -eq 1 ]; then
		SPECTRE_VERSION=""
		log "Selected: latest (default branch)"
	else
		SPECTRE_VERSION="$(printf '%s\n' "$TUI_ITEMS" | sed -n "${_sel}p")"
		log "Selected: ${SPECTRE_VERSION}"
	fi
}

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
	if command -v brew >/dev/null 2>&1; then
		echo "brew"
	elif command -v zypper >/dev/null 2>&1; then
		echo "zypper"
	elif command -v apt-get >/dev/null 2>&1; then
		echo "apt"
	elif command -v dnf >/dev/null 2>&1; then
		echo "dnf"
	elif command -v pacman >/dev/null 2>&1; then
		echo "pacman"
	elif command -v pkg >/dev/null 2>&1; then
		echo "pkg"
	else
		echo "unknown"
	fi
}

install_pkg() {
	PKG_MGR="$1"
	shift
	case "$PKG_MGR" in
	brew) brew install "$@" ;;
	zypper) sudo zypper install --non-interactive "$@" ;;
	apt) sudo apt-get install -y "$@" ;;
	dnf) sudo dnf install -y "$@" ;;
	pacman) sudo pacman -S --noconfirm "$@" ;;
	pkg) sudo pkg install -y "$@" ;;
	*) err "No supported package manager found. Install $* manually." ;;
	esac
}

git_clone_with_fallback() {
	PRIMARY="$1"
	MIRROR="$2"
	DEST="$3"
	git clone "$PRIMARY" "$DEST" 2>/dev/null || {
		log "Primary remote failed, trying mirror..."
		git clone "$MIRROR" "$DEST"
	}
}

install_network_deps() {
	if command -v pkg-config >/dev/null 2>&1 &&
		pkg-config --exists libcurl libnghttp2 openssl zlib libbrotlidec libbrotlicommon; then
		log "Networking development dependencies already installed."
		return
	fi

	log "Checking networking development dependencies (libcurl, nghttp2, openssl, zlib, brotli)..."
	case "$PKG_MGR" in
	apt)
		NET_DEPS="libcurl4-openssl-dev libnghttp2-dev libssl-dev zlib1g-dev libbrotli-dev"
		;;
	dnf)
		NET_DEPS="libcurl-devel libnghttp2-devel openssl-devel zlib-devel brotli-devel"
		;;
	zypper)
		NET_DEPS="libcurl-devel libnghttp2-devel libopenssl-devel zlib-devel libbrotli-devel"
		;;
	pacman)
		NET_DEPS="curl nghttp2 openssl zlib brotli"
		;;
	brew)
		NET_DEPS="curl nghttp2 openssl zlib brotli"
		;;
	pkg)
		NET_DEPS="curl nghttp2 openssl zlib brotli"
		;;
	*)
		log "warning: cannot install networking dependencies automatically; install libcurl, nghttp2, openssl, zlib, and brotli development packages manually."
		return
		;;
	esac

	if install_pkg "$PKG_MGR" $NET_DEPS; then
		log "Networking development dependencies installed."
	else
		log "warning: failed to install networking development dependencies; std/net/http may be unavailable."
	fi
}

SPECTRE_VERSION=""
MINIMAL=0
FORCE_LATEST=0

while [ "$#" -gt 0 ]; do
	case "$1" in
	-f)
		FORCE_LATEST=1
		SPECTRE_VERSION=""
		log "Flag: -f set — installing latest Spectre without version selection."
		shift
		;;
	-fallback)
		FORCE_FALLBACK=1
		log "Flag: -fallback set — skipping QBE, building from C source."
		shift
		;;
	-minimal)
		MINIMAL=1
		log "Flag: -minimal set — skipping optional networking deps and spectre-ls."
		shift
		;;
	-v)
		[ "$#" -ge 2 ] || err "Missing value for -v (expected a tag, e.g. -v v0.0.9)"
		SPECTRE_VERSION="$2"
		log "Flag: -v set — installing Spectre ${SPECTRE_VERSION}."
		shift 2
		;;
	*)
		err "Unknown flag: $1"
		;;
	esac
done

select_version_tui

log "Detecting package manager..."
PKG_MGR="$(detect_pkg_manager)"
log "Package manager: ${PKG_MGR}"

if [ "$MINIMAL" -eq 0 ]; then
	case "$PKG_MGR" in
	apt) sudo apt-get update -y ;;
	dnf) sudo dnf check-update -y || true ;;
	brew) brew update || true ;;
	esac
else
	log "Skipping package manager update (-minimal)."
fi

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
	if [ "$PKG_MGR" != "brew" ]; then
		install_pkg "$PKG_MGR" tcc 2>/dev/null || true
	fi
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

if [ "$MINIMAL" -eq 0 ]; then
	install_network_deps
else
	log "Skipping optional networking development dependencies (-minimal)."
fi

log "Preparing directories..."
mkdir -p "$BIN_DIR" "$SRC_DIR"

if [ "$FORCE_FALLBACK" -eq 1 ]; then
	log "Skipping QBE (forced fallback mode)."
	QBE_OK=0
elif command -v qbe >/dev/null 2>&1; then
	QBE_BIN="$(command -v qbe)"
	log "QBE already installed, skipping: ${QBE_BIN}"
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
		QBE_BIN="${BIN_DIR}/qbe"
	else
		log "QBE setup failed — will use C bootstrap fallback."
		QBE_OK=0
	fi
fi

log "Installing Spectre (self-hosted)..."

SPECTRE_DIR="${SRC_DIR}/spectre"

if [ -d "$SPECTRE_DIR" ]; then
	log "Spectre source already exists, updating..."
	if [ -n "$SPECTRE_VERSION" ]; then
		git -C "$SPECTRE_DIR" fetch --tags --force
	else
		DEFAULT_BRANCH="$(git -C "$SPECTRE_DIR" symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's@^origin/@@')"
		if [ -n "$DEFAULT_BRANCH" ]; then
			git -C "$SPECTRE_DIR" checkout --quiet "$DEFAULT_BRANCH" || true
		fi
		git -C "$SPECTRE_DIR" pull --ff-only
	fi
else
	git clone "$SPECTRE_REPO" "$SPECTRE_DIR"
fi

if [ -n "$SPECTRE_VERSION" ]; then
	log "Checking out Spectre tag ${SPECTRE_VERSION}..."
	git -C "$SPECTRE_DIR" checkout --quiet "$SPECTRE_VERSION" ||
		err "Could not checkout Spectre tag '${SPECTRE_VERSION}'. Check that the tag exists."
fi

CSOURCES_DIR="${SPECTRE_DIR}/spectrelib/csources"
PANIC_HANDLER_SRC="${CSOURCES_DIR}/panic_handler.c"
PANIC_HANDLER_OBJ="${CSOURCES_DIR}/panic_handler.o"
YYJSON_SHIM_SRC="${CSOURCES_DIR}/yyjson_shim.c"
YYJSON_SHIM_OBJ="${CSOURCES_DIR}/yyjson_shim.o"
QBE_RUNTIME_SRC="${CSOURCES_DIR}/qbe_runtime.c"
QBE_RUNTIME_OBJ="${CSOURCES_DIR}/qbe_runtime.o"
YYJSON_DIR="${SRC_DIR}/yyjson"
OS="$(uname -s)"
OTHER_PREFIX="/usr/local"

case "$OS" in
Darwin)
	if [ -d "/opt/homebrew" ]; then
		OTHER_PREFIX="/opt/homebrew"
	elif [ -d "/usr/local/Cellar" ]; then
		OTHER_PREFIX="/usr/local"
	fi
	;;
esac

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

log "Installing yyjson to ${OTHER_PREFIX}..."
sudo mkdir -p "${OTHER_PREFIX}/lib" "${OTHER_PREFIX}/include"
sudo cp "${YYJSON_DIR}/build/libyyjson.a" "${OTHER_PREFIX}/lib/"
sudo cp "${YYJSON_DIR}/src/yyjson.h" "${OTHER_PREFIX}/include/"

if [ "$OS" = "Linux" ]; then
	log "Refreshing linker cache (ldconfig)..."
	sudo ldconfig
fi

log "yyjson installed successfully."
log "CC Stage I (C sources)..."
cc -O2 -c "${PANIC_HANDLER_SRC}" -o "${PANIC_HANDLER_OBJ}"
cc -O2 -I"${OTHER_PREFIX}/include" -c "${YYJSON_SHIM_SRC}" -o "${YYJSON_SHIM_OBJ}"
if [ -f "${QBE_RUNTIME_SRC}" ]; then
	cc -O2 -c "${QBE_RUNTIME_SRC}" -o "${QBE_RUNTIME_OBJ}"
fi

case "$OS" in
Darwin)
	BOOTSTRAP_SSA="${SPECTRE_DIR}/bootstrap/sxc_darwin.ssa"
	BOOTSTRAP_OUT="${SPECTRE_DIR}/bootstrap/sxc_bootstrap"
	POSIX_BOOTSTRAP_SRC="${SPECTRE_DIR}/bootstrap/sxc_darwin.c"
	POSIX_BOOTSTRAP_OUT="${SPECTRE_DIR}/bootstrap/sxc_darwin_bootstrap"
	;;
*)
	BOOTSTRAP_SSA="${SPECTRE_DIR}/bootstrap/sxc.ssa"
	BOOTSTRAP_OUT="${SPECTRE_DIR}/bootstrap/sxc_bootstrap"
	POSIX_BOOTSTRAP_SRC="${SPECTRE_DIR}/bootstrap/sxc.c"
	POSIX_BOOTSTRAP_OUT="${SPECTRE_DIR}/bootstrap/sxc_posix_bootstrap"
	;;
esac

if [ "$QBE_OK" -eq 1 ]; then
	[ -f "$BOOTSTRAP_SSA" ] || err "Missing bootstrap SSA at ${BOOTSTRAP_SSA}"
	[ -n "$QBE_BIN" ] || err "QBE was not found in PATH or installed."

	log "Bootstrapping Spectre with QBE..."

	log "QBE Stage..."
	"${QBE_BIN}" -o "${BOOTSTRAP_OUT}.s" "$BOOTSTRAP_SSA"

	log "CC Stage II..."
	cc -O2 \
		"${BOOTSTRAP_OUT}.s" \
		"${QBE_RUNTIME_OBJ}" \
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
	[ -f "$POSIX_BOOTSTRAP_SRC" ] || err "Missing C bootstrap at ${POSIX_BOOTSTRAP_SRC}"

	log "Bootstrapping Spectre from C source ($(basename "$POSIX_BOOTSTRAP_SRC"))..."

	log "CC Stage II (C bootstrap)..."
	(cd "$SPECTRE_DIR" && cc -O2 \
		"${POSIX_BOOTSTRAP_SRC}" \
		"${PANIC_HANDLER_OBJ}" \
		"${YYJSON_SHIM_OBJ}" \
		-I"${OTHER_PREFIX}/include" \
		-L"${OTHER_PREFIX}/lib" \
		-Wno-everything \
		-lyyjson \
		-o "${POSIX_BOOTSTRAP_OUT}")

	log "Installing Spectre binary (C bootstrap)..."
	/usr/bin/install -m 0755 \
		"${POSIX_BOOTSTRAP_OUT}" \
		"${BIN_DIR}/spectre"
fi

STDLIB_SRC="${SPECTRE_DIR}/spectrelib"
STDLIB_DEST="${BIN_DIR}/spectrelib"

[ -d "$STDLIB_SRC" ] || err "Spectre std library not found at ${STDLIB_SRC}"

log "Installing standard library..."
rm -f "${CSOURCES_DIR}"/*.o
rm -rf "$STDLIB_DEST"
mkdir -p "$STDLIB_DEST"

(
	cd "$STDLIB_SRC"
	tar cf - .
) | (
	cd "$STDLIB_DEST"
	tar xf -
)

log "Installing C shim sources..."
mkdir -p "${STDLIB_DEST}/csources"
if [ -f "${YYJSON_DIR}/src/yyjson.h" ]; then
	cp "${YYJSON_DIR}/src/yyjson.h" "${STDLIB_DEST}/csources/"
fi

if [ "$MINIMAL" -eq 1 ]; then
	log "Skipping spectre-ls (-minimal)."
else
	log "Installing spectre-ls (language server)..."

	SPECTRE_LS_DIR="${SRC_DIR}/spectre-ls"

	if [ -d "$SPECTRE_LS_DIR" ]; then
		if [ -f "${SPECTRE_LS_DIR}/sx.mod.install.bak" ]; then
			mv "${SPECTRE_LS_DIR}/sx.mod.install.bak" "${SPECTRE_LS_DIR}/sx.mod"
		fi
		log "spectre-ls source already exists, updating..."
		git -C "$SPECTRE_LS_DIR" pull --ff-only
	else
		git clone "$SPECTRE_LS_REPO" "$SPECTRE_LS_DIR"
	fi

	SPECTRE_LS_SXMOD="${SPECTRE_LS_DIR}/sx.mod"
	[ -f "$SPECTRE_LS_SXMOD" ] || err "spectre-ls sx.mod not found at ${SPECTRE_LS_SXMOD}"

	SPECTRE_LS_REQUIRED="$(
		sed -n 's/^[[:space:]]*spectre_version[[:space:]]*"\([^"]*\)".*/\1/p' "$SPECTRE_LS_SXMOD" | head -n 1
	)"
	INSTALLED_SPECTRE="$("${BIN_DIR}/spectre" -v | sed -n '1s/^Spectre[[:space:]]\{1,\}\([^[:space:]]*\).*/\1/p')"

	SPECTRE_LS_SXMOD_PATCHED=0
	if [ -n "$INSTALLED_SPECTRE" ] && [ -n "$SPECTRE_LS_REQUIRED" ] &&
		[ "$SPECTRE_LS_REQUIRED" != "$INSTALLED_SPECTRE" ]; then
		log "spectre-ls expects Spectre ${SPECTRE_LS_REQUIRED}, installed is ${INSTALLED_SPECTRE} — building against installed version."
		cp "$SPECTRE_LS_SXMOD" "${SPECTRE_LS_SXMOD}.install.bak"
		sed "s|^\([[:space:]]*spectre_version[[:space:]]*\)\"[^\"]*\"|\1\"${INSTALLED_SPECTRE}\"|" \
			"$SPECTRE_LS_SXMOD" >"${SPECTRE_LS_SXMOD}.install.tmp"
		mv "${SPECTRE_LS_SXMOD}.install.tmp" "$SPECTRE_LS_SXMOD"
		SPECTRE_LS_SXMOD_PATCHED=1
	fi

	if [ "$QBE_OK" -eq 0 ]; then
		log "QBE unavailable — building spectre-ls with the fallback backend."
		if [ "$SPECTRE_LS_SXMOD_PATCHED" -eq 0 ]; then
			cp "$SPECTRE_LS_SXMOD" "${SPECTRE_LS_SXMOD}.install.bak"
			SPECTRE_LS_SXMOD_PATCHED=1
		fi
		sed 's|^\([[:space:]]*flags[[:space:]]*\)"\(.*\)"|\1"\2 -alt"|' \
			"$SPECTRE_LS_SXMOD" >"${SPECTRE_LS_SXMOD}.install.tmp"
		mv "${SPECTRE_LS_SXMOD}.install.tmp" "$SPECTRE_LS_SXMOD"
	fi

	log "Building spectre-ls..."
	SPECTRE_LS_BUILD_OK=1
	(cd "$SPECTRE_LS_DIR" && "${BIN_DIR}/spectre" build release) || SPECTRE_LS_BUILD_OK=0

	if [ "$SPECTRE_LS_SXMOD_PATCHED" -eq 1 ]; then
		mv "${SPECTRE_LS_SXMOD}.install.bak" "$SPECTRE_LS_SXMOD"
	fi

	[ "$SPECTRE_LS_BUILD_OK" -eq 1 ] || err "Failed to build spectre-ls."
	[ -x "${SPECTRE_LS_DIR}/spectre-ls" ] || err "spectre-ls binary not found after build."

	log "Installing spectre-ls to ${BIN_DIR}..."
	/usr/bin/install -m 0755 "${SPECTRE_LS_DIR}/spectre-ls" "${BIN_DIR}/spectre-ls"
fi

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
echo "  - spectre    -> ${BIN_DIR}/spectre"
echo "  - spectre-ls -> ${BIN_DIR}/spectre-ls"
echo "  - stdlib     -> ${BIN_DIR}/spectrelib"
if [ "$QBE_OK" -eq 1 ]; then
	echo "  - qbe        -> ${QBE_BIN}"
elif [ "$FORCE_FALLBACK" -eq 1 ]; then
	echo "  - qbe        -> (skipped, -fallback flag set)"
else
	echo "  - qbe        -> (skipped, used fallback)"
fi
if command -v tcc >/dev/null 2>&1; then
	echo "  - tcc        -> $(command -v tcc)"
else
	echo "  - tcc        -> (unavailable, used fallback)"
fi
echo "  - clang      -> $(command -v clang)"
echo "  - yyjson     -> ${OTHER_PREFIX}/lib/libyyjson.a"
echo
echo "Verify with:"
echo "  spectre -v"
