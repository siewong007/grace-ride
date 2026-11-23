package org.pjstatecm.graceride.ui.components

import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest

object EventIcons {
    val PRESET_KEYS: Set<String> = setOf(
        "church",
        "bookOpen",
        "music",
        "flame",
        "heartHandshake",
        "sparkles",
        "utensils",
        "coffee",
        "cake",
        "users",
        "gift",
        "home",
        "trophy",
        "footprints",
        "waves",
        "bike",
        "tent",
        "mountain",
        "dumbbell",
        "target",
        "gamepad",
        "film",
        "camera",
        "bus",
        "baby",
        "calendar",
        "sun",
        "moon",
    )

    val DEFAULT_KIND_ICON: Map<String, String> = mapOf(
        "badminton" to "trophy",
        "bowling" to "target",
        "kayak" to "waves",
        "walk" to "footprints",
        "other" to "sparkles",
    )

    fun gifAssetUri(icon: String?, kind: String = ""): String? {
        val trimmed = icon?.trim().orEmpty()
        if (trimmed in PRESET_KEYS) {
            return "file:///android_asset/icons/events/$trimmed.gif"
        }
        if (trimmed.isNotEmpty()) {
            return null
        }
        val fallbackKey = DEFAULT_KIND_ICON[kind] ?: "calendar"
        return "file:///android_asset/icons/events/$fallbackKey.gif"
    }
}

@Composable
fun EventIcon(
    icon: String?,
    kind: String = "",
    size: Dp = 22.dp,
    modifier: Modifier = Modifier,
) {
    val assetUri = EventIcons.gifAssetUri(icon, kind)
    if (assetUri != null) {
        AsyncImage(
            model = ImageRequest.Builder(LocalContext.current)
                .data(assetUri)
                .crossfade(false)
                .build(),
            contentDescription = null,
            contentScale = ContentScale.Fit,
            modifier = modifier.size(size),
        )
    } else {
        Text(
            text = icon?.trim().orEmpty(),
            fontSize = (size.value * 0.85f).sp,
            modifier = modifier,
        )
    }
}
