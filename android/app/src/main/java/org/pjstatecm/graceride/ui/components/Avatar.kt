package org.pjstatecm.graceride.ui.components


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
        "book" -> Icons.AutoMirrored.Outlined.MenuBook
        "church" -> Icons.Outlined.Church
        "bike" -> Icons.AutoMirrored.Outlined.DirectionsBike
        "smile" -> Icons.Outlined.SentimentSatisfied
        "flower" -> Icons.Outlined.LocalFlorist
        else -> null
    }

    /**
     * Resolves bundled Animated WebP avatar URIs. Returns `null` for `grok` and `muse`
     * because those mascots render natively via Compose `Canvas` without bundled bitmap assets.
     */
    fun assetUri(icon: String?, @Suppress("UNUSED_PARAMETER") avatarColor: String? = null): String? {
        val baseIcon = parseAvatarIcon(icon).icon
        if (baseIcon.isEmpty() || baseIcon == "initials" || baseIcon == "grok" || baseIcon == "muse") {
            return null
        }
        return if (baseIcon in ALL_ICONS) "file:///android_asset/icons/avatars/$baseIcon.webp" else null
    }
}

internal class AvatarDrawCache(
    val bodyPath: Path = Path(),
    val sparkPath: Path = Path(),
    var cachedShape: String = "",
    var cachedSx: Float = -1f,
    var cachedSy: Float = -1f,
    var strokeWidth: Float = -1f,
    var stroke: Stroke? = null,
) {
    fun bodyFor(shape: String, sx: Float, sy: Float): Path {
        if (cachedShape != shape || cachedSx != sx || cachedSy != sy) {
            cachedShape = shape
            cachedSx = sx
            cachedSy = sy
            fillBloubShapePath(shape, sx, sy, bodyPath)
            sparkPath.rewind()
            sparkPath.moveTo(39.5f * sx, 8.5f * sy)
            sparkPath.lineTo(40.3f * sx, 10.4f * sy)
            sparkPath.lineTo(42.2f * sx, 11.2f * sy)
            sparkPath.lineTo(40.3f * sx, 12.0f * sy)
            sparkPath.lineTo(39.5f * sx, 13.9f * sy)
            sparkPath.lineTo(38.7f * sx, 12.0f * sy)
            sparkPath.lineTo(36.8f * sx, 11.2f * sy)
            sparkPath.lineTo(38.7f * sx, 10.4f * sy)
            sparkPath.close()
        }
        return bodyPath
    }

    fun strokeFor(width: Float): Stroke {
        val current = stroke
        if (current != null && strokeWidth == width) return current
        strokeWidth = width
        return Stroke(width = width, join = StrokeJoin.Round).also { stroke = it }
    }
}

/** Shared 48x48 Bloub silhouette path helper supporting all 8 geometric & organic mascot shapes. */

@Composable
fun AnimatedMemberAvatar(
    icon: String,
    @Suppress("UNUSED_PARAMETER") tint: Color,
    modifier: Modifier = Modifier,
    avatarColor: String? = null,
    avatarShape: String? = null,
) {
    val parsedIcon = remember(icon) { AvatarStyle.parseAvatarIcon(icon) }
    val baseIcon = parsedIcon.icon
    val resolvedShape = avatarShape ?: parsedIcon.shape
    val resolvedFgColor = remember(parsedIcon.fgColor, avatarColor) {
        parsedIcon.fgColor ?: AvatarStyle.parseAvatarColor(avatarColor).bgColor.ifBlank { null }
    }
    when {
        baseIcon == "grok" -> GrokBotShapeAvatar(
            modifier = modifier,
            avatarColor = resolvedFgColor,
            shape = resolvedShape ?: "hexagon",
        )
        baseIcon == "muse" -> MuseBotShapeAvatar(
            modifier = modifier,
            avatarColor = resolvedFgColor,
            shape = resolvedShape ?: "cloud",
        )
        else -> {
            val uri = remember(icon, avatarColor) {
                AvatarStyle.assetUri(icon, avatarColor)
                    ?: AvatarStyle.assetUri(AvatarStyle.DEFAULT_ANIMATED_ICON, avatarColor)
            }
            val context = LocalContext.current
            val imageRequest = remember(context, uri) {
                ImageRequest.Builder(context)
                    .data(uri)
                    .crossfade(false)
                    .build()
            }
            AsyncImage(
                model = imageRequest,
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
    modifier: Modifier = Modifier,
    photoUrl: String? = null,
    avatarColor: String? = null,
    avatarIcon: String? = null,
    size: Dp = 48.dp,
) {
    val parsedColor = remember(avatarColor) { AvatarStyle.parseAvatarColor(avatarColor) }
    val grokBotSpec = remember(avatarIcon) { GrokBotSpec.parse(avatarIcon) }
    val parsedIcon = remember(avatarIcon, grokBotSpec) {
        AvatarStyle.parseAvatarIcon(if (grokBotSpec != null) null else avatarIcon)
    }
    val bgKey = parsedColor.bgColor.ifBlank { if (grokBotSpec != null) "gray" else "purple" }
    val (bg, baseFg) = AvatarStyle.colorPair(bgKey)
    val fg = AvatarStyle.resolveFgColor(parsedIcon.fgColor, bgKey, baseFg)
    val containerShape = remember(parsedColor.bgShape) { AvatarStyle.bgComposeShape(parsedColor.bgShape) }
    val assetUri = remember(avatarIcon, avatarColor, grokBotSpec) {
        if (grokBotSpec != null) null else AvatarStyle.assetUri(avatarIcon, avatarColor)
    }
    val hasAnimatedAvatar = grokBotSpec == null &&
        (parsedIcon.icon == "grok" || parsedIcon.icon == "muse" || assetUri != null)
    Surface(
        color = if (grokBotSpec != null && parsedColor.bgColor.isBlank()) MaterialTheme.colorScheme.surfaceVariant else bg,
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
                grokBotSpec != null -> GrokBotCanvas(
                    spec = grokBotSpec,
                    move = "idle",
                    variant = "avatar",
                    modifier = Modifier.size(size * 0.88f),
                )
                hasAnimatedAvatar -> AnimatedMemberAvatar(
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
