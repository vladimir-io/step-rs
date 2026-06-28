/**
 * Stock-removal heatmap — muted, analytical.
 */

import { getTheme } from "./theme.js";

let stockCache = null;

export function colormap(t, light) {
  const u = Math.max(0, Math.min(1, t));
  if (light) {
    const g = Math.round(244 - u * 100);
    const b = Math.round(248 - u * 40);
    const r = Math.round(228 + u * 80);
    return [r, g, b];
  }
  const base = Math.round(22 + u * 18);
  const warm = Math.round(u * 140);
  return [base + warm, base + Math.round(u * 60), base + Math.round(u * 20)];
}

function sample(data, nx, ny, fx, fy) {
  const ix = Math.min(nx - 1, Math.max(0, fx * (nx - 1)));
  const iy = Math.min(ny - 1, Math.max(0, fy * (ny - 1)));
  const x0 = Math.floor(ix);
  const y0 = Math.floor(iy);
  const x1 = Math.min(nx - 1, x0 + 1);
  const y1 = Math.min(ny - 1, y0 + 1);
  const tx = ix - x0;
  const ty = iy - y0;
  const h = (x, y) => data[y * nx + x];
  return (
    h(x0, y0) * (1 - tx) * (1 - ty) +
    h(x1, y0) * tx * (1 - ty) +
    h(x0, y1) * (1 - tx) * ty +
    h(x1, y1) * tx * ty
  );
}

export function drawStockHeatmap(canvas, simulation) {
  if (!simulation?.remaining_height?.length) return;

  const light = getTheme() === "light";
  const nx = simulation.grid_nx;
  const ny = simulation.grid_ny;
  const data = simulation.remaining_height;

  let minH = Infinity;
  let maxH = 0;
  for (let i = 0; i < data.length; i++) {
    if (data[i] < minH) minH = data[i];
    if (data[i] > maxH) maxH = data[i];
  }
  if (!Number.isFinite(minH)) minH = 0;
  if (maxH <= minH) maxH = minH + 1;

  const span = maxH - minH;
  const key = `${getTheme()}:${nx}x${ny}:${minH}:${maxH}:${simulation.removed_volume_mm3}`;

  const ctx = canvas.getContext("2d");
  const dpr = Math.min(window.devicePixelRatio || 1, 3);
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  if (w === 0 || h === 0) return;

  const ss = 4;
  const pw = Math.max(Math.round(w * dpr * ss), nx * ss * 2);
  const ph = Math.max(Math.round(h * dpr * ss), ny * ss * 2);
  if (canvas.width !== pw || canvas.height !== ph) {
    canvas.width = pw;
    canvas.height = ph;
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  if (!stockCache || stockCache.key !== key || stockCache.pw !== pw || stockCache.ph !== ph) {
    const off = document.createElement("canvas");
    off.width = pw;
    off.height = ph;
    const offCtx = off.getContext("2d");
    const imageData = offCtx.createImageData(pw, ph);
    const px = imageData.data;

    for (let py = 0; py < ph; py++) {
      const fy = 1 - py / (ph - 1 || 1);
      for (let px_i = 0; px_i < pw; px_i++) {
        const fx = px_i / (pw - 1 || 1);
        const remaining = sample(data, nx, ny, fx, fy);
        const removed = 1 - (remaining - minH) / span;
        const t = Math.max(0, Math.min(1, removed));
        const [r, g, b] = colormap(t, light);
        const i = (py * pw + px_i) * 4;
        px[i] = r;
        px[i + 1] = g;
        px[i + 2] = b;
        px[i + 3] = 255;
      }
    }
    offCtx.putImageData(imageData, 0, 0);
    stockCache = { key, off, pw, ph };
  }

  const bg = getComputedStyle(document.documentElement).getPropertyValue("--canvas-bg").trim();
  ctx.fillStyle = bg || (light ? "#f4f4f5" : "#111113");
  ctx.fillRect(0, 0, w, h);
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.drawImage(stockCache.off, 0, 0, stockCache.pw, stockCache.ph, 0, 0, w, h);

  drawOverlay(ctx, w, simulation, light);
}

function drawOverlay(ctx, w, simulation, light) {
  const vol = simulation.removed_volume_mm3;
  const volStr =
    vol >= 1000 ? `${(vol / 1000).toFixed(1)}k mm³` : `${vol?.toFixed(0) ?? "—"} mm³`;

  ctx.font = "500 9px IBM Plex Mono, monospace";
  ctx.fillStyle = light ? "rgba(24,24,27,0.5)" : "rgba(163,163,163,0.65)";
  ctx.fillText(volStr, 10, 16);

  const parts = [];
  if (simulation.collision_cells > 0) parts.push(`air ${simulation.collision_cells}`);
  if (simulation.gouge_cells > 0) parts.push(`gouge ${simulation.gouge_cells}`);
  if ((simulation.overcut_cells ?? 0) > 0) parts.push(`overcut ${simulation.overcut_cells}`);
  if (parts.length) {
    ctx.fillStyle = simulation.gouge_cells > 0 ? "#ff453a" : "#ff9f0a";
    ctx.fillText(parts.join(" · "), 10, 32);
  }
}

export function clearStockCache() {
  stockCache = null;
}

/** Raw heatmap pixels for 3D stock texture. */
export function buildHeatmapTextureData(simulation, light = false) {
  if (!simulation?.remaining_height?.length) return null;

  const nx = simulation.grid_nx;
  const ny = simulation.grid_ny;
  const data = simulation.remaining_height;

  let minH = Infinity;
  let maxH = 0;
  for (let i = 0; i < data.length; i++) {
    if (data[i] < minH) minH = data[i];
    if (data[i] > maxH) maxH = data[i];
  }
  if (!Number.isFinite(minH)) minH = 0;
  if (maxH <= minH) maxH = minH + 1;
  const span = maxH - minH;

  const scale = 4;
  const pw = nx * scale;
  const ph = ny * scale;
  const px = new Uint8ClampedArray(pw * ph * 4);

  for (let py = 0; py < ph; py++) {
    const fy = 1 - py / (ph - 1 || 1);
    for (let px_i = 0; px_i < pw; px_i++) {
      const fx = px_i / (pw - 1 || 1);
      const remaining = sample(data, nx, ny, fx, fy);
      const removed = 1 - (remaining - minH) / span;
      const t = Math.max(0, Math.min(1, removed));
      const [r, g, b] = colormap(t, light);
      const i = (py * pw + px_i) * 4;
      px[i] = r;
      px[i + 1] = g;
      px[i + 2] = b;
      px[i + 3] = 255;
    }
  }
  return { data: px, width: pw, height: ph };
}
