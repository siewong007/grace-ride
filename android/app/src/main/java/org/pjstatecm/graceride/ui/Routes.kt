package org.pjstatecm.graceride.ui

import androidx.navigation.NavHostController
import kotlinx.serialization.Serializable

/**
 * Type-safe `@Serializable` navigation graph destinations paired with legacy string route helpers
 * for compatibility with [NotifLinkRouter] and deep links.
 */
@Serializable
sealed interface AppRoute {
    @Serializable data object Login : AppRoute
    @Serializable data object Unlock : AppRoute
    @Serializable data object Pending : AppRoute
    @Serializable data object Terms : AppRoute
    @Serializable data object Home : AppRoute
    @Serializable data object Rides : AppRoute
    @Serializable data class RideDetail(val id: Long, val justBooked: Boolean = false) : AppRoute
    @Serializable data class RideNew(val eventId: Long? = null, val proxy: Boolean = false) : AppRoute
    @Serializable data object Events : AppRoute
    @Serializable data class EventDetail(val id: Long, val checkinCode: String? = null) : AppRoute
    @Serializable data object Costs : AppRoute
    @Serializable data object History : AppRoute
    @Serializable data object Profile : AppRoute
    @Serializable data object DriverSettings : AppRoute
    @Serializable data object ProxyRiders : AppRoute
    @Serializable data object SavedAddresses : AppRoute
    @Serializable data object PersonalDetails : AppRoute
    @Serializable data object AppearanceSettings : AppRoute
    @Serializable data object NotificationSettings : AppRoute
    @Serializable data object SecuritySettings : AppRoute
    @Serializable data object Notifications : AppRoute
    @Serializable data object DriveHub : AppRoute
    @Serializable data class Studio(
        val shape: String? = null,
        val color: String? = null,
        val face: String? = null,
    ) : AppRoute

    companion object {
        fun fromLegacyRoute(route: String): AppRoute? {
            val clean = route.substringBefore('?').trim()
            val query = route.substringAfter('?', "")
            val params = if (query.isBlank()) {
                emptyMap()
            } else {
                query.split('&').mapNotNull { kv ->
                    val idx = kv.indexOf('=')
                    if (idx <= 0) null else kv.substring(0, idx) to decodeQueryComponent(kv.substring(idx + 1))
                }.toMap()
            }
            return when {
                clean == Routes.LOGIN -> Login
                clean == Routes.UNLOCK -> Unlock
                clean == Routes.PENDING -> Pending
                clean == Routes.TERMS -> Terms
                clean == Routes.HOME -> Home
                clean == Routes.RIDES -> Rides
                clean == Routes.EVENTS -> Events
                clean == Routes.COSTS -> Costs
                clean == Routes.HISTORY -> History
                clean == Routes.PROFILE -> Profile
                clean == Routes.DRIVER_SETTINGS -> DriverSettings
                clean == Routes.PROXY_RIDERS -> ProxyRiders
                clean == Routes.SAVED_ADDRESSES -> SavedAddresses
                clean == Routes.PERSONAL_DETAILS -> PersonalDetails
                clean == Routes.APPEARANCE_SETTINGS -> AppearanceSettings
                clean == Routes.NOTIFICATION_SETTINGS -> NotificationSettings
                clean == Routes.SECURITY_SETTINGS -> SecuritySettings
                clean == Routes.NOTIFICATIONS -> Notifications
                clean == Routes.DRIVE_HUB -> DriveHub
                clean == "ride-new" -> RideNew(
                    eventId = params["event"]?.toLongOrNull(),
                    proxy = params["proxy"] == "1",
                )
                clean == "studio" -> Studio(
                    shape = params["shape"],
                    color = params["color"],
                    face = params["face"],
                )
                clean.startsWith("ride/") -> clean.removePrefix("ride/").toLongOrNull()?.let { id ->
                    RideDetail(id = id, justBooked = params["new"] == "1")
                }
                clean.startsWith("event/") -> clean.removePrefix("event/").toLongOrNull()?.let { id ->
                    EventDetail(id = id, checkinCode = params["code"]?.takeIf { it.isNotBlank() })
                }
                else -> null
            }
        }
    }
}

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
    const val PERSONAL_DETAILS = "personal-details"
    const val APPEARANCE_SETTINGS = "appearance-settings"
    const val NOTIFICATION_SETTINGS = "notification-settings"
    const val SECURITY_SETTINGS = "security-settings"
    const val DRIVER_SETTINGS = "driver-settings"
    const val PROXY_RIDERS = "proxy-riders"
    const val SAVED_ADDRESSES = "saved-addresses"
    const val NOTIFICATIONS = "notifications"
    const val DRIVE_HUB = "drive"
    const val STUDIO = "studio?shape={shape}&color={color}&face={face}"

    fun studio(shape: String? = null, color: String? = null, face: String? = null): String = buildString {
        append("studio")
        val params = mutableListOf<String>()
        if (!shape.isNullOrBlank()) params.add("shape=${encodeQueryComponent(shape)}")
        if (!color.isNullOrBlank()) params.add("color=${encodeQueryComponent(color)}")
        if (!face.isNullOrBlank()) params.add("face=${encodeQueryComponent(face)}")
        if (params.isNotEmpty()) {
            append("?")
            append(params.joinToString("&"))
        }
    }

    fun rideDetail(id: Long, justBooked: Boolean = false) =
        if (justBooked) "ride/$id?new=1" else "ride/$id"
    fun eventDetail(id: Long, checkinCode: String? = null): String = buildString {
        append("event/$id")
        if (!checkinCode.isNullOrBlank()) {
            append("?code=${encodeQueryComponent(checkinCode)}")
        }
    }
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
    val tabClasses: Set<String> = setOf(
        AppRoute.Home::class.qualifiedName!!,
        AppRoute.Events::class.qualifiedName!!,
        AppRoute.Profile::class.qualifiedName!!,
    )
    val authClasses: Set<String> = setOf(
        AppRoute.Login::class.qualifiedName!!,
        AppRoute.Unlock::class.qualifiedName!!,
        AppRoute.Pending::class.qualifiedName!!,
        AppRoute.Terms::class.qualifiedName!!,
    )

    fun isPermitted(route: AppRoute, me: org.pjstatecm.graceride.data.api.Me): Boolean = when (route) {
        AppRoute.DriveHub, AppRoute.DriverSettings -> me.can("drive")
        AppRoute.Rides, AppRoute.History, AppRoute.SavedAddresses, AppRoute.ProxyRiders -> me.can("rides")
        is AppRoute.RideNew -> me.can("rides", "create")
        AppRoute.Costs -> me.can("costs")
        AppRoute.Events -> me.can("events")
        is AppRoute.EventDetail -> me.can("events") || me.can("admin_events")
        else -> true
    }

    fun isLegacyRoutePermitted(legacyRoute: String, me: org.pjstatecm.graceride.data.api.Me): Boolean {
        val typed = AppRoute.fromLegacyRoute(legacyRoute) ?: return true
        return isPermitted(typed, me)
    }

    private val trafficHubPerms = listOf(
        "admin_plan",
        "admin_rides",
        "admin_live",
        "admin_area_photos",
        "admin_areas",
        "admin_fares",
    )

    private val adminMenuPerms = trafficHubPerms + listOf(
        "admin_dashboard",
        "admin_members",
        "admin_invites",
        "admin_recovery",
        "admin_roles",
        "admin_reimb_review",
        "admin_reimb_pay",
        "admin_reports",
        "admin_announcements",
        "admin_help",
        "admin_groups",
        "admin_settings",
        "admin_audit",
    )

    private val rideDetailWebRe = Regex("^/admin/rides/(\\d+)$")
    private val adminEventPlanWebRe = Regex("^/admin/events/(\\d+)/plan$")
    private val eventEditWebRe = Regex("^/events/(\\d+)/edit$")
    private val eventSettleWebRe = Regex("^/events/(\\d+)/settle$")
    private val settlementDetailWebRe = Regex("^/settlements/(\\d+)$")
    private val settlementEditWebRe = Regex("^/settlements/(\\d+)/edit$")
    private val reimbursementDetailWebRe = Regex("^/reimbursements/(\\d+)$")
    private val helpGuideEditWebRe = Regex("^/help/guides/([^/]+)/edit$")

    /**
     * Members hold `settlements.view` in DB to pay their own shares in native Costs.
     * Web settlement management is reserved for organisers/finance/staff.
     */
    fun canManageSettlements(me: org.pjstatecm.graceride.data.api.Me): Boolean =
        me.can("settlements", "create") ||
            me.can("settlements", "update") ||
            me.can("admin_bills") ||
            (me.hasStaffAccess() && me.can("settlements"))

    /** True when the user should see Web workspace links (staff/admin/event organisers). */
    fun canUseStaffWeb(me: org.pjstatecm.graceride.data.api.Me): Boolean =
        me.hasStaffAccess() ||
            canManageSettlements(me) ||
            me.can("events", "create") ||
            me.can("events", "update")

    /**
     * Enforces web route permissions matching `frontend/src/App.tsx`, default-denying
     * all `/admin*` and unrecognized paths so normal members are never sent to admin web pages.
     */
    fun isWebPathPermitted(path: String, me: org.pjstatecm.graceride.data.api.Me): Boolean {
        val raw = path.trim()
        if (raw.isEmpty() || !raw.startsWith("/") || raw.startsWith("//") || raw.startsWith("/api/")) {
            return false
        }
        val clean = raw.substringBefore('?').substringBefore('#').trimEnd('/').ifEmpty { "/" }
        if (clean.contains(':')) return false
        return when {
            clean == "/admin/menu" -> adminMenuPerms.any { me.can(it) }
            clean == "/admin/traffic" -> trafficHubPerms.any { me.can(it) }
            clean == "/admin" -> me.can("admin_dashboard")
            clean == "/admin/rides" || rideDetailWebRe.matches(clean) -> me.can("admin_rides")
            clean == "/admin/live" -> me.can("admin_live")
            clean == "/admin/members" -> me.can("admin_members") || me.can("admin_invites") || me.can("admin_recovery")
            clean == "/admin/settings" -> me.can("admin_groups") || me.can("admin_areas") || me.can("admin_settings") || me.can("admin_fares") || me.can("admin_area_photos")
            clean == "/admin/reimbursements" -> me.can("admin_reimb_review") || me.can("admin_reimb_pay")
            clean == "/admin/announcements" -> me.can("admin_announcements")
            clean == "/admin/audit" -> me.can("admin_audit")
            clean == "/admin/reports" -> me.can("admin_reports")
            clean == "/admin/roles" -> me.can("admin_roles")
            adminEventPlanWebRe.matches(clean) -> me.can("admin_plan")
            clean.startsWith("/admin/") -> false
            clean == "/events/new" -> me.can("events", "create")
            eventEditWebRe.matches(clean) -> me.can("events", "update") || me.can("admin_events", "update")
            eventSettleWebRe.matches(clean) -> me.can("settlements", "create")
            settlementEditWebRe.matches(clean) -> me.can("settlements", "update")
            clean == "/settlements" || settlementDetailWebRe.matches(clean) -> canManageSettlements(me)
            clean == "/reimbursements/new" -> me.can("reimbursements", "create") || me.can("reimbursements", "update")
            clean == "/reimbursements" || reimbursementDetailWebRe.matches(clean) -> me.can("reimbursements")
            clean == "/help/guides/new" || helpGuideEditWebRe.matches(clean) -> me.can("admin_help", "update")
            clean == "/help" || clean.startsWith("/help/") -> me.can("help") || me.can("admin_help")
            clean == "/report" -> me.can("report")
            else -> false
        }
    }
}

/** Pure-Kotlin RFC 3986 query component encoder (works on both Android and plain JVM unit tests). */
private fun encodeQueryComponent(value: String): String {
    val bytes = value.encodeToByteArray()
    return buildString(bytes.size) {
        for (b in bytes) {
            val code = b.toInt() and 0xFF
            val unreserved = (code in 'A'.code..'Z'.code) ||
                (code in 'a'.code..'z'.code) ||
                (code in '0'.code..'9'.code) ||
                code == '-'.code || code == '_'.code || code == '.'.code || code == '~'.code
            if (unreserved) {
                append(code.toChar())
            } else {
                append('%')
                append(HEX_DIGITS[code ushr 4])
                append(HEX_DIGITS[code and 0x0F])
            }
        }
    }
}

private fun decodeQueryComponent(encoded: String): String {
    if ('%' !in encoded) return encoded
    val out = ArrayList<Byte>(encoded.length)
    var i = 0
    while (i < encoded.length) {
        val ch = encoded[i]
        if (ch == '%' && i + 2 < encoded.length) {
            val hi = encoded[i + 1].digitToIntOrNull(16)
            val lo = encoded[i + 2].digitToIntOrNull(16)
            if (hi != null && lo != null) {
                out.add(((hi shl 4) or lo).toByte())
                i += 3
                continue
            }
        }
        for (b in ch.toString().encodeToByteArray()) out.add(b)
        i++
    }
    return out.toByteArray().decodeToString()
}

private const val HEX_DIGITS = "0123456789ABCDEF"

fun NavHostController.navigateTab(route: AppRoute) {
    navigate(route) {
        popUpTo<AppRoute.Home> { saveState = true }
        launchSingleTop = true
        restoreState = true
    }
}

fun NavHostController.navigateTab(route: String) {
    val typed = AppRoute.fromLegacyRoute(route)
    if (typed != null) {
        navigateTab(typed)
    } else {
        navigate(route) {
            popUpTo(Routes.HOME) { saveState = true }
            launchSingleTop = true
            restoreState = true
        }
    }
}

/**
 * Navigates using a legacy route string (e.g. from notification links), preferring the
 * matching type-safe [AppRoute] destination when available.
 */
fun NavHostController.navigateRoute(
    route: String,
    builder: androidx.navigation.NavOptionsBuilder.() -> Unit = {},
) {
    val typed = AppRoute.fromLegacyRoute(route)
    if (typed != null) {
        navigate(typed, builder)
    } else {
        navigate(route, builder)
    }
}
