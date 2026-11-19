import { describe, expect, it } from 'vitest';
import {
  decideSwipeAction,
  getBottomNavTabs,
  isBottomTabRoot,
  matchBottomTab,
  type BottomTab,
} from './bottomNav';

const memberTabs = getBottomNavTabs({ pending: false, canEvents: true, hasStaffNav: false });
const staffTabs = getBottomNavTabs({ pending: false, canEvents: true, hasStaffNav: true });

describe('getBottomNavTabs', () => {
  it('returns Home, Events and Profile for an ordinary member', () => {
    expect(memberTabs.map((t) => t.id)).toEqual(['home', 'events', 'profile']);
    expect(memberTabs.map((t) => t.to)).toEqual(['/', '/events', '/me']);
  });

  it('inserts the Admin tab for staff, pointing at the phone menu', () => {
    expect(staffTabs.map((t) => t.id)).toEqual(['home', 'events', 'admin', 'profile']);
    expect(staffTabs.find((t) => t.id === 'admin')?.to).toBe('/admin/menu');
  });

  it('drops Events when the member cannot open that page', () => {
    expect(
      getBottomNavTabs({ pending: false, canEvents: false, hasStaffNav: false }).map((t) => t.id),
    ).toEqual(['home', 'profile']);
  });

  it('keeps only Profile while the account is pending', () => {
    expect(
      getBottomNavTabs({ pending: true, canEvents: true, hasStaffNav: true }).map((t) => t.id),
    ).toEqual(['profile']);
  });
});

describe('matchBottomTab and isBottomTabRoot', () => {
  it('treats event details as the Events tab, but not as the tab root', () => {
    expect(matchBottomTab('/events/12', memberTabs)?.id).toBe('events');
    expect(isBottomTabRoot('/events/12')).toBe(false);
    expect(isBottomTabRoot('/events')).toBe(true);
  });

  it('treats any /admin/* page as the Admin tab, with only /admin/menu as the root', () => {
    expect(matchBottomTab('/admin/users', staffTabs)?.id).toBe('admin');
    expect(matchBottomTab('/admin', staffTabs)?.id).toBe('admin');
    expect(isBottomTabRoot('/admin/users')).toBe(false);
    expect(isBottomTabRoot('/admin/menu')).toBe(true);
  });

  it('treats /help as the Admin tab only for help-desk staff', () => {
    expect(matchBottomTab('/help', staffTabs, true)?.id).toBe('admin');
    expect(matchBottomTab('/help/4', staffTabs, true)?.id).toBe('admin');
    expect(matchBottomTab('/help', staffTabs, false)).toBeUndefined();
    expect(isBottomTabRoot('/help')).toBe(false);
  });

  it('treats a profile section as a Profile sub-page, not the tab root', () => {
    expect(matchBottomTab('/me', memberTabs)?.id).toBe('profile');
    expect(isBottomTabRoot('/me')).toBe(true);
    expect(isBottomTabRoot('/me', '?section=security')).toBe(false);
    expect(matchBottomTab('/rides/101', memberTabs)?.id).toBe('profile');
  });

  it('does not assign unmatched pages such as notifications to a tab', () => {
    expect(matchBottomTab('/notifications', memberTabs)).toBeUndefined();
    expect(isBottomTabRoot('/notifications')).toBe(false);
  });
});

function decide(
  direction: 'left' | 'right',
  pathname: string,
  tabs: readonly BottomTab[],
  history: { canGoBack?: boolean; canGoForward?: boolean; search?: string; helpDesk?: boolean } = {},
) {
  return decideSwipeAction({
    direction,
    pathname,
    search: history.search,
    tabs,
    helpDesk: history.helpDesk,
    canGoBack: Boolean(history.canGoBack),
    canGoForward: Boolean(history.canGoForward),
  });
}

describe('decideSwipeAction on tab roots', () => {
  it('lets a member swipe Home → Events → Profile and stops at the ends', () => {
    expect(decide('left', '/', memberTabs)).toEqual({ type: 'tab', to: '/events' });
    expect(decide('left', '/events', memberTabs)).toEqual({ type: 'tab', to: '/me' });
    expect(decide('left', '/me', memberTabs)).toEqual({ type: 'none' });
    expect(decide('right', '/me', memberTabs)).toEqual({ type: 'tab', to: '/events' });
    expect(decide('right', '/events', memberTabs)).toEqual({ type: 'tab', to: '/' });
    expect(decide('right', '/', memberTabs)).toEqual({ type: 'none' });
  });

  it('includes Admin in the staff order', () => {
    expect(decide('left', '/events', staffTabs)).toEqual({ type: 'tab', to: '/admin/menu' });
    expect(decide('left', '/admin/menu', staffTabs)).toEqual({ type: 'tab', to: '/me' });
    expect(decide('right', '/me', staffTabs)).toEqual({ type: 'tab', to: '/admin/menu' });
    expect(decide('right', '/admin/menu', staffTabs)).toEqual({ type: 'tab', to: '/events' });
  });

  it('ignores in-app history on a tab root and never wraps', () => {
    expect(decide('right', '/', memberTabs, { canGoBack: true })).toEqual({ type: 'none' });
    expect(decide('left', '/me', memberTabs, { canGoForward: true })).toEqual({ type: 'none' });
    expect(decide('left', '/', memberTabs, { canGoBack: true, canGoForward: true })).toEqual({
      type: 'tab',
      to: '/events',
    });
  });
});

describe('decideSwipeAction on sub-pages', () => {
  it('follows in-app history when the swiped direction has an entry', () => {
    expect(decide('right', '/events/12', memberTabs, { canGoBack: true })).toEqual({
      type: 'history-back',
    });
    expect(decide('left', '/events/12', memberTabs, { canGoForward: true })).toEqual({
      type: 'history-forward',
    });
  });

  it('falls back to the neighbouring bottom tab when that direction has no history', () => {
    expect(decide('right', '/events/12', memberTabs)).toEqual({ type: 'tab', to: '/' });
    expect(decide('left', '/events/12', memberTabs)).toEqual({ type: 'tab', to: '/me' });
    expect(decide('left', '/admin/users', staffTabs)).toEqual({ type: 'tab', to: '/me' });
    expect(decide('right', '/admin/users', staffTabs)).toEqual({ type: 'tab', to: '/events' });
    expect(decide('right', '/me', memberTabs, { search: '?section=security' })).toEqual({
      type: 'tab',
      to: '/events',
    });
  });

  it('does not wrap when a sub-page belongs to the first or last tab and has no history', () => {
    expect(decide('right', '/rides/101', memberTabs)).toEqual({ type: 'tab', to: '/events' });
    expect(decide('left', '/rides/101', memberTabs)).toEqual({ type: 'none' });
    expect(decide('left', '/rides/101', memberTabs, { canGoForward: true })).toEqual({
      type: 'history-forward',
    });
  });

  it('does nothing on a page that is not under a bottom tab when there is no history', () => {
    expect(decide('left', '/notifications', memberTabs)).toEqual({ type: 'none' });
    expect(decide('right', '/notifications', memberTabs)).toEqual({ type: 'none' });
    expect(decide('right', '/notifications', memberTabs, { canGoBack: true })).toEqual({
      type: 'history-back',
    });
  });
});
