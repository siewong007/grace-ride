import Foundation

/// Typed REST API surface mirroring `android/.../ApiService.kt` against the shared Rust/Axum backend.
public protocol GraceRideAPIProtocol: Sendable {
    // Auth & Session
    func me() async throws -> Me
    func devUsers() async throws -> [DevUser]
    func devLogin(email: String) async throws -> OkResponse
    func logout(_ body: LogoutReq) async throws -> OkResponse
    func logoutAll() async throws -> OkResponse
    func lock() async throws -> OkResponse
    func unlock(_ body: UnlockReq) async throws -> OkResponse
    func acceptTerms(_ body: TermsReq) async throws -> OkResponse
    func turnstileConfig() async throws -> TurnstileConfigDto
    func passkeyLoginStart(_ body: PasskeyStartReq) async throws -> PasskeyStart
    func passkeyLoginFinish(_ body: PasskeyFinishReq) async throws -> OkResponse
    func passkeyReauthStart() async throws -> PasskeyStart
    func passkeyReauthFinish(_ body: PasskeyFinishReq) async throws -> OkResponse
    func passkeyRegisterStart() async throws -> PasskeyStart
    func passkeyRegisterFinish(_ body: PasskeyFinishReq) async throws -> OkResponse
    func passkeys() async throws -> [PasskeyInfo]
    func deletePasskey(id: Int64) async throws -> OkResponse

    // Profile & Community
    func updateProfile(_ body: ProfileUpdateReq) async throws -> OkResponse
    func announcements() async throws -> [Announcement]
    func notificationsPage(before: Int64?, limit: Int, unreadOnly: Bool?, category: String?, passive: Bool) async throws -> NotificationPageDto
    func readAllNotifications(_ body: ReadAllReq) async throws -> NotificationActionResp
    func readNotification(id: Int64) async throws -> NotificationActionResp
    func unreadNotification(id: Int64) async throws -> NotificationActionResp
    func deleteNotification(id: Int64) async throws -> NotificationActionResp
    func notificationPrefs(passive: Bool) async throws -> NotificationPrefsDto
    func updateNotificationPrefs(_ body: UpdateNotificationPrefsReq) async throws -> OkResponse
    func registerMobileToken(_ body: MobilePushTokenReq) async throws -> OkResponse
    func unregisterMobileToken(_ body: MobilePushTokenDel) async throws -> OkResponse
    func myReportBadges() async throws -> MemberReportResp

    // Rides & Places & Drivers
    func areas() async throws -> [Area]
    func places(query: String) async throws -> [Place]
    func savedPlaces() async throws -> [SavedPlace]
    func createSavedPlace(_ body: SavedPlaceReq) async throws -> CreateSavedPlaceRes
    func updateSavedPlace(id: Int64, _ body: SavedPlaceReq) async throws -> OkResponse
    func deleteSavedPlace(id: Int64) async throws -> OkResponse
    func proxyRiders() async throws -> [ProxyRider]
    func createProxyRider(_ body: ProxyRiderReq) async throws -> CreateProxyRiderRes
    func updateProxyRider(id: Int64, _ body: ProxyRiderReq) async throws -> OkResponse
    func deleteProxyRider(id: Int64) async throws -> OkResponse
    func myRides(scope: String?, passive: Bool) async throws -> [Ride]
    func ride(id: Int64) async throws -> Ride
    func createRide(_ body: CreateRideReq) async throws -> CreateRideRes
    func cancelRide(id: Int64, _ body: CancelReq) async throws -> OkResponse
    func respondRide(id: Int64, _ body: RespondReq) async throws -> OkResponse
    func readiness(id: Int64, _ body: ReadinessReq) async throws -> OkResponse
    func confirmAssignment(id: Int64) async throws -> OkResponse
    func startRide(id: Int64) async throws -> OkResponse
    func arrivedGate(id: Int64) async throws -> OkResponse
    func pickupRide(id: Int64) async throws -> OkResponse
    func completeRide(id: Int64) async throws -> OkResponse
    func driverDelay(id: Int64, _ body: DriverDelayReq) async throws -> OkResponse
    func withdrawRide(id: Int64, _ body: WithdrawReq) async throws -> OkResponse
    func driverSchedule(from: String, to: String) async throws -> [Ride]
    func driverRequests(mineAreas: Bool?) async throws -> [Ride]
    func claimRide(id: Int64) async throws -> OkResponse
    func driverRoute() async throws -> DriverRouteResp
    func postDriverLocation(_ body: DriverLocationReq) async throws -> LocationShareResp
    func stopDriverLocation() async throws -> OkResponse
    func rideLocation(id: Int64) async throws -> RideLocationDto
    func postPassengerLocation(id: Int64, _ body: PassengerLocationReq) async throws -> LocationShareResp
    func stopPassengerLocation(id: Int64) async throws -> OkResponse
    func rideMessages(id: Int64, after: Int64?) async throws -> ChatThreadDto
    func postRideMessage(id: Int64, _ body: PostMessageReq) async throws -> PostMessageRes
    func markRideChatRead(id: Int64, _ body: MarkReadReq) async throws -> OkResponse
    func reactRideMessage(id: Int64, messageId: Int64, _ body: ReactReq) async throws -> ReactRes
    func driverProfile() async throws -> DriverProfile
    func updateDriverProfile(_ body: DriverProfileReq) async throws -> OkResponse
    func createDriverVehicle(_ body: DriverVehicleReq) async throws -> DriverVehicle
    func updateDriverVehicle(id: Int64, _ body: DriverVehicleReq) async throws -> DriverVehicle
    func deleteDriverVehicle(id: Int64) async throws -> OkResponse
    func selectDriverVehicle(id: Int64) async throws -> SelectVehicleRes

    // Events
    func events(scope: String?, hideDrafts: Bool?, passive: Bool) async throws -> [EventItem]
    func event(id: Int64) async throws -> EventItem
    func signup(id: Int64, _ body: SignupReq) async throws -> OkResponse
    func withdraw(id: Int64) async throws -> OkResponse
    func publishEvent(id: Int64) async throws -> OkResponse
    func unpublishEvent(id: Int64) async throws -> OkResponse
    func eventCheckinInfo(id: Int64) async throws -> CheckinInfoDto
    func selfCheckin(id: Int64, _ body: CheckinReq) async throws -> CheckinResp
    func checkinByCode(_ body: CheckinByCodeReq) async throws -> CheckinResp
    func scanMemberPass(id: Int64, _ body: ScanMemberReq) async throws -> ScanMemberResp
    func markAttendance(id: Int64, _ body: MarkAttendanceReq) async throws -> OkResponse

    // Costs & Files
    func myShares() async throws -> [Share]
    func submitProof(id: Int64, _ body: ProofReq) async throws -> OkResponse
    func upload(data: Data, filename: String, mime: String) async throws -> FileRef
}

public final class GraceRideAPI: GraceRideAPIProtocol, @unchecked Sendable {
    public let client: HTTPClient

    public init(client: HTTPClient) {
        self.client = client
    }

    // MARK: Auth & Session
    public func me() async throws -> Me { try await client.get("api/auth/me") }
    public func devUsers() async throws -> [DevUser] { try await client.get("api/auth/dev-users") }
    public func devLogin(email: String) async throws -> OkResponse {
        try await client.post("api/auth/dev-login", body: ["email": email])
    }
    public func logout(_ body: LogoutReq = LogoutReq()) async throws -> OkResponse {
        try await client.post("api/auth/logout", body: body)
    }
    public func logoutAll() async throws -> OkResponse { try await client.post("api/auth/logout-all") }
    public func lock() async throws -> OkResponse { try await client.post("api/auth/lock") }
    public func unlock(_ body: UnlockReq) async throws -> OkResponse { try await client.post("api/auth/unlock", body: body) }
    public func acceptTerms(_ body: TermsReq) async throws -> OkResponse { try await client.post("api/auth/terms", body: body) }
    public func turnstileConfig() async throws -> TurnstileConfigDto { try await client.get("api/auth/turnstile") }
    public func passkeyLoginStart(_ body: PasskeyStartReq) async throws -> PasskeyStart {
        try await client.post("api/auth/passkey/login/start", body: body)
    }
    public func passkeyLoginFinish(_ body: PasskeyFinishReq) async throws -> OkResponse {
        try await client.post("api/auth/passkey/login/finish", body: body)
    }
    public func passkeyReauthStart() async throws -> PasskeyStart {
        try await client.post("api/auth/passkey/reauth/start")
    }
    public func passkeyReauthFinish(_ body: PasskeyFinishReq) async throws -> OkResponse {
        try await client.post("api/auth/passkey/reauth/finish", body: body)
    }
    public func passkeyRegisterStart() async throws -> PasskeyStart {
        try await client.post("api/auth/passkey/register/start")
    }
    public func passkeyRegisterFinish(_ body: PasskeyFinishReq) async throws -> OkResponse {
        try await client.post("api/auth/passkey/register/finish", body: body)
    }
    public func passkeys() async throws -> [PasskeyInfo] { try await client.get("api/auth/passkeys") }
    public func deletePasskey(id: Int64) async throws -> OkResponse {
        try await client.delete("api/auth/passkeys/\(id)")
    }

    // MARK: Profile & Community
    public func updateProfile(_ body: ProfileUpdateReq) async throws -> OkResponse {
        try await client.put("api/me", body: body)
    }
    public func announcements() async throws -> [Announcement] { try await client.get("api/announcements") }
    public func notificationsPage(before: Int64? = nil, limit: Int = 30, unreadOnly: Bool? = nil, category: String? = nil, passive: Bool = false) async throws -> NotificationPageDto {
        var items = [URLQueryItem(name: "limit", value: String(limit))]
        if let before { items.append(URLQueryItem(name: "before", value: String(before))) }
        if let unreadOnly { items.append(URLQueryItem(name: "unread_only", value: unreadOnly ? "true" : "false")) }
        if let category, !category.isEmpty { items.append(URLQueryItem(name: "category", value: category)) }
        return try await client.get("api/notifications", query: items, headers: passive ? ["X-Passive-Poll": "1"] : [:])
    }
    public func readAllNotifications(_ body: ReadAllReq = ReadAllReq()) async throws -> NotificationActionResp {
        try await client.post("api/notifications/read-all", body: body)
    }
    public func readNotification(id: Int64) async throws -> NotificationActionResp {
        try await client.post("api/notifications/\(id)/read")
    }
    public func unreadNotification(id: Int64) async throws -> NotificationActionResp {
        try await client.post("api/notifications/\(id)/unread")
    }
    public func deleteNotification(id: Int64) async throws -> NotificationActionResp {
        try await client.delete("api/notifications/\(id)")
    }
    public func notificationPrefs(passive: Bool = false) async throws -> NotificationPrefsDto {
        try await client.get("api/notifications/preferences", headers: passive ? ["X-Passive-Poll": "1"] : [:])
    }
    public func updateNotificationPrefs(_ body: UpdateNotificationPrefsReq) async throws -> OkResponse {
        try await client.put("api/notifications/preferences", body: body)
    }
    public func registerMobileToken(_ body: MobilePushTokenReq) async throws -> OkResponse {
        try await client.post("api/push/mobile-token", body: body)
    }
    public func unregisterMobileToken(_ body: MobilePushTokenDel) async throws -> OkResponse {
        try await client.delete("api/push/mobile-token", body: body)
    }
    public func myReportBadges() async throws -> MemberReportResp {
        try await client.get("api/reports/my")
    }

    // MARK: Rides & Places & Drivers
    public func areas() async throws -> [Area] { try await client.get("api/areas") }
    public func places(query: String) async throws -> [Place] {
        try await client.get("api/places", query: [URLQueryItem(name: "q", value: query)])
    }
    public func savedPlaces() async throws -> [SavedPlace] { try await client.get("api/saved-places") }
    public func createSavedPlace(_ body: SavedPlaceReq) async throws -> CreateSavedPlaceRes {
        try await client.post("api/saved-places", body: body)
    }
    public func updateSavedPlace(id: Int64, _ body: SavedPlaceReq) async throws -> OkResponse {
        try await client.put("api/saved-places/\(id)", body: body)
    }
    public func deleteSavedPlace(id: Int64) async throws -> OkResponse {
        try await client.delete("api/saved-places/\(id)")
    }
    public func proxyRiders() async throws -> [ProxyRider] { try await client.get("api/proxy-riders") }
    public func createProxyRider(_ body: ProxyRiderReq) async throws -> CreateProxyRiderRes {
        try await client.post("api/proxy-riders", body: body)
    }
    public func updateProxyRider(id: Int64, _ body: ProxyRiderReq) async throws -> OkResponse {
        try await client.put("api/proxy-riders/\(id)", body: body)
    }
    public func deleteProxyRider(id: Int64) async throws -> OkResponse {
        try await client.delete("api/proxy-riders/\(id)")
    }
    public func myRides(scope: String? = nil, passive: Bool = false) async throws -> [Ride] {
        var q: [URLQueryItem] = []
        if let scope { q.append(URLQueryItem(name: "scope", value: scope)) }
        return try await client.get("api/rides", query: q, headers: passive ? ["X-Passive-Poll": "1"] : [:])
    }
    public func ride(id: Int64) async throws -> Ride { try await client.get("api/rides/\(id)") }
    public func createRide(_ body: CreateRideReq) async throws -> CreateRideRes {
        try await client.post("api/rides", body: body)
    }
    public func cancelRide(id: Int64, _ body: CancelReq) async throws -> OkResponse {
        try await client.post("api/rides/\(id)/cancel", body: body)
    }
    public func respondRide(id: Int64, _ body: RespondReq) async throws -> OkResponse {
        try await client.post("api/rides/\(id)/respond", body: body)
    }
    public func readiness(id: Int64, _ body: ReadinessReq) async throws -> OkResponse {
        try await client.post("api/rides/\(id)/readiness", body: body)
    }
    public func confirmAssignment(id: Int64) async throws -> OkResponse {
        try await client.post("api/rides/\(id)/confirm-assignment")
    }
    public func startRide(id: Int64) async throws -> OkResponse { try await client.post("api/rides/\(id)/start") }
    public func arrivedGate(id: Int64) async throws -> OkResponse { try await client.post("api/rides/\(id)/arrived-gate") }
    public func pickupRide(id: Int64) async throws -> OkResponse { try await client.post("api/rides/\(id)/pickup") }
    public func completeRide(id: Int64) async throws -> OkResponse { try await client.post("api/rides/\(id)/complete") }
    public func driverDelay(id: Int64, _ body: DriverDelayReq) async throws -> OkResponse {
        try await client.post("api/rides/\(id)/driver-delay", body: body)
    }
    public func withdrawRide(id: Int64, _ body: WithdrawReq) async throws -> OkResponse {
        try await client.post("api/rides/\(id)/withdraw", body: body)
    }
    public func driverSchedule(from: String, to: String) async throws -> [Ride] {
        try await client.get("api/driver/schedule", query: [
            URLQueryItem(name: "from", value: from),
            URLQueryItem(name: "to", value: to),
        ])
    }
    public func driverRequests(mineAreas: Bool? = nil) async throws -> [Ride] {
        var q: [URLQueryItem] = []
        if let mineAreas { q.append(URLQueryItem(name: "mine_areas", value: mineAreas ? "true" : "false")) }
        return try await client.get("api/driver/requests", query: q)
    }
    public func claimRide(id: Int64) async throws -> OkResponse { try await client.post("api/rides/\(id)/claim") }
    public func driverRoute() async throws -> DriverRouteResp { try await client.get("api/driver/route") }
    public func postDriverLocation(_ body: DriverLocationReq) async throws -> LocationShareResp {
        try await client.post("api/driver/location", body: body)
    }
    public func stopDriverLocation() async throws -> OkResponse { try await client.delete("api/driver/location") }
    public func rideLocation(id: Int64) async throws -> RideLocationDto { try await client.get("api/rides/\(id)/location") }
    public func postPassengerLocation(id: Int64, _ body: PassengerLocationReq) async throws -> LocationShareResp {
        try await client.post("api/rides/\(id)/passenger-location", body: body)
    }
    public func stopPassengerLocation(id: Int64) async throws -> OkResponse {
        try await client.delete("api/rides/\(id)/passenger-location")
    }
    public func rideMessages(id: Int64, after: Int64? = nil) async throws -> ChatThreadDto {
        var q: [URLQueryItem] = []
        if let after { q.append(URLQueryItem(name: "after", value: String(after))) }
        return try await client.get("api/rides/\(id)/messages", query: q)
    }
    public func postRideMessage(id: Int64, _ body: PostMessageReq) async throws -> PostMessageRes {
        try await client.post("api/rides/\(id)/messages", body: body)
    }
    public func markRideChatRead(id: Int64, _ body: MarkReadReq) async throws -> OkResponse {
        try await client.post("api/rides/\(id)/messages/read", body: body)
    }
    public func reactRideMessage(id: Int64, messageId: Int64, _ body: ReactReq) async throws -> ReactRes {
        try await client.post("api/rides/\(id)/messages/\(messageId)/reactions", body: body)
    }
    public func driverProfile() async throws -> DriverProfile { try await client.get("api/driver/profile") }
    public func updateDriverProfile(_ body: DriverProfileReq) async throws -> OkResponse {
        try await client.put("api/driver/profile", body: body)
    }
    public func createDriverVehicle(_ body: DriverVehicleReq) async throws -> DriverVehicle {
        try await client.post("api/driver/vehicles", body: body)
    }
    public func updateDriverVehicle(id: Int64, _ body: DriverVehicleReq) async throws -> DriverVehicle {
        try await client.put("api/driver/vehicles/\(id)", body: body)
    }
    public func deleteDriverVehicle(id: Int64) async throws -> OkResponse {
        try await client.delete("api/driver/vehicles/\(id)")
    }
    public func selectDriverVehicle(id: Int64) async throws -> SelectVehicleRes {
        try await client.post("api/driver/vehicles/\(id)/select")
    }

    // MARK: Events
    public func events(scope: String? = nil, hideDrafts: Bool? = nil, passive: Bool = false) async throws -> [EventItem] {
        var q: [URLQueryItem] = []
        if let scope { q.append(URLQueryItem(name: "scope", value: scope)) }
        if let hideDrafts { q.append(URLQueryItem(name: "hide_drafts", value: hideDrafts ? "true" : "false")) }
        return try await client.get("api/events", query: q, headers: passive ? ["X-Passive-Poll": "1"] : [:])
    }
    public func event(id: Int64) async throws -> EventItem { try await client.get("api/events/\(id)") }
    public func signup(id: Int64, _ body: SignupReq) async throws -> OkResponse {
        try await client.post("api/events/\(id)/signup", body: body)
    }
    public func withdraw(id: Int64) async throws -> OkResponse { try await client.post("api/events/\(id)/withdraw") }
    public func publishEvent(id: Int64) async throws -> OkResponse { try await client.post("api/events/\(id)/publish") }
    public func unpublishEvent(id: Int64) async throws -> OkResponse { try await client.post("api/events/\(id)/unpublish") }
    public func eventCheckinInfo(id: Int64) async throws -> CheckinInfoDto {
        try await client.get("api/events/\(id)/checkin")
    }
    public func selfCheckin(id: Int64, _ body: CheckinReq) async throws -> CheckinResp {
        try await client.post("api/events/\(id)/checkin", body: body)
    }
    public func checkinByCode(_ body: CheckinByCodeReq) async throws -> CheckinResp {
        try await client.post("api/events/checkin", body: body)
    }
    public func scanMemberPass(id: Int64, _ body: ScanMemberReq) async throws -> ScanMemberResp {
        try await client.post("api/events/\(id)/checkin/scan-member", body: body)
    }
    public func markAttendance(id: Int64, _ body: MarkAttendanceReq) async throws -> OkResponse {
        try await client.post("api/events/\(id)/attendance", body: body)
    }

    // MARK: Costs & Files
    public func myShares() async throws -> [Share] { try await client.get("api/me/shares") }
    public func submitProof(id: Int64, _ body: ProofReq) async throws -> OkResponse {
        try await client.post("api/shares/\(id)/proof", body: body)
    }
    public func upload(data: Data, filename: String, mime: String) async throws -> FileRef {
        try await client.uploadMultipart("api/files", filename: filename, mime: mime, fileData: data)
    }
}
