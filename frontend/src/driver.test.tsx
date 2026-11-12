import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from './i18n';
import Drive from './pages/Drive';

beforeAll(async () => {
  await i18n.changeLanguage('en');
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
    pickup_at: '2026-11-20T11:00:00',
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
  '/driver/requests?mine_areas=true': mockRequests,
  '/areas': mockAreas,
  '/driver/profile': mockProfile,
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Driver Alice', email: 'alice@example.org' },
    has: (role: string) => role === 'driver',
    isStaff: false,
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
  });
});

describe('Drive page - Available requests tab', () => {
  it('renders area filter chips, request cards with locked address notice, and offer button', async () => {
    render(
      <MemoryRouter>
        <Drive />
      </MemoryRouter>
    );

    // Switch to requests tab
    const requestsTab = screen.getByRole('tab', { name: /available ride requests/i });
    fireEvent.click(requestsTab);

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
    const settingsTab = screen.getByRole('tab', { name: /driver settings/i });
    fireEvent.click(settingsTab);

    await waitFor(() => {
      expect(screen.getByText('WXX 8888')).toBeInTheDocument();
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
});
