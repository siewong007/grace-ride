/** Thin wrapper over the Web Share API (the phone's own share sheet). */

export interface ShareData {
  title?: string;
  text?: string;
  url?: string;
}

export type ShareResult = 'shared' | 'cancelled' | 'fallback';

/** True when the browser can open the native share sheet for plain text/links. */
export function canNativeShare(): boolean {
  return typeof navigator !== 'undefined' && typeof navigator.share === 'function';
}

/**
 * Open the native share sheet; when it is unavailable or fails for a reason other than
 * the user dismissing it, run `fallback` (e.g. open WhatsApp or copy the link) instead.
 */
export async function shareOrFallback(data: ShareData, fallback: () => void): Promise<ShareResult> {
  if (canNativeShare() && (typeof navigator.canShare !== 'function' || navigator.canShare(data))) {
    try {
      await navigator.share(data);
      return 'shared';
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') return 'cancelled';
      /* NotAllowedError / unsupported payload: fall through to the fallback */
    }
  }
  fallback();
  return 'fallback';
}

/** Open a WhatsApp compose window with `text` (the pre-Web-Share way of sharing). */
export function openWhatsApp(text: string): void {
  window.open(`https://wa.me/?text=${encodeURIComponent(text)}`, '_blank', 'noopener,noreferrer');
}
