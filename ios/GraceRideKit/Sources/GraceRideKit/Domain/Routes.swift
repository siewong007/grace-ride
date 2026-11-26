import Foundation

public enum AppRoute: Hashable, Codable, Sendable {
    case home
    case rides
    case rideDetail(id: Int64, justBooked: Bool = false)
    case rideNew(eventId: Int64? = nil, proxy: Bool = false)
    case events
    case eventDetail(id: Int64, checkinCode: String? = nil)
    case costs
    case history
    case profile
    case driverSettings
    case proxyRiders
    case savedAddresses
    case notifications
    case driveHub
    case studio(shape: String? = nil, color: String? = nil, face: String? = nil)
}

public enum Routes {
    public static func rideDetail(id: Int64, justBooked: Bool = false) -> String {
        justBooked ? "ride/\(id)?new=1" : "ride/\(id)"
    }

    public static func eventDetail(id: Int64, checkinCode: String? = nil) -> String {
        if let code = checkinCode?.trimmingCharacters(in: .whitespacesAndNewlines), !code.isEmpty,
           let encoded = code.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) {
            return "event/\(id)?code=\(encoded)"
        }
        return "event/\(id)"
    }

    public static func rideNew(eventId: Int64? = nil, proxy: Bool = false) -> String {
        var params: [String] = []
        if let eventId { params.append("event=\(eventId)") }
        if proxy { params.append("proxy=1") }
        return params.isEmpty ? "ride-new" : "ride-new?\(params.joined(separator: "&"))"
    }

    public static func fromLegacyRoute(_ route: String) -> AppRoute? {
        let parts = route.split(separator: "?", maxSplits: 1, omittingEmptySubsequences: false)
        let clean = String(parts.first ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        let query = parts.count > 1 ? String(parts[1]) : ""
        let params = parseQuery(query)

        switch clean {
        case "home": return .home
        case "rides": return .rides
        case "events": return .events
        case "costs": return .costs
        case "history": return .history
        case "profile": return .profile
        case "driver-settings": return .driverSettings
        case "proxy-riders": return .proxyRiders
        case "saved-addresses": return .savedAddresses
        case "notifications": return .notifications
        case "drive-hub": return .driveHub
        case "ride-new":
            return .rideNew(
                eventId: params["event"].flatMap(Int64.init),
                proxy: params["proxy"] == "1" || params["proxy"]?.lowercased() == "true"
            )
        case "studio":
            return .studio(shape: params["shape"], color: params["color"], face: params["face"])
        default:
            if clean.hasPrefix("ride/"), let id = Int64(clean.dropFirst("ride/".count)) {
                return .rideDetail(id: id, justBooked: params["new"] == "1")
            }
            if clean.hasPrefix("event/"), let id = Int64(clean.dropFirst("event/".count)) {
                let code = params["code"]?.trimmingCharacters(in: .whitespacesAndNewlines)
                return .eventDetail(id: id, checkinCode: (code?.isEmpty == false) ? code : nil)
            }
            return nil
        }
    }

    public static func isPermitted(_ route: AppRoute, me: Me) -> Bool {
        switch route {
        case .driveHub, .driverSettings: return me.can("drive")
        case .rides, .history, .savedAddresses, .proxyRiders: return me.can("rides")
        case .rideNew: return me.can("rides", act: "create")
        case .costs: return me.can("costs")
        case .events: return me.can("events")
        case .eventDetail: return me.can("events") || me.can("admin_events")
        default: return true
        }
    }

    public static func isLegacyRoutePermitted(_ legacyRoute: String, me: Me) -> Bool {
        guard let typed = fromLegacyRoute(legacyRoute) else { return true }
        return isPermitted(typed, me: me)
    }

    private static let trafficHubPerms = [
        "admin_plan", "admin_rides", "admin_live",
        "admin_area_photos", "admin_areas", "admin_fares",
    ]

    private static let adminMenuPerms = trafficHubPerms + [
        "admin_dashboard", "admin_members", "admin_invites", "admin_recovery",
        "admin_roles", "admin_reimb_review", "admin_reimb_pay", "admin_reports",
        "admin_announcements", "admin_help", "admin_groups", "admin_settings", "admin_audit",
    ]

    public static func canManageSettlements(_ me: Me) -> Bool {
        me.can("settlements", act: "create")
            || me.can("settlements", act: "update")
            || me.can("admin_bills")
            || (me.hasStaffAccess && me.can("settlements"))
    }

    public static func canUseStaffWeb(_ me: Me) -> Bool {
        me.hasStaffAccess
            || canManageSettlements(me)
            || me.can("events", act: "create")
            || me.can("events", act: "update")
    }

    /// Enforces web route permissions matching `frontend/src/App.tsx`, default-denying
    /// all `/admin*` and unrecognized paths so normal members are never routed to admin pages.
    public static func isWebPathPermitted(_ path: String, me: Me) -> Bool {
        let raw = path.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !raw.isEmpty, raw.hasPrefix("/"), !raw.hasPrefix("//"), !raw.hasPrefix("/api/") else {
            return false
        }
        var clean = raw.components(separatedBy: "?").first ?? raw
        clean = clean.components(separatedBy: "#").first ?? clean
        while clean.count > 1 && clean.hasSuffix("/") {
            clean.removeLast()
        }
        guard !clean.contains(":") else { return false }

        switch clean {
        case "/admin/menu": return adminMenuPerms.contains { me.can($0) }
        case "/admin/traffic": return trafficHubPerms.contains { me.can($0) }
        case "/admin": return me.can("admin_dashboard")
        case "/admin/rides": return me.can("admin_rides")
        case "/admin/live": return me.can("admin_live")
        case "/admin/members": return me.can("admin_members") || me.can("admin_invites") || me.can("admin_recovery")
        case "/admin/settings": return me.can("admin_groups") || me.can("admin_areas") || me.can("admin_settings") || me.can("admin_fares") || me.can("admin_area_photos")
        case "/admin/reimbursements": return me.can("admin_reimb_review") || me.can("admin_reimb_pay")
        case "/admin/announcements": return me.can("admin_announcements")
        case "/admin/audit": return me.can("admin_audit")
        case "/admin/reports": return me.can("admin_reports")
        case "/admin/roles": return me.can("admin_roles")
        case "/events/new": return me.can("events", act: "create")
        case "/settlements": return canManageSettlements(me)
        case "/reimbursements/new": return me.can("reimbursements", act: "create") || me.can("reimbursements", act: "update")
        case "/reimbursements": return me.can("reimbursements")
        case "/help/guides/new": return me.can("admin_help", act: "update")
        case "/help": return me.can("help") || me.can("admin_help")
        case "/report": return me.can("report")
        default:
            if clean.range(of: #"^/admin/rides/\d+$"#, options: .regularExpression) != nil {
                return me.can("admin_rides")
            }
            if clean.range(of: #"^/admin/events/\d+/plan$"#, options: .regularExpression) != nil {
                return me.can("admin_plan")
            }
            if clean.hasPrefix("/admin/") { return false }
            if clean.range(of: #"^/events/\d+/edit$"#, options: .regularExpression) != nil {
                return me.can("events", act: "update") || me.can("admin_events", act: "update")
            }
            if clean.range(of: #"^/events/\d+/settle$"#, options: .regularExpression) != nil {
                return me.can("settlements", act: "create")
            }
            if clean.range(of: #"^/settlements/\d+/edit$"#, options: .regularExpression) != nil {
                return me.can("settlements", act: "update")
            }
            if clean.range(of: #"^/settlements/\d+$"#, options: .regularExpression) != nil {
                return canManageSettlements(me)
            }
            if clean.range(of: #"^/reimbursements/\d+$"#, options: .regularExpression) != nil {
                return me.can("reimbursements")
            }
            if clean.range(of: #"^/help/guides/[^/]+/edit$"#, options: .regularExpression) != nil {
                return me.can("admin_help", act: "update")
            }
            if clean.hasPrefix("/help/") {
                return me.can("help") || me.can("admin_help")
            }
            return false
        }
    }

    static func parseQuery(_ query: String) -> [String: String] {
        guard !query.isEmpty else { return [:] }
        var out: [String: String] = [:]
        for kv in query.split(separator: "&") {
            let pair = kv.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
            guard pair.count == 2, !pair[0].isEmpty else { continue }
            let key = String(pair[0])
            let rawVal = String(pair[1]).replacingOccurrences(of: "+", with: " ")
            out[key] = rawVal.removingPercentEncoding ?? rawVal
        }
        return out
    }
}
