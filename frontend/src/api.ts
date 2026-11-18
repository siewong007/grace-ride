import { useCallback, useEffect, useRef, useState } from 'react';
import i18n from './i18n';
import { clearOfflineSnapshots } from './offlineSnapshot';

const ERROR_GROUPS = ['common', 'admin', 'auth', 'events', 'rides', 'settlements', 'community', 'files'] as const

type ErrorBody = { error?: string; code?: string; params?: Record<string, unknown> } | null

/** Prefer the active UI language for a known error code. Fall back to the server message. */
export function apiMessage(data: ErrorBody, fallback: 'network' | 'uploadFailed'): string {
  const code = data?.code
  if (code) {
    for (const group of ERROR_GROUPS) {
      const key = `${group}.${code}`
      if (i18n.exists(key, { ns: 'errors' })) {
        return String(i18n.t(key, { ns: 'errors', ...(data?.params ?? {}) }))
      }
    }
  }
  if (data?.error) return data.error
  return String(i18n.t(fallback, { ns: 'errors' }))
}

export class ApiError extends Error {
  status: number;
  code?: string;
  constructor(status: number, message: string, code?: string) {
    super(message);
    this.status = status;
    this.code = code;
  }
}

export async function api<T = unknown>(path: string, opts: { method?: string; body?: unknown } = {}): Promise<T> {
  const acceptLanguage = i18n.language || 'zh';
  const res = await fetch(`/api${path}`, {
    method: opts.method ?? (opts.body !== undefined ? 'POST' : 'GET'),
    headers:
      opts.body !== undefined
        ? { 'Accept-Language': acceptLanguage, 'content-type': 'application/json' }
        : { 'Accept-Language': acceptLanguage },
    body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
    credentials: 'same-origin',
  });
  const text = await res.text();
  let data: ErrorBody = null;
  if (text) {
    try {
      data = JSON.parse(text);
    } catch {
      data = { error: text.slice(0, 300) };
    }
  }
  if (!res.ok) {
    // Public visitor and caregiver pages have no member session; their 401 must not send anyone to sign-in.
    if (
      res.status === 401 &&
      !path.startsWith('/auth/') &&
      !path.startsWith('/visit/') &&
      !path.startsWith('/care/')
    ) {
      window.dispatchEvent(new Event('gr:unauthorized'));
    }
    if ((res.status === 423 || data?.code === 'SESSION_LOCKED') && typeof window !== 'undefined') {
      window.dispatchEvent(new Event('gr:session-locked'));
    }
    if (data?.code === 'RIDE_STATUS_CHANGED' && typeof window !== 'undefined') {
      window.dispatchEvent(new Event('gr:ride-stale'));
    }
    throw new ApiError(res.status, apiMessage(data, 'network'), data?.code);
  }
  if (path.startsWith('/auth/logout')) {
    clearOfflineSnapshots();
    try {
      localStorage.removeItem('gr_autolock_mins');
      localStorage.removeItem('gr_locked_user');
    } catch {
      /* ignore storage errors */
    }
  }
  return data as T;
}

export const post = <T = unknown>(path: string, body: unknown = {}) => api<T>(path, { method: 'POST', body });
export const put = <T = unknown>(path: string, body: unknown) => api<T>(path, { method: 'PUT', body });
export const del = <T = unknown>(path: string) => api<T>(path, { method: 'DELETE' });

export async function uploadFile(file: File): Promise<{ id: number; mime: string; name: string }> {
  const fd = new FormData();
  fd.append('file', file);
  const res = await fetch('/api/files', {
    method: 'POST',
    headers: { 'Accept-Language': i18n.language || 'zh' },
    body: fd,
    credentials: 'same-origin',
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(res.status, apiMessage(data, 'uploadFailed'));
  return data;
}

/** One shared server-sent-events connection per tab. Polling stays as the
 *  fallback; events only make changes appear without waiting for the next tick.
 *  While SSE is open and healthy, fallback polling relaxes 4x. */
const listeners = new Set<(kind: string) => void>();
let stream: EventSource | null = null;
let streamOpen = false;
let sessionLocked = (() => {
  try {
    return typeof localStorage !== 'undefined' && Boolean(localStorage.getItem('gr_locked_user'));
  } catch {
    return false;
  }
})();

export function isStreamConnected(): boolean {
  return streamOpen;
}

function openStream() {
  if (
    stream ||
    sessionLocked ||
    typeof EventSource === 'undefined' ||
    document.visibilityState === 'hidden'
  ) {
    return;
  }
  const es = new EventSource('/api/stream');
  es.onopen = () => {
    streamOpen = true;
  };
  es.onerror = () => {
    streamOpen = false;
  };
  es.onmessage = (e) => {
    streamOpen = true;
    listeners.forEach((fn) => fn(e.data));
  };
  stream = es;
}

function closeStream() {
  stream?.close();
  stream = null;
  streamOpen = false;
}

export function subscribe(fn: (kind: string) => void) {
  listeners.add(fn);
  openStream();
  return () => {
    listeners.delete(fn);
    if (!listeners.size) closeStream();
  };
}

if (typeof window !== 'undefined') {
  // EventSource retries a rejected handshake forever; a 401 or lock closes it cleanly.
  window.addEventListener('gr:unauthorized', () => {
    sessionLocked = false;
    closeStream();
  });
  window.addEventListener('gr:session-locked', () => {
    sessionLocked = true;
    closeStream();
  });
  window.addEventListener('gr:session-unlocked', () => {
    sessionLocked = false;
    if (listeners.size && document.visibilityState !== 'hidden') {
      openStream();
      listeners.forEach((fn) => fn('unlocked'));
    }
  });
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      closeStream();
    } else if (listeners.size && !sessionLocked) {
      openStream();
      listeners.forEach((fn) => fn('resume'));
    }
  });
}

/** Fetch JSON for a path; re-fetch on `reload()` and optionally poll every `pollMs`.
 *  Responses for requests superseded by a newer path or reload are always ignored,
 *  so a slow reply for a previous path can never replace the current path's data. */
export function useApi<T>(path: string | null, pollMs?: number) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(Boolean(path));
  const [refreshing, setRefreshing] = useState(false);
  const pathRef = useRef(path);
  const prevBaseRef = useRef<string | null>(path ? path.split('?')[0] : null);
  const hasDataRef = useRef(false);
  const errorRef = useRef<string | null>(error);
  const requestId = useRef(0);
  pathRef.current = path;
  hasDataRef.current = data !== null;
  errorRef.current = error;

  const reload = useCallback(async () => {
    const currentRequest = ++requestId.current;
    const requestPath = pathRef.current;
    const isCurrent = () =>
      requestId.current === currentRequest && pathRef.current === requestPath;
    if (!requestPath) {
      setLoading(false);
      setRefreshing(false);
      return;
    }
    if (hasDataRef.current) setRefreshing(true);
    try {
      const d = await api<T>(requestPath);
      if (isCurrent()) {
        setData(d);
        setError(null);
      }
    } catch (e) {
      if (isCurrent()) setError((e as Error).message);
    } finally {
      if (isCurrent()) {
        setLoading(false);
        setRefreshing(false);
      }
    }
  }, []);

  useEffect(() => {
    const requests = requestId;
    if (!path) {
      requests.current++;
      prevBaseRef.current = null;
      setLoading(false);
      setRefreshing(false);
      setData(null);
      setError(null);
      return;
    }
    const base = path.split('?')[0];
    const sameEndpoint = prevBaseRef.current === base && hasDataRef.current;
    prevBaseRef.current = base;
    if (!sameEndpoint) {
      setLoading(true);
      setData(null);
    }
    reload();
    const onUnlocked = () => {
      if (errorRef.current !== null || !hasDataRef.current) {
        void reload();
      }
    };
    window.addEventListener('gr:session-unlocked', onUnlocked);
    if (!pollMs) {
      return () => {
        window.removeEventListener('gr:session-unlocked', onUnlocked);
        requests.current++;
      };
    }
    let skipped = 0;
    const t = setInterval(() => {
      if (document.visibilityState !== 'visible' || sessionLocked) return;
      if (streamOpen && ++skipped < 4) return;
      skipped = 0;
      reload();
    }, pollMs);
    const unsub = subscribe(reload);
    return () => {
      window.removeEventListener('gr:session-unlocked', onUnlocked);
      clearInterval(t);
      unsub();
      requests.current++;
    };
  }, [path, pollMs, reload]);

  return { data, error, loading, refreshing, reload, setData };
}
