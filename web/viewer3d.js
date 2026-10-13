/**
 * STEP viewer: OpenCascade tessellation (occt-import-js) + coaxial bore overlay.
 */
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { loadOcct } from "./occt-client.js";
import { canvasTheme, onThemeChange } from "./theme.js";

function getOcct() {
  return loadOcct();
}

function stepVec(v) {
  if (!v) return new THREE.Vector3();
  if (Array.isArray(v)) return new THREE.Vector3(v[0], v[1], v[2]);
  return new THREE.Vector3(v.x ?? 0, v.y ?? 0, v.z ?? 0);
}

function toColor(v) {
  if (typeof v === "number") return v;
  const h = String(v ?? "").replace("#", "").trim();
  return parseInt(h, 16) || 0x5b9aff;
}

function buildOcctMesh(geometryMesh, dark) {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    "position",
    new THREE.Float32BufferAttribute(geometryMesh.attributes.position.array, 3)
  );
  if (geometryMesh.attributes.normal) {
    geometry.setAttribute(
      "normal",
      new THREE.Float32BufferAttribute(geometryMesh.attributes.normal.array, 3)
    );
  } else {
    geometry.computeVertexNormals();
  }
  geometry.setIndex(geometryMesh.index.array);

  const base =
    geometryMesh.color != null
      ? new THREE.Color(geometryMesh.color[0], geometryMesh.color[1], geometryMesh.color[2])
      : new THREE.Color(dark ? 0x9aa3b0 : 0xb8c0cc);

  const mesh = new THREE.Mesh(
    geometry,
    new THREE.MeshPhongMaterial({
      color: base,
      specular: dark ? 0x222228 : 0x444450,
      shininess: 35,
      side: THREE.DoubleSide,
    })
  );
  mesh.name = geometryMesh.name ?? "part";
  return mesh;
}

function buildRustMesh(meshData, dark) {
  const verts = meshData?.vertices;
  const indices = meshData?.indices;
  if (!Array.isArray(verts) || !verts.length || !Array.isArray(indices) || !indices.length) {
    return null;
  }

  const positions = new Float32Array(verts.length * 3);
  for (let i = 0; i < verts.length; i++) {
    const v = verts[i];
    positions[i * 3] = v[0];
    positions[i * 3 + 1] = v[1];
    positions[i * 3 + 2] = v[2];
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geometry.setIndex(indices);
  geometry.computeVertexNormals();

  return new THREE.Mesh(
    geometry,
    new THREE.MeshPhongMaterial({
      color: dark ? 0x8a929e : 0xa8b0bc,
      specular: dark ? 0x1a1a20 : 0x333340,
      shininess: 20,
      side: THREE.DoubleSide,
    })
  );
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

export function createViewer3d(canvas, opts = {}) {
  if (!canvas) return { load() {}, resize() {}, refresh() {}, highlightHole() {} };
  const onStatus = typeof opts.onStatus === "function" ? opts.onStatus : () => {};

  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: false });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1e7);
  camera.up.set(0, 0, 1);

  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;

  const partGroup = new THREE.Group();
  const overlayGroup = new THREE.Group();
  scene.add(partGroup, overlayGroup);

  let ambient = null;
  let keyLight = null;
  let grid = null;
  let lastStepBytes = null;
  let lastAnalysis = null;
  let selectedHole = 0;
  let raf = 0;
  let running = false;
  let loadGen = 0;
  let partBox = new THREE.Box3();

  /** Bidirectional line ∩ AABB → [tMin, tMax] in axis parameter space, or null. */
  function lineAabbInterval(origin, dir, box) {
    let tMin = -Infinity;
    let tMax = Infinity;
    const o = [origin.x, origin.y, origin.z];
    const d = [dir.x, dir.y, dir.z];
    const bmin = [box.min.x, box.min.y, box.min.z];
    const bmax = [box.max.x, box.max.y, box.max.z];
    for (let i = 0; i < 3; i++) {
      if (Math.abs(d[i]) < 1e-12) {
        if (o[i] < bmin[i] || o[i] > bmax[i]) return null;
        continue;
      }
      let t1 = (bmin[i] - o[i]) / d[i];
      let t2 = (bmax[i] - o[i]) / d[i];
      if (t1 > t2) [t1, t2] = [t2, t1];
      tMin = Math.max(tMin, t1);
      tMax = Math.min(tMax, t2);
      if (tMin > tMax) return null;
    }
    if (!Number.isFinite(tMin) || !Number.isFinite(tMax)) return null;
    return [tMin, tMax];
  }

  /** Place bore overlay inside the solid when STEP axis placement sits outside the mesh. */
  function fitHoleToPart(origin, ax, segments) {
    const total = segments.reduce((s, seg) => s + seg.depth, 0);
    if (!total || partBox.isEmpty()) {
      return { origin, ax, segments };
    }

    const pad = partBox.getSize(new THREE.Vector3()).length() * 0.02 + 0.05;
    const box = partBox.clone().expandByScalar(pad);
    const hit = lineAabbInterval(origin, ax, box);
    if (!hit) return { origin, ax, segments };

    const [tMin, tMax] = hit;
    const span = tMax - tMin;
    if (span <= 1e-6) return { origin, ax, segments };

    const drawDepth = Math.min(total, span);
    // Prefer the end of the solid along +axis (typical stock top) working back into the part.
    const startT = tMax - drawDepth;
    const fittedOrigin = origin.clone().addScaledVector(ax, startT);
    const scale = drawDepth / total;
    const fitted =
      scale >= 0.999
        ? segments
        : segments.map((seg) => ({
            radius: seg.radius,
            depth: Math.max(0.01, seg.depth * scale),
          }));
    return { origin: fittedOrigin, ax, segments: fitted };
  }

  function applyTheme() {
    const t = canvasTheme();
    const dark = document.documentElement.dataset.theme !== "light";
    scene.background = new THREE.Color(toColor(t.threeBgHex));
    scene.fog = null;
    if (ambient) ambient.intensity = dark ? 0.55 : 0.75;
    if (keyLight) keyLight.intensity = dark ? 0.95 : 0.85;
    if (grid) {
      grid.material.color.setHex(toColor(t.threeGrid));
      grid.material.opacity = dark ? 0.25 : 0.35;
    }
  }

  function setupLights() {
    if (ambient) scene.remove(ambient);
    if (keyLight) scene.remove(keyLight);
    const dark = document.documentElement.dataset.theme !== "light";
    ambient = new THREE.AmbientLight(0xffffff, dark ? 0.55 : 0.75);
    keyLight = new THREE.DirectionalLight(0xffffff, dark ? 0.95 : 0.85);
    scene.add(ambient, keyLight);
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

  function orientAlongAxis(mesh, ax, origin, offset, length) {
    mesh.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), ax);
    mesh.position.copy(origin).addScaledVector(ax, offset + length / 2);
  }

  function addCoaxialOverlay(hole, accent, mode) {
    const o = hole.axis_origin;
    const d = hole.axis_direction;
    if (!o || !d) return;

    const rawAx = stepVec(d).normalize();
    const rawOrigin = stepVec(o);
    const rawSegments = holeSegments(hole);
    if (!rawSegments.length) return;

    const fitted = fitHoleToPart(rawOrigin, rawAx, rawSegments);
    const { origin, ax, segments } = fitted;

    let cursor = 0;
    for (const seg of segments) {
      if (mode === "focus") {
        const mat = new THREE.MeshBasicMaterial({
          color: accent,
          transparent: true,
          opacity: 0.45,
          depthWrite: false,
          depthTest: true,
          side: THREE.DoubleSide,
        });
        const coreGeo = new THREE.CylinderGeometry(seg.radius, seg.radius, seg.depth, 40, 1, true);
        const core = new THREE.Mesh(coreGeo, mat);
        orientAlongAxis(core, ax, origin, cursor, seg.depth);
        core.renderOrder = 2;
        overlayGroup.add(core);

        const ringMat = new THREE.MeshBasicMaterial({
          color: accent,
          transparent: true,
          opacity: 0.85,
          depthWrite: false,
        });
        for (const t of [cursor, cursor + seg.depth]) {
          const ring = new THREE.Mesh(
            new THREE.TorusGeometry(seg.radius, Math.max(seg.radius * 0.04, 0.12), 8, 40),
            ringMat
          );
          ring.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), ax);
          ring.position.copy(origin).addScaledVector(ax, t);
          ring.renderOrder = 3;
          overlayGroup.add(ring);
        }
      } else {
        const ring = new THREE.Mesh(
          new THREE.TorusGeometry(seg.radius * 0.95, Math.max(seg.radius * 0.025, 0.08), 8, 32),
          new THREE.MeshBasicMaterial({ color: accent, transparent: true, opacity: 0.35, depthWrite: false })
        );
        ring.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), ax);
        ring.position.copy(origin).addScaledVector(ax, cursor);
        ring.renderOrder = 2;
        overlayGroup.add(ring);
      }
      cursor += seg.depth;
    }
  }

  function addOverlays(analysis) {
    clearGroup(overlayGroup);
    const holes = analysis?.coaxial_holes ?? [];
    if (selectedHole >= holes.length) selectedHole = 0;
    const accent = toColor(canvasTheme().threeCut || "#4a9eff");
    holes.forEach((h, i) => addCoaxialOverlay(h, accent, i === selectedHole ? "focus" : "marker"));
  }

  function fitCamera(box) {
    if (box.isEmpty()) return;
    const size = new THREE.Vector3();
    const center = new THREE.Vector3();
    box.getSize(size);
    box.getCenter(center);
    const span = Math.max(size.x, size.y, size.z, 1);
    const fov = (camera.fov * Math.PI) / 180;
    const dist = (span / (2 * Math.tan(fov / 2))) * 1.55;

    camera.position.set(center.x + dist * 0.75, center.y - dist * 0.65, center.z + dist * 0.45);
    camera.near = Math.max(span * 0.002, 0.05);
    camera.far = span * 30;
    camera.updateProjectionMatrix();

    controls.target.copy(center);
    controls.update();
    controls.minDistance = span * 0.15;
    controls.maxDistance = span * 12;

    if (keyLight) keyLight.position.set(center.x + span, center.y - span, center.z + span * 1.5);

    if (grid) {
      scene.remove(grid);
      grid.geometry.dispose();
      grid.material.dispose();
    }
    const t = canvasTheme();
    grid = new THREE.GridHelper(span * 2.5, 20, toColor(t.threeGrid), toColor(t.threeGrid));
    grid.material.transparent = true;
    grid.material.opacity = document.documentElement.dataset.theme === "light" ? 0.35 : 0.25;
    grid.rotation.x = Math.PI / 2;
    grid.position.set(center.x, center.y, box.min.z - span * 0.02);
    scene.add(grid);
  }

  async function tessellateStep(stepText) {
    const occt = await getOcct();
    const bytes = new TextEncoder().encode(stepText);
    const result = occt.ReadStepFile(bytes, null);
    if (!result?.success) {
      throw new Error("OpenCascade could not tessellate this STEP file");
    }
    return result;
  }

  async function loadScene(stepText, analysis) {
    const gen = ++loadGen;
    lastStepBytes = stepText;
    lastAnalysis = analysis;

    clearGroup(partGroup);
    clearGroup(overlayGroup);
    partBox = new THREE.Box3();
    onStatus("tessellating STEP...");

    const dark = document.documentElement.dataset.theme !== "light";
    let meshCount = 0;
    let usedFallback = false;

    try {
      const result = await tessellateStep(stepText);
      if (gen !== loadGen) return;

      if (result?.success) {
        for (const g of result.meshes ?? []) {
          const mesh = buildOcctMesh(g, dark);
          partGroup.add(mesh);
          partBox.expandByObject(mesh);
          meshCount += 1;
        }
      }
    } catch {
      if (gen !== loadGen) return;
    }

    if (!meshCount) {
      const fallback = buildRustMesh(analysis?.preview?.mesh ?? analysis?.mesh, dark);
      if (fallback) {
        partGroup.add(fallback);
        partBox.expandByObject(fallback);
        meshCount = 1;
        usedFallback = true;
      }
    }

    if (gen !== loadGen) return;

    canvas.dataset.meshes = String(meshCount);
    addOverlays(analysis);

    if (!meshCount) {
      onStatus("no solid mesh; overlays only");
    } else if (usedFallback) {
      onStatus("stock envelope (OCCT could not tessellate)");
      setTimeout(() => {
        if (gen === loadGen) onStatus("");
      }, 3200);
    } else {
      onStatus("");
    }

    const viewBox = partBox.clone();
    if (viewBox.isEmpty()) {
      viewBox.setFromObject(overlayGroup);
    }
    fitCamera(viewBox);
  }

  function load(stepText, analysis) {
    if (!stepText) return;
    selectedHole = 0;
    loadScene(stepText, analysis).then(() => startLoop()).catch(() => startLoop());
  }

  function highlightHole(index) {
    if (!lastAnalysis?.coaxial_holes?.length) return;
    const n = lastAnalysis.coaxial_holes.length;
    selectedHole = ((index % n) + n) % n;
    addOverlays(lastAnalysis);
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
    setupLights();
    applyTheme();
    if (lastStepBytes && lastAnalysis) loadScene(lastStepBytes, lastAnalysis);
    resize();
  }

  setupLights();
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
      if (raf) cancelAnimationFrame(raf);
      ro?.disconnect();
      controls.dispose();
      renderer.dispose();
    },
  };
}
