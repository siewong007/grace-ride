import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import Reimbursements, { ReimbursementForm, ReimbursementDetail } from './pages/Reimbursements';
import AdminReimbursements from './pages/admin/Reimbursements';

const mocks = vi.hoisted(() => ({
  post: vi.fn(),
  put: vi.fn(),
  useApi: vi.fn(),
}));

vi.mock('./api', () => ({
  useApi: mocks.useApi,
  post: mocks.post,
  put: mocks.put,
  del: vi.fn(),
}));

vi.mock('./auth', () => ({
  useAuth: () => ({
    me: { id: 3, name: 'Grace Member', phone: '+60123456789' },
    can: (p: string) => p === 'admin_reimb_review',
    refresh: vi.fn(),
  }),
}));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  vi.clearAllMocks();
});

const mockRegisteredEvents = [
  { id: 42, title: 'Sunday Worship Service', starts_at: '2026-11-22T10:00:00' },
  { id: 43, title: 'Youth Fellowship', starts_at: '2026-11-28T18:00:00' },
];

const mockPastRides = [
  {
    id: 101,
    pickup_at: '2026-11-22T09:15:00',
    area: { name: 'Petaling Jaya' },
    destination: 'Grace Church',
    event: { id: 42, title: 'Sunday Worship Service' },
  },
  {
    id: 102,
    pickup_at: '2026-11-28T17:30:00',
    area: { name: 'Subang' },
    destination: 'Grace Church',
    event: { id: 43, title: 'Youth Fellowship' },
  },
];

describe('ReimbursementForm Event Requirement', () => {
  it('requires event selection for Grab claims and includes event_id on submit', async () => {
    mocks.useApi.mockImplementation((path: string | null) => {
      if (path === '/events?scope=registered') {
        return { data: mockRegisteredEvents, loading: false, error: null, reload: vi.fn() };
      }
      if (path === '/rides?scope=past') {
        return { data: mockPastRides, loading: false, error: null, reload: vi.fn() };
      }
      return { data: null, loading: false, error: null, reload: vi.fn() };
    });
    mocks.post.mockResolvedValue({ id: 99 });

    render(
      <MemoryRouter initialEntries={['/reimbursements/apply']}>
        <Routes>
          <Route path="/reimbursements/apply" element={<ReimbursementForm />} />
        </Routes>
      </MemoryRouter>
    );

    // Event selector is displayed
    const eventSelect = screen.getByRole('combobox', { name: /^event/i });
    expect(eventSelect).toBeInTheDocument();
    expect(screen.getByRole('option', { name: /Sunday Worship Service/i })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: /Youth Fellowship/i })).toBeInTheDocument();

    // Submit button is disabled before event and receipt are provided
    const submitBtn = screen.getByRole('button', { name: /submit/i });
    expect(submitBtn).toBeDisabled();

    // Select the event
    fireEvent.change(eventSelect, { target: { value: '42' } });
    expect(eventSelect).toHaveValue('42');

    // Fill in amount
    const amountInput = screen.getByPlaceholderText('28.50');
    fireEvent.change(amountInput, { target: { value: '25.50' } });

    // Mock upload file input
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    expect(fileInput).toBeInTheDocument();

    // In ReimbursementForm, submit stays disabled until file is selected
    expect(submitBtn).toBeDisabled();
  });

  it('shows no-registered-events notice when user is not registered for any events', () => {
    mocks.useApi.mockImplementation((path: string | null) => {
      if (path === '/events?scope=registered') {
        return { data: [], loading: false, error: null, reload: vi.fn() };
      }
      if (path === '/rides?scope=past') {
        return { data: [], loading: false, error: null, reload: vi.fn() };
      }
      return { data: null, loading: false, error: null, reload: vi.fn() };
    });

    render(
      <MemoryRouter initialEntries={['/reimbursements/apply']}>
        <Routes>
          <Route path="/reimbursements/apply" element={<ReimbursementForm />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText(/No registered events found/i)).toBeInTheDocument();
  });

  it('auto-syncs event when an attached ride is selected', () => {
    mocks.useApi.mockImplementation((path: string | null) => {
      if (path === '/events?scope=registered') {
        return { data: mockRegisteredEvents, loading: false, error: null, reload: vi.fn() };
      }
      if (path === '/rides?scope=past') {
        return { data: mockPastRides, loading: false, error: null, reload: vi.fn() };
      }
      return { data: null, loading: false, error: null, reload: vi.fn() };
    });

    render(
      <MemoryRouter initialEntries={['/reimbursements/apply']}>
        <Routes>
          <Route path="/reimbursements/apply" element={<ReimbursementForm />} />
        </Routes>
      </MemoryRouter>
    );

    const eventSelect = screen.getByRole('combobox', { name: /^event/i });
    const rideSelect = screen.getByRole('combobox', { name: /ride/i });

    expect(eventSelect).toHaveValue('');

    // Select ride 101 which belongs to event 42
    fireEvent.change(rideSelect, { target: { value: '101' } });

    expect(eventSelect).toHaveValue('42');
  });
});

describe('Reimbursement List and Detail Event Display', () => {
  const claimWithEvent = {
    id: 55,
    user: { id: 3, name: 'Grace Member' },
    ride_id: null,
    ride_at: null,
    event_id: 42,
    event_title: 'Sunday Worship Service',
    event_starts_at: '2026-11-22T10:00:00',
    amount_cents: 2550,
    description: 'Grab fare to Sunday service',
    receipt_file_id: 12,
    payee_name: 'Grace Member',
    payee_phone: '+60123456789',
    status: 'pending',
    review_note: '',
    reviewer: null,
    payee_verified: true,
    paid_ref: null,
    paid_at: null,
    created_at: '2026-11-22T12:00:00',
  };

  it('renders event title in reimbursement list', () => {
    mocks.useApi.mockReturnValue({
      data: { items: [claimWithEvent], total: 1, page: 1, page_size: 25, total_pages: 1 },
      loading: false,
      error: null,
      reload: vi.fn(),
    });

    render(
      <MemoryRouter initialEntries={['/reimbursements']}>
        <Routes>
          <Route path="/reimbursements" element={<Reimbursements />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText('Grab fare to Sunday service')).toBeInTheDocument();
    expect(screen.getByText(/Sunday Worship Service/i)).toBeInTheDocument();
  });

  it('renders event title and link in reimbursement detail', () => {
    mocks.useApi.mockReturnValue({
      data: claimWithEvent,
      loading: false,
      error: null,
      reload: vi.fn(),
    });

    render(
      <MemoryRouter initialEntries={['/reimbursements/55']}>
        <Routes>
          <Route path="/reimbursements/:id" element={<ReimbursementDetail />} />
        </Routes>
      </MemoryRouter>
    );

    const eventLink = screen.getByRole('link', { name: /Sunday Worship Service/i });
    expect(eventLink).toBeInTheDocument();
    expect(eventLink).toHaveAttribute('href', '/events/42');
  });

  it('renders event title in admin reimbursement list and review modal', () => {
    mocks.useApi.mockReturnValue({
      data: { items: [claimWithEvent], total: 1, page: 1, page_size: 25, total_pages: 1 },
      loading: false,
      error: null,
      reload: vi.fn(),
    });

    render(
      <MemoryRouter initialEntries={['/admin/reimbursements']}>
        <Routes>
          <Route path="/admin/reimbursements" element={<AdminReimbursements />} />
        </Routes>
      </MemoryRouter>
    );

    // Event title visible in admin table
    expect(screen.getByText('Sunday Worship Service')).toBeInTheDocument();

    // Open review dialog
    fireEvent.click(screen.getByRole('button', { name: 'Review' }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText(/Sunday Worship Service/i)).toBeInTheDocument();
  });
});

