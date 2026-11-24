import { useEffect, useId, useMemo, useRef } from 'react';
import {
  GROKBOT_COLOR_PALETTES,
  GROKBOT_SVG_META,
  type GrokBotColor,
  type GrokBotFace,
  type GrokBotMove,
  type GrokBotShape,
} from '../grokBotStudio';
import {
  GrokBotAnimator,
  type GrokBotRenderFrame,
  staticGrokBotFrame,
  subscribeGrokBotFrames,
} from '../grokBotMotion';

export interface GrokBotSvgProps {
  shape: GrokBotShape;
  face: GrokBotFace;
  color: GrokBotColor;
  move?: GrokBotMove;
  /** Bump to (re)play `move` once, the way a click on a Move tile does on grokbots.ai. */
  playKey?: number;
  variant?: 'tile' | 'stage' | 'avatar';
  trackCursor?: boolean;
  /** Freeze the move's mid-performance pose (Move tiles): no animation loop at all. */
  staticMovePreview?: boolean;
  className?: string;
}

function prefersReducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

export default function GrokBotSvg({
  shape,
  face,
  color,
  move = 'idle',
  playKey = 0,
  variant = 'tile',
  trackCursor = false,
  staticMovePreview = false,
  className = '',
}: GrokBotSvgProps) {
  const rawId = useId();
  const uid = useMemo(() => `gb-${rawId.replace(/[^a-zA-Z0-9_-]/g, '')}`, [rawId]);
  const isStage = variant === 'stage';
  const half = isStage ? 158 : 125;
  const span = half * 2;
  const gradCx = isStage ? -50.56 : -40;
  const gradCy = isStage ? -63.2 : -50;
  const gradR = isStage ? 244.9 : 193.75;
  const palette = GROKBOT_COLOR_PALETTES[color];

  const svgRef = useRef<SVGSVGElement | null>(null);
  const maskBodyRef = useRef<SVGPathElement | null>(null);
  const backingBodyRef = useRef<SVGPathElement | null>(null);
  const bodyWrapRef = useRef<SVGGElement | null>(null);
  const leftEyeRef = useRef<SVGPathElement | null>(null);
  const rightEyeRef = useRef<SVGPathElement | null>(null);
  const notifCutRef = useRef<SVGCircleElement | null>(null);
  const notifBadgeRef = useRef<SVGCircleElement | null>(null);
  const backLayerRef = useRef<SVGGElement | null>(null);
  const frontLayerRef = useRef<SVGGElement | null>(null);

  const animatorRef = useRef<GrokBotAnimator | null>(null);
  if (!animatorRef.current) animatorRef.current = new GrokBotAnimator(uid);
  const pointerRef = useRef<{ x: number; y: number } | null>(null);

  // Shape / face / colour tiles and Move tiles are stills, like the reference picker.
  const isStatic = staticMovePreview || (variant === 'tile' && !trackCursor && move === 'idle');
  const reducedMotion = useMemo(prefersReducedMotion, []);

  const livePropsRef = useRef({ shape, face, color, trackCursor, reducedMotion });
  livePropsRef.current = { shape, face, color, trackCursor, reducedMotion };

  const initialFrame = useMemo<GrokBotRenderFrame>(
    () => staticGrokBotFrame(shape, face, color, staticMovePreview ? move : 'idle', uid),
    [shape, face, color, move, staticMovePreview, uid],
  );

  // Each new playKey (or a newly chosen move) performs the move once, then the bot settles to idle.
  const lastPlayRef = useRef<{ move: GrokBotMove; key: number } | null>(null);
  useEffect(() => {
    if (isStatic) return;
    const prev = lastPlayRef.current;
    lastPlayRef.current = { move, key: playKey };
    if (!prev && move === 'idle') return;
    if (prev && prev.move === move && prev.key === playKey) return;
    animatorRef.current?.play(move, performance.now());
  }, [move, playKey, isStatic]);

  useEffect(() => {
    if (!trackCursor) {
      pointerRef.current = null;
      return;
    }
    let cachedRect: DOMRect | null = null;
    let lastRectMs = 0;
    const invalidateRect = () => {
      cachedRect = null;
    };
    const onMove = (e: PointerEvent) => {
      const el = svgRef.current;
      if (!el) return;
      const now = performance.now();
      if (!cachedRect || now - lastRectMs > 250) {
        cachedRect = el.getBoundingClientRect();
        lastRectMs = now;
      }
      if (cachedRect.width <= 0 || cachedRect.height <= 0) return;
      const cx = cachedRect.left + cachedRect.width * 0.5;
      const cy = cachedRect.top + cachedRect.height * 0.5;
      pointerRef.current = {
        x: Math.max(-1, Math.min(1, (e.clientX - cx) / Math.max(220, window.innerWidth * 0.38))),
        y: Math.max(-1, Math.min(1, (e.clientY - cy) / Math.max(220, window.innerHeight * 0.38))),
      };
    };
    window.addEventListener('pointermove', onMove, { passive: true });
    window.addEventListener('scroll', invalidateRect, { passive: true });
    window.addEventListener('resize', invalidateRect, { passive: true });
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('scroll', invalidateRect);
      window.removeEventListener('resize', invalidateRect);
    };
  }, [trackCursor]);

  useEffect(() => {
    if (isStatic) return;
    const svg = svgRef.current;
    const animator = animatorRef.current;
    if (!svg || !animator) return;
    // Under reduced motion an avatar stays still; the stage still answers clicks and the cursor.
    if (reducedMotion && variant === 'avatar' && !trackCursor) return;

    let prevBack = '';
    let prevFront = '';
    const setVisible = (el: SVGElement | null, on: boolean) => {
      if (el) el.style.display = on ? '' : 'none';
    };

    const apply = (now: number) => {
      const cur = livePropsRef.current;
      const f = animator.frame(now, {
        shape: cur.shape,
        face: cur.face,
        color: cur.color,
        uid,
        pointer: cur.trackCursor ? pointerRef.current ?? { x: 0, y: 0 } : null,
        ambient: !cur.reducedMotion,
      });
      if (f.bodyD) {
        maskBodyRef.current?.setAttribute('d', f.bodyD);
        backingBodyRef.current?.setAttribute('d', f.bodyD);
      }
      const wrap = bodyWrapRef.current;
      if (wrap) {
        if (f.wrapTransform) wrap.setAttribute('transform', f.wrapTransform);
        else wrap.removeAttribute('transform');
      }
      [leftEyeRef.current, rightEyeRef.current].forEach((el, i) => {
        if (!el) return;
        const e = f.eyes[i];
        // React may rewrite `d` on a face change, so compare with the DOM rather than a cache.
        if (el.getAttribute('d') !== e.d) el.setAttribute('d', e.d);
        el.setAttribute('transform', e.transform);
        el.setAttribute('opacity', f.eyeOpacity.toFixed(3));
        setVisible(el, f.eyeOpacity > 0.01);
      });
      const cut = notifCutRef.current;
      const badge = notifBadgeRef.current;
      if (f.badge) {
        const { cx, cy, r } = f.badge;
        cut?.setAttribute('cx', cx.toFixed(2));
        cut?.setAttribute('cy', cy.toFixed(2));
        cut?.setAttribute('r', (r + 5.4).toFixed(2));
        badge?.setAttribute('cx', cx.toFixed(2));
        badge?.setAttribute('cy', cy.toFixed(2));
        badge?.setAttribute('r', r.toFixed(2));
      }
      setVisible(cut, !!f.badge);
      setVisible(badge, !!f.badge);
      if (backLayerRef.current && f.back !== prevBack) {
        backLayerRef.current.innerHTML = f.back;
        prevBack = f.back;
      }
      if (frontLayerRef.current && f.front !== prevFront) {
        frontLayerRef.current.innerHTML = f.front;
        prevFront = f.front;
      }
    };

    let unsubscribe: (() => void) | null = null;
    const start = () => {
      if (!unsubscribe) unsubscribe = subscribeGrokBotFrames(apply);
    };
    const stop = () => {
      unsubscribe?.();
      unsubscribe = null;
    };

    // Avatars in long lists only animate while on screen.
    let io: IntersectionObserver | null = null;
    if (typeof IntersectionObserver !== 'undefined') {
      io = new IntersectionObserver((entries) => {
        if (entries.some((e) => e.isIntersecting)) start();
        else stop();
      });
      io.observe(svg);
    } else {
      start();
    }
    return () => {
      io?.disconnect();
      stop();
    };
  }, [isStatic, uid, variant, trackCursor, reducedMotion]);

  const backingFill =
    color === 'cream'
      ? 'var(--grok-cream-eye)'
      : color === 'ink'
        ? 'var(--grok-ink-eye)'
        : variant === 'stage'
          ? 'var(--grok-stage-bg, var(--bg))'
          : 'var(--grok-tile-eye, var(--bg))';
  const eyesShown = initialFrame.eyeOpacity > 0.01;
  const badge = initialFrame.badge;

  return (
    <svg
      ref={svgRef}
      viewBox={`${-half} ${-half} ${span} ${span}`}
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      focusable="false"
      className={`grokbot-svg ${className}`.trim()}
    >
      <defs>
        <mask
          id={`${uid}-mask`}
          maskUnits="userSpaceOnUse"
          x={-half}
          y={-half}
          width={span}
          height={span}
        >
          <path ref={maskBodyRef} d={initialFrame.bodyD ?? ''} fill="white" />
          <path
            ref={leftEyeRef}
            d={initialFrame.eyes[0].d}
            fill="black"
            transform={initialFrame.eyes[0].transform}
            style={{ display: eyesShown ? undefined : 'none' }}
          />
          <path
            ref={rightEyeRef}
            d={initialFrame.eyes[1].d}
            fill="black"
            transform={initialFrame.eyes[1].transform}
            style={{ display: eyesShown ? undefined : 'none' }}
          />
          <circle
            ref={notifCutRef}
            cx={badge?.cx ?? 74.32}
            cy={badge?.cy ?? -67.07}
            r={(badge?.r ?? 15) + 5.4}
            fill="black"
            style={{ display: badge ? undefined : 'none' }}
          />
        </mask>
        <radialGradient
          id={`${uid}-fill`}
          gradientUnits="userSpaceOnUse"
          cx={gradCx}
          cy={gradCy}
          r={gradR}
        >
          <stop offset="0%" stopColor={palette.stops[0]} />
          <stop offset="32%" stopColor={palette.stops[1]} />
          <stop offset="72%" stopColor={palette.stops[2]} />
          <stop offset="100%" stopColor={palette.stops[3]} />
        </radialGradient>
        {GROKBOT_SVG_META.swirlStops.map((stops, idx) => (
          <linearGradient
            key={idx}
            id={`${uid}-sw${idx}`}
            x1="-120"
            y1="0"
            x2="120"
            y2="0"
            gradientUnits="userSpaceOnUse"
          >
            <stop offset="0%" stopColor={stops[0]} />
            <stop offset="50%" stopColor={stops[1]} />
            <stop offset="100%" stopColor={stops[2]} />
          </linearGradient>
        ))}
        {(['rg', 'cm'] as const).flatMap((kind) =>
          (kind === 'rg' ? GROKBOT_SVG_META.rings : GROKBOT_SVG_META.comet).map((g, idx) => (
            <linearGradient
              key={`${kind}${idx}`}
              id={`${uid}-${kind}${idx}`}
              x1={g.x1}
              y1={g.y1}
              x2={g.x2}
              y2={g.y2}
              gradientUnits="userSpaceOnUse"
            >
              <stop offset="0" stopColor={g.stops[0]} />
              <stop offset="0.5" stopColor={g.stops[1]} />
              <stop offset="1" stopColor={g.stops[2]} />
            </linearGradient>
          )),
        )}
      </defs>

      <g
        ref={backLayerRef}
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
        dangerouslySetInnerHTML={isStatic && initialFrame.back ? { __html: initialFrame.back } : undefined}
      />

      <g ref={bodyWrapRef}>
        <path ref={backingBodyRef} d={initialFrame.bodyD ?? ''} fill={backingFill} />
        <g mask={`url(#${uid}-mask)`}>
          <rect x={-half} y={-half} width={span} height={span} fill={`url(#${uid}-fill)`} />
        </g>
      </g>

      <circle
        ref={notifBadgeRef}
        cx={badge?.cx ?? 74.32}
        cy={badge?.cy ?? -67.07}
        r={badge?.r ?? 15}
        fill={GROKBOT_SVG_META.notifBlue}
        style={{ display: badge ? undefined : 'none' }}
      />

      <g
        ref={frontLayerRef}
        fill="none"
        strokeLinecap="round"
        strokeLinejoin="round"
        dangerouslySetInnerHTML={isStatic && initialFrame.front ? { __html: initialFrame.front } : undefined}
      />
    </svg>
  );
}

export function GrokBotStageBackdrop({ color }: { color: GrokBotColor }) {
  const uid = useId().replace(/[^a-zA-Z0-9_-]/g, '');
  const palette = GROKBOT_COLOR_PALETTES[color];
  return (
    <svg className="grok-studio-backdrop-svg" aria-hidden="true" focusable="false">
      <defs>
        <pattern id={`${uid}-dots`} width="20" height="20" patternUnits="userSpaceOnUse">
          <circle cx="10" cy="10" r="1.2" fill="var(--grok-stage-dot)" />
        </pattern>
        <radialGradient id={`${uid}-glow`} cx="50%" cy="48%" r="62%">
          <stop offset="0%" stopColor={palette.hex} stopOpacity="0.18" />
          <stop offset="100%" stopColor={palette.hex} stopOpacity="0" />
        </radialGradient>
      </defs>
      <rect width="100%" height="100%" fill={`url(#${uid}-dots)`} />
      <rect width="100%" height="100%" fill={`url(#${uid}-glow)`} />
    </svg>
  );
}
