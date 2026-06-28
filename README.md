# steprs

**STEP → toolpath → G-code in your browser.** No upload. No server. Files never leave your machine.

Live at **[steprs.dev](https://steprs.dev)** · Source at **[github.com/vladimir-io/step-rs](https://github.com/vladimir-io/step-rs)**

Drop a `.step` file, get Fanuc / Haas / GRBL / ISO NC with 3D preview, path playback, and stock-removal simulation.

## Why steprs

| | |
|---|---|
| **Private** | WASM runs locally — your CAD never hits a server |
| **Fast** | Streaming STEP parse, worker-thread analysis |
| **Inspectable** | Open source Rust pipeline you can audit before running metal |

## Pipeline

```text
STEP text
  → steprs-core      parse (fail-soft streaming)
  → steprs-schema    typed entity cache + mesh extract
  → steprs-topology  B-rep faces / adjacency
  → steprs-features  pockets, holes, preview geometry
  → steprs-path      UV toolpaths, G81 holes, stock sim, post
  → steprs-wasm      slim JSON to browser
```

```text
STEP → streaming parse → schema registry → B-rep (outer/inner loops)
  → features (coaxial bores + planar pockets/slots)
  → UV-frame zigzag toolpaths with stepdown
  → post-processed G-code (Fanuc / Haas / GRBL / ISO)
  → swept-disc stock simulation + WebGL preview
```

## Capabilities

| Area | Behavior |
|------|----------|
| **Pockets** | Inner `FACE_BOUND` loops, recess floors, ordered edge walk + LINE geometry |
| **Toolpaths** | Face-local UV raster, stepdown, no hard-coded fallback rectangles |
| **Stock sim** | Local grid carve, arc sampling, volume removed, gouge + overcut counts |
| **AP214/242** | Schema-number protocol detection; `TESSELLATED_*` mesh ingestion |
| **Posts** | Fanuc, Haas, GRBL, ISO with NC validation |

## Quick start

```bash
# Run tests
make test

# CLI analysis
cargo run -p steprs-cli -- analyze samples/cylinder_block.step
cargo run -p steprs-cli -- analyze samples/rectangular_pocket.step --post haas
cargo run -p steprs-cli -- analyze samples/customer_00167362.step --json

# Local web UI
make wasm && make serve   # http://localhost:8080
make test-web             # WASM build + browser E2E smoke test
```

**Requirements:** Rust stable, `wasm-pack` (`cargo install wasm-pack`), Python 3 (for local serve).

## WASM API

```javascript
wasm.analyzeStepOptions(text, JSON.stringify({
  emit_gcode: true,
  simulate_stock: true,
  post: { processor: "grbl", wcs: "G54" }
}));
```

## Safety

Generated G-code is a **starting point for review**, not a production-ready program. Always verify toolpaths, feeds, speeds, and clearance planes in your CAM workflow before cutting metal.

## License

Dual-licensed under **MIT OR Apache-2.0** — your choice. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
