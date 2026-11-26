import Foundation
import Observation

@MainActor
@Observable
public final class RideDetailViewModel {
    private let container: AppContainer
    public let rideId: Int64
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var ride: Ride?
    public private(set) var liveLocation: RideLocationDto?
    public private(set) var sharingPassengerPin: Bool = false
    public private(set) var error: String?
    public private(set) var busy: Bool = false
    public private(set) var message: String?
    public private(set) var staleReload: Bool = false

    public init(container: AppContainer, rideId: Int64, startObservers: Bool = true) {
        self.container = container
        self.rideId = rideId
        if startObservers {
            realtimeTask = Task { [weak self] in
                let stream = container.realtimeStream.events()
                for await ev in stream where ev != .resume {
                    await self?.load()
                }
            }
            networkTask = Task { [weak self] in
                let stream = container.networkMonitor.onlineStream()
                for await online in stream where online {
                    await self?.refresh()
                }
            }
        }
    }

    deinit {
        realtimeTask?.cancel()
        networkTask?.cancel()
    }

    public func load() async {
        loading = ride == nil
        await reloadRide()
    }

    public func refresh() async {
        refreshing = ride != nil
        await reloadRide()
    }

    private func reloadRide() async {
        do {
            let loaded = try await container.rides.detail(id: rideId)
            let canViewLoc = loaded.viewer.isPassenger || loaded.viewer.isDriver || loaded.viewer.isStaff
            let loc: RideLocationDto?
            if LocationServicePolicy.canShareRideLocation(loaded) && canViewLoc {
                loc = try? await container.rides.rideLocation(id: rideId)
            } else {
                loc = nil
            }
            if loaded.viewer.isDriver && !LocationServicePolicy.canShareRideLocation(loaded) {
                await stopLiveLocationIfNoActiveTrip()
            }
            loading = false
            refreshing = false
            ride = loaded
            liveLocation = loc
            sharingPassengerPin = loc?.passenger != nil
            error = nil
        } catch let e as ApiException {
            loading = false
            refreshing = false
            error = e.message
        } catch {
            loading = false
            refreshing = false
            self.error = error.localizedDescription
        }
    }

    private func runAction(_ block: () async throws -> Void) async {
        guard !busy else { return }
        busy = true
        message = nil
        defer { busy = false }
        do {
            try await block()
            await reloadRide()
        } catch let e as ApiException {
            if e.code == "RIDE_STATUS_CHANGED" {
                staleReload = true
                await reloadRide()
            } else {
                message = e.message
            }
        } catch {
            message = error.localizedDescription
        }
    }

    private func stopLiveLocationIfNoActiveTrip() async {
        guard container.isLiveLocationSharing else { return }
        let stillLive = (try? await container.rides.driverRoute().route?.live) == true
        if !stillLive {
            container.onStopLiveLocation?(false)
        }
    }

    public func cancel(reason: String?) async {
        await runAction {
            _ = try await container.rides.cancel(id: rideId, reason: reason)
            await stopLiveLocationIfNoActiveTrip()
        }
    }

    public func respond(accept: Bool) async {
        await runAction {
            _ = try await container.rides.respond(id: rideId, accept: accept)
        }
    }

    public func readiness(status: String) async {
        await runAction {
            _ = try await container.rides.readiness(id: rideId, status: status)
        }
    }

    public func confirmAssignment() async {
        await runAction { _ = try await container.rides.confirmAssignment(id: rideId) }
    }

    public func startRide() async {
        await runAction { _ = try await container.rides.startRide(id: rideId) }
    }

    public func arrivedGate() async {
        await runAction { _ = try await container.rides.arrivedGate(id: rideId) }
    }

    public func pickup() async {
        await runAction { _ = try await container.rides.pickupRide(id: rideId) }
    }

    public func complete() async {
        await runAction {
            _ = try await container.rides.completeRide(id: rideId)
            await stopLiveLocationIfNoActiveTrip()
        }
    }

    public func driverDelay(minutes: Int) async {
        await runAction { _ = try await container.rides.driverDelay(id: rideId, minutes: minutes) }
    }

    public func withdraw(reason: String?) async {
        await runAction {
            _ = try await container.rides.withdrawRide(id: rideId, reason: reason)
            await stopLiveLocationIfNoActiveTrip()
        }
    }

    public func refreshLiveLocation() async {
        guard let r = ride else { return }
        let canViewLoc = r.viewer.isPassenger || r.viewer.isDriver || r.viewer.isStaff
        guard LocationServicePolicy.canShareRideLocation(r), canViewLoc else { return }
        if let loc = try? await container.rides.rideLocation(id: rideId) {
            liveLocation = loc
            sharingPassengerPin = loc.passenger != nil
        }
    }

    public func sharePassengerSpot(lat: Double, lng: Double) async {
        guard LocationServicePolicy.canSharePassengerWaitingSpot(ride) else { return }
        do {
            let res = try await container.rides.postPassengerLocation(id: rideId, lat: lat, lng: lng)
            sharingPassengerPin = res.sharing
            await refreshLiveLocation()
        } catch let e as ApiException {
            message = e.message
        } catch {
            message = error.localizedDescription
        }
    }

    public func stopPassengerSpot() async {
        _ = try? await container.rides.stopPassengerLocation(id: rideId)
        sharingPassengerPin = false
        await refreshLiveLocation()
    }

    public func clearMessage() {
        message = nil
        staleReload = false
    }
}
