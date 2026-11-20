import { useCallback, useEffect, useRef, useState, type TouchEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronLeft, ChevronRight, Image as ImageIcon, X } from 'lucide-react';
import { useApi } from '../api';

export interface AreaPhoto {
  id: number;
  file_id: number;
  url: string;
  caption: string;
  sort_order: number;
  can_edit: boolean;
  can_delete: boolean;
  mine?: boolean;
  created_by?: { id: number; name: string | null } | null;
}

export interface AreaPhotosResponse {
  area: { id: number; name: string };
  photos: AreaPhoto[];
  max: number;
  can_add: boolean;
  can_reorder: boolean;
}

/**
 * Photos of a pickup area, so a passenger can recognise where to wait. Shows nothing
 * while loading, when the area has no photos, or when they cannot be loaded.
 */
export function AreaPhotoStrip({ areaId, areaName }: { areaId: number | null | undefined; areaName?: string }) {
  const { t } = useTranslation();
  const { data } = useApi<AreaPhotosResponse>(areaId ? `/areas/${areaId}/photos` : null);
  const [open, setOpen] = useState<number | null>(null);
  const photos = data?.photos ?? [];
  if (!areaId || photos.length === 0) return null;
  const name = areaName ?? data?.area.name ?? '';
  return (
    <div className="area-photos stack-xs" data-testid="area-photos">
      <div className="row nowrap gap-xs muted small">
        <ImageIcon size={14} aria-hidden="true" />
        <span>{t('areaPhotos.title', { area: name })}</span>
      </div>
      <ul className="area-photo-strip" aria-label={t('areaPhotos.title', { area: name })}>
        {photos.map((p, i) => (
          <li key={p.id}>
            <button
              type="button"
              className="area-photo-thumb"
              onClick={() => setOpen(i)}
              aria-label={t('areaPhotos.open', { n: i + 1, count: photos.length, caption: p.caption || name })}
            >
              <img src={p.url} alt="" loading="lazy" />
              {p.caption && <span className="area-photo-caption">{p.caption}</span>}
            </button>
          </li>
        ))}
      </ul>
      {open != null && (
        <PhotoViewer photos={photos} start={open} title={name} onClose={() => setOpen(null)} />
      )}
    </div>
  );
}

/**
 * Full-screen photo viewer: swipe or use the arrows to move between photos, tap the
 * backdrop, the close button or Escape to close.
 */
export function PhotoViewer({
  photos,
  start,
  title,
  onClose,
}: {
  photos: Pick<AreaPhoto, 'id' | 'url' | 'caption'>[];
  start: number;
  title: string;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [i, setI] = useState(Math.min(Math.max(start, 0), photos.length - 1));
  const closeRef = useRef<HTMLButtonElement>(null);
  const touch = useRef<{ x: number; y: number } | null>(null);
  const n = photos.length;
  const go = useCallback((d: number) => setI((cur) => Math.min(Math.max(cur + d, 0), n - 1)), [n]);

  useEffect(() => {
    const prevFocus = document.activeElement as HTMLElement | null;
    const overflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      } else if (e.key === 'ArrowLeft') go(-1);
      else if (e.key === 'ArrowRight') go(1);
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      document.body.style.overflow = overflow;
      prevFocus?.focus();
    };
  }, [go, onClose]);

  const p = photos[i];
  if (!p) return null;

  const onTouchStart = (e: TouchEvent) => {
    const t0 = e.touches[0];
    touch.current = t0 ? { x: t0.clientX, y: t0.clientY } : null;
  };
  const onTouchEnd = (e: TouchEvent) => {
    const s = touch.current;
    touch.current = null;
    const t1 = e.changedTouches[0];
    if (!s || !t1) return;
    const dx = t1.clientX - s.x;
    const dy = t1.clientY - s.y;
    if (Math.abs(dx) > 40 && Math.abs(dx) > Math.abs(dy) * 1.5) go(dx < 0 ? 1 : -1);
    else if (dy > 90 && Math.abs(dy) > Math.abs(dx) * 1.5) onClose();
  };

  return (
    // Tapping the dark backdrop closes it; Escape and the close button do the same.
    // eslint-disable-next-line jsx-a11y/click-events-have-key-events, jsx-a11y/no-noninteractive-element-interactions
    <div
      className="photo-viewer"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      data-no-swipe-back
      data-no-swipe-next
      onClick={onClose}
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
    >
      <div className="photo-viewer-bar">
        <span className="photo-viewer-count">
          {title} · {t('areaPhotos.counter', { n: i + 1, count: n })}
        </span>
        <button
          ref={closeRef}
          type="button"
          className="photo-viewer-btn"
          onClick={(e) => {
            e.stopPropagation();
            onClose();
          }}
          aria-label={t('close')}
        >
          <X size={22} aria-hidden="true" />
        </button>
      </div>
      {/* eslint-disable-next-line jsx-a11y/click-events-have-key-events, jsx-a11y/no-noninteractive-element-interactions */}
      <img
        key={p.id}
        className="photo-viewer-img"
        src={p.url}
        alt={p.caption || title}
        onClick={(e) => e.stopPropagation()}
      />
      {p.caption && <p className="photo-viewer-caption">{p.caption}</p>}
      {n > 1 && (
        <>
          <button
            type="button"
            className="photo-viewer-btn photo-viewer-prev"
            disabled={i === 0}
            onClick={(e) => {
              e.stopPropagation();
              go(-1);
            }}
            aria-label={t('areaPhotos.prev')}
          >
            <ChevronLeft size={26} aria-hidden="true" />
          </button>
          <button
            type="button"
            className="photo-viewer-btn photo-viewer-next"
            disabled={i === n - 1}
            onClick={(e) => {
              e.stopPropagation();
              go(1);
            }}
            aria-label={t('areaPhotos.next')}
          >
            <ChevronRight size={26} aria-hidden="true" />
          </button>
        </>
      )}
    </div>
  );
}
