# steprs

**STEP → toolpath → G-code in your browser.** No upload. No server. Files never leave your machine.

**[steprs.dev](https://steprs.dev)** · **[github.com/vladimir-io/step-rs](https://github.com/vladimir-io/step-rs)**

Drop a `.step` file, get Fanuc / Haas / GRBL / ISO NC with 3D preview, path playback, and stock-removal simulation.

## Why steprs

- **Private** — WASM runs locally; your CAD never hits a server
- **Fast** — streaming STEP parse, worker-thread analysis
- **Inspectable** — open-source Rust pipeline you can audit before running metal

## Pipeline

```text
STEP → parse → schema → B-rep → features → toolpaths → post → G-code
```

| Crate | Role |
|-------|------|
| `steprs-core` | Streaming STEP parse |
| `steprs-schema` | Typed entity cache + mesh |
| `steprs-topology` | B-rep faces / adjacency |
| `steprs-features` | Pockets, bores, preview geometry |
| `steprs-path` | UV toolpaths, stock sim, post-processors |
| `steprs-wasm` | Browser bindings |

Supports AP214/AP242, coaxial bores, planar pockets/slots, tessellated mesh ingestion, and swept-disc stock simulation with gouge/overcut counts.

## Quick start

```bash
make test
cargo run -p steprs-cli -- analyze samples/cylinder_block.step
cargo run -p steprs-cli -- analyze samples/rectangular_pocket.step --post haas
make wasm && make serve   # http://localhost:8080
```

**Requirements:** Rust stable, `wasm-pack`, Python 3 (for local serve).

## WASM API

```javascript
wasm.analyzeStepOptions(text, JSON.stringify({
  emit_gcode: true,
  simulate_stock: true,
  post: { processor: "grbl", wcs: "G54" }
}));
```

## Safety

Generated G-code is a **starting point for review**, not a production-ready program. Always verify toolpaths, feeds, speeds, and clearance planes before cutting metal.

## License

[MIT](LICENSE)
