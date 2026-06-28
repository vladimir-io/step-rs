/** Shared motion primitives — respects prefers-reduced-motion. */

export const easeOutExpo = (t) => (t >= 1 ? 1 : 1 - 2 ** (-10 * t));
export const easeOutCubic = (t) => 1 - (1 - t) ** 3;
export const easeInOutCubic = (t) =>
  t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2;

export function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function wait(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

export function animateNumber(el, to, opts = {}) {
  const { duration = 720, decimals = 0, formatter } = opts;
  if (!el || prefersReducedMotion()) {
    const text = formatter ? formatter(to) : formatNum(to, decimals);
    el.textContent = text;
    return Promise.resolve();
  }

  const from = parseFloat(el.textContent?.replace(/[^0-9.-]/g, "")) || 0;
  const t0 = performance.now();

  return new Promise((resolve) => {
    const step = (now) => {
      const t = Math.min(1, (now - t0) / duration);
      const v = from + (to - from) * easeOutExpo(t);
      el.textContent = formatter ? formatter(v) : formatNum(v, decimals);
      if (t < 1) requestAnimationFrame(step);
      else resolve();
    };
    requestAnimationFrame(step);
  });
}

function formatNum(v, decimals) {
  if (decimals === 0) return Math.round(v).toLocaleString();
  return v.toFixed(decimals);
}

/** Smooth width/opacity progress bar driver. */
export function createProgressDriver(barEl, trackEl) {
  let target = 0;
  let current = 0;
  let raf = null;
  let indeterminate = false;

  function frame() {
    const delta = target - current;
    if (Math.abs(delta) < 0.2) current = target;
    else current += delta * 0.14;
    if (barEl) barEl.style.width = `${current}%`;
    if (Math.abs(target - current) > 0.05 || indeterminate) {
      raf = requestAnimationFrame(frame);
    } else {
      raf = null;
    }
  }

  function kick() {
    if (!raf) raf = requestAnimationFrame(frame);
  }

  return {
    set(pct) {
      indeterminate = false;
      trackEl?.classList.remove("is-indeterminate");
      target = Math.max(0, Math.min(100, pct));
      kick();
    },
    indeterminate(on) {
      indeterminate = on;
      trackEl?.classList.toggle("is-indeterminate", on);
      if (on) {
        target = 92;
        current = 8;
        kick();
      }
    },
    finish() {
      indeterminate = false;
      trackEl?.classList.remove("is-indeterminate");
      target = 100;
      kick();
      return wait(420);
    },
    reset() {
      target = 0;
      current = 0;
      if (barEl) barEl.style.width = "0%";
    },
  };
}

/** Reveal result sections. */
export async function revealStagger(elements) {
  const list = elements.filter(Boolean);
  if (!list.length) return;
  list.forEach((el) => {
    el.classList.remove("hidden");
    el.classList.add("flow-in");
  });
  if (!prefersReducedMotion()) await wait(40);
}

export async function collapseSections(elements) {
  const list = elements.filter(Boolean);
  if (!list.length) return;
  list.forEach((el) => {
    el.classList.remove("flow-in");
    el.classList.add("hidden");
  });
  if (!prefersReducedMotion()) await wait(120);
}
