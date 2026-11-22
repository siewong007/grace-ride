/* eslint-disable @typescript-eslint/no-explicit-any */
import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import AdminRides from './pages/admin/Rides';
import Members from './pages/admin/Members';
import AdminReimbursements from './pages/admin/Reimbursements';
import PlanTransport from './pages/admin/PlanTransport';
import AreaFares from './pages/admin/AreaFares';
import QuickJump from './components/QuickJump';
import * as apiMod from './api';
import * as authMod from './auth';

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

const mockRides = [
  {
    id: 101,
    status: 'pending_approval',
    pickup_at: '2026-11-22T08:30:00',
    seats: 1,
    destination: 'Grace Church',
    pickup_address: '12 Jalan Maju',
    area: { id: 1, name: 'PJ' },
    passenger: { id: 10, name: 'Brother Barnabas' },
    driver: { id: 2, name: 'Sister Phoebe' },
    needs_attention: false,
  },
  {
    id: 102,
    status: 'confirmed',
    pickup_at: '2026-11-22T09:00:00',
    seats: 2,
    destination: 'Grace Church',
    pickup_address: '14 Jalan SS2',
    area: { id: 1, name: 'PJ' },
    passenger: { id: 11, name: 'Sister Lydia' },
    driver: { id: 3, name: 'Brother Silas' },
    needs_attention: false,
  },
];

describe('Admin UX Enhancements', () => {
  it('AdminRides provides quick date preset chips and batch selection toolbar', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Admin Paul', roles: ['admin'] },
      refresh: vi.fn(),
    } as any);

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        items: mockRides,
        total: 2,
        page: 1,
        page_size: 50,
        total_pages: 1,
      },
      error: null,
      loading: false,
      reload: vi.fn(),
    } as any);

    const postSpy = vi.spyOn(apiMod, 'post').mockResolvedValue({ ok: true });

    render(
      <MemoryRouter initialEntries={['/admin/rides']}>
        <AdminRides />
      </MemoryRouter>,
    );

    // Quick date chips
    expect(screen.getByRole('button', { name: /^Today/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^This Sunday/i })).toBeInTheDocument();

    // Checkbox select all
    const selectAllBox = screen.getByRole('checkbox', { name: /select all/i });
    expect(selectAllBox).toBeInTheDocument();

    // Batch bar is hidden initially
    expect(screen.queryByRole('region', { name: /batch actions/i })).not.toBeInTheDocument();

    // Select first ride
    const rowCheckboxes = screen.getAllByRole('checkbox', { name: /select ride/i });
    expect(rowCheckboxes).toHaveLength(2);
    fireEvent.click(rowCheckboxes[0]);

    // Batch bar appears
    const batchBar = screen.getByRole('region', { name: /batch actions/i });
    expect(batchBar).toBeInTheDocument();
    expect(within(batchBar).getByText(/1 ride selected/i)).toBeInTheDocument();

    // Approve selected button appears because ride 101 is pending_approval
    const approveBtn = within(batchBar).getByRole('button', { name: /approve selected \(1\)/i });
    expect(approveBtn).toBeInTheDocument();

    fireEvent.click(approveBtn);
    await waitFor(() => {
      expect(postSpy).toHaveBeenCalledWith('/rides/101/approve');
    });

    // Clear selection
    const clearBtn = within(batchBar).getByRole('button', { name: /clear selection/i });
    fireEvent.click(clearBtn);
    expect(screen.queryByRole('region', { name: /batch actions/i })).not.toBeInTheDocument();
  });

  it('Members invite tab provides WhatsApp direct share buttons', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Admin Paul', roles: ['admin'] },
      refresh: vi.fn(),
    } as any);

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: [
        {
          id: 50,
          email: 'newfriend@grace.org',
          url: 'https://graceride.my/invite/token123',
          kind: 'invite',
          roles: ['member'],
          status: 'open',
          created_at: '2026-11-20T10:00:00',
        },
      ],
      loading: false,
      reload: vi.fn(),
    } as any);

    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);

    render(
      <MemoryRouter initialEntries={['/admin/members?tab=invites']}>
        <Members />
      </MemoryRouter>,
    );

    // Open invites list has WhatsApp share button
    const whatsappBtn = screen.getByRole('button', { name: /share via whatsapp/i });
    expect(whatsappBtn).toBeInTheDocument();

    fireEvent.click(whatsappBtn);
    expect(openSpy).toHaveBeenCalledWith(
      expect.stringContaining('https://wa.me/?text='),
      '_blank',
      'noopener,noreferrer',
    );
  });

  it('Reimbursements review modal provides interactive receipt zoom and DuitNow indicator', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Finance Grace', roles: ['finance'] },
      refresh: vi.fn(),
    } as any);

    const mockReimb = {
      id: 77,
      user: { id: 5, name: 'Brother David' },
      amount_cents: 2500,
      description: 'Sunday Dispersal Ride',
      status: 'pending',
      created_at: '2026-11-20T12:00:00',
      receipt_file_id: 999,
      kind: 'mileage',
      payee_name: 'David Lim',
      payee_phone: '+60123456789',
    };

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: {
        items: [mockReimb],
        total: 1,
        page: 1,
        page_size: 25,
        total_pages: 1,
      },
      error: null,
      loading: false,
      reload: vi.fn(),
    } as any);

    render(
      <MemoryRouter initialEntries={['/admin/reimbursements?tab=review']}>
        <AdminReimbursements />
      </MemoryRouter>,
    );

    // Open review modal
    const reviewBtn = screen.getByRole('button', { name: /review/i });
    fireEvent.click(reviewBtn);

    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText(/receipt inspection/i)).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: /zoom in/i })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: /zoom out/i })).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: /rotate/i })).toBeInTheDocument();

    // DuitNow badge
    expect(within(dialog).getByText('DuitNow Mobile')).toBeInTheDocument();
  });

  it('PlanTransport displays floating dock when unsaved changes exist', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Lead Zhang', roles: ['traffic_head'] },
      refresh: vi.fn(),
    } as any);

    const planPayload = {
      event: { id: 5, title: 'Sunday Worship', starts_at: '2026-11-22T09:00:00', location: 'Grace Hall', group_ids: [1] },
      rides: [
        {
          id: 11,
          status: 'pending_driver',
          pickup_at: '2026-11-22T08:15:00',
          pickup_address: '10 Jalan SS2',
          seats: 1,
          assistance: '',
          driver_id: null,
          area: { id: 1, name: 'SS2' },
          passenger: { id: 20, name: 'Uncle Lim' },
          match_group_id: 1,
          last: null,
        },
      ],
      drivers: [
        {
          id: 30,
          name: 'Driver Grace',
          carpool: true,
          cap: 4,
          other_seats: 0,
          area_ids: [1],
          group_ids: [1],
          vehicle: { model: 'Innova', plate: 'VAA 1111' },
        },
      ],
    };

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: planPayload,
      error: null,
      loading: false,
      reload: vi.fn(),
    } as any);

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    // Floating dock is not present before changes
    expect(screen.queryByRole('complementary', { name: /unsaved transport plan changes/i })).not.toBeInTheDocument();

    // Assign Uncle Lim to Driver Grace
    const select = screen.getByRole('combobox', { name: /driver for uncle lim/i });
    fireEvent.change(select, { target: { value: '30' } });

    // Floating dock appears
    const dock = screen.getByRole('complementary', { name: /unsaved transport plan changes/i });
    expect(dock).toBeInTheDocument();
    expect(within(dock).getByText(/1 unsaved change/i)).toBeInTheDocument();
    expect(within(dock).getByRole('button', { name: /quick approve/i })).toBeInTheDocument();
  });

  it('AreaFares provides 2D Matrix grid view and inline cell editing', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Admin', roles: ['admin'] },
      refresh: vi.fn(),
    } as any);

    const mockAreas = [
      { id: 1, name: 'Cheras', active: true },
      { id: 2, name: 'PJ', active: true },
    ];
    const mockFares = [
      { id: 10, area_a: { id: 1, name: 'Cheras' }, area_b: { id: 2, name: 'PJ' }, amount_cents: 1500, updated_at: '' },
    ];

    vi.spyOn(apiMod, 'useApi').mockImplementation((path: string | null) => {
      if (path === '/areas') return { data: mockAreas, loading: false, error: null, reload: vi.fn() } as any;
      if (path === '/admin/settings/fares') return { data: { fares: mockFares }, loading: false, error: null, reload: vi.fn() } as any;
      return { data: null, loading: false, error: null, reload: vi.fn() } as any;
    });

    const putSpy = vi.spyOn(apiMod, 'put').mockResolvedValue({ ok: true });

    render(
      <MemoryRouter>
        <AreaFares />
      </MemoryRouter>,
    );

    // Initial list view
    expect(screen.getByText('1 of 3 routes priced')).toBeInTheDocument();
    const matrixBtn = screen.getByRole('tab', { name: '2D Matrix grid' });
    expect(matrixBtn).toBeInTheDocument();

    // Switch to 2D Matrix Grid view
    fireEvent.click(matrixBtn);
    expect(screen.getByRole('region', { name: '2D Matrix grid' })).toBeInTheDocument();

    // Verify matrix headers and cells
    expect(screen.getByText('From \\ To')).toBeInTheDocument();
    expect(screen.getAllByText('RM 15.00').length).toBeGreaterThan(0);

    // Click on cell to edit
    const cell = screen.getAllByRole('button', { name: 'RM 15.00' })[0];
    fireEvent.click(cell);

    // Fare editor form is active
    const input = screen.getByLabelText('Fare for Cheras ↔ PJ');
    expect(input).toHaveValue(15);
    fireEvent.change(input, { target: { value: '22.00' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(putSpy).toHaveBeenCalledWith('/admin/settings/fares', {
        area_a_id: 1,
        area_b_id: 2,
        amount_cents: 2200,
      });
    });
  });

  it('Members provides fast-triage batch approval for pending members', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Admin', roles: ['superadmin'] },
      refresh: vi.fn(),
    } as any);

    const pendingUsers = [
      { id: 101, name: 'New Friend A', email: 'a@church.org', phone: '0123456789', roles: [], status: 'pending', created_at: '2026-11-20', passkeys: 0, groups: '' },
      { id: 102, name: 'New Friend B', email: 'b@church.org', phone: '0123456788', roles: [], status: 'pending', created_at: '2026-11-20', passkeys: 0, groups: '' },
    ];

    vi.spyOn(apiMod, 'useApi').mockImplementation((path: string | null) => {
      if (path?.startsWith('/admin/users')) {
        return { data: { items: pendingUsers, total: 2, page: 1, page_size: 50, total_pages: 1 }, loading: false, error: null, reload: vi.fn() } as any;
      }
      if (path === '/groups') {
        return { data: [{ id: 5, name: 'Youth Fellowship', is_all: false }], loading: false, error: null, reload: vi.fn() } as any;
      }
      if (path?.startsWith('/admin/roles')) {
        return { data: { roles: [] }, loading: false, error: null, reload: vi.fn() } as any;
      }
      return { data: [], loading: false, error: null, reload: vi.fn() } as any;
    });

    const putSpy = vi.spyOn(apiMod, 'put').mockResolvedValue({ ok: true });

    render(
      <MemoryRouter initialEntries={['/admin/members']}>
        <Members />
      </MemoryRouter>,
    );

    // Select all members via table header checkbox
    const selectAllCheckbox = screen.getByRole('checkbox', { name: /select all/i });
    fireEvent.click(selectAllCheckbox);

    // Floating batch bar appears
    expect(screen.getByText('2 members selected')).toBeInTheDocument();
    const batchBtn = screen.getByRole('button', { name: /batch approve \(2\)/i });
    fireEvent.click(batchBtn);

    // Modal dialog opens
    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText('Batch Approve Members')).toBeInTheDocument();
    expect(within(dialog).getByText('New Friend A')).toBeInTheDocument();
    expect(within(dialog).getByText('New Friend B')).toBeInTheDocument();

    // Select Youth Fellowship group
    const groupChip = within(dialog).getByRole('button', { name: 'Youth Fellowship' });
    fireEvent.click(groupChip);

    // Submit batch approval
    const confirmBtn = within(dialog).getByRole('button', { name: /approve all selected \(2\)/i });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(putSpy).toHaveBeenCalledTimes(2);
      expect(putSpy).toHaveBeenCalledWith('/admin/users/101', expect.objectContaining({
        status: 'active',
        group_ids: [5],
      }));
      expect(putSpy).toHaveBeenCalledWith('/admin/users/102', expect.objectContaining({
        status: 'active',
        group_ids: [5],
      }));
    });
  });

  it('Reimbursements renders the 4-stage Maker-Checker payout stepper', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: (perm: string) => perm === 'admin_reimb_pay',
      me: { id: 1, name: 'Treasurer Sarah', roles: ['admin'] },
      refresh: vi.fn(),
    } as any);

    const mockBatches = [
      {
        id: 77,
        month: '2026-11',
        title: 'Nov 2026 Batch #1',
        total_cents: 8500,
        claim_count: 3,
        status: 'open',
        paid_ref: '',
        created_by: 'Deacon Philip',
        paid_by: null,
        paid_at: null,
      },
    ];

    vi.spyOn(apiMod, 'useApi').mockImplementation((path: string | null) => {
      if (path === '/reimbursements/batches') {
        return { data: mockBatches, loading: false, error: null, reload: vi.fn() } as any;
      }
      return { data: [], loading: false, error: null, reload: vi.fn() } as any;
    });

    render(
      <MemoryRouter initialEntries={['/admin/reimbursements?tab=payout']}>
        <AdminReimbursements />
      </MemoryRouter>,
    );

    // Verify stepper elements
    expect(screen.getByText('Nov 2026 Batch #1')).toBeInTheDocument();
    const stepper = screen.getByRole('region', { name: 'Batch progress' });
    expect(stepper).toBeInTheDocument();
    expect(within(stepper).getByText('Batch created')).toBeInTheDocument();
    expect(within(stepper).getByText('Bank CSV')).toBeInTheDocument();
    expect(within(stepper).getByText('Bank transfer')).toBeInTheDocument();
    expect(within(stepper).getByText('Checker signed')).toBeInTheDocument();
  });

  it('PlanTransport opens Explainability Drawer with bottleneck diagnosis and driver recommendations', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Lead Zhang', roles: ['traffic_head'] },
      refresh: vi.fn(),
    } as any);

    const planPayload = {
      event: { id: 5, title: 'Sunday Worship', starts_at: '2026-11-22T09:00:00', location: 'Grace Hall', group_ids: [1] },
      rides: [
        {
          id: 11,
          status: 'pending_driver',
          pickup_at: '2026-11-22T08:15:00',
          pickup_address: '10 Jalan SS2',
          seats: 1,
          assistance: 'Wheelchair assistance required',
          driver_id: null,
          area: { id: 1, name: 'SS2' },
          passenger: { id: 20, name: 'Grandma Mary' },
          match_group_id: 1,
          last: null,
        },
      ],
      drivers: [
        {
          id: 30,
          name: 'Driver Grace',
          carpool: true,
          cap: 4,
          other_seats: 0,
          area_ids: [1],
          group_ids: [1],
          vehicle: { model: 'Innova', plate: 'VAA 1111' },
        },
      ],
    };

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: planPayload,
      error: null,
      loading: false,
      reload: vi.fn(),
    } as any);

    render(
      <MemoryRouter initialEntries={['/admin/events/5/plan']}>
        <Routes>
          <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
        </Routes>
      </MemoryRouter>,
    );

    // Click on Insights & Diagnosis button
    const diagBtn = screen.getByRole('button', { name: /Insights & Diagnosis/i });
    fireEvent.click(diagBtn);

    // Modal opens
    const modal = screen.getByRole('dialog');
    expect(modal).toBeInTheDocument();
    expect(within(modal).getByText(/Auto-Arrange Insights & Diagnosis/i)).toBeInTheDocument();
    expect(within(modal).getByText('Grandma Mary')).toBeInTheDocument();
    expect(within(modal).getByText(/Passenger requires special assistance: Wheelchair assistance required/i)).toBeInTheDocument();

    // Manual driver assignment in drawer
    const assignSelect = within(modal).getByRole('combobox', { name: /Driver for Grandma Mary/i });
    expect(assignSelect).toBeInTheDocument();
    fireEvent.change(assignSelect, { target: { value: '30' } });

    // Drawer closes and board reflects update
    fireEvent.click(within(modal).getAllByRole('button', { name: /Close/i })[0]);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('QuickJump supports > prefix and provides instant administrative quick action jumps', async () => {
    vi.spyOn(authMod, 'useAuth').mockReturnValue({
      can: () => true,
      me: { id: 1, name: 'Admin', roles: ['superadmin'] },
      refresh: vi.fn(),
    } as any);

    vi.spyOn(apiMod, 'useApi').mockReturnValue({
      data: [],
      loading: false,
      error: null,
      reload: vi.fn(),
    } as any);

    const onClose = vi.fn();

    render(
      <MemoryRouter>
        <QuickJump pages={[{ to: '/admin/dashboard', label: 'Dashboard', icon: null }]} onClose={onClose} />
      </MemoryRouter>,
    );

    const input = screen.getByRole('combobox');

    // Entering '>' enters command mode and lists quick actions
    fireEvent.change(input, { target: { value: '>' } });
    expect(screen.getByText('Quick Actions')).toBeInTheDocument();
    expect(screen.getByText('Approve pending rides')).toBeInTheDocument();
    expect(screen.getByText('Review pending member registrations')).toBeInTheDocument();
    expect(screen.getByText('Audit driver travel reimbursements')).toBeInTheDocument();
    expect(screen.getByText('Monthly payout batches & bank transfers')).toBeInTheDocument();
    expect(screen.getByText('2D Area fares matrix & pricing')).toBeInTheDocument();

    // Filtering by action keyword
    fireEvent.change(input, { target: { value: '>payout' } });
    expect(screen.getByText('Monthly payout batches & bank transfers')).toBeInTheDocument();
    expect(screen.queryByText('Approve pending rides')).not.toBeInTheDocument();

    // Clicking an action navigates and closes
    fireEvent.click(screen.getByText('Monthly payout batches & bank transfers'));
    expect(onClose).toHaveBeenCalled();
  });
});
