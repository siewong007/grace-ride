import type { SVGProps } from 'react';

type BadgeSvgProps = SVGProps<SVGSVGElement>;

const mergeBadgeClass = (name: string, className?: string) =>
  ['badge-animated-svg', name, className].filter(Boolean).join(' ');

/** Normal Member 1: Joyful Rider — cute cruiser with happy bunny passenger */
export function BadgeJoyRider({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-joy-rider', className)}
    >
      {/* Twinkling stars */}
      <path d="M10 14l1 2.2 2.2 1-2.2 1-1 2.2-1-2.2-2.2-1 2.2-1z" fill="var(--art-gold)" className="badge-twinkle-1" />
      <path d="M46 12l.9 2 2 .9-2 .9-.9 2-.9-2-2-.9 2-.9z" fill="var(--art-gold)" className="badge-twinkle-2" />
      {/* Floating heart */}
      <path
        d="M28 7.5c0-1.2 1.3-1.9 2.2-1 .4.4.5.7.5.7s.2-.3.5-.7c.9-.9 2.2-.2 2.2 1 0 1.6-2.7 3.2-2.7 3.2S28 9.1 28 7.5z"
        fill="var(--art-coral)"
        className="badge-heart-float"
      />
      <g className="badge-car-bob">
        {/* Cute bunny ears peeking through sunroof */}
        <ellipse cx="22.5" cy="15.5" rx="2.1" ry="5" transform="rotate(-10 22.5 15.5)" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.5" />
        <ellipse cx="22.5" cy="15.5" rx="1" ry="2.8" transform="rotate(-10 22.5 15.5)" fill="var(--art-pink)" />
        <ellipse cx="28" cy="15" rx="2.1" ry="5" transform="rotate(8 28 15)" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.5" />
        <ellipse cx="28" cy="15" rx="1" ry="2.8" transform="rotate(8 28 15)" fill="var(--art-pink)" />
        {/* Bunny head */}
        <circle cx="25.5" cy="21" r="5.5" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.6" />
        <circle cx="23.8" cy="20.5" r="1.1" fill="var(--art-ink)" />
        <circle cx="27.2" cy="20.5" r="1.1" fill="var(--art-ink)" />
        <ellipse cx="22.4" cy="22.3" rx="1.4" ry="0.8" fill="var(--art-blush)" />
        <ellipse cx="28.6" cy="22.3" rx="1.4" ry="0.8" fill="var(--art-blush)" />
        {/* Car cabin & body */}
        <path
          d="M11 31c1.5-5.5 5-8 10.5-8h13c5.5 0 9 2.5 10.5 8z"
          fill="var(--art-sky-soft)"
          stroke="var(--art-ink)"
          strokeWidth="1.7"
          strokeLinejoin="round"
        />
        <rect x="8" y="29" width="40" height="12.5" rx="6" fill="var(--art-sky)" stroke="var(--art-ink)" strokeWidth="1.8" />
        <rect x="11" y="36" width="34" height="3" rx="1.5" fill="var(--art-cream)" />
        {/* Friendly headlights & rosy cheeks */}
        <circle cx="12.5" cy="33" r="2.2" fill="var(--art-gold)" />
        <circle cx="43.5" cy="33" r="2.2" fill="var(--art-gold)" />
        <path d="M25.5 31.5l1.1 1.1c.6.6 1.5 0 1.9-.6.5.6 1.3 1.2 1.9.6l1.1-1.1" stroke="var(--art-coral)" strokeWidth="1.5" strokeLinecap="round" />
      </g>
      {/* Wheels */}
      <circle cx="17" cy="42.5" r="4.2" fill="var(--art-ink)" />
      <circle cx="17" cy="42.5" r="1.8" fill="var(--art-cream)" />
      <circle cx="39" cy="42.5" r="4.2" fill="var(--art-ink)" />
      <circle cx="39" cy="42.5" r="1.8" fill="var(--art-cream)" />
    </svg>
  );
}

/** Normal Member 2: Faithful Attendee — golden bell & cheerful fellowship blossom */
export function BadgeFellowshipSprout({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-fellowship-sprout', className)}
    >
      {/* Floating music notes & sparkles */}
      <path d="M12 15v-5.5l4-1.2v5.5" stroke="var(--art-teal)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" className="badge-note-1" />
      <circle cx="10.8" cy="15.2" r="1.6" fill="var(--art-teal)" className="badge-note-1" />
      <path d="M43 13l.9 2 2 .9-2 .9-.9 2-.9-2-2-.9 2-.9z" fill="var(--art-gold)" className="badge-twinkle-1" />

      {/* Golden church bell swinging gently */}
      <g className="badge-bell-swing">
        <path
          d="M28 10c-6.5 0-10.5 5-10.5 12.5 0 4.5-2 7-3.5 8.5h28c-1.5-1.5-3.5-4-3.5-8.5C38.5 15 34.5 10 28 10z"
          fill="var(--art-gold)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        <circle cx="28" cy="8.5" r="2.2" fill="var(--art-amber)" stroke="var(--art-ink)" strokeWidth="1.5" />
        <circle cx="28" cy="33" r="3" fill="var(--art-amber)" stroke="var(--art-ink)" strokeWidth="1.5" />
        {/* Cute kawaii bell face */}
        <circle cx="24" cy="22" r="1.5" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="32" cy="22" r="1.5" fill="var(--art-ink)" className="badge-blink" />
        <ellipse cx="21.5" cy="24.5" rx="1.9" ry="1.1" fill="var(--art-coral)" fillOpacity="0.8" />
        <ellipse cx="34.5" cy="24.5" rx="1.9" ry="1.1" fill="var(--art-coral)" fillOpacity="0.8" />
        <path d="M26 24.8c1 1.3 3 1.3 4 0" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinecap="round" />
      </g>

      {/* Cheerful pink blossom & mint leaves below */}
      <g className="badge-flower-sway">
        <ellipse cx="20" cy="43" rx="5.5" ry="2.6" transform="rotate(-18 20 43)" fill="var(--art-mint)" stroke="var(--art-ink)" strokeWidth="1.4" />
        <ellipse cx="36" cy="43" rx="5.5" ry="2.6" transform="rotate(18 36 43)" fill="var(--art-mint)" stroke="var(--art-ink)" strokeWidth="1.4" />
        <circle cx="28" cy="40" r="3.4" fill="var(--art-pink)" stroke="var(--art-ink)" strokeWidth="1.4" />
        <circle cx="24.5" cy="42.5" r="3.2" fill="var(--art-pink-soft)" stroke="var(--art-ink)" strokeWidth="1.4" />
        <circle cx="31.5" cy="42.5" r="3.2" fill="var(--art-pink-soft)" stroke="var(--art-ink)" strokeWidth="1.4" />
        <circle cx="28" cy="42.5" r="2.2" fill="var(--art-gold)" />
      </g>
    </svg>
  );
}

/** Normal Member 3: Ready & Punctual — kawaii mint-and-amber alarm clock */
export function BadgePunctualClock({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-punctual-clock', className)}
    >
      <g className="badge-clock-jiggle">
        {/* Twin alarm bells */}
        <circle cx="15.5" cy="14.5" r="5" fill="var(--art-amber)" stroke="var(--art-ink)" strokeWidth="1.7" />
        <circle cx="40.5" cy="14.5" r="5" fill="var(--art-amber)" stroke="var(--art-ink)" strokeWidth="1.7" />
        {/* Clock feet */}
        <path d="M18 42l-3 5M38 42l3 5" stroke="var(--art-ink)" strokeWidth="2.4" strokeLinecap="round" />
        {/* Mint body */}
        <circle cx="28" cy="28.5" r="15.5" fill="var(--art-mint)" stroke="var(--art-ink)" strokeWidth="1.8" />
        {/* Cream face */}
        <circle cx="28" cy="28.5" r="11.8" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.4" />
        {/* Clock hands */}
        <path d="M28 21.5v7l4.5 2.5" stroke="var(--art-teal)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
        {/* Cute eyes & cheeks */}
        <circle cx="23" cy="29.5" r="1.4" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="33" cy="29.5" r="1.4" fill="var(--art-ink)" className="badge-blink" />
        <ellipse cx="20.5" cy="32" rx="1.8" ry="1" fill="var(--art-blush)" />
        <ellipse cx="35.5" cy="32" rx="1.8" ry="1" fill="var(--art-blush)" />
        <path d="M26 32.8c1 1.2 3 1.2 4 0" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinecap="round" />
      </g>
      {/* Popping ready checkmark */}
      <g className="badge-tick-pop">
        <circle cx="43" cy="40.5" r="7" fill="var(--art-green)" stroke="var(--art-ink)" strokeWidth="1.6" />
        <path d="M39.8 40.5l2.2 2.2 4.3-4.3" stroke="var(--art-white)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      </g>
    </svg>
  );
}

/** Normal Member 4: Caring Helper — cuddling twin smiling hearts */
export function BadgeWarmWelcome({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-warm-welcome', className)}
    >
      {/* Rising mini hearts */}
      <path
        d="M26 9.5c0-1.2 1.2-1.8 2-1 .3.3.5.6.5.6s.2-.3.5-.6c.8-.8 2-.2 2 1 0 1.4-2.5 2.9-2.5 2.9S26 10.9 26 9.5z"
        fill="var(--art-gold)"
        className="badge-heart-float"
      />
      <circle cx="11" cy="16" r="1.8" fill="var(--art-pink)" className="badge-twinkle-1" />
      <circle cx="46" cy="17" r="1.8" fill="var(--art-gold)" className="badge-twinkle-2" />

      {/* Back blush heart */}
      <path
        d="M36.5 18c-3.2 0-5.2 2.3-5.2 2.3S29.3 18 26.1 18c-3.4 0-6.1 2.7-6.1 6.3 0 6.5 11.3 14.2 11.3 14.2S42.6 30.8 42.6 24.3c0-3.6-2.7-6.3-6.1-6.3z"
        fill="var(--art-pink-soft)"
        stroke="var(--art-ink)"
        strokeWidth="1.7"
        strokeLinejoin="round"
        className="badge-heart-tilt"
      />
      {/* Main cuddly coral heart */}
      <g className="badge-heart-pulse">
        <path
          d="M29.5 21.5c-3.6 0-5.8 2.6-5.8 2.6s-2.2-2.6-5.8-2.6c-3.9 0-7 3.1-7 7.1 0 7.5 12.8 16.4 12.8 16.4s12.8-8.9 12.8-16.4c0-4-3.1-7.1-7-7.1z"
          fill="var(--art-coral)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        {/* Kawaii face */}
        <circle cx="19.5" cy="30.5" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="28" cy="30.5" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        <ellipse cx="16.8" cy="33" rx="2" ry="1.1" fill="var(--art-cream)" fillOpacity="0.7" />
        <ellipse cx="30.7" cy="33" rx="2" ry="1.1" fill="var(--art-cream)" fillOpacity="0.7" />
        <path d="M21.8 33.4c1.2 1.3 2.8 1.3 4 0" stroke="var(--art-ink)" strokeWidth="1.5" strokeLinecap="round" />
      </g>
    </svg>
  );
}

/** Driver 1 ("moon"): Night Owl — cute baby owl perched on a golden crescent moon */
export function BadgeNightOwl({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-night-owl', className)}
    >
      {/* Twinkling night stars */}
      <path d="M12 13l1.1 2.4 2.4 1.1-2.4 1.1-1.1 2.4-1.1-2.4-2.4-1.1 2.4-1.1z" fill="var(--art-gold)" className="badge-twinkle-1" />
      <path d="M44 12l.9 2 2 .9-2 .9-.9 2-.9-2-2-.9 2-.9z" fill="var(--art-sky)" className="badge-twinkle-2" />

      <g className="badge-moon-rock">
        {/* Golden crescent moon */}
        <path
          d="M15 24c-3.5 9 1.5 20.5 11.5 23.5 8.5 2.5 17.5-1.5 21-9-6 2.5-13 1.5-18-3.5-5-5-6-12-3.5-18-5.5 1.2-9.5 3.5-11 7z"
          fill="var(--art-gold)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        {/* Adorable baby owl perched inside moon */}
        <ellipse cx="30" cy="27.5" rx="9" ry="9.5" fill="var(--art-purple)" stroke="var(--art-ink)" strokeWidth="1.7" />
        {/* Ear tufts */}
        <path d="M23.5 19.5l-1.5-3.5 4.2 2zM36.5 19.5l1.5-3.5-4.2 2z" fill="var(--art-indigo)" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinejoin="round" />
        {/* Cream belly */}
        <ellipse cx="30" cy="31.5" rx="5.2" ry="4.5" fill="var(--art-cream)" />
        {/* Big owl eye circles */}
        <circle cx="26.2" cy="24.5" r="3.6" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.3" />
        <circle cx="33.8" cy="24.5" r="3.6" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.3" />
        <circle cx="26.5" cy="24.6" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="33.5" cy="24.6" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        {/* Cute beak & rosy cheeks */}
        <polygon points="30,27.8 28.5,25.8 31.5,25.8" fill="var(--art-amber)" />
        <ellipse cx="22.8" cy="27.5" rx="1.6" ry="0.9" fill="var(--art-pink)" />
        <ellipse cx="37.2" cy="27.5" rx="1.6" ry="0.9" fill="var(--art-pink)" />
      </g>
    </svg>
  );
}

/** Driver 2 ("award"): Resident Driver — smiling golden star-medal with fluttering ribbons */
export function BadgeResidentDriver({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-resident-driver', className)}
    >
      {/* Twinkling sparkles */}
      <circle cx="10" cy="18" r="2" fill="var(--art-cyan)" className="badge-twinkle-1" />
      <circle cx="46" cy="18" r="2" fill="var(--art-pink)" className="badge-twinkle-2" />

      {/* Fluttering ribbons */}
      <g className="badge-ribbon-flutter">
        <path
          d="M21 32l-4.5 16 6.5-3.2 4 4.2 2-15z"
          fill="var(--art-blue)"
          stroke="var(--art-ink)"
          strokeWidth="1.6"
          strokeLinejoin="round"
        />
        <path
          d="M35 32l4.5 16-6.5-3.2-4 4.2-2-15z"
          fill="var(--art-coral)"
          stroke="var(--art-ink)"
          strokeWidth="1.6"
          strokeLinejoin="round"
        />
      </g>
      {/* Golden medallion mascot */}
      <g className="badge-medal-bob">
        <circle cx="28" cy="23" r="14.5" fill="var(--art-gold)" stroke="var(--art-ink)" strokeWidth="1.8" />
        <circle cx="28" cy="23" r="11" fill="var(--art-cream)" stroke="var(--art-amber)" strokeWidth="1.5" />
        {/* Smiling eyes & cheeks */}
        <circle cx="23.5" cy="22" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="32.5" cy="22" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        <ellipse cx="21" cy="24.8" rx="1.9" ry="1.1" fill="var(--art-coral)" fillOpacity="0.8" />
        <ellipse cx="35" cy="24.8" rx="1.9" ry="1.1" fill="var(--art-coral)" fillOpacity="0.8" />
        <path d="M25.8 25.2c1.2 1.4 3.2 1.4 4.4 0" stroke="var(--art-ink)" strokeWidth="1.5" strokeLinecap="round" />
        {/* Little crown */}
        <path d="M22.5 12l2.5 3.5 3-4.5 3 4.5 2.5-3.5-.8 5h-9.4z" fill="var(--art-amber)" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinejoin="round" />
      </g>
    </svg>
  );
}

/** Driver 3 ("heart"): Caring Companion — adorable coral carpool van with floating hearts */
export function BadgeCarpoolLove({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-carpool-love', className)}
    >
      {/* Floating twin hearts */}
      <path
        d="M17 10.5c0-1.2 1.2-1.8 2-1 .3.3.5.6.5.6s.2-.3.5-.6c.8-.8 2-.2 2 1 0 1.4-2.5 2.9-2.5 2.9S17 11.9 17 10.5z"
        fill="var(--art-pink)"
        className="badge-heart-float"
      />
      <path
        d="M35 8.5c0-1.2 1.2-1.8 2-1 .3.3.5.6.5.6s.2-.3.5-.6c.8-.8 2-.2 2 1 0 1.4-2.5 2.9-2.5 2.9S35 9.9 35 8.5z"
        fill="var(--art-coral)"
        className="badge-heart-float-delay"
      />
      <g className="badge-car-bob">
        {/* Cute rounded carpool van */}
        <rect x="9" y="18" width="38" height="22" rx="9" fill="var(--art-coral)" stroke="var(--art-ink)" strokeWidth="1.8" />
        {/* Cream roof */}
        <path d="M13 21c2-2 5-3 9-3h12c4 0 7 1 9 3H13z" fill="var(--art-cream)" />
        {/* Twin smiling windshield */}
        <rect x="13.5" y="21" width="29" height="10" rx="4.5" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.4" />
        <circle cx="22" cy="25.5" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="34" cy="25.5" r="1.6" fill="var(--art-ink)" className="badge-blink" />
        <path d="M26 27c1.2 1.3 2.8 1.3 4 0" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinecap="round" />
        {/* Rosy cheeks & headlights */}
        <circle cx="13.5" cy="35" r="2.2" fill="var(--art-gold)" />
        <circle cx="42.5" cy="35" r="2.2" fill="var(--art-gold)" />
        <ellipse cx="18.5" cy="35" rx="2" ry="1.1" fill="var(--art-pink)" />
        <ellipse cx="37.5" cy="35" rx="2" ry="1.1" fill="var(--art-pink)" />
        {/* Center heart emblem */}
        <path
          d="M25.2 34.2c0-1 1-1.5 1.7-.8.3.3.4.5.4.5s.1-.2.4-.5c.7-.7 1.7-.2 1.7.8 0 1.2-2.1 2.4-2.1 2.4s-2.1-1.2-2.1-2.4z"
          fill="var(--art-cream)"
        />
      </g>
      {/* Wheels */}
      <circle cx="18" cy="41.5" r="4.2" fill="var(--art-ink)" />
      <circle cx="18" cy="41.5" r="1.8" fill="var(--art-cream)" />
      <circle cx="38" cy="41.5" r="4.2" fill="var(--art-ink)" />
      <circle cx="38" cy="41.5" r="1.8" fill="var(--art-cream)" />
    </svg>
  );
}

/** Driver 4: Helping Wheels — winged sky-blue steering wheel with golden halo */
export function BadgeHelpingWheels({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-helping-wheels', className)}
    >
      {/* Golden halo */}
      <ellipse cx="28" cy="10" rx="9" ry="3" stroke="var(--art-gold)" strokeWidth="2.4" className="badge-halo-pulse" />

      {/* Angel wings */}
      <path
        d="M15 26c-6-4-10-1-9 4 1 3.5 4.5 5 9 4z"
        fill="var(--art-cream)"
        stroke="var(--art-ink)"
        strokeWidth="1.6"
        strokeLinejoin="round"
        className="badge-wing-l"
      />
      <path
        d="M41 26c6-4 10-1 9 4-1 3.5-4.5 5-9 4z"
        fill="var(--art-cream)"
        stroke="var(--art-ink)"
        strokeWidth="1.6"
        strokeLinejoin="round"
        className="badge-wing-r"
      />

      {/* Kawaii steering wheel */}
      <g className="badge-wheel-bob">
        <circle cx="28" cy="30" r="13.5" stroke="var(--art-blue)" strokeWidth="4.2" />
        <circle cx="28" cy="30" r="15.4" stroke="var(--art-ink)" strokeWidth="1.4" />
        <circle cx="28" cy="30" r="11.4" stroke="var(--art-ink)" strokeWidth="1.3" />
        {/* Spokes */}
        <path d="M16.5 29h23M28 32v10.5" stroke="var(--art-sky)" strokeWidth="3.4" strokeLinecap="round" />
        {/* Smiling center hub */}
        <circle cx="28" cy="30" r="6.5" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.5" />
        <circle cx="25.8" cy="29.2" r="1.2" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="30.2" cy="29.2" r="1.2" fill="var(--art-ink)" className="badge-blink" />
        <path d="M26.5 31.6c.9.9 2.1.9 3 0" stroke="var(--art-ink)" strokeWidth="1.3" strokeLinecap="round" />
      </g>
    </svg>
  );
}

/** Traffic Lead 1: Green-Light Captain — kawaii traffic light mascot with glowing sequence */
export function BadgeTrafficLight({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-traffic-light', className)}
    >
      {/* Sparkles */}
      <circle cx="11" cy="20" r="1.8" fill="var(--art-gold)" className="badge-twinkle-1" />
      <circle cx="45" cy="18" r="1.8" fill="var(--art-mint)" className="badge-twinkle-2" />

      <g className="badge-tl-bob">
        {/* Captain cap */}
        <path d="M20 11.5c2-3.5 14-3.5 16 0z" fill="var(--art-blue)" stroke="var(--art-ink)" strokeWidth="1.6" strokeLinejoin="round" />
        <path d="M18 12h20" stroke="var(--art-gold)" strokeWidth="2" strokeLinecap="round" />

        {/* Traffic light housing */}
        <rect x="17" y="13" width="22" height="33" rx="8" fill="var(--art-slate)" stroke="var(--art-ink)" strokeWidth="1.8" />
        {/* Red, amber, green glowing lights */}
        <circle cx="28" cy="20.5" r="3.8" fill="var(--art-red)" stroke="var(--art-ink)" strokeWidth="1.2" className="tl-light tl-red" />
        <circle cx="28" cy="29.5" r="3.8" fill="var(--art-gold)" stroke="var(--art-ink)" strokeWidth="1.2" className="tl-light tl-amber" />
        <circle cx="28" cy="38.5" r="3.8" fill="var(--art-green)" stroke="var(--art-ink)" strokeWidth="1.2" className="tl-light tl-green" />

        {/* Cute smiling visor cheeks on housing sides */}
        <ellipse cx="19.8" cy="30" rx="1.4" ry="0.9" fill="var(--art-pink)" />
        <ellipse cx="36.2" cy="30" rx="1.4" ry="0.9" fill="var(--art-pink)" />
      </g>
      {/* Stand base */}
      <path d="M28 46v4M22 50h12" stroke="var(--art-ink)" strokeWidth="2.2" strokeLinecap="round" />
    </svg>
  );
}

/** Traffic Lead 2: Master Route Planner — smiling golden compass with swinging needle */
export function BadgeRouteCompass({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-route-compass', className)}
    >
      <path d="M11 13l.9 2 2 .9-2 .9-.9 2-.9-2-2-.9 2-.9z" fill="var(--art-cyan)" className="badge-twinkle-1" />
      <path d="M45 38l.9 2 2 .9-2 .9-.9 2-.9-2-2-.9 2-.9z" fill="var(--art-gold)" className="badge-twinkle-2" />

      {/* Compass ring */}
      <circle cx="28" cy="9.5" r="3.2" stroke="var(--art-ink)" strokeWidth="1.8" fill="var(--art-gold)" />
      {/* Outer golden bezel */}
      <circle cx="28" cy="30" r="17" fill="var(--art-gold)" stroke="var(--art-ink)" strokeWidth="1.8" />
      {/* Sky-blue dial */}
      <circle cx="28" cy="30" r="13" fill="var(--art-sky-soft)" stroke="var(--art-ink)" strokeWidth="1.4" />
      {/* Swinging needle */}
      <g className="badge-compass-needle">
        <polygon points="28,18.5 31.2,30 24.8,30" fill="var(--art-red)" stroke="var(--art-ink)" strokeWidth="1.2" strokeLinejoin="round" />
        <polygon points="28,41.5 31.2,30 24.8,30" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.2" strokeLinejoin="round" />
      </g>
      {/* Cute face on center pin */}
      <circle cx="28" cy="30" r="5.2" fill="var(--art-cream)" stroke="var(--art-ink)" strokeWidth="1.4" />
      <circle cx="26.2" cy="29.3" r="1.1" fill="var(--art-ink)" className="badge-blink" />
      <circle cx="29.8" cy="29.3" r="1.1" fill="var(--art-ink)" className="badge-blink" />
      <path d="M26.7 31.4c.8.8 1.8.8 2.6 0" stroke="var(--art-ink)" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

/** Traffic Lead 3: Smooth Coordinator — cute sky-blue & coral traffic whistle with sound puffs */
export function BadgeTrafficWhistle({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-traffic-whistle', className)}
    >
      {/* Sound puffs & stars */}
      <path d="M39 14c2.5-2.5 5-2 6 0" stroke="var(--art-amber)" strokeWidth="2.2" strokeLinecap="round" className="badge-whistle-puff-1" />
      <path d="M42 20c3-1 5 0 5.5 2" stroke="var(--art-coral)" strokeWidth="2.2" strokeLinecap="round" className="badge-whistle-puff-2" />
      <path d="M12 14l.9 2 2 .9-2 .9-.9 2-.9-2-2-.9 2-.9z" fill="var(--art-gold)" className="badge-twinkle-1" />

      <g className="badge-whistle-tilt">
        {/* Lanyard ring */}
        <circle cx="13" cy="33" r="3.8" fill="var(--art-gold)" stroke="var(--art-ink)" strokeWidth="1.6" />
        {/* Whistle body */}
        <path
          d="M27 21c-7.2 0-13 5.8-13 13s5.8 13 13 13c7 0 12.6-5.4 13-12.3l6.5-2.7V23.5H31.8C30.4 22 28.8 21 27 21z"
          fill="var(--art-sky)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        {/* Coral mouthpiece tip */}
        <rect x="41.5" y="23.5" width="5" height="8.5" rx="1.8" fill="var(--art-coral)" stroke="var(--art-ink)" strokeWidth="1.5" />
        {/* Cute inner face */}
        <circle cx="27" cy="34.5" r="8.2" fill="var(--art-white)" stroke="var(--art-ink)" strokeWidth="1.4" />
        <circle cx="24" cy="33.5" r="1.4" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="30" cy="33.5" r="1.4" fill="var(--art-ink)" className="badge-blink" />
        <ellipse cx="21.8" cy="36" rx="1.6" ry="0.9" fill="var(--art-blush)" />
        <ellipse cx="32.2" cy="36" rx="1.6" ry="0.9" fill="var(--art-blush)" />
        <path d="M25.2 36.4c1 1.1 2.6 1.1 3.6 0" stroke="var(--art-ink)" strokeWidth="1.4" strokeLinecap="round" />
      </g>
    </svg>
  );
}

/** Traffic Lead 4: Care Steward — crowned mint-and-sky shield buddy with coral heart */
export function BadgeCareShield({ className, ...props }: BadgeSvgProps) {
  return (
    <svg
      viewBox="0 0 56 56"
      fill="none"
      aria-hidden="true"
      focusable="false"
      {...props}
      className={mergeBadgeClass('badge-care-shield', className)}
    >
      {/* Sparkles */}
      <circle cx="10" cy="21" r="1.8" fill="var(--art-gold)" className="badge-twinkle-1" />
      <circle cx="46" cy="21" r="1.8" fill="var(--art-cyan)" className="badge-twinkle-2" />

      {/* Bouncing golden crown */}
      <path
        d="M20 14.5l3-5 5 3.5 5-3.5 3 5-1.5 3.5h-13z"
        fill="var(--art-gold)"
        stroke="var(--art-ink)"
        strokeWidth="1.6"
        strokeLinejoin="round"
        className="badge-crown-bob"
      />

      {/* Mint-and-sky protective shield */}
      <g className="badge-shield-pulse">
        <path
          d="M28 17l15 5v11.5c0 8.5-6.5 14.5-15 17-8.5-2.5-15-8.5-15-17V22z"
          fill="var(--art-mint)"
          stroke="var(--art-ink)"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
        <path
          d="M28 20.5l11.5 3.8v9c0 6.5-5 11.2-11.5 13.3-6.5-2.1-11.5-6.8-11.5-13.3v-9z"
          fill="var(--art-sky-soft)"
        />
        {/* Cute eyes & cheeks */}
        <circle cx="23.5" cy="29" r="1.5" fill="var(--art-ink)" className="badge-blink" />
        <circle cx="32.5" cy="29" r="1.5" fill="var(--art-ink)" className="badge-blink" />
        <ellipse cx="20.8" cy="31.8" rx="1.8" ry="1" fill="var(--art-pink)" />
        <ellipse cx="35.2" cy="31.8" rx="1.8" ry="1" fill="var(--art-pink)" />
        {/* Little coral heart emblem */}
        <path
          d="M24.8 35.8c0-1.4 1.5-2.2 2.5-1.1.4.4.7.8.7.8s.3-.4.7-.8c1-1.1 2.5-.3 2.5 1.1 0 1.9-3.2 3.8-3.2 3.8s-3.2-1.9-3.2-3.8z"
          fill="var(--art-coral)"
        />
      </g>
    </svg>
  );
}
