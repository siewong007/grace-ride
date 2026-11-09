import { useCallback, useEffect, useRef, useState } from 'react';
import i18n from './i18n';

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
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

export async function api<T = any>(path: string, opts: { method?: string; body?: unknown } = {}): Promise<T> {
  const res = await fetch(`/api${path}`, {
    method: opts.method ?? (opts.body !== undefined ? 'POST' : 'GET'),
    headers: opts.body !== undefined ? { 'content-type': 'application/json' } : undefined,
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
    if (res.status === 401 && !path.startsWith('/auth/')) window.dispatchEvent(new Event('gr:unauthorized'));
    throw new ApiError(res.status, apiMessage(data, 'network'));
  }
  return data as T;
}

export const post = <T = any>(path: string, body: unknown = {}) => api<T>(path, { method: 'POST', body });
export const put = <T = any>(path: string, body: unknown) => api<T>(path, { method: 'PUT', body });
export const del = <T = any>(path: string) => api<T>(path, { method: 'DELETE' });

export async function uploadFile(file: File): Promise<{ id: number; mime: string; name: string }> {
  const fd = new FormData();
  fd.append('file', file);
  const res = await fetch('/api/files', { method: 'POST', body: fd, credentials: 'same-origin' });
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(res.status, apiMessage(data, 'uploadFailed'));
  return data;
}

/** Fetch JSON for a path; re-fetch on `reload()` and optionally poll every `pollMs`. */
export function useApi<T>(path: string | null, pollMs?: number) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const pathRef = useRef(path);
  pathRef.current = path;

  const reload = useCallback(async () => {
    if (!pathRef.current) return;
    try {
      const d = await api<T>(pathRef.current);
      setData(d);
      setError(null);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    setData(null);
    reload();
    if (!pollMs) return;
    const t = setInterval(() => {
      if (document.visibilityState === 'visible') reload();
    }, pollMs);
    return () => clearInterval(t);
  }, [path, pollMs, reload]);

  return { data, error, loading, reload, setData };
}
