/**
 * Three.js coaxial-bore hero — smooth damped motion, studio lighting.
 */
import * as THREE from "three";
import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/addons/postprocessing/UnrealBloomPass.js";
import { prefersReducedMotion } from "./motion.js";

function readTheme() {
  const s = getComputedStyle(document.documentElement);
  const isLight = document.documentElement.dataset.theme === "light";
  return {
    bg: new THREE.Color(s.getPropertyValue("--three-bg").trim() || (isLight ? "#f4f4f5" : "#050505")),
    accent: new THREE.Color(s.getPropertyValue("--accent").trim() || "#3b82f6"),
    metal: new THREE.Color(isLight ? "#a1a1aa" : "#71717a"),
    fog: isLight ? 0.0018 : 0.0009,
  };
}

function buildCoaxialPart(accent, metal) {
  const root = new THREE.Group();

  const block = new THREE.Mesh(
    new THREE.BoxGeometry(2.4, 1.35, 2.4),
    new THREE.MeshPhysicalMaterial({
      color: metal,
      metalness: 0.92,
      roughness: 0.22,
      clearcoat: 0.45,
      clearcoatRoughness: 0.25,
    })
  );
  block.castShadow = true;
  block.receiveShadow = true;
  root.add(block);

  const boreMat = new THREE.MeshPhysicalMaterial({
    color: accent,
    metalness: 0.55,
    roughness: 0.15,
    emissive: accent.clone().multiplyScalar(0.35),
    emissiveIntensity: 0.65,
    transparent: true,
    opacity: 0.92,
    side: THREE.DoubleSide,
    depthWrite: false,
  });

  const outerBore = new THREE.Mesh(new THREE.CylinderGeometry(0.52, 0.52, 0.55, 72, 1, true), boreMat);
  outerBore.position.y = 0.18;
  root.add(outerBore);

  const innerBore = new THREE.Mesh(new THREE.CylinderGeometry(0.26, 0.26, 0.62, 72, 1, true), boreMat.clone());
  innerBore.material.emissiveIntensity = 0.85;
  innerBore.position.y = -0.08;
  root.add(innerBore);

  const stepRing = new THREE.Mesh(
    new THREE.TorusGeometry(0.39, 0.012, 16, 96),
    new THREE.MeshBasicMaterial({ color: accent, transparent: true, opacity: 0.9 })
  );
  stepRing.rotation.x = Math.PI / 2;
  stepRing.position.y = -0.07;
  root.add(stepRing);

  const axis = new THREE.Mesh(
    new THREE.CylinderGeometry(0.006, 0.006, 1.5, 12),
    new THREE.MeshBasicMaterial({ color: accent, transparent: true, opacity: 0.35 })
  );
  root.add(axis);

  return root;
}

function buildParticles(count, spread) {
  const pos = new Float32Array(count * 3);
  for (let i = 0; i < count; i++) {
    pos[i * 3] = (Math.random() - 0.5) * spread;
    pos[i * 3 + 1] = (Math.random() - 0.5) * spread * 0.6;
    pos[i * 3 + 2] = (Math.random() - 0.5) * spread;
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  const mat = new THREE.PointsMaterial({
    size: 0.018,
    transparent: true,
    opacity: 0.45,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  });
  return new THREE.Points(geo, mat);
}

export function initLandingScene(canvas) {
  if (!canvas) return { destroy() {}, applyTheme() {} };

  const reduced = prefersReducedMotion();
  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    powerPreference: "high-performance",
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(34, 1, 0.1, 100);
  camera.position.set(3.2, 1.1, 4.4);

  let composer = null;
  let bloomPass = null;

  let theme = readTheme();
  scene.background = theme.bg;
  scene.fog = new THREE.FogExp2(theme.bg, theme.fog);

  const part = buildCoaxialPart(theme.accent, theme.metal);
  part.rotation.x = 0.28;
  scene.add(part);

  const particles = buildParticles(reduced ? 80 : 220, 9);
  scene.add(particles);

  const key = new THREE.DirectionalLight(0xffffff, 2.4);
  key.position.set(4, 6, 3);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  scene.add(key);

  const fill = new THREE.DirectionalLight(0x93c5fd, 0.55);
  fill.position.set(-3, 2, -2);
  scene.add(fill);

  scene.add(new THREE.AmbientLight(0xffffff, 0.35));

  const plane = new THREE.Mesh(
    new THREE.PlaneGeometry(20, 20),
    new THREE.ShadowMaterial({ opacity: 0.18 })
  );
  plane.rotation.x = -Math.PI / 2;
  plane.position.y = -0.72;
  plane.receiveShadow = true;
  scene.add(plane);

  const pointer = { x: 0, y: 0, active: false };
  const look = { x: 0, y: 0 };
  const lookTarget = { x: 0, y: 0 };
  let raf = 0;
  let running = true;
  let w = 0;
  let h = 0;
  const clock = new THREE.Clock();

  function resize() {
    const rect = canvas.parentElement?.getBoundingClientRect() ?? canvas.getBoundingClientRect();
    w = Math.max(1, rect.width);
    h = Math.max(1, rect.height);
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    if (composer) {
      composer.setSize(w, h);
      bloomPass?.resolution.set(w, h);
    }
  }

  if (!reduced) {
    resize();
    composer = new EffectComposer(renderer);
    composer.addPass(new RenderPass(scene, camera));
    bloomPass = new UnrealBloomPass(
      new THREE.Vector2(w, h),
      document.documentElement.dataset.theme === "light" ? 0.18 : 0.32,
      0.55,
      0.62
    );
    composer.addPass(bloomPass);
  }

  function onPointerMove(e) {
    const rect = canvas.getBoundingClientRect();
    pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    pointer.y = -(((e.clientY - rect.top) / rect.height) * 2 - 1);
    pointer.active = true;
  }

  function onPointerLeave() {
    pointer.active = false;
  }

  function applyTheme() {
    theme = readTheme();
    scene.background = theme.bg;
    scene.fog.color = theme.bg;
    scene.fog.density = theme.fog;
    particles.material.color = theme.accent;
    if (bloomPass) {
      bloomPass.strength = document.documentElement.dataset.theme === "light" ? 0.18 : 0.32;
    }
  }

  function frame() {
    if (!running) return;
    const t = clock.getElapsedTime();

    if (!reduced) {
      part.rotation.y = t * 0.14;
      particles.rotation.y = t * 0.03;
      lookTarget.x = pointer.active ? pointer.x * 0.38 : 0;
      lookTarget.y = pointer.active ? pointer.y * 0.22 : 0;
      look.x += (lookTarget.x - look.x) * 0.035;
      look.y += (lookTarget.y - look.y) * 0.035;
    }

    camera.position.x = 3.2 + look.x;
    camera.position.y = 1.1 + look.y;
    camera.lookAt(look.x * 0.3, look.y * 0.2, 0);

    if (composer) composer.render();
    else renderer.render(scene, camera);
    if (!reduced) raf = requestAnimationFrame(frame);
  }

  const ro = new ResizeObserver(resize);
  ro.observe(canvas.parentElement ?? canvas);
  if (reduced) resize();
  window.addEventListener("pointermove", onPointerMove, { passive: true });
  canvas.addEventListener("pointerleave", onPointerLeave);

  if (reduced) frame();
  else raf = requestAnimationFrame(frame);

  return {
    destroy() {
      running = false;
      cancelAnimationFrame(raf);
      ro.disconnect();
      window.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      renderer.dispose();
    },
    applyTheme,
  };
}
