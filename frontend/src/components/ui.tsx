import React, {
  createContext,
  FormEvent,
  forwardRef,
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
  ArrowUp,
  ArrowUpDown,
  CheckCircle2,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  Info,
  Paperclip,
  X,
  Lock,
} from 'lucide-react';
import { uploadFile } from '../api';
import { observeElementRect, useVirtualizer } from '@tanstack/react-virtual';
import { Tone } from '../format';
import { LoadingMark } from './Brand';
import { GuideHeaderActionContext, usePageChrome } from './PageChrome';
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
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button({
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
}: ButtonProps, ref) {
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
      ref={ref}
      type={type}
      className={classes}
      disabled={disabled || loading}
      aria-busy={loading ? 'true' : undefined}
      {...rest}
    >
      {content}
    </button>
  );
});

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
  const guideHeaderAction = useContext(GuideHeaderActionContext);
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
      {(actions || guideHeaderAction) && (
        <div className="btn-row page-head-actions">
          {guideHeaderAction}
          {actions}
        </div>
      )}
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

/**
 * One message for every page the viewer may not open, with a way back. The route guard
 * uses it too; `mfa` explains that staff pages wait for a passkey or 2FA sign-in.
 */
export function NoAccess({ mfa = false }: { mfa?: boolean }) {
  const { t } = useTranslation();
  return (
    <EmptyState
      icon={<Lock size={28} />}
      title={t('noAccess.title')}
      description={mfa ? t('noAccess.mfa') : t('noAccess.body')}
      action={
        <Link className="btn secondary" to="/">
          {t('noAccessHome')}
        </Link>
      }
    />
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

export function Field({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  children: ReactNode;
}) {
  return (
    <label className={`field ${error ? 'has-error' : ''}`}>
      <span>{label}</span>
      {children}
      {hint && !error && <small className="hint">{hint}</small>}
      {error && <span className="field-error" role="alert">{error}</span>}
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

// ==========================================
// Pagination & Infinite Scroll Components
// ==========================================

export interface PaginationProps {
  page: number;
  totalPages: number;
  total?: number;
  pageSize?: number;
  onPageChange: (page: number) => void;
  onPageSizeChange?: (pageSize: number) => void;
  pageSizeOptions?: number[];
  className?: string;
}

export function Pagination({
  page,
  totalPages,
  total,
  pageSize,
  onPageChange,
  onPageSizeChange,
  pageSizeOptions = [25, 50, 100],
  className = '',
}: PaginationProps) {
  const { t } = useTranslation('common');
  const [jumpPage, setJumpPage] = useState('');

  const handleJump = (e: FormEvent) => {
    e.preventDefault();
    const p = parseInt(jumpPage, 10);
    if (!isNaN(p) && p >= 1 && p <= totalPages) {
      onPageChange(p);
      setJumpPage('');
    }
  };

  if (totalPages <= 1 && (total === undefined || total <= (pageSize ?? 25))) {
    return null;
  }

  const pages: (number | string)[] = [];
  if (totalPages <= 7) {
    for (let i = 1; i <= totalPages; i++) pages.push(i);
  } else {
    pages.push(1);
    if (page > 3) pages.push('ellipsis-start');
    const start = Math.max(2, page - 1);
    const end = Math.min(totalPages - 1, page + 1);
    for (let i = start; i <= end; i++) {
      if (!pages.includes(i)) pages.push(i);
    }
    if (page < totalPages - 2) pages.push('ellipsis-end');
    if (!pages.includes(totalPages)) pages.push(totalPages);
  }

  return (
    <nav className={`pagination-wrap ${className}`} aria-label={t('pagination.page', { page })}>
      <div className="sr-only" aria-live="polite" aria-atomic="true">
        {t('pagination.page', { page })}
      </div>
      {total !== undefined && (
        <span className="pagination-total muted small">
          {pageSize
            ? t('pagination.showingRange', {
                from: total === 0 ? 0 : (page - 1) * pageSize + 1,
                to: Math.min(total, page * pageSize),
                total,
              })
            : t('pagination.totalRecords', { total })}
        </span>
      )}
      <div className="pagination-controls">
        <button
          type="button"
          className="btn pagination-btn"
          disabled={page <= 1}
          onClick={() => onPageChange(page - 1)}
          aria-label={t('pagination.prev')}
        >
          <ChevronLeft size={16} aria-hidden="true" />
          <span className="pagination-btn-text">{t('pagination.prev')}</span>
        </button>

        <div className="pagination-pages" role="list">
          {pages.map((p, idx) => {
            if (typeof p === 'string') {
              return (
                <span key={`${p}-${idx}`} className="pagination-ellipsis" aria-hidden="true">
                  …
                </span>
              );
            }
            const isCurrent = p === page;
            return (
              <button
                key={p}
                type="button"
                className={`btn pagination-page ${isCurrent ? 'active' : ''}`}
                aria-current={isCurrent ? 'page' : undefined}
                aria-label={t('pagination.page', { page: p })}
                onClick={() => onPageChange(p)}
              >
                {p}
              </button>
            );
          })}
        </div>

        <button
          type="button"
          className="btn pagination-btn"
          disabled={page >= totalPages}
          onClick={() => onPageChange(page + 1)}
          aria-label={t('pagination.next')}
        >
          <span className="pagination-btn-text">{t('pagination.next')}</span>
          <ChevronRight size={16} aria-hidden="true" />
        </button>
      </div>

      {totalPages > 5 && (
        <form onSubmit={handleJump} className="pagination-jump">
          <span className="small muted">{t('pagination.jumpTo')}</span>
          <input
            type="number"
            min={1}
            max={totalPages}
            className="input pagination-jump-input"
            value={jumpPage}
            onChange={(e) => setJumpPage(e.target.value)}
            aria-label={t('pagination.jumpTo')}
            placeholder={String(page)}
          />
          <button type="submit" className="btn pagination-btn" disabled={!jumpPage}>
            {t('pagination.pageGo')}
          </button>
        </form>
      )}

      {onPageSizeChange && pageSize !== undefined && (
        <div className="pagination-size">
          <label className="row nowrap small muted">
            {t('pagination.pageSize')}
            <select
              className="input pagination-select"
              value={pageSize}
              onChange={(e) => onPageSizeChange(Number(e.target.value))}
              aria-label={t('pagination.pageSize')}
            >
              {pageSizeOptions.map((sz) => (
                <option key={sz} value={sz}>
                  {sz}
                </option>
              ))}
            </select>
          </label>
        </div>
      )}
    </nav>
  );
}

export interface UseInfiniteScrollOptions {
  hasMore: boolean;
  loading: boolean;
  onLoadMore: () => void;
  rootMargin?: string;
  disabled?: boolean;
}

export function useInfiniteScroll({
  hasMore,
  loading,
  onLoadMore,
  rootMargin = '250px',
  disabled = false,
}: UseInfiniteScrollOptions) {
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (disabled || !hasMore || loading) return;
    if (typeof IntersectionObserver === 'undefined') return;
    const el = sentinelRef.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          onLoadMore();
        }
      },
      { rootMargin }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [disabled, hasMore, loading, onLoadMore, rootMargin]);

  return { sentinelRef };
}

export function InfiniteSentinel({
  hasMore,
  loading,
  sentinelRef,
  empty = false,
  className = '',
}: {
  hasMore: boolean;
  loading: boolean;
  sentinelRef: React.RefObject<HTMLDivElement>;
  empty?: boolean;
  className?: string;
}) {
  const { t } = useTranslation('common');
  if (empty) return null;

  return (
    <div
      ref={sentinelRef}
      className={`infinite-sentinel ${className}`}
      role="status"
      aria-live="polite"
    >
      {loading && (
        <div className="row center gap-sm py-md">
          <Spinner />
          <span className="small muted">{t('pagination.loadingMore')}</span>
        </div>
      )}
      {!hasMore && !loading && (
        <div className="center py-md text-muted small">
          {t('pagination.noMore')}
        </div>
      )}
    </div>
  );
}

// ==========================================
// Back To Top Floating Trigger
// ==========================================

export function BackToTop({ className = '' }: { className?: string }) {
  const { t } = useTranslation('common');
  const [visible, setVisible] = useState(false);
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (typeof IntersectionObserver === 'undefined') return;
    const el = sentinelRef.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        setVisible(!entry.isIntersecting);
      },
      { threshold: 0 },
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const scrollToTop = () => {
    const prefersReduced =
      typeof window !== 'undefined' &&
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    window.scrollTo({ top: 0, behavior: prefersReduced ? 'auto' : 'smooth' });
  };

  return (
    <>
      <div ref={sentinelRef} className="back-to-top-sentinel" aria-hidden="true" />
      {visible && (
        <button
          type="button"
          className={`back-to-top-btn ${className}`}
          onClick={scrollToTop}
          aria-label={t('backToTop')}
        >
          <ArrowUp size={16} aria-hidden="true" />
          <span className="small">{t('backToTop')}</span>
        </button>
      )}
    </>
  );
}

// ==========================================
// Universal Table Component
// ==========================================

export type SortDirection = 'asc' | 'desc';

export interface TableSort {
  key: string;
  dir: SortDirection;
}

export type TableFilter =
  | { type: 'text'; key: string; label: string; placeholder?: string }
  | {
      type: 'select';
      key: string;
      label: string;
      allLabel?: string;
      options: { value: string; label: string }[];
    }
  | { type: 'date'; key: string; label: string }
  | { type: 'checkbox'; key: string; label: string };

export interface TableColumn<T = unknown> {
  /** Stable key for the column, used as the cell key. */
  key: string;
  /** Header label for the column. */
  header: ReactNode;
  /** Renders the cell contents for a single row. */
  render: (item: T, rowIndex: number) => ReactNode;
  /** Label shown for this column on narrow (stacked) screens; falls back to `header`. */
  dataLabel?: string;
  /** Extra class name applied to the header cell. */
  headerClassName?: string;
  /** Extra class name applied to the cell. */
  cellClassName?: string;
  /** Enables the header sort control. */
  sortable?: boolean;
  /** Field used in sort state / server query; defaults to `key`. */
  sortKey?: string;
  /** Required for local sorting when the table is uncontrolled. */
  sortValue?: (item: T) => string | number | null | undefined;
  /** Accessible/mobile-sort label when `header` is not a string. */
  sortLabel?: string;
}

export interface UniversalTableProps<T> {
  /** Column definitions. */
  columns: TableColumn<T>[];
  /** Row data to render. */
  items: T[];
  /** When provided, rows become clickable and this is called with the row data. */
  onRowClick?: (item: T, rowIndex: number) => void;
  /** Stable key selector for each row. Defaults to the array index. */
  itemKey?: (item: T, index: number) => string | number;
  /** Content rendered when `items` is empty. */
  emptyContent?: ReactNode;
  /** Optional callback; returns true for a row that spans all columns (e.g. a group header). */
  isSectionRow?: (item: T, index: number) => boolean;
  /** Extra class name applied to the wrapping element. */
  className?: string;
  /** Controlled sort. When set, the table does not reorder `items`. */
  sort?: TableSort | null;
  /** Initial local sort when uncontrolled. */
  defaultSort?: TableSort | null;
  onSortChange?: (sort: TableSort | null) => void;
  /** Declarative filter toolbar. */
  filters?: TableFilter[];
  filterValues?: Record<string, string>;
  onFilterChange?: (key: string, value: string) => void;
  onClearFilters?: () => void;
  clearFiltersLabel?: string;
  toolbarActions?: ReactNode;
}

function columnSortKey<T>(col: TableColumn<T>): string {
  return col.sortKey ?? col.key;
}

function defaultDataLabel<T>(col: TableColumn<T>): string | undefined {
  return col.dataLabel ?? (typeof col.header === 'string' ? col.header : undefined);
}

function sortLabel<T>(col: TableColumn<T>): string {
  return col.sortLabel ?? (typeof col.header === 'string' ? col.header : col.key);
}

function compareValues(a: unknown, b: unknown): number {
  if (typeof a === 'number' && typeof b === 'number') return a - b;
  return String(a).localeCompare(String(b), undefined, {
    numeric: true,
    sensitivity: 'base',
  });
}

function isInteractive(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    Boolean(
      target.closest(
        'a,button,input,select,textarea,[role="button"],[data-row-click-ignore]',
      ),
    )
  );
}

export function UniversalTable<T>({
  columns,
  items,
  onRowClick,
  itemKey,
  emptyContent,
  isSectionRow,
  className = '',
  sort,
  defaultSort = null,
  onSortChange,
  filters,
  filterValues,
  onFilterChange,
  onClearFilters,
  clearFiltersLabel,
  toolbarActions,
}: UniversalTableProps<T>) {
  const { t } = useTranslation('common');
  const [localSort, setLocalSort] = useState<TableSort | null>(defaultSort);
  const controlled = sort !== undefined;
  const activeSort = controlled ? sort : localSort;
  const sortableColumns = columns.filter((col) => col.sortable);

  const rows = useMemo(() => {
    if (controlled || !activeSort) return items;
    const col = columns.find((c) => columnSortKey(c) === activeSort.key);
    if (!col?.sortValue) return items;
    const direction = activeSort.dir === 'asc' ? 1 : -1;
    return items
      .map((item, index) => ({ item, index }))
      .sort((a, b) => {
        const av = col.sortValue?.(a.item);
        const bv = col.sortValue?.(b.item);
        // Nulls always sort last, whichever direction is active.
        if (av == null && bv == null) return a.index - b.index;
        if (av == null) return 1;
        if (bv == null) return -1;
        const cmp = compareValues(av, bv);
        if (cmp === 0) return a.index - b.index;
        return cmp * direction;
      })
      .map(({ item }) => item);
  }, [activeSort, columns, controlled, items]);

  const changeSort = (col: TableColumn<T>) => {
    const key = columnSortKey(col);
    const next: TableSort = {
      key,
      dir: activeSort?.key === key && activeSort.dir === 'asc' ? 'desc' : 'asc',
    };
    if (!controlled) setLocalSort(next);
    onSortChange?.(next);
  };

  const filtersList = filters ?? [];
  const values = filterValues ?? {};
  const hasActiveFilters = filtersList.some((f) => (values[f.key] ?? '') !== '');

  const renderFilter = (f: TableFilter) => {
    const value = values[f.key] ?? '';
    switch (f.type) {
      case 'text':
        return (
          <div key={f.key} className="table-filter">
            <input
              className="input admin-input-compact"
              type="text"
              value={value}
              placeholder={f.placeholder ?? f.label}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            />
          </div>
        );
      case 'select':
        return (
          <div key={f.key} className="table-filter">
            <select
              className="input admin-input-compact"
              value={value}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            >
              {!f.options.some((o) => o.value === '') && (
                <option value="">{f.allLabel ?? f.label}</option>
              )}
              {f.options.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.value === '' ? (f.allLabel ?? o.label) : o.label}
                </option>
              ))}
            </select>
          </div>
        );
      case 'date':
        return (
          <div key={f.key} className="table-filter">
            <input
              className="input admin-input-compact"
              type="date"
              value={value}
              aria-label={f.label}
              onChange={(e) => onFilterChange?.(f.key, e.target.value)}
            />
          </div>
        );
      case 'checkbox':
        return (
          <div key={f.key} className="table-filter">
            <label className={`check ${value === 'true' ? 'on' : ''}`}>
              <input
                type="checkbox"
                checked={value === 'true'}
                aria-label={f.label}
                onChange={(e) => onFilterChange?.(f.key, e.target.checked ? 'true' : '')}
              />
              {f.label}
            </label>
          </div>
        );
    }
  };

  const tableClassName = onRowClick
    ? 'table responsive sticky-header row-clickable'
    : 'table responsive sticky-header';

  return (
    <div className={`table-block${className ? ` ${className}` : ''}`}>
      {sortableColumns.length > 0 && (
        <label className="table-mobile-sort">
          <span>{t('table.sortBy')}</span>
          <select
            className="input admin-input-compact"
            value={activeSort ? `${activeSort.key}:${activeSort.dir}` : ''}
            aria-label={t('table.sortBy')}
            onChange={(e) => {
              const raw = e.target.value;
              if (!raw) {
                if (!controlled) setLocalSort(defaultSort);
                onSortChange?.(null);
                return;
              }
              const separator = raw.lastIndexOf(':');
              const key = raw.slice(0, separator);
              const dir = raw.slice(separator + 1);
              const next: TableSort = { key, dir: dir === 'desc' ? 'desc' : 'asc' };
              if (!controlled) setLocalSort(next);
              onSortChange?.(next);
            }}
          >
            <option value="">{t('table.defaultOrder')}</option>
            {sortableColumns.map((col) => {
              const key = columnSortKey(col);
              const label = sortLabel(col);
              return (
                <optgroup key={key} label={label}>
                  <option value={`${key}:asc`}>{label} ↑</option>
                  <option value={`${key}:desc`}>{label} ↓</option>
                </optgroup>
              );
            })}
          </select>
        </label>
      )}

      {(filtersList.length > 0 || toolbarActions) && (
        <div className="table-toolbar">
          {filtersList.map(renderFilter)}
          {hasActiveFilters && onClearFilters && (
            <Button variant="ghost" size="sm" onClick={onClearFilters}>
              {clearFiltersLabel ?? t('table.clearFilters')}
            </Button>
          )}
          {toolbarActions && <div className="table-toolbar-actions">{toolbarActions}</div>}
        </div>
      )}

      <div className="table-wrap">
        <table className={tableClassName}>
          <thead>
            <tr>
              {columns.map((col) => {
                const sorted = col.sortable && activeSort?.key === columnSortKey(col);
                const direction = sorted ? activeSort?.dir : undefined;
                return (
                  <th
                    key={col.key}
                    scope="col"
                    className={col.headerClassName}
                    aria-sort={
                      col.sortable
                        ? sorted
                          ? direction === 'asc'
                            ? 'ascending'
                            : 'descending'
                          : 'none'
                        : undefined
                    }
                  >
                    {col.sortable ? (
                      <button
                        type="button"
                        className="th-sort"
                        onClick={() => changeSort(col)}
                        aria-label={`${t('table.sortBy')} ${sortLabel(col)}`}
                      >
                        <span>{col.header}</span>
                        {direction === 'asc' ? (
                          <ChevronUp size={14} aria-hidden="true" />
                        ) : direction === 'desc' ? (
                          <ChevronDown size={14} aria-hidden="true" />
                        ) : (
                          <ArrowUpDown size={14} aria-hidden="true" className="sort-indicator" />
                        )}
                      </button>
                    ) : (
                      col.header
                    )}
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {rows.map((item, rowIndex) => {
              const key = itemKey ? itemKey(item, rowIndex) : rowIndex;
              const isSection = isSectionRow?.(item, rowIndex) ?? false;
              return (
                <tr
                  key={key}
                  className={onRowClick ? 'clickable' : ''}
                  tabIndex={onRowClick ? 0 : undefined}
                  onClick={(e) => {
                    if (isInteractive(e.target)) return;
                    onRowClick?.(item, rowIndex);
                  }}
                  onKeyDown={(e) => {
                    if (e.target !== e.currentTarget) return;
                    if (onRowClick && (e.key === 'Enter' || e.key === ' ')) {
                      e.preventDefault();
                      onRowClick?.(item, rowIndex);
                    }
                  }}
                >
                  {isSection ? (
                    <td key={`section-${key}`} colSpan={columns.length}>
                      {columns[0]?.render?.(item, rowIndex)}
                    </td>
                  ) : (
                    columns.map((col) => (
                      <td
                        key={col.key}
                        data-label={defaultDataLabel(col)}
                        className={col.cellClassName}
                      >
                        {col.render(item, rowIndex)}
                      </td>
                    ))
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {items.length === 0 && emptyContent != null && <div>{emptyContent}</div>}
    </div>
  );
}

export default UniversalTable;

// Virtualized List Component (TanStack Virtual)
// ==========================================

export interface VirtualListProps<T> {
  items: T[];
  height: number | string;
  estimateSize: number;
  renderItem: (item: T, index: number) => ReactNode;
  overscan?: number;
  className?: string;
  getItemKey?: (item: T, index: number) => string | number;
}

export function VirtualList<T>({
  items,
  height,
  estimateSize,
  renderItem,
  overscan = 5,
  className = '',
  getItemKey,
}: VirtualListProps<T>) {
  const parentRef = useRef<HTMLDivElement | null>(null);
  const fallbackHeight = typeof height === 'number' ? height : 600;

  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => estimateSize,
    overscan,
    getItemKey: getItemKey ? (index) => getItemKey(items[index], index) : undefined,
    initialRect: { width: 300, height: fallbackHeight },
    observeElementRect: (instance, cb) => {
      return observeElementRect(instance, (rect) => {
        cb({
          width: rect.width || 300,
          height: rect.height || fallbackHeight,
        });
      });
    },
  });

  return (
    <div
      ref={parentRef}
      className={`virtual-list-container ${className}`}
      style={{
        height,
        overflow: 'auto',
        position: 'relative',
      }}
    >
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map((virtualItem) => (
          <div
            key={virtualItem.key}
            data-index={virtualItem.index}
            ref={virtualizer.measureElement}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              transform: `translateY(${virtualItem.start}px)`,
            }}
          >
            {renderItem(items[virtualItem.index], virtualItem.index)}
          </div>
        ))}
      </div>
    </div>
  );
}
