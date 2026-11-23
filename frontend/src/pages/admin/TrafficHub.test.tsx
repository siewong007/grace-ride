import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import TrafficHub from './TrafficHub';
import * as apiMod from '../../api';
import * as authMod from '../../auth';
import i18n from '../../i18n';

vi.mock('../../auth', () => ({
  useAuth: vi.fn(),
}));

const samplePlans = [
  {
    id: 10,
    title: 'Sunday Worship',
    kind: 'sunday',
    icon: null,
    starts_at: '2099-12-20T09:00:00',
    location: 'Main Sanctuary',
    status: 'published',
    area_name: 'SS2',
    groups: 'Youth · Adults',
    mileage_claimable: true,
    ride_count: 6,
    unassigned_count: 2,
    pending_confirm_count: 1,
    visitor_count: 1,
    draft_count: 1,
    pending_draft_count: 1,
    returned_draft_count: 0,
    latest_draft: {
      id: 3,
      event_id: 10,
      version: 1,
      label: 'Primary Plan',
      status: 'pending_approval',
      created_by: { id: 2, name: 'Member Bob' },
      updated_by: { id: 2, name: 'Member Bob' },
      updated_at: '2099-12-19T12:00:00',
      submitted_by: { id: 2, name: 'Member Bob' },
      locked_by: { id: 2, name: 'Member Bob' },
      lock_is_mine: false,
    },
  },
  {
    id: 11,
    title: 'Friday Cell',
    kind: 'cell',
    icon: null,
    starts_at: '2099-12-26T20:00:00',
    location: 'Hall B',
    status: 'published',
    area_name: 'Damansara',
    groups: 'Cell East',
    mileage_claimable: false,
    ride_count: 2,
    unassigned_count: 0,
    pending_confirm_count: 0,
    visitor_count: 0,
    draft_count: 0,
    pending_draft_count: 0,
    returned_draft_count: 0,
    latest_draft: null,
  },
];

const sampleRideDrafts = [
  {
    id: 5,
    ride_id: 22,
    version: 1,
    action: 'reassign',
    driver_id: 31,
    driver_name: 'Driver Grace',
    reason: 'Closer to pickup gate',
    reimbursement_excluded: null,
    status: 'pending_approval',
    created_by: { id: 2, name: 'Member Bob' },
    updated_by: { id: 2, name: 'Member Bob' },
    submitted_by: { id: 2, name: 'Member Bob' },
    submitted_at: '2099-12-19T13:00:00',
    review_notes: '',
    locked_by: null,
    ride: {
      id: 22,
      status: 'confirmed',
      pickup_at: '2099-12-20T08:15:00',
      passenger_name: 'Auntie Lim',
      area_name: 'SS2',
      current_driver_name: 'Brother Lim',
      event_id: 10,
      event_title: 'Sunday Worship',
    },
  },
];

describe('TrafficHub workspace', () => {
  beforeEach(async () => {
    vi.restoreAllMocks();
    await i18n.changeLanguage('en');
    (authMod.useAuth as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      can: () => true,
      me: { id: 18, name: 'Lead Zhang', roles: ['traffic_head'] },
    });
    vi.spyOn(apiMod, 'useApi').mockImplementation(((path: string | null) => {
      if (path === '/admin/dashboard') {
        return {
          data: {
            pending_approval: 1,
            no_driver: 2,
            open_help: 0,
            pending_reimbursements: 0,
            pending_recovery: 0,
            open_invites: 0,
            awaiting_payment: 0,
            pending_plan_drafts: 1,
            pending_ride_drafts: 1,
            my_returned_plan_drafts: 0,
            my_returned_ride_drafts: 0,
            live_drivers: 1,
          },
          error: null,
          loading: false,
          refreshing: false,
          reload: vi.fn(),
          setData: vi.fn(),
        };
      }
      if (path === '/admin/traffic/plans') {
        return { data: samplePlans, error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() };
      }
      if (path === '/admin/ride-drafts') {
        return { data: sampleRideDrafts, error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() };
      }
      return {
        data: { items: [], total: 0, page: 1, page_size: 50, total_pages: 1 },
        error: null,
        loading: false,
        refreshing: false,
        reload: vi.fn(),
        setData: vi.fn(),
      };
    }) as unknown as typeof apiMod.useApi);
  });

  it('renders Lead pending banner, pinned Next Event card, and plan filter chips', async () => {
    render(
      <MemoryRouter initialEntries={['/admin/traffic']}>
        <Routes>
          <Route path="/admin/traffic" element={<TrafficHub />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText(/2 pending draft\(s\) waiting for your approval/i)).toBeInTheDocument();
    const nextCard = document.querySelector('.traffic-next-event') as HTMLElement;
    expect(within(nextCard).getByText('Next event')).toBeInTheDocument();
    expect(within(nextCard).getByText('Sunday Worship')).toBeInTheDocument();
    expect(within(nextCard).getByText('Editing: Member Bob')).toBeInTheDocument();
    expect(within(nextCard).getByRole('link', { name: /review draft/i })).toHaveAttribute(
      'href',
      '/admin/events/10/plan?draft=3',
    );

    await userEvent.click(screen.getByRole('button', { name: 'Pending approval' }));
    expect(screen.queryByText('Friday Cell')).not.toBeInTheDocument();
    expect(screen.getAllByText('Sunday Worship').length).toBeGreaterThan(0);
  });

  it('lets Traffic Lead approve and return single-ride change drafts in Rides -> Proposed changes', async () => {
    const postSpy = vi.spyOn(apiMod, 'post').mockResolvedValue({ ok: true });

    render(
      <MemoryRouter initialEntries={['/admin/traffic?tab=rides&view=drafts']}>
        <Routes>
          <Route path="/admin/traffic" element={<TrafficHub />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText('Auntie Lim')).toBeInTheDocument();
    expect(screen.getByText('Reassign to Driver Grace')).toBeInTheDocument();

    // Return flow with quick-reason modal
    await userEvent.click(screen.getByRole('button', { name: 'Return' }));
    const dialog = screen.getByRole('dialog');
    await userEvent.click(within(dialog).getByRole('button', { name: /check driver seat capacity/i }));
    await userEvent.click(within(dialog).getByRole('button', { name: /return draft/i }));
    expect(postSpy).toHaveBeenCalledWith('/rides/22/drafts/5/return', {
      notes: 'Check driver seat capacity',
    });

    // Approve flow
    await userEvent.click(screen.getByRole('button', { name: /approve & apply/i }));
    expect(postSpy).toHaveBeenCalledWith('/rides/22/drafts/5/approve', {});
  });

  it('splits Areas & Photos tab into segmented sub-views without stacking all modules', async () => {
    vi.spyOn(apiMod, 'useApi').mockImplementation(((path: string | null) => {
      if (path === '/areas') {
        return { data: [{ id: 1, name: 'SS2', active: true }], error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() };
      }
      if (path === '/admin/settings/fares') {
        return { data: { fares: [] }, error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() };
      }
      return { data: null, error: null, loading: false, refreshing: false, reload: vi.fn(), setData: vi.fn() };
    }) as unknown as typeof apiMod.useApi);

    render(
      <MemoryRouter initialEntries={['/admin/traffic?tab=areas']}>
        <Routes>
          <Route path="/admin/traffic" element={<TrafficHub />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByRole('tab', { name: 'Area photos' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('heading', { name: /pickup area photos/i })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: /fixed fares between areas/i })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('tab', { name: 'Fare table' }));
    expect(screen.getByRole('tab', { name: 'Fare table' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('heading', { name: /fixed fares between areas/i })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: /pickup area photos/i })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('tab', { name: 'Areas' }));
    expect(screen.getByRole('tab', { name: 'Areas' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('heading', { name: /ride areas/i })).toBeInTheDocument();
  });
});
