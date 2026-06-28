/**
 * GSAP-orchestrated landing entrance — single timeline, no competing CSS loops.
 */
import gsap from "gsap";
import { prefersReducedMotion } from "./motion.js";

let entrancePlayed = false;

export function playLandingEntrance() {
  if (entrancePlayed) return;
  entrancePlayed = true;

  const targets = gsap.utils.toArray(".landing-motion");
  if (!targets.length) return;

  if (prefersReducedMotion()) {
    gsap.set(targets, { opacity: 1, y: 0, clearProps: "all" });
    return;
  }

  gsap.set(targets, { opacity: 0, y: 36 });

  const tl = gsap.timeline({ defaults: { ease: "power4.out" } });

  tl.to(".landing-eyebrow", { opacity: 1, y: 0, duration: 1.1 })
    .to(".landing-word", { opacity: 1, y: 0, duration: 1.2, stagger: 0.1 }, "-=0.8")
    .to(".landing-lede", { opacity: 1, y: 0, duration: 1 }, "-=0.75")
    .to(".landing-specs", { opacity: 1, y: 0, duration: 0.9 }, "-=0.8")
    .to(".landing-drop-wrap", { opacity: 1, y: 0, duration: 1.15, ease: "elastic.out(1, 0.72)" }, "-=0.7")
    .to(".landing-sample-cta", { opacity: 1, y: 0, duration: 0.95 }, "-=0.75")
    .to(".system-tests-landing", { opacity: 1, y: 0, duration: 1 }, "-=0.8")
    .to(".landing-dock", { opacity: 1, y: 0, duration: 0.9 }, "-=0.85");

  gsap.fromTo(
    ".landing-vignette",
    { opacity: 0 },
    { opacity: 1, duration: 2.2, ease: "power2.inOut" }
  );
}

export function fadeLandingOut(onComplete) {
  return gsap.to("#landing", {
    opacity: 0,
    duration: prefersReducedMotion() ? 0.15 : 0.55,
    ease: "power2.inOut",
    onComplete,
  });
}
