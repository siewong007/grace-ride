package org.pjstatecm.graceride.ui.components

import androidx.compose.foundation.layout.size
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
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
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

    sealed interface ResolvedBadgeIcon {
        data class Vector(val icon: ImageVector) : ResolvedBadgeIcon
        data class Text(val text: String) : ResolvedBadgeIcon
    }

    fun resolve(
        badge: Badge? = null,
        icon: String? = null,
        fallbackChar: String? = null,
    ): ResolvedBadgeIcon {
        val chosen = icon ?: badge?.icon
        val vec = chosen?.let { registered[it] }
        if (vec != null) return ResolvedBadgeIcon.Vector(vec)
        if (!chosen.isNullOrBlank()) return ResolvedBadgeIcon.Text(chosen.trim())
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
    when (val resolved = BadgeIcons.resolve(badge, icon, fallbackChar)) {
        is BadgeIcons.ResolvedBadgeIcon.Vector -> Icon(
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
