import { createContext, ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { api, subscribe } from './api';
import { clearOfflineSnapshots } from './offlineSnapshot';
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
  /** May the member do `act` (default: view) on `page`? */
  can: Can;
}

const Ctx = createContext<AuthCtx>(null as unknown as AuthCtx);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [me, setMe] = useState<Me | null>(null);
  const [loading, setLoading] = useState(true);
  const prevUnreadRef = useRef<number | null>(null);
  const lastNotifiedRef = useRef<number>(0);

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
    const onUnauth = () => {
      prevUnreadRef.current = null;
      clearOfflineSnapshots();
      try {
        localStorage.removeItem('gr_autolock_mins');
      } catch {
        /* ignore storage errors */
      }
      setMe(null);
    };
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
  const pushOptIn = me?.push_opt_in ?? true;
  const notifyHiddenTab = useCallback(() => {
    if (
      !pushOptIn ||
      typeof document === 'undefined' ||
      document.visibilityState !== 'hidden' ||
      typeof Notification === 'undefined' ||
      Notification.permission !== 'granted'
    ) {
      return;
    }
    const now = Date.now();
    if (now - lastNotifiedRef.current <= 2000) return;
    lastNotifiedRef.current = now;
    const msg = {
      type: 'SHOW_NOTIFICATION',
      title: 'Grace Ride',
      body: '',
      url: '/notifications',
    };
    if (navigator.serviceWorker?.controller) {
      const ctrl = navigator.serviceWorker.controller;
      const postFallback = () => ctrl.postMessage(msg);
      if (navigator.serviceWorker.ready) {
        navigator.serviceWorker.ready
          .then((reg) => reg.pushManager?.getSubscription?.() ?? null)
          .then((sub) => {
            if (!sub) postFallback();
          })
          .catch(postFallback);
      } else {
        postFallback();
      }
    } else {
      try {
        new Notification('Grace Ride', { data: { url: '/notifications' } } as NotificationOptions);
      } catch {
        // Browser requires ServiceWorkerRegistration.showNotification (Android Chrome)
      }
    }
  }, [pushOptIn]);

  useEffect(() => {
    const unread = me?.unread;
    if (unread === undefined || me == null) {
      prevUnreadRef.current = null;
      return;
    }
    if (prevUnreadRef.current !== null && unread > prevUnreadRef.current) {
      notifyHiddenTab();
    }
    prevUnreadRef.current = unread;
  }, [me, notifyHiddenTab]);

  useEffect(() => {
    if (meId == null) return;
    return subscribe((kind) => {
      if (kind === 'notify' || kind === 'resume') refresh();
      if (kind === 'notify') notifyHiddenTab();
    });
  }, [meId, notifyHiddenTab, refresh]);

  const granted = useMemo(() => new Set(me?.permissions ?? []), [me?.permissions]);
  const can = useCallback((page: PageKey, act: Act = 'view') => granted.has(`${page}.${act}`), [granted]);

  return <Ctx.Provider value={{ me, loading, refresh, can }}>{children}</Ctx.Provider>;
}

export const useAuth = () => useContext(Ctx);
