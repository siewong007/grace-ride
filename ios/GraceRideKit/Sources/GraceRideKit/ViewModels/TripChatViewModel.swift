import Foundation
import Observation

@MainActor
@Observable
public final class TripChatViewModel {
    private let container: AppContainer
    public let rideId: Int64
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    private var sheetVisible: Bool = true

    public private(set) var loading: Bool = true
    public private(set) var sending: Bool = false
    public private(set) var open: Bool = true
    public private(set) var canPost: Bool = true
    public private(set) var members: [String] = []
    public private(set) var reactionsAllowed: [String] = ["👍", "❤️", "😂", "😮", "😢", "🙏"]
    public private(set) var messages: [ChatMessageDto] = []
    public private(set) var error: String?

    public init(container: AppContainer, rideId: Int64, startObservers: Bool = true) {
        self.container = container
        self.rideId = rideId
        if startObservers {
            realtimeTask = Task { [weak self] in
                let stream = container.realtimeStream.events()
                for await ev in stream {
                    guard let self else { break }
                    if self.sheetVisible && (ev == .chat || ev == .resume) {
                        await self.load(silent: true)
                    }
                }
            }
        }
    }

    deinit {
        realtimeTask?.cancel()
    }

    public func setSheetVisible(_ visible: Bool) async {
        let becameVisible = !sheetVisible && visible
        sheetVisible = visible
        if becameVisible {
            await load(silent: !messages.isEmpty)
        }
    }

    public func load(silent: Bool = false) async {
        if !silent && messages.isEmpty {
            loading = true
            error = nil
        }
        do {
            let thread = try await container.rides.rideMessages(id: rideId)
            loading = false
            open = thread.open
            canPost = thread.canPost
            members = thread.members
            if !thread.reactionsAllowed.isEmpty {
                reactionsAllowed = thread.reactionsAllowed
            }
            messages = thread.messages
            error = nil
            if sheetVisible, let maxId = thread.messages.last?.id, maxId > 0 {
                _ = try? await container.rides.markChatRead(id: rideId, upTo: maxId)
            }
        } catch let e as ApiException {
            loading = false
            error = e.message
        } catch {
            loading = false
            self.error = error.localizedDescription
        }
    }

    public func send(_ body: String) async -> Bool {
        let clean = body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !clean.isEmpty, !sending else { return false }
        sending = true
        error = nil
        defer { sending = false }
        do {
            _ = try await container.rides.postMessage(id: rideId, body: String(clean.prefix(1000)))
            await load(silent: true)
            return true
        } catch let e as ApiException {
            error = e.message
            return false
        } catch {
            self.error = error.localizedDescription
            return false
        }
    }

    public func react(messageId: Int64, emoji: String) async {
        do {
            _ = try await container.rides.reactMessage(id: rideId, messageId: messageId, emoji: emoji)
            await load(silent: true)
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }
}
