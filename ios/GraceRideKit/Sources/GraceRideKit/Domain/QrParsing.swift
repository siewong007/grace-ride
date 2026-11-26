import Foundation

public enum CheckinParseResult: Equatable, Sendable {
    case code(String)
    case wrongEvent
    case invalid
}

public enum QrParsing {
    public static func parseCheckinInput(_ raw: String, currentEventId: Int64) -> CheckinParseResult {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return .invalid }

        let lower = trimmed.lowercased()
        if lower.hasPrefix("grpass:") {
            return .invalid
        }
        let looksLikeUrl = lower.hasPrefix("http://")
            || lower.hasPrefix("https://")
            || lower.hasPrefix("/events/")
            || lower.contains("code=")

        guard looksLikeUrl else {
            return .code(trimmed)
        }

        let normalizedUrl = lower.hasPrefix("code=") ? "https://local/?\(trimmed)" : trimmed
        let comps = URLComponents(string: normalizedUrl)
        let path = comps?.path ?? (trimmed.components(separatedBy: "?").first?.components(separatedBy: "#").first ?? "")

        if let regex = try? NSRegularExpression(pattern: #"/events/(\d+)/checkin"#, options: .caseInsensitive),
           let match = regex.firstMatch(in: path, range: NSRange(path.startIndex..<path.endIndex, in: path)),
           match.numberOfRanges >= 2,
           let range = Range(match.range(at: 1), in: path),
           let pathEventId = Int64(path[range]),
           pathEventId != currentEventId {
            return .wrongEvent
        }

        if let codeItem = comps?.queryItems?.first(where: { $0.name == "code" }),
           let val = codeItem.value?.trimmingCharacters(in: .whitespacesAndNewlines),
           !val.isEmpty {
            return .code(val)
        }

        let qPart = (trimmed.split(separator: "?", maxSplits: 1).dropFirst().first.map(String.init) ?? "")
            .components(separatedBy: "#").first ?? ""
        if let val = Routes.parseQuery(qPart)["code"]?.trimmingCharacters(in: .whitespacesAndNewlines), !val.isEmpty {
            return .code(val)
        }
        return .invalid
    }

    public static func parseMemberPassInput(_ raw: String, currentEventId: Int64) -> CheckinParseResult {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return .invalid }

        guard let regex = try? NSRegularExpression(pattern: #"^grpass:(\d*):(.+)$"#, options: .caseInsensitive),
              let match = regex.firstMatch(in: trimmed, range: NSRange(trimmed.startIndex..<trimmed.endIndex, in: trimmed)),
              match.numberOfRanges >= 3,
              let evRange = Range(match.range(at: 1), in: trimmed),
              let passRange = Range(match.range(at: 2), in: trimmed) else {
            return .code(trimmed)
        }

        let evPart = String(trimmed[evRange])
        let pass = String(trimmed[passRange]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard !pass.isEmpty else { return .invalid }

        if !evPart.isEmpty, Int64(evPart) != currentEventId {
            return .wrongEvent
        }
        return .code(pass)
    }
}
