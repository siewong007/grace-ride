import Foundation

public enum WatchdogAction: Equatable, Sendable {
    case keepRunning
    case stopAuthInvalid
    case stopJourneyEnded
}

/// Pure policy engine for active-journey GPS sharing on iOS, enforcing the exact
/// journey-only boundaries required by the PDPA v3 Terms & Privacy Notice.
public enum LocationServicePolicy {
    public static let maxConsecutiveErrors = 3

    public static func hasActiveJourneyTarget(rideId: Int64, tripId: Int64) -> Bool {
        rideId > 0 || tripId > 0
    }

    public static func isJourneyLive(_ route: TripRouteDto?) -> Bool {
        route?.live == true
    }

    public static func canShareDriverLocation(authState: AuthState, route: TripRouteDto?) -> Bool {
        if case .signedIn(let me) = authState {
            return me.termsAccepted && route?.live == true
        }
        return false
    }

    public static func canShareRideLocation(_ ride: Ride?) -> Bool {
        ride?.status == "in_progress"
    }

    public static func canSharePassengerWaitingSpot(_ ride: Ride?) -> Bool {
        guard let ride else { return false }
        return ride.status == "in_progress" && ride.viewer.isPassenger && ride.pickedUpAt == nil
    }

    public static func evaluateJourneyCheck(
        authState: AuthState,
        routeLive: Bool?,
        authRejected: Bool,
        consecutiveNetworkErrors: Int,
        maxConsecutiveNetworkErrors: Int = maxConsecutiveErrors
    ) -> WatchdogAction {
        guard case .signedIn(let me) = authState, me.termsAccepted, !authRejected else {
            return .stopAuthInvalid
        }
        if routeLive == false || (routeLive == nil && consecutiveNetworkErrors >= maxConsecutiveNetworkErrors) {
            return .stopJourneyEnded
        }
        return .keepRunning
    }
}
