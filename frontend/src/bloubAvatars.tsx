export const AVATAR_SHAPES = [
  'hexagon',
  'circle',
  'pebble',
  'squircle',
  'capsule',
  'triangle',
  'cloud',
  'droplet',
] as const;
export type AvatarShape = (typeof AVATAR_SHAPES)[number];

export const AVATAR_BG_SHAPES = [
  'circle',
  'squircle',
  'rounded',
  'hexagon',
  'cloud',
  'pebble',
] as const;
export type AvatarBgShape = (typeof AVATAR_BG_SHAPES)[number];

export const AVATAR_SHAPE_PATHS: Record<AvatarShape, string> = {
  hexagon:
    'M 19.45 8.85 L 28.55 8.85 Q 32.75 8.85 34.85 12.49 L 39.4 20.36 Q 41.5 24 39.4 27.64 L 34.85 35.51 Q 32.75 39.15 28.55 39.15 L 19.45 39.15 Q 15.25 39.15 13.15 35.51 L 8.6 27.64 Q 6.5 24 8.6 20.36 L 13.15 12.49 Q 15.25 8.85 19.45 8.85 Z',
  circle: 'M 24 7.8 A 16.2 16.2 0 1 0 24 40.2 A 16.2 16.2 0 1 0 24 7.8 Z',
  pebble:
    'M 23.5 8.2 C 33 7.8 40.2 13.8 40.4 22.8 C 40.6 32.2 34.2 39.6 24.4 39.6 C 14.2 39.6 7.8 33.4 7.8 24.2 C 7.8 14.8 14.4 8.6 23.5 8.2 Z',
  squircle:
    'M 18.5 8 L 29.5 8 C 36.4 8 40 11.6 40 18.5 L 40 29.5 C 40 36.4 36.4 40 29.5 40 L 18.5 40 C 11.6 40 8 36.4 8 29.5 L 8 18.5 C 8 11.6 11.6 8 18.5 8 Z',
  capsule:
    'M 18.5 11.5 L 29.5 11.5 A 12.5 12.5 0 0 1 29.5 36.5 L 18.5 36.5 A 12.5 12.5 0 0 1 18.5 11.5 Z',
  triangle:
    'M 19.8 12.5 Q 24 6.2 28.2 12.5 L 39.2 31.8 Q 42.4 38.4 35.2 38.4 L 12.8 38.4 Q 5.6 38.4 8.8 31.8 Z',
  cloud:
    'M 24 10.6 C 26.8 8.2 32.8 9.2 34.4 13.4 C 39.6 15.2 41 21.8 39.4 27.6 C 38.2 34.4 33.4 39 24 39 C 14.6 39 9.8 34.4 8.6 27.6 C 7 21.8 8.4 15.2 13.6 13.4 C 15.2 9.2 21.2 8.2 24 10.6 Z',
  droplet:
    'M 24 7.5 C 26.6 7.5 39.6 18.8 39.6 26.8 C 39.6 34.8 32.8 40 24 40 C 15.2 40 8.4 34.8 8.4 26.8 C 8.4 18.8 21.4 7.5 24 7.5 Z',
};

export const AVATAR_BG_SHAPE_PATHS: Record<'hexagon' | 'cloud', string> = {
  hexagon:
    'M 17.5 2.4 L 30.5 2.4 Q 36.2 2.4 39.1 7.4 L 44.8 17.3 Q 47.6 24 44.8 30.7 L 39.1 40.6 Q 36.2 45.6 30.5 45.6 L 17.5 45.6 Q 11.8 45.6 8.9 40.6 L 3.2 30.7 Q 0.4 24 3.2 17.3 L 8.9 7.4 Q 11.8 2.4 17.5 2.4 Z',
  cloud:
    'M 24 5.5 C 28 2 36.4 3.5 38.6 9.4 C 45.8 11.8 47.4 21 45.4 29 C 43.6 38.5 37 45 24 45 C 11 45 4.4 38.5 2.6 29 C 0.6 21 2.2 11.8 9.4 9.4 C 11.6 3.5 20 2 24 5.5 Z',
};

export interface BloubSvgProps {
  shape?: AvatarShape;
  className?: string;
}

/** Pure flat Bloub vector Grok Bot with 2D gaze parallax, body lean, and tilted pill wink. */
export function GrokBotSvg({ shape = 'hexagon', className = '' }: BloubSvgProps) {
  const cls = ['avatar-animated-icon', 'avatar-grok-bot', className].filter(Boolean).join(' ');
  return (
    <svg viewBox="0 0 48 48" fill="none" aria-hidden="true" focusable="false" className={cls}>
      <g className="grok-body">
        <path
          d={AVATAR_SHAPE_PATHS[shape]}
          fill="var(--av-skin)"
          stroke="var(--av-skin-ring)"
          strokeWidth="1.2"
          strokeLinejoin="round"
        />
        <g className="grok-eyes">
          <rect
            x="16.6"
            y="19.6"
            width="5.2"
            height="8.8"
            rx="2.6"
            fill="var(--av-eye)"
            className="grok-eye grok-eye-l"
          />
          <rect
            x="26.2"
            y="19.6"
            width="5.2"
            height="8.8"
            rx="2.6"
            fill="var(--av-eye)"
            className="grok-eye grok-eye-r"
          />
        </g>
      </g>
    </svg>
  );
}

/** Pure flat Bloub vector Muse companion with dark stadium eyes, rosy pill cheeks, and happy squint. */
export function MuseBotSvg({ shape = 'cloud', className = '' }: BloubSvgProps) {
  const cls = ['avatar-animated-icon', 'avatar-muse-bot', className].filter(Boolean).join(' ');
  return (
    <svg viewBox="0 0 48 48" fill="none" aria-hidden="true" focusable="false" className={cls}>
      <path
        d="M39.5 8.5l.8 1.9 1.9.8-1.9.8-.8 1.9-.8-1.9-1.9-.8 1.9-.8z"
        fill="var(--av-spark)"
        className="muse-spark"
      />
      <g className="muse-body-group">
        <path
          d={AVATAR_SHAPE_PATHS[shape]}
          fill="var(--av-plush)"
          stroke="var(--av-plush-shade)"
          strokeWidth="1.2"
          strokeLinejoin="round"
        />
        <g className="muse-face">
          <rect
            x="16.8"
            y="20.4"
            width="4.4"
            height="7.6"
            rx="2.2"
            fill="var(--art-ink)"
            className="muse-eye muse-eye-l"
          />
          <rect
            x="26.8"
            y="20.4"
            width="4.4"
            height="7.6"
            rx="2.2"
            fill="var(--art-ink)"
            className="muse-eye muse-eye-r"
          />
          <rect
            x="11.4"
            y="26.4"
            width="5"
            height="2.6"
            rx="1.3"
            fill="var(--art-blush)"
            fillOpacity="0.88"
            className="muse-cheek"
          />
          <rect
            x="31.6"
            y="26.4"
            width="5"
            height="2.6"
            rx="1.3"
            fill="var(--art-blush)"
            fillOpacity="0.88"
            className="muse-cheek"
          />
        </g>
      </g>
    </svg>
  );
}
