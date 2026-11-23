import type { SVGProps } from 'react';

type IconProps = SVGProps<SVGSVGElement>;

const mergeAvatarClass = (name: string, className?: string) =>
  ['avatar-animated-icon', name, className].filter(Boolean).join(' ');

/** Adorable Grace Cruiser animated car avatar */
export function GraceCarAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-grace-car', className)}
    >
      {/* Synced ground shadow */}
      <ellipse cx="24" cy="40.5" rx="14" ry="2.2" fill="var(--art-ink)" fillOpacity="0.14" className="av-shadow" />
      {/* Floating heart puff */}
      <path
        d="M36 9.5c0-1.3 1.4-2 2.4-1.1.4.4.6.8.6.8s.2-.4.6-.8c1-.9 2.4-.2 2.4 1.1 0 1.7-3 3.5-3 3.5s-3-1.8-3-3.5z"
        fill="var(--art-coral)"
        className="grace-car-heart"
      />
      <g className="grace-car-body">
        {/* Roof & cabin */}
        <path
          d="M12 22c1.2-6 5.5-9.5 12-9.5s10.8 3.5 12 9.5z"
          fill="var(--art-sky)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        {/* Cute windshield */}
        <path
          d="M15 21.5c1-4 4.2-6.5 9-6.5s8 2.5 9 6.5z"
          fill="var(--art-white)"
        />
        {/* Smiling eyes inside windshield */}
        <circle cx="20.5" cy="18.6" r="1.4" fill="var(--art-ink)" className="grace-car-eye" />
        <circle cx="27.5" cy="18.6" r="1.4" fill="var(--art-ink)" className="grace-car-eye" />
        <path d="M22.3 20c1.1 1 2.3 1 3.4 0" stroke="var(--art-ink)" strokeWidth="1.3" strokeLinecap="round" />

        {/* Car body */}
        <rect
          x="7"
          y="21"
          width="34"
          height="13"
          rx="6.5"
          fill="var(--art-blue)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
        />
        {/* Cream bumper stripe */}
        <rect x="9" y="29" width="30" height="3.2" rx="1.6" fill="var(--art-cream)" />
        {/* Headlights */}
        <circle cx="11.2" cy="25.5" r="2.6" fill="var(--art-gold)" className="grace-car-light" />
        <circle cx="36.8" cy="25.5" r="2.6" fill="var(--art-gold)" className="grace-car-light" />
        {/* Rosy cheeks */}
        <ellipse cx="15" cy="27.2" rx="2" ry="1.1" fill="var(--art-pink)" fillOpacity="0.85" />
        <ellipse cx="33" cy="27.2" rx="2" ry="1.1" fill="var(--art-pink)" fillOpacity="0.85" />
        {/* Grace Ride tiny cross emblem */}
        <rect x="20.5" y="23.2" width="7" height="5" rx="1.5" fill="var(--art-white)" />
        <path d="M24 24.1v3.2M22.4 25.7h3.2" stroke="var(--art-red)" strokeWidth="1.5" strokeLinecap="round" />
      </g>
      {/* Wheels */}
      <g className="grace-car-wheel grace-car-wheel-l">
        <circle cx="15" cy="35" r="4.3" fill="var(--art-ink)" />
        <circle cx="15" cy="35" r="1.9" fill="var(--art-cream)" />
      </g>
      <g className="grace-car-wheel grace-car-wheel-r">
        <circle cx="33" cy="35" r="4.3" fill="var(--art-ink)" />
        <circle cx="33" cy="35" r="1.9" fill="var(--art-cream)" />
      </g>
    </svg>
  );
}

/** Adorable Peace Dove animated avatar */
export function PeaceDoveAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-peace-dove', className)}
    >
      {/* Twinkling star */}
      <path
        d="M38 8l1 2.4 2.4 1-2.4 1-1 2.4-1-2.4-2.4-1 2.4-1z"
        fill="var(--art-gold)"
        className="dove-star"
      />
      <g className="dove-body">
        {/* Wing back */}
        <path
          d="M15 22c-5.5-5.5-8.5-2.5-6.5 3.5 1.5 4.2 5.5 6 9.5 5.5z"
          fill="var(--art-sky-soft)"
          stroke="var(--art-ink)"
          strokeWidth="1.7"
          strokeLinejoin="round"
          className="dove-wing-back"
        />
        {/* Tail */}
        <path
          d="M12 29l-6 4 4.5 2.5 4.5-3.5z"
          fill="var(--art-sky)"
          stroke="var(--art-ink)"
          strokeWidth="1.7"
          strokeLinejoin="round"
        />
        {/* Plump white dove body */}
        <ellipse
          cx="24.5"
          cy="25"
          rx="12.5"
          ry="11"
          fill="var(--art-white)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
        />
        {/* Wing front */}
        <path
          d="M18 22c2.5-6.5 9-7.5 10.5-1.5 1 4-2.5 8-8 7.5-2-.2-3-2.5-2.5-6z"
          fill="var(--art-sky-soft)"
          stroke="var(--art-ink)"
          strokeWidth="1.7"
          className="dove-wing-front"
        />
        {/* Cute eyes & cheeks */}
        <circle cx="29.5" cy="21" r="1.7" fill="var(--art-ink)" className="dove-eye" />
        <circle cx="30.1" cy="20.4" r="0.6" fill="var(--art-white)" />
        <ellipse cx="27.5" cy="24.2" rx="2.2" ry="1.3" fill="var(--art-blush)" fillOpacity="0.85" />
        {/* Beak */}
        <path
          d="M36.5 21.5l4.2 1.5-4 1.8z"
          fill="var(--art-amber)"
          stroke="var(--art-ink)"
          strokeWidth="1.4"
          strokeLinejoin="round"
        />
        {/* Olive sprig */}
        <path d="M38.5 23.5c2 2.2 3.5 4.5 3.5 6.5" stroke="var(--art-green)" strokeWidth="1.6" strokeLinecap="round" />
        <ellipse cx="41.2" cy="25.5" rx="2" ry="1.1" transform="rotate(-25 41.2 25.5)" fill="var(--art-mint)" />
        <ellipse cx="38.5" cy="27.2" rx="2" ry="1.1" transform="rotate(20 38.5 27.2)" fill="var(--art-green)" />
      </g>
    </svg>
  );
}

/** Adorable Sunny Smile animated cloud-and-sun avatar */
export function SunnyCloudAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-sunny-cloud', className)}
    >
      {/* Sun & rotating rays */}
      <g className="sun-rays">
        <line x1="31" y1="6.5" x2="31" y2="9.5" stroke="var(--art-amber)" strokeWidth="2.2" strokeLinecap="round" />
        <line x1="39.5" y1="10" x2="37.4" y2="12.1" stroke="var(--art-amber)" strokeWidth="2.2" strokeLinecap="round" />
        <line x1="43" y1="18.5" x2="40" y2="18.5" stroke="var(--art-amber)" strokeWidth="2.2" strokeLinecap="round" />
        <line x1="22.5" y1="10" x2="24.6" y2="12.1" stroke="var(--art-amber)" strokeWidth="2.2" strokeLinecap="round" />
      </g>
      <circle cx="31" cy="18.5" r="7.5" fill="var(--art-gold)" stroke="var(--art-ink)" strokeWidth="1.8" className="sun-orb" />

      {/* Fluffy smiling cloud */}
      <g className="cloud-puff">
        <path
          d="M14 36h20a7 7 0 0 0 1-13.9 8.5 8.5 0 0 0-16.4-1.8A6.8 6.8 0 0 0 14 36z"
          fill="var(--art-white)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        <circle cx="19" cy="28" r="1.6" fill="var(--art-ink)" className="cloud-eye" />
        <circle cx="29" cy="28" r="1.6" fill="var(--art-ink)" className="cloud-eye" />
        <ellipse cx="16.2" cy="30.8" rx="2.1" ry="1.2" fill="var(--art-blush)" fillOpacity="0.85" />
        <ellipse cx="31.8" cy="30.8" rx="2.1" ry="1.2" fill="var(--art-blush)" fillOpacity="0.85" />
        <path d="M21.8 30.2c1.2 1.8 3.2 1.8 4.4 0" stroke="var(--art-ink)" strokeWidth="1.6" strokeLinecap="round" />
      </g>
    </svg>
  );
}

/** Adorable Cuddly Bear animated avatar */
export function HappyBearAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-happy-bear', className)}
    >
      {/* Synced ground shadow */}
      <ellipse cx="24" cy="43" rx="11" ry="2" fill="var(--art-ink)" fillOpacity="0.14" className="av-shadow" />
      {/* Floating heart */}
      <path
        d="M37 8.5c0-1.2 1.3-1.9 2.2-1 .4.4.5.7.5.7s.2-.3.5-.7c.9-.9 2.2-.2 2.2 1 0 1.5-2.7 3.2-2.7 3.2S37 10 37 8.5z"
        fill="var(--art-coral)"
        className="bear-heart"
      />
      <g className="bear-head-group">
        {/* Ears */}
        <g className="bear-ear-l">
          <circle cx="13.5" cy="14" r="5.5" fill="var(--art-caramel)" stroke="var(--art-ink)" strokeWidth="1.8" />
          <circle cx="13.5" cy="14" r="2.7" fill="var(--art-cream)" />
        </g>
        <g className="bear-ear-r">
          <circle cx="34.5" cy="14" r="5.5" fill="var(--art-caramel)" stroke="var(--art-ink)" strokeWidth="1.8" />
          <circle cx="34.5" cy="14" r="2.7" fill="var(--art-cream)" />
        </g>
        {/* Bear head */}
        <ellipse cx="24" cy="24.5" rx="13.5" ry="11.5" fill="var(--art-caramel)" stroke="var(--art-ink)" strokeWidth="1.8" />
        {/* Muzzle */}
        <ellipse cx="24" cy="27.2" rx="6.2" ry="4.5" fill="var(--art-cream)" />
        {/* Nose & smile */}
        <ellipse cx="24" cy="25.5" rx="2.1" ry="1.4" fill="var(--art-brown)" />
        <path d="M21.8 28.4c1 1.1 3.4 1.1 4.4 0" stroke="var(--art-brown)" strokeWidth="1.5" strokeLinecap="round" />
        {/* Eyes & cheeks */}
        <circle cx="18.2" cy="22.5" r="1.8" fill="var(--art-ink)" className="bear-eye" />
        <circle cx="29.8" cy="22.5" r="1.8" fill="var(--art-ink)" className="bear-eye" />
        <ellipse cx="14.5" cy="25.8" rx="2.2" ry="1.2" fill="var(--art-coral)" fillOpacity="0.7" />
        <ellipse cx="33.5" cy="25.8" rx="2.2" ry="1.2" fill="var(--art-coral)" fillOpacity="0.7" />
        {/* Grace Ride blue scarf */}
        <path
          d="M14.5 34.5c3.5 2.5 15.5 2.5 19 0l1.5 4.2c-5 2.2-17 2.2-22 0z"
          fill="var(--art-blue)"
          stroke="var(--art-ink)"
          strokeWidth="1.6"
          strokeLinejoin="round"
        />
        <rect x="28.5" y="36" width="4.5" height="6.5" rx="1.8" fill="var(--art-sky)" stroke="var(--art-ink)" strokeWidth="1.4" />
      </g>
    </svg>
  );
}

/** Adorable Mochi Cat animated avatar */
export function FellowshipCatAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-fellowship-cat', className)}
    >
      {/* Sparkle */}
      <circle cx="39" cy="11" r="1.8" fill="var(--art-gold)" className="cat-sparkle" />
      <g className="cat-head-group">
        {/* Pointy kitty ears */}
        <path
          d="M12 20L10.5 9.5l8.5 5.5z"
          fill="var(--art-cream)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        <path d="M13 17.5L12.2 12.2l4.5 3z" fill="var(--art-pink)" />
        <path
          d="M36 20l1.5-10.5L29 15z"
          fill="var(--art-cream)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        <path d="M35 17.5l.8-5.3-4.5 3z" fill="var(--art-pink)" />
        {/* Chubby mochi head */}
        <ellipse cx="24" cy="27" rx="14" ry="12" fill="var(--art-cream)" stroke="var(--art-ink)" strokeWidth="1.8" />
        {/* Forehead stripes */}
        <path d="M24 15.5v3.2M21.2 16.2l.8 2.6M26.8 16.2l-.8 2.6" stroke="var(--art-caramel)" strokeWidth="1.8" strokeLinecap="round" />
        {/* Twinkling eyes */}
        <circle cx="18.2" cy="25.5" r="2.1" fill="var(--art-ink)" className="cat-eye" />
        <circle cx="18.9" cy="24.8" r="0.7" fill="var(--art-white)" />
        <circle cx="29.8" cy="25.5" r="2.1" fill="var(--art-ink)" className="cat-eye" />
        <circle cx="30.5" cy="24.8" r="0.7" fill="var(--art-white)" />
        {/* Pink nose & ":3" mouth */}
        <polygon points="24,28.8 22.3,27 25.7,27" fill="var(--art-coral)" />
        <path d="M21 30.2c1 1.3 2.5 1 3-.6.5 1.6 2 1.9 3 .6" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinecap="round" />
        {/* Rosy cheeks */}
        <ellipse cx="14.2" cy="28.5" rx="2.3" ry="1.3" fill="var(--art-pink)" fillOpacity="0.8" />
        <ellipse cx="33.8" cy="28.5" rx="2.3" ry="1.3" fill="var(--art-pink)" fillOpacity="0.8" />
        {/* Whiskers */}
        <g className="cat-whiskers" stroke="var(--art-ink)" strokeWidth="1.3" strokeLinecap="round">
          <line x1="12" y1="26.5" x2="7" y2="25.5" />
          <line x1="12" y1="29.5" x2="7" y2="30.5" />
          <line x1="36" y1="26.5" x2="41" y2="25.5" />
          <line x1="36" y1="29.5" x2="41" y2="30.5" />
        </g>
      </g>
    </svg>
  );
}

/** Adorable Joy Puppy animated avatar */
export function JoyPuppyAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-joy-puppy', className)}
    >
      {/* Synced ground shadow */}
      <ellipse cx="24" cy="42.5" rx="11.5" ry="2.1" fill="var(--art-ink)" fillOpacity="0.14" className="av-shadow" />
      {/* Tail wag */}
      <path
        d="M36 35c4-2 5.5-5.5 4-8"
        stroke="var(--art-caramel)"
        strokeWidth="3.5"
        strokeLinecap="round"
        className="puppy-tail"
      />
      <g className="puppy-head-group">
        {/* Floppy ears */}
        <ellipse
          cx="10.5"
          cy="22"
          rx="4.2"
          ry="8.2"
          transform="rotate(14 10.5 22)"
          fill="var(--art-brown)"
          stroke="var(--art-ink)"
          strokeWidth="1.7"
          className="puppy-ear-l"
        />
        <ellipse
          cx="37.5"
          cy="22"
          rx="4.2"
          ry="8.2"
          transform="rotate(-14 37.5 22)"
          fill="var(--art-brown)"
          stroke="var(--art-ink)"
          strokeWidth="1.7"
          className="puppy-ear-r"
        />
        {/* Golden puppy face */}
        <ellipse cx="24" cy="23.5" rx="12.5" ry="11.5" fill="var(--art-gold)" stroke="var(--art-ink)" strokeWidth="1.8" />
        {/* Cute eye patch */}
        <circle cx="18.2" cy="21.5" r="4.2" fill="var(--art-cream)" />
        {/* Muzzle */}
        <ellipse cx="24" cy="27.2" rx="5.8" ry="4.2" fill="var(--art-cream)" />
        {/* Eyes */}
        <circle cx="18.2" cy="21.5" r="1.8" fill="var(--art-ink)" className="puppy-eye" />
        <circle cx="29.8" cy="21.5" r="1.8" fill="var(--art-ink)" className="puppy-eye" />
        {/* Nose & happy tongue */}
        <ellipse cx="24" cy="25.2" rx="2.2" ry="1.5" fill="var(--art-ink)" />
        <path d="M22.5 28.8c0 1.8 3 1.8 3 0z" fill="var(--art-coral)" />
        <path d="M21.5 27.6c1 1.2 4 1.2 5 0" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinecap="round" />
        {/* Teal collar & gold bell */}
        <path d="M15 34.5c4.5 2 13.5 2 18 0" stroke="var(--art-teal)" strokeWidth="3.2" strokeLinecap="round" />
        <circle cx="24" cy="37.5" r="2.4" fill="var(--art-amber)" stroke="var(--art-ink)" strokeWidth="1.3" />
      </g>
    </svg>
  );
}

/** Adorable Starry Guide animated star lantern avatar */
export function StarLanternAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-star-lantern', className)}
    >
      {/* Orbiting sparkles */}
      <circle cx="9.5" cy="14" r="2" fill="var(--art-cyan)" className="lantern-sparkle-1" />
      <circle cx="38.5" cy="14" r="1.8" fill="var(--art-pink)" className="lantern-sparkle-2" />
      <circle cx="24" cy="42.5" r="1.7" fill="var(--art-mint)" className="lantern-sparkle-1" />
      <g className="lantern-star">
        <path
          d="M24 6.5l5 10.2 11.2 1.6-8.1 7.9 1.9 11.1L24 32l-10 5.3 1.9-11.1-8.1-7.9 11.2-1.6z"
          fill="var(--art-gold)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        {/* Inner glow highlight */}
        <circle cx="24" cy="24" r="6.5" fill="var(--art-cream)" fillOpacity="0.6" />
        {/* Kawaii face */}
        <circle cx="20.2" cy="22.8" r="1.6" fill="var(--art-ink)" className="lantern-eye" />
        <circle cx="27.8" cy="22.8" r="1.6" fill="var(--art-ink)" className="lantern-eye" />
        <ellipse cx="17.8" cy="25.5" rx="1.9" ry="1.1" fill="var(--art-coral)" fillOpacity="0.75" />
        <ellipse cx="30.2" cy="25.5" rx="1.9" ry="1.1" fill="var(--art-coral)" fillOpacity="0.75" />
        <path d="M22 25.8c1 1.4 3 1.4 4 0" stroke="var(--art-ink)" strokeWidth="1.5" strokeLinecap="round" />
      </g>
    </svg>
  );
}

/** Adorable Grace Blossom animated tulip sprout avatar */
export function BlossomSproutAvatar({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-blossom-sprout', className)}
    >
      {/* Floating dewdrop */}
      <circle cx="38" cy="11" r="2" fill="var(--art-sky)" className="blossom-dew" />
      {/* Stem & leaves */}
      <path d="M24 31v10" stroke="var(--art-green)" strokeWidth="3" strokeLinecap="round" />
      <path
        d="M24 37c-5.5-1-9.5-4.5-9.5-7.5 3.5-1 7.5 1.5 9.5 7.5z"
        fill="var(--art-mint)"
        stroke="var(--art-ink)"
        strokeWidth="1.5"
        strokeLinejoin="round"
        className="blossom-leaf-l"
      />
      <path
        d="M24 37c5.5-1 9.5-4.5 9.5-7.5-3.5-1-7.5 1.5-9.5 7.5z"
        fill="var(--art-mint)"
        stroke="var(--art-ink)"
        strokeWidth="1.5"
        strokeLinejoin="round"
        className="blossom-leaf-r"
      />
      {/* Blossom head */}
      <g className="blossom-flower">
        <path
          d="M12 19c0-5 4-9 6-9 2.5 0 4.5 3 6 5.5 1.5-2.5 3.5-5.5 6-5.5 2 0 6 4 6 9 0 7.5-5.5 12.5-12 12.5S12 26.5 12 19z"
          fill="var(--art-pink)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        <path d="M18.5 15c2.2 2.5 8.8 2.5 11 0" fill="var(--art-pink-soft)" />
        {/* Smiling eyes & cheeks */}
        <circle cx="19.5" cy="22" r="1.6" fill="var(--art-ink)" className="blossom-eye" />
        <circle cx="28.5" cy="22" r="1.6" fill="var(--art-ink)" className="blossom-eye" />
        <ellipse cx="16.8" cy="24.8" rx="2" ry="1.1" fill="var(--art-coral)" fillOpacity="0.8" />
        <ellipse cx="31.2" cy="24.8" rx="2" ry="1.1" fill="var(--art-coral)" fillOpacity="0.8" />
        <path d="M21.8 25.2c1.2 1.4 3.2 1.4 4.4 0" stroke="var(--art-ink)" strokeWidth="1.5" strokeLinecap="round" />
      </g>
    </svg>
  );
}

/** Grok Bot: minimalist flat-coloured geometric squircle with expressive animated white pill eyes */
export function GrokIcon({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-grok-bot', className)}
    >
      {/* Synced ground shadow */}
      <ellipse cx="24" cy="42.5" rx="11" ry="2.2" fill="var(--art-ink)" fillOpacity="0.14" className="grok-shadow" />
      <g className="grok-body">
        {/* Flat-coloured pebble / squircle silhouette */}
        <rect
          x="8.5"
          y="7.5"
          width="31"
          height="31"
          rx="13.5"
          fill="var(--av-skin)"
          stroke="var(--av-skin-ring)"
          strokeWidth="1.2"
        />
        {/* Subtle flat specular accent */}
        <ellipse
          cx="15.5"
          cy="13.2"
          rx="3.4"
          ry="1.5"
          transform="rotate(-24 15.5 13.2)"
          fill="var(--art-white)"
          fillOpacity="0.16"
        />
        {/* Signature expressive vertical pill eyes (invert on white skin via --av-eye) */}
        <g className="grok-eyes">
          <rect x="16.2" y="18" width="4.6" height="9.8" rx="2.3" fill="var(--av-eye)" className="grok-eye grok-eye-l" />
          <rect x="27.2" y="18" width="4.6" height="9.8" rx="2.3" fill="var(--av-eye)" className="grok-eye grok-eye-r" />
        </g>
      </g>
    </svg>
  );
}

/** Muse (Jolly): plump plush mascot with stubby limbs, glossy dark eyes, rosy cheeks & sweet grin */
export function MuseIcon({ className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeAvatarClass('avatar-muse-bot', className)}
    >
      {/* Synced ground shadow */}
      <ellipse cx="24" cy="43.2" rx="12" ry="2.3" fill="var(--art-ink)" fillOpacity="0.14" className="muse-shadow" />
      <g className="muse-body-group">
        {/* Stubby plush feet */}
        <ellipse
          cx="18.2"
          cy="39.6"
          rx="4.4"
          ry="2.8"
          fill="var(--av-plush-shade)"
          stroke="var(--art-ink)"
          strokeWidth="1.5"
          className="muse-foot muse-foot-l"
        />
        <ellipse
          cx="29.8"
          cy="39.6"
          rx="4.4"
          ry="2.8"
          fill="var(--av-plush-shade)"
          stroke="var(--art-ink)"
          strokeWidth="1.5"
          className="muse-foot muse-foot-r"
        />

        {/* Stubby plush arms */}
        <ellipse
          cx="9.4"
          cy="27"
          rx="3.4"
          ry="4.8"
          transform="rotate(24 9.4 27)"
          fill="var(--av-plush)"
          stroke="var(--art-ink)"
          strokeWidth="1.5"
          className="muse-arm-l"
        />
        <ellipse
          cx="38.6"
          cy="27"
          rx="3.4"
          ry="4.8"
          transform="rotate(-24 38.6 27)"
          fill="var(--av-plush)"
          stroke="var(--art-ink)"
          strokeWidth="1.5"
          className="muse-arm-r"
        />

        {/* Plump, compact rotund plush body silhouette + soft crown tuft */}
        <path
          d="M24 7.8c2.2 0 3.8 1 4.5 2.1 6.8 1.4 10.7 6.8 10.7 14.6 0 9.4-5.4 15.2-15.2 15.2S8.8 33.9 8.8 24.5c0-7.8 3.9-13.2 10.7-14.6.7-1.1 2.3-2.1 4.5-2.1z"
          fill="var(--av-plush)"
          stroke="var(--art-ink)"
          strokeWidth="1.6"
          strokeLinejoin="round"
        />
        {/* Cozy plush belly & highlight */}
        <ellipse cx="24" cy="30.6" rx="9" ry="6.2" fill="var(--av-plush-shade)" fillOpacity="0.55" />
        <ellipse cx="18.2" cy="14.2" rx="3.8" ry="1.6" transform="rotate(-18 18.2 14.2)" fill="var(--art-white)" fillOpacity="0.48" />

        {/* Small glossy dark blinking eyes with sparkle catchlights */}
        <g className="muse-eye">
          <ellipse cx="18.6" cy="21.4" rx="2.1" ry="2.4" fill="var(--art-ink)" />
          <circle cx="19.3" cy="20.6" r="0.7" fill="var(--art-white)" />
        </g>
        <g className="muse-eye">
          <ellipse cx="29.4" cy="21.4" rx="2.1" ry="2.4" fill="var(--art-ink)" />
          <circle cx="30.1" cy="20.6" r="0.7" fill="var(--art-white)" />
        </g>

        {/* Prominent soft pink rosy cheeks */}
        <ellipse cx="14.5" cy="24.8" rx="3.1" ry="1.8" fill="var(--art-blush)" fillOpacity="0.92" className="muse-cheek" />
        <ellipse cx="33.5" cy="24.8" rx="3.1" ry="1.8" fill="var(--art-blush)" fillOpacity="0.92" className="muse-cheek" />

        {/* Sweet toothless grin */}
        <path d="M21 26.8c1.5 2 4.5 2 6 0" stroke="var(--art-ink)" strokeWidth="1.6" strokeLinecap="round" className="muse-smile" />
      </g>
    </svg>
  );
}
