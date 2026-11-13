import { createContext, ReactNode, useCallback, useContext, useEffect, useState } from 'react';
import { api, subscribe } from './api';

export type Role = 'admin' | 'transport' | 'finance' | 'leader' | 'driver';

export interface Me {
  id: number;
  name: string;
  email: string;
  phone: string;
  photo_file_id: number | null;
  roles: Role[];
  status: 'active' | 'pending' | 'disabled';
  terms_accepted: boolean;
  mfa_ok: boolean;
  totp_enabled: boolean;
  has_password: boolean;
  passkey_count: number;
  badges_opt_out: boolean;
  privileged: boolean;
  unread: number;
  groups: { id: number; name: string }[];
  dev_mode: boolean;
}

interface AuthCtx {
  me: Me | null;
  loading: boolean;
  refresh: () => Promise<void>;
  has: (r: Role) => boolean;
  isStaff: boolean;
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

  // Mirrors the server: admins also act as transport lead and event lead (not finance / driver).
  const has = (r: Role) =>
    !!me && (me.roles.includes(r) || ((r === 'transport' || r === 'leader') && me.roles.includes('admin')));
  const isStaff = !!me && me.mfa_ok && (has('transport') || has('admin'));

  return <Ctx.Provider value={{ me, loading, refresh, has, isStaff }}>{children}</Ctx.Provider>;
}

export const useAuth = () => useContext(Ctx);
