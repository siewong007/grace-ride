import Foundation

/// Infers the best matching active pickup area ID from a street address or autocomplete label
/// by matching normalized active area names (longest match wins) with ASCII word-boundary checking.
public enum AreaMatch {
    public static func inferAreaId(address: String, areas: [Area]) -> Int64? {
        let normAddr = normalizeSpacesLower(address)
        guard !normAddr.isEmpty else { return nil }

        var bestId: Int64?
        var bestLen = 0
        for area in areas where area.active {
            let normName = normalizeSpacesLower(area.name)
            guard normName.count >= 2, normName.count > bestLen else { continue }
            let matched = isAsciiAreaName(normName)
                ? containsAsciiWord(haystack: normAddr, needle: normName)
                : normAddr.contains(normName)
            if matched {
                bestId = area.id
                bestLen = normName.count
            }
        }
        return bestId
    }

    private static func normalizeSpacesLower(_ input: String) -> String {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return "" }
        var out = ""
        out.reserveCapacity(trimmed.count)
        var prevSpace = false
        for scalar in trimmed.unicodeScalars {
            if CharacterSet.whitespacesAndNewlines.contains(scalar) {
                if !prevSpace {
                    out.append(" ")
                    prevSpace = true
                }
            } else {
                out.append(String(scalar).lowercased())
                prevSpace = false
            }
        }
        return out
    }

    private static func isAsciiAreaName(_ norm: String) -> Bool {
        for scalar in norm.unicodeScalars {
            let v = scalar.value
            let valid = (v >= 97 && v <= 122) || (v >= 48 && v <= 57) || v == 32 || v == 45
            if !valid { return false }
        }
        return true
    }

    private static func isAsciiAlphaNum(_ byte: UInt8) -> Bool {
        (byte >= 97 && byte <= 122) || (byte >= 48 && byte <= 57)
    }

    private static func containsAsciiWord(haystack: String, needle: String) -> Bool {
        let hBytes = Array(haystack.utf8)
        let nBytes = Array(needle.utf8)
        guard !nBytes.isEmpty, hBytes.count >= nBytes.count else { return false }

        var start = 0
        while start <= hBytes.count - nBytes.count {
            var matched = true
            for i in 0..<nBytes.count {
                if hBytes[start + i] != nBytes[i] {
                    matched = false
                    break
                }
            }
            if matched {
                let beforeOk = start == 0 || !isAsciiAlphaNum(hBytes[start - 1])
                let afterIdx = start + nBytes.count
                let afterOk = afterIdx == hBytes.count || !isAsciiAlphaNum(hBytes[afterIdx])
                if beforeOk && afterOk {
                    return true
                }
            }
            start += 1
        }
        return false
    }
}
