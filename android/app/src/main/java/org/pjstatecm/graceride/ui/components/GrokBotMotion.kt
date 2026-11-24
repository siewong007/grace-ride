package org.pjstatecm.graceride.ui.components

import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sin

/*
 * Grok Bot complex moves (thinking, alert, exclamation, sleep, orbit, swirl, burst, comet), ported
 * from the web motion engine `frontend/src/grokBotMotion.ts` and its data in
 * `frontend/src/grokBotStudio.ts`. Keep the numbers in step with those files: they were measured
 * frame by frame from the grokbots.ai studio stage.
 *
 * Everything here is pure geometry (no android.* types) so it runs in JVM unit tests;
 * `GrokBotCanvas` only draws the frames.
 */

/** One of the reference's fixed ellipses (centre, semi-axes, tilt in degrees) with its stroke gradient. */
class GrokEllipse(
    val cx: Float,
    val cy: Float,
    val rx: Float,
    val ry: Float,
    val tilt: Float,
    val width: Float,
    val x1: Float,
    val y1: Float,
    val x2: Float,
    val y2: Float,
    /** Gradient stops at 0, 0.5 and 1 (ARGB). */
    val stops: LongArray,
)

/** An elliptical arc split into polylines behind the body and in front of it (flat x, y pairs). */
class GrokArc(val back: List<FloatArray>, val front: List<FloatArray>)

/** A gradient-stroked polyline (orbit/swirl rings, comet strands). */
class GrokStroke(val points: FloatArray, val ellipse: GrokEllipse, val alpha: Float)

/** A solid dot (thinking dots, exclamation dot, burst particles). */
class GrokDot(val x: Float, val y: Float, val r: Float, val argb: Int, val alpha: Float)

/** The alert's teardrop dot, drawn as `translate(x y) rotate(17.7) scale(100)` of [GrokBotMotion.ALERT_DOT_POINTS]. */
class GrokAlertDot(val x: Float, val y: Float, val argb: Int, val alpha: Float)

/** An eye cutout placed in body space: position, tilt (degrees) and vertical scale. */
data class GrokEyePose(val pill: GrokEyePill, val x: Float, val y: Float, val deg: Float, val sy: Float = 1f)

/** Everything `GrokBotCanvas` needs to draw one frame of a complex move. */
class GrokMoveFrame(
    val body: FloatArray,
    val eyes: Pair<GrokEyePose, GrokEyePose>,
    val eyeOpacity: Float,
    val backStrokes: List<GrokStroke>,
    val frontStrokes: List<GrokStroke>,
    /** Solid dots behind the body (shown on Move tiles too). */
    val dotsBack: List<GrokDot>,
    /** Solid dots over the body (shown on Move tiles too). */
    val dotsFront: List<GrokDot>,
    val alertDot: GrokAlertDot?,
)

object GrokBotMotion {
    private const val N = 386
    private const val TAU = PI * 2.0
    private const val DEG = PI / 180.0

    /** Moves performed once on the web clock (the rest keep Android's own looped previews). */
    val ONE_SHOT_MOVES: Set<String> = setOf(
        "thinking", "alert", "exclamation", "sleep", "orbit", "swirl", "burst", "comet",
    )

    /* -------------------------------------------------------------- helpers */

    fun clamp01(v: Float): Float = if (v < 0f) 0f else if (v > 1f) 1f else v
    private fun lerp(a: Float, b: Float, t: Float): Float = a + (b - a) * t
    fun easeOutCubic(x: Float): Float = 1f - (1f - x).pow(3)
    fun easeInOutSine(x: Float): Float = (-(cos(PI * x) - 1.0) / 2.0).toFloat()
    private fun easeOutQuart(x: Float): Float = 1f - (1f - x).pow(4)

    /** 0 → 1 → 0 bump over |x| < 1. */
    private fun bump(x: Float): Float = if (abs(x) >= 1f) 0f else (0.5 * (1.0 + cos(PI * x))).toFloat()

    /** Piecewise-linear lookup over a flat [x0, y0, x1, y1, …] table, clamped at both ends. */
    fun pl(x: Float, tbl: FloatArray): Float {
        if (x <= tbl[0]) return tbl[1]
        var i = 2
        while (i < tbl.size) {
            if (x <= tbl[i]) {
                val x0 = tbl[i - 2]
                val y0 = tbl[i - 1]
                return y0 + (tbl[i + 1] - y0) * (x - x0) / (tbl[i] - x0)
            }
            i += 2
        }
        return tbl[tbl.size - 1]
    }

    /**
     * Swell of thinking dot [idx] (0 = left, 1 = centre, 2 = right): each dot in turn grows over 0.35 s,
     * holds 0.35 s, then snaps back, one dot every 0.5 s.
     */
    fun thinkSwell(t: Float, idx: Int): Float {
        val ph = (((t - 0.05f - idx * 0.5f) % 1.5f) + 1.5f) % 1.5f
        if (ph < 0.35f) return (ph / 0.35f).pow(1.4f)
        return if (ph < 0.7f) 1f else 0f
    }

    /** How far the orbit's spinning triangle has rounded into a circle (0..1). */
    fun orbitRound(t: Float): Float = easeInOutSine(clamp01((t - 1.75f) / 0.6f))

    /** Horizontal centroid of the alert glyph at [t] seconds into the move. */
    fun alertX(t: Float): Float = pl(t, ALERT_X)

    /** Channel-wise mix of two ARGB colours, rounded like the web's `mixHex`. */
    fun mixArgb(a: Int, b: Int, t: Float): Int {
        fun ch(shift: Int): Int {
            val ca = (a shr shift) and 255
            val cb = (b shr shift) and 255
            return lerp(ca.toFloat(), cb.toFloat(), t).roundToInt().coerceIn(0, 255)
        }
        return (0xFF shl 24) or (ch(16) shl 16) or (ch(8) shl 8) or ch(0)
    }

    private const val INK_STROKE = 0xFF52525B.toInt()
    private const val BLACK = 0xFF000000.toInt()

    /** Solid ink for dots, and the deep shade burst particles start from (as the web `paletteInk`). */
    fun paletteInk(color: String): Pair<Int, Int> {
        val p = GrokBotGeometry.COLOR_PALETTES[color] ?: GrokBotGeometry.COLOR_PALETTES.getValue("blue")
        val solid = if (color == "ink") INK_STROKE else p.stopColors[2].toInt()
        return solid to mixArgb(p.stopColors[3].toInt(), BLACK, 0.55f)
    }

    /**
     * Arc of an ellipse over [u0, u1] turns (θ = 2πu in the ellipse's own frame), split into the half
     * behind the body and the half in front of it (cos θ < 0), the way the reference layers its strokes.
     * The point where the side changes starts the new run, so the crossing segment joins the new side.
     */
    fun ellArc(e: GrokEllipse, u0: Float, u1: Float): GrokArc {
        val back = ArrayList<FloatArray>()
        val front = ArrayList<FloatArray>()
        if (u1 - u0 <= 0.002f) return GrokArc(back, front)
        val cs = cos(e.tilt * DEG)
        val sn = sin(e.tilt * DEG)
        val n = max(2, ceil(128.0 * (u1 - u0)).toInt())
        var pts = ArrayList<Float>()
        var runFront = false
        var side: Boolean? = null
        var px = 0f
        var py = 0f
        fun flush() {
            if (pts.size >= 4) (if (runFront) front else back).add(pts.toFloatArray())
        }
        for (i in 0..n) {
            val th = TAU * (u0 + (u1 - u0) * i / n)
            val lx = e.rx * cos(th)
            val ly = e.ry * sin(th)
            val x = (lx * cs - ly * sn + e.cx).toFloat()
            val y = (lx * sn + ly * cs + e.cy).toFloat()
            val isFront = cos(th) < 0.0
            if (isFront != side) {
                flush()
                pts = ArrayList()
                if (side != null) {
                    pts.add(px)
                    pts.add(py)
                }
                runFront = isFront
                side = isFront
            }
            pts.add(x)
            pts.add(y)
            px = x
            py = y
        }
        flush()
        return GrokArc(back, front)
    }

    /* ---------------------------------------------------------------- data */

    /** Orbit ring opacity while the rings appear one after another (reference stage, t in s). */
    val ORBIT_RING_IN: List<FloatArray> = listOf(
        floatArrayOf(0.07f, 0f, 0.1f, 0.02f, 0.15f, 0.07f, 0.2f, 0.14f, 0.25f, 0.24f, 0.3f, 0.36f, 0.35f, 0.43f, 0.4f, 0.5f, 0.45f, 0.56f, 0.5f, 0.62f, 0.55f, 0.69f, 0.6f, 0.75f, 0.65f, 0.81f, 0.7f, 0.87f, 0.75f, 0.94f, 0.8f, 1f),
        floatArrayOf(0.17f, 0f, 0.2f, 0.05f, 0.25f, 0.12f, 0.3f, 0.21f, 0.35f, 0.32f, 0.4f, 0.45f, 0.45f, 0.56f, 0.5f, 0.62f, 0.55f, 0.69f, 0.6f, 0.75f, 0.65f, 0.81f, 0.7f, 0.87f, 0.75f, 0.94f, 0.8f, 1f),
        floatArrayOf(0.28f, 0f, 0.3f, 0.05f, 0.35f, 0.13f, 0.4f, 0.23f, 0.45f, 0.36f, 0.5f, 0.5f, 0.55f, 0.66f, 0.6f, 0.75f, 0.65f, 0.81f, 0.7f, 0.87f, 0.75f, 0.94f, 0.8f, 1f),
        floatArrayOf(0.39f, 0f, 0.4f, 0.02f, 0.45f, 0.11f, 0.5f, 0.23f, 0.55f, 0.37f, 0.6f, 0.52f, 0.65f, 0.7f, 0.7f, 0.87f, 0.75f, 0.94f, 0.8f, 1f),
        floatArrayOf(0.53f, 0f, 0.55f, 0.07f, 0.6f, 0.2f, 0.65f, 0.35f, 0.7f, 0.52f, 0.75f, 0.72f, 0.8f, 0.93f, 0.85f, 1f),
        floatArrayOf(0.655f, 0f, 0.7f, 0.15f, 0.75f, 0.31f, 0.8f, 0.5f, 0.85f, 0.67f, 0.92f, 0.89f, 0.95f, 1f),
    )

    /**
     * While the orbit triangle spins, the eyes sway with the spin phase φ (0–120°, one third of a
     * turn): horizontal offsets of the left and right eye, sampled from the reference.
     */
    val ORBIT_EYE_L = floatArrayOf(0f, 2.4f, 7.5f, -1.1f, 17.5f, -3.5f, 22.5f, -3.7f, 30f, -1.1f, 37.5f, 1.3f, 47.5f, 3.2f, 52.5f, 5.4f, 62.5f, 6.4f, 72.5f, 7.4f, 82.5f, 7.3f, 90f, 6.7f, 97.5f, 6.4f, 105f, 5.1f, 112.5f, 3.4f, 120f, 2.4f)
    val ORBIT_EYE_R = floatArrayOf(0f, -6.5f, 7.5f, -7.4f, 17.5f, -8.1f, 22.5f, -8.3f, 30f, -7.45f, 37.5f, -7.2f, 47.5f, -6.3f, 52.5f, -4f, 62.5f, -2.2f, 67.5f, -0.1f, 72.5f, 2.7f, 82.5f, 2.7f, 90f, 0.65f, 97.5f, -2.1f, 105f, -4.05f, 112.5f, -5.8f, 120f, -6.5f)

    /** Extra spin angle (deg) while the orbit triangle winds up to its steady 450°/s. */
    val ORBIT_SPIN_LAG = floatArrayOf(0f, 0f, 0.05f, 22f, 0.1f, 42f, 0.15f, 46f, 0.2f, 34f, 0.25f, 16f, 0.3f, 6f, 0.35f, 2f, 0.45f, 0f)

    /** Horizontal centroid of the alert glyph: it slides right, pauses, then hurries back. */
    val ALERT_X = floatArrayOf(0f, -8.4f, 0.2f, -7.97f, 0.3f, -6.8f, 0.4f, -4.18f, 0.5f, 0.6f, 0.6f, 7.79f, 0.7f, 18.29f, 0.8f, 32.58f, 0.9f, 46.9f, 1.0f, 57.4f, 1.1f, 64.72f, 1.2f, 69.41f, 1.3f, 72.06f, 1.4f, 73.24f, 1.5f, 73.56f, 1.65f, 73.57f, 2.05f, 10.25f)

    /** Burst particles: birth time (s), starting angle (deg) and distance; each spirals into the dot. */
    val BURST_PARTICLES: List<FloatArray> = listOf(
        floatArrayOf(0.035f, -116f, 57.5f),
        floatArrayOf(0.205f, -128f, 61f),
        floatArrayOf(0.405f, -7.5f, 60f),
        floatArrayOf(0.615f, -102f, 61f),
        floatArrayOf(0.815f, 53f, 64f),
    )

    /*
     * Exact 64-segment move glyphs from the grokbots.ai studio stage (identical for every body shape).
     * `alert` is captured with its centroid at (46.9, -32.7), `exclamation` at rest, and `orbit` is the
     * spinning triangle at t = 1.001 s of the move (it turns 450°/s about the origin).
     */

    private const val GLYPH_ALERT_PATH = "M59.62 -28.65C59.49 -28.23 59.35 -27.81 59.22 -27.38C59.08 -26.95 58.95 -26.53 58.81 -26.09C58.67 -25.65 58.53 -25.21 58.38 -24.74C58.23 -24.28 58.09 -23.81 57.93 -23.32C57.77 -22.82 57.61 -22.31 57.44 -21.77C57.26 -21.22 57.08 -20.66 56.89 -20.05C56.69 -19.43 56.49 -18.8 56.27 -18.08C56.04 -17.37 55.8 -16.63 55.53 -15.78C55.26 -14.92 54.98 -14.03 54.64 -12.96C54.3 -11.9 53.95 -10.78 53.5 -9.38C53.06 -7.98 52.63 -6.13 51.97 -4.57C51.31 -3 50.49 -1.24 49.55 0.01C48.6 1.25 47.45 2.16 46.3 2.91C45.14 3.67 43.87 4.2 42.61 4.54C41.36 4.89 40.03 5.03 38.77 5.01C37.5 4.98 36.21 4.76 35.01 4.38C33.82 3.99 32.64 3.42 31.59 2.71C30.54 1.99 29.54 1.11 28.72 0.09C27.9 -0.92 27.16 -2.09 26.66 -3.38C26.15 -4.67 25.74 -6.08 25.68 -7.64C25.63 -9.21 25.98 -11.12 26.34 -12.78C26.71 -14.44 27.43 -16.2 27.88 -17.6C28.32 -19 28.67 -20.11 29.01 -21.18C29.35 -22.25 29.63 -23.14 29.91 -23.99C30.18 -24.85 30.41 -25.59 30.64 -26.3C30.87 -27.01 31.07 -27.65 31.26 -28.26C31.46 -28.88 31.64 -29.44 31.81 -29.98C31.98 -30.53 32.14 -31.04 32.3 -31.53C32.46 -32.03 32.61 -32.5 32.75 -32.96C32.9 -33.42 33.04 -33.87 33.18 -34.31C33.32 -34.74 33.46 -35.17 33.59 -35.6C33.73 -36.02 33.86 -36.44 34 -36.87C34.13 -37.29 34.26 -37.71 34.4 -38.13C34.53 -38.56 34.67 -38.99 34.81 -39.43C34.95 -39.87 35.09 -40.31 35.24 -40.77C35.38 -41.23 35.53 -41.7 35.69 -42.2C35.85 -42.69 36.01 -43.2 36.18 -43.75C36.35 -44.29 36.53 -44.85 36.73 -45.47C36.92 -46.08 37.13 -46.72 37.35 -47.43C37.58 -48.14 37.81 -48.89 38.08 -49.74C38.36 -50.59 38.64 -51.49 38.98 -52.55C39.32 -53.62 39.67 -54.73 40.12 -56.13C40.56 -57.53 40.99 -59.38 41.65 -60.95C42.3 -62.51 43.12 -64.28 44.07 -65.52C45.02 -66.77 46.16 -67.67 47.32 -68.43C48.48 -69.18 49.75 -69.71 51.01 -70.06C52.26 -70.41 53.58 -70.55 54.85 -70.52C56.12 -70.49 57.41 -70.28 58.61 -69.89C59.8 -69.51 60.98 -68.93 62.03 -68.22C63.08 -67.51 64.07 -66.62 64.9 -65.61C65.72 -64.59 66.45 -63.42 66.96 -62.13C67.47 -60.84 67.88 -59.44 67.93 -57.87C67.99 -56.3 67.64 -54.39 67.27 -52.73C66.91 -51.07 66.19 -49.31 65.74 -47.91C65.3 -46.52 64.94 -45.4 64.6 -44.33C64.27 -43.27 63.98 -42.38 63.71 -41.52C63.44 -40.67 63.2 -39.93 62.98 -39.21C62.75 -38.5 62.55 -37.86 62.35 -37.25C62.16 -36.64 61.98 -36.07 61.81 -35.53C61.63 -34.98 61.47 -34.48 61.32 -33.98C61.16 -33.49 61.01 -33.02 60.86 -32.55C60.72 -32.09 60.57 -31.65 60.44 -31.21C60.3 -30.77 60.16 -30.34 60.02 -29.92C59.89 -29.49 59.76 -29.07 59.62 -28.65Z"

    private const val GLYPH_EXCLAMATION_PATH = "M9.86 -18.48C9.83 -18.14 9.8 -17.81 9.77 -17.47C9.74 -17.13 9.71 -16.8 9.68 -16.45C9.65 -16.11 9.62 -15.77 9.58 -15.42C9.55 -15.06 9.52 -14.71 9.49 -14.34C9.45 -13.96 9.42 -13.59 9.38 -13.19C9.35 -12.79 9.31 -12.38 9.27 -11.94C9.23 -11.5 9.19 -11.04 9.15 -10.54C9.1 -10.05 9.05 -9.53 9 -8.95C8.95 -8.37 8.9 -7.77 8.83 -7.08C8.77 -6.38 8.7 -5.65 8.63 -4.78C8.55 -3.91 8.47 -3 8.36 -1.85C8.26 -0.7 8.15 0.49 8 2.12C7.86 3.75 7.75 5.58 7.48 7.93C7.21 10.27 7.08 14.23 6.37 16.2C5.66 18.17 4.38 18.99 3.23 19.75C2.08 20.51 0.72 20.77 -0.53 20.77C-1.79 20.77 -3.15 20.51 -4.3 19.75C-5.45 18.99 -6.72 18.17 -7.43 16.2C-8.14 14.23 -8.27 10.27 -8.54 7.93C-8.81 5.58 -8.92 3.75 -9.07 2.12C-9.21 0.49 -9.32 -0.7 -9.42 -1.85C-9.53 -3 -9.61 -3.91 -9.69 -4.78C-9.77 -5.65 -9.83 -6.38 -9.89 -7.08C-9.96 -7.77 -10.01 -8.37 -10.06 -8.95C-10.12 -9.53 -10.16 -10.05 -10.21 -10.54C-10.25 -11.04 -10.29 -11.5 -10.33 -11.94C-10.37 -12.38 -10.41 -12.79 -10.44 -13.19C-10.48 -13.59 -10.52 -13.96 -10.55 -14.34C-10.58 -14.71 -10.61 -15.06 -10.65 -15.42C-10.68 -15.77 -10.71 -16.11 -10.74 -16.45C-10.77 -16.8 -10.8 -17.13 -10.83 -17.47C-10.86 -17.81 -10.89 -18.14 -10.92 -18.48C-10.95 -18.83 -10.98 -19.17 -11.02 -19.52C-11.05 -19.87 -11.08 -20.22 -11.11 -20.59C-11.15 -20.96 -11.18 -21.33 -11.21 -21.73C-11.25 -22.12 -11.29 -22.52 -11.33 -22.96C-11.36 -23.39 -11.4 -23.83 -11.45 -24.32C-11.49 -24.81 -11.54 -25.31 -11.59 -25.87C-11.64 -26.44 -11.69 -27.02 -11.75 -27.69C-11.81 -28.37 -11.88 -29.07 -11.95 -29.91C-12.03 -30.74 -12.11 -31.62 -12.2 -32.71C-12.3 -33.8 -12.41 -34.94 -12.54 -36.46C-12.68 -37.99 -12.83 -39.56 -13.03 -41.87C-13.23 -44.18 -13.88 -47.88 -13.72 -50.33C-13.57 -52.79 -12.9 -54.91 -12.09 -56.59C-11.28 -58.28 -10.08 -59.44 -8.88 -60.46C-7.68 -61.47 -6.28 -62.2 -4.88 -62.69C-3.49 -63.19 -1.98 -63.44 -0.53 -63.44C0.92 -63.44 2.43 -63.19 3.82 -62.69C5.21 -62.2 6.62 -61.47 7.82 -60.46C9.02 -59.44 10.22 -58.28 11.03 -56.59C11.84 -54.91 12.5 -52.79 12.66 -50.33C12.82 -47.88 12.17 -44.18 11.97 -41.87C11.77 -39.56 11.62 -37.99 11.48 -36.46C11.34 -34.94 11.24 -33.8 11.14 -32.71C11.04 -31.62 10.97 -30.74 10.89 -29.91C10.82 -29.07 10.75 -28.37 10.69 -27.69C10.63 -27.02 10.58 -26.44 10.53 -25.87C10.48 -25.31 10.43 -24.81 10.39 -24.32C10.34 -23.83 10.3 -23.39 10.26 -22.96C10.23 -22.52 10.19 -22.12 10.15 -21.73C10.12 -21.33 10.08 -20.96 10.05 -20.59C10.02 -20.22 9.99 -19.87 9.95 -19.52C9.92 -19.17 9.89 -18.83 9.86 -18.48Z"

    private const val GLYPH_ORBIT_PATH = "M21.18 -78.27C23.75 -79.4 26.38 -80.53 29.23 -81.8C32.07 -83.08 35.02 -84.45 38.24 -85.89C41.47 -87.34 44.9 -89.01 48.58 -90.46C52.26 -91.91 56.26 -93.53 60.3 -94.59C64.34 -95.65 68.81 -96.75 72.84 -96.81C76.88 -96.87 81.14 -96.44 84.52 -94.94C87.9 -93.43 90.94 -90.85 93.12 -87.77C95.3 -84.7 96.65 -80.5 97.6 -76.49C98.56 -72.49 98.74 -67.93 98.84 -63.76C98.94 -59.58 98.51 -55.32 98.21 -51.44C97.91 -47.56 97.41 -43.9 97.05 -40.48C96.69 -37.06 96.37 -33.93 96.05 -30.9C95.73 -27.87 95.45 -25.05 95.15 -22.29C94.85 -19.54 94.51 -16.92 94.25 -14.36C93.98 -11.79 93.82 -9.35 93.56 -6.92C93.3 -4.49 92.94 -2.12 92.69 0.24C92.43 2.6 92.25 4.91 92.01 7.24C91.78 9.57 91.53 11.88 91.28 14.24C91.03 16.59 90.76 18.93 90.51 21.35C90.26 23.77 90.06 26.23 89.79 28.77C89.53 31.31 89.21 33.87 88.91 36.58C88.61 39.3 88.32 42.08 88 45.06C87.69 48.05 87.38 51.14 87.02 54.49C86.67 57.84 86.4 61.45 85.86 65.18C85.33 68.91 84.81 72.98 83.83 76.88C82.85 80.78 81.72 85.01 79.97 88.58C78.23 92.15 76.09 95.83 73.37 98.28C70.66 100.73 67.23 102.54 63.69 103.28C60.15 104.02 56 103.58 52.14 102.72C48.29 101.86 44.23 99.93 40.57 98.11C36.91 96.29 33.39 93.92 30.16 91.8C26.93 89.68 23.97 87.42 21.18 85.39C18.38 83.37 15.84 81.44 13.39 79.65C10.93 77.85 8.65 76.24 6.44 74.63C4.22 73.02 2.14 71.49 0.1 69.99C-1.94 68.5 -3.9 67.09 -5.82 65.68C-7.75 64.28 -9.62 62.92 -11.47 61.55C-13.32 60.19 -15.11 58.83 -16.93 57.49C-18.75 56.16 -20.57 54.88 -22.4 53.54C-24.22 52.21 -26.02 50.85 -27.88 49.49C-29.75 48.12 -31.64 46.77 -33.58 45.35C-35.53 43.94 -37.5 42.49 -39.56 40.98C-41.63 39.47 -43.72 37.92 -45.98 36.27C-48.24 34.63 -50.59 32.96 -53.1 31.13C-55.62 29.29 -58.23 27.39 -61.07 25.28C-63.91 23.18 -67.11 20.99 -70.14 18.47C-73.17 15.95 -76.49 13.21 -79.26 10.17C-82.02 7.13 -84.94 3.72 -86.73 0.24C-88.52 -3.25 -89.91 -7.15 -89.99 -10.76C-90.08 -14.37 -89.02 -18.15 -87.24 -21.42C-85.46 -24.68 -82.32 -27.72 -79.31 -30.37C-76.3 -33.02 -72.61 -35.3 -69.17 -37.34C-65.72 -39.37 -61.98 -41 -58.64 -42.6C-55.29 -44.19 -52.09 -45.55 -49.08 -46.9C-46.07 -48.24 -43.27 -49.45 -40.59 -50.65C-37.9 -51.86 -35.41 -53.01 -32.96 -54.12C-30.5 -55.22 -28.15 -56.26 -25.85 -57.3C-23.55 -58.33 -21.36 -59.37 -19.15 -60.35C-16.93 -61.33 -14.74 -62.21 -12.57 -63.16C-10.41 -64.11 -8.31 -65.08 -6.17 -66.05C-4.03 -67.02 -1.91 -67.99 0.27 -68.98C2.44 -69.96 4.62 -70.94 6.88 -71.94C9.13 -72.94 11.42 -73.93 13.8 -74.98C16.18 -76.03 18.61 -77.13 21.18 -78.27Z"

    private const val ALERT_DOT_PATH = "M-0.09 0.07L-0.1 0.06L-0.1 0.05L-0.11 0.04L-0.11 0.03L-0.12 0.02L-0.12 0.01L-0.12 0L-0.12 -0.01L-0.12 -0.02L-0.11 -0.03L-0.11 -0.04L-0.11 -0.05L-0.1 -0.06L-0.09 -0.07L-0.09 -0.08L-0.08 -0.09L-0.07 -0.09L-0.06 -0.1L-0.05 -0.11L-0.04 -0.11L-0.03 -0.11L-0.02 -0.12L-0.01 -0.12L0 -0.12L0.01 -0.12L0.02 -0.12L0.03 -0.11L0.04 -0.11L0.05 -0.11L0.06 -0.1L0.07 -0.09L0.08 -0.09L0.09 -0.08L0.09 -0.07L0.1 -0.06L0.11 -0.05L0.11 -0.04L0.11 -0.03L0.12 -0.02L0.12 -0.01L0.12 0L0.12 0.01L0.12 0.02L0.11 0.03L0.11 0.04L0.1 0.05L0.1 0.06L0.09 0.07L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0.01 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L0 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18L-0.01 0.18Z"

    val RINGS: List<GrokEllipse> = listOf(
        GrokEllipse(0f, 10f, 37.78f, 137.43f, -83.6f, 5.26f, -136.58f, -5.31f, 136.58f, 25.31f, longArrayOf(0xFFD37169, 0xFFD3A969, 0xFFC5D369)),
        GrokEllipse(0f, 10f, 42.72f, 134.78f, -32.17f, 6.02f, -71.76f, -104.09f, 71.76f, 124.09f, longArrayOf(0xFFADD369, 0xFF72D369, 0xFF69D39B)),
        GrokEllipse(0f, 10f, 56.87f, 139.6f, -26.75f, 5.62f, -62.83f, -114.67f, 62.83f, 134.67f, longArrayOf(0xFF69D39E, 0xFF69D2D3, 0xFF699CD3)),
        GrokEllipse(0f, 10f, 54.89f, 138.54f, 24.37f, 5.17f, 57.16f, -116.2f, -57.16f, 136.2f, longArrayOf(0xFF69C9D3, 0xFF6969D3, 0xFFCA69D3)),
        GrokEllipse(0f, 10f, 56.83f, 137.51f, 56.35f, 5.26f, 114.47f, -66.2f, -114.47f, 86.2f, longArrayOf(0xFF9E69D3, 0xFFD369C8, 0xFFD36987)),
        GrokEllipse(0f, 10f, 46.92f, 130.38f, 82.79f, 5.55f, 129.35f, -6.36f, -129.35f, 26.36f, longArrayOf(0xFFD369A3, 0xFFD37C69, 0xFFD3CA69)),
    )

    val COMET: List<GrokEllipse> = listOf(
        GrokEllipse(0f, 0f, 10.89f, 81.17f, -59.01f, 9.5f, -69.59f, -41.8f, 69.59f, 41.8f, longArrayOf(0xFFD37069, 0xFFD3B769, 0xFFA9D369)),
        GrokEllipse(0f, 0f, 13.59f, 83.73f, -57f, 9.5f, -70.22f, -45.6f, 70.22f, 45.6f, longArrayOf(0xFF93D369, 0xFF69D386, 0xFF69D3CD)),
        GrokEllipse(0f, 0f, 16.44f, 86.28f, -55f, 9.5f, -70.67f, -49.49f, 70.67f, 49.49f, longArrayOf(0xFF69C7D3, 0xFF6980D3, 0xFF9969D3)),
        GrokEllipse(0f, 0f, 19.44f, 88.83f, -52.99f, 9.5f, -70.93f, -53.47f, 70.93f, 53.47f, longArrayOf(0xFF9969D3, 0xFFD369C7, 0xFFD36980)),
    )

    val GLYPH_ALERT: FloatArray = GrokBotGeometry.parseBezier64Coords(GLYPH_ALERT_PATH)
    val GLYPH_EXCLAMATION: FloatArray = GrokBotGeometry.parseBezier64Coords(GLYPH_EXCLAMATION_PATH)
    val GLYPH_ORBIT: FloatArray = GrokBotGeometry.parseBezier64Coords(GLYPH_ORBIT_PATH)

    /** The alert dot's polygon in unit space (x, y pairs; the path is M/L only, not cubic). */
    val ALERT_DOT_POINTS: FloatArray = GrokBotGeometry.parseBezier64Coords(ALERT_DOT_PATH)

    /** The orbit/swirl rings never move, so their halves are built once. */
    val RING_ARCS: List<GrokArc> by lazy { RINGS.map { ellArc(it, 0f, 1f) } }

    private val CIRCLE: FloatArray get() = GrokBotGeometry.SHAPE_COORDS.getValue("circle")

    /* --------------------------------------------------------------- moves */

    private class Ctx(
        val t: Float,
        val solid: Int,
        val deep: Int,
        val faceEyes: Pair<GrokEyePose, GrokEyePose>,
        val out: FloatArray,
        val scratch: FloatArray,
    )

    private class Fx(
        val back: List<GrokStroke> = emptyList(),
        val front: List<GrokStroke> = emptyList(),
        val dots: List<GrokDot> = emptyList(),
        val dotsBack: List<GrokDot> = emptyList(),
        val alertDot: GrokAlertDot? = null,
    )

    private class MoveDef(
        val dur: Float,
        val inDur: Float,
        val outDur: Float,
        /** Seconds after the tap before the body starts morphing. */
        val inDelay: Float = 0f,
        /** Hidden eyes start fading this long after the tap (defaults to [inDelay]). */
        val eyeInDelay: Float = inDelay,
        /** Pill-shaped eyes resize smoothly into the move's eyes instead of swapping mid-blink. */
        val morphEyes: Boolean = false,
        /** Time (s) used for the static Move tile. */
        val preview: Float,
        /** Eyes slide together and fade out while the body becomes a glyph or dot. */
        val hideEyes: Boolean = false,
        /** The move ends on a circle (the web keeps the circle eye layout until the final morph). */
        val endsOnCircle: Boolean = false,
        /** For hideEyes moves that pop back into a circle with a face before the end (0..1). */
        val eyeReturn: ((Float) -> Float)? = null,
        val body: ((Ctx) -> FloatArray)? = null,
        val eyes: ((Ctx) -> Pair<GrokEyePose, GrokEyePose>)? = null,
        val fx: ((Ctx, Float) -> Fx)? = null,
    )

    /** Writes the circle scaled to radius [r] at ([cx], [cy]). */
    private fun dot(out: FloatArray, r: Float, cx: Float, cy: Float): FloatArray {
        GrokBotGeometry.lerpCoords(CIRCLE, CIRCLE, 0f, out, r / 100f, 0f, cx, cy)
        return out
    }

    /** Body pops from a small dot at ([cx], [cy]) to the full circle between [t0] and t0 + 0.45. */
    private fun popCircle(c: Ctx, rFrom: Float, t0: Float, cx: Float, cy: Float): FloatArray? {
        if (c.t < t0) return null
        val k = easeOutCubic(clamp01((c.t - t0) / 0.45f))
        return dot(c.out, lerp(rFrom, 100f, k), cx * (1f - k), cy * (1f - k))
    }

    private fun ringStrokes(arc: GrokArc, e: GrokEllipse, alpha: Float, back: MutableList<GrokStroke>, front: MutableList<GrokStroke>) {
        arc.back.forEach { back.add(GrokStroke(it, e, alpha)) }
        arc.front.forEach { front.add(GrokStroke(it, e, alpha)) }
    }

    private val SWIRL_START = floatArrayOf(0.075f, 0.12f, 0.17f)

    private val MOVES: Map<String, MoveDef> = mapOf(
        "thinking" to MoveDef(
            dur = 2.915f, inDur = 0.27f, outDur = 0.3f, preview = 1.0f, hideEyes = true,
            // The body is the centre dot; the side dots slide out of it, then each swells in turn.
            body = { c -> dot(c.out, 16.5f + 4.13f * thinkSwell(c.t, 1), -1.7f, -0.4f) },
            fx = { c, env ->
                val slide = easeOutQuart(clamp01(c.t / 0.3f))
                Fx(
                    dots = listOf(-56.2f to 0, 52.7f to 2).map { (x, idx) ->
                        val w = thinkSwell(c.t, idx)
                        GrokDot(-1.7f + (x + 1.7f) * slide, -0.4f, 16.5f + 4.13f * w, c.solid, (0.55f + 0.45f * w) * env)
                    },
                )
            },
        ),
        "alert" to MoveDef(
            dur = 2.367f, inDur = 0.3f, outDur = 0.3f, inDelay = 0.05f, preview = 0.3f, hideEyes = true,
            // A tilted "!" slides right, pauses, then hurries back; glyph and dot bob against each other.
            body = { c ->
                val x = alertX(c.t)
                val y = -32.4f - 0.5f * cos(TAU * (c.t - 0.95f) / 0.4f).toFloat()
                GrokBotGeometry.lerpCoords(GLYPH_ALERT, GLYPH_ALERT, 0f, c.out, 1f, 0f, x - 46.9f, y + 32.7f)
                c.out
            },
            fx = { c, env ->
                val x = alertX(c.t) - 17.72f
                val y = 22.83f + 1.42f * cos(TAU * (c.t - 0.95f) / 0.4f).toFloat()
                Fx(alertDot = GrokAlertDot(x, y, c.solid, env))
            },
        ),
        "exclamation" to MoveDef(
            dur = 2.367f, inDur = 0.3f, outDur = 0.3f, inDelay = 0.05f, preview = 1.0f, hideEyes = true,
            body = { GLYPH_EXCLAMATION },
            fx = { c, env -> Fx(dots = listOf(GrokDot(-1.2f, 52.6f, 11.3f, c.solid, env))) },
        ),
        "sleep" to MoveDef(
            dur = 2.767f, inDur = 0.3f, outDur = 0.3f, inDelay = 0.05f, preview = 0.95f, hideEyes = true,
            // A small dot bouncing every 0.6 s.
            body = { c -> dot(c.out, 15.85f, 0.05f, 11.15f + 19.05f * cos(TAU * (c.t - 0.767f) / 0.6f).toFloat()) },
        ),
        "orbit" to MoveDef(
            dur = 2.87f, inDur = 0.35f, outDur = 0.3f, preview = 1.0f, endsOnCircle = true, morphEyes = true,
            // A triangle spinning 450°/s about the centre while six rings appear; it rounds into a circle.
            body = { c ->
                // The spin eases in (lagging up to 46°), and the body freezes once it starts settling back.
                val t = min(c.t, 2.52f)
                val rotDeg = (1.001f - t) * 450f + pl(t, ORBIT_SPIN_LAG)
                GrokBotGeometry.lerpCoords(GLYPH_ORBIT, GLYPH_ORBIT, 0f, c.out, 1f, rotDeg)
                val k = orbitRound(t)
                if (k > 0f) {
                    // The circle's anchors trail the triangle's by 90° so the rounding doesn't twist.
                    GrokBotGeometry.lerpCoords(CIRCLE, CIRCLE, 0f, c.scratch, 1f, rotDeg - 90f)
                    GrokBotGeometry.lerpCoords(c.out, c.scratch, k, c.out)
                }
                c.out
            },
            eyes = { c ->
                val k = orbitRound(c.t)
                val ph = ((((c.t - 1.001f) * 450f) % 120f) + 120f) % 120f
                val dl = pl(ph, ORBIT_EYE_L) * (1f - k)
                val dr = pl(ph, ORBIT_EYE_R) * (1f - k)
                val pill = GrokBotGeometry.pill(9f, lerp(8f, 11.5f, k))
                val (l, r) = c.faceEyes
                Pair(
                    GrokEyePose(pill, l.x + dl, l.y - 0.24f * dl, l.deg, l.sy),
                    GrokEyePose(pill, r.x + dr, r.y - 0.24f * dr, r.deg, r.sy),
                )
            },
            fx = { c, _ ->
                val fade = 1f - easeOutCubic(clamp01((c.t - 2.53f) / 0.3f))
                val back = ArrayList<GrokStroke>()
                val front = ArrayList<GrokStroke>()
                RINGS.forEachIndexed { i, r ->
                    val op = pl(c.t, ORBIT_RING_IN[i]) * fade
                    if (op > 0.005f) ringStrokes(RING_ARCS[i], r, op, back, front)
                }
                Fx(back = back, front = front)
            },
        ),
        "swirl" to MoveDef(
            dur = 1.3f, inDur = 0.05f, outDur = 0.05f, preview = 0.6f,
            // Three of the orbit rings flash in one after another around a blink, then fade together.
            eyes = { c ->
                val blink = 1f - 0.77f * bump((c.t - 0.13f) / 0.1f)
                val (l, r) = c.faceEyes
                Pair(l.copy(sy = l.sy * blink), r.copy(sy = r.sy * blink))
            },
            fx = { c, _ ->
                val fade = clamp01((1.27f - c.t) / 0.34f)
                val back = ArrayList<GrokStroke>()
                val front = ArrayList<GrokStroke>()
                for (i in 0 until 3) {
                    val op = clamp01((c.t - SWIRL_START[i]) / 0.125f) * fade
                    if (op > 0.005f) ringStrokes(RING_ARCS[i], RINGS[i], op, back, front)
                }
                Fx(back = back, front = front)
            },
        ),
        "burst" to MoveDef(
            dur = 2.767f, inDur = 0.42f, outDur = 0.3f, inDelay = 0.05f, eyeInDelay = 0f, preview = 0.7f,
            hideEyes = true, endsOnCircle = true,
            eyeReturn = { t -> clamp01((t - 1.86f) / 0.385f) },
            // Dark particles spiral in from all sides, lightening as they merge into the dot behind it;
            // then the dot pops into a big round bot.
            body = { c -> popCircle(c, 16.6f, 1.698f, 0.3f, 0f) ?: dot(c.out, 16.6f, 0.3f, 0f) },
            fx = { c, _ ->
                val light = mixArgb(c.solid, BLACK, 0.12f)
                val dots = ArrayList<GrokDot>()
                for (p in BURST_PARTICLES) {
                    val age = c.t - p[0]
                    if (age < 0f || age > 0.6f) continue
                    val u = age / 0.6f
                    val ang = (p[1] + 60f * u) * DEG
                    val dist = 11f + (p[2] - 11f) * (1f - u).pow(1.8f)
                    val r = 4f + 2.8f * min(1f, age / 0.53f)
                    val op = min(1f, min(age / 0.06f, (0.6f - age) / 0.1f))
                    dots.add(
                        GrokDot(
                            (cos(ang) * dist).toFloat(),
                            (sin(ang) * dist).toFloat(),
                            r,
                            mixArgb(c.deep, light, u.pow(0.75f)),
                            op,
                        ),
                    )
                }
                Fx(dotsBack = dots)
            },
        ),
        "comet" to MoveDef(
            dur = 2.75f, inDur = 0.36f, outDur = 0.3f, inDelay = 0.04f, eyeInDelay = 0f, preview = 0.9f,
            hideEyes = true, endsOnCircle = true,
            eyeReturn = { t -> clamp01((t - 2.0f) / 0.35f) },
            // A four-strand rainbow tail (a third of a loop long) circles the dot once at a steady pace,
            // passing behind it on the way out and in front of it on the way back.
            body = { c -> popCircle(c, 13f, 1.85f, -0.4f, 2f) ?: dot(c.out, 13f, -0.4f, 2f) },
            fx = { c, _ ->
                val op = clamp01((c.t - 0.15f) / 0.25f) * clamp01((1.95f - c.t) / 0.3f)
                if (op <= 0.005f) {
                    Fx()
                } else {
                    val u0 = 0.367f + 0.5835f * (c.t - 0.201f)
                    val back = ArrayList<GrokStroke>()
                    val front = ArrayList<GrokStroke>()
                    COMET.forEach { e -> ringStrokes(ellArc(e, u0, u0 + 0.341f), e, op, back, front) }
                    Fx(back = back, front = front)
                }
            },
        ),
    )

    /** Length of a complex move in seconds (0 for moves this engine doesn't drive). */
    fun duration(move: String): Float = MOVES[move]?.dur ?: 0f

    /** Time (s) of the still frame shown on a Move tile. */
    fun previewTime(move: String): Float = MOVES[move]?.preview ?: 0f

    /* ---------------------------------------------------------------- eyes */

    /** Vertical pill from [GrokBotGeometry.pill] with a straight middle (the only cutouts that resize). */
    private fun GrokEyePill.isVerticalPill(): Boolean =
        halfH > halfW + 1e-3f && abs(rx - halfW) < 1e-3f && abs(ry - rx) < 1e-3f

    fun blendEyes(
        a: Pair<GrokEyePose, GrokEyePose>,
        b: Pair<GrokEyePose, GrokEyePose>,
        w: Float,
        morph: Boolean = false,
    ): Pair<GrokEyePose, GrokEyePose> {
        if (w <= 0f) return a
        if (w >= 1f) return b
        fun one(p: GrokEyePose, q: GrokEyePose): GrokEyePose {
            val same = p.pill == q.pill
            // Two pills can resize into each other; any other change of cutout swaps while squashed
            // shut, so it reads as a blink.
            if (morph && !same && p.pill.isVerticalPill() && q.pill.isVerticalPill()) {
                val rx = lerp(p.pill.rx, q.pill.rx, w)
                val straight = lerp(p.pill.halfH - p.pill.rx, q.pill.halfH - q.pill.rx, w)
                return GrokEyePose(
                    GrokBotGeometry.pill(rx, straight),
                    lerp(p.x, q.x, w),
                    lerp(p.y, q.y, w),
                    lerp(p.deg, q.deg, w),
                    lerp(p.sy, q.sy, w),
                )
            }
            val squash = if (same) 1f else max(0.04f, abs(1f - 2f * w))
            return GrokEyePose(
                if (same || w < 0.5f) p.pill else q.pill,
                lerp(p.x, q.x, w),
                lerp(p.y, q.y, w),
                lerp(p.deg, q.deg, w),
                lerp(p.sy, q.sy, w) * squash,
            )
        }
        return Pair(one(a.first, b.first), one(a.second, b.second))
    }

    /* --------------------------------------------------------------- frame */

    /**
     * One frame of complex [move] at [t] seconds after the tap. The body morphs out of [rest] (the
     * user's shape) into the move's pose and back; [faceEyes] are the face's resting eye poses.
     * Move tiles ([tile]) are stills without strokes and with hidden eyes fully gone, like the
     * reference picker. [body] and the scratch buffers may be reused between frames.
     */
    fun frame(
        move: String,
        t: Float,
        rest: FloatArray,
        faceEyes: Pair<GrokEyePose, GrokEyePose>,
        color: String,
        tile: Boolean = false,
        body: FloatArray = FloatArray(N),
        out: FloatArray = FloatArray(N),
        scratch: FloatArray = FloatArray(N),
    ): GrokMoveFrame? {
        val def = MOVES[move] ?: return null
        val (solid, deep) = paletteInk(color)
        val c = Ctx(t, solid, deep, faceEyes, out, scratch)
        val outStart = def.dur - def.outDur
        val envIn = if (def.inDur > 0f) easeOutCubic(clamp01((t - def.inDelay) / def.inDur)) else 1f
        // The reference snaps back quickly, then settles: a front-loaded ease.
        val envOut = if (def.outDur > 0f) 1f - easeOutCubic(clamp01((t - outStart) / def.outDur)) else 1f
        val env = min(envIn, envOut)
        val target = def.body?.invoke(c) ?: rest
        GrokBotGeometry.lerpCoords(rest, target, if (t < outStart) envIn else envOut, body)

        // Android's face layout is the same for every shape, so the circle-layout hand-off the web
        // does for moves that end on a circle is a no-op here.
        val settled = faceEyes
        val eyes: Pair<GrokEyePose, GrokEyePose>
        var eyeOpacity = 1f
        if (def.hideEyes) {
            // The eyes keep their shape: they slide in toward the shrinking body's centre (×0.22)
            // and fade out, then reverse on the way back.
            val eyeIn = easeOutCubic(clamp01((t - def.eyeInDelay) / 0.3f))
            val k = if (t < outStart) eyeIn else envOut
            val back = def.eyeReturn?.invoke(t) ?: 0f
            val squeeze = if (back > 0f) 1f else 1f - 0.78f * k
            eyes = Pair(
                settled.first.copy(x = settled.first.x * squeeze, y = settled.first.y * squeeze),
                settled.second.copy(x = settled.second.x * squeeze, y = settled.second.y * squeeze),
            )
            eyeOpacity = if (tile) 0f else max(1f - k, back)
        } else {
            val moveEyes = def.eyes?.invoke(c)
            eyes = if (moveEyes != null) blendEyes(settled, moveEyes, env, def.morphEyes) else settled
        }
        val fx = def.fx?.invoke(c, env) ?: Fx()
        return GrokMoveFrame(
            body = body,
            eyes = eyes,
            eyeOpacity = eyeOpacity,
            backStrokes = if (tile) emptyList() else fx.back,
            frontStrokes = if (tile) emptyList() else fx.front,
            dotsBack = fx.dotsBack,
            dotsFront = fx.dots,
            alertDot = fx.alertDot,
        )
    }
}
