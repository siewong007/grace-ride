package org.pjstatecm.graceride.ui

object Routes {
    const val LOGIN = "login"
    const val UNLOCK = "unlock"
    const val PENDING = "pending"
    const val TERMS = "terms"
    const val HOME = "home"
    const val RIDES = "rides"
    const val RIDE_DETAIL = "ride/{id}?new={new}"
    const val RIDE_NEW = "ride-new?event={eventId}"
    const val EVENTS = "events"
    const val EVENT_DETAIL = "event/{id}"
    const val COSTS = "costs"
    const val HISTORY = "history"
    const val PROFILE = "profile"
    const val NOTIFICATIONS = "notifications"

    fun rideDetail(id: Long, justBooked: Boolean = false) =
        if (justBooked) "ride/$id?new=1" else "ride/$id"
    fun eventDetail(id: Long) = "event/$id"
    fun rideNew(eventId: Long? = null) = if (eventId == null) "ride-new" else "ride-new?event=$eventId"

    val tabs = setOf(HOME, RIDES, EVENTS, PROFILE)
}
