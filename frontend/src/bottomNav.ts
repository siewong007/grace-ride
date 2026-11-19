/**
 * Phone bottom-navigation tabs, in left-to-right order, for the current member.
 * The tab bar and the mobile swipe gesture both use this so they cannot drift apart.
 */

export type BottomTabId = 'home' | 'events' | 'admin' | 'profile';

export interface BottomTab {
  id: BottomTabId;
  to: string;
}

export function getBottomNavTabs(opts: {
  pending: boolean;
  canEvents: boolean;
  hasStaffNav: boolean;
}): BottomTab[] {
  if (opts.pending) return [{ id: 'profile', to: '/me' }];
  const tabs: BottomTab[] = [{ id: 'home', to: '/' }];
  if (opts.canEvents) tabs.push({ id: 'events', to: '/events' });
  if (opts.hasStaffNav) tabs.push({ id: 'admin', to: '/admin/menu' });
  tabs.push({ id: 'profile', to: '/me' });
  return tabs;
}

function normalizePath(pathname: string): string {
  if (!pathname || pathname === '/') return '/';
  return pathname.length > 1 && pathname.endsWith('/') ? pathname.slice(0, -1) : pathname;
}

/** Pages whose member history lives under Profile (rides, driving, costs, and so on). */
function isProfileSection(path: string): boolean {
  return (
    path === '/me' ||
    path === '/rides' ||
    path.startsWith('/rides/') ||
    path === '/drive' ||
    path.startsWith('/drive/') ||
    path === '/costs' ||
    path.startsWith('/costs/') ||
    path === '/reimbursements' ||
    path.startsWith('/reimbursements/') ||
    path === '/report' ||
    path.startsWith('/report/')
  );
}

/**
 * Which bottom tab the current page belongs to, if any.
 * The Admin tab is current on any /admin/* page, and on /help for help-desk staff.
 */
export function matchBottomTab(
  pathname: string,
  tabs: readonly BottomTab[],
  helpDesk = false,
): BottomTab | undefined {
  const path = normalizePath(pathname);
  const has = (id: BottomTabId) => tabs.find((t) => t.id === id);
  if (
    has('admin') &&
    (path === '/admin' ||
      path.startsWith('/admin/') ||
      (helpDesk && (path === '/help' || path.startsWith('/help/'))))
  ) {
    return has('admin');
  }
  if (has('events') && (path === '/events' || path.startsWith('/events/'))) {
    return has('events');
  }
  if (has('home') && path === '/') return has('home');
  if (has('profile') && isProfileSection(path)) return has('profile');
  return undefined;
}

/** True on the tab's own root page (not an event detail, admin sub-page, or profile section). */
export function isBottomTabRoot(pathname: string, search = ''): boolean {
  const path = normalizePath(pathname);
  if (path === '/' || path === '/events' || path === '/admin/menu') return true;
  if (path === '/me' && !new URLSearchParams(search).get('section')) return true;
  return false;
}

export type SwipeDirection = 'left' | 'right';

export type SwipeNavAction =
  | { type: 'history-back' }
  | { type: 'history-forward' }
  | { type: 'tab'; to: string }
  | { type: 'none' };

/**
 * Decide what a mid-screen horizontal swipe should do.
 * Right is back/previous, left is forward/next — the same directions as the
 * existing swipe-back and swipe-next gestures.
 *
 * On a bottom-tab root, only neighbouring tabs are considered (no wrap, no history).
 * On a sub-page, in-app history in that direction is used first; if there is none,
 * fall back to the neighbouring bottom tab of the tab this page belongs to.
 */
export function decideSwipeAction(opts: {
  direction: SwipeDirection;
  pathname: string;
  search?: string;
  tabs: readonly BottomTab[];
  helpDesk?: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
}): SwipeNavAction {
  const search = opts.search ?? '';
  const onRoot = isBottomTabRoot(opts.pathname, search);
  const goingBack = opts.direction === 'right';

  if (!onRoot) {
    if (goingBack && opts.canGoBack) return { type: 'history-back' };
    if (!goingBack && opts.canGoForward) return { type: 'history-forward' };
  }

  const tab = matchBottomTab(opts.pathname, opts.tabs, opts.helpDesk);
  if (!tab) return { type: 'none' };
  const idx = opts.tabs.findIndex((t) => t.id === tab.id);
  if (idx < 0) return { type: 'none' };
  const nextIdx = goingBack ? idx - 1 : idx + 1;
  if (nextIdx < 0 || nextIdx >= opts.tabs.length) return { type: 'none' };
  return { type: 'tab', to: opts.tabs[nextIdx].to };
}
