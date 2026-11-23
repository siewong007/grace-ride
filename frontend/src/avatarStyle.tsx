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
import {
  BlossomSproutAvatar,
  FellowshipCatAvatar,
  GraceCarAvatar,
  GrokIcon,
  HappyBearAvatar,
  JoyPuppyAvatar,
  MuseIcon,
  PeaceDoveAvatar,
  StarLanternAvatar,
  SunnyCloudAvatar,
} from './animatedAvatars';

export {
  BlossomSproutAvatar,
  FellowshipCatAvatar,
  GraceCarAvatar,
  GrokIcon,
  HappyBearAvatar,
  JoyPuppyAvatar,
  MuseIcon,
  PeaceDoveAvatar,
  StarLanternAvatar,
  SunnyCloudAvatar,
};

/** Avatar background colours. Keys match the API's fixed list; each maps to a `.avatar.av-<key>` class. */
export const AVATAR_COLORS = ['purple', 'teal', 'coral', 'pink', 'blue', 'green', 'amber', 'gray', 'white'] as const;
export type AvatarColor = (typeof AVATAR_COLORS)[number];

export const ANIMATED_AVATARS = [
  { key: 'grace_car', label: 'Grace Cruiser', defaultColor: 'blue' },
  { key: 'peace_dove', label: 'Peace Dove', defaultColor: 'teal' },
  { key: 'sunny_cloud', label: 'Sunny Smile', defaultColor: 'amber' },
  { key: 'happy_bear', label: 'Cuddly Bear', defaultColor: 'coral' },
  { key: 'fellowship_cat', label: 'Mochi Cat', defaultColor: 'pink' },
  { key: 'joy_puppy', label: 'Joy Puppy', defaultColor: 'green' },
  { key: 'star_lantern', label: 'Starry Guide', defaultColor: 'purple' },
  { key: 'blossom_sprout', label: 'Grace Blossom', defaultColor: 'pink' },
  { key: 'grok', label: 'Grok Bot', defaultColor: 'gray' },
  { key: 'muse', label: 'Muse Bot', defaultColor: 'amber' },
] as const satisfies readonly { key: string; label: string; defaultColor: AvatarColor }[];

export type AnimatedAvatarKey = (typeof ANIMATED_AVATARS)[number]['key'];

export const DEFAULT_ANIMATED_AVATAR: AnimatedAvatarKey = 'grace_car';

export const isAnimatedAvatar = (icon?: string | null): icon is AnimatedAvatarKey =>
  !!icon && (ANIMATED_AVATARS as readonly { key: string }[]).some((a) => a.key === icon);

export const getAnimatedAvatarMeta = (icon?: string | null) =>
  ANIMATED_AVATARS.find((a) => a.key === icon);

/** Avatar icons. Keys match the API's fixed list. */
export const AVATAR_ICONS: Record<
  string,
  LucideIcon | React.ComponentType<SVGProps<SVGSVGElement>>
> = {
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
  grace_car: GraceCarAvatar,
  peace_dove: PeaceDoveAvatar,
  sunny_cloud: SunnyCloudAvatar,
  happy_bear: HappyBearAvatar,
  fellowship_cat: FellowshipCatAvatar,
  joy_puppy: JoyPuppyAvatar,
  star_lantern: StarLanternAvatar,
  blossom_sprout: BlossomSproutAvatar,
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
