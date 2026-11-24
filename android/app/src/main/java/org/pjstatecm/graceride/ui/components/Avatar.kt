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

object AvatarStyle {
    val COLORS = listOf("purple", "teal", "coral", "pink", "blue", "green", "amber", "gray", "white")
    val AVATAR_SHAPES = listOf(
        "hexagon",
        "circle",
        "pebble",
        "squircle",
        "capsule",
        "triangle",
        "cloud",
        "droplet",
    )
    val AVATAR_BG_SHAPES = listOf(
        "circle",
        "squircle",
        "rounded",
        "hexagon",
        "cloud",
        "pebble",
    )
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

    data class ParsedAvatarColor(val bgColor: String, val bgShape: String)
    data class ParsedAvatarIcon(val icon: String, val shape: String?, val fgColor: String?)

    fun parseAvatarColor(raw: String?): ParsedAvatarColor {
        val trimmed = raw?.trim().orEmpty()
        if (trimmed.isEmpty()) return ParsedAvatarColor("", "circle")
        val parts = trimmed.split(':')
        val bg = parts.getOrNull(0)?.takeIf { it in COLORS } ?: ""
        val shape = parts.getOrNull(1)?.takeIf { it in AVATAR_BG_SHAPES } ?: "circle"
        return ParsedAvatarColor(bg, shape)
    }

    fun encodeAvatarColor(bgColor: String?, bgShape: String? = null): String {
        val bg = bgColor?.trim()?.takeIf { it in COLORS } ?: ""
        val shape = bgShape?.trim()?.takeIf { it in AVATAR_BG_SHAPES } ?: "circle"
        return if (shape == "circle") bg else "$bg:$shape"
    }

    fun parseAvatarIcon(raw: String?): ParsedAvatarIcon {
        val trimmed = raw?.trim().orEmpty()
        if (trimmed.isEmpty()) return ParsedAvatarIcon("", null, null)
        val parts = trimmed.split(':')
        val base = parts.getOrNull(0).orEmpty()
        val icon = if (base == "initials" || base in ALL_ICONS) base else ""
        val shape = parts.getOrNull(1)?.takeIf { it in AVATAR_SHAPES }
        val fg = parts.getOrNull(2)?.takeIf { it in COLORS }
        return ParsedAvatarIcon(icon, shape, fg)
    }

    fun defaultAvatarShape(icon: String?): String = when (parseAvatarIcon(icon).icon) {
        "muse" -> "cloud"
        else -> "hexagon"
    }

    fun isShapeableAvatar(icon: String?): Boolean =
        parseAvatarIcon(icon).icon in listOf("grok", "muse")

    fun encodeAvatarIcon(icon: String?, shape: String? = null, fgColor: String? = null): String {
        val base = icon?.trim().orEmpty()
        val fg = fgColor?.trim()?.takeIf { it in COLORS }
        if (base.isEmpty() || base == "initials") {
            return if (fg != null) "initials::$fg" else ""
        }
        if (base !in ALL_ICONS) return ""
        val validShape = shape?.trim()?.takeIf { it in AVATAR_SHAPES && base in listOf("grok", "muse") }
        val includeShape = validShape != null && validShape != defaultAvatarShape(base)
        return when {
            includeShape && fg != null -> "$base:$validShape:$fg"
            fg != null -> "$base::$fg"
            includeShape -> "$base:$validShape"
            else -> base
        }
    }

    fun isAnimatedAvatar(icon: String?): Boolean =
        parseAvatarIcon(icon).icon in ANIMATED_ICONS

    fun defaultColorFor(icon: String?): String = when (parseAvatarIcon(icon).icon) {
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

    fun grokSkinColor(key: String?): Color = when (key?.substringBefore(':')) {
        "purple" -> Color(0xFF7C3AED)
        "teal" -> Color(0xFF0D9488)
        "coral" -> Color(0xFFEA580C)
        "pink" -> Color(0xFFDB2777)
        "blue" -> Color(0xFF5A96EB)
        "green" -> Color(0xFF16A34A)
        "amber" -> Color(0xFFD97706)
        "white" -> Color(0xFFFFFFFF)
        else -> Color(0xFF111113) // "gray" & default jet black
    }

    fun grokEyeColor(key: String?): Color = when (key?.substringBefore(':')) {
        "white" -> Color(0xFF18181B)
        else -> Color(0xFFFFFFFF)
    }

    fun grokRingColor(key: String?): Color = when (key?.substringBefore(':')) {
        "white" -> Color(0xFFCBD5E1)
        else -> Color.Transparent
    }

    fun musePlushColors(key: String?): Pair<Color, Color> = when (key?.substringBefore(':')) {
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

    fun colorPair(key: String?): Pair<Color, Color> = when (parseAvatarColor(key).bgColor) {
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

    fun resolveFgColor(fgKey: String?, bgKey: String, fallbackFg: Color): Color = when (fgKey) {
        null -> fallbackFg
        "white" -> if (bgKey == "white") colorPair("white").second else Color.White
        else -> colorPair(fgKey).second
    }

    fun bgComposeShape(bgShape: String?): Shape = when (bgShape) {
        "squircle" -> RoundedCornerShape(percent = 30)
        "rounded" -> RoundedCornerShape(percent = 20)
        "pebble" -> RoundedCornerShape(
            topStartPercent = 48,
            topEndPercent = 56,
            bottomEndPercent = 42,
            bottomStartPercent = 52,
        )
        "hexagon" -> GenericShape { sz, _ ->
            val sx = sz.width / 48f
            val sy = sz.height / 48f
            moveTo(17.5f * sx, 2.4f * sy)
            lineTo(30.5f * sx, 2.4f * sy)
            quadraticTo(36.2f * sx, 2.4f * sy, 39.1f * sx, 7.4f * sy)
            lineTo(44.8f * sx, 17.3f * sy)
            quadraticTo(47.6f * sx, 24f * sy, 44.8f * sx, 30.7f * sy)
            lineTo(39.1f * sx, 40.6f * sy)
            quadraticTo(36.2f * sx, 45.6f * sy, 30.5f * sx, 45.6f * sy)
            lineTo(17.5f * sx, 45.6f * sy)
            quadraticTo(11.8f * sx, 45.6f * sy, 8.9f * sx, 40.6f * sy)
            lineTo(3.2f * sx, 30.7f * sy)
            quadraticTo(0.4f * sx, 24f * sy, 3.2f * sx, 17.3f * sy)
            lineTo(8.9f * sx, 7.4f * sy)
            quadraticTo(11.8f * sx, 2.4f * sy, 17.5f * sx, 2.4f * sy)
            close()
        }
        "cloud" -> GenericShape { sz, _ ->
            val sx = sz.width / 48f
            val sy = sz.height / 48f
            moveTo(24f * sx, 5.5f * sy)
            cubicTo(28f * sx, 2f * sy, 36.4f * sx, 3.5f * sy, 38.6f * sx, 9.4f * sy)
            cubicTo(45.8f * sx, 11.8f * sy, 47.4f * sx, 21f * sy, 45.4f * sx, 29f * sy)
            cubicTo(43.6f * sx, 38.5f * sy, 37f * sx, 45f * sy, 24f * sx, 45f * sy)
            cubicTo(11f * sx, 45f * sy, 4.4f * sx, 38.5f * sy, 2.6f * sx, 29f * sy)
            cubicTo(0.6f * sx, 21f * sy, 2.2f * sx, 11.8f * sy, 9.4f * sx, 9.4f * sy)
            cubicTo(11.6f * sx, 3.5f * sy, 20f * sx, 2f * sy, 24f * sx, 5.5f * sy)
            close()
        }
        else -> CircleShape
    }

    fun staticIconVector(key: String?): ImageVector? = when (parseAvatarIcon(key).icon) {
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

    fun gifAssetUri(icon: String?, avatarColor: String? = null): String? {
        val parsedIcon = parseAvatarIcon(icon)
        val baseIcon = parsedIcon.icon
        if (baseIcon.isEmpty() || baseIcon == "initials") return null
        val bgColor = parseAvatarColor(avatarColor).bgColor.ifBlank { null }
        val effectiveColor = parsedIcon.fgColor ?: bgColor
        return when {
            baseIcon == "grok" -> "file:///android_asset/icons/avatars/grok_${effectiveColor ?: "gray"}.gif"
            baseIcon == "muse" -> "file:///android_asset/icons/avatars/muse_${effectiveColor ?: "amber"}.gif"
            baseIcon in ALL_ICONS -> "file:///android_asset/icons/avatars/$baseIcon.gif"
            else -> null
        }
    }
}

/** Shared 48x48 Bloub silhouette path helper supporting all 8 geometric & organic mascot shapes. */
private fun bloubShapePath(shape: String, sx: Float, sy: Float): Path = Path().apply {
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

@Composable
private fun GrokBotShapeAvatar(
    modifier: Modifier = Modifier,
    avatarColor: String? = null,
    shape: String = "hexagon",
) {
    val transition = rememberInfiniteTransition(label = "grok_bot")
    val bodyTilt by transition.animateFloat(
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
    )
    val lookX by transition.animateFloat(
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
    )
    val lookY by transition.animateFloat(
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
    )
    val leftScaleY by transition.animateFloat(
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
    )
    val rightScaleY by transition.animateFloat(
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
    )
    val rightRot by transition.animateFloat(
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
    )
    val skin = AvatarStyle.grokSkinColor(avatarColor)
    val ring = AvatarStyle.grokRingColor(avatarColor)
    val eyeColor = AvatarStyle.grokEyeColor(avatarColor)

    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val unit = minOf(sx, sy)

        rotate(degrees = bodyTilt, pivot = Offset(24f * sx, 24f * sy)) {
            val body = bloubShapePath(shape, sx, sy)
            drawPath(body, color = skin)
            if (ring != Color.Transparent) {
                drawPath(body, color = ring, style = Stroke(width = 1.2f * unit, join = StrokeJoin.Round))
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
private fun MuseBotShapeAvatar(
    modifier: Modifier = Modifier,
    avatarColor: String? = null,
    shape: String = "cloud",
) {
    val transition = rememberInfiniteTransition(label = "muse_bot")
    val bodyTilt by transition.animateFloat(
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
    )
    val faceX by transition.animateFloat(
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
    )
    val faceY by transition.animateFloat(
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
    )
    val eyeScaleY by transition.animateFloat(
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
    )
    val happyTilt by transition.animateFloat(
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
    )
    val sparkAlpha by transition.animateFloat(
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
    )
    val colorKey = avatarColor?.substringBefore(':')
    val (plush, plushShade) = AvatarStyle.musePlushColors(colorKey)
    val sparkColor = if (colorKey == "white") Color(0xFFF59E0B) else AvatarStyle.grokSkinColor(colorKey)
    val outline = if (colorKey == "white") Color(0xFFCBD5E1) else plushShade
    val inkColor = Color(0xFF18181B)
    val cheekColor = Color(0xFFF48FB1).copy(alpha = 0.88f)

    Canvas(modifier = modifier) {
        val sx = size.width / 48f
        val sy = size.height / 48f
        val unit = minOf(sx, sy)

        val sparkPath = Path().apply {
            moveTo(39.5f * sx, 8.5f * sy)
            lineTo(40.3f * sx, 10.4f * sy)
            lineTo(42.2f * sx, 11.2f * sy)
            lineTo(40.3f * sx, 12.0f * sy)
            lineTo(39.5f * sx, 13.9f * sy)
            lineTo(38.7f * sx, 12.0f * sy)
            lineTo(36.8f * sx, 11.2f * sy)
            lineTo(38.7f * sx, 10.4f * sy)
            close()
        }
        drawPath(sparkPath, color = sparkColor.copy(alpha = sparkAlpha))

        rotate(degrees = bodyTilt, pivot = Offset(24f * sx, 24f * sy)) {
            val body = bloubShapePath(shape, sx, sy)
            drawPath(body, color = plush)
            drawPath(body, color = outline, style = Stroke(width = 1.2f * unit, join = StrokeJoin.Round))

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

@Composable
fun AnimatedMemberAvatar(
    icon: String,
    @Suppress("UNUSED_PARAMETER") tint: Color,
    modifier: Modifier = Modifier,
    avatarColor: String? = null,
    avatarShape: String? = null,
) {
    val parsedIcon = AvatarStyle.parseAvatarIcon(icon)
    val baseIcon = parsedIcon.icon
    val resolvedShape = avatarShape ?: parsedIcon.shape
    val resolvedFgColor = parsedIcon.fgColor ?: AvatarStyle.parseAvatarColor(avatarColor).bgColor.ifBlank { null }
    when {
        baseIcon == "grok" && resolvedShape != null && resolvedShape != "hexagon" -> GrokBotShapeAvatar(
            modifier = modifier,
            avatarColor = resolvedFgColor,
            shape = resolvedShape,
        )
        baseIcon == "muse" && resolvedShape != null && resolvedShape != "cloud" -> MuseBotShapeAvatar(
            modifier = modifier,
            avatarColor = resolvedFgColor,
            shape = resolvedShape,
        )
        else -> {
            val uri = AvatarStyle.gifAssetUri(icon, avatarColor)
                ?: AvatarStyle.gifAssetUri(AvatarStyle.DEFAULT_ANIMATED_ICON, avatarColor)
            AsyncImage(
                model = ImageRequest.Builder(LocalContext.current)
                    .data(uri)
                    .crossfade(false)
                    .build(),
                contentDescription = null,
                contentScale = ContentScale.Fit,
                modifier = modifier,
            )
        }
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
    val parsedColor = AvatarStyle.parseAvatarColor(avatarColor)
    val parsedIcon = AvatarStyle.parseAvatarIcon(avatarIcon)
    val bgKey = parsedColor.bgColor.ifBlank { "purple" }
    val (bg, baseFg) = AvatarStyle.colorPair(bgKey)
    val fg = AvatarStyle.resolveFgColor(parsedIcon.fgColor, bgKey, baseFg)
    val containerShape = AvatarStyle.bgComposeShape(parsedColor.bgShape)
    val gifUri = AvatarStyle.gifAssetUri(avatarIcon, avatarColor)
    Surface(
        color = bg,
        contentColor = fg,
        shape = containerShape,
        border = if (avatarColor?.substringBefore(':') == "white") {
            BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
        } else {
            null
        },
        modifier = modifier.size(size),
    ) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            when {
                photoUrl != null -> AsyncImage(
                    model = photoUrl,
                    contentDescription = name,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize().clip(containerShape),
                )
                gifUri != null -> AnimatedMemberAvatar(
                    icon = avatarIcon!!,
                    tint = fg,
                    modifier = Modifier.size(size * 0.84f),
                    avatarColor = avatarColor,
                    avatarShape = parsedIcon.shape,
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
