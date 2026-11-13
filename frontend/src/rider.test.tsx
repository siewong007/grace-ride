import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
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
  '/rides/42': mockRide,
  '/rides/42/location': { sharing: false },
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Grace Member', email: 'member@example.org', groups: [{ id: 10, name: 'Youth' }] },
    has: (role: string) => role === 'driver',
    isStaff: true,
  }),
}));

vi.mock('./api', () => ({
  useApi: (path: string | null) => ({
    data: path ? mockApiData[path] ?? null : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  post: vi.fn().mockResolvedValue({ id: 99 }),
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
  it('renders greeting, one main action, and the next ride', () => {
    render(
      <MemoryRouter>
        <Home />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument();
    // One "Request a ride" action, not a tile plus a header button plus a footer button.
    expect(screen.getAllByRole('link', { name: /request a ride/i })).toHaveLength(1);
    expect(screen.getByRole('region', { name: /upcoming ride/i })).toBeInTheDocument();
    expect(screen.getAllByText(/David Driver/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Grace Community Church/i).length).toBeGreaterThan(0);
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
  it('renders sections When, Where, Who, Needs and passenger stepper', () => {
    render(
      <MemoryRouter>
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

    // Assistance chip toggle
    const wheelchairChip = screen.getByRole('button', { name: /wheelchair/i });
    expect(wheelchairChip).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(wheelchairChip);
    expect(wheelchairChip).toHaveAttribute('aria-pressed', 'true');
  });

  it('validates fields and shows error on empty submission', () => {
    render(
      <MemoryRouter>
        <RideNew />
      </MemoryRouter>
    );

    const submitBtn = screen.getByRole('button', { name: /submit ride request/i });
    fireEvent.click(submitBtn);

    const alerts = screen.getAllByRole('alert');
    expect(alerts.length).toBeGreaterThan(0);
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
});
