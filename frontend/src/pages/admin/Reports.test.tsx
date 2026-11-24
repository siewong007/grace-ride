import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import i18n from '../../i18n';
import { put } from '../../api';
import Reports from './Reports';

const members = [
  { id: 2, name: 'Brother Silas', email: 'silas@example.org', roles: [] },
  { id: 3, name: 'Sister Phoebe', email: 'phoebe@example.org', roles: [] },
  { id: 4, name: 'Auntie Lim', email: 'lim@example.org', roles: [] },
];

const defaultReport = {
  user: { id: 1, name: 'Mary' },
  month: '2026-11',
  range: 'monthly',
  type: 'all',
  period_start: '2026-11-01',
  period_end: '2026-11-30',
  rides_driven: 1,
  rides_received: 0,
  distance_km: 5,
  reimbursement_cents: 300,
  events_attended: 0,
  by_kind: [],
  events: [],
  costs_paid_cents: 0,
  badges: [
    {
      key: 'night',
      category: 'driver',
      role_match: true,
      char: 'N',
      icon: 'moon',
      name: 'Night Owl',
      rule: 'Complete rides after 9 pm (Bronze 3 · Silver 8 · Gold 20 · Platinum 40)',
      progress: 22,
      target: 40,
      earned: true,
      tier: 'gold',
      tier_level: 3,
      next_tier: 'platinum',
      next_target: 40,
      tiers: [
        { tier: 'bronze', target: 3, reached: true },
        { tier: 'silver', target: 8, reached: true },
        { tier: 'gold', target: 20, reached: true },
        { tier: 'platinum', target: 40, reached: false },
      ],
    },
  ],
};

let authCan: (page: string, act?: string) => boolean = () => true;
let apiResolver: (path: string | null) => unknown = (path) =>
  path?.startsWith('/admin/users') ? members : path?.startsWith('/reports/') ? defaultReport : null;

vi.mock('../../api', () => ({
  put: vi.fn().mockResolvedValue({ ok: true }),
  useApi: (path: string | null) => ({
    data: apiResolver(path),
    loading: false,
    error: null,
    reload: vi.fn(),
  }),
}));

vi.mock('../../auth', () => ({
  useAuth: () => ({
    me: { id: 1 },
    can: (page: string, act?: string) => authCan(page, act),
  }),
}));

vi.mock('jspdf', () => ({ default: class {} }));

beforeAll(async () => {
  await i18n.changeLanguage('en');
});

beforeEach(() => {
  authCan = () => true;
  apiResolver = (path) =>
    path?.startsWith('/admin/users') ? members : path?.startsWith('/reports/') ? defaultReport : null;
  vi.mocked(put).mockClear();
});

describe('Member reports filters', () => {
  it('labels every filter and narrows the member list as you type', () => {
    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );
    for (const label of ['Month', 'Range', 'Type', 'Find a member', 'Select member']) {
      expect(screen.getByLabelText(label)).toBeInTheDocument();
    }
    const picker = screen.getByLabelText('Select member');
    expect(within(picker).getAllByRole('option')).toHaveLength(4); // yourself + 3 members

    fireEvent.change(screen.getByLabelText('Find a member'), { target: { value: 'phoe' } });
    const names = within(picker).getAllByRole('option').map((o) => o.textContent);
    expect(names).toEqual(['Yourself (current user)', 'Sister Phoebe (phoebe@example.org)']);
  });

  it('allows admin to change the icon used for the achievement for the user', async () => {
    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );
    const editBtn = screen.getByLabelText(/Change achievement icon - Night Owl/i);
    expect(editBtn).toBeInTheDocument();
    fireEvent.click(editBtn);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    const trophyBtn = screen.getByRole('radio', { name: 'Trophy' });
    fireEvent.click(trophyBtn);

    const saveBtn = screen.getByRole('button', { name: 'Save' });
    fireEvent.click(saveBtn);

    await waitFor(() => {
      expect(put).toHaveBeenCalledWith('/admin/users/1/badges/night/icon', { icon: 'trophy' });
    });
  });

  it('allows admin to select an adorable animated role badge icon in the modal', async () => {
    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByLabelText(/Change achievement icon - Night Owl/i));

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    const trafficLightBtn = screen.getByRole('radio', { name: 'Green-Light Captain' });
    fireEvent.click(trafficLightBtn);
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(put).toHaveBeenCalledWith('/admin/users/1/badges/night/icon', {
        icon: 'badge_traffic_light',
      });
    });
  });

  it('displays progressive tier pills, next-tier target, and milestone steps in admin reports', () => {
    const { container } = render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );
    expect(container.querySelector('.badge-char.badge-tier-gold')).toBeInTheDocument();
    expect(screen.getByText('Gold')).toBeInTheDocument();
    expect(screen.getByText('Next: Platinum (22/40)')).toBeInTheDocument();
    expect(screen.getByText('Platinum 40')).toBeInTheDocument();
  });
});

describe('Role-privileged admin report views', () => {
  it('defaults to Overview for cell_leader and hides finance tab, member picker, and badge editor', () => {
    const cellPerms = new Set([
      'admin_reports.view',
      'admin_rides.view',
      'admin_plan.view',
      'admin_members.view',
      'events.create',
    ]);
    authCan = (p, a = 'view') => cellPerms.has(`${p}.${a}`);
    apiResolver = (path) => {
      if (path === '/admin/reports/capabilities') {
        return {
          overview: true,
          member_reports_global: false,
          member_badge_icon_edit: false,
          demand: true,
          demand_scope: 'own_events',
          events: true,
          events_scope: 'own_events',
          finance: false,
          finance_sections: { reimb_review: false, reimb_payouts: false, bills: false },
          membership: false,
        };
      }
      if (path?.startsWith('/reports/')) return defaultReport;
      if (path?.startsWith('/admin/reports/overview')) {
        return {
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-11-30',
          transport: {
            scope: 'own_events',
            total_rides: 2,
            completed_rides: 1,
            unfilled_rides: 1,
            cancelled_rides: 0,
            no_show_rides: 0,
            alt_transport_rides: 0,
            fulfillment_rate_pct: 50,
            total_distance_km: 8,
            active_drivers: 1,
            unique_passengers: 2,
            top_drivers: [],
          },
          events: null,
          reimb_review: null,
          finance: null,
          membership: null,
          monthly_series: [],
        };
      }
      return null;
    };

    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );

    const overviewTab = screen.getByRole('tab', { name: 'Overview' });
    expect(overviewTab).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('My organised events only')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Member reports' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Events & groups' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Demand & shortages' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Finance & claims' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Member reports' }));
    expect(screen.queryByLabelText('Select member')).not.toBeInTheDocument();
    expect(
      screen.getByText(/view your own member activity report and analytics for events you created/i),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText(/Change achievement icon/i)).not.toBeInTheDocument();
  });

  it('shows finance-only tabs, redacted member view, and hides badges when visible_sections.badges=false', () => {
    const finPerms = new Set(['admin_reports.view', 'admin_bills.view']);
    authCan = (p, a = 'view') => finPerms.has(`${p}.${a}`);
    apiResolver = (path) => {
      if (path === '/admin/reports/capabilities') {
        return {
          overview: true,
          member_reports_global: true,
          member_badge_icon_edit: false,
          demand: false,
          demand_scope: 'none',
          events: false,
          events_scope: 'none',
          finance: true,
          finance_sections: { reimb_review: false, reimb_payouts: false, bills: true },
          membership: false,
        };
      }
      if (path?.startsWith('/admin/users')) return members;
      if (path?.startsWith('/reports/')) {
        return {
          ...defaultReport,
          events_attended: null,
          rides_received: null,
          costs_paid_cents: 2000,
          badges: [],
          visible_sections: {
            events: false,
            driving: false,
            passenger_rides: false,
            costs_paid: true,
            reimbursements: false,
            organised: false,
            badges: false,
          },
        };
      }
      return null;
    };

    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );

    expect(screen.getByRole('tab', { name: 'Member reports' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Overview' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Finance & claims' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Events & groups' })).not.toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: 'Demand & shortages' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/Change achievement icon/i)).not.toBeInTheDocument();
    expect(screen.getByPlaceholderText('Member name')).toBeInTheDocument();
    expect(screen.getByText('RM 20.00')).toBeInTheDocument();
    expect(screen.queryByText('Events attended')).not.toBeInTheDocument();
    expect(screen.queryByText('Thank-you badges')).not.toBeInTheDocument();
    expect(screen.getByText(/Some report figures are hidden/i)).toBeInTheDocument();
  });

  it('shows review-only finance section and hides badge editor for secretary', () => {
    const secPerms = new Set([
      'admin_reports.view',
      'admin_dashboard.view',
      'admin_rides.view',
      'admin_events.view',
      'admin_reimb_review.view',
      'admin_members.view',
    ]);
    authCan = (p, a = 'view') => secPerms.has(`${p}.${a}`);
    apiResolver = (path) => {
      if (path === '/admin/reports/capabilities') {
        return {
          overview: true,
          member_reports_global: true,
          member_badge_icon_edit: false,
          demand: true,
          demand_scope: 'global',
          events: true,
          events_scope: 'global',
          finance: true,
          finance_sections: { reimb_review: true, reimb_payouts: false, bills: false },
          membership: false,
        };
      }
      if (path?.startsWith('/admin/users')) return members;
      if (path?.startsWith('/reports/')) return defaultReport;
      if (path?.startsWith('/admin/reports/finance')) {
        return {
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-12-01',
          permissions: { reimb_review: true, reimb_payouts: false, bills: false },
          reimbursement_review: {
            by_status: {
              pending: { count: 2, amount_cents: 4500 },
              returned: { count: 1, amount_cents: 1200 },
              rejected: { count: 0, amount_cents: 0 },
              approved: { count: 3, amount_cents: 8000 },
              paid: { count: 1, amount_cents: 2500 },
            },
            by_kind: [{ kind: 'grab', count: 2, amount_cents: 4500 }],
            high_amount_special_count: 0,
            driver_mileage_rides: {
              eligible_count: 5,
              eligible_cents: 6000,
              excluded_count: 1,
              receipt_needed_count: 0,
            },
          },
          reimbursement_payouts: null,
          bills: null,
        };
      }
      return null;
    };

    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );

    expect(screen.queryByLabelText(/Change achievement icon/i)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'Finance & claims' }));
    expect(screen.getByText('Reimbursement review breakdown')).toBeInTheDocument();
    expect(screen.getByText(/Payout batches and member cost bills stay with Finance/i)).toBeInTheDocument();
    expect(screen.queryByText('Approved & paid reimbursements')).not.toBeInTheDocument();
    expect(screen.queryByText('Activity settlements & collection')).not.toBeInTheDocument();
  });

  it('renders all 5 tabs and switches across Overview, Events, Demand, and Finance for president/superadmin', () => {
    apiResolver = (path) => {
      if (path === '/admin/reports/capabilities') {
        return {
          overview: true,
          member_reports_global: true,
          member_badge_icon_edit: true,
          demand: true,
          demand_scope: 'global',
          events: true,
          events_scope: 'global',
          finance: true,
          finance_sections: { reimb_review: true, reimb_payouts: true, bills: true },
          membership: true,
        };
      }
      if (path?.startsWith('/admin/users')) return members;
      if (path?.startsWith('/reports/')) return defaultReport;
      if (path?.startsWith('/admin/reports/overview')) {
        return {
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-12-01',
          transport: {
            scope: 'global',
            total_rides: 10,
            completed_rides: 8,
            unfilled_rides: 1,
            cancelled_rides: 1,
            no_show_rides: 0,
            alt_transport_rides: 0,
            fulfillment_rate_pct: 80,
            total_distance_km: 120,
            active_drivers: 3,
            unique_passengers: 7,
            top_drivers: [
              { user_id: 2, name: 'Brother Silas', completed_rides: 5, distance_km: 80, carpool_trips: 2 },
            ],
          },
          events: {
            scope: 'global',
            events_count: 2,
            total_going: 30,
            total_attended: 25,
            attendance_rate_pct: 83,
            top_groups: [{ group_id: 1, group_name: 'Youth Fellowship', going: 20, attended: 18 }],
          },
          reimb_review: null,
          finance: null,
          membership: { active_members: 50, pending_members: 2, new_members_in_period: 3, active_drivers_total: 10 },
          monthly_series: [],
        };
      }
      if (path?.startsWith('/admin/reports/events')) {
        return {
          scope: 'global',
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-12-01',
          totals: {
            events_count: 2,
            cancelled_events: 0,
            total_going: 30,
            total_waitlist: 0,
            total_attended: 25,
            attendance_rate_pct: 83,
            need_ride_count: 10,
            offer_drive_count: 4,
            self_transport_count: 16,
            visitor_passes_count: 1,
            plan_visitors_count: 2,
          },
          by_group: [
            { group_id: 1, group_name: 'Youth Fellowship', going: 20, attended: 18, attendance_rate_pct: 90, need_ride: 6, self_transport: 12, offer_drive: 2 },
          ],
          by_kind: [{ kind: 'badminton', events_count: 2, going: 30, attended: 25, attendance_rate_pct: 83 }],
          events: [],
        };
      }
      if (path?.startsWith('/admin/reports/demand')) {
        return {
          scope: 'global',
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-12-01',
          totals: {
            total_requests: 10,
            fulfilled: 8,
            unfilled: 2,
            completed: 7,
            cancelled_unassigned: 0,
            wheelchair_requests: 1,
            return_requests: 3,
            seats_unfilled: 2,
            active_drivers: 3,
            unique_passengers: 8,
            fulfillment_rate_pct: 80,
          },
          by_area: [
            { area_id: 1, area_name_zh: 'SS2', area_name_en: 'SS2', total: 6, fulfilled: 5, unfilled: 1, cancelled_unassigned: 0, seats_needed: 6, seats_unfilled: 1, fulfillment_rate_pct: 83 },
          ],
          by_slot: [{ weekday: 0, slot: '08:00-10:00', total: 6, fulfilled: 5, unfilled: 1, cancelled_unassigned: 0, seats_needed: 6 }],
          by_group: [{ group_id: 1, group_name: 'Youth Fellowship', total: 6, fulfilled: 5, unfilled: 1, seats_needed: 6, fulfillment_rate_pct: 83 }],
        };
      }
      if (path?.startsWith('/admin/reports/finance')) {
        return {
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-12-01',
          permissions: { reimb_review: true, reimb_payouts: true, bills: true },
          reimbursement_review: null,
          reimbursement_payouts: {
            approved_unpaid_count: 1,
            approved_unpaid_cents: 2200,
            all_time_awaiting_count: 1,
            all_time_awaiting_cents: 2200,
            paid_count: 1,
            paid_cents: 3000,
            payee_verified_count: 2,
            payee_unverified_count: 0,
            by_kind: [{ kind: 'grab', approved_count: 1, approved_cents: 2200, paid_count: 1, paid_cents: 3000 }],
            batches: [
              { id: 1, month: '2026-11', title: 'Batch Nov', status: 'paid', claim_count: 1, total_cents: 3000, created_by_name: 'Finance A', paid_by_name: 'President B', paid_at: '2026-11-20', paid_ref: 'REF-1', maker_checker_ok: true },
            ],
            monthly_trend: [],
          },
          bills: {
            totals: {
              total_settlements: 1,
              draft_count: 0,
              published_count: 1,
              closed_count: 0,
              gross_cents: 2500,
              subsidy_cents: 500,
              net_billed_cents: 2000,
              collected_cents: 2000,
              pending_verification_cents: 0,
              needs_info_cents: 0,
              unpaid_cents: 0,
              collection_rate_pct: 100,
            },
            settlements: [],
          },
        };
      }
      return null;
    };

    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('tab', { name: 'Overview' }));
    expect(screen.getByText('Transport overview')).toBeInTheDocument();
    expect(screen.getByText('Brother Silas')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Events & groups' }));
    expect(screen.getByText('Participation by church group')).toBeInTheDocument();
    expect(screen.getByText('Youth Fellowship')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Demand & shortages' }));
    expect(screen.getByText('Demand & shortage by church group')).toBeInTheDocument();
    expect(screen.getByText('SS2')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Finance & claims' }));
    expect(screen.getByText('Batch Nov')).toBeInTheDocument();
    expect(screen.getByText('Paid')).toBeInTheDocument();
    expect(screen.getByText('Maker-checker OK')).toBeInTheDocument();
    expect(screen.getByText('Activity settlements & collection')).toBeInTheDocument();
    expect(screen.getByLabelText('Download batch CSV - Batch Nov')).toHaveAttribute(
      'href',
      '/api/payout-batches/1/export',
    );
  });

  it('supports comparison deltas, group filtering, driver supply, aging alerts, and trend charts across panels', () => {
    apiResolver = (path) => {
      if (path === '/admin/reports/capabilities') {
        return {
          overview: true,
          member_reports_global: true,
          member_badge_icon_edit: true,
          demand: true,
          demand_scope: 'global',
          events: true,
          events_scope: 'global',
          finance: true,
          finance_sections: { reimb_review: true, reimb_payouts: true, bills: true },
          membership: true,
        };
      }
      if (path === '/groups') {
        return [{ id: 1, name: 'Youth Fellowship' }, { id: 2, name: 'Adults' }];
      }
      if (path?.startsWith('/admin/users')) return members;
      if (path?.startsWith('/reports/')) return defaultReport;
      if (path?.startsWith('/admin/reports/overview')) {
        return {
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-11-30',
          comparison: {
            prev_period: '2026-10',
            transport: { total_rides_delta: 3, completed_rides_delta: 2, fulfillment_rate_delta: 5 },
            events: null,
            reimb_review: null,
            reimb_payouts: null,
            bills: null,
            membership: null,
          },
          transport: {
            scope: 'global',
            total_rides: 10,
            completed_rides: 8,
            pending_confirmation_rides: 1,
            unfilled_rides: 1,
            cancelled_rides: 0,
            no_show_rides: 0,
            alt_transport_rides: 0,
            fulfillment_rate_pct: 80,
            total_distance_km: 120,
            active_drivers: 3,
            unique_passengers: 7,
            top_drivers: [
              { user_id: 2, name: 'Brother Silas', completed_rides: 5, distance_km: 80, carpool_trips: 2 },
            ],
          },
          events: {
            scope: 'global',
            events_count: 1,
            total_going: 10,
            total_attended: 8,
            attendance_rate_pct: 80,
            top_groups: [{ group_id: 1, group_name: 'Youth Fellowship', going: 10, attended: 8 }],
          },
          reimb_review: null,
          finance: null,
          membership: null,
          monthly_series: [],
        };
      }
      if (path?.startsWith('/admin/reports/events')) {
        return {
          scope: 'global',
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-11-30',
          totals: {
            events_count: 2,
            tracked_events_count: 1,
            cancelled_events: 0,
            total_going: 20,
            total_waitlist: 0,
            total_attended: 8,
            attendance_rate_pct: 80,
            need_ride_count: 4,
            offer_drive_count: 2,
            self_transport_count: 14,
            visitor_passes_count: 0,
            plan_visitors_count: null,
          },
          by_group: [],
          by_kind: [],
          events: [
            {
              id: 10,
              title: 'Open Fellowship',
              kind: 'other',
              starts_at: '2026-11-10T19:00:00',
              status: 'published',
              creator_name: 'Mary',
              going: 10,
              waitlist: 0,
              attended: 0,
              attendance_enabled: false,
              attendance_rate_pct: null,
              need_ride: 2,
              self_transport: 8,
              rides_matched: 2,
              rides_unfilled: 0,
            },
          ],
        };
      }
      if (path?.startsWith('/admin/reports/demand')) {
        return {
          scope: 'global',
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-11-30',
          totals: {
            total_requests: 8,
            fulfilled: 6,
            pending_confirmation: 1,
            unfilled: 1,
            completed: 5,
            cancelled_unassigned: 0,
            wheelchair_requests: 1,
            stroller_requests: 2,
            oxygen_requests: 0,
            return_requests: 2,
            seats_unfilled: 1,
            active_drivers: 3,
            unique_passengers: 6,
            fulfillment_rate_pct: 75,
          },
          by_area: [
            {
              area_id: 1,
              area_name_zh: 'SS2',
              area_name_en: 'SS2',
              total: 8,
              fulfilled: 6,
              unfilled: 1,
              cancelled_unassigned: 0,
              seats_needed: 8,
              seats_unfilled: 1,
              fulfillment_rate_pct: 75,
              registered_drivers: 4,
            },
          ],
          by_slot: [],
          by_group: [],
        };
      }
      if (path?.startsWith('/admin/reports/finance')) {
        return {
          period: '2026-11',
          range: 'monthly',
          from: '2026-11-01',
          to: '2026-11-30',
          permissions: { reimb_review: true, reimb_payouts: true, bills: true },
          reimbursement_review: {
            by_status: {
              pending: { count: 2, amount_cents: 4000 },
              returned: { count: 0, amount_cents: 0 },
              rejected: { count: 0, amount_cents: 0 },
              approved: { count: 1, amount_cents: 2000 },
              paid: { count: 1, amount_cents: 3000 },
            },
            overdue_pending_count: 2,
            by_kind: [{ kind: 'grab', count: 2, amount_cents: 4000 }],
            high_amount_special_count: 0,
            driver_mileage_rides: {
              eligible_count: 1,
              eligible_cents: 1000,
              excluded_count: 0,
              receipt_needed_count: 0,
            },
          },
          reimbursement_payouts: {
            approved_unpaid_count: 1,
            approved_unpaid_cents: 2000,
            all_time_awaiting_count: 1,
            all_time_awaiting_cents: 2000,
            overdue_approved_count: 1,
            overdue_approved_cents: 2000,
            paid_count: 1,
            paid_cents: 3000,
            payee_verified_count: 2,
            payee_unverified_count: 0,
            by_kind: [],
            batches: [],
            monthly_trend: [
              { month: '2026-10', approved_cents: 1000, paid_cents: 1500, claim_count: 1 },
              { month: '2026-11', approved_cents: 2000, paid_cents: 3000, claim_count: 2 },
            ],
          },
          bills: null,
        };
      }
      return null;
    };

    render(
      <MemoryRouter>
        <Reports />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole('tab', { name: 'Overview' }));
    expect(screen.getByText('+3')).toBeInTheDocument();
    expect(screen.getByText('1 awaiting confirmation')).toBeInTheDocument();

    // Drill down from top_groups into Events tab filtered to Youth Fellowship
    fireEvent.click(screen.getByText('Youth Fellowship'));
    expect(screen.getByRole('tab', { name: 'Events & groups' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(screen.getByLabelText('Church group')).toHaveValue('1');
    expect(screen.getByText(/1 of 2 events with attendance tracking enabled/i)).toBeInTheDocument();
    expect(screen.getByText('10 · —')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Demand & shortages' }));
    expect(screen.getByText('2 stroller')).toBeInTheDocument();
    expect(screen.getByText(/4 eligible drivers/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Finance & claims' }));
    expect(screen.getByText('2 pending > 7d')).toBeInTheDocument();
    expect(screen.getByText('1 awaiting payout > 7d')).toBeInTheDocument();
    expect(screen.getByText('Monthly reimbursement payouts')).toBeInTheDocument();
  });
});
