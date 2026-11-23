import type { ComponentType, ReactNode, SVGProps } from 'react';
import {
  Award,
  Car,
  Clock,
  Coffee,
  Compass,
  Crown,
  Flame,
  Gift,
  Heart,
  Medal,
  Moon,
  Shield,
  Smile,
  Sparkles,
  Star,
  Sun,
  Target,
  ThumbsUp,
  Trophy,
  Zap,
  type LucideIcon,
} from 'lucide-react';
import {
  BadgeCareShield,
  BadgeCarpoolLove,
  BadgeFellowshipSprout,
  BadgeHelpingWheels,
  BadgeJoyRider,
  BadgeNightOwl,
  BadgePunctualClock,
  BadgeResidentDriver,
  BadgeRouteCompass,
  BadgeTrafficLight,
  BadgeTrafficWhistle,
  BadgeWarmWelcome,
} from './animatedBadges';
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

export interface AnimatedBadgeMeta {
  key: string;
  label: string;
  category: BadgeCategory;
  component: ComponentType<SVGProps<SVGSVGElement>>;
}

export const ANIMATED_BADGE_ICONS: readonly AnimatedBadgeMeta[] = [
  // Normal Member
  { key: 'badge_joy_rider', label: 'Joyful Rider', category: 'member', component: BadgeJoyRider },
  { key: 'badge_fellowship_sprout', label: 'Faithful Attendee', category: 'member', component: BadgeFellowshipSprout },
  { key: 'badge_punctual_clock', label: 'Ready & Punctual', category: 'member', component: BadgePunctualClock },
  { key: 'badge_warm_welcome', label: 'Caring Helper', category: 'member', component: BadgeWarmWelcome },
  // Driver
  { key: 'moon', label: 'Night Owl', category: 'driver', component: BadgeNightOwl },
  { key: 'award', label: 'Resident Driver', category: 'driver', component: BadgeResidentDriver },
  { key: 'heart', label: 'Caring Companion', category: 'driver', component: BadgeCarpoolLove },
  { key: 'badge_helping_wheels', label: 'Helping Wheels', category: 'driver', component: BadgeHelpingWheels },
  // Traffic Lead
  { key: 'badge_traffic_light', label: 'Green-Light Captain', category: 'traffic_lead', component: BadgeTrafficLight },
  { key: 'badge_route_compass', label: 'Master Route Planner', category: 'traffic_lead', component: BadgeRouteCompass },
  { key: 'badge_traffic_whistle', label: 'Smooth Coordinator', category: 'traffic_lead', component: BadgeTrafficWhistle },
  { key: 'badge_care_shield', label: 'Care Steward', category: 'traffic_lead', component: BadgeCareShield },
] as const;

const ANIMATED_BADGE_MAP: Record<string, AnimatedBadgeMeta> = Object.fromEntries(
  ANIMATED_BADGE_ICONS.map((item) => [item.key, item]),
);

export const isAnimatedBadgeIcon = (key?: string | null): boolean =>
  !!key && key in ANIMATED_BADGE_MAP;

export const ACHIEVEMENT_ICONS: Record<
  string,
  { label: string; icon: LucideIcon | ComponentType<SVGProps<SVGSVGElement>>; animated?: boolean }
> = {
  award: { label: 'Award', icon: Award },
  trophy: { label: 'Trophy', icon: Trophy },
  medal: { label: 'Medal', icon: Medal },
  moon: { label: 'Moon', icon: Moon },
  star: { label: 'Star', icon: Star },
  heart: { label: 'Heart', icon: Heart },
  shield: { label: 'Shield', icon: Shield },
  crown: { label: 'Crown', icon: Crown },
  sparkles: { label: 'Sparkles', icon: Sparkles },
  flame: { label: 'Flame', icon: Flame },
  zap: { label: 'Lightning', icon: Zap },
  car: { label: 'Car', icon: Car },
  clock: { label: 'Clock', icon: Clock },
  target: { label: 'Target', icon: Target },
  sun: { label: 'Sun', icon: Sun },
  coffee: { label: 'Coffee', icon: Coffee },
  gift: { label: 'Gift', icon: Gift },
  smile: { label: 'Smile', icon: Smile },
  thumbsUp: { label: 'Thumbs Up', icon: ThumbsUp },
  compass: { label: 'Compass', icon: Compass },
  badge_joy_rider: { label: 'Joyful Rider', icon: BadgeJoyRider, animated: true },
  badge_fellowship_sprout: { label: 'Faithful Attendee', icon: BadgeFellowshipSprout, animated: true },
  badge_punctual_clock: { label: 'Ready & Punctual', icon: BadgePunctualClock, animated: true },
  badge_warm_welcome: { label: 'Caring Helper', icon: BadgeWarmWelcome, animated: true },
  badge_helping_wheels: { label: 'Helping Wheels', icon: BadgeHelpingWheels, animated: true },
  badge_traffic_light: { label: 'Green-Light Captain', icon: BadgeTrafficLight, animated: true },
  badge_route_compass: { label: 'Master Route Planner', icon: BadgeRouteCompass, animated: true },
  badge_traffic_whistle: { label: 'Smooth Coordinator', icon: BadgeTrafficWhistle, animated: true },
  badge_care_shield: { label: 'Care Steward', icon: BadgeCareShield, animated: true },
};

export interface BadgeIconProps {
  badge?: Partial<Badge>;
  icon?: string | null;
  fallbackChar?: string;
  size?: number;
  className?: string;
}

/**
 * Renders an achievement icon:
 * - If an adorable animated badge key is specified (e.g. "moon", "badge_joy_rider", "badge_traffic_light"),
 *   renders the multi-coloured animated SVG badge.
 * - If a registered Lucide icon key is specified (e.g. "trophy", "star"), renders the Lucide icon.
 * - If a custom short string/emoji is given, renders it as text.
 * - Otherwise falls back to the badge character.
 */
export function BadgeIcon({
  badge,
  icon,
  fallbackChar,
  size = 24,
  className = '',
}: BadgeIconProps): ReactNode {
  const chosenKey = icon ?? badge?.icon;
  if (chosenKey && ANIMATED_BADGE_MAP[chosenKey]) {
    const AnimatedComponent = ANIMATED_BADGE_MAP[chosenKey].component;
    return <AnimatedComponent className={className} aria-hidden="true" />;
  }
  if (chosenKey && ACHIEVEMENT_ICONS[chosenKey]) {
    const IconComponent = ACHIEVEMENT_ICONS[chosenKey].icon as LucideIcon;
    return <IconComponent size={size} className={className} aria-hidden="true" />;
  }
  if (chosenKey && chosenKey.trim()) {
    return <span className={className}>{chosenKey.trim()}</span>;
  }
  return <span className={className}>{fallbackChar ?? badge?.char ?? '★'}</span>;
}
