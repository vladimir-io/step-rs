import { prefersReducedMotion } from "./motion.js";
import { animateThemeToggle } from "./microinteractions.js";

const STORAGE_KEY = "steprs-theme";

export function getTheme() {
  return document.documentElement.getAttribute("data-theme") === "light"
    ? "light"
    : "dark";
}

function applyTheme(next) {
  document.documentElement.setAttribute("data-theme", next);
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    /* ignore */
  }
  document.querySelector('meta[name="theme-color"]')?.setAttribute(
    "content",
    next === "light" ? "#f5f7fc" : "#06060a"
  );
  window.dispatchEvent(new CustomEvent("steprs-theme", { detail: next }));
}

export function setTheme(theme) {
  const next = theme === "light" ? "light" : "dark";
  if (next === getTheme()) return;

  const reduced = prefersReducedMotion();
  const root = document.documentElement;
  const commit = () => applyTheme(next);

  if (!reduced && document.startViewTransition) {
    document.startViewTransition(commit);
    return;
  }

  if (!reduced) {
    root.classList.add("theme-transitioning");
    commit();
    window.setTimeout(() => root.classList.remove("theme-transitioning"), 480);
    return;
  }

  commit();
}

export function toggleTheme(btn) {
  setTheme(getTheme() === "light" ? "dark" : "light");
  if (btn) animateThemeToggle(btn);
}

export function initTheme() {
  const btn = document.getElementById("theme-toggle");
  if (btn) {
    btn.addEventListener("click", () => toggleTheme(btn));
    updateThemeButton(btn);
  }
  window.addEventListener("steprs-theme", () => btn && updateThemeButton(btn));
}

function updateThemeButton(btn) {
  const light = getTheme() === "light";
  btn.setAttribute("aria-label", light ? "Switch to dark mode" : "Switch to light mode");
  btn.setAttribute("title", light ? "Dark mode" : "Light mode");
  btn.dataset.mode = light ? "light" : "dark";
}

/** Read canvas palette from CSS variables (for 2D/WebGL). */
export function canvasTheme() {
  const s = getComputedStyle(document.documentElement);
  const v = (name) => s.getPropertyValue(name).trim();
  const hex = (name, fallback) => {
    const raw = v(name) || fallback;
    if (raw.startsWith("#")) return parseInt(raw.slice(1), 16);
    return parseInt(fallback.replace("#", ""), 16);
  };
  return {
    bg: v("--canvas-bg"),
    grid: v("--canvas-grid"),
    gridStrong: v("--canvas-grid-strong"),
    cut: v("--canvas-cut"),
    rapid: v("--canvas-rapid"),
    tip: v("--canvas-tip"),
    preview: v("--canvas-preview"),
    label: v("--canvas-label"),
    threeBgHex: v("--three-bg") || "#111113",
    threeFog: parseFloat(v("--three-fog")) || 0.00035,
    threePart: hex("--three-part", "#c8cdd4"),
    threeCut: hex("--three-cut", "#4a9eff"),
    threeRapid: hex("--three-rapid", "#636366"),
    threeGrid: hex("--three-grid", "#2c2c2e"),
    threeKeyHex: hex("--three-key", "#f5f5f7"),
  };
}

export function onThemeChange(fn) {
  window.addEventListener("steprs-theme", () => fn(getTheme()));
  fn(getTheme());
}
