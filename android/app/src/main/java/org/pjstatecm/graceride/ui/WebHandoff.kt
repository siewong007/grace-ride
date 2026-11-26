package org.pjstatecm.graceride.ui

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import androidx.browser.customtabs.CustomTabColorSchemeParams
import androidx.browser.customtabs.CustomTabsClient
import androidx.browser.customtabs.CustomTabsIntent
import androidx.core.net.toUri

/**
 * Opens web-only workflows (reimbursements, family & staff tools) in an in-app AndroidX
 * Custom Tab for HTTP(S) URLs with Grace Ride's light/dark toolbar chrome, pinning an external
 * browser package so verified App Links do not loop back into this app, and falling back to an
 * external `ACTION_VIEW` browser for non-Custom-Tabs environments or custom schemes (`tg://`).
 */
fun openInWebApp(context: Context, url: String) {
    val uri = url.toUri()
    val scheme = uri.scheme?.lowercase()
    val isHttpAllowed = scheme == "https" || (scheme == "http" && org.pjstatecm.graceride.BuildConfig.DEBUG)
    val isTelegram = scheme == "tg"
    if (!isHttpAllowed && !isTelegram) {
        return
    }
    val isWebUrl = isHttpAllowed
    if (isWebUrl) {
        val customTabsPkg = runCatching { CustomTabsClient.getPackageName(context, null) }
            .getOrNull()
            ?.takeIf { it != context.packageName }
        if (customTabsPkg != null) {
            val lightParams = CustomTabColorSchemeParams.Builder()
                .setToolbarColor(Color.WHITE)
                .setNavigationBarColor(Color.WHITE)
                .build()
            val darkParams = CustomTabColorSchemeParams.Builder()
                .setToolbarColor(0xFF141517.toInt())
                .setNavigationBarColor(0xFF141517.toInt())
                .build()
            val customTabs = CustomTabsIntent.Builder()
                .setColorScheme(CustomTabsIntent.COLOR_SCHEME_SYSTEM)
                .setDefaultColorSchemeParams(lightParams)
                .setColorSchemeParams(CustomTabsIntent.COLOR_SCHEME_DARK, darkParams)
                .setShowTitle(true)
                .setUrlBarHidingEnabled(true)
                .build()
            customTabs.intent.setPackage(customTabsPkg)
            if (context !is Activity) {
                customTabs.intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            try {
                customTabs.launchUrl(context, uri)
                return
            } catch (_: ActivityNotFoundException) {
                // Fall back to external browser handler below.
            } catch (_: SecurityException) {
                // Fall back to external browser handler below.
            }
        }
    }

    val intent = Intent(Intent.ACTION_VIEW, uri).apply {
        if (context !is Activity) {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        if (isWebUrl) {
            resolveExternalBrowserPackage(context, uri)?.let { setPackage(it) }
        }
    }
    if (isWebUrl && intent.`package` == null) {
        val resolvedPkg = runCatching {
            context.packageManager.resolveActivity(intent, PackageManager.MATCH_DEFAULT_ONLY)?.activityInfo?.packageName
        }.getOrNull()
        if (resolvedPkg == context.packageName) {
            // Never launch an un-packaged App Link that resolves back to MainActivity in a loop.
            return
        }
    }
    try {
        context.startActivity(intent)
    } catch (_: ActivityNotFoundException) {
        // No browser or handler available on this device.
    } catch (_: SecurityException) {
        // Cross-app launch disallowed by policy.
    }
}

private fun resolveExternalBrowserPackage(context: Context, uri: android.net.Uri): String? {
    fun findIn(probeUri: android.net.Uri): String? {
        val probe = Intent(Intent.ACTION_VIEW, probeUri).addCategory(Intent.CATEGORY_BROWSABLE)
        val candidates = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.packageManager.queryIntentActivities(
                probe,
                PackageManager.ResolveInfoFlags.of(PackageManager.MATCH_ALL.toLong()),
            )
        } else {
            @Suppress("DEPRECATION")
            context.packageManager.queryIntentActivities(probe, PackageManager.MATCH_ALL)
        }
        return candidates
            .firstOrNull { info ->
                val pkg = info.activityInfo?.packageName
                !pkg.isNullOrBlank() && pkg != context.packageName
            }
            ?.activityInfo
            ?.packageName
    }
    return findIn(uri) ?: findIn("https://example.com".toUri())
}
