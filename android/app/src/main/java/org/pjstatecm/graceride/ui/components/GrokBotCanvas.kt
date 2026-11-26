package org.pjstatecm.graceride.ui.components


import android.graphics.Bitmap
import android.graphics.Canvas as AndroidCanvas
import android.graphics.Matrix as AndroidMatrix
import android.graphics.Paint as AndroidPaint
import android.graphics.Path as AndroidPath
import android.graphics.RadialGradient as AndroidRadialGradient
import android.graphics.RectF
import android.graphics.Shader
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.asComposePath
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.ui.theme.Spacing
import java.util.Locale
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.hypot
import kotlin.math.pow
import kotlin.math.sign
import kotlin.math.sin
import kotlin.math.tan
import kotlin.random.Random

internal val SWIRL_COLORS = listOf(
    Color(0xFF69D395),
    Color(0xFF69A3D3),
    Color(0xFF9C69D3),
    Color(0xFFD3699C),
)
internal val SWIRL_RADII = floatArrayOf(38f, 56f, 74f, 92f)
internal val SLEEP_Z_X = floatArrayOf(-4f, 18f, -8f)
internal val SLEEP_Z_Y = floatArrayOf(-6f, -38f, -72f)
internal val SLEEP_Z_W = floatArrayOf(16f, 22f, 28f)
internal val BURST_COS = FloatArray(8) { cos(it * PI / 4.0).toFloat() }
internal val BURST_SIN = FloatArray(8) { sin(it * PI / 4.0).toFloat() }

internal class MoveStrokeCache(
    var scale: Float = -1f,
    var swirlStroke: Stroke? = null,
    var cometStroke: Stroke? = null,
) {
    fun swirlFor(scale: Float): Stroke {
        sync(scale)
        return swirlStroke!!
    }

    fun cometFor(scale: Float): Stroke {
        sync(scale)
        return cometStroke!!
    }

    private fun sync(s: Float) {
        if (scale != s || swirlStroke == null || cometStroke == null) {
            scale = s
            swirlStroke = Stroke(width = 12f * s, cap = StrokeCap.Round)
            cometStroke = Stroke(width = 13f * s, cap = StrokeCap.Round)
        }
    }
}

internal class RadialBrushCache(
    var brush: Brush? = null,
    var colorKey: String = "",
    var alpha: Float = -1f,
    var cx: Float = Float.NaN,
    var cy: Float = Float.NaN,
    var radius: Float = -1f,
)

private fun easeInOutSine(x: Float): Float = (-(cos(PI * x.coerceIn(0f, 1f)) - 1.0) / 2.0).toFloat()

@Composable
fun GrokBotCanvas(
    spec: GrokBotSpec,
    modifier: Modifier = Modifier,
    move: String = "idle",
    variant: String = "tile",
    gazeOffset: Offset = Offset.Zero,
    staticMovePreview: Boolean = false,
) {
    val isDark = MaterialTheme.colorScheme.background.luminance() < 0.5f
    val animOn = animatorsEnabled()
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

    val isAnimated = animOn && !(variant == "tile" && (move == "idle" || staticMovePreview))
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
    val shapeBlendCoords = remember { FloatArray(386) }
    val bodyAndroidPath = remember { AndroidPath() }
    val bodyComposePath = remember { bodyAndroidPath.asComposePath() }
    val leftEyeAndroidPath = remember { AndroidPath() }
    val rightEyeAndroidPath = remember { AndroidPath() }
    val leftEyeComposePath = remember { leftEyeAndroidPath.asComposePath() }
    val rightEyeComposePath = remember { rightEyeAndroidPath.asComposePath() }
    val scratchRect = remember { RectF() }
    val scratchMatrixValues = remember { FloatArray(9) }
    val scratchMatrix = remember { AndroidMatrix() }
    val brushCache = remember { RadialBrushCache() }
    val moveStrokeCache = remember { MoveStrokeCache() }

    val palette = GrokBotGeometry.COLOR_PALETTES[spec.color] ?: GrokBotGeometry.COLOR_PALETTES.getValue("blue")
    val faceDef = GrokBotGeometry.FACE_DEFS[spec.face] ?: GrokBotGeometry.FACE_DEFS.getValue("neutral")

    val eyeBackingColor = when (spec.color) {
        "cream" -> Color(0xFF22211E)
        "ink" -> Color(0xFFF2F2F5)
        else -> if (isDark) Color(0xFF131519) else Color(0xFFFAF9F6)
    }
    val accentStrokeColor = if (spec.color == "ink" && isDark) Color(0xFFE5E5EA) else Color(palette.stopColors[2])

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

        val fromCoords = GrokBotGeometry.SHAPE_COORDS[prevShape] ?: GrokBotGeometry.SHAPE_COORDS.getValue("circle")
        val toCoords = GrokBotGeometry.SHAPE_COORDS[targetShape] ?: GrokBotGeometry.SHAPE_COORDS.getValue("circle")
        GrokBotGeometry.lerpCoords(fromCoords, toCoords, if (prevShape == targetShape) 1f else shapeMorphT, shapeBlendCoords)

        var bodyAlpha = 1f
        var showEyes = true
        var leftPill: GrokEyePill? = faceDef.left
        var rightPill: GrokEyePill? = faceDef.right
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
        var hasDotCircle = false
        var dotCircleX = 0f
        var dotCircleY = 0f
        var dotCircleR = 0f

        when (move) {
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
            "thinking" -> {
                val ph = (cycleSec % 2.6f) / 2.6f
                val lookUp = 0.55f + 0.2f * sin(ph * PI.toFloat() * 2f)
                val tilt = -4f + sin(ph * PI.toFloat() * 2f) * 2.5f
                GrokBotGeometry.lerpCoords(shapeBlendCoords, shapeBlendCoords, 0f, workCoords, 0.96f, tilt, -4f, 4f)
                val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx * 0.4f + 0.48f, gy * 0.4f - lookUp, 1f, spec.shape)
                leftTr = trs.first
                rightTr = trs.second
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
            "alert" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 2.2f) / 2.2f
                val m = if (ph < 0.25f) easeInOutSine(ph / 0.25f) else if (ph < 0.78f) 1f else easeInOutSine((1f - ph) / 0.22f)
                val shake = if (m > 0.7f && !useStaticPose) sin(ph * PI.toFloat() * 16f) * 3.5f else 0f
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.ALERT_STEM_COORDS, m, workCoords, 1f, shake)
                showEyes = m < 0.38f
                if (showEyes) {
                    val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                    leftTr = trs.first
                    rightTr = trs.second
                }
                if (m > 0.25f) {
                    hasDotCircle = true
                    dotCircleX = -15.77f
                    dotCircleY = 52.87f
                    dotCircleR = 17.5f * ((m - 0.25f) / 0.75f).coerceAtMost(1f)
                }
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
            "exclamation" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 2.4f) / 2.4f
                val m = if (ph < 0.25f) easeInOutSine(ph / 0.25f) else if (ph < 0.78f) 1f else easeInOutSine((1f - ph) / 0.22f)
                val bounceY = if (m > 0.6f && !useStaticPose) -abs(sin(ph * PI.toFloat() * 4f)) * 5f else 0f
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.EXCLAMATION_STEM_COORDS, m, workCoords, 1f, 0f, 0f, bounceY)
                showEyes = m < 0.38f
                if (showEyes) {
                    val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                    leftTr = trs.first
                    rightTr = trs.second
                }
                if (m > 0.25f) {
                    hasDotCircle = true
                    dotCircleX = 0f
                    dotCircleY = 52f + bounceY
                    dotCircleR = 16.5f * ((m - 0.25f) / 0.75f).coerceAtMost(1f)
                }
            }
            "sleep" -> {
                val ph = if (useStaticPose) 0.55f else (cycleSec % 3.6f) / 3.6f
                val m = if (ph < 0.22f) easeInOutSine(ph / 0.22f) else if (ph < 0.82f) 1f else easeInOutSine((1f - ph) / 0.18f)
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.SLEEP_CORE_COORDS, m, workCoords)
                showEyes = m < 0.35f
                if (showEyes) {
                    val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                    leftTr = trs.first
                    rightTr = trs.second
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
            "orbit" -> {
                val ph = if (useStaticPose) 0.35f else (cycleSec % 3.2f) / 3.2f
                GrokBotGeometry.lerpCoords(shapeBlendCoords, shapeBlendCoords, 0f, workCoords, 0.62f, sin(ph * PI.toFloat() * 2f) * 4f)
                val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, cos(ph * PI.toFloat() * 2f) * 0.55f, sin(ph * PI.toFloat() * 2f) * 0.3f, 1f, spec.shape)
                leftTr = trs.first.copy(a = trs.first.a * 0.62f, b = trs.first.b * 0.62f, c = trs.first.c * 0.62f, d = trs.first.d * 0.62f, tx = trs.first.tx * 0.62f, ty = trs.first.ty * 0.62f)
                rightTr = trs.second.copy(a = trs.second.a * 0.62f, b = trs.second.b * 0.62f, c = trs.second.c * 0.62f, d = trs.second.d * 0.62f, tx = trs.second.tx * 0.62f, ty = trs.second.ty * 0.62f)
            }
            "swirl" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 3.0f) / 3.0f
                val m = if (ph < 0.2f) easeInOutSine(ph / 0.2f) else if (ph < 0.84f) 1f else easeInOutSine((1f - ph) / 0.16f)
                bodyAlpha = (1f - m * 1.25f).coerceIn(0f, 1f)
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.BURST_CORE_COORDS, m, workCoords, 1f, ph * 180f)
                showEyes = bodyAlpha > 0.35f
                if (showEyes) {
                    val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                    leftTr = trs.first
                    rightTr = trs.second
                }
            }
            "burst" -> {
                val ph = if (useStaticPose) 0.45f else (cycleSec % 2.4f) / 2.4f
                val m = if (ph < 0.2f) easeInOutSine(ph / 0.2f) else if (ph < 0.82f) 1f else easeInOutSine((1f - ph) / 0.18f)
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.BURST_CORE_COORDS, m, workCoords)
                showEyes = m < 0.35f
                if (showEyes) {
                    val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                    leftTr = trs.first
                    rightTr = trs.second
                }
            }
            "comet" -> {
                val ph = if (useStaticPose) 0.32f else (cycleSec % 2.4f) / 2.4f
                val m = if (ph < 0.18f) easeInOutSine(ph / 0.18f) else if (ph < 0.85f) 1f else easeInOutSine((1f - ph) / 0.15f)
                val ang = ph * PI.toFloat() * 2f - PI.toFloat() / 2f
                val headR = 72f * m
                GrokBotGeometry.lerpCoords(shapeBlendCoords, GrokBotGeometry.COMET_HEAD_COORDS, m, workCoords, 1f, 0f, cos(ang) * headR, sin(ang) * headR)
                showEyes = m < 0.3f
                if (showEyes) {
                    val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                    leftTr = trs.first
                    rightTr = trs.second
                }
            }
            else -> {
                GrokBotGeometry.lerpCoords(shapeBlendCoords, shapeBlendCoords, 0f, workCoords)
                val trs = GrokBotGeometry.computeFaceEyeTransforms(spec.face, gx, gy, 1f, spec.shape)
                leftTr = trs.first
                rightTr = trs.second
            }
        }

        // Back layer (orbit back half)
        if (move == "orbit") {
            val ph = if (useStaticPose) 0.35f else (cycleSec % 3.2f) / 3.2f
            val ang = ph * PI.toFloat() * 2f
            val satX = cos(ang) * 102f
            val satY = sin(ang) * 34f
            val finalX = cx + (satX * 0.906f - satY * -0.422f) * scale
            val finalY = cy + (satX * -0.422f + satY * 0.906f) * scale
            if (sin(ang) < 0f) {
                drawCircle(accentStrokeColor, radius = 9.5f * scale, center = Offset(finalX, finalY))
            }
        }

        if (bodyAlpha > 0.01f) {
            fillBezierPath(workCoords, bodyAndroidPath, scale, cx, cy)

            val gradScale = if (variant == "stage") 1.264f else 1f
            val brushCx = cx - 40f * gradScale * scale
            val brushCy = cy - 50f * gradScale * scale
            val brushRadius = (193.75f * gradScale * scale).coerceAtLeast(1f)
            val brush = brushCache.brush?.takeIf {
                brushCache.colorKey == spec.color &&
                    brushCache.alpha == bodyAlpha &&
                    brushCache.cx == brushCx &&
                    brushCache.cy == brushCy &&
                    brushCache.radius == brushRadius
            } ?: Brush.radialGradient(
                0f to Color(palette.stopColors[0]).copy(alpha = bodyAlpha),
                0.32f to Color(palette.stopColors[1]).copy(alpha = bodyAlpha),
                0.72f to Color(palette.stopColors[2]).copy(alpha = bodyAlpha),
                1f to Color(palette.stopColors[3]).copy(alpha = bodyAlpha),
                center = Offset(brushCx, brushCy),
                radius = brushRadius,
            ).also { created ->
                brushCache.brush = created
                brushCache.colorKey = spec.color
                brushCache.alpha = bodyAlpha
                brushCache.cx = brushCx
                brushCache.cy = brushCy
                brushCache.radius = brushRadius
            }
            drawPath(bodyComposePath, brush = brush)

            if (hasDotCircle) {
                drawCircle(
                    brush = brush,
                    radius = dotCircleR * scale,
                    center = Offset(cx + dotCircleX * scale, cy + dotCircleY * scale),
                )
            }

            // Paint eye pills and notification cutout cleanly atop the gradient body without CPU CSG Path.op.
            val overlayColor = if (bodyAlpha >= 0.999f) eyeBackingColor else eyeBackingColor.copy(alpha = bodyAlpha)
            if (showEyes && leftPill != null && rightPill != null && leftTr != null && rightTr != null) {
                fillEyeAndroidPath(
                    leftPill, leftTr, scale, cx, cy,
                    leftEyeAndroidPath, scratchRect, scratchMatrixValues, scratchMatrix,
                )
                fillEyeAndroidPath(
                    rightPill, rightTr, scale, cx, cy,
                    rightEyeAndroidPath, scratchRect, scratchMatrixValues, scratchMatrix,
                )
                drawPath(leftEyeComposePath, color = overlayColor)
                drawPath(rightEyeComposePath, color = overlayColor)
            }
            if (hasNotifCutout) {
                drawCircle(
                    color = overlayColor,
                    radius = notifCutoutR * scale,
                    center = Offset(cx + notifCutoutX * scale, cy + notifCutoutY * scale),
                )
            }
        }

        if (hasNotifBadge) {
            drawCircle(
                color = Color(0xFF2496E8),
                radius = notifBadgeR * scale,
                center = Offset(cx + notifBadgeX * scale, cy + notifBadgeY * scale),
            )
        }

        // Foreground effects per move
        when (move) {
            "thinking" -> {
                drawCircle(accentStrokeColor.copy(alpha = 0.72f), 5.5f * scale, Offset(cx + 68f * scale, cy - 58f * scale))
                drawCircle(accentStrokeColor.copy(alpha = 0.88f), 9.5f * scale, Offset(cx + 86f * scale, cy - 78f * scale))
                drawCircle(accentStrokeColor, 17f * scale, Offset(cx + 106f * scale, cy - 102f * scale))
                drawCircle(eyeBackingColor, 2.4f * scale, Offset(cx + 99f * scale, cy - 102f * scale))
                drawCircle(eyeBackingColor, 2.4f * scale, Offset(cx + 106f * scale, cy - 102f * scale))
                drawCircle(eyeBackingColor, 2.4f * scale, Offset(cx + 113f * scale, cy - 102f * scale))
            }
            "sleep" -> {
                val ph = if (useStaticPose) 0.55f else (cycleSec % 3.6f) / 3.6f
                for (idx in 0 until 3) {
                    val zx = SLEEP_Z_X[idx]
                    val zy = SLEEP_Z_Y[idx]
                    val zw = SLEEP_Z_W[idx]
                    val drift = if (useStaticPose) 0f else ((ph + idx * 0.25f) % 1f) * -12f
                    val x0 = cx + (zx - zw * 0.4f) * scale
                    val x1 = cx + (zx + zw * 0.4f) * scale
                    val y0 = cy + (zy + drift - zw * 0.35f) * scale
                    val y1 = cy + (zy + drift + zw * 0.35f) * scale
                    val sw = (5.5f + idx * 1.2f) * scale
                    drawLine(accentStrokeColor, Offset(x0, y0), Offset(x1, y0), strokeWidth = sw, cap = StrokeCap.Round)
                    drawLine(accentStrokeColor, Offset(x1, y0), Offset(x0, y1), strokeWidth = sw, cap = StrokeCap.Round)
                    drawLine(accentStrokeColor, Offset(x0, y1), Offset(x1, y1), strokeWidth = sw, cap = StrokeCap.Round)
                }
            }
            "orbit" -> {
                val ph = if (useStaticPose) 0.35f else (cycleSec % 3.2f) / 3.2f
                val ang = ph * PI.toFloat() * 2f
                val satX = cos(ang) * 102f
                val satY = sin(ang) * 34f
                val finalX = cx + (satX * 0.906f - satY * -0.422f) * scale
                val finalY = cy + (satX * -0.422f + satY * 0.906f) * scale
                if (sin(ang) >= 0f) {
                    drawCircle(accentStrokeColor, radius = 11.5f * scale, center = Offset(finalX, finalY))
                }
            }
            "swirl" -> {
                val ph = if (useStaticPose) 0.5f else (cycleSec % 3.0f) / 3.0f
                val spin = ph * 360f
                val swirlStroke = moveStrokeCache.swirlFor(scale)
                for (idx in SWIRL_RADII.indices) {
                    val r = SWIRL_RADII[idx]
                    val d = r * 2f * scale
                    drawArc(
                        color = SWIRL_COLORS[idx],
                        startAngle = spin * (if (idx % 2 == 0) 1f else -1.2f) + idx * 55f,
                        sweepAngle = 215f,
                        useCenter = false,
                        topLeft = Offset(cx - r * scale, cy - r * scale),
                        size = Size(d, d),
                        style = swirlStroke,
                    )
                }
            }
            "burst" -> {
                val ph = if (useStaticPose) 0.45f else (cycleSec % 2.4f) / 2.4f
                val burst = sin(ph * PI.toFloat())
                val r0 = (32f + 12f * burst) * scale
                val r1 = (58f + 34f * burst) * scale
                val strokeW = 11f * scale
                for (k in 0 until 8) {
                    val ca = BURST_COS[k]
                    val sa = BURST_SIN[k]
                    drawLine(
                        color = accentStrokeColor,
                        start = Offset(cx + ca * r0, cy + sa * r0),
                        end = Offset(cx + ca * r1, cy + sa * r1),
                        strokeWidth = strokeW,
                        cap = StrokeCap.Round,
                    )
                }
            }
            "comet" -> {
                val ph = if (useStaticPose) 0.32f else (cycleSec % 2.4f) / 2.4f
                val headDeg = ph * 360f - 90f
                val r = 72f * scale
                drawArc(
                    color = accentStrokeColor.copy(alpha = 0.55f),
                    startAngle = headDeg - 135f,
                    sweepAngle = 128f,
                    useCenter = false,
                    topLeft = Offset(cx - r, cy - r),
                    size = Size(r * 2f, r * 2f),
                    style = moveStrokeCache.cometFor(scale),
                )
            }
        }
    }
}
