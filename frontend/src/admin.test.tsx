import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { RIDE_STATUS } from './format';
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
    email: 'member@grace.org',
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
  '/admin/rides?page=1&limit=50': mockTodayRides,
  '/admin/live': mockLive,
  '/admin/users?': mockUsers,
  '/admin/users?page=1&limit=50': { items: mockUsers, total: 52, page: 1, page_size: 50, total_pages: 2 },
  '/admin/users?sort=email&dir=asc&page=1&limit=50': { items: [mockUsers[1], mockUsers[0]], total: 52, page: 1, page_size: 50, total_pages: 2 },
  '/groups': mockGroups,
  '/areas': mockAreas,
  '/announcements': mockAnnouncements,
  '/reimbursements?scope=review': mockReimbs,
  '/reimbursements?scope=payout': mockReimbs,
  '/reimbursements?scope=review&page=1&limit=25': mockReimbs,
  '/reimbursements?scope=payout&page=1&limit=25': mockReimbs,
  '/admin/invites': [],
  '/admin/recovery': [],
  '/admin/audit': mockAudit,
  '/admin/audit?limit=5': { items: mockAudit },
  '/admin/settings/system': { audit_retention_days: 730, chat_retention_days: 180, max_live_drivers: 10, session_timeout_mins: 15 },
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Admin Paul', email: 'paul@grace.org', mfa_ok: true },
    // Holds every permission, in a passkey / 2FA session.
    can: () => true,
    refresh: vi.fn(),
  }),
}));

// The audit log is read a page at a time: the first page says an older one follows.
const mockAuditOlder = [
  {
    id: 55,
    actor: null,
    action: 'ride.escalate',
    entity: 'ride',
    entity_id: 4,
    detail: 'no driver 12h before pickup',
    created_at: '2026-11-19T08:00:00',
  },
];
const mockApi = vi.hoisted(() => vi.fn());

vi.mock('./api', () => ({
  api: mockApi,
  useApi: (path: string | null) => ({
    data: path ? mockApiMap[path] ?? null : null,
    loading: Boolean(path?.startsWith('/admin/users?q=') && !mockApiMap[path]),
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
  it('renders the status filter and ride table', () => {
    render(
      <MemoryRouter>
        <AdminRides />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /ride management/i })).toBeInTheDocument();
    const statusFilter = screen.getByRole('combobox', { name: /status/i });
    const options = within(statusFilter).getAllByRole('option').map((o) => (o as HTMLOptionElement).value);
    // "All" plus real statuses only: the API filters on the exact value.
    expect(options[0]).toBe('');
    expect(options.slice(1).sort()).toEqual(Object.keys(RIDE_STATUS).sort());
    expect(screen.getByRole('option', { name: /all statuses/i })).toBeInTheDocument();
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
    expect(screen.getByRole('navigation', { name: 'Page 1' })).toBeInTheDocument();

    // Trigger edit modal
    const editBtn = screen.getByRole('button', { name: /^edit$/i });
    fireEvent.click(editBtn);

    await waitFor(() => {
      expect(screen.getByText('Edit member Admin Paul')).toBeInTheDocument();
    });
  });

  it('shows the server results when sorting by email with default pagination', () => {
    render(
      <MemoryRouter>
        <Members />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole('button', { name: /sort by email/i }));

    const [, firstMember] = screen.getAllByRole('row');
    expect(firstMember).toHaveTextContent('Member Timothy');
  });

  it('keeps the focused search field mounted while the member list refetches', () => {
    render(
      <MemoryRouter>
        <Members />
      </MemoryRouter>
    );

    const search = screen.getByRole('textbox', { name: /search name, email or phone/i });
    search.focus();
    fireEvent.change(search, { target: { value: 'T' } });
    expect(search).toHaveFocus();

    fireEvent.change(search, { target: { value: 'Ti' } });
    expect(search).toHaveValue('Ti');
    expect(screen.getByRole('button', { name: /clear filters/i })).toBeInTheDocument();
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
    expect(screen.getByText('Member preview')).toBeInTheDocument();
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
    expect(screen.getByText('Data retention & system limits')).toBeInTheDocument();
    expect(screen.getByText('Audit log retention (days)')).toBeInTheDocument();
    expect(screen.getByText('Session auto-lock (minutes)')).toBeInTheDocument();
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
  beforeEach(() => {
    mockApi.mockReset();
    mockApi.mockImplementation(async (path: string) =>
      path.includes('before=101')
        ? { items: mockAuditOlder, next_before: null }
        : { items: mockAudit, next_before: 101 },
    );
  });

  it('renders audit search box and logs table with monospace timestamps', async () => {
    render(
      <MemoryRouter>
        <Audit />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /audit log/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/search audit log/i)).toBeInTheDocument();
    expect(await screen.findByText('Admin Paul')).toBeInTheDocument();
    expect(screen.getAllByText('ride.reassign').length).toBeGreaterThanOrEqual(1);
    expect(mockApi).toHaveBeenCalledWith('/admin/audit');
  });

  it('loads older entries below the ones already shown until the log ends', async () => {
    render(
      <MemoryRouter>
        <Audit />
      </MemoryRouter>
    );

    fireEvent.click(await screen.findByRole('button', { name: /load older entries/i }));
    expect(await screen.findByText('ride.escalate')).toBeInTheDocument();
    expect(mockApi).toHaveBeenLastCalledWith('/admin/audit?before=101');
    // The first page stays, the entry with no actor reads "System", and the button goes.
    expect(screen.getAllByText('ride.reassign').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('System')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /load older entries/i })).not.toBeInTheDocument();
    expect(screen.getByText(/entries shown: 2\. that is all of them/i)).toBeInTheDocument();
  });

  it('sends the filters to the server and keeps the object a link asked for', async () => {
    render(
      <MemoryRouter initialEntries={['/admin/audit?entity=ride&entity_id=5']}>
        <Audit />
      </MemoryRouter>
    );

    await screen.findByText('Admin Paul');
    expect(mockApi).toHaveBeenLastCalledWith('/admin/audit?entity=ride&entity_id=5');
    expect(screen.getByText('Ride #5')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('From'), { target: { value: '2026-11-01' } });
    await waitFor(() =>
      expect(mockApi).toHaveBeenLastCalledWith('/admin/audit?entity=ride&entity_id=5&from=2026-11-01'),
    );

    fireEvent.change(screen.getByLabelText(/search audit log/i), { target: { value: 'reassign' } });
    await waitFor(() =>
      expect(mockApi).toHaveBeenLastCalledWith(
        '/admin/audit?q=reassign&entity=ride&entity_id=5&from=2026-11-01',
      ),
    );

    fireEvent.click(screen.getByRole('button', { name: /clear filters/i }));
    await waitFor(() => expect(mockApi).toHaveBeenLastCalledWith('/admin/audit'));
    expect(screen.getByLabelText(/search audit log/i)).toHaveValue('');
  });

  it('exports audit log to CSV and PDF', async () => {
    const createObjectURL = vi.fn().mockReturnValue('blob:mock');
    const revokeObjectURL = vi.fn();
    window.URL.createObjectURL = createObjectURL;
    window.URL.revokeObjectURL = revokeObjectURL;
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      blob: async () => new Blob(['ID,Timestamp\n1,2026-11-20']),
    });
    global.fetch = fetchMock;

    render(
      <MemoryRouter>
        <Audit />
      </MemoryRouter>
    );

    await screen.findByText('Admin Paul');
    const streamCsvBtn = screen.getByRole('button', { name: /stream all/i });
    const exportCsvBtn = screen.getByRole('button', { name: /export csv/i });
    const exportPdfBtn = screen.getByRole('button', { name: /export pdf/i });
    expect(streamCsvBtn).toBeEnabled();
    expect(exportCsvBtn).toBeEnabled();
    expect(exportPdfBtn).toBeEnabled();

    fireEvent.click(exportCsvBtn);
    expect(createObjectURL).toHaveBeenCalled();

    fireEvent.click(streamCsvBtn);
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('/api/admin/audit/export'),
        expect.objectContaining({ credentials: 'same-origin' })
      );
    });
  });

  it('opens and closes entry detail modal when viewing entry details', async () => {
    render(
      <MemoryRouter>
        <Audit />
      </MemoryRouter>
    );

    await screen.findByText('Admin Paul');
    const viewBtn = screen.getByRole('button', { name: new RegExp('view details #\\d+', 'i') });
    fireEvent.click(viewBtn);

    expect(await screen.findByRole('heading', { level: 2, name: new RegExp('audit entry #\\d+', 'i') })).toBeInTheDocument();
    expect(screen.getAllByText('Reassigned from Silas to Luke').length).toBeGreaterThanOrEqual(2);

    fireEvent.click(screen.getAllByRole('button', { name: /close/i })[0]);
    await waitFor(() => {
      expect(screen.queryByRole('heading', { level: 2, name: new RegExp('audit entry #\\d+', 'i') })).not.toBeInTheDocument();
    });
  });

  it('filters by action and actor on quick link click', async () => {
    render(
      <MemoryRouter>
        <Audit />
      </MemoryRouter>
    );

    await screen.findByText('Admin Paul');
    fireEvent.click(screen.getByTitle('Filter by action'));
    await waitFor(() => {
      expect(mockApi).toHaveBeenLastCalledWith('/admin/audit?action=ride.reassign');
    });

    fireEvent.click(screen.getByTitle('Filter by actor'));
    await waitFor(() => {
      expect(mockApi).toHaveBeenLastCalledWith('/admin/audit?q=Admin+Paul&action=ride.reassign');
    });
  });

  it('applies quick date presets when clicked', async () => {
    render(
      <MemoryRouter>
        <Audit />
      </MemoryRouter>
    );

    await screen.findByText('Admin Paul');
    fireEvent.click(screen.getByRole('button', { name: /today/i }));
    await waitFor(() => {
      expect(mockApi).toHaveBeenLastCalledWith(expect.stringMatching(/\/admin\/audit\?from=\d{4}-\d{2}-\d{2}&to=\d{4}-\d{2}-\d{2}/));
    });
  });
});
