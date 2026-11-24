import { useCallback, useEffect, useState } from 'react';

export const TURNSTILE_SCRIPT_URL =
  'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit';

export type TurnstileAction =
  | 'passkey_login'
  | 'recovery_request'
  | 'invite_accept'
  | 'visitor_claim';

export interface TurnstileConfig {
  enabled: boolean;
  siteKey: string | null;
}

export interface TurnstileRenderOptions {
  sitekey: string;
  action?: TurnstileAction;
  theme?: 'light' | 'dark' | 'auto';
  language?: string;
  size?: 'normal' | 'compact' | 'flexible';
  'refresh-expired'?: 'auto' | 'manual' | 'never';
  callback?: (token: string) => void;
  'expired-callback'?: () => void;
  'error-callback'?: (code?: string) => void;
  'timeout-callback'?: () => void;
}

export interface TurnstileApi {
  render: (container: HTMLElement | string, options: TurnstileRenderOptions) => string;
  reset: (widgetId?: string) => void;
  remove: (widgetId?: string) => void;
  getResponse: (widgetId?: string) => string | undefined;
}

declare global {
  interface Window {
    turnstile?: TurnstileApi;
  }
}

const DISABLED: TurnstileConfig = { enabled: false, siteKey: null };
const SCRIPT_TIMEOUT_MS = 15_000;

let cachedConfig: TurnstileConfig | null = null;
let configPromise: Promise<TurnstileConfig> | null = null;
let scriptPromise: Promise<TurnstileApi> | null = null;

/** Clear cached config and script promise (for unit tests). */
export function resetTurnstileForTests(): void {
  cachedConfig = null;
  configPromise = null;
  scriptPromise = null;
}

/**
 * Fetch runtime Turnstile availability and public site key.
 * Uses `fetch` directly so test suites that partially mock `./api` keep working.
 */
export function getTurnstileConfig(): Promise<TurnstileConfig> {
  if (cachedConfig) return Promise.resolve(cachedConfig);
  if (configPromise) return configPromise;
  if (typeof fetch !== 'function') return Promise.resolve(DISABLED);

  configPromise = (async () => {
    try {
      const res = await fetch('/api/auth/turnstile', {
        credentials: 'same-origin',
        headers: { Accept: 'application/json' },
      });
      if (!res.ok) {
        configPromise = null;
        return DISABLED;
      }
      const data = (await res.json()) as { enabled?: unknown; site_key?: unknown } | null;
      const siteKey =
        data?.enabled === true && typeof data.site_key === 'string' && data.site_key.trim()
          ? data.site_key.trim()
          : null;
      const cfg: TurnstileConfig = siteKey ? { enabled: true, siteKey } : DISABLED;
      cachedConfig = cfg;
      return cfg;
    } catch {
      configPromise = null;
      return DISABLED;
    }
  })();

  return configPromise;
}

/**
 * Load Cloudflare Turnstile's explicit-render API script once.
 * Clears its in-flight state on error or timeout so retries can recover.
 */
export function loadTurnstileScript(): Promise<TurnstileApi> {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return Promise.reject(new Error('Turnstile requires a browser environment'));
  }
  if (window.turnstile) {
    return Promise.resolve(window.turnstile);
  }
  if (scriptPromise) {
    return scriptPromise;
  }

  scriptPromise = new Promise<TurnstileApi>((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>(
      `script[data-gr-turnstile="1"], script[src="${TURNSTILE_SCRIPT_URL}"]`,
    );
    const script = existing ?? document.createElement('script');
    let settled = false;

    const cleanup = () => {
      window.clearTimeout(timer);
      script.removeEventListener('load', onLoad);
      script.removeEventListener('error', onError);
    };

    const fail = (err: Error) => {
      if (settled) return;
      settled = true;
      cleanup();
      scriptPromise = null;
      if (!existing && script.parentNode) {
        script.parentNode.removeChild(script);
      } else if (existing && existing.dataset.grTurnstile === '1' && existing.parentNode) {
        existing.parentNode.removeChild(existing);
      }
      reject(err);
    };

    const onLoad = () => {
      if (settled) return;
      if (window.turnstile) {
        settled = true;
        cleanup();
        resolve(window.turnstile);
      } else {
        fail(new Error('Turnstile API unavailable after script load'));
      }
    };

    const onError = () => {
      fail(new Error('Failed to load Turnstile script'));
    };

    const timer = window.setTimeout(() => {
      fail(new Error('Timed out loading Turnstile script'));
    }, SCRIPT_TIMEOUT_MS);

    script.addEventListener('load', onLoad);
    script.addEventListener('error', onError);

    if (!existing) {
      script.src = TURNSTILE_SCRIPT_URL;
      script.async = true;
      script.defer = true;
      script.dataset.grTurnstile = '1';
      document.head.appendChild(script);
    } else if (window.turnstile) {
      onLoad();
    }
  });

  return scriptPromise;
}

export interface TurnstileState {
  enabled: boolean;
  siteKey: string | null;
  token: string | null;
  setToken: (token: string | null) => void;
  resetKey: number;
  reset: () => void;
}

/**
 * Subscribe to runtime Turnstile config and manage a form's token + reset lifecycle.
 */
export function useTurnstile(): TurnstileState {
  const [cfg, setCfg] = useState<TurnstileConfig>(() => cachedConfig ?? DISABLED);
  const [token, setToken] = useState<string | null>(null);
  const [resetKey, setResetKey] = useState(0);

  useEffect(() => {
    let mounted = true;
    void getTurnstileConfig().then((next) => {
      if (!mounted) return;
      setCfg((prev) =>
        prev.enabled === next.enabled && prev.siteKey === next.siteKey ? prev : next,
      );
    });
    return () => {
      mounted = false;
    };
  }, []);

  const reset = useCallback(() => {
    setToken(null);
    setResetKey((k) => k + 1);
  }, []);

  return {
    enabled: cfg.enabled,
    siteKey: cfg.siteKey,
    token,
    setToken,
    resetKey,
    reset,
  };
}
