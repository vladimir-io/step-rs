#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/node_modules/occt-import-js/dist"
DST="$ROOT/vendor"
mkdir -p "$DST"
cp "$SRC/occt-import-js.js" "$SRC/occt-import-js.wasm" "$DST/"
echo "occt-import-js → $DST/"
