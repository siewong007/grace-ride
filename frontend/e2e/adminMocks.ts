import type { BrowserContext } from '@playwright/test';

/** Realistic, populated answers for every admin endpoint, so each staff screen renders with content. */
const ACTS = ['view', 'create', 'update', 'delete'];
const PAGES = [
  'rides', 'drive', 'events', 'settlements', 'costs', 'reimbursements', 'help', 'report',
  'admin_dashboard', 'admin_rides', 'admin_live', 'admin_plan', 'admin_events', 'admin_reimb_review',
  'admin_announcements', 'admin_help', 'admin_reports', 'admin_fares', 'admin_area_photos', 'admin_reimb_pay', 'admin_bills', 'admin_members',
  'admin_invites', 'admin_recovery', 'admin_groups', 'admin_areas', 'admin_roles', 'admin_audit', 'admin_settings',
];
const allPerms = PAGES.flatMap((p) => ACTS.map((a) => `${p}.${a}`));

const names = ['Sister Mary Catherine Wong', 'Brother Silas Tan', 'Auntie Lim Siew Fong', 'Uncle Raj Kumar a/l Subramaniam', 'Grace Lee', 'Daniel Ong'];
const statuses = ['pending_driver', 'pending_approval', 'confirmed', 'in_progress', 'completed', 'alt_transport'];

export const me = {
  id: 1, name: 'Sister Mary Catherine Wong', email: 'mary@example.org', phone: '+60123456789',
  roles: ['superadmin'], role_names: {}, permissions: allPerms, privileged: true, mfa_ok: true,
  status: 'active', terms_accepted: true, groups: [], unread: 0,
};

const ride = (i: number) => ({
  id: 100 + i, status: statuses[i % 6], pickup_at: `2026-11-20T${String(8 + i).padStart(2, '0')}:30:00`,
  area: { id: 1, name: ['Petaling Jaya', 'Subang Jaya USJ'][i % 2] }, pickup_address: '123 Jalan Damai 4/2, Petaling Jaya',
  destination: 'Grace Community Church', seats: 1 + (i % 3), assistance: '', notes: 'Near the guardhouse', event: null, trip_id: null,
  passenger: { id: 10 + i, name: names[i % 6], phone: '+60123456789' },
  driver: i % 3 === 0 ? null : { id: 2, name: names[(i + 1) % 6], phone: '+60198765432', vehicle: { model: 'Toyota Vios', color: 'Silver', plate: 'WXX 8888' } },
  needs_attention: i % 4 === 0, attention_reason: 'No driver within 24h of pickup',
  started_at: null, picked_up_at: null, completed_at: null, updated_at: '2026-11-19T10:00:00', created_at: '2026-11-19T08:00:00',
  timeline: [{ status: 'requested', at: '2026-11-19T08:00:00', by: 'Sister Mary' }],
  viewer: { is_passenger: false, is_driver: false, is_staff: true },
});
const rides = Array.from({ length: 8 }, (_, i) => ride(i));

const users = names.map((n, i) => ({
  id: 10 + i, name: n, email: `${n.toLowerCase().replace(/[^a-z]+/g, '.')}@example.org`, phone: '+60123456789',
  roles: i % 2 ? ['admin', 'transport'] : [], status: i === 2 ? 'pending' : 'active', created_at: '2026-01-05T08:00:00', passkeys: i % 3,
  groups: i % 2 ? 'Youth、Petaling Jaya Cell Group' : '',
}));

export const reimbs = Array.from({ length: 5 }, (_, i) => ({
  id: 500 + i, user: { id: 10 + i, name: names[i] }, ride_id: i % 2 ? 100 + i : null, ride_at: '2026-11-18T09:00:00',
  amount_cents: 1250 + i * 730, description: 'Fuel and toll for Sunday service pickups across three areas', receipt_file_id: 9,
  payee_name: names[i], payee_phone: '+60123456789', status: ['pending', 'approved', 'paid', 'returned', 'rejected'][i],
  review_note: '', reviewer: null, payee_verified: false, paid_ref: null, paid_at: null, created_at: '2026-11-17T08:00:00',
}));

const areas = ['Petaling Jaya', 'UT', 'Sec 17', 'UM', 'KK', 'Pacific/RM'].map((name, i) => ({ id: i + 1, name, active: true }));
const fare = (a: number, b: number, amount_cents: number) => ({
  id: a * 10 + b, area_a: { id: a, name: areas[a - 1].name }, area_b: { id: b, name: areas[b - 1].name }, amount_cents, updated_at: '',
});
// Some routes priced, the rest left for the traffic lead to fill in.
const fares = [fare(1, 1, 1500), fare(2, 3, 800), fare(2, 4, 1000), fare(2, 5, 1200), fare(3, 4, 600)];

const catalog = ['rides', 'admin_roles', 'admin_audit'].map((p, i) => ({ page: p, area: ['member', 'admin', 'admin'][i], actions: ACTS }));
const roles = {
  catalog,
  roles: [
    { key: 'member', name: '', description: '', system: true, implicit: true, permissions: ['rides.view'], locked: [], members: 40 },
    { key: 'admin', name: '', description: '', system: true, implicit: false, permissions: [], locked: [], members: 3, assignable: true },
    { key: 'c1', name: 'Sunday welcome team coordinator', description: '', system: false, implicit: false, permissions: [], locked: [], members: 2, assignable: true },
  ],
};

const audit = {
  items: Array.from({ length: 8 }, (_, i) => ({
    id: 900 - i, actor_id: 1, actor: i % 4 === 0 ? null : names[i % 6], action: ['ride.assign', 'announcement.update', 'role.update', 'reimbursement.approve'][i % 4],
    entity: ['ride', 'announcement', 'role', 'reimbursement'][i % 4], entity_id: 100 + i, entity_name: null,
    detail: 'Assigned driver Brother Silas Tan after the passenger changed the pickup time', created_at: `2026-11-20T0${8 - (i % 8)}:10:00`,
  })),
  next_before: null, retention_days: 730,
};

export const plan = {
  event: { id: 1, title: 'Sunday Worship Service & Fellowship Lunch', starts_at: '2026-11-22T10:00:00', location: 'Church', status: 'open' },
  rides: Array.from({ length: 4 }, (_, i) => ({
    id: 200 + i, passenger: { id: 10 + i, name: names[i] }, seats: 1, area: { id: 1, name: 'Petaling Jaya' }, pickup_at: '2026-11-22T09:00:00',
    pickup_address: '123 Jalan Damai', assistance: '', notes: '', status: 'pending_driver', driver_id: null, last: null, match_group_id: null,
  })),
  drivers: [{ id: 30, name: 'Brother Silas Tan', carpool: true, cap: 4, other_seats: 0, other_rides: 0, area_ids: [1], drive_group_ids: [], vehicle: { model: 'Vios', plate: 'WXX 8888', seats: 5 } }],
  // Visitors recorded for the plan (no account, no ride).
  visitors: [
    { id: 1, name: 'Mr Tan Ah Kow (first visit)', phone: '012-345 6789', notes: 'Brought by Grace Lee', area: { id: 1, name: 'Petaling Jaya' }, pickup_address: '8 Jalan SS2/24',
      seats: 2, leg: 'outbound', driver_id: 30, driver_name: 'Brother Silas Tan', created_by: { id: 1, name: 'Sister Mary Catherine Wong' }, created_at: '', updated_at: '', can_edit: true },
    { id: 2, name: 'Auntie Ho', phone: '', notes: '', area: null, pickup_address: '',
      seats: 1, leg: 'return', driver_id: null, driver_name: null, created_by: { id: 3, name: 'Grace Lee' }, created_at: '', updated_at: '', can_edit: false },
  ],
};

const report = {
  user: { id: 1, name: 'Sister Mary Catherine Wong' },
  month: '2026-11',
  range: 'monthly',
  type: 'all',
  period_start: '2026-11-01',
  period_end: '2026-12-01',
  period_end_inclusive: '2026-11-30',
  rides_driven: 14,
  rides_received: 3,
  distance_km: 212,
  reimbursement_cents: 18650,
  events_attended: 5,
  by_kind: [{ kind: 'badminton', count: 2 }, { kind: 'other', count: 1 }],
  events: [{ id: 1, title: 'Sunday Worship Service & Fellowship Lunch', kind: 'other', starts_at: '2026-11-01T10:00:00' }],
  costs_paid_cents: 14200,
  visible_sections: { events: true, driving: true, passenger_rides: true, costs_paid: true, badges: true, reimbursements: true },
  reimbursements_summary: {
    claim_count: 8,
    pending_count: 2,
    pending_cents: 4500,
    approved_count: 3,
    approved_cents: 8200,
    paid_count: 3,
    paid_cents: 12400,
    returned_count: 1,
    rejected_count: 1,
  },
  organised_events: {
    events_count: 4,
    total_going: 62,
    total_attended: 54,
    attendance_rate_pct: 87,
    settlements: {
      count: 3,
      gross_cents: 48000,
      subsidy_cents: 6000,
      collected_cents: 35000,
      outstanding_cents: 7000,
    },
  },
  badges: [
    {
      key: 'a',
      char: '恩',
      name: 'Faithful Driver',
      rule: 'Drive 10 rides',
      progress: 14,
      target: 20,
      earned: true,
      category: 'driver',
      role_match: true,
      tier: 'silver',
      next_tier: 'gold',
      next_target: 20,
      tiers: [
        { tier: 'bronze', target: 5, reached: true },
        { tier: 'silver', target: 10, reached: true },
        { tier: 'gold', target: 20, reached: false },
        { tier: 'platinum', target: 50, reached: false },
      ],
    },
  ],
};

const reportMonths12 = Array.from({ length: 12 }, (_, i) => `2026-${String(i + 1).padStart(2, '0')}`);

/** Route every `/api` call of this browser context to the canned answers above. */
export async function mockAdminApi(context: BrowserContext) {
  // No network beyond the preview server: map tiles and fonts are not needed to judge layout.
  await context.route(/^https?:\/\/(?!127\.0\.0\.1)/, (r) => r.abort());
  await context.route('**/api/**', (route) => {
    const u = new URL(route.request().url());
    const p = u.pathname.replace(/^\/api/, '');
    const j = (json: unknown) => route.fulfill({ status: 200, json });
    const paged = (items: unknown[], total = items.length) => ({ items, total, page: 1, page_size: 50, total_pages: total > 50 ? 3 : 1 });
    if (p === '/auth/me') return j(me);
    if (p === '/health') return j({ ok: true, db: true });
    if (p === '/admin/dashboard') return j({
      today: '2026-11-20', today_total: 8, today_confirmed: 3, today_changes: 0, pending_approval: 5, no_driver: 2, needs_attention: 4,
      live_drivers: 1, max_live_drivers: 10, open_help: 1, pending_reimbursements: 2, members: 142, pending_recovery: 1, open_invites: 2, awaiting_payment: 1,
      oldest_approval_at: '2026-11-19T08:00:00', next_no_driver_at: '2026-11-20T10:30:00', oldest_help_at: '2026-11-18T08:00:00', help_claimed_by_me: 0,
    });
    if (/^\/admin\/rides\/\d+/.test(p) || /^\/rides\/\d+$/.test(p)) return j(rides[1]);
    if (/^\/rides\/\d+\/(messages|location)/.test(p)) return j(p.endsWith('location') ? { sharing: false, points: [] } : []);
    if (p === '/admin/rides') return j(u.searchParams.has('page') ? paged(rides, 137) : rides);
    if (p === '/admin/users') return j(u.searchParams.has('page') ? paged(users, 137) : users);
    if (p === '/admin/invites') return j([{ id: 1, email: null, url: 'https://graceride.example.org/invite/8f3a9c2e7b1d', kind: 'invite', roles: [], status: 'open', created_at: '2026-11-19T08:00:00' }]);
    if (p === '/admin/recovery') return j([{ id: 1, name: names[3], email: 'raj.kumar.subramaniam@example.org', reason: 'Lost my phone last week and the new one has no passkey on it', status: 'pending', created_at: '2026-11-19T08:00:00' }]);
    if (p === '/admin/roles') return j(roles);
    if (p === '/reimbursements') return j(u.searchParams.has('scope') ? { ...paged(reimbs), total: 5 } : []);
    if (p === '/reports/monthly') return j(report);
    if (p === '/admin/reports/capabilities') return j({
      overview: true, member_reports_global: true, member_badge_icon_edit: true,
      demand: true, demand_scope: 'global', events: true, events_scope: 'global',
      finance: true, finance_sections: { reimb_review: true, reimb_payouts: true, bills: true }, membership: true,
    });
    if (p === '/admin/reports/overview') return j({
      period: '2026', range: 'yearly', from: '2026-01-01', to: '2026-12-31',
      comparison: {
        prev_period: '2025-01-01 – 2025-12-31',
        transport: { total_rides_delta: 24, completed_rides_delta: 19, fulfillment_rate_delta: 6 },
        events: { events_count_delta: 4, total_attended_delta: 85, attendance_rate_delta: -3 },
        reimb_review: { pending_count_delta: 2, approved_cents_delta: 18500 },
        reimb_payouts: { paid_cents_delta: 24500 },
        bills: { collected_cents_delta: -12000, collection_rate_delta: 5 },
        membership: { new_members_delta: 6 },
      },
      transport: {
        scope: 'global', total_rides: 218, completed_rides: 194, pending_confirmation_rides: 4, unfilled_rides: 12, cancelled_rides: 8,
        no_show_rides: 3, alt_transport_rides: 5, fulfillment_rate_pct: 89, total_distance_km: 3212,
        active_drivers: 18, unique_passengers: 64,
        top_drivers: [
          { user_id: 30, name: 'Brother Silas Tan Wei Ming', completed_rides: 48, distance_km: 720, carpool_trips: 18 },
          { user_id: 31, name: 'Uncle Raj Kumar a/l Subramaniam', completed_rides: 35, distance_km: 510, carpool_trips: 12 },
        ],
      },
      events: {
        scope: 'global', events_count: 28, tracked_events_count: 24, total_going: 645, total_attended: 538, attendance_rate_pct: 84,
        top_groups: [{ group_id: 2, group_name: 'Youth Fellowship & Young Adults', going: 225, attended: 202 }],
      },
      reimb_review: { pending_count: 12, pending_cents: 124500, returned_count: 3, approved_count: 18, approved_cents: 248200 },
      finance: {
        reimb_payouts: { approved_unpaid_count: 18, approved_unpaid_cents: 248200, paid_count: 45, paid_cents: 1240000, open_batches: 2, paid_batches: 10 },
        bills: { published_count: 4, closed_count: 12, gross_cents: 320000, subsidy_cents: 40000, collected_cents: 220000, outstanding_cents: 60000, collection_rate_pct: 78 },
      },
      membership: { active_members: 142, pending_members: 3, new_members_in_period: 14, active_drivers_total: 18 },
      monthly_series: reportMonths12.map((m, i) => ({
        month: m,
        rides_completed: i === 2 ? 0 : 12 + i * 3,
        events_attended: 35 + (i % 5) * 14,
        reimb_paid_cents: i === 1 ? 0 : 85000 + i * 11500,
        bills_collected_cents: 120000 + (i % 4) * 45000,
      })),
    });
    if (p === '/admin/reports/events') return j({
      scope: 'global', period: '2026-11', range: 'monthly', from: '2026-11-01', to: '2026-11-30',
      events_total: 1, events_truncated: false,
      comparison: { prev_period: '2026-10-01 – 2026-10-31', events_count_delta: 2, total_going_delta: 18, total_attended_delta: 12, attendance_rate_delta: -2 },
      totals: {
        events_count: 8, tracked_events_count: 6, cancelled_events: 1, total_going: 145, total_waitlist: 6, total_attended: 122,
        attendance_rate_pct: 84, need_ride_count: 44, offer_drive_count: 16, self_transport_count: 85,
        visitor_passes_count: 5, plan_visitors_count: 4,
      },
      by_group: [{ group_id: 2, group_name: 'Youth Fellowship & Young Adults', going: 65, attended: 58, attendance_rate_pct: 89, need_ride: 20, self_transport: 38, offer_drive: 7 }],
      by_kind: [{ kind: 'badminton', events_count: 3, going: 60, attended: 52, attendance_rate_pct: 87 }],
      events: [{ id: 1, title: 'Sunday Worship Service & Community Thanksgiving Lunch', kind: 'other', starts_at: '2026-11-22T10:00:00', status: 'published', creator_name: 'Sister Mary Catherine Wong', going: 65, waitlist: 2, attended: 58, attendance_enabled: true, attendance_rate_pct: 89, need_ride: 20, self_transport: 38, rides_matched: 18, rides_unfilled: 2 }],
    });
    if (p === '/admin/reports/demand') return j({
      scope: 'global', period: '2026-11', range: 'monthly', from: '2026-11-01', to: '2026-11-30',
      comparison: { prev_period: '2026-10-01 – 2026-10-31', total_requests_delta: 14, unfilled_delta: 3, fulfillment_rate_delta: -4 },
      totals: {
        total_requests: 68, fulfilled: 56, pending_confirmation: 3, unfilled: 9, completed: 52, cancelled_unassigned: 4,
        wheelchair_requests: 5, stroller_requests: 3, oxygen_requests: 1, return_requests: 22,
        proxy_rides: 9, plan_visitors_count: 6, plan_visitors_seats: 9, visitor_passes_count: 5,
        seats_unfilled: 12, active_drivers: 14, unique_passengers: 42, fulfillment_rate_pct: 82,
      },
      by_area: [{ area_id: 1, area_name_zh: '八打灵再也北区 (Petaling Jaya North)', area_name_en: 'Petaling Jaya North / Damansara Utama', total: 24, fulfilled: 18, unfilled: 6, cancelled_unassigned: 1, seats_needed: 30, seats_unfilled: 8, fulfillment_rate_pct: 75, registered_drivers: 5 }],
      by_slot: [{ weekday: 0, slot: '08:00-10:00', total: 32, fulfilled: 26, unfilled: 6, cancelled_unassigned: 1, seats_needed: 40 }],
      by_group: [{ group_id: 2, group_name: 'Youth Fellowship & Young Adults', total: 22, fulfilled: 19, unfilled: 3, seats_needed: 26, fulfillment_rate_pct: 86 }],
    });
    if (p === '/admin/reports/finance') return j({
      period: '2026', range: 'yearly', from: '2026-01-01', to: '2026-12-31',
      permissions: { reimb_review: true, reimb_payouts: true, bills: true },
      comparison: {
        prev_period: '2025-01-01 – 2025-12-31',
        reimb_review: { pending_count_delta: 4, approved_cents_delta: 42500 },
        reimb_payouts: { paid_cents_delta: 185000 },
        bills: { collected_cents_delta: 92000, collection_rate_delta: 4 },
      },
      reimbursement_review: {
        by_status: { pending: { count: 12, amount_cents: 124500 }, returned: { count: 3, amount_cents: 24000 }, rejected: { count: 2, amount_cents: 15000 }, approved: { count: 18, amount_cents: 248200 }, paid: { count: 45, amount_cents: 1240000 } },
        overdue_pending_count: 4,
        by_kind: [{ kind: 'grab', count: 36, amount_cents: 740000 }, { kind: 'mileage', count: 32, amount_cents: 586000 }, { kind: 'visitor_special', count: 12, amount_cents: 325700 }],
        high_amount_special_count: 3,
        driver_mileage_rides: { eligible_count: 112, eligible_cents: 315000, excluded_count: 8, receipt_needed_count: 5 },
      },
      reimbursement_payouts: {
        approved_unpaid_count: 18, approved_unpaid_cents: 248200, all_time_awaiting_count: 24, all_time_awaiting_cents: 318200,
        overdue_approved_count: 5, overdue_approved_cents: 84000,
        paid_count: 45, paid_cents: 1240000, payee_verified_count: 27, payee_unverified_count: 4,
        by_kind: [{ kind: 'grab', approved_count: 10, approved_cents: 150000, paid_count: 24, paid_cents: 690000 }, { kind: 'mileage', approved_count: 8, approved_cents: 98200, paid_count: 21, paid_cents: 550000 }],
        batches_total: 2, batches_truncated: false,
        batches: [
          { id: 1, month: '2026-11', title: 'November Driver & Grab Reimbursements Batch #2', status: 'paid', claim_count: 15, total_cents: 424000, created_by_name: 'Finance Officer Alice', paid_by_name: 'Treasurer Benjamin', paid_at: '2026-11-20T12:00:00', paid_ref: 'TXN-2026-1102', maker_checker_ok: true },
          { id: 2, month: '2026-11', title: 'Urgent Special Needs Transport Batch', status: 'open', claim_count: 4, total_cents: 89500, created_by_name: 'Finance Officer Alice', paid_by_name: null, paid_at: null, paid_ref: '', maker_checker_ok: false },
        ],
        monthly_trend: reportMonths12.map((m, i) => ({
          month: m, approved_count: 4 + (i % 3), approved_cents: 50000 + i * 8000, paid_count: 3 + (i % 4), paid_cents: i === 0 ? 0 : 95000 + i * 14500, claim_count: 8 + (i % 5),
        })),
      },
      bills: {
        totals: { total_settlements: 16, draft_count: 2, published_count: 4, closed_count: 12, overdue_settlements_count: 2, settlements_total: 1, settlements_truncated: false, gross_cents: 320000, subsidy_cents: 40000, net_billed_cents: 280000, collected_cents: 220000, pending_verification_cents: 20000, needs_info_cents: 12000, unpaid_cents: 28000, collection_rate_pct: 78 },
        settlements: [{ id: 1, title: 'Church Camp Bus & Badminton Court Booking', event_id: 1, event_title: 'Sunday Worship Service & Thanksgiving Lunch', event_starts_at: '2026-11-22T10:00:00', organiser_name: 'Sister Mary Catherine Wong', status: 'published', gross_cents: 160000, subsidy_cents: 20000, net_billed_cents: 140000, collected_cents: 105000, outstanding_cents: 35000, participant_count: 34, confirmed_count: 26 }],
      },
    });
    if (p === '/admin/live') return j({ max: 10, drivers: [
      { driver_id: 30, name: 'Brother Silas Tan', lat: 3.107, lng: 101.607, updated_at: '2026-11-20T08:59:30', rides: 3, first_pickup: '2026-11-20T09:30:00' },
      { driver_id: 32, name: 'Paul Ng', lat: null, lng: null, updated_at: null, rides: 2, first_pickup: '2026-11-20T10:30:00' },
    ] });
    if (p === '/admin/audit') return j(audit);
    if (p === '/groups') return j([
      { id: 1, name: 'Everyone', description: '', is_all: true, member_count: 142, parent_id: null, is_visitor: false },
      { id: 2, name: 'Youth', description: 'Ages 13 to 25', is_all: false, member_count: 31, parent_id: null, is_visitor: false },
    ]);
    if (p === '/areas') return j(areas);
    if (p === '/admin/settings/fares') return j({ fares: fares });
    if (p === '/admin/settings/system') return j({ audit_retention_days: 730, chat_retention_days: 180, max_live_drivers: 10, session_timeout_mins: 15 });
    if (/^\/admin\/events\/\d+\/plan\/visitors/.test(p)) return j({ visitors: plan.visitors });
    if (/^\/admin\/events\/\d+\/plan/.test(p)) return j(plan);
    if (p === '/announcements') return j([{ id: 1, title: 'Christmas service transport sign-up is open', body: 'Please request your ride by 15 December.', pinned: true, by: 'Sister Mary', created_at: '2026-11-19T08:00:00' }]);
    if (p.startsWith('/files/')) return route.fulfill({ status: 200, contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="400"><rect width="300" height="400" fill="#ddd"/></svg>' });
    if (/^\/(events|notifications|help|me\/shares|driver)/.test(p)) return j([]);
    if (p === '/costs') return j({ balance_cents: 0, items: [] });
    return j({});
  });
}
