import { createContext, CSSProperties, ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { AlertCircle, CheckCircle2, Info, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export type ToastVariant = 'success' | 'error' | 'info';

export interface ToastMessage {
  id: string;
  text: string;
  variant: ToastVariant;
  durationMs: number;
  leaving?: boolean;
}

const TOAST_LEAVE_MS = 160;

function prefersReducedMotion() {
  return typeof window !== 'undefined' && (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false);
}

export interface ToastFn {
  (msg: string, variant?: ToastVariant): void;
  success: (msg: string) => void;
  error: (msg: string) => void;
  info: (msg: string) => void;
}

const noopToast = Object.assign(() => {}, {
  success: () => {},
  error: () => {},
  info: () => {},
}) as ToastFn;

const ToastCtx = createContext<ToastFn>(noopToast);

/**
 * Accessible toast notification provider queueing up to 3 messages with variant icons.
 */
export function ToastProvider({ children }: { children: ReactNode }) {
  const { t: tr } = useTranslation();
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const timersRef = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    const timers = timersRef.current;
    return () => {
      timers.forEach((timerId) => window.clearTimeout(timerId));
      timers.clear();
    };
  }, []);

  const clearTimer = useCallback((key: string) => {
    const timer = timersRef.current.get(key);
    if (timer !== undefined) {
      window.clearTimeout(timer);
      timersRef.current.delete(key);
    }
  }, []);

  const removeImmediate = useCallback(
    (id: string) => {
      clearTimer(id);
      clearTimer(`leave:${id}`);
      setToasts((cur) => cur.filter((t) => t.id !== id));
    },
    [clearTimer],
  );

  const dismiss = useCallback(
    (id: string) => {
      clearTimer(id);
      if (prefersReducedMotion()) {
        removeImmediate(id);
        return;
      }
      setToasts((cur) => {
        const target = cur.find((t) => t.id === id);
        if (!target || target.leaving) return cur;
        return cur.map((t) => (t.id === id ? { ...t, leaving: true } : t));
      });
      if (!timersRef.current.has(`leave:${id}`)) {
        const leaveTimer = window.setTimeout(() => removeImmediate(id), TOAST_LEAVE_MS);
        timersRef.current.set(`leave:${id}`, leaveTimer);
      }
    },
    [clearTimer, removeImmediate],
  );

  const add = useCallback(
    (text: string, variant: ToastVariant = 'info') => {
      const id = Math.random().toString(36).slice(2, 9);
      const durationMs = variant === 'error' ? 6000 : 3000;
      setToasts((cur) => {
        const next = [...cur, { id, text, variant, durationMs }];
        const sliced = next.slice(-3); // queue up to 3
        const keptIds = new Set(sliced.map((t) => t.id));
        for (const item of cur) {
          if (!keptIds.has(item.id)) {
            clearTimer(item.id);
            clearTimer(`leave:${item.id}`);
          }
        }
        return sliced;
      });
      const autoTimer = window.setTimeout(() => {
        dismiss(id);
      }, durationMs);
      timersRef.current.set(id, autoTimer);
    },
    [clearTimer, dismiss],
  );

  const toastFn: ToastFn = useMemo(() => {
    const fn = ((msg: string, variant?: ToastVariant) => add(msg, variant)) as ToastFn;
    fn.success = (msg: string) => add(msg, 'success');
    fn.error = (msg: string) => add(msg, 'error');
    fn.info = (msg: string) => add(msg, 'info');
    return fn;
  }, [add]);

  return (
    <ToastCtx.Provider value={toastFn}>
      {children}
      {toasts.length > 0 && (
        <div className="toast-container" aria-live="polite">
          {toasts.map((t) => (
            <div
              key={t.id}
              className={`toast toast-${t.variant}${t.leaving ? ' is-leaving' : ''}`}
              role={t.variant === 'error' ? 'alert' : 'status'}
              style={{ '--toast-dur': `${t.durationMs}ms` } as CSSProperties}
            >
              <span className="toast-icon" aria-hidden="true">
                {t.variant === 'success' && <CheckCircle2 size={18} />}
                {t.variant === 'error' && <AlertCircle size={18} />}
                {t.variant === 'info' && <Info size={18} />}
              </span>
              <span className="toast-text grow">{t.text}</span>
              <button
                type="button"
                className="toast-close"
                onClick={() => dismiss(t.id)}
                aria-label={tr('close')}
              >
                <X size={16} />
              </button>
              <span className="toast-progress" aria-hidden="true" />
            </div>
          ))}
        </div>
      )}
    </ToastCtx.Provider>
  );
}

export const useToast = () => useContext(ToastCtx);

/** Wrap an async action with busy/error state and an optional success toast. */
export function useAction() {
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = useCallback(
    async (fn: () => Promise<unknown>, success?: string) => {
      setBusy(true);
      setError(null);
      try {
        await fn();
        if (success) toast(success, 'success');
        return true;
      } catch (e) {
        setError((e as Error).message);
        return false;
      } finally {
        setBusy(false);
      }
    },
    [toast],
  );
  return { busy, error, setError, run };
}
