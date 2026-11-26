import GraceRideKit
import SwiftUI

public struct RidesScreen: View {
    public let container: AppContainer
    public let initialTab: Int
    public let onNavigate: (AppRoute) -> Void

    @State private var vm: RidesViewModel

    public init(
        container: AppContainer,
        initialTab: Int = 0,
        onNavigate: @escaping (AppRoute) -> Void
    ) {
        self.container = container
        self.initialTab = initialTab
        self.onNavigate = onNavigate
        self._vm = State(initialValue: RidesViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Picker("", selection: Binding(
                    get: { vm.tab },
                    set: { newTab in Task { await vm.setTab(newTab) } }
                )) {
                    Text(L10n.tr("rides_tab_upcoming", lang: lang)).tag(0)
                    Text(L10n.tr("rides_tab_past", lang: lang)).tag(1)
                }
                .pickerStyle(.segmented)

                if vm.offline {
                    CalloutBanner(
                        tone: .warning,
                        icon: "wifi.slash",
                        text: L10n.tr("network_offline_live", lang: lang)
                    )
                }

                if let err = vm.error {
                    ErrorBannerView(
                        message: err,
                        retryTitle: L10n.tr("common_retry", lang: lang),
                        onRetry: { Task { await vm.refresh() } }
                    )
                }

                if vm.loading && vm.rides.isEmpty {
                    LoadingSkeletonView(rows: 3)
                } else if vm.rides.isEmpty {
                    EmptyStateCard(
                        systemImage: "car.fill",
                        title: L10n.tr(vm.tab == 1 ? "history_empty" : "rides_empty", lang: lang),
                        actionTitle: container.auth.me?.can("rides", act: "create") == true ? L10n.tr("rides_book", lang: lang) : nil
                    ) {
                        onNavigate(.rideNew(eventId: nil, proxy: false))
                    }
                } else {
                    ForEach(vm.rides) { ride in
                        Button {
                            onNavigate(.rideDetail(id: ride.id, justBooked: false))
                        } label: {
                            RideSummaryCard(ride: ride, lang: lang)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr(initialTab == 1 ? "history_title" : "rides_title", lang: lang))
        .toolbar {
            if container.auth.me?.can("rides", act: "create") == true {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        onNavigate(.rideNew(eventId: nil, proxy: false))
                    } label: {
                        Image(systemName: "plus")
                    }
                    .accessibilityLabel(L10n.tr("rides_book", lang: lang))
                }
            }
        }
        .refreshable {
            await vm.refresh()
        }
        .task {
            if initialTab != vm.tab {
                await vm.setTab(initialTab)
            } else {
                await vm.loadInitial()
            }
        }
    }
}

public struct HistoryScreen: View {
    public let container: AppContainer
    public let onNavigate: (AppRoute) -> Void

    public init(container: AppContainer, onNavigate: @escaping (AppRoute) -> Void) {
        self.container = container
        self.onNavigate = onNavigate
    }

    public var body: some View {
        RidesScreen(container: container, initialTab: 1, onNavigate: onNavigate)
    }
}

public struct RideSummaryCard: View {
    public let ride: Ride
    public let lang: String

    public init(ride: Ride, lang: String) {
        self.ride = ride
        self.lang = lang
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(KlTime.display(ride.pickupAt, lang: lang))
                    .font(GrTypography.bodyMedium)
                    .foregroundStyle(GrColors.text)
                Spacer()
                RideStatusBadge(status: ride.status, lang: lang)
            }
            Text("\(ride.area.name) → \(ride.destination)")
                .font(GrTypography.subheadline)
                .foregroundStyle(GrColors.text)
                .lineLimit(2)
            HStack(spacing: 10) {
                if ride.isProxy, let proxyName = ride.proxy?.riderName {
                    Label(L10n.tr("ride_proxy_label", lang: lang, proxyName), systemImage: "person.2.fill")
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.primary)
                }
                if let ev = ride.event?.title, !ev.isEmpty {
                    Label(ev, systemImage: "calendar")
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                        .lineLimit(1)
                }
                Spacer()
                if let driverName = ride.driver?.name, !driverName.isEmpty {
                    Text(driverName)
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .grCard()
    }
}

public struct RideNewScreen: View {
    public let container: AppContainer
    public let eventId: Int64?
    public let initialProxy: Bool
    public let onCreated: (Int64) -> Void
    public let onOpenEvent: (Int64) -> Void
    public let onBrowseEvents: () -> Void

    @State private var vm: RideNewViewModel

    public init(
        container: AppContainer,
        eventId: Int64?,
        initialProxy: Bool = false,
        onCreated: @escaping (Int64) -> Void,
        onOpenEvent: @escaping (Int64) -> Void,
        onBrowseEvents: @escaping () -> Void
    ) {
        self.container = container
        self.eventId = eventId
        self.initialProxy = initialProxy
        self.onCreated = onCreated
        self.onOpenEvent = onOpenEvent
        self.onBrowseEvents = onBrowseEvents
        self._vm = State(initialValue: RideNewViewModel(container: container, eventId: eventId, initialProxy: initialProxy))
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                if let loadError = vm.loadError {
                    ErrorBannerView(
                        message: loadError,
                        retryTitle: L10n.tr("common_retry", lang: lang),
                        onRetry: { Task { await vm.retryLoad() } }
                    )
                }

                if vm.loading {
                    LoadingSkeletonView(rows: 4)
                } else if eventId == nil {
                    VStack(alignment: .leading, spacing: 10) {
                        Text(L10n.tr("ride_new_event_first_title", lang: lang))
                            .font(GrTypography.titleSmall)
                        Text(L10n.tr("ride_new_event_first_desc", lang: lang))
                            .font(GrTypography.subheadline)
                            .foregroundStyle(GrColors.muted)
                        Button(L10n.tr("ride_new_browse_events", lang: lang), action: onBrowseEvents)
                            .buttonStyle(GrPrimaryButtonStyle())
                    }
                    .grCard()
                } else {
                    if let ev = vm.event {
                        HStack(spacing: 10) {
                            EventIconView(iconName: ev.icon, category: ev.kind, size: 36)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(ev.title).font(GrTypography.bodyMedium)
                                Text("\(KlTime.display(ev.startsAt, lang: lang)) · \(ev.location)")
                                    .font(GrTypography.caption)
                                    .foregroundStyle(GrColors.muted)
                            }
                            Spacer()
                        }
                        .grCard(background: GrColors.primarySoft)

                        if ev.mySignup == nil {
                            CalloutBanner(
                                tone: .warning,
                                icon: "exclamationmark.circle.fill",
                                text: L10n.tr("ride_new_signup_required_desc", lang: lang, ev.title),
                                actionTitle: L10n.tr("ride_new_go_to_event", lang: lang)
                            ) {
                                onOpenEvent(ev.id)
                            }
                        }
                    }

                    tripTypeAndScheduleSection(lang: lang)
                    proxySection(lang: lang)
                    locationSection(lang: lang)
                    detailsSection(lang: lang)

                    if let err = vm.error ?? vm.errorKey.map({ L10n.tr($0, lang: lang) }) {
                        ErrorBannerView(message: err)
                    }

                    Button {
                        Task { await vm.submit() }
                    } label: {
                        HStack {
                            if vm.submitting { ProgressView().tint(.white) }
                            Text(L10n.tr("ride_new_submit", lang: lang))
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                    .disabled(vm.submitting || (vm.event != nil && vm.event?.mySignup == nil))
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("ride_new_title", lang: lang))
        .task {
            await vm.retryLoad()
        }
        .onChange(of: vm.createdId) { _, newId in
            if let newId {
                Haptics.success()
                onCreated(newId)
            }
        }
    }

    @ViewBuilder
    private func tripTypeAndScheduleSection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("ride_new_section_schedule", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            Picker(L10n.tr("ride_new_trip_type_label", lang: lang), selection: Binding(
                get: { vm.tripType },
                set: { vm.setTripType($0) }
            )) {
                Text(L10n.tr("ride_new_trip_type_outbound", lang: lang)).tag(TripType.outbound)
                Text(L10n.tr("ride_new_trip_type_round", lang: lang)).tag(TripType.round)
                Text(L10n.tr("ride_new_trip_type_return", lang: lang)).tag(TripType.returnOnly)
            }
            .pickerStyle(.segmented)

            let outBinding = Binding<Date>(
                get: {
                    var comps = KlTime.components(vm.date)
                    comps.hour = vm.hour
                    comps.minute = vm.minute
                    return KlTime.klCalendar.date(from: comps) ?? vm.date
                },
                set: { newDate in
                    vm.date = newDate
                    let c = KlTime.components(newDate)
                    vm.hour = c.hour ?? 9
                    vm.minute = c.minute ?? 0
                }
            )
            DatePicker(L10n.tr("ride_new_pickup_time", lang: lang), selection: outBinding, displayedComponents: [.date, .hourAndMinute])

            if vm.tripType == .round {
                let retBinding = Binding<Date>(
                    get: {
                        var comps = KlTime.components(vm.returnDate)
                        comps.hour = vm.returnHour
                        comps.minute = vm.returnMinute
                        return KlTime.klCalendar.date(from: comps) ?? vm.returnDate
                    },
                    set: { newDate in
                        vm.returnDate = newDate
                        let c = KlTime.components(newDate)
                        vm.returnHour = c.hour ?? 12
                        vm.returnMinute = c.minute ?? 0
                    }
                )
                DatePicker(L10n.tr("ride_new_return_time", lang: lang), selection: retBinding, displayedComponents: [.date, .hourAndMinute])
                Text(L10n.tr("ride_new_time_hint_return", lang: lang))
                    .font(GrTypography.caption)
                    .foregroundStyle(GrColors.muted)
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func proxySection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Toggle(L10n.tr("ride_new_is_proxy", lang: lang), isOn: $vm.isProxy)
                .tint(GrColors.primary)

            if vm.isProxy {
                if !vm.proxyRiders.isEmpty {
                    Text(L10n.tr("ride_new_frequent_riders", lang: lang))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 8) {
                            ForEach(vm.proxyRiders) { rider in
                                Button(rider.name) {
                                    vm.selectProxyRider(rider)
                                }
                                .font(GrTypography.caption.weight(.semibold))
                                .padding(.horizontal, 10)
                                .padding(.vertical, 6)
                                .background(Capsule().fill(GrColors.primarySoft))
                            }
                        }
                    }
                }

                TextField(L10n.tr("ride_new_rider_name", lang: lang), text: $vm.riderName)
                    .textFieldStyle(.roundedBorder)
                TextField(L10n.tr("ride_new_rider_phone", lang: lang), text: $vm.riderPhone)
                    .keyboardType(.phonePad)
                    .textFieldStyle(.roundedBorder)

                Picker(L10n.tr("ride_new_rider_relationship", lang: lang), selection: $vm.riderRelationship) {
                    Text(L10n.tr("ride_new_relationship_parent", lang: lang)).tag("parent")
                    Text(L10n.tr("ride_new_relationship_child", lang: lang)).tag("child")
                    Text(L10n.tr("ride_new_relationship_spouse", lang: lang)).tag("spouse")
                    Text(L10n.tr("ride_new_relationship_friend", lang: lang)).tag("friend")
                    Text(L10n.tr("proxy_rel_cell", lang: lang)).tag("cell_member")
                    Text(L10n.tr("ride_new_relationship_other", lang: lang)).tag("other")
                }

                Toggle(L10n.tr("ride_new_save_proxy", lang: lang), isOn: $vm.saveProxyRider)
                    .font(GrTypography.subheadline)
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func locationSection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("ride_new_section_location", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            if !vm.savedPlaces.isEmpty && !vm.isProxy {
                Text(L10n.tr("ride_new_saved_addresses", lang: lang))
                    .font(GrTypography.caption)
                    .foregroundStyle(GrColors.muted)
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(vm.savedPlaces) { sp in
                            Button(sp.label) {
                                vm.applySavedPlace(sp)
                            }
                            .font(GrTypography.caption.weight(.semibold))
                            .padding(.horizontal, 10)
                            .padding(.vertical, 6)
                            .background(Capsule().fill(GrColors.surface2))
                            .overlay(Capsule().stroke(GrColors.border, lineWidth: 1))
                        }
                    }
                }
            }

            if !vm.areas.isEmpty {
                Picker(L10n.tr("ride_new_area", lang: lang), selection: $vm.areaId) {
                    Text("-").tag(Int64?.none)
                    ForEach(vm.areas.filter(\.active)) { area in
                        Text(area.name).tag(Int64?.some(area.id))
                    }
                }
            }

            TextField(
                L10n.tr("ride_new_address", lang: lang),
                text: Binding(
                    get: { vm.address },
                    set: { vm.onAddressChange($0) }
                )
            )
            .textFieldStyle(.roundedBorder)

            if !vm.suggestions.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(vm.suggestions) { place in
                        Button {
                            vm.pickSuggestion(place)
                        } label: {
                            Text(place.label)
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.text)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.vertical, 6)
                        }
                        .buttonStyle(.plain)
                        Divider()
                    }
                }
                .padding(8)
                .background(GrColors.surface2)
                .clipShape(RoundedRectangle(cornerRadius: 8))
            }

            TextField(L10n.tr("ride_new_destination", lang: lang), text: $vm.destination)
                .textFieldStyle(.roundedBorder)

            if !vm.isProxy {
                Toggle(L10n.tr("ride_new_save_address", lang: lang), isOn: $vm.saveAddress)
                    .font(GrTypography.subheadline)
                if vm.saveAddress {
                    TextField(L10n.tr("ride_new_save_address_label", lang: lang), text: $vm.saveAddressLabel)
                        .textFieldStyle(.roundedBorder)
                }
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func detailsSection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("ride_new_section_details", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            Stepper("\(L10n.tr("ride_new_seats", lang: lang)): \(vm.seats)", value: $vm.seats, in: 1...8)

            Picker(L10n.tr("ride_new_assistance", lang: lang), selection: $vm.assistance) {
                Text(L10n.tr("ride_new_assist_none", lang: lang)).tag("")
                Text(L10n.tr("ride_new_assist_wheelchair", lang: lang)).tag("wheelchair")
                Text(L10n.tr("ride_new_assist_stroller", lang: lang)).tag("stroller")
                Text(L10n.tr("ride_new_assist_oxygen", lang: lang)).tag("oxygen")
                Text(L10n.tr("ride_new_assist_other", lang: lang)).tag("other")
            }

            TextField(L10n.tr("ride_new_notes", lang: lang), text: $vm.notes, axis: .vertical)
                .lineLimit(2...4)
                .textFieldStyle(.roundedBorder)
        }
        .grCard()
    }
}
