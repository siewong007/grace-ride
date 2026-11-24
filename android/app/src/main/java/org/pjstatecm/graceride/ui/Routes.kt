package org.pjstatecm.graceride.ui

object Routes {
    const val LOGIN = "login"
    const val UNLOCK = "unlock"
    const val PENDING = "pending"
    const val TERMS = "terms"
    const val HOME = "home"
    const val RIDES = "rides"
    const val RIDE_DETAIL = "ride/{id}?new={new}"
    const val RIDE_NEW = "ride-new?event={eventId}&proxy={proxy}"
    const val EVENTS = "events"
    const val EVENT_DETAIL = "event/{id}"
    const val COSTS = "costs"
    const val HISTORY = "history"
    const val PROFILE = "profile"
    const val DRIVER_SETTINGS = "driver-settings"
    const val PROXY_RIDERS = "proxy-riders"
    const val SAVED_ADDRESSES = "saved-addresses"
    const val NOTIFICATIONS = "notifications"
    const val STUDIO = "studio?shape={shape}&color={color}&face={face}"

    fun studio(shape: String? = null, color: String? = null, face: String? = null): String = buildString {
        append("studio")
        val params = mutableListOf<String>()
        if (!shape.isNullOrBlank()) params.add("shape=$shape")
        if (!color.isNullOrBlank()) params.add("color=$color")
        if (!face.isNullOrBlank()) params.add("face=$face")
        if (params.isNotEmpty()) {
            append("?")
            append(params.joinToString("&"))
        }
    }

    fun rideDetail(id: Long, justBooked: Boolean = false) =
        if (justBooked) "ride/$id?new=1" else "ride/$id"
    fun eventDetail(id: Long) = "event/$id"
    fun rideNew(eventId: Long? = null, proxy: Boolean = false) = buildString {
        append("ride-new")
        val params = mutableListOf<String>()
        if (eventId != null) params.add("event=$eventId")
        if (proxy) params.add("proxy=1")
        if (params.isNotEmpty()) {
            append("?")
            append(params.joinToString("&"))
        }
    }

    val tabs = setOf(HOME, EVENTS, PROFILE)
}
