import Foundation
import Observation

@MainActor
@Observable
public final class RidesViewModel {
    private let container: AppContainer
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?

    public private(set) var tab: Int = 0
    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var offline: Bool = false
    public private(set) var error: String?
    public private(set) var rides: [Ride] = []

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
        realtimeTask?.cancel()
        networkTask?.cancel()
    }

    public func setTab(_ newTab: Int) async {
        guard newTab != tab else { return }
        tab = newTab
        rides = []
        error = nil
        loading = true
        await load(first: true)
    }

    public func loadInitial() async {
        await load(first: true)
    }

    public func refresh() async {
        await load(first: false)
    }

    private func load(first: Bool) async {
        let curTab = tab
        let cacheKey = curTab == 0 ? "rides_upcoming" : "rides_past"
        let uid = container.auth.me?.id

        if first, let uid, let cached = container.offlineCache.read([Ride].self, userId: uid, key: cacheKey), rides.isEmpty {
            loading = false
            refreshing = true
            rides = cached.data
        } else {
            loading = first && rides.isEmpty
            refreshing = !first || !rides.isEmpty
        }

        do {
            let fresh = try await (curTab == 0 ? container.rides.upcoming() : container.rides.past())
            if let uid {
                container.offlineCache.write(userId: uid, key: cacheKey, value: fresh)
            }
            loading = false
            refreshing = false
            rides = fresh
            error = nil
            offline = false
        } catch let e as ApiException {
            loading = false
            refreshing = false
            error = e.status == 0 ? nil : e.message
            offline = e.status == 0
        } catch {
            loading = false
            refreshing = false
            self.error = error.localizedDescription
        }
    }
}
