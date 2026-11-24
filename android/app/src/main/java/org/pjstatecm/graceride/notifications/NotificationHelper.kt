package org.pjstatecm.graceride.notifications

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import org.pjstatecm.graceride.MainActivity
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.ui.NotifLinkRouter

object NotificationHelper {
    const val CHANNEL_RIDES_URGENT = "rides_urgent"
    const val CHANNEL_RIDES_UPDATES = "rides_updates"
    const val CHANNEL_CHAT = "chat_messages"
    const val CHANNEL_COMMUNITY = "community"
    const val EXTRA_NOTIF_LINK = "notif_link"

    fun ensureChannels(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val channels = listOf(
            NotificationChannel(
                CHANNEL_RIDES_URGENT,
                context.getString(R.string.notif_channel_rides_urgent_name),
                NotificationManager.IMPORTANCE_HIGH,
            ).apply {
                description = context.getString(R.string.notif_channel_rides_urgent_desc)
            },
            NotificationChannel(
                CHANNEL_RIDES_UPDATES,
                context.getString(R.string.notif_channel_rides_updates_name),
                NotificationManager.IMPORTANCE_DEFAULT,
            ).apply {
                description = context.getString(R.string.notif_channel_rides_updates_desc)
            },
            NotificationChannel(
                CHANNEL_CHAT,
                context.getString(R.string.notif_channel_chat_name),
                NotificationManager.IMPORTANCE_DEFAULT,
            ).apply {
                description = context.getString(R.string.notif_channel_chat_desc)
            },
            NotificationChannel(
                CHANNEL_COMMUNITY,
                context.getString(R.string.notif_channel_community_name),
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = context.getString(R.string.notif_channel_community_desc)
            },
        )
        manager.createNotificationChannels(channels)
    }

    fun channelFor(category: String, eventType: String, priority: String): String = when {
        priority == "urgent" -> CHANNEL_RIDES_URGENT
        eventType == "trip_chat" -> CHANNEL_CHAT
        category == "ride" -> CHANNEL_RIDES_UPDATES
        else -> CHANNEL_COMMUNITY
    }

    fun hasPostPermission(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED

    fun showNotification(context: Context, item: NotificationItem) {
        if (!hasPostPermission(context)) return
        ensureChannels(context)

        val channelId = channelFor(item.category, item.eventType, item.priority)
        val notifId = (item.id and 0x7FFFFFFFL).toInt().coerceAtLeast(1)

        val tapIntent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra(EXTRA_NOTIF_LINK, item.link ?: "/notifications")
        }
        val tapPending = PendingIntent.getActivity(
            context,
            notifId,
            tapIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val builder = NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(item.title.ifBlank { context.getString(R.string.app_name) })
            .setContentText(item.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(item.body))
            .setContentIntent(tapPending)
            .setAutoCancel(true)
            .setPriority(
                if (channelId == CHANNEL_RIDES_URGENT) NotificationCompat.PRIORITY_HIGH
                else NotificationCompat.PRIORITY_DEFAULT
            )

        val rideId = NotifLinkRouter.extractRideId(item.link)
        if (rideId != null && item.link?.contains("action=readiness") == true) {
            val baseReq = (notifId * 31) and 0x3FFFFFFF
            val readyIntent = ReadinessActionReceiver.createIntent(context, rideId, "ready", notifId)
            val readyPending = PendingIntent.getBroadcast(
                context,
                baseReq + 1,
                readyIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            builder.addAction(0, context.getString(R.string.notif_action_ready), readyPending)

            val lateIntent = ReadinessActionReceiver.createIntent(context, rideId, "delayed_5m", notifId)
            val latePending = PendingIntent.getBroadcast(
                context,
                baseReq + 2,
                lateIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            builder.addAction(0, context.getString(R.string.notif_action_delayed_5m), latePending)
        }

        try {
            NotificationManagerCompat.from(context).notify(notifId, builder.build())
        } catch (_: SecurityException) {
            // Permission revoked concurrently.
        }
    }
}
