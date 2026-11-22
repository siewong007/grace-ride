package org.pjstatecm.graceride.ui.components

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Church
import androidx.compose.material.icons.outlined.DirectionsBike
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.Eco
import androidx.compose.material.icons.outlined.FavoriteBorder
import androidx.compose.material.icons.outlined.LocalCafe
import androidx.compose.material.icons.outlined.LocalFlorist
import androidx.compose.material.icons.outlined.MenuBook
import androidx.compose.material.icons.outlined.MusicNote
import androidx.compose.material.icons.outlined.SentimentSatisfied
import androidx.compose.material.icons.outlined.StarOutline
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage

object AvatarStyle {
    val COLORS = listOf("purple", "teal", "coral", "pink", "blue", "green", "amber", "gray")
    val ANIMATED_ICONS = listOf("grok", "muse")
    val STATIC_ICONS = listOf(
        "car", "heart", "star", "music", "sun", "leaf",
        "coffee", "book", "church", "bike", "smile", "flower",
    )
    val ALL_ICONS = STATIC_ICONS + ANIMATED_ICONS

    fun isAnimatedAvatar(icon: String?): Boolean = icon != null && icon in ANIMATED_ICONS

    fun colorPair(key: String?): Pair<Color, Color> = when (key) {
        "teal" -> Color(0xFFD7F3EE) to Color(0xFF0F6B5D)
        "coral" -> Color(0xFFFFE1D6) to Color(0xFFB5411D)
        "pink" -> Color(0xFFFCE0EE) to Color(0xFFA62966)
        "blue" -> Color(0xFFDCEAFE) to Color(0xFF1D4ED8)
        "green" -> Color(0xFFDCF5E4) to Color(0xFF157339)
        "amber" -> Color(0xFFFEF0D1) to Color(0xFF9A5B06)
        "gray" -> Color(0xFFE7EAF0) to Color(0xFF3E4656)
        else -> Color(0xFFECE3FC) to Color(0xFF5B2DB8) // "purple" & default
    }

    fun staticIconVector(key: String?): ImageVector? = when (key) {
        "car" -> Icons.Outlined.DirectionsCar
        "heart" -> Icons.Outlined.FavoriteBorder
        "star" -> Icons.Outlined.StarOutline
        "music" -> Icons.Outlined.MusicNote
        "sun" -> Icons.Outlined.WbSunny
        "leaf" -> Icons.Outlined.Eco
        "coffee" -> Icons.Outlined.LocalCafe
        "book" -> Icons.Outlined.MenuBook
        "church" -> Icons.Outlined.Church
        "bike" -> Icons.Outlined.DirectionsBike
        "smile" -> Icons.Outlined.SentimentSatisfied
        "flower" -> Icons.Outlined.LocalFlorist
        else -> null
    }
}

@Composable
fun GrokBotAvatar(tint: Color, modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "grok_bot")
    val pulse by transition.animateFloat(
        initialValue = 0.45f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1200, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "grok_pulse",
    )
    val eyeScaleY by transition.animateFloat(
        initialValue = 1f,
        targetValue = 0.25f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1600, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "grok_blink",
    )

    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 2.5f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)

        // Bot antenna with pulsing node
        drawLine(tint, Offset(24f * sx, 10f * sy), Offset(24f * sx, 4f * sy), strokeWidth = stroke.width, cap = StrokeCap.Round)
        drawCircle(tint.copy(alpha = pulse), radius = (2.2f + 0.8f * pulse) * minOf(sx, sy), center = Offset(24f * sx, 3f * sy))
        drawLine(tint, Offset(14f * sx, 12f * sy), Offset(10f * sx, 8f * sy), strokeWidth = stroke.width, cap = StrokeCap.Round)
        drawLine(tint, Offset(34f * sx, 12f * sy), Offset(38f * sx, 8f * sy), strokeWidth = stroke.width, cap = StrokeCap.Round)

        // Cyber robotic head chassis
        drawRoundRect(
            color = tint,
            topLeft = Offset(10f * sx, 10f * sy),
            size = Size(28f * sx, 24f * sy),
            cornerRadius = CornerRadius(7f * sx, 7f * sy),
            style = stroke,
        )

        // Visor screen
        drawRoundRect(
            color = tint.copy(alpha = 0.15f),
            topLeft = Offset(14f * sx, 16f * sy),
            size = Size(20f * sx, 12f * sy),
            cornerRadius = CornerRadius(3.5f * sx, 3.5f * sy),
        )

        // Pulsing / blinking visor eyes
        val r = 2.5f * minOf(sx, sy)
        drawOval(
            color = tint,
            topLeft = Offset(19f * sx - r, 22f * sy - r * eyeScaleY),
            size = Size(r * 2f, (r * 2f * eyeScaleY).coerceAtLeast(1.5f)),
        )
        drawOval(
            color = tint,
            topLeft = Offset(29f * sx - r, 22f * sy - r * eyeScaleY),
            size = Size(r * 2f, (r * 2f * eyeScaleY).coerceAtLeast(1.5f)),
        )

        // Grok diagonal accent slash
        drawLine(tint.copy(alpha = 0.4f + 0.6f * pulse), Offset(21f * sx, 28f * sy), Offset(27f * sx, 24f * sy), strokeWidth = stroke.width, cap = StrokeCap.Round)

        // Neck base
        val neck = Path().apply {
            moveTo(20f * sx, 34f * sy)
            lineTo(20f * sx, 38f * sy)
            lineTo(28f * sx, 38f * sy)
            lineTo(28f * sx, 34f * sy)
        }
        drawPath(neck, tint, style = stroke)
        drawLine(tint, Offset(16f * sx, 42f * sy), Offset(32f * sx, 42f * sy), strokeWidth = stroke.width, cap = StrokeCap.Round)
    }
}

@Composable
fun MuseBotAvatar(tint: Color, modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "muse_bot")
    val waveAlpha by transition.animateFloat(
        initialValue = 0.25f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1000, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "muse_wave",
    )
    val eyeScaleY by transition.animateFloat(
        initialValue = 1f,
        targetValue = 0.35f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1400, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "muse_eye",
    )

    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 2.5f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)

        // Soundwave aura arcs
        val leftWave1 = Path().apply {
            moveTo(7f * sx, 21f * sy)
            quadraticTo(5.5f * sx, 24f * sy, 7f * sx, 29f * sy)
        }
        val leftWave2 = Path().apply {
            moveTo(4f * sx, 18f * sy)
            quadraticTo(1.5f * sx, 24f * sy, 4f * sx, 32f * sy)
        }
        val rightWave1 = Path().apply {
            moveTo(41f * sx, 21f * sy)
            quadraticTo(42.5f * sx, 24f * sy, 41f * sx, 29f * sy)
        }
        val rightWave2 = Path().apply {
            moveTo(44f * sx, 18f * sy)
            quadraticTo(46.5f * sx, 24f * sy, 44f * sx, 32f * sy)
        }
        drawPath(leftWave1, tint.copy(alpha = waveAlpha), style = stroke)
        drawPath(rightWave1, tint.copy(alpha = waveAlpha), style = stroke)
        drawPath(leftWave2, tint.copy(alpha = 1.25f - waveAlpha), style = stroke)
        drawPath(rightWave2, tint.copy(alpha = 1.25f - waveAlpha), style = stroke)

        // Friendly rounded bot head
        drawRoundRect(
            color = tint,
            topLeft = Offset(10f * sx, 11f * sy),
            size = Size(28f * sx, 26f * sy),
            cornerRadius = CornerRadius(13f * sx, 13f * sy),
            style = stroke,
        )

        // Ear nodes
        drawCircle(tint, radius = 2f * minOf(sx, sy), center = Offset(10f * sx, 24f * sy))
        drawCircle(tint, radius = 2f * minOf(sx, sy), center = Offset(38f * sx, 24f * sy))

        // Animated eyes
        val rx = 2.5f * sx
        val ry = (3.5f * sy * eyeScaleY).coerceAtLeast(1.5f)
        drawOval(tint, topLeft = Offset(19f * sx - rx, 22f * sy - ry), size = Size(rx * 2f, ry * 2f))
        drawOval(tint, topLeft = Offset(29f * sx - rx, 22f * sy - ry), size = Size(rx * 2f, ry * 2f))

        // Smiling waveform mouth
        val smile = Path().apply {
            moveTo(20f * sx, 29f * sy)
            quadraticTo(24f * sx, 32f * sy, 28f * sx, 29f * sy)
        }
        drawPath(smile, tint, style = stroke)

        // Sparkle antenna
        drawLine(tint, Offset(24f * sx, 11f * sy), Offset(24f * sx, 5f * sy), strokeWidth = stroke.width, cap = StrokeCap.Round)
        drawCircle(tint.copy(alpha = waveAlpha), radius = 2.5f * minOf(sx, sy), center = Offset(24f * sx, 4f * sy))
    }
}

@Composable
fun MemberAvatar(
    name: String,
    photoUrl: String? = null,
    avatarColor: String? = null,
    avatarIcon: String? = null,
    size: Dp = 48.dp,
    modifier: Modifier = Modifier,
) {
    val (bg, fg) = AvatarStyle.colorPair(avatarColor)
    Surface(
        color = bg,
        contentColor = fg,
        shape = CircleShape,
        modifier = modifier.size(size),
    ) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            when {
                photoUrl != null -> AsyncImage(
                    model = photoUrl,
                    contentDescription = name,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize().clip(CircleShape),
                )
                avatarIcon == "grok" -> GrokBotAvatar(fg, Modifier.size(size * 0.72f))
                avatarIcon == "muse" -> MuseBotAvatar(fg, Modifier.size(size * 0.72f))
                AvatarStyle.staticIconVector(avatarIcon) != null -> Icon(
                    AvatarStyle.staticIconVector(avatarIcon)!!,
                    contentDescription = name,
                    tint = fg,
                    modifier = Modifier.size(size * 0.52f),
                )
                else -> {
                    val initial = name.trim().firstOrNull()?.uppercase() ?: "?"
                    Text(
                        initial,
                        style = if (size >= 56.dp) MaterialTheme.typography.headlineSmall else MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.SemiBold,
                        color = fg,
                    )
                }
            }
        }
    }
}
