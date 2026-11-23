export type InAppBrowser = 'wechat' | 'whatsapp' | 'line' | 'facebook' | 'instagram' | 'tiktok';

const LABELS: Record<InAppBrowser, string> = {
  wechat: 'WeChat',
  whatsapp: 'WhatsApp',
  line: 'LINE',
  facebook: 'Facebook',
  instagram: 'Instagram',
  tiktok: 'TikTok',
};

/**
 * Detects embedded social/chat in-app browsers whose WebViews often block
 * WebAuthn / passkey prompts or isolate passkey credential storage.
 */
export function detectInAppBrowser(
  ua: string = typeof navigator !== 'undefined' ? navigator.userAgent : '',
): InAppBrowser | null {
  if (!ua) return null;
  if (/MicroMessenger/i.test(ua)) return 'wechat';
  if (/WhatsApp/i.test(ua)) return 'whatsapp';
  if (/\bLine\//i.test(ua)) return 'line';
  if (/FBAN|FBAV|FB_IAB/i.test(ua)) return 'facebook';
  if (/Instagram/i.test(ua)) return 'instagram';
  if (/BytedanceWebview|musical_ly|TikTok/i.test(ua)) return 'tiktok';
  return null;
}

export function inAppBrowserName(kind: InAppBrowser): string {
  return LABELS[kind];
}
