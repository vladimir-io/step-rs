import {
  animateNumber,
  collapseSections,
  createProgressDriver,
  revealStagger,
  wait,
} from "./motion.js";
import { initTheme, onThemeChange } from "./theme.js";
import { createViewer3d } from "./viewer3d.js";

const $ = (id) => document.getElementById(id);

const FLOW_SECTIONS = () => [$("workspace")];

const SAMPLES = {
  "cylinder_block.step": "./samples/cylinder_block.step",
  "customer_00167362.step": "./samples/customer_00167362.step",
};

let wasm;
let worker;
let workerReady = false;
let workerId = 0;
let lastData = null;
let coaxialRows = [];
let analyzeGen = 0;
let systemTestsDone = false;
let ipMaskingEnabled = false;

const progress = createProgressDriver($("progress-bar"), $("progress"));
const viewer3d = createViewer3d($("canvas-3d"));

const PHASE_WEIGHTS = {
  data_scan: [0, 65],
  indexing: [65, 72],
  topology: [72, 82],
  features: [82, 98],
};

function mapProgress(phase, done, total) {
  const w = PHASE_WEIGHTS[phase];
  if (!w) return null;
  const [lo, hi] = w;
  const span = hi - lo;
  if (total > 0) return lo + span * Math.min(1, done / total);
  return lo + span * 0.5;
}

function initPageEntrance() {
  document.body.classList.add("is-ready");
}

async function initWasm() {
  try {
    const mod = await import("./pkg/steprs_wasm.js");
    await mod.default();
    wasm = mod;
    worker = new Worker("./worker.js", { type: "module" });
    await new Promise((resolve) => {
      worker.addEventListener("message", function onReady(ev) {
        if (ev.data?.type === "ready") {
          workerReady = true;
          worker.removeEventListener("message", onReady);
          resolve();
        }
      });
      worker.postMessage({ id: 0, type: "init" });
    });
    await runSystemTests();
  } catch (e) {
    setMessage("make wasm", true);
    console.warn(e);
  }
}

function analyzeInWorker(text, onProgress) {
  return new Promise((resolve, reject) => {
    if (!worker || !workerReady) return reject(new Error("worker not ready"));
    const id = ++workerId;
    const handler = (ev) => {
      if (ev.data?.id !== id) return;
      if (ev.data.type === "progress") {
        const pct = mapProgress(ev.data.phase, ev.data.done, ev.data.total ?? 0);
        if (pct != null) onProgress?.(pct);
        return;
      }
      worker.removeEventListener("message", handler);
      if (ev.data.ok) resolve(ev.data.json);
      else reject(new Error(ev.data.error));
    };
    worker.addEventListener("message", handler);
    worker.postMessage({ id, type: "analyze", text });
  });
}

async function analyze(content, onProgress) {
  if (workerReady) return analyzeInWorker(content, onProgress);
  return wasm.analyzeStepOptionsWithProgress(content, "{}", (phase, done, total) => {
    const pct = mapProgress(phase, done, total ?? 0);
    if (pct != null) onProgress?.(pct);
  });
}

function setMessage(text, isError = false) {
  const el = $("message");
  el.textContent = text;
  el.classList.toggle("error", isError);
  el.classList.toggle("hidden", !text);
}

function escapeHtml(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function vec3(v) {
  if (!v) return null;
  if (Array.isArray(v) && v.length >= 3) return v;
  if (v.x != null) return [v.x, v.y, v.z];
  return null;
}

function fmtNum(n, d = 3) {
  if (n == null || !Number.isFinite(n)) return "—";
  return n.toFixed(d);
}

function buildCoaxialRows(holes) {
  return holes.map((f, i) => {
    const o = vec3(f.axis_origin);
    const d = vec3(f.axis_direction);
    return {
      index: i + 1,
      kind: f.kind ?? "",
      label: f.label ?? "",
      face_ids: (f.face_ids ?? []).join(","),
      radius_mm: f.radius != null ? f.radius.toFixed(4) : "",
      depth_mm: f.depth != null ? f.depth.toFixed(4) : "",
      ox: o ? fmtNum(o[0]) : "",
      oy: o ? fmtNum(o[1]) : "",
      oz: o ? fmtNum(o[2]) : "",
      dx: d ? fmtNum(d[0], 4) : "",
      dy: d ? fmtNum(d[1], 4) : "",
      dz: d ? fmtNum(d[2], 4) : "",
      seg: f.face_ids?.length ?? 0,
    };
  });
}

function renderCoaxialTable(data) {
  const holes = data?.coaxial_holes ?? [];
  const tbody = $("coaxial-table-body");
  const table = $("coaxial-table");
  const empty = $("coaxial-empty");
  const exportBtn = $("export-csv");
  const countLabel = $("coaxial-count-label");

  coaxialRows = buildCoaxialRows(holes);

  if (!holes.length) {
    tbody.innerHTML = "";
    table?.classList.add("hidden");
    empty?.classList.remove("hidden");
    if (countLabel) countLabel.textContent = "0 clusters";
    if (exportBtn) exportBtn.disabled = true;
    return;
  }

  table?.classList.remove("hidden");
  empty?.classList.add("hidden");
  if (exportBtn) exportBtn.disabled = false;
  if (countLabel) countLabel.textContent = `${holes.length} cluster${holes.length === 1 ? "" : "s"}`;

  tbody.innerHTML = coaxialRows
    .map(
      (r) => `
    <tr>
      <td>${r.index}</td>
      <td>${escapeHtml(r.kind)}</td>
      <td class="col-label">${escapeHtml(r.label)}</td>
      <td class="col-ids">${escapeHtml(r.face_ids)}</td>
      <td>${escapeHtml(r.radius_mm)}</td>
      <td>${escapeHtml(r.depth_mm)}</td>
      <td>${escapeHtml(r.ox)}</td>
      <td>${escapeHtml(r.oy)}</td>
      <td>${escapeHtml(r.oz)}</td>
      <td>${escapeHtml(r.dx)}</td>
      <td>${escapeHtml(r.dy)}</td>
      <td>${escapeHtml(r.dz)}</td>
      <td>${r.seg}</td>
    </tr>`
    )
    .join("");
}

function renderDiagnostics(data) {
  const el = $("diag-out");
  if (!el) return;
  const s = data.stats;
  const lines = [];
  if (s.parse_errors > 0) lines.push(`${s.parse_errors} parse skips`);
  if (!lines.length) {
    el.classList.add("hidden");
    el.innerHTML = "";
    return;
  }
  el.classList.remove("hidden");
  el.innerHTML = lines.map((l) => `<span class="diag-line">${escapeHtml(l)}</span>`).join("");
}

async function animateMetrics(data) {
  const s = data.stats;
  const coaxial = data.coaxial_holes?.length ?? 0;
  const b = data.brep;

  await Promise.all([
    animateNumber($("m-records"), s.record_count, { duration: 300, decimals: 0 }),
    animateNumber($("m-coaxial"), coaxial, { duration: 300, decimals: 0 }),
  ]);

  if ($("m-faces")) $("m-faces").textContent = b?.face_count != null ? String(b.face_count) : "—";
  if ($("m-protocol")) $("m-protocol").textContent = s.application_protocol || "—";
  if ($("m-unit")) $("m-unit").textContent = s.length_unit ?? "mm";
}

function renderSystemTestLists(cases) {
  const html = cases
    .map((c) => {
      const cls = c.pass ? "pass" : "fail";
      const mark = c.pass ? "✓" : "✗";
      return `<li class="system-test-row ${cls}"><span class="system-test-mark">${mark}</span><span class="system-test-name">${escapeHtml(c.name)}</span><span class="system-test-detail">${escapeHtml(c.detail)}</span></li>`;
    })
    .join("");

  for (const id of ["system-tests-list", "system-tests-landing-list"]) {
    const el = $(id);
    if (el) el.innerHTML = html;
  }
}

async function runSystemTests() {
  if (!wasm || systemTestsDone) return;
  const cases = [];

  try {
    const cylinder = JSON.parse(wasm.verifyCylinderBlockRegression());
    cases.push(cylinder);
  } catch (e) {
    cases.push({ name: "cylinder_block.step", pass: false, detail: String(e) });
  }

  renderSystemTestLists(
    cases.concat([{ name: "customer suite", pass: false, detail: "running…" }])
  );

  try {
    const specs = JSON.parse(wasm.regressionSpecs());
    for (const spec of specs) {
      const url = SAMPLES[spec.name] ?? `./samples/${spec.name}`;
      try {
        const res = await fetch(url);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const text = await res.text();
        const result = JSON.parse(wasm.verifyRegressionSample(text, JSON.stringify(spec)));
        cases.push(result);
      } catch (e) {
        cases.push({ name: spec.name, pass: false, detail: String(e) });
      }
      renderSystemTestLists(cases);
    }
  } catch (e) {
    cases.push({ name: "customer suite", pass: false, detail: String(e) });
  }

  systemTestsDone = true;
  renderSystemTestLists(cases);
}

function updateExportButton() {
  const btn = $("export-csv");
  if (!btn) return;
  btn.textContent = ipMaskingEnabled ? "Export Manifest" : "Export CSV";
}

function refreshManifestPreview() {
  const panel = $("manifest-panel");
  const pre = $("manifest-preview");
  const copyBtn = $("copy-manifest");
  if (!panel || !pre) return;

  const show = ipMaskingEnabled && lastData?.structural_summary;
  panel.classList.toggle("hidden", !show);
  if (!show) {
    pre.textContent = "";
    if (copyBtn) copyBtn.disabled = true;
    return;
  }

  const text = JSON.stringify(lastData.structural_summary, null, 2);
  pre.textContent = text;
  if (copyBtn) copyBtn.disabled = false;
}

function initIpMaskToggle() {
  const toggle = $("ip-mask-toggle");
  if (!toggle) return;
  try {
    ipMaskingEnabled = localStorage.getItem("steprs-ip-mask") === "1";
    toggle.checked = ipMaskingEnabled;
  } catch {
    /* ignore */
  }
  toggle.addEventListener("change", () => {
    ipMaskingEnabled = toggle.checked;
    try {
      localStorage.setItem("steprs-ip-mask", ipMaskingEnabled ? "1" : "0");
    } catch {
      /* ignore */
    }
    updateExportButton();
    refreshManifestPreview();
  });
  updateExportButton();
  refreshManifestPreview();
}

function exportData() {
  if (ipMaskingEnabled) exportStructuralManifest();
  else exportCoaxialCsv();
}

function exportStructuralManifest() {
  const summary = lastData?.structural_summary;
  if (!summary) return;
  const text = JSON.stringify(summary, null, 2);
  const blob = new Blob([text], { type: "application/json;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "steprs-structural-manifest.json";
  a.click();
  URL.revokeObjectURL(url);
}

function exportCoaxialCsv() {
  if (!coaxialRows.length) return;
  const headers = [
    "index",
    "kind",
    "label",
    "face_ids",
    "radius_mm",
    "depth_mm",
    "ox",
    "oy",
    "oz",
    "dx",
    "dy",
    "dz",
    "seg",
  ];
  const esc = (v) => {
    const s = String(v ?? "");
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const lines = [headers.join(",")];
  for (const r of coaxialRows) {
    lines.push(headers.map((h) => esc(r[h])).join(","));
  }
  const blob = new Blob([lines.join("\n")], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "steprs-coaxial-holes.csv";
  a.click();
  URL.revokeObjectURL(url);
}

async function presentResults(data) {
  await revealStagger(FLOW_SECTIONS());
  await animateMetrics(data);
  renderDiagnostics(data);
  renderCoaxialTable(data);
  refreshManifestPreview();
  requestAnimationFrame(() => {
    viewer3d.load(data);
    viewer3d.resize?.();
  });
}

function onLayoutChange() {
  viewer3d.resize?.();
  if (lastData) viewer3d.refresh?.();
}

window.addEventListener("resize", onLayoutChange);
if (typeof ResizeObserver !== "undefined") {
  const ro = new ResizeObserver(onLayoutChange);
  const viz = $("viz");
  const stage = $("stage");
  if (viz) ro.observe(viz);
  if (stage) ro.observe(stage);
}
onThemeChange(() => onLayoutChange());

async function analyzeText(text, name, size) {
  if (!wasm) {
    setMessage("make wasm", true);
    return;
  }

  const gen = ++analyzeGen;
  if (!document.body.classList.contains("has-results")) {
    await collapseSections(FLOW_SECTIONS());
  }
  if (gen !== analyzeGen) return;

  document.body.classList.remove("has-results");
  document.body.classList.add("analyzing");
  $("dropzone")?.classList.add("is-busy");
  setMessage("");
  progress.reset();
  $("progress")?.classList.remove("hidden");
  progress.indeterminate(false);
  progress.set(2);

  try {
    const t0 = performance.now();
    const json = await analyze(text, (pct) => {
      if (gen === analyzeGen) progress.set(pct);
    });
    if (gen !== analyzeGen) return;

    progress.set(98);
    await wait(40);

    lastData = JSON.parse(json);
    const ms = Math.round(performance.now() - t0);

    await progress.finish();
    if (gen !== analyzeGen) return;

    await presentResults(lastData);
    document.body.classList.add("has-results");
    $("landing")?.classList.add("hidden");
    $("dropzone")?.classList.remove("hidden");
    $("status-bar")?.classList.remove("hidden");
    const label = $("drop-label");
    if (label) label.textContent = name;

    const coaxial = lastData.coaxial_holes?.length ?? 0;
    const unit = lastData.stats?.length_unit ?? "mm";
    $("file-meta").textContent = `${name} · ${formatBytes(size)} · ${ms} ms · ${coaxial} coaxial · ${unit}`;
    setMessage("");
  } catch (e) {
    if (gen !== analyzeGen) return;
    setMessage(String(e), true);
  } finally {
    if (gen === analyzeGen) {
      document.body.classList.remove("analyzing");
      $("dropzone")?.classList.remove("is-busy");
      await wait(300);
      $("progress")?.classList.add("hidden");
      progress.reset();
    }
  }
}

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

$("export-csv")?.addEventListener("click", exportData);

$("copy-manifest")?.addEventListener("click", async () => {
  const text = $("manifest-preview")?.textContent;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    const btn = $("copy-manifest");
    const prev = btn.textContent;
    btn.textContent = "Copied";
    setTimeout(() => {
      btn.textContent = prev;
    }, 1200);
  } catch {
    exportStructuralManifest();
  }
});

initTheme();
initPageEntrance();
initIpMaskToggle();

function openFilePicker() {
  const input = $("file-input");
  if (!input) return;
  input.value = "";
  input.click();
}

async function processFile(file) {
  const text = await file.text();
  await analyzeText(text, file.name, file.size);
}

function bindDropTarget(el) {
  if (!el) return;
  el.addEventListener("click", (e) => {
    if (e.target.closest(".sample-chip, .landing-dock, #landing-cta, button, a, input")) return;
    openFilePicker();
  });
  el.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openFilePicker();
    }
  });
  el.addEventListener("dragover", (e) => {
    e.preventDefault();
    el.classList.add("dragover");
  });
  el.addEventListener("dragleave", (e) => {
    if (e.currentTarget === e.target || !el.contains(e.relatedTarget)) el.classList.remove("dragover");
  });
  el.addEventListener("drop", (e) => {
    e.preventDefault();
    el.classList.remove("dragover");
    const f = e.dataTransfer?.files?.[0];
    if (f) processFile(f);
  });
}

bindDropTarget($("landing"));
$("landing-cta")?.addEventListener("click", (e) => {
  e.stopPropagation();
  openFilePicker();
});
$("dropzone")?.addEventListener("click", () => openFilePicker());

$("file-input")?.addEventListener("change", () => {
  const f = $("file-input").files[0];
  if (f) processFile(f);
});

document.querySelectorAll(".sample-chip").forEach((btn) => {
  btn.addEventListener("click", async (e) => {
    e.stopPropagation();
    const name = btn.dataset.sample;
    const url = SAMPLES[name] ?? `./samples/${name}`;
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`Sample not found (${res.status})`);
      const text = await res.text();
      await analyzeText(text, name, text.length);
    } catch (err) {
      setMessage(String(err), true);
    }
  });
});

initWasm();
