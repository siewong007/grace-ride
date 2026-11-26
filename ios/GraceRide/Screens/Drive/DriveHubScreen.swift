import CoreLocation
import GraceRideKit
import MapKit
import SwiftUI

public struct DriveHubScreen: View {
    public let container: AppContainer
    public let locationManager: LiveTripLocationManager
    public let onNavigate: (AppRoute) -> Void

    @State private var vm: DriveHubViewModel
    @State private var navTarget: (title: String, lat: Double?, lng: Double?, address: String?)?
    @State private var showNavSheet = false

    public init(
        container: AppContainer,
        locationManager: LiveTripLocationManager,
        onNavigate: @escaping (AppRoute) -> Void
    ) {
        self.container = container
        self.locationManager = locationManager
        self.onNavigate = onNavigate
        self._vm = State(initialValue: DriveHubViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if vm.accessDenied {
                    EmptyStateCard(
                        systemImage: "steeringwheel.slash",
                        title: L10n.tr("role_driver_access_required", lang: lang)
                    )
                } else {
                    if vm.offline {
                        CalloutBanner(
                            tone: .warning,
                            icon: "wifi.slash",
                            text: L10n.tr("network_offline_live", lang: lang)
                        )
                    }

                    if vm.claimedNotice {
                        CalloutBanner(
                            tone: .success,
                            icon: "checkmark.circle.fill",
                            text: L10n.tr("drive_claimed_msg", lang: lang),
                            actionTitle: L10n.tr("common_close", lang: lang)
                        ) {
                            vm.clearClaimedNotice()
                        }
                    }

                    if let err = vm.error, !err.isEmpty {
                        ErrorBannerView(message: err, onDismiss: { vm.clearError() })
                    }

                    Picker("", selection: $vm.tab) {
                        Text(L10n.tr("drive_tab_requests", lang: lang)).tag(0)
                        Text(L10n.tr("drive_tab_schedule", lang: lang)).tag(1)
                        Text(L10n.tr("drive_tab_route", lang: lang)).tag(2)
                    }
                    .pickerStyle(.segmented)

                    if vm.loading && vm.requests.isEmpty && vm.schedule.isEmpty {
                        LoadingSkeletonView(rows: 3)
                    } else {
                        switch vm.tab {
                        case 0:
                            requestsTab(lang: lang)
                        case 1:
                            scheduleTab(lang: lang)
                        default:
                            routeTab(lang: lang)
                        }
                    }
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("drive_hub_title", lang: lang))
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    onNavigate(.driverSettings)
                } label: {
                    Image(systemName: "slider.horizontal.3")
                }
                .accessibilityLabel(L10n.tr("driver_settings_title", lang: lang))
            }
        }
        .refreshable {
            await vm.refresh()
        }
        .task {
            await vm.load(first: true)
        }
        .background(
            MapsDestinationChooser(
                title: navTarget?.title ?? "",
                lat: navTarget?.lat,
                lng: navTarget?.lng,
                address: navTarget?.address,
                lang: lang,
                isPresented: $showNavSheet
            )
        )
    }

    @ViewBuilder
    private func requestsTab(lang: String) -> some View {
        Toggle(isOn: Binding(
            get: { vm.mineAreasOnly },
            set: { _ in Task { await vm.toggleMineAreas() } }
        )) {
            Text(L10n.tr("drive_filter_my_areas", lang: lang))
                .font(GrTypography.subheadline)
        }
        .toggleStyle(.switch)
        .tint(GrColors.primary)
        .grCard()

        if vm.requests.isEmpty {
            EmptyStateCard(
                systemImage: "car.badge.gearshape",
                title: L10n.tr("drive_requests_empty", lang: lang)
            )
        } else {
            ForEach(vm.requests) { ride in
                VStack(alignment: .leading, spacing: 10) {
                    HStack {
                        Text(KlTime.display(ride.pickupAt, lang: lang))
                            .font(GrTypography.bodyMedium)
                        Spacer()
                        Text("\(ride.seats)")
                            .font(GrTypography.badge)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 3)
                            .background(Capsule().fill(GrColors.primarySoft))
                    }
                    Text("\(ride.area.name) → \(ride.destination)")
                        .font(GrTypography.subheadline)
                        .foregroundStyle(GrColors.text)
                    Text(ride.passenger.name)
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                    if !ride.assistance.isEmpty && ride.assistance != "none" {
                        Text(ride.assistance)
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.warning)
                    }
                    HStack {
                        Button(L10n.tr("drive_claim", lang: lang)) {
                            Task { await vm.claim(rideId: ride.id) }
                        }
                        .buttonStyle(GrPrimaryButtonStyle())
                        .disabled(vm.busyRideId == ride.id)

                        Spacer()

                        Button(L10n.tr("ride_detail_title", lang: lang)) {
                            onNavigate(.rideDetail(id: ride.id, justBooked: false))
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                    }
                }
                .grCard()
            }
        }
    }

    @ViewBuilder
    private func scheduleTab(lang: String) -> some View {
        if vm.schedule.isEmpty {
            EmptyStateCard(
                systemImage: "calendar",
                title: L10n.tr("drive_schedule_empty", lang: lang)
            )
        } else {
            ForEach(vm.schedule) { ride in
                Button {
                    onNavigate(.rideDetail(id: ride.id, justBooked: false))
                } label: {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack {
                            Text(KlTime.display(ride.pickupAt, lang: lang))
                                .font(GrTypography.bodyMedium)
                                .foregroundStyle(GrColors.text)
                            Spacer()
                            RideStatusBadge(status: ride.status, lang: lang)
                        }
                        Text("\(ride.passenger.name) · \(ride.area.name)")
                            .font(GrTypography.subheadline)
                            .foregroundStyle(GrColors.text)
                        Text(ride.destination)
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.muted)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .grCard()
                }
                .buttonStyle(.plain)
            }
        }
    }

    @ViewBuilder
    private func routeTab(lang: String) -> some View {
        if let route = vm.route {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(L10n.tr("drive_route_onboard", lang: lang, route.onboard))
                            .font(GrTypography.titleSmall)
                        Text(route.live ? L10n.tr("live_location_driver_active", lang: lang) : L10n.tr("live_location_driver_idle", lang: lang))
                            .font(GrTypography.caption)
                            .foregroundStyle(route.live ? GrColors.success : GrColors.muted)
                    }
                    Spacer()
                    if LocationServicePolicy.canShareDriverLocation(authState: container.auth.state, route: route) {
                        Button(locationManager.isSharing ? L10n.tr("live_location_stop", lang: lang) : L10n.tr("live_location_start", lang: lang)) {
                            Task {
                                if locationManager.isSharing {
                                    await locationManager.stopDriverSharing(notifyServer: true)
                                } else {
                                    try? await locationManager.startDriverSharing()
                                }
                            }
                        }
                        .buttonStyle(GrPrimaryButtonStyle(destructive: locationManager.isSharing))
                    }
                }
                .grCard()

                ForEach(route.stops) { stop in
                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(stop.passenger)
                                .font(GrTypography.bodyMedium)
                            Text(stop.address)
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                            if let eta = stop.eta {
                                Text("ETA \(KlTime.displayTime(eta))")
                                    .font(GrTypography.label)
                                    .foregroundStyle(GrColors.primary)
                            }
                        }
                        Spacer()
                        Button(L10n.tr("drive_route_navigate", lang: lang)) {
                            navTarget = (stop.passenger, stop.lat, stop.lng, stop.address)
                            showNavSheet = true
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                    }
                    .grCard()
                }

                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(L10n.tr("drive_route_destination", lang: lang, route.destination.name))
                            .font(GrTypography.bodyMedium)
                        if route.approximate {
                            Text(L10n.tr("drive_route_approx", lang: lang))
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                        }
                    }
                    Spacer()
                    Button(L10n.tr("common_open_maps", lang: lang)) {
                        navTarget = (route.destination.name, route.destination.lat, route.destination.lng, route.destination.name)
                        showNavSheet = true
                    }
                    .buttonStyle(GrOutlineButtonStyle())
                }
                .grCard()
            }
        } else {
            EmptyStateCard(
                systemImage: "map",
                title: L10n.tr("drive_route_empty", lang: lang)
            )
        }
    }
}
