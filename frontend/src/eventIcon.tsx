import type { ReactNode } from 'react';
import {
  Baby,
  Bike,
  BookOpen,
  Bus,
  Cake,
  Calendar,
  Camera,
  Church,
  Coffee,
  Dumbbell,
  Film,
  Flame,
  Footprints,
  Gamepad2,
  Gift,
  HeartHandshake,
  Home,
  Moon,
  Mountain,
  Music,
  Sparkles,
  Sun,
  Target,
  Tent,
  Trophy,
  Users,
  Utensils,
  Waves,
  type LucideIcon,
} from 'lucide-react';

export const EVENT_ICONS: Record<string, { icon: LucideIcon }> = {
  church: { icon: Church },
  bookOpen: { icon: BookOpen },
  music: { icon: Music },
  flame: { icon: Flame },
  heartHandshake: { icon: HeartHandshake },
  sparkles: { icon: Sparkles },
  utensils: { icon: Utensils },
  coffee: { icon: Coffee },
  cake: { icon: Cake },
  users: { icon: Users },
  gift: { icon: Gift },
  home: { icon: Home },
  trophy: { icon: Trophy },
  footprints: { icon: Footprints },
  waves: { icon: Waves },
  bike: { icon: Bike },
  tent: { icon: Tent },
  mountain: { icon: Mountain },
  dumbbell: { icon: Dumbbell },
  target: { icon: Target },
  gamepad: { icon: Gamepad2 },
  film: { icon: Film },
  camera: { icon: Camera },
  bus: { icon: Bus },
  baby: { icon: Baby },
  calendar: { icon: Calendar },
  sun: { icon: Sun },
  moon: { icon: Moon },
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
 * Renders an activity's configured icon:
 * - Preset Lucide icon key (e.g. "church", "utensils")
 * - Custom emoji or short symbol (e.g. "🍲")
 * - Default icon derived from the event kind
 */
export function EventIcon({ icon, kind, size = 16, className = '' }: EventIconProps): ReactNode {
  const trimmed = icon?.trim() ?? '';
  if (trimmed && Object.prototype.hasOwnProperty.call(EVENT_ICONS, trimmed)) {
    const IconComp = EVENT_ICONS[trimmed].icon;
    return <IconComp size={size} className={className} aria-hidden="true" />;
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
  const fallbackKey = DEFAULT_KIND_ICON[kind ?? ''] ?? '';
  const FallbackComp = EVENT_ICONS[fallbackKey]?.icon ?? Calendar;
  return <FallbackComp size={size} className={className} aria-hidden="true" />;
}
