package org.pjstatecm.graceride.ui.components

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
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
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage

object AvatarStyle {
    val COLORS = listOf("purple", "teal", "coral", "pink", "blue", "green", "amber", "gray", "white")
    val ANIMATED_ICONS = listOf(
        "grace_car",
        "peace_dove",
        "sunny_cloud",
        "happy_bear",
        "fellowship_cat",
        "joy_puppy",
        "star_lantern",
        "blossom_sprout",
        "grok",
        "muse",
    )
    const val DEFAULT_ANIMATED_ICON = "grace_car"
    val STATIC_ICONS = listOf(
        "car", "heart", "star", "music", "sun", "leaf",
        "coffee", "book", "church", "bike", "smile", "flower",
    )
    val ALL_ICONS = STATIC_ICONS + ANIMATED_ICONS

    fun isAnimatedAvatar(icon: String?): Boolean = icon != null && icon in ANIMATED_ICONS

    fun defaultColorFor(icon: String?): String = when (icon) {
        "grace_car" -> "blue"
        "peace_dove" -> "teal"
        "sunny_cloud" -> "amber"
        "happy_bear" -> "coral"
        "fellowship_cat" -> "pink"
        "joy_puppy" -> "green"
        "star_lantern" -> "purple"
        "blossom_sprout" -> "pink"
        "grok" -> "gray"
        "muse" -> "amber"
        else -> "blue"
    }

    fun grokSkinColor(key: String?): Color = when (key) {
        "purple" -> Color(0xFF7C3AED)
        "teal" -> Color(0xFF0D9488)
        "coral" -> Color(0xFFEA580C)
        "pink" -> Color(0xFFDB2777)
        "blue" -> Color(0xFF2563EB)
        "green" -> Color(0xFF16A34A)
        "amber" -> Color(0xFFD97706)
        "white" -> Color(0xFFFFFFFF)
        else -> Color(0xFF18181B) // "gray" & default matte ink-black
    }

    fun grokEyeColor(key: String?): Color = when (key) {
        "white" -> Color(0xFF18181B)
        else -> Color(0xFFFFFFFF)
    }

    fun grokRingColor(key: String?): Color = when (key) {
        "white" -> Color(0xFFCBD5E1)
        else -> Color(0xFFFFFFFF).copy(alpha = 0.2f)
    }

    fun musePlushColors(key: String?): Pair<Color, Color> = when (key) {
        "purple" -> Color(0xFFDDD6FE) to Color(0xFFC4B5FD)
        "teal" -> Color(0xFFCCFBF1) to Color(0xFF99F6E4)
        "coral" -> Color(0xFFFED7AA) to Color(0xFFFDBA74)
        "pink" -> Color(0xFFFCE7F3) to Color(0xFFFBCFE8)
        "blue" -> Color(0xFFDBEAFE) to Color(0xFFBFDBFE)
        "green" -> Color(0xFFDCFCE7) to Color(0xFFBBF7D0)
        "gray" -> Color(0xFFE4E4E7) to Color(0xFFD4D4D8)
        "white" -> Color(0xFFFFFFFF) to Color(0xFFE2E8F0)
        else -> Color(0xFFFEF3C7) to Color(0xFFFDE68A) // "amber" & classic Jolly warm cream
    }

    fun colorPair(key: String?): Pair<Color, Color> = when (key) {
        "teal" -> Color(0xFFD7F3EE) to Color(0xFF0F6B5D)
        "coral" -> Color(0xFFFFE1D6) to Color(0xFFB5411D)
        "pink" -> Color(0xFFFCE0EE) to Color(0xFFA62966)
        "blue" -> Color(0xFFDCEAFE) to Color(0xFF1D4ED8)
        "green" -> Color(0xFFDCF5E4) to Color(0xFF157339)
        "amber" -> Color(0xFFFEF0D1) to Color(0xFF9A5B06)
        "gray" -> Color(0xFFE7EAF0) to Color(0xFF3E4656)
        "white" -> Color(0xFFFFFFFF) to Color(0xFF1E293B)
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

private object ArtPalette {
    val Blue = Color(0xFF2563EB)
    val Sky = Color(0xFF60A5FA)
    val SkySoft = Color(0xFFDBEAFE)
    val Red = Color(0xFFE8462F)
    val Coral = Color(0xFFFB7185)
    val Pink = Color(0xFFF472B6)
    val PinkSoft = Color(0xFFFCE7F3)
    val Blush = Color(0xFFFDA4AF)
    val Amber = Color(0xFFF59E0B)
    val Gold = Color(0xFFFBBF24)
    val Cream = Color(0xFFFEF3C7)
    val Mint = Color(0xFF34D399)
    val Green = Color(0xFF22C55E)
    val Teal = Color(0xFF14B8A6)
    val Lavender = Color(0xFFC4B5FD)
    val Purple = Color(0xFF8B5CF6)
    val Indigo = Color(0xFF4F46E5)
    val Brown = Color(0xFF92400E)
    val Caramel = Color(0xFFD97706)
    val Ink = Color(0xFF1E293B)
    val White = Color(0xFFFFFFFF)
    val Cyan = Color(0xFF22D3EE)
    val Slate = Color(0xFF475569)
}

@Composable
fun GraceCarAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "grace_car")
    val bob by transition.animateFloat(
        initialValue = 0f,
        targetValue = -2f,
        animationSpec = infiniteRepeatable(tween(900, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    val pulse by transition.animateFloat(
        initialValue = 0.55f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(tween(800, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "pulse",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        // Floating heart puff
        drawCircle(ArtPalette.Coral.copy(alpha = pulse), radius = 2.1f * minOf(sx, sy), center = Offset(38.5f * sx, (10.5f + bob) * sy))

        translate(top = bob * sy) {
            // Cabin roof
            val cabin = Path().apply {
                moveTo(12f * sx, 22f * sy)
                cubicTo(13.2f * sx, 16f * sy, 17.5f * sx, 12.5f * sy, 24f * sx, 12.5f * sy)
                cubicTo(30.5f * sx, 12.5f * sy, 34.8f * sx, 16f * sy, 36f * sx, 22f * sy)
                close()
            }
            drawPath(cabin, ArtPalette.Sky)
            drawPath(cabin, ArtPalette.Ink, style = stroke)

            // Windshield
            val glass = Path().apply {
                moveTo(15f * sx, 21.5f * sy)
                cubicTo(16f * sx, 17.5f * sy, 19.2f * sx, 15f * sy, 24f * sx, 15f * sy)
                cubicTo(28.8f * sx, 15f * sy, 32f * sx, 17.5f * sy, 33f * sx, 21.5f * sy)
                close()
            }
            drawPath(glass, ArtPalette.White)
            drawCircle(ArtPalette.Ink, radius = 1.4f * minOf(sx, sy), center = Offset(20.5f * sx, 18.6f * sy))
            drawCircle(ArtPalette.Ink, radius = 1.4f * minOf(sx, sy), center = Offset(27.5f * sx, 18.6f * sy))

            // Car body
            drawRoundRect(ArtPalette.Blue, Offset(7f * sx, 21f * sy), Size(34f * sx, 13f * sy), CornerRadius(6.5f * sx, 6.5f * sy))
            drawRoundRect(ArtPalette.Ink, Offset(7f * sx, 21f * sy), Size(34f * sx, 13f * sy), CornerRadius(6.5f * sx, 6.5f * sy), style = stroke)
            drawRoundRect(ArtPalette.Cream, Offset(9f * sx, 29f * sy), Size(30f * sx, 3f * sy), CornerRadius(1.5f * sx, 1.5f * sy))

            // Headlights & rosy cheeks
            drawCircle(ArtPalette.Gold.copy(alpha = pulse), radius = 2.6f * minOf(sx, sy), center = Offset(11.2f * sx, 25.5f * sy))
            drawCircle(ArtPalette.Gold.copy(alpha = pulse), radius = 2.6f * minOf(sx, sy), center = Offset(36.8f * sx, 25.5f * sy))
            drawOval(ArtPalette.Pink.copy(alpha = 0.85f), Offset(13f * sx, 26.2f * sy), Size(4f * sx, 2.2f * sy))
            drawOval(ArtPalette.Pink.copy(alpha = 0.85f), Offset(31f * sx, 26.2f * sy), Size(4f * sx, 2.2f * sy))

            // Grace Ride cross badge
            drawRoundRect(ArtPalette.White, Offset(20.5f * sx, 23.2f * sy), Size(7f * sx, 5f * sy), CornerRadius(1.5f * sx, 1.5f * sy))
            drawLine(ArtPalette.Red, Offset(24f * sx, 24.1f * sy), Offset(24f * sx, 27.3f * sy), strokeWidth = 1.5f * minOf(sx, sy), cap = StrokeCap.Round)
            drawLine(ArtPalette.Red, Offset(22.4f * sx, 25.7f * sy), Offset(25.6f * sx, 25.7f * sy), strokeWidth = 1.5f * minOf(sx, sy), cap = StrokeCap.Round)
        }
        // Wheels
        drawCircle(ArtPalette.Ink, radius = 4.3f * minOf(sx, sy), center = Offset(15f * sx, 35f * sy))
        drawCircle(ArtPalette.Cream, radius = 1.9f * minOf(sx, sy), center = Offset(15f * sx, 35f * sy))
        drawCircle(ArtPalette.Ink, radius = 4.3f * minOf(sx, sy), center = Offset(33f * sx, 35f * sy))
        drawCircle(ArtPalette.Cream, radius = 1.9f * minOf(sx, sy), center = Offset(33f * sx, 35f * sy))
    }
}

@Composable
fun PeaceDoveAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "peace_dove")
    val bob by transition.animateFloat(
        initialValue = 0f,
        targetValue = -2f,
        animationSpec = infiniteRepeatable(tween(1200, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    val flap by transition.animateFloat(
        initialValue = -10f,
        targetValue = 12f,
        animationSpec = infiniteRepeatable(tween(900, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "flap",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        drawCircle(ArtPalette.Gold, radius = 1.8f * minOf(sx, sy), center = Offset(38f * sx, 11f * sy))

        translate(top = bob * sy) {
            rotate(degrees = flap, pivot = Offset(18f * sx, 22f * sy)) {
                drawOval(ArtPalette.SkySoft, Offset(8f * sx, 16f * sy), Size(11f * sx, 9f * sy))
                drawOval(ArtPalette.Ink, Offset(8f * sx, 16f * sy), Size(11f * sx, 9f * sy), style = stroke)
            }
            drawOval(ArtPalette.White, Offset(12f * sx, 14f * sy), Size(25f * sx, 22f * sy))
            drawOval(ArtPalette.Ink, Offset(12f * sx, 14f * sy), Size(25f * sx, 22f * sy), style = stroke)
            rotate(degrees = -flap * 0.8f, pivot = Offset(22f * sx, 22f * sy)) {
                drawOval(ArtPalette.SkySoft, Offset(17f * sx, 17f * sy), Size(11f * sx, 10f * sy))
                drawOval(ArtPalette.Ink, Offset(17f * sx, 17f * sy), Size(11f * sx, 10f * sy), style = stroke)
            }
            drawCircle(ArtPalette.Ink, radius = 1.7f * minOf(sx, sy), center = Offset(29.5f * sx, 21f * sy))
            drawOval(ArtPalette.Blush, Offset(25.3f * sx, 23f * sy), Size(4.4f * sx, 2.5f * sy))
            // Beak & olive leaf
            val beak = Path().apply {
                moveTo(36.5f * sx, 21.5f * sy)
                lineTo(41f * sx, 23f * sy)
                lineTo(36.5f * sx, 24.8f * sy)
                close()
            }
            drawPath(beak, ArtPalette.Amber)
            drawPath(beak, ArtPalette.Ink, style = stroke)
            drawOval(ArtPalette.Mint, Offset(38f * sx, 25f * sy), Size(5f * sx, 2.5f * sy))
        }
    }
}

@Composable
fun SunnyCloudAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "sunny_cloud")
    val spin by transition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(tween(12000, easing = LinearEasing), RepeatMode.Restart),
        label = "spin",
    )
    val bob by transition.animateFloat(
        initialValue = 0f,
        targetValue = -1.8f,
        animationSpec = infiniteRepeatable(tween(1300, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        rotate(degrees = spin, pivot = Offset(31f * sx, 18.5f * sy)) {
            for (angle in listOf(0f, 60f, 120f, 180f, 240f, 300f)) {
                rotate(degrees = angle, pivot = Offset(31f * sx, 18.5f * sy)) {
                    drawLine(ArtPalette.Amber, Offset(31f * sx, 7.5f * sy), Offset(31f * sx, 10.5f * sy), strokeWidth = 2.2f * minOf(sx, sy), cap = StrokeCap.Round)
                }
            }
        }
        drawCircle(ArtPalette.Gold, radius = 7.5f * minOf(sx, sy), center = Offset(31f * sx, 18.5f * sy))
        drawCircle(ArtPalette.Ink, radius = 7.5f * minOf(sx, sy), center = Offset(31f * sx, 18.5f * sy), style = stroke)

        translate(top = bob * sy) {
            drawCircle(ArtPalette.White, radius = 8f * minOf(sx, sy), center = Offset(24f * sx, 25f * sy))
            drawCircle(ArtPalette.White, radius = 6.8f * minOf(sx, sy), center = Offset(14.5f * sx, 29.2f * sy))
            drawCircle(ArtPalette.White, radius = 6.8f * minOf(sx, sy), center = Offset(33.5f * sx, 29.2f * sy))
            drawRoundRect(ArtPalette.White, Offset(13f * sx, 26f * sy), Size(22f * sx, 10f * sy), CornerRadius(5f * sx, 5f * sy))
            drawRoundRect(ArtPalette.Ink, Offset(9f * sx, 22f * sy), Size(30f * sx, 14f * sy), CornerRadius(7f * sx, 7f * sy), style = stroke)
            drawCircle(ArtPalette.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(19f * sx, 28f * sy))
            drawCircle(ArtPalette.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(29f * sx, 28f * sy))
            drawOval(ArtPalette.Blush, Offset(14.2f * sx, 29.8f * sy), Size(4f * sx, 2.2f * sy))
            drawOval(ArtPalette.Blush, Offset(29.8f * sx, 29.8f * sy), Size(4f * sx, 2.2f * sy))
        }
    }
}

@Composable
fun HappyBearAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "happy_bear")
    val tilt by transition.animateFloat(
        initialValue = -3.5f,
        targetValue = 3.5f,
        animationSpec = infiniteRepeatable(tween(1500, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "tilt",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        drawCircle(ArtPalette.Coral, radius = 2.2f * minOf(sx, sy), center = Offset(39f * sx, 9.5f * sy))
        rotate(degrees = tilt, pivot = Offset(24f * sx, 25f * sy)) {
            drawCircle(ArtPalette.Caramel, radius = 5.5f * minOf(sx, sy), center = Offset(13.5f * sx, 14f * sy))
            drawCircle(ArtPalette.Ink, radius = 5.5f * minOf(sx, sy), center = Offset(13.5f * sx, 14f * sy), style = stroke)
            drawCircle(ArtPalette.Cream, radius = 2.7f * minOf(sx, sy), center = Offset(13.5f * sx, 14f * sy))
            drawCircle(ArtPalette.Caramel, radius = 5.5f * minOf(sx, sy), center = Offset(34.5f * sx, 14f * sy))
            drawCircle(ArtPalette.Ink, radius = 5.5f * minOf(sx, sy), center = Offset(34.5f * sx, 14f * sy), style = stroke)
            drawCircle(ArtPalette.Cream, radius = 2.7f * minOf(sx, sy), center = Offset(34.5f * sx, 14f * sy))

            drawOval(ArtPalette.Caramel, Offset(10.5f * sx, 13f * sy), Size(27f * sx, 23f * sy))
            drawOval(ArtPalette.Ink, Offset(10.5f * sx, 13f * sy), Size(27f * sx, 23f * sy), style = stroke)
            drawOval(ArtPalette.Cream, Offset(17.8f * sx, 22.7f * sy), Size(12.4f * sx, 9f * sy))
            drawOval(ArtPalette.Brown, Offset(21.9f * sx, 24.1f * sy), Size(4.2f * sx, 2.8f * sy))
            drawCircle(ArtPalette.Ink, radius = 1.8f * minOf(sx, sy), center = Offset(18.2f * sx, 22.5f * sy))
            drawCircle(ArtPalette.Ink, radius = 1.8f * minOf(sx, sy), center = Offset(29.8f * sx, 22.5f * sy))
            drawOval(ArtPalette.Coral.copy(alpha = 0.75f), Offset(12.3f * sx, 24.8f * sy), Size(4.4f * sx, 2.4f * sy))
            drawOval(ArtPalette.Coral.copy(alpha = 0.75f), Offset(31.3f * sx, 24.8f * sy), Size(4.4f * sx, 2.4f * sy))
            // Blue scarf
            drawRoundRect(ArtPalette.Blue, Offset(13.5f * sx, 34f * sy), Size(21f * sx, 5.2f * sy), CornerRadius(2.5f * sx, 2.5f * sy))
            drawRoundRect(ArtPalette.Sky, Offset(28.5f * sx, 36f * sy), Size(4.5f * sx, 6f * sy), CornerRadius(1.5f * sx, 1.5f * sy))
        }
    }
}

@Composable
fun FellowshipCatAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "fellowship_cat")
    val tilt by transition.animateFloat(
        initialValue = -3f,
        targetValue = 3f,
        animationSpec = infiniteRepeatable(tween(1700, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "tilt",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        drawCircle(ArtPalette.Gold, radius = 1.8f * minOf(sx, sy), center = Offset(39f * sx, 11f * sy))
        rotate(degrees = tilt, pivot = Offset(24f * sx, 27f * sy)) {
            val earL = Path().apply {
                moveTo(12f * sx, 20f * sy); lineTo(10.5f * sx, 9.5f * sy); lineTo(19f * sx, 15f * sy); close()
            }
            val earR = Path().apply {
                moveTo(36f * sx, 20f * sy); lineTo(37.5f * sx, 9.5f * sy); lineTo(29f * sx, 15f * sy); close()
            }
            drawPath(earL, ArtPalette.Cream); drawPath(earL, ArtPalette.Ink, style = stroke)
            drawPath(earR, ArtPalette.Cream); drawPath(earR, ArtPalette.Ink, style = stroke)
            drawOval(ArtPalette.Cream, Offset(10f * sx, 15f * sy), Size(28f * sx, 24f * sy))
            drawOval(ArtPalette.Ink, Offset(10f * sx, 15f * sy), Size(28f * sx, 24f * sy), style = stroke)
            drawCircle(ArtPalette.Ink, radius = 2.1f * minOf(sx, sy), center = Offset(18.2f * sx, 25.5f * sy))
            drawCircle(ArtPalette.Ink, radius = 2.1f * minOf(sx, sy), center = Offset(29.8f * sx, 25.5f * sy))
            drawCircle(ArtPalette.Coral, radius = 1.7f * minOf(sx, sy), center = Offset(24f * sx, 28f * sy))
            drawOval(ArtPalette.Pink.copy(alpha = 0.8f), Offset(12f * sx, 27.2f * sy), Size(4.6f * sx, 2.6f * sy))
            drawOval(ArtPalette.Pink.copy(alpha = 0.8f), Offset(31.4f * sx, 27.2f * sy), Size(4.6f * sx, 2.6f * sy))
        }
    }
}

@Composable
fun JoyPuppyAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "joy_puppy")
    val bob by transition.animateFloat(
        initialValue = 0f,
        targetValue = -2f,
        animationSpec = infiniteRepeatable(tween(1000, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        translate(top = bob * sy) {
            drawOval(ArtPalette.Brown, Offset(6.5f * sx, 14f * sy), Size(8f * sx, 16f * sy))
            drawOval(ArtPalette.Ink, Offset(6.5f * sx, 14f * sy), Size(8f * sx, 16f * sy), style = stroke)
            drawOval(ArtPalette.Brown, Offset(33.5f * sx, 14f * sy), Size(8f * sx, 16f * sy))
            drawOval(ArtPalette.Ink, Offset(33.5f * sx, 14f * sy), Size(8f * sx, 16f * sy), style = stroke)

            drawOval(ArtPalette.Gold, Offset(11.5f * sx, 12f * sy), Size(25f * sx, 23f * sy))
            drawOval(ArtPalette.Ink, Offset(11.5f * sx, 12f * sy), Size(25f * sx, 23f * sy), style = stroke)
            drawCircle(ArtPalette.Cream, radius = 4.2f * minOf(sx, sy), center = Offset(18.2f * sx, 21.5f * sy))
            drawOval(ArtPalette.Cream, Offset(18.2f * sx, 23f * sy), Size(11.6f * sx, 8.4f * sy))
            drawCircle(ArtPalette.Ink, radius = 1.8f * minOf(sx, sy), center = Offset(18.2f * sx, 21.5f * sy))
            drawCircle(ArtPalette.Ink, radius = 1.8f * minOf(sx, sy), center = Offset(29.8f * sx, 21.5f * sy))
            drawOval(ArtPalette.Ink, Offset(21.8f * sx, 23.8f * sy), Size(4.4f * sx, 2.8f * sy))
            drawLine(ArtPalette.Teal, Offset(15f * sx, 34.5f * sy), Offset(33f * sx, 34.5f * sy), strokeWidth = 3f * minOf(sx, sy), cap = StrokeCap.Round)
            drawCircle(ArtPalette.Amber, radius = 2.4f * minOf(sx, sy), center = Offset(24f * sx, 37.5f * sy))
        }
    }
}

@Composable
fun StarLanternAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "star_lantern")
    val pulse by transition.animateFloat(
        initialValue = 0.85f,
        targetValue = 1.08f,
        animationSpec = infiniteRepeatable(tween(1100, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "pulse",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        drawCircle(ArtPalette.Cyan, radius = 2f * minOf(sx, sy), center = Offset(9.5f * sx, 14f * sy))
        drawCircle(ArtPalette.Pink, radius = 1.8f * minOf(sx, sy), center = Offset(38.5f * sx, 14f * sy))
        val star = Path().apply {
            moveTo(24f * sx, 6.5f * sy)
            lineTo(29f * sx, 16.7f * sy); lineTo(40.2f * sx, 18.3f * sy)
            lineTo(32.1f * sx, 26.2f * sy); lineTo(34f * sx, 37.3f * sy)
            lineTo(24f * sx, 32f * sy); lineTo(14f * sx, 37.3f * sy)
            lineTo(15.9f * sx, 26.2f * sy); lineTo(7.8f * sx, 18.3f * sy)
            lineTo(19f * sx, 16.7f * sy); close()
        }
        drawPath(star, ArtPalette.Gold)
        drawPath(star, ArtPalette.Ink, style = stroke)
        drawCircle(ArtPalette.Cream.copy(alpha = 0.6f * (pulse / 1.08f)), radius = 6.5f * minOf(sx, sy), center = Offset(24f * sx, 24f * sy))
        drawCircle(ArtPalette.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(20.2f * sx, 22.8f * sy))
        drawCircle(ArtPalette.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(27.8f * sx, 22.8f * sy))
        drawOval(ArtPalette.Coral.copy(alpha = 0.75f), Offset(16f * sx, 24.4f * sy), Size(3.6f * sx, 2.2f * sy))
        drawOval(ArtPalette.Coral.copy(alpha = 0.75f), Offset(28.4f * sx, 24.4f * sy), Size(3.6f * sx, 2.2f * sy))
    }
}

@Composable
fun BlossomSproutAvatar(modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "blossom_sprout")
    val sway by transition.animateFloat(
        initialValue = -4f,
        targetValue = 4f,
        animationSpec = infiniteRepeatable(tween(1400, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "sway",
    )
    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        drawCircle(ArtPalette.Sky, radius = 2f * minOf(sx, sy), center = Offset(38f * sx, 11f * sy))
        drawLine(ArtPalette.Green, Offset(24f * sx, 31f * sy), Offset(24f * sx, 41f * sy), strokeWidth = 3f * minOf(sx, sy), cap = StrokeCap.Round)
        drawOval(ArtPalette.Mint, Offset(13.5f * sx, 30.5f * sy), Size(10f * sx, 5.5f * sy))
        drawOval(ArtPalette.Mint, Offset(24.5f * sx, 30.5f * sy), Size(10f * sx, 5.5f * sy))
        rotate(degrees = sway, pivot = Offset(24f * sx, 31f * sy)) {
            drawOval(ArtPalette.Pink, Offset(12f * sx, 10f * sy), Size(24f * sx, 21.5f * sy))
            drawOval(ArtPalette.Ink, Offset(12f * sx, 10f * sy), Size(24f * sx, 21.5f * sy), style = stroke)
            drawCircle(ArtPalette.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(19.5f * sx, 22f * sy))
            drawCircle(ArtPalette.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(28.5f * sx, 22f * sy))
            drawOval(ArtPalette.Coral.copy(alpha = 0.8f), Offset(14.8f * sx, 23.7f * sy), Size(4f * sx, 2.2f * sy))
            drawOval(ArtPalette.Coral.copy(alpha = 0.8f), Offset(29.2f * sx, 23.7f * sy), Size(4f * sx, 2.2f * sy))
        }
    }
}

@Composable
fun GrokBotAvatar(tint: Color, modifier: Modifier = Modifier, avatarColor: String? = null) {
    val transition = rememberInfiniteTransition(label = "grok_bot")
    val bob by transition.animateFloat(
        initialValue = 0.4f,
        targetValue = -2.2f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1700, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "grok_float",
    )
    val lookX by transition.animateFloat(
        initialValue = 0f,
        targetValue = 0f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 5600
                0f at 0
                0f at 1000
                2.3f at 1500
                2.3f at 2500
                -2.3f at 3100
                -2.3f at 4100
                0f at 4700
                0f at 5600
            },
        ),
        label = "grok_look",
    )
    val eyeScaleY by transition.animateFloat(
        initialValue = 1f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 4800
                1f at 0
                1f at 3150
                0.1f at 3300
                1f at 3450
                1f at 3700
                0.1f at 3850
                1f at 4050
                1f at 4800
            },
        ),
        label = "grok_blink",
    )
    val skin = AvatarStyle.grokSkinColor(avatarColor)

    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val shadowScale = (1f + bob * 0.06f).coerceIn(0.8f, 1.08f)

        // Synced ground shadow
        drawOval(
            color = ArtPalette.Ink.copy(alpha = 0.14f * shadowScale),
            topLeft = Offset((24f - 11f * shadowScale) * sx, 40.3f * sy),
            size = Size(22f * shadowScale * sx, 4.4f * sy),
        )

        translate(top = bob * sy) {
            // Minimalist flat-coloured squircle/pebble silhouette
            drawRoundRect(
                color = skin,
                topLeft = Offset(8.5f * sx, 7.5f * sy),
                size = Size(31f * sx, 31f * sy),
                cornerRadius = CornerRadius(13.5f * sx, 13.5f * sy),
            )
            drawRoundRect(
                color = AvatarStyle.grokRingColor(avatarColor),
                topLeft = Offset(8.5f * sx, 7.5f * sy),
                size = Size(31f * sx, 31f * sy),
                cornerRadius = CornerRadius(13.5f * sx, 13.5f * sy),
                style = Stroke(width = 1.2f * minOf(sx, sy)),
            )

            // Specular glossy dot
            rotate(degrees = -24f, pivot = Offset(15.5f * sx, 13.2f * sy)) {
                drawOval(
                    color = ArtPalette.White.copy(alpha = 0.16f),
                    topLeft = Offset(12.1f * sx, 11.7f * sy),
                    size = Size(6.8f * sx, 3f * sy),
                )
            }

            // Two expressive vertical pill eyes (invert on white skin)
            val eyeColor = AvatarStyle.grokEyeColor(avatarColor)
            val w = 4.6f * sx
            val h = (9.8f * sy * eyeScaleY).coerceAtLeast(1.4f)
            val cy = 22.9f * sy - h / 2f
            drawRoundRect(
                color = eyeColor,
                topLeft = Offset((16.2f + lookX) * sx, cy),
                size = Size(w, h),
                cornerRadius = CornerRadius(2.3f * sx, 2.3f * sy),
            )
            drawRoundRect(
                color = eyeColor,
                topLeft = Offset((27.2f + lookX) * sx, cy),
                size = Size(w, h),
                cornerRadius = CornerRadius(2.3f * sx, 2.3f * sy),
            )
        }
        val _unusedTint = tint
    }
}

@Composable
fun MuseBotAvatar(tint: Color, modifier: Modifier = Modifier, avatarColor: String? = null) {
    val transition = rememberInfiniteTransition(label = "muse_bot")
    val bob by transition.animateFloat(
        initialValue = 0.5f,
        targetValue = -2.3f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1450, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "muse_hop",
    )
    val armWave by transition.animateFloat(
        initialValue = 8f,
        targetValue = -22f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1450, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "muse_arm",
    )
    val eyeScaleY by transition.animateFloat(
        initialValue = 1f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = keyframes {
                durationMillis = 4200
                1f at 0
                1f at 3000
                0.12f at 3150
                1f at 3300
                1f at 3550
                0.12f at 3700
                1f at 3900
                1f at 4200
            },
        ),
        label = "muse_eye",
    )
    val (plush, plushShade) = AvatarStyle.musePlushColors(avatarColor)

    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val stroke = Stroke(width = 1.6f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)
        val shadowScale = (1f + bob * 0.06f).coerceIn(0.8f, 1.08f)

        // Synced ground shadow
        drawOval(
            color = ArtPalette.Ink.copy(alpha = 0.14f * shadowScale),
            topLeft = Offset((24f - 12f * shadowScale) * sx, 40.9f * sy),
            size = Size(24f * shadowScale * sx, 4.6f * sy),
        )

        translate(top = bob * sy) {
            // Stubby plush feet
            drawOval(plushShade, Offset(13.8f * sx, 36.8f * sy), Size(8.8f * sx, 5.6f * sy))
            drawOval(ArtPalette.Ink, Offset(13.8f * sx, 36.8f * sy), Size(8.8f * sx, 5.6f * sy), style = stroke)
            drawOval(plushShade, Offset(25.4f * sx, 36.8f * sy), Size(8.8f * sx, 5.6f * sy))
            drawOval(ArtPalette.Ink, Offset(25.4f * sx, 36.8f * sy), Size(8.8f * sx, 5.6f * sy), style = stroke)

            // Stubby waving plush arms
            rotate(degrees = 24f + armWave, pivot = Offset(11.4f * sx, 25f * sy)) {
                drawOval(plush, Offset(6f * sx, 22.2f * sy), Size(6.8f * sx, 9.6f * sy))
                drawOval(ArtPalette.Ink, Offset(6f * sx, 22.2f * sy), Size(6.8f * sx, 9.6f * sy), style = stroke)
            }
            rotate(degrees = -24f - armWave, pivot = Offset(36.6f * sx, 25f * sy)) {
                drawOval(plush, Offset(35.2f * sx, 22.2f * sy), Size(6.8f * sx, 9.6f * sy))
                drawOval(ArtPalette.Ink, Offset(35.2f * sx, 22.2f * sy), Size(6.8f * sx, 9.6f * sy), style = stroke)
            }

            // Plump rotund Jolly plush body with soft crown tuft
            val body = Path().apply {
                moveTo(24f * sx, 7.8f * sy)
                cubicTo(26.2f * sx, 7.8f * sy, 27.8f * sx, 8.8f * sy, 28.5f * sx, 9.9f * sy)
                cubicTo(35.3f * sx, 11.3f * sy, 39.2f * sx, 16.7f * sy, 39.2f * sx, 24.5f * sy)
                cubicTo(39.2f * sx, 33.9f * sy, 33.8f * sx, 39.7f * sy, 24f * sx, 39.7f * sy)
                cubicTo(14.2f * sx, 39.7f * sy, 8.8f * sx, 33.9f * sy, 8.8f * sx, 24.5f * sy)
                cubicTo(8.8f * sx, 16.7f * sy, 12.7f * sx, 11.3f * sy, 19.5f * sx, 9.9f * sy)
                cubicTo(20.2f * sx, 8.8f * sy, 21.8f * sx, 7.8f * sy, 24f * sx, 7.8f * sy)
                close()
            }
            drawPath(body, plush)
            drawPath(body, ArtPalette.Ink, style = stroke)

            // Belly pad & soft highlight
            drawOval(plushShade.copy(alpha = 0.55f), Offset(15f * sx, 24.4f * sy), Size(18f * sx, 12.4f * sy))
            rotate(degrees = -18f, pivot = Offset(18.2f * sx, 14.2f * sy)) {
                drawOval(ArtPalette.White.copy(alpha = 0.48f), Offset(14.4f * sx, 12.6f * sy), Size(7.6f * sx, 3.2f * sy))
            }

            // Small glossy dark blinking eyes with sparkle catchlights
            val rx = 2.1f * sx
            val ry = (2.4f * sy * eyeScaleY).coerceAtLeast(0.9f)
            drawOval(ArtPalette.Ink, topLeft = Offset(18.6f * sx - rx, 21.4f * sy - ry), size = Size(rx * 2f, ry * 2f))
            drawOval(ArtPalette.Ink, topLeft = Offset(29.4f * sx - rx, 21.4f * sy - ry), size = Size(rx * 2f, ry * 2f))
            if (eyeScaleY > 0.6f) {
                drawCircle(ArtPalette.White, radius = 0.7f * minOf(sx, sy), center = Offset(19.3f * sx, 20.6f * sy))
                drawCircle(ArtPalette.White, radius = 0.7f * minOf(sx, sy), center = Offset(30.1f * sx, 20.6f * sy))
            }

            // Prominent soft pink rosy cheeks
            drawOval(ArtPalette.Blush.copy(alpha = 0.92f), Offset(11.4f * sx, 23f * sy), Size(6.2f * sx, 3.6f * sy))
            drawOval(ArtPalette.Blush.copy(alpha = 0.92f), Offset(30.4f * sx, 23f * sy), Size(6.2f * sx, 3.6f * sy))

            // Sweet toothless grin
            val smile = Path().apply {
                moveTo(21f * sx, 26.8f * sy)
                quadraticTo(24f * sx, 29.8f * sy, 27f * sx, 26.8f * sy)
            }
            drawPath(smile, ArtPalette.Ink, style = stroke)
        }
        val _unusedTint = tint
    }
}

@Composable
fun AnimatedMemberAvatar(
    icon: String,
    tint: Color,
    modifier: Modifier = Modifier,
    avatarColor: String? = null,
) {
    when (icon) {
        "grace_car" -> GraceCarAvatar(modifier)
        "peace_dove" -> PeaceDoveAvatar(modifier)
        "sunny_cloud" -> SunnyCloudAvatar(modifier)
        "happy_bear" -> HappyBearAvatar(modifier)
        "fellowship_cat" -> FellowshipCatAvatar(modifier)
        "joy_puppy" -> JoyPuppyAvatar(modifier)
        "star_lantern" -> StarLanternAvatar(modifier)
        "blossom_sprout" -> BlossomSproutAvatar(modifier)
        "grok" -> GrokBotAvatar(tint, modifier, avatarColor = avatarColor)
        "muse" -> MuseBotAvatar(tint, modifier, avatarColor = avatarColor)
        else -> GraceCarAvatar(modifier)
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
        border = if (avatarColor == "white") BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant) else null,
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
                AvatarStyle.isAnimatedAvatar(avatarIcon) -> AnimatedMemberAvatar(
                    icon = avatarIcon!!,
                    tint = fg,
                    modifier = Modifier.size(size * 0.84f),
                    avatarColor = avatarColor,
                )
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
