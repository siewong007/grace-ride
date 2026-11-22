import type { SVGProps } from 'react';
import {
  Bike,
  BookOpen,
  Car,
  Church,
  Coffee,
  Flower2,
  Heart,
  Leaf,
  type LucideIcon,
  Music,
  Smile,
  Star,
  Sun,
} from 'lucide-react';

/** Animated Grok Bot Logo avatar */
export function GrokIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="avatar-animated-icon avatar-grok-bot"
      aria-hidden="true"
      {...props}
    >
      {/* Bot antenna with pulsing node */}
      <path d="M24 10V4" className="grok-antenna" />
      <circle cx="24" cy="3" r="2" fill="currentColor" className="grok-antenna-pulse" />
      <path d="M14 12l-4-4" />
      <path d="M34 12l4-4" />

      {/* Cyber robotic head chassis */}
      <rect x="10" y="10" width="28" height="24" rx="7" className="grok-head" />

      {/* Visor screen */}
      <rect x="14" y="16" width="20" height="12" rx="3.5" fill="currentColor" fillOpacity="0.15" />

      {/* Visor eyes that pulse and blink */}
      <circle cx="19" cy="22" r="2.5" fill="currentColor" className="grok-eye grok-eye-left" />
      <circle cx="29" cy="22" r="2.5" fill="currentColor" className="grok-eye grok-eye-right" />

      {/* Grok diagonal accent slash */}
      <path d="M21 28l6-4" className="grok-slash" />

      {/* Neck base */}
      <path d="M20 34v4h8v-4" />
      <path d="M16 42h16" />
    </svg>
  );
}

/** Animated Muse Bot Logo avatar */
export function MuseIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="avatar-animated-icon avatar-muse-bot"
      aria-hidden="true"
      {...props}
    >
      {/* Dynamic soundwave aura rings */}
      <path d="M7 21c-1.5 2-1.5 6 0 8" className="muse-wave muse-wave-1" />
      <path d="M4 18c-2.5 3.5-2.5 10 0 14" className="muse-wave muse-wave-2" />
      <path d="M41 21c1.5 2 1.5 6 0 8" className="muse-wave muse-wave-1" />
      <path d="M44 18c2.5 3.5 2.5 10 0 14" className="muse-wave muse-wave-2" />

      {/* Friendly bot head */}
      <rect x="10" y="11" width="28" height="26" rx="13" className="muse-head" />

      {/* Ear nodes */}
      <circle cx="10" cy="24" r="2" fill="currentColor" />
      <circle cx="38" cy="24" r="2" fill="currentColor" />

      {/* Animated eyes */}
      <ellipse cx="19" cy="22" rx="2.5" ry="3.5" fill="currentColor" className="muse-eye muse-eye-left" />
      <ellipse cx="29" cy="22" rx="2.5" ry="3.5" fill="currentColor" className="muse-eye muse-eye-right" />

      {/* Animated smiling waveform mouth */}
      <path d="M20 29c1.2 1.5 6.8 1.5 8 0" className="muse-smile" />

      {/* Sparkle antenna */}
      <path d="M24 11V5" />
      <circle cx="24" cy="4" r="2.5" fill="currentColor" className="muse-sparkle" />
    </svg>
  );
}

/** Avatar background colours. Keys match the API's fixed list; each maps to a `.avatar.av-<key>` class. */
export const AVATAR_COLORS = ['purple', 'teal', 'coral', 'pink', 'blue', 'green', 'amber', 'gray'] as const;
export type AvatarColor = (typeof AVATAR_COLORS)[number];

export const ANIMATED_AVATARS = [
  { key: 'grok', label: 'Grok Bot' },
  { key: 'muse', label: 'Muse Bot' },
] as const;
export type AnimatedAvatarKey = (typeof ANIMATED_AVATARS)[number]['key'];

export const isAnimatedAvatar = (icon?: string | null): icon is AnimatedAvatarKey =>
  !!icon && (ANIMATED_AVATARS as readonly { key: string }[]).some((a) => a.key === icon);

/** Avatar icons. Keys match the API's fixed list. */
export const AVATAR_ICONS: Record<string, LucideIcon | React.ComponentType<{ className?: string }>> = {
  car: Car,
  heart: Heart,
  star: Star,
  music: Music,
  sun: Sun,
  leaf: Leaf,
  coffee: Coffee,
  book: BookOpen,
  church: Church,
  bike: Bike,
  smile: Smile,
  flower: Flower2,
  grok: GrokIcon,
  muse: MuseIcon,
};

/** The avatar fields a member record may carry. */
export interface AvatarFields {
  photo_file_id?: number | null;
  avatar_color?: string | null;
  avatar_icon?: string | null;
}

export const isAvatarColor = (c?: string | null): c is AvatarColor =>
  !!c && (AVATAR_COLORS as readonly string[]).includes(c);
