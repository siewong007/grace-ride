import React, { FormEvent, useEffect, useRef, useState } from 'react';
import { ArrowUp, ChevronLeft, ChevronRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Spinner } from './Feedback';

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

        <div className="pagination-pages">
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
      { rootMargin },
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
  sentinelRef?: React.RefObject<HTMLDivElement>;
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
