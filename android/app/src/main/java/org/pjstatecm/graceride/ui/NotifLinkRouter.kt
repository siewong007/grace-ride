package org.pjstatecm.graceride.ui

import java.net.URI

sealed class NotifDestination {
    data class Native(val route: String) : NotifDestination()
    data class Web(val path: String) : NotifDestination()
    data object None : NotifDestination()
}

object NotifLinkRouter {
    private val rideDetailRe = Regex("^/rides/(\\d+)$")
    private val eventDetailRe = Regex("^/events/(\\d+)$")
    private val eventCheckinRe = Regex("^/events/(\\d+)/checkin$")
    private val trustedHosts = setOf("graceride.saliminn.my")

    fun extractRideId(link: String?): Long? {
        val clean = link?.trim()?.substringBefore('?')?.substringBefore('#') ?: return null
        return rideDetailRe.find(clean)?.groupValues?.get(1)?.toLongOrNull()
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
        val clean = raw.substringBefore('?').substringBefore('#')
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
        eventCheckinRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.eventDetail(id, checkinCode = params["code"]?.trim()?.takeIf { c -> c.isNotEmpty() }))
        }
        eventDetailRe.find(clean)?.let {
            val id = it.groupValues[1].toLongOrNull() ?: return NotifDestination.None
            return NotifDestination.Native(Routes.eventDetail(id))
        }

        return when {
            clean == "/" -> NotifDestination.Native(Routes.HOME)
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
            else -> NotifDestination.Web(raw)
        }
    }

    private val settlementWebRe = Regex("^/settlements(?:/\\d+)?$")

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
                    clean == "/me/family" && me.can("rides") ->
                        NotifDestination.Native(Routes.PROXY_RIDERS)
                    Routes.isWebPathPermitted(dest.path, me) -> dest
                    else -> NotifDestination.None
                }
            }
            NotifDestination.None -> NotifDestination.None
        }

    private fun decodeQueryValue(raw: String): String =
        runCatching { java.net.URLDecoder.decode(raw, "UTF-8") }.getOrDefault(raw)
}
