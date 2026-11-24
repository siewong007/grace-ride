import { test, expect, devices, Page, BrowserContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const { defaultBrowserType: _ignored, ...phone } = devices['iPhone 13'];

const memberMe = {
  id: 7,
  name: 'Sister Ruth',
  email: 'ruth@example.org',
  phone: '+60123456789',
  photo_file_id: null,
  roles: ['member'],
  role_names: {},
  permissions: [
    'rides.view', 'rides.create', 'rides.update', 'rides.delete',
    'events.view', 'events.create',
    'costs.view', 'costs.update',
    'reimbursements.view', 'reimbursements.create',
    'help.view', 'help.create',
    'report.view',
  ],
  status: 'active',
  terms_accepted: true,
  mfa_ok: true,
  totp_enabled: true,
  has_password: false,
  passkey_count: 0,
  badges_opt_out: false,
  readiness_prompt_mins: 45,
  auto_lock_mins: 0,
  session_timeout_mins: 15,
  backup_codes_count: 4,
  locked: false,
  telegram_chat_id: '',
  whatsapp_opt_in: false,
  push_opt_in: true,
  caregiver_user_id: null,
  privileged: false,
  unread: 1,
  groups: [{ id: 1, name: 'Youth' }],
  dev_mode: true,
};

const sampleRide = {
  id: 42,
  status: 'confirmed',
  pickup_at: '2026-11-20T10:30:00',
  area: { id: 1, name: 'Petaling Jaya' },
  pickup_address: '12 Jalan SS2/10, Petaling Jaya',
  destination: 'PJ State CMC',
  seats: 1,
  assistance: '',
  notes: 'Wait by the guardhouse',
  event: { id: 10, title: 'Sunday Fellowship Lunch' },
  trip_id: 501,
  leg: 'outbound',
  paired_ride_id: null,
  passenger: { id: 7, name: 'Sister Ruth', phone: '+60123456789' },
  driver: {
    id: 3,
    name: 'Brother Silas',
    phone: '+60198765432',
    vehicle: { model: 'Toyota Vios', color: 'Silver', plate: 'WXX 8888', wheelchair_ok: false, low_step: false, child_seat: false },
  },
  needs_attention: false,
  attention_reason: '',
  started_at: null,
  picked_up_at: null,
  completed_at: null,
  updated_at: '2026-11-20T08:00:00',
  viewer: { is_passenger: true, is_driver: false, is_staff: false },
};

const sampleEvent = {
  id: 10,
  title: 'Sunday Fellowship Lunch',
  kind: 'fellowship',
  description: 'Church family fellowship lunch after worship.',
  starts_at: '2026-11-22T12:00:00',
  location: 'Main Hall',
  capacity: 40,
  cost_note: 'RM 12',
  signup_deadline: '2026-11-21T20:00:00',
  status: 'published',
  created_by: { id: 1, name: 'Leader Alice' },
  going: 12,
  waitlist: 0,
  spots_left: 28,
  attendance_enabled: true,
  my_signup: { status: 'going', need_ride: true, offer_drive: false, attended: 0, pass_code: 'PASS107', as_group: { id: 1, name: 'Youth' } },
  groups: 'Youth',
  can_manage: false,
  group_ids: [1],
  join_as: [{ id: 1, name: 'Youth' }],
  my_ride: null,
  settlement: null,
};

async function installMemberRoutes(
  context: BrowserContext,
  opts: {
    meOverride?: Record<string, unknown>;
    careReadinessCalls?: unknown[];
    termsCalls?: unknown[];
    unlockCalls?: unknown[];
  } = {},
) {
  let unlocked = false;
  let termsAccepted = opts.meOverride?.terms_accepted ?? true;
  let visitClaimed = false;

  await context.route(/^https?:\/\/(?!127\.0\.0\.1)/, (r) => r.abort());
  await context.route('**/api/**', async (route) => {
    const req = route.request();
    const u = new URL(req.url());
    const p = u.pathname.replace(/^\/api/, '');
    const ok = (json: unknown) => route.fulfill({ status: 200, json });

    if (p === '/stream') {
      return route.fulfill({ status: 200, contentType: 'text/event-stream', body: '' });
    }
    if (p === '/health') return ok({ ok: true, db: true });
    if (p === '/auth/turnstile') return ok({ enabled: false, site_key: null });

    if (p === '/auth/me') {
      const base = { ...memberMe, ...opts.meOverride, terms_accepted: termsAccepted };
      if (unlocked) base.locked = false;
      return ok(base);
    }
    if (p === '/auth/terms' && req.method() === 'POST') {
      opts.termsCalls?.push(req.postDataJSON());
      termsAccepted = true;
      return ok({ ok: true });
    }
    if (p === '/auth/unlock' && req.method() === 'POST') {
      opts.unlockCalls?.push(req.postDataJSON());
      unlocked = true;
      return ok({ ok: true });
    }
    if (p === '/auth/passkeys') return ok([]);
    if (p === '/me/calendar-feed') return ok({ enabled: false });

    // Public Caregiver Safe-Arrival link
    if (p === '/care/caretok123') {
      return ok({
        ride_id: 42,
        label: 'Daughter Safe-Arrival Link',
        rider_name: 'Uncle Tan',
        booker_name: 'Sister Ruth',
        status: 'confirmed',
        leg: 'outbound',
        pickup_at: '2026-11-20T10:30:00',
        area: { id: 1, name: 'Petaling Jaya' },
        destination: 'PJ State CMC',
        readiness_status: 'unconfirmed',
        delay_minutes: 0,
        arrived_at_gate_at: null,
        started_at: null,
        picked_up_at: null,
        completed_at: null,
        driver: { name: 'Brother Silas', vehicle: { model: 'Toyota Vios', color: 'Silver', plate: 'WXX 8888' } },
        location: { sharing: false },
      });
    }
    if (p === '/care/caretok123/readiness' && req.method() === 'POST') {
      opts.careReadinessCalls?.push(req.postDataJSON());
      return ok({ ok: true });
    }

    // Public Visitor one-time pass
    if (p === '/visit/visittok123') {
      return ok({
        state: visitClaimed ? 'mine' : 'open',
        closed_reason: null,
        opens_at: '2026-11-19T12:00:00',
        name: 'Friend David',
        host: 'Sister Ruth',
        event: { title: 'Sunday Fellowship Lunch', starts_at: '2026-11-22T12:00:00', location: 'Main Hall' },
      });
    }
    if (p === '/visit/visittok123/claim' && req.method() === 'POST') {
      visitClaimed = true;
      return ok({ ok: true });
    }
    if (p === '/visit/me') {
      return ok({
        name: 'Friend David',
        host: { name: 'Sister Ruth', phone: '+60123456789' },
        event: { title: 'Sunday Fellowship Lunch', starts_at: '2026-11-22T12:00:00', location: 'Main Hall' },
        ride: {
          status: 'confirmed',
          pickup_at: '2026-11-22T11:00:00',
          pickup_address: '15 Jalan Timur',
          area: 'Petaling Jaya',
          destination: 'Main Hall',
          readiness_status: 'unconfirmed',
          delay_minutes: 0,
          arrived_at_gate_at: null,
          picked_up_at: null,
          driver: { name: 'Brother Silas', phone: '+60198765432', vehicle: { model: 'Vios', color: 'Silver', plate: 'WXX 8888' } },
        },
      });
    }

    // Public Invite onboarding
    if (p === '/invites/invtok123') {
      return ok({ kind: 'invite', email: '', name: '' });
    }

    // Member operational endpoints
    if (p === '/areas') return ok([{ id: 1, name: 'Petaling Jaya', active: true }]);
    if (p === '/proxy-riders') return ok([]);
    if (p === '/saved-places') return ok([{ id: 1, label: 'Home', area_id: 1, area_name: 'Petaling Jaya', address: '12 Jalan SS2/10', gate_notes: 'Gate B', is_default: 1 }]);
    if (p === '/recurring-schedules') return ok([]);
    if (p === '/announcements') return ok([{ id: 1, title: 'Welcome to Grace Ride', body: 'Christmas transport is open.', pinned: true, by: 'Admin', created_at: '2026-11-19T08:00:00' }]);
    if (p === '/rides') return ok(u.searchParams.get('scope') === 'past' ? { items: [], next_before: null } : [sampleRide]);
    if (p === '/events') return ok(u.searchParams.get('scope') === 'past' ? { items: [], next_before: null } : [sampleEvent]);
    if (p === '/events/10') return ok(sampleEvent);
    if (p === '/events/10/signups') return ok([]);
    if (p === '/events/10/checkin') {
      return ok({
        id: 10,
        title: 'Sunday Fellowship Lunch',
        starts_at: '2026-11-22T12:00:00',
        location: 'Main Hall',
        attendance_enabled: true,
        my_attended: false,
        is_member: true,
      });
    }
    if (p === '/events/10/visitors') return ok([]);
    if (p === '/me/shares') {
      return ok([{
        id: 5,
        settlement_id: 20,
        title: 'Lunch Split',
        event_title: 'Sunday Fellowship Lunch',
        amount_cents: 1200,
        status: 'unpaid',
        payee_name: 'Leader Alice',
        payee_account: 'TNG 012-3456789',
        due_date: '2026-11-25',
        note: '',
        txn_ref: '',
        proof_file_id: null,
      }]);
    }
    if (p === '/reimbursements') {
      return ok({
        items: [{
          id: 7, user: { id: 7, name: 'Sister Ruth' }, ride_id: 42, ride_at: '2026-11-20T10:30:00',
          amount_cents: 2400, description: 'Grab fallback', receipt_file_id: null,
          payee_name: 'Sister Ruth', payee_phone: '+60123456789', status: 'approved',
          review_note: '', reviewer: 'Transport', payee_verified: true, paid_ref: null, paid_at: null, created_at: '2026-11-19T12:00:00',
        }],
        total: 1,
        page: 1,
        page_size: 50,
        total_pages: 1,
      });
    }
    if (p === '/reports/monthly') {
      return ok({
        user: { id: 7, name: 'Sister Ruth' },
        month: '2026-11',
        range: 'monthly',
        type: 'all',
        period_start: '2026-11-01',
        period_end: '2026-12-01',
        rides_driven: 0,
        rides_received: 2,
        distance_km: 0,
        reimbursement_cents: 0,
        events_attended: 1,
        by_kind: [{ kind: 'fellowship', count: 1 }],
        events: [{ id: 10, title: 'Sunday Fellowship Lunch', kind: 'fellowship', starts_at: '2026-11-22T12:00:00' }],
        costs_paid_cents: 1200,
        badges: [],
      });
    }
    if (p === '/help') return ok([]);
    if (p === '/help/guides') return ok([]);
    if (p === '/notifications') {
      return ok({
        items: [{ id: 1, title: 'Ride confirmed', body: 'Pickup at 10:30', link: '/rides/42', read: false, created_at: '2026-11-20T08:00:00' }],
        next_before: null,
      });
    }
    return ok({});
  });
}

async function openAndAudit(page: Page, path: string, headingSelector = 'main h1') {
  await page.clock.setFixedTime(new Date('2026-11-20T09:00:00'));
  await page.goto(path);
  await page.locator('#splash').waitFor({ state: 'detached' });
  await expect(page.locator(headingSelector).first()).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Something went wrong' })).toHaveCount(0);

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow, `horizontal overflow on ${path}`).toBeLessThanOrEqual(0);

  const axe = await new AxeBuilder({ page }).exclude('.leaflet-container').analyze();
  const serious = axe.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical');
  expect(serious.map((v) => `${v.id}: ${v.nodes[0]?.target.join(' ')}`), `a11y violations on ${path}`).toEqual([]);
}

test.describe('Public zero-login journeys (iPhone 13)', () => {
  test.use(phone);

  test('caregiver /care/:token safe-arrival status, display controls, readiness post & a11y', async ({ context, page }) => {
    const calls: unknown[] = [];
    await installMemberRoutes(context, { careReadinessCalls: calls });
    await openAndAudit(page, '/care/caretok123', '.visit-title');

    await expect(page.getByText('Uncle Tan’s ride status')).toBeVisible();
    await expect(page.getByText('12 Jalan SS2/10')).toHaveCount(0);
    await expect(page.getByText('Brother Silas')).toBeVisible();

    await page.getByRole('button', { name: /Text size/i }).click();
    expect(await page.evaluate(() => document.documentElement.dataset.text)).toBe('large');

    await page.getByRole('button', { name: /Uncle Tan is ready/i }).click();
    await expect.poll(() => calls.length).toBe(1);
    expect(calls[0]).toEqual({ status: 'ready' });
  });

  test('visitor /visit/:token claim transition & a11y', async ({ context, page }) => {
    await installMemberRoutes(context);
    await openAndAudit(page, '/visit/visittok123', '.visit-title');
    await expect(page.getByText(/Sister Ruth has arranged a ride for you/i)).toBeVisible();

    await page.getByRole('button', { name: /Open my ride/i }).click();
    await expect(page.getByText(/Hi Friend David/i)).toBeVisible();
    await expect(page.getByText('WXX 8888')).toBeVisible();

    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  });

  test('invite /invite/:token details step & a11y', async ({ context, page }) => {
    await installMemberRoutes(context);
    await openAndAudit(page, '/invite/invtok123', 'h1');
  });
});

test.describe('Terms gate and Session AutoLock security flows', () => {
  test('terms_accepted=false blocks app until accepted', async ({ context, page }) => {
    const termsCalls: unknown[] = [];
    await installMemberRoutes(context, {
      meOverride: { terms_accepted: false },
      termsCalls,
    });
    await openAndAudit(page, '/rides', 'h1');
    await expect(page.getByRole('heading', { name: /Terms of use and privacy notice/i })).toBeVisible();

    await page.getByRole('checkbox').check();
    await page.getByRole('button', { name: /Agree and continue/i }).click();
    await expect.poll(() => termsCalls.length).toBe(1);
    await expect(page.getByRole('heading', { level: 1, name: /My rides/i })).toBeVisible();
  });

  test('locked=true shows server-enforced AutoLock overlay and unlocks via TOTP', async ({ context, page }) => {
    const unlockCalls: unknown[] = [];
    await installMemberRoutes(context, {
      meOverride: { locked: true, totp_enabled: true },
      unlockCalls,
    });
    await page.clock.setFixedTime(new Date('2026-11-20T09:00:00'));
    await page.goto('/rides');
    await page.locator('#splash').waitFor({ state: 'detached' });

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByPlaceholder('6-digit or backup code').fill('654321');
    await dialog.getByRole('button', { name: 'Unlock', exact: true }).click();

    await expect.poll(() => unlockCalls.length).toBe(1);
    expect(unlockCalls[0]).toEqual({ totp_code: '654321' });
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('heading', { level: 1, name: /My rides/i })).toBeVisible();
  });
});

const MEMBER_SCREENS = [
  '/',
  '/rides',
  '/rides/new',
  '/events',
  '/events/10',
  '/events/10/checkin',
  '/costs',
  '/reimbursements',
  '/report',
  '/help',
  '/notifications',
  '/me',
];

for (const vp of [
  { name: 'phone', options: phone },
  { name: 'desktop', options: { viewport: { width: 1280, height: 800 } } },
] as const) {
  test.describe(`member screens · ${vp.name}`, () => {
    test.use(vp.options as never);

    for (const screenPath of MEMBER_SCREENS) {
      test(screenPath, async ({ context, page }) => {
        await installMemberRoutes(context);
        await openAndAudit(page, screenPath);
      });
    }
  });
}
