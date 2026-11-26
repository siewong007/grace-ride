import Foundation
import XCTest
@testable import GraceRideKit

final class MockURLProtocol: URLProtocol {
    private static let lock = NSLock()
    private static var handler: ((URLRequest) throws -> (HTTPURLResponse, Data))?
    static var recordedRequests: [URLRequest] = []

    static func setHandler(_ block: @escaping (URLRequest) throws -> (HTTPURLResponse, Data)) {
        lock.withLock {
            recordedRequests = []
            handler = block
        }
    }

    static func reset() {
        lock.withLock {
            recordedRequests = []
            handler = nil
        }
    }

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        let block = Self.lock.withLock { () -> ((URLRequest) throws -> (HTTPURLResponse, Data))? in
            Self.recordedRequests.append(request)
            return Self.handler
        }
        guard let block else {
            client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
            return
        }
        do {
            let (resp, data) = try block(request)
            client?.urlProtocol(self, didReceive: resp, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

@MainActor
final class ApiSessionTests: XCTestCase {
    private func makeTestContainer() -> AppContainer {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.protocolClasses = [MockURLProtocol.self]
        cfg.httpShouldSetCookies = false
        return AppContainer(
            secureStore: InMemorySecureStore(),
            offlineCache: OfflineCache(cacheDir: nil),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: nil,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )
    }

    override func tearDown() {
        MockURLProtocol.reset()
        super.tearDown()
    }

    func testRequestsSendAcceptLanguageAndOmitOriginHeader() async throws {
        let container = makeTestContainer()
        container.lang = "zh"

        MockURLProtocol.setHandler { req in
            let json = #"{"id":1,"name":"Test","email":"test@example.org"}"#
            let resp = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: ["Content-Type": "application/json"])!
            return (resp, Data(json.utf8))
        }

        let me = try await container.api.me()
        XCTAssertEqual(me.id, 1)
        let req = try XCTUnwrap(MockURLProtocol.recordedRequests.first)
        XCTAssertEqual(req.value(forHTTPHeaderField: "Accept-Language"), "zh")
        XCTAssertNil(req.value(forHTTPHeaderField: "Origin"))
    }

    func testGlobalAuthEventHandlingFor401And423And403Pending() async throws {
        let container = makeTestContainer()
        container.cookieJar.save(cookies: [
            .init(name: "gr_session", value: "tok", domain: "127.0.0.1", path: "/", expiresAtMillis: PersistentCookieJar.currentMillis() + 60_000, secure: false, httpOnly: true, hostOnly: true)
        ])

        // 423 Locked -> AuthState.locked
        MockURLProtocol.setHandler { req in
            let body = #"{"code":"SESSION_LOCKED","error":"Session locked"}"#
            let resp = HTTPURLResponse(url: req.url!, statusCode: 423, httpVersion: nil, headerFields: nil)!
            return (resp, Data(body.utf8))
        }
        do {
            _ = try await container.api.me()
            XCTFail("Expected 423 ApiException")
        } catch let e as ApiException {
            XCTAssertEqual(e.status, 423)
            XCTAssertEqual(e.code, "SESSION_LOCKED")
        }
        try await Task.sleep(nanoseconds: 50_000_000)
        if case .locked = container.auth.state {} else {
            XCTFail("Expected .locked auth state, got \(container.auth.state)")
        }

        // 401 Unauthorized -> AuthState.signedOut & clears cookies
        MockURLProtocol.setHandler { req in
            let body = #"{"code":"UNAUTHORIZED","error":"Sign in"}"#
            let resp = HTTPURLResponse(url: req.url!, statusCode: 401, httpVersion: nil, headerFields: nil)!
            return (resp, Data(body.utf8))
        }
        do {
            _ = try await container.api.me()
            XCTFail("Expected 401")
        } catch let e as ApiException {
            XCTAssertEqual(e.status, 401)
        }
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(container.auth.state, .signedOut)
        XCTAssertFalse(container.cookieJar.hasCookies())
    }

    func testMalformedJsonMapsToDecodeErrorAndFlexibleBoolHandlesNumbers() async throws {
        let container = makeTestContainer()
        MockURLProtocol.setHandler { req in
            let resp = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!
            return (resp, Data("{not-json".utf8))
        }
        do {
            _ = try await container.api.me()
            XCTFail("Expected DECODE_ERROR")
        } catch let e as ApiException {
            XCTAssertEqual(e.code, "DECODE_ERROR")
        }

        let eventJson = #"{"id":5,"title":"Worship","starts_at":"2026-12-01 09:00:00","my_signup":{"status":"going","attended":1}}"#
        let decoded = try JSONDecoder().decode(EventItem.self, from: Data(eventJson.utf8))
        XCTAssertEqual(decoded.mySignup?.attended, true)
    }
}
