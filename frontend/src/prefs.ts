// Per-device display preferences (large text mode / theme). Stored in localStorage only;
// every access is guarded because storage can be unavailable (private mode, blocked).

export type TextSize = 'normal' | 'large' | 'xlarge';
export type Theme = 'system' | 'light' | 'dark';

const KEY = 'gr-prefs';

export interface Prefs {
  textSize: TextSize;
  theme: Theme;
}

const DEFAULTS: Prefs = { textSize: 'normal', theme: 'system' };

export function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return DEFAULTS;
    const p = JSON.parse(raw) as Partial<Prefs>;
    return {
      textSize: p.textSize === 'large' || p.textSize === 'xlarge' ? p.textSize : 'normal',
      theme: p.theme === 'light' || p.theme === 'dark' ? p.theme : 'system',
    };
  } catch {
    return DEFAULTS;
  }
}

export function applyPrefs(p: Prefs) {
  const root = document.documentElement;
  root.dataset.text = p.textSize;
  if (p.theme === 'system') delete root.dataset.theme;
  else root.dataset.theme = p.theme;
  const meta = document.querySelector('meta[name="theme-color"]');
  const dark = p.theme === 'dark' || (p.theme === 'system' && window.matchMedia?.('(prefers-color-scheme: dark)').matches);
  meta?.setAttribute('content', dark ? '#121412' : '#1E6B5C');
}

export function savePrefs(p: Prefs) {
  try {
    localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    /* preference simply won't persist */
  }
  applyPrefs(p);
}

// Apply before first paint (imported from main.tsx).
applyPrefs(loadPrefs());
