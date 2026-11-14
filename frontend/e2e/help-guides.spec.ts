import { expect, test, Page } from '@playwright/test';

const memberGuide = {
  id: 1,
  slug: 'getting-started',
  title_en: 'Getting started',
  title_zh: '开始使用',
  summary_en: 'Set up your account',
  summary_zh: '设置账号',
  audience: 'member',
  published: true,
  step_count: 1,
  steps: [{ title_en: 'Open events', title_zh: '打开活动', body_en: 'Browse events', body_zh: '浏览活动', link: '/events' }],
};

const staffGuide = {
  ...memberGuide,
  id: 2,
  slug: 'staff-only',
  title_en: 'Staff only',
  title_zh: '职员指南',
  audience: 'staff',
};

async function mockApi(page: Page, audience: 'member' | 'staff') {
  const permissions = audience === 'member'
    ? ['help.view', 'events.view']
    : ['help.view', 'events.view', 'admin_dashboard.view', 'admin_help.update', 'admin_audit.view'];
  await page.route('**/api/**', async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    if (path === '/api/auth/me') {
      return route.fulfill({ status: 200, json: {
        id: audience === 'member' ? 2 : 1,
        name: audience === 'member' ? 'Member' : 'Staff editor',
        email: audience === 'member' ? 'member@example.org' : 'staff@example.org',
        roles: audience === 'member' ? [] : ['admin'],
        permissions,
        status: 'active',
        terms_accepted: true,
        mfa_ok: true,
        groups: [],
        unread: 0,
      } });
    }
    if (path === '/api/help/guides') {
      return route.fulfill({ status: 200, json: audience === 'member' ? [memberGuide] : [memberGuide, staffGuide] });
    }
    if (path === '/api/help/guides/getting-started') {
      return route.fulfill({ status: 200, json: memberGuide });
    }
    if (path === '/api/help/guides/staff-only') {
      return route.fulfill(audience === 'member'
        ? { status: 404, json: { code: 'NOT_FOUND', error: 'Not found' } }
        : { status: 200, json: staffGuide });
    }
    if (path === '/api/help') return route.fulfill({ status: 200, json: [] });
    if (path === '/api/events') return route.fulfill({ status: 200, json: [] });
    if (path === '/api/admin/audit') {
      return route.fulfill({ status: 200, json: {
        items: [{ id: 10, actor_id: 1, actor: 'Staff editor', action: 'ride.claim', entity: 'ride', entity_id: 17, detail: 'Claimed', created_at: '2026-11-14T10:00:00' }],
        next_before: null,
        retention_days: 730,
      } });
    }
    return route.fulfill({ status: 200, json: {} });
  });
}

test('member sees only member guides and a staff guide is not found', async ({ page }) => {
  await mockApi(page, 'member');
  await page.goto('/help?tab=guides');
  await expect(page.getByRole('link', { name: /Getting started/ })).toBeVisible();
  await expect(page.getByText('Staff only')).toHaveCount(0);
  await page.goto('/help/guides/staff-only');
  await expect(page.getByText('Not found')).toBeVisible();
});

test('staff sees both audiences, can edit audience and follows guide links in app', async ({ page }) => {
  await mockApi(page, 'staff');
  await page.goto('/help?tab=guides');
  await expect(page.getByRole('heading', { name: 'For members' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'For staff' })).toBeVisible();
  await expect(page.getByText('Staff only')).toBeVisible();
  await page.getByLabel('Show guides for').selectOption('member');
  await expect(page.getByText('Staff only')).toHaveCount(0);
  await page.getByRole('button', { name: 'New guide' }).click();
  await expect(page.getByLabel('Audience')).toBeVisible();
  await page.getByLabel('Audience').selectOption('member');
  await expect(page.getByLabel('Audience')).toHaveValue('member');
  await page.goto('/help/guides/getting-started');
  await page.getByRole('link', { name: 'Open →' }).click();
  await expect(page).toHaveURL(/\/events$/);
  await expect(page.getByRole('heading', { name: /events/i })).toBeVisible();
});

test('audit object click sets entity and entity_id together', async ({ page }) => {
  await mockApi(page, 'staff');
  await page.goto('/admin/audit');
  await page.getByTitle('Filter by object').click();
  await expect(page).toHaveURL(/entity=ride/);
  await expect(page).toHaveURL(/entity_id=17/);
});
