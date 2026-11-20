#!/usr/bin/env bash
# Build the Blood shell from the generated C sources in navid-m/blood.
#
# This is the only shell bootstrap script in fw/. Everything else runs under
# Blood once this binary exists.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

BLOOD_REPO="${BLOOD_REPO:-https://github.com/navid-m/blood.git}"
BLOOD_REF="${BLOOD_REF:-master}"
BLOOD_SRC="${BLOOD_SRC:-}"
BLOOD_SRC_DIR="${BLOOD_SRC_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/blood-src}"
BLOOD_OUT_DIR="${BLOOD_OUT_DIR:-${ROOT}/.blood}"

case "$(uname -s)" in
Darwin)
	GENERATED="generated/blood_darwin.c"
	;;
Linux)
	GENERATED="generated/blood_linux.c"
	;;
*)
	echo "error: unsupported host platform: $(uname -s)" >&2
	exit 1
	;;
esac

if ! command -v cc >/dev/null 2>&1; then
	echo "error: cc is required to build Blood" >&2
	exit 1
fi

if [ -n "$BLOOD_SRC" ]; then
	if [ ! -f "${BLOOD_SRC}/${GENERATED}" ]; then
		echo "error: ${GENERATED} not found under BLOOD_SRC=${BLOOD_SRC}" >&2
		exit 1
	fi
elif [ -d "${BLOOD_SRC_DIR}/.git" ]; then
	echo "Updating Blood sources in ${BLOOD_SRC_DIR}..."
	git -C "$BLOOD_SRC_DIR" fetch --depth 1 origin "$BLOOD_REF"
	git -C "$BLOOD_SRC_DIR" checkout --quiet FETCH_HEAD
else
	if ! command -v git >/dev/null 2>&1; then
		echo "error: git is required to fetch Blood" >&2
		exit 1
	fi
	echo "Cloning ${BLOOD_REPO} (${BLOOD_REF}) into ${BLOOD_SRC_DIR}..."
	rm -rf "$BLOOD_SRC_DIR"
	git clone --depth 1 --branch "$BLOOD_REF" "$BLOOD_REPO" "$BLOOD_SRC_DIR"
fi

if [ -z "$BLOOD_SRC" ]; then
	SOURCE="${BLOOD_SRC_DIR}/${GENERATED}"
else
	SOURCE="${BLOOD_SRC}/${GENERATED}"
fi

mkdir -p "$BLOOD_OUT_DIR"
echo "Compiling ${SOURCE}..."
cc -O2 -w -o "${BLOOD_OUT_DIR}/blood" "$SOURCE" -lpthread -lm

"${BLOOD_OUT_DIR}/blood" -v
echo "Blood installed at ${BLOOD_OUT_DIR}/blood"
