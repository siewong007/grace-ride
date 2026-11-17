import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import { PublicDisplayBar } from './components/AppChrome';
import { AddressInput } from './components/AddressInput';
import Chat from './components/Chat';
import PlanTransport from './pages/admin/PlanTransport';
import RideDetail from './pages/RideDetail';
import {
  clearOfflineSnapshots,
  loadTodayRideById,
  loadTodayRidesSnapshot,
  saveTodayRidesSnapshot,
} from './offlineSnapshot';
import { readDone, writeDone } from './guideProgress';
import { todayStr } from './format';
import type { Ride } from './components/RideCard';
import * as apiMod from './api';

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 7, name: 'Sister Ruth', email: 'ruth@example.org', groups: [] },
    can: () => true,
    refresh: vi.fn(),
  }),
}));

const sampleTodayRide = (overrides: Partial<Ride> = {}): Ride => ({
  id: 88,
  status: 'confirmed',
  pickup_at: `${todayStr()}T10:30:00`,
  area: { id: 1, name: 'Petaling Jaya' },
  pickup_address: '12 Jalan SS2/10',
  destination: 'PJ State CMC',
  seats: 1,
  assistance: 'Walking aid',
  notes: 'Main gate',
  event: null,
  trip_id: 501,
  passenger: { id: 7, name: 'Sister Ruth', phone: '+60123456789' },
  driver: {
    id: 2,
    name: 'Brother Silas',
    phone: '+60198765432',
    vehicle: { model: 'Vios', color: 'Silver', plate: 'WXX 8888' },
  },
  needs_attention: false,
  attention_reason: '',
  started_at: null,
  picked_up_at: null,
  completed_at: null,
  updated_at: `${todayStr()}T08:00:00`,
  viewer: { is_passenger: true, is_driver: false, is_staff: false },
  ...overrides,
});

describe('UX enhancements', () => {
  beforeEach(async () => {
    localStorage.clear();
    await i18n.changeLanguage('en');
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('PublicDisplayBar cycles text sizes and updates root data-text', async () => {
    render(<PublicDisplayBar />);
    const sizeBtn = screen.getByRole('button', { name: /text size/i });
    expect(sizeBtn).toHaveTextContent('Aa');

    await userEvent.click(sizeBtn);
    expect(document.documentElement.dataset.text).toBe('large');
    expect(sizeBtn).toHaveTextContent('Aa+');

    await userEvent.click(sizeBtn);
    expect(document.documentElement.dataset.text).toBe('xlarge');
    expect(sizeBtn).toHaveTextContent('Aa++');

    await userEvent.click(sizeBtn);
    expect(document.documentElement.dataset.text).toBe('normal');
  });

  it('offlineSnapshot stores only own active today rides and purges cleanly', () => {
    const ownActive = sampleTodayRide({ id: 1 });
    const staffOnly = sampleTodayRide({
      id: 2,
      viewer: { is_passenger: false, is_driver: false, is_staff: true },
    });
    const completed = sampleTodayRide({ id: 3, status: 'completed' });

    saveTodayRidesSnapshot(7, [ownActive, staffOnly, completed]);
    const snap = loadTodayRidesSnapshot(7);
    expect(snap?.rides.map((r) => r.id)).toEqual([1]);

    // Another user cannot read user 7's snapshot; mismatch purges it.
    expect(loadTodayRidesSnapshot(99)).toBeNull();
    expect(loadTodayRidesSnapshot(7)).toBeNull();

    saveTodayRidesSnapshot(7, [ownActive]);
    expect(loadTodayRideById(7, 1)?.ride.destination).toBe('PJ State CMC');
    clearOfflineSnapshots();
    expect(loadTodayRidesSnapshot(7)).toBeNull();
  });

  it('RideDetail falls back to saved today snapshot when network fails, and refreshes on stale conflict', async () => {
    saveTodayRidesSnapshot(7, [sampleTodayRide({ id: 88 })]);

    const fetchMock = vi.fn(async () => new Response(JSON.stringify({ error: 'Offline' }), { status: 503 }));
    vi.stubGlobal('fetch', fetchMock);

    render(
      <MemoryRouter initialEntries={['/rides/88']}>
        <Routes>
          <Route path="/rides/:id" element={<RideDetail />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText(/offline — showing your saved trip details/i)).toBeInTheDocument();
    });
    expect(screen.getAllByText('Brother Silas').length).toBeGreaterThan(0);

    // Now simulate recovery + a stale-ride event triggering automatic reload
    fetchMock.mockImplementation(async () => new Response(JSON.stringify(sampleTodayRide({ id: 88 })), { status: 200 }));
    await act(async () => {
      window.dispatchEvent(new Event('gr:ride-stale'));
    });
    await waitFor(() => {
      expect(screen.getByText(/this ride’s status just changed/i)).toBeInTheDocument();
    });
  });

  it('Chat supports 1-tap quick replies and retry on send failure', async () => {
    let failSend = true;
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith('/rides/88/messages') && (!init?.method || init.method === 'GET')) {
        return new Response(JSON.stringify({ open: true, members: ['Sister Ruth', 'Brother Silas'], messages: [] }), {
          status: 200,
        });
      }
      if (url.endsWith('/rides/88/messages') && init?.method === 'POST') {
        if (failSend) {
          return new Response(JSON.stringify({ error: 'Temporary network drop' }), { status: 500 });
        }
        return new Response(JSON.stringify({ ok: true }), { status: 200 });
      }
      return new Response('{}', { status: 200 });
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<Chat rideId={88} viewerRole="passenger" />);
    const chip = await screen.findByRole('button', { name: /i’m ready at the pickup point/i });
    await userEvent.click(chip);

    const retryBtn = await screen.findByRole('button', { name: /tap to retry/i });
    expect(retryBtn).toBeInTheDocument();

    failSend = false;
    await userEvent.click(retryBtn);
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /tap to retry/i })).toBeNull();
    });
  });

  it('AddressInput reassures the user when no map pin matches', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('[]', { status: 200 })));
    const Wrapper = () => {
      const [val, setVal] = React.useState('');
      return <AddressInput id="pickup" label="Pickup address" value={val} onChange={setVal} />;
    };
    const React = await import('react');
    render(<Wrapper />);
    const input = screen.getByRole('combobox', { name: 'Pickup address' });
    await userEvent.type(input, 'Block C Lot 999');
    expect(await screen.findByText(/no map pin found/i)).toBeInTheDocument();
  });

  it('PlanTransport supports assistance filtering and multi-step undo', async () => {
    const planPayload = {
      event: { id: 5, title: 'Sunday Service', starts_at: '2026-11-22T09:00:00', location: 'PJ State CMC', group_ids: [1] },
      rides: [
        {
          id: 11,
          status: 'pending_driver',
          pickup_at: '2026-11-22T08:15:00',
          pickup_address: '10 Jalan SS2',
          seats: 1,
          assistance: 'Wheelchair',
          driver_id: null,
          area: { id: 1, name: 'SS2' },
          passenger: { id: 20, name: 'Uncle Lim' },
          match_group_id: 1,
          last: null,
        },
        {
          id: 12,
          status: 'pending_driver',
          pickup_at: '2026-11-22T08:20:00',
          pickup_address: '22 Jalan Damansara',
          seats: 1,
          assistance: '',
          driver_id: null,
          area: { id: 2, name: 'Damansara' },
          passenger: { id: 21, name: 'Brother Tan' },
          match_group_id: 1,
          last: null,
        },
      ],
      drivers: [
        {
          id: 30,
          name: 'Driver Grace',
          carpool: true,
          cap: 4,
          other_seats: 0,
          area_ids: [1, 2],
          group_ids: [1],
          vehicle: { model: 'Innova', plate: 'VAA 1111' },
        },
      ],
    };
    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: planPayload,
      error: null,
      loading: false,
      refreshing: false,
      reload: vi.fn(),
      setData: vi.fn(),
    });

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText('Uncle Lim')).toBeInTheDocument();
    expect(screen.getByText('Brother Tan')).toBeInTheDocument();

    // Filter to Needs assistance
    await userEvent.click(screen.getByRole('button', { name: /needs assistance/i }));
    expect(screen.getByText('Uncle Lim')).toBeInTheDocument();
    expect(screen.queryByText('Brother Tan')).toBeNull();

    // Assign Uncle Lim to Driver Grace, then undo with Ctrl+Z
    const select = screen.getByRole('combobox', { name: /driver for uncle lim/i });
    fireEvent.change(select, { target: { value: '30' } });
    expect(screen.getByRole('button', { name: /undo last move/i })).not.toBeDisabled();

    fireEvent.keyDown(window, { key: 'z', ctrlKey: true });
    expect((screen.getByRole('combobox', { name: /driver for uncle lim/i }) as HTMLSelectElement).value).toBe('');
  });

  it('guideProgress anchors completion to explicit step.id even when titles and order change', () => {
    const initial = [
      { id: 'step_a', title_en: 'Old Title A', title_zh: '', body_en: '', body_zh: '', link: '/events' },
      { id: 'step_b', title_en: 'Old Title B', title_zh: '', body_en: '', body_zh: '', link: '/rides' },
    ];
    writeDone('member-onboarding', initial, [0]);
    expect(readDone('member-onboarding', initial)).toEqual([0]);

    const reorderedAndRenamed = [
      { id: 'step_b', title_en: 'Renamed B', title_zh: '', body_en: '', body_zh: '', link: '/rides' },
      { id: 'step_a', title_en: 'Completely Rewritten Title A', title_zh: '', body_en: '', body_zh: '', link: '/' },
    ];
    expect(readDone('member-onboarding', reorderedAndRenamed)).toEqual([1]);
  });
});
