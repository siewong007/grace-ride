// Boot splash: #splash in index.html plays the brand loading animation from first paint,
// before any JavaScript runs (the CSP forbids inline scripts, so it is pure HTML + CSS).
// The app calls hideSplash() once it knows what to show.

/** The mark has landed and the wordmark is fully revealed by ~2.75s into the animation. */
const SPLASH_MIN_MS = 2800;
const FADE_MS = 300;

/** Fade out and remove the boot splash, letting its intro finish first. Safe to call repeatedly. */
export function hideSplash() {
  const el = document.getElementById('splash');
  if (!el || el.dataset.leaving) return;
  el.dataset.leaving = 'true';
  const reduce = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  // performance.now() counts from navigation start, which is when the splash began animating.
  const wait = reduce ? 0 : Math.max(0, SPLASH_MIN_MS - performance.now());
  window.setTimeout(() => {
    el.classList.add('is-done');
    window.setTimeout(() => el.remove(), reduce ? 0 : FADE_MS);
  }, wait);
}
