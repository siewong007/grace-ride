import React, {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from 'react';
import { Link } from 'react-router-dom';
import {
  AlertCircle,
  ArrowLeft,
  CheckCircle2,
  Info,
  Paperclip,
  X,
} from 'lucide-react';
import { uploadFile } from '../api';
import { Tone } from '../format';
import { LoadingMark } from './Brand';
import { usePageChrome } from './PageChrome';
import { useTranslation } from 'react-i18next';

// ==========================================
// Button & IconButton
// ==========================================

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  block?: boolean;
  loading?: boolean;
  icon?: ReactNode;
  to?: string;
}

/**
 * Standard button primitive with variants, loading state, icon support, and link routing.
 */
export function Button({
  variant = 'secondary',
  size = 'md',
  block = false,
  loading = false,
  icon,
  to,
  type = 'button',
  disabled,
  className = '',
  children,
  ...rest
}: ButtonProps) {
  const classes = [
    'btn',
    variant !== 'secondary' ? variant : '',
    size !== 'md' ? size : '',
    block ? 'block' : '',
    loading ? 'loading' : '',
    className,
  ].filter(Boolean).join(' ');

  const content = (
    <>
      {loading ? <span className="btn-spinner" aria-hidden="true" /> : icon}
      {children}
    </>
  );

  if (to && !disabled && !loading) {
    return (
      <Link to={to} className={classes} {...(rest as unknown as React.AnchorHTMLAttributes<HTMLAnchorElement>)}>
        {content}
      </Link>
    );
  }

  return (
    <button
      type={type}
      className={classes}
      disabled={disabled || loading}
      aria-busy={loading ? 'true' : undefined}
      {...rest}
    >
      {content}
    </button>
  );
}

export interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  icon: ReactNode;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md';
  to?: string;
  children?: ReactNode;
}

/**
 * Accessible 48px square icon button with a required aria-label.
 */
export function IconButton({
  label,
  icon,
  variant = 'secondary',
  size = 'md',
  to,
  type = 'button',
  className = '',
  children,
  ...rest
}: IconButtonProps) {
  const classes = [
    'icon-btn',
    variant !== 'secondary' ? variant : '',
    size === 'sm' ? 'sm' : '',
    className,
  ].filter(Boolean).join(' ');

  if (to && !rest.disabled) {
    return (
      <Link to={to} className={classes} aria-label={label} {...(rest as unknown as React.AnchorHTMLAttributes<HTMLAnchorElement>)}>
        {icon}
        {children}
      </Link>
    );
  }

  return (
    <button type={type} className={classes} aria-label={label} {...rest}>
      {icon}
      {children}
    </button>
  );
}

// ==========================================
// Card & Page Primitives
// ==========================================

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'flat' | 'link';
  padding?: 'sm' | 'md' | 'lg';
  to?: string;
  children: ReactNode;
}

/**
 * Container card wrapping responsive surface styles and padding variants.
 */
export function Card({
  variant = 'default',
  padding = 'md',
  to,
  className = '',
  children,
  ...rest
}: CardProps) {
  const classes = [
    'card',
    variant !== 'default' ? variant : '',
    padding === 'sm' ? 'tight' : padding === 'lg' ? 'pad-lg' : '',
    className,
  ].filter(Boolean).join(' ');

  if (to) {
    return (
      <Link to={to} className={classes} {...(rest as unknown as React.AnchorHTMLAttributes<HTMLAnchorElement>)}>
        {children}
      </Link>
    );
  }

  return (
    <div className={classes} {...rest}>
      {children}
    </div>
  );
}

export interface PageHeaderProps {
  title: ReactNode;
  subtitle?: ReactNode;
  back?: string;
  actions?: ReactNode;
  className?: string;
}

/**
 * Page heading with optional subtitle and actions. Inside the app frame the back link
 * lives in the top bar, and the top bar shows the title only once this heading scrolls away.
 */
export function PageHeader({ title, subtitle, back, actions, className = '' }: PageHeaderProps) {
  const { t } = useTranslation();
  const headingRef = useRef<HTMLHeadingElement>(null);
  const inFrame = usePageChrome(headingRef, back);
  return (
    <div className={`page-head ${className}`.trim()}>
      <div className="row nowrap page-head-title">
        {back && !inFrame && (
          <Link to={back} className="icon-btn sm hide-tablet-up" aria-label={t('back') || 'Back'}>
            <ArrowLeft size={20} />
          </Link>
        )}
        <div>
          <h1 ref={headingRef}>{title}</h1>
          {subtitle && <p className="muted">{subtitle}</p>}
        </div>
      </div>
      {actions && <div className="btn-row page-head-actions">{actions}</div>}
    </div>
  );
}

export interface PageProps extends React.HTMLAttributes<HTMLDivElement> {
  width?: 'narrow' | 'medium' | 'wide';
  children: ReactNode;
}

/**
 * Standard page wrapper replacing inline max-width layouts.
 */
export function Page({ width = 'wide', className = '', children, ...rest }: PageProps) {
  return (
    <div className={`page-container page-${width} ${className}`.trim()} {...rest}>
      {children}
    </div>
  );
}

// ==========================================
// Tabs
// ==========================================

export interface TabItem {
  value: string;
  label: ReactNode;
  count?: number;
  icon?: ReactNode;
}

export interface TabsProps {
  value: string;
  onChange: (value: string) => void;
  items: TabItem[];
  ariaLabel?: string;
  className?: string;
}

/**
 * Accessible tablist supporting keyboard arrow navigation and responsive phone scrolling.
 */
export function Tabs({ value, onChange, items, ariaLabel, className = '' }: TabsProps) {
  const handleKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    let nextIndex = index;
    if (e.key === 'ArrowRight') {
      nextIndex = (index + 1) % items.length;
    } else if (e.key === 'ArrowLeft') {
      nextIndex = (index - 1 + items.length) % items.length;
    } else if (e.key === 'Home') {
      nextIndex = 0;
    } else if (e.key === 'End') {
      nextIndex = items.length - 1;
    } else {
      return;
    }
    e.preventDefault();
    onChange(items[nextIndex].value);
    const btn = document.getElementById(`tab-${items[nextIndex].value}`);
    btn?.focus();
  };

  return (
    <div className={`tabs ${className}`.trim()} role="tablist" aria-label={ariaLabel}>
      {items.map((item, idx) => {
        const isSelected = item.value === value;
        return (
          <button
            key={item.value}
            id={`tab-${item.value}`}
            type="button"
            role="tab"
            aria-selected={isSelected}
            tabIndex={isSelected ? 0 : -1}
            className={isSelected ? 'on' : ''}
            onClick={() => onChange(item.value)}
            onKeyDown={(e) => handleKeyDown(e, idx)}
          >
            {item.icon && <span className="tab-icon" aria-hidden="true">{item.icon}</span>}
            <span>{item.label}</span>
            {item.count != null && (
              <span className="tab-count" aria-label={`(${item.count})`}>
                {item.count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}

// ==========================================
// Modal
// ==========================================

export interface ModalProps {
  title: string;
  description?: string;
  onClose: () => void;
  footer?: ReactNode;
  size?: 'sm' | 'md' | 'lg';
  children: ReactNode;
}

/**
 * Accessible dialog modal with focus trapping, escape dismissal, scroll lock, and phone bottom-sheet.
 */
export function Modal({
  title,
  description,
  onClose,
  footer,
  size = 'md',
  children,
}: ModalProps) {
  const { t } = useTranslation();
  const modalRef = useRef<HTMLDivElement>(null);
  const previousActiveElement = useRef<HTMLElement | null>(null);
  const titleId = useId();
  const descId = useId();

  useEffect(() => {
    previousActiveElement.current = document.activeElement as HTMLElement;
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    const focusable = modalRef.current?.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    if (focusable && focusable.length > 0) {
      focusable[0].focus();
    } else {
      modalRef.current?.focus();
    }

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === 'Tab') {
        if (!modalRef.current) return;
        const nodes = Array.from(
          modalRef.current.querySelectorAll<HTMLElement>(
            'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
          ),
        );
        if (nodes.length === 0) return;
        const first = nodes[0];
        const last = nodes[nodes.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener('keydown', onKeyDown);
      previousActiveElement.current?.focus();
    };
  }, [onClose]);

  return (
    <div className="modal-bg" onClick={onClose} role="presentation">
      {/* eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions */}
      <div
        ref={modalRef}
        className={`modal modal-${size} stack`}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={description ? descId : undefined}
        tabIndex={-1}
      >
        <div className="sheet-handle hide-tablet-up" aria-hidden="true" />
        <div className="row between">
          <div>
            <h2 id={titleId}>{title}</h2>
            {description && <p id={descId} className="muted small">{description}</p>}
          </div>
          <button className="btn ghost sm" onClick={onClose} aria-label={t('close') || 'Close'}>
            <X size={20} aria-hidden="true" />
          </button>
        </div>
        <div className="modal-body stack">{children}</div>
        {footer && <div className="modal-footer row between">{footer}</div>}
      </div>
    </div>
  );
}

// ==========================================
// Confirm Dialog & useConfirm
// ==========================================

export interface ConfirmOptions {
  title: string;
  body: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  tone?: 'danger' | 'warning' | 'primary';
}

const ConfirmCtx = createContext<(opts: ConfirmOptions) => Promise<boolean>>(() => Promise.resolve(false));

/**
 * Context provider managing asynchronous confirmation dialogs replacing window.confirm.
 */
export function ConfirmProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [opts, setOpts] = useState<ConfirmOptions | null>(null);
  const resolveRef = useRef<((val: boolean) => void) | null>(null);

  const confirm = useCallback((o: ConfirmOptions): Promise<boolean> => {
    return new Promise((resolve) => {
      resolveRef.current = resolve;
      setOpts(o);
    });
  }, []);

  const handleClose = (result: boolean) => {
    setOpts(null);
    resolveRef.current?.(result);
    resolveRef.current = null;
  };

  return (
    <ConfirmCtx.Provider value={confirm}>
      {children}
      {opts && (
        <Modal
          title={opts.title}
          onClose={() => handleClose(false)}
          size="sm"
          footer={
            <div className="btn-row end">
              <Button variant="ghost" onClick={() => handleClose(false)}>
                {opts.cancelLabel || t('cancel') || 'Cancel'}
              </Button>
              <Button
                variant={opts.tone === 'primary' ? 'primary' : 'danger'}
                onClick={() => handleClose(true)}
              >
                {opts.confirmLabel || t('confirm') || 'Confirm'}
              </Button>
            </div>
          }
        >
          <div className="confirm-body">{opts.body}</div>
        </Modal>
      )}
    </ConfirmCtx.Provider>
  );
}

export const useConfirm = () => useContext(ConfirmCtx);

// ==========================================
// EmptyState & Empty
// ==========================================

export interface EmptyStateProps {
  icon?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  children?: ReactNode;
  className?: string;
}

/**
 * Empty state display when lists or queries yield zero records.
 */
export function EmptyState({
  icon,
  title,
  description,
  action,
  children,
  className = '',
}: EmptyStateProps) {
  return (
    <div className={`empty ${className}`.trim()}>
      {icon && <div className="empty-icon" aria-hidden="true">{icon}</div>}
      {/* A bare "nothing here" line is a quiet message; with more to say it gets a heading. */}
      {icon || description || action || children ? (
        <h3 className="empty-title">{title}</h3>
      ) : (
        <p className="empty-title">{title}</p>
      )}
      {description && <p className="muted small empty-desc">{description}</p>}
      {children}
      {action && <div className="empty-action">{action}</div>}
    </div>
  );
}

/** Backward-compatible thin wrapper around EmptyState. */
export const Empty = ({ children }: { children: ReactNode }) => (
  <EmptyState title={children} />
);

// ==========================================
// Skeletons
// ==========================================

export interface SkeletonProps {
  lines?: number;
  width?: string | number;
  height?: string | number;
  circle?: boolean;
  className?: string;
}

/**
 * Shimmering loading placeholder, disabled under prefers-reduced-motion.
 */
export function Skeleton({ lines = 1, width, height, circle, className = '' }: SkeletonProps) {
  if (lines > 1) {
    return (
      <div className={`skeleton-stack ${className}`.trim()} aria-hidden="true">
        {Array.from({ length: lines }).map((_, i) => (
          <span
            key={i}
            className="skeleton"
            style={{
              width: i === lines - 1 ? '70%' : width,
              height: height ?? '1rem',
            }}
          />
        ))}
      </div>
    );
  }
  return (
    <span
      className={`skeleton ${circle ? 'circle' : ''} ${className}`.trim()}
      aria-hidden="true"
      style={{ width, height }}
    />
  );
}

/**
 * List skeleton placeholder rendering placeholder items.
 */
export function ListSkeleton({ count = 3 }: { count?: number }) {
  return (
    <div className="list list-skeleton" aria-hidden="true">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="item">
          <Skeleton circle width={44} height={44} />
          <div className="grow stack-sm">
            <Skeleton width="45%" height="1.1rem" />
            <Skeleton width="75%" height="0.875rem" />
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * Card skeleton placeholder rendering container cards with shimmering lines.
 */
export function CardSkeleton({ count = 1 }: { count?: number }) {
  return (
    <div className="grid cols-2" aria-hidden="true">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="card stack">
          <Skeleton width="40%" height="1.2rem" />
          <Skeleton lines={3} height="0.9rem" />
        </div>
      ))}
    </div>
  );
}

// ==========================================
// Pill & StatusPill
// ==========================================

export type PillTone =
  | 'neutral'
  | 'success'
  | 'warning'
  | 'danger'
  | 'info'
  | 'primary'
  // Legacy aliases
  | 'green'
  | 'amber'
  | 'blue'
  | 'red'
  | 'gray'
  | 'teal';

export interface PillProps {
  tone?: PillTone;
  icon?: ReactNode;
  children: ReactNode;
  className?: string;
}

/**
 * Status and tag badge with semantic tones and optional leading icon.
 */
export function Pill({ tone = 'neutral', icon, children, className = '' }: PillProps) {
  const mappedTone =
    tone === 'green' ? 'success' :
    tone === 'amber' ? 'warning' :
    tone === 'blue' ? 'info' :
    tone === 'red' ? 'danger' :
    tone === 'gray' || tone === 'teal' ? 'neutral' : tone;

  return (
    <span className={`pill ${mappedTone} ${className}`.trim()}>
      {icon && <span className="pill-icon" aria-hidden="true">{icon}</span>}
      {children}
    </span>
  );
}

export function StatusPill({
  map,
  status,
  icon,
}: {
  map: Record<string, { label: string; tone: Tone }>;
  status: string;
  icon?: ReactNode;
}) {
  const { t } = useTranslation();
  const s = map[status] ?? { label: status, tone: 'neutral' as Tone };
  return (
    <Pill tone={s.tone as PillTone} icon={icon}>
      {t(s.label)}
    </Pill>
  );
}

// ==========================================
// Toasts
// ==========================================

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

  const add = useCallback((text: string, variant: ToastVariant = 'info') => {
    const id = Math.random().toString(36).slice(2, 9);
    setToasts((cur) => {
      const next = [...cur, { id, text, variant }];
      return next.slice(-3); // queue up to 3
    });
    const dur = variant === 'error' ? 6000 : 3000;
    setTimeout(() => {
      remove(id);
    }, dur);
  }, [remove]);

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

// ==========================================
// Stat & KeyValue
// ==========================================

export interface StatProps {
  label: ReactNode;
  value: ReactNode;
  icon?: ReactNode;
  tone?: 'normal' | 'warn' | 'bad' | 'success';
  to?: string;
  className?: string;
}

/**
 * Metric summary tile for dashboards with optional navigation link and tone.
 */
export function Stat({
  label,
  value,
  tone = 'normal',
  to,
  className = '',
}: StatProps) {
  const toneCls = tone !== 'normal' ? tone : '';
  const classes = ['card', to ? 'link' : '', 'stat', toneCls, className].filter(Boolean).join(' ');

  const content = (
    <>
      <span className="label">{label}</span>
      <span className="num">{value}</span>
    </>
  );

  if (to) {
    return (
      <Link to={to} className={classes}>
        {content}
      </Link>
    );
  }

  return <div className={classes}>{content}</div>;
}

export interface KeyValueItem {
  label: ReactNode;
  value: ReactNode;
}

export interface KeyValueProps {
  items: KeyValueItem[];
  className?: string;
}

/**
 * Structured key-value definition list replacing ad-hoc dl.kv pairs.
 */
export function KeyValue({ items, className = '' }: KeyValueProps) {
  return (
    <dl className={`kv ${className}`.trim()}>
      {items.map((item, idx) => (
        <React.Fragment key={idx}>
          <dt>{item.label}</dt>
          <dd>{item.value}</dd>
        </React.Fragment>
      ))}
    </dl>
  );
}

// ==========================================
// Existing Utilities & Field Components
// ==========================================

/** Loading indicator: the animated Grace Ride mark from the logo kit. */
export const Spinner = () => {
  const { t } = useTranslation();
  return (
    <div className="spinner" role="status" aria-label={t('loading')}>
      <LoadingMark />
    </div>
  );
};

export const ErrorBox = ({ error }: { error?: string | null }) =>
  error ? <div className="error" role="alert">{error}</div> : null;

export function Avatar({ name, fileId, size }: { name: string; fileId?: number | null; size?: 'sm' | 'md' | 'lg' }) {
  return (
    <div className={`avatar ${size ?? ''}`.trim()} aria-hidden>
      {fileId ? <img src={`/api/files/${fileId}`} alt="" /> : name.slice(0, 1)}
    </div>
  );
}

export interface NoticeProps extends React.HTMLAttributes<HTMLDivElement> {
  tone?: 'warning' | 'info' | 'danger' | 'success';
  icon?: ReactNode;
  children: ReactNode;
}

/**
 * Notice banner with tone and icon for account status, announcements, and alerts.
 */
export function Notice({ tone = 'warning', icon, children, className = '', ...rest }: NoticeProps) {
  return (
    <div className={`notice ${tone} ${className}`.trim()} role="status" {...rest}>
      {icon && <span className="notice-icon" aria-hidden="true">{icon}</span>}
      <div className="notice-content">{children}</div>
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

export function FileInput({
  label,
  value,
  onChange,
  accept = 'image/*,application/pdf',
}: {
  label: string;
  value: number | null;
  onChange: (id: number | null) => void;
  accept?: string;
}) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState<string>('');
  return (
    <div className="field">
      <span>{label}</span>
      <label className="check upload">
        <input
          type="file"
          accept={accept}
          className="sr-only"
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
        <Paperclip size={16} aria-hidden="true" />
        {busy ? t('uploading') : value ? t('uploaded', { name: name || '' }).trim() : t('chooseFile')}
      </label>
      {err && <small className="error">{err}</small>}
    </div>
  );
}

export function ReceiptLink({ id, label }: { id?: number | null; label?: string }) {
  const { t } = useTranslation();
  if (!id) return null;
  return (
    <a className="btn sm" href={`/api/files/${id}`} target="_blank" rel="noreferrer">
      {label ?? t('viewReceipt')}
    </a>
  );
}
