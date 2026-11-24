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
  user: { id: 1, name: 'Mary' }, month: '2026-11', range: 'monthly', type: 'all', period_start: '2026-11-01', period_end: '2026-11-30',
  rides_driven: 14, rides_received: 3, distance_km: 212, reimbursement_cents: 18650, events_attended: 5,
  by_kind: [{ kind: 'badminton', count: 2 }], events: [{ id: 1, title: 'Sunday Worship Service', kind: 'other', starts_at: '2026-11-01T10:00:00' }],
  costs_paid_cents: 4200, badges: [{ key: 'a', char: '恩', name: 'Faithful driver', rule: 'Drive 10 rides', progress: 14, target: 10, earned: true }],
};

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
      period: '2026-11', range: 'monthly', from: '2026-11-01', to: '2026-12-01',
      transport: {
        scope: 'global', total_rides: 18, completed_rides: 14, unfilled_rides: 2, cancelled_rides: 1,
        no_show_rides: 0, alt_transport_rides: 1, fulfillment_rate_pct: 83, total_distance_km: 212,
        active_drivers: 4, unique_passengers: 12,
        top_drivers: [{ user_id: 30, name: 'Brother Silas Tan', completed_rides: 8, distance_km: 120, carpool_trips: 3 }],
      },
      events: {
        scope: 'global', events_count: 3, total_going: 45, total_attended: 38, attendance_rate_pct: 84,
        top_groups: [{ group_id: 2, group_name: 'Youth', going: 25, attended: 22 }],
      },
      reimb_review: { pending_count: 2, pending_cents: 4500, returned_count: 1, approved_count: 3, approved_cents: 8200 },
      finance: {
        reimb_payouts: { approved_unpaid_count: 3, approved_unpaid_cents: 8200, paid_count: 5, paid_cents: 12400, open_batches: 1, paid_batches: 1 },
        bills: { published_count: 1, closed_count: 1, gross_cents: 32000, subsidy_cents: 4000, collected_cents: 22000, outstanding_cents: 6000, collection_rate_pct: 78 },
      },
      membership: { active_members: 142, pending_members: 3, new_members_in_period: 4, active_drivers_total: 18 },
      monthly_series: [{ month: '2026-11', rides_completed: 14, events_attended: 38, reimb_paid_cents: 12400, bills_collected_cents: 22000 }],
    });
    if (p === '/admin/reports/events') return j({
      scope: 'global', period: '2026-11', range: 'monthly', from: '2026-11-01', to: '2026-12-01',
      totals: {
        events_count: 3, cancelled_events: 0, total_going: 45, total_waitlist: 2, total_attended: 38,
        attendance_rate_pct: 84, need_ride_count: 14, offer_drive_count: 6, self_transport_count: 25,
        visitor_passes_count: 2, plan_visitors_count: 2,
      },
      by_group: [{ group_id: 2, group_name: 'Youth', going: 25, attended: 22, attendance_rate_pct: 88, need_ride: 8, self_transport: 14, offer_drive: 3 }],
      by_kind: [{ kind: 'badminton', events_count: 2, going: 30, attended: 26, attendance_rate_pct: 86 }],
      events: [{ id: 1, title: 'Sunday Worship Service', kind: 'other', starts_at: '2026-11-22T10:00:00', status: 'published', creator_name: 'Sister Mary', going: 25, waitlist: 0, attended: 22, attendance_enabled: true, attendance_rate_pct: 88, need_ride: 8, self_transport: 14, rides_matched: 7, rides_unfilled: 1 }],
    });
    if (p === '/admin/reports/demand') return j({
      scope: 'global', period: '2026-11', range: 'monthly', from: '2026-11-01', to: '2026-12-01',
      totals: {
        total_requests: 18, fulfilled: 15, unfilled: 2, completed: 14, cancelled_unassigned: 1,
        wheelchair_requests: 2, stroller_requests: 0, oxygen_requests: 0, return_requests: 6,
        proxy_rides: 3, plan_visitors_count: 2, plan_visitors_seats: 3, visitor_passes_count: 2,
        seats_unfilled: 3, active_drivers: 4, unique_passengers: 12, fulfillment_rate_pct: 83,
      },
      by_area: [{ area_id: 1, area_name_zh: 'Petaling Jaya', area_name_en: 'Petaling Jaya', total: 10, fulfilled: 8, unfilled: 2, cancelled_unassigned: 0, seats_needed: 12, seats_unfilled: 3, fulfillment_rate_pct: 80 }],
      by_slot: [{ weekday: 0, slot: '08:00-10:00', total: 12, fulfilled: 10, unfilled: 2, cancelled_unassigned: 0, seats_needed: 15 }],
      by_group: [{ group_id: 2, group_name: 'Youth', total: 8, fulfilled: 7, unfilled: 1, seats_needed: 9, fulfillment_rate_pct: 87 }],
    });
    if (p === '/admin/reports/finance') return j({
      period: '2026-11', range: 'monthly', from: '2026-11-01', to: '2026-12-01',
      permissions: { reimb_review: true, reimb_payouts: true, bills: true },
      reimbursement_review: {
        by_status: { pending: { count: 2, amount_cents: 4500 }, returned: { count: 1, amount_cents: 1500 }, rejected: { count: 0, amount_cents: 0 }, approved: { count: 3, amount_cents: 8200 }, paid: { count: 5, amount_cents: 12400 } },
        by_kind: [{ kind: 'grab', count: 6, amount_cents: 14000 }, { kind: 'mileage', count: 4, amount_cents: 8600 }, { kind: 'visitor_special', count: 1, amount_cents: 4000 }],
        high_amount_special_count: 0,
        driver_mileage_rides: { eligible_count: 12, eligible_cents: 15000, excluded_count: 2, receipt_needed_count: 1 },
      },
      reimbursement_payouts: {
        approved_unpaid_count: 3, approved_unpaid_cents: 8200, all_time_awaiting_count: 3, all_time_awaiting_cents: 8200,
        paid_count: 5, paid_cents: 12400, payee_verified_count: 7, payee_unverified_count: 1,
        by_kind: [{ kind: 'grab', approved_count: 2, approved_cents: 5000, paid_count: 4, paid_cents: 9000 }],
        batches: [{ id: 1, month: '2026-11', title: 'Payout 2026-11', status: 'paid', claim_count: 5, total_cents: 12400, created_by_name: 'Finance A', paid_by_name: 'Finance B', paid_at: '2026-11-20T12:00:00', paid_ref: 'TXN-1', maker_checker_ok: true }],
        monthly_trend: [{ month: '2026-11', approved_cents: 8200, paid_cents: 12400, claim_count: 8 }],
      },
      bills: {
        totals: { total_settlements: 2, draft_count: 0, published_count: 1, closed_count: 1, gross_cents: 32000, subsidy_cents: 4000, net_billed_cents: 28000, collected_cents: 22000, pending_verification_cents: 2000, needs_info_cents: 0, unpaid_cents: 4000, collection_rate_pct: 78 },
        settlements: [{ id: 1, title: 'Badminton Court', event_id: 1, event_title: 'Sunday Worship Service', event_starts_at: '2026-11-22T10:00:00', organiser_name: 'Sister Mary', status: 'published', gross_cents: 16000, subsidy_cents: 2000, net_billed_cents: 14000, collected_cents: 10000, outstanding_cents: 4000, participant_count: 10, confirmed_count: 7 }],
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
