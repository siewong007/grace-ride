import Foundation
import Observation

public enum ThemeMode: String, CaseIterable, Sendable {
    case system
    case light
    case dark
}

public enum AppContainerError: Error, LocalizedError, Sendable {
    case releaseOverrideNotAllowed
    case invalidServerURL

    public var errorDescription: String? {
        switch self {
        case .releaseOverrideNotAllowed:
            return "Server URL override is only available in debug builds"
        case .invalidServerURL:
            return "Enter a valid http:// or https:// server URL."
        }
    }
}

@MainActor
@Observable
public final class AppContainer {
    public static let productionBaseURLString = "https://graceride.saliminn.my/"
    public static let debugDefaultBaseURLString = "http://127.0.0.1:8080/"
    public static let clearImageCacheNotification = Notification.Name("org.pjstatecm.graceride.clearImageCache")

    public let secureStore: SecureStore
    public let cookieJar: PersistentCookieJar
    public let authEvents: AuthEventBus
    public let offlineCache: OfflineCache
    public let networkMonitor: NetworkMonitor
    public let apiProvider: ApiProvider
    public let auth: AuthRepository
    public let rides: RidesRepository
    public let events: EventsRepository
    public let shares: SharesRepository
    public let community: CommunityRepository
    public let realtimeStream: RealtimeStreamClient
    public let widgetUserDefaults: UserDefaults?

    public let debugMode: Bool
    public var launchSplashShown: Bool = false

    public var isLiveLocationSharing: Bool = false
    public var onStopLiveLocation: (@MainActor (_ notifyServer: Bool) -> Void)?
    public var onSessionCleared: (@MainActor () -> Void)?
    public var onSessionLocked: (@MainActor () -> Void)?
    public var onWidgetChanged: (@MainActor () -> Void)?
    public var onNotifySurfaced: (@MainActor ([NotificationItem]) -> Void)?

    private let uiDefaults: UserDefaults?
    private let sessionConfig: URLSessionConfiguration?
    private var currentHttpClient: HTTPClient!
    private var authEventTask: Task<Void, Never>?

    public var lang: String {
        didSet {
            let norm = lang.hasPrefix("zh") ? "zh" : "en"
            if lang != norm { lang = norm; return }
            uiDefaults?.set(norm, forKey: "gr_lang")
            widgetUserDefaults?.set(norm, forKey: "gr_lang")
            secureStore.putString("lang", norm)
        }
    }

    public var themeMode: ThemeMode {
        didSet {
            uiDefaults?.set(themeMode.rawValue, forKey: "gr_theme_mode")
            secureStore.putString("theme_mode", themeMode.rawValue)
        }
    }

    public private(set) var baseURLString: String

    public var baseURL: URL {
        URL(string: baseURLString) ?? URL(string: Self.productionBaseURLString)!
    }

    public var api: any GraceRideAPIProtocol { apiProvider.get() }
    public var httpClient: HTTPClient { currentHttpClient }

    public init(
        secureStore: SecureStore? = nil,
        offlineCache: OfflineCache? = nil,
        networkMonitor: NetworkMonitor? = nil,
        uiDefaults: UserDefaults? = .standard,
        widgetUserDefaults: UserDefaults? = UserDefaults(suiteName: WidgetSnapshotStore.appGroupSuiteName),
        urlSessionConfiguration: URLSessionConfiguration? = nil,
        apiOverride: (any GraceRideAPIProtocol)? = nil,
        debugMode: Bool? = nil,
        initialBaseURL: String? = nil,
        uptimeProvider: @escaping @Sendable () -> TimeInterval = { ProcessInfo.processInfo.systemUptime }
    ) {
        let isDebug: Bool
        if let debugMode {
            isDebug = debugMode
        } else {
            #if DEBUG
            isDebug = true
            #else
            isDebug = false
            #endif
        }
        self.debugMode = isDebug

        let store = secureStore ?? KeychainSecureStore()
        let jar = PersistentCookieJar(store: store)
        let bus = AuthEventBus()
        self.secureStore = store
        self.cookieJar = jar
        self.authEvents = bus
        self.offlineCache = offlineCache ?? OfflineCache(cacheDir: OfflineCache.defaultCacheDirectory(), secureStore: store)
        self.networkMonitor = networkMonitor ?? NetworkMonitor(startMonitoring: true)
        self.uiDefaults = uiDefaults
        self.widgetUserDefaults = widgetUserDefaults
        self.sessionConfig = urlSessionConfiguration

        let devicePref = Locale.preferredLanguages.first?.hasPrefix("zh") == true ? "zh" : "en"
        let savedLang = uiDefaults?.string(forKey: "gr_lang") ?? store.getString("lang") ?? devicePref
        self.lang = savedLang.hasPrefix("zh") ? "zh" : "en"

        let savedThemeRaw = uiDefaults?.string(forKey: "gr_theme_mode") ?? store.getString("theme_mode") ?? "system"
        self.themeMode = ThemeMode(rawValue: savedThemeRaw) ?? .system

        let savedBase = (uiDefaults?.string(forKey: "gr_base_url_override") ?? store.getString("base_url_override"))
        let defaultUrl = isDebug ? Self.debugDefaultBaseURLString : Self.productionBaseURLString
        let resolvedBaseURL: String
        if let initialBaseURL {
            resolvedBaseURL = initialBaseURL.hasSuffix("/") ? initialBaseURL : "\(initialBaseURL)/"
        } else if isDebug, let savedBase, !savedBase.isEmpty {
            resolvedBaseURL = savedBase.hasSuffix("/") ? savedBase : "\(savedBase)/"
        } else {
            resolvedBaseURL = defaultUrl
        }
        self.baseURLString = resolvedBaseURL

        final class DefaultsBox: @unchecked Sendable {
            let defaults: UserDefaults?
            init(_ defaults: UserDefaults?) { self.defaults = defaults }
        }
        let defaultsBox = DefaultsBox(uiDefaults)

        let initialClient = HTTPClient(
            baseURL: URL(string: resolvedBaseURL) ?? URL(string: defaultUrl)!,
            cookieJar: jar,
            authEvents: bus,
            langProvider: { [store, defaultsBox] in
                defaultsBox.defaults?.string(forKey: "gr_lang") ?? store.getString("lang") ?? "en"
            },
            configuration: urlSessionConfiguration
        )
        self.currentHttpClient = initialClient

        let initialApi = apiOverride ?? GraceRideAPI(client: initialClient)
        let provider = ApiProvider(initialApi)
        self.apiProvider = provider

        let ridesRepo = RidesRepository(apiProvider: provider)
        let eventsRepo = EventsRepository(apiProvider: provider)
        let sharesRepo = SharesRepository(apiProvider: provider)
        let communityRepo = CommunityRepository(apiProvider: provider)
        self.rides = ridesRepo
        self.events = eventsRepo
        self.shares = sharesRepo
        self.community = communityRepo

        final class ContainerWeakBox: @unchecked Sendable {
            weak var container: AppContainer?
        }
        let weakBox = ContainerWeakBox()

        let authRepo = AuthRepository(
            apiProvider: provider,
            bus: bus,
            cookies: jar,
            store: store,
            onCleared: {
                ridesRepo.clearAreasCache()
                Task { @MainActor in
                    guard let container = weakBox.container else { return }
                    container.offlineCache.clearAll()
                    NotificationCenter.default.post(name: Self.clearImageCacheNotification, object: nil)
                    URLCache.shared.removeAllCachedResponses()
                    container.onStopLiveLocation?(false)
                    container.isLiveLocationSharing = false
                    WidgetSnapshotStore.clear(store: container.secureStore, userDefaults: container.widgetUserDefaults)
                    container.onWidgetChanged?()
                    container.onSessionCleared?()
                    container.realtimeStream.onAuthChanged()
                }
            },
            onLockedCallback: {
                ridesRepo.clearAreasCache()
                Task { @MainActor in
                    guard let container = weakBox.container else { return }
                    container.offlineCache.clearAll()
                    NotificationCenter.default.post(name: Self.clearImageCacheNotification, object: nil)
                    URLCache.shared.removeAllCachedResponses()
                    container.onStopLiveLocation?(false)
                    container.isLiveLocationSharing = false
                    WidgetSnapshotStore.clear(store: container.secureStore, userDefaults: container.widgetUserDefaults)
                    container.onWidgetChanged?()
                    container.onSessionLocked?()
                    container.realtimeStream.onAuthChanged()
                }
            },
            uptimeProvider: uptimeProvider
        )
        self.auth = authRepo

        self.realtimeStream = RealtimeStreamClient(
            httpClientProvider: {
                MainActor.assumeIsolated {
                    weakBox.container?.currentHttpClient ?? initialClient
                }
            },
            auth: authRepo,
            community: communityRepo,
            authEvents: bus,
            store: store,
            onNotifySurfaced: { items in
                weakBox.container?.onNotifySurfaced?(items)
            }
        )

        weakBox.container = self
        startAuthEventListener()
    }

    private func startAuthEventListener() {
        authEventTask?.cancel()
        let stream = authEvents.events()
        authEventTask = Task { [weak self] in
            for await event in stream {
                guard let self else { break }
                switch event {
                case .unauthorized:
                    self.onStopLiveLocation?(false)
                    self.isLiveLocationSharing = false
                    self.auth.onUnauthorized()
                    self.realtimeStream.onAuthChanged()
                case .locked:
                    self.onStopLiveLocation?(false)
                    self.isLiveLocationSharing = false
                    self.auth.onLocked()
                    self.realtimeStream.onAuthChanged()
                case .pending:
                    self.auth.onPending()
                    self.realtimeStream.onAuthChanged()
                }
            }
        }
    }

    public func setBaseUrl(_ raw: String) throws {
        guard debugMode else { throw AppContainerError.releaseOverrideNotAllowed }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let parsed = URL(string: trimmed),
              let scheme = parsed.scheme?.lowercased(),
              ["http", "https"].contains(scheme),
              let host = parsed.host,
              !host.isEmpty else {
            throw AppContainerError.invalidServerURL
        }
        let normalized = trimmed.hasSuffix("/") ? trimmed : "\(trimmed)/"
        baseURLString = normalized
        uiDefaults?.set(normalized, forKey: "gr_base_url_override")
        secureStore.putString("base_url_override", normalized)
        rebuildApi()
    }

    private func rebuildApi() {
        realtimeStream.stop()
        final class DefaultsBox: @unchecked Sendable {
            let defaults: UserDefaults?
            init(_ defaults: UserDefaults?) { self.defaults = defaults }
        }
        let langStore = secureStore
        let defaultsBox = DefaultsBox(uiDefaults)
        let client = HTTPClient(
            baseURL: baseURL,
            cookieJar: cookieJar,
            authEvents: authEvents,
            langProvider: { [langStore, defaultsBox] in
                defaultsBox.defaults?.string(forKey: "gr_lang") ?? langStore.getString("lang") ?? "en"
            },
            configuration: sessionConfig
        )
        currentHttpClient = client
        apiProvider.swap(GraceRideAPI(client: client))
        realtimeStream.start()
    }

    public func fileURL(id: Int64) -> URL? {
        try? currentHttpClient.resolveURL(path: "api/files/\(id)")
    }

    public func webURL(path: String) -> URL? {
        let clean = path.hasPrefix("/") ? String(path.dropFirst()) : path
        return URL(string: clean, relativeTo: baseURL)?.absoluteURL
    }
}
