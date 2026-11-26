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
    @Serializable data class EventNew(val duplicateId: Long? = null) : AppRoute
    @Serializable data class EventEdit(val id: Long) : AppRoute
    @Serializable data class EventDetail(
        val id: Long,
        val checkinCode: String? = null,
        val openCheckin: Boolean = false,
    ) : AppRoute
    @Serializable data object Costs : AppRoute
    @Serializable data object History : AppRoute
    @Serializable data object Profile : AppRoute
    @Serializable data object DriverSettings : AppRoute
    @Serializable data object ProxyRiders : AppRoute
    @Serializable data object Family : AppRoute
    @Serializable data object SavedAddresses : AppRoute
    @Serializable data object PersonalDetails : AppRoute
    @Serializable data object AppearanceSettings : AppRoute
    @Serializable data object NotificationSettings : AppRoute
    @Serializable data object SecuritySettings : AppRoute
    @Serializable data object TermsPublic : AppRoute
    @Serializable data object Licenses : AppRoute
    @Serializable data class Invite(val token: String) : AppRoute
    @Serializable data class Visit(val token: String) : AppRoute
    @Serializable data class CarePass(val token: String) : AppRoute
    @Serializable data object Notifications : AppRoute
    @Serializable data object DriveHub : AppRoute
    @Serializable data class TrafficHub(
        val tab: String = "plans",
        val view: String = "rides",
        val areaTab: String = "photos",
        val status: String? = null,
        val attention: Boolean = false,
        val planFilter: String? = null,
    ) : AppRoute
    @Serializable data class PlanTransport(
        val eventId: Long,
        val draftId: Long? = null,
    ) : AppRoute
    @Serializable data class Studio(
        val shape: String? = null,
        val color: String? = null,
        val face: String? = null,
    ) : AppRoute
    @Serializable data class AdminMembers(
        val tab: String = "members",
        val status: String? = null,
    ) : AppRoute
    @Serializable data class Announcements(
        val manage: Boolean = false,
    ) : AppRoute
    @Serializable data object Settlements : AppRoute
    @Serializable data class SettlementDetail(val id: Long) : AppRoute
    @Serializable data class SettlementForm(
        val settlementId: Long? = null,
        val eventId: Long? = null,
    ) : AppRoute
    @Serializable data object Reimbursements : AppRoute
    @Serializable data class ReimbursementDetail(val id: Long) : AppRoute
    @Serializable data class ReimbursementForm(val editId: Long? = null) : AppRoute
    @Serializable data class AdminReimbursements(val tab: String = "review") : AppRoute
    @Serializable data class Help(val rideId: Long? = null, val tab: String? = null) : AppRoute
    @Serializable data class HelpDetail(val id: Long) : AppRoute
    @Serializable data class HelpGuideDetail(val key: String) : AppRoute
    @Serializable data class HelpGuideEditor(val key: String? = null) : AppRoute
    @Serializable data object Report : AppRoute
    @Serializable data class AdminReports(val tab: String? = null) : AppRoute
    @Serializable data object AdminConsole : AppRoute
    @Serializable data object AdminRoles : AppRoute
    @Serializable data object AdminAudit : AppRoute
    @Serializable data class AdminSettings(val tab: String? = null) : AppRoute

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
                clean == Routes.FAMILY -> Family
                clean == Routes.SAVED_ADDRESSES -> SavedAddresses
                clean == Routes.PERSONAL_DETAILS -> PersonalDetails
                clean == Routes.APPEARANCE_SETTINGS -> AppearanceSettings
                clean == Routes.NOTIFICATION_SETTINGS -> NotificationSettings
                clean == Routes.SECURITY_SETTINGS -> SecuritySettings
                clean == Routes.TERMS_PUBLIC -> TermsPublic
                clean == Routes.LICENSES -> Licenses
                clean.startsWith("invite/") -> clean.removePrefix("invite/").trim().takeIf { it.isNotEmpty() }?.let { Invite(it) }
                clean.startsWith("visit/") -> clean.removePrefix("visit/").trim().takeIf { it.isNotEmpty() }?.let { Visit(it) }
                clean.startsWith("care/") -> clean.removePrefix("care/").trim().takeIf { it.isNotEmpty() }?.let { CarePass(it) }
                clean == Routes.NOTIFICATIONS -> Notifications
                clean == Routes.DRIVE_HUB -> DriveHub
                clean == Routes.TRAFFIC -> TrafficHub(
                    tab = params["tab"]?.takeIf { it in setOf("plans", "rides", "live", "areas") } ?: "plans",
                    view = params["view"]?.takeIf { it in setOf("rides", "drafts") } ?: "rides",
                    areaTab = params["areaTab"]?.takeIf { it in setOf("photos", "fares", "areas") } ?: "photos",
                    status = params["status"]?.takeIf { it.isNotBlank() },
                    attention = params["attention"] == "1" || params["attention"].equals("true", ignoreCase = true),
                    planFilter = params["planFilter"]?.takeIf { it.isNotBlank() },
                )
                clean.startsWith("plan-transport/") -> clean.removePrefix("plan-transport/").toLongOrNull()?.let { eventId ->
                    PlanTransport(
                        eventId = eventId,
                        draftId = params["draftId"]?.toLongOrNull(),
                    )
                }
                clean == "ride-new" -> RideNew(
                    eventId = params["event"]?.toLongOrNull(),
                    proxy = params["proxy"] == "1",
                )
                clean == "studio" -> Studio(
                    shape = params["shape"],
                    color = params["color"],
                    face = params["face"],
                )
                clean == Routes.ADMIN_MEMBERS -> AdminMembers(
                    tab = params["tab"]?.takeIf { it in setOf("members", "invites", "recovery") } ?: "members",
                    status = params["status"]?.takeIf { it.isNotBlank() },
                )
                clean == Routes.ANNOUNCEMENTS -> Announcements(
                    manage = params["manage"] == "1" || params["manage"].equals("true", ignoreCase = true),
                )
                clean == Routes.SETTLEMENTS -> Settlements
                clean.startsWith("settlement/") -> clean.removePrefix("settlement/").toLongOrNull()?.let { SettlementDetail(it) }
                clean == "settlement-form" -> SettlementForm(
                    settlementId = params["id"]?.toLongOrNull(),
                    eventId = params["event"]?.toLongOrNull(),
                )
                clean == Routes.REIMBURSEMENTS -> Reimbursements
                clean.startsWith("reimbursement/") -> clean.removePrefix("reimbursement/").toLongOrNull()?.let { ReimbursementDetail(it) }
                clean == "reimbursement-form" -> ReimbursementForm(editId = params["edit"]?.toLongOrNull())
                clean == Routes.ADMIN_REIMBURSEMENTS -> AdminReimbursements(
                    tab = params["tab"]?.takeIf { it in setOf("review", "payout", "batches") } ?: "review",
                )
                clean == Routes.HELP -> Help(
                    rideId = params["ride"]?.toLongOrNull(),
                    tab = params["tab"]?.takeIf { it in setOf("guides", "tickets") },
                )
                clean.startsWith("help-detail/") -> clean.removePrefix("help-detail/").toLongOrNull()?.let { HelpDetail(it) }
                clean.startsWith("help-guide-edit/") -> HelpGuideEditor(key = clean.removePrefix("help-guide-edit/").takeIf { it.isNotBlank() && it != "new" })
                clean == "help-guide-new" -> HelpGuideEditor(key = null)
                clean.startsWith("help-guide/") -> clean.removePrefix("help-guide/").takeIf { it.isNotBlank() }?.let { HelpGuideDetail(it) }
                clean == Routes.REPORT -> Report
                clean == Routes.ADMIN_REPORTS -> AdminReports(tab = params["tab"]?.takeIf { it.isNotBlank() })
                clean == Routes.ADMIN_CONSOLE -> AdminConsole
                clean == Routes.ADMIN_ROLES -> AdminRoles
                clean == Routes.ADMIN_AUDIT -> AdminAudit
                clean == Routes.ADMIN_SETTINGS -> AdminSettings(tab = params["tab"]?.takeIf { it.isNotBlank() })
                clean.startsWith("ride/") -> clean.removePrefix("ride/").toLongOrNull()?.let { id ->
                    RideDetail(id = id, justBooked = params["new"] == "1")
                }
                clean.startsWith("event/") -> clean.removePrefix("event/").toLongOrNull()?.let { id ->
                    val code = params["code"]?.takeIf { it.isNotBlank() }
                    EventDetail(
                        id = id,
                        checkinCode = code,
                        openCheckin = code != null || params["checkin"] == "1",
                    )
                }
                clean == Routes.EVENT_NEW -> EventNew(duplicateId = params["duplicate"]?.toLongOrNull())
                clean.startsWith("event-edit/") -> clean.removePrefix("event-edit/").toLongOrNull()?.let { EventEdit(it) }
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
    const val EVENT_NEW = "event-new"
    const val EVENT_DETAIL = "event/{id}"

    fun eventEdit(id: Long) = "event-edit/$id"
    fun eventNew(duplicateId: Long? = null) = if (duplicateId != null) "$EVENT_NEW?duplicate=$duplicateId" else EVENT_NEW
    const val COSTS = "costs"
    const val HISTORY = "history"
    const val PROFILE = "profile"
    const val PERSONAL_DETAILS = "personal-details"
    const val APPEARANCE_SETTINGS = "appearance-settings"
    const val NOTIFICATION_SETTINGS = "notification-settings"
    const val SECURITY_SETTINGS = "security-settings"
    const val DRIVER_SETTINGS = "driver-settings"
    const val PROXY_RIDERS = "proxy-riders"
    const val FAMILY = "family"
    const val SAVED_ADDRESSES = "saved-addresses"
    const val TERMS_PUBLIC = "terms-public"
    const val LICENSES = "licenses"

    fun invite(token: String) = "invite/${encodeQueryComponent(token)}"
    fun visit(token: String) = "visit/${encodeQueryComponent(token)}"
    fun carePass(token: String) = "care/${encodeQueryComponent(token)}"
    const val NOTIFICATIONS = "notifications"
    const val DRIVE_HUB = "drive"
    const val TRAFFIC = "traffic"
    const val PLAN_TRANSPORT = "plan-transport/{eventId}"
    const val STUDIO = "studio?shape={shape}&color={color}&face={face}"
    const val ADMIN_MEMBERS = "admin-members"
    const val ANNOUNCEMENTS = "announcements"
    const val SETTLEMENTS = "settlements"
    const val REIMBURSEMENTS = "reimbursements"
    const val ADMIN_REIMBURSEMENTS = "admin-reimbursements"

    fun settlementDetail(id: Long) = "settlement/$id"
    fun settlementForm(settlementId: Long? = null, eventId: Long? = null) = buildString {
        append("settlement-form")
        val params = mutableListOf<String>()
        if (settlementId != null) params.add("id=$settlementId")
        if (eventId != null) params.add("event=$eventId")
        if (params.isNotEmpty()) {
            append("?")
            append(params.joinToString("&"))
        }
    }
    fun reimbursementDetail(id: Long) = "reimbursement/$id"
    fun reimbursementForm(editId: Long? = null) =
        if (editId != null) "reimbursement-form?edit=$editId" else "reimbursement-form"
    fun adminReimbursements(tab: String = "review") =
        if (tab != "review") "$ADMIN_REIMBURSEMENTS?tab=${encodeQueryComponent(tab)}" else ADMIN_REIMBURSEMENTS
    const val HELP = "help"
    const val REPORT = "report"
    const val ADMIN_REPORTS = "admin-reports"
    const val ADMIN_CONSOLE = "admin-console"
    const val ADMIN_ROLES = "admin-roles"
    const val ADMIN_AUDIT = "admin-audit"
    const val ADMIN_SETTINGS = "admin-settings"

    fun help(rideId: Long? = null, tab: String? = null): String = buildString {
        append(HELP)
        val params = mutableListOf<String>()
        if (rideId != null) params.add("ride=$rideId")
        if (!tab.isNullOrBlank()) params.add("tab=${encodeQueryComponent(tab)}")
        if (params.isNotEmpty()) {
            append("?")
            append(params.joinToString("&"))
        }
    }
    fun helpDetail(id: Long) = "help-detail/$id"
    fun helpGuideDetail(key: String) = "help-guide/${encodeQueryComponent(key)}"
    fun helpGuideEditor(key: String? = null) =
        if (key.isNullOrBlank()) "help-guide-new" else "help-guide-edit/${encodeQueryComponent(key)}"
    fun adminReports(tab: String? = null) =
        if (!tab.isNullOrBlank()) "$ADMIN_REPORTS?tab=${encodeQueryComponent(tab)}" else ADMIN_REPORTS

    fun adminSettings(tab: String? = null) =
        if (!tab.isNullOrBlank()) "$ADMIN_SETTINGS?tab=${encodeQueryComponent(tab)}" else ADMIN_SETTINGS

    fun adminMembers(tab: String = "members", status: String? = null): String = buildString {
        append(ADMIN_MEMBERS)
        val params = mutableListOf<String>()
        if (tab != "members") params.add("tab=${encodeQueryComponent(tab)}")
        if (!status.isNullOrBlank()) params.add("status=${encodeQueryComponent(status)}")
        if (params.isNotEmpty()) {
            append("?")
            append(params.joinToString("&"))
        }
    }

    fun announcements(manage: Boolean = false): String =
        if (manage) "$ANNOUNCEMENTS?manage=1" else ANNOUNCEMENTS

    fun traffic(
        tab: String = "plans",
        view: String = "rides",
        areaTab: String = "photos",
        status: String? = null,
        attention: Boolean = false,
    ): String = buildString {
        append("traffic")
        val params = mutableListOf<String>()
        if (tab != "plans") params.add("tab=${encodeQueryComponent(tab)}")
        if (view != "rides") params.add("view=${encodeQueryComponent(view)}")
        if (areaTab != "photos") params.add("areaTab=${encodeQueryComponent(areaTab)}")
        if (!status.isNullOrBlank()) params.add("status=${encodeQueryComponent(status)}")
        if (attention) params.add("attention=1")
        if (params.isNotEmpty()) {
            append("?")
            append(params.joinToString("&"))
        }
    }

    fun planTransport(eventId: Long, draftId: Long? = null): String = buildString {
        append("plan-transport/$eventId")
        if (draftId != null) append("?draftId=$draftId")
    }

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
    fun eventDetail(id: Long, checkinCode: String? = null, openCheckin: Boolean = false): String = buildString {
        append("event/$id")
        if (!checkinCode.isNullOrBlank()) {
            append("?code=${encodeQueryComponent(checkinCode)}")
        } else if (openCheckin) {
            append("?checkin=1")
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

    val tabs = setOf(HOME, EVENTS, TRAFFIC, PROFILE)
    val tabClasses: Set<String> = setOf(
        AppRoute.Home::class.qualifiedName!!,
        AppRoute.Events::class.qualifiedName!!,
        AppRoute.TrafficHub::class.qualifiedName!!,
        AppRoute.Profile::class.qualifiedName!!,
    )
    val authClasses: Set<String> = setOf(
        AppRoute.Login::class.qualifiedName!!,
        AppRoute.Unlock::class.qualifiedName!!,
        AppRoute.Pending::class.qualifiedName!!,
        AppRoute.Terms::class.qualifiedName!!,
    )
    val publicClasses: Set<String> = setOf(
        AppRoute.Invite::class.qualifiedName!!,
        AppRoute.Visit::class.qualifiedName!!,
        AppRoute.CarePass::class.qualifiedName!!,
        AppRoute.TermsPublic::class.qualifiedName!!,
        AppRoute.Licenses::class.qualifiedName!!,
    )

    fun isPermitted(route: AppRoute, me: org.pjstatecm.graceride.data.api.Me): Boolean = when (route) {
        AppRoute.DriveHub, AppRoute.DriverSettings -> me.can("drive")
        AppRoute.Rides, AppRoute.History, AppRoute.SavedAddresses, AppRoute.ProxyRiders -> me.can("rides")
        is AppRoute.RideNew -> me.can("rides", "create")
        AppRoute.Costs -> me.can("costs")
        AppRoute.Events -> me.can("events")
        is AppRoute.EventNew -> me.can("events", "create")
        is AppRoute.EventEdit -> me.can("events", "update") || me.can("admin_events", "update")
        is AppRoute.EventDetail -> me.can("events") || me.can("admin_events")
        is AppRoute.TrafficHub -> when (route.tab) {
            "rides" -> me.can("admin_rides")
            "live" -> me.can("admin_live")
            "areas" -> me.can("admin_area_photos") || me.can("admin_areas") || me.can("admin_fares")
            else -> canTrafficHub(me)
        }
        is AppRoute.PlanTransport -> me.can("admin_plan")
        is AppRoute.AdminMembers -> when (route.tab) {
            "invites" -> me.can("admin_invites")
            "recovery" -> me.can("admin_recovery")
            else -> canAdminMembers(me)
        }
        is AppRoute.Announcements -> if (route.manage) me.can("admin_announcements") else true
        AppRoute.Settlements, is AppRoute.SettlementDetail -> canManageSettlements(me)
        is AppRoute.SettlementForm -> me.can("settlements", "create") || me.can("settlements", "update") || me.can("admin_bills")
        AppRoute.Reimbursements -> me.can("reimbursements")
        is AppRoute.ReimbursementDetail -> me.can("reimbursements") || me.can("admin_reimb_review") || me.can("admin_reimb_pay")
        is AppRoute.ReimbursementForm -> me.can("reimbursements", "create") || me.can("reimbursements", "update")
        is AppRoute.AdminReimbursements -> me.can("admin_reimb_review") || me.can("admin_reimb_pay")
        is AppRoute.Help, is AppRoute.HelpDetail, is AppRoute.HelpGuideDetail -> me.can("help") || me.can("admin_help")
        is AppRoute.HelpGuideEditor -> me.can("admin_help", "update")
        AppRoute.Report -> me.can("report")
        is AppRoute.AdminReports -> me.can("admin_reports")
        AppRoute.AdminConsole -> me.can("admin_dashboard") || canAdminConsole(me)
        AppRoute.AdminRoles -> me.can("admin_roles")
        AppRoute.AdminAudit -> me.can("admin_audit")
        is AppRoute.AdminSettings -> when (route.tab) {
            "groups" -> me.can("admin_groups")
            "alerts", "system" -> me.can("admin_settings")
            else -> me.can("admin_settings") || me.can("admin_groups")
        }
        else -> true
    }

    /** True when the account can open the native Staff & Admin Console. */
    fun canAdminConsole(me: org.pjstatecm.graceride.data.api.Me): Boolean =
        adminMenuPerms.any { me.can(it) }

    /** True when the account has permission for at least one Traffic Hub tab. */
    fun canTrafficHub(me: org.pjstatecm.graceride.data.api.Me): Boolean =
        trafficHubPerms.any { me.can(it) }

    /** True when the account can access native Members, Invites, or Recovery management. */
    fun canAdminMembers(me: org.pjstatecm.graceride.data.api.Me): Boolean =
        me.can("admin_members") || me.can("admin_invites") || me.can("admin_recovery")

    /** True when the account can access Announcement administration. */
    fun canManageAnnouncements(me: org.pjstatecm.graceride.data.api.Me): Boolean =
        me.can("admin_announcements")

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
