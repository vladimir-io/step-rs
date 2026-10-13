# Changelog

## 1.1.0 (2026-10-13)

Coaxial / stepped hole detection only.

### Web
- Landing: drop zone, one sample, FAQ
- Hole table: kind, label, Ø, depth, segments
- OpenCascade tessellation (`occt-import-js`) with Rust mesh fallback
- `llms.txt`, JSON-LD, Open Graph / Twitter cards
- Removed ITAR/manifest UI, landing regression panel, axes columns

### Engine
- Streaming STEP parser (AP203 / AP214)
- B-rep + coaxial bore detection
- WASM worker + CLI analyze

### Dropped from product UI
- CAM / toolpath / G-code / stock simulation
- Browser regression dashboard (still in `cargo test` / CLI)

## 1.0.0 (2026-06-27)

First public release.
