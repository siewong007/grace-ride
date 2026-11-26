import Foundation

public final class EventsRepository: @unchecked Sendable {
    private let apiProvider: ApiProvider
    private var api: any GraceRideAPIProtocol { apiProvider.get() }

    public init(apiProvider: ApiProvider) {
        self.apiProvider = apiProvider
    }

    public func upcoming(passive: Bool = false) async throws -> [EventItem] { try await api.events(scope: "upcoming", hideDrafts: nil, passive: passive) }
    public func past() async throws -> [EventItem] { try await api.events(scope: "past", hideDrafts: nil, passive: false) }
    public func managing(hideDrafts: Bool = false) async throws -> [EventItem] {
        try await api.events(scope: "managing", hideDrafts: hideDrafts ? true : nil, passive: false)
    }
    public func detail(id: Int64) async throws -> EventItem { try await api.event(id: id) }
    public func signup(id: Int64, needRide: Bool, offerDrive: Bool, asGroupId: Int64?, offerSeats: Int? = nil) async throws -> OkResponse {
        try await api.signup(id: id, SignupReq(
            needRide: needRide,
            offerDrive: offerDrive,
            offerSeats: offerSeats.map(Int64.init),
            asGroupId: asGroupId
        ))
    }
    public func withdraw(id: Int64) async throws -> OkResponse { try await api.withdraw(id: id) }
    public func publish(id: Int64) async throws -> OkResponse { try await api.publishEvent(id: id) }
    public func unpublish(id: Int64) async throws -> OkResponse { try await api.unpublishEvent(id: id) }
    public func checkinInfo(id: Int64) async throws -> CheckinInfoDto { try await api.eventCheckinInfo(id: id) }
    public func selfCheckin(id: Int64, code: String? = nil) async throws -> CheckinResp {
        try await api.selfCheckin(id: id, CheckinReq(code: code))
    }
    public func checkinByCode(_ code: String) async throws -> CheckinResp {
        try await api.checkinByCode(CheckinByCodeReq(code: code))
    }
    public func scanMemberPass(id: Int64, passCode: String) async throws -> ScanMemberResp {
        try await api.scanMemberPass(id: id, ScanMemberReq(passCode: passCode))
    }
    public func markAttendance(id: Int64, userId: Int64, attended: Bool?) async throws -> OkResponse {
        try await api.markAttendance(id: id, MarkAttendanceReq(userId: userId, attended: attended))
    }
}

public final class SharesRepository: @unchecked Sendable {
    private let apiProvider: ApiProvider
    private var api: any GraceRideAPIProtocol { apiProvider.get() }

    public init(apiProvider: ApiProvider) {
        self.apiProvider = apiProvider
    }

    public func mine() async throws -> [Share] { try await api.myShares() }

    public func upload(data: Data, filename: String, mime: String) async throws -> FileRef {
        try await api.upload(data: data, filename: filename, mime: mime)
    }

    public func submitProof(
        shareId: Int64,
        fileId: Int64,
        txnRef: String,
        paidDate: String = KlTime.today()
    ) async throws -> OkResponse {
        try await api.submitProof(id: shareId, ProofReq(proofFileId: fileId, txnRef: txnRef, paidDate: paidDate))
    }
}

public final class CommunityRepository: @unchecked Sendable {
    private let apiProvider: ApiProvider
    private var api: any GraceRideAPIProtocol { apiProvider.get() }

    public init(apiProvider: ApiProvider) {
        self.apiProvider = apiProvider
    }

    public func announcements() async throws -> [Announcement] { try await api.announcements() }

    public func notificationsPage(
        before: Int64? = nil,
        limit: Int = 30,
        unreadOnly: Bool? = nil,
        category: String? = nil,
        passive: Bool = false
    ) async throws -> NotificationPageDto {
        try await api.notificationsPage(
            before: before,
            limit: limit,
            unreadOnly: unreadOnly,
            category: category?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == true ? nil : category,
            passive: passive
        )
    }

    public func notifications() async throws -> [NotificationItem] {
        try await notificationsPage(limit: 30).items
    }

    public func readAll(category: String? = nil) async throws -> NotificationActionResp {
        let clean = category?.trimmingCharacters(in: .whitespacesAndNewlines)
        return try await api.readAllNotifications(ReadAllReq(category: (clean?.isEmpty == false) ? clean : nil))
    }

    public func readOne(id: Int64) async throws -> NotificationActionResp { try await api.readNotification(id: id) }
    public func unreadOne(id: Int64) async throws -> NotificationActionResp { try await api.unreadNotification(id: id) }
    public func deleteOne(id: Int64) async throws -> NotificationActionResp { try await api.deleteNotification(id: id) }
    public func notificationPrefs(passive: Bool = false) async throws -> NotificationPrefsDto {
        try await api.notificationPrefs(passive: passive)
    }
    public func updateNotificationPrefs(_ req: UpdateNotificationPrefsReq) async throws -> OkResponse {
        try await api.updateNotificationPrefs(req)
    }
    public func registerMobileToken(_ req: MobilePushTokenReq) async throws -> OkResponse {
        try await api.registerMobileToken(req)
    }
    public func unregisterMobileToken(_ token: String) async throws -> OkResponse {
        try await api.unregisterMobileToken(MobilePushTokenDel(token: token))
    }
    public func updateProfile(_ req: ProfileUpdateReq) async throws -> OkResponse {
        try await api.updateProfile(req)
    }
    public func upload(data: Data, filename: String, mime: String) async throws -> FileRef {
        try await api.upload(data: data, filename: filename, mime: mime)
    }
    public func passkeys() async throws -> [PasskeyInfo] { try await api.passkeys() }
    public func deletePasskey(id: Int64) async throws -> OkResponse { try await api.deletePasskey(id: id) }
    public func myReportBadges() async throws -> [Badge] {
        try await api.myReportBadges().badges
    }
}
