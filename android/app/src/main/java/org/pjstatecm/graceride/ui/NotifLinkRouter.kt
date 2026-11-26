package org.pjstatecm.graceride.ui

import java.net.URI

sealed class NotifDestination {
    data class Native(val route: String) : NotifDestination()
    data class Web(val path: String) : NotifDestination()
    data object None : NotifDestination()
}

object NotifLinkRouter {
    private val rideDetailRe = Regex("^/rides/(\\d+)$")
    private val adminRideDetailRe = Regex("^/admin/rides/(\\d+)$")
    private val planTransportRe = Regex("^/admin/events/(\\d+)/plan$")
    private val eventDetailRe = Regex("^/events/(\\d+)$")
    private val eventEditRe = Regex("^/events/(\\d+)/edit$")
    private val eventCheckinRe = Regex("^/events/(\\d+)/checkin$")
    private val inviteTokenRe = Regex("^/invite/([^/]+)$")
    private val visitTokenRe = Regex("^/visit/([^/]+)$")
    private val careTokenRe = Regex("^/care/([^/]+)$")
    private val trustedHosts = setOf("graceride.saliminn.my")

    fun extractRideId(link: String?): Long? {
        val clean = link?.trim()?.substringBefore('?')?.substringBefore('#')?.trimEnd('/') ?: return null
        return (rideDetailRe.find(clean) ?: adminRideDetailRe.find(clean))?.groupValues?.get(1)?.toLongOrNull()
    }

    /**
     * Converts verified HTTPS App Links (`https://graceride.saliminn.my/rides/12`) and
     * internal shortcut URIs (`graceride://open/drive`) into a normalized internal path
     * suitable for [resolve], rejecting untrusted external hosts.
     */
    fun extractPathFromIntentUri(uriString: String?, extraTrustedHosts: Set<String> = emptySet()): String? {
        val raw = uriString?.trim()?.takeIf { it.isNotEmpty() } ?: return null
        val uri = runCatching { URI(raw) }.getOrNull() ?: return null
        val scheme = uri.scheme?.lowercase() ?: return null
        val host = uri.host?.lowercase() ?: return null
        val rawPath = uri.rawPath.orEmpty().ifEmpty { "/" }
        val query = uri.rawQuery?.takeIf { it.isNotBlank() }?.let { "?$it" }.orEmpty()
        return when {
            scheme == "graceride" && host == "open" -> rawPath + query
            scheme == "https" && (host in trustedHosts || host in extraTrustedHosts) -> rawPath + query
            else -> null
        }
    }

    fun resolve(link: String?): NotifDestination {
        val raw = link?.trim() ?: return NotifDestination.None
        if (raw.isEmpty() || !raw.startsWith("/") || raw.startsWith("//") || raw.startsWith("/api/")) {
            return NotifDestination.None
        }
        val clean = raw.substringBefore('?').substringBefore('#').trimEnd('/').ifEmpty { "/" }
        val query = raw.substringAfter('?', "").substringBefore('#')
        val params = if (query.isBlank()) {
            emptyMap()
        } else {
            query.split('&').mapNotNull { kv ->
                val idx = kv.indexOf('=')
                if (idx <= 0) null else kv.substring(0, idx) to decodeQueryValue(kv.substring(idx + 1))
            }.toMap()
        }
        if (clean.contains(':')) return NotifDestination.None

        rideDetailRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.rideDetail(id))
        }
        adminRideDetailRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.rideDetail(id))
        }
        planTransportRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.planTransport(id, draftId = params["draft"]?.toLongOrNull()))
        }
        eventCheckinRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            val code = params["code"]?.trim()?.takeIf { c -> c.isNotEmpty() }
            return NotifDestination.Native(Routes.eventDetail(id, checkinCode = code, openCheckin = code == null))
        }
        eventEditRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.eventEdit(id))
        }
        eventDetailRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.eventDetail(id))
        }
        inviteTokenRe.find(clean)?.let {
            return NotifDestination.Native(Routes.invite(it.groupValues[1]))
        }
        visitTokenRe.find(clean)?.let {
            return NotifDestination.Native(Routes.visit(it.groupValues[1]))
        }
        careTokenRe.find(clean)?.let {
            return NotifDestination.Native(Routes.carePass(it.groupValues[1]))
        }

        return when {
            clean == "/" -> NotifDestination.Native(Routes.HOME)
            clean == "/events/new" -> NotifDestination.Native(Routes.eventNew(params["duplicate"]?.toLongOrNull()))
            clean == "/terms" -> NotifDestination.Native(Routes.TERMS_PUBLIC)
            clean == "/licences" || clean == "/licenses" -> NotifDestination.Native(Routes.LICENSES)
            clean == "/me/family" -> NotifDestination.Native(Routes.FAMILY)
            clean == "/rides" -> NotifDestination.Native(Routes.RIDES)
            clean == "/ride-new" || clean == "/rides/new" -> NotifDestination.Native(
                Routes.rideNew(
                    eventId = params["event"]?.toLongOrNull(),
                    proxy = params["proxy"] == "1" || params["proxy"].equals("true", ignoreCase = true),
                ),
            )
            clean == "/events" -> NotifDestination.Native(Routes.EVENTS)
            clean == "/costs" -> NotifDestination.Native(Routes.COSTS)
            clean == "/history" -> NotifDestination.Native(Routes.HISTORY)
            clean == "/me" || clean == "/profile" -> NotifDestination.Native(Routes.PROFILE)
            clean == "/notifications" -> NotifDestination.Native(Routes.NOTIFICATIONS)
            clean == "/drive" -> NotifDestination.Native(Routes.DRIVE_HUB)
            clean == "/driver" -> NotifDestination.Native(Routes.DRIVER_SETTINGS)
            clean == "/admin/traffic" -> NotifDestination.Native(
                Routes.traffic(
                    tab = params["tab"]?.takeIf { it in setOf("plans", "rides", "live", "areas") } ?: "plans",
                    view = params["view"]?.takeIf { it in setOf("rides", "drafts") } ?: "rides",
                    areaTab = params["areaTab"]?.takeIf { it in setOf("photos", "fares", "areas") } ?: "photos",
                ),
            )
            clean == "/admin/live" -> NotifDestination.Native(Routes.traffic(tab = "live"))
            clean == "/admin/rides" -> NotifDestination.Native(
                Routes.traffic(
                    tab = "rides",
                    view = params["view"]?.takeIf { it in setOf("rides", "drafts") } ?: "rides",
                    status = params["status"]?.takeIf { it.isNotBlank() },
                    attention = params["attention"] == "1" || params["attention"].equals("true", ignoreCase = true),
                ),
            )
            clean == "/studio" -> NotifDestination.Native(
                Routes.studio(
                    shape = params["shape"],
                    color = params["color"],
                    face = params["face"],
                ),
            )
            clean == "/admin/members" -> NotifDestination.Native(
                Routes.adminMembers(
                    tab = params["tab"]?.takeIf { it in setOf("members", "invites", "recovery") } ?: "members",
                    status = params["status"]?.takeIf { it.isNotBlank() },
                ),
            )
            clean == "/admin/announcements" -> NotifDestination.Native(Routes.announcements(manage = true))
            clean == "/announcements" -> NotifDestination.Native(Routes.announcements(manage = false))
            else -> NotifDestination.Web(raw)
        }
    }

    private val settlementWebRe = Regex("^/settlements(?:/\\d+)?$")

    fun resolvePublic(link: String?): AppRoute? {
        val raw = link?.trim() ?: return null
        if (raw.isEmpty() || !raw.startsWith("/") || raw.startsWith("//") || raw.startsWith("/api/")) {
            return null
        }
        val clean = raw.substringBefore('?').substringBefore('#').trimEnd('/').ifEmpty { "/" }
        if (clean.contains(':')) return null
        inviteTokenRe.find(clean)?.let { return AppRoute.Invite(it.groupValues[1]) }
        visitTokenRe.find(clean)?.let { return AppRoute.Visit(it.groupValues[1]) }
        careTokenRe.find(clean)?.let { return AppRoute.CarePass(it.groupValues[1]) }
        return when (clean) {
            "/terms" -> AppRoute.TermsPublic
            "/licences", "/licenses" -> AppRoute.Licenses
            else -> null
        }
    }

    /**
     * Resolves a notification link in the context of [me], routing members smoothly to native
     * Costs/Frequent riders where possible and suppressing unpermitted admin/web redirects.
     */
    fun resolveFor(link: String?, me: org.pjstatecm.graceride.data.api.Me): NotifDestination =
        when (val dest = resolve(link)) {
            is NotifDestination.Native ->
                if (Routes.isLegacyRoutePermitted(dest.route, me)) dest else NotifDestination.None
            is NotifDestination.Web -> {
                val clean = dest.path.substringBefore('?').substringBefore('#').trimEnd('/')
                when {
                    settlementWebRe.matches(clean) && !Routes.canManageSettlements(me) && me.can("costs") ->
                        NotifDestination.Native(Routes.COSTS)
                    Routes.isWebPathPermitted(dest.path, me) -> dest
                    else -> NotifDestination.None
                }
            }
            NotifDestination.None -> NotifDestination.None
        }

    private fun decodeQueryValue(raw: String): String {
        if ('%' !in raw) return raw
        val out = ArrayList<Byte>(raw.length)
        var i = 0
        while (i < raw.length) {
            val ch = raw[i]
            if (ch == '%' && i + 2 < raw.length) {
                val hi = raw[i + 1].digitToIntOrNull(16)
                val lo = raw[i + 2].digitToIntOrNull(16)
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
}
