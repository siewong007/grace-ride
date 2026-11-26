import { createContext, ReactNode, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { api, ApiError, clearClientSessionStorage, isSessionLocked, isStreamConnected, post, subscribe } from './api';
import { clearAutoLock } from './offlineSnapshot';
import { resetPushState, syncAppBadge } from './pushNotifications';
import type { Act, Can, PageKey } from './perms';

export interface Me {
  id: number;
  name: string;
  email: string;
  phone: string;
  photo_file_id: number | null;
  /** A key from `AVATAR_COLORS`, or null for the default background. */
  avatar_color?: string | null;
  /** A key from `AVATAR_ICONS`, or null to show the initial. */
  avatar_icon?: string | null;
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
  google_verified?: boolean;
  google_email?: string | null;
  badges_opt_out: boolean;
  readiness_prompt_mins?: number;
  gate_wait_mins?: number;
  auto_lock_mins?: number;
  session_timeout_mins?: number;
  backup_codes_count?: number;
  locked?: boolean;
  telegram_chat_id?: string;
  whatsapp_opt_in?: boolean;
  push_opt_in?: boolean;
  /** The member hid the Getting started checklist on Home. */
  onboarding_dismissed?: boolean;
  caregiver_user_id?: number | null;
  privileged: boolean;
  unread: number;
  groups: { id: number; name: string }[];
  dev_mode: boolean;
}

interface AuthCtx {
  me: Me | null;
  loading: boolean;
  refresh: () => Promise<void>;
  clearLocalAuth: () => void;
  /** May the member do `act` (default: view) on `page`? */
  can: Can;
}

const Ctx = createContext<AuthCtx>(null as unknown as AuthCtx);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [me, setMe] = useState<Me | null>(null);
  const [loading, setLoading] = useState(true);

  const clearLocalAuth = useCallback(() => {
    setMe(null);
  }, []);

  const refresh = useCallback(async () => {
    try {
      if (typeof localStorage !== 'undefined' && localStorage.getItem('gr_pending_logout') === '1') {
        try {
          await post('/auth/logout', {});
          localStorage.removeItem('gr_pending_logout');
        } catch (logoutErr) {
          if (logoutErr instanceof ApiError && logoutErr.status === 401) {
            localStorage.removeItem('gr_pending_logout');
          } else {
            setMe(null);
            return;
          }
        }
      }
      setMe(await api<Me>('/auth/me'));
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        syncAppBadge(0);
        resetPushState();
        clearAutoLock();
        clearClientSessionStorage();
        setMe(null);
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const onUnauth = () => {
      syncAppBadge(0);
      resetPushState();
      clearClientSessionStorage();
      setMe(null);
    };
    window.addEventListener('gr:unauthorized', onUnauth);
    return () => {
      window.removeEventListener('gr:unauthorized', onUnauth);
    };
  }, [refresh]);

  // Keep the unread badge fresh. Re-subscribe only when the signed-in user changes.
  const meId = me?.id;

  useEffect(() => {
    syncAppBadge(me?.unread ?? 0);
  }, [me?.unread]);

  useEffect(() => {
    if (meId == null) return;
    let skipped = 0;
    const t = setInterval(() => {
      if (document.visibilityState !== 'visible' || isSessionLocked()) return;
      if (isStreamConnected() && ++skipped < 4) return;
      skipped = 0;
      void refresh();
    }, 30_000);
    const unsub = subscribe((kind) => {
      if (kind.startsWith('unread:')) {
        const nextUnread = Number.parseInt(kind.slice(7), 10);
        if (Number.isFinite(nextUnread) && nextUnread >= 0) {
          setMe((prev) => (prev ? { ...prev, unread: nextUnread } : prev));
        }
        return;
      }
      if (kind === 'notify' || kind === 'resume' || kind === 'unlocked') {
        void refresh();
      }
    });
    return () => {
      clearInterval(t);
      unsub();
    };
  }, [meId, refresh]);

  const granted = useMemo(() => new Set(me?.permissions ?? []), [me?.permissions]);
  const can = useCallback((page: PageKey, act: Act = 'view') => granted.has(`${page}.${act}`), [granted]);

  return <Ctx.Provider value={{ me, loading, refresh, clearLocalAuth, can }}>{children}</Ctx.Provider>;
}

export const useAuth = () => useContext(Ctx);
