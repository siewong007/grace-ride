import Foundation

/// Passenger readiness values accepted by `POST /api/rides/{id}/readiness`.
public enum Readiness {
    public static let ready = "ready"
    public static let delayed5m = "delayed_5m"
    public static let delayed10m = "delayed_10m"
    public static let unable = "unable_to_join"

    public static let values = [ready, delayed5m, delayed10m, unable]

    public static func shouldShowPrompt(ride: Ride) -> Bool {
        ride.viewer.isPassenger && ride.status == "confirmed" && !values.contains(ride.readinessStatus)
    }
}

/// Base64URL and WebAuthn `publicKey` helper for bridging `webauthn-rs` JSON payloads
/// with Apple `AuthenticationServices` (`ASAuthorizationPlatformPublicKeyCredentialProvider`).
public enum PasskeyOptions {
    public static func publicKey(_ options: [String: JSONValue]) -> [String: JSONValue] {
        options["publicKey"]?.objectValue ?? options
    }

    public static func decodeBase64URL(_ raw: String) -> Data? {
        var b64 = raw
            .replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        let rem = b64.count % 4
        if rem > 0 {
            b64 += String(repeating: "=", count: 4 - rem)
        }
        return Data(base64Encoded: b64)
    }

    public static func encodeBase64URL(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    public static func buildAssertionCredentialJSON(
        credentialID: Data,
        rawAuthenticatorData: Data,
        rawClientDataJSON: Data,
        signature: Data,
        userID: Data?
    ) -> JSONValue {
        let idB64 = encodeBase64URL(credentialID)
        var response: [String: JSONValue] = [
            "authenticatorData": .string(encodeBase64URL(rawAuthenticatorData)),
            "clientDataJSON": .string(encodeBase64URL(rawClientDataJSON)),
            "signature": .string(encodeBase64URL(signature)),
        ]
        if let userID, !userID.isEmpty {
            response["userHandle"] = .string(encodeBase64URL(userID))
        }
        return .object([
            "id": .string(idB64),
            "rawId": .string(idB64),
            "type": .string("public-key"),
            "response": .object(response),
            "extensions": .object([:]),
        ])
    }

    public static func buildRegistrationCredentialJSON(
        credentialID: Data,
        rawAttestationObject: Data?,
        rawClientDataJSON: Data
    ) -> JSONValue {
        let idB64 = encodeBase64URL(credentialID)
        var response: [String: JSONValue] = [
            "clientDataJSON": .string(encodeBase64URL(rawClientDataJSON)),
        ]
        if let rawAttestationObject {
            response["attestationObject"] = .string(encodeBase64URL(rawAttestationObject))
        }
        return .object([
            "id": .string(idB64),
            "rawId": .string(idB64),
            "type": .string("public-key"),
            "response": .object(response),
            "extensions": .object([:]),
        ])
    }
}
