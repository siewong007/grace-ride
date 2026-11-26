package org.pjstatecm.graceride.ui.components

import android.graphics.Matrix as AndroidMatrix
import android.graphics.Path as AndroidPath
import android.graphics.RectF
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path as ComposePath
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.asComposePath
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.luminance
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.sin

internal class RadialBrushCache(
    var brush: Brush? = null,
    var colorKey: String = "",
    var alpha: Float = -1f,
    var cx: Float = Float.NaN,
    var cy: Float = Float.NaN,
    var radius: Float = -1f,
)

private fun easeInOutSine(x: Float): Float = (-(cos(PI * x.coerceIn(0f, 1f)) - 1.0) / 2.0).toFloat()

private fun DrawScope.drawStrokes(
    strokes: List<GrokStroke>,
    scale: Float,
    cx: Float,
    cy: Float,
    path: AndroidPath,
    composePath: ComposePath,
) {
    for (s in strokes) {
        val pts = s.points
        if (pts.size < 2) continue
        path.reset()
        path.moveTo(cx + pts[0] * scale, cy + pts[1] * scale)
        var i = 2
        while (i + 1 < pts.size) {
            path.lineTo(cx + pts[i] * scale, cy + pts[i + 1] * scale)
            i += 2
        }
        val e = s.ellipse
        drawPath(
            composePath,
            brush = Brush.linearGradient(
                0f to Color(e.stops[0]),
                0.5f to Color(e.stops[1]),
                1f to Color(e.stops[2]),
                start = Offset(cx + e.x1 * scale, cy + e.y1 * scale),
                end = Offset(cx + e.x2 * scale, cy + e.y2 * scale),
            ),
            alpha = s.alpha.coerceIn(0f, 1f),
            style = Stroke(width = e.width * scale, cap = StrokeCap.Round, join = StrokeJoin.Round),
        )
    }
}

private fun DrawScope.drawDots(dots: List<GrokDot>, scale: Float, cx: Float, cy: Float) {
    for (d in dots) {
        if (d.alpha <= 0.005f || d.r <= 0f) continue
        drawCircle(
            Color(d.argb),
            radius = d.r * scale,
            center = Offset(cx + d.x * scale, cy + d.y * scale),
            alpha = d.alpha.coerceIn(0f, 1f),
        )
    }
}

/** Draws one complex-move frame in the web's layer order: back strokes and dots, body, front. */
private fun DrawScope.drawMoveFrame(
    f: GrokMoveFrame,
    scale: Float,
    cx: Float,
    cy: Float,
    eyeBackingColor: Color,
    bodyBrush: Brush,
    backingPath: AndroidPath,
    backingComposePath: ComposePath,
    bodyPath: AndroidPath,
    bodyComposePath: ComposePath,
    leftEyePath: AndroidPath,
    leftEyeComposePath: ComposePath,
    rightEyePath: AndroidPath,
    scratchPath: AndroidPath,
    scratchComposePath: ComposePath,
    scratchRect: RectF,
    scratchMatrixValues: FloatArray,
    scratchMatrix: AndroidMatrix,
) {
    drawStrokes(f.backStrokes, scale, cx, cy, scratchPath, scratchComposePath)
    drawDots(f.dotsBack, scale, cx, cy)

    fillBezierPath(f.body, backingPath, scale, cx, cy)
    drawPath(backingComposePath, color = eyeBackingColor)
    fillBezierPath(f.body, bodyPath, scale, cx, cy)
    val (le, re) = f.eyes
    fillEyeAndroidPath(
        le.pill, GrokBotGeometry.buildEyeTransform(le), scale, cx, cy,
        leftEyePath, scratchRect, scratchMatrixValues, scratchMatrix,
    )
    fillEyeAndroidPath(
        re.pill, GrokBotGeometry.buildEyeTransform(re), scale, cx, cy,
        rightEyePath, scratchRect, scratchMatrixValues, scratchMatrix,
    )
    if (f.eyeOpacity >= 0.999f) {
        bodyPath.op(leftEyePath, AndroidPath.Op.DIFFERENCE)
        bodyPath.op(rightEyePath, AndroidPath.Op.DIFFERENCE)
        drawPath(bodyComposePath, brush = bodyBrush)
    } else {
        // Fading eyes: lower eye cutout opacity in the body mask shows the backing through the body
        // without double-blending overlapping eyes.
        drawPath(bodyComposePath, brush = bodyBrush)
        if (f.eyeOpacity > 0.01f) {
            leftEyePath.op(rightEyePath, AndroidPath.Op.UNION)
            leftEyePath.op(bodyPath, AndroidPath.Op.INTERSECT)
            drawPath(leftEyeComposePath, color = eyeBackingColor, alpha = f.eyeOpacity)
        }
    }

    drawStrokes(f.frontStrokes, scale, cx, cy, scratchPath, scratchComposePath)
    drawDots(f.dotsFront, scale, cx, cy)
    f.alertDot?.let { a ->
        if (a.alpha <= 0.005f) return@let
        val pts = GrokBotMotion.ALERT_DOT_POINTS
        val rad = 17.7 * PI / 180.0
        val cs = cos(rad).toFloat() * 100f
        val sn = sin(rad).toFloat() * 100f
        scratchPath.reset()
        var i = 0
        while (i + 1 < pts.size) {
            val x = cx + (a.x + pts[i] * cs - pts[i + 1] * sn) * scale
            val y = cy + (a.y + pts[i] * sn + pts[i + 1] * cs) * scale
            if (i == 0) scratchPath.moveTo(x, y) else scratchPath.lineTo(x, y)
            i += 2
        }
        scratchPath.close()
        drawPath(scratchComposePath, color = Color(a.argb), alpha = a.alpha.coerceIn(0f, 1f))
    }
}

@Composable
fun GrokBotCanvas(
    spec: GrokBotSpec,
    modifier: Modifier = Modifier,
    move: String = "idle",
    variant: String = "tile",
    gazeOffset: Offset = Offset.Zero,
    staticMovePreview: Boolean = false,
    /** Bump to replay the current move (complex moves perform once, like the web studio). */
    playKey: Int = 0,
) {
    val isDark = MaterialTheme.colorScheme.background.luminance() < 0.5f
    val animOn = animatorsEnabled()
    // Complex moves run once on their own clock from the tap, then the bot settles back to idle.
    val oneShot = move in GrokBotMotion.ONE_SHOT_MOVES
    val tileStill = staticMovePreview && variant == "tile"
    var moveT by remember { mutableFloatStateOf(Float.MAX_VALUE) }
    LaunchedEffect(move, playKey, oneShot, tileStill, animOn) {
        moveT = Float.MAX_VALUE
        if (!oneShot || tileStill || !animOn) return@LaunchedEffect
        val dur = GrokBotMotion.duration(move)
        val start = withFrameNanos { it }
        moveT = 0f
        while (true) {
            val t = (withFrameNanos { it } - start) / 1_000_000_000f
            moveT = t
            if (t >= dur) break
        }
    }
    val motionT = if (tileStill) GrokBotMotion.previewTime(move) else moveT
    val motionActive = oneShot && (tileStill || (animOn && motionT < GrokBotMotion.duration(move)))
    // The looped previews below never see a complex move; once one has finished the bot idles.
    val loopMove = if (oneShot) "idle" else move

    var prevShape by remember { mutableStateOf(spec.shape) }
    var targetShape by remember { mutableStateOf(spec.shape) }
    val shapeMorphAnim = remember { Animatable(1f) }
    LaunchedEffect(spec.shape, animOn) {
        if (spec.shape != targetShape) {
            prevShape = targetShape
            targetShape = spec.shape
            if (animOn) {
                shapeMorphAnim.snapTo(0f)
                shapeMorphAnim.animateTo(1f, tween(durationMillis = 240))
            } else {
                shapeMorphAnim.snapTo(1f)
            }
        }
    }
    val shapeMorphState = shapeMorphAnim.asState()

    val isAnimated = animOn && !(variant == "tile" && (loopMove == "idle" || staticMovePreview))
    val cycleSecState: State<Float>? = if (isAnimated) {
        val inf = rememberInfiniteTransition(label = "grokMove")
        inf.animateFloat(
            initialValue = 0f,
            targetValue = 12f,
            animationSpec = infiniteRepeatable(
                animation = tween(durationMillis = 12000, easing = LinearEasing),
                repeatMode = RepeatMode.Restart,
            ),
            label = "grokSec",
        )
    } else {
        null
    }

    val workCoords = remember { FloatArray(386) }
    val motionOut = remember { FloatArray(386) }
    val motionScratch = remember { FloatArray(386) }
    val shapeBlendCoords = remember { FloatArray(386) }
    val bodyAndroidPath = remember { AndroidPath() }
    val bodyComposePath = remember { bodyAndroidPath.asComposePath() }
    val backingAndroidPath = remember { AndroidPath() }
    val backingComposePath = remember { backingAndroidPath.asComposePath() }
    val leftEyeAndroidPath = remember { AndroidPath() }
    val rightEyeAndroidPath = remember { AndroidPath() }
    val leftEyeComposePath = remember { leftEyeAndroidPath.asComposePath() }
    val rightEyeComposePath = remember { rightEyeAndroidPath.asComposePath() }
    val scratchPath = remember { AndroidPath() }
    val scratchComposePath = remember { scratchPath.asComposePath() }
    val scratchRect = remember { RectF() }
    val scratchMatrixValues = remember { FloatArray(9) }
    val scratchMatrix = remember { AndroidMatrix() }
    val brushCache = remember { RadialBrushCache() }

    val palette = GrokBotGeometry.COLOR_PALETTES[spec.color] ?: GrokBotGeometry.COLOR_PALETTES.getValue("blue")
    val faceDef = GrokBotGeometry.FACE_DEFS[spec.face] ?: GrokBotGeometry.FACE_DEFS.getValue("neutral")

    val eyeBackingColor = when (spec.color) {
        "cream" -> Color(0xFF22211E)
        "ink" -> Color(0xFFF2F2F5)
        else -> if (isDark) Color(0xFF131519) else Color(0xFFFAF9F6)
    }

    Canvas(modifier = modifier) {
        if (size.minDimension <= 0f) return@Canvas
        val useStaticPose = staticMovePreview || !isAnimated
        val cycleSec = cycleSecState?.value ?: 1.1f
        val shapeMorphT = shapeMorphState.value
        val span = if (variant == "stage") 316f else 250f
        val scale = size.minDimension / span
        val cx = size.width * 0.5f
        val cy = size.height * 0.5f
        val gx = gazeOffset.x.coerceIn(-1f, 1f)
        val gy = gazeOffset.y.coerceIn(-1f, 1f)

        val gradScale = if (variant == "stage") 1.264f else 1f
        val brushCx = cx - 40f * gradScale * scale
        val brushCy = cy - 50f * gradScale * scale
        val brushRadius = (193.75f * gradScale * scale).coerceAtLeast(1f)
        val brush = brushCache.brush?.takeIf {
            brushCache.colorKey == spec.color &&
                brushCache.alpha == 1f &&
                brushCache.cx == brushCx &&
                brushCache.cy == brushCy &&
                brushCache.radius == brushRadius
        } ?: Brush.radialGradient(
            0f to Color(palette.stopColors[0]),
            0.32f to Color(palette.stopColors[1]),
            0.72f to Color(palette.stopColors[2]),
            1f to Color(palette.stopColors[3]),
            center = Offset(brushCx, brushCy),
            radius = brushRadius,
        ).also { created ->
            brushCache.brush = created
            brushCache.colorKey = spec.color
            brushCache.alpha = 1f
            brushCache.cx = brushCx
            brushCache.cy = brushCy
            brushCache.radius = brushRadius
        }

        val fromCoords = GrokBotGeometry.SHAPE_COORDS[prevShape] ?: GrokBotGeometry.SHAPE_COORDS.getValue("circle")
        val toCoords = GrokBotGeometry.SHAPE_COORDS[targetShape] ?: GrokBotGeometry.SHAPE_COORDS.getValue("circle")
        GrokBotGeometry.lerpCoords(fromCoords, toCoords, if (prevShape == targetShape) 1f else shapeMorphT, shapeBlendCoords)

        if (motionActive) {
            val f = GrokBotMotion.frame(
                move = move,
                t = motionT,
                rest = shapeBlendCoords,
                faceEyes = GrokBotGeometry.computeFaceEyePoses(spec.face, gx, gy, 1f, spec.shape),
                color = spec.color,
                tile = tileStill,
                body = workCoords,
                out = motionOut,
                scratch = motionScratch,
            )
            if (f != null) {
                drawMoveFrame(
                    f = f,
                    scale = scale,
                    cx = cx,
                    cy = cy,
                    eyeBackingColor = eyeBackingColor,
                    bodyBrush = brush,
                    backingPath = backingAndroidPath,
                    backingComposePath = backingComposePath,
                    bodyPath = bodyAndroidPath,
                    bodyComposePath = bodyComposePath,
                    leftEyePath = leftEyeAndroidPath,
                    leftEyeComposePath = leftEyeComposePath,
                    rightEyePath = rightEyeAndroidPath,
                    scratchPath = scratchPath,
                    scratchComposePath = scratchComposePath,
                    scratchRect = scratchRect,
                    scratchMatrixValues = scratchMatrixValues,
                    scratchMatrix = scratchMatrix,
                )
                return@Canvas
            }
        }

        var leftPill: GrokEyePill = faceDef.left
        var rightPill: GrokEyePill = faceDef.right
        var leftTr: GrokEyeTransform? = null
        var rightTr: GrokEyeTransform? = null
        var hasNotifCutout = false
        var notifCutoutX = 0f
        var notifCutoutY = 0f
        var notifCutoutR = 0f
        var hasNotifBadge = false
        var notifBadgeX = 0f
        var notifBadgeY = 0f
        var notifBadgeR = 0f

        when (loopMove) {
            "idle" -> {
                val breath = if (isAnimated) sin(cycleSec * 1.9f) * 0.014f else 0f
                val bobY = if (isAnimated) cos(cycleSec * 1.9f) * 1.6f else 0f
                val blinkMod = cycleSec % 4.2f
                val blink = if (isAnimated && blinkMod in 3.95f..4.15f) {
                    val u = abs((blinkMod - 4.05f) / 0.10f)
                    u.coerceAtLeast(0.08f)
                } else 1f
                GrokBotGeometry.lerpCoords(shapeBlendCoords, shapeBlendCoords, 0f, workCoords, 1f + breath, 0f, 0f, bobY)
                val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, blink, spec.shape)
                leftTr = trs.first.copy(ty = trs.first.ty + bobY)
                rightTr = trs.second.copy(ty = trs.second.ty + bobY)
            }
            "wink" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 2.4f) / 2.4f
                val w = if (ph in 0.18f..0.72f) sin(((ph - 0.18f) / 0.54f) * PI.toFloat()) else 0f
                GrokBotGeometry.lerpCoords(shapeBlendCoords, shapeBlendCoords, 0f, workCoords, 1f + w * 0.02f, -3.5f * w)
                leftPill = GrokBotGeometry.WINK_LEFT_EYE
                rightPill = if (w > 0.35f) GrokBotGeometry.WINK_RIGHT_BAR else GrokBotGeometry.WINK_LEFT_EYE
                leftTr = GrokBotGeometry.buildEyeTransform(-28.7f + gx * 10f, -15.6f + gy * 8f, 6.2f)
                rightTr = GrokBotGeometry.buildEyeTransform(28.3f + gx * 10f, -14.8f + gy * 8f, -6.1f, (1f - w * 0.85f).coerceAtLeast(0.18f))
            }
            "wide_eyes" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 2.4f) / 2.4f
                val pop = sin(ph * PI.toFloat())
                GrokBotGeometry.lerpCoords(shapeBlendCoords, shapeBlendCoords, 0f, workCoords, 1f + pop * 0.04f, 0f, 0f, -2f * pop)
                leftPill = GrokBotGeometry.WIDE_EYE
                rightPill = GrokBotGeometry.WIDE_EYE
                val sy = 0.72f + 0.32f * pop
                leftTr = GrokBotGeometry.buildEyeTransform(-31.7f + gx * 12f, -6f + gy * 9f, 0f, sy)
                rightTr = GrokBotGeometry.buildEyeTransform(31.7f + gx * 12f, -6f + gy * 9f, 0f, sy)
            }
            "notification" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 2.8f) / 2.8f
                val pop = if (ph < 0.2f) easeInOutSine(ph / 0.2f) else if (ph < 0.85f) 1f else easeInOutSine((1f - ph) / 0.15f)
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.SHAPE_COORDS.getValue("circle"), pop * 0.5f, workCoords)
                leftPill = GrokBotGeometry.NOTIF_EYE
                rightPill = GrokBotGeometry.NOTIF_EYE
                leftTr = GrokBotGeometry.buildEyeTransform(-22.25f + 8f * pop, -11.4f - 6f * pop, 0f, 1f, 0.9f)
                rightTr = GrokBotGeometry.buildEyeTransform(34.3f + 5f * pop, -20.6f - 5f * pop, 0f, 1f, 0.78f)
                val rBadge = 15.2f * pop
                if (rBadge > 0.8f) {
                    hasNotifCutout = true
                    notifCutoutX = 74.32f
                    notifCutoutY = -67.07f
                    notifCutoutR = rBadge + 5.4f
                    hasNotifBadge = true
                    notifBadgeX = 74.32f
                    notifBadgeY = -67.07f
                    notifBadgeR = rBadge
                }
            }
            "egg" -> {
                val ph = if (useStaticPose) 0.45f else (cycleSec % 2.6f) / 2.6f
                val wobble = sin(ph * PI.toFloat() * 4f) * 7.5f * sin(ph * PI.toFloat())
                val m = if (ph < 0.2f) easeInOutSine(ph / 0.2f) else if (ph < 0.85f) 1f else easeInOutSine((1f - ph) / 0.15f)
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.EGG_COORDS, m, workCoords, 1f, wobble)
                leftPill = GrokBotGeometry.EGG_EYE
                rightPill = GrokBotGeometry.EGG_EYE
                leftTr = GrokBotGeometry.buildEyeTransform(-23.75f + wobble * 0.4f + gx * 8f, -15f + gy * 6f, wobble)
                rightTr = GrokBotGeometry.buildEyeTransform(21.15f + wobble * 0.4f + gx * 8f, -16.8f + gy * 6f, wobble)
            }
            "hexagon" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 3.0f) / 3.0f
                val m = if (ph < 0.22f) easeInOutSine(ph / 0.22f) else if (ph < 0.82f) 1f else easeInOutSine((1f - ph) / 0.18f)
                val rot = easeInOutSine(ph) * 60f
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.SHAPE_COORDS.getValue("hexagon"), m, workCoords, 1f, rot)
                leftPill = GrokBotGeometry.HEX_EYE
                rightPill = GrokBotGeometry.HEX_EYE
                leftTr = GrokBotGeometry.buildEyeTransform(-25.2f + gx * 10f, -14.2f + gy * 8f, -4f)
                rightTr = GrokBotGeometry.buildEyeTransform(25.2f + gx * 10f, -14.2f + gy * 8f, 4f)
            }
            "play" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 2.6f) / 2.6f
                val m = if (ph < 0.22f) easeInOutSine(ph / 0.22f) else if (ph < 0.82f) 1f else easeInOutSine((1f - ph) / 0.18f)
                val pulse = 1f + sin(ph * PI.toFloat() * 2f) * 0.03f * m
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.PLAY_COORDS, m, workCoords, pulse)
                leftPill = GrokBotGeometry.PLAY_EYE
                rightPill = GrokBotGeometry.PLAY_EYE
                leftTr = GrokBotGeometry.buildEyeTransform(-21.3f + gx * 8f, -9f + gy * 6f, 0f)
                rightTr = GrokBotGeometry.buildEyeTransform(18.7f + gx * 8f, -9f + gy * 6f, 0f)
            }
            else -> {
                GrokBotGeometry.lerpCoords(shapeBlendCoords, shapeBlendCoords, 0f, workCoords)
                val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                leftTr = trs.first
                rightTr = trs.second
            }
        }

        fillBezierPath(workCoords, bodyAndroidPath, scale, cx, cy)
        drawPath(bodyComposePath, brush = brush)

        // Paint eye pills and notification cutout cleanly atop the gradient body without CPU CSG Path.op.
        if (leftTr != null && rightTr != null) {
            fillEyeAndroidPath(
                leftPill, leftTr, scale, cx, cy,
                leftEyeAndroidPath, scratchRect, scratchMatrixValues, scratchMatrix,
            )
            fillEyeAndroidPath(
                rightPill, rightTr, scale, cx, cy,
                rightEyeAndroidPath, scratchRect, scratchMatrixValues, scratchMatrix,
            )
            drawPath(leftEyeComposePath, color = eyeBackingColor)
            drawPath(rightEyeComposePath, color = eyeBackingColor)
        }
        if (hasNotifCutout) {
            drawCircle(
                color = eyeBackingColor,
                radius = notifCutoutR * scale,
                center = Offset(cx + notifCutoutX * scale, cy + notifCutoutY * scale),
            )
        }
        if (hasNotifBadge) {
            drawCircle(
                color = Color(0xFF2496E8),
                radius = notifBadgeR * scale,
                center = Offset(cx + notifBadgeX * scale, cy + notifBadgeY * scale),
            )
        }
    }
}
