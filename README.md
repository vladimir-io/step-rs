# steprs

**Coaxial bore and hole detection from STEP — entirely in your browser.**

steprs parses ISO 10303-21 (Part 21) CAD exchange files locally, builds a B-rep face graph, and runs deterministic coaxial hole clustering. No uploads. No server round-trip.

**[steprs.dev](https://steprs.dev)** · **[github.com/vladimir-io/step-rs](https://github.com/vladimir-io/step-rs)**

---

## What it does

| Capability | Output |
|------------|--------|
| **Coaxial step bores** | Cluster aligned cylindrical faces → Ø large → Ø small, segment count, depth |
| **Through & blind holes** | Isolated cylindrical faces with axis, radius, depth |
| **ITAR structural manifest** | Context-free JSON — classifications, diameters, depths, volumetric limits only |
| **3D part preview** | Lightweight Three.js mesh + coaxial overlay |

Validated against synthetic fixtures and anonymized **AP214 export** samples.

**Not CAM.** Not toolpath optimization. Not a CAD replacement.

---

## Technical architecture

### 1. Pure Rust byte-streaming parse layer (`nom`)

STEP Part 21 is a text format: `HEADER` + `DATA` sections of `#id = TYPE(params);` records. steprs does **not** load the file into an intermediate AST.

```
ISO-10303-21 file
       │
       ▼
  nom combinators          ← entity.rs, parameter.rs, tokens.rs
  (incremental slice parse)
       │
       ▼
  streaming DATA scan      ← streaming.rs
  (byte cursor, fail-soft per entity)
       │
       ▼
  RecordStore arena
```

- **`nom` combinators** parse entity instances, parameters, strings, and numerics directly from `&str` slices — zero intermediate token buffer.
- **Fail-soft ingestion**: a malformed entity is logged to `ParseReport`, skipped at the next `;`, and parsing continues. Messy real-world exports do not abort the pipeline.
- **Progress hooks** (`ParsePhase`) fire during header scan, data scan, indexing, topology, and feature stages — surfaced to the UI progress bar and WASM callbacks.

### 2. Flat arena memory allocator — O(1) graph resolution

Entities are stored in a **dense `Vec<Option<Record>>` indexed by STEP `#id`** (1-based):

```rust
// crates/steprs-core/src/store.rs
pub struct RecordStore {
    pub records: Vec<Option<Record>>,   // arena[slot] → entity
    pub by_type: HashMap<String, Vec<u32>>,
}
```

- `store.get(id)` is **O(1)** — direct index, no hash lookup per dereference.
- `by_type` provides O(1) access to all `CYLINDRICAL_SURFACE`, `ADVANCED_FACE`, etc. for schema cache warm-up.
- The streaming parser **pre-allocates** `Vec::with_capacity(total_est)` from a fast `#` marker count pass, avoiding repeated reallocations on large AP214 files.

Downstream, `SchemaCache` and `BRepModel` hold face adjacency and cylindrical axis data as flat structures for feature detectors.

### 3. WebAssembly client-side thread layout

```
┌─────────────────────────────────────────────┐
│  Main thread (app.js)                       │
│  · UI, progress, coaxial table, 3D viewer   │
│  · ITAR toggle → structural_summary panel   │
└──────────────┬──────────────────────────────┘
               │ postMessage
               ▼
┌─────────────────────────────────────────────┐
│  Web Worker (worker.js)                     │
│  · WASM init + analyzeStepOptionsWithProgress│
│  · Progress events → main thread bar        │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│  steprs-wasm (release, wasm-opt -Os)        │
│  parse → topology → detect_coaxial_holes    │
│  → anonymize → JSON                         │
└─────────────────────────────────────────────┘
```

- **All STEP bytes stay on-device.** The worker never posts raw geometry back except the analysis JSON needed for the grid and viewer.
- **Release profile**: `lto = true`, `opt-level = 3` workspace-wide; `steprs-wasm` uses `opt-level = "s"` + `wasm-opt -Os` for minimal bundle size.
- **No debug hooks in production builds** — `console_error_panic_hook` is disabled via `--no-default-features`.

### Pipeline

```text
STEP → parse (nom/streaming) → schema cache → B-rep → detect_coaxial_holes → anonymizer → JSON
```

| Crate | Role |
|-------|------|
| `steprs-core` | Streaming Part 21 parse, arena store, progress |
| `steprs-schema` | Typed entity cache, mesh extraction |
| `steprs-topology` | B-rep faces, cylindrical axes, adjacency |
| `steprs-features` | Coaxial detection, ITAR anonymizer |
| `steprs-wasm` | Browser bindings |
| `steprs-cli` | Local inspect / analyze |

---

## ITAR compliance & security

### Local browser isolation

steprs runs as a **static site + WASM**. Your STEP file is read with `FileReader` / drag-and-drop and passed to a **dedicated Web Worker**. Nothing is sent to a backend.

### Zero-knowledge structural manifest

When **Local IP Masking (ITAR)** is enabled, the UI switches from the full coaxial parameter grid (which includes axis origins, directions, and face IDs) to a **structural manifest** produced by `anonymizer.rs`:

**Stripped before export:**
- Axis origins and directions
- Face IDs and normals
- Pocket corner coordinates
- Any holistic spatial relationships

**Retained:**
- Feature classification (`coaxial_step_bore`, `blind_hole`, …)
- Segment count
- Diameters and segment depths (mm)
- Volumetric limits (mm³)

The manifest is computed **in the same WASM worker** that parsed the file. Toggling ITAR does not re-upload or re-parse — it selects the pre-computed `structural_summary` field from the in-memory analysis result.

```json
{
  "schema_version": "1",
  "profile": "anonymized_structural",
  "unit": "mm",
  "quantities": { "coaxial_step_bore": 1, "total_features": 1 },
  "features": [{
    "classification": "coaxial_step_bore",
    "segment_quantity": 2,
    "diameters_mm": [25.4, 12.7],
    "segment_depths_mm": [10.0, 12.7],
    "volume_limit_mm3": 5124.783
  }]
}
```

This is suitable for **cloud ingestion of machinability metadata** without exposing part orientation or export-controlled spatial data. Verify against your compliance officer before relying on it for regulated workflows.

---

## Reproducible local testing

### Full workspace

```bash
cargo test --workspace
make test-web    # WASM build + Playwright E2E
```

### Differential regression — mathematical determinism baseline

Ground-truth dimensions for `cylinder_block.step` (multi-segment coaxial bore) are asserted with **ε = 1e-6** and a **50 ms WASM client budget**:

```bash
cargo test -p steprs differential_regression -- --nocapture
```

This runs four tests:

| Test | Asserts |
|------|---------|
| `differential_regression_cylinder_block_dims` | Radii, depths, axis, face IDs vs ground truth |
| `differential_regression_cylinder_block_structural_summary` | Anonymized manifest has no spatial leaks |
| `differential_regression_wasm_client_budget` | `analyze_web_json` < 50 ms on fixture |
| `differential_regression_fixture_ingest` | All regression fixtures parse without panic |

### CLI quick check

```bash
cargo run -p steprs-cli -- analyze samples/cylinder_block.step
cargo run -p steprs-cli -- analyze samples/ap214_medium.step --json
```

### Local web dev

```bash
make wasm && make serve   # http://localhost:8080
```

No STEP file? Click **cylinder_block.step** on the landing page — one-click sample load.

---

## WASM API

```javascript
import init, { analyzeStep } from "./pkg/steprs_wasm.js";
await init();
const json = analyzeStep(stepText);  // → WebAnalysisResult JSON string
```

Legacy aliases `analyzeStepOptions` / `analyzeStepOptionsWithProgress` accept an options JSON string (ignored) for backward compatibility.

---

## Deploy

Push to `main` triggers GitHub Actions: `make wasm` → GitHub Pages → [steprs.dev](https://steprs.dev).

---

## License

[MIT](LICENSE)
