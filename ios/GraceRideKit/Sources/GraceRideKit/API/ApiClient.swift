import Foundation

/// Server error: HTTP status + stable error code + localized server message.
public struct ApiException: Error, Equatable, LocalizedError, Sendable {
    public let status: Int
    public let code: String?
    public let message: String

    public init(status: Int, code: String? = nil, message: String) {
        self.status = status
        self.code = code
        self.message = message
    }

    public var errorDescription: String? { message }
}

public enum AuthEvent: Equatable, Sendable {
    case unauthorized
    case locked
    case pending
}

/// Broadcasts global session transitions (401 → sign-in, 423 → unlock, 403 ACCOUNT_PENDING → approval screen).
public final class AuthEventBus: @unchecked Sendable {
    private let lock = NSLock()
    private var continuations: [UUID: AsyncStream<AuthEvent>.Continuation] = [:]

    public init() {}

    public func emit(_ event: AuthEvent) {
        let targets: [AsyncStream<AuthEvent>.Continuation] = lock.withLock {
            Array(continuations.values)
        }
        for cont in targets {
            cont.yield(event)
        }
    }

    public func events() -> AsyncStream<AuthEvent> {
        let id = UUID()
        return AsyncStream { continuation in
            lock.withLock {
                continuations[id] = continuation
            }
            continuation.onTermination = { [weak self] _ in
                guard let self else { return }
                self.lock.withLock {
                    _ = self.continuations.removeValue(forKey: id)
                }
            }
        }
    }
}

private struct ServerErrorPayload: Decodable {
    let code: String?
    let error: String?
}

public final class CookieStrippingRedirectDelegate: NSObject, URLSessionTaskDelegate, Sendable {
    public override init() {}

    public func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest,
        completionHandler: @escaping (URLRequest?) -> Void
    ) {
        let origHost = task.originalRequest?.url?.host?.lowercased()
        let origScheme = task.originalRequest?.url?.scheme?.lowercased()
        let newHost = request.url?.host?.lowercased()
        let newScheme = request.url?.scheme?.lowercased()
        let downgraded = origScheme == "https" && newScheme != "https"
        if origHost != newHost || downgraded {
            var sanitized = request
            sanitized.setValue(nil, forHTTPHeaderField: "Cookie")
            sanitized.setValue(nil, forHTTPHeaderField: "Authorization")
            sanitized.setValue(nil, forHTTPHeaderField: "X-Passive-Poll")
            completionHandler(sanitized)
        } else {
            completionHandler(request)
        }
    }
}

/// Native URLSession HTTP client reusing the Grace Ride REST API.
/// Never sends an `Origin` header so the backend's `origin_guard` admits native requests.
public final class HTTPClient: @unchecked Sendable {
    public let baseURL: URL
    public let cookieJar: PersistentCookieJar
    public let authEvents: AuthEventBus
    public let session: URLSession
    private let delegate = CookieStrippingRedirectDelegate()
    private let langProvider: @Sendable () -> String

    public init(
        baseURL: URL,
        cookieJar: PersistentCookieJar,
        authEvents: AuthEventBus,
        langProvider: @escaping @Sendable () -> String,
        configuration: URLSessionConfiguration? = nil
    ) {
        self.baseURL = baseURL
        self.cookieJar = cookieJar
        self.authEvents = authEvents
        self.langProvider = langProvider

        let cfg = configuration ?? {
            let c = URLSessionConfiguration.ephemeral
            c.httpShouldSetCookies = false
            c.httpCookieAcceptPolicy = .never
            c.timeoutIntervalForRequest = 30
            c.timeoutIntervalForResource = 60
            return c
        }()
        self.session = URLSession(configuration: cfg, delegate: delegate, delegateQueue: nil)
    }

    public func resolveURL(path: String, query: [URLQueryItem] = []) throws -> URL {
        let cleanPath = path.hasPrefix("/") ? String(path.dropFirst()) : path
        guard let rel = URL(string: cleanPath, relativeTo: baseURL)?.absoluteURL,
              var comps = URLComponents(url: rel, resolvingAgainstBaseURL: false) else {
            throw ApiException(status: 0, code: nil, message: "Invalid URL path: \(path)")
        }
        if !query.isEmpty {
            comps.queryItems = query
        }
        guard let finalURL = comps.url else {
            throw ApiException(status: 0, code: nil, message: "Invalid URL components")
        }
        return finalURL
    }

    public func get<T: Decodable>(_ path: String, query: [URLQueryItem] = [], headers: [String: String] = [:]) async throws -> T {
        try await request(method: "GET", path: path, query: query, bodyData: nil, contentType: nil, headers: headers)
    }

    public func post<T: Decodable>(_ path: String, body: (any Encodable)? = nil) async throws -> T {
        let data = try encodeOptional(body)
        return try await request(
            method: "POST",
            path: path,
            query: [],
            bodyData: data,
            contentType: data == nil ? nil : "application/json"
        )
    }

    public func put<T: Decodable>(_ path: String, body: (any Encodable)? = nil) async throws -> T {
        let data = try encodeOptional(body)
        return try await request(
            method: "PUT",
            path: path,
            query: [],
            bodyData: data,
            contentType: data == nil ? nil : "application/json"
        )
    }

    public func delete<T: Decodable>(_ path: String, body: (any Encodable)? = nil) async throws -> T {
        let data = try encodeOptional(body)
        return try await request(
            method: "DELETE",
            path: path,
            query: [],
            bodyData: data,
            contentType: data == nil ? nil : "application/json"
        )
    }

    public func uploadMultipart<T: Decodable>(
        _ path: String,
        fieldName: String = "file",
        filename: String,
        mime: String,
        fileData: Data
    ) async throws -> T {
        let boundary = "Boundary-\(UUID().uuidString)"
        let safeName = filename
            .replacingOccurrences(of: "\"", with: "")
            .replacingOccurrences(of: "\r", with: "")
            .replacingOccurrences(of: "\n", with: "")
        var body = Data()
        body.append("--\(boundary)\r\n".data(using: .utf8)!)
        body.append("Content-Disposition: form-data; name=\"\(fieldName)\"; filename=\"\(safeName)\"\r\n".data(using: .utf8)!)
        body.append("Content-Type: \(mime)\r\n\r\n".data(using: .utf8)!)
        body.append(fileData)
        body.append("\r\n--\(boundary)--\r\n".data(using: .utf8)!)
        return try await request(
            method: "POST",
            path: path,
            query: [],
            bodyData: body,
            contentType: "multipart/form-data; boundary=\(boundary)"
        )
    }

    public func buildRequest(
        method: String,
        url: URL,
        bodyData: Data? = nil,
        contentType: String? = nil,
        accept: String = "application/json",
        headers: [String: String] = [:]
    ) -> URLRequest {
        var req = URLRequest(url: url)
        req.httpMethod = method
        req.httpShouldHandleCookies = false
        req.setValue(accept, forHTTPHeaderField: "Accept")
        req.setValue(langProvider(), forHTTPHeaderField: "Accept-Language")
        if let cookie = cookieJar.cookieHeader(for: url) {
            req.setValue(cookie, forHTTPHeaderField: "Cookie")
        }
        if let contentType {
            req.setValue(contentType, forHTTPHeaderField: "Content-Type")
        }
        for (k, v) in headers {
            req.setValue(v, forHTTPHeaderField: k)
        }
        req.httpBody = bodyData
        return req
    }

    private func encodeOptional(_ value: (any Encodable)?) throws -> Data? {
        guard let value else { return nil }
        return try JSONEncoder().encode(AnyEncodable(value))
    }

    private func request<T: Decodable>(
        method: String,
        path: String,
        query: [URLQueryItem],
        bodyData: Data?,
        contentType: String?,
        headers: [String: String] = [:]
    ) async throws -> T {
        let url = try resolveURL(path: path, query: query)
        let req = buildRequest(method: method, url: url, bodyData: bodyData, contentType: contentType, headers: headers)

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.data(for: req)
        } catch let err as ApiException {
            throw err
        } catch {
            throw ApiException(status: 0, code: nil, message: error.localizedDescription)
        }

        guard let http = response as? HTTPURLResponse else {
            throw ApiException(status: 0, code: nil, message: "Invalid HTTP response")
        }

        if let headerFields = http.allHeaderFields as? [String: String] {
            cookieJar.saveFromResponse(url: http.url ?? url, headerFields: headerFields)
        } else {
            var normalized: [String: String] = [:]
            for (k, v) in http.allHeaderFields {
                normalized[String(describing: k)] = String(describing: v)
            }
            cookieJar.saveFromResponse(url: http.url ?? url, headerFields: normalized)
        }

        guard (200...299).contains(http.statusCode) else {
            let (code, message) = parseErrorBody(data: data, fallbackStatus: http.statusCode)
            if http.statusCode == 401 {
                authEvents.emit(.unauthorized)
            } else if http.statusCode == 423 || code == "SESSION_LOCKED" {
                authEvents.emit(.locked)
            } else if http.statusCode == 403 && code == "ACCOUNT_PENDING" {
                authEvents.emit(.pending)
            }
            throw ApiException(status: http.statusCode, code: code, message: message)
        }

        if T.self == OkResponse.self && data.isEmpty {
            return OkResponse(ok: true) as! T
        }

        do {
            return try JSONDecoder().decode(T.self, from: data)
        } catch {
            throw ApiException(status: -1, code: "DECODE_ERROR", message: error.localizedDescription)
        }
    }

    private func parseErrorBody(data: Data, fallbackStatus: Int) -> (String?, String) {
        if let parsed = try? JSONDecoder().decode(ServerErrorPayload.self, from: data) {
            let msg = parsed.error?.trimmingCharacters(in: .whitespacesAndNewlines)
            if let msg, !msg.isEmpty {
                return (parsed.code, msg)
            }
            return (parsed.code, HTTPURLResponse.localizedString(forStatusCode: fallbackStatus))
        }
        let text = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !text.isEmpty {
            return (nil, String(text.prefix(300)))
        }
        return (nil, HTTPURLResponse.localizedString(forStatusCode: fallbackStatus))
    }
}

private struct AnyEncodable: Encodable {
    private let encodeClosure: (Encoder) throws -> Void

    init(_ wrapped: any Encodable) {
        self.encodeClosure = { encoder in
            try wrapped.encode(to: encoder)
        }
    }

    func encode(to encoder: Encoder) throws {
        try encodeClosure(encoder)
    }
}
