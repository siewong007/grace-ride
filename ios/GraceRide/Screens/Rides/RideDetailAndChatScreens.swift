import CoreLocation
import GraceRideKit
import MapKit
import SwiftUI

public struct RideDetailScreen: View {
    public let container: AppContainer
    public let locationManager: LiveTripLocationManager
    public let rideId: Int64
    public let justBooked: Bool
    public let onNavigate: (AppRoute) -> Void

    @State private var vm: RideDetailViewModel
    @State private var chatVM: TripChatViewModel
    @State private var showChat = false
    @State private var showCancelDialog = false
    @State private var cancelReason = ""
    @State private var showWithdrawDialog = false
    @State private var withdrawReason = ""
    @State private var showNavChooser = false
    @State private var sharingBusy = false

    public init(
        container: AppContainer,
        locationManager: LiveTripLocationManager,
        rideId: Int64,
        justBooked: Bool = false,
        onNavigate: @escaping (AppRoute) -> Void
    ) {
        self.container = container
        self.locationManager = locationManager
        self.rideId = rideId
        self.justBooked = justBooked
        self.onNavigate = onNavigate
        self._vm = State(initialValue: RideDetailViewModel(container: container, rideId: rideId))
        self._chatVM = State(initialValue: TripChatViewModel(container: container, rideId: rideId))
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if justBooked {
                    CalloutBanner(
                        tone: .success,
                        icon: "checkmark.circle.fill",
                        text: L10n.tr("ride_new_success", lang: lang)
                    )
                }

                if vm.staleReload {
                    CalloutBanner(
                        tone: .warning,
                        icon: "arrow.triangle.2.circlepath",
                        text: L10n.tr("error_ride_changed", lang: lang)
                    )
                }

                if let msg = vm.message {
                    ErrorBannerView(message: msg, onDismiss: { vm.clearMessage() })
                }

                if let err = vm.error {
                    ErrorBannerView(
                        message: err,
                        retryTitle: L10n.tr("common_retry", lang: lang),
                        onRetry: { Task { await vm.refresh() } }
                    )
                }

                if vm.loading && vm.ride == nil {
                    LoadingSkeletonView(rows: 4)
                } else if let ride = vm.ride {
                    statusHeaderCard(ride: ride, lang: lang)
                    routeSection(ride: ride, lang: lang)
                    peopleSection(ride: ride, lang: lang)

                    if RideStatus.isLiveTrackable(ride.status) && (ride.viewer.isPassenger || ride.viewer.isDriver || ride.viewer.isStaff) {
                        LiveTrackCard(
                            ride: ride,
                            liveLocation: vm.liveLocation,
                            locationManager: locationManager,
                            sharingPassengerPin: vm.sharingPassengerPin,
                            sharingBusy: sharingBusy,
                            lang: lang,
                            onStartDriverSharing: {
                                sharingBusy = true
                                try? await locationManager.startDriverSharing()
                                await vm.refreshLiveLocation()
                                sharingBusy = false
                            },
                            onStopDriverSharing: {
                                sharingBusy = true
                                await locationManager.stopDriverSharing(notifyServer: true)
                                await vm.refreshLiveLocation()
                                sharingBusy = false
                            },
                            onSharePassengerSpot: {
                                sharingBusy = true
                                if let coord = try? await locationManager.captureCurrentLocation() {
                                    await vm.sharePassengerSpot(lat: coord.lat, lng: coord.lng)
                                }
                                sharingBusy = false
                            },
                            onStopPassengerSpot: {
                                sharingBusy = true
                                await vm.stopPassengerSpot()
                                sharingBusy = false
                            }
                        )
                    }

                    passengerActionsSection(ride: ride, lang: lang)
                    driverActionsSection(ride: ride, lang: lang)
                    detailsSection(ride: ride, lang: lang)
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("ride_detail_title", lang: lang))
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    showChat = true
                } label: {
                    Label(L10n.tr("chat_title", lang: lang), systemImage: "bubble.left.and.bubble.right.fill")
                }
            }
        }
        .refreshable {
            await vm.refresh()
        }
        .task {
            await vm.load()
        }
        .sheet(isPresented: $showChat) {
            TripChatSheet(vm: chatVM, lang: lang)
        }
        .alert(L10n.tr("ride_cancel", lang: lang), isPresented: $showCancelDialog) {
            TextField(L10n.tr("driver_withdraw_reason", lang: lang), text: $cancelReason)
            Button(L10n.tr("common_cancel", lang: lang), role: .cancel) {}
            Button(L10n.tr("common_confirm", lang: lang), role: .destructive) {
                Task {
                    await vm.cancel(reason: cancelReason.isEmpty ? nil : cancelReason)
                    cancelReason = ""
                }
            }
        } message: {
            Text(L10n.tr("ride_cancel_confirm", lang: lang))
        }
        .alert(L10n.tr("driver_withdraw", lang: lang), isPresented: $showWithdrawDialog) {
            TextField(L10n.tr("driver_withdraw_reason", lang: lang), text: $withdrawReason)
            Button(L10n.tr("common_cancel", lang: lang), role: .cancel) {}
            Button(L10n.tr("common_confirm", lang: lang), role: .destructive) {
                Task {
                    await vm.withdraw(reason: withdrawReason.isEmpty ? nil : withdrawReason)
                    withdrawReason = ""
                }
            }
        } message: {
            Text(L10n.tr("driver_withdraw_confirm", lang: lang))
        }
        .background(
            MapsDestinationChooser(
                title: vm.ride?.destination ?? "",
                lat: vm.liveLocation?.lat,
                lng: vm.liveLocation?.lng,
                address: vm.ride?.pickupAddress ?? vm.ride?.destination,
                lang: lang,
                isPresented: $showNavChooser
            )
        )
    }

    @ViewBuilder
    private func statusHeaderCard(ride: Ride, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(KlTime.display(ride.pickupAt, lang: lang))
                    .font(GrTypography.titleSmall)
                Spacer()
                RideStatusBadge(status: ride.status, lang: lang)
            }

            let step = RideStatus.stepIndex(ride.status)
            if step >= 0 {
                let labels = [
                    L10n.tr("ride_step_requested", lang: lang),
                    L10n.tr("ride_step_confirmed", lang: lang),
                    L10n.tr("ride_step_en_route", lang: lang),
                    L10n.tr("ride_step_done", lang: lang),
                ]
                HStack(spacing: 4) {
                    ForEach(0..<4, id: \.self) { idx in
                        VStack(spacing: 4) {
                            Capsule()
                                .fill(idx <= step ? GrColors.primary : GrColors.surfaceHigh)
                                .frame(height: 6)
                            Text(labels[idx])
                                .font(.system(size: 11, weight: idx == step ? .semibold : .regular))
                                .foregroundStyle(idx <= step ? GrColors.primary : GrColors.muted)
                                .lineLimit(1)
                        }
                    }
                }
            }

            if ride.delayMinutes > 0 {
                CalloutBanner(
                    tone: .warning,
                    icon: "clock.badge.exclamationmark",
                    text: L10n.tr("ride_driver_delay", lang: lang, Int64(ride.delayMinutes))
                )
            }

            if let arrived = ride.arrivedAtGateAt {
                CalloutBanner(
                    tone: .info,
                    icon: "bell.badge.fill",
                    text: L10n.tr("ride_arrived_gate", lang: lang, KlTime.displayTime(arrived))
                )
            }

            if ride.needsAttention && !ride.attentionReason.isEmpty {
                CalloutBanner(tone: .danger, icon: "exclamationmark.triangle.fill", text: ride.attentionReason)
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func routeSection(ride: Ride, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("ride_section_route", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            VStack(alignment: .leading, spacing: 8) {
                Label("\(ride.area.name)\(ride.pickupAddress.map { " · \($0)" } ?? "")", systemImage: "figure.wave")
                    .font(GrTypography.bodyMedium)
                Label(ride.destination, systemImage: "flag.fill")
                    .font(GrTypography.body)
                    .foregroundStyle(GrColors.muted)
            }

            HStack {
                if let paired = ride.pairedRideId {
                    Button(L10n.tr("ride_view_paired", lang: lang)) {
                        onNavigate(.rideDetail(id: paired, justBooked: false))
                    }
                    .buttonStyle(GrOutlineButtonStyle())
                }
                Button(L10n.tr("common_open_maps", lang: lang)) {
                    showNavChooser = true
                }
                .buttonStyle(GrOutlineButtonStyle())
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .grCard()
    }

    @ViewBuilder
    private func peopleSection(ride: Ride, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("ride_section_people", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(L10n.tr("ride_passenger", lang: lang))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                    Text(ride.proxy?.riderName ?? ride.passenger.name)
                        .font(GrTypography.bodyMedium)
                    if let proxy = ride.proxy, proxy.isProxy {
                        Text(L10n.tr("ride_proxy_booker", lang: lang, proxy.bookerName))
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.muted)
                    }
                }
                Spacer()
                if let phone = ride.proxy?.riderPhone ?? ride.passenger.phone, !phone.isEmpty,
                   ride.viewer.isDriver || ride.viewer.isStaff {
                    Button {
                        DeviceActions.dial(phone: phone)
                    } label: {
                        Label(L10n.tr("common_call", lang: lang), systemImage: "phone.fill")
                    }
                    .buttonStyle(GrOutlineButtonStyle())
                }
            }

            if let driver = ride.driver {
                Divider()
                HStack(spacing: 10) {
                    AvatarView(
                        name: driver.name ?? "",
                        photoFileId: driver.photoFileId,
                        avatarColor: driver.avatarColor,
                        avatarIcon: driver.avatarIcon,
                        size: 38,
                        container: container
                    )
                    VStack(alignment: .leading, spacing: 2) {
                        Text(L10n.tr("ride_driver", lang: lang))
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.muted)
                        Text(driver.name ?? "-")
                            .font(GrTypography.bodyMedium)
                        if let v = driver.vehicle {
                            Text("\(v.color ?? "") \(v.model) · \(v.plate ?? "")")
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                        }
                    }
                    Spacer()
                    if let phone = driver.phone, !phone.isEmpty {
                        Button {
                            DeviceActions.dial(phone: phone)
                        } label: {
                            Label(L10n.tr("common_call", lang: lang), systemImage: "phone.fill")
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                    }
                }
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func passengerActionsSection(ride: Ride, lang: String) -> some View {
        if ride.viewer.isPassenger {
            VStack(alignment: .leading, spacing: 10) {
                if ride.status == "pending_passenger" {
                    HStack(spacing: 10) {
                        Button(L10n.tr("ride_respond_accept", lang: lang)) {
                            Task { await vm.respond(accept: true) }
                        }
                        .buttonStyle(GrPrimaryButtonStyle())
                        .disabled(vm.busy)

                        Button(L10n.tr("ride_respond_decline", lang: lang)) {
                            Task { await vm.respond(accept: false) }
                        }
                        .buttonStyle(GrOutlineButtonStyle(tint: GrColors.danger))
                        .disabled(vm.busy)
                    }
                }

                if Readiness.shouldShowPrompt(ride: ride) || ride.status == "in_progress" {
                    Text(L10n.tr("ride_readiness", lang: lang))
                        .font(GrTypography.label)
                    HStack(spacing: 8) {
                        readinessButton("ready", label: L10n.tr("readiness_ready", lang: lang), current: ride.readinessStatus)
                        readinessButton("delayed_5m", label: L10n.tr("readiness_delayed_5m", lang: lang), current: ride.readinessStatus)
                        readinessButton("delayed_10m", label: L10n.tr("readiness_delayed_10m", lang: lang), current: ride.readinessStatus)
                    }
                }

                if RideStatus.cancellable(ride.status) {
                    Button(L10n.tr("ride_cancel", lang: lang)) {
                        showCancelDialog = true
                    }
                    .buttonStyle(GrOutlineButtonStyle(tint: GrColors.danger))
                    .disabled(vm.busy)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .grCard()
        }
    }

    private func readinessButton(_ value: String, label: String, current: String) -> some View {
        let active = current == value
        return Button(label) {
            Haptics.selection()
            Task { await vm.readiness(status: value) }
        }
        .font(GrTypography.caption.weight(.semibold))
        .foregroundStyle(active ? GrColors.primaryInk : GrColors.text)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(Capsule().fill(active ? GrColors.primary : GrColors.surface2))
        .overlay(Capsule().stroke(active ? GrColors.primary : GrColors.border, lineWidth: 1))
        .disabled(vm.busy)
    }

    @ViewBuilder
    private func driverActionsSection(ride: Ride, lang: String) -> some View {
        if ride.viewer.isDriver {
            VStack(alignment: .leading, spacing: 10) {
                Text(L10n.tr("driver_actions_title", lang: lang))
                    .font(GrTypography.label)
                    .foregroundStyle(GrColors.muted)

                if ride.status == "pending_driver_confirm" {
                    Button(L10n.tr("driver_confirm_assignment", lang: lang)) {
                        Task { await vm.confirmAssignment() }
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                    .disabled(vm.busy)
                } else if ride.status == "confirmed" {
                    Button(L10n.tr("driver_start_trip", lang: lang)) {
                        Task { await vm.startRide() }
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                    .disabled(vm.busy)
                } else if ride.status == "in_progress" {
                    HStack(spacing: 8) {
                        if ride.arrivedAtGateAt == nil {
                            Button(L10n.tr("driver_arrived_gate", lang: lang)) {
                                Task { await vm.arrivedGate() }
                            }
                            .buttonStyle(GrOutlineButtonStyle())
                            .disabled(vm.busy)
                        }
                        if ride.pickedUpAt == nil {
                            Button(L10n.tr("driver_picked_up", lang: lang)) {
                                Task { await vm.pickup() }
                            }
                            .buttonStyle(GrPrimaryButtonStyle())
                            .disabled(vm.busy)
                        } else {
                            Button(L10n.tr("driver_complete_trip", lang: lang)) {
                                Task { await vm.complete() }
                            }
                            .buttonStyle(GrPrimaryButtonStyle())
                            .disabled(vm.busy)
                        }
                    }

                    HStack(spacing: 8) {
                        Button(L10n.tr("driver_delay_5m", lang: lang)) {
                            Task { await vm.driverDelay(minutes: 5) }
                        }
                        .buttonStyle(GrOutlineButtonStyle(tint: GrColors.warning))
                        Button(L10n.tr("driver_delay_10m", lang: lang)) {
                            Task { await vm.driverDelay(minutes: 10) }
                        }
                        .buttonStyle(GrOutlineButtonStyle(tint: GrColors.warning))
                    }
                }

                if ["pending_driver_confirm", "confirmed"].contains(ride.status) {
                    Button(L10n.tr("driver_withdraw", lang: lang)) {
                        showWithdrawDialog = true
                    }
                    .buttonStyle(GrOutlineButtonStyle(tint: GrColors.danger))
                    .disabled(vm.busy)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .grCard()
        }
    }

    @ViewBuilder
    private func detailsSection(ride: Ride, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(L10n.tr("ride_section_details", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)
            detailRow(L10n.tr("ride_seats", lang: lang), "\(ride.seats)")
            if !ride.assistance.isEmpty && ride.assistance != "none" {
                detailRow(L10n.tr("ride_assistance", lang: lang), ride.assistance)
            }
            if !ride.notes.isEmpty {
                detailRow(L10n.tr("ride_notes", lang: lang), ride.notes)
            }
            if let ev = ride.event?.title {
                detailRow(L10n.tr("ride_event", lang: lang), ev)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .grCard()
    }

    private func detailRow(_ title: String, _ value: String) -> some View {
        HStack(alignment: .top) {
            Text(title).font(GrTypography.caption).foregroundStyle(GrColors.muted)
            Spacer()
            Text(value).font(GrTypography.subheadline).foregroundStyle(GrColors.text).multilineTextAlignment(.trailing)
        }
    }
}

public struct LiveTrackCard: View {
    public let ride: Ride
    public let liveLocation: RideLocationDto?
    public let locationManager: LiveTripLocationManager
    public let sharingPassengerPin: Bool
    public let sharingBusy: Bool
    public let lang: String
    public let onStartDriverSharing: () async -> Void
    public let onStopDriverSharing: () async -> Void
    public let onSharePassengerSpot: () async -> Void
    public let onStopPassengerSpot: () async -> Void

    public var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Label(L10n.tr("live_location_title", lang: lang), systemImage: "location.fill")
                    .font(GrTypography.titleSmall)
                    .foregroundStyle(GrColors.primary)
                Spacer()
                if let eta = liveLocation?.eta?.eta {
                    Text("ETA \(KlTime.displayTime(eta))")
                        .font(GrTypography.badge)
                        .foregroundStyle(GrColors.info)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(Capsule().fill(GrColors.infoSoft))
                }
            }

            if let lat = liveLocation?.lat, let lng = liveLocation?.lng {
                let driverCoord = CLLocationCoordinate2D(latitude: lat, longitude: lng)
                Map(initialPosition: .region(MKCoordinateRegion(
                    center: driverCoord,
                    span: MKCoordinateSpan(latitudeDelta: 0.015, longitudeDelta: 0.015)
                ))) {
                    Marker(L10n.tr("ride_driver", lang: lang), coordinate: driverCoord)
                        .tint(GrColors.primary)
                    if let pLat = liveLocation?.passenger?.lat, let pLng = liveLocation?.passenger?.lng {
                        Marker(
                            L10n.tr("ride_passenger", lang: lang),
                            coordinate: CLLocationCoordinate2D(latitude: pLat, longitude: pLng)
                        )
                        .tint(GrColors.brandRed)
                    }
                }
                .frame(height: 170)
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            }

            if let stopsBefore = liveLocation?.eta?.stopsBefore, stopsBefore > 0 {
                Text(L10n.tr("live_location_stops_before", lang: lang, Int64(stopsBefore)))
                    .font(GrTypography.caption)
                    .foregroundStyle(GrColors.muted)
            }

            if ride.viewer.isDriver && ride.status == "in_progress" {
                Button(locationManager.isSharing ? L10n.tr("live_location_stop", lang: lang) : L10n.tr("live_location_start", lang: lang)) {
                    Task {
                        if locationManager.isSharing {
                            await onStopDriverSharing()
                        } else {
                            await onStartDriverSharing()
                        }
                    }
                }
                .buttonStyle(GrPrimaryButtonStyle(destructive: locationManager.isSharing))
                .disabled(sharingBusy)
            }

            if LocationServicePolicy.canSharePassengerWaitingSpot(ride) {
                Button(sharingPassengerPin ? L10n.tr("live_location_stop_spot", lang: lang) : L10n.tr("live_location_share_spot", lang: lang)) {
                    Task {
                        if sharingPassengerPin {
                            await onStopPassengerSpot()
                        } else {
                            await onSharePassengerSpot()
                        }
                    }
                }
                .buttonStyle(GrOutlineButtonStyle())
                .disabled(sharingBusy)
            }
        }
        .grCard()
    }
}

public struct TripChatSheet: View {
    public var vm: TripChatViewModel
    public let lang: String

    @Environment(\.dismiss) private var dismiss
    @State private var draft = ""

    public init(vm: TripChatViewModel, lang: String) {
        self.vm = vm
        self.lang = lang
    }

    public var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                if !vm.members.isEmpty {
                    Text(L10n.tr("chat_members_label", lang: lang, vm.members.joined(separator: ", ")))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                        .padding(.horizontal)
                        .padding(.vertical, 6)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(GrColors.surface2)
                }

                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(spacing: 10) {
                            if vm.messages.isEmpty && !vm.loading {
                                Text(L10n.tr("chat_empty", lang: lang))
                                    .font(GrTypography.subheadline)
                                    .foregroundStyle(GrColors.muted)
                                    .padding(.vertical, 32)
                            }
                            ForEach(vm.messages) { msg in
                                chatBubble(msg: msg)
                                    .id(msg.id)
                            }
                        }
                        .padding()
                    }
                    .onChange(of: vm.messages.last?.id) { _, lastId in
                        if let lastId {
                            withAnimation { proxy.scrollTo(lastId, anchor: .bottom) }
                        }
                    }
                }

                if let err = vm.error {
                    ErrorBannerView(message: err)
                        .padding(.horizontal)
                }

                if vm.canPost && vm.open {
                    Divider()
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 8) {
                            presetChip(L10n.tr("chat_preset_at_gate", lang: lang))
                            presetChip(L10n.tr("chat_preset_5_mins", lang: lang))
                            presetChip(L10n.tr("chat_preset_thanks", lang: lang))
                        }
                        .padding(.horizontal)
                        .padding(.top, 8)
                    }

                    HStack(spacing: 8) {
                        TextField(L10n.tr("chat_placeholder", lang: lang), text: $draft, axis: .vertical)
                            .lineLimit(1...4)
                            .padding(10)
                            .background(GrColors.surface2)
                            .clipShape(RoundedRectangle(cornerRadius: 12))
                        Button {
                            let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
                            guard !text.isEmpty else { return }
                            draft = ""
                            Task { _ = await vm.send(text) }
                        } label: {
                            Image(systemName: "paperplane.fill")
                                .font(.body.weight(.semibold))
                                .padding(10)
                                .background(Circle().fill(GrColors.primary))
                                .foregroundStyle(.white)
                        }
                        .disabled(vm.sending || draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    }
                    .padding()
                } else if !vm.open {
                    Text(L10n.tr("chat_closed", lang: lang))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                        .padding()
                }
            }
            .navigationTitle(L10n.tr("chat_title", lang: lang))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.tr("common_close", lang: lang)) { dismiss() }
                }
            }
            .task {
                await vm.setSheetVisible(true)
            }
            .onDisappear {
                Task { await vm.setSheetVisible(false) }
            }
        }
    }

    private func presetChip(_ text: String) -> some View {
        Button(text) {
            Task { _ = await vm.send(text) }
        }
        .font(GrTypography.caption)
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(Capsule().fill(GrColors.surface2))
        .overlay(Capsule().stroke(GrColors.border, lineWidth: 1))
    }

    @ViewBuilder
    private func chatBubble(msg: ChatMessageDto) -> some View {
        VStack(alignment: msg.mine ? .trailing : .leading, spacing: 4) {
            if !msg.mine, let sender = msg.name {
                Text(sender)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(GrColors.muted)
            }
            Text(msg.body)
                .font(GrTypography.body)
                .foregroundStyle(msg.mine ? GrColors.primaryInk : GrColors.text)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .fill(msg.mine ? GrColors.primary : GrColors.surface2)
                )
            HStack(spacing: 6) {
                Text(KlTime.displayTime(msg.createdAt))
                    .font(.system(size: 11))
                    .foregroundStyle(GrColors.muted)
                if !msg.reactions.isEmpty {
                    ForEach(msg.reactions, id: \.emoji) { reaction in
                        Button {
                            Task { await vm.react(messageId: msg.id, emoji: reaction.emoji) }
                        } label: {
                            Text("\(reaction.emoji) \(reaction.count)")
                                .font(.system(size: 12))
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Capsule().fill(reaction.mine ? GrColors.primarySoft : GrColors.surface2))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: msg.mine ? .trailing : .leading)
        .contextMenu {
            ForEach(vm.reactionsAllowed, id: \.self) { emoji in
                Button(emoji) {
                    Task { await vm.react(messageId: msg.id, emoji: emoji) }
                }
            }
        }
    }
}
