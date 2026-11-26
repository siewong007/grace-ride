import XCTest
@testable import GraceRideKit

final class PersistentCookieJarTests: XCTestCase {

    func testRfc6265PathMatching() {
        XCTAssertTrue(PersistentCookieJar.pathMatch(requestPath: "/api", cookiePath: "/api"))
        XCTAssertTrue(PersistentCookieJar.pathMatch(requestPath: "/api/rides", cookiePath: "/api"))
        XCTAssertFalse(PersistentCookieJar.pathMatch(requestPath: "/apix", cookiePath: "/api"))
        XCTAssertTrue(PersistentCookieJar.pathMatch(requestPath: "/any", cookiePath: "/"))
    }

    func testSavesPersistsLoadsAndClearsCookies() {
        let store = InMemorySecureStore()
        let jar1 = PersistentCookieJar(store: store)
        let url = URL(string: "https://graceride.saliminn.my/api/auth/me")!

        jar1.saveFromResponse(
            url: url,
            headerFields: ["Set-Cookie": "__Host-gr_session=secret123; Path=/; Secure; HttpOnly; SameSite=Lax"]
        )
        XCTAssertTrue(jar1.hasCookies())
        XCTAssertEqual(jar1.cookieHeader(for: url), "__Host-gr_session=secret123")

        // HTTP URL must NOT receive Secure cookies
        let httpURL = URL(string: "http://graceride.saliminn.my/api/auth/me")!
        XCTAssertNil(jar1.cookieHeader(for: httpURL))

        // Reload from persisted SecureStore
        let jar2 = PersistentCookieJar(store: store)
        XCTAssertEqual(jar2.cookieHeader(for: url), "__Host-gr_session=secret123")

        // Subdomain must not match hostOnly cookie
        let subURL = URL(string: "https://sub.graceride.saliminn.my/api/auth/me")!
        XCTAssertNil(jar2.cookieHeader(for: subURL))

        jar2.clear()
        XCTAssertFalse(jar2.hasCookies())
        XCTAssertNil(jar2.cookieHeader(for: url))
    }

    func testExpiredCookiesAreEvicted() {
        let store = InMemorySecureStore()
        let jar = PersistentCookieJar(store: store)
        let now: Int64 = 1_700_000_000_000
        let url = URL(string: "https://graceride.saliminn.my/api/rides")!
        jar.save(cookies: [
            .init(
                name: "gr_session",
                value: "old",
                domain: "graceride.saliminn.my",
                path: "/",
                expiresAtMillis: now - 1000,
                secure: true,
                httpOnly: true,
                hostOnly: true
            )
        ], nowMillis: now)
        XCTAssertFalse(jar.hasCookies(nowMillis: now))
        XCTAssertNil(jar.cookieHeader(for: url, nowMillis: now))
    }

    func testCookiePrefixEnforcementForHostAndSecurePrefixes() {
        let store = InMemorySecureStore()
        let jar = PersistentCookieJar(store: store)
        let httpsRoot = URL(string: "https://graceride.saliminn.my/")!
        let httpRoot = URL(string: "http://graceride.saliminn.my/")!
        let httpsApi = URL(string: "https://graceride.saliminn.my/api/rides")!

        // __Host- rejected on HTTP
        jar.saveFromResponse(url: httpRoot, headerFields: ["Set-Cookie": "__Host-gr_session=bad1; Path=/; Secure"])
        XCTAssertNil(jar.cookieHeader(for: httpsRoot))

        // __Host- rejected without Secure
        jar.saveFromResponse(url: httpsRoot, headerFields: ["Set-Cookie": "__Host-gr_session=bad2; Path=/; HttpOnly"])
        XCTAssertNil(jar.cookieHeader(for: httpsRoot))

        // __Host- rejected when Path != "/"
        jar.saveFromResponse(url: httpsRoot, headerFields: ["Set-Cookie": "__Host-gr_session=bad3; Path=/api; Secure"])
        XCTAssertNil(jar.cookieHeader(for: httpsApi))

        // __Host- rejected when explicit Domain attribute is set (with or without leading dot)
        jar.saveFromResponse(
            url: httpsRoot,
            headerFields: ["Set-Cookie": "__Host-gr_session=bad4; Path=/; Secure; Domain=graceride.saliminn.my"]
        )
        XCTAssertNil(jar.cookieHeader(for: httpsRoot))
        jar.saveFromResponse(
            url: httpsRoot,
            headerFields: ["Set-Cookie": "__Host-gr_session=bad5; Path=/; Secure; Domain=.graceride.saliminn.my"]
        )
        XCTAssertNil(jar.cookieHeader(for: httpsRoot))

        // __Host- accepted when HTTPS + Secure + Path=/ + host-only
        jar.saveFromResponse(
            url: httpsRoot,
            headerFields: ["Set-Cookie": "__Host-gr_session=good1; Path=/; Secure; HttpOnly"]
        )
        XCTAssertEqual(jar.cookieHeader(for: httpsRoot), "__Host-gr_session=good1")

        // __Secure- rejected on HTTP or without Secure; allowed with Path=/api on HTTPS + Secure
        jar.saveFromResponse(url: httpRoot, headerFields: ["Set-Cookie": "__Secure-flag=bad; Path=/; Secure"])
        XCTAssertFalse(jar.cookieHeader(for: httpsApi)?.contains("__Secure-flag") ?? false)

        jar.saveFromResponse(url: httpsApi, headerFields: ["Set-Cookie": "__Secure-flag=bad2; Path=/api"])
        XCTAssertFalse(jar.cookieHeader(for: httpsApi)?.contains("__Secure-flag") ?? false)

        jar.saveFromResponse(url: httpsApi, headerFields: ["Set-Cookie": "__Secure-flag=good2; Path=/api; Secure"])
        let hdr = jar.cookieHeader(for: httpsApi) ?? ""
        XCTAssertTrue(hdr.contains("__Host-gr_session=good1"))
        XCTAssertTrue(hdr.contains("__Secure-flag=good2"))
    }
}
