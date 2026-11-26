// Shared Web Push subscription, permission state, and app icon badge helpers.
// Keeps iOS Safari's user-gesture requirement intact by invoking
// Notification.requestPermission() synchronously inside tap handlers before any await.

import { useEffect, useState } from 'react';
import { api, post, put } from './api';
import { clearClientSessionStorage } from './offlineSnapshot';
import { detectPlatform, isStandalone, PwaPlatform } from './pwaInstall';

export type PushAvailability =
  | 'needs_install_ios'
  | 'unsupported'
  | 'denied'
  | 'granted'
  | 'prompt';

export type EnablePushResult =
  | { ok: true; mode: 'push' | 'local' }
  | { ok: false; reason: 'needs_install_ios' | 'unsupported' | 'denied' | 'dismissed' | 'subscribe_failed' };

export interface PushNotificationState {
  availability: PushAvailability;
  permission: NotificationPermission | 'unsupported';
  isSubscribed: boolean | null;
  isStandalone: boolean;
  platform: PwaPlatform;
}

const listeners = new Set<() => void>();
let knownSubscribed: boolean | null = null;

function notifyPushListeners() {
  for (const listener of listeners) {
    listener();
  }
}

export function urlBase64ToUint8Array(base64String: string): Uint8Array<ArrayBuffer> {
  const pad = '='.repeat((4 - (base64String.length % 4)) % 4);
  const b64 = (base64String + pad).replace(/-/g, '+').replace(/_/g, '/');
  const raw = atob(b64);
  const buffer = new ArrayBuffer(raw.length);
  const bytes = new Uint8Array(buffer);
  for (let i = 0; i < raw.length; i++) {
    bytes[i] = raw.charCodeAt(i);
  }
  return bytes;
}

export function getNotificationPermission(): NotificationPermission | 'unsupported' {
  if (typeof Notification === 'undefined') return 'unsupported';
  return Notification.permission;
}

export function getPushAvailability(): PushAvailability {
  const platform = detectPlatform();
  const standalone = isStandalone();
  if (
    platform === 'ios' &&
    !standalone &&
    (typeof Notification === 'undefined' ||
      typeof window === 'undefined' ||
      !('PushManager' in window))
  ) {
    return 'needs_install_ios';
  }
  if (typeof Notification === 'undefined') return 'unsupported';
  if (Notification.permission === 'granted') return 'granted';
  if (Notification.permission === 'denied') return 'denied';
  return 'prompt';
}

function hasPushManagerSupport(): boolean {
  return (
    typeof navigator !== 'undefined' &&
    'serviceWorker' in navigator &&
    typeof window !== 'undefined' &&
    'PushManager' in window
  );
}

export async function ensureServiceWorkerReady(
  timeoutMs = 4000,
): Promise<ServiceWorkerRegistration | null> {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) {
    return null;
  }
  let reg: ServiceWorkerRegistration | undefined;
  try {
    reg = await navigator.serviceWorker.getRegistration?.();
  } catch {
    reg = undefined;
  }
  if (!reg && typeof navigator.serviceWorker.register === 'function') {
    try {
      reg = await navigator.serviceWorker.register('/sw.js');
    } catch {
      reg = undefined;
    }
  }
  if (reg?.active) {
    return reg;
  }
  if (navigator.serviceWorker.ready) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const readyReg = await Promise.race<ServiceWorkerRegistration | null>([
        navigator.serviceWorker.ready,
        new Promise<null>((resolve) => {
          timer = setTimeout(() => resolve(null), timeoutMs);
        }),
      ]);
      if (readyReg) return readyReg;
    } catch {
      // Fall back to registration object below.
    } finally {
      if (timer !== undefined) clearTimeout(timer);
    }
  }
  return reg ?? null;
}

export async function getCurrentPushSubscription(): Promise<PushSubscription | null> {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) {
    return null;
  }
  try {
    const reg = await navigator.serviceWorker.getRegistration?.();
    return (await reg?.pushManager?.getSubscription?.()) ?? null;
  } catch {
    return null;
  }
}

export async function getCurrentPushEndpoint(): Promise<string> {
  const sub = await getCurrentPushSubscription();
  return sub?.endpoint ?? '';
}

export function resetPushState(): void {
  knownSubscribed = null;
  notifyPushListeners();
}

export async function signOutClient(opts: { allDevices?: boolean } = {}): Promise<void> {
  let endpoint = '';
  const sw =
    typeof navigator !== 'undefined' &&
    typeof navigator.serviceWorker?.getRegistration === 'function'
      ? navigator.serviceWorker
      : undefined;
  if (sw) {
    try {
      const reg = await Promise.race([
        sw.getRegistration(),
        new Promise<undefined>((resolve) => setTimeout(() => resolve(undefined), 1500)),
      ]);
      const sub = await reg?.pushManager?.getSubscription?.();
      if (sub) {
        endpoint = sub.endpoint;
        await sub.unsubscribe().catch(() => {});
      }
    } catch {
      /* ignore push teardown errors */
    }
  }
  const path = opts.allDevices ? '/auth/logout-all' : '/auth/logout';
  try {
    await (endpoint ? post(path, { push_endpoint: endpoint }) : post(path));
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem('gr_pending_logout');
    }
  } catch (err) {
    const isHttpErr =
      typeof err === 'object' &&
      err !== null &&
      'status' in err &&
      typeof (err as { status?: unknown }).status === 'number';
    if (!isHttpErr && typeof localStorage !== 'undefined') {
      localStorage.setItem('gr_pending_logout', '1');
    }
  } finally {
    clearClientSessionStorage();
    resetPushState();
    syncAppBadge(0);
  }
}

if (typeof window !== 'undefined') {
  window.addEventListener('gr:push-reset', () => {
    resetPushState();
  });
}

export async function enablePushNotifications(): Promise<EnablePushResult> {
  const avail = getPushAvailability();
  if (avail === 'needs_install_ios') {
    return { ok: false, reason: 'needs_install_ios' };
  }
  if (avail === 'unsupported') {
    return { ok: false, reason: 'unsupported' };
  }
  if (avail === 'denied') {
    return { ok: false, reason: 'denied' };
  }

  if (avail !== 'granted') {
    // Must be the first await so iOS Safari recognizes the user tap gesture.
    const perm = await Notification.requestPermission();
    notifyPushListeners();
    if (perm !== 'granted') {
      return { ok: false, reason: perm === 'denied' ? 'denied' : 'dismissed' };
    }
  }

  await put('/me', { push_opt_in: true }).catch(() => {});

  const pushSupported = hasPushManagerSupport();
  if (!pushSupported) {
    notifyPushListeners();
    return { ok: true, mode: 'local' };
  }

  try {
    const cfg = await api<{ enabled?: boolean; vapid_public_key?: string }>('/push/config');
    const reg = await ensureServiceWorkerReady();
    if (cfg?.vapid_public_key && reg?.pushManager) {
      const keyBytes = urlBase64ToUint8Array(cfg.vapid_public_key);
      let sub: PushSubscription;
      try {
        sub = await reg.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: keyBytes,
        });
      } catch (err) {
        const existing = await reg.pushManager.getSubscription?.();
        if (existing) {
          await existing.unsubscribe();
          sub = await reg.pushManager.subscribe({
            userVisibleOnly: true,
            applicationServerKey: keyBytes,
          });
        } else {
          throw err;
        }
      }
      const json = sub.toJSON();
      if (json.endpoint) {
        await post('/push/subscribe', {
          endpoint: json.endpoint,
          p256dh: json.keys?.p256dh ?? '',
          auth: json.keys?.auth ?? '',
        });
        knownSubscribed = true;
        notifyPushListeners();
        return { ok: true, mode: 'push' };
      }
    }
  } catch {
    // Fall through to subscribe_failed below.
  }

  notifyPushListeners();
  return { ok: false, reason: 'subscribe_failed' };
}

export async function disableDevicePush(): Promise<boolean> {
  let endpoint = '';
  const reg = await ensureServiceWorkerReady(2000);
  const sub = await reg?.pushManager?.getSubscription?.();
  if (sub) {
    endpoint = sub.endpoint;
    await sub.unsubscribe();
  }
  if (endpoint) {
    await api('/push/subscribe', { method: 'DELETE', body: { endpoint } });
  }
  knownSubscribed = false;
  notifyPushListeners();
  return true;
}

type BadgingNavigator = Navigator & {
  setAppBadge?: (contents?: number) => Promise<void>;
  clearAppBadge?: () => Promise<void>;
};

export function syncAppBadge(unreadCount: number | null | undefined): void {
  if (typeof navigator === 'undefined') return;
  const nav = navigator as BadgingNavigator;
  const count =
    typeof unreadCount === 'number' && Number.isFinite(unreadCount)
      ? Math.max(0, Math.floor(unreadCount))
      : 0;
  if (count > 0 && typeof nav.setAppBadge === 'function') {
    void Promise.resolve(nav.setAppBadge(count)).catch(() => {});
  } else if (typeof nav.clearAppBadge === 'function') {
    void Promise.resolve(nav.clearAppBadge()).catch(() => {});
  } else if (typeof nav.setAppBadge === 'function') {
    void Promise.resolve(nav.setAppBadge(0)).catch(() => {});
  }
}

function readSnapshot(prevSubscribed: boolean | null): PushNotificationState {
  const availability = getPushAvailability();
  return {
    availability,
    permission: getNotificationPermission(),
    isSubscribed:
      availability !== 'granted'
        ? false
        : knownSubscribed !== null
          ? knownSubscribed
          : prevSubscribed,
    isStandalone: isStandalone(),
    platform: detectPlatform(),
  };
}

export function usePushNotifications(): PushNotificationState {
  const [state, setState] = useState<PushNotificationState>(() => readSnapshot(null));

  useEffect(() => {
    let cancelled = false;
    let permStatus: PermissionStatus | null = null;

    const refreshSub = () => {
      const avail = getPushAvailability();
      if (avail !== 'granted') {
        if (!cancelled) {
          setState(readSnapshot(false));
        }
        return;
      }
      if (!hasPushManagerSupport()) {
        if (!cancelled) {
          setState(readSnapshot(null));
        }
        return;
      }
      void getCurrentPushSubscription().then((sub) => {
        if (cancelled) return;
        const subscribed = sub ? true : knownSubscribed === true;
        setState({
          availability: getPushAvailability(),
          permission: getNotificationPermission(),
          isSubscribed: subscribed,
          isStandalone: isStandalone(),
          platform: detectPlatform(),
        });
      });
    };

    const update = () => {
      if (cancelled) return;
      setState((prev) => readSnapshot(prev.isSubscribed));
      refreshSub();
    };

    listeners.add(update);
    refreshSub();

    const onVisibility = () => {
      if (document.visibilityState === 'visible') update();
    };
    document.addEventListener('visibilitychange', onVisibility);

    const mql =
      typeof window !== 'undefined'
        ? window.matchMedia?.('(display-mode: standalone)')
        : undefined;
    mql?.addEventListener?.('change', update);

    if (typeof navigator !== 'undefined' && navigator.permissions?.query) {
      try {
        void navigator.permissions
          .query({ name: 'notifications' as PermissionName })
          .then((status) => {
            if (cancelled) return;
            permStatus = status;
            status.onchange = update;
          })
          .catch(() => {});
      } catch {
        // Permissions API for notifications unsupported on this browser.
      }
    }

    return () => {
      cancelled = true;
      listeners.delete(update);
      document.removeEventListener('visibilitychange', onVisibility);
      mql?.removeEventListener?.('change', update);
      if (permStatus) permStatus.onchange = null;
    };
  }, []);

  return state;
}

export function _resetPushStateForTesting(): void {
  knownSubscribed = null;
}
