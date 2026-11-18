import { createContext, ReactNode, useCallback, useContext, useMemo, useState } from 'react';
import { AlertCircle, CheckCircle2, Info, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export type ToastVariant = 'success' | 'error' | 'info';

export interface ToastMessage {
  id: string;
  text: string;
  variant: ToastVariant;
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

  const remove = useCallback((id: string) => {
    setToasts((cur) => cur.filter((t) => t.id !== id));
  }, []);

  const add = useCallback(
    (text: string, variant: ToastVariant = 'info') => {
      const id = Math.random().toString(36).slice(2, 9);
      setToasts((cur) => {
        const next = [...cur, { id, text, variant }];
        return next.slice(-3); // queue up to 3
      });
      const dur = variant === 'error' ? 6000 : 3000;
      setTimeout(() => {
        remove(id);
      }, dur);
    },
    [remove],
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
              className={`toast toast-${t.variant}`}
              role={t.variant === 'error' ? 'alert' : 'status'}
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
                onClick={() => remove(t.id)}
                aria-label={tr('close')}
              >
                <X size={16} />
              </button>
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
