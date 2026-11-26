package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.data.api.Area

/**
 * Infers the best matching active pickup area ID from a street address or autocomplete label
 * by matching normalized active area names (longest match wins) without per-call Regex allocations.
 */
fun inferAreaId(address: String, areas: List<Area>): Long? {
    val normAddr = normalizeSpacesLower(address)
    if (normAddr.isEmpty()) return null

    var bestId: Long? = null
    var bestLen = 0
    for (area in areas) {
        if (!area.active) continue
        val normName = normalizeSpacesLower(area.name)
        if (normName.length < 2 || normName.length <= bestLen) continue
        val matched = if (isAsciiAreaName(normName)) {
            containsAsciiWord(normAddr, normName)
        } else {
            normAddr.contains(normName)
        }
        if (matched) {
            bestId = area.id
            bestLen = normName.length
        }
    }
    return bestId
}

private fun normalizeSpacesLower(input: String): String {
    val trimmed = input.trim()
    if (trimmed.isEmpty()) return ""
    return buildString(trimmed.length) {
        var prevSpace = false
        for (i in trimmed.indices) {
            val ch = trimmed[i]
            if (ch.isWhitespace()) {
                if (!prevSpace) {
                    append(' ')
                    prevSpace = true
                }
            } else {
                append(ch.lowercaseChar())
                prevSpace = false
            }
        }
    }
}

private fun isAsciiAreaName(norm: String): Boolean {
    for (i in norm.indices) {
        val ch = norm[i]
        val valid = (ch in 'a'..'z') || (ch in '0'..'9') || ch == ' ' || ch == '-'
        if (!valid) return false
    }
    return true
}

private fun isAsciiAlphaNum(ch: Char): Boolean = (ch in 'a'..'z') || (ch in '0'..'9')

private fun containsAsciiWord(haystack: String, needle: String): Boolean {
    var start = 0
    while (start <= haystack.length - needle.length) {
        val idx = haystack.indexOf(needle, start)
        if (idx < 0) return false
        val beforeOk = idx == 0 || !isAsciiAlphaNum(haystack[idx - 1])
        val afterIdx = idx + needle.length
        val afterOk = afterIdx == haystack.length || !isAsciiAlphaNum(haystack[afterIdx])
        if (beforeOk && afterOk) return true
        start = idx + 1
    }
    return false
}
