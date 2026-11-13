// Lets a page talk to the app frame (Layout): where "back" goes, and whether the page's own
// heading is on screen. The top bar shows the page title only once that heading scrolls away,
// so a page never shows its title twice.
import { createContext, RefObject, useContext, useEffect } from 'react';

export interface PageChrome {
  setBack: (to: string | null) => void;
  setHeadingVisible: (visible: boolean) => void;
}

export const PageChromeContext = createContext<PageChrome | null>(null);

/** Height of the sticky top bar; a heading under it counts as scrolled away. */
const TOPBAR_OFFSET = 72;

/**
 * Registers a page heading (and optional back target) with the app frame.
 * Returns false when there is no frame (e.g. a page rendered on its own in a test),
 * so the caller can draw its own back link.
 */
export function usePageChrome(heading: RefObject<HTMLElement | null>, back?: string): boolean {
  const chrome = useContext(PageChromeContext);

  useEffect(() => {
    if (!chrome) return;
    chrome.setBack(back ?? null);
    return () => chrome.setBack(null);
  }, [chrome, back]);

  useEffect(() => {
    const el = heading.current;
    if (!chrome || !el || typeof IntersectionObserver === 'undefined') return;
    // Measure once now: the observer's first report waits for the next painted frame,
    // which would briefly show the top bar title over a heading that is already visible.
    chrome.setHeadingVisible(el.getBoundingClientRect().bottom > TOPBAR_OFFSET);
    const obs = new IntersectionObserver(([entry]) => chrome.setHeadingVisible(entry.isIntersecting), {
      rootMargin: `-${TOPBAR_OFFSET}px 0px 0px 0px`,
    });
    obs.observe(el);
    return () => {
      obs.disconnect();
      chrome.setHeadingVisible(false);
    };
  }, [chrome, heading]);

  return chrome != null;
}
