import Foundation

/// Thread-safe swappable holder around `GraceRideAPIProtocol` so debug server URL overrides
/// take effect immediately across all repositories without recreating them.
public final class ApiProvider: @unchecked Sendable {
    private let lock = NSLock()
    private var current: any GraceRideAPIProtocol

    public init(_ initial: any GraceRideAPIProtocol) {
        self.current = initial
    }

    public func get() -> any GraceRideAPIProtocol {
        lock.withLock { current }
    }

    public func swap(_ next: any GraceRideAPIProtocol) {
        lock.withLock { current = next }
    }
}
