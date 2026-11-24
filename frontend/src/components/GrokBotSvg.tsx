import { useEffect, useId, useMemo, useRef } from 'react';
import {
  buildEyeMatrix,
  computeFaceEyeMatrices,
  coordsToBezier64Path,
  GROKBOT_COLOR_PALETTES,
  GROKBOT_FACE_DEFS,
  GROKBOT_SHAPE_PATHS,
  GROKBOT_SVG_META,
  type GrokBotColor,
  type GrokBotFace,
  type GrokBotMove,
  type GrokBotShape,
  lerpCoords,
  MOVE_EYE_PATHS,
  MOVE_TARGET_COORDS,
  SHAPE_COORDS,
} from '../grokBotStudio';

export interface GrokBotSvgProps {
  shape: GrokBotShape;
  face: GrokBotFace;
  color: GrokBotColor;
  move?: GrokBotMove;
  variant?: 'tile' | 'stage' | 'avatar';
  trackCursor?: boolean;
  /** Static snapshot phase (0..1) for Move option tiles so 15 tiles render rich previews at 0% idle CPU. */
  staticMovePreview?: boolean;
  className?: string;
}

interface ExtraRender {
  bodyPath: string;
  bodyOpacity: number;
  leftD: string | null;
  leftTransform: string;
  rightD: string | null;
  rightTransform: string;
  notifCutout: { cx: number; cy: number; r: number } | null;
  notifBadge: { cx: number; cy: number; r: number } | null;
  backHtml: string;
  frontHtml: string;
}

function easeInOutSine(x: number): number {
  return -(Math.cos(Math.PI * x) - 1) / 2;
}

function computeFrame(
  shape: GrokBotShape,
  face: GrokBotFace,
  color: GrokBotColor,
  move: GrokBotMove,
  tSec: number,
  gazeX: number,
  gazeY: number,
  uid: string,
  morphFromCoords: Float32Array | null,
  morphAlpha: number,
  scratch: Float32Array,
  baseSnapshot: Float32Array,
): ExtraRender {
  const palette = GROKBOT_COLOR_PALETTES[color];
  const baseHex = color === 'ink' ? GROKBOT_SVG_META.inkStroke : palette.hex;
  const targetBase = SHAPE_COORDS[shape];

  // Interpolate user shape transitions smoothly
  if (morphFromCoords && morphAlpha < 1) {
    lerpCoords(baseSnapshot, morphFromCoords, targetBase, morphAlpha);
  } else {
    baseSnapshot.set(targetBase);
  }

  const faceDef = GROKBOT_FACE_DEFS[face];
  // Periodic natural blink every 3.8s
  const blinkCycle = tSec % 3.8;
  const autoBlink =
    blinkCycle > 3.45 && blinkCycle < 3.65
      ? Math.max(0.1, Math.abs((blinkCycle - 3.55) / 0.1))
      : 1;

  if (move === 'idle') {
    const breath = 1 + Math.sin(tSec * 2.1) * 0.014;
    const bobY = Math.sin(tSec * 2.1) * -2.2;
    lerpCoords(scratch, baseSnapshot, baseSnapshot, 0, 0, bobY, 0, breath);
    const [lMat, rMat] = computeFaceEyeMatrices(face, gazeX, gazeY, autoBlink, bobY);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: faceDef.leftPath,
      leftTransform: lMat,
      rightD: faceDef.rightPath,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: '',
    };
  }

  if (move === 'thinking') {
    const dots = [-54, 0, 54]
      .map((cx, idx) => {
        const phase = (tSec * 2.6 - idx * 0.32) % (Math.PI * 2);
        const w = (Math.sin(phase) + 1) * 0.5;
        const r = (14.5 + w * 6.2).toFixed(2);
        const op = (0.48 + w * 0.52).toFixed(2);
        const cy = (Math.sin(phase) * -4.5).toFixed(2);
        return `<circle cx="${cx}" cy="${cy}" r="${r}" fill="url(#${uid}-fill)" opacity="${op}" />`;
      })
      .join('');
    return {
      bodyPath: coordsToBezier64Path(baseSnapshot),
      bodyOpacity: 0.16,
      leftD: null,
      leftTransform: '',
      rightD: null,
      rightTransform: '',
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: dots,
    };
  }

  if (move === 'wink') {
    const cyc = (tSec % 2.2) / 2.2;
    const winking = cyc > 0.18 && cyc < 0.68;
    const tilt = winking ? 0.09 * Math.sin(((cyc - 0.18) / 0.5) * Math.PI) : 0;
    const bob = winking ? -4 * Math.sin(((cyc - 0.18) / 0.5) * Math.PI) : 0;
    lerpCoords(scratch, baseSnapshot, baseSnapshot, 0, 0, bob, tilt, 1);
    const lMat = buildEyeMatrix(-26 + gazeX * 14, -8 + gazeY * 10 + bob, -8, 1);
    const rMat = buildEyeMatrix(28 + gazeX * 14, -11 + gazeY * 10 + bob, winking ? -6 : -8, 1);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: MOVE_EYE_PATHS.winkLeft,
      leftTransform: lMat,
      rightD: winking ? MOVE_EYE_PATHS.winkRightBar : MOVE_EYE_PATHS.winkLeft,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: '',
    };
  }

  if (move === 'wide_eyes') {
    const pulse = (Math.sin(tSec * 4.2) + 1) * 0.5;
    const sc = 1 + pulse * 0.035;
    const jitX = Math.sin(tSec * 14) * 1.1;
    lerpCoords(scratch, baseSnapshot, baseSnapshot, 0, jitX, -pulse * 3, 0, sc);
    const eyeSy = 0.88 + pulse * 0.16;
    const lMat = buildEyeMatrix(-28 + gazeX * 16 + jitX, -2 + gazeY * 10, -3, eyeSy);
    const rMat = buildEyeMatrix(28 + gazeX * 16 + jitX, -2 + gazeY * 10, 3, eyeSy);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: MOVE_EYE_PATHS.wideEye,
      leftTransform: lMat,
      rightD: MOVE_EYE_PATHS.wideEye,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: '',
    };
  }

  if (move === 'alert') {
    const w = easeInOutSine((Math.sin(tSec * 3.4) + 1) * 0.5);
    const morph = 0.78 + w * 0.22;
    const sway = Math.sin(tSec * 6.8) * 6;
    lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.alertStem, morph, sway, -10 - w * 5, 0, 1);
    const dotX = (-14 + sway * 0.85).toFixed(2);
    const dotY = (54 - w * 3).toFixed(2);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: null,
      leftTransform: '',
      rightD: null,
      rightTransform: '',
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: `<circle cx="${dotX}" cy="${dotY}" r="13.5" fill="url(#${uid}-fill)" />`,
    };
  }

  if (move === 'notification') {
    const pop = (Math.sin(tSec * 3.6) + 1) * 0.5;
    const badgeR = 13.5 + pop * 2.8;
    const bob = -pop * 3;
    lerpCoords(scratch, baseSnapshot, SHAPE_COORDS.circle, 0.35, 0, bob, 0, 1);
    const lMat = buildEyeMatrix(-28 + gazeX * 14 + 4, -2 + bob, 0, 1);
    const rMat = buildEyeMatrix(30 + gazeX * 14 + 4, -4 + bob, 0, 1);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: MOVE_EYE_PATHS.notificationEye,
      leftTransform: lMat,
      rightD: MOVE_EYE_PATHS.notificationEye,
      rightTransform: rMat,
      notifCutout: { cx: 74.32, cy: -67.07 + bob, r: 20.4 },
      notifBadge: { cx: 74.32, cy: -67.07 + bob, r: badgeR },
      backHtml: '',
      frontHtml: '',
    };
  }

  if (move === 'exclamation') {
    const bounce = Math.abs(Math.sin(tSec * 3.5));
    lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.exclamationStem, 0.92, 0, -bounce * 9, 0, 1);
    const dotY = (53 - bounce * 4).toFixed(2);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: null,
      leftTransform: '',
      rightD: null,
      rightTransform: '',
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: `<circle cx="-1" cy="${dotY}" r="11.8" fill="url(#${uid}-fill)" />`,
    };
  }

  if (move === 'sleep') {
    const breath = (Math.sin(tSec * 1.6) + 1) * 0.5;
    const sc = 0.92 + breath * 0.16;
    lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.sleepCore, 0.95, 0, 0, 0, sc);
    const zSpecs: [number, number, number][] = [
      [-4, -6, 16],
      [18, -38, 22],
      [-8, -72, 28],
    ];
    const zPaths = zSpecs
      .map(([zx, zy, zw], idx) => {
        const cyc = ((tSec * 0.35 + idx * 0.28) % 1);
        const driftY = cyc * -14;
        const op = Math.max(0.2, Math.sin(cyc * Math.PI)).toFixed(2);
        const x0 = (zx - zw * 0.4).toFixed(1);
        const x1 = (zx + zw * 0.4).toFixed(1);
        const y0 = (zy + driftY - zw * 0.35).toFixed(1);
        const y1 = (zy + driftY + zw * 0.35).toFixed(1);
        const sw = (5.2 + idx * 1.1).toFixed(1);
        return `<path d="M${x0} ${y0} L${x1} ${y0} L${x0} ${y1} L${x1} ${y1}" stroke="${baseHex}" stroke-width="${sw}" stroke-linejoin="round" opacity="${op}" />`;
      })
      .join('');
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: null,
      leftTransform: '',
      rightD: null,
      rightTransform: '',
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: zPaths,
    };
  }

  if (move === 'egg') {
    const rock = Math.sin(tSec * 2.5) * 0.11;
    const eggBlink = (tSec % 2.6) > 2.3 ? 0.32 : 0.96;
    lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.egg, 0.92, 0, 0, rock, 1);
    const lMat = buildEyeMatrix(-23 + rock * 40 + gazeX * 12, -8 + gazeY * 10, (rock * 180) / Math.PI, eggBlink);
    const rMat = buildEyeMatrix(23 + rock * 40 + gazeX * 12, -8 + gazeY * 10, (rock * 180) / Math.PI, eggBlink);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: MOVE_EYE_PATHS.eggEye,
      leftTransform: lMat,
      rightD: MOVE_EYE_PATHS.eggEye,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: '',
    };
  }

  if (move === 'hexagon') {
    const rock = Math.sin(tSec * 2.2) * 0.14;
    const bob = Math.cos(tSec * 4.4) * 3.5;
    lerpCoords(scratch, baseSnapshot, SHAPE_COORDS.hexagon, 0.94, 0, bob, rock, 0.92);
    const lMat = buildEyeMatrix(-24 + gazeX * 14, -8 + bob + gazeY * 10, (rock * 150) / Math.PI, autoBlink);
    const rMat = buildEyeMatrix(24 + gazeX * 14, -8 + bob + gazeY * 10, (rock * 150) / Math.PI, autoBlink);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: MOVE_EYE_PATHS.hexEye,
      leftTransform: lMat,
      rightD: MOVE_EYE_PATHS.hexEye,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: '',
    };
  }

  if (move === 'play') {
    const bob = Math.sin(tSec * 3.0) * 4;
    lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.play, 0.92, 0, bob, 0, 0.88);
    const phase = tSec * 2.4;
    const ribbon = (idx: number, yOff: number, amp: number) => {
      const s1 = Math.sin(phase + idx * 0.9) * amp;
      const s2 = Math.cos(phase + idx * 1.1) * amp;
      return `<path d="M-132 ${(yOff + s1).toFixed(1)} C-55 ${(yOff - s2).toFixed(1)}, 45 ${(yOff + s2).toFixed(1)}, 132 ${(yOff - s1).toFixed(1)}" stroke="url(#${uid}-sw${idx})" stroke-width="5.2" opacity="0.9" />`;
    };
    const back = ribbon(0, -46, 18) + ribbon(1, 48, 18);
    const front = ribbon(2, -18, 14) + ribbon(3, 22, 14);
    const lMat = buildEyeMatrix(-16 + gazeX * 10, -4 + bob, 0, autoBlink);
    const rMat = buildEyeMatrix(22 + gazeX * 10, -4 + bob, 0, autoBlink);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: MOVE_EYE_PATHS.smallPlayEye,
      leftTransform: lMat,
      rightD: MOVE_EYE_PATHS.smallPlayEye,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: back,
      frontHtml: front,
    };
  }

  if (move === 'orbit') {
    const ang = tSec * 2.1;
    const ox = Math.cos(ang) * 34;
    const oy = Math.sin(ang) * 20;
    lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.play, 0.88, ox, oy, ang + Math.PI * 0.5, 0.68);
    let back = '';
    let front = '';
    for (let i = 0; i < 4; i++) {
      const tilt = -24 + i * 16 + Math.sin(tSec * 1.4 + i) * 6;
      const rx = 108 + i * 8;
      const ry = 36 + i * 5;
      // Back half-arc (top)
      back += `<path d="M-${rx} 0 A${rx} ${ry} 0 0 1 ${rx} 0" transform="rotate(${tilt.toFixed(1)})" stroke="url(#${uid}-sw${i})" stroke-width="5.4" opacity="0.72" />`;
      // Front half-arc (bottom)
      front += `<path d="M${rx} 0 A${rx} ${ry} 0 0 1 -${rx} 0" transform="rotate(${tilt.toFixed(1)})" stroke="url(#${uid}-sw${i})" stroke-width="5.6" opacity="0.95" />`;
    }
    const lMat = buildEyeMatrix(ox - 13, oy - 3, 0, 0.85);
    const rMat = buildEyeMatrix(ox + 13, oy - 3, 0, 0.85);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: MOVE_EYE_PATHS.smallPlayEye,
      leftTransform: lMat,
      rightD: MOVE_EYE_PATHS.smallPlayEye,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: back,
      frontHtml: front,
    };
  }

  if (move === 'swirl') {
    const bob = Math.sin(tSec * 2.4) * 3.5;
    lerpCoords(scratch, baseSnapshot, SHAPE_COORDS.circle, 0.6, 0, bob, 0, 0.96);
    const rotBase = (tSec * 72) % 360;
    let back = '';
    let front = '';
    for (let i = 0; i < 3; i++) {
      const deg = (rotBase + i * 120).toFixed(1);
      back += `<path d="M-118 -24 C-70 -128, 70 -128, 118 -24" transform="rotate(${deg})" stroke="url(#${uid}-sw${i})" stroke-width="5.6" opacity="0.78" />`;
      front += `<path d="M118 24 C70 128, -70 128, -118 24" transform="rotate(${deg})" stroke="url(#${uid}-sw${(i + 1) % 4})" stroke-width="5.8" opacity="0.95" />`;
    }
    const [lMat, rMat] = computeFaceEyeMatrices('neutral', gazeX, gazeY, autoBlink);
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: GROKBOT_FACE_DEFS.neutral.leftPath,
      leftTransform: lMat,
      rightD: GROKBOT_FACE_DEFS.neutral.rightPath,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: back,
      frontHtml: front,
    };
  }

  if (move === 'burst') {
    const cyc = (tSec * 0.85) % 1;
    const coreSc = 0.85 + Math.sin(cyc * Math.PI) * 0.35;
    lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.burstCore, 0.95, 0, 0, 0, coreSc);
    let particles = '';
    const count = 14;
    for (let i = 0; i < count; i++) {
      const pCyc = (cyc + (i % 2) * 0.35) % 1;
      const dist = 24 + easeInOutSine(pCyc) * 96;
      const ang = (i / count) * Math.PI * 2 + tSec * 0.35;
      const px = (Math.cos(ang) * dist).toFixed(1);
      const py = (Math.sin(ang) * dist).toFixed(1);
      const pr = ((1 - pCyc * 0.55) * 6.8).toFixed(2);
      const op = Math.max(0.12, Math.sin(pCyc * Math.PI)).toFixed(2);
      const col = i % 2 === 0 ? baseHex : palette.stops[1];
      particles += `<circle cx="${px}" cy="${py}" r="${pr}" fill="${col}" opacity="${op}" />`;
    }
    return {
      bodyPath: coordsToBezier64Path(scratch),
      bodyOpacity: 1,
      leftD: null,
      leftTransform: '',
      rightD: null,
      rightTransform: '',
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: particles,
    };
  }

  // 'comet'
  const cyc = (tSec * 0.9) % 1;
  const headX = Math.cos(cyc * Math.PI * 2) * 62;
  const headY = Math.sin(cyc * Math.PI * 2) * 38;
  lerpCoords(scratch, baseSnapshot, MOVE_TARGET_COORDS.cometHead, 0.96, headX, headY, 0, 1);
  let tails = '';
  for (let i = 0; i < 4; i++) {
    const lag1 = cyc - 0.08 - i * 0.025;
    const lag2 = cyc - 0.22 - i * 0.035;
    const c1x = (Math.cos(lag1 * Math.PI * 2) * (62 + (i - 1.5) * 9)).toFixed(1);
    const c1y = (Math.sin(lag1 * Math.PI * 2) * (38 + (i - 1.5) * 9)).toFixed(1);
    const tx = (Math.cos(lag2 * Math.PI * 2) * (62 + (i - 1.5) * 14)).toFixed(1);
    const ty = (Math.sin(lag2 * Math.PI * 2) * (38 + (i - 1.5) * 14)).toFixed(1);
    tails += `<path d="M${headX.toFixed(1)} ${headY.toFixed(1)} Q${c1x} ${c1y} ${tx} ${ty}" stroke="url(#${uid}-sw${i})" stroke-width="9.2" opacity="${(0.92 - i * 0.14).toFixed(2)}" />`;
  }
  return {
    bodyPath: coordsToBezier64Path(scratch),
    bodyOpacity: 1,
    leftD: null,
    leftTransform: '',
    rightD: null,
    rightTransform: '',
    notifCutout: null,
    notifBadge: null,
    backHtml: tails,
    frontHtml: '',
  };
}

export default function GrokBotSvg({
  shape,
  face,
  color,
  move = 'idle',
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

  const pointerTargetRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  const prevShapeRef = useRef<GrokBotShape>(shape);
  const morphStateRef = useRef<{ from: Float32Array | null; startMs: number }>({
    from: null,
    startMs: 0,
  });

  useEffect(() => {
    if (prevShapeRef.current !== shape) {
      morphStateRef.current = {
        from: new Float32Array(SHAPE_COORDS[prevShapeRef.current]),
        startMs: performance.now(),
      };
      prevShapeRef.current = shape;
    }
  }, [shape]);

  useEffect(() => {
    if (!trackCursor) {
      pointerTargetRef.current = { x: 0, y: 0 };
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
      const normX = Math.max(-1, Math.min(1, (e.clientX - cx) / Math.max(220, window.innerWidth * 0.38)));
      const normY = Math.max(-1, Math.min(1, (e.clientY - cy) / Math.max(220, window.innerHeight * 0.38)));
      pointerTargetRef.current = { x: normX, y: normY };
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

  const isStaticTile =
    (variant === 'tile' && !trackCursor && (move === 'idle' || staticMovePreview)) ||
    (variant === 'avatar' && !trackCursor && move === 'idle');

  const livePropsRef = useRef({ shape, face, color, move, trackCursor });
  livePropsRef.current = { shape, face, color, move, trackCursor };

  const initialFrame = useMemo(() => {
    const scratch = new Float32Array(386);
    const snap = new Float32Array(386);
    if (staticMovePreview && move !== 'idle') {
      return computeFrame(shape, face, color, move, 0.55, 0, 0, uid, null, 1, scratch, snap);
    }
    const [lMat, rMat] = computeFaceEyeMatrices(face, 0, 0, 1);
    const def = GROKBOT_FACE_DEFS[face];
    return {
      bodyPath: GROKBOT_SHAPE_PATHS[shape],
      bodyOpacity: 1,
      leftD: def.leftPath,
      leftTransform: lMat,
      rightD: def.rightPath,
      rightTransform: rMat,
      notifCutout: null,
      notifBadge: null,
      backHtml: '',
      frontHtml: '',
    } satisfies ExtraRender;
  }, [shape, face, color, move, staticMovePreview, uid]);

  useEffect(() => {
    if (isStaticTile) return;
    const reducedMotion =
      typeof window !== 'undefined' &&
      typeof window.matchMedia === 'function' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reducedMotion && livePropsRef.current.move === 'idle' && !livePropsRef.current.trackCursor) return;

    let rafId = 0;
    const start = performance.now();
    const scratch = new Float32Array(386);
    const snap = new Float32Array(386);
    let smoothGx = 0;
    let smoothGy = 0;
    let prevBack = '';
    let prevFront = '';

    const tick = (nowMs: number) => {
      if (typeof document !== 'undefined' && document.hidden) {
        rafId = requestAnimationFrame(tick);
        return;
      }
      const cur = livePropsRef.current;
      const tSec = (nowMs - start) / 1000;
      const target = pointerTargetRef.current;
      const driftX = cur.trackCursor ? Math.sin(tSec * 0.7) * 0.06 : Math.sin(tSec * 0.85) * 0.14;
      const driftY = cur.trackCursor ? Math.cos(tSec * 0.55) * 0.05 : Math.cos(tSec * 0.65) * 0.1;
      smoothGx += (target.x + driftX - smoothGx) * 0.16;
      smoothGy += (target.y + driftY - smoothGy) * 0.16;

      let morphFrom: Float32Array | null = null;
      let morphAlpha = 1;
      if (morphStateRef.current.from) {
        const elapsed = (nowMs - morphStateRef.current.startMs) / 260;
        if (elapsed >= 1) {
          morphStateRef.current.from = null;
        } else {
          morphFrom = morphStateRef.current.from;
          morphAlpha = 1 - Math.pow(1 - elapsed, 3);
        }
      }

      const frame = computeFrame(
        cur.shape,
        cur.face,
        cur.color,
        cur.move,
        tSec,
        smoothGx,
        smoothGy,
        uid,
        morphFrom,
        morphAlpha,
        scratch,
        snap,
      );

      maskBodyRef.current?.setAttribute('d', frame.bodyPath);
      backingBodyRef.current?.setAttribute('d', frame.bodyPath);
      if (bodyWrapRef.current) {
        bodyWrapRef.current.setAttribute('opacity', String(frame.bodyOpacity));
      }

      if (leftEyeRef.current) {
        if (frame.leftD) {
          leftEyeRef.current.setAttribute('d', frame.leftD);
          leftEyeRef.current.setAttribute('transform', frame.leftTransform);
          leftEyeRef.current.style.display = '';
        } else {
          leftEyeRef.current.style.display = 'none';
        }
      }
      if (rightEyeRef.current) {
        if (frame.rightD) {
          rightEyeRef.current.setAttribute('d', frame.rightD);
          rightEyeRef.current.setAttribute('transform', frame.rightTransform);
          rightEyeRef.current.style.display = '';
        } else {
          rightEyeRef.current.style.display = 'none';
        }
      }

      if (notifCutRef.current) {
        if (frame.notifCutout) {
          notifCutRef.current.setAttribute('cx', frame.notifCutout.cx.toFixed(2));
          notifCutRef.current.setAttribute('cy', frame.notifCutout.cy.toFixed(2));
          notifCutRef.current.setAttribute('r', frame.notifCutout.r.toFixed(2));
          notifCutRef.current.style.display = '';
        } else {
          notifCutRef.current.style.display = 'none';
        }
      }
      if (notifBadgeRef.current) {
        if (frame.notifBadge) {
          notifBadgeRef.current.setAttribute('cx', frame.notifBadge.cx.toFixed(2));
          notifBadgeRef.current.setAttribute('cy', frame.notifBadge.cy.toFixed(2));
          notifBadgeRef.current.setAttribute('r', frame.notifBadge.r.toFixed(2));
          notifBadgeRef.current.style.display = '';
        } else {
          notifBadgeRef.current.style.display = 'none';
        }
      }

      if (backLayerRef.current && frame.backHtml !== prevBack) {
        backLayerRef.current.innerHTML = frame.backHtml;
        prevBack = frame.backHtml;
      }
      if (frontLayerRef.current && frame.frontHtml !== prevFront) {
        frontLayerRef.current.innerHTML = frame.frontHtml;
        prevFront = frame.frontHtml;
      }

      rafId = requestAnimationFrame(tick);
    };

    rafId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafId);
  }, [isStaticTile, uid]);

  const showBacking = true;
  const backingFill =
    color === 'cream'
      ? 'var(--grok-cream-eye)'
      : color === 'ink'
        ? 'var(--grok-ink-eye)'
        : variant === 'stage'
          ? 'var(--grok-stage-bg, var(--bg))'
          : 'var(--grok-tile-eye, var(--bg))';

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
          <path ref={maskBodyRef} d={initialFrame.bodyPath} fill="white" />
          <path
            ref={leftEyeRef}
            d={initialFrame.leftD ?? ''}
            fill="black"
            transform={initialFrame.leftTransform}
            style={{ display: initialFrame.leftD ? undefined : 'none' }}
          />
          <path
            ref={rightEyeRef}
            d={initialFrame.rightD ?? ''}
            fill="black"
            transform={initialFrame.rightTransform}
            style={{ display: initialFrame.rightD ? undefined : 'none' }}
          />
          <circle
            ref={notifCutRef}
            cx={initialFrame.notifCutout?.cx ?? 74.32}
            cy={initialFrame.notifCutout?.cy ?? -67.07}
            r={initialFrame.notifCutout?.r ?? 20.4}
            fill="black"
            style={{ display: initialFrame.notifCutout ? undefined : 'none' }}
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
      </defs>

      <g
        ref={backLayerRef}
        fill="none"
        strokeLinecap="round"
        dangerouslySetInnerHTML={isStaticTile && initialFrame.backHtml ? { __html: initialFrame.backHtml } : undefined}
      />

      <g ref={bodyWrapRef} opacity={initialFrame.bodyOpacity}>
        {showBacking && <path ref={backingBodyRef} d={initialFrame.bodyPath} fill={backingFill} />}
        <g mask={`url(#${uid}-mask)`}>
          <rect x={-half} y={-half} width={span} height={span} fill={`url(#${uid}-fill)`} />
        </g>
      </g>

      <circle
        ref={notifBadgeRef}
        cx={initialFrame.notifBadge?.cx ?? 74.32}
        cy={initialFrame.notifBadge?.cy ?? -67.07}
        r={initialFrame.notifBadge?.r ?? 15}
        fill={GROKBOT_SVG_META.notifBlue}
        style={{ display: initialFrame.notifBadge ? undefined : 'none' }}
      />

      <g
        ref={frontLayerRef}
        fill="none"
        strokeLinecap="round"
        dangerouslySetInnerHTML={isStaticTile && initialFrame.frontHtml ? { __html: initialFrame.frontHtml } : undefined}
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
