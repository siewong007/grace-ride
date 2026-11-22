import { expect, test, devices, type Page } from '@playwright/test';

const { defaultBrowserType: _ignored, ...phone } = devices['iPhone 13'];

const memberMe = {
  id: 2,
  name: 'Grace Member',
  email: 'member@example.org',
  roles: ['member'],
  permissions: ['events.view', 'rides.view', 'help.view', 'report.view', 'costs.view', 'reimbursements.view'],
  status: 'active',
  terms_accepted: true,
  mfa_ok: true,
  groups: [],
  unread: 0,
};

const staffMe = {
  ...memberMe,
  id: 1,
  name: 'Staff',
  email: 'staff@example.org',
  roles: ['admin'],
  permissions: [
    ...memberMe.permissions,
    'admin_dashboard.view',
    'admin_rides.view',
    'admin_live.view',
    'admin_help.view',
    'admin_members.view',
  ],
};

const sampleEvent = {
  id: 12,
  title: 'Sunday Fellowship Lunch',
  kind: 'fellowship',
  description: '',
  starts_at: '2026-11-22T12:00:00',
  location: 'Main Hall',
  status: 'published',
  going: 4,
  waitlist: 0,
  spots_left: 20,
  attendance_enabled: false,
  my_signup: null,
  can_manage: false,
  group_ids: [],
  join_as: [],
  my_ride: null,
  settlement: null,
};

async function mockApi(page: Page, audience: 'member' | 'staff') {
  await page.route('**/api/**', async (route) => {
    const path = new URL(route.request().url()).pathname.replace(/^\/api/, '');
    const ok = (json: unknown) => route.fulfill({ status: 200, json });
    if (path === '/stream') {
      return route.fulfill({ status: 200, contentType: 'text/event-stream', body: '' });
    }
    if (path === '/auth/me') return ok(audience === 'staff' ? staffMe : memberMe);
    if (path === '/events' || path === '/announcements' || path === '/rides' || path === '/help') {
      return ok(path === '/events' ? [sampleEvent] : []);
    }
    if (path === '/events/12') return ok(sampleEvent);
    if (path === '/admin/dashboard') {
      return ok({
        pending_approval: 0,
        no_driver: 0,
        open_help: 0,
        pending_reimbursements: 0,
        pending_recovery: 0,
        pending_members: 0,
        open_invites: 0,
        awaiting_payment: 0,
      });
    }
    return ok([]);
  });
}

async function swipe(page: Page, direction: 'left' | 'right') {
  await page.evaluate((dir) => {
    const target = document.querySelector('.shell') ?? document.getElementById('main') ?? document.body;
    const y = Math.round(window.innerHeight * 0.45);
    const startX = Math.round(window.innerWidth * 0.5);
    const endX = dir === 'left' ? startX - 140 : startX + 140;
    const midX = Math.round((startX + endX) / 2);
    const point = (x: number) =>
      new Touch({ identifier: 1, target, clientX: x, clientY: y, screenX: x, screenY: y, pageX: x, pageY: y });
    const fire = (type: string, x: number, touches: Touch[]) => {
      target.dispatchEvent(new TouchEvent(type, {
        bubbles: true,
        cancelable: true,
        composed: true,
        touches,
        targetTouches: touches,
        changedTouches: [point(x)],
      }));
    };
    fire('touchstart', startX, [point(startX)]);
    fire('touchmove', midX, [point(midX)]);
    fire('touchmove', endX, [point(endX)]);
    fire('touchend', endX, []);
  }, direction);
}

test.use({ ...phone });

test.describe('Mobile bottom-tab swipe (iPhone 13)', () => {
  test('member swipes Home → Events → Profile and stops at the ends', async ({ page }) => {
    await mockApi(page, 'member');
    await page.goto('/');
    await expect(page.getByRole('navigation', { name: 'Bottom navigation' })).toBeVisible();
    await swipe(page, 'left');
    await expect(page).toHaveURL(/\/events$/);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await swipe(page, 'left');
    await expect(page).toHaveURL(/\/me$/);
    await expect(page.getByRole('heading', { name: 'Grace Member' })).toBeVisible();
    await swipe(page, 'left');
    await expect(page).toHaveURL(/\/me$/);
    await swipe(page, 'right');
    await expect(page).toHaveURL(/\/events$/);
    await swipe(page, 'right');
    await expect(page).toHaveURL(/\/$/);
    await swipe(page, 'right');
    await expect(page).toHaveURL(/\/$/);
  });

  test('staff swipe includes the Admin tab', async ({ page }) => {
    await mockApi(page, 'staff');
    await page.goto('/events');
    await expect(page.getByRole('navigation', { name: 'Bottom navigation' }).getByRole('link', { name: /Admin/ })).toBeVisible();
    await swipe(page, 'left');
    await expect(page).toHaveURL(/\/admin\/menu$/);
    await expect(page.getByRole('heading', { name: 'Admin' })).toBeVisible();
    await swipe(page, 'left');
    await expect(page).toHaveURL(/\/me$/);
    await expect(page.getByRole('heading', { name: 'Staff' })).toBeVisible();
    await swipe(page, 'right');
    await expect(page).toHaveURL(/\/admin\/menu$/);
  });

  test('swipe on an event detail with no history falls back to the neighbouring tab', async ({ page }) => {
    await mockApi(page, 'member');
    await page.goto('/events/12');
    await expect(page).toHaveURL(/\/events\/12$/);
    await expect(page.getByRole('heading', { level: 1, name: 'Sunday Fellowship Lunch' })).toBeVisible();
    await swipe(page, 'right');
    await expect(page).toHaveURL(/\/$/);
  });

  test('swipe on an event detail with history goes back instead of jumping tabs', async ({ page }) => {
    await mockApi(page, 'member');
    await page.goto('/events');
    await page.getByRole('link', { name: /Sunday Fellowship Lunch/ }).first().click();
    await expect(page).toHaveURL(/\/events\/12$/);
    await expect(page.getByRole('heading', { level: 1, name: 'Sunday Fellowship Lunch' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Back' })).toBeVisible();
    await swipe(page, 'right');
    await expect(page).toHaveURL(/\/events$/);
  });
});
