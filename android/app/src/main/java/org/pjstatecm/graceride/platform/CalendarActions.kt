package org.pjstatecm.graceride.platform

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.provider.CalendarContract
import org.pjstatecm.graceride.domain.KlTime

/** Opens the system calendar editor pre-filled with the Grace Ride event (zero calendar permissions needed). */
fun addEventToCalendar(
    context: Context,
    title: String,
    startsAt: String,
    location: String,
    description: String,
): Boolean {
    val dt = KlTime.parse(startsAt) ?: return false
    val startMs = dt.atZone(KlTime.zone).toInstant().toEpochMilli()
    val endMs = startMs + 2 * 60 * 60 * 1000L
    val intent = Intent(Intent.ACTION_INSERT).apply {
        data = CalendarContract.Events.CONTENT_URI
        putExtra(CalendarContract.Events.TITLE, title)
        putExtra(CalendarContract.EXTRA_EVENT_BEGIN_TIME, startMs)
        putExtra(CalendarContract.EXTRA_EVENT_END_TIME, endMs)
        putExtra(CalendarContract.Events.EVENT_TIMEZONE, KlTime.zone.id)
        if (location.isNotBlank()) putExtra(CalendarContract.Events.EVENT_LOCATION, location)
        if (description.isNotBlank()) putExtra(CalendarContract.Events.DESCRIPTION, description)
        if (context !is Activity) addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    return try {
        context.startActivity(intent)
        true
    } catch (_: ActivityNotFoundException) {
        false
    } catch (_: SecurityException) {
        false
    }
}
