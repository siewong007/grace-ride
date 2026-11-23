import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { post } from './api';
import i18n from './i18n';
import RideCard, { Ride } from './components/RideCard';
import Home from './pages/Home';
import Rides from './pages/Rides';
import RideNew from './pages/RideNew';
import RideDetail from './pages/RideDetail';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mockRide: Ride = {
  id: 42,
  status: 'confirmed',
  pickup_at: '2026-11-12T14:30:00',
  area: { id: 1, name: 'Petaling Jaya' },
  pickup_address: null, // locked
  destination: 'Grace Community Church',
  seats: 2,
  assistance: 'Wheelchair',
  notes: 'Front gate entry',
  event: { id: 101, title: 'Sunday Service' },
  trip_id: 202,
  passenger: { id: 1, name: 'Grace Member', phone: '+60123456789' },
  driver: {
    id: 2,
    name: 'David Driver',
    phone: '+60198765432',
    vehicle: { model: 'Civic', color: 'White', plate: 'WXY 1234' },
  },
  needs_attention: false,
  attention_reason: '',
  started_at: null,
  picked_up_at: null,
  completed_at: null,
  updated_at: '2026-11-12T10:00:00',
  viewer: { is_passenger: true, is_driver: false, is_staff: false },
};

const mockApiData: Record<string, unknown> = {
  '/rides': [mockRide],
  '/rides?scope=upcoming': [mockRide],
  '/rides?scope=past': [],
  '/events': [{ id: 101, title: 'Sunday Service', kind: 'service', location: 'Grace Hall', starts_at: '2026-11-15T09:00:00' }],
  '/events/1': { id: 1, title: 'Sunday Service', kind: 'service', location: 'Grace Hall', starts_at: '2026-11-15T09:00:00', my_signup: { status: 'going' } },
  '/events/2': { id: 2, title: 'Prayer Meeting', kind: 'prayer', location: 'Prayer Room', starts_at: '2026-11-16T19:00:00', my_signup: null },
  '/me/shares': [{ id: 1, title: 'Lunch share', amount_cents: 1500, status: 'pending', due_date: null }],
  '/announcements': [
    {
      id: 1,
      title: 'Welcome notice',
      body: 'Welcome to Grace Ride',
      pinned: true,
      by: 'Staff',
      created_at: '2026-11-12T00:00:00',
    },
  ],
  '/areas': [{ id: 1, name: 'Petaling Jaya', active: true }],
  '/proxy-riders': [{ id: 1, name: 'Sister Chen', phone: '012-3456789', relationship: 'parent', notes: 'Need cane' }],
  '/rides/42': mockRide,
  '/rides/43': {
    ...mockRide,
    id: 43,
    is_proxy: true,
    proxy: {
      is_proxy: true,
      rider_name: 'Sister Chen',
      rider_phone: '012-3456789',
      rider_relationship: 'parent',
      booker_name: 'Grace Member',
      booker_phone: '+60123456789',
    },
  },
  '/rides/42/location': { sharing: false },
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Grace Member', email: 'member@example.org', groups: [{ id: 10, name: 'Youth' }] },
    can: () => true,
  }),
}));

vi.mock('./api', () => ({
  api: vi.fn().mockResolvedValue([]),
  useApi: (path: string | null) => ({
    data: path ? mockApiData[path] ?? null : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  post: vi.fn().mockResolvedValue({ id: 99 }),
  put: vi.fn().mockResolvedValue({ ok: true }),
  del: vi.fn().mockResolvedValue({}),
}));

describe('RideCard', () => {
  it('renders time, destination, pickup area and links to detail', () => {
    render(
      <MemoryRouter>
        <RideCard ride={mockRide} />
      </MemoryRouter>
    );

    expect(screen.getByText('Grace Community Church')).toBeInTheDocument();
    expect(screen.getByText('Petaling Jaya')).toBeInTheDocument();
    expect(screen.getByText('David Driver')).toBeInTheDocument();

    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/rides/42');
  });

  it('shows the waiting status once when ride has no driver', () => {
    const noDriverRide: Ride = {
      ...mockRide,
      id: 43,
      driver: null,
      status: 'pending_driver',
    };

    render(
      <MemoryRouter>
        <RideCard ride={noDriverRide} />
      </MemoryRouter>
    );

    expect(screen.getAllByText('Awaiting driver')).toHaveLength(1);
    expect(screen.queryByText('Waiting for driver')).not.toBeInTheDocument();
  });
});

describe('Home page', () => {
  it('renders greeting, event-first focus ride, and omits empty dashboard widgets', () => {
    render(
      <MemoryRouter>
        <Home />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: /upcoming ride/i })).toBeInTheDocument();
    expect(screen.getAllByText(/Sunday Service/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/David Driver/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Grace Community Church/i).length).toBeGreaterThan(0);
    expect(screen.queryByText(/no driving tasks today/i)).not.toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /find an event to ride to/i })).not.toBeInTheDocument();
  });

  it('shows an RSVP event focus with contextual ride CTA when no ride is booked yet', () => {
    const savedRides = mockApiData['/rides'];
    const savedEvents = mockApiData['/events'];
    mockApiData['/rides'] = [];
    mockApiData['/events'] = [
      {
        id: 101,
        title: 'Sunday Service',
        kind: 'service',
        location: 'Grace Hall',
        starts_at: '2099-11-15T09:00:00',
        my_signup: { status: 'going' },
      },
    ];
    try {
      render(
        <MemoryRouter>
          <Home />
        </MemoryRouter>
      );

      expect(screen.getByText(/transportation not arranged yet/i)).toBeInTheDocument();
      expect(screen.getByRole('link', { name: /i need a ride/i })).toHaveAttribute('href', '/rides/new?event=101');
    } finally {
      mockApiData['/rides'] = savedRides;
      mockApiData['/events'] = savedEvents;
    }
  });
});

describe('Rides page', () => {
  it('renders tabs for upcoming and past, and request button', () => {
    render(
      <MemoryRouter>
        <Rides />
      </MemoryRouter>
    );

    expect(screen.getByRole('tablist')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /upcoming/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /past/i })).toBeInTheDocument();
  });
});

describe('RideNew page', () => {
  it('without an event sends the member to the events list instead of a form that cannot be saved', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new']}>
        <RideNew />
      </MemoryRouter>
    );
    expect(screen.queryByRole('button', { name: /submit ride request/i })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /browse events/i })).toHaveAttribute('href', '/events');
  });

  it('requires event signup and prompts to sign up when not going', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new?event=2']}>
        <RideNew />
      </MemoryRouter>
    );
    expect(screen.queryByRole('button', { name: /submit ride request/i })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: /go to event/i })).toHaveAttribute('href', '/events/2');
  });

  it('renders sections When, Where, Who, Needs and passenger stepper', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new?event=1']}>
        <RideNew />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 3, name: /when/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 3, name: /where/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 3, name: /who/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 3, name: /needs/i })).toBeInTheDocument();

    // Stepper buttons
    const incBtn = screen.getByRole('button', { name: /increase/i });
    const decBtn = screen.getByRole('button', { name: /decrease/i });
    expect(incBtn).toBeInTheDocument();
    expect(decBtn).toBeInTheDocument();

    // Click increase
    fireEvent.click(incBtn);
    expect(screen.getByText(/2 seats/i)).toBeInTheDocument();

    // Needs & notes starts collapsed and opens on demand
    expect(document.getElementById('ride-needs-panel')).toBeNull();
    const needsToggle = screen.getByRole('button', { name: /special needs & notes/i });
    expect(needsToggle).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(needsToggle);
    expect(needsToggle).toHaveAttribute('aria-expanded', 'true');
    const needsPanel = document.getElementById('ride-needs-panel')!;
    expect(needsPanel).not.toBeNull();

    // Assistance chip toggle
    const wheelchairChip = within(needsPanel).getByRole('button', { name: /wheelchair/i });
    expect(wheelchairChip).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(wheelchairChip);
    expect(wheelchairChip).toHaveAttribute('aria-pressed', 'true');
  });

  it('allows toggling proxy booking mode in RideNew', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new?event=1']}>
        <RideNew />
      </MemoryRouter>
    );

    const forOtherBtn = screen.getByRole('button', { name: /book for family/i });
    expect(forOtherBtn).toBeInTheDocument();
    fireEvent.click(forOtherBtn);

    expect(screen.getByPlaceholderText(/Sister Chen/i)).toBeInTheDocument();
  });

  it('preselects proxy booking mode when ?proxy=1 query param is set', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new?event=1&proxy=1']}>
        <RideNew />
      </MemoryRouter>
    );

    expect(screen.getByPlaceholderText(/Sister Chen/i)).toBeInTheDocument();
  });

  it('allows selecting trip direction (to event, round trip, return only)', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new?event=1']}>
        <RideNew />
      </MemoryRouter>
    );

    const roundBtn = screen.getByRole('button', { name: /round trip/i });
    const returnBtn = screen.getByRole('button', { name: /return only/i });
    expect(roundBtn).toBeInTheDocument();
    expect(returnBtn).toBeInTheDocument();

    fireEvent.click(returnBtn);
    expect(returnBtn).toHaveAttribute('aria-pressed', 'true');
    expect(roundBtn).toHaveAttribute('aria-pressed', 'false');
  });

  it('validates fields and shows error on empty submission', () => {
    render(
      <MemoryRouter initialEntries={['/rides/new?event=1']}>
        <RideNew />
      </MemoryRouter>
    );

    const submitBtn = screen.getByRole('button', { name: /submit ride request/i });
    fireEvent.click(submitBtn);

    const alerts = screen.getAllByRole('alert');
    expect(alerts.length).toBeGreaterThan(0);
  });

  it('auto-prefills area and pickup address from default saved place', () => {
    mockApiData['/saved-places'] = [
      {
        id: 5,
        label: 'Home',
        area_id: 1,
        area_name: 'Petaling Jaya',
        address: '88 Jalan Bahagia, PJ',
        gate_notes: 'Ring bell',
        is_default: 1,
      },
    ];
    try {
      render(
        <MemoryRouter initialEntries={['/rides/new?event=1']}>
          <RideNew />
        </MemoryRouter>
      );

      expect(screen.getByDisplayValue('88 Jalan Bahagia, PJ')).toBeInTheDocument();
      expect(screen.getByText(/prefilled from saved place: home/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /special needs & notes/i })).toHaveAttribute('aria-expanded', 'true');
      expect(screen.getByDisplayValue('Ring bell')).toBeInTheDocument();
    } finally {
      delete mockApiData['/saved-places'];
    }
  });

  it('switches multiple saved addresses without duplicating gate notes and applies to return destination', () => {
    const savedAreas = mockApiData['/areas'];
    mockApiData['/areas'] = [
      { id: 1, name: 'Petaling Jaya', active: true },
      { id: 2, name: 'Subang Jaya', active: true },
    ];
    mockApiData['/saved-places'] = [
      { id: 1, label: 'Home', area_id: 1, area_name: 'Petaling Jaya', address: '10 Jalan SS2', gate_notes: 'Ring bell 1', is_default: 1 },
      { id: 2, label: 'Office', area_id: 2, area_name: 'Subang Jaya', address: '88 Tower B', gate_notes: 'Guardhouse pass', is_default: 0 },
    ];
    try {
      render(
        <MemoryRouter initialEntries={['/rides/new?event=1']}>
          <RideNew />
        </MemoryRouter>
      );

      const notesBox = screen.getByLabelText(/notes/i) as HTMLTextAreaElement;
      expect(notesBox.value).toBe('Ring bell 1');

      fireEvent.click(screen.getByRole('button', { name: 'Office' }));
      expect(screen.getByDisplayValue('88 Tower B')).toBeInTheDocument();
      expect((screen.getByLabelText(/pickup area/i) as HTMLSelectElement).value).toBe('2');
      expect(notesBox.value).toBe('Guardhouse pass');
      expect(notesBox.value).not.toContain('Ring bell 1');

      // Return trip: tapping Home populates destination while keeping event location as pickup
      fireEvent.click(screen.getByRole('button', { name: /return only/i }));
      fireEvent.click(screen.getByRole('button', { name: 'Home' }));
      expect(screen.getByRole('combobox', { name: /detailed pickup address/i })).toHaveValue('Grace Hall');
      expect(screen.getByRole('combobox', { name: /destination/i })).toHaveValue('10 Jalan SS2');
      expect(notesBox.value).toBe('Ring bell 1');
    } finally {
      mockApiData['/areas'] = savedAreas;
      delete mockApiData['/saved-places'];
    }
  });

  it('offers progressive save-address option for a newly typed address and saves on submit', async () => {
    const postMock = vi.mocked(post);
    postMock.mockClear();
    mockApiData['/saved-places'] = [];
    try {
      render(
        <MemoryRouter initialEntries={['/rides/new?event=1']}>
          <RideNew />
        </MemoryRouter>
      );

      fireEvent.change(screen.getByLabelText(/pickup area/i), { target: { value: '1' } });
      fireEvent.change(screen.getByRole('combobox', { name: /detailed pickup address/i }), {
        target: { value: '77 Jalan Sejahtera' },
      });

      const saveCheck = screen.getByLabelText(/save this address for next time/i);
      fireEvent.click(saveCheck);
      fireEvent.click(screen.getByRole('button', { name: 'Parents' }));

      fireEvent.click(screen.getByRole('button', { name: /submit ride request/i }));

      await waitFor(() => {
        expect(postMock).toHaveBeenCalledWith(
          '/rides',
          expect.objectContaining({
            pickup_area_id: 1,
            pickup_address: '77 Jalan Sejahtera',
          }),
        );
        expect(postMock).toHaveBeenCalledWith('/saved-places', {
          label: 'Parents',
          area_id: 1,
          address: '77 Jalan Sejahtera',
          gate_notes: '',
          is_default: true,
        });
      });
    } finally {
      delete mockApiData['/saved-places'];
    }
  });

  it('auto-fills Home label when save-address checked, validates cleared label, and opens inline Manage modal without losing form state', async () => {
    const postMock = vi.mocked(post);
    postMock.mockClear();
    mockApiData['/saved-places'] = [];
    try {
      render(
        <MemoryRouter initialEntries={['/rides/new?event=1']}>
          <RideNew />
        </MemoryRouter>
      );

      fireEvent.change(screen.getByLabelText(/pickup area/i), { target: { value: '1' } });
      fireEvent.change(screen.getByRole('combobox', { name: /detailed pickup address/i }), {
        target: { value: '77 Jalan Sejahtera' },
      });

      // Open Manage modal in-place and confirm form input stays intact when closed
      fireEvent.click(screen.getByRole('button', { name: /manage/i }));
      expect(await screen.findByRole('dialog', { name: /saved addresses/i })).toBeInTheDocument();
      fireEvent.click(screen.getByRole('button', { name: /close/i }));
      expect(screen.getByDisplayValue('77 Jalan Sejahtera')).toBeInTheDocument();

      // Checking save-address auto-fills "Home"; clearing it triggers validation alert instead of silent skip
      const saveCheck = screen.getByLabelText(/save this address for next time/i);
      fireEvent.click(saveCheck);
      const labelInput = screen.getByLabelText(/address label/i) as HTMLInputElement;
      expect(labelInput.value).toBe('Home');

      fireEvent.change(labelInput, { target: { value: '' } });
      fireEvent.click(screen.getByRole('button', { name: /submit ride request/i }));
      expect(await screen.findByText(/please enter a label/i)).toBeInTheDocument();
      expect(postMock).not.toHaveBeenCalled();
    } finally {
      delete mockApiData['/saved-places'];
    }
  });

  it('opens inline frequent riders manager modal in proxy mode without losing booking state', async () => {
    render(
      <MemoryRouter initialEntries={['/rides/new?event=1']}>
        <RideNew />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole('button', { name: /book for family/i }));
    fireEvent.change(screen.getByLabelText(/actual passenger name/i), { target: { value: 'Uncle Tan' } });
    fireEvent.click(screen.getByRole('button', { name: /manage/i }));
    expect(await screen.findByRole('dialog', { name: /frequent riders/i })).toBeInTheDocument();
    expect(screen.getByText('Sister Chen')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /close/i }));
    expect(screen.getByDisplayValue('Uncle Tan')).toBeInTheDocument();
  });
});

describe('RideDetail page', () => {
  it('renders vertical timeline, driver card, and trip details', () => {
    render(
      <MemoryRouter initialEntries={['/rides/42']}>
        <Routes>
          <Route path="/rides/:id" element={<RideDetail />} />
        </Routes>
      </MemoryRouter>
    );

    // Timeline heading
    expect(screen.getByText(/status timeline/i)).toBeInTheDocument();
    expect(screen.getAllByText('David Driver').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Grace Community Church').length).toBeGreaterThan(0);
  });

  it('renders proxy booking information and personalized readiness check', () => {
    render(
      <MemoryRouter initialEntries={['/rides/43']}>
        <Routes>
          <Route path="/rides/:id" element={<RideDetail />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText('Sister Chen')).toBeInTheDocument();
    expect(screen.getByText(/Grace Member/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Sister Chen is ready/i })).toBeInTheDocument();
  });

  it('renders proxy badge in RideCard', () => {
    const mockProxyRide: Ride = {
      ...mockRide,
      id: 43,
      is_proxy: true,
      proxy: {
        is_proxy: true,
        rider_name: 'Sister Chen',
        rider_phone: '012-3456789',
        rider_relationship: 'parent',
        booker_name: 'Grace Member',
        booker_phone: '+60123456789',
      },
    };

    render(
      <MemoryRouter>
        <RideCard ride={mockProxyRide} perspective="passenger" />
      </MemoryRouter>
    );

    expect(screen.getByText(/Sister Chen/i)).toBeInTheDocument();
    expect(screen.getByText(/Proxy/i)).toBeInTheDocument();
  });
});
