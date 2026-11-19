import { useEffect, useRef, useState } from 'react';
import {
  decideSwipeAction,
  type BottomTab,
  type SwipeNavAction,
} from '../bottomNav';

const LOCK_DISTANCE_PX = 12;
const THRESHOLD_PX = 96;
const QUICK_FLICK_MIN_PX = 60;
/** Ignore starts near either screen edge so this does not fight the iOS/app back-swipe. */
export const SWIPE_EDGE_IGNORE_PX = 32;
export const MOBILE_NAV_MQ = '(max-width: 767px)';

export type SwipeTabsAction = Exclude<SwipeNavAction, { type: 'none' }>;

function touchTarget(raw: EventTarget | null): Element | null {
  if (raw instanceof Element) return raw;
  if (raw instanceof Node && raw.parentElement) return raw.parentElement;
  return null;
}

/** True when the touch started inside something that scrolls sideways (tab strips, tables, carousels). */
function inHorizontalScroller(target: Element): boolean {
  let el: Element | null = target;
  while (el && el !== document.documentElement) {
    if (el.scrollWidth > el.clientWidth + 1) {
      const ox = window.getComputedStyle(el).overflowX;
      if (ox === 'auto' || ox === 'scroll') return true;
    }
    el = el.parentElement;
  }
  return false;
}

function hasTextSelection(): boolean {
  const sel = window.getSelection();
  return Boolean(sel && !sel.isCollapsed && sel.toString().trim());
}

/** True when the viewport matches the phone layout that shows the bottom tab bar. */
export function useMobileNav(): boolean {
  const [mobile, setMobile] = useState(() =>
    typeof window !== 'undefined' && typeof window.matchMedia === 'function'
      ? window.matchMedia(MOBILE_NAV_MQ).matches
      : false,
  );

  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia(MOBILE_NAV_MQ);
    const apply = () => setMobile(mq.matches);
    apply();
    mq.addEventListener?.('change', apply);
    return () => mq.removeEventListener?.('change', apply);
  }, []);

  return mobile;
}

/**
 * Mid-screen swipe between the phone bottom-nav tabs.
 * On a sub-page, in-app history in the swiped direction is used first; if there is
 * none, the neighbouring tab is opened instead. Never wraps, and never runs on
 * desktop widths. Starts within the screen-edge ignore zone are left to swipe-back.
 */
export function SwipeTabsGesture({
  enabled,
  tabs,
  helpDesk,
  pathname,
  search,
  canGoBack,
  canGoForward,
  onAction,
}: {
  enabled: boolean;
  tabs: readonly BottomTab[];
  helpDesk?: boolean;
  pathname: string;
  search: string;
  canGoBack: boolean;
  canGoForward: boolean;
  onAction: (action: SwipeTabsAction) => void;
}) {
  const stateRef = useRef({
    tabs,
    helpDesk: Boolean(helpDesk),
    pathname,
    search,
    canGoBack,
    canGoForward,
    onAction,
  });
  stateRef.current = {
    tabs,
    helpDesk: Boolean(helpDesk),
    pathname,
    search,
    canGoBack,
    canGoForward,
    onAction,
  };

  useEffect(() => {
    if (!enabled) return;
    let g: { x: number; y: number; t: number; locked: 'left' | 'right' | null } | null = null;

    const start = (e: TouchEvent) => {
      g = null;
      if (e.touches.length !== 1) return;
      if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
      if (hasTextSelection()) return;
      const touch = e.touches[0];
      if (touch.clientX <= SWIPE_EDGE_IGNORE_PX) return;
      if (touch.clientX >= window.innerWidth - SWIPE_EDGE_IGNORE_PX) return;
      const target = touchTarget(e.target);
      if (!target) return;
      if (
        target.closest(
          '.leaflet-container, [data-no-swipe-next], [data-no-swipe-back], [draggable="true"], input, textarea, select, ' +
            '[contenteditable]:not([contenteditable="false"]), .sidebar, .bottom-nav',
        )
      ) return;
      if (inHorizontalScroller(target)) return;
      g = { x: touch.clientX, y: touch.clientY, t: Date.now(), locked: null };
    };

    const move = (e: TouchEvent) => {
      if (!g) return;
      if (e.touches.length !== 1) {
        g = null;
        return;
      }
      const touch = e.touches[0];
      const dx = touch.clientX - g.x;
      const dy = touch.clientY - g.y;
      if (!g.locked) {
        if (Math.hypot(dx, dy) < LOCK_DISTANCE_PX) return;
        if (Math.abs(dx) > Math.abs(dy) * 1.6) g.locked = dx < 0 ? 'left' : 'right';
        else g = null;
      }
    };

    const end = (e: TouchEvent) => {
      const s = g;
      g = null;
      if (!s || !s.locked) return;
      const touch = e.changedTouches?.[0];
      if (!touch) return;
      const dx = touch.clientX - s.x;
      const dy = touch.clientY - s.y;
      const along = s.locked === 'left' ? -dx : dx;
      const velocity = along / Math.max(1, Date.now() - s.t);
      if (
        along < THRESHOLD_PX &&
        !(along >= QUICK_FLICK_MIN_PX && Math.abs(dy) < 36 && velocity >= 0.6)
      ) {
        return;
      }
      const cur = stateRef.current;
      const action = decideSwipeAction({
        direction: s.locked,
        pathname: cur.pathname,
        search: cur.search,
        tabs: cur.tabs,
        helpDesk: cur.helpDesk,
        canGoBack: cur.canGoBack,
        canGoForward: cur.canGoForward,
      });
      if (action.type !== 'none') cur.onAction(action);
    };

    const cancel = () => {
      g = null;
    };

    window.addEventListener('touchstart', start, { passive: true });
    window.addEventListener('touchmove', move, { passive: true });
    window.addEventListener('touchend', end, { passive: true });
    window.addEventListener('touchcancel', cancel, { passive: true });
    return () => {
      window.removeEventListener('touchstart', start);
      window.removeEventListener('touchmove', move);
      window.removeEventListener('touchend', end);
      window.removeEventListener('touchcancel', cancel);
    };
  }, [enabled]);

  return null;
}
