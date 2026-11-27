import Foundation
import Observation

public enum AuthState: Equatable, Sendable {
    case loading
    case signedOut
    case signedIn(Me)
    case locked(Me?)
    case pending(Me)

    public var me: Me? {
        switch self {
        case .signedIn(let m): return m
        case .locked(let m): return m
        case .pending(let m): return m
        case .loading, .signedOut: return nil
        }
    }
}

@MainActor
@Observable
public final class AuthRepository {
    private let apiProvider: ApiProvider
    private let bus: AuthEventBus
    private let cookies: PersistentCookieJar
    private let store: SecureStore?
    private let onCleared: (@Sendable () -> Void)?
    private let onLockedCallback: (@Sendable () -> Void)?
    private let uptimeProvider: @Sendable () -> TimeInterval

    public private(set) var state: AuthState = .loading

    public var me: Me? { state.me }

    private var api: any GraceRideAPIProtocol { apiProvider.get() }

    public static let cachedMeKey = "cached_me_v1"
    public static let cachedMeAtKey = "cached_me_at_ms"
    public static let cachedMeUptimeKey = "cached_me_uptime_s"
    public static let apnsTokenKey = "apns_device_token_v1"
    public static let hideLockDetailsKey = "notif_hide_lock_details"
    public static let notifPrefsKey = "notif_prefs_v1"

    public init(
        apiProvider: ApiProvider,
        bus: AuthEventBus,
        cookies: PersistentCookieJar,
        store: SecureStore? = nil,
        onCleared: (@Sendable () -> Void)? = nil,
        onLockedCallback: (@Sendable () -> Void)? = nil,
        uptimeProvider: @escaping @Sendable () -> TimeInterval = { ProcessInfo.processInfo.systemUptime }
    ) {
        self.apiProvider = apiProvider
        self.bus = bus
        self.cookies = cookies
        self.store = store
        self.onCleared = onCleared
        self.onLockedCallback = onLockedCallback
        self.uptimeProvider = uptimeProvider
    }

    private func stateFor(_ m: Me) -> AuthState {
        if m.locked { return .locked(m) }
        if m.status == "pending" { return .pending(m) }
        return .signedIn(m)
    }

    public func setApnsToken(_ token: String) {
        let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        store?.putString(Self.apnsTokenKey, trimmed)
    }

    public func storedApnsToken() -> String? {
        let trimmed = store?.getString(Self.apnsTokenKey)?.trimmingCharacters(in: .whitespacesAndNewlines)
        return (trimmed?.isEmpty == false) ? trimmed : nil
    }

    public func recordBackgroundTimestamp(
        nowMillis: Int64 = PersistentCookieJar.currentMillis(),
        nowUptime: TimeInterval? = nil
    ) {
        let resolvedUptime = nowUptime ?? uptimeProvider()
        switch state {
        case .signedIn, .pending:
            store?.putString(Self.cachedMeAtKey, String(nowMillis))
            store?.putString(Self.cachedMeUptimeKey, String(resolvedUptime))
        case .loading, .signedOut, .locked:
            break
        }
    }

    private func cacheMe(
        _ m: Me,
        nowMillis: Int64 = PersistentCookieJar.currentMillis(),
        nowUptime: TimeInterval? = nil
    ) {
        let resolvedUptime = nowUptime ?? uptimeProvider()
        guard let data = try? JSONEncoder().encode(m),
              let raw = String(data: data, encoding: .utf8) else {
            return
        }
        store?.putString(Self.cachedMeKey, raw)
        store?.putString(Self.cachedMeAtKey, String(nowMillis))
        store?.putString(Self.cachedMeUptimeKey, String(resolvedUptime))
    }

    private func loadCachedMe() -> Me? {
        guard let raw = store?.getString(Self.cachedMeKey),
              let data = raw.data(using: .utf8) else {
            return nil
        }
        return try? JSONDecoder().decode(Me.self, from: data)
    }

    public func isCachedIdleExpired(
        _ m: Me,
        nowMillis: Int64 = PersistentCookieJar.currentMillis(),
        nowUptime: TimeInterval? = nil
    ) -> Bool {
        let resolvedUptime = nowUptime ?? uptimeProvider()
        let effectiveMins = m.autoLockMins > 0
            ? m.autoLockMins
            : (m.autoLockMins == -1 ? 0 : m.sessionTimeoutMins)
        guard effectiveMins > 0 else { return false }
        guard let savedStr = store?.getString(Self.cachedMeAtKey),
              let savedAt = Int64(savedStr) else {
            return true
        }
        // Reject device wall-clock rollbacks offline.
        if nowMillis < savedAt {
            return true
        }
        let elapsedMins = (nowMillis - savedAt) / 60_000
        if elapsedMins >= Int64(effectiveMins) {
            return true
        }
        if let uptimeStr = store?.getString(Self.cachedMeUptimeKey),
           let savedUptime = Double(uptimeStr) {
            // Device rebooted (uptime reset) or monotonic elapsed idle reached threshold.
            if resolvedUptime < savedUptime {
                return true
            }
            let elapsedUptimeMins = Int64((resolvedUptime - savedUptime) / 60.0)
            if elapsedUptimeMins >= Int64(effectiveMins) {
                return true
            }
        }
        return false
    }

    @discardableResult
    public func enforceIdleTimeoutIfNeeded(
        nowMillis: Int64 = PersistentCookieJar.currentMillis(),
        nowUptime: TimeInterval? = nil
    ) -> Bool {
        switch state {
        case .signedIn(let m), .pending(let m):
            if isCachedIdleExpired(m, nowMillis: nowMillis, nowUptime: nowUptime) {
                onLocked()
                return true
            }
            return false
        case .loading, .signedOut, .locked:
            return false
        }
    }

    private func clearLocalSessionIfNeeded() {
        let hasLocal = state != .signedOut
            || cookies.hasCookies()
            || store?.getString(Self.cachedMeKey) != nil
            || store?.getString(Self.cachedMeAtKey) != nil
            || store?.getString(Self.cachedMeUptimeKey) != nil
            || store?.getString(Self.apnsTokenKey) != nil
            || store?.getString(Self.hideLockDetailsKey) != nil
            || store?.getString(Self.notifPrefsKey) != nil
        state = .signedOut
        guard hasLocal else { return }
        cookies.clear()
        store?.remove(Self.cachedMeKey)
        store?.remove(Self.cachedMeAtKey)
        store?.remove(Self.cachedMeUptimeKey)
        store?.remove(Self.apnsTokenKey)
        store?.remove(Self.hideLockDetailsKey)
        store?.remove(Self.notifPrefsKey)
        if let store {
            OfflineSnapshotStore.clear(store: store)
        }
        onCleared?()
    }

    public func refresh(throwOnError: Bool = false) async throws {
        do {
            let m = try await api.me()
            cacheMe(m)
            state = stateFor(m)
        } catch let e as ApiException {
            if e.status == 401 || e.status == 403 {
                clearLocalSessionIfNeeded()
                if throwOnError { throw e }
            } else {
                // Offline cold-start with valid session cookies: restore last verified profile.
                if state == .loading {
                    if e.status == 0 && cookies.hasCookies(), let cached = loadCachedMe() {
                        if isCachedIdleExpired(cached) && !cached.locked {
                            var lockedCopy = cached
                            lockedCopy.locked = true
                            if let store {
                                OfflineSnapshotStore.clear(store: store)
                            }
                            onLockedCallback?()
                            state = .locked(lockedCopy)
                        } else {
                            if cached.locked {
                                if let store {
                                    OfflineSnapshotStore.clear(store: store)
                                }
                                onLockedCallback?()
                            }
                            state = stateFor(cached)
                        }
                    } else if !cookies.hasCookies() {
                        clearLocalSessionIfNeeded()
                    } else {
                        state = .signedOut
                    }
                }
                if throwOnError { throw e }
            }
        }
    }

    public func onUnauthorized() {
        clearLocalSessionIfNeeded()
    }

    public func onLocked() {
        var lockedMe = me
        lockedMe?.locked = true
        if let lockedMe {
            cacheMe(lockedMe)
        }
        if let store {
            OfflineSnapshotStore.clear(store: store)
        }
        onLockedCallback?()
        state = .locked(lockedMe)
    }

    public func onPending() {
        guard var pendingMe = me else { return }
        pendingMe.status = "pending"
        cacheMe(pendingMe)
        state = .pending(pendingMe)
    }

    public func updateUnread(_ count: Int) {
        let safe = max(0, count)
        switch state {
        case .signedIn(var m):
            if m.unread != safe {
                m.unread = safe
                state = .signedIn(m)
            }
        case .locked(var m):
            if m != nil, m!.unread != safe {
                m!.unread = safe
                state = .locked(m)
            }
        case .pending(var m):
            if m.unread != safe {
                m.unread = safe
                state = .pending(m)
            }
        case .loading, .signedOut:
            break
        }
    }

    public func devUsers() async throws -> [DevUser] {
        try await api.devUsers()
    }

    public func devLogin(email: String) async throws {
        _ = try await api.devLogin(email: email)
        try await refresh(throwOnError: true)
    }

    public func turnstileConfig() async throws -> TurnstileConfigDto {
        try await api.turnstileConfig()
    }

    public func passkeyLoginStart(email: String, turnstileToken: String? = nil) async throws -> PasskeyStart {
        try await api.passkeyLoginStart(PasskeyStartReq(email: email.trimmingCharacters(in: .whitespacesAndNewlines), turnstileToken: turnstileToken))
    }

    public func passkeyLoginFinish(challengeId: String, credential: JSONValue) async throws {
        _ = try await api.passkeyLoginFinish(PasskeyFinishReq(challengeId: challengeId, credential: credential))
        try await refresh(throwOnError: true)
    }

    public func passkeyReauthStart() async throws -> PasskeyStart {
        try await api.passkeyReauthStart()
    }

    public func passkeyReauthFinish(challengeId: String, credential: JSONValue) async throws {
        _ = try await api.passkeyReauthFinish(PasskeyFinishReq(challengeId: challengeId, credential: credential))
        try await refresh(throwOnError: true)
    }

    public func passkeyRegisterStart() async throws -> PasskeyStart {
        try await api.passkeyRegisterStart()
    }

    public func passkeyRegisterFinish(challengeId: String, credential: JSONValue, label: String? = nil) async throws {
        _ = try await api.passkeyRegisterFinish(PasskeyFinishReq(challengeId: challengeId, credential: credential, label: label))
        try await refresh(throwOnError: true)
    }

    public func unlock(code: String, isBackupCode: Bool) async throws {
        _ = try await api.unlock(isBackupCode ? UnlockReq(backupCode: code) : UnlockReq(totpCode: code))
        try await refresh(throwOnError: true)
    }

    public func devUnlock() async throws {
        _ = try await api.unlock(UnlockReq(devUnlock: true))
        try await refresh(throwOnError: true)
    }

    public func logout(mobileToken: String? = nil) async {
        let token = mobileToken?.trimmingCharacters(in: .whitespacesAndNewlines)
        let stored = storedApnsToken()
        let effective = (token?.isEmpty == false) ? token : stored
        _ = try? await api.logout(LogoutReq(mobileToken: effective))
        clearLocalSessionIfNeeded()
    }

    public func logoutAll() async {
        _ = try? await api.logoutAll()
        clearLocalSessionIfNeeded()
    }

    public func acceptTerms() async throws {
        _ = try await api.acceptTerms(TermsReq(accepted: true))
        try await refresh(throwOnError: true)
    }
}
