import {
  createProgressDriver,
  wait,
} from "./motion.js";
import { initTheme, onThemeChange } from "./theme.js";
import { createViewer3d } from "./viewer3d.js";

const $ = (id) => document.getElementById(id);

const SAMPLES = {
  "ap214_compact.step": "./samples/ap214_compact.step",
};

const PHASE_LABELS = {
  header: "reading header...",
  data_scan: "scanning entities...",
  indexing: "indexing records...",
  topology: "building B-rep...",
  features: "clustering coaxial faces...",
  post_process: "finishing...",
};

const KIND_LABELS = {
  blind_hole: "blind",
  through_hole: "through",
  coaxial_step_bore: "step bore",
  counterbore: "c'bore",
  countersink: "c'sink",
};

let wasm;
let worker;
let workerReady = false;
let workerId = 0;
let lastData = null;
let lastStepText = null;
let coaxialRows = [];
let analyzeGen = 0;
let selectedRow = 0;
let lastMs = 0;

const progress = createProgressDriver($("progress-bar"), $("progress"));
const viewer3d = createViewer3d($("canvas-3d"), {
  onStatus: setViewerStatus,
});

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

function setPhase(text) {
  const el = $("phase-label");
  if (!el) return;
  el.textContent = text || "";
  el.classList.toggle("hidden", !text);
}

function setViewerStatus(text) {
  const el = $("viewer-status");
  if (!el) return;
  el.textContent = text || "";
  el.classList.toggle("hidden", !text);
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
  } catch {
    setMessage("Run `make wasm` then reload.", true);
  }
}

function analyzeInWorker(text, onProgress) {
  return new Promise((resolve, reject) => {
    if (!worker || !workerReady) return reject(new Error("worker not ready"));
    const id = ++workerId;
    const handler = (ev) => {
      if (ev.data?.id !== id) return;
      if (ev.data.type === "progress") {
        const phase = ev.data.phase;
        setPhase(PHASE_LABELS[phase] ?? phase);
        const pct = mapProgress(phase, ev.data.done, ev.data.total ?? 0);
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
    setPhase(PHASE_LABELS[phase] ?? phase);
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

function diameterMm(radius) {
  if (radius == null || !Number.isFinite(radius)) return "";
  return (radius * 2).toFixed(3);
}

function formatKind(kind) {
  if (!kind) return "";
  return KIND_LABELS[kind] ?? String(kind).replaceAll("_", " ");
}

function buildCoaxialRows(holes) {
  return holes.map((f, i) => ({
    index: i + 1,
    kind: f.kind ?? "",
    kindLabel: formatKind(f.kind),
    label: f.label ?? "",
    radius_mm: diameterMm(f.radius),
    depth_mm: f.depth != null ? Number(f.depth).toFixed(3) : "",
    seg: f.face_ids?.length ?? 0,
  }));
}

function selectHoleRow(index, { scroll = true } = {}) {
  const rows = $("coaxial-table-body")?.querySelectorAll("tr");
  if (!rows?.length) return;
  selectedRow = Math.max(0, Math.min(index, rows.length - 1));
  rows.forEach((tr, i) => tr.classList.toggle("selected", i === selectedRow));
  viewer3d.highlightHole?.(selectedRow);
  const row = coaxialRows[selectedRow];
  const label = $("viewer-hole-label");
  if (label && row) {
    const segs = row.seg > 1 ? ` · ${row.seg} segs` : "";
    label.textContent = `${row.kindLabel} · Ø${row.radius_mm} × ${row.depth_mm} mm${segs}`;
  }
  if (scroll) {
    rows[selectedRow]?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
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
    if (countLabel) countLabel.textContent = "0 found";
    if (exportBtn) exportBtn.disabled = true;
    return;
  }

  table?.classList.remove("hidden");
  empty?.classList.add("hidden");
  if (exportBtn) exportBtn.disabled = false;
  if (countLabel) {
    countLabel.textContent = `${holes.length} coaxial cluster${holes.length === 1 ? "" : "s"} · mm`;
  }

  tbody.innerHTML = coaxialRows
    .map(
      (r, i) => `
    <tr data-index="${i}" class="${i === selectedRow ? "selected" : ""}" tabindex="-1">
      <td class="col-num">${r.index}</td>
      <td class="col-kind">${escapeHtml(r.kindLabel)}</td>
      <td class="col-label" title="${escapeHtml(r.label)}">${escapeHtml(r.label)}</td>
      <td class="col-num">${escapeHtml(r.radius_mm)}</td>
      <td class="col-num">${escapeHtml(r.depth_mm)}</td>
      <td class="col-num">${r.seg}</td>
    </tr>`
    )
    .join("");

  tbody.querySelectorAll("tr").forEach((tr) => {
    tr.addEventListener("click", () => selectHoleRow(Number(tr.dataset.index)));
  });

  if (coaxialRows.length) selectHoleRow(selectedRow, { scroll: false });
}

function renderMetrics(data, ms) {
  const coaxial = data.coaxial_holes?.length ?? 0;
  $("m-coaxial").textContent = String(coaxial);
  if ($("m-time")) $("m-time").textContent = ms != null ? `${ms} ms` : "-";
}

function exportCoaxialCsv() {
  if (!coaxialRows.length) return;
  const headers = ["index", "kind", "label", "diameter_mm", "depth_mm", "segments"];
  const esc = (v) => {
    const s = String(v ?? "");
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const lines = [headers.join(",")];
  for (const r of coaxialRows) {
    const row = {
      index: r.index,
      kind: r.kind,
      label: r.label,
      diameter_mm: r.radius_mm,
      depth_mm: r.depth_mm,
      segments: r.seg,
    };
    lines.push(headers.map((h) => esc(row[h])).join(","));
  }
  const blob = new Blob([lines.join("\n")], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "steprs-coaxial-holes.csv";
  a.click();
  URL.revokeObjectURL(url);
}

function presentResults(data, stepText) {
  $("workspace")?.classList.remove("hidden");
  renderMetrics(data, lastMs);
  renderCoaxialTable(data);
  requestAnimationFrame(() => {
    viewer3d.load(stepText, data);
    viewer3d.resize?.();
    if (data.coaxial_holes?.length) selectHoleRow(0, { scroll: false });
  });
}

function onLayoutChange() {
  viewer3d.resize?.();
}

window.addEventListener("resize", onLayoutChange);
if (typeof ResizeObserver !== "undefined") {
  const ro = new ResizeObserver(onLayoutChange);
  const viz = $("viz");
  const stage = $("stage");
  if (viz) ro.observe(viz);
  if (stage) ro.observe(stage);
}
onThemeChange(() => {
  onLayoutChange();
  if (lastStepText && lastData) viewer3d.load(lastStepText, lastData);
});

const MAX_UPLOAD_BYTES = 120 * 1024 * 1024;

function isStepText(text) {
  return text.includes("ISO-10303") || text.includes("DATA;");
}

async function loadSample(name) {
  const url = SAMPLES[name] ?? `./samples/${name}`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`sample not found (${res.status})`);
  const text = await res.text();
  if (!isStepText(text)) throw new Error("not a STEP Part 21 file");
  await analyzeText(text, name, text.length);
}

async function analyzeText(text, name, size) {
  if (!wasm) {
    setMessage("Run `make wasm` then reload.", true);
    return;
  }

  const gen = ++analyzeGen;
  document.body.classList.add("analyzing");
  $("dropzone")?.classList.add("is-busy");
  setMessage("");
  setPhase("starting...");
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

    setPhase("rendering...");
    progress.set(98);
    await wait(40);

    lastData = JSON.parse(json);
    lastStepText = text;
    lastMs = Math.round(performance.now() - t0);

    await progress.finish();
    if (gen !== analyzeGen) return;

    presentResults(lastData, text);
    document.body.classList.add("has-results");
    $("landing")?.classList.add("hidden");
    $("dropzone")?.classList.remove("hidden");
    $("status-bar")?.classList.remove("hidden");
    const label = $("drop-label");
    if (label) label.textContent = truncateName(name);
    selectedRow = 0;

    const coaxial = lastData.coaxial_holes?.length ?? 0;
    const proto = lastData.stats?.application_protocol || "STEP";
    $("file-meta").textContent =
      `${truncateName(name, 40)} · ${formatBytes(size)} · ${proto} · ${lastMs} ms · ${coaxial} bore${coaxial === 1 ? "" : "s"} · mm`;
    setMessage("");
  } catch (e) {
    if (gen !== analyzeGen) return;
    setMessage(explainError(e), true);
  } finally {
    if (gen === analyzeGen) {
      document.body.classList.remove("analyzing");
      $("dropzone")?.classList.remove("is-busy");
      setPhase("");
      await wait(180);
      $("progress")?.classList.add("hidden");
      progress.reset();
    }
  }
}

function explainError(e) {
  const s = String(e?.message ?? e);
  if (s.includes("worker not ready")) return "Engine still loading. Try again.";
  if (s.includes("not STEP") || s.includes("ISO-10303")) return "Not a STEP Part 21 file (.step / .stp / .p21).";
  return s;
}

function truncateName(name, max = 36) {
  if (!name || name.length <= max) return name;
  const ext = name.includes(".") ? name.slice(name.lastIndexOf(".")) : "";
  const stem = ext ? name.slice(0, -ext.length) : name;
  const keep = max - ext.length - 1;
  if (keep < 8) return name.slice(0, max - 1) + "...";
  return stem.slice(0, keep) + "..." + ext;
}

function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

$("export-csv")?.addEventListener("click", exportCoaxialCsv);

initTheme();

function openFilePicker() {
  const input = $("file-input");
  if (!input) return;
  input.value = "";
  input.click();
}

async function processFile(file) {
  if (!file) return;
  if (file.size === 0) {
    setMessage("Empty file.", true);
    return;
  }
  if (file.size > MAX_UPLOAD_BYTES) {
    setMessage("File exceeds 120 MB limit.", true);
    return;
  }
  let text;
  try {
    text = await file.text();
  } catch {
    setMessage("Couldn't read that file.", true);
    return;
  }
  if (!isStepText(text)) {
    setMessage("Not a STEP Part 21 file (.step / .stp / .p21).", true);
    return;
  }
  await analyzeText(text, file.name, file.size);
}

function bindDropTarget(el) {
  if (!el) return;
  el.addEventListener("click", (e) => {
    if (e.target.closest(".sample-link, #landing-cta, .landing-faq, button, a, input")) return;
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

document.querySelectorAll(".sample-link").forEach((btn) => {
  btn.addEventListener("click", async (e) => {
    e.stopPropagation();
    const name = btn.dataset.sample;
    if (!name) return;
    try {
      await loadSample(name);
    } catch (err) {
      setMessage(explainError(err), true);
    }
  });
});

document.addEventListener("keydown", (e) => {
  if (!document.body.classList.contains("has-results")) return;
  if (e.target.matches("input, textarea, select, button")) return;
  if (!coaxialRows.length) return;

  if (e.key === "ArrowDown" || e.key === "j") {
    e.preventDefault();
    selectHoleRow(selectedRow + 1);
  } else if (e.key === "ArrowUp" || e.key === "k") {
    e.preventDefault();
    selectHoleRow(selectedRow - 1);
  } else if (e.key === "Home") {
    e.preventDefault();
    selectHoleRow(0);
  } else if (e.key === "End") {
    e.preventDefault();
    selectHoleRow(coaxialRows.length - 1);
  }
});

initWasm();
