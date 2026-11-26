package org.pjstatecm.graceride.ui.components

import android.animation.ValueAnimator
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.ui.theme.BrandRed
import org.pjstatecm.graceride.ui.theme.GrPrimary
import org.pjstatecm.graceride.ui.theme.Spacing
import kotlin.math.min

private val BrandOutEasing = CubicBezierEasing(0.2f, 0.8f, 0.2f, 1f)
private val BrandSweepEasing = CubicBezierEasing(0.4f, 0f, 0.2f, 1f)
private val BrandExitEasing = CubicBezierEasing(0.22f, 1f, 0.36f, 1f)

/** Animation mode mirroring the web Grace Ride brand mark. */
enum class BrandMarkMode {
    INTRO,
    LOADING,
    IDLE_HOP,
    STATIC,
}

internal fun animatorsEnabled(): Boolean =
    runCatching { ValueAnimator.areAnimatorsEnabled() }.getOrDefault(true)

private class BrandMarkDrawCache(
    val crossPath: Path = Path(),
    var width: Float = -1f,
    var height: Float = -1f,
    var s: Float = 0f,
    var ox: Float = 0f,
    var oy: Float = 0f,
    var arcStroke: Stroke = Stroke(width = 1f, cap = StrokeCap.Round),
    var groundStroke: Stroke = Stroke(width = 1f),
    var rippleStroke: Stroke = Stroke(width = 1f),
) {
    fun sync(w: Float, h: Float) {
        if (width == w && height == h) return
        width = w
        height = h
        val scale = min(w, h) / 240f
        val offsetX = (w - 240f * scale) / 2f
        val offsetY = (h - 240f * scale) / 2f - 4f * scale
        s = scale
        ox = offsetX
        oy = offsetY
        arcStroke = Stroke(width = 5f * scale, cap = StrokeCap.Round)
        groundStroke = Stroke(width = 2.5f * scale)
        rippleStroke = Stroke(width = 1.8f * scale)

        crossPath.rewind()
        crossPath.moveTo(113f * scale + offsetX, 30f * scale + offsetY)
        crossPath.lineTo(127f * scale + offsetX, 30f * scale + offsetY)
        crossPath.lineTo(127f * scale + offsetX, 52f * scale + offsetY)
        crossPath.lineTo(159f * scale + offsetX, 52f * scale + offsetY)
        crossPath.lineTo(159f * scale + offsetX, 66f * scale + offsetY)
        crossPath.lineTo(127f * scale + offsetX, 66f * scale + offsetY)
        crossPath.lineTo(127f * scale + offsetX, 158f * scale + offsetY)
        crossPath.lineTo(120f * scale + offsetX, 212f * scale + offsetY)
        crossPath.lineTo(113f * scale + offsetX, 158f * scale + offsetY)
        crossPath.lineTo(113f * scale + offsetX, 66f * scale + offsetY)
        crossPath.lineTo(81f * scale + offsetX, 66f * scale + offsetY)
        crossPath.lineTo(81f * scale + offsetX, 52f * scale + offsetY)
        crossPath.lineTo(113f * scale + offsetX, 52f * scale + offsetY)
        crossPath.close()
    }
}

/**
 * Choreographed native vector animation of the official Grace Ride brand mark
 * (`docs/brand/grace-ride-logo-kit/animation/grace-ride-loading-mark.svg`):
 * 1. Red cross pin lands smoothly from above
 * 2. Blue ground ring opens horizontally
 * 3. Endpoint dots pop and both sanctuary arcs sweep inward together
 * 4. Optional breathing ground ripples pulse while loading
 */
@Composable
fun GraceRideBrandMark(
    modifier: Modifier = Modifier,
    animated: Boolean = true,
    showRipples: Boolean = false,
    darkTheme: Boolean = MaterialTheme.colorScheme.background.luminance() < 0.5f,
    mode: BrandMarkMode = when {
        !animated -> BrandMarkMode.STATIC
        showRipples -> BrandMarkMode.LOADING
        else -> BrandMarkMode.INTRO
    },
) {
    val shouldAnimate = animated && mode != BrandMarkMode.STATIC && animatorsEnabled()
    val crossProgress = remember(shouldAnimate, mode) { Animatable(if (shouldAnimate) 0f else 1f) }
    val groundProgress = remember(shouldAnimate, mode) { Animatable(if (shouldAnimate) 0f else 1f) }
    val dotProgress = remember(shouldAnimate, mode) { Animatable(if (shouldAnimate) 0f else 1f) }
    val arcProgress = remember(shouldAnimate, mode) { Animatable(if (shouldAnimate) 0f else 1f) }
    val wantContinuousRipples = mode == BrandMarkMode.LOADING || (showRipples && mode != BrandMarkMode.IDLE_HOP)
    val wantIdleHop = mode == BrandMarkMode.IDLE_HOP
    var ripplesStarted by remember(shouldAnimate, mode, showRipples) { mutableStateOf(false) }
    var idleStarted by remember(shouldAnimate, mode) { mutableStateOf(false) }

    LaunchedEffect(shouldAnimate, mode, wantContinuousRipples, wantIdleHop) {
        if (!shouldAnimate) {
            crossProgress.snapTo(1f)
            groundProgress.snapTo(1f)
            dotProgress.snapTo(1f)
            arcProgress.snapTo(1f)
            ripplesStarted = false
            idleStarted = false
            return@LaunchedEffect
        }
        crossProgress.snapTo(0f)
        groundProgress.snapTo(0f)
        dotProgress.snapTo(0f)
        arcProgress.snapTo(0f)
        ripplesStarted = false
        idleStarted = false

        launch {
            crossProgress.animateTo(1f, tween(durationMillis = 560, easing = BrandOutEasing))
        }
        launch {
            delay(260)
            groundProgress.animateTo(1f, tween(durationMillis = 420, easing = BrandOutEasing))
        }
        launch {
            delay(340)
            dotProgress.animateTo(1f, tween(durationMillis = 240, easing = BrandOutEasing))
        }
        launch {
            delay(360)
            arcProgress.animateTo(1f, tween(durationMillis = 520, easing = BrandSweepEasing))
        }
        if (wantContinuousRipples) {
            delay(850)
            ripplesStarted = true
        } else if (wantIdleHop) {
            delay(1200)
            idleStarted = true
        }
    }

    val ripplePhaseState: State<Float>? = if (wantContinuousRipples && shouldAnimate && ripplesStarted) {
        val infinite = rememberInfiniteTransition(label = "brand_ripples")
        infinite.animateFloat(
            initialValue = 0f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(
                animation = tween(durationMillis = 2400, easing = LinearEasing),
                repeatMode = RepeatMode.Restart,
            ),
            label = "ripple_phase",
        )
    } else {
        null
    }

    val idlePhaseState: State<Float>? = if (wantIdleHop && shouldAnimate && idleStarted) {
        val infinite = rememberInfiniteTransition(label = "brand_idle_hop")
        infinite.animateFloat(
            initialValue = 0f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(
                animation = tween(durationMillis = 6000, easing = LinearEasing),
                repeatMode = RepeatMode.Restart,
            ),
            label = "idle_hop_phase",
        )
    } else {
        null
    }

    val crossColor = if (darkTheme) Color.White else BrandRed
    val accentColor = if (darkTheme) Color.White else GrPrimary
    val drawCache = remember { BrandMarkDrawCache() }

    Canvas(modifier = modifier) {
        if (size.width <= 0f || size.height <= 0f) return@Canvas
        drawCache.sync(size.width, size.height)
        val s = drawCache.s
        val ox = drawCache.ox
        val oy = drawCache.oy

        // 1. Symmetrical sweeping arcs centered at (120, 171) with radius 81
        val arcP = arcProgress.value
        if (arcP > 0.01f) {
            val arcTopLeft = Offset((120f - 81f) * s + ox, (171f - 81f) * s + oy)
            val arcSize = Size(162f * s, 162f * s)
            drawArc(
                color = accentColor,
                startAngle = 195f,
                sweepAngle = 65.77f * arcP,
                useCenter = false,
                topLeft = arcTopLeft,
                size = arcSize,
                style = drawCache.arcStroke,
            )
            drawArc(
                color = accentColor,
                startAngle = 345f,
                sweepAngle = -65.77f * arcP,
                useCenter = false,
                topLeft = arcTopLeft,
                size = arcSize,
                style = drawCache.arcStroke,
            )
        }

        // 2. Arc endpoint dots at (41.76, 150.04) and (198.24, 150.04)
        val dotP = dotProgress.value
        if (dotP > 0.01f) {
            val r = 2.5f * dotP * s
            val dotColor = accentColor.copy(alpha = dotP.coerceIn(0f, 1f))
            drawCircle(
                color = dotColor,
                radius = r,
                center = Offset(41.76f * s + ox, 150.04f * s + oy),
            )
            drawCircle(
                color = dotColor,
                radius = r,
                center = Offset(198.24f * s + ox, 150.04f * s + oy),
            )
        }

        // 3. Staggered breathing ground ripples at (120, 213)
        val ripplePhase = ripplePhaseState?.value
        if (ripplePhase != null) {
            for (i in 0..1) {
                val phase = if (i == 0) ripplePhase else (ripplePhase + 0.5f) % 1f
                if (phase <= 0.65f) {
                    val t = (phase / 0.65f).coerceIn(0f, 1f)
                    val eased = FastOutSlowInEasing.transform(t)
                    val rippleScale = 0.65f + (2.55f - 0.65f) * eased
                    val rippleAlpha = if (t < 0.18f) {
                        (t / 0.18f) * 0.50f
                    } else {
                        ((1f - t) / 0.82f) * 0.50f
                    }.coerceIn(0f, 1f)
                    val rx = 20f * rippleScale * s
                    val ry = 5.5f * rippleScale * s
                    drawOval(
                        color = accentColor.copy(alpha = rippleAlpha),
                        topLeft = Offset(120f * s + ox - rx, 213f * s + oy - ry),
                        size = Size(rx * 2f, ry * 2f),
                        style = drawCache.rippleStroke,
                    )
                }
            }
        }

        // 3b. Gentle 6s idle landing ripple when header hops
        val idlePhase = idlePhaseState?.value
        if (idlePhase != null && idlePhase in 0.09f..0.35f) {
            val t = ((idlePhase - 0.09f) / 0.26f).coerceIn(0f, 1f)
            val eased = FastOutSlowInEasing.transform(t)
            val rippleScale = 0.6f + (2.4f - 0.6f) * eased
            val rippleAlpha = if (t < 0.15f) (t / 0.15f) * 0.50f else ((1f - t) / 0.85f) * 0.50f
            val rx = 20f * rippleScale * s
            val ry = 5.5f * rippleScale * s
            drawOval(
                color = accentColor.copy(alpha = rippleAlpha.coerceIn(0f, 1f)),
                topLeft = Offset(120f * s + ox - rx, 213f * s + oy - ry),
                size = Size(rx * 2f, ry * 2f),
                style = drawCache.rippleStroke,
            )
        }

        // 4. Ground ring at (120, 213)
        val groundP = groundProgress.value
        if (groundP > 0.01f) {
            val rx = 20f * groundP * s
            val ry = 5.5f * s
            drawOval(
                color = accentColor.copy(alpha = groundP.coerceIn(0f, 1f)),
                topLeft = Offset(120f * s + ox - rx, 213f * s + oy - ry),
                size = Size(rx * 2f, ry * 2f),
                style = drawCache.groundStroke,
            )
        }

        // 5. Cross pin: M113 30 H127 V52 H159 V66 H127 V158 L120 212 L113 158 V66 H81 V52 H113 Z
        val crossP = crossProgress.value
        if (crossP > 0.01f) {
            val hopOffsetY = if (idlePhase != null && idlePhase < 0.14f) {
                val h = when {
                    idlePhase < 0.05f -> FastOutSlowInEasing.transform(idlePhase / 0.05f) * -14f
                    idlePhase < 0.10f -> (1f - FastOutSlowInEasing.transform((idlePhase - 0.05f) / 0.05f)) * -14f
                    idlePhase < 0.12f -> FastOutSlowInEasing.transform((idlePhase - 0.10f) / 0.02f) * -3f
                    else -> (1f - FastOutSlowInEasing.transform((idlePhase - 0.12f) / 0.02f)) * -3f
                }
                h * s
            } else {
                0f
            }
            val crossScale = 0.94f + 0.06f * crossP
            val pivot = Offset(120f * s + ox, 120f * s + oy)
            withTransform({
                translate(left = 0f, top = -22f * s * (1f - crossP) + hopOffsetY)
                scale(scaleX = crossScale, scaleY = crossScale, pivot = pivot)
            }) {
                drawPath(
                    path = drawCache.crossPath,
                    color = crossColor.copy(alpha = crossP.coerceIn(0f, 1f)),
                )
            }
        }
    }
}

/** Two-tone "Grace Ride" wordmark rendered as a single semantic Text node. */
@Composable
fun GraceRideWordmark(
    modifier: Modifier = Modifier,
    fontSize: TextUnit = 36.sp,
    darkTheme: Boolean = MaterialTheme.colorScheme.background.luminance() < 0.5f,
) {
    val graceColor = if (darkTheme) Color.White else BrandRed
    val rideColor = if (darkTheme) Color.White else GrPrimary
    Text(
        text = buildAnnotatedString {
            withStyle(SpanStyle(color = graceColor, fontWeight = FontWeight.Bold)) {
                append("Grace ")
            }
            withStyle(SpanStyle(color = rideColor, fontWeight = FontWeight.Light)) {
                append("Ride")
            }
        },
        fontSize = fontSize,
        lineHeight = fontSize * 1.1f,
        letterSpacing = (-0.5).sp,
        modifier = modifier,
    )
}

/**
 * Full-screen branded launch overlay mirroring `grace-ride-loading-screen.html`:
 * animated mark + staggered wordmark, tagline, and church lockup, fading out cleanly
 * with an upward slide once the initial auth session check resolves.
 */
@Composable
fun GraceRideLandingSplash(
    visible: Boolean,
    onFinished: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val shouldAnimate = animatorsEnabled()
    val currentOnFinished by rememberUpdatedState(onFinished)
    val darkTheme = MaterialTheme.colorScheme.background.luminance() < 0.5f
    val wordAlpha = remember(shouldAnimate) { Animatable(if (shouldAnimate) 0f else 1f) }
    val tagAlpha = remember(shouldAnimate) { Animatable(if (shouldAnimate) 0f else 1f) }
    val churchAlpha = remember(shouldAnimate) { Animatable(if (shouldAnimate) 0f else 1f) }
    val exitProgress = remember { Animatable(0f) }

    LaunchedEffect(shouldAnimate) {
        if (!shouldAnimate) return@LaunchedEffect
        launch {
            delay(480)
            wordAlpha.animateTo(1f, tween(durationMillis = 450, easing = BrandOutEasing))
        }
        launch {
            delay(660)
            tagAlpha.animateTo(1f, tween(durationMillis = 440, easing = BrandOutEasing))
        }
        launch {
            delay(800)
            churchAlpha.animateTo(1f, tween(durationMillis = 440, easing = BrandOutEasing))
        }
    }

    LaunchedEffect(visible, shouldAnimate) {
        if (visible) {
            exitProgress.snapTo(0f)
        } else {
            if (shouldAnimate) {
                exitProgress.animateTo(1f, tween(durationMillis = 280, easing = BrandExitEasing))
            } else {
                exitProgress.snapTo(1f)
            }
            currentOnFinished()
        }
    }

    val a11yLoading = stringResource(R.string.splash_loading_a11y)
    val mutedColor = if (darkTheme) Color(0xFFA3A3A3) else Color(0xFF5F636A)
    val churchBlue = if (darkTheme) Color.White else GrPrimary
    val churchInk = if (darkTheme) Color.White else Color(0xFF474A4F)

    Box(
        modifier = modifier
            .fillMaxSize()
            .graphicsLayer {
                val p = if (!visible && !shouldAnimate) 1f else exitProgress.value
                val scale = 1f - 0.03f * p
                alpha = (1f - p).coerceIn(0f, 1f)
                translationY = -28.dp.toPx() * p
                scaleX = scale
                scaleY = scale
            }
            .background(MaterialTheme.colorScheme.background)
            .pointerInput(Unit) {
                awaitPointerEventScope {
                    while (true) {
                        awaitPointerEvent().changes.forEach { it.consume() }
                    }
                }
            }
            .statusBarsPadding()
            .navigationBarsPadding()
            .padding(horizontal = Spacing.lg, vertical = Spacing.xl)
            .semantics { contentDescription = a11yLoading },
    ) {
        Spacer(Modifier.height(20.dp).align(Alignment.TopCenter))

        Column(
            modifier = Modifier.align(Alignment.Center),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            GraceRideBrandMark(
                modifier = Modifier.size(168.dp),
                animated = true,
                showRipples = true,
                darkTheme = darkTheme,
            )
            GraceRideWordmark(
                fontSize = 38.sp,
                darkTheme = darkTheme,
                modifier = Modifier.graphicsLayer {
                    alpha = wordAlpha.value
                    translationY = 18f * (1f - wordAlpha.value)
                },
            )
            Text(
                text = stringResource(R.string.splash_tagline),
                fontSize = 12.sp,
                fontWeight = FontWeight.Medium,
                letterSpacing = 2.5.sp,
                color = mutedColor,
                textAlign = TextAlign.Center,
                modifier = Modifier.graphicsLayer {
                    alpha = tagAlpha.value
                    translationY = 14f * (1f - tagAlpha.value)
                },
            )
        }

        Column(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .graphicsLayer {
                    alpha = churchAlpha.value
                    translationY = 14f * (1f - churchAlpha.value)
                },
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(
                text = buildAnnotatedString {
                    withStyle(SpanStyle(color = churchBlue, fontWeight = FontWeight.Medium)) {
                        append("卫理公会八达灵")
                    }
                    withStyle(SpanStyle(color = churchInk, fontWeight = FontWeight.Bold)) {
                        append("新市堂")
                    }
                },
                fontSize = 15.sp,
                letterSpacing = 3.sp,
            )
            Text(
                text = "PJ STATE",
                fontSize = 11.sp,
                fontWeight = FontWeight.Medium,
                letterSpacing = 3.sp,
                color = mutedColor,
            )
        }
    }
}
