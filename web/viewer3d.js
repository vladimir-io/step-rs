/**
 * Fast part preview — tessellated mesh + B-rep cylinders, orbit controls.
 */
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { canvasTheme, onThemeChange } from "./theme.js";

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

export function createViewer3d(canvas) {
  if (!canvas) return { load() {}, resize() {}, refresh() {} };

  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: false,
    powerPreference: "high-performance",
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(42, 1, 0.05, 50000);
  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.06;
  controls.rotateSpeed = 0.65;
  controls.panSpeed = 0.75;
  controls.zoomSpeed = 0.9;
  controls.minDistance = 1;
  controls.maxDistance = 8000;

  const group = new THREE.Group();
  scene.add(group);

  const holeGroup = new THREE.Group();
  scene.add(holeGroup);

  let grid = null;
  let lastAnalysis = null;
  let raf = 0;
  let running = false;

  function applyTheme() {
    const t = canvasTheme();
    scene.background = new THREE.Color(toColor(t.threeBgHex));
    scene.fog = new THREE.FogExp2(toColor(t.threeBgHex), t.threeFog || 0.0004);
    if (grid) {
      grid.material.color.setHex(toColor(t.threeGrid));
      grid.material.opacity = 0.35;
    }
  }

  function partMaterial(t, opts = {}) {
    return new THREE.MeshPhysicalMaterial({
      color: toColor(t.threePart),
      metalness: opts.metalness ?? 0.42,
      roughness: opts.roughness ?? 0.28,
      clearcoat: 0.15,
      clearcoatRoughness: 0.4,
      envMapIntensity: 0.6,
    });
  }

  function holeMaterial(t) {
    return new THREE.MeshPhysicalMaterial({
      color: toColor(t.threeCut || "#4a9eff"),
      metalness: 0.55,
      roughness: 0.22,
      transparent: true,
      opacity: 0.55,
      depthWrite: false,
    });
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

  function fitCamera(box) {
    if (box.isEmpty()) return;
    const size = new THREE.Vector3();
    const center = new THREE.Vector3();
    box.getSize(size);
    box.getCenter(center);
    const maxDim = Math.max(size.x, size.y, size.z, 1);
    const dist = maxDim / (2 * Math.tan((camera.fov * Math.PI) / 360)) * 1.35;
    camera.position.set(center.x + dist * 0.75, center.y + dist * 0.55, center.z + dist * 0.85);
    controls.target.copy(center);
    controls.update();
    controls.minDistance = maxDim * 0.15;
    controls.maxDistance = maxDim * 12;
  }

  function addLights(t) {
    const key = new THREE.DirectionalLight(toColor(t.threeKeyHex || "#f5f5f7"), 1.35);
    key.position.set(40, 60, 30);
    key.castShadow = true;
    key.shadow.mapSize.set(1024, 1024);
    key.shadow.bias = -0.0002;
    const fill = new THREE.DirectionalLight(toColor(t.threePart), 0.35);
    fill.position.set(-30, 20, -40);
    const rim = new THREE.DirectionalLight(toColor(t.threeCut || "#4a9eff"), 0.25);
    rim.position.set(0, -20, 50);
    scene.add(key, fill, rim);
    return [key, fill, rim];
  }

  let lights = addLights(canvasTheme());

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
    const span = Math.max(size.x, size.z, 20) * 2.5;
    grid = new THREE.GridHelper(span, 24, toColor(t.threeGrid), toColor(t.threeGrid));
    grid.material.transparent = true;
    grid.material.opacity = 0.35;
    grid.position.set(center.x, box.min.y - 0.02, center.z);
    scene.add(grid);
  }

  function loadScene(analysis) {
    lastAnalysis = analysis;
    clearGroup(group);
    clearGroup(holeGroup);

    const t = canvasTheme();
    const partBox = new THREE.Box3();
    const preview = analysis.preview;
    const mesh = analysis.mesh ?? preview?.mesh;

    if (preview?.bounds?.min?.[0] != null) {
      partBox.expandByPoint(stepToThree(preview.bounds.min[0], preview.bounds.min[1], preview.bounds.min[2]));
      partBox.expandByPoint(stepToThree(preview.bounds.max[0], preview.bounds.max[1], preview.bounds.max[2]));
    }

    const flat = flattenVerts(mesh?.vertices);
    if (flat && mesh?.indices?.length >= 3) {
      const transformed = new Float32Array(flat.length);
      for (let i = 0; i < flat.length; i += 3) {
        const v = stepToThree(flat[i], flat[i + 1], flat[i + 2]);
        transformed[i] = v.x;
        transformed[i + 1] = v.y;
        transformed[i + 2] = v.z;
      }
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.BufferAttribute(transformed, 3));
      geo.setIndex(mesh.indices);
      geo.computeVertexNormals();
      const solid = new THREE.Mesh(geo, partMaterial(t));
      solid.castShadow = true;
      solid.receiveShadow = true;
      group.add(solid);
      partBox.expandByObject(solid);
    }

    for (const c of preview?.cylinders || []) {
      if (!isVec3(c.axis) || !isVec3(c.origin) || !Number.isFinite(c.radius) || c.radius <= 0) continue;
      const h = Number(c.height);
      if (!Number.isFinite(h) || h <= 0) continue;
      const geo = new THREE.CylinderGeometry(c.radius, c.radius, h, 48, 1, false);
      const m = new THREE.Mesh(geo, partMaterial(t, { metalness: 0.5, roughness: 0.32 }));
      m.castShadow = true;
      m.receiveShadow = true;
      const ax = stepToThree(c.axis[0], c.axis[1], c.axis[2]).normalize();
      const pos = stepToThree(c.origin[0], c.origin[1], c.origin[2]);
      m.position.copy(pos);
      m.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), ax);
      m.position.addScaledVector(ax, -h / 2);
      group.add(m);
      partBox.expandByObject(m);
    }

    for (const h of analysis.coaxial_holes || []) {
      const r = h.radius;
      const depth = h.depth;
      const o = h.axis_origin;
      const d = h.axis_direction;
      if (r == null || depth == null || !o || !d) continue;
      const ox = o.x ?? o[0];
      const oy = o.y ?? o[1];
      const oz = o.z ?? o[2];
      const dx = d.x ?? d[0];
      const dy = d.y ?? d[1];
      const dz = d.z ?? d[2];
      const geo = new THREE.CylinderGeometry(r, r, depth, 40, 1, true);
      const m = new THREE.Mesh(geo, holeMaterial(t));
      const ax = stepToThree(dx, dy, dz).normalize();
      const pos = stepToThree(ox, oy, oz);
      m.position.copy(pos);
      m.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), ax);
      m.position.addScaledVector(ax, depth / 2);
      holeGroup.add(m);
    }

    rebuildGrid(partBox);
    fitCamera(partBox.isEmpty() ? new THREE.Box3().setFromObject(group) : partBox);
  }

  function load(analysis) {
    try {
      loadScene(analysis);
      startLoop();
    } catch (e) {
      console.error("viewer3d:", e);
    }
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
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
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

  applyTheme();
  onThemeChange(refresh);

  const ro = typeof ResizeObserver !== "undefined" ? new ResizeObserver(resize) : null;
  ro?.observe(canvas);

  return {
    load,
    resize,
    refresh,
    dispose() {
      stopLoop();
      ro?.disconnect();
      controls.dispose();
      renderer.dispose();
    },
  };
}
