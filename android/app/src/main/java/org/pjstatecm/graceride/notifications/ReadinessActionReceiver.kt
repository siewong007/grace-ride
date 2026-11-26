package org.pjstatecm.graceride.notifications

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import org.pjstatecm.graceride.GraceRideApp
import org.pjstatecm.graceride.MainActivity
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.runSuspendCatching

class ReadinessActionReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val rideId = intent.getLongExtra(EXTRA_RIDE_ID, -1L)
        val status = intent.getStringExtra(EXTRA_STATUS)
        val notifId = intent.getIntExtra(EXTRA_NOTIF_ID, -1)
        val notifItemId = intent.getLongExtra(EXTRA_NOTIF_ITEM_ID, -1L)
        if (rideId <= 0L || status.isNullOrBlank()) return

        val app = context.applicationContext as? GraceRideApp ?: return
        val pending = goAsync()
        CoroutineScope(SupervisorJob() + Dispatchers.IO).launch {
            try {
                withTimeout(7_500) {
                    app.container.rides.readiness(rideId, status)
                    if (notifItemId > 0L) {
                        runSuspendCatching { app.container.community.readOne(notifItemId) }
                    }
                    if (notifId > 0) {
                        NotificationManagerCompat.from(context).cancel(notifId)
                    }
                    org.pjstatecm.graceride.widget.WidgetSnapshotStore.markReadyLocally(context, rideId, status)
                    org.pjstatecm.graceride.widget.refreshGraceRideWidget(context)
                    app.container.auth.refresh()
                }
            } catch (_: Exception) {
                if (notifId > 0 && NotificationHelper.hasPostPermission(context)) {
                    val lang = app.container.lang
                    NotificationHelper.ensureChannels(context, lang)
                    val ctx = NotificationHelper.localizedContext(context, lang)
                    val tapIntent = Intent(context, MainActivity::class.java).apply {
                        flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
                        putExtra(NotificationHelper.EXTRA_NOTIF_LINK, "/rides/$rideId")
                        putExtra(NotificationHelper.EXTRA_NOTIF_TOKEN, NotificationHelper.internalToken(context))
                    }
                    val tapPending = PendingIntent.getActivity(
                        context,
                        ((notifId.coerceAtLeast(1) * 10L + 5L) and 0x3FFFFFFFL).toInt(),
                        tapIntent,
                        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
                    )
                    val publicVersion = NotificationCompat.Builder(context, NotificationHelper.CHANNEL_RIDES_URGENT)
                        .setSmallIcon(R.drawable.ic_launcher_monochrome)
                        .setContentTitle(ctx.getString(R.string.app_name))
                        .setContentText(ctx.getString(R.string.notif_locked_generic))
                        .setAutoCancel(true)
                        .build()
                    val failed = NotificationCompat.Builder(context, NotificationHelper.CHANNEL_RIDES_URGENT)
                        .setSmallIcon(R.drawable.ic_launcher_monochrome)
                        .setContentTitle(ctx.getString(R.string.app_name))
                        .setContentText(ctx.getString(R.string.notif_readiness_failed))
                        .setContentIntent(tapPending)
                        .setCategory(NotificationCompat.CATEGORY_REMINDER)
                        .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
                        .setPublicVersion(publicVersion)
                        .setAutoCancel(true)
                        .build()
                    try {
                        NotificationManagerCompat.from(context).notify(notifId, failed)
                    } catch (_: SecurityException) {
                    }
                }
            } finally {
                pending.finish()
            }
        }
    }

    companion object {
        private const val EXTRA_RIDE_ID = "ride_id"
        private const val EXTRA_STATUS = "readiness_status"
        private const val EXTRA_NOTIF_ID = "notif_id"
        private const val EXTRA_NOTIF_ITEM_ID = "notif_item_id"

        fun createIntent(
            context: Context,
            rideId: Long,
            status: String,
            notifId: Int,
            notifItemId: Long = -1L,
        ): Intent =
            Intent(context, ReadinessActionReceiver::class.java).apply {
                putExtra(EXTRA_RIDE_ID, rideId)
                putExtra(EXTRA_STATUS, status)
                putExtra(EXTRA_NOTIF_ID, notifId)
                putExtra(EXTRA_NOTIF_ITEM_ID, notifItemId)
            }
    }
}
