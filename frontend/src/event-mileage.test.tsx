import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EventDetail from './pages/EventDetail';
import EventForm from './pages/EventForm';

let mockRoles = ['member', 'leader'];
let mockEventData: Record<string, unknown> | null = null;

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 1, name: 'Alice', roles: mockRoles, status: 'active' },
    can: () => true,
    loading: false,
  }),
}));

vi.mock('./api', async () => {
  const actual = await vi.importActual('./api');
  return {
    ...actual,
    post: vi.fn().mockResolvedValue({ id: 101, ok: true }),
    put: vi.fn().mockResolvedValue({ ok: true }),
    useApi: (path: string | null) => {
      if (path === '/groups') {
        return { data: [{ id: 1, name: 'All Members', is_all: true, member_count: 50 }], error: null, reload: vi.fn() };
      }
      if (path === '/events/10') {
        return { data: mockEventData, error: null, reload: vi.fn() };
      }
      if (path?.includes('/signups')) {
        return { data: [], error: null, reload: vi.fn() };
      }
      return { data: null, error: null, reload: vi.fn() };
    },
  };
});

describe('Event mileage claimable UI', () => {
  beforeEach(() => {
    mockRoles = ['member', 'leader'];
    mockEventData = null;
  });

  it('disables the mileage claimable checkbox for non-leadership organisers', () => {
    mockRoles = ['member', 'leader'];
    render(
      <MemoryRouter initialEntries={['/events/new']}>
        <Routes>
          <Route path="/events/new" element={<EventForm />} />
        </Routes>
      </MemoryRouter>
    );

    const checkbox = screen.getByRole('checkbox', {
      name: /allow volunteer drivers to claim mileage reimbursement/i,
    });
    expect(checkbox).toBeInTheDocument();
    expect(checkbox).toBeDisabled();
    expect(
      screen.getByText(/only the superadmin, president, or vice-president can authorize/i),
    ).toBeInTheDocument();
  });

  it('enables the mileage claimable checkbox for President or Vice-President', async () => {
    mockRoles = ['member', 'president'];
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/events/new']}>
        <Routes>
          <Route path="/events/new" element={<EventForm />} />
        </Routes>
      </MemoryRouter>
    );

    const checkbox = screen.getByRole('checkbox', {
      name: /allow volunteer drivers to claim mileage reimbursement/i,
    });
    expect(checkbox).toBeInTheDocument();
    expect(checkbox).not.toBeDisabled();
    expect(checkbox).not.toBeChecked();

    await user.click(checkbox);
    expect(checkbox).toBeChecked();
  });

  it('displays mileage claimability on EventDetail', () => {
    mockEventData = {
      id: 10,
      title: 'Church Camp',
      kind: 'other',
      description: 'Annual camp',
      starts_at: '2026-11-25T09:00:00',
      location: 'Campsite',
      capacity: 50,
      cost_note: '',
      signup_deadline: null,
      status: 'published',
      created_by: { id: 1, name: 'Alice' },
      going: 20,
      waitlist: 0,
      spots_left: 30,
      my_signup: null,
      groups: 'All Members',
      can_manage: true,
      attendance_enabled: true,
      mileage_claimable: true,
      my_ride: null,
      settlement: null,
    };

    render(
      <MemoryRouter initialEntries={['/events/10']}>
        <Routes>
          <Route path="/events/:id" element={<EventDetail />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText(/claimable \(eligible for volunteer drivers\)/i)).toBeInTheDocument();
  });
});
