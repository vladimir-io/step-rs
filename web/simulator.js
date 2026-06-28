/**
 * 2D toolpath simulator — theme-aware, eased playback.
 */

import { easeOutCubic, prefersReducedMotion } from "./motion.js";
import { canvasTheme, onThemeChange } from "./theme.js";

export function createSimulator(canvasXY, canvasXZ, controls) {
  const state = {
    toolpath: null,
    preview: null,
    progress: 0,
    displayProgress: 0,
    playing: false,
    speed: 1,
    raf: null,
    animRaf: null,
    lastTime: 0,
    points: [],
    needsDraw: true,
    autoPlayTimer: null,
  };

  const panels = [
    { canvas: canvasXY, label: "XY", project: projectXY },
    { canvas: canvasXZ, label: "XZ", project: projectXZ },
  ];

  onThemeChange(() => scheduleDraw(state.displayProgress));

  function load(analysis, opts = {}) {
    if (state.autoPlayTimer) clearTimeout(state.autoPlayTimer);
    state.toolpath = analysis.toolpath;
    state.preview = analysis.preview;
    state.progress = 0;
    state.displayProgress = 0;
    state.points = flattenToolpath(state.toolpath);
    scheduleDraw(0);
    updateReadout(0);

    if (opts.autoPlay && state.points.length && !prefersReducedMotion()) {
      state.autoPlayTimer = setTimeout(() => {
        controls.playBtn?.classList.add("active");
        play();
      }, 480);
    }
  }

  function flattenToolpath(tp) {
    if (!tp?.segments?.length) return [];
    const pts = [];
    let cur = null;

    for (const seg of tp.segments) {
      const chunk = expandSegment(seg, cur);
      for (const p of chunk) {
        pts.push(p);
        cur = p.pos;
      }
    }
    return pts;
  }

  function expandSegment(seg, cur) {
    if (seg.kind === "drill") {
      const { xy, z_safe, z_bottom } = seg;
      if (!Array.isArray(xy) || xy.length < 2) return [];
      if (!Number.isFinite(z_safe) || !Number.isFinite(z_bottom)) return [];
      const out = [];
      if (cur) out.push({ pos: [xy[0], xy[1], z_safe], rapid: true });
      for (let i = 1; i <= 12; i++) {
        const t = i / 12;
        out.push({
          pos: [xy[0], xy[1], z_safe + (z_bottom - z_safe) * t],
          rapid: false,
        });
      }
      out.push({ pos: [xy[0], xy[1], z_safe], rapid: true });
      return out;
    }

    if (seg.kind === "arc" && cur) {
      const to = seg.to;
      if (!Array.isArray(to) || to.length < 3) return [];
      const [i, j] = seg.center_offset ?? [0, 0];
      return arcPoints(cur, to, i, j, !!seg.clockwise, 20, false);
    }

    const to = seg.to ?? seg.end_point;
    if (!to) return [];
    const end = Array.isArray(to) ? to : [to?.x ?? 0, to?.y ?? 0, to?.z ?? 0];
    if (!end.every((n) => Number.isFinite(n))) return [];
    const rapid = seg.kind === "rapid";
    const steps = rapid ? 1 : 1;
    const out = [];
    const from = cur ?? end;
    for (let s = 1; s <= steps; s++) {
      const t = s / steps;
      out.push({ pos: lerp3(from, end, t), rapid });
    }
    return out;
  }

  function arcPoints(from, to, i, j, cw, steps, rapid) {
    const cx = from[0] + i;
    const cy = from[1] + j;
    const a0 = Math.atan2(from[1] - cy, from[0] - cx);
    const a1 = Math.atan2(to[1] - cy, to[0] - cx);
    let delta = a1 - a0;
    if (cw) {
      if (delta >= 0) delta -= Math.PI * 2;
    } else if (delta <= 0) {
      delta += Math.PI * 2;
    }
    const r = Math.hypot(i, j) || 1;
    const out = [];
    for (let s = 1; s <= steps; s++) {
      const t = s / steps;
      const a = a0 + delta * t;
      out.push({
        pos: [cx + Math.cos(a) * r, cy + Math.sin(a) * r, from[2] + (to[2] - from[2]) * t],
        rapid,
      });
    }
    return out;
  }

  function lerp3(a, b, t) {
    return [
      a[0] + (b[0] - a[0]) * t,
      a[1] + (b[1] - a[1]) * t,
      a[2] + (b[2] - a[2]) * t,
    ];
  }

  function bounds() {
    const b = state.toolpath?.bounds;
    if (b?.min && b?.max) return { min: b.min, max: b.max };
    const p = state.preview?.bounds;
    if (p?.min?.[0] !== undefined) return { min: p.min, max: p.max };
    return { min: [-20, -20, -10], max: [20, 20, 10] };
  }

  function projectXY(p, w, h, pad, b) {
    const spanX = b.max[0] - b.min[0] || 1;
    const spanY = b.max[1] - b.min[1] || 1;
    const s = Math.min((w - 2 * pad) / spanX, (h - 2 * pad) / spanY);
    const ox = pad + (w - 2 * pad - spanX * s) / 2;
    const oy = h - pad - spanY * s;
    return [ox + (p[0] - b.min[0]) * s, oy - (p[1] - b.min[1]) * s];
  }

  function projectXZ(p, w, h, pad, b) {
    const spanX = b.max[0] - b.min[0] || 1;
    const spanZ = b.max[2] - b.min[2] || 1;
    const s = Math.min((w - 2 * pad) / spanX, (h - 2 * pad) / spanZ);
    const ox = pad + (w - 2 * pad - spanX * s) / 2;
    const oz = h - pad;
    return [ox + (p[0] - b.min[0]) * s, oz - (p[2] - b.min[2]) * s];
  }

  function setupCanvas(canvas, ctx) {
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w === 0 || h === 0) return null;
    const pw = Math.round(w * dpr);
    const ph = Math.round(h * dpr);
    if (canvas.width !== pw || canvas.height !== ph) {
      canvas.width = pw;
      canvas.height = ph;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    return { w, h };
  }

  function drawGrid(ctx, w, h, pad, c) {
    const step = 24;
    ctx.strokeStyle = c.grid;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let x = pad; x <= w - pad; x += step) {
      ctx.moveTo(x, pad);
      ctx.lineTo(x, h - pad);
    }
    for (let y = pad; y <= h - pad; y += step) {
      ctx.moveTo(pad, y);
      ctx.lineTo(w - pad, y);
    }
    ctx.stroke();
    ctx.strokeStyle = c.gridStrong;
    ctx.strokeRect(pad, pad, w - 2 * pad, h - 2 * pad);
  }

  function drawPanel({ canvas, label, project }, progress) {
    const ctx = canvas.getContext("2d");
    const dim = setupCanvas(canvas, ctx);
    if (!dim) return;
    const { w, h } = dim;
    const pad = 30;
    const b = bounds();
    const c = canvasTheme();

    ctx.fillStyle = c.bg;
    ctx.fillRect(0, 0, w, h);
    drawGrid(ctx, w, h, pad, c);
    drawPreview(ctx, project, w, h, pad, c);

    const pts = state.points;
    if (!pts.length) {
      drawLabel(ctx, label, w, c);
      return;
    }

    const eased = easeOutCubic(Math.min(1, progress));
    const idx = Math.min(pts.length - 1, Math.floor(eased * (pts.length - 1)));
    const frac = eased * (pts.length - 1) - idx;
    const nextIdx = Math.min(pts.length - 1, idx + 1);

    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    for (let i = 1; i <= idx; i++) {
      if (!pts[i - 1]?.pos || !pts[i]?.pos) continue;
      const a = project(pts[i - 1].pos, w, h, pad, b);
      const p = project(pts[i].pos, w, h, pad, b);
      ctx.strokeStyle = pts[i].rapid ? c.rapid : c.cut;
      ctx.lineWidth = pts[i].rapid ? 0.6 : 1;
      ctx.beginPath();
      ctx.moveTo(a[0], a[1]);
      ctx.lineTo(p[0], p[1]);
      ctx.stroke();
    }

    if (frac > 0.001 && idx < pts.length - 1 && pts[idx]?.pos && pts[nextIdx]?.pos) {
      const a = project(pts[idx].pos, w, h, pad, b);
      const p = project(pts[nextIdx].pos, w, h, pad, b);
      const seg = pts[nextIdx];
      ctx.strokeStyle = seg.rapid ? c.rapid : c.cut;
      ctx.lineWidth = seg.rapid ? 0.6 : 1;
      ctx.beginPath();
      ctx.moveTo(a[0], a[1]);
      ctx.lineTo(a[0] + (p[0] - a[0]) * frac, a[1] + (p[1] - a[1]) * frac);
      ctx.stroke();
    }

    if (!pts[idx]?.pos || !pts[nextIdx]?.pos) {
      drawLabel(ctx, label, w, c);
      return;
    }
    const tipPos = lerp3(pts[idx].pos, pts[nextIdx].pos, frac);
    const tip = project(tipPos, w, h, pad, b);
    ctx.fillStyle = c.tip;
    ctx.beginPath();
    ctx.arc(tip[0], tip[1], 2, 0, Math.PI * 2);
    ctx.fill();

    drawLabel(ctx, label, w, c);
  }

  function drawLabel(ctx, label, w, c) {
    ctx.fillStyle = c.label;
    ctx.font = "500 9px IBM Plex Mono, monospace";
    ctx.fillText(label.toUpperCase(), 10, 16);
  }

  function drawPreview(ctx, project, w, h, pad, c) {
    const prev = state.preview;
    if (!prev) return;
    const b = bounds();

    for (const cyl of prev.cylinders || []) {
      const o = project(cyl.origin, w, h, pad, b);
      const r =
        Math.abs(project([cyl.origin[0] + cyl.radius, cyl.origin[1], cyl.origin[2]], w, h, pad, b)[0] - o[0]) || 5;
      ctx.strokeStyle = c.preview;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(o[0], o[1], r, 0, Math.PI * 2);
      ctx.stroke();
    }

    if (prev.stock) {
      const a = project(prev.stock.min, w, h, pad, b);
      const z = project(prev.stock.max, w, h, pad, b);
      ctx.strokeStyle = c.preview;
      ctx.strokeRect(a[0], z[1], z[0] - a[0], a[1] - z[1]);
    }
  }

  function setProgress(p) {
    state.progress = p;
    state.needsDraw = true;
    if (!state.animRaf) {
      state.animRaf = requestAnimationFrame(smoothDrawLoop);
    }
  }

  function smoothDrawLoop() {
    const delta = state.progress - state.displayProgress;
    if (Math.abs(delta) < 0.0008) {
      state.displayProgress = state.progress;
    } else {
      state.displayProgress += delta * 0.18;
    }

    if (state.needsDraw) {
      state.needsDraw = false;
      for (const panel of panels) drawPanel(panel, state.displayProgress);
    }

    if (Math.abs(state.progress - state.displayProgress) > 0.0008 || state.playing) {
      state.animRaf = requestAnimationFrame(smoothDrawLoop);
    } else {
      state.animRaf = null;
    }
  }

  function scheduleDraw(progress) {
    state.progress = progress;
    state.displayProgress = progress;
    state.needsDraw = true;
    if (state.raf) return;
    state.raf = requestAnimationFrame(() => {
      state.raf = null;
      for (const panel of panels) drawPanel(panel, state.displayProgress);
    });
  }

  function updateReadout(progress) {
    const tp = state.toolpath;
    if (!tp || !controls.readout) return;
    const pct = (progress * 100).toFixed(0);
    const cut = tp.stats?.estimated_cut_length_mm?.toFixed(1) ?? "—";
    const t = tp.stats?.estimated_time_min?.toFixed(2) ?? "—";
    controls.readout.textContent = `${pct}% · ${cut} mm · ${t} min`;
  }

  function tick(now) {
    if (!state.playing || !state.points.length) return;
    const dt = (now - state.lastTime) / 1000;
    state.lastTime = now;
    const next = Math.min(1, state.progress + dt * 0.085 * state.speed);
    setProgress(next);
    updateReadout(next);
    if (next >= 1) {
      state.playing = false;
      controls.playBtn?.classList.remove("active");
      return;
    }
    requestAnimationFrame(tick);
  }

  function play() {
    if (!state.points.length) return;
    state.playing = true;
    state.lastTime = performance.now();
    if (state.progress >= 1) setProgress(0);
    requestAnimationFrame(tick);
  }

  const ro =
    typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(() => scheduleDraw(state.progress))
      : null;
  for (const p of panels) ro?.observe(p.canvas);

  return {
    load,
    play,
    pause() {
      state.playing = false;
    },
    reset() {
      if (state.autoPlayTimer) clearTimeout(state.autoPlayTimer);
      state.playing = false;
      setProgress(0);
      updateReadout(0);
    },
    setSpeed(v) {
      state.speed = v;
    },
    resize() {
      scheduleDraw(state.progress);
    },
  };
}
