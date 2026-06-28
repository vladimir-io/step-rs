# steprs

**Coaxial bore and hole detection from STEP.** Rust B-rep, runs in your browser — files never uploaded.

**[steprs.dev](https://steprs.dev)** · **[github.com/vladimir-io/step-rs](https://github.com/vladimir-io/step-rs)**

## The wedge

Most STEP viewers show geometry. steprs **recognizes machinable holes** from the B-rep:

- **Coaxial step bores** — cluster aligned cylindrical faces, report Ø large → Ø small, segment count, depth
- **Through & blind holes** — isolated cylindrical faces with axis, radius, depth
- **Planar pockets & slots** — inner `FACE_BOUND` loops on recess floors (synthetic + simple parts)

Tested against real **AP214 customer STEP** exports. This is the reliable core.

## What it is not

Not CAM. Not toolpath optimization. Not a Fusion replacement.

G-code output is a **preview export** (G81 for detected bores, basic pocket raster) — useful for inspection, not production programs.

## Pipeline

```text
STEP → parse → schema → B-rep → feature detection → hole table + 3D preview
```

| Crate | Role |
|-------|------|
| `steprs-core` | Streaming STEP parse |
| `steprs-schema` | Typed entity cache |
| `steprs-topology` | B-rep faces, cylindrical axes |
| `steprs-features` | **Coaxial bore + hole detection** |
| `steprs-path` | Optional G81 / pocket preview export |
| `steprs-wasm` | Browser bindings |

## Quick start

```bash
make test
cargo run -p steprs-cli -- analyze samples/cylinder_block.step
cargo run -p steprs-cli -- analyze samples/customer_00167362.step --json
make wasm && make serve   # http://localhost:8080
```

## WASM API

```javascript
wasm.analyzeStepOptions(text, JSON.stringify({
  emit_gcode: false,       // hole detection only
  emit_toolpath: false,
  simulate_stock: false,
}));
```

## License

[MIT](LICENSE)
