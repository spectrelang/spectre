#!/bin/sh
set -eu

: "${SPECTRE_BIN:=spectre}"
exec "$SPECTRE_BIN" app.sx "$@"
