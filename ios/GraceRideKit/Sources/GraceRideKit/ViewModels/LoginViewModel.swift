import Foundation
import Observation

@MainActor
@Observable
public final class LoginViewModel {
    private let container: AppContainer

    public var email: String = ""
    public var signingIn: Bool = false
    public var error: String?
    public var errorKey: String?
    public var noPasskeyHint: Bool = false
    public var devUsers: [DevUser] = []
    public var serverUrl: String
    public let devAvailable: Bool
    public var turnstileEnabled: Bool = false
    public var turnstileSiteKey: String?
    public var turnstilePrompted: Bool = false

    public init(container: AppContainer, devAvailable: Bool? = nil) {
        self.container = container
        self.serverUrl = container.baseURLString
        self.devAvailable = devAvailable ?? container.debugMode
        if let savedEmail = container.secureStore.getString("last_email"), !savedEmail.isEmpty {
            self.email = savedEmail
        }
        if self.devAvailable {
            Task { await loadDevUsers() }
        }
    }

    public func loadTurnstileConfig() async {
        do {
            let cfg = try await container.auth.turnstileConfig()
            let validKey = cfg.siteKey.flatMap { key -> String? in
                key.range(of: #"^[A-Za-z0-9_-]+$"#, options: .regularExpression) != nil ? key : nil
            }
            turnstileEnabled = cfg.enabled && validKey != nil
            turnstileSiteKey = validKey
            turnstilePrompted = false
        } catch {
            turnstileEnabled = false
            turnstileSiteKey = nil
            turnstilePrompted = false
        }
    }

    public func setEmail(_ value: String) {
        email = value
        error = nil
        errorKey = nil
    }

    public func loadDevUsers() async {
        do {
            devUsers = try await container.auth.devUsers()
            error = nil
            errorKey = nil
        } catch {
            devUsers = []
        }
    }

    public func applyServer(_ url: String) async {
        do {
            try container.setBaseUrl(url)
            serverUrl = container.baseURLString
            error = nil
            errorKey = nil
            await loadTurnstileConfig()
            await loadDevUsers()
        } catch {
            self.error = nil
            self.errorKey = "login_invalid_server"
        }
    }

    public func clearError() {
        error = nil
        errorKey = nil
        noPasskeyHint = false
    }

    public func dismissTurnstile() {
        turnstilePrompted = false
    }

    public func validateAndStartPasskey(
        turnstileToken: String? = nil,
        performPasskeyAssertion: (_ start: PasskeyStart) async throws -> JSONValue
    ) async {
        if signingIn { return }
        let cleanEmail = email.trimmingCharacters(in: .whitespacesAndNewlines)
        if cleanEmail.isEmpty || !cleanEmail.contains("@") {
            error = nil
            errorKey = "login_email_required"
            noPasskeyHint = false
            return
        }
        if turnstileEnabled && (turnstileToken?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true) {
            error = nil
            errorKey = nil
            noPasskeyHint = false
            turnstilePrompted = true
            return
        }
        signingIn = true
        error = nil
        errorKey = nil
        noPasskeyHint = false
        turnstilePrompted = false

        defer { signingIn = false }
        do {
            let start = try await container.auth.passkeyLoginStart(email: cleanEmail, turnstileToken: turnstileToken)
            let credential = try await performPasskeyAssertion(start)
            try await container.auth.passkeyLoginFinish(challengeId: start.challengeId, credential: credential)
            container.secureStore.putString("last_email", cleanEmail)
            container.realtimeStream.onAuthChanged()
        } catch let e as PasskeyFlowError {
            switch e {
            case .noCredential:
                noPasskeyHint = true
            case .cancelled:
                break
            case .failed(let msg):
                error = msg
            }
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }

    public func devLogin(email: String) async {
        if signingIn { return }
        signingIn = true
        error = nil
        errorKey = nil
        defer { signingIn = false }
        do {
            try await container.auth.devLogin(email: email)
            container.secureStore.putString("last_email", email.trimmingCharacters(in: .whitespacesAndNewlines))
            container.realtimeStream.onAuthChanged()
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }
}

public enum PasskeyFlowError: Error, Equatable, Sendable {
    case cancelled
    case noCredential
    case failed(String)
}
