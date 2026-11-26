package org.pjstatecm.graceride.ui.components

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest
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

    val allAssetKeys: Set<String> = animatedKeys + registered.keys

    fun assetUri(key: String?): String? =
        key?.trim()?.takeIf { it in allAssetKeys }?.let { "file:///android_asset/icons/badges/$it.webp" }

    fun isAnimated(key: String?): Boolean = assetUri(key) != null

    sealed interface ResolvedBadgeIcon {
        data class AnimatedAsset(val key: String, val assetUri: String) : ResolvedBadgeIcon
        data class Text(val text: String) : ResolvedBadgeIcon
    }

    fun resolve(
        badge: Badge? = null,
        icon: String? = null,
        fallbackChar: String? = null,
    ): ResolvedBadgeIcon {
        val chosen = icon?.trim()?.takeIf { it.isNotEmpty() } ?: badge?.icon?.trim()
        val uri = assetUri(chosen)
        if (!chosen.isNullOrEmpty() && uri != null) {
            return ResolvedBadgeIcon.AnimatedAsset(chosen, uri)
        }
        if (!chosen.isNullOrEmpty()) {
            return ResolvedBadgeIcon.Text(chosen)
        }
        return ResolvedBadgeIcon.Text(fallbackChar ?: badge?.char ?: "★")
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
    val resolved = remember(badge, icon, fallbackChar) {
        BadgeIcons.resolve(badge, icon, fallbackChar)
    }
    when (resolved) {
        is BadgeIcons.ResolvedBadgeIcon.AnimatedAsset -> {
            val context = LocalContext.current
            val request = remember(context, resolved.assetUri) {
                ImageRequest.Builder(context)
                    .data(resolved.assetUri)
                    .crossfade(false)
                    .build()
            }
            AsyncImage(
                model = request,
                contentDescription = null,
                contentScale = ContentScale.Fit,
                modifier = modifier.size(size),
            )
        }
        is BadgeIcons.ResolvedBadgeIcon.Text -> Text(
            text = resolved.text,
            color = tint,
            fontSize = (size.value * 0.85f).sp,
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

    fun palette(tier: String?, darkTheme: Boolean = false): BadgeTierPalette = if (darkTheme) {
        when (tier) {
            "bronze" -> BadgeTierPalette(
                bg = Color(0xFF2C2115),
                ring = Color(0xFFD97706),
                ink = Color(0xFFFCD39B),
            )
            "silver" -> BadgeTierPalette(
                bg = Color(0xFF1F242C),
                ring = Color(0xFF94A3B8),
                ink = Color(0xFFE2E8F0),
            )
            "gold" -> BadgeTierPalette(
                bg = Color(0xFF2D2610),
                ring = Color(0xFFF59E0B),
                ink = Color(0xFFFDE68A),
                spark = Color(0xFFFBBF24),
            )
            "platinum" -> BadgeTierPalette(
                bg = Color(0xFF112536),
                ring = Color(0xFF38BDF8),
                ink = Color(0xFFBAE6FD),
                spark = Color(0xFFA855F7),
            )
            else -> BadgeTierPalette(
                bg = Color(0xFF1F2124),
                ring = Color(0xFF3A3F47),
                ink = Color(0xFFA3A3A3),
            )
        }
    } else {
        when (tier) {
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
    val darkTheme = MaterialTheme.colorScheme.background.luminance() < 0.5f
    val palette = BadgeTierStyle.palette(tier, darkTheme = darkTheme)
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
