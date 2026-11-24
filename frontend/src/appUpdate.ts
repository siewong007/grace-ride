import { useEffect, useState } from 'react';

/**
 * Detects that a newer build of the app is live and lets the user switch to it on their
 * own terms. Two signals: a service-worker update waiting to activate (sw.js changed), and
 * /version.json reporting a different build id than the one this page was built with
 * (every deploy, even when sw.js itself did not change).
 */

export const UPDATE_EVENT = 'gr-update-available';
const CHECK_INTERVAL_MS = 30 * 60_000;
const MIN_RECHECK_MS = 5 * 60_000;

let available = false;
let registration: ServiceWorkerRegistration | null = null;
let lastCheck = 0;

function announce() {
  if (available) return;
  available = true;
  window.dispatchEvent(new Event(UPDATE_EVENT));
}

export function isUpdateAvailable(): boolean {
  return available;
}

/** Compare the deployed build id with ours; network or parse failures are ignored. */
export async function checkForNewBuild(currentBuild: string = __APP_BUILD_ID__): Promise<boolean> {
  lastCheck = Date.now();
  try {
    const res = await fetch('/version.json', { cache: 'no-store', credentials: 'same-origin' });
    if (!res.ok) return false;
    const body = (await res.json()) as { build?: unknown };
    if (typeof body.build === 'string' && body.build && body.build !== currentBuild) {
      announce();
      return true;
    }
  } catch {
    /* offline, or the SPA fallback answered with HTML */
  }
  return false;
}

function watchRegistration(reg: ServiceWorkerRegistration) {
  registration = reg;
  // Only an *update* counts: with no controller this is the very first install.
  if (reg.waiting && navigator.serviceWorker.controller) announce();
  reg.addEventListener('updatefound', () => {
    const installing = reg.installing;
    if (!installing) return;
    installing.addEventListener('statechange', () => {
      if (installing.state === 'installed' && navigator.serviceWorker.controller) announce();
    });
  });
}

/** Register the service worker and start watching for new deploys (production only). */
export function startUpdateWatch(): void {
  if (typeof window === 'undefined') return;
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker
      .register('/sw.js')
      .then(watchRegistration)
      .catch(() => {});
  }
  const recheck = () => {
    if (available || Date.now() - lastCheck < MIN_RECHECK_MS) return;
    void checkForNewBuild();
    void registration?.update().catch(() => {});
  };
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') recheck();
  });
  window.setInterval(recheck, CHECK_INTERVAL_MS);
  void checkForNewBuild();
}

/** Activate the waiting service worker (if any) and reload into the new version. */
export function applyUpdate(): void {
  const waiting = registration?.waiting;
  if (!waiting || !('serviceWorker' in navigator)) {
    window.location.reload();
    return;
  }
  let reloaded = false;
  const reload = () => {
    if (reloaded) return;
    reloaded = true;
    window.location.reload();
  };
  navigator.serviceWorker.addEventListener('controllerchange', reload, { once: true });
  waiting.postMessage({ type: 'SKIP_WAITING' });
  // If the worker never takes control (e.g. it was replaced meanwhile), reload anyway.
  window.setTimeout(reload, 3000);
}

export function useUpdateAvailable(): boolean {
  const [state, setState] = useState(available);
  useEffect(() => {
    const on = () => setState(true);
    window.addEventListener(UPDATE_EVENT, on);
    if (available) setState(true);
    return () => window.removeEventListener(UPDATE_EVENT, on);
  }, []);
  return state;
}

/** Test hook: reset module state between tests. */
export function __resetUpdateStateForTests(): void {
  available = false;
  registration = null;
  lastCheck = 0;
}
