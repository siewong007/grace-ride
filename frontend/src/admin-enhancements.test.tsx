import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import i18n from './i18n';
import AdminRides from './pages/admin/Rides';
import Members from './pages/admin/Members';
import AdminReimbursements from './pages/admin/Reimbursements';
import PlanTransport from './pages/admin/PlanTransport';
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
    });

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
    });

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
    });

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
    });

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
    });

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
    });

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
    });

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
    });

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
});
