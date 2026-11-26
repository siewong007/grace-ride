import Foundation
import Security

/// Key-value credential/preference store backed by iOS Keychain in production
/// and an in-memory dictionary in unit tests.
public protocol SecureStore: AnyObject, Sendable {
    func getString(_ key: String) -> String?
    func putString(_ key: String, _ value: String)
    func remove(_ key: String)
    func clearAll()
}

/// Thread-safe in-memory `SecureStore` for deterministic unit tests.
public final class InMemorySecureStore: SecureStore, @unchecked Sendable {
    private let lock = NSLock()
    private var values: [String: String] = [:]

    public init(initial: [String: String] = [:]) {
        self.values = initial
    }

    public func getString(_ key: String) -> String? {
        lock.withLock { values[key] }
    }

    public func putString(_ key: String, _ value: String) {
        lock.withLock { values[key] = value }
    }

    public func remove(_ key: String) {
        lock.withLock { _ = values.removeValue(forKey: key) }
    }

    public func clearAll() {
        lock.withLock { values.removeAll() }
    }
}

/// Hardware/OS-protected Keychain store with per-key accessibility levels
/// (`WhenUnlockedThisDeviceOnly` for foreground PII, `AfterFirstUnlockThisDeviceOnly` for session tokens)
/// and shared App Group accessGroup support with automatic fallback on unsigned builds.
public final class KeychainSecureStore: SecureStore, @unchecked Sendable {
    private let service: String
    private let accessGroup: String?
    private let lock = NSLock()

    private static let whenUnlockedKeys: Set<String> = [
        "cached_me_v1",
        "cached_me_at_ms",
        "cached_me_uptime_s",
        "today_rides_snapshot_v1",
        "offline_cache_aes256_key_v1",
    ]

    /// Keys that must remain confined to the main app Keychain (never shared with extensions).
    private static let appOnlyKeys: Set<String> = [
        "offline_cache_aes256_key_v1",
    ]

    public static let sharedAccessGroupSuffix = "org.pjstatecm.graceride.shared"

    public static let sharedAccessGroup: String? = {
        resolveSharedAccessGroup(suffix: sharedAccessGroupSuffix)
    }()

    private static func resolveSharedAccessGroup(suffix: String) -> String? {
        #if os(macOS)
        return nil
        #else
        var q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "org.pjstatecm.graceride.probe",
            kSecAttrAccount as String: "team_prefix_probe",
            kSecReturnAttributes as String: kCFBooleanTrue as Any,
        ]
        var result: AnyObject?
        var status = SecItemCopyMatching(q as CFDictionary, &result)
        if status == errSecItemNotFound {
            var addQ = q
            addQ.removeValue(forKey: kSecReturnAttributes as String)
            addQ[kSecValueData as String] = Data("1".utf8)
            addQ[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            let addStatus = SecItemAdd(addQ as CFDictionary, &result)
            if addStatus == errSecSuccess || addStatus == errSecDuplicateItem {
                status = SecItemCopyMatching(q as CFDictionary, &result)
            }
        }
        if status == errSecSuccess,
           let dict = result as? [String: Any],
           let grp = dict[kSecAttrAccessGroup as String] as? String,
           let dot = grp.firstIndex(of: ".") {
            let prefix = String(grp[...dot])
            return prefix + suffix
        }
        return nil
        #endif
    }

    public init(service: String = "org.pjstatecm.graceride.secure", accessGroup: String? = KeychainSecureStore.sharedAccessGroup) {
        self.service = service
        self.accessGroup = accessGroup
    }

    private func accessibleAttribute(for key: String) -> CFString {
        Self.whenUnlockedKeys.contains(key)
            ? kSecAttrAccessibleWhenUnlockedThisDeviceOnly
            : kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
    }

    private func baseQuery(for key: String? = nil, includeAccessGroup: Bool) -> [String: Any] {
        var q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
        ]
        if includeAccessGroup, let accessGroup, !accessGroup.isEmpty {
            q[kSecAttrAccessGroup as String] = accessGroup
        }
        if let key {
            q[kSecAttrAccount as String] = key
        }
        return q
    }

    public func getString(_ key: String) -> String? {
        let useSharedGroup = (accessGroup?.isEmpty == false) && !Self.appOnlyKeys.contains(key)
        return lock.withLock {
            func lookup(useGroup: Bool) -> (OSStatus, AnyObject?) {
                var q = baseQuery(for: key, includeAccessGroup: useGroup)
                q[kSecReturnData as String] = kCFBooleanTrue
                q[kSecMatchLimit as String] = kSecMatchLimitOne
                var item: CFTypeRef?
                return (SecItemCopyMatching(q as CFDictionary, &item), item)
            }
            var (status, item) = lookup(useGroup: useSharedGroup)
            if status == errSecMissingEntitlement, useSharedGroup {
                (status, item) = lookup(useGroup: false)
            }
            guard status == errSecSuccess, let data = item as? Data else {
                return nil
            }
            return String(data: data, encoding: .utf8)
        }
    }

    public func putString(_ key: String, _ value: String) {
        guard let data = value.data(using: .utf8) else { return }
        let accessible = accessibleAttribute(for: key)
        let useSharedGroup = (accessGroup?.isEmpty == false) && !Self.appOnlyKeys.contains(key)
        lock.withLock {
            func write(useGroup: Bool) -> OSStatus {
                let query = baseQuery(for: key, includeAccessGroup: useGroup)
                let attrs: [String: Any] = [
                    kSecValueData as String: data,
                    kSecAttrAccessible as String: accessible,
                ]
                var status = SecItemUpdate(query as CFDictionary, attrs as CFDictionary)
                if status == errSecItemNotFound {
                    var addQuery = query
                    addQuery[kSecValueData as String] = data
                    addQuery[kSecAttrAccessible as String] = accessible
                    status = SecItemAdd(addQuery as CFDictionary, nil)
                } else if status != errSecSuccess && status != errSecMissingEntitlement {
                    SecItemDelete(query as CFDictionary)
                    var addQuery = query
                    addQuery[kSecValueData as String] = data
                    addQuery[kSecAttrAccessible as String] = accessible
                    status = SecItemAdd(addQuery as CFDictionary, nil)
                }
                return status
            }
            let status = write(useGroup: useSharedGroup)
            if status == errSecMissingEntitlement, useSharedGroup {
                _ = write(useGroup: false)
            }
        }
    }

    public func remove(_ key: String) {
        let useSharedGroup = (accessGroup?.isEmpty == false) && !Self.appOnlyKeys.contains(key)
        lock.withLock {
            if useSharedGroup {
                let status = SecItemDelete(baseQuery(for: key, includeAccessGroup: true) as CFDictionary)
                if status == errSecMissingEntitlement {
                    SecItemDelete(baseQuery(for: key, includeAccessGroup: false) as CFDictionary)
                }
            } else {
                SecItemDelete(baseQuery(for: key, includeAccessGroup: false) as CFDictionary)
            }
        }
    }

    public func clearAll() {
        lock.withLock {
            if let accessGroup, !accessGroup.isEmpty {
                SecItemDelete(baseQuery(for: nil, includeAccessGroup: true) as CFDictionary)
            }
            SecItemDelete(baseQuery(for: nil, includeAccessGroup: false) as CFDictionary)
        }
    }
}
