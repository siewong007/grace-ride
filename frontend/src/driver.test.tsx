import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from './i18n';
import Drive from './pages/Drive';
import { DriveRequests } from './components/drive/DriveRequests';

beforeAll(async () => {
  await i18n.changeLanguage('en');
  // The schedule mocks below are keyed to this date; Drive asks for "today".
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date('2026-11-12T09:00:00'));
});

afterAll(() => {
  vi.useRealTimers();
});

const mockScheduleRides = [
  {
    id: 101,
    status: 'confirmed',
    pickup_at: '2026-11-20T09:30:00',
    seats: 2,
    pickup_address: '123 Jalan Damai, PJ',
    destination: 'Grace Community Church',
    area: { id: 1, name: 'Petaling Jaya' },
    passenger: { id: 10, name: 'Sister Martha', phone: '+60123456789' },
    assistance: 'Walking aid',
    viewer: { is_driver: true, is_passenger: false, is_staff: false },
  },
  {
    id: 102,
    status: 'in_progress',
    // Pickup time already passed (mocked "now" is 2026-11-12T09:00): the no-show
    // button only appears once the passenger can actually be late.
    pickup_at: '2026-11-12T08:30:00',
    seats: 1,
    pickup_address: '45 Jalan Masjid, KL',
    destination: 'Grace Community Church',
    area: { id: 2, name: 'Kuala Lumpur' },
    passenger: { id: 11, name: 'Brother David', phone: '+60198765432' },
    assistance: null,
    picked_up_at: null,
    viewer: { is_driver: true, is_passenger: false, is_staff: false },
  },
];

const mockRequests = [
  {
    id: 201,
    status: 'requested',
    pickup_at: '2026-11-20T14:00:00',
    seats: 3,
    pickup_address: null,
    destination: 'Grace Community Church',
    area: { id: 1, name: 'Petaling Jaya' },
    passenger: { id: 12, name: 'Elder John', phone: '+601122334455' },
    assistance: 'Wheelchair',
    event: { id: 5, title: 'Sunday Service' },
  },
];

const mockAreas = [
  { id: 1, name: 'Petaling Jaya', active: true },
  { id: 2, name: 'Kuala Lumpur', active: true },
  { id: 3, name: 'Subang Jaya', active: true },
];

const mockProfile = {
  carpool_enabled: true,
  spare_seats: 2,
  area_ids: [1, 2],
  vehicle: {
    model: 'Toyota Vios',
    color: 'Silver',
    plate: 'WXX 8888',
    seats: 5,
  },
};

const mockApiMap: Record<string, unknown> = {
  '/driver/schedule?from=2026-11-12&to=2026-11-12': mockScheduleRides,
  '/driver/schedule?from=2026-11-12&to=2026-11-18': mockScheduleRides,
  '/driver/requests?event_id=7': mockRequests,
  '/areas': mockAreas,
  '/driver/profile': mockProfile,
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Driver Alice', email: 'alice@example.org' },
    can: (page: string, act = 'view') =>
      ['rides', 'drive', 'costs', 'reimbursements', 'help', 'report'].includes(page) || (page === 'events' && act === 'view'),
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
  post: vi.fn().mockResolvedValue({ ok: true }),
  put: vi.fn().mockResolvedValue({ ok: true }),
  del: vi.fn().mockResolvedValue({ ok: true }),
}));

describe('Drive page - Schedule tab', () => {
  it('renders trip cards with passenger name, route, navigation links, and primary action', () => {
    render(
      <MemoryRouter>
        <Drive />
      </MemoryRouter>
    );

    expect(screen.getByRole('heading', { level: 1, name: /driving/i })).toBeInTheDocument();
    expect(screen.getByText('Sister Martha')).toBeInTheDocument();
    expect(screen.getByText(/123 Jalan Damai, PJ/i)).toBeInTheDocument();
    expect(screen.getByText('Walking aid')).toBeInTheDocument();

    // Navigation links
    expect(screen.getAllByRole('link', { name: /google maps/i })).toHaveLength(2);
    expect(screen.getAllByRole('link', { name: /waze/i })).toHaveLength(2);

    // Large next-step action buttons
    expect(screen.getByRole('button', { name: /start trip/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /picked up/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /no-show/i })).toBeInTheDocument();
    const disclosures = screen.getAllByText(/notify delay or withdraw/i);
    expect(disclosures.length).toBeGreaterThan(0);
    fireEvent.click(disclosures[0]);
    expect(screen.getAllByRole('button', { name: /notify 5 min late/i }).length).toBeGreaterThan(0);
  });

  it('renders browse events CTA when schedule has no rides', () => {
    mockApiMap['/driver/schedule?from=2026-11-12&to=2026-11-12'] = [];
    render(
      <MemoryRouter>
        <Drive />
      </MemoryRouter>
    );

    expect(screen.getByRole('link', { name: /browse events/i })).toHaveAttribute('href', '/events');
    mockApiMap['/driver/schedule?from=2026-11-12&to=2026-11-12'] = mockScheduleRides;
  });
});

describe('Drive page - no generic requests tab', () => {
  it('offers only schedule and settings; requests are volunteered for from an event', () => {
    render(
      <MemoryRouter>
        <Drive />
      </MemoryRouter>
    );
    expect(screen.queryByRole('tab', { name: /requests/i })).toBeNull();
    expect(screen.getByRole('tab', { name: /schedule/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /^settings$/i })).toBeInTheDocument();
  });
});

describe('Event driver requests', () => {
  it('lists that event\'s requests with locked address notice and offer button', async () => {
    render(
      <MemoryRouter>
        <DriveRequests eventId={7} />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('Elder John')).toBeInTheDocument();
    });

    expect(screen.getByText(/detailed address shown after approval/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /offer to drive/i })).toBeInTheDocument();
    expect(screen.getByText('Sunday Service')).toBeInTheDocument();
  });
});

describe('Drive page - Settings tab', () => {
  it('renders service areas chips, carpool stepper, vehicle fields with plate preview, and dirty-state indicator', async () => {
    render(
      <MemoryRouter>
        <Drive />
      </MemoryRouter>
    );

    // Switch to settings tab
    const settingsTab = screen.getByRole('tab', { name: /^settings$/i });
    fireEvent.click(settingsTab);

    await waitFor(() => {
      expect(screen.getAllByText('WXX 8888').length).toBeGreaterThanOrEqual(1);
    });

    // Plate badge preview
    expect(screen.getByText(/silver · toyota vios/i)).toBeInTheDocument();

    // Initial state: clean
    expect(screen.getByText(/all settings saved/i)).toBeInTheDocument();

    // Toggle an area chip to make form dirty
    const subangChip = screen.getByRole('checkbox', { name: 'Subang Jaya' });
    fireEvent.click(subangChip);

    // Now dirty indicator appears
    await waitFor(() => {
      expect(screen.getByText(/unsaved changes/i)).toBeInTheDocument();
    });

    const saveBtn = screen.getByRole('button', { name: /save settings/i });
    expect(saveBtn).not.toBeDisabled();
  });

  it('allows volunteer driver to view multiple vehicles, choose active vehicle cleanly, and clamp spare seats', async () => {
    const prevProfile = mockApiMap['/driver/profile'];
    mockApiMap['/driver/profile'] = {
      carpool_enabled: true,
      spare_seats: 3,
      area_ids: [1, 2],
      active_vehicle_id: 1,
      vehicle: {
        id: 1,
        model: 'Toyota Vios',
        color: 'Silver',
        plate: 'WXX 8888',
        seats: 5,
        is_active: true,
      },
      vehicles: [
        {
          id: 1,
          model: 'Toyota Vios',
          color: 'Silver',
          plate: 'WXX 8888',
          seats: 5,
          is_active: true,
        },
        {
          id: 2,
          model: 'Smart Fortwo',
          color: 'Yellow',
          plate: 'VYY 9999',
          seats: 2,
          is_active: false,
        },
      ],
    };

    try {
      render(
        <MemoryRouter>
          <Drive />
        </MemoryRouter>
      );

      const settingsTab = screen.getByRole('tab', { name: /^settings$/i });
      fireEvent.click(settingsTab);

      await waitFor(() => {
        expect(screen.getAllByText('WXX 8888').length).toBeGreaterThanOrEqual(1);
        expect(screen.getByText('VYY 9999')).toBeInTheDocument();
      });

      // Both vehicles rendered in the list
      expect(screen.getByText(/yellow smart fortwo/i)).toBeInTheDocument();

      // Vehicle 2 has "Select for driving" button
      const selectBtn = screen.getByRole('button', { name: /select for driving/i });
      expect(selectBtn).toBeInTheDocument();

      // Click select for 2-seat vehicle 2
      fireEvent.click(selectBtn);

      // Active vehicle switches cleanly without false unsaved state, and spare seats clamp from 3 -> 1
      await waitFor(() => {
        expect(screen.getByText(/yellow · smart fortwo/i)).toBeInTheDocument();
        expect(screen.getByText(/all settings saved/i)).toBeInTheDocument();
        expect(screen.getByText(/1 seat$/i)).toBeInTheDocument();
      });
    } finally {
      mockApiMap['/driver/profile'] = prevProfile;
    }
  });

  it('renders service areas and empty vehicle state instead of spinning when driver has no vehicles', async () => {
    const prevProfile = mockApiMap['/driver/profile'];
    mockApiMap['/driver/profile'] = {
      carpool_enabled: false,
      spare_seats: 1,
      area_ids: [1],
      vehicle: null,
      vehicles: [],
      active_vehicle_id: null,
    };

    try {
      render(
        <MemoryRouter>
          <Drive />
        </MemoryRouter>
      );

      fireEvent.click(screen.getByRole('tab', { name: /^settings$/i }));

      await waitFor(() => {
        expect(screen.getByText(/no vehicles configured yet/i)).toBeInTheDocument();
      });
      expect(screen.getByRole('checkbox', { name: 'Petaling Jaya' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /add vehicle/i })).toBeInTheDocument();
    } finally {
      mockApiMap['/driver/profile'] = prevProfile;
    }
  });

  it('allows volunteer driver to open add vehicle modal and validates required colour/model/plate', async () => {
    render(
      <MemoryRouter>
        <Drive />
      </MemoryRouter>
    );

    const settingsTab = screen.getByRole('tab', { name: /^settings$/i });
    fireEvent.click(settingsTab);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /add vehicle/i })).toBeInTheDocument();
    });

    const addBtn = screen.getByRole('button', { name: /add vehicle/i });
    fireEvent.click(addBtn);

    // Modal opens with dialog
    const dlg = await screen.findByRole('dialog', { name: /add vehicle/i });
    expect(dlg).toBeInTheDocument();

    // Submit empty form shows validation error instead of silently ignoring
    const submitBtns = screen.getAllByRole('button', { name: /add vehicle/i });
    fireEvent.click(submitBtns[submitBtns.length - 1]);
    await waitFor(() => {
      expect(screen.getByText(/please enter the model, colour and plate number/i)).toBeInTheDocument();
    });
  });
});
