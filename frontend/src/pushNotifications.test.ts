import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api, post, put } from './api';
import { _setDeferredPromptForTesting, _setInstalledForTesting } from './pwaInstall';
import {
  _resetPushStateForTesting,
  disableDevicePush,
  enablePushNotifications,
  getPushAvailability,
  syncAppBadge,
  urlBase64ToUint8Array,
} from './pushNotifications';

vi.mock('./api', () => ({
  api: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
}));

const origUA = navigator.userAgent;
const origNotification = globalThis.Notification;
const origPushManager = (window as unknown as { PushManager?: unknown }).PushManager;
const origSW = Object.getOwnPropertyDescriptor(navigator, 'serviceWorker');
const origSetAppBadge = Object.getOwnPropertyDescriptor(navigator, 'setAppBadge');
const origClearAppBadge = Object.getOwnPropertyDescriptor(navigator, 'clearAppBadge');

function setUA(ua: string) {
  Object.defineProperty(navigator, 'userAgent', { value: ua, configurable: true });
}

function setNotificationMock(
  permission: NotificationPermission,
  requestPermission = vi.fn().mockResolvedValue(permission),
) {
  Object.defineProperty(globalThis, 'Notification', {
    configurable: true,
    writable: true,
    value: {
      permission,
      requestPermission,
    },
  });
  return requestPermission;
}

function removeNotificationMock() {
  Object.defineProperty(globalThis, 'Notification', {
    configurable: true,
    writable: true,
    value: undefined,
  });
}

function setPushManagerMock(enabled: boolean) {
  Object.defineProperty(window, 'PushManager', {
    configurable: true,
    writable: true,
    value: enabled ? function MockPushManager() {} : undefined,
  });
}

describe('pushNotifications', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    _resetPushStateForTesting();
    _setInstalledForTesting(null);
    _setDeferredPromptForTesting(null);
    vi.mocked(put).mockResolvedValue({ ok: true });
    vi.mocked(post).mockResolvedValue({ ok: true });
    vi.mocked(api).mockResolvedValue({ enabled: true, vapid_public_key: 'BAECAwQFBgc' });
  });

  afterEach(() => {
    setUA(origUA);
    if (origNotification === undefined) {
      delete (globalThis as { Notification?: unknown }).Notification;
    } else {
      Object.defineProperty(globalThis, 'Notification', {
        configurable: true,
        writable: true,
        value: origNotification,
      });
    }
    if (origPushManager === undefined) {
      delete (window as unknown as { PushManager?: unknown }).PushManager;
    } else {
      Object.defineProperty(window, 'PushManager', {
        configurable: true,
        writable: true,
        value: origPushManager,
      });
    }
    if (origSW) {
      Object.defineProperty(navigator, 'serviceWorker', origSW);
    } else {
      delete (navigator as { serviceWorker?: unknown }).serviceWorker;
    }
    if (origSetAppBadge) {
      Object.defineProperty(navigator, 'setAppBadge', origSetAppBadge);
    } else {
      delete (navigator as { setAppBadge?: unknown }).setAppBadge;
    }
    if (origClearAppBadge) {
      Object.defineProperty(navigator, 'clearAppBadge', origClearAppBadge);
    } else {
      delete (navigator as { clearAppBadge?: unknown }).clearAppBadge;
    }
    _resetPushStateForTesting();
    _setInstalledForTesting(null);
  });

  it('decodes URL-safe unpadded base64 VAPID keys into an ArrayBuffer-backed Uint8Array', () => {
    const bytes = urlBase64ToUint8Array('-_8BAg');
    expect(Array.from(bytes)).toEqual([0xfb, 0xff, 0x01, 0x02]);
    expect(bytes.buffer).toBeInstanceOf(ArrayBuffer);
  });

  describe('getPushAvailability', () => {
    it('returns needs_install_ios on non-standalone iOS Safari without Push APIs', () => {
      setUA('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148 Safari/604.1');
      _setInstalledForTesting(false);
      removeNotificationMock();
      setPushManagerMock(false);
      expect(getPushAvailability()).toBe('needs_install_ios');
    });

    it('reports prompt, granted, and denied on standalone iOS and Android', () => {
      setUA('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148 Safari/604.1');
      _setInstalledForTesting(true);
      setPushManagerMock(true);

      setNotificationMock('default');
      expect(getPushAvailability()).toBe('prompt');

      setNotificationMock('granted');
      expect(getPushAvailability()).toBe('granted');

      setNotificationMock('denied');
      expect(getPushAvailability()).toBe('denied');

      setUA('Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/120.0 Mobile Safari/537.36');
      _setInstalledForTesting(false);
      setNotificationMock('default');
      expect(getPushAvailability()).toBe('prompt');
    });

    it('returns unsupported when Notification API is missing on non-iOS browsers', () => {
      setUA('Mozilla/5.0 (Linux; Android 14) Chrome/120.0');
      removeNotificationMock();
      expect(getPushAvailability()).toBe('unsupported');
    });
  });

  describe('enablePushNotifications', () => {
    it('calls Notification.requestPermission before any async API or service worker call, registers SW, and subscribes', async () => {
      setUA('Mozilla/5.0 (Linux; Android 14; Pixel 8) Chrome/120.0');
      const order: string[] = [];

      const requestPermission = vi.fn().mockImplementation(async () => {
        order.push('requestPermission');
        setNotificationMock('granted', requestPermission);
        return 'granted' as NotificationPermission;
      });
      setNotificationMock('default', requestPermission);
      setPushManagerMock(true);

      vi.mocked(put).mockImplementation(async () => {
        order.push('put:/me');
        return { ok: true };
      });
      vi.mocked(api).mockImplementation(async () => {
        order.push('api:/push/config');
        return { enabled: true, vapid_public_key: 'BAECAwQFBgc' };
      });

      const subMock = {
        endpoint: 'https://fcm.googleapis.com/fcm/send/sub123',
        toJSON: () => ({
          endpoint: 'https://fcm.googleapis.com/fcm/send/sub123',
          keys: { p256dh: 'pubKey', auth: 'authSecret' },
        }),
      };
      const subscribeMock = vi.fn().mockImplementation(async () => {
        order.push('pushManager.subscribe');
        return subMock;
      });
      const regMock = {
        active: { state: 'activated' },
        pushManager: {
          subscribe: subscribeMock,
          getSubscription: vi.fn().mockResolvedValue(null),
        },
      };
      const registerMock = vi.fn().mockImplementation(async () => {
        order.push('sw.register');
        return regMock;
      });

      Object.defineProperty(navigator, 'serviceWorker', {
        configurable: true,
        value: {
          getRegistration: vi.fn().mockImplementation(async () => {
            order.push('sw.getRegistration');
            return undefined;
          }),
          register: registerMock,
          ready: Promise.resolve(regMock),
        },
      });

      vi.mocked(post).mockImplementation(async () => {
        order.push('post:/push/subscribe');
        return { ok: true };
      });

      const res = await enablePushNotifications();
      expect(res).toEqual({ ok: true, mode: 'push' });
      expect(order[0]).toBe('requestPermission');
      expect(registerMock).toHaveBeenCalledWith('/sw.js');
      expect(subscribeMock).toHaveBeenCalledWith(
        expect.objectContaining({ userVisibleOnly: true }),
      );
      expect(post).toHaveBeenCalledWith('/push/subscribe', {
        endpoint: 'https://fcm.googleapis.com/fcm/send/sub123',
        p256dh: 'pubKey',
        auth: 'authSecret',
      });
      expect(put).toHaveBeenCalledWith('/me', { push_opt_in: true });
    });

    it('unsubscribes an existing stale subscription and retries subscribe on key rotation mismatch', async () => {
      setUA('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) Safari/604.1');
      _setInstalledForTesting(true);
      setNotificationMock('granted');
      setPushManagerMock(true);

      const unsubExisting = vi.fn().mockResolvedValue(true);
      const freshSub = {
        endpoint: 'https://web.push.apple.com/Q123',
        toJSON: () => ({
          endpoint: 'https://web.push.apple.com/Q123',
          keys: { p256dh: 'applePub', auth: 'appleAuth' },
        }),
      };
      const subscribeMock = vi
        .fn()
        .mockRejectedValueOnce(new Error('InvalidStateError'))
        .mockResolvedValueOnce(freshSub);

      const regMock = {
        active: { state: 'activated' },
        pushManager: {
          subscribe: subscribeMock,
          getSubscription: vi.fn().mockResolvedValue({ unsubscribe: unsubExisting }),
        },
      };
      Object.defineProperty(navigator, 'serviceWorker', {
        configurable: true,
        value: {
          getRegistration: vi.fn().mockResolvedValue(regMock),
          ready: Promise.resolve(regMock),
        },
      });

      const res = await enablePushNotifications();
      expect(res).toEqual({ ok: true, mode: 'push' });
      expect(unsubExisting).toHaveBeenCalledTimes(1);
      expect(subscribeMock).toHaveBeenCalledTimes(2);
      expect(post).toHaveBeenCalledWith('/push/subscribe', {
        endpoint: 'https://web.push.apple.com/Q123',
        p256dh: 'applePub',
        auth: 'appleAuth',
      });
    });

    it('returns denied or dismissed without subscribing when user declines or closes dialog', async () => {
      setUA('Mozilla/5.0 (Linux; Android 14) Chrome/120.0');
      setPushManagerMock(true);

      setNotificationMock('default', vi.fn().mockResolvedValue('denied'));
      expect(await enablePushNotifications()).toEqual({ ok: false, reason: 'denied' });
      expect(post).not.toHaveBeenCalled();

      setNotificationMock('default', vi.fn().mockResolvedValue('default'));
      expect(await enablePushNotifications()).toEqual({ ok: false, reason: 'dismissed' });
      expect(post).not.toHaveBeenCalled();
    });

    it('returns needs_install_ios on non-standalone iOS without invoking requestPermission', async () => {
      setUA('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Safari/604.1');
      _setInstalledForTesting(false);
      removeNotificationMock();
      setPushManagerMock(false);

      expect(await enablePushNotifications()).toEqual({ ok: false, reason: 'needs_install_ios' });
    });

    it('falls back to local mode when serviceWorker / PushManager is unavailable', async () => {
      setUA('Mozilla/5.0 (Macintosh; Intel Mac OS X) Safari/605.1');
      setNotificationMock('default', vi.fn().mockResolvedValue('granted'));
      setPushManagerMock(false);
      delete (navigator as { serviceWorker?: unknown }).serviceWorker;

      expect(await enablePushNotifications()).toEqual({ ok: true, mode: 'local' });
      expect(put).toHaveBeenCalledWith('/me', { push_opt_in: true });
      expect(post).not.toHaveBeenCalled();
    });
  });

  it('unsubscribes and deletes the endpoint on disableDevicePush', async () => {
    const unsubMock = vi.fn().mockResolvedValue(true);
    const regMock = {
      active: { state: 'activated' },
      pushManager: {
        getSubscription: vi.fn().mockResolvedValue({
          endpoint: 'https://fcm.googleapis.com/fcm/send/old',
          unsubscribe: unsubMock,
        }),
      },
    };
    Object.defineProperty(navigator, 'serviceWorker', {
      configurable: true,
      value: {
        getRegistration: vi.fn().mockResolvedValue(regMock),
        ready: Promise.resolve(regMock),
      },
    });
    vi.mocked(api).mockResolvedValue({ ok: true });

    await expect(disableDevicePush()).resolves.toBe(true);
    expect(unsubMock).toHaveBeenCalledTimes(1);
    expect(api).toHaveBeenCalledWith('/push/subscribe', {
      method: 'DELETE',
      body: { endpoint: 'https://fcm.googleapis.com/fcm/send/old' },
    });
  });

  it('updates and clears app icon badges via syncAppBadge', () => {
    const setAppBadge = vi.fn().mockResolvedValue(undefined);
    const clearAppBadge = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'setAppBadge', { configurable: true, value: setAppBadge });
    Object.defineProperty(navigator, 'clearAppBadge', { configurable: true, value: clearAppBadge });

    syncAppBadge(3);
    expect(setAppBadge).toHaveBeenCalledWith(3);

    syncAppBadge(0);
    syncAppBadge(null);
    expect(clearAppBadge).toHaveBeenCalledTimes(2);

    Object.defineProperty(navigator, 'clearAppBadge', { configurable: true, value: undefined });
    syncAppBadge(0);
    expect(setAppBadge).toHaveBeenLastCalledWith(0);
  });

  it('reads current subscription endpoint and returns subscribe_failed when PushManager throws', async () => {
    setUA('Mozilla/5.0 (Linux; Android 14) Chrome/120.0');
    _setInstalledForTesting(true);
    setPushManagerMock(true);

    const requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(globalThis, 'Notification', {
      configurable: true,
      writable: true,
      value: { permission: 'granted', requestPermission },
    });

    const subscribe = vi.fn().mockRejectedValue(new Error('GCM error'));
    const regMock = {
      active: { state: 'activated' },
      pushManager: {
        getSubscription: vi.fn().mockResolvedValue({ endpoint: 'https://fcm.googleapis.com/fcm/send/dev-1' }),
        subscribe,
      },
    };
    Object.defineProperty(navigator, 'serviceWorker', {
      configurable: true,
      value: {
        getRegistration: vi.fn().mockResolvedValue(regMock),
        ready: Promise.resolve(regMock),
      },
    });
    vi.mocked(api).mockResolvedValue({
      enabled: true,
      configured: true,
      vapid_public_key: 'BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8',
    });

    const { getCurrentPushEndpoint } = await import('./pushNotifications');
    await expect(getCurrentPushEndpoint()).resolves.toBe('https://fcm.googleapis.com/fcm/send/dev-1');

    // No active sub + failing subscribe() => structured subscribe_failed
    regMock.pushManager.getSubscription.mockResolvedValue(null);
    await expect(enablePushNotifications()).resolves.toEqual({ ok: false, reason: 'subscribe_failed' });
  });
});
