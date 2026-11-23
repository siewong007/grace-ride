import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { ApiError, api, post } from './api';
import i18n from './i18n';
import { todayStr } from './format';
import Events, { type EventItem } from './pages/Events';
import EventDetail from './pages/EventDetail';
import Home from './pages/Home';
import { ToastProvider } from './components/ui';

let mockApiData: Record<string, unknown> = {};
let mockPermissions: Record<string, boolean> = {};

vi.mock('./api', async () => {
  const actual = await vi.importActual<typeof import('./api')>('./api');
  return {
    ...actual,
    api: vi.fn((path: string) => {
      if (path in mockApiData) {
        return Promise.resolve(mockApiData[path]);
      }
      return Promise.resolve([]);
    }),
    post: vi.fn().mockResolvedValue({ ok: true, status: 'going' }),
    useApi: (path: string | null) => ({
      data: path ? (mockApiData[path] ?? null) : null,
      loading: false,
      refreshing: false,
      error: null,
      reload: vi.fn(),
      setData: vi.fn(),
    }),
  };
});

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 2, name: 'Grace Member' },
    can: (page: string, act = 'view') => mockPermissions[`${page}.${act}`] ?? true,
    loading: false,
  }),
}));

function LocationSpy() {
  const loc = useLocation();
  return <div data-testid="loc">{loc.pathname + loc.search}</div>;
}

function makeEvent(overrides: Partial<EventItem> = {}): EventItem {
  return {
    id: 10,
    title: 'Badminton Night',
    kind: 'badminton',
    description: 'Community sports fellowship',
    starts_at: '2099-11-20T19:30:00',
    location: 'Sports Complex',
    capacity: 20,
    cost_note: '',
    signup_deadline: null,
    status: 'published',
    created_by: { id: 1, name: 'Leader Alice' },
    going: 8,
    waitlist: 0,
    spots_left: 12,
    my_signup: null,
    groups: 'Youth',
    can_manage: false,
    attendance_enabled: true,
    ...overrides,
  };
}

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  vi.clearAllMocks();
  mockApiData = {
    '/events?scope=upcoming': [],
    '/events?scope=managing': [],
    '/rides?scope=upcoming': [],
    '/driver/requests?mine_areas=true': [],
  };
  mockPermissions = {};
});

describe('Events tab redesign', () => {
  it('shows Today banner with pass and scan buttons when attendance is enabled, and hides them when disabled', () => {
    const today = todayStr();
    mockApiData['/events?scope=upcoming'] = [
      makeEvent({
        id: 1,
        title: 'Sunday Morning Worship',
        starts_at: `${today}T09:30:00`,
        location: 'Main Sanctuary',
        attendance_enabled: true,
        my_signup: {
          status: 'going',
          need_ride: false,
          offer_drive: false,
          attended: 0,
          pass_code: 'GR-8492',
        },
      }),
    ];

    const { rerender } = render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    const banner = screen.getByRole('region', { name: /today/i });
    expect(within(banner).getByRole('button', { name: /my pass/i })).toBeInTheDocument();
    expect(within(banner).getByRole('button', { name: /scan qr to check in/i })).toBeInTheDocument();
    // Header does not duplicate scan button while Today banner already shows it
    expect(screen.getAllByRole('button', { name: /scan qr to check in/i })).toHaveLength(1);

    // Disable attendance module on Today's event
    mockApiData['/events?scope=upcoming'] = [
      makeEvent({
        id: 1,
        title: 'Sunday Morning Worship',
        starts_at: `${today}T09:30:00`,
        location: 'Main Sanctuary',
        attendance_enabled: false,
        my_signup: {
          status: 'going',
          need_ride: false,
          offer_drive: false,
          attended: 0,
          pass_code: 'GR-8492',
        },
      }),
    ];

    rerender(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    const updatedBanner = screen.getByRole('region', { name: /today/i });
    expect(within(updatedBanner).queryByRole('button', { name: /my pass/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /scan qr to check in/i })).not.toBeInTheDocument();
    expect(within(updatedBanner).getByRole('link', { name: /i need a ride/i })).toBeInTheDocument();
  });

  it('hides header scan button when all loaded upcoming events have attendance disabled', () => {
    mockApiData['/events?scope=upcoming'] = [
      makeEvent({ id: 2, title: 'Cell Badminton', attendance_enabled: false }),
      makeEvent({ id: 3, title: 'Christmas Carols', attendance_enabled: false }),
    ];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.queryByRole('button', { name: /scan qr to check in/i })).not.toBeInTheDocument();
  });

  it('hides scan button and avoids duplicate Create buttons when upcoming is empty', async () => {
    const user = userEvent.setup();
    mockApiData['/events?scope=upcoming'] = [];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <LocationSpy />
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.queryByRole('button', { name: /scan qr to check in/i })).not.toBeInTheDocument();
    const createLinks = screen.getAllByRole('link', { name: /create an event/i });
    expect(createLinks).toHaveLength(1);
    expect(createLinks[0]).toHaveAttribute('href', '/events/new');

    await user.click(screen.getByRole('button', { name: /browse past events/i }));
    expect(screen.getByTestId('loc')).toHaveTextContent('/events?tab=past');
  });

  it('handles JOIN_AS_REQUIRED inline in 2-step QuickRsvpModal without leaving /events', async () => {
    const user = userEvent.setup();
    const dinner = makeEvent({
      id: 22,
      title: 'Youth Fellowship Dinner',
      my_signup: null,
    });
    mockApiData['/events?scope=upcoming'] = [dinner];
    mockApiData['/events/22'] = {
      ...dinner,
      join_as: [
        { id: 5, name: 'Youth Fellowship' },
        { id: 6, name: 'SS2 Cell Group' },
      ],
    };

    vi.mocked(post)
      .mockRejectedValueOnce(new ApiError(400, 'Choose which group you are joining as', 'JOIN_AS_REQUIRED'))
      .mockResolvedValueOnce({ ok: true, status: 'going' });

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    await user.click(screen.getByRole('button', { name: /^sign up$/i }));

    // Step 1: group picker modal
    expect(await screen.findByText(/joining as/i)).toBeInTheDocument();
    await user.click(screen.getByRole('radio', { name: 'SS2 Cell Group' }));
    const dialog = screen.getByRole('dialog');
    await user.click(within(dialog).getByRole('button', { name: /^sign up$/i }));

    await waitFor(() => {
      expect(post).toHaveBeenNthCalledWith(2, '/events/22/signup', { as_group_id: 6 });
    });

    // Step 2: transport follow-up modal
    expect(await screen.findByText(/how are you getting there\?/i)).toBeInTheDocument();
    expect(within(screen.getByRole('dialog')).getByRole('link', { name: /i need a ride/i })).toHaveAttribute(
      'href',
      '/rides/new?event=22',
    );
  });

  it('keeps /events clean by default and filters + persists tab/filter in URL params', async () => {
    const user = userEvent.setup();
    mockApiData['/events?scope=upcoming'] = [
      makeEvent({
        id: 30,
        title: 'Open Fellowship',
        my_signup: null,
      }),
      makeEvent({
        id: 31,
        title: 'Going Without Ride',
        my_signup: { status: 'going', need_ride: false, offer_drive: false, attended: 0 },
      }),
    ];
    mockApiData['/driver/requests?mine_areas=true'] = [
      { id: 500, event: { id: 30, title: 'Open Fellowship' } },
    ];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <LocationSpy />
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.getByTestId('loc')).toHaveTextContent('/events');

    await user.click(screen.getByRole('button', { name: /needs transport/i }));
    expect(screen.queryByText('Open Fellowship')).not.toBeInTheDocument();
    expect(screen.getByText('Going Without Ride')).toBeInTheDocument();
    expect(screen.getByTestId('loc')).toHaveTextContent('/events?filter=needs_transport');

    await user.click(screen.getByRole('button', { name: /rides needed \(1\)/i }));
    expect(screen.getByText('Open Fellowship')).toBeInTheDocument();
    expect(screen.queryByText('Going Without Ride')).not.toBeInTheDocument();
    expect(screen.getByTestId('loc')).toHaveTextContent('/events?filter=rides_needed');
  });

  it('renders connected Outing Blocks with DayStrip counts in My schedule and gates My pass on attendance_enabled', async () => {
    const user = userEvent.setup();
    mockApiData['/events?scope=upcoming'] = [
      makeEvent({
        id: 40,
        title: 'Morning Worship',
        starts_at: '2099-11-20T09:30:00',
        attendance_enabled: true,
        my_signup: {
          status: 'going',
          need_ride: false,
          offer_drive: false,
          attended: 0,
          pass_code: 'PASS-40',
        },
      }),
      makeEvent({
        id: 41,
        title: 'Casual Badminton',
        starts_at: '2099-11-29T16:00:00',
        attendance_enabled: false,
        my_signup: {
          status: 'going',
          need_ride: false,
          offer_drive: false,
          attended: 0,
          pass_code: 'PASS-41',
        },
      }),
    ];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events?tab=my']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.getByText('Morning Worship')).toBeInTheDocument();
    expect(screen.getByText('Casual Badminton')).toBeInTheDocument();
    // Only Morning Worship has attendance_enabled: true -> only 1 My pass button
    expect(screen.getAllByRole('button', { name: /my pass/i })).toHaveLength(1);
    expect(screen.getAllByRole('button', { name: /add to calendar/i })).toHaveLength(2);

    // DayStrip shows counts and filters cleanly
    await user.click(screen.getByRole('button', { name: /29 nov/i }));
    expect(screen.queryByText('Morning Worship')).not.toBeInTheDocument();
    expect(screen.getByText('Casual Badminton')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /my pass/i })).not.toBeInTheDocument();
  });

  it('renders organiser I manage cards with attendance gating and post-event reimbursement trigger', () => {
    mockApiData['/events?scope=managing'] = [
      makeEvent({
        id: 50,
        title: 'Youth Dinner',
        status: 'published',
        can_manage: true,
        attendance_enabled: true,
        going: 22,
        capacity: 25,
        unassigned_rides: 2,
      }),
      makeEvent({
        id: 51,
        title: 'Cell Badminton',
        status: 'published',
        can_manage: true,
        attendance_enabled: false,
        going: 12,
        capacity: 16,
      }),
      makeEvent({
        id: 52,
        title: 'Thanksgiving Lunch',
        status: 'finished',
        can_manage: true,
        attendance_enabled: true,
        going: 38,
        capacity: 40,
        attended_count: 35,
      }),
    ];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events?tab=managing']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.getByText('2 rides unassigned')).toBeInTheDocument();
    expect(screen.getByText('35 attended')).toBeInTheDocument();
    // Only Youth Dinner (published + attendance_enabled) gets Attendance QR
    expect(screen.getAllByRole('button', { name: /attendance qr code/i })).toHaveLength(1);
    // Finished event shows Log Post-Event Reimbursements
    expect(screen.getByRole('button', { name: /log post-event reimbursements/i })).toBeInTheDocument();
  });

  it('filters Past events by search query and Attended only chip', async () => {
    const user = userEvent.setup();
    vi.mocked(api).mockResolvedValueOnce({
      items: [
        makeEvent({
          id: 60,
          title: 'Mid-Autumn Lunch',
          status: 'finished',
          attendance_enabled: true,
          location: 'Fellowship Hall',
          my_signup: { status: 'going', need_ride: false, offer_drive: false, attended: 1 },
        }),
        makeEvent({
          id: 61,
          title: 'Morning Prayer Walk',
          status: 'finished',
          attendance_enabled: true,
          location: 'Taman Jaya Park',
          my_signup: { status: 'going', need_ride: false, offer_drive: false, attended: 0 },
        }),
        makeEvent({
          id: 62,
          title: 'Casual Badminton',
          status: 'finished',
          attendance_enabled: false,
          location: 'Court 2',
          my_signup: { status: 'going', need_ride: false, offer_drive: false, attended: 1 },
        }),
      ],
      next_before: null,
    });

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events?tab=past']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(await screen.findByText('Mid-Autumn Lunch')).toBeInTheDocument();
    expect(screen.getByText('Morning Prayer Walk')).toBeInTheDocument();
    expect(screen.getByText('Casual Badminton')).toBeInTheDocument();
    expect(screen.getAllByText(/^attended$/i)).toHaveLength(1);

    await user.click(screen.getByRole('button', { name: /attended only/i }));
    expect(screen.getByText('Mid-Autumn Lunch')).toBeInTheDocument();
    expect(screen.queryByText('Morning Prayer Walk')).not.toBeInTheDocument();
    expect(screen.queryByText('Casual Badminton')).not.toBeInTheDocument();

    await user.type(screen.getByPlaceholderText(/search past events/i), 'Park');
    expect(screen.getByText(/no past events match your search/i)).toBeInTheDocument();
  });

  it('auto-opens DriveRequests on EventDetail when ?drive=1 is present', () => {
    const ev = makeEvent({
      id: 70,
      title: 'Sunday Service',
      my_signup: { status: 'going', need_ride: false, offer_drive: false, attended: 0 },
    });
    mockApiData['/events/70'] = { ...ev, group_ids: [1], my_ride: null, settlement: null };
    mockApiData['/events/70/signups'] = [];
    mockApiData['/driver/requests?event_id=70'] = [];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events/70?drive=1']}>
          <Routes>
            <Route path="/events/:id" element={<EventDetail />} />
          </Routes>
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.getByText(/ride requests for this event/i)).toBeInTheDocument();
  });

  it('persists Going on my own and hides ride/drive buttons on EventDetail', async () => {
    const user = userEvent.setup();
    const ev = makeEvent({
      id: 71,
      title: 'Fellowship Lunch',
      my_signup: { status: 'going', need_ride: false, offer_drive: false, self_transport: false, attended: 0 },
    });
    mockApiData['/events/71'] = { ...ev, group_ids: [1], my_ride: null, settlement: null };
    mockApiData['/events/71/signups'] = [];

    const { rerender } = render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events/71']}>
          <Routes>
            <Route path="/events/:id" element={<EventDetail />} />
          </Routes>
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.getByRole('link', { name: /i need a ride/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^i can drive$/i })).toBeInTheDocument();
    expect(screen.getByText(/not arranged yet/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /going on my own/i }));
    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/events/71/transport', { self_transport: true });
    });

    mockApiData['/events/71'] = {
      ...ev,
      my_signup: { status: 'going', need_ride: false, offer_drive: false, self_transport: true, attended: 0 },
      group_ids: [1],
      my_ride: null,
      settlement: null,
    };

    rerender(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events/71']}>
          <Routes>
            <Route path="/events/:id" element={<EventDetail />} />
          </Routes>
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.queryByRole('link', { name: /i need a ride/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^i can drive$/i })).not.toBeInTheDocument();
    expect(screen.queryByText(/not arranged yet/i)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /change transport/i })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /change transport/i }));
    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/events/71/transport', { self_transport: false });
    });
  });

  it('saves self_transport when clicking Going on my own in QuickRsvpModal', async () => {
    const user = userEvent.setup();
    const dinner = makeEvent({
      id: 22,
      title: 'Youth Fellowship Dinner',
      my_signup: null,
    });
    mockApiData['/events?scope=upcoming'] = [dinner];
    vi.mocked(post).mockResolvedValue({ ok: true, status: 'going' });

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    await user.click(screen.getByRole('button', { name: /^sign up$/i }));
    const dialog = await screen.findByRole('dialog');
    await user.click(within(dialog).getByRole('button', { name: /going on my own/i }));

    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/events/22/transport', { self_transport: true });
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });

  it('hides ride/drive prompts on Upcoming, Today banner, Needs transport filter, and My schedule when self_transport is true', async () => {
    const user = userEvent.setup();
    const today = todayStr();
    mockApiData['/events?scope=upcoming'] = [
      makeEvent({
        id: 80,
        title: 'Self Transport Gathering',
        starts_at: `${today}T18:00:00`,
        attendance_enabled: false,
        my_signup: { status: 'going', need_ride: false, offer_drive: false, self_transport: true, attended: 0 },
      }),
      makeEvent({
        id: 81,
        title: 'Needs Ride Gathering',
        starts_at: '2099-11-25T18:00:00',
        attendance_enabled: false,
        my_signup: { status: 'going', need_ride: false, offer_drive: false, self_transport: false, attended: 0 },
      }),
    ];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/events']}>
          <Events />
        </MemoryRouter>
      </ToastProvider>,
    );

    const banner = screen.getByRole('region', { name: /today/i });
    expect(within(banner).queryByRole('link', { name: /i need a ride/i })).not.toBeInTheDocument();
    expect(within(banner).getByRole('link', { name: /view event/i })).toBeInTheDocument();

    const selfCard = screen
      .getByText('Self Transport Gathering', { selector: '.event-card-title' })
      .closest('article')!;
    expect(within(selfCard).queryByRole('link', { name: /i need a ride/i })).not.toBeInTheDocument();
    expect(within(selfCard).queryByRole('link', { name: /i can drive/i })).not.toBeInTheDocument();
    expect(within(selfCard).getByText(/going on my own/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /needs transport/i }));
    expect(
      screen.queryByText('Self Transport Gathering', { selector: '.event-card-title' }),
    ).not.toBeInTheDocument();
    expect(screen.getByText('Needs Ride Gathering')).toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: /my schedule/i }));
    const selfRow = screen
      .getByText('Self Transport Gathering', { selector: '.schedule-node-title' })
      .closest('li')!;
    expect(selfRow).toBeInTheDocument();
    expect(within(selfRow).getByText(/going on my own/i)).toBeInTheDocument();
    expect(within(selfRow).queryByRole('link', { name: /i need a ride/i })).not.toBeInTheDocument();
    expect(within(selfRow).queryByRole('link', { name: /i can drive/i })).not.toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: /i need a ride/i })).toHaveLength(1);
    expect(screen.getAllByRole('link', { name: /i can drive/i })).toHaveLength(1);
  });

  it('hides ride/drive buttons on Home NextGoingEventFocus when self_transport is true', () => {
    mockApiData['/rides'] = [];
    mockApiData['/events'] = [
      makeEvent({
        id: 90,
        title: 'Prayer Meeting',
        my_signup: { status: 'going', need_ride: false, offer_drive: false, self_transport: true, attended: 0 },
      }),
    ];
    mockApiData['/me/shares'] = [];
    mockApiData['/announcements'] = [];

    render(
      <ToastProvider>
        <MemoryRouter initialEntries={['/']}>
          <Home />
        </MemoryRouter>
      </ToastProvider>,
    );

    expect(screen.getByText('Prayer Meeting')).toBeInTheDocument();
    expect(screen.getByText(/going on your own/i)).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /i need a ride/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /i can drive/i })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /view event/i })).toHaveAttribute('href', '/events/90');
  });
});
