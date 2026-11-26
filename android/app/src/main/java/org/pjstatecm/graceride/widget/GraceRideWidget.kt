package org.pjstatecm.graceride.widget

import android.appwidget.AppWidgetManager
import android.content.Context
import android.content.Intent
import androidx.core.content.edit
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.action.actionSendBroadcast
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.appwidget.updateAll
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.color.ColorProvider
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.Serializable
import org.pjstatecm.graceride.GraceRideApp
import org.pjstatecm.graceride.MainActivity
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.notifications.ReadinessActionReceiver
import java.util.Locale

@Serializable
data class WidgetSummary(
    val signedIn: Boolean = false,
    val isRide: Boolean = false,
    val rideId: Long? = null,
    val eventId: Long? = null,
    val scheduledAt: String = "",
    val badge: String = "",
    val title: String = "",
    val subtitle: String = "",
    val showReadyButton: Boolean = false,
    val readiness: String? = null,
)

object WidgetSnapshotStore {
    private const val PREFS_NAME = "gr_widget_snapshot"
    private const val KEY_SUMMARY = "summary_v1"

    fun saveSummary(context: Context, summary: WidgetSummary) {
        val raw = apiJson.encodeToString(WidgetSummary.serializer(), summary)
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit {
                putString(KEY_SUMMARY, raw)
            }
    }

    fun saveFromHome(
        context: Context,
        signedIn: Boolean,
        nextRide: Ride?,
        nextEvent: EventItem?,
        hideLockDetails: Boolean = false,
    ) {
        if (!signedIn) {
            clear(context)
            return
        }
        val lang = (context.applicationContext as? GraceRideApp)?.container?.lang
        val ctx = NotificationHelper.localizedContext(context, lang)
        val locale = when (lang) {
            "zh" -> Locale.SIMPLIFIED_CHINESE
            "en" -> Locale.ENGLISH
            else -> Locale.getDefault()
        }
        val summary = when {
            nextRide != null -> {
                val timeLabel = KlTime.display(nextRide.pickupAt, locale)
                val dest = if (hideLockDetails) {
                    ctx.getString(R.string.widget_next_ride)
                } else {
                    nextRide.event?.title ?: nextRide.destination.ifBlank { nextRide.area.name }
                }
                WidgetSummary(
                    signedIn = true,
                    isRide = true,
                    rideId = nextRide.id,
                    scheduledAt = nextRide.pickupAt,
                    badge = ctx.getString(R.string.widget_next_ride),
                    title = dest,
                    subtitle = if (hideLockDetails) timeLabel else "$timeLabel · ${nextRide.statusLabel(ctx)}",
                    showReadyButton = !hideLockDetails &&
                        nextRide.viewer.isPassenger &&
                        nextRide.status == "confirmed" &&
                        nextRide.readinessStatus != "ready",
                    readiness = nextRide.readinessStatus,
                )
            }
            nextEvent != null -> {
                WidgetSummary(
                    signedIn = true,
                    isRide = false,
                    eventId = nextEvent.id,
                    scheduledAt = nextEvent.startsAt,
                    badge = ctx.getString(R.string.widget_next_event),
                    title = if (hideLockDetails) ctx.getString(R.string.widget_next_event) else nextEvent.title,
                    subtitle = if (hideLockDetails) {
                        KlTime.display(nextEvent.startsAt, locale)
                    } else {
                        "${KlTime.display(nextEvent.startsAt, locale)} · ${nextEvent.location}"
                    },
                )
            }
            else -> WidgetSummary(signedIn = true)
        }
        saveSummary(context, summary)
    }

    fun markReadyLocally(context: Context, rideId: Long, status: String) {
        val cur = load(context)
        if (!cur.signedIn || cur.rideId != rideId) return
        saveSummary(context, cur.copy(showReadyButton = false, readiness = status))
    }

    fun freshForDisplay(summary: WidgetSummary): WidgetSummary {
        if (!summary.signedIn || summary.scheduledAt.isBlank()) return summary
        val dt = KlTime.parse(summary.scheduledAt) ?: return summary
        val now = KlTime.now()
        return when {
            summary.isRide && now.isAfter(dt.plusHours(6)) -> WidgetSummary(signedIn = true)
            summary.isRide && now.isAfter(dt.plusHours(2)) -> summary.copy(showReadyButton = false)
            !summary.isRide && now.isAfter(dt.plusHours(12)) -> WidgetSummary(signedIn = true)
            else -> summary
        }
    }

    fun load(context: Context): WidgetSummary {
        val raw = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .getString(KEY_SUMMARY, null)
            ?: return WidgetSummary(signedIn = false)
        return runCatching { apiJson.decodeFromString(WidgetSummary.serializer(), raw) }
            .getOrDefault(WidgetSummary(signedIn = false))
    }

    fun clear(context: Context) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit {
                remove(KEY_SUMMARY)
            }
    }

    private fun Ride.statusLabel(ctx: Context): String =
        ctx.getString(RideStatus.labelRes(status))
}

class GraceRideWidget : GlanceAppWidget() {

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val summary = WidgetSnapshotStore.freshForDisplay(WidgetSnapshotStore.load(context))
        val lang = (context.applicationContext as? GraceRideApp)?.container?.lang
        val ctx = NotificationHelper.localizedContext(context, lang)
        provideContent {
            GlanceTheme {
                WidgetContent(context, ctx, summary)
            }
        }
    }

    @Composable
    private fun WidgetContent(baseContext: Context, ctx: Context, summary: WidgetSummary) {
        val openLink = when {
            !summary.signedIn -> null
            summary.rideId != null -> "/rides/${summary.rideId}"
            summary.eventId != null -> "/events/${summary.eventId}"
            else -> "/rides"
        }
        val launchIntent = Intent(baseContext, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            if (openLink != null) {
                putExtra(NotificationHelper.EXTRA_NOTIF_LINK, openLink)
                putExtra(NotificationHelper.EXTRA_NOTIF_TOKEN, NotificationHelper.internalToken(baseContext))
            }
        }
        val cardBg = ColorProvider(day = Color(0xFFFFFFFF), night = Color(0xFF141517))
        val inkPrimary = ColorProvider(day = Color(0xFF1A1A1A), night = Color(0xFFEDEDED))
        val inkSecondary = ColorProvider(day = Color(0xFF6B6B6B), night = Color(0xFFA3A3A3))
        val badgeBg = ColorProvider(day = Color(0xFFEEF3FB), night = Color(0xFF1A2A44))
        val badgeText = ColorProvider(day = Color(0xFF1F5DB4), night = Color(0xFF7AA7EE))
        val ctaBg = ColorProvider(day = Color(0xFF1F5DB4), night = Color(0xFF1F5DB4))
        val ctaText = ColorProvider(day = Color.White, night = Color.White)

        Column(
            modifier = GlanceModifier
                .fillMaxSize()
                .background(cardBg)
                .cornerRadius(16.dp)
                .clickable(actionStartActivity(launchIntent))
                .padding(14.dp),
            verticalAlignment = Alignment.Vertical.CenterVertically,
        ) {
            if (!summary.signedIn) {
                Text(
                    text = ctx.getString(R.string.app_name),
                    style = TextStyle(color = inkPrimary, fontSize = 15.sp, fontWeight = FontWeight.Bold),
                )
                Spacer(GlanceModifier.height(4.dp))
                Text(
                    text = ctx.getString(R.string.widget_signed_out),
                    style = TextStyle(color = inkSecondary, fontSize = 13.sp),
                )
            } else if (summary.title.isBlank()) {
                Text(
                    text = ctx.getString(R.string.widget_empty_title),
                    style = TextStyle(color = inkPrimary, fontSize = 15.sp, fontWeight = FontWeight.Bold),
                )
                Spacer(GlanceModifier.height(4.dp))
                Text(
                    text = ctx.getString(R.string.widget_empty_sub),
                    style = TextStyle(color = inkSecondary, fontSize = 13.sp),
                )
            } else {
                Row(
                    modifier = GlanceModifier.fillMaxWidth(),
                    verticalAlignment = Alignment.Vertical.CenterVertically,
                ) {
                    Text(
                        text = summary.badge.uppercase(),
                        style = TextStyle(color = badgeText, fontSize = 11.sp, fontWeight = FontWeight.Bold),
                        modifier = GlanceModifier
                            .background(badgeBg)
                            .cornerRadius(8.dp)
                            .padding(horizontal = 8.dp, vertical = 3.dp),
                    )
                    Spacer(GlanceModifier.width(8.dp))
                    Text(
                        text = ctx.getString(R.string.app_name),
                        style = TextStyle(color = inkSecondary, fontSize = 11.sp, fontWeight = FontWeight.Medium),
                    )
                }
                Spacer(GlanceModifier.height(6.dp))
                Text(
                    text = summary.title,
                    style = TextStyle(color = inkPrimary, fontSize = 16.sp, fontWeight = FontWeight.Bold),
                    maxLines = 1,
                )
                if (summary.subtitle.isNotBlank()) {
                    Spacer(GlanceModifier.height(2.dp))
                    Text(
                        text = summary.subtitle,
                        style = TextStyle(color = inkSecondary, fontSize = 13.sp),
                        maxLines = 2,
                    )
                }
                if (summary.showReadyButton && summary.rideId != null) {
                    Spacer(GlanceModifier.height(8.dp))
                    val readyIntent = ReadinessActionReceiver.createIntent(
                        context = baseContext,
                        rideId = summary.rideId,
                        status = "ready",
                        notifId = 0,
                    )
                    Text(
                        text = ctx.getString(R.string.widget_action_ready),
                        style = TextStyle(color = ctaText, fontSize = 12.sp, fontWeight = FontWeight.Bold),
                        modifier = GlanceModifier
                            .background(ctaBg)
                            .cornerRadius(10.dp)
                            .clickable(actionSendBroadcast(readyIntent))
                            .padding(horizontal = 12.dp, vertical = 6.dp),
                    )
                }
            }
        }
    }
}

class GraceRideWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = GraceRideWidget()

    override fun onUpdate(context: Context, appWidgetManager: AppWidgetManager, appWidgetIds: IntArray) {
        super.onUpdate(context, appWidgetManager, appWidgetIds)
    }
}

suspend fun refreshGraceRideWidget(context: Context) {
    try {
        GraceRideWidget().updateAll(context)
    } catch (ce: CancellationException) {
        throw ce
    } catch (_: Exception) {
    }
}
