import { createContext, ReactNode, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { api, subscribe } from './api';
import type { Act, Can, PageKey } from './perms';

export interface Me {
  id: number;
  name: string;
  email: string;
  phone: string;
  photo_file_id: number | null;
  /** Role keys. Built-in ones are named by the app, custom ones by `role_names`. */
  roles: string[];
  role_names: Record<string, string>;
  /** What this session may do, as '<page>.<action>'. Staff permissions are absent until `mfa_ok`. */
  permissions: string[];
  status: 'active' | 'pending' | 'disabled';
  terms_accepted: boolean;
  mfa_ok: boolean;
  totp_enabled: boolean;
  has_password: boolean;
  passkey_count: number;
  badges_opt_out: boolean;
  readiness_prompt_mins?: number;
  gate_wait_mins?: number;
  privileged: boolean;
  unread: number;
  groups: { id: number; name: string }[];
  dev_mode: boolean;
}

interface AuthCtx {
  me: Me | null;
  loading: boolean;
  refresh: () => Promise<void>;
  /** May the member do `act` (default: view) on `page`? */
  can: Can;
}

const Ctx = createContext<AuthCtx>(null as unknown as AuthCtx);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [me, setMe] = useState<Me | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      setMe(await api<Me>('/auth/me'));
    } catch {
      setMe(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const onUnauth = () => setMe(null);
    window.addEventListener('gr:unauthorized', onUnauth);
    // Keep the unread badge fresh.
    const t = setInterval(() => document.visibilityState === 'visible' && refresh(), 30000);
    return () => {
      window.removeEventListener('gr:unauthorized', onUnauth);
      clearInterval(t);
    };
  }, [refresh]);

  // Keep the unread badge fresh. Re-subscribe only when the signed-in user changes.
  const meId = me?.id;
  useEffect(() => {
    if (meId == null) return;
    return subscribe((kind) => {
      if (kind === 'notify' || kind === 'resume') refresh();
    });
  }, [meId, refresh]);

  const granted = useMemo(() => new Set(me?.permissions ?? []), [me?.permissions]);
  const can = useCallback((page: PageKey, act: Act = 'view') => granted.has(`${page}.${act}`), [granted]);

  return <Ctx.Provider value={{ me, loading, refresh, can }}>{children}</Ctx.Provider>;
}

export const useAuth = () => useContext(Ctx);
