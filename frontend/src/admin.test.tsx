import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from './i18n';
import Dashboard from './pages/admin/Dashboard';
import AdminRides from './pages/admin/Rides';
import AdminLive from './pages/admin/Live';
import Members from './pages/admin/Members';
import Announcements from './pages/admin/Announcements';
import Settings from './pages/admin/Settings';
import AdminReimbursements from './pages/admin/Reimbursements';
import Audit from './pages/admin/Audit';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mockDashboardData = {
  today: '2026-11-20',
  today_total: 5,
  today_confirmed: 4,
  today_changes: 1,
  pending_approval: 2,
  no_driver: 1,
  needs_attention: 1,
  live_drivers: 3,
  max_live_drivers: 10,
  open_help: 2,
  pending_reimbursements: 3,
  members: 42,
  pending_recovery: 1,
};

const mockTodayRides = [
  {
    id: 1,
    status: 'confirmed',
    pickup_at: '2026-11-20T08:30:00',
    seats: 2,
    destination: 'Grace Church',
    pickup_address: '12 Jalan Maju',
    area: { id: 1, name: 'PJ' },
    passenger: { id: 10, name: 'Brother Barnabas' },
    driver: { id: 2, name: 'Sister Phoebe' },
    needs_attention: false,
  },
];

const mockApprovals = [
  {
    id: 2,
    status: 'pending_approval',
    pickup_at: '2026-11-20T10:00:00',
    seats: 1,
    destination: 'Grace Church',
    area: { id: 1, name: 'PJ' },
    passenger: { id: 11, name: 'Sister Lydia' },
    driver: { id: 3, name: 'Brother Silas' },
  },
];

const mockAttention = [
  {
    id: 3,
    status: 'requested',
    pickup_at: '2026-11-20T12:00:00',
    seats: 1,
    destination: 'Grace Church',
    area: { id: 2, name: 'KL' },
    passenger: { id: 12, name: 'Brother Titus' },
    needs_attention: true,
    attention_reason: 'Driver cancelled last minute',
  },
];

const mockUsers = [
  {
    id: 1,
    name: 'Admin Paul',
    email: 'paul@grace.org',
    phone: '+6011111111',
    roles: ['admin', 'transport'],
    status: 'active',
    created_at: '2026-01-01T00:00:00',
    passkeys: 2,
    groups: 'Leaders',
  },
  {
    id: 2,
    name: 'Member Timothy',
    email: 'timothy@grace.org',
    phone: '+6022222222',
    roles: [],
    status: 'pending',
    created_at: '2026-11-19T00:00:00',
    passkeys: 1,
    groups: '',
  },
];

const mockGroups = [
  { id: 1, name: 'Leaders', description: 'Core leaders', is_all: false, member_count: 5 },
  { id: 2, name: 'All Church', description: 'General', is_all: true, member_count: 50 },
];

const mockAreas = [
  { id: 1, name: 'Petaling Jaya', active: true },
  { id: 2, name: 'Subang', active: false },
];

const mockLive = {
  max: 10,
  drivers: [
    {
      driver_id: 1,
      name: 'Brother Silas',
      lat: 3.1234,
      lng: 101.5678,
      updated_at: '2026-11-20T10:05:00',
      rides: 2,
      first_pickup: '2026-11-20T09:00:00',
    },
  ],
};

const mockAnnouncements = [
  {
    id: 1,
    title: 'Combined Thanksgiving Service',
    body: 'Join us this Sunday 10am for Thanksgiving!',
    pinned: true,
    by: 'Admin Paul',
    created_at: '2026-11-15T08:00:00',
  },
];

const mockReimbs = [
  {
    id: 10,
    user: { id: 1, name: 'Sister Phoebe' },
    amount_cents: 2500,
    description: 'Petrol share for elderly ride',
    created_at: '2026-11-18T10:00:00',
    status: 'pending',
    receipt_file_id: 99,
  },
];

const mockAudit = [
  {
    id: 101,
    actor: 'Admin Paul',
    action: 'ride.reassign',
    entity: 'ride',
    entity_id: 5,
    detail: 'Reassigned from Silas to Luke',
    created_at: '2026-11-20T10:30:00',
  },
];

const mockApiMap: Record<string, unknown> = {
  '/admin/dashboard': mockDashboardData,
  '/admin/rides?date=2026-11-12': mockTodayRides,
  '/admin/rides?attention=true': mockAttention,
  '/admin/rides?status=pending_approval': mockApprovals,
  '/admin/rides?': mockTodayRides,
  '/admin/live': mockLive,
  '/admin/users?q=': mockUsers,
  '/groups': mockGroups,
  '/areas': mockAreas,
  '/announcements': mockAnnouncements,
  '/reimbursements?scope=review': mockReimbs,
  '/reimbursements?scope=payout': mockReimbs,
  '/admin/invites': [],
  '/admin/recovery': [],
  '/admin/audit': mockAudit,
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Admin Paul', email: 'paul@grace.org', mfa_ok: true },
    has: (role: string) => role === 'admin' || role === 'transport' || role === 'finance',
    isStaff: true,
    refresh: vi.fn(),
  }),
}));

vi.mock('./api', () => ({
  useApi: (path: string | null) => ({
    data: path ? mockApiMap[path] ?? null : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  post: vi.fn().mockResolvedValue({ id: 101, url: 'https://grace.org/inv/abc' }),
  put: vi.fn().mockResolvedValue({ ok: true }),
  del: vi.fn().mockResolvedValue({ ok: true }),
}));

describe('Admin Dashboard', () => {
  it('renders 8 stat tiles and two-column layout with today rides and pending approvals', () => {
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>
    );

    expect(screen.getByText('Ride overview', { exact: false })).toBeInTheDocument();
    expect(screen.getByText('Rides today')).toBeInTheDocument();
    expect(screen.getByText('Confirmed today')).toBeInTheDocument();
    expect(screen.getAllByText(/pending approval/i).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('No driver found')).toBeInTheDocument();
    expect(screen.getByText('Needs attention')).toBeInTheDocument();
    expect(screen.getByText('Drivers out')).toBeInTheDocument();
    expect(screen.getByText('Help requests unanswered')).toBeInTheDocument();
    expect(screen.getByText('Reimbursements pending')).toBeInTheDocument();

    // Two-column layout elements
    expect(screen.getByText('Today’s rides')).toBeInTheDocument();
    expect(screen.getByText('Needs follow-up')).toBeInTheDocument();
  });
});

describe('Admin Rides', () => {
  it('renders status filter chips and ride table', () => {
    render(
      <MemoryRouter>
        <AdminRides />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /ride management/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /all statuses/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/only needs attention/i)).toBeInTheDocument();
    expect(screen.getByText(/Brother Barnabas/)).toBeInTheDocument();
  });
});

describe('Admin Live', () => {
  it('renders live map, side list of active drivers, and legend', () => {
    render(
      <MemoryRouter>
        <AdminLive />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /live map/i })).toBeInTheDocument();
    expect(screen.getByText('Brother Silas')).toBeInTheDocument();
    expect(screen.getByText('Legend:')).toBeInTheDocument();
    expect(screen.getByText('Active trip')).toBeInTheDocument();
  });
});

describe('Admin Members', () => {
  it('renders member table, roles as pills, and unified modal trigger', async () => {
    render(
      <MemoryRouter>
        <Members />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /members & invites/i })).toBeInTheDocument();
    expect(screen.getByText('Admin Paul')).toBeInTheDocument();
    expect(screen.getByText('Member Timothy')).toBeInTheDocument();

    // Trigger edit modal
    const editBtn = screen.getByRole('button', { name: /^edit$/i });
    fireEvent.click(editBtn);

    await waitFor(() => {
      expect(screen.getByText('Edit member Admin Paul')).toBeInTheDocument();
    });
  });
});

describe('Admin Announcements', () => {
  it('renders creation form, live Member Preview card, and published list', () => {
    render(
      <MemoryRouter>
        <Announcements />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /announcements/i })).toBeInTheDocument();
    expect(screen.getByText('Member Preview')).toBeInTheDocument();
    expect(screen.getByText('Combined Thanksgiving Service')).toBeInTheDocument();
  });
});

describe('Admin Settings', () => {
  it('renders member groups and ride areas cards', () => {
    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /groups & areas/i })).toBeInTheDocument();
    expect(screen.getByText('Member groups')).toBeInTheDocument();
    expect(screen.getByText('Ride areas')).toBeInTheDocument();
    expect(screen.getByText('Leaders')).toBeInTheDocument();
    expect(screen.getByText('Petaling Jaya')).toBeInTheDocument();
  });
});

describe('Admin Reimbursements', () => {
  it('renders tabs and role-specific notice', () => {
    render(
      <MemoryRouter>
        <AdminReimbursements />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /reimbursement review/i })).toBeInTheDocument();
    expect(screen.getByText(/transport team: verify receipts/i)).toBeInTheDocument();
    expect(screen.getByText('Sister Phoebe')).toBeInTheDocument();
  });
});

describe('Admin Audit', () => {
  it('renders audit search box and logs table with monospace timestamps', () => {
    render(
      <MemoryRouter>
        <Audit />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /audit log/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/search audit log/i)).toBeInTheDocument();
    expect(screen.getByText('Admin Paul')).toBeInTheDocument();
    expect(screen.getByText('ride.reassign')).toBeInTheDocument();
  });
});
