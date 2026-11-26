import Foundation
import Observation

@MainActor
@Observable
public final class DriveHubViewModel {
    private let container: AppContainer
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?

    public var tab: Int = 0
    public var mineAreasOnly: Bool = true
    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var busyRideId: Int64?
    public private(set) var requests: [Ride] = []
    public private(set) var schedule: [Ride] = []
    public private(set) var route: TripRouteDto?
    public private(set) var offline: Bool = false
    public private(set) var accessDenied: Bool = false
    public private(set) var error: String?
    public var claimedNotice: Bool = false

    public init(container: AppContainer, startObservers: Bool = true) {
        self.container = container
        if startObservers {
            realtimeTask = Task { [weak self] in
                let stream = container.realtimeStream.events()
                for await ev in stream where ev != .resume {
                    await self?.load(first: false)
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

    public func toggleMineAreas() async {
        mineAreasOnly.toggle()
        await load(first: false)
    }

    public func refresh() async {
        await load(first: false)
    }

    public func load(first: Bool = true) async {
        guard container.auth.me?.can("drive", act: "view") == true else {
            loading = false
            refreshing = false
            accessDenied = true
            error = nil
            return
        }
        accessDenied = false
        loading = first && requests.isEmpty && schedule.isEmpty
        refreshing = !first || !requests.isEmpty || !schedule.isEmpty
        error = nil

        let mineOnly = mineAreasOnly
        let from = KlTime.today()
        let to = KlTime.todayPlusDays(14)

        do {
            async let reqDef = container.rides.driverRequests(mineAreas: mineOnly)
            async let schedDef = container.rides.drivingToday(from: from, to: to)
            async let routeDef: TripRouteDto? = (try? await container.rides.driverRoute().route)

            let (reqs, sched, routeResp) = try await (reqDef, schedDef, routeDef)
            if !LocationServicePolicy.isJourneyLive(routeResp) && container.isLiveLocationSharing {
                container.onStopLiveLocation?(false)
            }
            loading = false
            refreshing = false
            requests = reqs
            schedule = sched.sorted { $0.pickupAt < $1.pickupAt }
            route = routeResp
            offline = false
            error = nil
        } catch let e as ApiException {
            loading = false
            refreshing = false
            offline = e.status == 0
            error = e.status == 0 ? nil : e.message
        } catch {
            loading = false
            refreshing = false
            self.error = error.localizedDescription
        }
    }

    public func claim(rideId: Int64) async {
        guard busyRideId == nil else { return }
        busyRideId = rideId
        error = nil
        claimedNotice = false
        do {
            _ = try await container.rides.claimRide(id: rideId)
            busyRideId = nil
            claimedNotice = true
            await load(first: false)
        } catch let e as ApiException {
            busyRideId = nil
            error = e.message
        } catch {
            busyRideId = nil
            self.error = error.localizedDescription
        }
    }

    public func clearClaimedNotice() { claimedNotice = false }
    public func clearError() { error = nil }
}
