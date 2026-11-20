import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import PlanTransport from './PlanTransport';
import { driverLoad, Plan, PlanVisitor } from './planLogic';
import * as apiMod from '../../api';
import * as authMod from '../../auth';
import i18n from '../../i18n';

vi.mock('../../auth', () => ({ useAuth: vi.fn() }));

const visitor = (over: Partial<PlanVisitor>): PlanVisitor => ({
  id: 1,
  name: 'Mr Tan',
  phone: '',
  notes: '',
  area: { id: 1, name: 'SS2' },
  pickup_address: '',
  seats: 1,
  leg: 'outbound',
  driver_id: null,
  driver_name: null,
  created_by: { id: 2, name: 'Bob' },
  created_at: '',
  updated_at: '',
  can_edit: true,
  ...over,
});

const plan: Plan = {
  event: { id: 5, title: 'Sunday Service', starts_at: '2026-11-22T09:00:00', location: 'Hall', status: 'published' },
  rides: [
    {
      id: 11, status: 'pending_driver', pickup_at: '2026-11-22T08:15:00', pickup_address: '10 Jalan SS2', seats: 1,
      assistance: '', notes: '', driver_id: null, area: { id: 1, name: 'SS2' }, passenger: { id: 20, name: 'Uncle Lim' }, last: null,
    },
  ],
  drivers: [
    { id: 30, name: 'Driver Grace', carpool: true, cap: 4, other_seats: 0, other_rides: 0, area_ids: [1], vehicle: { model: 'Innova', plate: 'VAA 1111', seats: 4 } },
  ],
  visitors: [
    visitor({ id: 1, name: 'Mr Tan', seats: 2, driver_id: 30, driver_name: 'Driver Grace' }),
    visitor({ id: 2, name: 'Auntie Ho', leg: 'return', created_by: { id: 9, name: 'Other' } }),
  ],
};

const show = () =>
  render(
    <MemoryRouter initialEntries={['/admin/events/5/plan']}>
      <Routes>
        <Route path="/admin/events/:id/plan" element={<PlanTransport />} />
      </Routes>
    </MemoryRouter>,
  );

describe('Visitors on the transport plan', () => {
  beforeEach(async () => {
    vi.restoreAllMocks();
    await i18n.changeLanguage('en');
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 2, name: 'Bob', roles: ['traffic_head'] },
    });
    // Stable answers: a fresh object every render would reset the board forever.
    const areas = { data: [{ id: 1, name: 'SS2', active: true }], error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() };
    const board = { data: { ...plan, can_approve: true, draft: null }, error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() };
    vi.spyOn(apiMod, 'useApi').mockImplementation(((path: string | null) =>
      path === '/areas' ? areas : board) as unknown as typeof apiMod.useApi);
  });

  it('lists visitors marked as such, counts them, and fills their car', () => {
    show();
    const card = screen.getByRole('heading', { name: /Visitors \(for the record\)/ }).closest('.plan-visitors') as HTMLElement;
    expect(within(card).getByText('Mr Tan')).toBeInTheDocument();
    expect(within(card).getByText('Auntie Ho')).toBeInTheDocument();
    expect(within(card).getAllByText('Visitor')).toHaveLength(2);
    expect(within(card).getByText("SS2 · 2 passengers · In Driver Grace's car")).toBeInTheDocument();
    expect(screen.getAllByText(/2 visitors, 3 seats/).length).toBeGreaterThan(0);
    // The traffic lead may change any visitor.
    expect(within(card).getByRole('button', { name: 'Edit Mr Tan' })).toBeInTheDocument();
    expect(within(card).getByRole('button', { name: 'Remove Auntie Ho' })).toBeInTheDocument();
    // Mr Tan's two seats count in Driver Grace's car.
    expect(screen.getByText('2 / 4 seats')).toBeInTheDocument();
  });

  it('adds a visitor for the record and refreshes the list without touching the draft', async () => {
    const post = vi.spyOn(apiMod, 'post').mockResolvedValue({ id: 3 });
    const get = vi.spyOn(apiMod, 'api').mockResolvedValue({ visitors: [] });
    show();
    await userEvent.click(screen.getByRole('button', { name: 'Add visitor' }));
    const dialog = screen.getByRole('dialog');
    await userEvent.type(within(dialog).getByLabelText('Name'), 'Sister Mei');
    await userEvent.selectOptions(within(dialog).getByLabelText('Riding with'), '30');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add visitor' }));
    await waitFor(() =>
      expect(post).toHaveBeenCalledWith('/admin/events/5/plan/visitors', expect.objectContaining({
        name: 'Sister Mei', seats: 1, leg: 'outbound', driver_id: 30, area_id: null,
      })),
    );
    await waitFor(() => expect(get).toHaveBeenCalledWith('/admin/events/5/plan/visitors'));
  });

  it('lets plan staff below the traffic lead see visitors but not add or change them', () => {
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 3, name: 'Amina', roles: ['traffic_member'] },
    });
    const board = {
      data: { ...plan, can_approve: false, draft: null, visitors: plan.visitors!.map((v) => ({ ...v, can_edit: false })) },
      error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn(),
    };
    vi.spyOn(apiMod, 'useApi').mockReturnValue(board as unknown as ReturnType<typeof apiMod.useApi>);
    show();
    expect(screen.getAllByText('Mr Tan').length).toBeGreaterThan(0);
    expect(screen.getByText('Only the traffic lead can add or change visitors.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Add visitor' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit Mr Tan' })).not.toBeInTheDocument();
  });

  it('counts visitor seats per trip in the driver load', () => {
    const d = plan.drivers[0];
    expect(driverLoad(plan, {}, d, 'outbound').seats).toBe(2);
    expect(driverLoad(plan, {}, d, 'return').seats).toBe(0);
    const full = { ...plan, visitors: [visitor({ seats: 4, driver_id: 30 })] };
    expect(driverLoad(full, { 11: 30 }, d).over).toBe(true);
  });
});
