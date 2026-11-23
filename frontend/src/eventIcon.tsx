import type { ReactNode } from 'react';

export const eventIconGifUrl = (key: string): string => `/icons/events/${key}.gif`;

export const EVENT_ICONS: Record<string, { gif: string }> = {
  church: { gif: eventIconGifUrl('church') },
  bookOpen: { gif: eventIconGifUrl('bookOpen') },
  music: { gif: eventIconGifUrl('music') },
  flame: { gif: eventIconGifUrl('flame') },
  heartHandshake: { gif: eventIconGifUrl('heartHandshake') },
  sparkles: { gif: eventIconGifUrl('sparkles') },
  utensils: { gif: eventIconGifUrl('utensils') },
  coffee: { gif: eventIconGifUrl('coffee') },
  cake: { gif: eventIconGifUrl('cake') },
  users: { gif: eventIconGifUrl('users') },
  gift: { gif: eventIconGifUrl('gift') },
  home: { gif: eventIconGifUrl('home') },
  trophy: { gif: eventIconGifUrl('trophy') },
  footprints: { gif: eventIconGifUrl('footprints') },
  waves: { gif: eventIconGifUrl('waves') },
  bike: { gif: eventIconGifUrl('bike') },
  tent: { gif: eventIconGifUrl('tent') },
  mountain: { gif: eventIconGifUrl('mountain') },
  dumbbell: { gif: eventIconGifUrl('dumbbell') },
  target: { gif: eventIconGifUrl('target') },
  gamepad: { gif: eventIconGifUrl('gamepad') },
  film: { gif: eventIconGifUrl('film') },
  camera: { gif: eventIconGifUrl('camera') },
  bus: { gif: eventIconGifUrl('bus') },
  baby: { gif: eventIconGifUrl('baby') },
  calendar: { gif: eventIconGifUrl('calendar') },
  sun: { gif: eventIconGifUrl('sun') },
  moon: { gif: eventIconGifUrl('moon') },
};

export const DEFAULT_KIND_ICON: Record<string, string> = {
  badminton: 'trophy',
  bowling: 'target',
  kayak: 'waves',
  walk: 'footprints',
  other: 'sparkles',
};

export interface EventIconProps {
  icon?: string | null;
  kind?: string;
  size?: number;
  className?: string;
}

/**
 * Renders an activity's configured icon as an animated GIF:
 * - Preset icon key (e.g. "church", "utensils")
 * - Custom emoji or short symbol (e.g. "🍲")
 * - Default animated icon derived from the event kind
 */
export function EventIcon({ icon, kind, size = 20, className = '' }: EventIconProps): ReactNode {
  const trimmed = icon?.trim() ?? '';
  if (trimmed && Object.prototype.hasOwnProperty.call(EVENT_ICONS, trimmed)) {
    return (
      <img
        src={EVENT_ICONS[trimmed].gif}
        alt=""
        width={size}
        height={size}
        className={`event-icon-gif ${className}`.trim()}
        aria-hidden="true"
        decoding="async"
      />
    );
  }
  if (trimmed) {
    return (
      <span
        className={`event-icon-glyph ${className}`.trim()}
        style={{ fontSize: size }}
        aria-hidden="true"
      >
        {trimmed}
      </span>
    );
  }
  const fallbackKey = DEFAULT_KIND_ICON[kind ?? ''] ?? 'calendar';
  const gifUrl = EVENT_ICONS[fallbackKey]?.gif ?? eventIconGifUrl('calendar');
  return (
    <img
      src={gifUrl}
      alt=""
      width={size}
      height={size}
      className={`event-icon-gif ${className}`.trim()}
      aria-hidden="true"
      decoding="async"
    />
  );
}
