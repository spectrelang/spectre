#!/usr/bin/env sh
set -eu

PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"
SRC_DIR="${PREFIX}/src"

QBE_OK=1
UQBE_BIN=""
FORCE_FALLBACK=0
UQBE_REPO="https://github.com/spectrelang/uqbe.git"
SPECTRE_REPO="https://github.com/spectrelang/spectre.git"
SPECTRE_LS_REPO="https://github.com/spectrelang/spectre-ls.git"
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

uqbe_local_hash() {
	UQBE_PATH="$1"
	[ -n "$UQBE_PATH" ] || return 1
	UQBE_VERSION_OUT="$("$UQBE_PATH" -v 2>/dev/null || true)"
	case "$UQBE_VERSION_OUT" in
	"uqbe (spectre) "*) printf '%s\n' "${UQBE_VERSION_OUT#uqbe (spectre) }" ;;
	*) return 1 ;;
	esac
}

uqbe_remote_hash() {
	git ls-remote "$UQBE_REPO" HEAD 2>/dev/null | awk 'NR == 1 { print $1 }'
}

uqbe_backup() {
	UQBE_SRC="$1"
	UQBE_TMP="${TMPDIR:-/tmp}"
	UQBE_DATE="$(date +%Y%m%d)"
	UQBE_BACKUP="${UQBE_TMP}/uqbe-backup-${UQBE_DATE}"
	UQBE_SUFFIX=1
	while [ -e "$UQBE_BACKUP" ]; do
		UQBE_BACKUP="${UQBE_TMP}/uqbe-backup-${UQBE_DATE}-${UQBE_SUFFIX}"
		UQBE_SUFFIX=$((UQBE_SUFFIX + 1))
	done
	cp -p "$UQBE_SRC" "$UQBE_BACKUP"
	log "Backed up old UQBE binary to ${UQBE_BACKUP}"
}

uqbe_build() {
	UQBE_SOURCE_DIR="$1"
	if [ -d "$UQBE_SOURCE_DIR/.git" ]; then
		log "UQBE source already exists, updating..."
		git -C "$UQBE_SOURCE_DIR" remote set-url origin "$UQBE_REPO" 2>/dev/null || true
		git -C "$UQBE_SOURCE_DIR" pull --ff-only || return 1
	else
		git clone "$UQBE_REPO" "$UQBE_SOURCE_DIR" || return 1
	fi
	log "Building UQBE..."
	(cd "$UQBE_SOURCE_DIR" && make) || return 1
	[ -x "${UQBE_SOURCE_DIR}/qbe" ] || return 1
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
CI=0

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
		log "Flag: -fallback set — skipping UQBE, building from C source."
		shift
		;;
	-minimal)
		MINIMAL=1
		log "Flag: -minimal set — skipping optional networking deps and spectre-ls."
		shift
		;;
	-ci)
		CI=1
		log "Flag: -ci set — skipping spectre-ls."
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

log "Checking for lld (used by -target: cross builds)..."
if ! command -v lld-link >/dev/null 2>&1 && ! command -v ld.lld >/dev/null 2>&1; then
	log "lld not found — attempting install (optional)..."
	if [ "$PKG_MGR" != "unknown" ]; then
		install_pkg "$PKG_MGR" lld 2>/dev/null || true
	fi
	if command -v lld-link >/dev/null 2>&1 || command -v ld.lld >/dev/null 2>&1; then
		log "lld installed successfully."
	else
		log "lld unavailable; cross builds (-target:...) will need lld installed manually."
	fi
else
	log "lld already installed."
fi

log "Checking required tools..."
require_cmd git
require_cmd make
require_cmd cc

if [ "$MINIMAL" -eq 0 ]; then
	install_network_deps
else
	log "Skipping optional networking development dependencies (-minimal)."
fi

log "Preparing directories..."
mkdir -p "$BIN_DIR" "$SRC_DIR"

UQBE_DIR="${SRC_DIR}/uqbe"

if [ "$FORCE_FALLBACK" -eq 1 ]; then
	log "Skipping UQBE (forced fallback mode)."
	QBE_OK=0
else
	UQBE_BIN="$(command -v uqbe 2>/dev/null || true)"
	if [ -z "$UQBE_BIN" ] && [ -x "${BIN_DIR}/uqbe" ]; then
		UQBE_BIN="${BIN_DIR}/uqbe"
	fi
	UQBE_NEEDS_INSTALL=0

	if [ -n "$UQBE_BIN" ]; then
		UQBE_INSTALL_PATH="$UQBE_BIN"
		UQBE_LOCAL_HASH="$(uqbe_local_hash "$UQBE_BIN" || true)"
		UQBE_REMOTE_HASH="$(uqbe_remote_hash || true)"
		if [ -z "$UQBE_REMOTE_HASH" ]; then
			log "Could not check for UQBE updates; keeping ${UQBE_BIN}"
		elif [ -z "$UQBE_LOCAL_HASH" ]; then
			log "UQBE version is unknown; updating ${UQBE_BIN}"
			UQBE_NEEDS_INSTALL=1
		else
			case "$UQBE_REMOTE_HASH" in
			"$UQBE_LOCAL_HASH"*)
				log "UQBE is up to date: ${UQBE_BIN}"
				;;
			*)
				log "UQBE update available: ${UQBE_LOCAL_HASH} -> ${UQBE_REMOTE_HASH}"
				UQBE_NEEDS_INSTALL=1
				;;
			esac
		fi
	else
		UQBE_INSTALL_PATH="${BIN_DIR}/uqbe"
		UQBE_NEEDS_INSTALL=1
	fi

	if [ "$UQBE_NEEDS_INSTALL" -eq 1 ]; then
		if uqbe_build "$UQBE_DIR"; then
			if [ -n "$UQBE_BIN" ] && [ -x "$UQBE_BIN" ]; then
				uqbe_backup "$UQBE_BIN"
			fi
			log "Installing UQBE to ${UQBE_INSTALL_PATH}..."
			if ! /usr/bin/install -m 0755 "${UQBE_DIR}/qbe" "$UQBE_INSTALL_PATH" 2>/dev/null; then
				if [ "$UQBE_INSTALL_PATH" != "${BIN_DIR}/uqbe" ]; then
					log "Could not replace ${UQBE_INSTALL_PATH}, thus installing to ${BIN_DIR}/uqbe instead."
					UQBE_INSTALL_PATH="${BIN_DIR}/uqbe"
				fi
				/usr/bin/install -m 0755 "${UQBE_DIR}/qbe" "$UQBE_INSTALL_PATH" || QBE_OK=0
			fi
			if [ "$QBE_OK" -eq 1 ]; then
				UQBE_BIN="$UQBE_INSTALL_PATH"
			fi
		elif [ -n "$UQBE_BIN" ] && [ -x "$UQBE_BIN" ]; then
			log "UQBE update failed - keeping existing ${UQBE_BIN}."
		else
			log "UQBE setup failed - will use C bootstrap fallback."
			QBE_OK=0
		fi
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
QBE_RUNTIME_SRC="${CSOURCES_DIR}/qbe_runtime.c"
QBE_RUNTIME_OBJ="${CSOURCES_DIR}/qbe_runtime.o"
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

log "CC Stage I (C sources)..."
cc -O2 -c "${PANIC_HANDLER_SRC}" -o "${PANIC_HANDLER_OBJ}"
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

STAGE0="${SPECTRE_DIR}/spectre-stage0"
STAGE1="${SPECTRE_DIR}/spectre-stage1"
STAGE2="${SPECTRE_DIR}/spectre-stage2"

if [ "$QBE_OK" -eq 1 ]; then
	[ -f "$BOOTSTRAP_SSA" ] || err "Missing bootstrap SSA at ${BOOTSTRAP_SSA}"
	[ -n "$UQBE_BIN" ] || err "UQBE was not found in PATH or installed."

	log "Bootstrapping Spectre with UQBE..."

	log "UQBE Stage..."
	"${UQBE_BIN}" -o "${BOOTSTRAP_OUT}.s" "$BOOTSTRAP_SSA"

	log "CC Stage II..."
	cc -O2 \
		"${BOOTSTRAP_OUT}.s" \
		"${QBE_RUNTIME_OBJ}" \
		"${PANIC_HANDLER_OBJ}" \
		-o "${BOOTSTRAP_OUT}"
else
	[ -f "$POSIX_BOOTSTRAP_SRC" ] || err "Missing C bootstrap at ${POSIX_BOOTSTRAP_SRC}"

	log "Bootstrapping Spectre from C source ($(basename "$POSIX_BOOTSTRAP_SRC"))..."

	log "CC Stage II (C bootstrap)..."
	(cd "$SPECTRE_DIR" && cc -O2 \
		"${POSIX_BOOTSTRAP_SRC}" \
		"${PANIC_HANDLER_OBJ}" \
		-Wno-everything \
		-o "${POSIX_BOOTSTRAP_OUT}")
fi

log "Building Spectre stage 1 from source..."
if [ "$QBE_OK" -eq 1 ]; then
	/usr/bin/install -m 0755 "${BOOTSTRAP_OUT}" "$STAGE0"
else
	/usr/bin/install -m 0755 "${POSIX_BOOTSTRAP_OUT}" "$STAGE0"
fi
(cd "$SPECTRE_DIR" && ./spectre-stage0 ./src/sxc.sx -release -alt -runtime:hosted -o ./spectre-stage1)

GIT_HASH="unknown"
if command -v git >/dev/null 2>&1; then
	GIT_HASH="$(git -C "$SPECTRE_DIR" rev-parse --short HEAD 2>/dev/null || echo unknown)"
fi
SPECTRE_MOD_VERSION="$(
	sed -n 's/^[[:space:]]*version[[:space:]]*"\([^"]*\)".*/\1/p' \
		"${SPECTRE_DIR}/sx.mod" | head -n 1
)"
STAGE_DEFINES="-d:GIT_HASH=${GIT_HASH}"
if [ -n "$SPECTRE_MOD_VERSION" ]; then
	STAGE_DEFINES="${STAGE_DEFINES} -d:SPECTRE_MOD_VERSION=${SPECTRE_MOD_VERSION}"
fi

log "Building Spectre stage 2 from source..."
(cd "$SPECTRE_DIR" && ./spectre-stage1 $STAGE_DEFINES ./src/sxc.sx -release -alt -runtime:hosted -o ./spectre-stage2)

log "Installing Spectre binary..."
/usr/bin/install -m 0755 "${STAGE2}" "${BIN_DIR}/spectre"
rm -f "$STAGE0" "$STAGE1" "$STAGE2"

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

if [ "$CI" -eq 1 ]; then
	log "Skipping spectre-ls (-ci)."
elif [ "$MINIMAL" -eq 1 ]; then
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
		log "UQBE unavailable — building spectre-ls with the fallback backend."
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
	(cd "$SPECTRE_LS_DIR" && "${BIN_DIR}/spectre" restore && "${BIN_DIR}/spectre" build release) || SPECTRE_LS_BUILD_OK=0

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
if [ "$CI" -eq 1 ]; then
	echo "  - spectre-ls -> (skipped, -ci flag set)"
elif [ "$MINIMAL" -eq 1 ]; then
	echo "  - spectre-ls -> (skipped, -minimal flag set)"
else
	echo "  - spectre-ls -> ${BIN_DIR}/spectre-ls"
fi
echo "  - stdlib     -> ${BIN_DIR}/spectrelib"
if [ "$QBE_OK" -eq 1 ]; then
	echo "  - uqbe       -> ${UQBE_BIN}"
elif [ "$FORCE_FALLBACK" -eq 1 ]; then
	echo "  - uqbe       -> (skipped, -fallback flag set)"
else
	echo "  - uqbe       -> (skipped, used fallback)"
fi
if command -v tcc >/dev/null 2>&1; then
	echo "  - tcc        -> $(command -v tcc)"
else
	echo "  - tcc        -> (unavailable, used fallback)"
fi
echo "  - clang      -> $(command -v clang)"
echo
echo "Verify with:"
echo "  spectre -v"
