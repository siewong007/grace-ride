import type { ReactNode } from 'react';
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
import type { Badge } from './types/domain';

export const ACHIEVEMENT_ICONS: Record<string, { label: string; icon: LucideIcon }> = {
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
 * - If a registered Lucide icon key is specified (e.g. "moon", "award", "trophy"), renders the Lucide icon.
 * - If a custom short string/emoji is given (e.g. "🦉", "🏆", "🌟"), renders it as text.
 * - Otherwise falls back to the badge character (e.g. "N", "R", "C").
 */
export function BadgeIcon({
  badge,
  icon,
  fallbackChar,
  size = 24,
  className = '',
}: BadgeIconProps): ReactNode {
  const chosenKey = icon ?? badge?.icon;
  if (chosenKey && ACHIEVEMENT_ICONS[chosenKey]) {
    const IconComponent = ACHIEVEMENT_ICONS[chosenKey].icon;
    return <IconComponent size={size} className={className} aria-hidden="true" />;
  }
  if (chosenKey && chosenKey.trim()) {
    return <span className={className}>{chosenKey.trim()}</span>;
  }
  return <span className={className}>{fallbackChar ?? badge?.char ?? '★'}</span>;
}
