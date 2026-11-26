import XCTest
@testable import GraceRideKit

@MainActor
final class SecurityAndOfflineTests: XCTestCase {

    func testOfflineColdStartRestoresCachedProfileWhenCookiesExistAndEnforcesAutoLock() async throws {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.protocolClasses = [MockURLProtocol.self]
        let store = InMemorySecureStore()
        let container = AppContainer(
            secureStore: store,
            offlineCache: OfflineCache(cacheDir: nil),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: nil,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )

        // 1. Online fetch caches Me
        MockURLProtocol.setHandler { req in
            let json = #"{"id":42,"name":"Elder Lim","email":"lim@example.org","auto_lock_mins":10,"permissions":["rides.view"]}"#
            let resp = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: ["Set-Cookie": "gr_session=abc; Path=/"])!
            return (resp, Data(json.utf8))
        }
        try await container.auth.refresh()
        XCTAssertEqual(container.auth.me?.id, 42)
        XCTAssertTrue(container.cookieJar.hasCookies())

        // 2. Simulate cold start offline with unexpired autoLock
        let coldContainer = AppContainer(
            secureStore: store,
            offlineCache: OfflineCache(cacheDir: nil),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: nil,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )
        MockURLProtocol.setHandler { _ in throw URLError(.notConnectedToInternet) }
        try await coldContainer.auth.refresh()
        if case .signedIn(let restored) = coldContainer.auth.state {
            XCTAssertEqual(restored.id, 42)
        } else {
            XCTFail("Expected offline restored SignedIn state, got \(coldContainer.auth.state)")
        }

        // 3. Simulate expired autoLock timestamp -> cold start enters Locked state
        store.putString(AuthRepository.cachedMeAtKey, String(PersistentCookieJar.currentMillis() - 15 * 60_000))
        let expiredContainer = AppContainer(
            secureStore: store,
            offlineCache: OfflineCache(cacheDir: nil),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: nil,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )
        try await expiredContainer.auth.refresh()
        if case .locked(let m) = expiredContainer.auth.state {
            XCTAssertEqual(m?.id, 42)
        } else {
            XCTFail("Expected .locked state when idle expired, got \(expiredContainer.auth.state)")
        }
    }

    func testOfflineCachePerUserIsolationAndSnapshotStore() {
        let cache = OfflineCache(cacheDir: nil)
        cache.write(userId: 1, key: "rides_upcoming", value: ["ride-a"])
        cache.write(userId: 2, key: "rides_upcoming", value: ["ride-b"])

        XCTAssertEqual(cache.read([String].self, userId: 1, key: "rides_upcoming")?.data, ["ride-a"])
        XCTAssertEqual(cache.read([String].self, userId: 2, key: "rides_upcoming")?.data, ["ride-b"])
        cache.clearUser(1)
        XCTAssertNil(cache.read([String].self, userId: 1, key: "rides_upcoming"))
        XCTAssertEqual(cache.read([String].self, userId: 2, key: "rides_upcoming")?.data, ["ride-b"])

        let store = InMemorySecureStore()
        let r = Ride(
            id: 99,
            status: "confirmed",
            pickupAt: KlTime.today() + " 08:30:00",
            area: RideArea(id: 1, name: "SS2"),
            passenger: RidePassenger(id: 1, name: "Member")
        )
        OfflineSnapshotStore.save(store: store, userId: 1, rides: [r])
        XCTAssertEqual(OfflineSnapshotStore.load(store: store, userId: 1)?.rides.first?.id, 99)
        XCTAssertNil(OfflineSnapshotStore.load(store: store, userId: 2))
        OfflineSnapshotStore.clear(store: store)
        XCTAssertNil(OfflineSnapshotStore.load(store: store, userId: 1))
    }

    func testOfflineCacheEncryptsWithAesGcmRejectsAadTamperAndDeletesLegacyJson() throws {
        let tmpDir = FileManager.default.temporaryDirectory.appendingPathComponent(
            "gr_offline_sec_\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: tmpDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tmpDir) }

        let legacyJson = tmpDir.appendingPathComponent("u1_secret.json")
        try Data(#"{"legacy":"plaintext"}"#.utf8).write(to: legacyJson)

        let store = InMemorySecureStore()
        let cache1 = OfflineCache(cacheDir: tmpDir, secureStore: store)
        cache1.write(userId: 1, key: "secret", value: ["HomeAddress-42-Gate9999"])

        XCTAssertFalse(FileManager.default.fileExists(atPath: legacyJson.path))
        let binFile = tmpDir.appendingPathComponent("u1_secret.bin")
        let blob = try Data(contentsOf: binFile)
        XCTAssertEqual(blob.prefix(4), Data("GRC1".utf8))
        XCTAssertNil(blob.range(of: Data("HomeAddress-42-Gate9999".utf8)))

        // Fresh OfflineCache instance using same SecureStore decrypts persisted ciphertext
        let cache2 = OfflineCache(cacheDir: tmpDir, secureStore: store)
        XCTAssertEqual(cache2.read([String].self, userId: 1, key: "secret")?.data, ["HomeAddress-42-Gate9999"])

        // Copying u1_secret.bin to u2_secret.bin must fail GCM authentication due to filename AAD binding
        let swapped = tmpDir.appendingPathComponent("u2_secret.bin")
        try FileManager.default.copyItem(at: binFile, to: swapped)
        XCTAssertNil(cache2.read([String].self, userId: 2, key: "secret"))
    }

    func testOfflineClockRollbackMonotonicUptimeAnd423PurgeCaches() async throws {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.protocolClasses = [MockURLProtocol.self]
        let tmpDir = FileManager.default.temporaryDirectory.appendingPathComponent(
            "gr_lock_purge_\(UUID().uuidString)",
            isDirectory: true
        )
        defer { try? FileManager.default.removeItem(at: tmpDir) }

        let store = InMemorySecureStore()
        let makeContainer = {
            AppContainer(
                secureStore: store,
                offlineCache: OfflineCache(cacheDir: tmpDir, secureStore: store),
                networkMonitor: NetworkMonitor(startMonitoring: false),
                uiDefaults: nil,
                widgetUserDefaults: nil,
                urlSessionConfiguration: cfg,
                debugMode: true,
                initialBaseURL: "http://127.0.0.1:8080/"
            )
        }

        let online = makeContainer()
        MockURLProtocol.setHandler { req in
            let json = #"{"id":42,"name":"Elder Lim","email":"lim@example.org","auto_lock_mins":10,"permissions":["rides.view"]}"#
            let resp = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: ["Set-Cookie": "gr_session=abc; Path=/"])!
            return (resp, Data(json.utf8))
        }
        try await online.auth.refresh()
        online.offlineCache.write(userId: 42, key: "rides_upcoming", value: ["ride-secret"])
        OfflineSnapshotStore.save(
            store: store,
            userId: 42,
            rides: [Ride(id: 10, status: "confirmed", pickupAt: KlTime.today() + " 08:30:00", area: RideArea(id: 1, name: "SS2"), passenger: RidePassenger(id: 42, name: "Elder Lim"))]
        )

        // 1. Wall-clock rollback (cached_me_at_ms in the future) forces lock and purges caches
        MockURLProtocol.setHandler { _ in throw URLError(.notConnectedToInternet) }
        let nowMs = PersistentCookieJar.currentMillis()
        store.putString(AuthRepository.cachedMeAtKey, String(nowMs + 5 * 60_000))
        let rollbackContainer = makeContainer()
        try await rollbackContainer.auth.refresh()
        await Task.yield()
        if case .locked = rollbackContainer.auth.state {
            // Expected
        } else {
            XCTFail("Expected .locked on wall-clock rollback, got \(rollbackContainer.auth.state)")
        }
        XCTAssertNil(OfflineSnapshotStore.load(store: store, userId: 42))
        XCTAssertNil(rollbackContainer.offlineCache.read([String].self, userId: 42, key: "rides_upcoming"))

        // 2. Monotonic uptime expiry (even when wall-clock looks fresh) forces lock
        store.putString(AuthRepository.cachedMeAtKey, String(nowMs))
        store.putString(AuthRepository.cachedMeUptimeKey, String(max(0.0, ProcessInfo.processInfo.systemUptime - 15 * 60.0)))
        let uptimeExpired = makeContainer()
        try await uptimeExpired.auth.refresh()
        if case .locked = uptimeExpired.auth.state {
            // Expected
        } else {
            XCTFail("Expected .locked when monotonic uptime exceeds autoLockMins")
        }

        // 3. Simulated reboot (saved uptime higher than current systemUptime) forces lock
        store.putString(AuthRepository.cachedMeAtKey, String(nowMs))
        store.putString(AuthRepository.cachedMeUptimeKey, String(ProcessInfo.processInfo.systemUptime + 5000.0))
        let rebooted = makeContainer()
        try await rebooted.auth.refresh()
        if case .locked = rebooted.auth.state {
            // Expected
        } else {
            XCTFail("Expected .locked when uptime resets across reboot")
        }

        // 4. Online 423 Locked from server purges OfflineCache and OfflineSnapshotStore
        let c423 = makeContainer()
        MockURLProtocol.setHandler { req in
            let json = #"{"id":42,"name":"Elder Lim","email":"lim@example.org","auto_lock_mins":10,"permissions":["rides.view"]}"#
            let resp = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: ["Set-Cookie": "gr_session=abc; Path=/"])!
            return (resp, Data(json.utf8))
        }
        try await c423.auth.refresh()
        c423.offlineCache.write(userId: 42, key: "rides_upcoming", value: ["ride-secret-2"])
        OfflineSnapshotStore.save(
            store: store,
            userId: 42,
            rides: [Ride(id: 11, status: "confirmed", pickupAt: KlTime.today() + " 09:00:00", area: RideArea(id: 1, name: "SS2"), passenger: RidePassenger(id: 42, name: "Elder Lim"))]
        )
        MockURLProtocol.setHandler { req in
            let resp = HTTPURLResponse(url: req.url!, statusCode: 423, httpVersion: nil, headerFields: nil)!
            return (resp, Data(#"{"code":"SESSION_LOCKED","error":"Locked"}"#.utf8))
        }
        _ = try? await c423.rides.upcoming()
        try? await Task.sleep(nanoseconds: 50_000_000)
        if case .locked = c423.auth.state {
            // Expected
        } else {
            XCTFail("Expected .locked after 423 response, got \(c423.auth.state)")
        }
        XCTAssertNil(OfflineSnapshotStore.load(store: store, userId: 42))
        XCTAssertNil(c423.offlineCache.read([String].self, userId: 42, key: "rides_upcoming"))
    }

    func testLogoutUnregistersApnsTokenClearsCachesAndWidgetRedactsHideLockDetails() async throws {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.protocolClasses = [MockURLProtocol.self]
        let suiteName = "group.org.pjstatecm.graceride.test.\(UUID().uuidString)"
        let widgetDefaults = UserDefaults(suiteName: suiteName)
        defer { widgetDefaults?.removePersistentDomain(forName: suiteName) }

        let store = InMemorySecureStore()
        let container = AppContainer(
            secureStore: store,
            offlineCache: OfflineCache(cacheDir: nil, secureStore: store),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: widgetDefaults,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )

        let ride = Ride(
            id: 77,
            status: "confirmed",
            pickupAt: KlTime.today() + " 09:00:00",
            area: RideArea(id: 1, name: "22 Jalan Harmoni SS2"),
            passenger: RidePassenger(id: 7, name: "Sister Tan"),
            viewer: RideViewer(isPassenger: true)
        )
        let event = EventItem(
            id: 88,
            title: "Cell Group Secret Address",
            startsAt: KlTime.today() + " 19:30:00",
            location: "No. 15 Gate 9999"
        )

        WidgetSnapshotStore.saveFromHome(
            signedIn: true,
            nextRide: ride,
            nextEvent: nil,
            lang: "en",
            hideLockDetails: true,
            store: store,
            userDefaults: widgetDefaults
        )
        let redactedRideSummary = WidgetSnapshotStore.load(from: store, userDefaults: widgetDefaults)
        XCTAssertEqual(redactedRideSummary.title, "Upcoming Ride")
        XCTAssertFalse(redactedRideSummary.subtitle.contains("22 Jalan Harmoni"))
        XCTAssertFalse(redactedRideSummary.showReadyButton)

        WidgetSnapshotStore.saveFromHome(
            signedIn: true,
            nextRide: nil,
            nextEvent: event,
            lang: "en",
            hideLockDetails: true,
            store: store,
            userDefaults: widgetDefaults
        )
        let redactedEventSummary = WidgetSnapshotStore.load(from: store, userDefaults: widgetDefaults)
        XCTAssertEqual(redactedEventSummary.title, "Upcoming Event")
        XCTAssertFalse(redactedEventSummary.subtitle.contains("Gate 9999"))

        // Verify passive polling header propagation
        MockURLProtocol.setHandler { req in
            let path = req.url?.path ?? ""
            let resp = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!
            if path.contains("/notifications") {
                return (resp, Data(#"{"items":[],"has_more":false,"unread":0}"#.utf8))
            }
            return (resp, Data("[]".utf8))
        }
        _ = try await container.community.notificationsPage(limit: 5, passive: true)
        _ = try await container.rides.upcoming(passive: true)
        _ = try await container.events.upcoming(passive: true)
        _ = try await container.rides.upcoming(passive: false)
        let recorded = MockURLProtocol.recordedRequests
        XCTAssertEqual(recorded.count, 4)
        XCTAssertEqual(recorded[0].value(forHTTPHeaderField: "X-Passive-Poll"), "1")
        XCTAssertEqual(recorded[1].value(forHTTPHeaderField: "X-Passive-Poll"), "1")
        XCTAssertEqual(recorded[2].value(forHTTPHeaderField: "X-Passive-Poll"), "1")
        XCTAssertNil(recorded[3].value(forHTTPHeaderField: "X-Passive-Poll"))

        // Verify APNs token forwarded in /api/auth/logout and all stores cleared
        container.auth.setApnsToken("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2")
        container.offlineCache.write(userId: 7, key: "rides_upcoming", value: ["ride-77"])
        OfflineSnapshotStore.save(store: store, userId: 7, rides: [ride])

        var logoutBodyData: Data?
        MockURLProtocol.setHandler { req in
            if let body = req.httpBody {
                logoutBodyData = body
            } else if let stream = req.httpBodyStream {
                stream.open()
                var data = Data()
                var buf = [UInt8](repeating: 0, count: 1024)
                while stream.hasBytesAvailable {
                    let n = stream.read(&buf, maxLength: buf.count)
                    if n > 0 { data.append(buf, count: n) } else { break }
                }
                stream.close()
                logoutBodyData = data
            }
            let resp = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!
            return (resp, Data(#"{"ok":true}"#.utf8))
        }
        await container.auth.logout()
        try? await Task.sleep(nanoseconds: 50_000_000)

        let bodyJson = logoutBodyData.flatMap { String(data: $0, encoding: .utf8) } ?? ""
        XCTAssertTrue(bodyJson.contains("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"))
        XCTAssertNil(container.auth.storedApnsToken())
        XCTAssertNil(container.offlineCache.read([String].self, userId: 7, key: "rides_upcoming"))
        XCTAssertNil(OfflineSnapshotStore.load(store: store, userId: 7))
        XCTAssertFalse(WidgetSnapshotStore.load(from: store, userDefaults: widgetDefaults).signedIn)
    }
}
