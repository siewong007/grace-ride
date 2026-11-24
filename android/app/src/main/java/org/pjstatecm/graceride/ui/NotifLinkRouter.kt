package org.pjstatecm.graceride.ui

sealed class NotifDestination {
    data class Native(val route: String) : NotifDestination()
    data class Web(val path: String) : NotifDestination()
    data object None : NotifDestination()
}

object NotifLinkRouter {
    private val rideDetailRe = Regex("^/rides/(\\d+)$")
    private val eventDetailRe = Regex("^/events/(\\d+)$")

    fun extractRideId(link: String?): Long? {
        val clean = link?.trim()?.substringBefore('?')?.substringBefore('#') ?: return null
        return rideDetailRe.find(clean)?.groupValues?.get(1)?.toLongOrNull()
    }

    fun resolve(link: String?): NotifDestination {
        val raw = link?.trim() ?: return NotifDestination.None
        if (raw.isEmpty() || !raw.startsWith("/") || raw.startsWith("//") || raw.startsWith("/api/")) {
            return NotifDestination.None
        }
        val clean = raw.substringBefore('?').substringBefore('#')
        if (clean.contains(':')) return NotifDestination.None

        rideDetailRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.rideDetail(id))
        }
        eventDetailRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.eventDetail(id))
        }

        return when {
            clean == "/rides" -> NotifDestination.Native(Routes.RIDES)
            clean == "/events" -> NotifDestination.Native(Routes.EVENTS)
            clean == "/costs" || clean == "/settlements" || clean.startsWith("/settlements/") ->
                NotifDestination.Native(Routes.COSTS)
            clean == "/history" -> NotifDestination.Native(Routes.HISTORY)
            clean == "/me" || clean == "/profile" -> NotifDestination.Native(Routes.PROFILE)
            clean == "/notifications" -> NotifDestination.Native(Routes.NOTIFICATIONS)
            clean == "/driver" -> NotifDestination.Native(Routes.DRIVER_SETTINGS)
            else -> NotifDestination.Web(raw)
        }
    }
}
