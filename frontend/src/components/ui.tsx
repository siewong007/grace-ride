import { createContext, ReactNode, useCallback, useContext, useState } from 'react';
import { uploadFile } from '../api';
import { Tone } from '../format';
import { useTranslation } from 'react-i18next';

export const Pill = ({ tone = 'gray', children }: { tone?: Tone; children: ReactNode }) => (
  <span className={`pill ${tone}`}>{children}</span>
);

export function StatusPill({ map, status }: { map: Record<string, { label: string; tone: Tone }>; status: string }) {
  const { t } = useTranslation();
  const s = map[status] ?? { label: status, tone: 'gray' as Tone };
  return <Pill tone={s.tone}>{t(s.label)}</Pill>;
}

export const Spinner = () => <div className="spinner" role="status" aria-label="载入中" />;

export const Empty = ({ children }: { children: ReactNode }) => <div className="empty">{children}</div>;

export const ErrorBox = ({ error }: { error?: string | null }) => (error ? <div className="error" role="alert">{error}</div> : null);

export function Avatar({ name, fileId, size }: { name: string; fileId?: number | null; size?: 'lg' }) {
  return (
    <div className={`avatar ${size ?? ''}`} aria-hidden>
      {fileId ? <img src={`/api/files/${fileId}`} alt="" /> : name.slice(0, 1)}
    </div>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
      {hint && <small className="hint">{hint}</small>}
    </label>
  );
}

export function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
  return (
    <div className="modal-bg" onClick={onClose} role="dialog" aria-modal="true" aria-label={title}>
      <div className="modal stack" onClick={(e) => e.stopPropagation()}>
        <div className="row between">
          <h2>{title}</h2>
          <button className="btn ghost sm" onClick={onClose}>关闭</button>
        </div>
        {children}
      </div>
    </div>
  );
}

// ---------- Toasts ----------
const ToastCtx = createContext<(msg: string) => void>(() => {});
export function ToastProvider({ children }: { children: ReactNode }) {
  const [msg, setMsg] = useState<string | null>(null);
  const show = useCallback((m: string) => {
    setMsg(m);
    setTimeout(() => setMsg((cur) => (cur === m ? null : cur)), 3000);
  }, []);
  return (
    <ToastCtx.Provider value={show}>
      {children}
      {msg && <div className="toast" role="status">{msg}</div>}
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
        if (success) toast(success);
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

export function FileInput({ label, value, onChange, accept = 'image/*,application/pdf' }: {
  label: string;
  value: number | null;
  onChange: (id: number | null) => void;
  accept?: string;
}) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState<string>('');
  return (
    <div className="field">
      <span>{label}</span>
      <label className="check" style={{ justifyContent: 'center', borderStyle: 'dashed' }}>
        <input
          type="file"
          accept={accept}
          style={{ display: 'none' }}
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (!f) return;
            setBusy(true);
            setErr(null);
            try {
              const r = await uploadFile(f);
              setName(f.name);
              onChange(r.id);
            } catch (x) {
              setErr((x as Error).message);
            } finally {
              setBusy(false);
            }
          }}
        />
        {busy ? '上传中…' : value ? `✓ 已上传 ${name || ''}` : '📎 选择图片或 PDF'}
      </label>
      {err && <small className="error">{err}</small>}
    </div>
  );
}

export function ReceiptLink({ id, label = '查看收据' }: { id?: number | null; label?: string }) {
  if (!id) return null;
  return (
    <a className="btn sm" href={`/api/files/${id}`} target="_blank" rel="noreferrer">
      {label}
    </a>
  );
}
