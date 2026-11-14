import { createContext, ReactNode, useCallback, useContext, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Lock, LogOut } from 'lucide-react';
import { useAuth } from '../auth';
import { post } from '../api';
import { Avatar, Button } from './ui';

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

export function AutoLockProvider({ children }: { children: ReactNode }) {
  const { me, refresh } = useAuth();
  const { t } = useTranslation('common');
  const nav = useNavigate();
  const [isLocked, setIsLocked] = useState(false);
  const lastActiveRef = useRef<number>(Date.now());
  const unlockBtnRef = useRef<HTMLButtonElement>(null);

  // User timeout preference in minutes: prioritize user profile, fallback to localStorage
  const timeoutMins = me?.auto_lock_mins ?? (parseInt(localStorage.getItem('gr_autolock_mins') || '0', 10) || 0);

  const lock = useCallback(() => {
    setIsLocked(true);
  }, []);

  const unlock = useCallback(() => {
    lastActiveRef.current = Date.now();
    setIsLocked(false);
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
      const elapsed = Date.now() - lastActiveRef.current;
      if (elapsed >= timeoutMins * 60 * 1000) {
        setIsLocked(true);
      }
    }, 5000);

    return () => {
      events.forEach((ev) => window.removeEventListener(ev, onActivity));
      clearInterval(interval);
    };
  }, [me, timeoutMins, isLocked]);

  // Focus lock button when locked
  useEffect(() => {
    if (isLocked) {
      unlockBtnRef.current?.focus();
    }
  }, [isLocked]);

  const handleLogout = async () => {
    await post('/auth/logout');
    setIsLocked(false);
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
            </div>
            <div className="btn-row" style={{ marginTop: 'var(--space-2)' }}>
              <button
                ref={unlockBtnRef}
                type="button"
                className="btn primary"
                onClick={unlock}
              >
                <Lock size={16} />
                {t('unlock')}
              </button>
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
