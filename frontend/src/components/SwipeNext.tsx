import { useEffect, useRef } from 'react';

const LOCK_DISTANCE_PX = 12;
const THRESHOLD_PX = 96;
const QUICK_FLICK_MIN_PX = 60;

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

/**
 * Global swipe-left-for-next-page gesture. Mirrors SwipeBackGesture: ignores maps, inputs,
 * modals, sideways-scrolling areas and drags that are mostly vertical.
 */
export function SwipeNextGesture({ enabled, onNext }: { enabled: boolean; onNext: () => void }) {
  const onNextRef = useRef(onNext);
  onNextRef.current = onNext;

  useEffect(() => {
    if (!enabled) return;
    let g: { x: number; y: number; t: number; locked: boolean } | null = null;

    const start = (e: TouchEvent) => {
      g = null;
      if (e.touches.length !== 1) return;
      if (document.querySelector('[role="dialog"][aria-modal="true"]')) return;
      const target = touchTarget(e.target);
      if (!target) return;
      if (
        target.closest(
          '.leaflet-container, [data-no-swipe-next], [data-no-swipe-back], [draggable="true"], input, textarea, select, ' +
            '[contenteditable]:not([contenteditable="false"]), .sidebar, .bottom-nav',
        )
      ) return;
      if (inHorizontalScroller(target)) return;
      const t = e.touches[0];
      g = { x: t.clientX, y: t.clientY, t: Date.now(), locked: false };
    };

    const move = (e: TouchEvent) => {
      if (!g) return;
      if (e.touches.length !== 1) { g = null; return; }
      const t = e.touches[0];
      const dx = t.clientX - g.x;
      const dy = t.clientY - g.y;
      if (!g.locked) {
        if (Math.hypot(dx, dy) < LOCK_DISTANCE_PX) return;
        if (dx < 0 && -dx > Math.abs(dy) * 1.6) g.locked = true;
        else g = null;
      }
    };

    const end = (e: TouchEvent) => {
      const s = g;
      g = null;
      if (!s || !s.locked) return;
      const t = e.changedTouches?.[0];
      if (!t) return;
      const dx = s.x - t.clientX;
      const dy = t.clientY - s.y;
      const velocity = dx / Math.max(1, Date.now() - s.t);
      if (dx >= THRESHOLD_PX || (dx >= QUICK_FLICK_MIN_PX && Math.abs(dy) < 36 && velocity >= 0.6)) {
        onNextRef.current();
      }
    };

    const cancel = () => { g = null; };

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
