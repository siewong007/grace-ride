import type { BrowserContext } from '@playwright/test';

/** Realistic, populated answers for every admin endpoint, so each staff screen renders with content. */
const ACTS = ['view', 'create', 'update', 'delete'];
const PAGES = [
  'rides', 'drive', 'events', 'settlements', 'costs', 'reimbursements', 'help', 'report',
  'admin_dashboard', 'admin_rides', 'admin_live', 'admin_plan', 'admin_events', 'admin_reimb_review',
  'admin_announcements', 'admin_help', 'admin_reports', 'admin_reimb_pay', 'admin_bills', 'admin_members',
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

const reimbs = Array.from({ length: 5 }, (_, i) => ({
  id: 500 + i, user: { id: 10 + i, name: names[i] }, ride_id: i % 2 ? 100 + i : null, ride_at: '2026-11-18T09:00:00',
  amount_cents: 1250 + i * 730, description: 'Fuel and toll for Sunday service pickups across three areas', receipt_file_id: 9,
  payee_name: names[i], payee_phone: '+60123456789', status: ['pending', 'approved', 'paid', 'returned', 'rejected'][i],
  review_note: '', reviewer: null, payee_verified: false, paid_ref: null, paid_at: null, created_at: '2026-11-17T08:00:00',
}));

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

const plan = {
  event: { id: 1, title: 'Sunday Worship Service & Fellowship Lunch', starts_at: '2026-11-22T10:00:00', location: 'Church', status: 'open' },
  rides: Array.from({ length: 4 }, (_, i) => ({
    id: 200 + i, passenger: { id: 10 + i, name: names[i] }, seats: 1, area: { id: 1, name: 'Petaling Jaya' }, pickup_at: '2026-11-22T09:00:00',
    pickup_address: '123 Jalan Damai', assistance: '', notes: '', status: 'pending_driver', driver_id: null, last: null, match_group_id: null,
  })),
  drivers: [{ id: 30, name: 'Brother Silas Tan', carpool: true, cap: 4, other_seats: 0, other_rides: 0, area_ids: [1], drive_group_ids: [], vehicle: { model: 'Vios', plate: 'WXX 8888', seats: 5 } }],
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
    if (p === '/admin/live') return j({ max: 10, drivers: [
      { driver_id: 30, name: 'Brother Silas Tan', lat: 3.107, lng: 101.607, updated_at: '2026-11-20T08:59:30', rides: 3, first_pickup: '2026-11-20T09:30:00' },
      { driver_id: 32, name: 'Paul Ng', lat: null, lng: null, updated_at: null, rides: 2, first_pickup: '2026-11-20T10:30:00' },
    ] });
    if (p === '/admin/audit') return j(audit);
    if (p === '/groups') return j([
      { id: 1, name: 'Everyone', description: '', is_all: true, member_count: 142, parent_id: null, is_visitor: false },
      { id: 2, name: 'Youth', description: 'Ages 13 to 25', is_all: false, member_count: 31, parent_id: null, is_visitor: false },
    ]);
    if (p === '/areas') return j([{ id: 1, name: 'Petaling Jaya', active: true }]);
    if (p === '/admin/settings/fares') return j({ fares: [{ id: 1, area_a: { id: 1, name: 'Petaling Jaya' }, area_b: { id: 1, name: 'Petaling Jaya' }, amount_cents: 1500, updated_at: '' }] });
    if (p === '/admin/settings/system') return j({ audit_retention_days: 730, chat_retention_days: 180, max_live_drivers: 10, session_timeout_mins: 15 });
    if (/^\/admin\/events\/\d+\/plan/.test(p)) return j(plan);
    if (p === '/announcements') return j([{ id: 1, title: 'Christmas service transport sign-up is open', body: 'Please request your ride by 15 December.', pinned: true, by: 'Sister Mary', created_at: '2026-11-19T08:00:00' }]);
    if (p.startsWith('/files/')) return route.fulfill({ status: 200, contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="400"><rect width="300" height="400" fill="#ddd"/></svg>' });
    if (/^\/(events|notifications|help|me\/shares|driver)/.test(p)) return j([]);
    if (p === '/costs') return j({ balance_cents: 0, items: [] });
    return j({});
  });
}
