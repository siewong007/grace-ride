import { test, expect, Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

/** The boot splash (index.html) covers the app until its intro finishes; wait for it to go. */
const splashGone = (page: Page) => page.locator('#splash').waitFor({ state: 'detached' });

const mockMe = {
  id: 1,
  name: 'Sister Mary',
  email: 'mary@example.org',
  phone: '+60123456789',
  roles: ['admin', 'driver', 'transport', 'finance'],
  mfa_ok: true,
};

const mockRide = {
  id: 101,
  status: 'confirmed',
  pickup_at: '2026-11-20T10:00:00',
  destination: 'Grace Community Church',
  seats: 1,
  pickup_address: '123 Jalan Damai, Petaling Jaya',
  area: { id: 1, name: 'Petaling Jaya' },
  passenger: { id: 1, name: 'Sister Mary', phone: '+60123456789' },
  driver: {
    id: 2,
    name: 'Brother Silas',
    phone: '+60198765432',
    vehicle: { model: 'Toyota Vios', color: 'Silver', plate: 'WXX 8888', seats: 5 },
  },
  assistance: null,
  notes: 'Near the guardhouse',
  created_at: '2026-11-19T08:00:00',
  picked_up_at: null,
  completed_at: null,
  cancelled_at: null,
  timeline: [
    { status: 'requested', at: '2026-11-19T08:00:00', by: 'Sister Mary' },
    { status: 'matched', at: '2026-11-19T09:30:00', by: 'Brother Silas' },
    { status: 'confirmed', at: '2026-11-19T10:00:00', by: 'Transport Admin' },
  ],
  viewer: { is_passenger: true, is_driver: false, is_staff: false },
};

const mockDriverSchedule = [mockRide];

const mockDriverProfile = {
  carpool_enabled: true,
  spare_seats: 2,
  area_ids: [1],
  vehicle: {
    model: 'Perodua Myvi',
    color: 'White',
    plate: 'VXX 1234',
    seats: 5,
  },
};

const mockDashboard = {
  today: '2026-11-20',
  today_total: 4,
  today_confirmed: 3,
  today_changes: 0,
  pending_approval: 1,
  no_driver: 0,
  needs_attention: 0,
  live_drivers: 2,
  max_live_drivers: 10,
  open_help: 1,
  pending_reimbursements: 1,
  members: 42,
  pending_recovery: 0,
};

test.beforeEach(async ({ page }) => {
  await page.route('**/api/**', async (route) => {
    const url = route.request().url();

    if (url.includes('/api/auth/me')) {
      return route.fulfill({ status: 200, json: mockMe });
    }
    if (url.includes('/api/auth/dev-users')) {
      return route.fulfill({
        status: 200,
        json: [{ name: 'Demo', email: 'member@example.org', roles: '' }],
      });
    }
    if (url.includes('/api/health')) {
      return route.fulfill({ status: 200, json: { ok: true, db: true } });
    }
    if (url.includes('/api/rides/101/messages')) {
      return route.fulfill({ status: 200, json: [] });
    }
    if (url.includes('/api/rides/101/location')) {
      return route.fulfill({ status: 200, json: { sharing: false, points: [] } });
    }
    if (url.includes('/api/rides/101')) {
      return route.fulfill({ status: 200, json: mockRide });
    }
    if (url.includes('/api/rides')) {
      return route.fulfill({ status: 200, json: [mockRide] });
    }
    if (url.includes('/api/driver/schedule')) {
      return route.fulfill({ status: 200, json: mockDriverSchedule });
    }
    if (url.includes('/api/driver/requests')) {
      return route.fulfill({ status: 200, json: [] });
    }
    if (url.includes('/api/driver/profile')) {
      return route.fulfill({ status: 200, json: mockDriverProfile });
    }
    if (url.includes('/api/areas')) {
      return route.fulfill({ status: 200, json: [{ id: 1, name: 'PJ', active: true }] });
    }
    if (url.includes('/api/admin/dashboard')) {
      return route.fulfill({ status: 200, json: mockDashboard });
    }
    if (url.includes('/api/admin/rides')) {
      return route.fulfill({ status: 200, json: [mockRide] });
    }
    if (url.includes('/api/costs')) {
      return route.fulfill({ status: 200, json: { balance_cents: 0, items: [] } });
    }
    if (url.includes('/api/events')) {
      return route.fulfill({ status: 200, json: [] });
    }
    if (url.includes('/api/announcements')) {
      return route.fulfill({ status: 200, json: [] });
    }
    if (url.includes('/api/notifications')) {
      return route.fulfill({ status: 200, json: [] });
    }

    return route.fulfill({ status: 200, json: {} });
  });
});

const pagesToTest = [
  { name: 'Home', path: '/' },
  { name: 'RideDetail', path: '/rides/101' },
  { name: 'Drive', path: '/drive' },
  { name: 'Dashboard', path: '/admin' },
];

test.describe('Visual regression and accessibility', () => {
  for (const p of pagesToTest) {
    test(`${p.name} - 375px mobile & accessibility`, async ({ page }) => {
      await page.setViewportSize({ width: 375, height: 812 });
      await page.goto(p.path);
      await page.waitForLoadState('networkidle');
      await splashGone(page);

      // Accessibility audit
      const a11yResults = await new AxeBuilder({ page }).analyze();
      const criticalOrSerious = a11yResults.violations.filter(
        (v) => v.impact === 'critical' || v.impact === 'serious'
      );
      expect(criticalOrSerious).toEqual([]);

      // Light mode snapshot
      await expect(page).toHaveScreenshot(`${p.name}-375-light.png`, {
        maxDiffPixelRatio: 0.05,
      });

      // Dark mode snapshot
      await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
      await expect(page).toHaveScreenshot(`${p.name}-375-dark.png`, {
        maxDiffPixelRatio: 0.05,
      });
    });

    test(`${p.name} - 1280px desktop & accessibility`, async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto(p.path);
      await page.waitForLoadState('networkidle');
      await splashGone(page);

      // Accessibility audit
      const a11yResults = await new AxeBuilder({ page }).analyze();
      const criticalOrSerious = a11yResults.violations.filter(
        (v) => v.impact === 'critical' || v.impact === 'serious'
      );
      expect(criticalOrSerious).toEqual([]);

      // Light mode snapshot
      await expect(page).toHaveScreenshot(`${p.name}-1280-light.png`, {
        maxDiffPixelRatio: 0.05,
      });

      // Dark mode snapshot
      await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
      await expect(page).toHaveScreenshot(`${p.name}-1280-dark.png`, {
        maxDiffPixelRatio: 0.05,
      });
    });
  }

  test('Login - visual & accessibility (unauthenticated)', async ({ page }) => {
    // Override /api/auth/me for login test to return 401
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({ status: 401, json: { error: 'unauthorized' } })
    );

    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
    await splashGone(page);

    // Accessibility audit
    const a11yResults = await new AxeBuilder({ page }).analyze();
    const criticalOrSerious = a11yResults.violations.filter(
      (v) => v.impact === 'critical' || v.impact === 'serious'
    );
    expect(criticalOrSerious).toEqual([]);

    await expect(page).toHaveScreenshot('Login-375-light.png', {
      maxDiffPixelRatio: 0.05,
    });

    await page.setViewportSize({ width: 1280, height: 800 });
    await expect(page).toHaveScreenshot('Login-1280-light.png', {
      maxDiffPixelRatio: 0.05,
    });
  });
});
