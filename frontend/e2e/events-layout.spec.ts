import { expect, test, type Page } from '@playwright/test';

const splashGone = (page: Page) => page.locator('#splash').waitFor({ state: 'detached' });

const mockMe = {
  id: 2,
  name: 'Grace Member',
  email: 'member@example.org',
  phone: '+60123456789',
  roles: ['member', 'driver'],
  role_names: {},
  permissions: [
    'events.view',
    'rides.view',
    'rides.create',
    'drive.view',
    'drive.create',
    'help.view',
  ],
  privileged: false,
  mfa_ok: true,
  status: 'active',
  terms_accepted: true,
  onboarding_dismissed: true,
  groups: [],
  unread: 0,
};

const mockEvent = {
  id: 27,
  title: '青年团契 Youth Fellowship',
  kind: 'fellowship',
  icon: null,
  description: '',
  starts_at: '2026-11-27T20:00:00',
  location: 'PJ State CMC 教会',
  capacity: 50,
  cost_note: '免费 Free',
  signup_deadline: null,
  status: 'published',
  created_by: { id: 1, name: 'Leader' },
  going: 10,
  waitlist: 0,
  spots_left: 40,
  my_signup: {
    status: 'going',
    need_ride: false,
    offer_drive: false,
    self_transport: false,
    attended: 0,
  },
  groups: '青年团契',
  can_manage: false,
  attendance_enabled: true,
};

const viewports = [
  { width: 320, height: 568 },
  { width: 375, height: 812 },
  { width: 390, height: 844 },
  { width: 1280, height: 800 },
];

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem('gr-lang', 'en');
  });
  await page.clock.setFixedTime(new Date('2026-11-24T10:00:00'));

  await page.route('**/api/**', async (route) => {
    const path = new URL(route.request().url()).pathname.replace(/^\/api/, '');
    const ok = (json: unknown) => route.fulfill({ status: 200, json });

    if (path === '/stream') {
      return route.fulfill({ status: 200, contentType: 'text/event-stream', body: '' });
    }
    if (path === '/health') {
      return ok({ ok: true, db: true });
    }
    if (path === '/auth/me') {
      return ok(mockMe);
    }
    if (path === '/events') {
      return ok([mockEvent]);
    }
    if (
      path === '/rides' ||
      path === '/driver/schedule' ||
      path === '/driver/requests' ||
      path === '/announcements' ||
      path === '/notifications' ||
      path === '/help/guides' ||
      path === '/me/shares' ||
      path === '/areas'
    ) {
      return ok([]);
    }

    return ok({});
  });
});

for (const viewport of viewports) {
  test(`events transport alert wraps without horizontal overflow at ${viewport.width}x${viewport.height}`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await page.goto('/events');
    await splashGone(page);

    const alert = page.locator('.event-transport-alert');
    await expect(alert).toBeVisible();

    const actions = alert.locator('.event-transport-alert-actions');
    await expect(actions.getByRole('link', { name: 'I need a ride' })).toBeVisible();
    await expect(actions.getByRole('link', { name: 'I can drive' })).toBeVisible();
    await expect(actions.getByRole('button', { name: 'Going on my own' })).toBeVisible();

    const layout = await page.evaluate(() => {
      const alertEl = document.querySelector('.event-transport-alert');
      const labelEl = document.querySelector('.event-transport-alert-label');
      const actionsEl = document.querySelector('.event-transport-alert-actions');
      if (!alertEl || !labelEl || !actionsEl) {
        return null;
      }

      const toBox = (el: Element) => {
        const rect = el.getBoundingClientRect();
        return { left: rect.left, right: rect.right, width: rect.width };
      };

      const buttons = Array.from(actionsEl.querySelectorAll('.btn')).map((btn) => ({
        text: btn.textContent?.trim() ?? '',
        ...toBox(btn),
      }));

      return {
        alert: toBox(alertEl),
        label: toBox(labelEl),
        actions: toBox(actionsEl),
        buttons,
        scrollWidth: document.documentElement.scrollWidth,
        innerWidth: window.innerWidth,
      };
    });

    expect(layout).not.toBeNull();
    if (!layout) return;

    expect(layout.scrollWidth).toBeLessThanOrEqual(layout.innerWidth);
    expect(layout.alert.right).toBeLessThanOrEqual(viewport.width + 1);

    for (const box of [layout.label, layout.actions, ...layout.buttons]) {
      expect(box.left).toBeGreaterThanOrEqual(layout.alert.left - 1);
      expect(box.right).toBeLessThanOrEqual(layout.alert.right + 1);
    }
  });
}
