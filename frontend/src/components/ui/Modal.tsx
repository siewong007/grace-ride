import {
  createContext,
  ReactNode,
  RefObject,
  useCallback,
  useContext,
  useEffect,
  useId,
  useRef,
  useState,
} from 'react';
import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from './Button';

export interface ModalProps {
  title: string;
  description?: string;
  onClose: () => void;
  footer?: ReactNode;
  size?: 'sm' | 'md' | 'lg';
  initialFocusRef?: RefObject<HTMLElement | null>;
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
  initialFocusRef,
  children,
}: ModalProps) {
  const { t } = useTranslation();
  const backdropRef = useRef<HTMLDivElement>(null);
  const modalRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const previousActiveElement = useRef<HTMLElement | null>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
  const initialFocusTargetRef = useRef(initialFocusRef);
  initialFocusTargetRef.current = initialFocusRef;
  const titleId = useId();
  const descId = useId();

  useEffect(() => {
    previousActiveElement.current = document.activeElement as HTMLElement;
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';

    const backdrop = backdropRef.current ?? modalRef.current?.parentElement ?? null;
    if (backdrop) {
      const index = Array.from(document.querySelectorAll('.modal-bg')).indexOf(backdrop);
      if (index > 0) backdrop.style.zIndex = String(100 + index * 10);
    }

    const isTopModal = () => {
      const all = document.querySelectorAll('.modal-bg');
      return !!backdrop && all[all.length - 1] === backdrop;
    };

    if (initialFocusTargetRef.current?.current) {
      initialFocusTargetRef.current.current.focus();
    } else {
      const focusable = modalRef.current?.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (focusable && focusable.length > 0) {
        focusable[0].focus();
      } else {
        modalRef.current?.focus();
      }
    }

    const onKeyDown = (e: KeyboardEvent) => {
      if (!isTopModal()) return;
      if (e.key === 'Escape') {
        e.preventDefault();
        onCloseRef.current();
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

    window.addEventListener('keydown', onKeyDown, true);
    return () => {
      document.body.style.overflow = originalOverflow;
      window.removeEventListener('keydown', onKeyDown, true);
      previousActiveElement.current?.focus();
    };
  }, []);

  useEffect(() => {
    const modal = modalRef.current;
    const backdrop = backdropRef.current;
    if (!modal || !backdrop) return;

    let snapTimer: number | null = null;
    let dismissTimer: number | null = null;
    let gesture: {
      startX: number;
      startY: number;
      lastY: number;
      startTime: number;
      startedOnHeader: boolean;
      active: boolean;
    } | null = null;

    const clearTimers = () => {
      if (snapTimer !== null) {
        window.clearTimeout(snapTimer);
        snapTimer = null;
      }
      if (dismissTimer !== null) {
        window.clearTimeout(dismissTimer);
        dismissTimer = null;
      }
    };

    const isBottomSheetViewport = () =>
      typeof window !== 'undefined' &&
      (window.matchMedia?.('(max-width: 767px)').matches ?? window.innerWidth < 768);

    const onTouchStart = (e: TouchEvent) => {
      if (e.touches.length !== 1 || !isBottomSheetViewport()) {
        gesture = null;
        return;
      }
      const target = e.target instanceof Element ? e.target : null;
      if (target?.closest('input, textarea, select, [role="slider"], .leaflet-container')) {
        gesture = null;
        return;
      }
      const startedOnHeader = Boolean(target?.closest('.sheet-handle, .modal-head'));
      if (!startedOnHeader && (bodyRef.current?.scrollTop ?? 0) > 0) {
        gesture = null;
        return;
      }
      clearTimers();
      modal.classList.remove('is-snapping', 'is-dismissing');
      backdrop.classList.remove('is-snapping', 'is-dismissing');
      const touch = e.touches[0];
      gesture = {
        startX: touch.clientX,
        startY: touch.clientY,
        lastY: touch.clientY,
        startTime: Date.now(),
        startedOnHeader,
        active: false,
      };
    };

    const onTouchMove = (e: TouchEvent) => {
      if (!gesture || e.touches.length !== 1) return;
      const touch = e.touches[0];
      const dx = touch.clientX - gesture.startX;
      const dy = touch.clientY - gesture.startY;
      gesture.lastY = touch.clientY;

      if (!gesture.active) {
        if (Math.hypot(dx, dy) < 8) return;
        const bodyAtTop = (bodyRef.current?.scrollTop ?? 0) <= 0;
        if (dy > 0 && dy > Math.abs(dx) * 1.25 && (gesture.startedOnHeader || bodyAtTop)) {
          gesture.active = true;
          modal.classList.add('is-dragging');
          backdrop.classList.add('is-dragging');
        } else {
          gesture = null;
          return;
        }
      }

      const dragY = Math.max(0, dy);
      const progress = Math.min(1, dragY / 320);
      modal.style.transform = `translate3d(0, ${dragY}px, 0)`;
      backdrop.style.opacity = String(Math.max(0.2, 1 - progress * 0.75));
    };

    const finishDrag = (e?: TouchEvent) => {
      const g = gesture;
      gesture = null;
      if (!g?.active) return;

      const endY = e?.changedTouches?.[0]?.clientY ?? g.lastY;
      const dy = Math.max(0, endY - g.startY);
      const velocity = dy / Math.max(1, Date.now() - g.startTime);
      const shouldDismiss = dy >= 96 || (dy >= 40 && velocity >= 0.55);
      const reduce =
        typeof window !== 'undefined' &&
        (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false);

      modal.classList.remove('is-dragging');
      backdrop.classList.remove('is-dragging');

      if (shouldDismiss) {
        if (reduce) {
          onCloseRef.current();
          return;
        }
        modal.classList.add('is-dismissing');
        backdrop.classList.add('is-dismissing');
        modal.style.transform = 'translate3d(0, 100%, 0)';
        backdrop.style.opacity = '0';
        dismissTimer = window.setTimeout(() => {
          onCloseRef.current();
        }, 160);
      } else {
        modal.classList.add('is-snapping');
        backdrop.classList.add('is-snapping');
        modal.style.transform = '';
        backdrop.style.opacity = '';
        snapTimer = window.setTimeout(() => {
          modal.classList.remove('is-snapping');
          backdrop.classList.remove('is-snapping');
        }, 220);
      }
    };

    const onTouchCancel = () => finishDrag();

    modal.addEventListener('touchstart', onTouchStart, { passive: true });
    modal.addEventListener('touchmove', onTouchMove, { passive: true });
    modal.addEventListener('touchend', finishDrag, { passive: true });
    modal.addEventListener('touchcancel', onTouchCancel, { passive: true });

    return () => {
      clearTimers();
      modal.removeEventListener('touchstart', onTouchStart);
      modal.removeEventListener('touchmove', onTouchMove);
      modal.removeEventListener('touchend', finishDrag);
      modal.removeEventListener('touchcancel', onTouchCancel);
    };
  }, []);

  return (
    <div ref={backdropRef} className="modal-bg" onClick={onClose} role="presentation">
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
        <div className="modal-head">
          <div className="grow">
            <h2 id={titleId}>{title}</h2>
            {description && <p id={descId} className="muted small">{description}</p>}
          </div>
          <button type="button" className="icon-btn modal-close" onClick={onClose} aria-label={t('close') || 'Close'}>
            <X size={20} aria-hidden="true" />
          </button>
        </div>
        <div ref={bodyRef} className="modal-body stack">{children}</div>
        {footer && <div className="modal-footer row between">{footer}</div>}
      </div>
    </div>
  );
}

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
