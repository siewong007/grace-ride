import Foundation
import Observation

@MainActor
@Observable
public final class SavedAddressesViewModel {
    private let container: AppContainer
    nonisolated(unsafe) private var searchTask: Task<Void, Never>?
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var places: [SavedPlace] = []
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
    public func refresh() async { await loadInternal(refreshing: !places.isEmpty) }

    private func loadInternal(refreshing: Bool) async {
        let uid = container.auth.me?.id
        if !refreshing, let uid, places.isEmpty,
           let cached = container.offlineCache.read([SavedPlace].self, userId: uid, key: "saved_places") {
            places = cached.data
            loading = false
            self.refreshing = true
        } else {
            loading = places.isEmpty && !refreshing
            self.refreshing = refreshing
        }
        error = nil

        do {
            let fallbackAreas = areas
            async let placesDef = container.rides.savedPlaces()
            async let areasDef: [Area] = (try? await container.rides.areas().filter(\.active)) ?? fallbackAreas
            let (loadedPlaces, loadedAreas) = try await (placesDef, areasDef)
            if let uid {
                container.offlineCache.write(userId: uid, key: "saved_places", value: loadedPlaces)
            }
            loading = false
            self.refreshing = false
            places = loadedPlaces
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

    public func save(
        editingId: Int64?,
        label: String,
        areaId: Int64?,
        address: String,
        gateNotes: String,
        isDefault: Bool,
        lat: Double? = nil,
        lng: Double? = nil,
        clearCoords: Bool = false
    ) async -> Bool {
        guard !busy else { return false }
        let cleanLabel = label.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleanAddress = address.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !cleanLabel.isEmpty, !cleanAddress.isEmpty else { return false }

        busy = true
        error = nil
        defer { busy = false }
        do {
            let req = SavedPlaceReq(
                label: cleanLabel,
                areaId: areaId,
                address: cleanAddress,
                gateNotes: gateNotes.trimmingCharacters(in: .whitespacesAndNewlines),
                isDefault: isDefault,
                lat: lat,
                lng: lng,
                clearCoords: (clearCoords && lat == nil && lng == nil) ? true : nil
            )
            if let editingId {
                _ = try await container.rides.updateSavedPlace(id: editingId, req)
            } else {
                _ = try await container.rides.createSavedPlace(req)
            }
            let updated = try await container.rides.savedPlaces()
            if let uid = container.auth.me?.id {
                container.offlineCache.write(userId: uid, key: "saved_places", value: updated)
            }
            places = updated
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

    public func setDefault(_ place: SavedPlace) async {
        guard !busy, place.isDefault != 1 else { return }
        busy = true
        error = nil
        defer { busy = false }
        do {
            _ = try await container.rides.updateSavedPlace(id: place.id, SavedPlaceReq(
                label: place.label,
                areaId: place.areaId,
                address: place.address,
                gateNotes: place.gateNotes,
                isDefault: true,
                lat: place.lat,
                lng: place.lng
            ))
            let updated = try await container.rides.savedPlaces()
            if let uid = container.auth.me?.id {
                container.offlineCache.write(userId: uid, key: "saved_places", value: updated)
            }
            places = updated
            message = "default_updated"
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func delete(id: Int64) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        defer { busy = false }
        do {
            _ = try await container.rides.deleteSavedPlace(id: id)
            let updated = try await container.rides.savedPlaces()
            if let uid = container.auth.me?.id {
                container.offlineCache.write(userId: uid, key: "saved_places", value: updated)
            }
            places = updated
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
