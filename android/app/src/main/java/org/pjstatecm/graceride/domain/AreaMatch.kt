package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.data.api.Area

private val WHITESPACE_RE = Regex("\\s+")
private val ASCII_NAME_RE = Regex("^[a-z0-9\\s-]+$", RegexOption.IGNORE_CASE)

/**
 * Infers the best matching active pickup area ID from a street address or autocomplete label
 * by matching normalized active area names (longest match wins).
 */
fun inferAreaId(address: String, areas: List<Area>): Long? {
    val normAddr = address.trim().lowercase().replace(WHITESPACE_RE, " ")
    if (normAddr.isEmpty()) return null

    val candidates = areas
        .asSequence()
        .filter { it.active }
        .map { it.id to it.name.trim().lowercase().replace(WHITESPACE_RE, " ") }
        .filter { it.second.length >= 2 }
        .sortedByDescending { it.second.length }

    for ((id, normName) in candidates) {
        if (ASCII_NAME_RE.matches(normName)) {
            val boundary = Regex("(^|[^a-z0-9])${Regex.escape(normName)}($|[^a-z0-9])", RegexOption.IGNORE_CASE)
            if (boundary.containsMatchIn(normAddr)) return id
        } else if (normAddr.contains(normName)) {
            return id
        }
    }
    return null
}
