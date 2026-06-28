#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
wasm-pack build ../crates/steprs-wasm --target web --out-dir ./pkg --release
echo "WASM built → web/pkg/"
