import Foundation
import Observation

@MainActor
@Observable
public final class NotificationsViewModel {
    private struct PendingDelete {
        let item: NotificationItem
        let index: Int
    }

    private let container: AppContainer
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    private var pendingDeletes: [Int64: PendingDelete] = [:]

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var loadingMore: Bool = false
    public private(set) var loadMoreError: Bool = false
    public private(set) var items: [NotificationItem] = []
    public private(set) var nextBefore: Int64?
    public private(set) var unreadCount: Int = 0
    public private(set) var unreadOnly: Bool = false
    public private(set) var category: String?
    public private(set) var error: String?
    public private(set) var deletedItemForUndo: NotificationItem?

    public init(container: AppContainer, startObservers: Bool = true) {
        self.container = container
        if startObservers {
            realtimeTask = Task { [weak self] in
                let stream = container.realtimeStream.events()
                for await ev in stream where ev == .notify || ev == .resume {
                    await self?.refresh()
                }
            }
        }
    }

    deinit {
        realtimeTask?.cancel()
    }

    public func clearError() { error = nil }

    public func load(reset: Bool = true) async {
        let targetUnreadOnly = unreadOnly
        let targetCategory = category
        loading = reset && items.isEmpty
        refreshing = !reset
        error = nil

        do {
            let page = try await container.community.notificationsPage(
                before: nil,
                limit: 30,
                unreadOnly: targetUnreadOnly,
                category: targetCategory
            )
            guard unreadOnly == targetUnreadOnly, category == targetCategory else { return }
            let visible = page.items.filter { pendingDeletes[$0.id] == nil }
            let pendingUnread = pendingDeletes.values.filter { !$0.item.read }.count
            let effectiveUnread = max(0, page.unread - pendingUnread)
            container.auth.updateUnread(effectiveUnread)
            loading = false
            refreshing = false
            loadMoreError = false
            items = visible
            nextBefore = page.nextBefore
            unreadCount = effectiveUnread
        } catch let e as ApiException {
            guard unreadOnly == targetUnreadOnly, category == targetCategory else { return }
            loading = false
            refreshing = false
            error = e.message
        } catch {
            guard unreadOnly == targetUnreadOnly, category == targetCategory else { return }
            loading = false
            refreshing = false
            self.error = error.localizedDescription
        }
    }

    public func refresh() async {
        await load(reset: false)
    }

    public func loadMore() async {
        guard let before = nextBefore, !loadingMore else { return }
        let targetUnreadOnly = unreadOnly
        let targetCategory = category
        loadingMore = true
        loadMoreError = false

        do {
            let page = try await container.community.notificationsPage(
                before: before,
                limit: 30,
                unreadOnly: targetUnreadOnly,
                category: targetCategory
            )
            guard unreadOnly == targetUnreadOnly, category == targetCategory else {
                loadingMore = false
                return
            }
            let existingIds = Set(items.map(\.id))
            let merged = items + page.items.filter { !existingIds.contains($0.id) && pendingDeletes[$0.id] == nil }
            loadingMore = false
            items = merged
            nextBefore = page.nextBefore
            unreadCount = page.unread
        } catch {
            if unreadOnly == targetUnreadOnly, category == targetCategory {
                loadingMore = false
                loadMoreError = true
            }
        }
    }

    public func setUnreadOnly(_ enabled: Bool) async {
        guard unreadOnly != enabled else { return }
        unreadOnly = enabled
        items = []
        nextBefore = nil
        error = nil
        loading = true
        await load(reset: true)
    }

    public func setCategory(_ newCategory: String?) async {
        let normalized = newCategory?.trimmingCharacters(in: .whitespacesAndNewlines)
        let effective = (normalized?.isEmpty == false) ? normalized : nil
        guard category != effective else { return }
        category = effective
        items = []
        nextBefore = nil
        error = nil
        loading = true
        await load(reset: true)
    }

    public func resetFilters() async {
        guard unreadOnly || category != nil else { return }
        unreadOnly = false
        category = nil
        items = []
        nextBefore = nil
        error = nil
        loading = true
        await load(reset: true)
    }

    public func toggleRead(_ item: NotificationItem) async {
        let nextRead = !item.read
        if unreadOnly && nextRead {
            items.removeAll { $0.id == item.id }
        } else {
            items = items.map {
                guard $0.id == item.id else { return $0 }
                var copy = $0
                copy.read = nextRead
                return copy
            }
        }
        unreadCount = max(0, unreadCount + (nextRead ? -1 : 1))
        do {
            let res = nextRead
                ? try await container.community.readOne(id: item.id)
                : try await container.community.unreadOne(id: item.id)
            container.auth.updateUnread(res.unread)
            unreadCount = res.unread
        } catch {
            await load(reset: false)
        }
    }

    public func stageDelete(_ item: NotificationItem) async {
        let existingKeys = Array(pendingDeletes.keys)
        for pendingId in existingKeys where pendingId != item.id {
            await commitDelete(id: pendingId)
        }
        let idx = items.firstIndex { $0.id == item.id } ?? 0
        pendingDeletes[item.id] = PendingDelete(item: item, index: idx)
        items.removeAll { $0.id == item.id }
        if !item.read {
            unreadCount = max(0, unreadCount - 1)
        }
        deletedItemForUndo = item
    }

    public func undoDelete(id: Int64) {
        guard let pending = pendingDeletes.removeValue(forKey: id) else { return }
        if !items.contains(where: { $0.id == id }) {
            let safeIdx = min(max(0, pending.index), items.count)
            items.insert(pending.item, at: safeIdx)
        }
        if !pending.item.read {
            unreadCount += 1
        }
        if deletedItemForUndo?.id == id {
            deletedItemForUndo = nil
        }
    }

    public func commitDelete(id: Int64) async {
        guard pendingDeletes.removeValue(forKey: id) != nil else { return }
        if deletedItemForUndo?.id == id {
            deletedItemForUndo = nil
        }
        do {
            let res = try await container.community.deleteOne(id: id)
            container.auth.updateUnread(res.unread)
            unreadCount = res.unread
        } catch {
            await load(reset: false)
        }
    }

    public func commitAllPendingDeletes() async {
        for id in Array(pendingDeletes.keys) {
            await commitDelete(id: id)
        }
    }

    public func deleteImmediate(_ item: NotificationItem) async {
        items.removeAll { $0.id == item.id }
        if !item.read { unreadCount = max(0, unreadCount - 1) }
        do {
            let res = try await container.community.deleteOne(id: item.id)
            container.auth.updateUnread(res.unread)
            unreadCount = res.unread
        } catch {
            await load(reset: false)
        }
    }

    public func markAllRead() async {
        let clearedCount = items.filter { !$0.read }.count
        if unreadOnly {
            items = []
        } else {
            items = items.map {
                var copy = $0
                copy.read = true
                return copy
            }
        }
        unreadCount = category == nil ? 0 : max(0, unreadCount - clearedCount)
        do {
            let res = try await container.community.readAll(category: category)
            container.auth.updateUnread(res.unread)
            unreadCount = res.unread
        } catch {
            await load(reset: false)
        }
    }

    public func open(_ item: NotificationItem) async -> NotifDestination {
        if !item.read {
            await toggleRead(item)
        }
        if let me = container.auth.me {
            return NotifLinkRouter.resolveFor(item.link, me: me)
        }
        return NotifLinkRouter.resolve(item.link)
    }
}
