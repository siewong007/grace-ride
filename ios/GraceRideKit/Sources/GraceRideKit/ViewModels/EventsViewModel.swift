import Foundation
import Observation

public enum EventActionNotice: Equatable, Sendable {
    case published
    case unpublished
}

@MainActor
@Observable
public final class EventsViewModel {
    private let container: AppContainer
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?
    nonisolated(unsafe) private var loadTask: Task<Void, Never>?

    public private(set) var tab: Int = 0
    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var offline: Bool = false
    public private(set) var error: String?
    public private(set) var events: [EventItem] = []
    public private(set) var myRides: [Ride] = []
    public private(set) var myDrives: [Ride] = []
    public var selectedDay: String = "all"
    public private(set) var requestsByEvent: [Int64: Int] = [:]
    public private(set) var hideDrafts: Bool = false
    public private(set) var draftCount: Int = 0
    public private(set) var busyEventId: Int64?
    public private(set) var message: String?
    public private(set) var actionNotice: EventActionNotice?

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

    public func setTab(_ newTab: Int) async {
        guard newTab != tab else { return }
        tab = newTab
        selectedDay = "all"
        events = []
        myRides = []
        myDrives = []
        error = nil
        loading = true
        await load(first: true)
    }

    public func toggleHideDrafts() async {
        hideDrafts.toggle()
        refreshing = true
        await load(first: false)
    }

    public func publishEvent(id: Int64) async {
        guard busyEventId == nil else { return }
        busyEventId = id
        message = nil
        actionNotice = nil
        defer { busyEventId = nil }
        do {
            _ = try await container.events.publish(id: id)
            await reloadData(first: false)
            actionNotice = .published
        } catch let e as ApiException {
            message = e.message
        } catch {
            message = error.localizedDescription
        }
    }

    public func unpublishEvent(id: Int64) async {
        guard busyEventId == nil else { return }
        busyEventId = id
        message = nil
        actionNotice = nil
        defer { busyEventId = nil }
        do {
            _ = try await container.events.unpublish(id: id)
            if hideDrafts {
                draftCount += 1
            }
            await reloadData(first: false)
            actionNotice = .unpublished
        } catch let e as ApiException {
            message = e.message
        } catch {
            message = error.localizedDescription
        }
    }

    public func clearMessage() { message = nil }
    public func clearNotice() { actionNotice = nil }

    public func loadInitial() async {
        await load(first: true)
    }

    public func refresh() async {
        await load(first: false)
    }

    private func load(first: Bool) async {
        loadTask?.cancel()
        loading = first
        refreshing = !first
        let task = Task { [weak self] in
            guard let self else { return }
            await self.reloadData(first: first)
        }
        loadTask = task
        await task.value
    }

    private func reloadData(first: Bool) async {
        let curTab = tab
        let uid = container.auth.me?.id
        let cacheKey: String? = switch curTab {
        case 0: "events_upcoming"
        case 2: "events_past"
        default: nil
        }

        if first, let uid, let cacheKey,
           let cached = container.offlineCache.read([EventItem].self, userId: uid, key: cacheKey),
           events.isEmpty {
            loading = false
            refreshing = true
            events = cached.data
        } else {
            loading = first
            refreshing = !first
        }

        do {
            let curHideDrafts = hideDrafts
            let loadedEvents: [EventItem]
            switch curTab {
            case 2: loadedEvents = try await container.events.past()
            case 3: loadedEvents = try await container.events.managing(hideDrafts: curHideDrafts)
            default: loadedEvents = try await container.events.upcoming()
            }
            guard !Task.isCancelled, tab == curTab else { return }

            if let uid, let cacheKey {
                container.offlineCache.write(userId: uid, key: cacheKey, value: loadedEvents)
            }

            let nextDraftCount: Int = (curTab == 3 && !curHideDrafts)
                ? loadedEvents.filter { $0.status == "draft" }.count
                : draftCount

            var reqCounts: [Int64: Int] = [:]
            if curTab == 0, container.auth.me?.can("drive") == true {
                if let reqs = try? await container.rides.driverRequests(mineAreas: true) {
                    for r in reqs {
                        if let eid = r.event?.id {
                            reqCounts[eid, default: 0] += 1
                        }
                    }
                }
            }

            var loadedMyRides: [Ride] = []
            if curTab == 1, container.auth.me?.can("rides") == true {
                loadedMyRides = (try? await container.rides.upcoming()) ?? []
            }

            var loadedMyDrives: [Ride] = []
            if curTab == 1, container.auth.me?.can("drive") == true {
                loadedMyDrives = (try? await container.rides.drivingToday(from: KlTime.today(), to: KlTime.todayPlusDays(60))) ?? []
            }

            guard !Task.isCancelled, tab == curTab else { return }
            loading = false
            refreshing = false
            events = loadedEvents
            myRides = loadedMyRides
            myDrives = loadedMyDrives
            requestsByEvent = reqCounts
            draftCount = nextDraftCount
            error = nil
            offline = false
        } catch let e as ApiException {
            guard !Task.isCancelled else { return }
            loading = false
            refreshing = false
            error = e.status == 0 ? nil : e.message
            offline = e.status == 0
        } catch {
            guard !Task.isCancelled else { return }
            loading = false
            refreshing = false
            self.error = error.localizedDescription
        }
    }
}
