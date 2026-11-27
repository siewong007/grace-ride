import { createContext, ReactNode, useCallback, useContext, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Lock } from 'lucide-react';
import { useAuth } from '../auth';
import { post } from '../api';
import { signOutClient } from '../pushNotifications';
import { Avatar } from './ui';
import { passkeySupported, reauthWithPasskey } from '../webauthn';
import { clearAutoLock, clearOfflineSnapshots } from '../offlineSnapshot';

export { clearAutoLock };

interface AutoLockCtx {
  isLocked: boolean;
  /** False when this device has no way to unlock (no usable passkey, no 2FA): locking would strand the member. */
  canLock: boolean;
  lock: () => void;
  unlock: () => void;
}

const Ctx = createContext<AutoLockCtx>({
  isLocked: false,
  canLock: false,
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

export function AutoLockProvider({ children }: { children: ReactNode }) {
  const { me, refresh, clearLocalAuth } = useAuth();
  const { t } = useTranslation('common');
  const nav = useNavigate();
  const meId = me ? String(me.id) : null;
  const [isLocked, setIsLocked] = useState(() => meId !== null && readStore(LOCKED_KEY) === meId);
  const [unlockedServerLock, setUnlockedServerLock] = useState(false);
  const effectiveLocked = Boolean(
    me && (isLocked || (me.locked && !unlockedServerLock) || (meId !== null && readStore(LOCKED_KEY) === meId)),
  );
  const meIdRef = useRef(meId);
  meIdRef.current = meId;
  const lastActiveRef = useRef<number>(Date.now());
  const unlockBtnRef = useRef<HTMLButtonElement>(null);
  const codeInputRef = useRef<HTMLInputElement>(null);
  const bgRef = useRef<HTMLDivElement>(null);
  const [unlocking, setUnlocking] = useState(false);
  const [unlockError, setUnlockError] = useState<string | null>(null);
  const [shaking, setShaking] = useState(false);
  const [code, setCode] = useState('');
  const [showCode, setShowCode] = useState(false);
  // Set only when the idle timer locked this tab, so the card can say why.
  const [idleLocked, setIdleLocked] = useState(false);
  const canPasskey = passkeySupported() && (me?.passkey_count ?? 0) > 0;
  const canTotp = Boolean(me?.totp_enabled);
  const canDevUnlock = Boolean(me?.dev_mode && (me?.passkey_count ?? 0) === 0 && !me?.totp_enabled);
  // The code form is its own view, reached from the link under the card, unless it is the only way in.
  // With no unlock method the lock screen offers only Log out, and with no passkey the member
  // can't sign back in, so the idle lock would push them into account recovery. Skip it.
  const canLock = canPasskey || canTotp || canDevUnlock;
  const codeView = canTotp && (showCode || (!canPasskey && !canDevUnlock));

  // Inactivity timeout in minutes:
  // 1. If user set auto_lock_mins > 0: user's custom duration
  // 2. If user set auto_lock_mins === -1: user explicitly disabled auto-lock
  // 3. Otherwise: fallback to organization's configured default session_timeout_mins (fallback 15)
  const defaultTimeout = me?.session_timeout_mins ?? 15;
  const userPref = me?.auto_lock_mins ?? (parseInt(readStore('gr_autolock_mins') || '0', 10) || 0);
  const timeoutMins = userPref > 0 ? userPref : userPref === -1 ? 0 : defaultTimeout;

  const lock = useCallback(() => {
    if (meIdRef.current) writeStore(LOCKED_KEY, meIdRef.current);
    clearOfflineSnapshots();
    setIsLocked(true);
    window.dispatchEvent(new Event('gr:session-locked'));
    void post('/auth/lock').catch(() => {});
  }, []);

  const unlock = useCallback(() => {
    lastActiveRef.current = Date.now();
    writeStore(ACTIVE_KEY, String(lastActiveRef.current));
    writeStore(LOCKED_KEY, null);
    setUnlockError(null);
    setCode('');
    setShowCode(false);
    setIdleLocked(false);
    setUnlockedServerLock(true);
    setIsLocked(false);
    window.dispatchEvent(new Event('gr:session-unlocked'));
  }, []);

  useEffect(() => {
    if (!me?.locked && unlockedServerLock) {
      setUnlockedServerLock(false);
    }
  }, [me?.locked, unlockedServerLock]);

  useEffect(() => {
    const el = bgRef.current;
    if (!el) return;
    el.inert = effectiveLocked;
  }, [effectiveLocked]);

  // `me` loads after the first render: pick up a local or server-enforced lock.
  useEffect(() => {
    if (meId !== null && (readStore(LOCKED_KEY) === meId || (me?.locked && !unlockedServerLock))) {
      writeStore(LOCKED_KEY, meId);
      clearOfflineSnapshots();
      setIsLocked(true);
      window.dispatchEvent(new Event('gr:session-locked'));
    }
  }, [meId, me?.locked, unlockedServerLock]);

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
      window.dispatchEvent(new Event(locked ? 'gr:session-locked' : 'gr:session-unlocked'));
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
    if (!me || timeoutMins <= 0 || effectiveLocked || !canLock) return;

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
        setIdleLocked(true);
        lock();
      }
    }, 5000);

    return () => {
      events.forEach((ev) => window.removeEventListener(ev, onActivity));
      clearInterval(interval);
    };
  }, [me, timeoutMins, effectiveLocked, canLock, lock]);

  // Focus the code field in the code view, otherwise the unlock button.
  useEffect(() => {
    if (!effectiveLocked) return;
    if (codeView) codeInputRef.current?.focus();
    else unlockBtnRef.current?.focus();
  }, [effectiveLocked, codeView]);

  // Unlocking needs a fresh passkey, TOTP/backup code, or dev-mode unlock; never opens on a plain click without auth.
  const triggerShake = useCallback(() => {
    if (
      typeof window !== 'undefined' &&
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    ) {
      return;
    }
    setShaking(false);
    window.requestAnimationFrame(() => setShaking(true));
  }, []);

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
      triggerShake();
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
      triggerShake();
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
      triggerShake();
    } finally {
      setUnlocking(false);
    }
  };

  const handleLogout = async () => {
    await signOutClient();
    clearAutoLock();
    clearLocalAuth?.();
    setIsLocked(false);
    setUnlockError(null);
    nav('/login');
  };

  return (
    <Ctx.Provider value={{ isLocked: effectiveLocked, canLock, lock, unlock }}>
      <div ref={bgRef} data-autolock-state={effectiveLocked ? 'locked' : 'unlocked'}>
        {!effectiveLocked && children}
      </div>
      {effectiveLocked && me && (
        <div
          className="lock-screen-bg"
          role="dialog"
          aria-modal="true"
          aria-label={t('sessionLocked')}
        >
          <div className="lock-stack">
            <div
              className={`lock-card${shaking ? ' is-shaking' : ''}`}
              onAnimationEnd={() => setShaking(false)}
            >
              <div className="lock-avatar">
                <Avatar name={me.name} fileId={me.photo_file_id} color={me.avatar_color} icon={me.avatar_icon} size="lg" />
                <span className="lock-badge" aria-hidden="true">
                  <Lock size={12} />
                </span>
              </div>
              <div>
                <h3>{me.name}</h3>
                <p className="muted small">{me.email}</p>
              </div>
              <p className="muted small">
                {idleLocked ? t('sessionLockedDesc', { count: timeoutMins }) : t('sessionLocked')}
              </p>
              {unlockError && (
                <div className="notice error" role="alert">
                  {unlockError}
                </div>
              )}
              {!canPasskey && !canTotp && !canDevUnlock && (
                <p className="muted small">{t('unlockNoPasskey')}</p>
              )}
              {codeView && (
                <input
                  className="input"
                  ref={codeInputRef}
                  type={code.trim() !== '' && !/^\d+$/.test(code.trim()) ? 'password' : 'text'}
                  inputMode="text"
                  autoComplete="off"
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                  placeholder={t('unlockCodePlaceholder')}
                  aria-label={t('unlockCodePlaceholder')}
                  onKeyDown={(e) => e.key === 'Enter' && void handleCodeUnlock()}
                />
              )}
              <div className="lock-links">
                {codeView ? (
                  <button
                    type="button"
                    className="link-btn"
                    onClick={handleCodeUnlock}
                    disabled={unlocking || !code.trim()}
                  >
                    {unlocking ? t('unlocking') : t('unlock')}
                  </button>
                ) : (
                  <>
                    {canPasskey && (
                      <button
                        ref={unlockBtnRef}
                        type="button"
                        className="link-btn"
                        onClick={handleUnlock}
                        disabled={unlocking}
                      >
                        {unlocking ? t('unlocking') : t('unlockWithPasskey')}
                      </button>
                    )}
                    {canDevUnlock && (
                      <button type="button" className="link-btn" onClick={handleDevUnlock} disabled={unlocking}>
                        {unlocking ? t('unlocking') : t('unlock')}
                      </button>
                    )}
                  </>
                )}
                <button type="button" className="link-btn muted-link" onClick={handleLogout}>
                  {t('logout')}
                </button>
              </div>
            </div>
            {canTotp && (canPasskey || canDevUnlock) && (
              <button
                type="button"
                className="link-btn lock-switch"
                onClick={() => {
                  setShowCode((v) => !v);
                  setCode('');
                  setUnlockError(null);
                }}
              >
                {codeView ? t('unlockUsePasskey') : t('unlockWithCode')}
              </button>
            )}
          </div>
        </div>
      )}
    </Ctx.Provider>
  );
}
