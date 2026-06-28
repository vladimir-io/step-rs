/**
 * Part preview — stock envelope + selected bore x-ray.
 */
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { mergeVertices } from "three/addons/utils/BufferGeometryUtils.js";
import { canvasTheme, onThemeChange } from "./theme.js";

const MAX_COORD = 100_000;

function stepToThree(x, y, z) {
  const nx = Number(x);
  const ny = Number(y);
  const nz = Number(z);
  return new THREE.Vector3(
    Number.isFinite(nx) ? nx : 0,
    Number.isFinite(nz) ? nz : 0,
    Number.isFinite(ny) ? ny : 0
  );
}

function isVec3(v) {
  return Array.isArray(v) && v.length >= 3 && Number.isFinite(Number(v[0]));
}

function flattenVerts(vertices) {
  if (!vertices?.length) return null;
  if (typeof vertices[0] === "number") return Float32Array.from(vertices);
  const out = new Float32Array(vertices.length * 3);
  let n = 0;
  for (const v of vertices) {
    if (!isVec3(v)) continue;
    out[n++] = Number(v[0]);
    out[n++] = Number(v[1]);
    out[n++] = Number(v[2]);
  }
  return n >= 9 ? out.subarray(0, n) : null;
}

function toColor(v) {
  if (typeof v === "number") return v;
  const h = String(v ?? "").replace("#", "").trim();
  return parseInt(h, 16) || 0x888888;
}

function isStockEnvelopeMesh(mesh) {
  return mesh?.vertices?.length === 8 && (mesh?.indices?.length ?? 0) >= 36;
}

function hasNativeMesh(mesh) {
  return mesh?.indices?.length >= 3 && mesh?.vertices?.length >= 3 && !isStockEnvelopeMesh(mesh);
}

function saneStock(stock) {
  if (!stock?.min || !stock?.max) return null;
  for (const c of [...stock.min, ...stock.max]) {
    if (!Number.isFinite(c) || Math.abs(c) > MAX_COORD) return null;
  }
  const size = stock.max.map((v, i) => v - stock.min[i]);
  if (size.some((s) => s <= 0 || s > MAX_COORD)) return null;
  return stock;
}

function stockFromHoles(holes) {
  if (!holes?.length) return null;
  let min = [Infinity, Infinity, Infinity];
  let max = [-Infinity, -Infinity, -Infinity];
  const grow = (p, r) => {
    for (let i = 0; i < 3; i++) {
      min[i] = Math.min(min[i], p[i] - r);
      max[i] = Math.max(max[i], p[i] + r);
    }
  };
  for (const h of holes) {
    const o = h.axis_origin;
    const d = h.axis_direction;
    if (!o || !d) continue;
    const ox = o.x ?? o[0];
    const oy = o.y ?? o[1];
    const oz = o.z ?? o[2];
    const dx = d.x ?? d[0];
    const dy = d.y ?? d[1];
    const dz = d.z ?? d[2];
    const r = Math.max(h.radius ?? 1, 0.5);
    const depth = Math.max(h.depth ?? r * 2, r);
    const p0 = [ox, oy, oz];
    const p1 = [ox + dx * depth, oy + dy * depth, oz + dz * depth];
    if (![...p0, ...p1].every((c) => Number.isFinite(c) && Math.abs(c) <= MAX_COORD)) continue;
    grow(p0, r);
    grow(p1, r);
  }
  if (!min[0].isFinite()) return null;
  const pad = Math.max(...max.map((v, i) => v - min[i])) * 0.1;
  const p = Math.max(pad, 3);
  return {
    min: min.map((v) => v - p),
    max: max.map((v) => v + p),
  };
}

export function createViewer3d(canvas) {
  if (!canvas) return { load() {}, resize() {}, refresh() {}, highlightHole() {} };

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: false,
    powerPreference: "high-performance",
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.NoToneMapping;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(38, 1, 0.1, 500000);
  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  controls.rotateSpeed = 0.55;

  const partGroup = new THREE.Group();
  const overlayGroup = new THREE.Group();
  scene.add(partGroup, overlayGroup);

  let grid = null;
  let lights = [];
  let lastAnalysis = null;
  let selectedHole = 0;
  let raf = 0;
  let running = false;
  let partScale = 1;

  function applyTheme() {
    const t = canvasTheme();
    scene.background = new THREE.Color(toColor(t.threeBgHex));
    scene.fog = null;
    if (grid) {
      grid.material.color.setHex(toColor(t.threeGrid));
      grid.material.opacity = document.documentElement.dataset.theme === "light" ? 0.4 : 0.28;
    }
  }

  function partMaterial(t) {
    const dark = document.documentElement.dataset.theme !== "light";
    return new THREE.MeshPhongMaterial({
      color: dark ? 0x7a808c : 0x9aa3b2,
      specular: dark ? 0x222228 : 0x444450,
      shininess: 28,
      flatShading: false,
    });
  }

  function edgeMaterial() {
    const dark = document.documentElement.dataset.theme !== "light";
    return new THREE.LineBasicMaterial({
      color: dark ? 0x1a1a22 : 0x3a4250,
      transparent: true,
      opacity: dark ? 0.55 : 0.45,
    });
  }

  function focusBoreMaterial(accent) {
    return new THREE.MeshBasicMaterial({
      color: accent,
      transparent: true,
      opacity: 0.5,
      depthWrite: false,
      side: THREE.DoubleSide,
    });
  }

  function markerRingMaterial(accent, opacity) {
    return new THREE.MeshBasicMaterial({
      color: accent,
      transparent: true,
      opacity,
      depthWrite: false,
    });
  }

  function holeSegments(hole) {
    const diameters = hole.segment_diameters_mm;
    const depths = hole.segment_depths_mm;
    if (Array.isArray(diameters) && diameters.length && Array.isArray(depths) && depths.length) {
      return diameters.map((d, i) => ({
        radius: Math.max(0.01, d / 2),
        depth: Math.max(0.01, depths[i] ?? depths[depths.length - 1]),
      }));
    }
    const r = hole.radius;
    const depth = hole.depth;
    if (r == null || depth == null) return [];
    return [{ radius: Math.max(0.01, r), depth: Math.max(0.01, depth) }];
  }

  function orientAlongAxis(mesh, ax, origin, offset, length) {
    mesh.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), ax);
    mesh.position.copy(origin).addScaledVector(ax, offset + length / 2);
  }

  function addRing(radius, ax, origin, offset, accent, opacity) {
    const ring = new THREE.Mesh(
      new THREE.TorusGeometry(radius, Math.max(radius * 0.035, 0.12), 10, 40),
      markerRingMaterial(accent, opacity)
    );
    ring.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), ax);
    ring.position.copy(origin).addScaledVector(ax, offset);
    ring.renderOrder = 2;
    overlayGroup.add(ring);
  }

  function addCoaxialOverlay(hole, t, mode) {
    const o = hole.axis_origin;
    const d = hole.axis_direction;
    if (!o || !d) return;

    const ox = o.x ?? o[0];
    const oy = o.y ?? o[1];
    const oz = o.z ?? o[2];
    const dx = d.x ?? d[0];
    const dy = d.y ?? d[1];
    const dz = d.z ?? d[2];
    const ax = stepToThree(dx, dy, dz).normalize();
    const origin = stepToThree(ox, oy, oz);
    const accent = toColor(t.threeCut || "#4a9eff");
    const segments = holeSegments(hole);
    if (!segments.length) return;

    let cursor = 0;
    for (const seg of segments) {
      if (mode === "focus") {
        const coreGeo = new THREE.CylinderGeometry(seg.radius, seg.radius, seg.depth, 40, 1, true);
        const core = new THREE.Mesh(coreGeo, focusBoreMaterial(accent));
        orientAlongAxis(core, ax, origin, cursor, seg.depth);
        core.renderOrder = 2;
        overlayGroup.add(core);
        addRing(seg.radius, ax, origin, cursor, accent, 0.9);
        addRing(seg.radius, ax, origin, cursor + seg.depth, accent, 0.9);
      } else {
        addRing(seg.radius * 0.95, ax, origin, cursor, accent, 0.28);
      }
      cursor += seg.depth;
    }
  }

  function clearGroup(g) {
    while (g.children.length) {
      const c = g.children.pop();
      c.traverse?.((o) => {
        o.geometry?.dispose();
        if (o.material) {
          Array.isArray(o.material) ? o.material.forEach((m) => m.dispose()) : o.material.dispose();
        }
      });
    }
  }

  function buildSolidGeometry(mesh) {
    const flat = flattenVerts(mesh.vertices);
    if (!flat || !mesh.indices?.length) return null;

    const transformed = new Float32Array(flat.length);
    for (let i = 0; i < flat.length; i += 3) {
      const v = stepToThree(flat[i], flat[i + 1], flat[i + 2]);
      transformed[i] = v.x;
      transformed[i + 1] = v.y;
      transformed[i + 2] = v.z;
    }

    let geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(transformed, 3));
    geo.setIndex(mesh.indices);
    geo = mergeVertices(geo);
    geo.computeVertexNormals();
    return geo;
  }

  function addStockBox(stock, t, partBox) {
    const min = stepToThree(stock.min[0], stock.min[1], stock.min[2]);
    const max = stepToThree(stock.max[0], stock.max[1], stock.max[2]);
    const size = new THREE.Vector3().subVectors(max, min);
    if (size.x <= 0 || size.y <= 0 || size.z <= 0) return;
    const center = new THREE.Vector3().addVectors(min, max).multiplyScalar(0.5);
    const geo = new THREE.BoxGeometry(size.x, size.y, size.z);
    const box = new THREE.Mesh(geo, partMaterial(t));
    box.position.copy(center);
    partGroup.add(box);

    const edges = new THREE.LineSegments(new THREE.EdgesGeometry(geo), edgeMaterial());
    edges.position.copy(center);
    partGroup.add(edges);

    partBox.expandByPoint(min);
    partBox.expandByPoint(max);
  }

  function resolveStock(preview, holes) {
    return saneStock(preview?.stock) ?? stockFromHoles(holes);
  }

  function fitCamera(box) {
    if (box.isEmpty()) return;
    const size = new THREE.Vector3();
    const center = new THREE.Vector3();
    box.getSize(size);
    box.getCenter(center);
    partScale = Math.max(size.x, size.y, size.z, 1);

    const fov = (camera.fov * Math.PI) / 180;
    const dist = (partScale / (2 * Math.tan(fov / 2))) * 1.5;
    camera.position.set(center.x + dist * 0.75, center.y + dist * 0.55, center.z + dist * 0.8);
    camera.near = Math.max(partScale * 0.005, 0.1);
    camera.far = Math.max(partScale * 20, 2000);
    camera.updateProjectionMatrix();

    controls.target.copy(center);
    controls.update();
    controls.minDistance = partScale * 0.3;
    controls.maxDistance = partScale * 8;

    repositionLights(center, partScale);
  }

  function repositionLights(center, scale) {
    const s = scale * 2.2;
    if (lights[0]) {
      lights[0].position.set(center.x + s * 0.7, center.y + s, center.z + s * 0.6);
    }
    if (lights[1]) lights[1].position.set(center.x - s * 0.6, center.y + s * 0.35, center.z - s * 0.5);
    if (lights[2]) lights[2].position.set(center.x, center.y - s * 0.2, center.z + s * 0.8);
  }

  function addLights(t) {
    const ambient = new THREE.AmbientLight(0xffffff, document.documentElement.dataset.theme === "light" ? 0.72 : 0.45);
    const key = new THREE.DirectionalLight(0xffffff, document.documentElement.dataset.theme === "light" ? 0.85 : 0.95);
    const fill = new THREE.DirectionalLight(0xc8d0e0, 0.35);
    const rim = new THREE.DirectionalLight(toColor(t.threeCut || "#5b9aff"), 0.15);
    scene.add(ambient);
    for (const l of [key, fill, rim]) scene.add(l);
    return [key, fill, rim];
  }

  function rebuildGrid(box) {
    if (grid) {
      scene.remove(grid);
      grid.geometry.dispose();
      grid.material.dispose();
      grid = null;
    }
    if (box.isEmpty()) return;
    const t = canvasTheme();
    const size = new THREE.Vector3();
    box.getSize(size);
    const center = new THREE.Vector3();
    box.getCenter(center);
    const span = Math.max(size.x, size.z, partScale) * 2.4;
    grid = new THREE.GridHelper(span, 20, toColor(t.threeGrid), toColor(t.threeGrid));
    grid.material.transparent = true;
    grid.material.opacity = document.documentElement.dataset.theme === "light" ? 0.4 : 0.28;
    grid.position.set(center.x, box.min.y - partScale * 0.005, center.z);
    scene.add(grid);
  }

  function loadScene(analysis) {
    lastAnalysis = analysis;
    clearGroup(partGroup);
    clearGroup(overlayGroup);

    const t = canvasTheme();
    const partBox = new THREE.Box3();
    const preview = analysis.preview;
    const mesh = analysis.mesh ?? preview?.mesh;
    const holes = analysis.coaxial_holes || [];

    if (selectedHole >= holes.length) selectedHole = 0;

    const stock = resolveStock(preview, holes);

    if (hasNativeMesh(mesh)) {
      const geo = buildSolidGeometry(mesh);
      if (geo) {
        const solid = new THREE.Mesh(geo, partMaterial(t));
        partGroup.add(solid);
        partBox.expandByObject(solid);
      }
    } else if (stock) {
      addStockBox(stock, t, partBox);
    }

    holes.forEach((h, i) => {
      addCoaxialOverlay(h, t, i === selectedHole ? "focus" : "marker");
      const r = h.radius ?? 0;
      const depth = h.depth ?? 0;
      if (r > 0 && depth > 0 && h.axis_origin && h.axis_direction) {
        const o = h.axis_origin;
        const d = h.axis_direction;
        const ax = stepToThree(d.x ?? d[0], d.y ?? d[1], d.z ?? d[2]).normalize();
        const origin = stepToThree(o.x ?? o[0], o.y ?? o[1], o.z ?? o[2]);
        partBox.expandByPoint(origin);
        partBox.expandByPoint(origin.clone().addScaledVector(ax, depth));
        const perp = new THREE.Vector3(1, 0, 0).cross(ax);
        if (perp.lengthSq() < 0.01) perp.set(0, 1, 0).cross(ax);
        perp.normalize();
        partBox.expandByPoint(origin.clone().addScaledVector(perp, r));
      }
    });

    if (partBox.isEmpty() && stock) {
      addStockBox(stock, t, partBox);
    }

    rebuildGrid(partBox);
    fitCamera(partBox.isEmpty() ? new THREE.Box3().setFromObject(partGroup) : partBox);
  }

  function load(analysis) {
    try {
      selectedHole = 0;
      loadScene(analysis);
      startLoop();
    } catch {
      /* viewer optional */
    }
  }

  function highlightHole(index) {
    if (!lastAnalysis?.coaxial_holes?.length) return;
    const n = lastAnalysis.coaxial_holes.length;
    selectedHole = ((index % n) + n) % n;
    loadScene(lastAnalysis);
  }

  function frame() {
    raf = requestAnimationFrame(frame);
    controls.update();
    renderer.render(scene, camera);
  }

  function startLoop() {
    if (running) return;
    running = true;
    frame();
  }

  function stopLoop() {
    running = false;
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
  }

  function resize() {
    const parent = canvas.parentElement ?? canvas;
    const w = parent.clientWidth;
    const h = parent.clientHeight;
    if (w < 2 || h < 2) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }

  function refresh() {
    lights.forEach((l) => scene.remove(l));
    lights = addLights(canvasTheme());
    applyTheme();
    if (lastAnalysis) loadScene(lastAnalysis);
    resize();
  }

  lights = addLights(canvasTheme());
  applyTheme();

  onThemeChange(refresh);

  const ro = typeof ResizeObserver !== "undefined" ? new ResizeObserver(resize) : null;
  ro?.observe(canvas.parentElement ?? canvas);

  return {
    load,
    resize,
    refresh,
    highlightHole,
    dispose() {
      stopLoop();
      ro?.disconnect();
      controls.dispose();
      renderer.dispose();
    },
  };
}
