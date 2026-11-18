import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import CarePass from './pages/CarePass';
import HelpGuideEditor from './pages/HelpGuideEditor';
import StepText from './components/StepText';
import { AutoLockProvider } from './components/AutoLock';
import { exportToCsv } from './format';
import {
  applyMatchReturnDrivers,
  driverLoad,
  fits,
  Plan,
  PlanDriver,
  PlanRide,
  vehicleCompatible,
} from './pages/admin/planLogic';

const mocks = vi.hoisted(() => ({
  api: vi.fn(),
  post: vi.fn(),
  useApiData: new Map<string, unknown>(),
}));

vi.mock('./api', () => ({
  api: (...a: unknown[]) => mocks.api(...a),
  post: (...a: unknown[]) => mocks.post(...a),
  put: vi.fn().mockResolvedValue({ ok: true }),
  del: vi.fn().mockResolvedValue({ ok: true }),
  useApi: (path: string | null) => ({
    data: path ? mocks.useApiData.get(path) ?? null : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  subscribe: () => () => {},
}));

vi.mock('./components/LiveMap', () => ({
  default: () => <div data-testid="live-map" />,
}));

vi.mock('./webauthn', () => ({
  passkeySupported: () => false,
  reauthWithPasskey: vi.fn(),
}));

let mockMe: Record<string, unknown> = {
  id: 1,
  name: 'Sister Mary',
  email: 'mary@example.org',
  phone: '0123456789',
  photo_file_id: null,
  roles: ['member'],
  role_names: {},
  permissions: ['rides.view'],
  status: 'active',
  terms_accepted: true,
  mfa_ok: true,
  totp_enabled: true,
  has_password: false,
  passkey_count: 0,
  badges_opt_out: false,
  backup_codes_count: 4,
  locked: false,
  privileged: false,
  unread: 0,
  groups: [],
  dev_mode: false,
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: mockMe,
    loading: false,
    can: () => true,
    refresh: vi.fn(),
  }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

describe('StepText rich formatting', () => {
  it('renders bold, italic, code, and bullet lists safely without raw HTML', () => {
    const { container } = render(
      <StepText
        text={
          'Follow these steps:\n- Press **Confirm** and *review*\n- Enter `DN2026` reference\nTip: keep receipt <script>alert(1)</script>'
        }
      />,
    );
    expect(screen.getByText('Confirm').tagName).toBe('STRONG');
    expect(screen.getByText('review').tagName).toBe('EM');
    expect(screen.getByText('DN2026').tagName).toBe('CODE');
    expect(container.querySelectorAll('ul.step-bullets li')).toHaveLength(2);
    expect(container.querySelector('script')).toBeNull();
  });
});

describe('planLogic accessibility and return-driver matching', () => {
  const plainDriver: PlanDriver = {
    id: 10,
    name: 'Plain Car',
    carpool: true,
    cap: 4,
    other_seats: 0,
    other_rides: 0,
    area_ids: [1],
    vehicle: { model: 'Myvi', plate: 'VXX 1', seats: 4, wheelchair_ok: false, low_step: false, child_seat: false },
  };
  const accessibleDriver: PlanDriver = {
    id: 11,
    name: 'Van Driver',
    carpool: true,
    cap: 6,
    other_seats: 0,
    other_rides: 0,
    area_ids: [1],
    vehicle: { model: 'Alza', plate: 'VXX 2', seats: 6, wheelchair_ok: true, low_step: true, child_seat: true },
  };

  it('requires compatible vehicles for wheelchair and child seat rides', () => {
    const wheelchairRide: PlanRide = {
      id: 1,
      passenger: { id: 50, name: 'Elder Lim' },
      seats: 1,
      area: { id: 1, name: 'PJ' },
      pickup_at: '2026-11-22T09:00',
      pickup_address: 'Jalan 1',
      assistance: 'Wheelchair',
      notes: '',
      status: 'pending_driver',
      driver_id: null,
      last: null,
    };
    const plan: Plan = {
      event: { id: 9, title: 'Sunday', starts_at: '2026-11-22T09:30', location: 'Church', status: 'published' },
      rides: [wheelchairRide],
      drivers: [plainDriver, accessibleDriver],
    };

    expect(vehicleCompatible(plainDriver, wheelchairRide)).toBe(false);
    expect(vehicleCompatible(accessibleDriver, wheelchairRide)).toBe(true);
    expect(fits(plan, {}, plainDriver, wheelchairRide)).toBe(false);
    expect(fits(plan, {}, accessibleDriver, wheelchairRide)).toBe(true);
  });

  it('matches unassigned return trips to their paired outbound driver', () => {
    const outRide: PlanRide = {
      id: 101,
      passenger: { id: 50, name: 'Elder Lim' },
      seats: 1,
      area: { id: 1, name: 'PJ' },
      pickup_at: '2026-11-22T08:30',
      pickup_address: 'Jalan 1',
      assistance: '',
      notes: '',
      status: 'confirmed',
      driver_id: 10,
      leg: 'outbound',
      paired_ride_id: 102,
      last: null,
    };
    const retRide: PlanRide = {
      ...outRide,
      id: 102,
      pickup_at: '2026-11-22T12:00',
      status: 'pending_driver',
      driver_id: null,
      leg: 'return',
      paired_ride_id: 101,
    };
    const plan: Plan = {
      event: { id: 9, title: 'Sunday', starts_at: '2026-11-22T09:30', location: 'Church', status: 'published' },
      rides: [outRide, retRide],
      drivers: [plainDriver],
    };
    const res = applyMatchReturnDrivers(plan, { 101: 10, 102: null });
    expect(res.placed).toBe(1);
    expect(res.draft[102]).toBe(10);
  });
});

describe('CarePass scoped caregiver view', () => {
  it('shows route without home address and sends readiness on behalf of rider', async () => {
    mocks.post.mockReset().mockResolvedValue({ ok: true });
    mocks.useApiData.set('/care/caretok123', {
      ride_id: 77,
      label: 'Daughter Care View',
      rider_name: 'Uncle Tan',
      booker_name: 'Sister Mei',
      status: 'in_progress',
      leg: 'outbound',
      pickup_at: '2026-11-22T09:00:00',
      area: { id: 1, name: 'Petaling Jaya' },
      destination: 'PJ State CMC',
      readiness_status: 'unconfirmed',
      delay_minutes: 0,
      arrived_at_gate_at: null,
      started_at: '2026-11-22T08:45:00',
      picked_up_at: null,
      completed_at: null,
      driver: { name: 'Brother Silas', vehicle: { model: 'Myvi', color: 'White', plate: 'VXX 1234' } },
      location: { sharing: true, lat: 3.1, lng: 101.6, updated_at: '2026-11-22T08:50:00' },
    });

    render(
      <MemoryRouter initialEntries={['/care/caretok123']}>
        <Routes>
          <Route path="/care/:token" element={<CarePass />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByRole('heading', { name: /Uncle Tan/ })).toBeInTheDocument();
    expect(screen.getAllByText(/Petaling Jaya/).length).toBeGreaterThan(0);
    expect(screen.getByText('Brother Silas')).toBeInTheDocument();
    expect(screen.getByTestId('live-map')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Uncle Tan is ready/i }));
    await waitFor(() =>
      expect(mocks.post).toHaveBeenCalledWith('/care/caretok123/readiness', { status: 'ready' }),
    );
  });
});

describe('Server-enforced AutoLock integration', () => {
  it('locks on gr:session-locked and unlocks with TOTP code', async () => {
    localStorage.clear();
    mocks.post.mockReset().mockResolvedValue({ ok: true });
    mockMe = { ...mockMe, totp_enabled: true, passkey_count: 0 };

    render(
      <MemoryRouter>
        <AutoLockProvider>
          <div>App content</div>
        </AutoLockProvider>
      </MemoryRouter>,
    );

    expect(screen.queryByRole('dialog')).toBeNull();
    fireEvent(window, new Event('gr:session-locked'));
    expect(await screen.findByRole('dialog')).toBeInTheDocument();

    const codeInput = screen.getByPlaceholderText('6-digit code or backup code');
    fireEvent.change(codeInput, { target: { value: '123456' } });
    fireEvent.click(screen.getByRole('button', { name: 'Unlock with code' }));

    await waitFor(() => expect(mocks.post).toHaveBeenCalledWith('/auth/unlock', { totp_code: '123456' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });
});

describe('Security & flow audit regressions', () => {
  it('computes per-leg driverLoad and pairs non-carpool return legs safely', () => {
    const nonCarpoolDriver: PlanDriver = {
      id: 20,
      name: 'Non-Carpool Driver',
      carpool: false,
      cap: 4,
      other_seats: 0,
      other_rides: 0,
      area_ids: [1],
      vehicle: { model: 'Myvi', plate: 'VXX 20', seats: 4, wheelchair_ok: false, low_step: false, child_seat: false },
    };
    const outRide: PlanRide = {
      id: 201,
      passenger: { id: 60, name: 'Brother Lee' },
      seats: 2,
      area: { id: 1, name: 'PJ' },
      pickup_at: '2026-11-22T08:30',
      pickup_address: 'Jalan 2',
      assistance: '',
      notes: '',
      status: 'confirmed',
      driver_id: 20,
      leg: 'outbound',
      paired_ride_id: 202,
      last: null,
    };
    const retRide: PlanRide = {
      ...outRide,
      id: 202,
      pickup_at: '2026-11-22T12:00',
      status: 'pending_driver',
      driver_id: null,
      leg: 'return',
      paired_ride_id: 201,
    };
    const plan: Plan = {
      event: { id: 12, title: 'Worship', starts_at: '2026-11-22T09:30', location: 'Church', status: 'published' },
      rides: [outRide, retRide],
      drivers: [nonCarpoolDriver],
    };

    expect(driverLoad(plan, { 201: 20, 202: 20 }, nonCarpoolDriver, 'outbound')).toEqual({
      seats: 2,
      rides: 1,
      over: false,
    });
    expect(driverLoad(plan, { 201: 20, 202: 20 }, nonCarpoolDriver, 'return')).toEqual({
      seats: 2,
      rides: 1,
      over: false,
    });

    const res = applyMatchReturnDrivers(plan, { 201: 20, 202: null });
    expect(res.placed).toBe(1);
    expect(res.draft[202]).toBe(20);
  });

  it('neutralizes CSV formula prefixes in exportToCsv while preserving plain numbers and text', async () => {
    let capturedBlob: Blob | null = null;
    const origCreate = URL.createObjectURL;
    const origRevoke = URL.revokeObjectURL;
    URL.createObjectURL = vi.fn((b: Blob) => {
      capturedBlob = b;
      return 'blob:mock';
    }) as typeof URL.createObjectURL;
    URL.revokeObjectURL = vi.fn();

    exportToCsv('audit-test', ['Name', 'Amount', 'Note'], [
      ['=1+1', 25, '  @SUM(A1)'],
      ['+cmd', -5, '-2+3'],
      ['Alice', 10, 'Normal text'],
    ]);

    expect(capturedBlob).not.toBeNull();
    const csvText = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => reject(reader.error);
      reader.readAsText(capturedBlob!);
    });
    URL.createObjectURL = origCreate;
    URL.revokeObjectURL = origRevoke;

    expect(csvText).toContain('"\'=1+1","25","\'  @SUM(A1)"');
    expect(csvText).toContain('"\'+cmd","-5","\'-2+3"');
    expect(csvText).toContain('"Alice","10","Normal text"');
  });

  it('creates a new draft via POST instead of overwriting when duplicating a guide', async () => {
    mocks.post.mockReset().mockResolvedValue({ id: 404 });
    mocks.useApiData.set('/help/guides/welcome', {
      id: 7,
      slug: 'welcome',
      title_en: 'Welcome Guide',
      title_zh: 'Welcome Guide ZH',
      summary_en: 'Start here',
      summary_zh: 'Start here ZH',
      published: true,
      audience: 'member',
      steps: [
        {
          id: 'orig_step_1',
          title_en: 'Open home',
          title_zh: 'Open home ZH',
          body_en: 'Go home',
          body_zh: 'Go home ZH',
          link: '/',
          image_file_id: null,
        },
      ],
    });

    render(
      <MemoryRouter initialEntries={['/help/guides/welcome/edit']}>
        <Routes>
          <Route path="/help/guides/:slug/edit" element={<HelpGuideEditor />} />
          <Route path="/help/guides/:slug" element={<div>guide-saved-route</div>} />
        </Routes>
      </MemoryRouter>,
    );

    const dupBtn = await screen.findByRole('button', { name: /duplicate/i });
    fireEvent.click(dupBtn);

    const saveDraftBtn = await screen.findByRole('button', { name: 'Save draft' });
    fireEvent.click(saveDraftBtn);

    await waitFor(() => expect(mocks.post).toHaveBeenCalledTimes(1));
    expect(mocks.post.mock.calls[0][0]).toBe('/help/guides');
    const submitted = mocks.post.mock.calls[0][1] as { slug: string; steps: { id: string }[] };
    expect(submitted.slug).toBe('welcome-copy');
    expect(submitted.steps[0].id).not.toBe('orig_step_1');
  });
});
