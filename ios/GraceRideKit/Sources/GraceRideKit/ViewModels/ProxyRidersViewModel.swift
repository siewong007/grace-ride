import Foundation
import Observation

@MainActor
@Observable
public final class ProxyRidersViewModel {
    private let container: AppContainer
    nonisolated(unsafe) private var searchTask: Task<Void, Never>?
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var riders: [ProxyRider] = []
    public private(set) var areas: [Area] = []
    public private(set) var suggestions: [Place] = []
    public private(set) var busy: Bool = false
    public private(set) var message: String?
    public private(set) var error: String?

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
        searchTask?.cancel()
        realtimeTask?.cancel()
        networkTask?.cancel()
    }

    public func load() async { await loadInternal(refreshing: false) }
    public func refresh() async { await loadInternal(refreshing: !riders.isEmpty) }

    private func loadInternal(refreshing: Bool) async {
        let uid = container.auth.me?.id
        if riders.isEmpty, let uid,
           let cached = container.offlineCache.read([ProxyRider].self, userId: uid, key: "proxy_riders") {
            riders = cached.data
            loading = false
        }
        loading = riders.isEmpty && !refreshing
        self.refreshing = refreshing
        error = nil

        do {
            let fallbackAreas = areas
            async let ridersDef = container.rides.proxyRiders()
            async let areasDef: [Area] = (try? await container.rides.areas().filter(\.active)) ?? fallbackAreas
            let (loadedRiders, loadedAreas) = try await (ridersDef, areasDef)
            if let uid {
                container.offlineCache.write(userId: uid, key: "proxy_riders", value: loadedRiders)
            }
            loading = false
            self.refreshing = false
            riders = loadedRiders
            areas = loadedAreas
        } catch let e as ApiException {
            loading = false
            self.refreshing = false
            error = e.message
        } catch {
            loading = false
            self.refreshing = false
            self.error = error.localizedDescription
        }
    }

    public func searchAddress(_ q: String) {
        searchTask?.cancel()
        let trimmed = q.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count >= 3 else {
            suggestions = []
            return
        }
        searchTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 400_000_000)
            guard !Task.isCancelled, let self else { return }
            self.suggestions = (try? await self.container.rides.places(query: trimmed)) ?? []
        }
    }

    public func clearSuggestions() {
        searchTask?.cancel()
        suggestions = []
    }

    public func clearMessage() { message = nil }
    public func clearError() { error = nil }

    public func save(editingId: Int64?, req: ProxyRiderReq) async -> Bool {
        guard !busy else { return false }
        let cleanName = req.name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !cleanName.isEmpty else { return false }
        busy = true
        error = nil
        defer { busy = false }
        do {
            var payload = req
            payload.name = cleanName
            payload.phone = req.phone?.trimmingCharacters(in: .whitespacesAndNewlines)
            payload.relationship = req.relationship?.trimmingCharacters(in: .whitespacesAndNewlines)
            payload.pickupAddress = req.pickupAddress?.trimmingCharacters(in: .whitespacesAndNewlines)
            payload.gateNotes = req.gateNotes?.trimmingCharacters(in: .whitespacesAndNewlines)
            payload.notes = req.notes?.trimmingCharacters(in: .whitespacesAndNewlines)

            if let editingId {
                _ = try await container.rides.updateProxyRider(id: editingId, payload)
            } else {
                _ = try await container.rides.createProxyRider(payload)
            }
            let updated = try await container.rides.proxyRiders()
            if let uid = container.auth.me?.id {
                container.offlineCache.write(userId: uid, key: "proxy_riders", value: updated)
            }
            riders = updated
            suggestions = []
            message = "saved"
            return true
        } catch let e as ApiException {
            error = e.message
            return false
        } catch {
            self.error = error.localizedDescription
            return false
        }
    }

    public func delete(id: Int64) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        defer { busy = false }
        do {
            _ = try await container.rides.deleteProxyRider(id: id)
            let updated = try await container.rides.proxyRiders()
            if let uid = container.auth.me?.id {
                container.offlineCache.write(userId: uid, key: "proxy_riders", value: updated)
            }
            riders = updated
            message = "removed"
            return true
        } catch let e as ApiException {
            error = e.message
            return false
        } catch {
            self.error = error.localizedDescription
            return false
        }
    }
}
