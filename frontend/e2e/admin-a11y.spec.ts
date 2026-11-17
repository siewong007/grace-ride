// Staff screens at phone, tablet and desktop size, with mocked data: no crash, no sideways
// page scroll, and no serious accessibility violations. No screenshot baselines: this checks
// structure, so it runs the same on any machine.
import { test, expect, devices, Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { mockAdminApi } from './adminMocks';

const SCREENS = [
  '/admin/menu', '/admin', '/admin/rides', '/admin/rides/101', '/admin/live',
  '/admin/members', '/admin/members?tab=invites', '/admin/members?tab=recovery',
  '/admin/roles', '/admin/reimbursements', '/admin/reimbursements?tab=payout',
  '/admin/reports', '/admin/announcements', '/admin/settings', '/admin/audit', '/admin/events/1/plan',
];

const { defaultBrowserType: _ignored, ...phone } = devices['iPhone 13'];
const SIZES = [
  { name: 'phone', options: phone },
  { name: 'tablet', options: { viewport: { width: 900, height: 1100 } } },
  { name: 'desktop', options: { viewport: { width: 1440, height: 900 } } },
] as const;

async function open(page: Page, path: string) {
  await page.clock.setFixedTime(new Date('2026-11-20T09:00:00'));
  await page.goto(path);
  await page.locator('#splash').waitFor({ state: 'detached' });
  await page.waitForLoadState('networkidle');
}

for (const size of SIZES) {
  test.describe(`admin screens · ${size.name}`, () => {
    test.use(size.options as never);
    test.beforeEach(async ({ context }) => mockAdminApi(context));

    for (const path of SCREENS) {
      test(path, async ({ page }) => {
        await open(page, path);
        await expect(page.getByRole('heading', { name: 'Something went wrong' })).toHaveCount(0);
        await expect(page.locator('main h1').first()).toBeVisible();

        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
        expect(overflow, 'page scrolls sideways').toBeLessThanOrEqual(0);

        // The map's attribution link belongs to Leaflet, not to this app.
        const axe = await new AxeBuilder({ page }).exclude('.leaflet-container').analyze();
        const bad = axe.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical');
        expect(bad.map((v) => `${v.id}: ${v.nodes[0]?.target.join(' ')}`)).toEqual([]);
      });
    }
  });
}

test.describe('tablet navigation rail', () => {
  test.use({ viewport: { width: 900, height: 1100 } });
  test.beforeEach(async ({ context }) => mockAdminApi(context));

  test('opens into a labelled sidebar and remembers it', async ({ page }) => {
    await open(page, '/admin');
    const toggle = page.getByRole('button', { name: 'Show menu names' });
    await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    // Collapsed: names are for screen readers only.
    expect(await page.locator('.sidebar-link .sidebar-label').first().evaluate((el) => el.getBoundingClientRect().width)).toBeLessThan(5);

    await toggle.click();
    await expect(page.getByRole('button', { name: 'Hide menu names' })).toHaveAttribute('aria-expanded', 'true');
    const label = page.locator('.sidebar-link .sidebar-label', { hasText: 'Ride management' });
    await expect(label).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);

    await page.reload();
    await page.locator('#splash').waitFor({ state: 'detached' });
    await expect(page.getByRole('button', { name: 'Hide menu names' })).toBeVisible();
  });
});
