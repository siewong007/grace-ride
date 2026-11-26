import AuthenticationServices
import GraceRideKit
import SwiftUI
import UIKit
import WebKit

public enum PasskeyWebAuthn {
    public static func decodeBase64URL(_ string: String) -> Data? {
        var s = string
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        let rem = s.count % 4
        if rem > 0 {
            s.append(String(repeating: "=", count: 4 - rem))
        }
        return Data(base64Encoded: s)
    }

    public static func encodeBase64URL(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}

@MainActor
public final class PasskeyAuthManager: NSObject, ASAuthorizationControllerDelegate, ASAuthorizationControllerPresentationContextProviding {
    private var continuation: CheckedContinuation<JSONValue, Error>?
    private var isRegistration = false

    public func signIn(start: PasskeyStart, fallbackRpId: String?) async throws -> JSONValue {
        let pk = start.options["publicKey"]?.objectValue ?? start.options
        guard let challengeB64 = pk["challenge"]?.stringValue,
              let challengeData = PasskeyWebAuthn.decodeBase64URL(challengeB64) else {
            throw PasskeyFlowError.failed("Invalid passkey challenge")
        }
        let rpId = pk["rpId"]?.stringValue ?? fallbackRpId ?? "graceride.saliminn.my"
        let provider = ASAuthorizationPlatformPublicKeyCredentialProvider(relyingPartyIdentifier: rpId)
        let request = provider.createCredentialAssertionRequest(challenge: challengeData)

        if let allowCreds = pk["allowCredentials"]?.arrayValue {
            let descriptors: [ASAuthorizationPlatformPublicKeyCredentialDescriptor] = allowCreds.compactMap { entry in
                guard let idStr = entry.objectValue?["id"]?.stringValue,
                      let idData = PasskeyWebAuthn.decodeBase64URL(idStr) else {
                    return nil
                }
                return ASAuthorizationPlatformPublicKeyCredentialDescriptor(credentialID: idData)
            }
            if !descriptors.isEmpty {
                request.allowedCredentials = descriptors
            }
        }
        if let uv = pk["userVerification"]?.stringValue {
            request.userVerificationPreference = Self.mapUserVerification(uv)
        }

        isRegistration = false
        return try await runAuthorization(request: request)
    }

    public func register(start: PasskeyStart, fallbackRpId: String?) async throws -> JSONValue {
        let pk = start.options["publicKey"]?.objectValue ?? start.options
        guard let challengeB64 = pk["challenge"]?.stringValue,
              let challengeData = PasskeyWebAuthn.decodeBase64URL(challengeB64),
              let userObj = pk["user"]?.objectValue,
              let userIdStr = userObj["id"]?.stringValue,
              let userIdData = PasskeyWebAuthn.decodeBase64URL(userIdStr),
              let userName = userObj["name"]?.stringValue else {
            throw PasskeyFlowError.failed("Invalid passkey registration options")
        }
        let rpId = pk["rp"]?.objectValue?["id"]?.stringValue
            ?? pk["rpId"]?.stringValue
            ?? fallbackRpId
            ?? "graceride.saliminn.my"

        let provider = ASAuthorizationPlatformPublicKeyCredentialProvider(relyingPartyIdentifier: rpId)
        let request = provider.createCredentialRegistrationRequest(
            challenge: challengeData,
            name: userName,
            userID: userIdData
        )
        if let uv = pk["authenticatorSelection"]?.objectValue?["userVerification"]?.stringValue {
            request.userVerificationPreference = Self.mapUserVerification(uv)
        }
        if let excludes = pk["excludeCredentials"]?.arrayValue {
            let descriptors: [ASAuthorizationPlatformPublicKeyCredentialDescriptor] = excludes.compactMap { entry in
                guard let idStr = entry.objectValue?["id"]?.stringValue,
                      let idData = PasskeyWebAuthn.decodeBase64URL(idStr) else {
                    return nil
                }
                return ASAuthorizationPlatformPublicKeyCredentialDescriptor(credentialID: idData)
            }
            if #available(iOS 17.4, *), !descriptors.isEmpty {
                request.excludedCredentials = descriptors
            }
        }

        isRegistration = true
        return try await runAuthorization(request: request)
    }

    private func runAuthorization(request: ASAuthorizationRequest) async throws -> JSONValue {
        try await withCheckedThrowingContinuation { cont in
            self.continuation = cont
            let controller = ASAuthorizationController(authorizationRequests: [request])
            controller.delegate = self
            controller.presentationContextProvider = self
            controller.performRequests()
        }
    }

    private static func mapUserVerification(_ value: String) -> ASAuthorizationPublicKeyCredentialUserVerificationPreference {
        switch value {
        case "required": return .required
        case "discouraged": return .discouraged
        default: return .preferred
        }
    }

    public func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        if let keyWindow = scenes.flatMap(\.windows).first(where: \.isKeyWindow) {
            return keyWindow
        }
        if let firstWindow = scenes.flatMap(\.windows).first {
            return firstWindow
        }
        return ASPresentationAnchor()
    }

    public func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithAuthorization authorization: ASAuthorization
    ) {
        guard let cont = continuation else { return }
        continuation = nil

        if let assertion = authorization.credential as? ASAuthorizationPlatformPublicKeyCredentialAssertion {
            let id = PasskeyWebAuthn.encodeBase64URL(assertion.credentialID)
            let clientData = PasskeyWebAuthn.encodeBase64URL(assertion.rawClientDataJSON)
            let authData = PasskeyWebAuthn.encodeBase64URL(assertion.rawAuthenticatorData)
            let sig = PasskeyWebAuthn.encodeBase64URL(assertion.signature)
            var responseObj: [String: JSONValue] = [
                "clientDataJSON": .string(clientData),
                "authenticatorData": .string(authData),
                "signature": .string(sig),
            ]
            if !assertion.userID.isEmpty {
                responseObj["userHandle"] = .string(PasskeyWebAuthn.encodeBase64URL(assertion.userID))
            }
            cont.resume(returning: .object([
                "id": .string(id),
                "rawId": .string(id),
                "type": .string("public-key"),
                "response": .object(responseObj),
            ]))
            return
        }

        if let registration = authorization.credential as? ASAuthorizationPlatformPublicKeyCredentialRegistration {
            let id = PasskeyWebAuthn.encodeBase64URL(registration.credentialID)
            let clientData = PasskeyWebAuthn.encodeBase64URL(registration.rawClientDataJSON)
            guard let attestationData = registration.rawAttestationObject else {
                cont.resume(throwing: PasskeyFlowError.failed("Missing attestation object"))
                return
            }
            let attestation = PasskeyWebAuthn.encodeBase64URL(attestationData)
            cont.resume(returning: .object([
                "id": .string(id),
                "rawId": .string(id),
                "type": .string("public-key"),
                "response": .object([
                    "clientDataJSON": .string(clientData),
                    "attestationObject": .string(attestation),
                ]),
            ]))
            return
        }

        cont.resume(throwing: PasskeyFlowError.failed("Unsupported credential type"))
    }

    public func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithError error: Error
    ) {
        guard let cont = continuation else { return }
        continuation = nil

        if let authError = error as? ASAuthorizationError {
            switch authError.code {
            case .canceled:
                cont.resume(throwing: PasskeyFlowError.cancelled)
            case .unknown, .notHandled:
                cont.resume(throwing: isRegistration ? PasskeyFlowError.failed(authError.localizedDescription) : PasskeyFlowError.noCredential)
            default:
                cont.resume(throwing: PasskeyFlowError.failed(authError.localizedDescription))
            }
            return
        }
        cont.resume(throwing: PasskeyFlowError.failed(error.localizedDescription))
    }
}

/// Modal Cloudflare Turnstile verification sheet loaded under the server's configured origin.
public struct TurnstileWebSheet: View {
    public let baseURL: URL
    public let siteKey: String
    public let lang: String
    public let onToken: (String) -> Void
    public let onCancel: () -> Void

    public init(
        baseURL: URL,
        siteKey: String,
        lang: String,
        onToken: @escaping (String) -> Void,
        onCancel: @escaping () -> Void
    ) {
        self.baseURL = baseURL
        self.siteKey = siteKey
        self.lang = lang
        self.onToken = onToken
        self.onCancel = onCancel
    }

    public var body: some View {
        NavigationStack {
            TurnstileWKView(baseURL: baseURL, siteKey: siteKey, onToken: onToken)
                .navigationTitle(L10n.tr("login_turnstile_title", lang: lang))
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button(L10n.tr("common_cancel", lang: lang), action: onCancel)
                    }
                }
        }
    }
}

private struct TurnstileWKView: UIViewRepresentable {
    let baseURL: URL
    let siteKey: String
    let onToken: (String) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(baseHost: baseURL.host?.lowercased(), onToken: onToken)
    }

    func makeUIView(context: Context) -> WKWebView {
        let contentController = WKUserContentController()
        contentController.add(context.coordinator, name: "grTurnstile")

        let config = WKWebViewConfiguration()
        config.userContentController = contentController
        config.websiteDataStore = .nonPersistent()

        let webView = WKWebView(frame: .zero, configuration: config)
        webView.navigationDelegate = context.coordinator
        webView.isOpaque = false
        webView.backgroundColor = .clear

        let escapedKey = siteKey
            .replacingOccurrences(of: "&", with: "&amp;")
            .replacingOccurrences(of: "\"", with: "&quot;")
        let html = """
        <!doctype html>
        <html>
        <head>
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <style>
            body { margin: 0; min-height: 100vh; display: flex; align-items: center; justify-content: center; background: transparent; font-family: -apple-system, sans-serif; }
          </style>
          <script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>
          <script>
            function onGrTurnstile(token) {
              if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.grTurnstile) {
                window.webkit.messageHandlers.grTurnstile.postMessage(token);
              }
            }
          </script>
        </head>
        <body>
          <div class="cf-turnstile" data-sitekey="\(escapedKey)" data-callback="onGrTurnstile"></div>
        </body>
        </html>
        """
        webView.loadHTMLString(html, baseURL: baseURL)
        return webView
    }

    func updateUIView(_ uiView: WKWebView, context: Context) {}

    final class Coordinator: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
        let baseHost: String?
        let onToken: (String) -> Void

        init(baseHost: String?, onToken: @escaping (String) -> Void) {
            self.baseHost = baseHost
            self.onToken = onToken
        }

        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            if let token = message.body as? String, !token.isEmpty {
                DispatchQueue.main.async {
                    self.onToken(token)
                }
            }
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction,
            decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
        ) {
            guard let url = navigationAction.request.url else {
                decisionHandler(.cancel)
                return
            }
            if url.scheme == "about" {
                decisionHandler(.allow)
                return
            }
            let host = url.host?.lowercased() ?? ""
            let allowed = host == baseHost
                || host == "challenges.cloudflare.com"
                || host.hasSuffix(".challenges.cloudflare.com")
            decisionHandler(allowed ? .allow : .cancel)
        }
    }
}
