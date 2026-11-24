import type { ComponentType } from 'react';
import {
  AVATAR_BG_SHAPES,
  AVATAR_BG_SHAPE_PATHS,
  AVATAR_SHAPES,
  AVATAR_SHAPE_PATHS,
  type AvatarBgShape,
  type AvatarShape,
  GrokBotSvg,
  MuseBotSvg,
} from './bloubAvatars';
import {
  encodeGrokBotIcon,
  type GrokBotSpec,
  isGrokBotIcon,
  parseGrokBotIcon,
} from './grokBotStudio';

export {
  AVATAR_BG_SHAPES,
  AVATAR_BG_SHAPE_PATHS,
  AVATAR_SHAPES,
  AVATAR_SHAPE_PATHS,
  type AvatarBgShape,
  type AvatarShape,
  encodeGrokBotIcon,
  type GrokBotSpec,
  GrokBotSvg,
  isGrokBotIcon,
  MuseBotSvg,
  parseGrokBotIcon,
};

/** Avatar background colours. Keys match the API's fixed list; each maps to a `.avatar.av-<key>` class. */
export const AVATAR_COLORS = ['purple', 'teal', 'coral', 'pink', 'blue', 'green', 'amber', 'gray', 'white'] as const;
export type AvatarColor = (typeof AVATAR_COLORS)[number];

export const ANIMATED_AVATARS = [
  { key: 'grace_car', label: 'Grace Cruiser', defaultColor: 'blue', className: 'avatar-grace-car' },
  { key: 'peace_dove', label: 'Peace Dove', defaultColor: 'teal', className: 'avatar-peace-dove' },
  { key: 'sunny_cloud', label: 'Sunny Smile', defaultColor: 'amber', className: 'avatar-sunny-cloud' },
  { key: 'happy_bear', label: 'Cuddly Bear', defaultColor: 'coral', className: 'avatar-happy-bear' },
  { key: 'fellowship_cat', label: 'Mochi Cat', defaultColor: 'pink', className: 'avatar-fellowship-cat' },
  { key: 'joy_puppy', label: 'Joy Puppy', defaultColor: 'green', className: 'avatar-joy-puppy' },
  { key: 'star_lantern', label: 'Starry Guide', defaultColor: 'purple', className: 'avatar-star-lantern' },
  { key: 'blossom_sprout', label: 'Grace Blossom', defaultColor: 'pink', className: 'avatar-blossom-sprout' },
  { key: 'grok', label: 'Grok Bot', defaultColor: 'gray', className: 'avatar-grok-bot' },
  { key: 'muse', label: 'Muse Bot', defaultColor: 'amber', className: 'avatar-muse-bot' },
] as const satisfies readonly { key: string; label: string; defaultColor: AvatarColor; className: string }[];

export type AnimatedAvatarKey = (typeof ANIMATED_AVATARS)[number]['key'];

export const DEFAULT_ANIMATED_AVATAR: AnimatedAvatarKey = 'grace_car';

export const STATIC_AVATAR_ICON_KEYS = [
  'car',
  'heart',
  'star',
  'music',
  'sun',
  'leaf',
  'coffee',
  'book',
  'church',
  'bike',
  'smile',
  'flower',
] as const;

export const isAvatarColor = (c?: string | null): c is AvatarColor =>
  !!c && (AVATAR_COLORS as readonly string[]).includes(c);

export const isAvatarShape = (s?: string | null): s is AvatarShape =>
  !!s && (AVATAR_SHAPES as readonly string[]).includes(s);

export const isAvatarBgShape = (s?: string | null): s is AvatarBgShape =>
  !!s && (AVATAR_BG_SHAPES as readonly string[]).includes(s);

export const defaultAvatarShape = (icon?: string | null): AvatarShape =>
  parseAvatarIcon(icon).icon === 'muse' ? 'cloud' : 'hexagon';

export const isShapeableAvatar = (icon?: string | null): boolean => {
  const base = parseAvatarIcon(icon).icon;
  return base === 'grok' || base === 'muse';
};

export function parseAvatarColor(raw?: string | null): {
  bgColor: AvatarColor | null;
  bgShape: AvatarBgShape;
} {
  if (!raw) return { bgColor: null, bgShape: 'circle' };
  const [cRaw, shRaw] = raw.split(':');
  return {
    bgColor: isAvatarColor(cRaw) ? cRaw : null,
    bgShape: isAvatarBgShape(shRaw) ? shRaw : 'circle',
  };
}

export function encodeAvatarColor(bgColor: AvatarColor | null, bgShape: AvatarBgShape): string {
  const c = bgColor ?? '';
  if (bgShape === 'circle') return c;
  return `${c}:${bgShape}`;
}

export function parseAvatarIcon(raw?: string | null): {
  icon: string | null;
  avatarShape: AvatarShape | null;
  fgColor: AvatarColor | null;
} {
  if (!raw) return { icon: null, avatarShape: null, fgColor: null };
  const [icRaw, shRaw, fgRaw] = raw.split(':');
  return {
    icon: icRaw || null,
    avatarShape: isAvatarShape(shRaw) ? shRaw : null,
    fgColor: isAvatarColor(fgRaw) ? fgRaw : null,
  };
}

export function encodeAvatarIcon(
  icon: string | null,
  avatarShape: AvatarShape | null,
  fgColor: AvatarColor | null,
): string {
  if (!icon) {
    return fgColor ? `initials::${fgColor}` : '';
  }
  const shapePart =
    isShapeableAvatar(icon) && avatarShape && avatarShape !== defaultAvatarShape(icon)
      ? avatarShape
      : '';
  if (!shapePart && !fgColor) return icon;
  if (shapePart && !fgColor) return `${icon}:${shapePart}`;
  return `${icon}:${shapePart}:${fgColor ?? ''}`;
}

export const isAnimatedAvatar = (icon?: string | null): icon is AnimatedAvatarKey => {
  const base = parseAvatarIcon(icon).icon;
  return !!base && (ANIMATED_AVATARS as readonly { key: string }[]).some((a) => a.key === base);
};

export const getAnimatedAvatarMeta = (icon?: string | null) => {
  const base = parseAvatarIcon(icon).icon;
  return ANIMATED_AVATARS.find((a) => a.key === base);
};

export function avatarIconClass(icon?: string | null): string {
  const base = parseAvatarIcon(icon).icon;
  const anim = getAnimatedAvatarMeta(base);
  if (anim) return anim.className;
  return base ? `avatar-${base}` : '';
}

export function avatarGifUrl(icon: string, color?: string | null): string {
  const base = parseAvatarIcon(icon).icon ?? icon;
  const bg = parseAvatarColor(color).bgColor ?? (isAvatarColor(color) ? color : null);
  if (base === 'grok') {
    const c = bg ?? 'gray';
    return `/icons/avatars/grok_${c}.gif`;
  }
  if (base === 'muse') {
    const c = bg ?? 'amber';
    return `/icons/avatars/muse_${c}.gif`;
  }
  return `/icons/avatars/${base}.gif`;
}

export interface AvatarGifProps {
  className?: string;
  color?: string | null;
  shape?: AvatarShape;
}

function makeAvatarGifComponent(key: string): ComponentType<AvatarGifProps> {
  return function AvatarGif({ className = '', color }: AvatarGifProps) {
    const cls = ['avatar-animated-icon', avatarIconClass(key), 'avatar-gif', className].filter(Boolean).join(' ');
    return <img src={avatarGifUrl(key, color)} alt="" className={cls} aria-hidden="true" decoding="async" />;
  };
}

/** Avatar icons use animated GIFs by default, keeping Bloub SVGs only for non-default custom shapes on Grok & Muse. */
export const AVATAR_ICONS: Record<string, ComponentType<AvatarGifProps>> = {
  car: makeAvatarGifComponent('car'),
  heart: makeAvatarGifComponent('heart'),
  star: makeAvatarGifComponent('star'),
  music: makeAvatarGifComponent('music'),
  sun: makeAvatarGifComponent('sun'),
  leaf: makeAvatarGifComponent('leaf'),
  coffee: makeAvatarGifComponent('coffee'),
  book: makeAvatarGifComponent('book'),
  church: makeAvatarGifComponent('church'),
  bike: makeAvatarGifComponent('bike'),
  smile: makeAvatarGifComponent('smile'),
  flower: makeAvatarGifComponent('flower'),
  grace_car: makeAvatarGifComponent('grace_car'),
  peace_dove: makeAvatarGifComponent('peace_dove'),
  sunny_cloud: makeAvatarGifComponent('sunny_cloud'),
  happy_bear: makeAvatarGifComponent('happy_bear'),
  fellowship_cat: makeAvatarGifComponent('fellowship_cat'),
  joy_puppy: makeAvatarGifComponent('joy_puppy'),
  star_lantern: makeAvatarGifComponent('star_lantern'),
  blossom_sprout: makeAvatarGifComponent('blossom_sprout'),
  grok: ({ className = '', color, shape }: AvatarGifProps) =>
    shape && shape !== 'hexagon' ? (
      <GrokBotSvg className={className} shape={shape} />
    ) : (
      <img
        src={avatarGifUrl('grok', color)}
        alt=""
        className={['avatar-animated-icon', 'avatar-grok-bot', 'avatar-gif', className].filter(Boolean).join(' ')}
        aria-hidden="true"
        decoding="async"
      />
    ),
  muse: ({ className = '', color, shape }: AvatarGifProps) =>
    shape && shape !== 'cloud' ? (
      <MuseBotSvg className={className} shape={shape} />
    ) : (
      <img
        src={avatarGifUrl('muse', color)}
        alt=""
        className={['avatar-animated-icon', 'avatar-muse-bot', 'avatar-gif', className].filter(Boolean).join(' ')}
        aria-hidden="true"
        decoding="async"
      />
    ),
};

/** The avatar fields a member record may carry. */
export interface AvatarFields {
  photo_file_id?: number | null;
  avatar_color?: string | null;
  avatar_icon?: string | null;
}
