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
import android.provider.Settings
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.app.Person
import androidx.core.app.RemoteInput
import androidx.core.content.ContextCompat
import androidx.core.content.edit
import org.pjstatecm.graceride.MainActivity
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.NotifLinkRouter
import java.util.Locale

object NotificationHelper {
    const val CHANNEL_RIDES_URGENT = "rides_urgent"
    const val CHANNEL_RIDES_UPDATES = "rides_updates"
    const val CHANNEL_CHAT = "chat_messages"
    const val CHANNEL_COMMUNITY = "community"
    const val CHANNEL_LIVE_LOCATION = "live_trip_location"
    const val GROUP_NOTIFICATIONS = "org.pjstatecm.graceride.NOTIFICATIONS"
    const val GROUP_SUMMARY_ID = 900_001
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
                    prefs.edit { putString(KEY_INTERNAL_TOKEN, generated) }
                }
            cachedToken = tok
            tok
        }
    }

    @Volatile
    private var lastRegisteredLang: String? = null

    internal fun localizedContext(context: Context, lang: String?): Context {
        val code = lang?.takeIf { it == "zh" || it == "en" } ?: return context
        val locale = Locale.forLanguageTag(code)
        val config = Configuration(context.resources.configuration).apply { setLocale(locale) }
        return context.createConfigurationContext(config)
    }

    fun ensureChannels(context: Context, lang: String? = null) {
        val key = lang?.takeIf { it == "zh" || it == "en" } ?: Locale.getDefault().toLanguageTag()
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
                NotificationChannel(
                    CHANNEL_LIVE_LOCATION,
                    ctx.getString(R.string.live_location_notif_channel_name),
                    NotificationManager.IMPORTANCE_LOW,
                ).apply {
                    description = ctx.getString(R.string.live_location_notif_channel_desc)
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

    fun categoryLabelRes(category: String): Int? = when (category) {
        "ride" -> R.string.notif_cat_ride
        "event" -> R.string.notif_cat_event
        "cost" -> R.string.notif_cat_cost
        "help" -> R.string.notif_cat_help
        "family" -> R.string.notif_cat_family
        "admin" -> R.string.notif_cat_admin
        "announcement" -> R.string.notif_cat_announcement
        "chat" -> R.string.notif_cat_chat
        "general" -> R.string.notif_cat_general
        else -> null
    }

    internal fun androidCategoryFor(category: String, eventType: String, link: String?): String = when {
        category == "chat" || eventType == "trip_chat" -> NotificationCompat.CATEGORY_MESSAGE
        eventType == "ride_reminder" || link?.contains("action=readiness") == true ->
            NotificationCompat.CATEGORY_REMINDER
        category == "event" -> NotificationCompat.CATEGORY_EVENT
        else -> NotificationCompat.CATEGORY_STATUS
    }

    internal fun priorityFor(channelId: String): Int = when (channelId) {
        CHANNEL_RIDES_URGENT -> NotificationCompat.PRIORITY_HIGH
        CHANNEL_COMMUNITY -> NotificationCompat.PRIORITY_LOW
        else -> NotificationCompat.PRIORITY_DEFAULT
    }

    internal fun subTextFor(ctx: Context, item: NotificationItem): String? {
        val catLabel = categoryLabelRes(item.category)?.let { ctx.getString(it) }
        return when {
            item.priority == "urgent" && catLabel != null ->
                ctx.getString(R.string.notif_subtext_urgent_cat, catLabel)
            item.priority == "urgent" -> ctx.getString(R.string.notif_urgent)
            else -> catLabel
        }
    }

    internal fun parseChatSenderAndText(item: NotificationItem, fallbackSender: String): Pair<String, String> {
        val idx = item.body.indexOf(": ")
        if (idx in 1..60) {
            val sender = item.body.substring(0, idx).trim()
            val text = item.body.substring(idx + 2).trim()
            if (sender.isNotEmpty() && text.isNotEmpty()) {
                return sender to text
            }
        }
        return item.title.ifBlank { fallbackSender } to item.body
    }

    fun hasPostPermission(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED

    fun areNotificationsEnabled(context: Context): Boolean =
        hasPostPermission(context) && NotificationManagerCompat.from(context).areNotificationsEnabled()

    fun openNotificationSettingsIntent(context: Context, channelId: String? = null): Intent =
        if (channelId != null) {
            Intent(Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS).apply {
                putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
                putExtra(Settings.EXTRA_CHANNEL_ID, channelId)
            }
        } else {
            Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).apply {
                putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
            }
        }

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
        val link = item.link?.takeIf { it.isNotBlank() } ?: "/notifications"
        val createdMs = KlTime.parseEpochMillis(item.createdAt)

        val tapIntent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra(EXTRA_NOTIF_LINK, link)
            putExtra(EXTRA_NOTIF_TOKEN, internalToken(context))
        }
        val tapPending = PendingIntent.getActivity(
            context,
            notifId,
            tapIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val visibleTitle = if (hideLockDetails) {
            ctx.getString(R.string.app_name)
        } else {
            item.title.ifBlank { ctx.getString(R.string.app_name) }
        }
        val visibleBody = if (hideLockDetails) {
            ctx.getString(R.string.notif_locked_generic)
        } else {
            item.body
        }

        val publicVersion = NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(ctx.getString(R.string.app_name))
            .setContentText(ctx.getString(R.string.notif_locked_generic))
            .setAutoCancel(true)
            .build()

        val builder = NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(visibleTitle)
            .setContentText(visibleBody)
            .setStyle(NotificationCompat.BigTextStyle().bigText(visibleBody))
            .setContentIntent(tapPending)
            .setAutoCancel(true)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setPublicVersion(publicVersion)
            .setCategory(androidCategoryFor(item.category, item.eventType, link))
            .setPriority(priorityFor(channelId))
            .setGroup(GROUP_NOTIFICATIONS)
            .setGroupAlertBehavior(NotificationCompat.GROUP_ALERT_CHILDREN)

        if (createdMs != null) {
            builder.setWhen(createdMs).setShowWhen(true)
        }
        if (!hideLockDetails) {
            subTextFor(ctx, item)?.let { builder.setSubText(it) }
        }

        val rideId = NotifLinkRouter.extractRideId(link)
        val baseReq = ((item.id * 31L) and 0x3FFFFFFFL).toInt()
        if (!hideLockDetails && rideId != null && (item.category == "chat" || item.eventType == "trip_chat")) {
            val mePerson = Person.Builder()
                .setName(ctx.getString(R.string.map_you_label))
                .build()
            val (senderName, messageText) = parseChatSenderAndText(item, ctx.getString(R.string.app_name))
            val senderPerson = Person.Builder()
                .setName(senderName)
                .build()
            val convTitle = item.title.ifBlank { ctx.getString(R.string.notif_channel_chat_name) }
            builder.setStyle(
                NotificationCompat.MessagingStyle(mePerson)
                    .setConversationTitle(convTitle)
                    .addMessage(messageText, createdMs ?: System.currentTimeMillis(), senderPerson),
            )
            val replyInput = RemoteInput.Builder(ChatReplyReceiver.KEY_CHAT_REPLY)
                .setLabel(ctx.getString(R.string.notif_reply_hint))
                .build()
            val mutableFlag = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
            val replyPending = PendingIntent.getBroadcast(
                context,
                (baseReq + 3) and 0x3FFFFFFF,
                ChatReplyReceiver.createIntent(context, rideId, notifId, item.id),
                PendingIntent.FLAG_UPDATE_CURRENT or mutableFlag,
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
        if (!hideLockDetails && rideId != null && link.contains("action=readiness")) {
            val readyIntent = ReadinessActionReceiver.createIntent(context, rideId, "ready", notifId, item.id)
            val readyPending = PendingIntent.getBroadcast(
                context,
                (baseReq + 1) and 0x3FFFFFFF,
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

            val late5Intent = ReadinessActionReceiver.createIntent(context, rideId, "delayed_5m", notifId, item.id)
            val late5Pending = PendingIntent.getBroadcast(
                context,
                (baseReq + 2) and 0x3FFFFFFF,
                late5Intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            builder.addAction(
                NotificationCompat.Action.Builder(
                    0,
                    ctx.getString(R.string.notif_action_delayed_5m),
                    late5Pending,
                ).setAuthenticationRequired(true).build(),
            )

            val late10Intent = ReadinessActionReceiver.createIntent(context, rideId, "delayed_10m", notifId, item.id)
            val late10Pending = PendingIntent.getBroadcast(
                context,
                (baseReq + 4) and 0x3FFFFFFF,
                late10Intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            builder.addAction(
                NotificationCompat.Action.Builder(
                    0,
                    ctx.getString(R.string.notif_action_delayed_10m),
                    late10Pending,
                ).setAuthenticationRequired(true).build(),
            )
        }

        try {
            NotificationManagerCompat.from(context).notify(notifId, builder.build())
        } catch (_: SecurityException) {
            // Permission revoked concurrently.
        }
    }

    fun showGroupSummary(
        context: Context,
        items: List<NotificationItem>,
        lang: String? = null,
        hideLockDetails: Boolean = false,
    ) {
        if (items.size < 2 || !hasPostPermission(context)) return
        ensureChannels(context, lang)
        val ctx = localizedContext(context, lang)
        val channelId = if (items.any { it.priority == "urgent" }) {
            CHANNEL_RIDES_URGENT
        } else {
            CHANNEL_RIDES_UPDATES
        }
        val tapIntent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra(EXTRA_NOTIF_LINK, "/notifications")
            putExtra(EXTRA_NOTIF_TOKEN, internalToken(context))
        }
        val tapPending = PendingIntent.getActivity(
            context,
            GROUP_SUMMARY_ID,
            tapIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val summaryTitle = ctx.getString(R.string.notif_group_summary_title)
        val summaryText = ctx.getString(R.string.notif_group_summary_text, items.size)
        val lockedText = ctx.getString(R.string.notif_locked_generic)

        val inbox = NotificationCompat.InboxStyle()
            .setBigContentTitle(summaryTitle)
            .setSummaryText(summaryText)
        if (hideLockDetails) {
            inbox.addLine(lockedText)
        } else {
            items.takeLast(5).asReversed().forEach { n ->
                inbox.addLine(n.title.ifBlank { ctx.getString(R.string.app_name) })
            }
        }

        val publicVersion = NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(ctx.getString(R.string.app_name))
            .setContentText(lockedText)
            .setAutoCancel(true)
            .build()

        val summaryNotif = NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(summaryTitle)
            .setContentText(if (hideLockDetails) lockedText else summaryText)
            .setStyle(inbox)
            .setContentIntent(tapPending)
            .setGroup(GROUP_NOTIFICATIONS)
            .setGroupSummary(true)
            .setGroupAlertBehavior(NotificationCompat.GROUP_ALERT_CHILDREN)
            .setAutoCancel(true)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setPublicVersion(publicVersion)
            .setPriority(priorityFor(channelId))
            .build()

        try {
            NotificationManagerCompat.from(context).notify(GROUP_SUMMARY_ID, summaryNotif)
        } catch (_: SecurityException) {
            // Permission revoked concurrently.
        }
    }
}
