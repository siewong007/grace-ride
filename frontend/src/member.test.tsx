import { beforeAll, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { post } from './api';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import Events from './pages/Events';
import EventDetail from './pages/EventDetail';
import Costs from './pages/Costs';
import Settlements from './pages/Settlements';
import { ReimbursementDetail } from './pages/Reimbursements';
import HelpDetail from './pages/HelpDetail';
import Notifications from './pages/Notifications';
import Report from './pages/Report';
import Home from './pages/Home';

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
  event_id: 10,
  event_title: 'Badminton Night',
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

const today = new Date();
const localMonth = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}`;
const localDay = `${localMonth}-${String(today.getDate()).padStart(2, '0')}`;

const mockNotifs = [
  {
    id: 1,
    title: 'Ride confirmed',
    body: 'Your ride to Sunday Service is confirmed',
    link: '/rides/42',
    read: false,
    // Local date: toISOString() is UTC and names yesterday before 08:00 in UTC+8.
    created_at: `${localDay}T09:00:00`,
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
  '/events/11': {
    ...mockEvent, id: 11, my_signup: null, can_manage: false,
    join_as: [{ id: 2, name: 'Youth' }, { id: 3, name: 'Sports' }],
  },
  '/events/11/signups': [],
  '/me/shares': [mockShare],
  '/rides': [
    {
      id: 44, status: 'pending_passenger', pickup_at: '2026-12-06T09:30:00', destination: 'Grace Church',
      area: { id: 1, name: 'PJ' }, pickup_address: null, seats: 1, assistance: '', notes: '', event: null, trip_id: null,
      passenger: { id: 2, name: 'Grace Member', phone: null },
      driver: { id: 3, name: 'Brother Silas', phone: null, vehicle: null },
      needs_attention: false, attention_reason: '', started_at: null, picked_up_at: null, completed_at: null,
      updated_at: '2026-11-19T10:00:00', viewer: { is_passenger: true, is_driver: false, is_staff: false },
    },
  ],
  '/settlements': [mockSettlement],
  '/settlements/20': mockSettlement,
  '/settlements/20/revisions': [],
  '/reimbursements': [mockReimb],
  '/reimbursements/7': mockReimb,
  '/help': [{ id: 3, subject: 'Driver arrived early', user_name: 'Grace Member', status: 'in_progress', updated_at: '2026-11-12T14:16:00' }],
  '/help/3': mockTicket,
  '/notifications': mockNotifs,
  [`/reports/monthly?month=${localMonth}&range=monthly&type=all`]: mockReport,
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 2, name: 'Grace Member', email: 'member@example.org', phone: '+60123456789' },
    can: () => true,
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

  it('expands an event row inline to show contextual transport or 1-tap sign up', async () => {
    const user = userEvent.setup();
    const savedUpcoming = mockApiMap['/events?scope=upcoming'];
    mockApiMap['/events?scope=upcoming'] = [
      mockEvent,
      {
        ...mockEvent,
        id: 12,
        title: 'Choir Fellowship',
        starts_at: '2099-11-28T18:30:00',
        signup_deadline: '2099-11-27T23:59:59',
        my_signup: null,
      },
    ];
    try {
      render(
        <MemoryRouter>
          <Events />
        </MemoryRouter>
      );

      const expandBtns = screen.getAllByRole('button', { name: /show event actions/i });
      expect(expandBtns).toHaveLength(2);

      // First row is signed up without transport -> offers I need a ride + I can drive
      await user.click(expandBtns[0]);
      expect(screen.getByText(/transportation not arranged yet/i)).toBeInTheDocument();
      expect(screen.getByRole('link', { name: /i need a ride/i })).toHaveAttribute('href', '/rides/new?event=10');

      // Second row is open -> offers 1-tap Sign up
      await user.click(expandBtns[1]);
      await user.click(screen.getByRole('button', { name: /^sign up$/i }));
      expect(post).toHaveBeenCalledWith('/events/12/signup', {});
    } finally {
      mockApiMap['/events?scope=upcoming'] = savedUpcoming;
    }
  });

  it('shows month chips when events span multiple months and filters rows', async () => {
    const user = userEvent.setup();
    const savedUpcoming = mockApiMap['/events?scope=upcoming'];
    mockApiMap['/events?scope=upcoming'] = [
      { ...mockEvent, id: 10, title: 'November Gathering', starts_at: '2099-11-20T19:30:00' },
      { ...mockEvent, id: 13, title: 'December Caroling', starts_at: '2099-12-24T19:30:00' },
    ];
    try {
      render(
        <MemoryRouter>
          <Events />
        </MemoryRouter>
      );

      expect(screen.getByText('November Gathering')).toBeInTheDocument();
      expect(screen.getByText('December Caroling')).toBeInTheDocument();

      await user.click(screen.getByRole('button', { name: /^dec$/i }));
      expect(screen.queryByText('November Gathering')).not.toBeInTheDocument();
      expect(screen.getByText('December Caroling')).toBeInTheDocument();
    } finally {
      mockApiMap['/events?scope=upcoming'] = savedUpcoming;
    }
  });

  it('renders chronological My schedule day strip and vertical timeline with rides and events', async () => {
    const user = userEvent.setup();
    const savedUpcoming = mockApiMap['/events?scope=upcoming'];
    const savedRides = mockApiMap['/rides?scope=upcoming'];
    mockApiMap['/events?scope=upcoming'] = [
      { ...mockEvent, id: 10, title: 'Badminton Night', starts_at: '2099-11-20T19:30:00' },
      { ...mockEvent, id: 14, title: 'Sunday Worship', starts_at: '2099-11-22T09:00:00' },
    ];
    mockApiMap['/rides?scope=upcoming'] = [
      {
        id: 77,
        status: 'confirmed',
        pickup_at: '2099-11-20T18:45:00',
        leg: 'outbound',
        destination: 'Sports Complex',
        area: { id: 1, name: 'PJ' },
        pickup_address: null,
        seats: 1,
        assistance: '',
        notes: '',
        event: { id: 10, title: 'Badminton Night' },
        trip_id: 9,
        passenger: { id: 2, name: 'Grace Member', phone: null },
        driver: { id: 3, name: 'Brother Silas', phone: null, vehicle: null },
        needs_attention: false,
        attention_reason: '',
        started_at: null,
        picked_up_at: null,
        completed_at: null,
        updated_at: '2099-11-19T10:00:00',
        viewer: { is_passenger: true, is_driver: false, is_staff: false },
      },
    ];
    try {
      render(
        <MemoryRouter>
          <Events />
        </MemoryRouter>
      );

      await user.click(screen.getByRole('tab', { name: /my schedule/i }));

      expect(screen.getByText(/ride to sports complex/i)).toBeInTheDocument();
      expect(screen.getByText(/brother silas · pj/i)).toBeInTheDocument();
      expect(screen.getByText(/^confirmed$/i)).toBeInTheDocument();
      expect(screen.getByText('Badminton Night')).toBeInTheDocument();
      expect(screen.getByText('Sunday Worship')).toBeInTheDocument();

      // Selecting Day 1 filters out Sunday Worship
      await user.click(screen.getByRole('button', { name: /20 nov/i }));
      expect(screen.getByText('Badminton Night')).toBeInTheDocument();
      expect(screen.queryByText('Sunday Worship')).not.toBeInTheDocument();
    } finally {
      mockApiMap['/events?scope=upcoming'] = savedUpcoming;
      if (savedRides === undefined) delete mockApiMap['/rides?scope=upcoming'];
      else mockApiMap['/rides?scope=upcoming'] = savedRides;
    }
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

  it('shows driver requests for this event only once the member is going and opens them', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/events/10']}>
        <Routes>
          <Route path="/events/:id" element={<EventDetail />} />
        </Routes>
      </MemoryRouter>
    );
    expect(screen.queryByText(/ride requests for this event/i)).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /^i can drive$/i }));
    expect(screen.getByText(/ride requests for this event/i)).toBeInTheDocument();
  });

  it('offers no driving before the member has signed up', () => {
    render(
      <MemoryRouter initialEntries={['/events/11']}>
        <Routes>
          <Route path="/events/:id" element={<EventDetail />} />
        </Routes>
      </MemoryRouter>
    );
    expect(screen.queryByRole('button', { name: /^i can drive$/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /drive/i })).not.toBeInTheDocument();
  });

  it('asks a member in two of the event groups which one they join as', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/events/11']}>
        <Routes>
          <Route path="/events/:id" element={<EventDetail />} />
        </Routes>
      </MemoryRouter>
    );
    // No ride before signing up: the server needs a "going" signup first.
    expect(screen.queryByRole('link', { name: /need a ride/i })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /sign up/i }));
    expect(screen.getByRole('alert')).toHaveTextContent(/choose which group/i);
    expect(post).not.toHaveBeenCalledWith('/events/11/signup', expect.anything());
    await user.click(screen.getByRole('radio', { name: 'Sports' }));
    await user.click(screen.getByRole('button', { name: /sign up/i }));
    expect(post).toHaveBeenCalledWith('/events/11/signup', { as_group_id: 3 });
  });

  it('shows event-day chronological schedule when a member has outbound and return rides', () => {
    const saved = mockApiMap['/events/10'];
    const savedUpcoming = mockApiMap['/rides?scope=upcoming'];
    mockApiMap['/events/10'] = {
      ...mockEvent,
      my_ride: { id: 44, status: 'confirmed' },
      my_return_ride: { id: 45, status: 'pending_driver' },
    };
    mockApiMap['/rides?scope=upcoming'] = [
      {
        id: 44,
        status: 'confirmed',
        pickup_at: '2026-11-20T18:50:00',
        leg: 'outbound',
        destination: 'Sports Complex',
        area: { id: 1, name: 'PJ' },
        pickup_address: null,
        seats: 1,
        assistance: '',
        notes: '',
        event: { id: 10, title: 'Badminton Night' },
        trip_id: null,
        passenger: { id: 2, name: 'Grace Member', phone: null },
        driver: { id: 3, name: 'Brother Silas', phone: null, vehicle: null },
        needs_attention: false,
        attention_reason: '',
        started_at: null,
        picked_up_at: null,
        completed_at: null,
        updated_at: '2026-11-19T10:00:00',
        viewer: { is_passenger: true, is_driver: false, is_staff: false },
      },
    ];
    try {
      render(
        <MemoryRouter initialEntries={['/events/10']}>
          <Routes>
            <Route path="/events/:id" element={<EventDetail />} />
          </Routes>
        </MemoryRouter>
      );
      expect(screen.getByText(/your event schedule/i)).toBeInTheDocument();
      expect(screen.getByRole('link', { name: /my ride/i })).toHaveAttribute('href', '/rides/44');
      expect(screen.getByRole('link', { name: /return ride/i })).toHaveAttribute('href', '/rides/45');
    } finally {
      mockApiMap['/events/10'] = saved;
      if (savedUpcoming === undefined) delete mockApiMap['/rides?scope=upcoming'];
      else mockApiMap['/rides?scope=upcoming'] = savedUpcoming;
    }
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
    expect(screen.getByRole('link', { name: /badminton night/i })).toBeInTheDocument();
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

describe('Home page', () => {
  const renderHome = () =>
    render(
      <MemoryRouter>
        <Home />
      </MemoryRouter>
    );

  it('lists unpaid shares under "Needs you", linking to My costs where they are paid', () => {
    renderHome();
    expect(screen.getByRole('heading', { name: /needs you/i })).toBeInTheDocument();
    // Members cannot open the bills list, and a share id is not a bill id.
    expect(screen.getByRole('link', { name: /badminton court share/i })).toHaveAttribute('href', '/costs');
    expect(screen.getByRole('link', { name: /badminton court share/i })).toHaveTextContent(/pay/i);
  });

  it('asks the member to answer a driver who offered to drive them', () => {
    renderHome();
    expect(screen.getByRole('link', { name: /brother silas offered to drive you/i })).toHaveAttribute('href', '/rides/44');
  });

  it('shows events the member is going to under "Coming up"', () => {
    mockApiMap['/events'] = [mockEvent, { ...mockEvent, id: 11, title: 'Choir Practice', my_signup: null }];
    try {
      renderHome();
      expect(screen.getByRole('heading', { name: /coming up/i })).toBeInTheDocument();
      expect(screen.getByRole('link', { name: /badminton night/i })).toHaveAttribute('href', '/events/10');
      // Events they have not joined stay on the Events tab.
      expect(screen.queryByText('Choir Practice')).not.toBeInTheDocument();
    } finally {
      delete mockApiMap['/events'];
    }
  });

  it('on pickup day, lets the passenger say they cannot make it, with an optional reason', async () => {
    const saved = mockApiMap['/rides'];
    const ride = (saved as Record<string, unknown>[])[0];
    const now = new Date();
    const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
    mockApiMap['/rides'] = [{ ...ride, id: 45, status: 'confirmed', pickup_at: `${today}T23:30:00`, readiness_status: 'unconfirmed' }];
    try {
      const user = userEvent.setup();
      renderHome();
      expect(screen.getByRole('button', { name: /i’m ready/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /running late/i })).toBeInTheDocument();
      await user.click(screen.getByRole('button', { name: /can’t make it today/i }));
      await user.type(screen.getByLabelText(/reason/i), '  Feeling unwell  ');
      await user.click(screen.getByRole('button', { name: /i can't make it/i }));
      expect(post).toHaveBeenCalledWith('/rides/45/readiness', { status: 'unable_to_join', notes: 'Feeling unwell' });
    } finally {
      mockApiMap['/rides'] = saved;
    }
  });
});
