#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/web/pkg"
rm -rf "$OUT"
wasm-pack build "$ROOT/crates/steprs-wasm" --target web --out-dir "$OUT" --release
test -f "$OUT/steprs_wasm.js"
echo "WASM built → web/pkg/"
