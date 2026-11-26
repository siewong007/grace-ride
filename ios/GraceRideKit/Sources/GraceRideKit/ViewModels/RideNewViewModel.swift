import Foundation
import Observation

public enum TripType: Int, CaseIterable, Sendable {
    case outbound = 0
    case round = 1
    case returnOnly = 2
}

@MainActor
@Observable
public final class RideNewViewModel {
    private let container: AppContainer
    public let eventId: Int64?
    private var searchTask: Task<Void, Never>?
    private var lastAppliedPlaceNotes: String?

    public var loading: Bool
    public var event: EventItem?
    public var areas: [Area] = []
    public var savedPlaces: [SavedPlace] = []
    public var proxyRiders: [ProxyRider] = []
    public var tripType: TripType = .outbound
    public var isProxy: Bool
    public var riderName: String = ""
    public var riderPhone: String = ""
    public var riderRelationship: String = "parent"
    public var saveProxyRider: Bool = false
    public var date: Date
    public var hour: Int = 9
    public var minute: Int = 0
    public var areaId: Int64?
    public var address: String = ""
    public var destination: String = ""
    public var seats: Int = 1
    public var assistance: String = ""
    public var notes: String = ""
    public var saveAddress: Bool = false
    public var saveAddressLabel: String = ""
    public var returnDate: Date
    public var returnHour: Int = 12
    public var returnMinute: Int = 0
    public var suggestions: [Place] = []
    public var pickedPlace: Place?
    public var submitting: Bool = false
    public var loadError: String?
    public var error: String?
    public var errorKey: String?
    public var createdId: Int64?

    public init(container: AppContainer, eventId: Int64?, initialProxy: Bool = false) {
        self.container = container
        self.eventId = eventId
        self.loading = (eventId ?? 0) != 0
        self.isProxy = initialProxy
        let tomorrow = KlTime.klCalendar.date(byAdding: .day, value: 1, to: Date()) ?? Date()
        self.date = tomorrow
        self.returnDate = tomorrow
    }

    public func retryLoad() async {
        loading = eventId != nil
        loadError = nil
        do {
            async let areasDef = container.rides.areas()
            async let savedDef: [SavedPlace] = (try? await container.rides.savedPlaces()) ?? []
            async let proxyDef: [ProxyRider] = (try? await container.rides.proxyRiders()) ?? []
            async let eventDef: EventItem? = {
                guard let eid = eventId else { return nil }
                return try await container.events.detail(id: eid)
            }()

            let (loadedAreas, saved, proxies, loadedEvent) = try await (areasDef, savedDef, proxyDef, eventDef)

            let defaultPlace = isProxy ? nil : (saved.first { $0.isDefault == 1 } ?? saved.first)
            let defaultNotes = defaultPlace?.gateNotes.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            if !defaultNotes.isEmpty {
                lastAppliedPlaceNotes = defaultNotes
            }

            if let loadedEvent {
                if let dt = KlTime.parse(loadedEvent.startsAt) {
                    let comps = KlTime.components(dt)
                    let startMins = (comps.hour ?? 9) * 60 + (comps.minute ?? 0)
                    let outMins = max(0, startMins - 40)
                    let retMins = min(23 * 60 + 59, startMins + 120)
                    date = dt
                    hour = outMins / 60
                    minute = outMins % 60
                    returnDate = dt
                    returnHour = retMins / 60
                    returnMinute = retMins % 60
                }
                destination = loadedEvent.location
            }

            loading = false
            event = loadedEvent
            areas = loadedAreas.filter(\.active)
            savedPlaces = saved
            proxyRiders = proxies
            if let defArea = defaultPlace?.areaId {
                areaId = defArea
            }
            if let defAddr = defaultPlace?.address {
                address = defAddr
            }
            if notes.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && !defaultNotes.isEmpty {
                notes = defaultNotes
            }
        } catch let e as ApiException {
            loading = false
            loadError = e.message
        } catch {
            loading = false
            loadError = error.localizedDescription
        }
    }

    public func refreshPlacesAndProxies() async {
        async let savedDef = try? container.rides.savedPlaces()
        async let proxyDef = try? container.rides.proxyRiders()
        let (saved, proxies) = await (savedDef, proxyDef)
        if let saved { savedPlaces = saved }
        if let proxies { proxyRiders = proxies }
    }

    public func setTripType(_ newType: TripType) {
        error = nil
        errorKey = nil
        guard let ev = event, let dt = KlTime.parse(ev.startsAt) else {
            if newType == .round {
                let outMins = hour * 60 + minute
                let curRetMins = returnHour * 60 + returnMinute
                let sameOrBefore = !KlTime.klCalendar.isDate(returnDate, inSameDayAs: date) && returnDate > date
                if !sameOrBefore && curRetMins <= outMins {
                    let retMins = min(23 * 60 + 59, outMins + 120)
                    returnHour = retMins / 60
                    returnMinute = retMins % 60
                }
            }
            tripType = newType
            return
        }

        let defaultHomeAddress = (savedPlaces.first { $0.isDefault == 1 } ?? savedPlaces.first)?.address ?? ""
        let comps = KlTime.components(dt)
        let startMins = (comps.hour ?? 9) * 60 + (comps.minute ?? 0)

        switch newType {
        case .returnOnly:
            let retMins = min(23 * 60 + 59, startMins + 120)
            let homeDest: String
            if !address.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && address != ev.location {
                homeDest = address
            } else if !destination.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && destination != ev.location {
                homeDest = destination
            } else {
                homeDest = defaultHomeAddress
            }
            tripType = newType
            date = dt
            hour = retMins / 60
            minute = retMins % 60
            address = ev.location
            destination = homeDest
            suggestions = []
        case .outbound, .round:
            let outMins = max(0, startMins - 40)
            let retMins = min(23 * 60 + 59, startMins + 120)
            let homeAddr: String
            if address == ev.location {
                homeAddr = (!destination.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && destination != ev.location)
                    ? destination
                    : defaultHomeAddress
            } else {
                homeAddr = address
            }
            tripType = newType
            date = dt
            hour = outMins / 60
            minute = outMins % 60
            if newType == .round {
                returnDate = dt
                returnHour = retMins / 60
                returnMinute = retMins % 60
            }
            destination = ev.location
            address = homeAddr
        }
    }

    public func selectProxyRider(_ rider: ProxyRider) {
        if tripType != .returnOnly && !rider.pickupAddress.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            address = rider.pickupAddress
        } else if tripType == .returnOnly && !rider.pickupAddress.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            destination = rider.pickupAddress
        }
        let combinedNotes = [rider.gateNotes.trimmingCharacters(in: .whitespacesAndNewlines),
                             rider.notes.trimmingCharacters(in: .whitespacesAndNewlines)]
            .filter { !$0.isEmpty }
            .joined(separator: " · ")
        if !combinedNotes.isEmpty && (notes.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || notes == lastAppliedPlaceNotes) {
            lastAppliedPlaceNotes = combinedNotes
            notes = combinedNotes
        }
        isProxy = true
        riderName = rider.name
        riderPhone = rider.phone
        riderRelationship = rider.relationship.isEmpty ? "parent" : rider.relationship
        if let ridArea = rider.areaId { areaId = ridArea }
        error = nil
        errorKey = nil
    }

    public func onAddressChange(_ value: String) {
        address = value
        error = nil
        errorKey = nil
        searchTask?.cancel()
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count >= 3 else {
            suggestions = []
            return
        }
        searchTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 400_000_000)
            guard !Task.isCancelled, let self else { return }
            do {
                self.suggestions = try await self.container.rides.places(query: trimmed)
            } catch {
                self.suggestions = []
            }
        }
    }

    public func pickSuggestion(_ place: Place) {
        address = place.label
        pickedPlace = place
        if areaId == nil {
            areaId = AreaMatch.inferAreaId(address: place.label, areas: areas)
        }
        suggestions = []
    }

    public func applySavedPlace(_ place: SavedPlace) {
        let nextNotes = stripAppliedNotes(curNotes: notes, applied: lastAppliedPlaceNotes)
        let gate = place.gateNotes.trimmingCharacters(in: .whitespacesAndNewlines)
        let mergedNotes: String
        if gate.isEmpty {
            mergedNotes = nextNotes
        } else if nextNotes.contains(gate) {
            mergedNotes = nextNotes
        } else if nextNotes.isEmpty {
            mergedNotes = gate
        } else {
            mergedNotes = "\(nextNotes)\n\(gate)"
        }
        lastAppliedPlaceNotes = gate.isEmpty ? nil : gate

        if let pArea = place.areaId {
            areaId = pArea
        }
        if tripType == .returnOnly {
            destination = place.address
        } else {
            address = place.address
        }
        notes = mergedNotes
        suggestions = []
        saveAddress = false
        saveAddressLabel = ""
        error = nil
        errorKey = nil
    }

    private func stripAppliedNotes(curNotes: String, applied: String?) -> String {
        let clean = applied?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if clean.isEmpty { return curNotes }
        if curNotes.trimmingCharacters(in: .whitespacesAndNewlines) == clean { return "" }
        return curNotes
            .split(separator: "\n")
            .map(String.init)
            .filter { $0.trimmingCharacters(in: .whitespacesAndNewlines) != clean }
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    public func submit() async {
        guard !submitting else { return }
        guard let event else {
            error = nil
            errorKey = "ride_new_err_no_event"
            return
        }
        guard event.mySignup?.status == "going" else {
            error = nil
            errorKey = "ride_new_err_need_signup"
            return
        }
        if isProxy && riderName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            error = nil
            errorKey = "ride_new_err_rider_name"
            return
        }
        guard let areaId,
              !address.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !destination.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            error = nil
            errorKey = "ride_new_err_required"
            return
        }

        let pickupAt = KlTime.apiStamp(date: date, hour: hour, minute: minute)
        let isReturn = tripType == .returnOnly
        let isRound = tripType == .round
        let memberAddr = (isReturn ? destination : address).trimmingCharacters(in: .whitespacesAndNewlines)
        let label = saveAddressLabel.trimmingCharacters(in: .whitespacesAndNewlines)
        let alreadySaved = savedPlaces.contains { $0.address.trimmingCharacters(in: .whitespacesAndNewlines).caseInsensitiveCompare(memberAddr) == .orderedSame }
        let isEventLocation = memberAddr.caseInsensitiveCompare(event.location.trimmingCharacters(in: .whitespacesAndNewlines)) == .orderedSame
        let shouldSaveAddress = !isProxy && saveAddress && !memberAddr.isEmpty && !alreadySaved && !isEventLocation && savedPlaces.count < 50

        if shouldSaveAddress && label.isEmpty {
            error = nil
            errorKey = "ride_new_err_save_label"
            return
        }

        let returnPickupAt = isRound ? KlTime.apiStamp(date: returnDate, hour: returnHour, minute: returnMinute) : nil
        if let returnPickupAt, returnPickupAt <= pickupAt {
            error = nil
            errorKey = "ride_new_err_return_after"
            return
        }

        submitting = true
        error = nil
        errorKey = nil
        defer { submitting = false }

        do {
            let cleanNotes = notes.trimmingCharacters(in: .whitespacesAndNewlines)
            let cleanPhone = riderPhone.trimmingCharacters(in: .whitespacesAndNewlines)
            let res = try await container.rides.create(CreateRideReq(
                pickupAt: pickupAt,
                pickupAreaId: areaId,
                pickupAddress: address.trimmingCharacters(in: .whitespacesAndNewlines),
                destination: destination.trimmingCharacters(in: .whitespacesAndNewlines),
                seats: Int64(seats),
                assistance: assistance.isEmpty ? nil : assistance,
                notes: cleanNotes.isEmpty ? nil : cleanNotes,
                eventId: event.id,
                leg: isReturn ? "return" : "outbound",
                isProxy: isProxy ? true : nil,
                riderName: isProxy ? riderName.trimmingCharacters(in: .whitespacesAndNewlines) : nil,
                riderPhone: isProxy && !cleanPhone.isEmpty ? cleanPhone : nil,
                riderRelationship: isProxy ? riderRelationship : nil,
                saveProxyRider: (isProxy && saveProxyRider) ? true : nil,
                returnPickupAt: returnPickupAt,
                returnPickupAreaId: isRound ? areaId : nil,
                returnPickupAddress: isRound ? destination.trimmingCharacters(in: .whitespacesAndNewlines) : nil,
                returnDestination: isRound ? address.trimmingCharacters(in: .whitespacesAndNewlines) : nil
            ))

            if shouldSaveAddress {
                let matchingCoords = pickedPlace.flatMap {
                    $0.label.trimmingCharacters(in: .whitespacesAndNewlines).caseInsensitiveCompare(memberAddr) == .orderedSame ? $0 : nil
                }
                _ = try? await container.rides.createSavedPlace(SavedPlaceReq(
                    label: label,
                    areaId: areaId,
                    address: memberAddr,
                    gateNotes: String(cleanNotes.prefix(300)),
                    isDefault: savedPlaces.isEmpty,
                    lat: matchingCoords?.lat,
                    lng: matchingCoords?.lng
                ))
            }
            createdId = res.id
        } catch let e as ApiException {
            error = e.message
        } catch {
            self.error = error.localizedDescription
        }
    }
}
