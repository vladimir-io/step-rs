/** Progress bar driver. Respects prefers-reduced-motion. */

export function wait(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

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
