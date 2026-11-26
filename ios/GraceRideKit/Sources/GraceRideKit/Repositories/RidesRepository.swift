import Foundation

public final class RidesRepository: @unchecked Sendable {
    private let apiProvider: ApiProvider
    private let lock = NSLock()
    private var cachedAreas: [Area]?
    private var areasFetchedAt: Date?
    private let areasTTL: TimeInterval = 5 * 60

    private var api: any GraceRideAPIProtocol { apiProvider.get() }

    public init(apiProvider: ApiProvider) {
        self.apiProvider = apiProvider
    }

    public func upcoming(passive: Bool = false) async throws -> [Ride] { try await api.myRides(scope: "upcoming", passive: passive) }
    public func past() async throws -> [Ride] { try await api.myRides(scope: "past", passive: false) }
    public func detail(id: Int64) async throws -> Ride { try await api.ride(id: id) }

    public func areas(forceRefresh: Bool = false) async throws -> [Area] {
        let existing: [Area]? = lock.withLock {
            guard !forceRefresh, let cachedAreas, let fetched = areasFetchedAt,
                  Date().timeIntervalSince(fetched) < areasTTL else {
                return nil
            }
            return cachedAreas
        }
        if let existing { return existing }
        let fresh = try await api.areas()
        lock.withLock {
            cachedAreas = fresh
            areasFetchedAt = Date()
        }
        return fresh
    }

    public func clearAreasCache() {
        lock.withLock {
            cachedAreas = nil
            areasFetchedAt = nil
        }
    }

    public func places(query: String) async throws -> [Place] { try await api.places(query: query) }
    public func savedPlaces() async throws -> [SavedPlace] { try await api.savedPlaces() }
    public func createSavedPlace(_ req: SavedPlaceReq) async throws -> CreateSavedPlaceRes { try await api.createSavedPlace(req) }
    public func updateSavedPlace(id: Int64, _ req: SavedPlaceReq) async throws -> OkResponse { try await api.updateSavedPlace(id: id, req) }
    public func deleteSavedPlace(id: Int64) async throws -> OkResponse { try await api.deleteSavedPlace(id: id) }

    public func proxyRiders() async throws -> [ProxyRider] { try await api.proxyRiders() }
    public func createProxyRider(_ req: ProxyRiderReq) async throws -> CreateProxyRiderRes { try await api.createProxyRider(req) }
    public func updateProxyRider(id: Int64, _ req: ProxyRiderReq) async throws -> OkResponse { try await api.updateProxyRider(id: id, req) }
    public func deleteProxyRider(id: Int64) async throws -> OkResponse { try await api.deleteProxyRider(id: id) }

    public func create(_ req: CreateRideReq) async throws -> CreateRideRes { try await api.createRide(req) }
    public func cancel(id: Int64, reason: String?) async throws -> OkResponse { try await api.cancelRide(id: id, CancelReq(reason: reason)) }
    public func respond(id: Int64, accept: Bool) async throws -> OkResponse { try await api.respondRide(id: id, RespondReq(accept: accept)) }
    public func readiness(id: Int64, status: String, notes: String? = nil) async throws -> OkResponse {
        try await api.readiness(id: id, ReadinessReq(status: status, notes: notes))
    }
    public func confirmAssignment(id: Int64) async throws -> OkResponse { try await api.confirmAssignment(id: id) }
    public func startRide(id: Int64) async throws -> OkResponse { try await api.startRide(id: id) }
    public func arrivedGate(id: Int64) async throws -> OkResponse { try await api.arrivedGate(id: id) }
    public func pickupRide(id: Int64) async throws -> OkResponse { try await api.pickupRide(id: id) }
    public func completeRide(id: Int64) async throws -> OkResponse { try await api.completeRide(id: id) }
    public func driverDelay(id: Int64, minutes: Int) async throws -> OkResponse {
        try await api.driverDelay(id: id, DriverDelayReq(delayMinutes: minutes))
    }
    public func withdrawRide(id: Int64, reason: String?) async throws -> OkResponse {
        try await api.withdrawRide(id: id, WithdrawReq(reason: reason))
    }
    public func drivingToday(from: String, to: String) async throws -> [Ride] {
        try await api.driverSchedule(from: from, to: to)
    }
    public func driverRequests(mineAreas: Bool? = nil) async throws -> [Ride] {
        try await api.driverRequests(mineAreas: mineAreas)
    }
    public func claimRide(id: Int64) async throws -> OkResponse { try await api.claimRide(id: id) }
    public func driverRoute() async throws -> DriverRouteResp { try await api.driverRoute() }
    public func postDriverLocation(lat: Double, lng: Double, heading: Double? = nil) async throws -> LocationShareResp {
        try await api.postDriverLocation(DriverLocationReq(lat: lat, lng: lng, heading: heading))
    }
    public func stopDriverLocation() async throws -> OkResponse { try await api.stopDriverLocation() }
    public func rideLocation(id: Int64) async throws -> RideLocationDto { try await api.rideLocation(id: id) }
    public func postPassengerLocation(id: Int64, lat: Double, lng: Double) async throws -> LocationShareResp {
        try await api.postPassengerLocation(id: id, PassengerLocationReq(lat: lat, lng: lng))
    }
    public func stopPassengerLocation(id: Int64) async throws -> OkResponse { try await api.stopPassengerLocation(id: id) }
    public func rideMessages(id: Int64, after: Int64? = nil) async throws -> ChatThreadDto {
        try await api.rideMessages(id: id, after: after)
    }
    public func postMessage(id: Int64, body: String) async throws -> PostMessageRes {
        try await api.postRideMessage(id: id, PostMessageReq(body: body))
    }
    public func markChatRead(id: Int64, upTo: Int64) async throws -> OkResponse {
        try await api.markRideChatRead(id: id, MarkReadReq(upTo: upTo))
    }
    public func reactMessage(id: Int64, messageId: Int64, emoji: String) async throws -> ReactRes {
        try await api.reactRideMessage(id: id, messageId: messageId, ReactReq(emoji: emoji))
    }
    public func driverProfile() async throws -> DriverProfile { try await api.driverProfile() }
    public func updateDriverProfile(_ req: DriverProfileReq) async throws -> OkResponse { try await api.updateDriverProfile(req) }
    public func createDriverVehicle(_ req: DriverVehicleReq) async throws -> DriverVehicle { try await api.createDriverVehicle(req) }
    public func updateDriverVehicle(id: Int64, _ req: DriverVehicleReq) async throws -> DriverVehicle {
        try await api.updateDriverVehicle(id: id, req)
    }
    public func deleteDriverVehicle(id: Int64) async throws -> OkResponse { try await api.deleteDriverVehicle(id: id) }
    public func selectDriverVehicle(id: Int64) async throws -> SelectVehicleRes { try await api.selectDriverVehicle(id: id) }
}
