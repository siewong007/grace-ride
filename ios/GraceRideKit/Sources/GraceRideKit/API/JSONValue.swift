import Foundation

/// Recursive JSON representation used for opaque WebAuthn options and credentials.
public enum JSONValue: Equatable, Hashable, Sendable {
    case string(String)
    case number(Double)
    case int(Int64)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    public var stringValue: String? {
        if case .string(let s) = self { return s }
        return nil
    }

    public var intValue: Int64? {
        switch self {
        case .int(let i): return i
        case .number(let n): return Int64(n)
        default: return nil
        }
    }

    public var boolValue: Bool? {
        if case .bool(let b) = self { return b }
        return nil
    }

    public var objectValue: [String: JSONValue]? {
        if case .object(let o) = self { return o }
        return nil
    }

    public var arrayValue: [JSONValue]? {
        if case .array(let a) = self { return a }
        return nil
    }

    public subscript(key: String) -> JSONValue? {
        objectValue?[key]
    }

    public func encodedString() throws -> String {
        let data = try JSONEncoder().encode(self)
        return String(decoding: data, as: UTF8.self)
    }

    public static func parse(_ string: String) throws -> JSONValue {
        guard let data = string.data(using: .utf8) else {
            throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "Invalid UTF-8"))
        }
        return try JSONDecoder().decode(JSONValue.self, from: data)
    }
}

extension JSONValue: Codable {
    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let b = try? container.decode(Bool.self) {
            self = .bool(b)
        } else if let i = try? container.decode(Int64.self) {
            self = .int(i)
        } else if let d = try? container.decode(Double.self) {
            self = .number(d)
        } else if let s = try? container.decode(String.self) {
            self = .string(s)
        } else if let a = try? container.decode([JSONValue].self) {
            self = .array(a)
        } else if let o = try? container.decode([String: JSONValue].self) {
            self = .object(o)
        } else {
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unsupported JSON value")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .string(let s): try container.encode(s)
        case .number(let n): try container.encode(n)
        case .int(let i): try container.encode(i)
        case .bool(let b): try container.encode(b)
        case .object(let o): try container.encode(o)
        case .array(let a): try container.encode(a)
        case .null: try container.encodeNil()
        }
    }
}

/// Decodes booleans sent as JSON booleans, integer flags (0/1), or null (matching Android `FlexibleBooleanSerializer`).
public struct FlexibleBool: Codable, Equatable, Hashable, Sendable {
    public let value: Bool?

    public init(_ value: Bool?) {
        self.value = value
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self.value = nil
        } else if let b = try? container.decode(Bool.self) {
            self.value = b
        } else if let i = try? container.decode(Int64.self) {
            self.value = i != 0
        } else if let s = try? container.decode(String.self) {
            let clean = s.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if clean == "true" || clean == "1" {
                self.value = true
            } else if clean == "false" || clean == "0" {
                self.value = false
            } else {
                self.value = nil
            }
        } else {
            self.value = nil
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        if let value {
            try container.encode(value)
        } else {
            try container.encodeNil()
        }
    }
}

extension KeyedDecodingContainer {
    func decodeOrDefault<T: Decodable>(_ type: T.Type, forKey key: Key, default defaultValue: T) throws -> T {
        if let decoded = try decodeIfPresent(type, forKey: key) {
            return decoded
        }
        return defaultValue
    }

    func decodeFlexibleBoolIfPresent(forKey key: Key) throws -> Bool? {
        if let flex = try decodeIfPresent(FlexibleBool.self, forKey: key) {
            return flex.value
        }
        return nil
    }
}
