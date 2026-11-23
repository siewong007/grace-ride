package org.pjstatecm.graceride.ui.components

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.AutoAwesome
import androidx.compose.material.icons.outlined.Bolt
import androidx.compose.material.icons.outlined.CardGiftcard
import androidx.compose.material.icons.outlined.DarkMode
import androidx.compose.material.icons.outlined.DirectionsCar
import androidx.compose.material.icons.outlined.EmojiEvents
import androidx.compose.material.icons.outlined.Explore
import androidx.compose.material.icons.outlined.FavoriteBorder
import androidx.compose.material.icons.outlined.GpsFixed
import androidx.compose.material.icons.outlined.LocalCafe
import androidx.compose.material.icons.outlined.LocalFireDepartment
import androidx.compose.material.icons.outlined.MilitaryTech
import androidx.compose.material.icons.outlined.Schedule
import androidx.compose.material.icons.outlined.SentimentSatisfied
import androidx.compose.material.icons.outlined.Shield
import androidx.compose.material.icons.outlined.StarOutline
import androidx.compose.material.icons.outlined.ThumbUp
import androidx.compose.material.icons.outlined.WbSunny
import androidx.compose.material.icons.outlined.WorkspacePremium
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
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
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import org.pjstatecm.graceride.data.api.Badge

object BadgeIcons {
    val registered: Map<String, ImageVector> = mapOf(
        "award" to Icons.Outlined.WorkspacePremium,
        "trophy" to Icons.Outlined.EmojiEvents,
        "medal" to Icons.Outlined.MilitaryTech,
        "moon" to Icons.Outlined.DarkMode,
        "star" to Icons.Outlined.StarOutline,
        "heart" to Icons.Outlined.FavoriteBorder,
        "shield" to Icons.Outlined.Shield,
        "crown" to Icons.Outlined.WorkspacePremium,
        "sparkles" to Icons.Outlined.AutoAwesome,
        "flame" to Icons.Outlined.LocalFireDepartment,
        "zap" to Icons.Outlined.Bolt,
        "car" to Icons.Outlined.DirectionsCar,
        "clock" to Icons.Outlined.Schedule,
        "target" to Icons.Outlined.GpsFixed,
        "sun" to Icons.Outlined.WbSunny,
        "coffee" to Icons.Outlined.LocalCafe,
        "gift" to Icons.Outlined.CardGiftcard,
        "smile" to Icons.Outlined.SentimentSatisfied,
        "thumbsUp" to Icons.Outlined.ThumbUp,
        "compass" to Icons.Outlined.Explore,
    )

    val animatedKeys: Set<String> = setOf(
        "badge_joy_rider",
        "badge_fellowship_sprout",
        "badge_punctual_clock",
        "badge_warm_welcome",
        "badge_helping_wheels",
        "badge_traffic_light",
        "badge_route_compass",
        "badge_traffic_whistle",
        "badge_care_shield",
    )

    private val allAnimatedDisplayKeys: Set<String> = animatedKeys + setOf("moon", "award", "heart")

    fun isAnimated(key: String?): Boolean = key != null && key in allAnimatedDisplayKeys

    sealed interface ResolvedBadgeIcon {
        data class Animated(val key: String) : ResolvedBadgeIcon
        data class Vector(val icon: ImageVector) : ResolvedBadgeIcon
        data class Text(val text: String) : ResolvedBadgeIcon
    }

    fun resolve(
        badge: Badge? = null,
        icon: String? = null,
        fallbackChar: String? = null,
    ): ResolvedBadgeIcon {
        val chosen = icon ?: badge?.icon
        if (chosen != null && chosen in animatedKeys) {
            return ResolvedBadgeIcon.Animated(chosen)
        }
        val vec = chosen?.let { registered[it] }
        if (vec != null) return ResolvedBadgeIcon.Vector(vec)
        if (!chosen.isNullOrBlank()) return ResolvedBadgeIcon.Text(chosen.trim())
        return ResolvedBadgeIcon.Text(fallbackChar ?: badge?.char ?: "★")
    }
}

private object BadgeArtColors {
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
    val Purple = Color(0xFF8B5CF6)
    val Indigo = Color(0xFF4F46E5)
    val Ink = Color(0xFF1E293B)
    val White = Color(0xFFFFFFFF)
    val Slate = Color(0xFF475569)
}

@Composable
fun AnimatedBadgeCanvas(key: String, modifier: Modifier = Modifier) {
    val transition = rememberInfiniteTransition(label = "badge_$key")
    val bob by transition.animateFloat(
        initialValue = 0f,
        targetValue = -2.2f,
        animationSpec = infiniteRepeatable(tween(950, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "bob",
    )
    val swing by transition.animateFloat(
        initialValue = -12f,
        targetValue = 12f,
        animationSpec = infiniteRepeatable(tween(1300, easing = FastOutSlowInEasing), RepeatMode.Reverse),
        label = "swing",
    )
    val phase by transition.animateFloat(
        initialValue = 0f,
        targetValue = 3f,
        animationSpec = infiniteRepeatable(tween(2400), RepeatMode.Restart),
        label = "phase",
    )

    Canvas(modifier = modifier) {
        val sx = size.width / 56f
        val sy = size.height / 56f
        val stroke = Stroke(width = 1.7f * minOf(sx, sy), cap = StrokeCap.Round, join = StrokeJoin.Round)

        when (key) {
            "badge_joy_rider" -> {
                translate(top = bob * sy) {
                    drawOval(BadgeArtColors.White, Offset(20f * sx, 15.5f * sy), Size(11f * sx, 11f * sy))
                    drawOval(BadgeArtColors.Ink, Offset(20f * sx, 15.5f * sy), Size(11f * sx, 11f * sy), style = stroke)
                    drawRoundRect(BadgeArtColors.Sky, Offset(8f * sx, 29f * sy), Size(40f * sx, 12.5f * sy), CornerRadius(6f * sx, 6f * sy))
                    drawRoundRect(BadgeArtColors.Ink, Offset(8f * sx, 29f * sy), Size(40f * sx, 12.5f * sy), CornerRadius(6f * sx, 6f * sy), style = stroke)
                    drawCircle(BadgeArtColors.Gold, radius = 2.2f * minOf(sx, sy), center = Offset(12.5f * sx, 33f * sy))
                    drawCircle(BadgeArtColors.Gold, radius = 2.2f * minOf(sx, sy), center = Offset(43.5f * sx, 33f * sy))
                }
                drawCircle(BadgeArtColors.Ink, radius = 4.2f * minOf(sx, sy), center = Offset(17f * sx, 42.5f * sy))
                drawCircle(BadgeArtColors.Ink, radius = 4.2f * minOf(sx, sy), center = Offset(39f * sx, 42.5f * sy))
            }
            "badge_fellowship_sprout" -> {
                rotate(degrees = swing * 0.45f, pivot = Offset(28f * sx, 10f * sy)) {
                    drawOval(BadgeArtColors.Gold, Offset(17f * sx, 11f * sy), Size(22f * sx, 20f * sy))
                    drawOval(BadgeArtColors.Ink, Offset(17f * sx, 11f * sy), Size(22f * sx, 20f * sy), style = stroke)
                    drawCircle(BadgeArtColors.Ink, radius = 1.5f * minOf(sx, sy), center = Offset(24f * sx, 22f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.5f * minOf(sx, sy), center = Offset(32f * sx, 22f * sy))
                }
                drawOval(BadgeArtColors.Mint, Offset(14.5f * sx, 40.4f * sy), Size(11f * sx, 5.2f * sy))
                drawOval(BadgeArtColors.Mint, Offset(30.5f * sx, 40.4f * sy), Size(11f * sx, 5.2f * sy))
                drawCircle(BadgeArtColors.Pink, radius = 3.4f * minOf(sx, sy), center = Offset(28f * sx, 40f * sy))
                drawCircle(BadgeArtColors.PinkSoft, radius = 3.2f * minOf(sx, sy), center = Offset(24.5f * sx, 42.5f * sy))
                drawCircle(BadgeArtColors.PinkSoft, radius = 3.2f * minOf(sx, sy), center = Offset(31.5f * sx, 42.5f * sy))
            }
            "badge_punctual_clock" -> {
                rotate(degrees = swing * 0.3f, pivot = Offset(28f * sx, 28.5f * sy)) {
                    drawCircle(BadgeArtColors.Amber, radius = 5f * minOf(sx, sy), center = Offset(15.5f * sx, 14.5f * sy))
                    drawCircle(BadgeArtColors.Amber, radius = 5f * minOf(sx, sy), center = Offset(40.5f * sx, 14.5f * sy))
                    drawCircle(BadgeArtColors.Mint, radius = 15.5f * minOf(sx, sy), center = Offset(28f * sx, 28.5f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 15.5f * minOf(sx, sy), center = Offset(28f * sx, 28.5f * sy), style = stroke)
                    drawCircle(BadgeArtColors.White, radius = 11.8f * minOf(sx, sy), center = Offset(28f * sx, 28.5f * sy))
                    drawLine(BadgeArtColors.Teal, Offset(28f * sx, 21.5f * sy), Offset(28f * sx, 28.5f * sy), strokeWidth = 2f * minOf(sx, sy), cap = StrokeCap.Round)
                    drawLine(BadgeArtColors.Teal, Offset(28f * sx, 28.5f * sy), Offset(32.5f * sx, 31f * sy), strokeWidth = 2f * minOf(sx, sy), cap = StrokeCap.Round)
                }
                drawCircle(BadgeArtColors.Green, radius = 7f * minOf(sx, sy), center = Offset(43f * sx, 40.5f * sy))
                drawCircle(BadgeArtColors.Ink, radius = 7f * minOf(sx, sy), center = Offset(43f * sx, 40.5f * sy), style = stroke)
            }
            "badge_warm_welcome", "heart" -> {
                translate(top = bob * sy) {
                    drawCircle(BadgeArtColors.PinkSoft, radius = 10f * minOf(sx, sy), center = Offset(32f * sx, 25f * sy))
                    drawCircle(BadgeArtColors.Coral, radius = 12f * minOf(sx, sy), center = Offset(24f * sx, 30f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 12f * minOf(sx, sy), center = Offset(24f * sx, 30f * sy), style = stroke)
                    drawCircle(BadgeArtColors.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(20f * sx, 29.5f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(28f * sx, 29.5f * sy))
                    drawOval(BadgeArtColors.Cream, Offset(15.5f * sx, 32f * sy), Size(3.6f * sx, 2f * sy))
                    drawOval(BadgeArtColors.Cream, Offset(28.9f * sx, 32f * sy), Size(3.6f * sx, 2f * sy))
                }
            }
            "moon" -> {
                rotate(degrees = swing * 0.4f, pivot = Offset(28f * sx, 28f * sy)) {
                    drawCircle(BadgeArtColors.Gold, radius = 14f * minOf(sx, sy), center = Offset(26f * sx, 30f * sy))
                    drawOval(BadgeArtColors.Purple, Offset(21f * sx, 18f * sy), Size(18f * sx, 19f * sy))
                    drawOval(BadgeArtColors.Ink, Offset(21f * sx, 18f * sy), Size(18f * sx, 19f * sy), style = stroke)
                    drawCircle(BadgeArtColors.White, radius = 3.5f * minOf(sx, sy), center = Offset(26.2f * sx, 24.5f * sy))
                    drawCircle(BadgeArtColors.White, radius = 3.5f * minOf(sx, sy), center = Offset(33.8f * sx, 24.5f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.5f * minOf(sx, sy), center = Offset(26.5f * sx, 24.6f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.5f * minOf(sx, sy), center = Offset(33.5f * sx, 24.6f * sy))
                }
            }
            "award" -> {
                translate(top = bob * sy) {
                    drawLine(BadgeArtColors.Blue, Offset(21f * sx, 32f * sy), Offset(18f * sx, 46f * sy), strokeWidth = 5f * minOf(sx, sy), cap = StrokeCap.Round)
                    drawLine(BadgeArtColors.Coral, Offset(35f * sx, 32f * sy), Offset(38f * sx, 46f * sy), strokeWidth = 5f * minOf(sx, sy), cap = StrokeCap.Round)
                    drawCircle(BadgeArtColors.Gold, radius = 14.5f * minOf(sx, sy), center = Offset(28f * sx, 23f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 14.5f * minOf(sx, sy), center = Offset(28f * sx, 23f * sy), style = stroke)
                    drawCircle(BadgeArtColors.Cream, radius = 11f * minOf(sx, sy), center = Offset(28f * sx, 23f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(23.5f * sx, 22f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.6f * minOf(sx, sy), center = Offset(32.5f * sx, 22f * sy))
                }
            }
            "badge_helping_wheels" -> {
                drawOval(BadgeArtColors.Gold, Offset(19f * sx, 7f * sy), Size(18f * sx, 6f * sy), style = stroke)
                translate(top = bob * sy) {
                    drawOval(BadgeArtColors.Cream, Offset(6f * sx, 24f * sy), Size(10f * sx, 8f * sy))
                    drawOval(BadgeArtColors.Cream, Offset(40f * sx, 24f * sy), Size(10f * sx, 8f * sy))
                    drawCircle(BadgeArtColors.Blue, radius = 13.5f * minOf(sx, sy), center = Offset(28f * sx, 30f * sy), style = Stroke(width = 4.2f * minOf(sx, sy)))
                    drawCircle(BadgeArtColors.White, radius = 6.5f * minOf(sx, sy), center = Offset(28f * sx, 30f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 6.5f * minOf(sx, sy), center = Offset(28f * sx, 30f * sy), style = stroke)
                    drawCircle(BadgeArtColors.Ink, radius = 1.2f * minOf(sx, sy), center = Offset(25.8f * sx, 29.2f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.2f * minOf(sx, sy), center = Offset(30.2f * sx, 29.2f * sy))
                }
            }
            "badge_traffic_light" -> {
                val redAlpha = if (phase < 1f) 1f else 0.32f
                val amberAlpha = if (phase >= 1f && phase < 2f) 1f else 0.32f
                val greenAlpha = if (phase >= 2f) 1f else 0.32f
                translate(top = bob * sy) {
                    drawRoundRect(BadgeArtColors.Blue, Offset(20f * sx, 8f * sy), Size(16f * sx, 5f * sy), CornerRadius(2.5f * sx, 2.5f * sy))
                    drawRoundRect(BadgeArtColors.Slate, Offset(17f * sx, 13f * sy), Size(22f * sx, 33f * sy), CornerRadius(8f * sx, 8f * sy))
                    drawRoundRect(BadgeArtColors.Ink, Offset(17f * sx, 13f * sy), Size(22f * sx, 33f * sy), CornerRadius(8f * sx, 8f * sy), style = stroke)
                    drawCircle(BadgeArtColors.Red.copy(alpha = redAlpha), radius = 3.8f * minOf(sx, sy), center = Offset(28f * sx, 20.5f * sy))
                    drawCircle(BadgeArtColors.Gold.copy(alpha = amberAlpha), radius = 3.8f * minOf(sx, sy), center = Offset(28f * sx, 29.5f * sy))
                    drawCircle(BadgeArtColors.Green.copy(alpha = greenAlpha), radius = 3.8f * minOf(sx, sy), center = Offset(28f * sx, 38.5f * sy))
                }
            }
            "badge_route_compass" -> {
                drawCircle(BadgeArtColors.Gold, radius = 17f * minOf(sx, sy), center = Offset(28f * sx, 30f * sy))
                drawCircle(BadgeArtColors.Ink, radius = 17f * minOf(sx, sy), center = Offset(28f * sx, 30f * sy), style = stroke)
                drawCircle(BadgeArtColors.SkySoft, radius = 13f * minOf(sx, sy), center = Offset(28f * sx, 30f * sy))
                rotate(degrees = swing, pivot = Offset(28f * sx, 30f * sy)) {
                    val needleTop = Path().apply {
                        moveTo(28f * sx, 18.5f * sy); lineTo(31.2f * sx, 30f * sy); lineTo(24.8f * sx, 30f * sy); close()
                    }
                    val needleBottom = Path().apply {
                        moveTo(28f * sx, 41.5f * sy); lineTo(31.2f * sx, 30f * sy); lineTo(24.8f * sx, 30f * sy); close()
                    }
                    drawPath(needleTop, BadgeArtColors.Red)
                    drawPath(needleBottom, BadgeArtColors.White)
                }
                drawCircle(BadgeArtColors.Cream, radius = 5.2f * minOf(sx, sy), center = Offset(28f * sx, 30f * sy))
                drawCircle(BadgeArtColors.Ink, radius = 1.1f * minOf(sx, sy), center = Offset(26.2f * sx, 29.3f * sy))
                drawCircle(BadgeArtColors.Ink, radius = 1.1f * minOf(sx, sy), center = Offset(29.8f * sx, 29.3f * sy))
            }
            "badge_traffic_whistle" -> {
                rotate(degrees = swing * 0.45f, pivot = Offset(27f * sx, 34.5f * sy)) {
                    drawCircle(BadgeArtColors.Gold, radius = 3.8f * minOf(sx, sy), center = Offset(13f * sx, 33f * sy))
                    drawRoundRect(BadgeArtColors.Sky, Offset(14f * sx, 21f * sy), Size(26f * sx, 26f * sy), CornerRadius(13f * sx, 13f * sy))
                    drawRoundRect(BadgeArtColors.Ink, Offset(14f * sx, 21f * sy), Size(26f * sx, 26f * sy), CornerRadius(13f * sx, 13f * sy), style = stroke)
                    drawRoundRect(BadgeArtColors.Coral, Offset(36f * sx, 23.5f * sy), Size(10.5f * sx, 8.5f * sy), CornerRadius(2f * sx, 2f * sy))
                    drawCircle(BadgeArtColors.White, radius = 8.2f * minOf(sx, sy), center = Offset(27f * sx, 34.5f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.4f * minOf(sx, sy), center = Offset(24f * sx, 33.5f * sy))
                    drawCircle(BadgeArtColors.Ink, radius = 1.4f * minOf(sx, sy), center = Offset(30f * sx, 33.5f * sy))
                }
            }
            "badge_care_shield" -> {
                translate(top = bob * sy) {
                    val crown = Path().apply {
                        moveTo(20f * sx, 14.5f * sy); lineTo(23f * sx, 9.5f * sy); lineTo(28f * sx, 13f * sy)
                        lineTo(33f * sx, 9.5f * sy); lineTo(36f * sx, 14.5f * sy); lineTo(34.5f * sx, 18f * sy)
                        lineTo(21.5f * sx, 18f * sy); close()
                    }
                    drawPath(crown, BadgeArtColors.Gold)
                    drawPath(crown, BadgeArtColors.Ink, style = stroke)
                }
                val shield = Path().apply {
                    moveTo(28f * sx, 17f * sy); lineTo(43f * sx, 22f * sy); lineTo(43f * sx, 33.5f * sy)
                    cubicTo(43f * sx, 42f * sy, 36.5f * sx, 48f * sy, 28f * sx, 50.5f * sy)
                    cubicTo(19.5f * sx, 48f * sy, 13f * sx, 42f * sy, 13f * sx, 33.5f * sy)
                    lineTo(13f * sx, 22f * sy); close()
                }
                drawPath(shield, BadgeArtColors.Mint)
                drawPath(shield, BadgeArtColors.Ink, style = stroke)
                drawCircle(BadgeArtColors.Ink, radius = 1.5f * minOf(sx, sy), center = Offset(23.5f * sx, 29f * sy))
                drawCircle(BadgeArtColors.Ink, radius = 1.5f * minOf(sx, sy), center = Offset(32.5f * sx, 29f * sy))
                drawCircle(BadgeArtColors.Coral, radius = 3.2f * minOf(sx, sy), center = Offset(28f * sx, 36.5f * sy))
            }
        }
    }
}

@Composable
fun BadgeIcon(
    badge: Badge? = null,
    icon: String? = null,
    fallbackChar: String? = null,
    size: Dp = 24.dp,
    tint: Color = LocalContentColor.current,
    modifier: Modifier = Modifier,
) {
    val chosen = icon ?: badge?.icon
    if (BadgeIcons.isAnimated(chosen)) {
        AnimatedBadgeCanvas(
            key = chosen!!,
            modifier = modifier.size(size),
        )
        return
    }
    when (val resolved = BadgeIcons.resolve(badge, icon, fallbackChar)) {
        is BadgeIcons.ResolvedBadgeIcon.Animated -> AnimatedBadgeCanvas(
            key = resolved.key,
            modifier = modifier.size(size),
        )
        is BadgeIcons.ResolvedBadgeIcon.Vector -> androidx.compose.material3.Icon(
            imageVector = resolved.icon,
            contentDescription = null,
            tint = tint,
            modifier = modifier.size(size),
        )
        is BadgeIcons.ResolvedBadgeIcon.Text -> Text(
            text = resolved.text,
            color = tint,
            modifier = modifier,
        )
    }
}

data class BadgeTierPalette(
    val bg: Color,
    val ring: Color,
    val ink: Color,
    val spark: Color? = null,
)

object BadgeTierStyle {
    val ALL_TIERS = listOf("bronze", "silver", "gold", "platinum")

    fun resolveTier(badge: Badge?): String {
        if (badge == null) return "none"
        if (badge.tier in ALL_TIERS || badge.tier == "none") return badge.tier
        return if (badge.earned) "bronze" else "none"
    }

    fun palette(tier: String?): BadgeTierPalette = when (tier) {
        "bronze" -> BadgeTierPalette(
            bg = Color(0xFFFEF3E6),
            ring = Color(0xFFD97706),
            ink = Color(0xFF92400E),
        )
        "silver" -> BadgeTierPalette(
            bg = Color(0xFFF1F5F9),
            ring = Color(0xFF94A3B8),
            ink = Color(0xFF334155),
        )
        "gold" -> BadgeTierPalette(
            bg = Color(0xFFFEF9C3),
            ring = Color(0xFFF59E0B),
            ink = Color(0xFF854D0E),
            spark = Color(0xFFFBBF24),
        )
        "platinum" -> BadgeTierPalette(
            bg = Color(0xFFE0F2FE),
            ring = Color(0xFF38BDF8),
            ink = Color(0xFF075985),
            spark = Color(0xFFA855F7),
        )
        else -> BadgeTierPalette(
            bg = Color(0xFFF1F5F9),
            ring = Color(0xFFCBD5E1),
            ink = Color(0xFF64748B),
        )
    }
}

@Composable
fun TieredBadgeMedallion(
    badge: Badge? = null,
    icon: String? = null,
    fallbackChar: String? = null,
    size: Dp = 56.dp,
    modifier: Modifier = Modifier,
) {
    val tier = BadgeTierStyle.resolveTier(badge)
    val palette = BadgeTierStyle.palette(tier)
    val ringColor = if (tier == "gold" || tier == "platinum") {
        val pulse by rememberInfiniteTransition(label = "tier_medallion").animateFloat(
            initialValue = 0.68f,
            targetValue = 1f,
            animationSpec = infiniteRepeatable(tween(1400, easing = FastOutSlowInEasing), RepeatMode.Reverse),
            label = "tier_ring_alpha",
        )
        palette.ring.copy(alpha = pulse)
    } else {
        palette.ring
    }

    Surface(
        color = palette.bg,
        contentColor = palette.ink,
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(if (tier == "none") 1.5.dp else 2.dp, ringColor),
        modifier = modifier.size(size),
    ) {
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.padding(4.dp),
        ) {
            BadgeIcon(
                badge = badge,
                icon = icon,
                fallbackChar = fallbackChar,
                size = size * 0.78f,
                tint = palette.ink,
            )
        }
    }
}
