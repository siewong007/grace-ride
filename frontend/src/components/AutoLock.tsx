import { createContext, ReactNode, useCallback, useContext, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Fingerprint, Lock, LogOut } from 'lucide-react';
import { useAuth } from '../auth';
import { post } from '../api';
import { Avatar, Button } from './ui';
import { passkeySupported, reauthWithPasskey } from '../webauthn';
import { clearOfflineSnapshots } from '../offlineSnapshot';

interface AutoLockCtx {
  isLocked: boolean;
  lock: () => void;
  unlock: () => void;
}

const Ctx = createContext<AutoLockCtx>({
  isLocked: false,
  lock: () => {},
  unlock: () => {},
});

export const useAutoLock = () => useContext(Ctx);

// The lock is kept in localStorage so a reload or a new tab doesn't open it, and tabs
// share one idle clock so a forgotten background tab can't lock the tab in use.
// It is a screen lock only: the session itself stays valid on the server.
const LOCKED_KEY = 'gr_locked_user';
const ACTIVE_KEY = 'gr_last_active';

const readStore = (key: string): string | null => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};

const writeStore = (key: string, value: string | null) => {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    /* storage blocked: the lock still works for this page */
  }
};

/** Forgets a saved lock. Call after a fresh sign-in, which proves who is at the device. */
export const clearAutoLock = () => writeStore(LOCKED_KEY, null);

export function AutoLockProvider({ children }: { children: ReactNode }) {
  const { me, refresh } = useAuth();
  const { t } = useTranslation('common');
  const nav = useNavigate();
  const meId = me ? String(me.id) : null;
  const [isLocked, setIsLocked] = useState(() => meId !== null && readStore(LOCKED_KEY) === meId);
  const meIdRef = useRef(meId);
  meIdRef.current = meId;
  const lastActiveRef = useRef<number>(Date.now());
  const unlockBtnRef = useRef<HTMLButtonElement>(null);
  const [unlocking, setUnlocking] = useState(false);
  const [unlockError, setUnlockError] = useState<string | null>(null);
  const [code, setCode] = useState('');
  const canPasskey = passkeySupported() && (me?.passkey_count ?? 0) > 0;
  const canTotp = Boolean(me?.totp_enabled || (me?.backup_codes_count ?? 0) > 0);
  const canDevUnlock = Boolean(me?.dev_mode && !canPasskey && !canTotp);

  // Inactivity timeout in minutes:
  // 1. If user set auto_lock_mins > 0: user's custom duration
  // 2. If user set auto_lock_mins === -1: user explicitly disabled auto-lock
  // 3. Otherwise: fallback to organization's configured default session_timeout_mins (fallback 15)
  const defaultTimeout = me?.session_timeout_mins ?? 15;
  const userPref = me?.auto_lock_mins ?? (parseInt(localStorage.getItem('gr_autolock_mins') || '0', 10) || 0);
  const timeoutMins = userPref > 0 ? userPref : userPref === -1 ? 0 : defaultTimeout;

  const lock = useCallback(() => {
    if (meIdRef.current) writeStore(LOCKED_KEY, meIdRef.current);
    clearOfflineSnapshots();
    setIsLocked(true);
    void post('/auth/lock').catch(() => {});
  }, []);

  const unlock = useCallback(() => {
    lastActiveRef.current = Date.now();
    writeStore(ACTIVE_KEY, String(lastActiveRef.current));
    writeStore(LOCKED_KEY, null);
    setUnlockError(null);
    setCode('');
    setIsLocked(false);
  }, []);

  // `me` loads after the first render: pick up a local or server-enforced lock.
  useEffect(() => {
    if (meId !== null && (readStore(LOCKED_KEY) === meId || me?.locked)) {
      writeStore(LOCKED_KEY, meId);
      setIsLocked(true);
    }
  }, [meId, me?.locked]);

  useEffect(() => {
    const onServerLocked = () => {
      if (meIdRef.current) writeStore(LOCKED_KEY, meIdRef.current);
      clearOfflineSnapshots();
      setIsLocked(true);
    };
    window.addEventListener('gr:session-locked', onServerLocked);
    return () => window.removeEventListener('gr:session-locked', onServerLocked);
  }, []);

  // Locking or unlocking in another tab applies here too.
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key !== LOCKED_KEY && e.key !== null) return;
      const locked = meIdRef.current !== null && readStore(LOCKED_KEY) === meIdRef.current;
      if (!locked) lastActiveRef.current = Date.now();
      setIsLocked(locked);
    };
    window.addEventListener('storage', onStorage);
    return () => window.removeEventListener('storage', onStorage);
  }, []);

  // A session that has ended no longer needs guarding; the next sign-in starts unlocked.
  useEffect(() => {
    const onUnauth = () => {
      clearAutoLock();
      setIsLocked(false);
    };
    window.addEventListener('gr:unauthorized', onUnauth);
    return () => window.removeEventListener('gr:unauthorized', onUnauth);
  }, []);

  // Activity listeners to reset the inactivity timer
  useEffect(() => {
    if (!me || timeoutMins <= 0 || isLocked) return;

    const onActivity = () => {
      lastActiveRef.current = Date.now();
    };

    const events = ['pointerdown', 'keydown', 'scroll', 'touchstart'];
    events.forEach((ev) => window.addEventListener(ev, onActivity, { passive: true }));

    // Check periodically for idle timeout (every 5 seconds)
    const interval = setInterval(() => {
      // Share this tab's latest activity, and count activity in other tabs.
      const shared = Number(readStore(ACTIVE_KEY)) || 0;
      if (lastActiveRef.current > shared) writeStore(ACTIVE_KEY, String(lastActiveRef.current));
      const elapsed = Date.now() - Math.max(lastActiveRef.current, shared);
      if (elapsed >= timeoutMins * 60 * 1000) {
        lock();
      }
    }, 5000);

    return () => {
      events.forEach((ev) => window.removeEventListener(ev, onActivity));
      clearInterval(interval);
    };
  }, [me, timeoutMins, isLocked, lock]);

  // Focus lock button when locked
  useEffect(() => {
    if (isLocked) {
      unlockBtnRef.current?.focus();
    }
  }, [isLocked]);

  // Unlocking needs a fresh passkey, TOTP/backup code, or dev-mode unlock; never opens on a plain click without auth.
  const handleUnlock = async () => {
    if (unlocking) return;
    setUnlocking(true);
    setUnlockError(null);
    try {
      await reauthWithPasskey();
      unlock();
      await Promise.resolve(refresh()).catch(() => {});
    } catch (e) {
      setUnlockError((e as Error).message || t('unlockFailed'));
    } finally {
      setUnlocking(false);
    }
  };

  const handleCodeUnlock = async () => {
    const cleaned = code.trim();
    if (!cleaned || unlocking) return;
    setUnlocking(true);
    setUnlockError(null);
    try {
      const isSixDigits = /^\d{6}$/.test(cleaned);
      await post('/auth/unlock', isSixDigits ? { totp_code: cleaned } : { backup_code: cleaned });
      unlock();
      await Promise.resolve(refresh()).catch(() => {});
    } catch (e) {
      setUnlockError((e as Error).message || t('unlockFailed'));
    } finally {
      setUnlocking(false);
    }
  };

  const handleDevUnlock = async () => {
    if (unlocking) return;
    setUnlocking(true);
    setUnlockError(null);
    try {
      await post('/auth/unlock', { dev_unlock: true });
      unlock();
      await Promise.resolve(refresh()).catch(() => {});
    } catch (e) {
      setUnlockError((e as Error).message || t('unlockFailed'));
    } finally {
      setUnlocking(false);
    }
  };

  const handleLogout = async () => {
    await post('/auth/logout');
    clearAutoLock();
    setIsLocked(false);
    setUnlockError(null);
    await refresh();
    nav('/login');
  };

  return (
    <Ctx.Provider value={{ isLocked, lock, unlock }}>
      {children}
      {isLocked && me && (
        <div
          className="lock-screen-bg"
          role="dialog"
          aria-modal="true"
          aria-label={t('sessionLocked')}
        >
          <div className="lock-card">
            <div className="lock-icon-wrap" aria-hidden="true">
              <Lock size={28} />
            </div>
            <Avatar name={me.name} fileId={me.photo_file_id} size="lg" />
            <div>
              <h3>{me.name}</h3>
              <p className="muted small">{me.email}</p>
            </div>
            <div className="stack-sm">
              <div className="bold">{t('sessionLocked')}</div>
              <p className="muted small">{t('sessionLockedDesc')}</p>
              <p className="muted small">
                {canPasskey ? t('unlockPasskeyDesc') : canTotp ? t('unlockCodeDesc') : canDevUnlock ? t('unlockPasskeyDesc') : t('unlockNoPasskey')}
              </p>
              {unlockError && (
                <div className="notice error" role="alert">
                  {unlockError}
                </div>
              )}
            </div>
            {canTotp && (
              <div className="stack-sm" style={{ width: '100%' }}>
                <input
                  className="input"
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                  placeholder={t('unlockCodePlaceholder')}
                  aria-label={t('unlockCodePlaceholder')}
                  onKeyDown={(e) => e.key === 'Enter' && void handleCodeUnlock()}
                />
                <button
                  type="button"
                  className="btn secondary"
                  onClick={handleCodeUnlock}
                  disabled={unlocking || !code.trim()}
                >
                  {unlocking ? t('unlocking') : t('unlockWithCode')}
                </button>
              </div>
            )}
            <div className="btn-row" style={{ marginTop: 'var(--space-2)' }}>
              {canPasskey && (
                <button
                  ref={unlockBtnRef}
                  type="button"
                  className="btn primary"
                  onClick={handleUnlock}
                  disabled={unlocking}
                >
                  <Fingerprint size={16} />
                  {unlocking ? t('unlocking') : t('unlockWithPasskey')}
                </button>
              )}
              {canDevUnlock && (
                <button
                  type="button"
                  className="btn primary"
                  onClick={handleDevUnlock}
                  disabled={unlocking}
                >
                  {unlocking ? t('unlocking') : t('unlock')}
                </button>
              )}
              <Button
                variant="ghost"
                onClick={handleLogout}
              >
                <LogOut size={16} />
                {t('logout')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </Ctx.Provider>
  );
}
