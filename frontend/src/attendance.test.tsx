import { beforeAll, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import EventDetail from './pages/EventDetail';
import EventCheckin from './pages/EventCheckin';
import EventForm from './pages/EventForm';
import Events from './pages/Events';
import { extractCheckinDetails } from './components/event/qrUtils';
import { post } from './api';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mockEventWithAttendance = {
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
  my_signup: { status: 'going', need_ride: false, offer_drive: false, attended: 0 },
  groups: 'Youth',
  can_manage: true,
  attendance_enabled: true,
  checkin_code: 'TESTCODE12',
  group_ids: [1],
  my_ride: null,
  settlement: null,
};

const mockEventWithoutAttendance = {
  ...mockEventWithAttendance,
  id: 20,
  title: 'Dinner Gathering',
  attendance_enabled: false,
  checkin_code: null,
};

vi.mock('./api', async () => {
  const actual = await vi.importActual('./api');
  return {
    ...actual,
    post: vi.fn().mockImplementation((path: string) => {
      if (path.includes('/checkin')) {
        return Promise.resolve({ ok: true, already_attended: false, event_id: 10 });
      }
      return Promise.resolve({ ok: true });
    }),
    put: vi.fn().mockResolvedValue({ ok: true }),
    useApi: (path: string | null) => {
      if (path === '/events/10') {
        return { data: mockEventWithAttendance, error: null, reload: vi.fn() };
      }
      if (path === '/events/10/signups') {
        return { data: [], error: null, reload: vi.fn() };
      }
      if (path === '/events/20') {
        return { data: mockEventWithoutAttendance, error: null, reload: vi.fn() };
      }
      if (path === '/events/20/signups') {
        return { data: [], error: null, reload: vi.fn() };
      }
      if (path === '/events/10/checkin') {
        return {
          data: {
            id: 10,
            title: 'Badminton Night',
            starts_at: '2026-11-20T19:30:00',
            location: 'Sports Complex',
            attendance_enabled: true,
            my_attended: false,
            is_member: true,
          },
          error: null,
          reload: vi.fn(),
        };
      }
      if (path === '/groups') {
        return { data: [{ id: 1, name: 'Youth', is_all: true }], error: null, reload: vi.fn() };
      }
      return { data: null, error: null, reload: vi.fn() };
    },
  };
});

let mockCurrentUser = {
  id: 1,
  name: 'Test Member',
  roles: ['member', 'admin'],
  status: 'active',
  perms: { has: () => true },
};

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: mockCurrentUser,
    can: () => true,
    loading: false,
  }),
}));

describe('Attendance module in EventDetail', () => {
  it('renders attendance module section and QR buttons when enabled', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/events/10']}>
        <Routes>
          <Route path="/events/:id" element={<EventDetail />} />
        </Routes>
      </MemoryRouter>
    );

    // Shows attendance module label and participation note
    expect(screen.getByText('Attendance module')).toBeInTheDocument();
    expect(screen.getByText(/without needing a ride/i)).toBeInTheDocument();

    // User can click scan QR button
    const scanBtn = screen.getByRole('button', { name: /scan qr to check in/i });
    expect(scanBtn).toBeInTheDocument();
    await user.click(scanBtn);
    expect(screen.getByText(/scan event qr code/i)).toBeInTheDocument();

    // Close scanner modal
    await user.click(screen.getByRole('button', { name: /close/i }));

    // Organiser can click attendance QR code button
    const qrBtn = screen.getByRole('button', { name: /attendance qr code/i });
    expect(qrBtn).toBeInTheDocument();
    await user.click(qrBtn);
    expect(screen.getByText(/event check-in qr code/i)).toBeInTheDocument();
    expect(screen.getByText('TESTCODE12')).toBeInTheDocument();
  });

  it('hides attendance module and QR buttons when disabled', () => {
    render(
      <MemoryRouter initialEntries={['/events/20']}>
        <Routes>
          <Route path="/events/:id" element={<EventDetail />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.queryByText('Attendance module')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /scan qr to check in/i })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /attendance qr code/i })).not.toBeInTheDocument();
  });
});

describe('EventCheckin page', () => {
  it('automatically submits check-in when code is present in query parameters', async () => {
    mockCurrentUser = {
      id: 1,
      name: 'Test Member',
      roles: ['member'],
      status: 'active',
      perms: { has: () => true },
    };

    render(
      <MemoryRouter initialEntries={['/events/10/checkin?code=TESTCODE12']}>
        <Routes>
          <Route path="/events/:id/checkin" element={<EventCheckin />} />
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/events/10/checkin', { code: 'TESTCODE12' });
    });

    await waitFor(() => {
      expect(screen.getByText(/checked in successfully/i)).toBeInTheDocument();
      expect(screen.getByText(/without needing a ride/i)).toBeInTheDocument();
    });
  });

  it('shows members only notice when a pending user accesses checkin', async () => {
    mockCurrentUser = {
      id: 99,
      name: 'Pending User',
      roles: ['member'],
      status: 'pending',
      perms: { has: () => true },
    };

    render(
      <MemoryRouter initialEntries={['/events/10/checkin?code=TESTCODE12']}>
        <Routes>
          <Route path="/events/:id/checkin" element={<EventCheckin />} />
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText(/attendance check-in is for members only/i)).toBeInTheDocument();
    });
  });

  it('renders camera QR scanner controls and manual input when no query code', async () => {
    mockCurrentUser = {
      id: 1,
      name: 'Test Member',
      roles: ['member'],
      status: 'active',
      perms: { has: () => true },
    };

    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/events/10/checkin']}>
        <Routes>
          <Route path="/events/:id/checkin" element={<EventCheckin />} />
        </Routes>
      </MemoryRouter>
    );

    // Shows camera toggle controls or error fallback
    await waitFor(() => {
      expect(
        screen.getByText(/cannot scan\? enter check-in code directly below/i),
      ).toBeInTheDocument();
    });

    // Check manual submission
    const input = screen.getByPlaceholderText(/enter check-in code/i);
    expect(input).toBeInTheDocument();
    await user.type(input, 'MANUAL123');
    const submitBtn = screen.getByRole('button', { name: /^check in$/i });
    expect(submitBtn).toBeEnabled();
    await user.click(submitBtn);

    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/events/10/checkin', { code: 'MANUAL123' });
    });
  });
});

describe('extractCheckinDetails utility', () => {
  it('extracts code and event ID from full event URLs', () => {
    const res = extractCheckinDetails('https://grace-ride.app/events/10/checkin?code=EVENT_PASS_99');
    expect(res.code).toBe('EVENT_PASS_99');
    expect(res.eventId).toBe(10);
  });

  it('extracts code and event ID from relative URLs', () => {
    const res = extractCheckinDetails('/events/25/checkin?code=REL789');
    expect(res.code).toBe('REL789');
    expect(res.eventId).toBe(25);
  });

  it('extracts pass code and event ID from GRPASS format', () => {
    const res = extractCheckinDetails('GRPASS:15:MEMBERPASS77');
    expect(res.code).toBe('MEMBERPASS77');
    expect(res.eventId).toBe(15);
  });

  it('returns bare code when raw string provided', () => {
    const res = extractCheckinDetails('PLAINCODE42');
    expect(res.code).toBe('PLAINCODE42');
    expect(res.eventId).toBeUndefined();
  });
});

describe('Events list universal checkin button', () => {
  it('renders Scan QR to check in button in header actions', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/events']}>
        <Routes>
          <Route path="/events" element={<Events />} />
        </Routes>
      </MemoryRouter>
    );

    const scanBtn = screen.getByRole('button', { name: /scan qr to check in/i });
    expect(scanBtn).toBeInTheDocument();
    await user.click(scanBtn);

    expect(screen.getByRole('heading', { name: /scan qr to check in/i })).toBeInTheDocument();
    expect(
      screen.getByText(/point your camera at the event check-in qr code/i),
    ).toBeInTheDocument();

    // Allows entering code when cannot scan
    expect(
      screen.getByText(/cannot scan\? enter check-in code directly below/i),
    ).toBeInTheDocument();
    const codeInput = screen.getByPlaceholderText(/enter check-in code.*or link/i);
    expect(codeInput).toBeInTheDocument();
    await user.type(codeInput, 'CODE_GLOBAL_10');
    const submitBtn = screen.getByRole('button', { name: /^check in$/i });
    expect(submitBtn).toBeEnabled();
    await user.click(submitBtn);

    await waitFor(() => {
      expect(post).toHaveBeenCalledWith('/events/checkin', { code: 'CODE_GLOBAL_10' });
    });
  });
});

describe('EventForm attendance option', () => {
  it('renders attendance module toggle checkbox', () => {
    render(
      <MemoryRouter initialEntries={['/events/new']}>
        <Routes>
          <Route path="/events/new" element={<EventForm />} />
        </Routes>
      </MemoryRouter>
    );

    const checkbox = screen.getByRole('checkbox', {
      name: /enable attendance tracking/i,
    });
    expect(checkbox).toBeInTheDocument();
    expect(checkbox).toBeChecked();
  });
});
