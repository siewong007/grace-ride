import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import CarePass from './pages/CarePass';
import Profile from './pages/Profile';
import Rides from './pages/Rides';
import { ConfirmProvider, ToastProvider } from './components/ui';
import { todayStr } from './format';
import { saveTodayRidesSnapshot } from './offlineSnapshot';
import type { Ride } from './components/RideCard';

const mocks = vi.hoisted(() => ({
  api: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  del: vi.fn(),
  reload: vi.fn(),
  refresh: vi.fn(),
  lock: vi.fn(),
  useApiData: new Map<string, unknown>(),
  useApiErrors: new Map<string, string>(),
}));

let mockMe: {
  id: number;
  name: string;
  email: string;
  phone: string;
  photo_file_id: number | null;
  roles: string[];
  role_names: Record<string, string>;
  permissions: string[];
  status: 'active' | 'pending' | 'disabled';
  terms_accepted: boolean;
  mfa_ok: boolean;
  totp_enabled: boolean;
  has_password: boolean;
  passkey_count: number;
  badges_opt_out: boolean;
  readiness_prompt_mins: number;
  auto_lock_mins: number;
  session_timeout_mins: number;
  backup_codes_count: number;
  locked: boolean;
  telegram_chat_id: string;
  whatsapp_opt_in: boolean;
  push_opt_in: boolean;
  caregiver_user_id: number | null;
  privileged: boolean;
  unread: number;
  groups: { id: number; name: string }[];
  dev_mode: boolean;
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: mockMe,
    loading: false,
    can: () => true,
    refresh: mocks.refresh,
  }),
}));

vi.mock('./components/AutoLock', () => ({
  useAutoLock: () => ({ isLocked: false, lock: mocks.lock, unlock: vi.fn() }),
  clearAutoLock: vi.fn(),
}));

vi.mock('./components/LiveMap', () => ({
  default: () => <div data-testid="live-map" />,
}));

vi.mock('./api', () => ({
  api: (...a: unknown[]) => mocks.api(...a),
  post: (...a: unknown[]) => mocks.post(...a),
  put: (...a: unknown[]) => mocks.put(...a),
  del: (...a: unknown[]) => mocks.del(...a),
  useApi: (pathStr: string | null) => ({
    data: pathStr ? (mocks.useApiData.get(pathStr) ?? null) : null,
    loading: false,
    error: pathStr ? (mocks.useApiErrors.get(pathStr) ?? null) : null,
    reload: mocks.reload,
  }),
  subscribe: () => () => {},
}));

function TestWrap({ children, route = '/' }: { children: ReactNode; route?: string }) {
  return (
    <MemoryRouter initialEntries={[route]}>
      <ToastProvider>
        <ConfirmProvider>{children}</ConfirmProvider>
      </ToastProvider>
    </MemoryRouter>
  );
}

describe('Scenario Matrix — Frontend Components & Service Worker', () => {
  beforeEach(async () => {
    localStorage.clear();
    await i18n.changeLanguage('en');
    vi.clearAllMocks();
    mocks.useApiData.clear();
    mocks.useApiErrors.clear();
    mocks.api.mockResolvedValue({ items: [], next_before: null });
    mocks.post.mockResolvedValue({ ok: true });
    mocks.put.mockResolvedValue({ ok: true });
    mocks.del.mockResolvedValue({ ok: true });
    mockMe = {
      id: 7,
      name: 'Sister Ruth',
      email: 'ruth@example.org',
      phone: '+60123456789',
      photo_file_id: null,
      roles: ['member'],
      role_names: {},
      permissions: ['rides.view', 'rides.create', 'costs.view', 'reimbursements.view', 'report.view', 'help.view'],
      status: 'active',
      terms_accepted: true,
      mfa_ok: true,
      totp_enabled: false,
      has_password: false,
      passkey_count: 0,
      badges_opt_out: false,
      readiness_prompt_mins: 45,
      auto_lock_mins: 0,
      session_timeout_mins: 15,
      backup_codes_count: 0,
      locked: false,
      telegram_chat_id: '',
      whatsapp_opt_in: false,
      push_opt_in: true,
      caregiver_user_id: null,
      privileged: false,
      unread: 0,
      groups: [{ id: 1, name: 'Youth' }],
      dev_mode: true,
    };
  });

  describe('CarePass state matrix', () => {
    const baseCare = {
      ride_id: 77,
      label: 'Family View',
      rider_name: 'Uncle Tan',
      booker_name: 'Sister Mei',
      status: 'in_progress',
      leg: 'outbound' as 'outbound' | 'return',
      pickup_at: `${todayStr()}T09:00:00`,
      area: { id: 1, name: 'Petaling Jaya' },
      destination: 'PJ State CMC',
      readiness_status: 'unconfirmed',
      delay_minutes: 0,
      arrived_at_gate_at: null as string | null,
      started_at: `${todayStr()}T08:45:00`,
      picked_up_at: null as string | null,
      completed_at: null as string | null,
      driver: { name: 'Brother Silas', vehicle: { model: 'Myvi', color: 'White', plate: 'VXX 1234' } },
      location: { sharing: true, lat: 3.1, lng: 101.6, updated_at: `${todayStr()}T08:50:00` },
    };

    it('renders closed state cleanly when link expired or revoked', () => {
      mocks.useApiErrors.set('/care/expiredtok', 'Care pass has closed');
      render(
        <MemoryRouter initialEntries={['/care/expiredtok']}>
          <Routes>
            <Route path="/care/:token" element={<CarePass />} />
          </Routes>
        </MemoryRouter>,
      );

      expect(screen.getByRole('heading', { name: /safe-arrival link closed/i })).toBeInTheDocument();
      expect(screen.getByText('Care pass has closed')).toBeInTheDocument();
    });

    it('hides readiness controls and map once completed or rider is aboard', () => {
      mocks.useApiData.set('/care/done', {
        ...baseCare,
        status: 'completed',
        picked_up_at: `${todayStr()}T09:02:00`,
        completed_at: `${todayStr()}T09:28:00`,
      });
      const { unmount } = render(
        <MemoryRouter initialEntries={['/care/done']}>
          <Routes>
            <Route path="/care/:token" element={<CarePass />} />
          </Routes>
        </MemoryRouter>,
      );

      expect(screen.getByText(/arrived safely at pj state cmc/i)).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /uncle tan is ready/i })).toBeNull();
      expect(screen.queryByTestId('live-map')).toBeNull();
      unmount();

      mocks.useApiData.set('/care/aboard', {
        ...baseCare,
        status: 'in_progress',
        picked_up_at: `${todayStr()}T09:02:00`,
      });
      render(
        <MemoryRouter initialEntries={['/care/aboard']}>
          <Routes>
            <Route path="/care/:token" element={<CarePass />} />
          </Routes>
        </MemoryRouter>,
      );
      expect(screen.getByText(/picked up — on the way to pj state cmc/i)).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /uncle tan is ready/i })).toBeNull();
    });

    it('renders arrived-at-gate, delay warning, stale GPS warning, and return leg label', () => {
      const staleIso = new Date(Date.now() - 12 * 60_000).toISOString().slice(0, 19);
      mocks.useApiData.set('/care/alerts', {
        ...baseCare,
        leg: 'return',
        arrived_at_gate_at: staleIso,
        delay_minutes: 10,
        location: { sharing: true, lat: 3.1, lng: 101.6, updated_at: staleIso },
      });

      render(
        <MemoryRouter initialEntries={['/care/alerts']}>
          <Routes>
            <Route path="/care/:token" element={<CarePass />} />
          </Routes>
        </MemoryRouter>,
      );

      expect(screen.getByText(/return leg/i)).toBeInTheDocument();
      expect(screen.getByText(/driver is at your pickup point/i)).toBeInTheDocument();
      expect(screen.getByText(/running about 10 min late/i)).toBeInTheDocument();
      expect(screen.getByText(/location may be delayed/i)).toBeInTheDocument();
      expect(screen.getByTestId('live-map')).toBeInTheDocument();
    });
  });

  describe('Profile sheets, Saved Places, Calendar ICS & pending guard', () => {
    it('keeps keyboard focus while typing multi-character text in section inputs', async () => {
      const user = userEvent.setup();
      mocks.useApiData.set('/auth/passkeys', []);

      render(
        <TestWrap>
          <Profile />
        </TestWrap>,
      );

      await user.click(screen.getByRole('button', { name: /personal details/i }));
      const dialog = document.body;
      const nameInput = within(dialog).getByLabelText(/name/i) as HTMLInputElement;
      await user.clear(nameInput);
      await user.type(nameInput, 'Temporary Unsaved Name');
      expect(nameInput.value).toBe('Temporary Unsaved Name');
      expect(document.activeElement).toBe(nameInput);

    });

    it('manages saved places, rotates/revokes calendar ICS, and saves alert channel preferences', async () => {
      const user = userEvent.setup();
      mocks.useApiData.set('/auth/passkeys', []);
      mocks.useApiData.set('/saved-places', [
        { id: 1, label: 'Home', area_id: 2, area_name: 'Petaling Jaya', address: '10 Jalan SS2', gate_notes: 'Gate 2', is_default: 0 },
      ]);
      mocks.useApiData.set('/areas', [{ id: 2, name: 'Petaling Jaya', active: true }]);
      mocks.useApiData.set('/me/calendar-feed', { enabled: true });
      mocks.post.mockResolvedValue({ token: 'caltok999', ics_path: '/api/calendar/caltok999.ics' });

      render(
        <TestWrap>
          <Profile />
        </TestWrap>,
      );

      // Saved addresses sheet
      await user.click(screen.getByRole('button', { name: /saved addresses/i }));
      const placesDlg = document.body;
      expect(within(placesDlg).getByText('Home')).toBeInTheDocument();
      expect(within(placesDlg).getByText(/Petaling Jaya/)).toBeInTheDocument();

      await user.click(within(placesDlg).getByRole('button', { name: /edit home/i }));
      await user.click(within(placesDlg).getByRole('button', { name: /^remove$/i }));
      const confirmRemoveBtns = screen.getAllByRole('button', { name: /^remove$/i });
      if (confirmRemoveBtns.length > 1) {
        await user.click(confirmRemoveBtns[confirmRemoveBtns.length - 1]);
      }
      await waitFor(() => {
        expect(mocks.del).toHaveBeenCalledWith('/saved-places/1');
      });

      await user.click(within(placesDlg).getByRole('button', { name: /add address/i }));
      await user.type(within(placesDlg).getByLabelText(/^label$/i), 'Office');
      await user.selectOptions(within(placesDlg).getByLabelText(/^area$/i), '2');
      await user.type(within(placesDlg).getByRole('combobox', { name: /street address/i }), '88 Tower B');
      await user.type(within(placesDlg).getByLabelText(/gate \/ pickup notes/i), 'Lobby');
      await user.click(within(placesDlg).getByRole('button', { name: /^save$/i }));
      expect(mocks.post).toHaveBeenCalledWith('/saved-places', {
        label: 'Office',
        area_id: 2,
        address: '88 Tower B',
        gate_notes: 'Lobby',
        is_default: false,
      });
      await user.click(screen.getByRole('link', { name: /back/i }));

      // Calendar feed sheet (folded inside Preferences)
      await user.click(screen.getByRole('button', { name: /preferences/i }));
      await user.click(screen.getByRole('button', { name: /calendar feed/i }));
      const calDlg = document.body;
      await user.click(within(calDlg).getByRole('button', { name: /reset calendar link/i }));
      await waitFor(() => {
        expect(mocks.post).toHaveBeenCalledWith('/me/calendar-feed/rotate');
      });
      const calLink = within(calDlg).getByRole('link', { name: /open in calendar app/i });
      expect(calLink).toHaveAttribute('href', expect.stringMatching(/^webcal:\/\/.*\/api\/calendar\/caltok999\.ics$/));

      await user.click(within(calDlg).getByRole('button', { name: /turn off feed/i }));
      expect(mocks.del).toHaveBeenCalledWith('/me/calendar-feed');
      await user.click(screen.getByRole('link', { name: /back/i }));

      // Preferences sheet (readiness prompt + WhatsApp + Telegram)
      await user.click(screen.getByRole('button', { name: /preferences/i }));
      const prefDlg = document.body;
      await user.selectOptions(within(prefDlg).getByRole('combobox'), '30');
      await user.click(within(prefDlg).getByLabelText(/allow whatsapp alert notifications/i));
      await user.type(within(prefDlg).getByPlaceholderText('123456789'), '987654321');
      await user.click(within(prefDlg).getByRole('button', { name: /^save$/i }));

      expect(mocks.put).toHaveBeenCalledWith('/me', {
        badges_opt_out: false,
        readiness_prompt_mins: 30,
        telegram_chat_id: '987654321',
        whatsapp_opt_in: true,
        push_opt_in: true,
      });
    });

    it('hides saved places and related member links when account status is pending', () => {
      mockMe = { ...mockMe, status: 'pending' };
      mocks.useApiData.set('/auth/passkeys', []);

      render(
        <TestWrap>
          <Profile />
        </TestWrap>,
      );

      expect(screen.getByRole('button', { name: /personal details/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /preferences/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /account security/i })).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /saved addresses/i })).toBeNull();
      expect(screen.queryByRole('button', { name: /calendar feed/i })).toBeNull();
      expect(screen.queryByRole('link', { name: /my report/i })).toBeNull();
    });
  });

  describe('Rides recurring schedules & offline snapshot fallback', () => {
    it('is history only: no recurring modal or new-ride button, empty state points to events', () => {
      mocks.useApiData.set('/rides?scope=upcoming', []);

      render(
        <TestWrap>
          <Rides />
        </TestWrap>,
      );

      expect(screen.queryByRole('button', { name: /recurring/i })).toBeNull();
      expect(screen.queryByRole('link', { name: /request a ride/i })).toBeNull();
      expect(screen.getByRole('link', { name: /browse events/i })).toHaveAttribute('href', '/events');
    });

    it('falls back to saved today ride snapshot when upcoming API fails offline', () => {
      const todayRide: Ride = {
        id: 301,
        status: 'confirmed',
        pickup_at: `${todayStr()}T10:30:00`,
        area: { id: 1, name: 'Petaling Jaya' },
        pickup_address: '12 Jalan SS2/10',
        destination: 'Offline Snapshot Destination',
        seats: 1,
        assistance: '',
        notes: '',
        event: null,
        trip_id: null,
        passenger: { id: 7, name: 'Sister Ruth', phone: '+60123456789' },
        driver: null,
        needs_attention: false,
        attention_reason: '',
        started_at: null,
        picked_up_at: null,
        completed_at: null,
        updated_at: `${todayStr()}T08:00:00`,
        viewer: { is_passenger: true, is_driver: false, is_staff: false },
      };
      saveTodayRidesSnapshot(7, [todayRide]);
      mocks.useApiErrors.set('/rides?scope=upcoming', 'Network error');

      render(
        <TestWrap>
          <Rides />
        </TestWrap>,
      );

      expect(screen.getByText(/Offline — showing your saved rides/i)).toBeInTheDocument();
      expect(screen.getByText('Offline Snapshot Destination')).toBeInTheDocument();
      expect(screen.queryByText('Network error')).toBeNull();
    });
  });

  describe('Service Worker privacy and redirect guards (public/sw.js)', () => {
    function loadSwHarness() {
      const swSource = fs.readFileSync(path.resolve(__dirname, '../public/sw.js'), 'utf-8');
      const listeners = new Map<string, (e: unknown) => void>();
      const storedKeys: string[] = [];
      const mockCache = {
        addAll: vi.fn().mockResolvedValue(undefined),
        put: vi.fn((k: string | { url?: string }) => {
          storedKeys.push(typeof k === 'string' ? k : String(k?.url ?? k));
          return Promise.resolve();
        }),
      };
      const fakeWindow = {
        navigate: vi.fn().mockResolvedValue(undefined),
        focus: vi.fn(),
      };
      const mockSelf = {
        location: { origin: 'https://graceride.org' },
        skipWaiting: vi.fn().mockResolvedValue(undefined),
        addEventListener: (type: string, fn: (e: unknown) => void) => listeners.set(type, fn),
        caches: {
          open: vi.fn().mockResolvedValue(mockCache),
          match: vi.fn().mockResolvedValue(undefined),
          keys: vi.fn().mockResolvedValue([]),
          delete: vi.fn().mockResolvedValue(true),
        },
        clients: {
          claim: vi.fn().mockResolvedValue(undefined),
          matchAll: vi.fn().mockResolvedValue([fakeWindow]),
          openWindow: vi.fn().mockResolvedValue(undefined),
        },
        registration: {
          showNotification: vi.fn().mockResolvedValue(undefined),
        },
        navigator: {
          setAppBadge: vi.fn().mockResolvedValue(undefined),
        },
      };
      const fakeHtmlResponse = {
        ok: true,
        headers: { get: (name: string) => (name.toLowerCase() === 'content-type' ? 'text/html; charset=utf-8' : null) },
        clone: () => fakeHtmlResponse,
      };
      const mockFetch = vi.fn().mockResolvedValue(fakeHtmlResponse);
      vm.runInNewContext(swSource, {
        self: mockSelf,
        caches: mockSelf.caches,
        fetch: mockFetch,
        URL,
        Promise,
      });
      return { listeners, storedKeys, mockCache, fakeWindow, mockSelf };
    }

    it('never intercepts /api/* and never caches bearer capability URLs', async () => {
      const { listeners, storedKeys, mockCache } = loadSwHarness();
      const onFetch = listeners.get('fetch')!;
      expect(onFetch).toBeTypeOf('function');

      // 1. /api/* requests must pass through untouched
      const apiRespond = vi.fn();
      onFetch({
        request: { method: 'GET', url: 'https://graceride.org/api/rides', mode: 'cors' },
        respondWith: apiRespond,
      });
      expect(apiRespond).not.toHaveBeenCalled();

      // 2. Navigation to /care/<secret> must never be cached; normal SPA navigation caches under '/index.html'
      let careNavPromise: Promise<unknown> | undefined;
      onFetch({
        request: {
          method: 'GET',
          url: 'https://graceride.org/care/SECRET_BEARER_TOKEN',
          mode: 'navigate',
        },
        respondWith: (p: Promise<unknown>) => {
          careNavPromise = p;
        },
      });
      await careNavPromise;
      await Promise.resolve();
      expect(mockCache.put).not.toHaveBeenCalled();

      let appNavPromise: Promise<unknown> | undefined;
      onFetch({
        request: {
          method: 'GET',
          url: 'https://graceride.org/rides',
          mode: 'navigate',
        },
        respondWith: (p: Promise<unknown>) => {
          appNavPromise = p;
        },
      });
      await appNavPromise;
      await Promise.resolve();

      expect(mockCache.put).toHaveBeenCalled();
      expect(storedKeys).toContain('/index.html');
      expect(storedKeys.some((k) => k.includes('SECRET_BEARER_TOKEN'))).toBe(false);
    });

    it('blocks cross-origin and protocol-relative notification click targets', async () => {
      const { listeners, fakeWindow } = loadSwHarness();
      const onClick = listeners.get('notificationclick')!;

      for (const badUrl of ['https://evil.example/steal', '//evil.example/phish']) {
        fakeWindow.navigate.mockClear();
        let waitP: Promise<unknown> | undefined;
        onClick({
          notification: { close: vi.fn(), data: { url: badUrl } },
          waitUntil: (p: Promise<unknown>) => {
            waitP = p;
          },
        });
        await waitP;
        expect(fakeWindow.navigate).toHaveBeenCalledWith('https://graceride.org/notifications');
      }

      fakeWindow.navigate.mockClear();
      let safeWait: Promise<unknown> | undefined;
      onClick({
        notification: { close: vi.fn(), data: { url: '/rides/42?action=readiness' } },
        waitUntil: (p: Promise<unknown>) => {
          safeWait = p;
        },
      });
      await safeWait;
      expect(fakeWindow.navigate).toHaveBeenCalledWith('https://graceride.org/rides/42?action=readiness');
    });

    it('parses JSON push payload and shows notification with icon and target URL', async () => {
      const { listeners, mockSelf } = loadSwHarness();
      const onPush = listeners.get('push')!;
      expect(onPush).toBeTypeOf('function');

      let waitP: Promise<unknown> | undefined;
      onPush({
        data: {
          json: () => ({
            title: 'Ride reminder',
            body: 'Driver arriving at 09:00',
            url: '/rides/42?action=readiness',
          }),
          text: () => 'fallback',
        },
        waitUntil: (p: Promise<unknown>) => {
          waitP = p;
        },
      });
      await waitP;

      expect(mockSelf.registration.showNotification).toHaveBeenCalledWith('Ride reminder', {
        body: 'Driver arriving at 09:00',
        icon: '/icon-192.png?v=2',
        badge: '/badge-monochrome.png',
        data: { url: '/rides/42?action=readiness' },
      });
    });

    it('updates app icon badge via setAppBadge when push event arrives', async () => {
      const { listeners, mockSelf } = loadSwHarness();
      const onPush = listeners.get('push')!;

      let waitCount: Promise<unknown> | undefined;
      onPush({
        data: {
          json: () => ({
            title: 'New alert',
            body: 'Check rides',
            url: '/notifications',
            badge_count: 4,
          }),
        },
        waitUntil: (p: Promise<unknown>) => {
          waitCount = p;
        },
      });
      await waitCount;
      expect(mockSelf.navigator.setAppBadge).toHaveBeenCalledWith(4);

      mockSelf.navigator.setAppBadge.mockClear();
      let waitDefault: Promise<unknown> | undefined;
      onPush({
        data: {
          json: () => ({
            title: 'New alert',
            body: 'Check rides',
            url: '/notifications',
          }),
        },
        waitUntil: (p: Promise<unknown>) => {
          waitDefault = p;
        },
      });
      await waitDefault;
      expect(mockSelf.navigator.setAppBadge).toHaveBeenCalledWith();
    });
  });
});
