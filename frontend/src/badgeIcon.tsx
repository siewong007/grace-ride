import type { ReactNode } from 'react';
import type { Badge, BadgeTier, BadgeTierStep, EarnedBadgeTier } from './types/domain';

export type { BadgeTier, BadgeTierStep, EarnedBadgeTier };
export type BadgeCategory = 'member' | 'driver' | 'traffic_lead';

export const BADGE_TIERS: readonly EarnedBadgeTier[] = ['bronze', 'silver', 'gold', 'platinum'];

const ALL_BADGE_TIERS: readonly BadgeTier[] = ['none', ...BADGE_TIERS];

export const resolveBadgeTier = (badge?: Partial<Badge>): BadgeTier => {
  if (badge?.tier && (ALL_BADGE_TIERS as readonly string[]).includes(badge.tier)) {
    return badge.tier;
  }
  return badge?.earned ? 'bronze' : 'none';
};

export const BADGE_TIER_ROMAN: Record<EarnedBadgeTier, string> = {
  bronze: 'I',
  silver: 'II',
  gold: 'III',
  platinum: 'IV',
};

export const badgeGifUrl = (key: string): string => `/icons/badges/${key}.gif`;

export interface AnimatedBadgeMeta {
  key: string;
  label: string;
  category: BadgeCategory;
  className: string;
  gif: string;
}

export const ANIMATED_BADGE_ICONS: readonly AnimatedBadgeMeta[] = [
  // Normal Member
  { key: 'badge_joy_rider', label: 'Joyful Rider', category: 'member', className: 'badge-joy-rider', gif: badgeGifUrl('badge_joy_rider') },
  { key: 'badge_fellowship_sprout', label: 'Faithful Attendee', category: 'member', className: 'badge-fellowship-sprout', gif: badgeGifUrl('badge_fellowship_sprout') },
  { key: 'badge_punctual_clock', label: 'Ready & Punctual', category: 'member', className: 'badge-punctual-clock', gif: badgeGifUrl('badge_punctual_clock') },
  { key: 'badge_warm_welcome', label: 'Caring Helper', category: 'member', className: 'badge-warm-welcome', gif: badgeGifUrl('badge_warm_welcome') },
  // Driver
  { key: 'moon', label: 'Night Owl', category: 'driver', className: 'badge-night-owl', gif: badgeGifUrl('moon') },
  { key: 'award', label: 'Resident Driver', category: 'driver', className: 'badge-resident-driver', gif: badgeGifUrl('award') },
  { key: 'heart', label: 'Caring Companion', category: 'driver', className: 'badge-carpool-love', gif: badgeGifUrl('heart') },
  { key: 'badge_helping_wheels', label: 'Helping Wheels', category: 'driver', className: 'badge-helping-wheels', gif: badgeGifUrl('badge_helping_wheels') },
  // Traffic Lead
  { key: 'badge_traffic_light', label: 'Green-Light Captain', category: 'traffic_lead', className: 'badge-traffic-light', gif: badgeGifUrl('badge_traffic_light') },
  { key: 'badge_route_compass', label: 'Master Route Planner', category: 'traffic_lead', className: 'badge-route-compass', gif: badgeGifUrl('badge_route_compass') },
  { key: 'badge_traffic_whistle', label: 'Smooth Coordinator', category: 'traffic_lead', className: 'badge-traffic-whistle', gif: badgeGifUrl('badge_traffic_whistle') },
  { key: 'badge_care_shield', label: 'Care Steward', category: 'traffic_lead', className: 'badge-care-shield', gif: badgeGifUrl('badge_care_shield') },
] as const;

const ANIMATED_BADGE_MAP: Record<string, AnimatedBadgeMeta> = Object.fromEntries(
  ANIMATED_BADGE_ICONS.map((item) => [item.key, item]),
);

export const isAnimatedBadgeIcon = (key?: string | null): boolean =>
  !!key && key in ANIMATED_BADGE_MAP;

export const ACHIEVEMENT_ICONS: Record<
  string,
  { label: string; gif: string; animated?: boolean }
> = {
  award: { label: 'Award', gif: badgeGifUrl('award'), animated: true },
  trophy: { label: 'Trophy', gif: badgeGifUrl('trophy') },
  medal: { label: 'Medal', gif: badgeGifUrl('medal') },
  moon: { label: 'Moon', gif: badgeGifUrl('moon'), animated: true },
  star: { label: 'Star', gif: badgeGifUrl('star') },
  heart: { label: 'Heart', gif: badgeGifUrl('heart'), animated: true },
  shield: { label: 'Shield', gif: badgeGifUrl('shield') },
  crown: { label: 'Crown', gif: badgeGifUrl('crown') },
  sparkles: { label: 'Sparkles', gif: badgeGifUrl('sparkles') },
  flame: { label: 'Flame', gif: badgeGifUrl('flame') },
  zap: { label: 'Lightning', gif: badgeGifUrl('zap') },
  car: { label: 'Car', gif: badgeGifUrl('car') },
  clock: { label: 'Clock', gif: badgeGifUrl('clock') },
  target: { label: 'Target', gif: badgeGifUrl('target') },
  sun: { label: 'Sun', gif: badgeGifUrl('sun') },
  coffee: { label: 'Coffee', gif: badgeGifUrl('coffee') },
  gift: { label: 'Gift', gif: badgeGifUrl('gift') },
  smile: { label: 'Smile', gif: badgeGifUrl('smile') },
  thumbsUp: { label: 'Thumbs Up', gif: badgeGifUrl('thumbsUp') },
  compass: { label: 'Compass', gif: badgeGifUrl('compass') },
  badge_joy_rider: { label: 'Joyful Rider', gif: badgeGifUrl('badge_joy_rider'), animated: true },
  badge_fellowship_sprout: { label: 'Faithful Attendee', gif: badgeGifUrl('badge_fellowship_sprout'), animated: true },
  badge_punctual_clock: { label: 'Ready & Punctual', gif: badgeGifUrl('badge_punctual_clock'), animated: true },
  badge_warm_welcome: { label: 'Caring Helper', gif: badgeGifUrl('badge_warm_welcome'), animated: true },
  badge_helping_wheels: { label: 'Helping Wheels', gif: badgeGifUrl('badge_helping_wheels'), animated: true },
  badge_traffic_light: { label: 'Green-Light Captain', gif: badgeGifUrl('badge_traffic_light'), animated: true },
  badge_route_compass: { label: 'Master Route Planner', gif: badgeGifUrl('badge_route_compass'), animated: true },
  badge_traffic_whistle: { label: 'Smooth Coordinator', gif: badgeGifUrl('badge_traffic_whistle'), animated: true },
  badge_care_shield: { label: 'Care Steward', gif: badgeGifUrl('badge_care_shield'), animated: true },
};

export interface BadgeIconProps {
  badge?: Partial<Badge>;
  icon?: string | null;
  fallbackChar?: string;
  size?: number;
  className?: string;
}

/**
 * Renders an achievement icon as a high-quality Bloub-style animated GIF:
 * - Registered role badges & classic achievement icon keys render their animated GIF.
 * - Custom short string/emoji renders as text.
 * - Otherwise falls back to the badge character.
 */
export function BadgeIcon({
  badge,
  icon,
  fallbackChar,
  size = 40,
  className = '',
}: BadgeIconProps): ReactNode {
  const chosenKey = (icon ?? badge?.icon)?.trim();
  if (chosenKey && (ANIMATED_BADGE_MAP[chosenKey] || Object.prototype.hasOwnProperty.call(ACHIEVEMENT_ICONS, chosenKey))) {
    const animMeta = ANIMATED_BADGE_MAP[chosenKey];
    const cls = ['badge-gif', animMeta ? 'badge-animated-svg' : '', animMeta?.className ?? '', className]
      .filter(Boolean)
      .join(' ');
    return (
      <img
        src={badgeGifUrl(chosenKey)}
        alt=""
        width={size}
        height={size}
        className={cls}
        aria-hidden="true"
        decoding="async"
      />
    );
  }
  if (chosenKey) {
    return <span className={className}>{chosenKey}</span>;
  }
  return <span className={className}>{fallbackChar ?? badge?.char ?? '★'}</span>;
}
