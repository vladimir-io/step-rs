import { createSimulator } from "./simulator.js";
import {
  animateNumber,
  collapseSections,
  createProgressDriver,
  revealStagger,
  wait,
} from "./motion.js";
import { initTheme, onThemeChange } from "./theme.js";
import { createViewer3d } from "./viewer3d.js";
import { clearStockCache, drawStockHeatmap } from "./stock-sim.js";

const $ = (id) => document.getElementById(id);

const FLOW_SECTIONS = () => [$("workspace")];

const SAMPLES = {
  "cylinder_block.step": "./samples/cylinder_block.step",
  "rectangular_pocket.step": "./samples/rectangular_pocket.step",
  "faceted_prism.step": "./samples/faceted_prism.step",
  "customer_00167362.step": "./samples/customer_00167362.step",
};

let wasm;
let worker;
let workerReady = false;
let workerId = 0;
let activeTab = "gcode";
let lastData = null;
let analyzeGen = 0;

const simulator = createSimulator($("canvas-xy"), $("canvas-xz"), {
  readout: $("sim-readout"),
  playBtn: $("sim-play"),
});
const viewer3d = createViewer3d($("canvas-3d"));
const progress = createProgressDriver($("progress-bar"), $("progress"));

function codeEl() {
  return $("code-out")?.querySelector("code") ?? $("code-out");
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
    worker.addEventListener("message", (ev) => {
      if (ev.data?.type === "ready") workerReady = true;
    });
    worker.postMessage({ id: 0, type: "init" });
  } catch (e) {
    setMessage("make wasm", true);
    console.warn(e);
  }
}

function buildOptions() {
  const post = $("post-select").value;
  const dia = parseFloat($("tool-dia")?.value) || 6;
  const feed = parseFloat($("tool-feed")?.value) || 800;
  return JSON.stringify({
    emit_gcode: $("gcode-toggle").checked,
    emit_toolpath: true,
    validate_gcode: true,
    simulate_stock: true,
    stock_grid: 160,
    tool: {
      diameter_mm: dia,
      feed_mm_min: feed,
      spindle_rpm: 12000,
      safe_z_mm: 5,
    },
    post: {
      processor: post,
      wcs: "g54",
      program_number: 1000,
      line_numbers: true,
      tool_number: 1,
    },
  });
}

const PHASE_WEIGHTS = {
  data_scan: [0, 62],
  indexing: [62, 68],
  topology: [68, 76],
  features: [76, 84],
  toolpath: [84, 90],
  stock_sim: [90, 95],
  post_process: [95, 98],
};

function mapProgress(phase, done, total) {
  const w = PHASE_WEIGHTS[phase];
  if (!w) return null;
  const [lo, hi] = w;
  const span = hi - lo;
  if (total > 0) return lo + span * Math.min(1, done / total);
  return lo + span * 0.5;
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
    worker.postMessage({ id, type: "analyze", text, optionsJson: buildOptions() });
  });
}

async function analyze(content, onProgress) {
  const opts = buildOptions();
  if (workerReady) return analyzeInWorker(content, onProgress);
  if (wasm.analyzeStepOptionsWithProgress) {
    return wasm.analyzeStepOptionsWithProgress(content, opts, (phase, done, total) => {
      const pct = mapProgress(phase, done, total ?? 0);
      if (pct != null) onProgress?.(pct);
    });
  }
  if (wasm.analyzeStepOptions) return wasm.analyzeStepOptions(content, opts);
  return wasm.analyzeStep(content, $("gcode-toggle").checked);
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

function humanizeKind(kind) {
  if (!kind) return "";
  return String(kind)
    .replace(/_/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

function fmtDims(f) {
  const parts = [];
  if (f.radius != null) parts.push({ label: "ø", value: f.radius.toFixed(2) });
  if (f.depth != null) parts.push({ label: "depth", value: `${f.depth.toFixed(1)} mm` });
  if (f.width != null && f.length != null)
    parts.push({ label: "size", value: `${f.width.toFixed(1)}×${f.length.toFixed(1)}` });
  return parts;
}

function fmtVol(vol) {
  if (vol == null) return "—";
  if (vol >= 1e6) return `${(vol / 1e6).toFixed(2)} cm³`;
  if (vol >= 1000) return `${(vol / 1000).toFixed(1)}k mm³`;
  return `${Math.round(vol)} mm³`;
}

function renderDiagnostics(data) {
  const el = $("diag-out");
  if (!el) return;
  const lines = [];
  const s = data.stats;
  if (s.parse_errors > 0) lines.push(`${s.parse_errors} parse skips`);
  const v = data.gcode_validation;
  if (v && !v.valid) lines.push("NC invalid");
  if (v?.warnings?.length) lines.push(...v.warnings.slice(0, 2));
  const sim = data.stock_simulation;
  if (sim?.collision_cells > 0) lines.push(`${sim.collision_cells} air cuts`);
  if (sim?.gouge_cells > 0) lines.push(`${sim.gouge_cells} gouge`);
  if (sim?.overcut_cells > 0) lines.push(`${sim.overcut_cells} overcut`);
  if (!lines.length) {
    el.classList.add("hidden");
    el.innerHTML = "";
    return;
  }
  el.classList.remove("hidden");
  el.innerHTML = lines.map((l) => `<span class="diag-line">${escapeHtml(l)}</span>`).join("");
}

function renderFeatures(data) {
  const feats = data?.features?.features ?? [];
  const out = $("features-out");
  if (!feats.length) {
    out.innerHTML = `<p class="empty-hint">No machinable features detected</p>`;
    return;
  }
  out.innerHTML = feats
    .map((f) => {
      const dims = fmtDims(f);
      const conf = f.confidence != null ? Math.round(f.confidence * 100) : null;
      return `
      <article class="feat-card">
        <div class="feat-card-head">
          <p class="feat-name">${escapeHtml(f.label)}</p>
          <span class="feat-badge">${escapeHtml(humanizeKind(f.kind))}</span>
        </div>
        ${dims.length ? `<div class="feat-dims">${dims.map((d) => `<span class="feat-dim">${escapeHtml(d.label)} ${escapeHtml(d.value)}</span>`).join("")}</div>` : ""}
        ${conf != null ? `<p class="feat-conf">${conf}% confidence</p>` : ""}
      </article>`;
    })
    .join("");
}

function renderPathStats(data) {
  const el = $("path-stats");
  const panel = $("path-stats-panel");
  if (!el) return;
  const tp = data.toolpath;
  if (!tp) {
    panel?.classList.add("hidden");
    return;
  }
  panel?.classList.remove("hidden");
  const s = tp.stats;
  const rows = [
    ["Segments", s.segment_count],
    ["Linear", s.linear_count],
    ["Arcs", s.arc_count],
    ["Rapids", s.rapid_count],
    ["Cut", `${s.estimated_cut_length_mm.toFixed(1)} mm`],
    ["Rapid dist", `${s.estimated_rapid_length_mm.toFixed(1)} mm`],
    ["Est. time", `${s.estimated_time_min.toFixed(2)} min`],
    ["Tool Ø", `${tp.tool_diameter_mm} mm`],
  ];
  el.innerHTML = rows
    .map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${escapeHtml(String(v))}</dd>`)
    .join("");
}

function renderBrepStats(data) {
  const el = $("brep-stats");
  const panel = $("brep-panel");
  if (!el) return;
  const b = data.brep;
  const m = data.mesh;
  const fs = data.features?.summary;
  if (!b && !m) {
    panel?.classList.add("hidden");
    return;
  }
  panel?.classList.remove("hidden");
  const rows = [];
  if (b) {
    rows.push(["Solids", b.solid_count ?? "—"]);
    rows.push(["Faces", b.face_count ?? "—"]);
    rows.push(["Adjacency", b.adjacency_count ?? "—"]);
  }
  if (m?.vertices?.length) {
    const vcount = Array.isArray(m.vertices[0]) ? m.vertices.length : Math.floor(m.vertices.length / 3);
    rows.push(["Mesh verts", vcount]);
  }
  if (fs) {
    const parts = [];
    if (fs.hole_count) parts.push(`${fs.hole_count} holes`);
    if (fs.pocket_count) parts.push(`${fs.pocket_count} pockets`);
    if (fs.slot_count) parts.push(`${fs.slot_count} slots`);
    if (parts.length) rows.push(["Summary", parts.join(", ")]);
  }
  el.innerHTML = rows
    .map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${escapeHtml(String(v))}</dd>`)
    .join("");
}

function renderStockOverlay(data) {
  const el = $("stock-overlay");
  if (!el) return;
  const sim = data.stock_simulation;
  if (!sim) {
    el.textContent = "—";
    return;
  }
  const parts = [`${fmtVol(sim.removed_volume_mm3)} removed`];
  if (sim.gouge_cells) parts.push(`${sim.gouge_cells} gouge`);
  if (sim.overcut_cells) parts.push(`${sim.overcut_cells} overcut`);
  if (sim.collision_cells) parts.push(`${sim.collision_cells} air`);
  el.textContent = parts.join(" · ");
}

async function animateMetrics(data) {
  const s = data.stats;
  const cutMm = data.toolpath?.stats?.estimated_cut_length_mm;
  const timeMin = data.toolpath?.stats?.estimated_time_min;

  await Promise.all([
    animateNumber($("m-records"), s.record_count, { duration: 400, decimals: 0 }),
    animateNumber($("m-features"), data.features.features.length, {
      duration: 400,
      decimals: 0,
    }),
    cutMm != null
      ? animateNumber($("m-cut"), cutMm, {
          duration: 400,
          decimals: 1,
          formatter: (v) => `${v.toFixed(1)} mm`,
        })
      : Promise.resolve(($("m-cut").textContent = "—")),
  ]);

  const typesEl = $("m-types");
  if (typesEl) typesEl.textContent = `${s.type_count} types`;

  const protoEl = $("m-protocol");
  if (protoEl) {
    protoEl.textContent = s.application_protocol?.replace(/^AP/, "AP") || "—";
    protoEl.className = "metric-value metric-sm";
  }

  const schemaEl = $("m-schema");
  if (schemaEl) schemaEl.textContent = s.schema || "—";

  const featSumEl = $("m-feat-summary");
  if (featSumEl) {
    const fs = data.features?.summary;
    if (fs) {
      const parts = [];
      if (fs.hole_count) parts.push(`${fs.hole_count} holes`);
      if (fs.pocket_count) parts.push(`${fs.pocket_count} pockets`);
      if (fs.slot_count) parts.push(`${fs.slot_count} slots`);
      featSumEl.textContent = parts.length ? parts.join(" · ") : "detected";
    } else {
      featSumEl.textContent = "detected";
    }
  }

  const timeEl = $("m-time");
  if (timeEl) {
    timeEl.textContent =
      timeMin != null ? `~${timeMin.toFixed(2)} min est.` : "—";
  }

  const stockSim = data.stock_simulation;
  const stockEl = $("m-stock");
  const stockDetailEl = $("m-stock-detail");
  if (stockSim) {
    const vol = stockSim.removed_volume_mm3;
    if (vol >= 1000) {
      await animateNumber(stockEl, vol / 1000, {
        duration: 400,
        decimals: 1,
        formatter: (v) => `${v.toFixed(1)}k mm³`,
      });
    } else {
      await animateNumber(stockEl, vol, {
        duration: 400,
        decimals: 0,
        formatter: (v) => `${Math.round(v)} mm³`,
      });
    }
    const warn = stockSim.gouge_cells > 0 || (stockSim.overcut_cells ?? 0) > 0;
    stockEl.className = warn ? "metric-value warn" : "metric-value";
    if (stockDetailEl) {
      const parts = [];
      if (stockSim.gouge_cells) parts.push(`${stockSim.gouge_cells} gouge`);
      if (stockSim.overcut_cells) parts.push(`${stockSim.overcut_cells} overcut`);
      stockDetailEl.textContent = parts.length ? parts.join(" · ") : "clean pass";
    }
  } else {
    stockEl.textContent = "—";
    stockEl.className = "metric-value";
    if (stockDetailEl) stockDetailEl.textContent = "no simulation";
  }

  const ncEl = $("m-nc");
  const ncDetailEl = $("m-nc-detail");
  const ncCard = $("m-nc-card");
  const v = data.gcode_validation;
  if (ncEl) {
    ncCard?.classList.remove("ok", "warn", "fail");
    if (!data.gcode) {
      ncEl.textContent = "—";
      ncEl.className = "metric-value";
      if (ncDetailEl) ncDetailEl.textContent = "NC output off";
    } else if (v?.valid) {
      ncEl.textContent = "Pass";
      ncEl.className = "metric-value ok";
      ncCard?.classList.add("ok");
      if (ncDetailEl) {
        const w = v.warnings?.length ?? 0;
        ncDetailEl.textContent = w ? `${w} warning${w > 1 ? "s" : ""}` : "validated";
      }
    } else {
      ncEl.textContent = "Fail";
      ncEl.className = "metric-value warn";
      ncCard?.classList.add("fail");
      if (ncDetailEl) {
        const errs = v?.errors?.length ?? 0;
        ncDetailEl.textContent = errs ? `${errs} error${errs > 1 ? "s" : ""}` : "invalid";
      }
    }
  }
}

function loadVisualizations(data) {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      if (data.toolpath) {
        simulator.load(data, { autoPlay: true });
        try {
          viewer3d.load(data);
        } catch (e) {
          console.error(e);
        }
        viewer3d.resize?.();
      }
      if (data.stock_simulation) {
        clearStockCache();
        drawStockHeatmap($("canvas-stock"), data.stock_simulation);
      }
      renderStockOverlay(data);
    });
  });
}

async function presentResults(data) {
  await revealStagger(FLOW_SECTIONS());
  await animateMetrics(data);
  renderDiagnostics(data);
  renderFeatures(data);
  renderPathStats(data);
  renderBrepStats(data);
  loadVisualizations(data);

  activeTab = "gcode";
  document.querySelectorAll(".tab").forEach((b) => {
    b.classList.remove("active");
    b.setAttribute("aria-selected", "false");
  });
  const gcodeTab = document.querySelector('.tab[data-tab="gcode"]');
  gcodeTab?.classList.add("active");
  gcodeTab?.setAttribute("aria-selected", "true");
  await renderCode(true);
}

function formatSchemaSummary(data) {
  const s = data.stats;
  const r = data.registry_summary;
  const lines = [
    `parse skips: ${s.parse_errors} (non-fatal entity parse failures)`,
    `entity types: ${s.type_count}`,
    `length unit: ${s.length_unit ?? "mm"}`,
    `typed entity hits: ${r?.typed_entity_hits ?? "—"}`,
    `unknown entity types: ${r?.unknown_entity_types ?? 0}`,
    `ap242 styling kinds: ${r?.ap242_entity_kinds ?? 0}`,
    "",
    "top types:",
    ...(s.top_types ?? []).map((t) => `  ${t.name}: ${t.count}`),
  ];
  return lines.join("\n");
}

function highlightGcode(text) {
  return text
    .split("\n")
    .map((line) => {
      if (!line.trim()) return "";
      let h = escapeHtml(line);
      h = h.replace(/^(\([^)]*\))/, '<span class="gc-comment">$1</span>');
      h = h.replace(/;(.*)$/, '<span class="gc-comment">;$1</span>');
      h = h.replace(/\b(N\d+)\b/g, '<span class="gc-line">$1</span>');
      h = h.replace(/\b(O\d+)\b/gi, '<span class="gc-key">$1</span>');
      h = h.replace(/\b(G\d+(?:\.\d+)?)\b/gi, '<span class="gc-g">$1</span>');
      h = h.replace(/\b(M\d+)\b/gi, '<span class="gc-m">$1</span>');
      h = h.replace(
        /\b([XYZIJABCUVW])(-?\d+\.?\d*)/gi,
        '<span class="gc-xyz">$1</span><span class="gc-num">$2</span>'
      );
      h = h.replace(
        /\b([FS])(-?\d+\.?\d*)/gi,
        '<span class="gc-key">$1</span><span class="gc-num">$2</span>'
      );
      h = h.replace(/\b(T\d+)\b/gi, '<span class="gc-key">$1</span>');
      h = h.replace(/%+/g, '<span class="gc-key">%</span>');
      return h;
    })
    .join("\n");
}

function updateGutter(lineCount) {
  const gutter = $("code-gutter");
  if (!gutter) return;
  if (lineCount <= 1) {
    gutter.textContent = "";
    gutter.classList.add("hidden");
    return;
  }
  gutter.classList.remove("hidden");
  const lines = [];
  for (let i = 1; i <= lineCount; i++) lines.push(String(i));
  gutter.textContent = lines.join("\n");
}

function getCodeText() {
  if (!lastData) return "";
  if (activeTab === "gcode") {
    const g = lastData.gcode;
    return g && String(g).trim() ? g : "(no g-code — enable NC output)";
  }
  if (activeTab === "toolpath") return JSON.stringify(lastData.toolpath, null, 2);
  return formatSchemaSummary(lastData);
}

function renderCode(animate = false) {
  if (!lastData) return;
  const out = codeEl();
  const wrap = $("code-wrap");
  const gutter = $("code-gutter");
  const text = getCodeText();
  const lineCount = text.split("\n").length;

  const apply = () => {
    if (activeTab === "gcode" && text.startsWith("(") === false && !text.startsWith("(no")) {
      out.innerHTML = highlightGcode(text);
      updateGutter(lineCount);
    } else {
      out.textContent = text;
      if (gutter) {
        if (activeTab !== "gcode" && lineCount > 1) {
          gutter.classList.remove("hidden");
          updateGutter(lineCount);
        } else if (text.startsWith("(no")) {
          gutter.classList.add("hidden");
        }
      }
    }
  };

  if (!animate || !wrap) {
    apply();
    return;
  }

  wrap.classList.add("is-swapping");
  wait(120).then(() => {
    apply();
    wrap.classList.remove("is-swapping");
  });
}

const codePre = $("code-out");
const codeGutter = $("code-gutter");
if (codePre && codeGutter) {
  codePre.addEventListener("scroll", () => {
    codeGutter.scrollTop = codePre.scrollTop;
  });
}

document.querySelectorAll(".tab").forEach((btn) => {
  btn.addEventListener("click", () => {
    if (btn.classList.contains("active")) return;
    document.querySelectorAll(".tab").forEach((b) => {
      b.classList.remove("active");
      b.setAttribute("aria-selected", "false");
    });
    btn.classList.add("active");
    btn.setAttribute("aria-selected", "true");
    activeTab = btn.dataset.tab;
    renderCode(true);
  });
});

async function copyText(text) {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      /* fall through */
    }
  }
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.setAttribute("readonly", "");
  ta.style.cssText = "position:fixed;left:-9999px;top:0;opacity:0";
  document.body.appendChild(ta);
  ta.select();
  let ok = false;
  try {
    ok = document.execCommand("copy");
  } catch {
    ok = false;
  }
  document.body.removeChild(ta);
  return ok;
}

$("copy-code")?.addEventListener("click", async () => {
  const text = getCodeText();
  const btn = $("copy-code");
  const ok = await copyText(text);
  if (!ok) return;
  btn.textContent = "Copied";
  btn.classList.add("copied");
  setTimeout(() => {
    btn.textContent = "Copy";
    btn.classList.remove("copied");
  }, 1500);
});

$("download-code")?.addEventListener("click", () => {
  const text = getCodeText();
  const ext = activeTab === "gcode" ? "nc" : activeTab === "toolpath" ? "json" : "txt";
  const blob = new Blob([text], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `steprs.${ext}`;
  a.click();
  URL.revokeObjectURL(url);
});

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
    await wait(60);

    lastData = JSON.parse(json);
    const ms = Math.round(performance.now() - t0);

    await progress.finish();
    if (gen !== analyzeGen) return;

    await presentResults(lastData);
    document.body.classList.add("has-results");
    $("landing")?.classList.add("hidden");
    $("dropzone")?.classList.remove("hidden");
    $("tools")?.classList.remove("hidden");
    $("status-bar")?.classList.remove("hidden");
    const label = $("drop-label");
    if (label) label.textContent = name;

    const tp = lastData.toolpath?.stats;
    const unit = lastData.stats?.length_unit ?? "mm";
    $("file-meta").textContent = `${name} · ${formatBytes(size)} · ${ms} ms · ${tp?.segment_count ?? 0} segs · ${unit}`;
    setMessage("");
  } catch (e) {
    if (gen !== analyzeGen) return;
    setMessage(String(e), true);
  } finally {
    if (gen === analyzeGen) {
      document.body.classList.remove("analyzing");
      $("dropzone")?.classList.remove("is-busy");
      await wait(500);
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

$("sim-play")?.addEventListener("click", () => {
  const btn = $("sim-play");
  if (btn.classList.toggle("active")) simulator.play();
  else simulator.pause();
});
$("sim-reset")?.addEventListener("click", () => {
  $("sim-play")?.classList.remove("active");
  simulator.reset();
});
$("sim-speed")?.addEventListener("input", (e) =>
  simulator.setSpeed(parseFloat(e.target.value))
);

function onLayoutChange() {
  simulator.resize();
  viewer3d.resize();
  if (lastData?.stock_simulation) {
    clearStockCache();
    drawStockHeatmap($("canvas-stock"), lastData.stock_simulation);
  }
  if (lastData?.toolpath) viewer3d.refresh?.();
}

window.addEventListener("resize", onLayoutChange);

if (typeof ResizeObserver !== "undefined") {
  const layoutRo = new ResizeObserver(onLayoutChange);
  const viz = $("viz");
  const stage = $("stage");
  if (viz) layoutRo.observe(viz);
  if (stage) layoutRo.observe(stage);
}

onThemeChange(() => onLayoutChange());
initTheme();
initPageEntrance();

function openFilePicker() {
  const input = $("file-input");
  if (!input) return;
  input.value = "";
  input.click();
}

async function processFile(file) {
  $("landing")?.classList.add("is-received");
  $("dropzone")?.classList.add("is-received");
  await wait(200);
  $("landing")?.classList.remove("is-received");
  $("dropzone")?.classList.remove("is-received");
  const text = await file.text();
  await analyzeText(text, file.name, file.size);
}

function bindDropTarget(el) {
  if (!el) return;
  el.addEventListener("click", (e) => {
    if (e.target.closest(".sample-chip, .landing-samples, button, a, input, select, label")) return;
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
    if (e.currentTarget === e.target || !el.contains(e.relatedTarget)) {
      el.classList.remove("dragover");
    }
  });
  el.addEventListener("drop", (e) => {
    e.preventDefault();
    el.classList.remove("dragover");
    const f = e.dataTransfer?.files?.[0];
    if (f) processFile(f);
  });
}

bindDropTarget($("landing"));
$("dropzone")?.addEventListener("click", () => openFilePicker());

$("file-input")?.addEventListener("change", () => {
  const f = $("file-input").files[0];
  if (f) processFile(f);
});

document.querySelectorAll(".sample-chip").forEach((btn) => {
  btn.addEventListener("click", async (e) => {
    e.stopPropagation();
    const name = btn.dataset.sample;
    const url = SAMPLES[name];
    if (!url) return;
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`Sample not found (${res.status}) — run make serve from project root`);
      const text = await res.text();
      await analyzeText(text, name, text.length);
    } catch (err) {
      setMessage(String(err), true);
    }
  });
});

initWasm();
