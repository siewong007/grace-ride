/**
 * Grok Bot motion engine, modelled on the grokbots.ai studio stage.
 *
 * - Idle is alive: a slow vertical breath pinned at the body's base, a wandering gaze (or the
 *   cursor), and irregular blinks that are sometimes doubled.
 * - Every other move is a one-shot performance of roughly 1.2–2.9 s: the body morphs out of its
 *   current outline into the move's pose, performs, then morphs back to the living idle bot.
 *
 * Everything here is pure geometry and strings; `GrokBotSvg` applies the frames to the DOM.
 */
import {
  buildPolar64Coords,
  coordsToBezier64Path,
  eyePoseMatrix,
  faceEyePoses,
  GLYPH_COORDS,
  GROKBOT_ALERT_DOT_PATH,
  GROKBOT_COLOR_PALETTES,
  GROKBOT_SHAPE_EYE_LAYOUT,
  GROKBOT_SVG_META,
  type GrokBotColor,
  type GrokBotEyePose,
  type GrokBotFace,
  type GrokBotMove,
  type GrokBotShape,
  lerpCoords,
  MOVE_EYE_PATHS,
  PILL,
  SHAPE_COORDS,
} from './grokBotStudio';

const N = 386;
const TAU = Math.PI * 2;
const DEG = Math.PI / 180;

const clamp01 = (v: number) => (v < 0 ? 0 : v > 1 ? 1 : v);
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const easeOutCubic = (x: number) => 1 - Math.pow(1 - x, 3);
const easeInOutSine = (x: number) => -(Math.cos(Math.PI * x) - 1) / 2;
const easeOutQuart = (x: number) => 1 - Math.pow(1 - x, 4);
/** Piecewise-linear lookup over a flat [x0, y0, x1, y1, …] table, clamped at both ends. */
function pl(x: number, tbl: readonly number[]): number {
  if (x <= tbl[0]) return tbl[1];
  for (let i = 2; i < tbl.length; i += 2) {
    if (x <= tbl[i]) {
      const x0 = tbl[i - 2];
      const y0 = tbl[i - 1];
      return y0 + ((tbl[i + 1] - y0) * (x - x0)) / (tbl[i] - x0);
    }
  }
  return tbl[tbl.length - 1];
}
const easeOutBack = (x: number) => {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2);
};
/** 0 → 1 → 0 bump over |x| < 1. */
const bump = (x: number) => (Math.abs(x) >= 1 ? 0 : 0.5 * (1 + Math.cos(Math.PI * x)));
const f1 = (v: number) => v.toFixed(1);

/* ------------------------------------------------------------------ geometry */

/** Re-orders a closed 64-segment outline so it starts at the anchor nearest +X (keeps morphs untwisted). */
function rephase(c: Float32Array): Float32Array {
  let best = 0;
  let bestAng = Infinity;
  for (let i = 0; i < 64; i++) {
    const x = c[i === 0 ? 0 : 2 + (i - 1) * 6 + 4];
    const y = c[i === 0 ? 1 : 2 + (i - 1) * 6 + 5];
    const a = Math.abs(Math.atan2(y, x));
    if (a < bestAng) {
      bestAng = a;
      best = i;
    }
  }
  if (best === 0) return c;
  const out = new Float32Array(N);
  const anchor = (i: number) => (i === 0 ? [c[0], c[1]] : [c[2 + (i - 1) * 6 + 4], c[2 + (i - 1) * 6 + 5]]);
  const [sx, sy] = anchor(best);
  out[0] = sx;
  out[1] = sy;
  for (let k = 0; k < 64; k++) {
    const seg = (best + k) % 64;
    out.set(c.subarray(2 + seg * 6, 8 + seg * 6), 2 + k * 6);
  }
  return out;
}

/** Affine copy of an outline: non-uniform scale, then rotate, then translate. */
function affine(src: Float32Array, sx: number, sy: number, rotDeg: number, tx: number, ty: number): Float32Array {
  const out = new Float32Array(N);
  const cos = Math.cos(rotDeg * DEG);
  const sin = Math.sin(rotDeg * DEG);
  for (let i = 0; i < N; i += 2) {
    const x = src[i] * sx;
    const y = src[i + 1] * sy;
    out[i] = x * cos - y * sin + tx;
    out[i + 1] = x * sin + y * cos + ty;
  }
  return rephase(out);
}

const CIRCLE = SHAPE_COORDS.circle; // r ≈ 100
const TARGETS = {
  egg: buildPolar64Coords((theta) => {
    const y = Math.sin(theta);
    return [80 * (1 + 0.1 * y) * Math.cos(theta) + 1.5, 100.5 * y - 4.5];
  }),
  hexTall: affine(SHAPE_COORDS.hexagon, 0.97, 0.97, 90, 0, 3),
  play: affine(SHAPE_COORDS.triangle, 0.98, 0.98, 8, 0, 20),
};
/** Scratch outline for the orbit's spinning circle. */
const orbitCircle = new Float32Array(N);

/** Writes the circle scaled to radius `r` at (cx, cy). */
function dot(out: Float32Array, r: number, cx: number, cy: number): Float32Array {
  lerpCoords(out, CIRCLE, CIRCLE, 0, cx, cy, 0, r / 100);
  return out;
}

const bottomCache = new Map<GrokBotShape, number>();
function shapeBottom(shape: GrokBotShape): number {
  let b = bottomCache.get(shape);
  if (b === undefined) {
    const c = SHAPE_COORDS[shape];
    b = -Infinity;
    for (let i = 1; i < N; i += 2) b = Math.max(b, c[i]);
    bottomCache.set(shape, b);
  }
  return b;
}

/** Where a ray from the centre at -42° leaves the outline: the notification badge sits there. */
const badgeCache = new Map<GrokBotShape, [number, number]>();
export function notificationAnchor(shape: GrokBotShape): [number, number] {
  let p = badgeCache.get(shape);
  if (!p) {
    const c = SHAPE_COORDS[shape];
    const target = -42.06 * DEG;
    let best: [number, number] = [74.32, -67.07];
    let bestD = Infinity;
    // Sample along every cubic segment for an accurate hit.
    let x0 = c[0];
    let y0 = c[1];
    for (let s = 0; s < 64; s++) {
      const b = 2 + s * 6;
      for (let k = 1; k <= 8; k++) {
        const t = k / 8;
        const mt = 1 - t;
        const x = mt * mt * mt * x0 + 3 * mt * mt * t * c[b] + 3 * mt * t * t * c[b + 2] + t * t * t * c[b + 4];
        const y = mt * mt * mt * y0 + 3 * mt * mt * t * c[b + 1] + 3 * mt * t * t * c[b + 3] + t * t * t * c[b + 5];
        let d = Math.abs(Math.atan2(y, x) - target);
        if (d > Math.PI) d = TAU - d;
        if (d < bestD) {
          bestD = d;
          best = [x, y];
        }
      }
      x0 = c[b + 4];
      y0 = c[b + 5];
    }
    p = best;
    badgeCache.set(shape, p);
  }
  return p;
}

/* ------------------------------------------------------------------- colours */

function mixHex(a: string, b: string, t: number): string {
  const pa = parseInt(a.slice(1), 16);
  const pb = parseInt(b.slice(1), 16);
  const ch = (shift: number) => Math.round(lerp((pa >> shift) & 255, (pb >> shift) & 255, t));
  return `#${((1 << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0)).toString(16).slice(1)}`;
}

/* ------------------------------------------------------------------- strokes */

interface Ring {
  cx: number;
  cy: number;
  rx: number;
  ry: number;
  tilt: number;
  phase: number;
}

/** Splits an elliptical arc into the half in front of the body (lower) and the half behind it. */
function ringArc(r: Ring, from: number, to: number): { back: string; front: string } {
  let back = '';
  let front = '';
  if (to - from <= 0.002) return { back, front };
  const cos = Math.cos(r.tilt * DEG);
  const sin = Math.sin(r.tilt * DEG);
  const n = Math.max(2, Math.ceil(64 * (to - from)));
  let prev = '';
  let side: boolean | null = null;
  for (let i = 0; i <= n; i++) {
    const th = r.phase + TAU * (from + ((to - from) * i) / n);
    const lx = r.rx * Math.cos(th);
    const ly = r.ry * Math.sin(th);
    const pt = `${f1(lx * cos - ly * sin + r.cx)} ${f1(lx * sin + ly * cos + r.cy)}`;
    const isFront = Math.sin(th) > 0;
    if (isFront !== side) {
      const seg = prev ? `M${prev}L${pt}` : `M${pt}`;
      if (isFront) front += seg;
      else back += seg;
      side = isFront;
    } else if (isFront) front += `L${pt}`;
    else back += `L${pt}`;
    prev = pt;
  }
  return { back, front };
}

const strokePath = (d: string, uid: string, grad: number, width: number, opacity: number) =>
  d ? `<path d="${d}" stroke="url(#${uid}-sw${grad % 4})" stroke-width="${width}" opacity="${opacity.toFixed(2)}"/>` : '';

interface Ell {
  cx: number;
  cy: number;
  rx: number;
  ry: number;
  tilt: number;
}

/**
 * Arc of an ellipse over [u0, u1] turns (θ = 2πu in the ellipse's own frame), split into the half
 * behind the body and the half in front of it (cos θ < 0), the way the reference layers its strokes.
 */
function ellArc(e: Ell, u0: number, u1: number): { back: string; front: string } {
  let back = '';
  let front = '';
  if (u1 - u0 <= 0.002) return { back, front };
  const cos = Math.cos(e.tilt * DEG);
  const sin = Math.sin(e.tilt * DEG);
  const n = Math.max(2, Math.ceil(128 * (u1 - u0)));
  let prev = '';
  let side: boolean | null = null;
  for (let i = 0; i <= n; i++) {
    const th = TAU * (u0 + ((u1 - u0) * i) / n);
    const lx = e.rx * Math.cos(th);
    const ly = e.ry * Math.sin(th);
    const pt = `${f1(lx * cos - ly * sin + e.cx)} ${f1(lx * sin + ly * cos + e.cy)}`;
    const isFront = Math.cos(th) < 0;
    if (isFront !== side) {
      const seg = prev ? `M${prev}L${pt}` : `M${pt}`;
      if (isFront) front += seg;
      else back += seg;
      side = isFront;
    } else if (isFront) front += `L${pt}`;
    else back += `L${pt}`;
    prev = pt;
  }
  return { back, front };
}

const gradStroke = (d: string, gradId: string, width: number, opacity: number) =>
  d ? `<path d="${d}" stroke="url(#${gradId})" stroke-width="${width}" opacity="${opacity.toFixed(2)}"/>` : '';

/** The orbit/swirl rings never move, so their halves are built once. */
const RING_ARCS = GROKBOT_SVG_META.rings.map((r) => ellArc(r, 0, 1));

/** Orbit ring opacity while the rings appear one after another (reference stage, t in s). */
const ORBIT_RING_IN: readonly (readonly number[])[] = [
  [0.07, 0, 0.1, 0.02, 0.15, 0.07, 0.2, 0.14, 0.25, 0.24, 0.3, 0.36, 0.35, 0.43, 0.4, 0.5, 0.45, 0.56, 0.5, 0.62, 0.55, 0.69, 0.6, 0.75, 0.65, 0.81, 0.7, 0.87, 0.75, 0.94, 0.8, 1],
  [0.17, 0, 0.2, 0.05, 0.25, 0.12, 0.3, 0.21, 0.35, 0.32, 0.4, 0.45, 0.45, 0.56, 0.5, 0.62, 0.55, 0.69, 0.6, 0.75, 0.65, 0.81, 0.7, 0.87, 0.75, 0.94, 0.8, 1],
  [0.28, 0, 0.3, 0.05, 0.35, 0.13, 0.4, 0.23, 0.45, 0.36, 0.5, 0.5, 0.55, 0.66, 0.6, 0.75, 0.65, 0.81, 0.7, 0.87, 0.75, 0.94, 0.8, 1],
  [0.39, 0, 0.4, 0.02, 0.45, 0.11, 0.5, 0.23, 0.55, 0.37, 0.6, 0.52, 0.65, 0.7, 0.7, 0.87, 0.75, 0.94, 0.8, 1],
  [0.53, 0, 0.55, 0.07, 0.6, 0.2, 0.65, 0.35, 0.7, 0.52, 0.75, 0.72, 0.8, 0.93, 0.85, 1],
  [0.655, 0, 0.7, 0.15, 0.75, 0.31, 0.8, 0.5, 0.85, 0.67, 0.92, 0.89, 0.95, 1],
];

/**
 * While the orbit triangle spins, the eyes (circle layout) sway with the spin phase φ (0–120°, one
 * third of a turn): horizontal offsets of the left and right eye, sampled from the reference.
 */
const ORBIT_EYE_L = [0, 2.4, 7.5, -1.1, 17.5, -3.5, 22.5, -3.7, 30, -1.1, 37.5, 1.3, 47.5, 3.2, 52.5, 5.4, 62.5, 6.4, 72.5, 7.4, 82.5, 7.3, 90, 6.7, 97.5, 6.4, 105, 5.1, 112.5, 3.4, 120, 2.4];
const ORBIT_EYE_R = [0, -6.5, 7.5, -7.4, 17.5, -8.1, 22.5, -8.3, 30, -7.45, 37.5, -7.2, 47.5, -6.3, 52.5, -4, 62.5, -2.2, 67.5, -0.1, 72.5, 2.7, 82.5, 2.7, 90, 0.65, 97.5, -2.1, 105, -4.05, 112.5, -5.8, 120, -6.5];

/** Extra spin angle (deg) while the orbit triangle winds up to its steady 450°/s. */
const ORBIT_SPIN_LAG = [0, 0, 0.05, 22, 0.1, 42, 0.15, 46, 0.2, 34, 0.25, 16, 0.3, 6, 0.35, 2, 0.45, 0];

/** Horizontal centroid of the alert glyph: it slides right, pauses, then hurries back. */
const ALERT_X = [0, -8.4, 0.2, -7.97, 0.3, -6.8, 0.4, -4.18, 0.5, 0.6, 0.6, 7.79, 0.7, 18.29, 0.8, 32.58, 0.9, 46.9, 1.0, 57.4, 1.1, 64.72, 1.2, 69.41, 1.3, 72.06, 1.4, 73.24, 1.5, 73.56, 1.65, 73.57, 2.05, 10.25];

/** Burst particles: birth time (s), starting angle (deg) and distance; each spirals into the dot. */
const BURST_PARTICLES: readonly (readonly [number, number, number])[] = [
  [0.035, -116, 57.5],
  [0.205, -128, 61],
  [0.405, -7.5, 60],
  [0.615, -102, 61],
  [0.815, 53, 64],
];

/* --------------------------------------------------------------------- moves */

interface Ctx {
  t: number;
  shape: GrokBotShape;
  uid: string;
  solid: string;
  deep: string;
  gx: number;
  gy: number;
  faceEyes: [GrokBotEyePose, GrokBotEyePose];
  faceEyesCircle: [GrokBotEyePose, GrokBotEyePose];
  out: Float32Array;
  rest: Float32Array;
}

interface Fx {
  back?: string;
  front?: string;
  /** Solid dots drawn over the body (also shown on Move tiles). */
  dots?: string;
  /** Solid dots drawn behind the body (also shown on Move tiles). */
  dotsBack?: string;
  badge?: { cx: number; cy: number; r: number };
}

interface MoveDef {
  dur: number;
  inDur: number;
  outDur: number;
  /** Seconds after the click before the body starts morphing (keeps the reference timings on one clock). */
  inDelay?: number;
  /** Hidden eyes start fading this long after the click (defaults to `inDelay`). */
  eyeInDelay?: number;
  /** Pill-shaped eyes resize smoothly into the move's eyes instead of swapping mid-blink. */
  morphEyes?: boolean;
  /** Time (s) used for the static Move tile. */
  preview: number;
  body?: (c: Ctx) => Float32Array;
  eyes?: (c: Ctx) => [GrokBotEyePose, GrokBotEyePose];
  /** Eyes slide together and fade out while the body becomes a glyph or dot. */
  hideEyes?: boolean;
  /** For hideEyes moves that pop back into a circle with a face before the end (0..1). */
  eyeReturn?: (t: number) => number;
  /** The move ends on a circle, so the face eyes use the circle layout until the final morph. */
  endsOnCircle?: boolean;
  fx?: (c: Ctx, env: number) => Fx;
}

/** Eye pose in the user's shape layout (positions authored for the circle). */
function onShape(c: Ctx, d: string, x: number, y: number, deg: number, sy = 1): GrokBotEyePose {
  const L = GROKBOT_SHAPE_EYE_LAYOUT[c.shape];
  return { d, x: L.dx + x * L.s + c.gx * 11, y: L.dy + y * L.s + c.gy * 7, deg, sy };
}

function pose(d: string, x: number, y: number, deg: number, c: Ctx, sy = 1): GrokBotEyePose {
  return { d, x: x + c.gx * 11, y: y + c.gy * 7, deg, sy };
}

/** Body pops from a small dot at (cx, cy) to the full circle between `t0` and `t0 + 0.45`. */
function popCircle(c: Ctx, rFrom: number, t0: number, cx = 0, cy = 0): Float32Array | null {
  if (c.t < t0) return null;
  const k = easeOutCubic(clamp01((c.t - t0) / 0.45));
  return dot(c.out, lerp(rFrom, 100, k), cx * (1 - k), cy * (1 - k));
}

const MOVES: Record<GrokBotMove, MoveDef> = {
  idle: { dur: 0.32, inDur: 0.32, outDur: 0, preview: 0 },

  thinking: {
    dur: 2.915,
    inDur: 0.27,
    outDur: 0.3,
    preview: 1.0,
    hideEyes: true,
    // The body is the centre dot; the side dots slide out of it, then each swells in turn.
    body: (c) => dot(c.out, 16.5 + 4.13 * thinkSwell(c.t, 1), -1.7, -0.4),
    fx: (c, env) => {
      const slide = easeOutQuart(clamp01(c.t / 0.3));
      return {
        dots: ([
          [-56.2, 0],
          [52.7, 2],
        ] as const)
          .map(([x, idx]) => {
            const w = thinkSwell(c.t, idx);
            return `<circle cx="${f1(-1.7 + (x + 1.7) * slide)}" cy="-0.4" r="${(16.5 + 4.13 * w).toFixed(2)}" fill="${c.solid}" opacity="${((0.55 + 0.45 * w) * env).toFixed(2)}"/>`;
          })
          .join(''),
      };
    },
  },

  wink: {
    dur: 2.0,
    inDur: 0.22,
    outDur: 0.4,
    preview: 1.0,
    eyes: (c) => [
      onShape(c, MOVE_EYE_PATHS.winkLeft, -13.3, -11.3, 5),
      onShape(c, MOVE_EYE_PATHS.winkRightBar, 38.3, 0, 8),
    ],
  },

  wide_eyes: {
    dur: 2.1,
    inDur: 0.28,
    outDur: 0.3,
    preview: 1.0,
    eyes: (c) => {
      const pop = easeOutBack(clamp01(c.t / 0.32));
      const sy = lerp(0.7, 1, pop);
      return [
        onShape(c, MOVE_EYE_PATHS.wideEye, -26.5, -24, 12, sy),
        onShape(c, MOVE_EYE_PATHS.wideEye, 26.5, -8, 12, sy),
      ];
    },
  },

  alert: {
    dur: 2.367,
    inDur: 0.3,
    outDur: 0.3,
    inDelay: 0.05,
    preview: 0.3,
    hideEyes: true,
    // A tilted "!" slides right, pauses, then hurries back; glyph and dot bob against each other.
    body: (c) => {
      const x = pl(c.t, ALERT_X);
      const y = -32.4 - 0.5 * Math.cos((TAU * (c.t - 0.95)) / 0.4);
      lerpCoords(c.out, GLYPH_COORDS.alert, GLYPH_COORDS.alert, 0, x - 46.9, y + 32.7);
      return c.out;
    },
    fx: (c, env) => {
      const x = pl(c.t, ALERT_X) - 17.72;
      const y = 22.83 + 1.42 * Math.cos((TAU * (c.t - 0.95)) / 0.4);
      return {
        dots: `<path d="${GROKBOT_ALERT_DOT_PATH}" fill="${c.solid}" opacity="${env.toFixed(2)}" transform="translate(${f1(x)} ${f1(y)}) rotate(17.7) scale(100)"/>`,
      };
    },
  },

  notification: {
    dur: 2.45,
    inDur: 0.25,
    outDur: 0.25,
    preview: 1.0,
    eyes: (c) => [
      onShape(c, MOVE_EYE_PATHS.notificationEye, -18.1, 21.8, -12),
      onShape(c, MOVE_EYE_PATHS.notificationEye, 41.5, 4.8, -12),
    ],
    fx: (c, env) => {
      const [bx, by] = notificationAnchor(c.shape);
      const grow = easeOutBack(clamp01((c.t - 0.08) / 0.3));
      const r = 15 * grow * (env < 1 && c.t > 1 ? env : 1);
      return r > 0.2 ? { badge: { cx: bx, cy: by, r } } : {};
    },
  },

  exclamation: {
    dur: 2.367,
    inDur: 0.3,
    outDur: 0.3,
    inDelay: 0.05,
    preview: 1.0,
    hideEyes: true,
    body: () => GLYPH_COORDS.exclamation,
    fx: (c, env) => ({
      dots: `<circle cx="-1.2" cy="52.6" r="11.3" fill="${c.solid}" opacity="${env.toFixed(2)}"/>`,
    }),
  },

  sleep: {
    dur: 2.767,
    inDur: 0.3,
    outDur: 0.3,
    inDelay: 0.05,
    preview: 0.95,
    hideEyes: true,
    // A small dot bouncing every 0.6 s.
    body: (c) => dot(c.out, 15.85, 0.05, 11.15 + 19.05 * Math.cos((TAU * (c.t - 0.767)) / 0.6)),
  },

  egg: {
    dur: 2.1,
    inDur: 0.25,
    outDur: 0.32,
    preview: 1.0,
    body: () => TARGETS.egg,
    eyes: (c) => {
      const blink = 1 - 0.9 * bump((c.t - 1.62) / 0.09);
      return [
        pose(MOVE_EYE_PATHS.eggEye, -15, 11, -20, c, blink),
        pose(MOVE_EYE_PATHS.eggEye, 16, 3, -20, c, blink),
      ];
    },
  },

  hexagon: {
    dur: 1.95,
    inDur: 0.25,
    outDur: 0.3,
    preview: 1.0,
    body: () => TARGETS.hexTall,
    eyes: (c) => [pose(MOVE_EYE_PATHS.hexEye, -13, 14, -12, c), pose(MOVE_EYE_PATHS.hexEye, 23, 10, -12, c)],
  },

  play: {
    dur: 2.3,
    inDur: 0.25,
    outDur: 0.3,
    preview: 1.0,
    body: () => TARGETS.play,
    eyes: (c) => [pose(MOVE_EYE_PATHS.smallPlayEye, -11, 9, -6, c), pose(MOVE_EYE_PATHS.smallPlayEye, 28, 6, -6, c)],
    fx: (c, env) => {
      const head = -0.35 + 3.9 * Math.max(0, c.t - 0.08);
      const len = Math.min(2.5, 3.9 * Math.max(0, c.t - 0.08)) * clamp01((2.15 - c.t) / 0.5 + 0.0001);
      let back = '';
      let front = '';
      for (let s = 0; s < 4; s++) {
        const o = (s - 1.5) * 6;
        const ring: Ring = { cx: 0, cy: -8, rx: 135 + o, ry: 22 + o, tilt: -26, phase: 0 };
        const arc = ringArc(ring, (head - len) / TAU, head / TAU);
        back += strokePath(arc.back, c.uid, s, 5, env);
        front += strokePath(arc.front, c.uid, s, 5, env);
      }
      return { back, front };
    },
  },

  orbit: {
    dur: 2.87,
    inDur: 0.35,
    outDur: 0.3,
    preview: 1.0,
    endsOnCircle: true,
    morphEyes: true,
    // A triangle spinning 450°/s about the centre while six rings appear; it rounds into a circle.
    body: (c) => {
      // The spin eases in (lagging up to 46°), and the body freezes once it starts settling back.
      const t = Math.min(c.t, 2.52);
      const rot = ((1.001 - t) * 450 + pl(t, ORBIT_SPIN_LAG)) * DEG;
      lerpCoords(c.out, GLYPH_COORDS.orbit, GLYPH_COORDS.orbit, 0, 0, 0, rot, 1);
      const k = orbitRound(t);
      if (k > 0) {
        // The circle's anchors trail the triangle's by 90° so the rounding doesn't twist.
        lerpCoords(orbitCircle, CIRCLE, CIRCLE, 0, 0, 0, rot - 90 * DEG, 1);
        lerpCoords(c.out, c.out, orbitCircle, k);
      }
      return c.out;
    },
    eyes: (c) => {
      const k = orbitRound(c.t);
      const ph = ((((c.t - 1.001) * 450) % 120) + 120) % 120;
      const dl = pl(ph, ORBIT_EYE_L) * (1 - k);
      const dr = pl(ph, ORBIT_EYE_R) * (1 - k);
      const d = PILL(9, +lerp(8, 11.5, k).toFixed(2));
      const [l, r] = c.faceEyesCircle;
      return [
        { d, x: l.x + dl, y: l.y - 0.24 * dl, deg: l.deg, sy: l.sy },
        { d, x: r.x + dr, y: r.y - 0.24 * dr, deg: r.deg, sy: r.sy },
      ];
    },
    fx: (c) => {
      const fade = 1 - easeOutCubic(clamp01((c.t - 2.53) / 0.3));
      let back = '';
      let front = '';
      GROKBOT_SVG_META.rings.forEach((r, i) => {
        const op = pl(c.t, ORBIT_RING_IN[i]) * fade;
        if (op <= 0.005) return;
        back += gradStroke(RING_ARCS[i].back, `${c.uid}-rg${i}`, r.width, op);
        front += gradStroke(RING_ARCS[i].front, `${c.uid}-rg${i}`, r.width, op);
      });
      return { back, front };
    },
  },

  swirl: {
    dur: 1.3,
    inDur: 0.05,
    outDur: 0.05,
    preview: 0.6,
    // Three of the orbit rings flash in one after another around a blink, then fade together.
    eyes: (c) => {
      const blink = 1 - 0.77 * bump((c.t - 0.13) / 0.1);
      const e = c.faceEyes;
      return [
        { ...e[0], sy: e[0].sy * blink },
        { ...e[1], sy: e[1].sy * blink },
      ];
    },
    fx: (c) => {
      const fade = clamp01((1.27 - c.t) / 0.34);
      let back = '';
      let front = '';
      for (let i = 0; i < 3; i++) {
        const op = clamp01((c.t - [0.075, 0.12, 0.17][i]) / 0.125) * fade;
        if (op <= 0.005) continue;
        const r = GROKBOT_SVG_META.rings[i];
        back += gradStroke(RING_ARCS[i].back, `${c.uid}-rg${i}`, r.width, op);
        front += gradStroke(RING_ARCS[i].front, `${c.uid}-rg${i}`, r.width, op);
      }
      return { back, front };
    },
  },

  burst: {
    dur: 2.767,
    inDur: 0.42,
    outDur: 0.3,
    inDelay: 0.05,
    eyeInDelay: 0,
    preview: 0.7,
    hideEyes: true,
    endsOnCircle: true,
    eyeReturn: (t) => clamp01((t - 1.86) / 0.385),
    // Dark particles spiral in from all sides, lightening as they merge into the dot behind it;
    // then the dot pops into a big round bot.
    body: (c) => popCircle(c, 16.6, 1.698, 0.3, 0) ?? dot(c.out, 16.6, 0.3, 0),
    fx: (c) => {
      const light = mixHex(c.solid, '#000000', 0.12);
      let dotsBack = '';
      for (const [born, a0, d0] of BURST_PARTICLES) {
        const age = c.t - born;
        if (age < 0 || age > 0.6) continue;
        const u = age / 0.6;
        const ang = (a0 + 60 * u) * DEG;
        const dist = 11 + (d0 - 11) * Math.pow(1 - u, 1.8);
        const r = 4 + 2.8 * Math.min(1, age / 0.53);
        const op = Math.min(1, age / 0.06, (0.6 - age) / 0.1);
        dotsBack += `<circle cx="${f1(Math.cos(ang) * dist)}" cy="${f1(Math.sin(ang) * dist)}" r="${r.toFixed(2)}" fill="${mixHex(c.deep, light, Math.pow(u, 0.75))}" opacity="${op.toFixed(2)}"/>`;
      }
      return { dotsBack };
    },
  },

  comet: {
    dur: 2.75,
    inDur: 0.36,
    outDur: 0.3,
    inDelay: 0.04,
    eyeInDelay: 0,
    preview: 0.9,
    hideEyes: true,
    endsOnCircle: true,
    eyeReturn: (t) => clamp01((t - 2.0) / 0.35),
    // A four-strand rainbow tail (a third of a loop long) circles the dot once at a steady pace,
    // passing behind it on the way out and in front of it on the way back.
    body: (c) => popCircle(c, 13, 1.85, -0.4, 2) ?? dot(c.out, 13, -0.4, 2),
    fx: (c) => {
      const op = clamp01((c.t - 0.15) / 0.25) * clamp01((1.95 - c.t) / 0.3);
      if (op <= 0.005) return {};
      const u0 = 0.367 + 0.5835 * (c.t - 0.201);
      let back = '';
      let front = '';
      GROKBOT_SVG_META.comet.forEach((e, i) => {
        const arc = ellArc(e, u0, u0 + 0.341);
        back += gradStroke(arc.back, `${c.uid}-cm${i}`, e.width, op);
        front += gradStroke(arc.front, `${c.uid}-cm${i}`, e.width, op);
      });
      return { back, front };
    },
  },
};

/**
 * Swell of thinking dot `idx` (0 = left, 1 = centre, 2 = right): each dot in turn grows over 0.35 s,
 * holds 0.35 s, then snaps back, one dot every 0.5 s.
 */
function thinkSwell(t: number, idx: number): number {
  const ph = (((t - 0.05 - idx * 0.5) % 1.5) + 1.5) % 1.5;
  if (ph < 0.35) return Math.pow(ph / 0.35, 1.4);
  return ph < 0.7 ? 1 : 0;
}

/** How far the orbit's spinning triangle has rounded into a circle (0..1). */
function orbitRound(t: number): number {
  return easeInOutSine(clamp01((t - 1.75) / 0.6));
}

const PILL_RE = /^M-([\d.]+) -([\d.]+)A/;

function blendEyes(
  a: [GrokBotEyePose, GrokBotEyePose],
  b: [GrokBotEyePose, GrokBotEyePose],
  w: number,
  morph = false,
): [GrokBotEyePose, GrokBotEyePose] {
  if (w <= 0) return a;
  if (w >= 1) return b;
  const one = (p: GrokBotEyePose, q: GrokBotEyePose): GrokBotEyePose => {
    const sameShape = p.d === q.d;
    // Two pills can resize into each other; any other change of cutout swaps while squashed shut,
    // so it reads as a blink.
    const pa = morph && !sameShape ? PILL_RE.exec(p.d) : null;
    const pb = pa ? PILL_RE.exec(q.d) : null;
    if (pa && pb) {
      return {
        d: PILL(+lerp(+pa[1], +pb[1], w).toFixed(2), +lerp(+pa[2], +pb[2], w).toFixed(2)),
        x: lerp(p.x, q.x, w),
        y: lerp(p.y, q.y, w),
        deg: lerp(p.deg, q.deg, w),
        sy: lerp(p.sy, q.sy, w),
      };
    }
    const squash = sameShape ? 1 : Math.max(0.04, Math.abs(1 - 2 * w));
    return {
      d: sameShape || w < 0.5 ? p.d : q.d,
      x: lerp(p.x, q.x, w),
      y: lerp(p.y, q.y, w),
      deg: lerp(p.deg, q.deg, w),
      sy: lerp(p.sy, q.sy, w) * squash,
    };
  };
  return [one(a[0], b[0]), one(a[1], b[1])];
}

/* -------------------------------------------------------------------- output */

export interface GrokBotRenderFrame {
  /** Body outline; null when it is unchanged since the previous frame. */
  bodyD: string | null;
  /** Breathing transform for the body group (body, mask and eyes). */
  wrapTransform: string;
  eyes: [{ d: string; transform: string }, { d: string; transform: string }];
  eyeOpacity: number;
  badge: { cx: number; cy: number; r: number } | null;
  back: string;
  front: string;
}

export interface GrokBotFrameInput {
  shape: GrokBotShape;
  face: GrokBotFace;
  color: GrokBotColor;
  uid: string;
  /** Normalised cursor position (-1..1), or null to let the gaze wander. */
  pointer: { x: number; y: number } | null;
  /** Breathing, wandering gaze and blinks (off under reduced motion). */
  ambient: boolean;
}

function paletteInk(color: GrokBotColor): { solid: string; deep: string } {
  const p = GROKBOT_COLOR_PALETTES[color];
  const solid = color === 'ink' ? GROKBOT_SVG_META.inkStroke : p.stops[2];
  return { solid, deep: mixHex(p.stops[3], '#000000', 0.55) };
}

function frameFor(
  def: MoveDef,
  c: Ctx,
  from: Float32Array | null,
  body: Float32Array,
  tile: boolean,
): { eyes: [GrokBotEyePose, GrokBotEyePose]; eyeOpacity: number; fx: Fx; env: number } {
  const t = c.t;
  const outStart = def.dur - def.outDur;
  const envIn = def.inDur > 0 ? easeOutCubic(clamp01((t - (def.inDelay ?? 0)) / def.inDur)) : 1;
  // The reference snaps back quickly, then settles: a front-loaded ease.
  const envOut = def.outDur > 0 ? 1 - easeOutCubic(clamp01((t - outStart) / def.outDur)) : 1;
  const env = Math.min(envIn, envOut);
  const target = def.body ? def.body(c) : c.rest;
  if (t < outStart) lerpCoords(body, from ?? c.rest, target, envIn);
  else lerpCoords(body, c.rest, target, envOut);

  // When a move ends on a circle the face eyes sit in the circle layout until the final morph.
  const settled = def.endsOnCircle && t >= outStart ? blendEyes(c.faceEyes, c.faceEyesCircle, envOut) : c.faceEyes;
  let eyes: [GrokBotEyePose, GrokBotEyePose];
  let eyeOpacity = 1;
  if (def.hideEyes) {
    // As in the reference, the eyes keep their shape: they slide in toward the shrinking body's
    // centre and fade out, then reverse on the way back.
    const eyeIn = easeOutCubic(clamp01((t - (def.eyeInDelay ?? def.inDelay ?? 0)) / 0.3));
    const k = t < outStart ? eyeIn : envOut;
    const back = def.eyeReturn ? def.eyeReturn(t) : 0;
    const squeeze = back > 0 ? 1 : 1 - 0.78 * k;
    const base = def.endsOnCircle ? settled : c.faceEyes;
    eyes = [
      { ...base[0], x: base[0].x * squeeze, y: base[0].y * squeeze },
      { ...base[1], x: base[1].x * squeeze, y: base[1].y * squeeze },
    ];
    eyeOpacity = tile ? 0 : Math.max(1 - k, back);
  } else {
    eyes = blendEyes(settled, def.eyes ? def.eyes(c) : settled, def.eyes ? env : 0, def.morphEyes);
  }
  const fx = def.fx ? def.fx(c, env) : {};
  return { eyes, eyeOpacity, fx, env };
}

function eyeOut(e: [GrokBotEyePose, GrokBotEyePose]): GrokBotRenderFrame['eyes'] {
  return [
    { d: e[0].d, transform: eyePoseMatrix(e[0]) },
    { d: e[1].d, transform: eyePoseMatrix(e[1]) },
  ];
}

/** A still frame for Move tiles, exports and reduced motion: the move's pose mid-performance. */
export function staticGrokBotFrame(
  shape: GrokBotShape,
  face: GrokBotFace,
  color: GrokBotColor,
  move: GrokBotMove,
  uid: string,
): GrokBotRenderFrame {
  const def = MOVES[move];
  const ink = paletteInk(color);
  const out = new Float32Array(N);
  const body = new Float32Array(N);
  const c: Ctx = {
    t: def.preview,
    shape,
    uid,
    solid: ink.solid,
    deep: ink.deep,
    gx: 0,
    gy: 0,
    faceEyes: faceEyePoses(face, shape),
    faceEyesCircle: faceEyePoses(face, 'circle'),
    out,
    rest: SHAPE_COORDS[shape],
  };
  const r = frameFor(def, c, null, body, true);
  return {
    bodyD: coordsToBezier64Path(body),
    wrapTransform: '',
    eyes: eyeOut(r.eyes),
    eyeOpacity: r.eyeOpacity,
    badge: r.fx.badge ?? null,
    // Tiles are stills without strokes, like the reference picker: only the solid dots.
    back: r.fx.dotsBack ?? '',
    front: r.fx.dots ?? '',
  };
}

/** Per-instance animation state: gaze, blinks, breathing and the move currently performing. */
export class GrokBotAnimator {
  private rand: () => number;
  private move: GrokBotMove = 'idle';
  private moveStart = -1e9;
  private from: Float32Array | null = null;
  private body = new Float32Array(N);
  private out = new Float32Array(N);
  private morphRest = new Float32Array(N);
  private bodyIsRest = true;
  private shape: GrokBotShape | null = null;
  private shapeFrom: Float32Array | null = null;
  private shapeMorphStart = 0;
  private lastPathKey = '';
  private lastNow = 0;
  private t0 = -1;
  private gx = 0;
  private gy = 0;
  private wanderX = 0;
  private wanderY = 0;
  private nextWander = 0;
  private nextBlink = 0;
  private blinks: number[] = [];

  constructor(seed: string) {
    let h = 2166136261;
    for (let i = 0; i < seed.length; i++) h = Math.imul(h ^ seed.charCodeAt(i), 16777619);
    let s = h >>> 0;
    this.rand = () => {
      s = (s + 0x6d2b79f5) | 0;
      let r = Math.imul(s ^ (s >>> 15), 1 | s);
      r = (r + Math.imul(r ^ (r >>> 7), 61 | r)) ^ r;
      return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
    };
    this.nextBlink = 0.6 + this.rand() * 3.4;
  }

  /** Starts (or replays) a move, morphing from whatever is on screen now. */
  play(move: GrokBotMove, nowMs: number): void {
    this.from = this.bodyIsRest && this.shape ? new Float32Array(SHAPE_COORDS[this.shape]) : new Float32Array(this.body);
    this.move = move;
    this.moveStart = nowMs;
  }

  frame(nowMs: number, p: GrokBotFrameInput): GrokBotRenderFrame {
    if (this.t0 < 0) this.t0 = nowMs;
    const time = (nowMs - this.t0) / 1000;
    const dt = Math.min(0.1, Math.max(0, (nowMs - (this.lastNow || nowMs)) / 1000));
    this.lastNow = nowMs;

    if (this.shape !== null && this.shape !== p.shape) {
      this.shapeFrom = new Float32Array(this.bodyIsRest ? SHAPE_COORDS[this.shape] : this.body);
      this.shapeMorphStart = nowMs;
    }
    this.shape = p.shape;

    // Resting outline (eased while the user switches shape).
    let rest = SHAPE_COORDS[p.shape];
    let morphing = false;
    if (this.shapeFrom) {
      const k = (nowMs - this.shapeMorphStart) / 280;
      if (k >= 1) this.shapeFrom = null;
      else {
        lerpCoords(this.morphRest, this.shapeFrom, rest, easeOutCubic(k));
        rest = this.morphRest;
        morphing = true;
      }
    }

    // Gaze: the cursor when tracking, otherwise a slow wander with pauses.
    if (p.ambient && time >= this.nextWander) {
      this.wanderX = (this.rand() * 2 - 1) * 0.2;
      this.wanderY = (this.rand() * 2 - 1) * 0.24;
      this.nextWander = time + 1.3 + this.rand() * 2;
    }
    const tx = p.pointer ? p.pointer.x : p.ambient ? this.wanderX : 0;
    const ty = p.pointer ? p.pointer.y : p.ambient ? this.wanderY : 0;
    const k = 1 - Math.exp(-dt / (p.pointer ? 0.12 : 0.45));
    this.gx += (tx - this.gx) * k;
    this.gy += (ty - this.gy) * k;

    // Irregular blinks, sometimes doubled.
    let blink = 1;
    if (p.ambient) {
      if (time >= this.nextBlink) {
        this.blinks.push(time);
        if (this.rand() < 0.25) this.blinks.push(time + 0.3);
        this.nextBlink = time + 2.4 + this.rand() * 3.2;
      }
      this.blinks = this.blinks.filter((b) => time - b < 0.2);
      for (const b of this.blinks) {
        const u = (time - b) / 0.17;
        if (u >= 0 && u <= 1) blink = Math.min(blink, 1 - 0.92 * Math.sin(Math.PI * u));
      }
    }

    const ink = paletteInk(p.color);
    const faceEyes = faceEyePoses(p.face, p.shape, this.gx, this.gy);
    const faceEyesCircle = faceEyePoses(p.face, 'circle', this.gx, this.gy);
    const def = MOVES[this.move];
    const t = (nowMs - this.moveStart) / 1000;
    const active = t < def.dur;
    const c: Ctx = {
      t,
      shape: p.shape,
      uid: p.uid,
      solid: ink.solid,
      deep: ink.deep,
      gx: this.gx,
      gy: this.gy,
      faceEyes,
      faceEyesCircle,
      out: this.out,
      rest,
    };

    let eyes = faceEyes;
    let eyeOpacity = 1;
    let fx: Fx = {};
    let env = 0;
    if (active) {
      const r = frameFor(def, c, this.from, this.body, false);
      eyes = r.eyes;
      eyeOpacity = r.eyeOpacity;
      fx = r.fx;
      env = r.env;
      this.bodyIsRest = false;
    } else {
      this.from = null;
      if (morphing) this.body.set(rest);
      this.bodyIsRest = !morphing;
    }

    // Blinks only touch open, face-style eyes.
    if (blink < 1 && eyeOpacity > 0.5) {
      eyes = [
        { ...eyes[0], sy: eyes[0].sy * blink },
        { ...eyes[1], sy: eyes[1].sy * blink },
      ];
    }

    // Breathing: a ~1 % vertical stretch pinned at the base, faded out while a move performs.
    const breath = p.ambient ? 1 - env : 0;
    const sy = 1 + 0.01 * breath * (0.5 - 0.5 * Math.cos((TAU * time) / 4.4));
    const sway = 0.4 * breath * Math.sin((TAU * time) / 6.3);
    const yb = shapeBottom(p.shape);
    const wrapTransform =
      breath > 0 ? `matrix(1,0,0,${sy.toFixed(4)},${sway.toFixed(2)},${(yb * (1 - sy)).toFixed(2)})` : '';

    let bodyD: string | null = null;
    if (this.bodyIsRest) {
      const key = `rest:${p.shape}`;
      if (key !== this.lastPathKey) {
        bodyD = coordsToBezier64Path(SHAPE_COORDS[p.shape]);
        this.lastPathKey = key;
      }
    } else {
      bodyD = coordsToBezier64Path(this.body);
      this.lastPathKey = '';
    }

    return {
      bodyD,
      wrapTransform,
      eyes: eyeOut(eyes),
      eyeOpacity,
      badge: fx.badge ?? null,
      back: (fx.back ?? '') + (fx.dotsBack ?? ''),
      front: (fx.front ?? '') + (fx.dots ?? ''),
    };
  }

  /** True while a move (or a shape morph) is still changing the outline. */
  get busy(): boolean {
    return !this.bodyIsRest;
  }
}

/* ------------------------------------------------------- shared frame ticker */

type FrameSub = (nowMs: number) => void;
const subs = new Set<FrameSub>();
let rafId = 0;

function loop(now: number) {
  if (typeof document === 'undefined' || !document.hidden) subs.forEach((f) => f(now));
  rafId = subs.size ? requestAnimationFrame(loop) : 0;
}

/** One requestAnimationFrame loop shared by every animated Grok Bot; stops when nobody listens. */
export function subscribeGrokBotFrames(fn: FrameSub): () => void {
  subs.add(fn);
  if (!rafId && typeof requestAnimationFrame === 'function') rafId = requestAnimationFrame(loop);
  return () => {
    subs.delete(fn);
    if (!subs.size && rafId) {
      cancelAnimationFrame(rafId);
      rafId = 0;
    }
  };
}

export const GROKBOT_MOVE_DURATIONS: Record<GrokBotMove, number> = Object.fromEntries(
  Object.entries(MOVES).map(([k, v]) => [k, v.dur]),
) as Record<GrokBotMove, number>;
