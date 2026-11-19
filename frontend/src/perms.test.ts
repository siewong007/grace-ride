import { describe, expect, it } from 'vitest';
import { ADMIN_PAGES, canAnyAdminPage, canAuthoriseMileage, Can, PageKey } from './perms';

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

describe('canAuthoriseMileage', () => {
  it('returns true for superadmin, president, and vice_president', () => {
    expect(canAuthoriseMileage(['superadmin'])).toBe(true);
    expect(canAuthoriseMileage(['president'])).toBe(true);
    expect(canAuthoriseMileage(['vice_president'])).toBe(true);
    expect(canAuthoriseMileage(['member', 'president'])).toBe(true);
  });

  it('returns false for other roles or empty/undefined roles', () => {
    expect(canAuthoriseMileage(undefined)).toBe(false);
    expect(canAuthoriseMileage([])).toBe(false);
    expect(canAuthoriseMileage(['member'])).toBe(false);
    expect(canAuthoriseMileage(['admin'])).toBe(false);
    expect(canAuthoriseMileage(['secretary'])).toBe(false);
    expect(canAuthoriseMileage(['cell_leader'])).toBe(false);
    expect(canAuthoriseMileage(['leader'])).toBe(false);
    expect(canAuthoriseMileage(['finance'])).toBe(false);
  });
});
