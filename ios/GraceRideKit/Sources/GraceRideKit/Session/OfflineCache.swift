import CryptoKit
import Foundation

public struct CachedEntry<T: Codable>: Codable {
    public var savedAtMillis: Int64
    public var data: T

    public init(savedAtMillis: Int64, data: T) {
        self.savedAtMillis = savedAtMillis
        self.data = data
    }
}

/// Per-user AES-256-GCM encrypted offline cache backed by `Application Support/offline_cache`
/// (`NSFileProtectionComplete` on iOS, excluded from backups, with filename AAD binding).
public final class OfflineCache: @unchecked Sendable {
    private let cacheDir: URL?
    private let secureStore: SecureStore?
    private let lock = NSLock()
    private var memory: [String: Data] = [:]
    private var cachedKey: SymmetricKey?

    private static let keyStorageName = "offline_cache_aes256_key_v1"
    private static let magicHeader = Data("GRC1".utf8)
    private static let sharedTestKey = SymmetricKey(size: .bits256)

    public init(cacheDir: URL? = nil, secureStore: SecureStore? = nil) {
        self.cacheDir = cacheDir
        self.secureStore = secureStore
        if let cacheDir {
            Self.ensureDirectory(cacheDir)
        }
    }

    private static func ensureDirectory(_ dir: URL) {
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        var mutableDir = dir
        try? mutableDir.setResourceValues(values)
    }

    private func resolveSymmetricKeyLocked() -> SymmetricKey {
        if let cachedKey { return cachedKey }
        guard let secureStore else {
            cachedKey = Self.sharedTestKey
            return Self.sharedTestKey
        }
        if let rawB64 = secureStore.getString(Self.keyStorageName),
           let keyData = Data(base64Encoded: rawB64),
           keyData.count == 32 {
            let loaded = SymmetricKey(data: keyData)
            cachedKey = loaded
            return loaded
        }
        let fresh = SymmetricKey(size: .bits256)
        let b64 = fresh.withUnsafeBytes { Data($0).base64EncodedString() }
        secureStore.putString(Self.keyStorageName, b64)
        cachedKey = fresh
        return fresh
    }

    private func sealLocked(filename: String, payload: Data) -> Data? {
        let key = resolveSymmetricKeyLocked()
        guard let box = try? AES.GCM.seal(payload, using: key, authenticating: Data(filename.utf8)),
              let combined = box.combined else {
            return nil
        }
        var out = Self.magicHeader
        out.append(combined)
        return out
    }

    private func unsealLocked(filename: String, blob: Data) -> Data? {
        guard blob.count > Self.magicHeader.count,
              blob.prefix(Self.magicHeader.count) == Self.magicHeader else {
            return nil
        }
        let key = resolveSymmetricKeyLocked()
        let combined = blob.dropFirst(Self.magicHeader.count)
        guard let box = try? AES.GCM.SealedBox(combined: combined),
              let plain = try? AES.GCM.open(box, using: key, authenticating: Data(filename.utf8)) else {
            return nil
        }
        return plain
    }

    public static func defaultCacheDirectory() -> URL? {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)
            .first?
            .appendingPathComponent("offline_cache", isDirectory: true)
    }

    public func write<T: Codable>(
        userId: Int64,
        key: String,
        value: T,
        nowMillis: Int64 = PersistentCookieJar.currentMillis()
    ) {
        let filename = entryKey(userId: userId, key: key)
        guard let payload = try? JSONEncoder().encode(CachedEntry(savedAtMillis: nowMillis, data: value)) else {
            return
        }
        lock.withLock {
            guard let sealed = sealLocked(filename: filename, payload: payload) else { return }
            memory[filename] = sealed
            guard let dir = cacheDir else { return }
            Self.ensureDirectory(dir)
            let legacyJson = dir.appendingPathComponent("\(filename).json")
            if FileManager.default.fileExists(atPath: legacyJson.path) {
                try? FileManager.default.removeItem(at: legacyJson)
            }
            let dst = dir.appendingPathComponent("\(filename).bin")
            #if os(iOS)
            let writeOpts: Data.WritingOptions = [.atomic, .completeFileProtection]
            #else
            let writeOpts: Data.WritingOptions = [.atomic]
            #endif
            try? sealed.write(to: dst, options: writeOpts)
        }
    }

    public func read<T: Codable>(_ type: T.Type, userId: Int64, key: String) -> CachedEntry<T>? {
        let filename = entryKey(userId: userId, key: key)
        let plain: Data? = lock.withLock {
            let sealed: Data?
            if let mem = memory[filename] {
                sealed = mem
            } else if let dir = cacheDir {
                let legacyJson = dir.appendingPathComponent("\(filename).json")
                if FileManager.default.fileExists(atPath: legacyJson.path) {
                    try? FileManager.default.removeItem(at: legacyJson)
                }
                let dst = dir.appendingPathComponent("\(filename).bin")
                if let diskData = try? Data(contentsOf: dst) {
                    memory[filename] = diskData
                    sealed = diskData
                } else {
                    sealed = nil
                }
            } else {
                sealed = nil
            }
            guard let sealed else { return nil }
            if let unwrapped = unsealLocked(filename: filename, blob: sealed) {
                return unwrapped
            }
            memory.removeValue(forKey: filename)
            if let dir = cacheDir {
                try? FileManager.default.removeItem(at: dir.appendingPathComponent("\(filename).bin"))
            }
            return nil
        }
        guard let plain else { return nil }
        return try? JSONDecoder().decode(CachedEntry<T>.self, from: plain)
    }

    public func clearUser(_ userId: Int64) {
        let prefix = "u\(userId)_"
        lock.withLock {
            for k in memory.keys where k.hasPrefix(prefix) {
                memory.removeValue(forKey: k)
            }
            guard let dir = cacheDir,
                  let files = try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil) else {
                return
            }
            for f in files where f.lastPathComponent.hasPrefix(prefix) {
                try? FileManager.default.removeItem(at: f)
            }
        }
    }

    public func clearAll() {
        lock.withLock {
            memory.removeAll()
            guard let dir = cacheDir,
                  let files = try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil) else {
                return
            }
            for f in files {
                try? FileManager.default.removeItem(at: f)
            }
        }
    }

    private func entryKey(userId: Int64, key: String) -> String {
        let clean = key.unicodeScalars.map { scalar -> Character in
            if CharacterSet.alphanumerics.contains(scalar) || scalar == "_" || scalar == "-" {
                return Character(scalar)
            }
            return "_"
        }
        return "u\(userId)_\(String(clean))"
    }
}
