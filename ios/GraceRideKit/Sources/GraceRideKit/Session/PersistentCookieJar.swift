import Foundation

/// Persists the HttpOnly session cookie (`gr_session` / `__Host-gr_session`) in encrypted
/// `SecureStore`. Session tokens are credentials: never logged and cleared on sign-out.
public final class PersistentCookieJar: @unchecked Sendable {
    public struct StoredCookie: Codable, Equatable, Sendable {
        public var name: String
        public var value: String
        public var domain: String
        public var path: String
        public var expiresAtMillis: Int64
        public var secure: Bool
        public var httpOnly: Bool
        public var hostOnly: Bool

        public init(
            name: String,
            value: String,
            domain: String,
            path: String,
            expiresAtMillis: Int64,
            secure: Bool,
            httpOnly: Bool,
            hostOnly: Bool
        ) {
            self.name = name
            self.value = value
            self.domain = domain
            self.path = path
            self.expiresAtMillis = expiresAtMillis
            self.secure = secure
            self.httpOnly = httpOnly
            self.hostOnly = hostOnly
        }
    }

    private let store: SecureStore
    private let key = "cookies"
    private let lock = NSLock()
    private var cache: [StoredCookie]?

    public init(store: SecureStore) {
        self.store = store
    }

    private func ensureLoadedLocked() -> [StoredCookie] {
        if let cache { return cache }
        let loaded = loadFromStore()
        cache = loaded
        return loaded
    }

    private func loadFromStore() -> [StoredCookie] {
        guard let raw = store.getString(key), let data = raw.data(using: .utf8) else {
            return []
        }
        return (try? JSONDecoder().decode([StoredCookie].self, from: data)) ?? []
    }

    private func persistLocked(_ list: [StoredCookie]) {
        guard let data = try? JSONEncoder().encode(list),
              let text = String(data: data, encoding: .utf8) else {
            return
        }
        store.putString(key, text)
    }

    public func saveFromResponse(url: URL, headerFields: [String: String], nowMillis: Int64 = PersistentCookieJar.currentMillis()) {
        let httpCookies = HTTPCookie.cookies(withResponseHeaderFields: headerFields, for: url)
        if httpCookies.isEmpty { return }

        let isHttps = url.scheme?.lowercased() == "https"
        let parsed = httpCookies.compactMap { cookie -> StoredCookie? in
            let rawDomain = cookie.domain.lowercased()
            let explicitDomainProp = (cookie.properties?[.domain] as? String ?? "").lowercased()
            let hasExplicitDomain = Self.hasExplicitDomainAttribute(cookieName: cookie.name, headerFields: headerFields)
            let isHostOnly = !rawDomain.hasPrefix(".") && !explicitDomainProp.hasPrefix(".") && !hasExplicitDomain
            let cleanDomain = rawDomain.hasPrefix(".") ? String(rawDomain.dropFirst()) : rawDomain
            let path = cookie.path.isEmpty ? "/" : cookie.path
            guard Self.isValidPrefixPolicy(
                name: cookie.name,
                path: path,
                secure: cookie.isSecure,
                hostOnly: isHostOnly,
                isHttps: isHttps
            ) else {
                return nil
            }
            let expMs: Int64
            if let exp = cookie.expiresDate {
                expMs = Int64(exp.timeIntervalSince1970 * 1000.0)
            } else {
                // Session cookie fallback: persist across process restarts for 14 days (server enforces real TTL).
                expMs = nowMillis + 14 * 24 * 60 * 60 * 1000
            }
            return StoredCookie(
                name: cookie.name,
                value: cookie.value,
                domain: cleanDomain.isEmpty ? (url.host?.lowercased() ?? "") : cleanDomain,
                path: path,
                expiresAtMillis: expMs,
                secure: cookie.isSecure,
                httpOnly: cookie.isHTTPOnly,
                hostOnly: isHostOnly
            )
        }
        if !parsed.isEmpty {
            save(cookies: parsed, nowMillis: nowMillis)
        }
    }

    public func save(cookies: [StoredCookie], nowMillis: Int64 = PersistentCookieJar.currentMillis()) {
        lock.withLock {
            var list = ensureLoadedLocked()
            list.removeAll { $0.expiresAtMillis <= nowMillis }
            for c in cookies {
                guard Self.isValidPrefixPolicy(
                    name: c.name,
                    path: c.path,
                    secure: c.secure,
                    hostOnly: c.hostOnly,
                    isHttps: c.secure
                ) else {
                    continue
                }
                list.removeAll { $0.name == c.name && $0.domain == c.domain && $0.path == c.path }
                if c.expiresAtMillis > nowMillis {
                    list.append(c)
                }
            }
            cache = list
            persistLocked(list)
        }
    }

    public func loadForRequest(url: URL, nowMillis: Int64 = PersistentCookieJar.currentMillis()) -> [StoredCookie] {
        guard let host = url.host?.lowercased() else { return [] }
        let reqPath = url.path.isEmpty ? "/" : url.path
        let isHttps = url.scheme?.lowercased() == "https"
        return lock.withLock {
            let list = ensureLoadedLocked()
            return list.filter { c in
                guard c.expiresAtMillis > nowMillis else { return false }
                guard Self.isValidPrefixPolicy(
                    name: c.name,
                    path: c.path,
                    secure: c.secure,
                    hostOnly: c.hostOnly,
                    isHttps: isHttps
                ) else {
                    return false
                }
                let domainMatches = c.hostOnly
                    ? (host == c.domain)
                    : (host == c.domain || host.hasSuffix(".\(c.domain)"))
                guard domainMatches else { return false }
                guard Self.pathMatch(requestPath: reqPath, cookiePath: c.path) else { return false }
                guard !c.secure || isHttps else { return false }
                return true
            }
        }
    }

    public func cookieHeader(for url: URL, nowMillis: Int64 = PersistentCookieJar.currentMillis()) -> String? {
        let matches = loadForRequest(url: url, nowMillis: nowMillis)
        guard !matches.isEmpty else { return nil }
        return matches.map { "\($0.name)=\($0.value)" }.joined(separator: "; ")
    }

    public func hasCookies(nowMillis: Int64 = PersistentCookieJar.currentMillis()) -> Bool {
        lock.withLock {
            ensureLoadedLocked().contains { $0.expiresAtMillis > nowMillis }
        }
    }

    public func clear() {
        lock.withLock {
            cache = []
            store.remove(key)
        }
    }

    /// Detects whether any `Set-Cookie` header for `cookieName` includes an explicit `Domain=` attribute
    /// (even without a leading dot, where Foundation `HTTPCookie` normalizes `domain` identically to host-only).
    private static func hasExplicitDomainAttribute(cookieName: String, headerFields: [String: String]) -> Bool {
        let targetName = cookieName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !targetName.isEmpty else { return false }
        let setCookieHeaders = headerFields.compactMap { (k, v) -> String? in
            k.lowercased() == "set-cookie" ? v : nil
        }
        guard !setCookieHeaders.isEmpty else { return false }
        let weekdayPattern = #"(?i)\b(Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s*"#
        for header in setCookieHeaders {
            let normalized = header.replacingOccurrences(of: weekdayPattern, with: "$1 ", options: .regularExpression)
            for cookieSegment in normalized.split(separator: ",", omittingEmptySubsequences: true) {
                let parts = cookieSegment.split(separator: ";", omittingEmptySubsequences: true)
                guard let nameValue = parts.first,
                      let eq = nameValue.firstIndex(of: "=") else {
                    continue
                }
                let parsedName = nameValue[..<eq].trimmingCharacters(in: .whitespacesAndNewlines)
                guard parsedName == targetName else { continue }
                for attr in parts.dropFirst() {
                    if attr.trimmingCharacters(in: .whitespacesAndNewlines).lowercased().hasPrefix("domain=") {
                        return true
                    }
                }
            }
        }
        return false
    }

    /// RFC 6265bis cookie prefix policy for `__Host-` and `__Secure-`.
    public static func isValidPrefixPolicy(
        name: String,
        path: String,
        secure: Bool,
        hostOnly: Bool,
        isHttps: Bool
    ) -> Bool {
        if name.hasPrefix("__Host-") {
            return isHttps && secure && hostOnly && path == "/"
        }
        if name.hasPrefix("__Secure-") {
            return isHttps && secure
        }
        return true
    }

    /// RFC 6265 path-match: `/api` must not match `/apix`.
    public static func pathMatch(requestPath: String, cookiePath: String) -> Bool {
        if requestPath == cookiePath { return true }
        guard requestPath.hasPrefix(cookiePath) else { return false }
        if cookiePath.hasSuffix("/") { return true }
        let idx = requestPath.index(requestPath.startIndex, offsetBy: cookiePath.count)
        return idx < requestPath.endIndex && requestPath[idx] == "/"
    }

    public static func currentMillis() -> Int64 {
        Int64(Date().timeIntervalSince1970 * 1000.0)
    }
}
