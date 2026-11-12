import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import { fmtDistance, fmtDuration } from './format';
import { DriverRouteCard, googleRouteUrl, TripRoute, wazeStopUrl } from './components/TripRoute';
import type { Ride } from './components/RideCard';
import RideDetail from './pages/RideDetail';
import { StartTripButton } from './components/TripActions';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

// Leaflet needs real layout; the map itself isn't under test here.
vi.mock('./components/LiveMap', () => ({
  default: ({ points, route }: { points: unknown[]; route?: unknown[] }) => (
    <div data-testid="map" data-points={points.length} data-route={route?.length ?? 0} />
  ),
}));

const route: TripRoute = {
  trip_id: 7,
  live: true,
  origin: { lat: 3.1, lng: 101.6, updated_at: '2026-11-12T08:40:00' },
  stops: [
    {
      ride_id: 2, passenger: 'Brother Ben', address: 'No 5, Jalan SS 2/24, Petaling Jaya', pickup_at: '2026-11-12T09:00',
      seats: 1, started: true, lat: 3.1, lng: 101.62, location: 'address',
      leg_distance_m: 2800, leg_duration_s: 420, distance_m: 2800, duration_s: 420, eta: '2026-11-12T08:47:00',
    },
    {
      ride_id: 1, passenger: 'Sister Ann', address: 'Taman Megah, PJ', pickup_at: '2026-11-12T09:00',
      seats: 2, started: false, lat: 3.1, lng: 101.68, location: 'live',
      leg_distance_m: 7200, leg_duration_s: 900, distance_m: 10000, duration_s: 1380, eta: '2026-11-12T09:03:00',
    },
    {
      ride_id: 3, passenger: 'Uncle Lim', address: 'Somewhere unmapped', pickup_at: '2026-11-12T09:00',
      seats: 1, started: true, lat: null, lng: null, location: null,
      leg_distance_m: null, leg_duration_s: null, distance_m: null, duration_s: null, eta: null,
    },
  ],
  onboard: 0,
  destination: {
    name: 'PJ State CMC', lat: 3.1, lng: 101.7, distance_m: 12500, duration_s: 1740, eta: '2026-11-12T09:09:00',
  },
  distance_m: 12500,
  duration_s: 1740,
  approximate: false,
  computed_at: '2026-11-12T08:40:00',
  geometry: [[3.1, 101.6], [3.1, 101.62], [3.1, 101.68], [3.1, 101.7]],
};

const passengerRide: Ride = {
  id: 1,
  status: 'in_progress',
  pickup_at: '2026-11-12T09:00:00',
  area: { id: 1, name: 'Petaling Jaya' },
  pickup_address: 'Taman Megah, PJ',
  destination: 'PJ State CMC',
  seats: 2,
  assistance: '',
  notes: '',
  event: null,
  trip_id: 7,
  passenger: { id: 6, name: 'Sister Ann', phone: null },
  driver: { id: 5, name: 'David Driver', phone: '+60198765432', vehicle: null },
  needs_attention: false,
  attention_reason: '',
  started_at: '2026-11-12T08:30:00',
  picked_up_at: null,
  completed_at: null,
  updated_at: '2026-11-12T08:30:00',
  viewer: { is_passenger: true, is_driver: false, is_staff: false },
};

const apiData: Record<string, unknown> = {
  '/driver/route': { route },
  '/rides/1': passengerRide,
  '/rides/1/location': {
    sharing: true,
    lat: 3.1,
    lng: 101.6,
    updated_at: '2026-11-12T08:40:00',
    eta: {
      phase: 'pickup',
      located: true,
      stops_before: 1,
      approximate: false,
      computed_at: '2026-11-12T08:40:00',
      distance_m: 10000,
      duration_s: 1380,
      eta: '2026-11-12T09:03:00',
      location: 'address',
      target: [3.1, 101.68],
    },
  },
};

vi.mock('./api', () => ({
  useApi: (path: string | null) => ({
    data: path ? apiData[path] ?? null : null,
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
  post: vi.fn().mockResolvedValue({ ok: true }),
  del: vi.fn().mockResolvedValue({ ok: true }),
}));

describe('distance and duration formatting', () => {
  it('rounds for reading at a glance', () => {
    expect(fmtDistance(420)).toBe('420 m');
    expect(fmtDistance(3240)).toBe('3.2 km');
    expect(fmtDistance(12_600)).toBe('13 km');
    expect(fmtDuration(20)).toBe('1 min');
    expect(fmtDuration(1380)).toBe('23 min');
    expect(fmtDuration(3900)).toBe('1 h 5 min');
  });
});

describe('navigation hand-off links', () => {
  it('sends every stop to Google Maps in the chosen order, GPS when shared', () => {
    const url = new URL(googleRouteUrl(route.stops, 'PJ State CMC'));
    expect(url.origin + url.pathname).toBe('https://www.google.com/maps/dir/');
    expect(url.searchParams.get('destination')).toBe('PJ State CMC');
    expect(url.searchParams.get('travelmode')).toBe('driving');
    expect(url.searchParams.get('waypoints')).toBe(
      'No 5, Jalan SS 2/24, Petaling Jaya|3.1,101.68|Somewhere unmapped',
    );
  });

  it('starts Waze navigation to one stop', () => {
    expect(wazeStopUrl(route.stops[1])).toBe('https://www.waze.com/ul?ll=3.1,101.68&navigate=yes');
    expect(wazeStopUrl(route.stops[0])).toBe(
      'https://www.waze.com/ul?q=No%205%2C%20Jalan%20SS%202%2F24%2C%20Petaling%20Jaya&navigate=yes',
    );
  });
});

describe('DriverRouteCard', () => {
  it('lists pickups in the suggested order with ETAs and hand-off buttons', () => {
    render(
      <MemoryRouter>
        <DriverRouteCard />
      </MemoryRouter>,
    );
    expect(screen.getByRole('heading', { name: /suggested pickup route/i })).toBeInTheDocument();
    expect(screen.getByText(/best order for 3 pickups · 13 km · about 29 min/i)).toBeInTheDocument();

    const stops = within(screen.getByRole('list', { name: /pickup stops in order/i })).getAllByRole('listitem');
    expect(stops.map((li) => within(li).queryByRole('link', { name: /brother ben|sister ann|uncle lim/i })?.textContent))
      .toEqual(['Brother Ben', 'Sister Ann · 2 seats', 'Uncle Lim', undefined]);
    expect(within(stops[0]).getByText(/2.8 km · 7 min/)).toBeInTheDocument();
    expect(within(stops[1]).getByText('Live location')).toBeInTheDocument();
    expect(within(stops[1]).getByText(/can’t see your ETA yet/i)).toBeInTheDocument();
    expect(within(stops[2]).getByText(/couldn’t find this address/i)).toBeInTheDocument();
    expect(within(stops[3]).getByText('PJ State CMC')).toBeInTheDocument();

    const google = screen.getByRole('link', { name: /navigate all stops in google maps/i });
    expect(google.getAttribute('href')).toContain('waypoints=');
    expect(google).toHaveAttribute('target', '_blank');
    expect(screen.getByRole('link', { name: /next stop in waze: brother ben/i })).toHaveAttribute(
      'href',
      expect.stringContaining('waze.com/ul?q='),
    );
    expect(screen.getByTestId('map')).toHaveAttribute('data-route', '4');
    expect(screen.getByText(/live traffic isn’t included/i)).toBeInTheDocument();
  });
});

describe('Passenger ETA on the ride page', () => {
  it('shows how far away the driver is and that others are picked up first', () => {
    render(
      <MemoryRouter initialEntries={['/rides/1']}>
        <Routes>
          <Route path="/rides/:id" element={<RideDetail />} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByText('Your driver is about 23 min away')).toBeInTheDocument();
    expect(screen.getByText(/10 km by road · arriving around/i)).toBeInTheDocument();
    expect(screen.getByText('Picking up others first: 1 before you')).toBeInTheDocument();
    // Driver + the passenger's pickup point; no route line (it would pass another pickup).
    expect(screen.getByTestId('map')).toHaveAttribute('data-points', '2');
    expect(screen.getByTestId('map')).toHaveAttribute('data-route', '0');
  });
});

describe('Start trip for a carpool', () => {
  it('names the other passengers who start at the same time', () => {
    render(<StartTripButton ride={{ id: 1 }} onStarted={vi.fn()} label="Start trip" others={['Brother Ben', 'Uncle Lim']} />);
    fireEvent.click(screen.getByRole('button', { name: 'Start trip' }));
    expect(
      screen.getByText('This also starts the trip for Brother Ben, Uncle Lim. They’ll be notified and can see your ETA.'),
    ).toBeInTheDocument();
  });

  it('says nothing extra for a single passenger', () => {
    render(<StartTripButton ride={{ id: 1 }} onStarted={vi.fn()} label="Start trip" />);
    fireEvent.click(screen.getByRole('button', { name: 'Start trip' }));
    expect(screen.queryByText(/this also starts the trip/i)).not.toBeInTheDocument();
  });
});
