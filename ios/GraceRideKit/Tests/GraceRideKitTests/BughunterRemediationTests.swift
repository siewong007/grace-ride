import Foundation
import XCTest
@testable import GraceRideKit

@MainActor
final class BughunterRemediationTests: XCTestCase {

    override func tearDown() {
        MockURLProtocol.reset()
        super.tearDown()
    }

    func testCachedIdleExpiryFallsBackToSessionTimeoutWhenAutoLockZero() {
        let store = InMemorySecureStore()
        let cookies = PersistentCookieJar(store: store)
        let client = HTTPClient(
            baseURL: URL(string: "http://127.0.0.1:8080/")!,
            cookieJar: cookies,
            authEvents: AuthEventBus(),
            langProvider: { "en" }
        )
        let auth = AuthRepository(
            apiProvider: ApiProvider(GraceRideAPI(client: client)),
            bus: AuthEventBus(),
            cookies: cookies,
            store: store
        )

        let nowMs: Int64 = 1_700_000_000_000
        let nowUptime: TimeInterval = 5_000.0
        store.putString(AuthRepository.cachedMeAtKey, String(nowMs - 10 * 60_000))
        store.putString(AuthRepository.cachedMeUptimeKey, String(nowUptime - 10 * 60.0))

        let fallbackMe = Me(
            id: 10,
            name: "Member",
            email: "member@example.org",
            autoLockMins: 0,
            sessionTimeoutMins: 15
        )
        XCTAssertFalse(auth.isCachedIdleExpired(fallbackMe, nowMillis: nowMs, nowUptime: nowUptime))

        store.putString(AuthRepository.cachedMeAtKey, String(nowMs - 16 * 60_000))
        store.putString(AuthRepository.cachedMeUptimeKey, String(nowUptime - 16 * 60.0))
        XCTAssertTrue(auth.isCachedIdleExpired(fallbackMe, nowMillis: nowMs, nowUptime: nowUptime))

        let disabledMe = Me(
            id: 10,
            name: "Member",
            email: "member@example.org",
            autoLockMins: -1,
            sessionTimeoutMins: 15
        )
        store.putString(AuthRepository.cachedMeAtKey, String(nowMs - 120 * 60_000))
        store.putString(AuthRepository.cachedMeUptimeKey, String(nowUptime - 120 * 60.0))
        XCTAssertFalse(auth.isCachedIdleExpired(disabledMe, nowMillis: nowMs, nowUptime: nowUptime))
    }

    func testEnforceIdleTimeoutLocksAndWipesOfflineCaches() async throws {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.protocolClasses = [MockURLProtocol.self]
        cfg.httpShouldSetCookies = false
        let store = InMemorySecureStore()
        let container = AppContainer(
            secureStore: store,
            offlineCache: OfflineCache(cacheDir: nil, secureStore: store),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: nil,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )

        MockURLProtocol.setHandler { req in
            let json = #"{"id":1,"name":"Member","email":"m@example.org","auto_lock_mins":5,"session_timeout_mins":60,"permissions":["rides.view"]}"#
            let resp = HTTPURLResponse(
                url: req.url!,
                statusCode: 200,
                httpVersion: nil,
                headerFields: ["Set-Cookie": "gr_session=tok-idle; Path=/; HttpOnly"]
            )!
            return (resp, Data(json.utf8))
        }
        try await container.auth.refresh(throwOnError: true)
        guard case .signedIn = container.auth.state else {
            return XCTFail("Expected .signedIn before idle timeout, got \(container.auth.state)")
        }

        let ride = Ride(
            id: 55,
            status: "confirmed",
            pickupAt: KlTime.today() + " 09:00:00",
            area: RideArea(id: 1, name: "SS2"),
            passenger: RidePassenger(id: 1, name: "Member"),
            viewer: RideViewer(isPassenger: true)
        )
        container.offlineCache.write(userId: 1, key: "rides_upcoming", value: ["ride-55"])
        OfflineSnapshotStore.save(store: store, userId: 1, rides: [ride])

        let futureMs = PersistentCookieJar.currentMillis() + 6 * 60_000
        let futureUptime = ProcessInfo.processInfo.systemUptime + 360.0
        let locked = container.auth.enforceIdleTimeoutIfNeeded(nowMillis: futureMs, nowUptime: futureUptime)
        XCTAssertTrue(locked)
        try? await Task.sleep(nanoseconds: 50_000_000)

        if case .locked(let m) = container.auth.state {
            XCTAssertEqual(m?.id, 1)
        } else {
            XCTFail("Expected .locked state after enforceIdleTimeoutIfNeeded, got \(container.auth.state)")
        }
        XCTAssertNil(OfflineSnapshotStore.load(store: store, userId: 1))
        XCTAssertNil(container.offlineCache.read([String].self, userId: 1, key: "rides_upcoming"))
    }

    func testOfflineRefreshWipesLocalSessionWhenCookiesMissing() async throws {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.protocolClasses = [MockURLProtocol.self]
        cfg.httpShouldSetCookies = false
        let store = InMemorySecureStore()
        store.putString(AuthRepository.cachedMeKey, #"{"id":9,"name":"Stale","email":"stale@example.org","permissions":[]}"#)
        store.putString(AuthRepository.cachedMeAtKey, String(PersistentCookieJar.currentMillis()))

        let container = AppContainer(
            secureStore: store,
            offlineCache: OfflineCache(cacheDir: nil, secureStore: store),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: nil,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )
        XCTAssertFalse(container.cookieJar.hasCookies())

        MockURLProtocol.setHandler { _ in throw URLError(.notConnectedToInternet) }
        try await container.auth.refresh()

        XCTAssertEqual(container.auth.state, .signedOut)
        XCTAssertNil(store.getString(AuthRepository.cachedMeKey))
        XCTAssertNil(store.getString(AuthRepository.cachedMeAtKey))
    }

    func testRoutesAndNotifLinkRouterRejectTraversalAndControlChars() {
        let member = Me(
            id: 1,
            name: "Member",
            email: "member@example.org",
            permissions: ["rides.view", "rides.create", "reimbursements.view"]
        )
        XCTAssertTrue(Routes.isWebPathPermitted("/reimbursements", me: member))
        XCTAssertEqual(NotifLinkRouter.resolve("/rides/42"), .native(route: "ride/42"))

        let unsafe = [
            "/../admin",
            "/rides/../admin",
            "/./rides",
            "/%2e%2e/admin",
            "/%2E%2E/admin",
            "/%2fadmin",
            "/rides%5cadmin",
            "/rides\\admin",
            "/rides\nfoo",
            "/rides\u{0000}",
            "//evil.com/rides",
            "javascript:alert(1)",
            "/api/auth/me",
        ]
        for candidate in unsafe {
            XCTAssertFalse(Routes.isWebPathPermitted(candidate, me: member), "Expected denied web path: \(candidate)")
            XCTAssertEqual(NotifLinkRouter.resolve(candidate), .none, "Expected .none destination: \(candidate)")
        }
    }

    func testQrParsingRejectsLegacyGrpassInput() {
        XCTAssertEqual(QrParsing.parseCheckinInput("GRPASS:10:MEM42", currentEventId: 10), .invalid)
        XCTAssertEqual(QrParsing.parseCheckinInput("grpass:10:MEM42", currentEventId: 10), .invalid)
    }

    func testWidgetSnapshotStoreUsesSecureStoreOnlyAndScrubsLegacyUserDefaults() {
        let suiteName = "group.org.pjstatecm.graceride.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let store = InMemorySecureStore()
        defaults.set(#"{"signedIn":true,"title":"LEGACY PLAINTEXT PII"}"#, forKey: "widget_summary_v1")

        // Loading must ignore legacy UserDefaults plaintext and scrub it immediately.
        let beforeSave = WidgetSnapshotStore.load(from: store, userDefaults: defaults)
        XCTAssertFalse(beforeSave.signedIn)
        XCTAssertNil(defaults.string(forKey: "widget_summary_v1"))

        let ride = Ride(
            id: 91,
            status: "confirmed",
            pickupAt: KlTime.today() + " 08:30:00",
            area: RideArea(id: 1, name: "SS2"),
            passenger: RidePassenger(id: 1, name: "Member"),
            viewer: RideViewer(isPassenger: true)
        )
        WidgetSnapshotStore.saveFromHome(
            signedIn: true,
            nextRide: ride,
            nextEvent: nil,
            lang: "en",
            hideLockDetails: false,
            store: store,
            userDefaults: defaults
        )

        XCTAssertNotNil(store.getString("widget_summary_v1"))
        XCTAssertNil(defaults.string(forKey: "widget_summary_v1"))
        let loaded = WidgetSnapshotStore.load(from: store, userDefaults: defaults)
        XCTAssertTrue(loaded.signedIn)
        XCTAssertEqual(loaded.rideId, 91)

        WidgetSnapshotStore.clear(store: store, userDefaults: defaults)
        XCTAssertNil(store.getString("widget_summary_v1"))
        XCTAssertNil(defaults.string(forKey: "widget_summary_v1"))
        XCTAssertFalse(WidgetSnapshotStore.load(from: store, userDefaults: defaults).signedIn)
    }
}
