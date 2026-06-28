# Changelog

## 1.0.0 — 2026-06-27

First public release.

### Web app
- Drop-zone STEP import with sample files
- 3D part + toolpath preview (WebGL)
- XY / XZ path playback with stock-removal heatmap
- G-code output with Fanuc, Haas, GRBL, and ISO post-processors
- NC validation, copy, and download
- Dark / light theme

### Engine
- Streaming STEP parser (AP214 / AP242)
- B-rep topology, coaxial bore + planar pocket detection
- UV-frame zigzag toolpaths with stepdown
- Swept-disc stock simulation
- WASM bindings for browser worker offload

### Tooling
- `steprs-cli` for headless analysis
- Playwright E2E smoke tests
- GitHub Actions CI + Pages deploy
