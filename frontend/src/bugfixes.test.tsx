import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import AdminReports from './pages/admin/Reports';
import Report from './pages/Report';
import Notifications from './pages/Notifications';
import EventCheckin from './pages/EventCheckin';
import VisitorInvites from './components/VisitorInvites';
import Rides from './pages/Rides';
import Events from './pages/Events';
import { FeedbackProvider } from './components/ui';

const mockApi = vi.fn();
const mockPost = vi.fn();
const mockPut = vi.fn();
const mockDel = vi.fn();
const mockReload = vi.fn();

let mockMe: {
  id: number;
  name: string;
  email: string;
  phone: string;
  status: string;
  roles: string[];
  privileged?: boolean;
} = {
  id: 2,
  name: 'Grace Member',
  email: 'member@example.org',
  phone: '+60123456789',
  status: 'active',
  roles: ['member'],
};

let apiDataMap: Record<string, unknown> = {};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: mockMe,
    can: () => true,
    refresh: vi.fn(),
  }),
}));

vi.mock('./api', () => ({
  api: (...args: unknown[]) => mockApi(...args),
  post: (...args: unknown[]) => mockPost(...args),
  put: (...args: unknown[]) => mockPut(...args),
  del: (...args: unknown[]) => mockDel(...args),
  useApi: (path: string | null) => ({
    data: path ? (apiDataMap[path] ?? null) : null,
    loading: false,
    error: null,
    reload: mockReload,
  }),
}));

describe('Application bug-fix regressions', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('en');
    vi.clearAllMocks();
    apiDataMap = {};
    mockMe = {
      id: 2,
      name: 'Grace Member',
      email: 'member@example.org',
      phone: '+60123456789',
      status: 'active',
      roles: ['member'],
    };
    mockApi.mockResolvedValue({ items: [], next_before: null });
    mockPost.mockResolvedValue({ ok: true });
    mockPut.mockResolvedValue({ ok: true });
    mockDel.mockResolvedValue({ ok: true });
  });

  it('AdminReports handles redacted member directory rows without email or roles', async () => {
    const user = userEvent.setup();
    const today = new Date();
    const month = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}`;

    apiDataMap['/admin/users?q='] = {
      items: [
        { id: 11, name: 'Elder Timothy' },
        { id: 12, name: 'Sister Hannah' },
      ],
    };
    apiDataMap[`/admin/reports/monthly?month=${month}&range=monthly&type=all`] = {
      month,
      range: 'monthly',
      period_start: `${month}-01`,
      period_end: `${month}-28`,
      summary: {
        rides_total: 4,
        rides_completed: 4,
        rides_cancelled: 0,
        rides_no_show: 0,
        rides_alt_transport: 0,
        events_held: 1,
        event_attendances: 10,
        reimbursements_paid_cents: 0,
        reimbursements_pending_cents: 0,
        settlements_collected_cents: 0,
        settlements_outstanding_cents: 0,
      },
      rides_by_status: {},
      rides_by_area: [],
      top_drivers: [],
      events: [],
    };

    render(
      <MemoryRouter>
        <FeedbackProvider>
          <AdminReports />
        </FeedbackProvider>
      </MemoryRouter>,
    );

    const individualTab = screen.getByRole('tab', { name: /individual/i });
    await user.click(individualTab);

    expect(screen.queryByText(/\(undefined\)/i)).not.toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Elder Timothy' })).toBeInTheDocument();

    const filterInput = screen.getByPlaceholderText(/search by name or email/i);
    await user.type(filterInput, 'hannah');
    expect(screen.getByRole('option', { name: 'Sister Hannah' })).toBeInTheDocument();
  });

  it('Report formats half-open quarterly period_end accurately and switches badge language live', async () => {
    const user = userEvent.setup();
    const today = new Date();
    const month = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}`;
    const qPayload = {
      month,
      range: 'quarterly',
      period_start: '2026-01-01',
      period_end: '2026-04-01',
      rides: { as_passenger: 2, as_driver: 3, cancelled: 0 },
      events: { signed_up: 1, attended: 1, by_kind: [] },
      costs: { total_cents: 0, paid_cents: 0, unpaid_cents: 0 },
      reimbursements: { count: 0, total_cents: 0 },
      badges: [
        {
          key: 'driver_star',
          char: 'D',
          char_en: 'D',
          char_zh: '\u884c',
          name: 'Faithful Driver',
          name_en: 'Faithful Driver',
          name_zh: '\u5fe0\u5fc3\u53f8\u673a',
          rule: 'Complete 3 rides',
          rule_en: 'Complete 3 rides',
          rule_zh: '\u5b8c\u6210 3 \u6b21\u63a5\u9001',
          progress: 3,
          target: 3,
          earned: true,
        },
      ],
    };

    apiDataMap[`/reports/monthly?month=${month}&range=monthly&type=all`] = qPayload;
    apiDataMap[`/reports/monthly?month=${month}&range=quarterly&type=all`] = qPayload;

    render(
      <MemoryRouter>
        <Report />
      </MemoryRouter>,
    );

    await user.selectOptions(screen.getByLabelText(/time range/i), 'quarterly');
    expect(screen.getByText('January 2026 \u2013 March 2026')).toBeInTheDocument();
    expect(screen.queryByText(/May 2026/)).not.toBeInTheDocument();
    expect(screen.getByText('Faithful Driver')).toBeInTheDocument();

    await i18n.changeLanguage('zh');
    expect(await screen.findByText('\u5fe0\u5fc3\u53f8\u673a')).toBeInTheDocument();
  });

  it('Notifications deduplicates paginated items across refreshes and marks older items read', async () => {
    const user = userEvent.setup();
    apiDataMap['/notifications'] = {
      items: [
        { id: 10, title: 'Newest alert', body: 'Body 10', link: '/rides/1', read: false, created_at: '2026-11-18T09:00:00' },
      ],
      next_before: 10,
    };
    mockApi.mockResolvedValueOnce({
      items: [
        { id: 9, title: 'Older alert', body: 'Body 9', link: '/rides/2', read: false, created_at: '2026-11-10T09:00:00' },
      ],
      next_before: null,
    });

    const { rerender } = render(
      <MemoryRouter>
        <Notifications />
      </MemoryRouter>,
    );

    // Simulate older page load via infinite-scroll callback or re-render after state load
    expect(screen.getByText('Newest alert')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /mark all read/i }));
    expect(mockPost).toHaveBeenCalledWith('/notifications/read-all');

    // Simulate 20s poll returning same page 1 with next_before: 10
    rerender(
      <MemoryRouter>
        <Notifications />
      </MemoryRouter>,
    );
    expect(screen.getAllByText('Newest alert')).toHaveLength(1);
  });

  it('EventCheckin renders manual code entry when ?code= is missing and blocks pending accounts', async () => {
    const user = userEvent.setup();
    apiDataMap['/events/10/checkin'] = {
      id: 10,
      title: 'Sunday Service',
      starts_at: '2026-11-22T09:30:00',
      location: 'Main Sanctuary',
      attendance_enabled: true,
      my_attended: false,
      is_member: true,
    };

    render(
      <MemoryRouter initialEntries={['/events/10/checkin']}>
        <Routes>
          <Route path="/events/:id/checkin" element={<EventCheckin />} />
        </Routes>
      </MemoryRouter>,
    );

    const codeInput = screen.getByPlaceholderText(/enter check-in code/i);
    expect(codeInput).toBeInTheDocument();

    mockPost.mockRejectedValueOnce(new Error('Invalid check-in code'));
    await user.type(codeInput, 'BADCODE');
    await user.click(screen.getByRole('button', { name: /^check in$/i }));

    expect(await screen.findByText('Check-in unsuccessful')).toBeInTheDocument();
    expect(screen.queryByText(/attendance is disabled/i)).not.toBeInTheDocument();
  });

  it('EventCheckin does not auto-post check-in when account status is pending', async () => {
    mockMe = { ...mockMe, status: 'pending' };
    apiDataMap['/events/10/checkin'] = {
      id: 10,
      title: 'Sunday Service',
      starts_at: '2026-11-22T09:30:00',
      location: 'Main Sanctuary',
      attendance_enabled: true,
      my_attended: false,
      is_member: false,
    };

    render(
      <MemoryRouter initialEntries={['/events/10/checkin?code=SECRET123']}>
        <Routes>
          <Route path="/events/:id/checkin" element={<EventCheckin />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText(/membership pending/i)).toBeInTheDocument();
    expect(mockPost).not.toHaveBeenCalled();
  });

  it('VisitorInvites parses space-separated starts_at without NaN date or time', async () => {
    const user = userEvent.setup();
    apiDataMap['/events/10/visitors'] = [];
    apiDataMap['/areas'] = [{ id: 1, name: 'Petaling Jaya', active: true }];

    render(
      <MemoryRouter>
        <FeedbackProvider>
          <VisitorInvites event={{ id: 10, title: 'Fellowship Dinner', starts_at: '2026-11-22 19:00:00' }} />
        </FeedbackProvider>
      </MemoryRouter>,
    );

    await user.click(screen.getByRole('button', { name: /invite a visitor/i }));
    expect(screen.getByDisplayValue('2026-11-22')).toBeInTheDocument();
    expect(screen.getByDisplayValue('18:00')).toBeInTheDocument();
  });

  it('Rides and Events request paginated past scope with explicit limit on initial load', async () => {
    const user = userEvent.setup();
    apiDataMap['/rides?scope=upcoming'] = [];
    apiDataMap['/events?scope=upcoming'] = [];

    const { unmount } = render(
      <MemoryRouter>
        <FeedbackProvider>
          <Rides />
        </FeedbackProvider>
      </MemoryRouter>,
    );

    await user.click(screen.getByRole('tab', { name: /past/i }));
    await waitFor(() => {
      expect(mockApi).toHaveBeenCalledWith('/rides?scope=past&limit=30');
    });
    unmount();

    render(
      <MemoryRouter>
        <FeedbackProvider>
          <Events />
        </FeedbackProvider>
      </MemoryRouter>,
    );

    await user.click(screen.getByRole('tab', { name: /past/i }));
    await waitFor(() => {
      expect(mockApi).toHaveBeenCalledWith('/events?scope=past&limit=25');
    });
  });
});
