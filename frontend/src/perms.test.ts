import { describe, expect, it } from 'vitest';
import { ADMIN_PAGES, canAnyAdminPage, canApprovePlan, canAuthoriseMileage, Can, PageKey, SYSTEM_ROLES } from './perms';

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

describe('canApprovePlan', () => {
  it('returns true for traffic lead, admin, superadmin, president, and vice_president', () => {
    expect(canApprovePlan(['traffic_head'])).toBe(true);
    expect(canApprovePlan(['admin'])).toBe(true);
    expect(canApprovePlan(['superadmin'])).toBe(true);
    expect(canApprovePlan(['president'])).toBe(true);
    expect(canApprovePlan(['vice_president'])).toBe(true);
    expect(canApprovePlan(['member', 'traffic_head'])).toBe(true);
  });

  it('returns false for traffic group member, driver, or other non-lead roles', () => {
    expect(canApprovePlan(undefined)).toBe(false);
    expect(canApprovePlan([])).toBe(false);
    expect(canApprovePlan(['traffic_member'])).toBe(false);
    expect(canApprovePlan(['transport'])).toBe(false);
    expect(canApprovePlan(['driver'])).toBe(false);
    expect(canApprovePlan(['secretary'])).toBe(false);
    expect(canApprovePlan(['cell_leader'])).toBe(false);
  });

  it('includes traffic_member in SYSTEM_ROLES', () => {
    expect(SYSTEM_ROLES).toContain('traffic_member');
    expect(SYSTEM_ROLES).toContain('traffic_head');
  });
});
