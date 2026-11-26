import Foundation
import Observation

@MainActor
@Observable
public final class DriverSettingsViewModel {
    private struct SavedPrefs: Equatable {
        let carpoolEnabled: Bool
        let spareSeats: Int
        let gateWaitMins: Int
        let areaIds: Set<Int64>
    }

    private let container: AppContainer
    private var savedPrefs: SavedPrefs?

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var busy: Bool = false
    public private(set) var areas: [Area] = []
    public private(set) var vehicles: [DriverVehicle] = []
    public private(set) var activeVehicleId: Int64?
    public private(set) var carpoolEnabled: Bool = false
    public private(set) var spareSeats: Int = 1
    public private(set) var gateWaitMins: Int = 5
    public private(set) var selectedAreaIds: Set<Int64> = []
    public private(set) var dirty: Bool = false
    public private(set) var accessDenied: Bool = false
    public private(set) var message: String?
    public private(set) var error: String?

    public var activeVehicle: DriverVehicle? {
        vehicles.first { activeVehicleId != nil && $0.id == activeVehicleId }
            ?? vehicles.first { $0.isActive }
            ?? vehicles.first
    }

    public var maxSpareSeats: Int {
        activeVehicle.map { min(max($0.seats - 1, 1), 7) } ?? 6
    }

    public init(container: AppContainer) {
        self.container = container
    }

    public func load() async {
        await loadInternal(refreshing: false)
    }

    public func refresh() async {
        await loadInternal(refreshing: savedPrefs != nil)
    }

    private func loadInternal(refreshing: Bool) async {
        guard container.auth.me?.can("drive", act: "view") == true else {
            loading = false
            self.refreshing = false
            accessDenied = true
            error = nil
            return
        }
        accessDenied = false
        loading = savedPrefs == nil && !refreshing
        self.refreshing = refreshing
        error = nil

        do {
            let fallbackAreas = areas
            async let profileDef = container.rides.driverProfile()
            async let areasDef: [Area] = (try? await container.rides.areas().filter(\.active)) ?? fallbackAreas
            let (profile, loadedAreas) = try await (profileDef, areasDef)
            applyProfile(profile, areas: loadedAreas, resetPrefs: true)
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

    private func currentPrefs() -> SavedPrefs {
        SavedPrefs(carpoolEnabled: carpoolEnabled, spareSeats: spareSeats, gateWaitMins: gateWaitMins, areaIds: selectedAreaIds)
    }

    private func applyProfile(_ profile: DriverProfile, areas: [Area], resetPrefs: Bool, message: String? = nil) {
        var vehs = profile.vehicles
        if vehs.isEmpty, var fallback = profile.vehicle {
            fallback.isActive = true
            vehs = [fallback]
        }
        let active = vehs.first { profile.activeVehicleId != nil && $0.id == profile.activeVehicleId }
            ?? vehs.first { $0.isActive }
            ?? vehs.first
        let maxSpare = active.map { min(max($0.seats - 1, 1), 7) } ?? 6

        self.loading = false
        self.refreshing = false
        self.busy = false
        self.areas = areas
        self.vehicles = vehs
        self.activeVehicleId = active?.id
        self.message = message
        self.error = nil

        if resetPrefs || savedPrefs == nil {
            let prefs = SavedPrefs(
                carpoolEnabled: profile.carpoolEnabled,
                spareSeats: min(max(profile.spareSeats, 1), maxSpare),
                gateWaitMins: profile.gateWaitMins,
                areaIds: Set(profile.areaIds)
            )
            savedPrefs = prefs
            carpoolEnabled = prefs.carpoolEnabled
            spareSeats = prefs.spareSeats
            gateWaitMins = prefs.gateWaitMins
            selectedAreaIds = prefs.areaIds
            dirty = false
        } else if let prev = savedPrefs {
            let nextSaved = SavedPrefs(
                carpoolEnabled: prev.carpoolEnabled,
                spareSeats: min(max(prev.spareSeats, 1), maxSpare),
                gateWaitMins: prev.gateWaitMins,
                areaIds: prev.areaIds
            )
            savedPrefs = nextSaved
            spareSeats = min(max(spareSeats, 1), maxSpare)
            dirty = currentPrefs() != nextSaved
        }
    }

    private func syncDirty() {
        dirty = savedPrefs.map { currentPrefs() != $0 } ?? false
        error = nil
    }

    public func toggleArea(_ areaId: Int64) {
        if selectedAreaIds.contains(areaId) {
            selectedAreaIds.remove(areaId)
        } else {
            selectedAreaIds.insert(areaId)
        }
        syncDirty()
    }

    public func setCarpool(_ enabled: Bool) {
        carpoolEnabled = enabled
        syncDirty()
    }

    public func stepSpareSeats(_ delta: Int) {
        spareSeats = min(max(spareSeats + delta, 1), maxSpareSeats)
        syncDirty()
    }

    public func setGateWaitMins(_ mins: Int) {
        gateWaitMins = mins
        syncDirty()
    }

    public func clearMessage() { message = nil }
    public func clearError() { error = nil }

    public func saveProfile() async {
        guard !busy else { return }
        busy = true
        error = nil
        defer { busy = false }
        let clampedSpare = min(max(spareSeats, 1), maxSpareSeats)
        do {
            _ = try await container.rides.updateDriverProfile(DriverProfileReq(
                carpoolEnabled: carpoolEnabled,
                spareSeats: clampedSpare,
                gateWaitMins: gateWaitMins,
                areaIds: selectedAreaIds.sorted(),
                activeVehicleId: activeVehicleId
            ))
            savedPrefs = SavedPrefs(
                carpoolEnabled: carpoolEnabled,
                spareSeats: clampedSpare,
                gateWaitMins: gateWaitMins,
                areaIds: selectedAreaIds
            )
            spareSeats = clampedSpare
            dirty = false
            message = "saved"
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func saveVehicle(editingId: Int64?, req: DriverVehicleReq) async -> Bool {
        guard !busy else { return false }
        let cleanModel = req.model.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleanColor = req.color.trimmingCharacters(in: .whitespacesAndNewlines)
        let cleanPlate = req.plate.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard !cleanModel.isEmpty, !cleanColor.isEmpty, !cleanPlate.isEmpty else { return false }

        busy = true
        error = nil
        defer { busy = false }
        do {
            var cleanReq = req
            cleanReq.model = cleanModel
            cleanReq.color = cleanColor
            cleanReq.plate = cleanPlate
            cleanReq.seats = min(max(req.seats, 2), 9)
            if let editingId {
                _ = try await container.rides.updateDriverVehicle(id: editingId, cleanReq)
            } else {
                _ = try await container.rides.createDriverVehicle(cleanReq)
            }
            let profile = try await container.rides.driverProfile()
            applyProfile(profile, areas: areas, resetPrefs: false, message: "vehicle_saved")
            return true
        } catch let e as ApiException {
            error = e.message
            return false
        } catch {
            self.error = error.localizedDescription
            return false
        }
    }

    public func selectVehicle(_ vehicleId: Int64) async {
        guard !busy else { return }
        busy = true
        error = nil
        defer { busy = false }
        do {
            _ = try await container.rides.selectDriverVehicle(id: vehicleId)
            let profile = try await container.rides.driverProfile()
            applyProfile(profile, areas: areas, resetPrefs: false, message: "vehicle_selected")
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func deleteVehicle(_ vehicleId: Int64) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        defer { busy = false }
        do {
            _ = try await container.rides.deleteDriverVehicle(id: vehicleId)
            let profile = try await container.rides.driverProfile()
            applyProfile(profile, areas: areas, resetPrefs: false, message: "vehicle_deleted")
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
