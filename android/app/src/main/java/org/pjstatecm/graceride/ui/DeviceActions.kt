package org.pjstatecm.graceride.ui

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.view.HapticFeedbackConstants
import android.view.View
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ChevronRight
import androidx.compose.material.icons.outlined.Map
import androidx.compose.material.icons.outlined.Navigation
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.theme.Spacing

/** Preferred turn-by-turn navigation provider for drivers. */
enum class NavProvider(val key: String) {
    ASK("ask"),
    WAZE("waze"),
    GOOGLE_MAPS("google"),
    ;

    companion object {
        fun fromKey(raw: String?): NavProvider = entries.firstOrNull { it.key == raw } ?: ASK
    }
}

private data class PendingNavTarget(
    val lat: Double?,
    val lng: Double?,
    val query: String,
)

/** Launch the system dialer pre-filled with [phone] without requiring CALL_PHONE permission. */
fun dialPhone(context: Context, phone: String) {
    val cleaned = phone.filter { it.isDigit() || it == '+' }
    if (cleaned.isEmpty()) return
    val intent = Intent(Intent.ACTION_DIAL, "tel:$cleaned".toUri()).apply {
        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    try {
        context.startActivity(intent)
    } catch (_: ActivityNotFoundException) {
        // No dialer activity available on this device.
    }
}

/** Launch Waze turn-by-turn navigation, falling back to `waze.com/ul` in web/browser. */
fun openInWaze(context: Context, lat: Double?, lng: Double?, query: String) {
    val trimmed = query.trim()
    val wazeUri = when {
        lat != null && lng != null -> "waze://?ll=$lat,$lng&navigate=yes"
        trimmed.isNotEmpty() -> "waze://?q=${Uri.encode(trimmed)}&navigate=yes"
        else -> return
    }
    val intent = Intent(Intent.ACTION_VIEW, wazeUri.toUri()).apply {
        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    try {
        context.startActivity(intent)
    } catch (_: ActivityNotFoundException) {
        val webUrl = when {
            lat != null && lng != null -> "https://waze.com/ul?ll=$lat,$lng&navigate=yes"
            else -> "https://waze.com/ul?q=${Uri.encode(trimmed)}&navigate=yes"
        }
        openInWebApp(context, webUrl)
    }
}

/** Launch Google Maps turn-by-turn navigation, falling back to [openInMaps]. */
fun openInGoogleMaps(context: Context, lat: Double?, lng: Double?, query: String) {
    val trimmed = query.trim()
    val target = when {
        lat != null && lng != null -> "google.navigation:q=$lat,$lng"
        trimmed.isNotEmpty() -> "google.navigation:q=${Uri.encode(trimmed)}"
        else -> return
    }
    val intent = Intent(Intent.ACTION_VIEW, target.toUri()).apply {
        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    try {
        context.startActivity(intent)
    } catch (_: ActivityNotFoundException) {
        val fallback = if (lat != null && lng != null) "$lat,$lng" else trimmed
        openInMaps(context, fallback)
    }
}

/** Launch navigation for ([lat], [lng]) or [query] using [provider]. */
fun openInNavigation(
    context: Context,
    lat: Double?,
    lng: Double?,
    query: String,
    provider: NavProvider = NavProvider.GOOGLE_MAPS,
) {
    when (provider) {
        NavProvider.WAZE -> openInWaze(context, lat, lng, query)
        NavProvider.GOOGLE_MAPS, NavProvider.ASK -> openInGoogleMaps(context, lat, lng, query)
    }
}

/**
 * Returns a navigation callback that respects the user's saved [AppContainer.navAppPref],
 * or opens a clean minimalist Waze vs. Google Maps chooser sheet when set to `"ask"`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun rememberSmartNavigator(container: AppContainer): (Double?, Double?, String) -> Unit {
    val context = LocalContext.current
    val view = LocalView.current
    val prefRaw by container.navAppPref.collectAsStateWithLifecycle()
    val pref = NavProvider.fromKey(prefRaw)
    var pendingTarget by remember { mutableStateOf<PendingNavTarget?>(null) }

    pendingTarget?.let { target ->
        val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
        var rememberChoice by remember(target) { mutableStateOf(true) }
        ModalBottomSheet(
            onDismissRequest = { pendingTarget = null },
            sheetState = sheetState,
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .navigationBarsPadding()
                    .padding(horizontal = Spacing.sm, vertical = Spacing.sm),
                verticalArrangement = Arrangement.spacedBy(Spacing.sm),
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        stringResource(R.string.nav_chooser_title),
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    if (target.query.isNotBlank()) {
                        Text(
                            target.query,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth(),
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
                ) {
                    Column {
                        NavOptionRow(
                            title = stringResource(R.string.nav_app_waze),
                            subtitle = stringResource(R.string.nav_waze_subtitle),
                            icon = Icons.Outlined.Navigation,
                            onClick = {
                                UiHaptics.confirm(view)
                                if (rememberChoice) container.setNavAppPref(NavProvider.WAZE.key)
                                openInWaze(context, target.lat, target.lng, target.query)
                                pendingTarget = null
                            },
                        )
                        HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                        NavOptionRow(
                            title = stringResource(R.string.nav_app_google),
                            subtitle = stringResource(R.string.nav_google_subtitle),
                            icon = Icons.Outlined.Map,
                            onClick = {
                                UiHaptics.confirm(view)
                                if (rememberChoice) container.setNavAppPref(NavProvider.GOOGLE_MAPS.key)
                                openInGoogleMaps(context, target.lat, target.lng, target.query)
                                pendingTarget = null
                            },
                        )
                    }
                }
                Row(
                    Modifier
                        .fillMaxWidth()
                        .clickable { rememberChoice = !rememberChoice }
                        .padding(vertical = 2.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Spacing.xs),
                ) {
                    Checkbox(checked = rememberChoice, onCheckedChange = { rememberChoice = it })
                    Text(
                        stringResource(R.string.nav_remember_choice),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                OutlinedButton(
                    onClick = { pendingTarget = null },
                    modifier = Modifier.fillMaxWidth().height(46.dp),
                ) {
                    Text(stringResource(R.string.common_cancel))
                }
                Spacer(Modifier.height(Spacing.xs))
            }
        }
    }

    return { lat, lng, query ->
        when (pref) {
            NavProvider.WAZE -> openInWaze(context, lat, lng, query)
            NavProvider.GOOGLE_MAPS -> openInGoogleMaps(context, lat, lng, query)
            NavProvider.ASK -> pendingTarget = PendingNavTarget(lat, lng, query)
        }
    }
}

@Composable
private fun NavOptionRow(
    title: String,
    subtitle: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    onClick: () -> Unit,
) {
    Surface(onClick = onClick, color = Color.Transparent) {
        Row(
            Modifier
                .fillMaxWidth()
                .defaultMinSize(minHeight = 56.dp)
                .padding(horizontal = Spacing.sm, vertical = Spacing.xs),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Spacing.sm),
        ) {
            Icon(icon, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
            Column(Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onSurface)
                Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Icon(
                Icons.Outlined.ChevronRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/** Open [query] in the native maps app, falling back to OpenStreetMap in the browser. */
fun openInMaps(context: Context, query: String) {
    val trimmed = query.trim()
    if (trimmed.isEmpty()) return
    val intent = Intent(Intent.ACTION_VIEW, "geo:0,0?q=${Uri.encode(trimmed)}".toUri()).apply {
        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    try {
        context.startActivity(intent)
    } catch (_: ActivityNotFoundException) {
        openInWebApp(context, "https://www.openstreetmap.org/search?query=${Uri.encode(trimmed)}")
    }
}

/** Copy plain [text] to the system clipboard and return true if the service accepted it. */
fun copyToClipboard(context: Context, label: String, text: String, sensitive: Boolean = false): Boolean {
    val cm = context.getSystemService(ClipboardManager::class.java) ?: return false
    val clip = ClipData.newPlainText(label, text).apply {
        if (sensitive && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            description.extras = (description.extras ?: android.os.PersistableBundle()).apply {
                putBoolean(android.content.ClipDescription.EXTRA_IS_SENSITIVE, true)
            }
        }
    }
    cm.setPrimaryClip(clip)
    return true
}

private tailrec fun Context.findWindowActivity(): android.app.Activity? = when (this) {
    is android.app.Activity -> this
    is android.content.ContextWrapper -> baseContext?.findWindowActivity()
    else -> null
}

private object SecureWindowRegistry {
    private val counts = java.util.WeakHashMap<android.view.Window, Int>()

    fun acquire(window: android.view.Window) {
        val next = (counts[window] ?: 0) + 1
        counts[window] = next
        if (next == 1) {
            window.addFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE)
        }
    }

    fun release(window: android.view.Window) {
        val prev = counts[window] ?: return
        val next = (prev - 1).coerceAtLeast(0)
        if (next == 0) {
            counts.remove(window)
            window.clearFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE)
        } else {
            counts[window] = next
        }
    }
}

/**
 * Ref-counted `FLAG_SECURE` window guard for screens and sheets displaying or accepting
 * sensitive PII, home addresses, pass/check-in codes, or unlock credentials.
 */
@Composable
fun SecureScreenEffect() {
    val context = LocalContext.current
    androidx.compose.runtime.DisposableEffect(context) {
        val window = context.findWindowActivity()?.window
        if (window != null) {
            SecureWindowRegistry.acquire(window)
        }
        onDispose {
            if (window != null) {
                SecureWindowRegistry.release(window)
            }
        }
    }
}

/** Tactile feedback helpers for state transitions, steppers, and confirmation actions. */
object UiHaptics {
    fun tick(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
    }

    fun confirm(view: View) {
        val code = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            HapticFeedbackConstants.CONFIRM
        } else {
            HapticFeedbackConstants.VIRTUAL_KEY
        }
        view.performHapticFeedback(code)
    }

    fun tap(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
    }
}
