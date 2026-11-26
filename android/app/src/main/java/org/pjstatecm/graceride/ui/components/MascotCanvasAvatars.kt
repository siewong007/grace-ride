package org.pjstatecm.graceride.ui.components


import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.GenericShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.DirectionsBike
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.outlined.Church
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Eco
import androidx.compose.material.icons.outlined.FavoriteBorder
import androidx.compose.material.icons.outlined.LocalCafe
import androidx.compose.material.icons.outlined.LocalFlorist
import androidx.compose.material.icons.outlined.MusicNote
import androidx.compose.material.icons.outlined.SentimentSatisfied
import androidx.compose.material.icons.outlined.StarOutline
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.request.ImageRequest

internal fun fillBloubShapePath(shape: String, sx: Float, sy: Float, outPath: Path) {
    outPath.rewind()
    with(outPath) {
        when (shape) {
            "circle" -> addOval(Rect(7.8f * sx, 7.8f * sy, 40.2f * sx, 40.2f * sy))
            "pebble" -> {
                moveTo(23.5f * sx, 8.2f * sy)
                cubicTo(33f * sx, 7.8f * sy, 40.2f * sx, 13.8f * sy, 40.4f * sx, 22.8f * sy)
                cubicTo(40.6f * sx, 32.2f * sy, 34.2f * sx, 39.6f * sy, 24.4f * sx, 39.6f * sy)
                cubicTo(14.2f * sx, 39.6f * sy, 7.8f * sx, 33.4f * sy, 7.8f * sx, 24.2f * sy)
                cubicTo(7.8f * sx, 14.8f * sy, 14.4f * sx, 8.6f * sy, 23.5f * sx, 8.2f * sy)
                close()
            }
            "squircle" -> {
                moveTo(18.5f * sx, 8f * sy)
                lineTo(29.5f * sx, 8f * sy)
                cubicTo(36.4f * sx, 8f * sy, 40f * sx, 11.6f * sy, 40f * sx, 18.5f * sy)
                lineTo(40f * sx, 29.5f * sy)
                cubicTo(40f * sx, 36.4f * sy, 36.4f * sx, 40f * sy, 29.5f * sx, 40f * sy)
                lineTo(18.5f * sx, 40f * sy)
                cubicTo(11.6f * sx, 40f * sy, 8f * sx, 36.4f * sy, 8f * sx, 29.5f * sy)
                lineTo(8f * sx, 18.5f * sy)
                cubicTo(8f * sx, 11.6f * sy, 11.6f * sx, 8f * sy, 18.5f * sx, 8f * sy)
                close()
            }
            "capsule" -> addRoundRect(
                RoundRect(
                    left = 6f * sx,
                    top = 11.5f * sy,
                    right = 42f * sx,
                    bottom = 36.5f * sy,
                    cornerRadius = CornerRadius(12.5f * sx, 12.5f * sy),
                ),
            )
            "triangle" -> {
                moveTo(19.8f * sx, 12.5f * sy)
                quadraticTo(24f * sx, 6.2f * sy, 28.2f * sx, 12.5f * sy)
                lineTo(39.2f * sx, 31.8f * sy)
                quadraticTo(42.4f * sx, 38.4f * sy, 35.2f * sx, 38.4f * sy)
                lineTo(12.8f * sx, 38.4f * sy)
                quadraticTo(5.6f * sx, 38.4f * sy, 8.8f * sx, 31.8f * sy)
                close()
            }
            "cloud" -> {
                moveTo(24f * sx, 10.6f * sy)
                cubicTo(26.8f * sx, 8.2f * sy, 32.8f * sx, 9.2f * sy, 34.4f * sx, 13.4f * sy)
                cubicTo(39.6f * sx, 15.2f * sy, 41f * sx, 21.8f * sy, 39.4f * sx, 27.6f * sy)
                cubicTo(38.2f * sx, 34.4f * sy, 33.4f * sx, 39f * sy, 24f * sx, 39f * sy)
                cubicTo(14.6f * sx, 39f * sy, 9.8f * sx, 34.4f * sy, 8.6f * sx, 27.6f * sy)
                cubicTo(7f * sx, 21.8f * sy, 8.4f * sx, 15.2f * sy, 13.6f * sx, 13.4f * sy)
                cubicTo(15.2f * sx, 9.2f * sy, 21.2f * sx, 8.2f * sy, 24f * sx, 10.6f * sy)
                close()
            }
            "droplet" -> {
                moveTo(24f * sx, 7.5f * sy)
                cubicTo(26.6f * sx, 7.5f * sy, 39.6f * sx, 18.8f * sy, 39.6f * sx, 26.8f * sy)
                cubicTo(39.6f * sx, 34.8f * sy, 32.8f * sx, 40f * sy, 24f * sx, 40f * sy)
                cubicTo(15.2f * sx, 40f * sy, 8.4f * sx, 34.8f * sy, 8.4f * sx, 26.8f * sy)
                cubicTo(8.4f * sx, 18.8f * sy, 21.4f * sx, 7.5f * sy, 24f * sx, 7.5f * sy)
                close()
            }
            else -> {
                moveTo(19.45f * sx, 8.85f * sy)
                lineTo(28.55f * sx, 8.85f * sy)
                quadraticTo(32.75f * sx, 8.85f * sy, 34.85f * sx, 12.49f * sy)
                lineTo(39.4f * sx, 20.36f * sy)
                quadraticTo(41.5f * sx, 24f * sy, 39.4f * sx, 27.64f * sy)
                lineTo(34.85f * sx, 35.51f * sy)
                quadraticTo(32.75f * sx, 39.15f * sy, 28.55f * sx, 39.15f * sy)
                lineTo(19.45f * sx, 39.15f * sy)
                quadraticTo(15.25f * sx, 39.15f * sy, 13.15f * sx, 35.51f * sy)
                lineTo(8.6f * sx, 27.64f * sy)
                quadraticTo(6.5f * sx, 24f * sy, 8.6f * sx, 20.36f * sy)
                lineTo(13.15f * sx, 12.49f * sy)
                quadraticTo(15.25f * sx, 8.85f * sy, 19.45f * sx, 8.85f * sy)
                close()
            }
        }
    }
}

@Composable
internal fun GrokBotShapeAvatar(
    modifier: Modifier = Modifier,
    avatarColor: String? = null,
    shape: String = "hexagon",
) {
    val animOn = animatorsEnabled()
    val transition = if (animOn) rememberInfiniteTransition(label = "grok_bot") else null
    val bodyTiltState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 6400
                0f at 0
                0f at 960
                6.5f at 1400
                6.5f at 2300
                -8.5f at 2800
                -7.5f at 3950
                -3f at 4500
                -3f at 5350
                0f at 5900
                0f at 6400
            },
        ),
        label = "grok_tilt",
    ) ?: remember { mutableFloatStateOf(0f) }
    val lookXState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 6400
                0f at 0
                0f at 960
                4.2f at 1350
                4.2f at 2350
                -2.2f at 2820
                -2.2f at 3950
                -3.8f at 4500
                -3.8f at 5350
                0f at 5900
                0f at 6400
            },
        ),
        label = "grok_look_x",
    ) ?: remember { mutableFloatStateOf(0f) }
    val lookYState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 6400
                0f at 0
                0f at 960
                -3.2f at 1350
                -3.2f at 2350
                1f at 2820
                1f at 3950
                3.2f at 4500
                3.2f at 5350
                0f at 5900
                0f at 6400
            },
        ),
        label = "grok_look_y",
    ) ?: remember { mutableFloatStateOf(0f) }
    val leftScaleYState: State<Float> = transition?.animateFloat(
        initialValue = 1f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 6400
                1f at 0
                1f at 200
                0.1f at 380
                1f at 560
                0.1f at 700
                1f at 880
                1f at 2560
                1.05f at 2820
                1.05f at 3950
                1f at 4250
                1f at 5900
                0.1f at 6080
                1f at 6260
                1f at 6400
            },
        ),
        label = "grok_eye_l",
    ) ?: remember { mutableFloatStateOf(1f) }
    val rightScaleYState: State<Float> = transition?.animateFloat(
        initialValue = 1f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 6400
                1f at 0
                1f at 200
                0.1f at 380
                1f at 560
                0.1f at 700
                1f at 880
                1f at 2560
                0.42f at 2820
                0.42f at 3950
                1f at 4250
                1f at 5900
                0.1f at 6080
                1f at 6260
                1f at 6400
            },
        ),
        label = "grok_eye_r",
    ) ?: remember { mutableFloatStateOf(1f) }
    val rightRotState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 6400
                0f at 0
                0f at 2560
                -28f at 2820
                -28f at 3950
                0f at 4250
                0f at 6400
            },
        ),
        label = "grok_wink_rot",
    ) ?: remember { mutableFloatStateOf(0f) }
    val skin = AvatarStyle.grokSkinColor(avatarColor)
    val ring = AvatarStyle.grokRingColor(avatarColor)
    val eyeColor = AvatarStyle.grokEyeColor(avatarColor)
    val drawCache = remember { AvatarDrawCache() }

    Canvas(modifier = modifier) {
        if (size.width <= 0f || size.height <= 0f) return@Canvas
        val sx = size.width / 48f
        val sy = size.height / 48f
        val unit = minOf(sx, sy)
        val bodyTilt = bodyTiltState.value
        val lookX = lookXState.value
        val lookY = lookYState.value
        val leftScaleY = leftScaleYState.value
        val rightScaleY = rightScaleYState.value
        val rightRot = rightRotState.value

        rotate(degrees = bodyTilt, pivot = Offset(24f * sx, 24f * sy)) {
            val body = drawCache.bodyFor(shape, sx, sy)
            drawPath(body, color = skin)
            if (ring != Color.Transparent) {
                drawPath(body, color = ring, style = drawCache.strokeFor(1.2f * unit))
            }

            val lw = 5.2f * sx
            val lh = (8.8f * sy * leftScaleY).coerceAtLeast(1.4f)
            val lCenter = Offset((19.2f + lookX) * sx, (24f + lookY) * sy)
            drawRoundRect(
                color = eyeColor,
                topLeft = Offset(lCenter.x - lw / 2f, lCenter.y - lh / 2f),
                size = Size(lw, lh),
                cornerRadius = CornerRadius(2.6f * sx, 2.6f * sy),
            )

            val rw = 5.2f * sx
            val rh = (8.8f * sy * rightScaleY).coerceAtLeast(1.4f)
            val rCenter = Offset((28.8f + lookX) * sx, (24f + lookY) * sy)
            rotate(degrees = rightRot, pivot = rCenter) {
                drawRoundRect(
                    color = eyeColor,
                    topLeft = Offset(rCenter.x - rw / 2f, rCenter.y - rh / 2f),
                    size = Size(rw, rh),
                    cornerRadius = CornerRadius(2.6f * sx, 2.6f * sy),
                )
            }
        }
    }
}

@Composable
internal fun MuseBotShapeAvatar(
    modifier: Modifier = Modifier,
    avatarColor: String? = null,
    shape: String = "cloud",
) {
    val animOn = animatorsEnabled()
    val transition = if (animOn) rememberInfiniteTransition(label = "muse_bot") else null
    val bodyTiltState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 5800
                0f at 0
                0f at 920
                -6f at 1300
                -6f at 2150
                7.5f at 2660
                7.5f at 3700
                0f at 4250
                0f at 5800
            },
        ),
        label = "muse_tilt",
    ) ?: remember { mutableFloatStateOf(0f) }
    val faceXState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 5800
                0f at 0
                0f at 920
                -3.4f at 1300
                -3.4f at 2150
                2.6f at 2660
                2.6f at 3700
                0f at 4250
                0f at 5800
            },
        ),
        label = "muse_face_x",
    ) ?: remember { mutableFloatStateOf(0f) }
    val faceYState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 5800
                0f at 0
                0f at 920
                2.0f at 1300
                2.0f at 2150
                -2.2f at 2660
                -2.2f at 3700
                0f at 4250
                0f at 5800
            },
        ),
        label = "muse_face_y",
    ) ?: remember { mutableFloatStateOf(0f) }
    val eyeScaleYState: State<Float> = transition?.animateFloat(
        initialValue = 1f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 5800
                1f at 0
                1f at 2200
                0.26f at 2660
                0.26f at 3700
                1f at 4250
                1f at 5800
            },
        ),
        label = "muse_eye_scale_y",
    ) ?: remember { mutableFloatStateOf(1f) }
    val happyTiltState: State<Float> = transition?.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 5800
                0f at 0
                0f at 2200
                14f at 2660
                14f at 3700
                0f at 4250
                0f at 5800
            },
        ),
        label = "muse_happy_tilt",
    ) ?: remember { mutableFloatStateOf(0f) }
    val sparkAlphaState: State<Float> = transition?.animateFloat(
        initialValue = 0.35f,
        targetValue = 0.35f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 5800
                0.35f at 0
                0.35f at 2200
                1f at 2660
                1f at 3700
                0.35f at 4200
                0.35f at 5800
            },
        ),
        label = "muse_spark_alpha",
    ) ?: remember { mutableFloatStateOf(0.35f) }
    val colorKey = avatarColor?.substringBefore(':')
    val (plush, plushShade) = AvatarStyle.musePlushColors(colorKey)
    val sparkColor = if (colorKey == "white") Color(0xFFF59E0B) else AvatarStyle.grokSkinColor(colorKey)
    val outline = if (colorKey == "white") Color(0xFFCBD5E1) else plushShade
    val inkColor = Color(0xFF18181B)
    val cheekColor = Color(0xFFF48FB1).copy(alpha = 0.88f)
    val drawCache = remember { AvatarDrawCache() }

    Canvas(modifier = modifier) {
        if (size.width <= 0f || size.height <= 0f) return@Canvas
        val sx = size.width / 48f
        val sy = size.height / 48f
        val unit = minOf(sx, sy)
        val bodyTilt = bodyTiltState.value
        val faceX = faceXState.value
        val faceY = faceYState.value
        val eyeScaleY = eyeScaleYState.value
        val happyTilt = happyTiltState.value
        val sparkAlpha = sparkAlphaState.value

        val body = drawCache.bodyFor(shape, sx, sy)
        drawPath(drawCache.sparkPath, color = sparkColor.copy(alpha = sparkAlpha))

        rotate(degrees = bodyTilt, pivot = Offset(24f * sx, 24f * sy)) {
            drawPath(body, color = plush)
            drawPath(body, color = outline, style = drawCache.strokeFor(1.2f * unit))

            val ew = 4.4f * sx
            val eh = (7.6f * sy * eyeScaleY).coerceAtLeast(1.3f)
            val lEyeCenter = Offset((19.0f + faceX) * sx, (24.2f + faceY) * sy)
            rotate(degrees = happyTilt, pivot = lEyeCenter) {
                drawRoundRect(
                    color = inkColor,
                    topLeft = Offset(lEyeCenter.x - ew / 2f, lEyeCenter.y - eh / 2f),
                    size = Size(ew, eh),
                    cornerRadius = CornerRadius(2.2f * sx, 2.2f * sy),
                )
            }
            val rEyeCenter = Offset((29.0f + faceX) * sx, (24.2f + faceY) * sy)
            rotate(degrees = -happyTilt, pivot = rEyeCenter) {
                drawRoundRect(
                    color = inkColor,
                    topLeft = Offset(rEyeCenter.x - ew / 2f, rEyeCenter.y - eh / 2f),
                    size = Size(ew, eh),
                    cornerRadius = CornerRadius(2.2f * sx, 2.2f * sy),
                )
            }

            drawRoundRect(
                color = cheekColor,
                topLeft = Offset((11.4f + faceX) * sx, (26.4f + faceY) * sy),
                size = Size(5f * sx, 2.6f * sy),
                cornerRadius = CornerRadius(1.3f * sx, 1.3f * sy),
            )
            drawRoundRect(
                color = cheekColor,
                topLeft = Offset((31.6f + faceX) * sx, (26.4f + faceY) * sy),
                size = Size(5f * sx, 2.6f * sy),
                cornerRadius = CornerRadius(1.3f * sx, 1.3f * sy),
            )
        }
    }
}
