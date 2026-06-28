import * as THREE from "https://cdn.jsdelivr.net/npm/three@0.170.0/build/three.module.js";
import { buildHeatmapTextureData } from "./stock-sim.js";
import { canvasTheme, getTheme, onThemeChange } from "./theme.js";

function hex(css) {
  const h = css.replace("#", "").trim();
  return parseInt(h, 16) || 0x06080c;
}

/** STEP (Z-up) → Three.js (Y-up): x stays, y↔z */
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

function isVec3(arr) {
  return Array.isArray(arr) && arr.length >= 3 && Number.isFinite(Number(arr[0]));
}

function flattenMeshVerts(vertices) {
  if (!vertices?.length) return null;
  if (typeof vertices[0] === "number") return vertices;
  const out = [];
  for (const v of vertices) {
    if (!isVec3(v)) continue;
    out.push(Number(v[0]), Number(v[1]), Number(v[2]));
  }
  return out.length >= 9 ? out : null;
}

function transformBounds(min, max) {
  if (!isVec3(min) || !isVec3(max)) {
    return { min: [0, 0, 0], max: [1, 1, 1] };
  }
  const corners = [
    [min[0], min[1], min[2]],
    [max[0], min[1], min[2]],
    [max[0], max[1], min[2]],
    [min[0], max[1], min[2]],
    [min[0], min[1], max[2]],
    [max[0], min[1], max[2]],
    [max[0], max[1], max[2]],
    [min[0], max[1], max[2]],
  ];
  const outMin = [Infinity, Infinity, Infinity];
  const outMax = [-Infinity, -Infinity, -Infinity];
  for (const [x, y, z] of corners) {
    const v = stepToThree(x, y, z);
    outMin[0] = Math.min(outMin[0], v.x);
    outMin[1] = Math.min(outMin[1], v.y);
    outMin[2] = Math.min(outMin[2], v.z);
    outMax[0] = Math.max(outMax[0], v.x);
    outMax[1] = Math.max(outMax[1], v.y);
    outMax[2] = Math.max(outMax[2], v.z);
  }
  return { min: outMin, max: outMax };
}

function expandBoxBounds(box, min, max) {
  box.expandByPoint(stepToThree(min[0], min[1], min[2]));
  box.expandByPoint(stepToThree(max[0], max[1], max[2]));
}

export function createViewer3d(canvas) {
  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: false,
    powerPreference: "high-performance",
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 3));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.12;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(36, 1, 0.1, 5000);
  const group = new THREE.Group();
  scene.add(group);

  let hemi, key, fill, rim, grid, shadowPlane;
  let animId = null;
  let visible = true;
  let theta = 0.55;
  let thetaVel = 0;
  let target = new THREE.Vector3();
  let orbitRadius = 120;
  let orbitTarget = 120;
  let introT = 1;
  let lastAnalysis = null;
  let dragging = false;
  let lastPointerX = 0;

  function applyTheme() {
    const t = canvasTheme();
    const bg = hex(t.threeBgHex);
    scene.background = new THREE.Color(bg);
    scene.fog = new THREE.FogExp2(bg, t.threeFog);
    if (hemi) {
      hemi.color.setHex(t.threeKeyHex);
      hemi.groundColor.setHex(bg);
      hemi.intensity = 0.55;
    }
    if (grid) {
      grid.material.opacity = 0.14;
      grid.material.color.setHex(t.threeGrid);
    }
    if (shadowPlane) shadowPlane.material.opacity = getTheme() === "light" ? 0.08 : 0.22;
  }

  function setupLights() {
    if (hemi) return;
    hemi = new THREE.HemisphereLight(0xf5f5f7, 0x111113, 0.48);
    scene.add(hemi);

    key = new THREE.DirectionalLight(0xffffff, 1.05);
    key.position.set(60, 90, 45);
    key.castShadow = true;
    key.shadow.mapSize.set(1024, 1024);
    key.shadow.bias = -0.0004;
    key.shadow.camera.near = 1;
    key.shadow.camera.far = 500;
    key.shadow.camera.left = -80;
    key.shadow.camera.right = 80;
    key.shadow.camera.top = 80;
    key.shadow.camera.bottom = -80;
    scene.add(key);

    fill = new THREE.DirectionalLight(0xffffff, 0.28);
    fill.position.set(-50, 30, -40);
    scene.add(fill);

    rim = new THREE.DirectionalLight(0x8ab4ff, 0.22);
    rim.position.set(-30, 20, 70);
    scene.add(rim);

    grid = new THREE.GridHelper(200, 48, 0x2a2a2c, 0x1c1c1e);
    grid.material.transparent = true;
    grid.material.opacity = 0.14;
    scene.add(grid);

    const shadowGeo = new THREE.PlaneGeometry(1, 1);
    const shadowMat = new THREE.MeshBasicMaterial({
      color: 0x000000,
      transparent: true,
      opacity: 0.22,
      depthWrite: false,
    });
    shadowPlane = new THREE.Mesh(shadowGeo, shadowMat);
    shadowPlane.rotation.x = -Math.PI / 2;
    shadowPlane.renderOrder = -1;
    scene.add(shadowPlane);

    applyTheme();
  }

  setupLights();
  onThemeChange(() => {
    applyTheme();
    if (lastAnalysis) load(lastAnalysis);
  });

  const observer =
    typeof ResizeObserver !== "undefined" ? new ResizeObserver(resize) : null;
  if (observer) observer.observe(canvas);

  document.addEventListener("visibilitychange", () => {
    visible = !document.hidden;
    if (visible && group.children.length) startLoop();
    else stopLoop();
  });

  function resize() {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w === 0 || h === 0) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }

  function disposeObject(obj) {
    if (obj.geometry) obj.geometry.dispose();
    if (obj.material) {
      if (Array.isArray(obj.material)) obj.material.forEach((m) => m.dispose());
      else obj.material.dispose();
    }
  }

  function clearGroup() {
    stopLoop();
    while (group.children.length) {
      const c = group.children.pop();
      c.traverse?.((child) => {
        if (child.isMesh || child.isLine) disposeObject(child);
      });
      disposeObject(c);
    }
  }

  function fitCamera(partBox, animateIntro = false) {
    if (!partBox || partBox.isEmpty()) return;
    const center = partBox.getCenter(new THREE.Vector3());
    const size = partBox.getSize(new THREE.Vector3());
    const maxDim = Math.max(size.x, size.y, size.z, 1);
    orbitTarget = maxDim * 1.55;
    if (animateIntro) {
      orbitRadius = orbitTarget * 1.35;
      introT = 0;
    } else {
      orbitRadius = orbitTarget;
      introT = 1;
    }
    target.copy(center);
    camera.near = maxDim * 0.006;
    camera.far = maxDim * 60;
    camera.updateProjectionMatrix();
    const floorY = partBox.min.y - 0.02;
    if (grid) {
      grid.position.y = floorY;
      const gs = Math.ceil(maxDim * 2.8);
      grid.scale.set(gs / 200, 1, gs / 200);
    }
    if (shadowPlane) {
      const spread = maxDim * 1.15;
      shadowPlane.position.set(center.x, floorY + 0.01, center.z);
      shadowPlane.scale.set(spread, spread, 1);
    }
    if (key?.target) {
      key.target.position.copy(center);
      key.position.set(center.x + maxDim, center.y + maxDim * 1.2, center.z + maxDim * 0.6);
    }
  }

  function partMaterial(t, opts = {}) {
    return new THREE.MeshPhysicalMaterial({
      color: t.threePart,
      metalness: opts.metalness ?? 0.42,
      roughness: opts.roughness ?? 0.38,
      clearcoat: 0.35,
      clearcoatRoughness: 0.18,
      side: THREE.DoubleSide,
      ...opts,
    });
  }

  /** Sample in STEP coords; convert to Three at use site. */
  function sampleSegmentStep(seg, prev) {
    if (seg.kind === "drill") {
      const { xy, z_safe, z_bottom } = seg;
      if (!isVec3(xy) || !Number.isFinite(z_safe) || !Number.isFinite(z_bottom)) return [];
      const out = [];
      const a = [xy[0], xy[1], z_safe];
      if (prev) out.push({ v: a, rapid: true });
      for (let i = 1; i <= 8; i++) {
        const t = i / 8;
        out.push({
          v: [xy[0], xy[1], z_safe + (z_bottom - z_safe) * t],
          rapid: false,
        });
      }
      out.push({ v: a, rapid: true });
      return out;
    }
    if (seg.kind === "arc" && prev) {
      const to = seg.to;
      if (!isVec3(to)) return [];
      const [i, j] = seg.center_offset ?? [0, 0];
      const cx = prev[0] + i;
      const cy = prev[1] + j;
      const a0 = Math.atan2(prev[1] - cy, prev[0] - cx);
      const a1 = Math.atan2(to[1] - cy, to[0] - cx);
      let delta = a1 - a0;
      if (seg.clockwise) {
        if (delta >= 0) delta -= Math.PI * 2;
      } else if (delta <= 0) delta += Math.PI * 2;
      const r = Math.hypot(i, j) || 1;
      const out = [];
      for (let s = 1; s <= 16; s++) {
        const t = s / 16;
        const a = a0 + delta * t;
        out.push({
          v: [
            cx + Math.cos(a) * r,
            cy + Math.sin(a) * r,
            prev[2] + (to[2] - prev[2]) * t,
          ],
          rapid: false,
        });
      }
      return out;
    }
    const to = seg.to ?? seg.end_point;
    if (!to) return [];
    return [{ v: [to[0], to[1], to[2]], rapid: seg.kind === "rapid" }];
  }

  function addStockBlock(s, t, partBox, simulation) {
    const b = transformBounds(s.min, s.max);
    const dx = b.max[0] - b.min[0];
    const dy = b.max[1] - b.min[1];
    const dz = b.max[2] - b.min[2];
    const cx = (b.min[0] + b.max[0]) / 2;
    const cy = (b.min[1] + b.max[1]) / 2;
    const cz = (b.min[2] + b.max[2]) / 2;

    const geo = new THREE.BoxGeometry(dx, dy, dz);
    const solid = new THREE.Mesh(geo, partMaterial(t, { metalness: 0.08, roughness: 0.72 }));
    solid.position.set(cx, cy, cz);
    solid.castShadow = true;
    solid.receiveShadow = true;
    group.add(solid);
    partBox.expandByObject(solid);

    const edges = new THREE.LineSegments(
      new THREE.EdgesGeometry(geo),
      new THREE.LineBasicMaterial({
        color: t.threeRapid,
        transparent: true,
        opacity: 0.35,
      })
    );
    edges.position.copy(solid.position);
    group.add(edges);

    if (simulation) {
      const light = getTheme() === "light";
      const texData = buildHeatmapTextureData(simulation, light);
      if (texData) {
        const texCanvas = document.createElement("canvas");
        texCanvas.width = texData.width;
        texCanvas.height = texData.height;
        const ctx = texCanvas.getContext("2d");
        ctx.putImageData(new ImageData(texData.data, texData.width, texData.height), 0, 0);
        const map = new THREE.CanvasTexture(texCanvas);
        map.colorSpace = THREE.SRGBColorSpace;
        map.minFilter = THREE.LinearFilter;
        map.magFilter = THREE.LinearFilter;

        const top = new THREE.Mesh(
          new THREE.PlaneGeometry(dx * 0.998, dz * 0.998),
          new THREE.MeshBasicMaterial({
            map,
            transparent: true,
            opacity: 0.72,
            side: THREE.DoubleSide,
            depthWrite: false,
            polygonOffset: true,
            polygonOffsetFactor: -1,
          })
        );
        top.rotation.x = -Math.PI / 2;
        top.position.set(cx, b.max[1] + 0.08, cz);
        group.add(top);
      }
    }
    geo.dispose();
  }

  function load(analysis) {
    try {
      loadScene(analysis);
    } catch (err) {
      console.error("viewer3d:", err);
      clearGroup();
      resize();
      startLoop();
    }
  }

  function loadScene(analysis) {
    lastAnalysis = analysis;
    clearGroup();
    const t = canvasTheme();
    const partBox = new THREE.Box3();
    const preview = analysis.preview;
    const mesh = analysis.mesh ?? preview?.mesh;

    if (preview?.stock) {
      addStockBlock(preview.stock, t, partBox, analysis.stock_simulation);
    } else if (preview?.bounds?.min?.[0] != null) {
      expandBoxBounds(partBox, preview.bounds.min, preview.bounds.max);
    }

    const hasSolidMesh = mesh?.vertices?.length >= 9;

    if (!hasSolidMesh) {
      for (const pl of (preview?.planes || []).slice(0, 8)) {
        if (!isVec3(pl.normal) || !isVec3(pl.point) || !Number.isFinite(pl.half_extent)) continue;
        const size = pl.half_extent * 2;
        const geo = new THREE.PlaneGeometry(size, size);
        const mat = new THREE.MeshPhysicalMaterial({
          color: t.threePart,
          metalness: 0.4,
          roughness: 0.3,
          transparent: true,
          opacity: 0.75,
          side: THREE.DoubleSide,
        });
        const m = new THREE.Mesh(geo, mat);
        const n = stepToThree(pl.normal[0], pl.normal[1], pl.normal[2]).normalize();
        const p = stepToThree(pl.point[0], pl.point[1], pl.point[2]);
        m.position.copy(p);
        m.lookAt(p.clone().add(n));
        m.castShadow = true;
        group.add(m);
        partBox.expandByObject(m);
        geo.dispose();
      }
    }

    const cylinders = preview?.cylinders || [];
    for (const c of cylinders) {
      if (!isVec3(c.axis) || !isVec3(c.origin) || !Number.isFinite(c.radius) || !Number.isFinite(c.height)) continue;
      const axisLen = Math.hypot(c.axis[0], c.axis[1], c.axis[2]);
      if (axisLen < 1e-9 || c.radius <= 0 || c.height <= 0) continue;
      const geo = new THREE.CylinderGeometry(c.radius, c.radius, c.height, 40, 1, false);
      const mat = partMaterial(t, { metalness: 0.48, roughness: 0.34 });
      const m = new THREE.Mesh(geo, mat);
      m.castShadow = true;
      m.receiveShadow = true;
      const ax = stepToThree(c.axis[0], c.axis[1], c.axis[2]).normalize();
      const pos = stepToThree(c.origin[0], c.origin[1], c.origin[2]);
      m.position.copy(pos);
      m.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), ax);
      m.position.addScaledVector(ax, -c.height / 2);
      group.add(m);
      partBox.expandByObject(m);
      geo.dispose();
    }

    const showTessMesh = hasSolidMesh && mesh?.indices?.length >= 3;

    if (showTessMesh) {
      const flat = flattenMeshVerts(mesh.vertices);
      if (flat) {
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
    }

    const tp = analysis.toolpath;
    if (tp?.segments?.length) {
      const cutPts = [];
      const rapidPts = [];
      let prevStep = null;
      let prevThree = null;

      for (const seg of tp.segments) {
        const samples = sampleSegmentStep(seg, prevStep);
        for (const p of samples) {
          if (!isVec3(p.v)) continue;
          const v = stepToThree(p.v[0], p.v[1], p.v[2]);
          if (prevThree && Number.isFinite(v.x)) {
            if (p.rapid) rapidPts.push(prevThree.clone(), v);
            else cutPts.push(prevThree.clone(), v);
          }
          prevStep = p.v;
          prevThree = v;
        }
      }

      if (cutPts.length > 1) {
        const g = new THREE.BufferGeometry().setFromPoints(cutPts);
        group.add(
          new THREE.Line(
            g,
            new THREE.LineBasicMaterial({
              color: t.threeCut,
              transparent: true,
              opacity: 0.75,
            })
          )
        );
      }

      if (rapidPts.length > 1) {
        const g = new THREE.BufferGeometry().setFromPoints(rapidPts);
        const rapidLine = new THREE.Line(
          g,
          new THREE.LineDashedMaterial({
            color: t.threeRapid,
            transparent: true,
            opacity: 0.4,
            dashSize: 1.5,
            gapSize: 1,
          })
        );
        rapidLine.computeLineDistances();
        group.add(rapidLine);
      }
    }

    fitCamera(partBox, true);
    requestAnimationFrame(() => {
      resize();
      startLoop();
    });
  }

  function bindPointer() {
    canvas.style.touchAction = "none";
    canvas.addEventListener("pointerdown", (e) => {
      dragging = true;
      lastPointerX = e.clientX;
      canvas.setPointerCapture(e.pointerId);
    });
    canvas.addEventListener("pointermove", (e) => {
      if (!dragging) return;
      const dx = e.clientX - lastPointerX;
      lastPointerX = e.clientX;
      thetaVel += dx * 0.004;
    });
    const end = () => {
      dragging = false;
    };
    canvas.addEventListener("pointerup", end);
    canvas.addEventListener("pointercancel", end);
  }

  bindPointer();

  function renderFrame() {
    if (introT < 1) {
      introT = Math.min(1, introT + 0.024);
      const eased = 1 - (1 - introT) ** 3;
      orbitRadius = orbitTarget * (1.35 - 0.35 * eased);
    } else {
      orbitRadius += (orbitTarget - orbitRadius) * 0.06;
    }

    if (!dragging) thetaVel *= 0.92;
    theta += 0.0004 + thetaVel;
    thetaVel *= 0.94;

    camera.position.set(
      target.x + orbitRadius * Math.cos(theta),
      target.y + orbitRadius * 0.38,
      target.z + orbitRadius * Math.sin(theta)
    );
    camera.lookAt(target);
    renderer.render(scene, camera);
  }

  function startLoop() {
    stopLoop();
    if (!visible) return;
    const loop = () => {
      renderFrame();
      animId = requestAnimationFrame(loop);
    };
    loop();
  }

  function stopLoop() {
    if (animId) {
      cancelAnimationFrame(animId);
      animId = null;
    }
  }

  resize();

  return {
    load,
    resize,
    refresh: () => lastAnalysis && load(lastAnalysis),
    stop: stopLoop,
  };
}
