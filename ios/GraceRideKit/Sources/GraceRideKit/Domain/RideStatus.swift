import Foundation

public enum RideStatusKind: Equatable, Sendable {
    case active, pending, done, cancelled
}

public enum RideStatus {
    public static let active: Set<String> = [
        "pending_driver", "pending_passenger", "pending_approval",
        "pending_driver_confirm", "confirmed", "in_progress",
    ]
    public static let terminal: Set<String> = [
        "completed", "cancelled", "no_show", "alt_transport",
    ]

    public static func isTerminal(_ status: String) -> Bool {
        terminal.contains(status)
    }

    public static func kind(_ status: String) -> RideStatusKind {
        switch status {
        case "confirmed", "in_progress": return .active
        case "completed": return .done
        case "cancelled", "no_show", "alt_transport": return .cancelled
        default: return .pending
        }
    }

    public static func cancellable(_ status: String) -> Bool {
        ["pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm", "confirmed"].contains(status)
    }

    public static func isLiveTrackable(_ status: String) -> Bool {
        ["confirmed", "in_progress"].contains(status)
    }

    public static func stepIndex(_ status: String) -> Int {
        switch status {
        case "pending_approval", "pending_driver", "pending_driver_confirm", "pending_passenger":
            return 0
        case "confirmed":
            return 1
        case "in_progress":
            return 2
        case "completed":
            return 3
        default:
            return -1
        }
    }
}
