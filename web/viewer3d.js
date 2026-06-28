/**
 * Part preview — opaque tessellated mesh, focused x-ray on selected bore.
 */
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { mergeVertices } from "three/addons/utils/BufferGeometryUtils.js";
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

function hasSolidMesh(mesh) {
  return mesh?.indices?.length >= 3 && mesh?.vertices?.length >= 3;
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
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.0;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;

  const pmrem = new THREE.PMREMGenerator(renderer);
  pmrem.compileEquirectangularShader();

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 500000);
  const controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.06;
  controls.rotateSpeed = 0.5;

  const partGroup = new THREE.Group();
  const overlayGroup = new THREE.Group();
  scene.add(partGroup, overlayGroup);

  let grid = null;
  let envTexture = null;
  let lights = [];
  let lastAnalysis = null;
  let selectedHole = 0;
  let raf = 0;
  let running = false;
  let partScale = 1;

  function applyTheme() {
    const t = canvasTheme();
    scene.background = new THREE.Color(toColor(t.threeBgHex));
    const fogDensity = t.threeFog || 0.00006;
    scene.fog = fogDensity > 0 ? new THREE.FogExp2(toColor(t.threeBgHex), fogDensity) : null;
    renderer.toneMappingExposure = document.documentElement.dataset.theme === "light" ? 0.98 : 1.02;
    if (grid) {
      grid.material.color.setHex(toColor(t.threeGrid));
      grid.material.opacity = document.documentElement.dataset.theme === "light" ? 0.35 : 0.22;
    }
  }

  function setEnvironment() {
    if (envTexture) envTexture.dispose();
    envTexture = pmrem.fromScene(new RoomEnvironment(renderer), 0.04).texture;
    scene.environment = envTexture;
  }

  function partMaterial(t) {
    return new THREE.MeshStandardMaterial({
      color: toColor(t.threePart),
      metalness: 0.42,
      roughness: 0.48,
      envMapIntensity: 0.7,
      transparent: false,
      opacity: 1,
    });
  }

  function focusBoreMaterial(accent) {
    return new THREE.MeshPhysicalMaterial({
      color: accent,
      emissive: accent,
      emissiveIntensity: 1.4,
      metalness: 0,
      roughness: 0.2,
      transparent: true,
      opacity: 0.62,
      side: THREE.DoubleSide,
      depthWrite: false,
      depthTest: true,
      toneMapped: false,
    });
  }

  function markerRingMaterial(accent) {
    return new THREE.MeshBasicMaterial({
      color: accent,
      transparent: true,
      opacity: 0.35,
      depthWrite: false,
      toneMapped: false,
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
      new THREE.TorusGeometry(radius, Math.max(radius * 0.04, 0.15), 8, 48),
      markerRingMaterial(accent)
    );
    ring.material.opacity = opacity;
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
    const accent = toColor(t.threeCut || "#5b9aff");
    const segments = holeSegments(hole);
    if (!segments.length) return;

    let cursor = 0;
    for (const seg of segments) {
      if (mode === "focus") {
        const coreGeo = new THREE.CylinderGeometry(seg.radius, seg.radius, seg.depth, 48, 1, true);
        const core = new THREE.Mesh(coreGeo, focusBoreMaterial(accent));
        orientAlongAxis(core, ax, origin, cursor, seg.depth);
        core.renderOrder = 2;
        overlayGroup.add(core);
        addRing(seg.radius, ax, origin, cursor, accent, 0.85);
        addRing(seg.radius, ax, origin, cursor + seg.depth, accent, 0.85);
      } else {
        addRing(seg.radius * 0.92, ax, origin, cursor, accent, 0.22);
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

  function addAnalyticFallback(preview, t, partBox) {
    for (const c of preview?.cylinders || []) {
      if (!isVec3(c.axis) || !isVec3(c.origin) || !Number.isFinite(c.radius) || c.radius <= 0) continue;
      const h = Number(c.height);
      if (!Number.isFinite(h) || h <= 0) continue;
      const geo = new THREE.CylinderGeometry(c.radius, c.radius, h, 64, 1, false);
      const m = new THREE.Mesh(geo, partMaterial(t));
      m.castShadow = true;
      m.receiveShadow = true;
      const ax = stepToThree(c.axis[0], c.axis[1], c.axis[2]).normalize();
      const pos = stepToThree(c.origin[0], c.origin[1], c.origin[2]);
      m.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), ax);
      m.position.copy(pos).addScaledVector(ax, h / 2);
      partGroup.add(m);
      partBox.expandByObject(m);
    }
  }

  function fitCamera(box) {
    if (box.isEmpty()) return;
    const size = new THREE.Vector3();
    const center = new THREE.Vector3();
    box.getSize(size);
    box.getCenter(center);
    partScale = Math.max(size.x, size.y, size.z, 1);

    const fov = (camera.fov * Math.PI) / 180;
    const dist = (partScale / (2 * Math.tan(fov / 2))) * 1.35;
    camera.position.set(center.x + dist * 0.68, center.y + dist * 0.48, center.z + dist * 0.72);
    camera.near = Math.max(partScale * 0.002, 0.05);
    camera.far = Math.max(partScale * 50, 1000);
    camera.updateProjectionMatrix();

    controls.target.copy(center);
    controls.update();
    controls.minDistance = partScale * 0.25;
    controls.maxDistance = partScale * 12;

    repositionLights(center, partScale);
  }

  function repositionLights(center, scale) {
    const s = scale * 1.8;
    if (lights[0]) {
      lights[0].position.set(center.x + s, center.y + s * 1.2, center.z + s * 0.8);
      lights[0].target.position.copy(center);
      lights[0].shadow.camera.left = -s;
      lights[0].shadow.camera.right = s;
      lights[0].shadow.camera.top = s;
      lights[0].shadow.camera.bottom = -s;
      lights[0].shadow.camera.near = 0.1;
      lights[0].shadow.camera.far = s * 6;
      lights[0].shadow.camera.updateProjectionMatrix();
    }
    if (lights[1]) lights[1].position.set(center.x - s * 0.8, center.y + s * 0.4, center.z - s);
    if (lights[2]) lights[2].position.set(center.x, center.y + s * 0.3, center.z - s * 1.2);
  }

  function addLights(t) {
    scene.add(new THREE.HemisphereLight(toColor(t.threeKeyHex || "#fafafa"), toColor(t.threeGrid), 0.35));

    const key = new THREE.DirectionalLight(toColor(t.threeKeyHex || "#ffffff"), 1.35);
    key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048);
    key.shadow.bias = -0.00015;
    key.shadow.normalBias = 0.02;

    const fill = new THREE.DirectionalLight(toColor(t.threePart), 0.55);
    const rim = new THREE.DirectionalLight(toColor(t.threeCut || "#5b9aff"), 0.2);

    for (const l of [key, fill, rim]) {
      scene.add(l);
      if (l.target) scene.add(l.target);
    }
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
    const span = Math.max(size.x, size.z, partScale) * 2.2;
    grid = new THREE.GridHelper(span, 24, toColor(t.threeGrid), toColor(t.threeGrid));
    grid.material.transparent = true;
    grid.material.opacity = document.documentElement.dataset.theme === "light" ? 0.35 : 0.22;
    grid.position.set(center.x, box.min.y - partScale * 0.01, center.z);
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

    if (preview?.bounds?.min?.[0] != null) {
      partBox.expandByPoint(stepToThree(preview.bounds.min[0], preview.bounds.min[1], preview.bounds.min[2]));
      partBox.expandByPoint(stepToThree(preview.bounds.max[0], preview.bounds.max[1], preview.bounds.max[2]));
    }

    if (hasSolidMesh(mesh)) {
      const geo = buildSolidGeometry(mesh);
      if (geo) {
        const solid = new THREE.Mesh(geo, partMaterial(t));
        solid.castShadow = true;
        solid.receiveShadow = true;
        solid.renderOrder = 0;
        partGroup.add(solid);
        partBox.expandByObject(solid);
      }
    } else {
      addAnalyticFallback(preview, t, partBox);
    }

    holes.forEach((h, i) => {
      addCoaxialOverlay(h, t, i === selectedHole ? "focus" : "marker");
    });

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
    canvas.width = Math.floor(w * renderer.getPixelRatio());
    canvas.height = Math.floor(h * renderer.getPixelRatio());
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }

  function refresh() {
    lights.forEach((l) => {
      scene.remove(l);
      if (l.target) scene.remove(l.target);
    });
    lights = addLights(canvasTheme());
    setEnvironment();
    applyTheme();
    if (lastAnalysis) loadScene(lastAnalysis);
    resize();
  }

  setEnvironment();
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
      envTexture?.dispose();
      pmrem.dispose();
      renderer.dispose();
    },
  };
}
