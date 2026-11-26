import Foundation

public enum NotifDestination: Equatable, Sendable {
    case native(route: String)
    case web(path: String)
    case none
}

public enum NotifLinkRouter {
    private static let trustedHosts: Set<String> = ["graceride.saliminn.my"]

    public static func extractRideId(_ link: String?) -> Int64? {
        guard let raw = link?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty else {
            return nil
        }
        let clean = raw.components(separatedBy: "?").first?.components(separatedBy: "#").first ?? raw
        guard clean.hasPrefix("/rides/") else { return nil }
        let suffix = String(clean.dropFirst("/rides/".count))
        guard !suffix.isEmpty, suffix.allSatisfy(\.isNumber) else { return nil }
        return Int64(suffix)
    }

    /// Converts verified HTTPS Universal Links (`https://graceride.saliminn.my/rides/12`) and
    /// internal shortcut URIs (`graceride://open/drive`) into a normalized internal path
    /// suitable for `resolve`, rejecting untrusted external hosts.
    public static func extractPathFromIntentUri(_ uriString: String?, extraTrustedHosts: Set<String> = []) -> String? {
        guard let raw = uriString?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty,
              let url = URL(string: raw),
              let scheme = url.scheme?.lowercased(),
              let host = url.host?.lowercased() else {
            return nil
        }
        let path = url.path.isEmpty ? "/" : url.path
        let query = url.query.map { $0.isEmpty ? "" : "?\($0)" } ?? ""
        let candidate = path + query
        guard Routes.sanitizeCleanPath(candidate) != nil else {
            return nil
        }
        if scheme == "graceride" && host == "open" {
            return candidate
        }
        if scheme == "https" && (trustedHosts.contains(host) || extraTrustedHosts.contains(host)) {
            return candidate
        }
        return nil
    }

    public static func resolve(_ link: String?) -> NotifDestination {
        guard let raw = link?.trimmingCharacters(in: .whitespacesAndNewlines),
              let clean = Routes.sanitizeCleanPath(raw) else {
            return .none
        }

        let queryAndFrag = raw.contains("?") ? String(raw.split(separator: "?", maxSplits: 1)[1]) : ""
        let query = queryAndFrag.components(separatedBy: "#").first ?? ""
        let params = Routes.parseQuery(query)

        if let rideId = extractRideId(clean) {
            return .native(route: Routes.rideDetail(id: rideId))
        }

        if let checkinId = matchSingleId(clean, pattern: #"^/events/(\d+)/checkin$"#) {
            let code = params["code"]?.trimmingCharacters(in: .whitespacesAndNewlines)
            return .native(route: Routes.eventDetail(id: checkinId, checkinCode: (code?.isEmpty == false) ? code : nil))
        }

        if let eventId = matchSingleId(clean, pattern: #"^/events/(\d+)$"#) {
            return .native(route: Routes.eventDetail(id: eventId))
        }

        switch clean {
        case "/": return .native(route: "home")
        case "/rides": return .native(route: "rides")
        case "/ride-new", "/rides/new":
            return .native(route: Routes.rideNew(
                eventId: params["event"].flatMap(Int64.init),
                proxy: params["proxy"] == "1" || params["proxy"]?.lowercased() == "true"
            ))
        case "/events": return .native(route: "events")
        case "/costs": return .native(route: "costs")
        case "/history": return .native(route: "history")
        case "/me", "/profile": return .native(route: "profile")
        case "/notifications": return .native(route: "notifications")
        case "/drive": return .native(route: "drive-hub")
        case "/driver": return .native(route: "driver-settings")
        default: return .web(path: raw)
        }
    }

    public static func resolveFor(_ link: String?, me: Me) -> NotifDestination {
        switch resolve(link) {
        case .native(let route):
            return Routes.isLegacyRoutePermitted(route, me: me) ? .native(route: route) : .none
        case .web(let path):
            var clean = path.components(separatedBy: "?").first?.components(separatedBy: "#").first ?? path
            while clean.count > 1 && clean.hasSuffix("/") { clean.removeLast() }
            if clean.range(of: #"^/settlements(?:/\d+)?$"#, options: .regularExpression) != nil,
               !Routes.canManageSettlements(me), me.can("costs") {
                return .native(route: "costs")
            }
            if clean == "/me/family" && me.can("rides") {
                return .native(route: "proxy-riders")
            }
            return Routes.isWebPathPermitted(path, me: me) ? .web(path: path) : .none
        case .none:
            return .none
        }
    }

    private static func matchSingleId(_ text: String, pattern: String) -> Int64? {
        guard let regex = try? NSRegularExpression(pattern: pattern),
              let match = regex.firstMatch(in: text, range: NSRange(text.startIndex..<text.endIndex, in: text)),
              match.numberOfRanges >= 2,
              let range = Range(match.range(at: 1), in: text) else {
            return nil
        }
        return Int64(text[range])
    }
}
