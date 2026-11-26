package org.pjstatecm.graceride.notifications

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.os.Build
import java.util.Locale
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
    const val EXTRA_NOTIF_TOKEN = "notif_token"
    private const val GUARD_PREFS = "gr_intent_guard"
    private const val KEY_INTERNAL_TOKEN = "internal_link_token"

    @Volatile
    private var cachedToken: String? = null

    fun internalToken(context: Context): String {
        cachedToken?.let { return it }
        return synchronized(this) {
            cachedToken?.let { return@synchronized it }
            val prefs = context.applicationContext.getSharedPreferences(GUARD_PREFS, Context.MODE_PRIVATE)
            val tok = prefs.getString(KEY_INTERNAL_TOKEN, null)?.takeIf { it.length >= 16 }
                ?: java.util.UUID.randomUUID().toString().also { generated ->
                    prefs.edit().putString(KEY_INTERNAL_TOKEN, generated).apply()
                }
            cachedToken = tok
            tok
        }
    }

    @Volatile
    private var lastRegisteredLang: String? = null

    private fun localizedContext(context: Context, lang: String?): Context {
        val code = lang?.takeIf { it == "zh" || it == "en" } ?: return context
        val locale = Locale.forLanguageTag(code)
        val config = Configuration(context.resources.configuration).apply { setLocale(locale) }
        return context.createConfigurationContext(config)
    }

    fun ensureChannels(context: Context, lang: String? = null) {
        val key = lang?.takeIf { it == "zh" || it == "en" } ?: "default"
        if (lastRegisteredLang == key) return
        synchronized(this) {
            if (lastRegisteredLang == key) return
            val manager = context.getSystemService(NotificationManager::class.java) ?: return
            val ctx = localizedContext(context, lang)
            val channels = listOf(
                NotificationChannel(
                    CHANNEL_RIDES_URGENT,
                    ctx.getString(R.string.notif_channel_rides_urgent_name),
                    NotificationManager.IMPORTANCE_HIGH,
                ).apply {
                    description = ctx.getString(R.string.notif_channel_rides_urgent_desc)
                },
                NotificationChannel(
                    CHANNEL_RIDES_UPDATES,
                    ctx.getString(R.string.notif_channel_rides_updates_name),
                    NotificationManager.IMPORTANCE_DEFAULT,
                ).apply {
                    description = ctx.getString(R.string.notif_channel_rides_updates_desc)
                },
                NotificationChannel(
                    CHANNEL_CHAT,
                    ctx.getString(R.string.notif_channel_chat_name),
                    NotificationManager.IMPORTANCE_DEFAULT,
                ).apply {
                    description = ctx.getString(R.string.notif_channel_chat_desc)
                },
                NotificationChannel(
                    CHANNEL_COMMUNITY,
                    ctx.getString(R.string.notif_channel_community_name),
                    NotificationManager.IMPORTANCE_LOW,
                ).apply {
                    description = ctx.getString(R.string.notif_channel_community_desc)
                },
            )
            manager.createNotificationChannels(channels)
            lastRegisteredLang = key
        }
    }

    fun channelFor(category: String, eventType: String, priority: String): String = when {
        priority == "urgent" -> CHANNEL_RIDES_URGENT
        category == "chat" || eventType == "trip_chat" -> CHANNEL_CHAT
        category == "ride" -> CHANNEL_RIDES_UPDATES
        else -> CHANNEL_COMMUNITY
    }

    fun hasPostPermission(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED

    fun showNotification(
        context: Context,
        item: NotificationItem,
        lang: String? = null,
        hideLockDetails: Boolean = false,
    ) {
        if (!hasPostPermission(context)) return
        ensureChannels(context, lang)
        val ctx = localizedContext(context, lang)

        val channelId = channelFor(item.category, item.eventType, item.priority)
        val notifId = (item.id and 0x7FFFFFFFL).toInt().coerceAtLeast(1)

        val tapIntent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra(EXTRA_NOTIF_LINK, item.link ?: "/notifications")
            putExtra(EXTRA_NOTIF_TOKEN, internalToken(context))
        }
        val tapPending = PendingIntent.getActivity(
            context,
            notifId,
            tapIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val builder = NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(item.title.ifBlank { ctx.getString(R.string.app_name) })
            .setContentText(item.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(item.body))
            .setContentIntent(tapPending)
            .setAutoCancel(true)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setPriority(
                if (channelId == CHANNEL_RIDES_URGENT) NotificationCompat.PRIORITY_HIGH
                else NotificationCompat.PRIORITY_DEFAULT
            )

        if (hideLockDetails) {
            val publicVersion = NotificationCompat.Builder(context, channelId)
                .setSmallIcon(R.drawable.ic_launcher_monochrome)
                .setContentTitle(ctx.getString(R.string.app_name))
                .setContentText(ctx.getString(R.string.notif_locked_generic))
                .setAutoCancel(true)
                .build()
            builder.setPublicVersion(publicVersion)
        }

        val rideId = NotifLinkRouter.extractRideId(item.link)
        if (!hideLockDetails && rideId != null && (item.category == "chat" || item.eventType == "trip_chat")) {
            val senderName = item.title.ifBlank { ctx.getString(R.string.app_name) }
            val sender = androidx.core.app.Person.Builder()
                .setName(senderName)
                .build()
            builder.setStyle(
                NotificationCompat.MessagingStyle(sender)
                    .addMessage(item.body, System.currentTimeMillis(), sender),
            )
            val replyInput = androidx.core.app.RemoteInput.Builder(ChatReplyReceiver.KEY_CHAT_REPLY)
                .setLabel(ctx.getString(R.string.notif_reply_hint))
                .build()
            val replyPending = PendingIntent.getBroadcast(
                context,
                notifId * 10 + 3,
                ChatReplyReceiver.createIntent(context, rideId, notifId),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
            )
            builder.addAction(
                NotificationCompat.Action.Builder(
                    R.drawable.ic_launcher_monochrome,
                    ctx.getString(R.string.notif_action_reply),
                    replyPending,
                ).addRemoteInput(replyInput)
                    .setAllowGeneratedReplies(true)
                    .setAuthenticationRequired(true)
                    .build(),
            )
        }
        if (rideId != null && item.link?.contains("action=readiness") == true) {
            val baseReq = (notifId * 31) and 0x3FFFFFFF
            val readyIntent = ReadinessActionReceiver.createIntent(context, rideId, "ready", notifId)
            val readyPending = PendingIntent.getBroadcast(
                context,
                baseReq + 1,
                readyIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            builder.addAction(
                NotificationCompat.Action.Builder(
                    0,
                    ctx.getString(R.string.notif_action_ready),
                    readyPending,
                ).setAuthenticationRequired(true).build(),
            )

            val lateIntent = ReadinessActionReceiver.createIntent(context, rideId, "delayed_5m", notifId)
            val latePending = PendingIntent.getBroadcast(
                context,
                baseReq + 2,
                lateIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            builder.addAction(
                NotificationCompat.Action.Builder(
                    0,
                    ctx.getString(R.string.notif_action_delayed_5m),
                    latePending,
                ).setAuthenticationRequired(true).build(),
            )
        }

        try {
            NotificationManagerCompat.from(context).notify(notifId, builder.build())
        } catch (_: SecurityException) {
            // Permission revoked concurrently.
        }
    }
}
