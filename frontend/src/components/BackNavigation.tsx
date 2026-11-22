import React, {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useLocation, useNavigate, useNavigationType } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import type { Can } from '../perms';
import { ADMIN_HOME } from '../staffNav';

export interface BackHistoryEntry {
  key: string;
  to: string;
  pathname: string;
}

interface BackNavigationContextValue {
  previousEntry: BackHistoryEntry | null;
  nextEntry: BackHistoryEntry | null;
}

const BackNavigationContext = createContext<BackNavigationContextValue | null>(null);

const MAX_HISTORY_ENTRIES = 200;

interface HistoryState {
  entries: BackHistoryEntry[];
  index: number;
}

function reduceBackHistory(
  prev: HistoryState,
  entry: BackHistoryEntry,
  navType: ReturnType<typeof useNavigationType>,
): HistoryState {
  const current = prev.entries[prev.index];
  if (current && current.key === entry.key && current.to === entry.to) {
    return prev;
  }

  if (navType === 'PUSH') {
    if (current && current.to === entry.to) {
      const entries = prev.entries.slice();
      entries[prev.index] = entry;
      return { entries, index: prev.index };
    }
    let entries = prev.entries.slice(0, prev.index + 1);
    entries.push(entry);
    if (entries.length > MAX_HISTORY_ENTRIES) {
      entries = entries.slice(entries.length - MAX_HISTORY_ENTRIES);
    }
    return { entries, index: entries.length - 1 };
  }

  if (navType === 'REPLACE') {
    const entries = prev.entries.slice();
    entries[prev.index] = entry;
    return { entries, index: prev.index };
  }

  // POP: match backward from index - 1 first (back navigation), then forward (browser forward).
  let matchIdx = -1;
  for (let i = prev.index - 1; i >= 0; i--) {
    if (prev.entries[i].key === entry.key) {
      matchIdx = i;
      break;
    }
  }
  if (matchIdx === -1) {
    for (let i = prev.index + 1; i < prev.entries.length; i++) {
      if (prev.entries[i].key === entry.key) {
        matchIdx = i;
        break;
      }
    }
  }
  if (matchIdx !== -1) {
    const entries = prev.entries.slice();
    entries[matchIdx] = entry;
    return { entries, index: matchIdx };
  }

  return { entries: [entry], index: 0 };
}

function BackNavigationTracker({ children }: { children: ReactNode }) {
  const loc = useLocation();
  const navType = useNavigationType();
  const nextTo = loc.pathname + loc.search;
  const entry: BackHistoryEntry = { key: loc.key, to: nextTo, pathname: loc.pathname };

  const [state, setState] = useState<HistoryState>(() => ({
    entries: [{ key: loc.key, to: nextTo, pathname: loc.pathname }],
    index: 0,
  }));

  const current = state.entries[state.index];
  let active = state;
  if (!current || current.key !== entry.key || current.to !== entry.to) {
    active = reduceBackHistory(state, entry, navType);
    setState(active);
  }

  const previousEntry = active.index > 0 ? active.entries[active.index - 1] : null;
  const nextEntry = active.index < active.entries.length - 1 ? active.entries[active.index + 1] : null;
  const value = useMemo(() => ({ previousEntry, nextEntry }), [previousEntry, nextEntry]);

  return <BackNavigationContext.Provider value={value}>{children}</BackNavigationContext.Provider>;
}

/**
 * Tracks in-app navigation history so Back buttons and swipe-back gestures return to
 * the previous in-app screen when available. Safe to nest; an inner provider is a no-op.
 */
export function BackNavigationProvider({ children }: { children: ReactNode }) {
  const existing = useContext(BackNavigationContext);
  if (existing) return <>{children}</>;
  return <BackNavigationTracker>{children}</BackNavigationTracker>;
}

export function useOptionalBackNavigation(): BackNavigationContextValue | null {
  return useContext(BackNavigationContext);
}

/**
 * Route-derived fallback target for any non-Home page when opened directly or when
 * a page does not explicitly supply `back` to PageHeader.
 */
export function getDefaultBack(pathname: string, can?: Can): string {
  const path = pathname.length > 1 && pathname.endsWith('/') ? pathname.slice(0, -1) : pathname;
  if (!path || path === '/') return '/';
  if (path === '/rides') return '/me';
  if (path.startsWith('/rides/')) return '/rides';
  if (path === '/drive') return '/me';
  if (path === '/events') return '/';
  if (path === '/events/new') return '/events';
  const eventSub = path.match(/^\/events\/([^/]+)\/(checkin|edit)$/);
  if (eventSub) return `/events/${eventSub[1]}`;
  if (/^\/events\/[^/]+\/settle$/.test(path)) return '/settlements';
  if (/^\/events\/[^/]+$/.test(path)) return '/events';
  if (path === '/settlements') return '/';
  const settlementEdit = path.match(/^\/settlements\/([^/]+)\/edit$/);
  if (settlementEdit) return `/settlements/${settlementEdit[1]}`;
  if (/^\/settlements\/[^/]+$/.test(path)) return '/settlements';
  if (path === '/costs') return '/me';
  if (path === '/reimbursements') return '/me';
  if (path.startsWith('/reimbursements/')) return '/reimbursements';
  if (path === '/help/guides' || path.startsWith('/help/guides/')) return '/help?tab=guides';
  if (path.startsWith('/help/')) return '/help';
  if (path === '/help') {
    return can?.('admin_help') ? (can?.('admin_dashboard') ? '/admin' : '/') : '/me';
  }
  if (path === '/report') return '/me';
  if (path === '/notifications') return '/';
  if (path === '/me') return '/';
  if (path === '/admin/menu') return '/';
  if (path === '/admin/traffic') {
    const hasNonTraffic = (
      [
        'admin_members', 'admin_invites', 'admin_recovery', 'admin_roles',
        'admin_reimb_review', 'admin_reimb_pay', 'admin_reports',
        'admin_announcements', 'admin_help', 'admin_groups', 'admin_settings', 'admin_audit',
      ] as const
    ).some((p) => Boolean(can?.(p)));
    return hasNonTraffic ? ADMIN_HOME : '/';
  }
  if (path.startsWith('/admin/rides/')) return can?.('admin_plan') || can?.('admin_rides') ? '/admin/traffic?tab=rides' : '/admin/rides';
  const planMatch = path.match(/^\/admin\/events\/([^/]+)\/plan$/);
  if (planMatch) return can?.('admin_plan') && can?.('admin_dashboard') ? '/admin/traffic' : `/events/${planMatch[1]}`;
  if (path === '/admin' || path.startsWith('/admin/')) return ADMIN_HOME;
  return '/';
}

/**
 * Resolves a smart back action that pops valid in-app history when available and falls
 * back to `fallbackTo` on direct/deep-link entry (never popping authenticated users to auth routes).
 */
export function useSmartBack(
  fallbackTo: string | null,
  opts?: { disabled?: boolean; isAuthenticated?: boolean },
) {
  const ctx = useOptionalBackNavigation();
  const navigate = useNavigate();
  const disabled = Boolean(opts?.disabled);
  const isAuthenticated = Boolean(opts?.isAuthenticated);
  const previousEntry = ctx?.previousEntry ?? null;

  const canGoBackInApp =
    !disabled &&
    previousEntry !== null &&
    !(
      isAuthenticated &&
      (previousEntry.pathname === '/login' || previousEntry.pathname.startsWith('/invite/'))
    );

  const href = canGoBackInApp ? previousEntry.to : (fallbackTo ?? '/');

  const goBack = useCallback(() => {
    if (disabled) return;
    if (canGoBackInApp) {
      navigate(-1);
    } else if (fallbackTo) {
      navigate(fallbackTo);
    }
  }, [disabled, canGoBackInApp, navigate, fallbackTo]);

  const onClick = useCallback(
    (e: React.MouseEvent<HTMLElement>) => {
      if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) {
        return;
      }
      e.preventDefault();
      goBack();
    },
    [goBack],
  );

  return { href, canGoBackInApp, goBack, onClick };
}

const EDGE_ZONE_PX = 32;
const LOCK_DISTANCE_PX = 12;
const EDGE_THRESHOLD_PX = 72;
const CONTENT_THRESHOLD_PX = 96;

interface GestureState {
  status: 'idle' | 'pending' | 'swiping' | 'rejected';
  startX: number;
  startY: number;
  lastX: number;
  lastY: number;
  startTime: number;
  isEdge: boolean;
}

const IDLE_GESTURE: GestureState = {
  status: 'idle',
  startX: 0,
  startY: 0,
  lastX: 0,
  lastY: 0,
  startTime: 0,
  isEdge: false,
};

function resolveTouchTarget(rawTarget: EventTarget | null): Element | null {
  if (rawTarget instanceof Element) return rawTarget;
  if (rawTarget instanceof Node && rawTarget.parentElement) return rawTarget.parentElement;
  if (rawTarget === document || rawTarget === window) return document.body;
  return null;
}

function hasScrolledHorizontalAncestor(target: Element): boolean {
  let el: Element | null = target;
  while (el && el !== document.documentElement) {
    if (el.scrollWidth > el.clientWidth + 1) {
      const ox = window.getComputedStyle(el).overflowX;
      if ((ox === 'auto' || ox === 'scroll') && el.scrollLeft > 0) {
        return true;
      }
    }
    el = el.parentElement;
  }
  return false;
}

/**
 * Global touch swipe-right-to-go-back handler with a floating left-edge arrow indicator.
 * Supports left-edge swipes and horizontal right-drags on non-scrollable content.
 */
export function SwipeBackGesture({
  enabled,
  onBack,
  edgeOnly = false,
}: {
  enabled: boolean;
  onBack: () => void;
  /** When set, only left-edge swipes go back, so mid-screen swipes can switch tabs. */
  edgeOnly?: boolean;
}) {
  const loc = useLocation();
  const indicatorRef = useRef<HTMLDivElement>(null);
  const gestureRef = useRef<GestureState>({ ...IDLE_GESTURE });
  const activeRef = useRef(false);
  const armedRef = useRef(false);
  const [active, setActive] = useState(false);
  const [armed, setArmed] = useState(false);
  const onBackRef = useRef(onBack);
  onBackRef.current = onBack;
  const edgeOnlyRef = useRef(edgeOnly);
  edgeOnlyRef.current = edgeOnly;

  const resetGesture = useCallback(() => {
    gestureRef.current = { ...IDLE_GESTURE };
    const el = indicatorRef.current;
    if (el) {
      el.style.transform = '';
      el.style.opacity = '';
    }
    if (activeRef.current) {
      activeRef.current = false;
      setActive(false);
    }
    if (armedRef.current) {
      armedRef.current = false;
      setArmed(false);
    }
  }, []);

  useEffect(() => {
    resetGesture();
  }, [enabled, loc.key, resetGesture]);

  useEffect(() => {
    if (!enabled) return;

    const onTouchStart = (e: TouchEvent) => {
      if (e.touches.length !== 1) {
        resetGesture();
        return;
      }
      if (document.querySelector('[role="dialog"][aria-modal="true"]')) {
        return;
      }
      const target = resolveTouchTarget(e.target);
      if (!target) return;
      if (
        target.closest(
          '.leaflet-container, [data-no-swipe-back], [draggable="true"], input[type="range"], .sidebar, .bottom-nav',
        )
      ) {
        return;
      }

      const touch = e.touches[0];
      const isEdge = touch.clientX <= EDGE_ZONE_PX;
      if (!isEdge && edgeOnlyRef.current) return;
      if (!isEdge) {
        if (target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])')) {
          return;
        }
        if (hasScrolledHorizontalAncestor(target)) {
          return;
        }
      }

      gestureRef.current = {
        status: 'pending',
        startX: touch.clientX,
        startY: touch.clientY,
        lastX: touch.clientX,
        lastY: touch.clientY,
        startTime: Date.now(),
        isEdge,
      };
    };

    const onTouchMove = (e: TouchEvent) => {
      const g = gestureRef.current;
      if (g.status === 'idle' || g.status === 'rejected') return;
      if (e.touches.length !== 1) {
        resetGesture();
        return;
      }

      const touch = e.touches[0];
      const dx = touch.clientX - g.startX;
      const dy = touch.clientY - g.startY;
      g.lastX = touch.clientX;
      g.lastY = touch.clientY;

      if (g.status === 'pending') {
        if (Math.hypot(dx, dy) < LOCK_DISTANCE_PX) return;
        const ratio = g.isEdge ? 1.2 : 1.6;
        if (dx > 0 && dx > Math.abs(dy) * ratio) {
          g.status = 'swiping';
        } else {
          g.status = 'rejected';
          return;
        }
      }

      const threshold = g.isEdge ? EDGE_THRESHOLD_PX : CONTENT_THRESHOLD_PX;
      const progress = Math.min(1, Math.max(0, dx) / threshold);
      const nextActive = dx > 0;
      const nextArmed = progress >= 1;

      const el = indicatorRef.current;
      if (el) {
        const reduced =
          typeof window !== 'undefined' &&
          typeof window.matchMedia === 'function' &&
          window.matchMedia('(prefers-reduced-motion: reduce)').matches;
        if (reduced) {
          el.style.transform = 'translate3d(0, -50%, 0)';
        } else {
          const offset = -48 + Math.round(progress * 48);
          el.style.transform = `translate3d(${offset}px, -50%, 0)`;
        }
        el.style.opacity = nextActive ? String(Math.min(1, 0.2 + progress * 0.8)) : '0';
      }

      if (activeRef.current !== nextActive) {
        activeRef.current = nextActive;
        setActive(nextActive);
      }
      if (armedRef.current !== nextArmed) {
        armedRef.current = nextArmed;
        setArmed(nextArmed);
      }
    };

    const onTouchEnd = (e: TouchEvent) => {
      const g = gestureRef.current;
      if (g.status !== 'swiping') {
        resetGesture();
        return;
      }

      const touch = e.changedTouches?.[0];
      const endX = touch ? touch.clientX : g.lastX;
      const endY = touch ? touch.clientY : g.lastY;
      const dx = endX - g.startX;
      const dy = endY - g.startY;
      const dt = Math.max(1, Date.now() - g.startTime);
      const velocity = dx / dt;
      const threshold = g.isEdge ? EDGE_THRESHOLD_PX : CONTENT_THRESHOLD_PX;
      const quickFlickMin = g.isEdge ? 44 : 60;
      const shouldTrigger =
        dx >= threshold || (dx >= quickFlickMin && Math.abs(dy) < 36 && velocity >= 0.6);

      resetGesture();
      if (shouldTrigger) {
        onBackRef.current();
      }
    };

    const onTouchCancel = () => {
      resetGesture();
    };

    window.addEventListener('touchstart', onTouchStart, { passive: true });
    window.addEventListener('touchmove', onTouchMove, { passive: true });
    window.addEventListener('touchend', onTouchEnd, { passive: true });
    window.addEventListener('touchcancel', onTouchCancel, { passive: true });

    return () => {
      window.removeEventListener('touchstart', onTouchStart);
      window.removeEventListener('touchmove', onTouchMove);
      window.removeEventListener('touchend', onTouchEnd);
      window.removeEventListener('touchcancel', onTouchCancel);
    };
  }, [enabled, resetGesture]);

  return (
    <div
      ref={indicatorRef}
      className={`swipe-back-indicator${active ? ' is-active' : ''}${armed ? ' is-ready' : ''}`}
      aria-hidden="true"
      data-testid="swipe-back-indicator"
    >
      <ArrowLeft size={20} />
    </div>
  );
}
