package org.pjstatecm.graceride.notifications

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.app.RemoteInput
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import org.pjstatecm.graceride.GraceRideApp
import org.pjstatecm.graceride.R

/**
 * Handles inline direct replies from Android chat notifications (`RemoteInput`) and posts the
 * message straight to `/api/rides/{id}/messages` without opening the app.
 */
class ChatReplyReceiver : BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val remoteResults = RemoteInput.getResultsFromIntent(intent)
        val reply = remoteResults
            ?.getCharSequence(KEY_CHAT_REPLY)
            ?.toString()
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
        val rideId = intent.getLongExtra(EXTRA_RIDE_ID, -1L)
        val notifId = intent.getIntExtra(EXTRA_NOTIF_ID, -1)
        val notifItemId = intent.getLongExtra(EXTRA_NOTIF_ITEM_ID, -1L)
        if (reply == null) {
            if (remoteResults != null && notifId > 0) {
                runCatching { NotificationManagerCompat.from(context).cancel(notifId) }
            }
            return
        }
        if (rideId <= 0L) return

        val app = context.applicationContext as? GraceRideApp ?: return
        val pending = goAsync()
        CoroutineScope(SupervisorJob() + Dispatchers.IO).launch {
            var tapPending: android.app.PendingIntent? = null
            var ctx = context
            try {
                withTimeout(7_500) {
                    val lang = app.container.lang
                    val hideLockDetails = app.container.secureStore.getString("notif_hide_lock_details") == "1"
                    NotificationHelper.ensureChannels(context, lang)
                    ctx = NotificationHelper.localizedContext(context, lang)
                    val tapIntent = Intent(context, org.pjstatecm.graceride.MainActivity::class.java).apply {
                        flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
                        putExtra(NotificationHelper.EXTRA_NOTIF_LINK, "/rides/$rideId")
                        putExtra(NotificationHelper.EXTRA_NOTIF_TOKEN, NotificationHelper.internalToken(context))
                    }
                    tapPending = android.app.PendingIntent.getActivity(
                        context,
                        ((notifId.coerceAtLeast(1) * 10L + 4L) and 0x3FFFFFFFL).toInt(),
                        tapIntent,
                        android.app.PendingIntent.FLAG_UPDATE_CURRENT or android.app.PendingIntent.FLAG_IMMUTABLE,
                    )
                    app.container.rides.postMessage(rideId, reply.take(1000))
                    if (notifItemId > 0L) {
                        org.pjstatecm.graceride.data.api.runSuspendCatching {
                            app.container.community.readOne(notifItemId)
                        }
                    }
                    if (notifId > 0) {
                        val publicVersion = NotificationCompat.Builder(context, NotificationHelper.CHANNEL_CHAT)
                            .setSmallIcon(R.drawable.ic_launcher_monochrome)
                            .setContentTitle(ctx.getString(R.string.app_name))
                            .setContentText(ctx.getString(R.string.notif_locked_generic))
                            .setAutoCancel(true)
                            .build()
                        val builder = NotificationCompat.Builder(context, NotificationHelper.CHANNEL_CHAT)
                            .setSmallIcon(R.drawable.ic_launcher_monochrome)
                            .setContentTitle(ctx.getString(R.string.notif_reply_sent))
                            .setContentText(if (hideLockDetails) ctx.getString(R.string.notif_locked_generic) else reply)
                            .setContentIntent(tapPending)
                            .setCategory(NotificationCompat.CATEGORY_MESSAGE)
                            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
                            .setPublicVersion(publicVersion)
                            .setAutoCancel(true)
                        try {
                            NotificationManagerCompat.from(context).notify(notifId, builder.build())
                        } catch (_: SecurityException) {
                        }
                    }
                }
            } catch (_: Exception) {
                if (notifId > 0) {
                    val lang = app.container.lang
                    NotificationHelper.ensureChannels(context, lang)
                    ctx = NotificationHelper.localizedContext(context, lang)
                    val publicVersion = NotificationCompat.Builder(context, NotificationHelper.CHANNEL_CHAT)
                        .setSmallIcon(R.drawable.ic_launcher_monochrome)
                        .setContentTitle(ctx.getString(R.string.app_name))
                        .setContentText(ctx.getString(R.string.notif_locked_generic))
                        .setAutoCancel(true)
                        .build()
                    val failed = NotificationCompat.Builder(context, NotificationHelper.CHANNEL_CHAT)
                        .setSmallIcon(R.drawable.ic_launcher_monochrome)
                        .setContentTitle(ctx.getString(R.string.app_name))
                        .setContentText(ctx.getString(R.string.notif_reply_failed))
                        .apply { tapPending?.let { setContentIntent(it) } }
                        .setCategory(NotificationCompat.CATEGORY_MESSAGE)
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
        const val KEY_CHAT_REPLY = "key_chat_reply"
        private const val EXTRA_RIDE_ID = "ride_id"
        private const val EXTRA_NOTIF_ID = "notif_id"
        private const val EXTRA_NOTIF_ITEM_ID = "notif_item_id"

        fun createIntent(
            context: Context,
            rideId: Long,
            notifId: Int,
            notifItemId: Long = -1L,
        ): Intent =
            Intent(context, ChatReplyReceiver::class.java).apply {
                putExtra(EXTRA_RIDE_ID, rideId)
                putExtra(EXTRA_NOTIF_ID, notifId)
                putExtra(EXTRA_NOTIF_ITEM_ID, notifItemId)
            }
    }
}
