/**
 * Disney-grade microinteractions — spring press, magnetic hover, delight pops.
 */
import gsap from "gsap";
import { prefersReducedMotion } from "./motion.js";

const SPRING = "elastic.out(1, 0.62)";
const SNAP = "back.out(2)";

export function initMicrointeractions() {
  if (prefersReducedMotion()) return;

  bindSpringPress(
    ".landing-drop, .sample-chip, .action-btn, .theme-btn, .landing-sample-cta, .drop, .brand"
  );
  bindMagnetic(".landing-drop-wrap", 0.18);
  bindMagnetic(".sample-chip", 0.12);
  bindLandingDragDelight();
  bindBrandDelight();
}

function bindSpringPress(selector) {
  document.querySelectorAll(selector).forEach((el) => {
    if (el.dataset.miPress) return;
    el.dataset.miPress = "1";

    const down = (e) => {
      if (el.disabled || e.button > 0) return;
      gsap.killTweensOf(el);
      gsap.to(el, { scale: 0.94, duration: 0.14, ease: "power2.out" });
    };
    const up = () => {
      gsap.to(el, { scale: 1, duration: 0.65, ease: SPRING });
    };

    el.addEventListener("pointerdown", down);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
    el.addEventListener("pointerleave", up);
  });
}

function bindMagnetic(selector, strength) {
  document.querySelectorAll(selector).forEach((el) => {
    if (el.dataset.miMag) return;
    el.dataset.miMag = "1";

    const inner = el.querySelector(".landing-drop") ?? el;
    let bounds = null;

    el.addEventListener("pointerenter", () => {
      bounds = el.getBoundingClientRect();
    });
    el.addEventListener("pointermove", (e) => {
      if (!bounds) return;
      const x = e.clientX - bounds.left - bounds.width / 2;
      const y = e.clientY - bounds.top - bounds.height / 2;
      gsap.to(inner, {
        x: x * strength,
        y: y * strength,
        duration: 0.45,
        ease: "power3.out",
      });
    });
    el.addEventListener("pointerleave", () => {
      bounds = null;
      gsap.to(inner, { x: 0, y: 0, duration: 0.85, ease: SPRING });
    });
  });
}

function bindLandingDragDelight() {
  const landing = document.getElementById("landing");
  const drop = document.getElementById("landing-cta");
  if (!landing || !drop) return;

  landing.addEventListener("dragover", () => {
    gsap.to(drop, { scale: 1.04, duration: 0.35, ease: SNAP });
  });
  landing.addEventListener("dragleave", (e) => {
    if (landing.contains(e.relatedTarget)) return;
    gsap.to(drop, { scale: 1, duration: 0.55, ease: SPRING });
  });
  landing.addEventListener("drop", () => {
    gsap.fromTo(drop, { scale: 1.06 }, { scale: 1, duration: 0.7, ease: SPRING });
  });
}

function bindBrandDelight() {
  const brand = document.querySelector(".brand");
  if (!brand || brand.dataset.miBrand) return;
  brand.dataset.miBrand = "1";
  brand.addEventListener("pointerenter", () => {
    gsap.fromTo(brand, { letterSpacing: "-0.04em" }, {
      letterSpacing: "-0.02em",
      duration: 0.5,
      ease: "power2.out",
    });
  });
  brand.addEventListener("pointerleave", () => {
    gsap.to(brand, { letterSpacing: "-0.04em", duration: 0.6, ease: SPRING });
  });
}

export function animateThemeToggle(btn) {
  if (!btn || prefersReducedMotion()) return;
  const icon = btn.querySelector("svg");
  gsap.fromTo(
    btn,
    { rotate: 0, scale: 1 },
    { rotate: 180, scale: 1.08, duration: 0.55, ease: SNAP }
  );
  gsap.to(btn, { rotate: 360, scale: 1, duration: 0.45, ease: SPRING, delay: 0.1 });
  if (icon) {
    gsap.fromTo(icon, { opacity: 0, y: 6 }, { opacity: 1, y: 0, duration: 0.4, ease: "power2.out" });
  }
}

export function popSystemTestRow(row) {
  if (!row || prefersReducedMotion()) return;
  const mark = row.querySelector(".system-test-mark");
  gsap.fromTo(
    row,
    { opacity: 0.4, x: -8 },
    { opacity: 1, x: 0, duration: 0.55, ease: "power3.out" }
  );
  if (row.classList.contains("pass") && mark) {
    gsap.fromTo(mark, { scale: 0, rotate: -40 }, { scale: 1, rotate: 0, duration: 0.65, ease: SNAP });
  }
}

export function revealManifestPanel(panel) {
  if (!panel || panel.classList.contains("hidden") || prefersReducedMotion()) return;
  gsap.fromTo(
    panel,
    { opacity: 0, y: -12, scaleY: 0.92, transformOrigin: "top center" },
    { opacity: 1, y: 0, scaleY: 1, duration: 0.65, ease: SPRING }
  );
}

export function toggleIpMaskDelight(toggle) {
  if (!toggle || prefersReducedMotion()) return;
  const label = toggle.closest(".ip-mask-toggle");
  gsap.fromTo(label, { scale: 1 }, { scale: 1.04, duration: 0.2, yoyo: true, repeat: 1, ease: "power2.inOut" });
}

export function celebrateAnalysisComplete() {
  if (prefersReducedMotion()) return;
  const metrics = document.querySelectorAll(".metric-card .metric-value");
  gsap.fromTo(metrics, { scale: 1 }, {
    scale: 1.06,
    duration: 0.22,
    yoyo: true,
    repeat: 1,
    ease: "power2.inOut",
    stagger: 0.04,
  });
}

export function flashMessage(el, isError = false) {
  if (!el || el.classList.contains("hidden") || prefersReducedMotion()) return;
  gsap.fromTo(
    el,
    { opacity: 0, y: isError ? -6 : -4 },
    { opacity: 1, y: 0, duration: 0.45, ease: SPRING }
  );
}
