# steprs

**Detect coaxial and stepped holes in STEP, entirely in your browser.**

Parse ISO 10303-21 (Part 21) locally, build a B-rep face graph, cluster coaxial bores. No uploads. Not CAM.

**[steprs.dev](https://steprs.dev)** · **[github.com/vladimir-io/step-rs](https://github.com/vladimir-io/step-rs)** · [llms.txt](https://steprs.dev/llms.txt)

[![CI](https://github.com/vladimir-io/step-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/vladimir-io/step-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## What it does

| Capability | Output |
|------------|--------|
| **Coaxial step bores** | Aligned cylinders → Ø large → Ø small, segment count, depth |
| **Through & blind holes** | Axis, radius, depth |
| **3D preview** | OpenCascade tessellation + coaxial overlay |
| **CSV export** | Kind, label, diameter, depth, segments |

**Not CAM.** No toolpaths, no G-code, no CAD editing.

---

## Quick start

```bash
make wasm && make serve   # http://localhost:8080
```

Or:

```bash
cargo run -p steprs-cli -- analyze samples/ap214_compact.step
cargo test --workspace
make test-web             # WASM + Playwright
```

---

## Pipeline

```text
STEP → parse (nom/streaming) → schema cache → B-rep → detect_coaxial_holes → JSON
```

| Crate | Role |
|-------|------|
| `steprs-core` | Streaming Part 21 parse, arena store |
| `steprs-schema` | Typed entity cache |
| `steprs-topology` | B-rep faces, cylindrical axes |
| `steprs-features` | Coaxial detection |
| `steprs-wasm` | Browser bindings |
| `steprs-cli` | Local inspect / analyze |

Browser layout: main thread (UI + Three.js) ↔ Web Worker (`analyzeStep`) ↔ `steprs-wasm`. STEP bytes stay on-device.

### WASM API

```javascript
import init, { analyzeStep } from "./pkg/steprs_wasm.js";
await init();
const json = analyzeStep(stepText);  // → WebAnalysisResult JSON string
```

---

## Privacy

Static site + WASM. Files are read via drag-and-drop / file picker and processed in a Web Worker. Nothing is uploaded.

An anonymized `structural_summary` field still exists in the analysis JSON for API/CLI use; it is not exposed in the default UI.

---

## Deploy

Push to `main` → GitHub Actions: `make wasm` + vendor OCCT → GitHub Pages → [steprs.dev](https://steprs.dev).

---

## License

[MIT](LICENSE)
