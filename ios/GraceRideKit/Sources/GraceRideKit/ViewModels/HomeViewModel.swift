import Foundation
import Observation

@MainActor
@Observable
public final class HomeViewModel {
    private let container: AppContainer
    nonisolated(unsafe) private var loadTask: Task<Void, Never>?
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var offline: Bool = false
    public private(set) var offlineSnapshotAt: Int64?
    public private(set) var error: String?
    public private(set) var me: Me?
    public private(set) var nextRide: Ride?
    public private(set) var drivingToday: [Ride] = []
    public private(set) var nextUpcomingDrive: Ride?
    public private(set) var upcomingDrives: [Ride] = []
    public private(set) var events: [EventItem] = []
    public private(set) var unpaidShares: [Share] = []
    public private(set) var unpaidShareCount: Int = 0
    public private(set) var announcements: [Announcement] = []

    public init(container: AppContainer, startObservers: Bool = true) {
        self.container = container
        if startObservers {
            realtimeTask = Task { [weak self] in
                let stream = container.realtimeStream.events()
                for await ev in stream where ev != .resume {
                    await self?.refresh()
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
        loadTask?.cancel()
        realtimeTask?.cancel()
        networkTask?.cancel()
    }

    public func loadInitial() async {
        await load(first: true)
    }

    public func refresh() async {
        await load(first: false)
    }

    private struct SectionResult<T> {
        let data: T
        let ok: Bool
        let offline: Bool
        let error: String?
    }

    private func fetchSection<T>(_ allowed: Bool, fallback: T, _ block: () async throws -> T) async -> SectionResult<T> {
        guard allowed else {
            return SectionResult(data: fallback, ok: false, offline: false, error: nil)
        }
        do {
            return SectionResult(data: try await block(), ok: true, offline: false, error: nil)
        } catch let e as ApiException {
            return SectionResult(data: fallback, ok: false, offline: e.status == 0, error: e.status == 0 ? nil : e.message)
        } catch {
            return SectionResult(data: fallback, ok: false, offline: false, error: error.localizedDescription)
        }
    }

    private func load(first: Bool) async {
        loadTask?.cancel()
        loading = first && me == nil
        refreshing = !first

        if !first {
            try? await container.auth.refresh(throwOnError: false)
        }
        let currentMe = container.auth.me
        let today = KlTime.today()
        let nextWeek = KlTime.todayPlusDays(7)

        async let rSec = fetchSection(currentMe?.can("rides") == true, fallback: [Ride]()) {
            try await container.rides.upcoming()
        }
        async let eSec = fetchSection(currentMe?.can("events") == true, fallback: [EventItem]()) {
            try await container.events.upcoming()
        }
        async let sSec = fetchSection(currentMe?.can("costs") == true, fallback: [Share]()) {
            try await container.shares.mine()
        }
        async let aSec = fetchSection(true, fallback: [Announcement]()) {
            try await container.community.announcements()
        }
        async let dSec = fetchSection(currentMe?.can("drive") == true, fallback: [Ride]()) {
            try await container.rides.drivingToday(from: today, to: nextWeek)
        }

        let (ridesRes, eventsRes, sharesRes, annRes, drivingRes) = await (rSec, eSec, sSec, aSec, dSec)

        var effectiveRides = ridesRes.data
        var effectiveDrivingAll = drivingRes.data
        var effectiveEvents = eventsRes.data
        var effectiveAnn = annRes.data
        let isOffline = ridesRes.offline || eventsRes.offline || sharesRes.offline || annRes.offline || drivingRes.offline
        let firstErr = [ridesRes.error, eventsRes.error, sharesRes.error, annRes.error, drivingRes.error].compactMap { $0 }.first
        var snapshotAt: Int64?

        if let m = currentMe {
            if ridesRes.ok || drivingRes.ok {
                OfflineSnapshotStore.save(store: container.secureStore, userId: m.id, rides: ridesRes.data + drivingRes.data)
            } else if isOffline && effectiveRides.isEmpty && effectiveDrivingAll.isEmpty {
                if let snap = OfflineSnapshotStore.load(store: container.secureStore, userId: m.id) {
                    effectiveRides = snap.rides.filter { $0.viewer.isPassenger }
                    effectiveDrivingAll = snap.rides.filter { $0.viewer.isDriver }
                    snapshotAt = snap.savedAtMillis
                }
            }
            if eventsRes.ok {
                container.offlineCache.write(userId: m.id, key: "home_events", value: eventsRes.data)
            } else if isOffline && effectiveEvents.isEmpty {
                if let cached = container.offlineCache.read([EventItem].self, userId: m.id, key: "home_events") {
                    effectiveEvents = cached.data
                }
            }
            if annRes.ok {
                container.offlineCache.write(userId: m.id, key: "home_announcements", value: annRes.data)
            } else if isOffline && effectiveAnn.isEmpty {
                if let cached = container.offlineCache.read([Announcement].self, userId: m.id, key: "home_announcements") {
                    effectiveAnn = cached.data
                }
            }
        }

        let todayDrives = effectiveDrivingAll.filter { String($0.pickupAt.prefix(10)) == today }
        let futureDrives = effectiveDrivingAll
            .filter { String($0.pickupAt.prefix(10)) > today && RideStatus.active.contains($0.status) }
            .sorted { $0.pickupAt < $1.pickupAt }
        let nextDrive = futureDrives.first
        let nextPassengerRide = effectiveRides
            .filter { RideStatus.active.contains($0.status) && $0.viewer.isPassenger }
            .min { $0.pickupAt < $1.pickupAt }

        let allUnpaid = sharesRes.data.filter { $0.status == "unpaid" || $0.status == "needs_info" }
        let topUnpaid = Array(allUnpaid.prefix(3))
        let topEvents = Array(effectiveEvents.prefix(3))
        let topAnn = Array(effectiveAnn.sorted { ($0.pinned ? 1 : 0) > ($1.pinned ? 1 : 0) }.prefix(3))

        self.loading = false
        self.refreshing = false
        self.offline = isOffline
        self.offlineSnapshotAt = snapshotAt
        self.error = (effectiveRides.isEmpty && effectiveEvents.isEmpty && topUnpaid.isEmpty && effectiveAnn.isEmpty && effectiveDrivingAll.isEmpty) ? firstErr : nil
        self.me = currentMe
        self.nextRide = nextPassengerRide
        self.drivingToday = todayDrives
        self.nextUpcomingDrive = nextDrive
        self.upcomingDrives = futureDrives
        self.events = topEvents
        self.unpaidShares = topUnpaid
        self.unpaidShareCount = allUnpaid.count
        self.announcements = topAnn

        WidgetSnapshotStore.saveFromHome(
            signedIn: currentMe != nil,
            nextRide: nextPassengerRide,
            nextEvent: effectiveEvents.first,
            lang: container.lang,
            hideLockDetails: container.secureStore.getString(AuthRepository.hideLockDetailsKey) == "1",
            store: container.secureStore,
            userDefaults: container.widgetUserDefaults
        )
        container.onWidgetChanged?()
    }
}
