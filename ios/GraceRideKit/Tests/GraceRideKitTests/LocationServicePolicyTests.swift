import XCTest
@testable import GraceRideKit

final class LocationServicePolicyTests: XCTestCase {
    private let signedInMe = Me(id: 1, name: "Driver", email: "driver@example.org", termsAccepted: true)

    private func sampleRide(
        status: String = "in_progress",
        isPassenger: Bool = true,
        isDriver: Bool = false,
        pickedUpAt: String? = nil
    ) -> Ride {
        Ride(
            id: 10,
            status: status,
            pickupAt: "2026-11-25 09:00:00",
            area: RideArea(id: 1, name: "SS2"),
            destination: "Church Hall",
            passenger: RidePassenger(id: 2, name: "Passenger"),
            pickedUpAt: pickedUpAt,
            viewer: RideViewer(isPassenger: isPassenger, isDriver: isDriver)
        )
    }

    func testDriverLocationStrictlyRequiresSignedInAcceptedTermsAndLiveRoute() {
        let liveRoute = TripRouteDto(tripId: 5, live: true)
        let idleRoute = TripRouteDto(tripId: 5, live: false)

        XCTAssertFalse(LocationServicePolicy.hasActiveJourneyTarget(rideId: -1, tripId: -1))
        XCTAssertTrue(LocationServicePolicy.hasActiveJourneyTarget(rideId: 10, tripId: -1))
        XCTAssertTrue(LocationServicePolicy.canShareDriverLocation(authState: .signedIn(signedInMe), route: liveRoute))
        XCTAssertFalse(LocationServicePolicy.canShareDriverLocation(authState: .signedIn(signedInMe), route: idleRoute))
        XCTAssertFalse(LocationServicePolicy.canShareDriverLocation(authState: .signedOut, route: liveRoute))
        XCTAssertFalse(LocationServicePolicy.canShareDriverLocation(authState: .locked(signedInMe), route: liveRoute))

        var unaccepted = signedInMe
        unaccepted.termsAccepted = false
        XCTAssertFalse(LocationServicePolicy.canShareDriverLocation(authState: .signedIn(unaccepted), route: liveRoute))
    }

    func testPassengerWaitingPinAndWatchdogPolicy() {
        XCTAssertTrue(LocationServicePolicy.canShareRideLocation(sampleRide(status: "in_progress")))
        XCTAssertFalse(LocationServicePolicy.canShareRideLocation(sampleRide(status: "confirmed")))
        XCTAssertTrue(LocationServicePolicy.canSharePassengerWaitingSpot(sampleRide(status: "in_progress", isPassenger: true, pickedUpAt: nil)))
        XCTAssertFalse(LocationServicePolicy.canSharePassengerWaitingSpot(sampleRide(status: "in_progress", isPassenger: true, pickedUpAt: "2026-11-25 09:10:00")))
        XCTAssertFalse(LocationServicePolicy.canSharePassengerWaitingSpot(sampleRide(status: "in_progress", isPassenger: false, isDriver: true)))

        let auth = AuthState.signedIn(signedInMe)
        XCTAssertEqual(LocationServicePolicy.evaluateJourneyCheck(authState: auth, routeLive: true, authRejected: false, consecutiveNetworkErrors: 0), .keepRunning)
        XCTAssertEqual(LocationServicePolicy.evaluateJourneyCheck(authState: auth, routeLive: false, authRejected: false, consecutiveNetworkErrors: 0), .stopJourneyEnded)
        XCTAssertEqual(LocationServicePolicy.evaluateJourneyCheck(authState: auth, routeLive: nil, authRejected: false, consecutiveNetworkErrors: 3), .stopJourneyEnded)
        XCTAssertEqual(LocationServicePolicy.evaluateJourneyCheck(authState: .locked(signedInMe), routeLive: true, authRejected: false, consecutiveNetworkErrors: 0), .stopAuthInvalid)
    }
}
