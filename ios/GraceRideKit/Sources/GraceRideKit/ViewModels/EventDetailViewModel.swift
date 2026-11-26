import Foundation
import Observation

@MainActor
@Observable
public final class EventDetailViewModel {
    private let container: AppContainer
    public let eventId: Int64
    nonisolated(unsafe) private var realtimeTask: Task<Void, Never>?
    nonisolated(unsafe) private var networkTask: Task<Void, Never>?

    public private(set) var loading: Bool = true
    public private(set) var refreshing: Bool = false
    public private(set) var event: EventItem?
    public private(set) var outboundRide: Ride?
    public private(set) var returnRide: Ride?
    public private(set) var drivingForEvent: [Ride] = []
    public private(set) var error: String?
    public private(set) var busy: Bool = false
    public private(set) var message: String?
    public private(set) var checkinError: String?
    public private(set) var actionNotice: EventActionNotice?
    public private(set) var checkinResult: CheckinResp?
    public private(set) var scanMemberResult: ScanMemberResp?

    public init(container: AppContainer, eventId: Int64, startObservers: Bool = true) {
        self.container = container
        self.eventId = eventId
        if startObservers {
            realtimeTask = Task { [weak self] in
                let stream = container.realtimeStream.events()
                for await ev in stream where ev != .resume {
                    await self?.refresh()
                }
            }
            networkTask = Task { [weak self] in
                let stream = container.networkMonitor.onlineStream()
                for await online in stream where online {
                    await self?.refresh()
                }
            }
        }
    }

    deinit {
        realtimeTask?.cancel()
        networkTask?.cancel()
    }

    public func load() async {
        loading = event == nil
        await reloadDetail()
    }

    public func refresh() async {
        refreshing = event != nil
        await reloadDetail()
    }

    private func reloadDetail() async {
        do {
            let loadedEvent = try await container.events.detail(id: eventId)
            var out: Ride?
            var ret: Ride?
            var driving: [Ride] = []

            if loadedEvent.mySignup?.status == "going" {
                let canRides = container.auth.me?.can("rides") == true
                let canDrive = container.auth.me?.can("drive") == true
                let day = KlTime.date(loadedEvent.startsAt)

                if canRides && (loadedEvent.myRide != nil || loadedEvent.myReturnRide != nil) {
                    let upcoming = (try? await container.rides.upcoming()) ?? []
                    out = loadedEvent.myRide.flatMap { ref in upcoming.first { $0.id == ref.id } }
                    ret = loadedEvent.myReturnRide.flatMap { ref in upcoming.first { $0.id == ref.id } }
                    if (loadedEvent.myRide != nil && out == nil) || (loadedEvent.myReturnRide != nil && ret == nil) {
                        let past = (try? await container.rides.past()) ?? []
                        if out == nil {
                            out = loadedEvent.myRide.flatMap { ref in past.first { $0.id == ref.id } }
                        }
                        if ret == nil {
                            ret = loadedEvent.myReturnRide.flatMap { ref in past.first { $0.id == ref.id } }
                        }
                    }
                }
                if canDrive && day.count == 10 {
                    let sched = (try? await container.rides.drivingToday(from: day, to: day)) ?? []
                    driving = sched.filter { $0.event?.id == loadedEvent.id }
                }
            }

            loading = false
            refreshing = false
            event = loadedEvent
            outboundRide = out
            returnRide = ret
            drivingForEvent = driving
            error = nil
        } catch let e as ApiException {
            loading = false
            refreshing = false
            error = e.message
        } catch {
            loading = false
            refreshing = false
            self.error = error.localizedDescription
        }
    }

    public func signup(needRide: Bool, offerDrive: Bool, asGroupId: Int64?, offerSeats: Int? = nil) async {
        guard !busy else { return }
        busy = true
        message = nil
        defer { busy = false }
        do {
            _ = try await container.events.signup(id: eventId, needRide: needRide, offerDrive: offerDrive, asGroupId: asGroupId, offerSeats: offerSeats)
            await reloadDetail()
        } catch let e as ApiException {
            message = e.message
        } catch {
            message = error.localizedDescription
        }
    }

    public func withdraw() async {
        guard !busy else { return }
        busy = true
        message = nil
        defer { busy = false }
        do {
            _ = try await container.events.withdraw(id: eventId)
            await reloadDetail()
        } catch let e as ApiException {
            message = e.message
        } catch {
            message = error.localizedDescription
        }
    }

    public func publish() async {
        guard !busy else { return }
        busy = true
        message = nil
        actionNotice = nil
        defer { busy = false }
        do {
            _ = try await container.events.publish(id: eventId)
            await reloadDetail()
            actionNotice = .published
        } catch let e as ApiException {
            message = e.message
        } catch {
            message = error.localizedDescription
        }
    }

    public func unpublish() async {
        guard !busy else { return }
        busy = true
        message = nil
        actionNotice = nil
        defer { busy = false }
        do {
            _ = try await container.events.unpublish(id: eventId)
            await reloadDetail()
            actionNotice = .unpublished
        } catch let e as ApiException {
            message = e.message
        } catch {
            message = error.localizedDescription
        }
    }

    public func selfCheckin(code: String?) async {
        guard !busy else { return }
        busy = true
        message = nil
        checkinError = nil
        checkinResult = nil
        defer { busy = false }
        do {
            let trimmed = code?.trimmingCharacters(in: .whitespacesAndNewlines)
            let res = try await container.events.selfCheckin(id: eventId, code: (trimmed?.isEmpty == false) ? trimmed : nil)
            await reloadDetail()
            checkinResult = res
        } catch let e as ApiException {
            checkinError = e.message
        } catch {
            checkinError = error.localizedDescription
        }
    }

    public func scanMemberPass(passCode: String) async {
        let clean = passCode.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !busy, !clean.isEmpty else { return }
        busy = true
        message = nil
        checkinError = nil
        scanMemberResult = nil
        defer { busy = false }
        do {
            let res = try await container.events.scanMemberPass(id: eventId, passCode: clean)
            await reloadDetail()
            scanMemberResult = res
        } catch let e as ApiException {
            checkinError = e.message
        } catch {
            checkinError = error.localizedDescription
        }
    }

    public func clearMessage() { message = nil }
    public func clearCheckinError() { checkinError = nil }
    public func clearNotice() { actionNotice = nil }
    public func clearCheckinResults() {
        checkinResult = nil
        scanMemberResult = nil
        checkinError = nil
    }
}
