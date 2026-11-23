import React, { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Tone } from '../../format';
import { LoadingMark } from '../Brand';
import {
  AVATAR_BG_SHAPE_PATHS,
  AVATAR_ICONS,
  avatarGifUrl,
  avatarIconClass,
  GrokBotSvg,
  isAnimatedAvatar,
  MuseBotSvg,
  parseAvatarColor,
  parseAvatarIcon,
} from '../../avatarStyle';

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
 * One message for every page the viewer may not open, with a way back.
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
  const wrapperClass = count === 1 ? 'stack' : 'grid cols-2';
  return (
    <div className={wrapperClass} aria-hidden="true">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="card stack">
          <Skeleton width="40%" height="1.2rem" />
          <Skeleton lines={3} height="0.9rem" />
        </div>
      ))}
    </div>
  );
}

export type PillTone =
  | 'neutral'
  | 'success'
  | 'warning'
  | 'danger'
  | 'info'
  | 'primary'
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

/** Loading indicator: the animated Grace Ride mark from the logo kit. */
export const Spinner = () => {
  const { t } = useTranslation();
  return (
    <div className="spinner" role="status" aria-label={t('loading')}>
      <LoadingMark />
    </div>
  );
};

export function ErrorBox({ error, onRetry }: { error?: string | null; onRetry?: () => void }) {
  const { t } = useTranslation();
  if (!error) return null;
  return (
    <div className="error error-box-row" role="alert">
      <span className="grow min-w-0">{error}</span>
      {onRetry && (
        <button type="button" className="btn sm secondary error-retry" onClick={onRetry}>
          {t('tryAgain')}
        </button>
      )}
    </div>
  );
}

export interface AvatarProps {
  name: string;
  fileId?: number | null;
  /** A key from `AVATAR_COLORS`; unknown keys fall back to the neutral background. */
  color?: string | null;
  /** A key from `AVATAR_ICONS`; shown instead of the initial when there is no photo. */
  icon?: string | null;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  /** A local preview image (object URL) that overrides `fileId`. */
  src?: string | null;
}

/** A member's picture: their photo, else their chosen icon/mascot, else their first initial, on their chosen colour and shape. */
export function Avatar({ name, fileId, color, icon, size, src }: AvatarProps) {
  const { bgColor, bgShape } = parseAvatarColor(color);
  const { icon: baseIcon, avatarShape, fgColor } = parseAvatarIcon(icon);
  const hasPresetIcon = !!(
    baseIcon &&
    baseIcon !== 'initials' &&
    Object.prototype.hasOwnProperty.call(AVATAR_ICONS, baseIcon)
  );
  const img = src ?? (fileId ? `/api/files/${fileId}` : null);
  const isAnim = isAnimatedAvatar(baseIcon) || hasPresetIcon;
  const hasSvgBg = !img && (bgShape === 'hexagon' || bgShape === 'cloud');
  const cls = [
    'avatar',
    size ?? '',
    bgColor ? `av-${bgColor}` : '',
    bgShape !== 'circle' ? `av-bg-${bgShape}` : '',
    fgColor ? `av-fg-${fgColor}` : '',
    isAnim ? 'avatar-animated' : '',
    hasSvgBg ? 'avatar-svg-bg' : '',
  ]
    .filter(Boolean)
    .join(' ');
  const iconCls = ['avatar-animated-icon', avatarIconClass(baseIcon), 'avatar-icon', 'avatar-gif']
    .filter(Boolean)
    .join(' ');
  return (
    <div className={cls} aria-hidden>
      {hasSvgBg && (
        <svg viewBox="0 0 48 48" fill="none" className="avatar-bg-svg" aria-hidden="true">
          <path d={AVATAR_BG_SHAPE_PATHS[bgShape]} />
        </svg>
      )}
      {img ? (
        <img src={img} alt="" />
      ) : baseIcon === 'grok' && avatarShape && avatarShape !== 'hexagon' ? (
        <GrokBotSvg shape={avatarShape} className="avatar-icon" />
      ) : baseIcon === 'muse' && avatarShape && avatarShape !== 'cloud' ? (
        <MuseBotSvg shape={avatarShape} className="avatar-icon" />
      ) : hasPresetIcon && baseIcon ? (
        <img src={avatarGifUrl(baseIcon, fgColor ?? bgColor)} alt="" className={iconCls} decoding="async" />
      ) : (
        <span className="avatar-initial">{Array.from(name.trim())[0] ?? '?'}</span>
      )}
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
