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
  SHAPE_COORDS,
} from './grokBotStudio';

const N = 386;
const TAU = Math.PI * 2;
const DEG = Math.PI / 180;

const clamp01 = (v: number) => (v < 0 ? 0 : v > 1 ? 1 : v);
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const easeOutCubic = (x: number) => 1 - Math.pow(1 - x, 3);
const easeInCubic = (x: number) => x * x * x;
const easeInOutCubic = (x: number) => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
const easeInOutSine = (x: number) => -(Math.cos(Math.PI * x) - 1) / 2;
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

/** Tapered stem (wide top, narrow foot) used for the "!" glyphs, rotated `deg` about its centre. */
function stem(halfH: number, halfW: number, cx: number, cy: number, deg: number): Float32Array {
  const cos = Math.cos(deg * DEG);
  const sin = Math.sin(deg * DEG);
  return buildPolar64Coords((theta) => {
    const c = Math.cos(theta);
    const s = Math.sin(theta);
    const taper = 1 - 0.42 * ((1 + s) * 0.5);
    const x = Math.sign(c) * Math.pow(Math.abs(c), 0.7) * halfW * taper;
    const y = s * halfH;
    return [x * cos - y * sin + cx, x * sin + y * cos + cy];
  });
}

const CIRCLE = SHAPE_COORDS.circle; // r ≈ 100
const TARGETS = {
  egg: buildPolar64Coords((theta) => {
    const y = Math.sin(theta);
    return [80 * (1 + 0.1 * y) * Math.cos(theta) + 1.5, 100.5 * y - 4.5];
  }),
  hexTall: affine(SHAPE_COORDS.hexagon, 0.97, 0.97, 90, 0, 3),
  play: affine(SHAPE_COORDS.triangle, 0.98, 0.98, 8, 0, 20),
  orbit: affine(SHAPE_COORDS.triangle, 0.98, 0.98, 0, 0, 16),
  exclamation: stem(42, 13, 0, -22, 0),
  alert: stem(35, 11, 0, -32, 17.7),
};

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

const SWIRL_RINGS: Ring[] = [
  { cx: 0, cy: -4, rx: 137, ry: 27, tilt: 8, phase: 2.4 },
  { cx: 6, cy: 9, rx: 128, ry: 40, tilt: 58, phase: 3.6 },
  { cx: 11, cy: 9, rx: 140, ry: 18, tilt: 64, phase: 4.4 },
];
const ORBIT_RINGS: Ring[] = [
  ...SWIRL_RINGS,
  { cx: 2, cy: 17, rx: 140, ry: 30, tilt: -34, phase: 0.6 },
  { cx: -10, cy: 30, rx: 130, ry: 22, tilt: -6, phase: 1.4 },
  { cx: -8, cy: -6, rx: 122, ry: 26, tilt: 20, phase: 5.2 },
];
const RING_WIDTHS = [5.26, 6.02, 5.62, 5.17, 5.26, 5.55];

/** Catmull-Rom sample of the comet's flight path (upper-left hook → lower-right hook). */
const COMET_PTS: [number, number][] = [
  [-58, -26],
  [-70, -38],
  [-68, -52],
  [-50, -50],
  [-18, -30],
  [20, -2],
  [52, 22],
  [68, 40],
  [64, 54],
];
const COMET_PATH: [number, number][] = (() => {
  const out: [number, number][] = [];
  const p = COMET_PTS;
  for (let i = 0; i < p.length - 1; i++) {
    const p0 = p[Math.max(0, i - 1)];
    const p1 = p[i];
    const p2 = p[i + 1];
    const p3 = p[Math.min(p.length - 1, i + 2)];
    for (let k = 0; k < 12; k++) {
      const t = k / 12;
      const t2 = t * t;
      const t3 = t2 * t;
      out.push([
        0.5 * (2 * p1[0] + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3),
        0.5 * (2 * p1[1] + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3),
      ]);
    }
  }
  out.push(p[p.length - 1]);
  return out;
})();

function cometStrands(u0: number, u1: number, uid: string): string {
  if (u1 - u0 < 0.005) return '';
  const last = COMET_PATH.length - 1;
  const i0 = Math.max(0, Math.floor(u0 * last));
  const i1 = Math.min(last, Math.ceil(u1 * last));
  let html = '';
  for (let s = 0; s < 4; s++) {
    const off = (s - 1.5) * 3.4;
    let d = '';
    for (let i = i0; i <= i1; i++) {
      const [x, y] = COMET_PATH[i];
      const [nx0, ny0] = COMET_PATH[Math.min(last, i + 1)];
      const [px, py] = COMET_PATH[Math.max(0, i - 1)];
      const tx = nx0 - px;
      const ty = ny0 - py;
      const len = Math.hypot(tx, ty) || 1;
      d += `${i === i0 ? 'M' : 'L'}${f1(x - (ty / len) * off)} ${f1(y + (tx / len) * off)}`;
    }
    html += strokePath(d, uid, s, 9.5, 1);
  }
  return html;
}

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
  dots?: string;
  badge?: { cx: number; cy: number; r: number };
}

interface MoveDef {
  dur: number;
  inDur: number;
  outDur: number;
  /** Time (s) used for the static Move tile. */
  preview: number;
  body?: (c: Ctx) => Float32Array;
  eyes?: (c: Ctx) => [GrokBotEyePose, GrokBotEyePose];
  /** Eyes squash shut and fade while the body becomes a glyph or dot. */
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

/** Body pops from a small dot to a full circle between `t0` and `t0 + 0.5`. */
function popCircle(c: Ctx, rFrom: number, t0: number): Float32Array | null {
  if (c.t < t0) return null;
  const k = easeOutCubic(clamp01((c.t - t0) / 0.5));
  return dot(c.out, lerp(rFrom, 100, k), 0, 0);
}

const MOVES: Record<GrokBotMove, MoveDef> = {
  idle: { dur: 0.32, inDur: 0.32, outDur: 0, preview: 0 },

  thinking: {
    dur: 2.95,
    inDur: 0.3,
    outDur: 0.35,
    preview: 1.0,
    hideEyes: true,
    body: (c) => dot(c.out, 16.5 + 4.1 * thinkPulse(c.t, 1), 0, 0),
    fx: (c, env) => ({
      dots: [-1, 1]
        .map((side) => {
          const w = thinkPulse(c.t, side < 0 ? 0 : 2);
          const r = 16.5 + 4.1 * w;
          return `<circle cx="${side < 0 ? -55.5 : 53.3}" cy="0" r="${r.toFixed(2)}" fill="${c.solid}" opacity="${((0.55 + 0.45 * w) * env).toFixed(2)}"/>`;
        })
        .join(''),
    }),
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
    dur: 2.35,
    inDur: 0.3,
    outDur: 0.32,
    preview: 0.3,
    hideEyes: true,
    body: (c) => {
      lerpCoords(c.out, TARGETS.alert, TARGETS.alert, 0, alertX(c.t), 0, 0, 1);
      return c.out;
    },
    fx: (c, env) => {
      const x = alertX(c.t) - 17;
      return {
        dots: `<circle cx="${f1(x)}" cy="21.4" r="${(12.5 * env).toFixed(2)}" fill="${c.solid}" opacity="${env.toFixed(2)}"/>`,
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
    dur: 2.35,
    inDur: 0.3,
    outDur: 0.32,
    preview: 1.0,
    hideEyes: true,
    body: (c) => {
      lerpCoords(c.out, TARGETS.exclamation, TARGETS.exclamation, 0, 0, -0.4 * Math.sin(c.t * 4), 0, 1);
      return c.out;
    },
    fx: (c, env) => ({
      dots: `<circle cx="-1" cy="${f1(52 + 0.6 * Math.sin(c.t * 4 + 0.6))}" r="${(11.3 * env).toFixed(2)}" fill="${c.solid}" opacity="${env.toFixed(2)}"/>`,
    }),
  },

  sleep: {
    dur: 2.8,
    inDur: 0.3,
    outDur: 0.4,
    preview: 0.95,
    hideEyes: true,
    body: (c) => dot(c.out, 16, 0, 11 + 19 * Math.cos((TAU * (c.t - 0.74)) / 0.62)),
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
    dur: 2.8,
    inDur: 0.25,
    outDur: 0.35,
    preview: 0.4,
    endsOnCircle: true,
    body: (c) => {
      const spin = (c.t * TAU) / 0.8;
      lerpCoords(c.out, TARGETS.orbit, TARGETS.orbit, 0, 0, 0, spin, 1);
      if (c.t > 2.15) {
        // Pop out of the spin into a round "ta-da" before settling back to the chosen shape.
        const k = easeOutCubic(clamp01((c.t - 2.15) / 0.3));
        lerpCoords(c.out, c.out, CIRCLE, k);
      }
      return c.out;
    },
    eyes: (c) => {
      const play: [GrokBotEyePose, GrokBotEyePose] = [
        pose(MOVE_EYE_PATHS.smallPlayEye, -11, 9, -6, c),
        pose(MOVE_EYE_PATHS.smallPlayEye, 28, 6, -6, c),
      ];
      return blendEyes(play, c.faceEyesCircle, easeInOutCubic(clamp01((c.t - 2.15) / 0.3)));
    },
    fx: (c, env) => {
      let back = '';
      let front = '';
      ORBIT_RINGS.forEach((r, i) => {
        const draw = easeOutCubic(clamp01((c.t - 0.05 - i * 0.1) / 0.35));
        const arc = ringArc(r, 0, draw);
        const op = env * (1 - clamp01((c.t - 2.25) / 0.2));
        back += strokePath(arc.back, c.uid, i, RING_WIDTHS[i], op);
        front += strokePath(arc.front, c.uid, i, RING_WIDTHS[i], op);
      });
      return { back, front };
    },
  },

  swirl: {
    dur: 1.25,
    inDur: 0.2,
    outDur: 0.2,
    preview: 0.6,
    fx: (c) => {
      const head = easeOutCubic(clamp01(c.t / 0.4));
      const tail = easeInCubic(clamp01((c.t - 0.72) / 0.48));
      const op = 1 - clamp01((c.t - 0.95) / 0.3);
      let back = '';
      let front = '';
      SWIRL_RINGS.forEach((r, i) => {
        const arc = ringArc(r, tail, head);
        back += strokePath(arc.back, c.uid, i, RING_WIDTHS[i], op);
        front += strokePath(arc.front, c.uid, i, RING_WIDTHS[i], op);
      });
      return { back, front };
    },
  },

  burst: {
    dur: 2.65,
    inDur: 0.3,
    outDur: 0.27,
    preview: 0.8,
    hideEyes: true,
    endsOnCircle: true,
    eyeReturn: (t) => clamp01((t - 1.85) / 0.4),
    body: (c) => popCircle(c, 16.5, 1.65) ?? dot(c.out, 16.5, 0, 0),
    fx: (c) => {
      let dots = '';
      for (let k = 0; k < 8; k++) {
        const born = 0.15 + k * 0.18;
        const age = (c.t - born) / 0.5;
        if (age < 0 || age > 1) continue;
        const ang = k * 2.39996 + c.t * 0.8;
        const dist = 17 + easeOutCubic(age) * 34;
        const r = lerp(6.8, 4, age);
        const op = age < 0.6 ? 1 : 1 - (age - 0.6) / 0.4;
        dots += `<circle cx="${f1(Math.cos(ang) * dist)}" cy="${f1(Math.sin(ang) * dist)}" r="${r.toFixed(2)}" fill="${mixHex(c.solid, c.deep, age)}" opacity="${op.toFixed(2)}"/>`;
      }
      return { dots };
    },
  },

  comet: {
    dur: 2.7,
    inDur: 0.3,
    outDur: 0.28,
    preview: 0.8,
    hideEyes: true,
    endsOnCircle: true,
    eyeReturn: (t) => clamp01((t - 2.0) / 0.35),
    body: (c) => popCircle(c, 13, 1.88) ?? dot(c.out, 13, 0, 0),
    fx: (c) => {
      const u = easeInOutSine(clamp01((c.t - 0.12) / 1.65));
      const tailLag = Math.min(0.42, u);
      const u0 = Math.max(0, u - tailLag);
      const ends = c.t > 1.77 ? clamp01((c.t - 1.77) / 0.12) : 0; // tail catches the head on arrival
      return { back: cometStrands(lerp(u0, u, ends), u, c.uid) };
    },
  },
};

/** Sequential pulse of the three thinking dots (0 = left, 1 = centre, 2 = right). */
function thinkPulse(t: number, idx: number): number {
  const phase = (((t - 0.5 - idx * 0.5) % 1.5) + 1.5) % 1.5;
  return Math.max(bump(phase / 0.32), bump((phase - 1.5) / 0.32));
}

/** Horizontal slide of the alert glyph: across to the right, a beat, then back toward the centre. */
function alertX(t: number): number {
  if (t < 0.3) return lerp(0, -5, t / 0.3);
  if (t < 1.3) return lerp(-5, 73, easeInOutSine((t - 0.3) / 1.0));
  if (t < 1.55) return 73;
  if (t < 2.05) return lerp(73, 10, easeInOutSine((t - 1.55) / 0.5));
  return lerp(10, 0, clamp01((t - 2.05) / 0.35));
}

function blendEyes(
  a: [GrokBotEyePose, GrokBotEyePose],
  b: [GrokBotEyePose, GrokBotEyePose],
  w: number,
): [GrokBotEyePose, GrokBotEyePose] {
  if (w <= 0) return a;
  if (w >= 1) return b;
  const one = (p: GrokBotEyePose, q: GrokBotEyePose): GrokBotEyePose => {
    const sameShape = p.d === q.d;
    // Different cutouts swap while squashed shut, so the change reads as a blink.
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
  const envIn = def.inDur > 0 ? easeOutCubic(clamp01(t / def.inDur)) : 1;
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
    const closing = t < outStart ? envIn : 0;
    let vis = t < outStart ? 1 - envIn : 1 - envOut;
    let sy = 1 - 0.92 * closing;
    const back = def.eyeReturn ? def.eyeReturn(t) : 0;
    if (back > 0) {
      vis = Math.max(vis, back);
      sy = 1;
    }
    const base = def.endsOnCircle ? settled : c.faceEyes;
    eyes = [
      { ...base[0], sy: base[0].sy * sy },
      { ...base[1], sy: base[1].sy * sy },
    ];
    eyeOpacity = tile ? 0 : vis;
  } else {
    eyes = blendEyes(settled, def.eyes ? def.eyes(c) : settled, def.eyes ? env : 0);
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
    back: '',
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
      back: fx.back ?? '',
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
