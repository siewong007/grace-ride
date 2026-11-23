package org.pjstatecm.graceride.ui

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri

/**
 * Hands off web-only workflows (trip chat, live map, QR check-in, reimbursements,
 * family & staff tools) cleanly to the browser / installed PWA without crashing if no
 * browser activity is available.
 */
fun openInWebApp(context: Context, url: String) {
    val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url)).apply {
        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    try {
        context.startActivity(intent)
    } catch (_: ActivityNotFoundException) {
        // No browser handler available on this device.
    }
}
