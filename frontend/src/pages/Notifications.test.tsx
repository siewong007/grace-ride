import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../i18n';
import { api, post, put } from '../api';
import { _setDeferredPromptForTesting, _setInstalledForTesting } from '../pwaInstall';
import { _resetPushStateForTesting } from '../pushNotifications';
import Notifications from './Notifications';

const useApiPaths: string[] = [];
let mockUseApiData: unknown = {
  items: [
    {
      id: 101,
      title: 'Ride confirmed',
      body: 'Driver on the way',
      link: '/rides/10',
      read: false,
      category: 'ride',
      priority: 'urgent',
      created_at: new Date().toISOString().slice(0, 19),
    },
  ],
  next_before: null,
  unread: 1,
};

vi.mock('../api', () => ({
  api: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  useApi: (path: string | null) => {
    if (path) useApiPaths.push(path);
    return {
      data: mockUseApiData,
      loading: false,
      error: null,
      reload: vi.fn(),
    };
  },
  subscribe: () => () => {},
}));

let mockMe: {
  id: number;
  name: string;
  status: 'active' | 'pending' | 'disabled';
  push_opt_in?: boolean;
} | null = {
  id: 1,
  name: 'Grace Member',
  status: 'active',
  push_opt_in: true,
};
const refreshMock = vi.fn().mockResolvedValue(undefined);

vi.mock('../auth', () => ({
  useAuth: () => ({
    me: mockMe,
    can: () => true,
    refresh: refreshMock,
  }),
}));

const origUA = navigator.userAgent;
const origNotification = globalThis.Notification;
const origPushManager = (window as unknown as { PushManager?: unknown }).PushManager;
const origSW = Object.getOwnPropertyDescriptor(navigator, 'serviceWorker');

function setUA(ua: string) {
  Object.defineProperty(navigator, 'userAgent', { value: ua, configurable: true });
}

function setNotification(
  permission: NotificationPermission | undefined,
  requestPermission = vi.fn(),
) {
  Object.defineProperty(globalThis, 'Notification', {
    configurable: true,
    writable: true,
    value: permission === undefined ? undefined : { permission, requestPermission },
  });
}

function setPushManager(enabled: boolean) {
  Object.defineProperty(window, 'PushManager', {
    configurable: true,
    writable: true,
    value: enabled ? function MockPushManager() {} : undefined,
  });
}

function renderNotifications() {
  return render(
    <MemoryRouter initialEntries={['/notifications']}>
      <Notifications />
    </MemoryRouter>,
  );
}

describe('Notifications page PWA push prompt', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en');
    vi.clearAllMocks();
    useApiPaths.length = 0;
    mockUseApiData = { items: [], next_before: null };
    sessionStorage.clear();
    _resetPushStateForTesting();
    _setInstalledForTesting(null);
    _setDeferredPromptForTesting(null);
    mockMe = { id: 1, name: 'Grace Member', status: 'active', push_opt_in: true };
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
    _resetPushStateForTesting();
    _setInstalledForTesting(null);
  });

  it('prompts inside installed PWA and enables push when user taps Allow', async () => {
    setUA('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Safari/604.1');
    _setInstalledForTesting(true);
    setPushManager(true);

    const requestPermission = vi.fn().mockImplementation(async () => {
      setNotification('granted', requestPermission);
      return 'granted' as NotificationPermission;
    });
    setNotification('default', requestPermission);

    const regMock = {
      active: { state: 'activated' },
      pushManager: {
        getSubscription: vi.fn().mockResolvedValue(null),
        subscribe: vi.fn().mockResolvedValue({
          endpoint: 'https://web.push.apple.com/Qabc',
          toJSON: () => ({
            endpoint: 'https://web.push.apple.com/Qabc',
            keys: { p256dh: 'k1', auth: 'a1' },
          }),
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

    renderNotifications();
    const enableBtn = screen.getByRole('button', { name: 'Enable Notifications' });
    expect(enableBtn).toBeInTheDocument();

    fireEvent.click(enableBtn);

    await waitFor(() => expect(requestPermission).toHaveBeenCalledTimes(1));
    await waitFor(() =>
      expect(post).toHaveBeenCalledWith('/push/subscribe', {
        endpoint: 'https://web.push.apple.com/Qabc',
        p256dh: 'k1',
        auth: 'a1',
      }),
    );
    expect(await screen.findByText('Notifications enabled on this device.')).toBeInTheDocument();
  });

  it('shows install-first guidance on iOS Safari and opens setup modal', () => {
    setUA('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Safari/604.1');
    _setInstalledForTesting(false);
    setNotification(undefined);
    setPushManager(false);

    renderNotifications();
    expect(screen.getByText('Add Grace Ride to Home Screen for notifications')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Add to Home Screen' }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Add App to Home Screen (No App Store Needed)')).toBeInTheDocument();
  });

  it('shows platform Settings instructions and hides Enable button when permission is denied', () => {
    setUA('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Safari/604.1');
    _setInstalledForTesting(true);
    setPushManager(true);
    setNotification('denied');

    renderNotifications();
    expect(screen.getByText('Notifications are turned off')).toBeInTheDocument();
    expect(screen.getByText(/iPhone Settings → Notifications → Grace Ride/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Enable Notifications' })).not.toBeInTheDocument();
  });

  it('hides banner when already granted & subscribed, when opted out, or when dismissed', async () => {
    setUA('Mozilla/5.0 (Linux; Android 14) Chrome/120.0');
    _setInstalledForTesting(true);
    setPushManager(true);
    setNotification('granted');

    const regMock = {
      active: { state: 'activated' },
      pushManager: {
        getSubscription: vi.fn().mockResolvedValue({ endpoint: 'https://fcm.googleapis.com/fcm/send/ok' }),
      },
    };
    Object.defineProperty(navigator, 'serviceWorker', {
      configurable: true,
      value: {
        getRegistration: vi.fn().mockResolvedValue(regMock),
        ready: Promise.resolve(regMock),
      },
    });

    const first = renderNotifications();
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: 'Enable Notifications' })).not.toBeInTheDocument();
    });
    first.unmount();

    // Opted out user
    setNotification('default');
    mockMe = { id: 1, name: 'Grace Member', status: 'active', push_opt_in: false };
    const second = renderNotifications();
    expect(screen.queryByRole('button', { name: 'Enable Notifications' })).not.toBeInTheDocument();
    second.unmount();

    // Dismiss via "Not now" persists in sessionStorage
    mockMe = { id: 1, name: 'Grace Member', status: 'active', push_opt_in: true };
    const third = renderNotifications();
    fireEvent.click(screen.getByRole('button', { name: 'Not now' }));
    expect(sessionStorage.getItem('gr_notif_prompt_dismissed')).toBe('1');
    expect(screen.queryByRole('button', { name: 'Enable Notifications' })).not.toBeInTheDocument();
    third.unmount();
  });
});

describe('Notifications filters, actions and subscribe_failed', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en');
    vi.clearAllMocks();
    useApiPaths.length = 0;
    sessionStorage.clear();
    _resetPushStateForTesting();
    _setInstalledForTesting(true);
    _setDeferredPromptForTesting(null);
    mockMe = { id: 1, name: 'Grace Member', status: 'active', push_opt_in: true };
    mockUseApiData = {
      items: [
        {
          id: 101,
          title: 'Ride confirmed',
          body: 'Driver on the way',
          link: '/rides/10',
          read: false,
          category: 'ride',
          priority: 'urgent',
          created_at: new Date().toISOString().slice(0, 19),
        },
        {
          id: 102,
          title: 'Event published',
          body: 'Sunday worship',
          link: '/events/4',
          read: true,
          category: 'event',
          priority: 'normal',
          created_at: '2026-01-01 08:00:00',
        },
      ],
      next_before: null,
      unread: 1,
    };
    vi.mocked(post).mockResolvedValue({ ok: true, unread: 0 });
    vi.mocked(api).mockResolvedValue({ ok: true, unread: 0 });
  });

  afterEach(() => {
    Object.defineProperty(navigator, 'userAgent', { value: origUA, configurable: true });
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
    if (origSW) Object.defineProperty(navigator, 'serviceWorker', origSW);
  });

  it('requests filtered endpoints when toggling Unread and category chips, and triggers toggleRead / delete', async () => {
    renderNotifications();

    expect(useApiPaths[0]).toBe('/notifications');
    fireEvent.click(screen.getByRole('button', { name: 'Unread' }));
    expect(useApiPaths).toContain('/notifications?limit=30&unread_only=true');

    fireEvent.click(screen.getByRole('button', { name: 'Rides' }));
    expect(useApiPaths).toContain('/notifications?limit=30&unread_only=true&category=ride');

    fireEvent.click(screen.getByRole('button', { name: 'All' }));

    fireEvent.click(screen.getByRole('button', { name: 'Mark as read' }));
    await waitFor(() => expect(post).toHaveBeenCalledWith('/notifications/101/read'));

    fireEvent.click(screen.getAllByRole('button', { name: 'Mark as unread' })[1]);
    await waitFor(() => expect(post).toHaveBeenCalledWith('/notifications/102/unread'));

    const removeButtons = screen.getAllByRole('button', { name: 'Remove notification' });
    fireEvent.click(removeButtons[0]);
    await waitFor(() =>
      expect(api).toHaveBeenCalledWith('/notifications/101', { method: 'DELETE' }),
    );
  });

  it('surfaces subscribe_failed message when PushManager subscription fails', async () => {
    setUA('Mozilla/5.0 (Linux; Android 14) Chrome/120.0');
    setPushManager(true);
    setNotification('default', vi.fn().mockResolvedValue('granted'));
    const regMock = {
      active: { state: 'activated' },
      pushManager: {
        getSubscription: vi.fn().mockResolvedValue(null),
        subscribe: vi.fn().mockRejectedValue(new Error('GCM error')),
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
      vapid_public_key: 'BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8',
    });

    renderNotifications();
    fireEvent.click(screen.getByRole('button', { name: 'Enable Notifications' }));
    expect(
      await screen.findByText(/Could not complete push subscription on this browser/i),
    ).toBeInTheDocument();
  });
});
