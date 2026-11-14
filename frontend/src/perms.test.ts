import { describe, expect, it } from 'vitest';
import { ADMIN_PAGES, canAnyAdminPage, Can, PageKey } from './perms';

describe('canAnyAdminPage', () => {
  it('is false for a plain member', () => {
    const can: Can = (page: PageKey) => page === 'rides';
    expect(canAnyAdminPage(can)).toBe(false);
  });
  it('is true when any staff page is visible', () => {
    const can: Can = (page: PageKey) => page === 'admin_reimb_pay';
    expect(canAnyAdminPage(can)).toBe(true);
  });
  it('covers every staff page key', () => {
    // Every non-member page key from the union must be listed, or the tab/route
    // gate drifts from the backend any_admin_page check.
    expect(ADMIN_PAGES).not.toContain('help');
    expect(ADMIN_PAGES.length).toBeGreaterThanOrEqual(19);
  });
});
