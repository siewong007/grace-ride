import GraceRideKit
import SwiftUI

public struct HomeScreen: View {
    public let container: AppContainer
    public let onNavigate: (AppRoute) -> Void
    public let onOpenNotifications: () -> Void

    @State private var vm: HomeViewModel

    public init(
        container: AppContainer,
        onNavigate: @escaping (AppRoute) -> Void,
        onOpenNotifications: @escaping () -> Void
    ) {
        self.container = container
        self.onNavigate = onNavigate
        self.onOpenNotifications = onOpenNotifications
        self._vm = State(initialValue: HomeViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang
        let me = vm.me ?? container.auth.me

        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                headerSection(me: me, lang: lang)

                if vm.offline {
                    let cachedLabel = vm.offlineSnapshotAt.map { KlTime.formatEpochMillis($0) } ?? ""
                    CalloutBanner(
                        tone: .warning,
                        icon: "wifi.slash",
                        text: cachedLabel.isEmpty
                            ? L10n.tr("network_offline_live", lang: lang)
                            : L10n.tr("drive_offline_snapshot", lang: lang, cachedLabel)
                    )
                }

                if let err = vm.error, !err.isEmpty {
                    ErrorBannerView(
                        message: err,
                        retryTitle: L10n.tr("common_retry", lang: lang),
                        onRetry: { Task { await vm.refresh() } }
                    )
                }

                if vm.loading && me == nil {
                    LoadingSkeletonView(rows: 3)
                } else {
                    if !vm.announcements.isEmpty {
                        announcementsSection(lang: lang)
                    }

                    if vm.unpaidShareCount > 0 && me?.can("costs") == true {
                        unpaidBanner(lang: lang)
                    }

                    if me?.can("rides") == true {
                        nextRideSection(lang: lang)
                    }

                    if me?.can("drive") == true {
                        drivingSection(lang: lang)
                    }

                    if me?.can("events") == true {
                        upcomingEventsSection(lang: lang)
                    }

                    quickActionsSection(me: me, lang: lang)
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("nav_home", lang: lang))
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button(action: onOpenNotifications) {
                    ZStack(alignment: .topTrailing) {
                        Image(systemName: "bell")
                            .font(.body.weight(.semibold))
                        if (me?.unread ?? 0) > 0 {
                            Circle()
                                .fill(GrColors.brandRed)
                                .frame(width: 9, height: 9)
                                .offset(x: 3, y: -3)
                        }
                    }
                }
                .accessibilityLabel(L10n.tr("notif_title", lang: lang))
            }
        }
        .refreshable {
            await vm.refresh()
        }
        .task {
            await vm.loadInitial()
        }
    }

    @ViewBuilder
    private func headerSection(me: Me?, lang: String) -> some View {
        HStack(spacing: 12) {
            GraceRideBrandMark(size: 40, animated: false)
            VStack(alignment: .leading, spacing: 2) {
                Text(L10n.tr("home_greeting", lang: lang, me?.name ?? ""))
                    .font(GrTypography.titleMedium)
                    .foregroundStyle(GrColors.text)
                if let role = me?.roles.first {
                    Text(L10n.permissionRole(role, lang: lang))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                }
            }
            Spacer()
            if let me {
                Button {
                    onNavigate(.profile)
                } label: {
                    AvatarView(
                        name: me.name,
                        photoFileId: me.photoFileId,
                        avatarColor: me.avatarColor,
                        avatarIcon: me.avatarIcon,
                        size: 42,
                        container: container
                    )
                }
                .buttonStyle(.plain)
            }
        }
    }

    @ViewBuilder
    private func announcementsSection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(L10n.tr("home_announcements", lang: lang))
                .font(GrTypography.titleSmall)
                .foregroundStyle(GrColors.text)
            ForEach(vm.announcements.prefix(3)) { item in
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 6) {
                        if item.pinned {
                            Image(systemName: "pin.fill")
                                .font(.caption)
                                .foregroundStyle(GrColors.primary)
                        }
                        Text(item.title)
                            .font(GrTypography.bodyMedium)
                            .foregroundStyle(GrColors.text)
                        Spacer()
                    }
                    if !item.body.isEmpty {
                        Text(item.body)
                            .font(GrTypography.subheadline)
                            .foregroundStyle(GrColors.muted)
                            .lineLimit(3)
                    }
                }
                .grCard(background: GrColors.surface2)
            }
        }
    }

    @ViewBuilder
    private func unpaidBanner(lang: String) -> some View {
        let total = vm.unpaidShares.reduce(Int64(0)) { $0 + $1.amountCents }
        Button {
            onNavigate(.costs)
        } label: {
            HStack(spacing: 12) {
                Image(systemName: "creditcard.fill")
                    .font(.title3)
                    .foregroundStyle(GrColors.warning)
                VStack(alignment: .leading, spacing: 2) {
                    Text(L10n.tr("home_unpaid_shares", lang: lang))
                        .font(GrTypography.bodyMedium)
                        .foregroundStyle(GrColors.text)
                    Text("\(L10n.tr("costs_items_due", lang: lang, Int64(vm.unpaidShareCount))) · \(Money.rm(total))")
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.warning)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(GrColors.muted)
            }
            .padding(14)
            .background(GrColors.warningSoft)
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .stroke(GrColors.warning.opacity(0.35), lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }

    @ViewBuilder
    private func nextRideSection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(L10n.tr("home_next_ride", lang: lang))
                .font(GrTypography.titleSmall)
                .foregroundStyle(GrColors.text)

            if let ride = vm.nextRide {
                Button {
                    onNavigate(.rideDetail(id: ride.id, justBooked: false))
                } label: {
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
                            .foregroundStyle(GrColors.muted)
                            .lineLimit(2)
                        if Readiness.shouldShowPrompt(ride: ride) {
                            Label(L10n.tr("home_action_readiness", lang: lang), systemImage: "checkmark.circle.fill")
                                .font(GrTypography.label)
                                .foregroundStyle(GrColors.primary)
                        } else if ride.status == "pending_passenger" {
                            Label(L10n.tr("home_action_respond", lang: lang), systemImage: "hand.raised.fill")
                                .font(GrTypography.label)
                                .foregroundStyle(GrColors.warning)
                        }
                    }
                    .grCard()
                }
                .buttonStyle(.plain)
            } else {
                EmptyStateCard(
                    systemImage: "car.fill",
                    title: L10n.tr("home_no_ride", lang: lang),
                    actionTitle: L10n.tr("home_book_ride", lang: lang)
                ) {
                    onNavigate(.rideNew(eventId: nil, proxy: false))
                }
            }
        }
    }

    @ViewBuilder
    private func drivingSection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(L10n.tr("home_today_driving", lang: lang))
                    .font(GrTypography.titleSmall)
                    .foregroundStyle(GrColors.text)
                Spacer()
                Button(L10n.tr("home_view_all", lang: lang)) {
                    onNavigate(.driveHub)
                }
                .font(GrTypography.label)
                .foregroundStyle(GrColors.primary)
            }

            if !vm.drivingToday.isEmpty {
                ForEach(vm.drivingToday.prefix(3)) { ride in
                    Button {
                        onNavigate(.rideDetail(id: ride.id, justBooked: false))
                    } label: {
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(L10n.tr("home_drive_for", lang: lang, ride.passenger.name))
                                    .font(GrTypography.bodyMedium)
                                    .foregroundStyle(GrColors.text)
                                Text("\(KlTime.displayTime(ride.pickupAt)) · \(ride.area.name)")
                                    .font(GrTypography.caption)
                                    .foregroundStyle(GrColors.muted)
                            }
                            Spacer()
                            RideStatusBadge(status: ride.status, lang: lang)
                        }
                        .grCard(background: GrColors.surface2)
                    }
                    .buttonStyle(.plain)
                }
            } else if let upcoming = vm.nextUpcomingDrive {
                Button {
                    onNavigate(.rideDetail(id: upcoming.id, justBooked: false))
                } label: {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(L10n.tr("home_next_drive_upcoming", lang: lang, KlTime.display(upcoming.pickupAt, lang: lang)))
                                .font(GrTypography.bodyMedium)
                                .foregroundStyle(GrColors.text)
                            Text("\(upcoming.passenger.name) · \(upcoming.area.name)")
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").foregroundStyle(GrColors.muted)
                    }
                    .grCard(background: GrColors.surface2)
                }
                .buttonStyle(.plain)
            } else {
                Text(L10n.tr("home_no_drive_today", lang: lang))
                    .font(GrTypography.subheadline)
                    .foregroundStyle(GrColors.muted)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .grCard()
            }
        }
    }

    @ViewBuilder
    private func upcomingEventsSection(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(L10n.tr("home_upcoming_events", lang: lang))
                    .font(GrTypography.titleSmall)
                    .foregroundStyle(GrColors.text)
                Spacer()
                Button(L10n.tr("home_view_all", lang: lang)) {
                    onNavigate(.events)
                }
                .font(GrTypography.label)
                .foregroundStyle(GrColors.primary)
            }

            if vm.events.isEmpty {
                Text(L10n.tr("events_empty", lang: lang))
                    .font(GrTypography.subheadline)
                    .foregroundStyle(GrColors.muted)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .grCard()
            } else {
                ForEach(vm.events.prefix(3)) { ev in
                    Button {
                        onNavigate(.eventDetail(id: ev.id, checkinCode: nil))
                    } label: {
                        HStack(spacing: 12) {
                            EventIconView(iconName: ev.icon, category: ev.kind, size: 40)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(ev.title)
                                    .font(GrTypography.bodyMedium)
                                    .foregroundStyle(GrColors.text)
                                    .lineLimit(1)
                                Text("\(KlTime.display(ev.startsAt, lang: lang)) · \(ev.location)")
                                    .font(GrTypography.caption)
                                    .foregroundStyle(GrColors.muted)
                                    .lineLimit(1)
                            }
                            Spacer()
                            Image(systemName: "chevron.right")
                                .font(.caption)
                                .foregroundStyle(GrColors.muted)
                        }
                        .grCard()
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    @ViewBuilder
    private func quickActionsSection(me: Me?, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("home_quick_actions", lang: lang))
                .font(GrTypography.titleSmall)
                .foregroundStyle(GrColors.text)

            LazyVGrid(columns: [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)], spacing: 10) {
                if me?.can("rides", act: "create") == true {
                    quickTile(icon: "plus.circle.fill", title: L10n.tr("home_book_ride", lang: lang)) {
                        onNavigate(.rideNew(eventId: nil, proxy: false))
                    }
                }
                if me?.can("rides") == true {
                    quickTile(icon: "car.2.fill", title: L10n.tr("home_quick_rides", lang: lang)) {
                        onNavigate(.rides)
                    }
                    quickTile(icon: "mappin.and.ellipse", title: L10n.tr("home_quick_addresses", lang: lang)) {
                        onNavigate(.savedAddresses)
                    }
                }
                if me?.can("costs") == true {
                    quickTile(icon: "banknote.fill", title: L10n.tr("home_quick_costs", lang: lang)) {
                        onNavigate(.costs)
                    }
                }
                if me?.can("drive") == true {
                    quickTile(icon: "steeringwheel", title: L10n.tr("home_quick_drive", lang: lang)) {
                        onNavigate(.driveHub)
                    }
                }
            }
        }
    }

    private func quickTile(icon: String, title: String, action: @escaping () -> Void) -> some View {
        Button {
            Haptics.selection()
            action()
        } label: {
            HStack(spacing: 10) {
                Image(systemName: icon)
                    .font(.title3)
                    .foregroundStyle(GrColors.primary)
                Text(title)
                    .font(GrTypography.label)
                    .foregroundStyle(GrColors.text)
                    .lineLimit(2)
                    .multilineTextAlignment(.leading)
                Spacer(minLength: 0)
            }
            .padding(12)
            .frame(maxWidth: .infinity, minHeight: 56)
            .background(GrColors.surface2)
            .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .stroke(GrColors.border, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }
}
