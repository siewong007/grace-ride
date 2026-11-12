// Grace Ride brand pieces from the logo kit (docs/brand/grace-ride-logo-kit).
// Colours come from the --logo-* tokens in styles.css, which switch to the kit's
// white version in dark mode.

/**
 * The "Grace Ride" wordmark in Jost: "Grace" Bold, "Ride" Light. The logotype is not
 * translated. `mono` drops the brand colours and inherits the surrounding text colour,
 * for coloured panels where the kit asks for the white / one-colour version.
 */
export function Wordmark({ mono = false, className = '' }: { mono?: boolean; className?: string }) {
  return (
    <span className={['wordmark', mono ? 'is-mono' : '', className].filter(Boolean).join(' ')}>
      <span className="wordmark-grace">Grace</span> <span className="wordmark-ride">Ride</span>
    </span>
  );
}

/**
 * The animated loading mark (animation/grace-ride-loading-mark.svg): the cross lands,
 * ripples spread, the arc sweeps in, then the ripples keep pulsing while loading.
 * Decorative; the caller provides the accessible loading status.
 */
export function LoadingMark({ size = 72 }: { size?: number }) {
  return (
    <svg className="loading-mark" viewBox="0 4 240 240" width={size} height={size} aria-hidden="true" focusable="false">
      <path className="lm-arc lm-arc-l" d="M41.76 150.04 A81 81 0 0 1 107 91.05" />
      <path className="lm-arc lm-arc-r" d="M133 91.05 A81 81 0 0 1 198.24 150.04" />
      <circle className="lm-dot lm-dot-l" cx="41.76" cy="150.04" r="2.5" />
      <circle className="lm-dot lm-dot-r" cx="198.24" cy="150.04" r="2.5" />
      <ellipse className="lm-ping" cx="120" cy="213" rx="20" ry="5.5" vectorEffect="non-scaling-stroke" />
      <ellipse className="lm-ping lm-ping-2" cx="120" cy="213" rx="20" ry="5.5" vectorEffect="non-scaling-stroke" />
      <ellipse className="lm-ground" cx="120" cy="213" rx="20" ry="5.5" />
      <path className="lm-drop" d="M113 30H127V52H159V66H127V158L120 212L113 158V66H81V52H113Z" />
    </svg>
  );
}
