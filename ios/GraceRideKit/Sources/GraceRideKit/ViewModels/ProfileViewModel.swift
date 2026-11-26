import Foundation
import Observation

@MainActor
@Observable
public final class ProfileViewModel {
    private let container: AppContainer

    public private(set) var me: Me?
    public private(set) var passkeys: [PasskeyInfo] = []
    public private(set) var notifPrefs: NotificationPrefsDto?
    public private(set) var badges: [Badge] = []
    public private(set) var busy: Bool = false
    public private(set) var message: String?
    public private(set) var error: String?
    public var lang: String

    public init(container: AppContainer) {
        self.container = container
        self.me = container.auth.me
        self.lang = container.lang
    }

    public func refreshMe() async {
        try? await container.auth.refresh(throwOnError: false)
        me = container.auth.me
    }

    public func loadPasskeys() async {
        if let list = try? await container.community.passkeys() {
            passkeys = list
        }
    }

    public func loadBadges() async {
        if let list = try? await container.community.myReportBadges() {
            badges = list
        }
    }

    public func loadNotificationPrefs() async {
        if let prefs = try? await container.community.notificationPrefs() {
            RealtimeStreamClient.writeCachedPrefs(store: container.secureStore, prefs: prefs)
            notifPrefs = prefs
        }
    }

    public func clearMessage() { message = nil }
    public func clearError() { error = nil }

    public func saveProfile(name: String, phone: String) async {
        guard !busy else { return }
        busy = true
        error = nil
        message = nil
        defer { busy = false }
        do {
            _ = try await container.community.updateProfile(ProfileUpdateReq(
                name: name.trimmingCharacters(in: .whitespacesAndNewlines),
                phone: phone.trimmingCharacters(in: .whitespacesAndNewlines)
            ))
            try? await container.auth.refresh(throwOnError: false)
            me = container.auth.me
            message = "saved"
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func saveNotificationPrefs(_ req: UpdateNotificationPrefsReq) async {
        guard !busy else { return }
        busy = true
        error = nil
        message = nil
        defer { busy = false }
        do {
            _ = try await container.community.updateNotificationPrefs(req)
            let prefs = try await container.community.notificationPrefs()
            RealtimeStreamClient.writeCachedPrefs(store: container.secureStore, prefs: prefs)
            try? await container.auth.refresh(throwOnError: false)
            me = container.auth.me
            notifPrefs = prefs
            message = "prefs_saved"
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func saveAvatarStyle(avatarIcon: String, avatarColor: String) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        message = nil
        defer { busy = false }
        do {
            _ = try await container.community.updateProfile(ProfileUpdateReq(
                clearPhoto: true,
                avatarColor: avatarColor,
                avatarIcon: avatarIcon
            ))
            try? await container.auth.refresh(throwOnError: false)
            me = container.auth.me
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

    public func saveAvatarPhotoColor(photoFileId: Int64, avatarColor: String) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        message = nil
        defer { busy = false }
        do {
            _ = try await container.community.updateProfile(ProfileUpdateReq(
                photoFileId: photoFileId,
                avatarColor: avatarColor
            ))
            try? await container.auth.refresh(throwOnError: false)
            me = container.auth.me
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

    public func saveAvatarPhoto(data: Data, filename: String, mime: String, avatarColor: String) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        message = nil
        defer { busy = false }
        do {
            let uploaded = try await container.community.upload(data: data, filename: filename, mime: mime)
            _ = try await container.community.updateProfile(ProfileUpdateReq(
                photoFileId: uploaded.id,
                avatarColor: avatarColor
            ))
            try? await container.auth.refresh(throwOnError: false)
            me = container.auth.me
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

    public func setLang(_ newLang: String) async {
        container.lang = newLang
        lang = container.lang
        try? await container.community.updateProfile(ProfileUpdateReq(lang: container.lang))
        try? await container.auth.refresh(throwOnError: false)
        me = container.auth.me
    }

    public func setAutoLock(minutes: Int) async {
        do {
            _ = try await container.community.updateProfile(ProfileUpdateReq(autoLockMins: minutes))
            try? await container.auth.refresh(throwOnError: false)
            me = container.auth.me
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func addPasskey(
        performRegistration: (_ start: PasskeyStart) async throws -> JSONValue,
        performReauth: (_ start: PasskeyStart) async throws -> JSONValue
    ) async {
        guard !busy else { return }
        busy = true
        error = nil
        defer { busy = false }

        func registerOnce() async throws {
            let start = try await container.auth.passkeyRegisterStart()
            let credential = try await performRegistration(start)
            try await container.auth.passkeyRegisterFinish(challengeId: start.challengeId, credential: credential)
            await loadPasskeys()
            me = container.auth.me
        }

        do {
            try await registerOnce()
        } catch PasskeyFlowError.cancelled {
            return
        } catch let e as PasskeyFlowError {
            if case .failed(let msg) = e { error = msg }
        } catch let e as ApiException {
            let hasPasskeys = (container.auth.me?.passkeyCount ?? passkeys.count) > 0
            if e.code == "RECENT_SIGN_IN_REQUIRED" && hasPasskeys {
                do {
                    let reauthStart = try await container.auth.passkeyReauthStart()
                    let cred = try await performReauth(reauthStart)
                    try await container.auth.passkeyReauthFinish(challengeId: reauthStart.challengeId, credential: cred)
                    try await registerOnce()
                } catch PasskeyFlowError.cancelled {
                    return
                } catch let retryErr {
                    error = retryErr.localizedDescription
                }
            } else {
                error = e.message
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func deletePasskey(
        id: Int64,
        performReauth: (_ start: PasskeyStart) async throws -> JSONValue
    ) async {
        guard !busy else { return }
        busy = true
        error = nil
        defer { busy = false }

        func syncAfterDelete() async {
            try? await container.auth.refresh(throwOnError: false)
            me = container.auth.me
            if let updated = try? await container.community.passkeys() {
                passkeys = updated
            } else {
                passkeys.removeAll { $0.id == id }
            }
        }

        do {
            _ = try await container.community.deletePasskey(id: id)
            await syncAfterDelete()
        } catch let e as ApiException where e.code == "RECENT_SIGN_IN_REQUIRED" {
            do {
                let reauthStart = try await container.auth.passkeyReauthStart()
                let cred = try await performReauth(reauthStart)
                try await container.auth.passkeyReauthFinish(challengeId: reauthStart.challengeId, credential: cred)
                _ = try await container.community.deletePasskey(id: id)
                await syncAfterDelete()
            } catch PasskeyFlowError.cancelled {
                return
            } catch let retryErr {
                error = retryErr.localizedDescription
            }
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func logout() async {
        await container.auth.logout()
    }

    public func logoutAll() async {
        await container.auth.logoutAll()
    }
}
