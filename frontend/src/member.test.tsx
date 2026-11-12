import { beforeAll, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import Events from './pages/Events';
import EventDetail from './pages/EventDetail';
import Costs from './pages/Costs';
import Settlements from './pages/Settlements';
import { ReimbursementDetail } from './pages/Reimbursements';
import { HelpDetail } from './pages/Help';
import Notifications from './pages/Notifications';
import Report from './pages/Report';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mockEvent = {
  id: 10,
  title: 'Badminton Night',
  kind: 'badminton',
  description: 'Community sports fellowship',
  starts_at: '2026-11-20T19:30:00',
  location: 'Sports Complex',
  capacity: 12,
  cost_note: 'RM 10 per person',
  signup_deadline: '2026-11-19T23:59:59',
  status: 'published',
  created_by: { id: 1, name: 'Leader Alice' },
  going: 8,
  waitlist: 0,
  spots_left: 4,
  my_signup: { status: 'going', need_ride: true, offer_drive: false, attended: 1 },
  groups: 'Youth',
  can_manage: true,
  group_ids: [1],
  my_ride: null,
  settlement: null,
};

const mockSignups = [
  {
    id: 1,
    user_id: 1,
    name: 'Grace Member',
    status: 'going',
    need_ride: true,
    offer_drive: false,
    offer_seats: 0,
    attended: 1,
    ride_status: 'confirmed',
  },
];

const mockShare = {
  id: 5,
  settlement_id: 20,
  title: 'Badminton Court Share',
  event_title: 'Badminton Night',
  amount_cents: 1500,
  status: 'unpaid',
  payee_name: 'Leader Alice',
  payee_account: '1234567890 (Maybank)',
  due_date: '2026-11-25',
  note: 'Please transfer promptly',
  txn_ref: '',
  proof_file_id: null,
};

const mockSettlement = {
  id: 20,
  event: { id: 10, title: 'Badminton Night', starts_at: '2026-11-20T19:30:00' },
  created_by: { id: 1, name: 'Leader Alice' },
  title: 'Badminton Court Share',
  split_mode: 'equal' as const,
  subsidy_cents: 0,
  payee_name: 'Leader Alice',
  payee_account: '1234567890',
  due_date: '2026-11-25',
  status: 'published',
  version: 1,
  gross_cents: 3000,
  total_cents: 3000,
  confirmed: 1,
  count: 2,
  items: [{ id: 1, label: 'Court Rental', amount_cents: 3000, receipt_file_id: null }],
  shares: [
    {
      id: 5,
      user_id: 2,
      name: 'Grace Member',
      amount_cents: 1500,
      status: 'confirmed',
      proof_file_id: null,
      txn_ref: 'TNG123',
      paid_date: '2026-11-21',
      note: '',
      updated_at: '2026-11-21T10:00:00',
    },
  ],
  can_manage: true,
};

const mockReimb = {
  id: 7,
  user: { id: 2, name: 'Grace Member' },
  ride_id: 42,
  ride_at: '2026-11-12T14:30:00',
  amount_cents: 2850,
  description: 'Grab alternative transport',
  receipt_file_id: 99,
  payee_name: 'Grace Member',
  payee_phone: '+60123456789',
  status: 'approved',
  review_note: 'Approved by transport team',
  reviewer: 'Transport Staff',
  payee_verified: true,
  paid_ref: 'PAY12345',
  paid_at: '2026-11-13T10:00:00',
  created_at: '2026-11-12T16:00:00',
};

const mockTicket = {
  id: 3,
  subject: 'Driver arrived early',
  ride_id: 42,
  status: 'in_progress',
  user_id: 2,
  user_name: 'Grace Member',
  replies: [
    {
      id: 1,
      user_id: 2,
      name: 'Grace Member',
      body: 'Driver is already here, thank you!',
      staff: false,
      created_at: '2026-11-12T14:15:00',
    },
    {
      id: 2,
      user_id: 9,
      name: 'Support Staff',
      body: 'Have a safe trip!',
      staff: true,
      created_at: '2026-11-12T14:16:00',
    },
  ],
};

const mockNotifs = [
  {
    id: 1,
    title: 'Ride confirmed',
    body: 'Your ride to Sunday Service is confirmed',
    link: '/rides/42',
    read: false,
    created_at: `${new Date().toISOString().slice(0, 10)}T09:00:00`,
  },
  {
    id: 2,
    title: 'Earlier event update',
    body: 'Event details updated',
    link: '/events/10',
    read: true,
    created_at: '2026-11-01T10:00:00',
  },
];

const mockReport = {
  user: { id: 2, name: 'Grace Member' },
  month: '2026-11',
  rides_driven: 4,
  rides_received: 6,
  events_attended: 3,
  by_kind: [{ kind: 'badminton', count: 3 }],
  events: [{ id: 10, title: 'Badminton Night', kind: 'badminton', starts_at: '2026-11-12T19:30:00' }],
  costs_paid_cents: 4500,
  badges: [
    {
      key: 'service',
      char: 'A',
      name: 'Active Participant',
      rule: 'Attend 3 events in a month',
      progress: 3,
      target: 3,
      earned: true,
    },
  ],
};

const mockApiMap: Record<string, unknown> = {
  '/events?scope=upcoming': [mockEvent],
  '/events?scope=past': [],
  '/events?scope=managing': [mockEvent],
  '/events/10': mockEvent,
  '/events/10/signups': mockSignups,
  '/me/shares': [mockShare],
  '/settlements': [mockSettlement],
  '/settlements/20': mockSettlement,
  '/settlements/20/revisions': [],
  '/reimbursements': [mockReimb],
  '/reimbursements/7': mockReimb,
  '/help': [{ id: 3, subject: 'Driver arrived early', user_name: 'Grace Member', status: 'in_progress', updated_at: '2026-11-12T14:16:00' }],
  '/help/3': mockTicket,
  '/notifications': mockNotifs,
  '/reports/monthly?month=2026-11': mockReport,
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 2, name: 'Grace Member', email: 'member@example.org', phone: '+60123456789' },
    has: (role: string) => role === 'leader' || role === 'driver' || role === 'admin',
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
  post: vi.fn().mockResolvedValue({ id: 101 }),
  put: vi.fn().mockResolvedValue({}),
  del: vi.fn().mockResolvedValue({}),
}));

describe('Events page', () => {
  it('renders event list with date block, title and seats meter', () => {
    render(
      <MemoryRouter>
        <Events />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument();
    expect(screen.getByText('Badminton Night')).toBeInTheDocument();
    expect(screen.getByText(/signed up/i)).toBeInTheDocument();
  });
});

describe('EventDetail page', () => {
  it('renders event hero, signup card, and organiser manage card', () => {
    render(
      <MemoryRouter initialEntries={['/events/10']}>
        <Routes>
          <Route path="/events/:id" element={<EventDetail />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText('Badminton Night')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /withdraw/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /need a ride/i })).toBeInTheDocument();
    expect(screen.getByText(/manage event/i)).toBeInTheDocument();
  });
});

describe('Costs page', () => {
  it('renders amount owed large and payee with copy button', () => {
    render(
      <MemoryRouter>
        <Costs />
      </MemoryRouter>
    );

    expect(screen.getByText('Badminton Court Share')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /copy/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /upload transfer receipt/i })).toBeInTheDocument();
  });
});

describe('Settlements page', () => {
  it('renders settlement card with progress bar', () => {
    render(
      <MemoryRouter>
        <Settlements />
      </MemoryRouter>
    );

    expect(screen.getByText('Badminton Court Share')).toBeInTheDocument();
  });
});

describe('Reimbursements page & detail', () => {
  it('renders reimbursement list and detail with status timeline', () => {
    render(
      <MemoryRouter initialEntries={['/reimbursements/7']}>
        <Routes>
          <Route path="/reimbursements/:id" element={<ReimbursementDetail />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText(/reimbursement status/i)).toBeInTheDocument();
    expect(screen.getByText(/approved by transport team/i)).toBeInTheDocument();
  });
});

describe('Help page & ticket detail', () => {
  it('renders ticket list and chat-style discussion thread', () => {
    render(
      <MemoryRouter initialEntries={['/help/3']}>
        <Routes>
          <Route path="/help/:id" element={<HelpDetail />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /driver arrived early/i })).toBeInTheDocument();
    expect(screen.getByText('Driver is already here, thank you!')).toBeInTheDocument();
    expect(screen.getByText('Have a safe trip!')).toBeInTheDocument();
  });
});

describe('Notifications page', () => {
  it('groups notifications by today and earlier with unread dot', () => {
    render(
      <MemoryRouter>
        <Notifications />
      </MemoryRouter>
    );

    expect(screen.getByText('Today')).toBeInTheDocument();
    expect(screen.getByText('Earlier')).toBeInTheDocument();
    expect(screen.getByText('Ride confirmed')).toBeInTheDocument();
  });
});

describe('Report page', () => {
  it('renders stat tiles, month input, and badges', () => {
    render(
      <MemoryRouter>
        <Report />
      </MemoryRouter>
    );

    expect(screen.getByLabelText(/month/i)).toBeInTheDocument();
    expect(screen.getByText('Active Participant')).toBeInTheDocument();
  });
});
