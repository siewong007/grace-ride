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

/** Avatar background colours. Keys match the API's fixed list; each maps to a `.avatar.av-<key>` class. */
export const AVATAR_COLORS = ['purple', 'teal', 'coral', 'pink', 'blue', 'green', 'amber', 'gray'] as const;
export type AvatarColor = (typeof AVATAR_COLORS)[number];

/** Avatar icons. Keys match the API's fixed list. */
export const AVATAR_ICONS: Record<string, LucideIcon> = {
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
};

/** The avatar fields a member record may carry. */
export interface AvatarFields {
  photo_file_id?: number | null;
  avatar_color?: string | null;
  avatar_icon?: string | null;
}

export const isAvatarColor = (c?: string | null): c is AvatarColor =>
  !!c && (AVATAR_COLORS as readonly string[]).includes(c);
