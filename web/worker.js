/** Off-main-thread STEP analysis (large files). */
let wasmReady = null;

async function ensureWasm() {
  if (wasmReady) return wasmReady;
  wasmReady = (async () => {
    const mod = await import("./pkg/steprs_wasm.js");
    await mod.default();
    return mod;
  })();
  return wasmReady;
}

self.onmessage = async (event) => {
  const { id, type, text, includeGcode, optionsJson } = event.data;
  try {
    if (type === "init") {
      await ensureWasm();
      self.postMessage({ id, ok: true, type: "ready" });
      return;
    }
    if (type === "analyze") {
      const mod = await ensureWasm();
      const onProgress = (phase, done, total) => {
        self.postMessage({ id, ok: true, type: "progress", phase, done, total });
      };
      const json = mod.analyzeStepOptionsWithProgress
        ? mod.analyzeStepOptionsWithProgress(text, optionsJson || "{}", onProgress)
        : mod.analyzeStepOptions
          ? mod.analyzeStepOptions(text, optionsJson || "{}")
          : mod.analyzeStep(text, includeGcode ?? true);
      self.postMessage({ id, ok: true, type: "result", json });
      return;
    }
    self.postMessage({ id, ok: false, error: "unknown message type" });
  } catch (err) {
    self.postMessage({ id, ok: false, error: String(err) });
  }
};
