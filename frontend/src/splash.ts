// Boot splash: #splash in index.html plays the brand loading animation from first paint,
// before any JavaScript runs (the CSP forbids inline scripts, so it is pure HTML + CSS).
// The app calls hideSplash() once it knows what to show; lazy pages hold it (holdSplash) until
// their code has loaded, so the splash hands straight over to real content instead of a spinner.

/** The mark has landed and the wordmark is fully revealed by ~2.75s into the animation. */
const SPLASH_MIN_MS = 2800;
/** Keep in sync with the #splash transition in index.html. */
const FADE_MS = 500;

let holds = 0;
let wanted = false;

function leave() {
  const el = document.getElementById('splash');
  if (!el || el.dataset.leaving) return;
  el.dataset.leaving = 'true';
  const reduce = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  // performance.now() counts from navigation start, which is when the splash began animating.
  const wait = reduce ? 0 : Math.max(0, SPLASH_MIN_MS - performance.now());
  window.setTimeout(() => {
    // Two frames so the app underneath has painted before the cross-fade starts.
    requestAnimationFrame(() => requestAnimationFrame(() => {
      el.classList.add('is-done');
      window.setTimeout(() => el.remove(), reduce ? 0 : FADE_MS);
    }));
  }, wait);
}

/** Fade out and remove the boot splash, letting its intro finish first. Safe to call repeatedly.
 *  Waits for any holds to be released unless `force` is set. */
export function hideSplash(force = false) {
  wanted = true;
  if (force || holds === 0) leave();
}

/** Keep the splash up while something it should hand over to is still loading. Returns the release. */
export function holdSplash(): () => void {
  if (!document.getElementById('splash')) return () => {};
  holds += 1;
  let released = false;
  return () => {
    if (released) return;
    released = true;
    holds -= 1;
    // Defer a tick so a fallback that unmounts as the page mounts doesn't hide before the page paints.
    if (holds === 0 && wanted) window.setTimeout(() => holds === 0 && leave(), 0);
  };
}
