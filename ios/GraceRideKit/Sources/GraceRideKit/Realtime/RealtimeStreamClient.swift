import Foundation
import Observation

public enum RealtimeEvent: Equatable, Sendable {
    case chat
    case notify
    case resume
    case other(String)

    public static func parse(_ kind: String) -> RealtimeEvent {
        switch kind {
        case "chat": return .chat
        case "notify": return .notify
        case "resume": return .resume
        default: return .other(kind)
        }
    }
}

public struct NotificationSyncResult: Equatable, Sendable {
    public var newWatermark: Int64
    public var surfaced: [NotificationItem]

    public init(newWatermark: Int64, surfaced: [NotificationItem]) {
        self.newWatermark = newWatermark
        self.surfaced = surfaced
    }
}

/// Real-time SSE client for `/api/stream`, syncing per-user notification watermarks,
/// badge counts, and broadcasting typed `RealtimeEvent` refreshes across active ViewModels.
@MainActor
public final class RealtimeStreamClient {
    private let httpClientProvider: () -> HTTPClient
    private let auth: AuthRepository
    private let community: CommunityRepository
    private let authEvents: AuthEventBus
    private let store: SecureStore?
    private let onNotifySurfaced: (@MainActor ([NotificationItem]) -> Void)?

    private var sseTask: Task<Void, Never>?
    private var isForeground: Bool = false
    private var isStarted: Bool = false
    private var lastUserId: Int64?
    private var lastSeenNotifId: Int64 = 0
    private var baselineInitialized: Bool = false
    private var continuations: [UUID: AsyncStream<RealtimeEvent>.Continuation] = [:]

    public init(
        httpClientProvider: @escaping () -> HTTPClient,
        auth: AuthRepository,
        community: CommunityRepository,
        authEvents: AuthEventBus,
        store: SecureStore? = nil,
        onNotifySurfaced: (@MainActor ([NotificationItem]) -> Void)? = nil
    ) {
        self.httpClientProvider = httpClientProvider
        self.auth = auth
        self.community = community
        self.authEvents = authEvents
        self.store = store
        self.onNotifySurfaced = onNotifySurfaced
    }

    public func events() -> AsyncStream<RealtimeEvent> {
        let id = UUID()
        return AsyncStream { continuation in
            continuations[id] = continuation
            continuation.onTermination = { [weak self] _ in
                Task { @MainActor [weak self] in
                    _ = self?.continuations.removeValue(forKey: id)
                }
            }
        }
    }

    private func emit(_ event: RealtimeEvent) {
        for c in continuations.values {
            c.yield(event)
        }
    }

    public func start() {
        isStarted = true
        evaluateConnection()
    }

    public func stop() {
        isStarted = false
        sseTask?.cancel()
        sseTask = nil
    }

    public func setForeground(_ foreground: Bool) {
        let becameForeground = !isForeground && foreground
        isForeground = foreground
        if becameForeground {
            emit(.resume)
        }
        evaluateConnection()
    }

    public func onAuthChanged() {
        let uid = auth.state.me?.id
        if uid != lastUserId {
            lastUserId = uid
            if let uid, let saved = Self.readWatermark(store: store, userId: uid) {
                lastSeenNotifId = saved
                baselineInitialized = true
            } else {
                lastSeenNotifId = 0
                baselineInitialized = false
            }
        }
        evaluateConnection()
    }

    public func evaluateConnection() {
        let shouldConnect: Bool
        if isStarted, isForeground, case .signedIn(let me) = auth.state, me.termsAccepted {
            shouldConnect = true
        } else {
            shouldConnect = false
        }

        if !shouldConnect {
            sseTask?.cancel()
            sseTask = nil
            return
        }
        if sseTask != nil { return }

        sseTask = Task { [weak self] in
            await self?.runSseLoop()
        }
    }

    private func runSseLoop() async {
        var backoffSecs: Double = 2.0
        while !Task.isCancelled {
            let client = httpClientProvider()
            guard let url = try? client.resolveURL(path: "api/stream") else { break }
            var req = client.buildRequest(method: "GET", url: url, accept: "text/event-stream")
            req.timeoutInterval = 3600

            var terminalAuthStatus = false
            do {
                let sseConfig = URLSessionConfiguration.ephemeral
                sseConfig.httpShouldSetCookies = false
                sseConfig.httpCookieAcceptPolicy = .never
                sseConfig.timeoutIntervalForRequest = 3600
                sseConfig.timeoutIntervalForResource = 0
                let redirectDelegate = CookieStrippingRedirectDelegate()
                let session = URLSession(configuration: sseConfig, delegate: redirectDelegate, delegateQueue: nil)
                defer { session.invalidateAndCancel() }

                let (bytes, response) = try await session.bytes(for: req)
                if let http = response as? HTTPURLResponse {
                    if http.statusCode == 401 {
                        terminalAuthStatus = true
                        auth.onUnauthorized()
                        authEvents.emit(.unauthorized)
                    } else if http.statusCode == 423 {
                        terminalAuthStatus = true
                        auth.onLocked()
                        authEvents.emit(.locked)
                    } else if (200...299).contains(http.statusCode) {
                        backoffSecs = 2.0
                        await syncLatestAndNotify(surfaceNew: false)

                        var currentEvent: String?
                        var dataLines: [String] = []

                        for try await line in bytes.lines {
                            if Task.isCancelled { break }
                            if line.isEmpty {
                                if !dataLines.isEmpty {
                                    let payload = dataLines.joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
                                    await handleSseFrame(type: currentEvent, data: payload)
                                }
                                currentEvent = nil
                                dataLines.removeAll(keepingCapacity: true)
                            } else if line.hasPrefix("event:") {
                                currentEvent = String(line.dropFirst("event:".count)).trimmingCharacters(in: .whitespaces)
                            } else if line.hasPrefix("data:") {
                                dataLines.append(String(line.dropFirst("data:".count)).trimmingCharacters(in: .whitespaces))
                            }
                        }
                    }
                }
            } catch {
                // Transient drop; retry with exponential backoff + jitter.
            }

            if terminalAuthStatus || Task.isCancelled { break }
            let jitter = Double.random(in: 0...(backoffSecs * 0.25))
            try? await Task.sleep(nanoseconds: UInt64((backoffSecs + jitter) * 1_000_000_000))
            backoffSecs = min(backoffSecs * 2.0, 30.0)
        }
        sseTask = nil
    }

    private func handleSseFrame(type: String?, data: String) async {
        if type == "unread", let count = Int(data) {
            auth.updateUnread(count)
            return
        }
        guard !data.isEmpty else { return }
        let ev = RealtimeEvent.parse(data)
        emit(ev)
        if ev == .notify {
            await syncLatestAndNotify(surfaceNew: true)
        }
    }

    public func syncLatestAndNotify(surfaceNew: Bool, passive: Bool = false) async {
        guard case .signedIn(let me) = auth.state else { return }
        let prev = store != nil ? Self.readWatermark(store: store, userId: me.id) : (baselineInitialized ? lastSeenNotifId : nil)
        guard let res = try? await Self.syncNotifications(
            community: community,
            auth: auth,
            store: store,
            userId: me.id,
            inMemoryLastSeenId: prev,
            surfaceNew: surfaceNew,
            passive: passive
        ) else {
            return
        }
        lastSeenNotifId = res.newWatermark
        baselineInitialized = true
        if !res.surfaced.isEmpty {
            onNotifySurfaced?(res.surfaced)
        }
    }

    // MARK: - Notification Filtering & Watermarks

    public static func watermarkKey(userId: Int64) -> String {
        "notif_last_seen_id_\(userId)"
    }

    public static func readWatermark(store: SecureStore?, userId: Int64) -> Int64? {
        store?.getString(watermarkKey(userId: userId)).flatMap(Int64.init)
    }

    public static func writeWatermark(store: SecureStore?, userId: Int64, id: Int64) {
        store?.putString(watermarkKey(userId: userId), String(id))
    }

    public static func readCachedPrefs(store: SecureStore?) -> NotificationPrefsDto? {
        guard let raw = store?.getString(AuthRepository.notifPrefsKey),
              let data = raw.data(using: .utf8) else {
            return nil
        }
        return try? JSONDecoder().decode(NotificationPrefsDto.self, from: data)
    }

    public static func writeCachedPrefs(store: SecureStore?, prefs: NotificationPrefsDto) {
        if let data = try? JSONEncoder().encode(prefs),
           let raw = String(data: data, encoding: .utf8) {
            store?.putString(AuthRepository.notifPrefsKey, raw)
        }
        store?.putString(AuthRepository.hideLockDetailsKey, prefs.hideLockDetails ? "1" : "0")
    }

    private nonisolated static func parseHmMinutes(_ hm: String?) -> Int? {
        guard let parts = hm?.trimmingCharacters(in: .whitespacesAndNewlines).split(separator: ":"),
              parts.count == 2,
              let h = Int(parts[0]), let m = Int(parts[1]),
              (0...23).contains(h), (0...59).contains(m) else {
            return nil
        }
        return h * 60 + m
    }

    public nonisolated static func shouldSurfaceNotification(
        _ item: NotificationItem,
        prefs: NotificationPrefsDto?,
        nowHm: String? = nil
    ) -> Bool {
        guard let prefs else { return true }
        guard prefs.pushOptIn else { return false }
        if item.priority == "urgent" { return true }
        if prefs.mutedCategories.contains(item.category) { return false }

        let currentHm: String = nowHm ?? {
            let comps = KlTime.components(Date())
            return String(format: "%02d:%02d", comps.hour ?? 0, comps.minute ?? 0)
        }()

        if let startMin = parseHmMinutes(prefs.quietHoursStart),
           let endMin = parseHmMinutes(prefs.quietHoursEnd),
           let nowMin = parseHmMinutes(currentHm),
           startMin != endMin {
            let inQuiet = startMin < endMin
                ? (nowMin >= startMin && nowMin < endMin)
                : (nowMin >= startMin || nowMin < endMin)
            if inQuiet { return false }
        }
        return true
    }

    public static func syncNotifications(
        community: CommunityRepository,
        auth: AuthRepository,
        store: SecureStore?,
        userId: Int64,
        inMemoryLastSeenId: Int64? = nil,
        surfaceNew: Bool = true,
        prefs: NotificationPrefsDto? = nil,
        passive: Bool = false
    ) async throws -> NotificationSyncResult {
        let effectivePrefs = prefs ?? readCachedPrefs(store: store)
        let page = try await community.notificationsPage(limit: 5, passive: passive)
        auth.updateUnread(page.unread)
        let maxId = page.items.map(\.id).max() ?? 0
        guard let existing = readWatermark(store: store, userId: userId) ?? inMemoryLastSeenId else {
            writeWatermark(store: store, userId: userId, id: maxId)
            return NotificationSyncResult(newWatermark: maxId, surfaced: [])
        }
        let newlySurfaced: [NotificationItem]
        if surfaceNew && maxId > existing {
            newlySurfaced = page.items
                .filter { $0.id > existing && !$0.read && shouldSurfaceNotification($0, prefs: effectivePrefs) }
                .sorted { $0.id < $1.id }
        } else {
            newlySurfaced = []
        }
        let nextWatermark = max(existing, maxId)
        if nextWatermark > existing {
            writeWatermark(store: store, userId: userId, id: nextWatermark)
        }
        return NotificationSyncResult(newWatermark: nextWatermark, surfaced: newlySurfaced)
    }
}
