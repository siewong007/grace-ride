import Foundation
import Network
import Observation

@MainActor
@Observable
public final class NetworkMonitor {
    private final class PathMonitorBox: @unchecked Sendable {
        let monitor = NWPathMonitor()
        func cancel() { monitor.cancel() }
    }

    public private(set) var isOnline: Bool
    private let monitorBox: PathMonitorBox?
    private let queue = DispatchQueue(label: "org.pjstatecm.graceride.netmon")
    private var continuations: [UUID: AsyncStream<Bool>.Continuation] = [:]

    public init(startMonitoring: Bool = true, initialOnline: Bool = true) {
        self.isOnline = initialOnline
        let box: PathMonitorBox? = startMonitoring ? PathMonitorBox() : nil
        self.monitorBox = box
        if let box {
            box.monitor.pathUpdateHandler = { [weak self] path in
                let online = path.status == .satisfied
                Task { @MainActor [weak self] in
                    self?.updateOnline(online)
                }
            }
            box.monitor.start(queue: queue)
        }
    }

    deinit {
        monitorBox?.cancel()
    }

    public func updateOnline(_ online: Bool) {
        guard isOnline != online else { return }
        isOnline = online
        for c in continuations.values {
            c.yield(online)
        }
    }

    public func onlineStream() -> AsyncStream<Bool> {
        let id = UUID()
        return AsyncStream { continuation in
            continuations[id] = continuation
            continuation.onTermination = { [weak self] _ in
                Task { @MainActor [weak self] in
                    _ = self?.continuations.removeValue(forKey: id)
                }
            }
        }
    }
}
