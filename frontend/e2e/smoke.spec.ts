import { test, expect } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.route('**/api/**', async (route) => {
    const url = route.request().url();
    if (url.includes('/api/auth/me')) {
      return route.fulfill({ status: 401, json: { error: 'unauthorized' } });
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
    return route.fulfill({ status: 404, json: { error: 'mocked' } });
  });
});

test('login page loads and shows passkey gate', async ({ page }) => {
  await page.goto('/login');
  await expect(page.locator('h2')).toBeVisible();
  await expect(page.locator('input[type="email"]')).toBeVisible();
});

test('unauthenticated home redirects to login', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveURL(/\/login/);
});
