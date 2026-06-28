const STORAGE_KEY = "steprs-theme";

export function getTheme() {
  return document.documentElement.getAttribute("data-theme") === "light"
    ? "light"
    : "dark";
}

export function setTheme(theme) {
  const next = theme === "light" ? "light" : "dark";
  document.documentElement.setAttribute("data-theme", next);
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    /* ignore */
  }
  document.querySelector('meta[name="theme-color"]')?.setAttribute(
    "content",
    next === "light" ? "#fafafa" : "#0c0c0e"
  );
  window.dispatchEvent(new CustomEvent("steprs-theme", { detail: next }));
}

export function toggleTheme() {
  setTheme(getTheme() === "light" ? "dark" : "light");
}

export function initTheme() {
  const btn = document.getElementById("theme-toggle");
  if (btn) {
    btn.addEventListener("click", () => {
      toggleTheme();
      updateThemeButton(btn);
    });
    updateThemeButton(btn);
  }
  window.addEventListener("steprs-theme", () => btn && updateThemeButton(btn));
}

function updateThemeButton(btn) {
  const light = getTheme() === "light";
  btn.setAttribute("aria-label", light ? "Switch to dark mode" : "Switch to light mode");
  btn.setAttribute("title", light ? "dark" : "light");
  btn.dataset.mode = light ? "light" : "dark";
}

/** Read canvas palette from CSS variables (for 2D/WebGL). */
export function canvasTheme() {
  const s = getComputedStyle(document.documentElement);
  const v = (name) => s.getPropertyValue(name).trim();
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
    threePart: parseInt(v("--three-part").replace("#", ""), 16) || 0xc8cdd4,
    threeCut: parseInt(v("--three-cut").replace("#", ""), 16) || 0x4a9eff,
    threeRapid: parseInt(v("--three-rapid").replace("#", ""), 16) || 0x636366,
    threeGrid: parseInt(v("--three-grid").replace("#", ""), 16) || 0x2c2c2e,
    threeKeyHex: parseInt(v("--three-key").replace("#", ""), 16) || 0xf5f5f7,
  };
}

export function onThemeChange(fn) {
  window.addEventListener("steprs-theme", () => fn(getTheme()));
  fn(getTheme());
}
