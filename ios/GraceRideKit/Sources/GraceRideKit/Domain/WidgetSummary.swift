import Foundation

public struct WidgetSummary: Codable, Equatable, Sendable {
    public var signedIn: Bool
    public var isRide: Bool
    public var rideId: Int64?
    public var eventId: Int64?
    public var scheduledAt: String
    public var badge: String
    public var title: String
    public var subtitle: String
    public var showReadyButton: Bool
    public var readiness: String?

    public init(
        signedIn: Bool = false,
        isRide: Bool = false,
        rideId: Int64? = nil,
        eventId: Int64? = nil,
        scheduledAt: String = "",
        badge: String = "",
        title: String = "",
        subtitle: String = "",
        showReadyButton: Bool = false,
        readiness: String? = nil
    ) {
        self.signedIn = signedIn
        self.isRide = isRide
        self.rideId = rideId
        self.eventId = eventId
        self.scheduledAt = scheduledAt
        self.badge = badge
        self.title = title
        self.subtitle = subtitle
        self.showReadyButton = showReadyButton
        self.readiness = readiness
    }
}

public enum WidgetSnapshotStore {
    public static let appGroupSuiteName = "group.org.pjstatecm.graceride"
    private static let keySummary = "widget_summary_v1"

    public static func saveSummary(_ summary: WidgetSummary, to store: SecureStore? = nil, userDefaults: UserDefaults? = UserDefaults(suiteName: appGroupSuiteName)) {
        guard let data = try? JSONEncoder().encode(summary),
              let raw = String(data: data, encoding: .utf8) else {
            return
        }
        store?.putString(keySummary, raw)
        userDefaults?.set(raw, forKey: keySummary)
    }

    public static func load(from store: SecureStore? = nil, userDefaults: UserDefaults? = UserDefaults(suiteName: appGroupSuiteName)) -> WidgetSummary {
        let raw = userDefaults?.string(forKey: keySummary) ?? store?.getString(keySummary)
        guard let raw, let data = raw.data(using: .utf8),
              let summary = try? JSONDecoder().decode(WidgetSummary.self, from: data) else {
            return WidgetSummary(signedIn: false)
        }
        return summary
    }

    public static func clear(store: SecureStore? = nil, userDefaults: UserDefaults? = UserDefaults(suiteName: appGroupSuiteName)) {
        store?.remove(keySummary)
        userDefaults?.removeObject(forKey: keySummary)
    }

    public static func saveFromHome(
        signedIn: Bool,
        nextRide: Ride?,
        nextEvent: EventItem?,
        lang: String,
        hideLockDetails: Bool = false,
        store: SecureStore? = nil,
        userDefaults: UserDefaults? = UserDefaults(suiteName: appGroupSuiteName)
    ) {
        guard signedIn else {
            clear(store: store, userDefaults: userDefaults)
            return
        }
        let summary: WidgetSummary
        if let ride = nextRide {
            if hideLockDetails {
                summary = WidgetSummary(
                    signedIn: true,
                    isRide: true,
                    rideId: ride.id,
                    scheduledAt: ride.pickupAt,
                    badge: lang.hasPrefix("zh") ? "下一趟行程" : "Next Ride",
                    title: lang.hasPrefix("zh") ? "即将有接送行程" : "Upcoming Ride",
                    subtitle: lang.hasPrefix("zh") ? "打开应用查看接送详情" : "Open app for pickup details",
                    showReadyButton: false,
                    readiness: ride.readinessStatus
                )
            } else {
                let timeLabel = KlTime.display(ride.pickupAt, lang: lang)
                let dest = ride.event?.title ?? (ride.destination.isEmpty ? ride.area.name : ride.destination)
                let statusStr = statusLabel(for: ride, lang: lang)
                summary = WidgetSummary(
                    signedIn: true,
                    isRide: true,
                    rideId: ride.id,
                    scheduledAt: ride.pickupAt,
                    badge: lang.hasPrefix("zh") ? "下一趟行程" : "Next Ride",
                    title: dest,
                    subtitle: "\(timeLabel) · \(statusStr)",
                    showReadyButton: ride.viewer.isPassenger && ride.status == "confirmed" && ride.readinessStatus != Readiness.ready,
                    readiness: ride.readinessStatus
                )
            }
        } else if let ev = nextEvent {
            if hideLockDetails {
                summary = WidgetSummary(
                    signedIn: true,
                    isRide: false,
                    eventId: ev.id,
                    scheduledAt: ev.startsAt,
                    badge: lang.hasPrefix("zh") ? "近期活动" : "Next Event",
                    title: lang.hasPrefix("zh") ? "即将举行的活动" : "Upcoming Event",
                    subtitle: lang.hasPrefix("zh") ? "打开应用查看活动详情" : "Open app for event details"
                )
            } else {
                summary = WidgetSummary(
                    signedIn: true,
                    isRide: false,
                    eventId: ev.id,
                    scheduledAt: ev.startsAt,
                    badge: lang.hasPrefix("zh") ? "近期活动" : "Next Event",
                    title: ev.title,
                    subtitle: "\(KlTime.display(ev.startsAt, lang: lang)) · \(ev.location)"
                )
            }
        } else {
            summary = WidgetSummary(signedIn: true)
        }
        saveSummary(summary, to: store, userDefaults: userDefaults)
    }

    public static func markReadyLocally(
        rideId: Int64,
        status: String,
        store: SecureStore? = nil,
        userDefaults: UserDefaults? = UserDefaults(suiteName: appGroupSuiteName)
    ) {
        var cur = load(from: store, userDefaults: userDefaults)
        guard cur.signedIn, cur.rideId == rideId else { return }
        cur.showReadyButton = false
        cur.readiness = status
        saveSummary(cur, to: store, userDefaults: userDefaults)
    }

    public static func freshForDisplay(_ summary: WidgetSummary, now: Date = Date()) -> WidgetSummary {
        guard summary.signedIn, !summary.scheduledAt.isEmpty,
              let dt = KlTime.parse(summary.scheduledAt) else {
            return summary
        }
        let cal = KlTime.klCalendar
        if summary.isRide {
            if let sixHoursAfter = cal.date(byAdding: .hour, value: 6, to: dt), now > sixHoursAfter {
                return WidgetSummary(signedIn: true)
            }
            if let twoHoursAfter = cal.date(byAdding: .hour, value: 2, to: dt), now > twoHoursAfter {
                var copy = summary
                copy.showReadyButton = false
                return copy
            }
        } else {
            if let twelveHoursAfter = cal.date(byAdding: .hour, value: 12, to: dt), now > twelveHoursAfter {
                return WidgetSummary(signedIn: true)
            }
        }
        return summary
    }

    private static func statusLabel(for ride: Ride, lang: String) -> String {
        let zh = lang.hasPrefix("zh")
        switch ride.status {
        case "confirmed":
            return zh ? "已确认" : "Confirmed"
        case "in_progress":
            return zh ? "进行中" : "On the way"
        default:
            return ride.status.replacingOccurrences(of: "_", with: " ").capitalized
        }
    }
}
