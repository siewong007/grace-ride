import GraceRideKit
import SwiftUI

private enum MainTab: Hashable {
    case home, rides, events, profile
}

private struct IdentifiableURL: Identifiable {
    let id = UUID()
    let url: URL
}

public struct GraceRideAppRoot: View {
    @Bindable public var container: AppContainer
    public let locationManager: LiveTripLocationManager
    @Binding public var pendingLinkPath: String?

    public init(
        container: AppContainer,
        locationManager: LiveTripLocationManager,
        pendingLinkPath: Binding<String?>
    ) {
        self.container = container
        self.locationManager = locationManager
        self._pendingLinkPath = pendingLinkPath
    }

    public var body: some View {
        Group {
            switch container.auth.state {
            case .loading:
                GraceRideSplashOverlay(lang: container.lang)
            case .signedOut:
                LoginScreen(container: container)
            case .locked(let me):
                UnlockScreen(container: container, me: me)
            case .pending(let me):
                PendingScreen(container: container, me: me)
            case .signedIn(let me):
                if !me.termsAccepted {
                    TermsScreen(container: container, me: me)
                } else {
                    MainShellView(
                        container: container,
                        me: me,
                        locationManager: locationManager,
                        pendingLinkPath: $pendingLinkPath
                    )
                }
            }
        }
        .preferredColorScheme(container.themeMode.colorScheme)
        .environment(\.locale, Locale.grLocale(for: container.lang))
        .task {
            if case .loading = container.auth.state {
                try? await container.auth.refresh(throwOnError: false)
            }
            container.realtimeStream.start()
        }
        .onOpenURL { url in
            if let path = NotifLinkRouter.extractPathFromIntentUri(
                url.absoluteString,
                extraTrustedHosts: Set([container.baseURL.host?.lowercased()].compactMap { $0 })
            ) {
                pendingLinkPath = path
            }
        }
    }
}

private struct MainShellView: View {
    @Bindable var container: AppContainer
    let me: Me
    let locationManager: LiveTripLocationManager
    @Binding var pendingLinkPath: String?

    @Environment(\.horizontalSizeClass) private var sizeClass
    @Environment(\.colorScheme) private var colorScheme
    @Namespace private var tabBarNamespace
    @State private var selectedTab: MainTab = .home
    @State private var homeStack: [AppRoute] = []
    @State private var ridesStack: [AppRoute] = []
    @State private var eventsStack: [AppRoute] = []
    @State private var profileStack: [AppRoute] = []
    @State private var webSheetItem: IdentifiableURL?

    private var visibleTabs: [MainTab] {
        var tabs: [MainTab] = [.home]
        if me.can("rides") { tabs.append(.rides) }
        if me.can("events") { tabs.append(.events) }
        tabs.append(.profile)
        return tabs
    }

    private var isOnTabRoot: Bool {
        switch selectedTab {
        case .home: return homeStack.isEmpty
        case .rides: return ridesStack.isEmpty
        case .events: return eventsStack.isEmpty
        case .profile: return profileStack.isEmpty
        }
    }

    var body: some View {
        let lang = container.lang

        VStack(spacing: 0) {
            if !container.networkMonitor.isOnline {
                HStack(spacing: 8) {
                    Image(systemName: "wifi.slash")
                    Text(L10n.tr("common_offline", lang: lang))
                        .font(GrTypography.caption.weight(.semibold))
                    Spacer()
                }
                .foregroundStyle(GrColors.warning)
                .padding(.horizontal, 14)
                .padding(.vertical, 8)
                .background(GrColors.warningSoft)
            }

            if sizeClass == .regular {
                splitShell(lang: lang)
            } else {
                tabShell(lang: lang)
            }
        }
        .sheet(item: $webSheetItem) { item in
            SafariWebSheet(url: item.url)
                .ignoresSafeArea()
        }
        .onChange(of: pendingLinkPath) { _, newPath in
            guard let newPath else { return }
            pendingLinkPath = nil
            handleLinkPath(newPath)
        }
        .onAppear {
            if let initial = pendingLinkPath {
                pendingLinkPath = nil
                handleLinkPath(initial)
            }
        }
    }

    @ViewBuilder
    private func tabShell(lang: String) -> some View {
        TabView(selection: $selectedTab) {
            NavigationStack(path: $homeStack) {
                HomeScreen(
                    container: container,
                    onNavigate: { push($0, on: .home) },
                    onOpenNotifications: { push(.notifications, on: .home) }
                )
                .navigationDestination(for: AppRoute.self) { route in
                    destinationView(for: route, tab: .home)
                        .toolbar(.hidden, for: .tabBar)
                }
                .toolbar(.hidden, for: .tabBar)
            }
            .toolbar(.hidden, for: .tabBar)
            .tag(MainTab.home)

            if me.can("rides") {
                NavigationStack(path: $ridesStack) {
                    RidesScreen(container: container) {
                        push($0, on: .rides)
                    }
                    .navigationDestination(for: AppRoute.self) { route in
                        destinationView(for: route, tab: .rides)
                            .toolbar(.hidden, for: .tabBar)
                    }
                    .toolbar(.hidden, for: .tabBar)
                }
                .toolbar(.hidden, for: .tabBar)
                .tag(MainTab.rides)
            }

            if me.can("events") {
                NavigationStack(path: $eventsStack) {
                    EventsScreen(
                        container: container,
                        onNavigate: { push($0, on: .events) },
                        onOpenWeb: openWebPath
                    )
                    .navigationDestination(for: AppRoute.self) { route in
                        destinationView(for: route, tab: .events)
                            .toolbar(.hidden, for: .tabBar)
                    }
                    .toolbar(.hidden, for: .tabBar)
                }
                .toolbar(.hidden, for: .tabBar)
                .tag(MainTab.events)
            }

            NavigationStack(path: $profileStack) {
                ProfileScreen(
                    container: container,
                    onNavigate: { push($0, on: .profile) },
                    onOpenWeb: openWebPath
                )
                .navigationDestination(for: AppRoute.self) { route in
                    destinationView(for: route, tab: .profile)
                        .toolbar(.hidden, for: .tabBar)
                }
                .toolbar(.hidden, for: .tabBar)
            }
            .toolbar(.hidden, for: .tabBar)
            .tag(MainTab.profile)
        }
        .tint(GrColors.primary)
        .safeAreaInset(edge: .bottom, spacing: 0) {
            if isOnTabRoot {
                FloatingGlassTabBar(
                    tabs: visibleTabs,
                    selectedTab: $selectedTab,
                    me: me,
                    container: container,
                    lang: lang,
                    namespace: tabBarNamespace
                )
                .padding(.horizontal, 24)
                .padding(.top, 4)
                .padding(.bottom, 8)
                .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(.spring(response: 0.32, dampingFraction: 0.82), value: isOnTabRoot)
        .animation(.spring(response: 0.32, dampingFraction: 0.78), value: selectedTab)
    }

    @ViewBuilder
    private func splitShell(lang: String) -> some View {
        NavigationSplitView {
            List(selection: Binding<MainTab?>(
                get: { selectedTab },
                set: { if let t = $0 { selectedTab = t } }
            )) {
                Label(L10n.tr("nav_home", lang: lang), systemImage: "house.fill").tag(MainTab.home)
                if me.can("rides") {
                    Label(L10n.tr("nav_rides", lang: lang), systemImage: "car.fill").tag(MainTab.rides)
                }
                if me.can("events") {
                    Label(L10n.tr("nav_events", lang: lang), systemImage: "calendar").tag(MainTab.events)
                }
                Label(L10n.tr("nav_profile", lang: lang), systemImage: "person.crop.circle.fill").tag(MainTab.profile)
            }
            .navigationTitle("Grace Ride")
        } detail: {
            switch selectedTab {
            case .home:
                NavigationStack(path: $homeStack) {
                    HomeScreen(
                        container: container,
                        onNavigate: { push($0, on: .home) },
                        onOpenNotifications: { push(.notifications, on: .home) }
                    )
                    .navigationDestination(for: AppRoute.self) { destinationView(for: $0, tab: .home) }
                }
            case .rides:
                NavigationStack(path: $ridesStack) {
                    RidesScreen(container: container) { push($0, on: .rides) }
                        .navigationDestination(for: AppRoute.self) { destinationView(for: $0, tab: .rides) }
                }
            case .events:
                NavigationStack(path: $eventsStack) {
                    EventsScreen(container: container, onNavigate: { push($0, on: .events) }, onOpenWeb: openWebPath)
                        .navigationDestination(for: AppRoute.self) { destinationView(for: $0, tab: .events) }
                }
            case .profile:
                NavigationStack(path: $profileStack) {
                    ProfileScreen(container: container, onNavigate: { push($0, on: .profile) }, onOpenWeb: openWebPath)
                        .navigationDestination(for: AppRoute.self) { destinationView(for: $0, tab: .profile) }
                }
            }
        }
    }

    @ViewBuilder
    private func destinationView(for route: AppRoute, tab: MainTab) -> some View {
        switch route {
        case .home:
            HomeScreen(
                container: container,
                onNavigate: { push($0, on: tab) },
                onOpenNotifications: { push(.notifications, on: tab) }
            )
        case .rides:
            RidesScreen(container: container) { push($0, on: tab) }
        case .history:
            HistoryScreen(container: container) { push($0, on: tab) }
        case .rideDetail(let id, let justBooked):
            RideDetailScreen(container: container, locationManager: locationManager, rideId: id, justBooked: justBooked) {
                push($0, on: tab)
            }
        case .rideNew(let eventId, let proxy):
            RideNewScreen(
                container: container,
                eventId: eventId,
                initialProxy: proxy,
                onCreated: { newId in push(.rideDetail(id: newId, justBooked: true), on: tab) },
                onOpenEvent: { eid in push(.eventDetail(id: eid, checkinCode: nil), on: tab) },
                onBrowseEvents: {
                    selectedTab = .events
                    eventsStack = []
                }
            )
        case .events:
            EventsScreen(container: container, onNavigate: { push($0, on: tab) }, onOpenWeb: openWebPath)
        case .eventDetail(let id, let checkinCode):
            EventDetailScreen(container: container, eventId: id, checkinCode: checkinCode, onNavigate: { push($0, on: tab) }, onOpenWeb: openWebPath)
        case .costs:
            CostsScreen(container: container)
        case .notifications:
            NotificationsScreen(container: container) { link in
                handleLinkPath(link)
            }
        case .driveHub:
            DriveHubScreen(container: container, locationManager: locationManager) { push($0, on: tab) }
        case .profile:
            ProfileScreen(container: container, onNavigate: { push($0, on: tab) }, onOpenWeb: openWebPath)
        case .driverSettings:
            DriverSettingsScreen(container: container)
        case .proxyRiders:
            ProxyRidersScreen(container: container)
        case .savedAddresses:
            SavedAddressesScreen(container: container, locationManager: locationManager)
        case .studio(let shape, let color, let face):
            GrokBotStudioScreen(
                container: container,
                initialSpec: GrokBotSpec(
                    shape: shape ?? "circle",
                    face: face ?? "neutral",
                    color: color ?? "blue"
                )
            )
        }
    }

    private func push(_ route: AppRoute, on tab: MainTab) {
        guard Routes.isPermitted(route, me: me) else { return }
        switch route {
        case .home:
            selectedTab = .home
            homeStack = []
        case .rides where homeStack.isEmpty && selectedTab == .rides:
            ridesStack = []
        case .events where selectedTab == .events && eventsStack.isEmpty:
            eventsStack = []
        case .profile where selectedTab == .profile && profileStack.isEmpty:
            profileStack = []
        default:
            switch tab {
            case .home: homeStack.append(route)
            case .rides: ridesStack.append(route)
            case .events: eventsStack.append(route)
            case .profile: profileStack.append(route)
            }
        }
    }

    private func openWebPath(_ path: String) {
        guard Routes.isWebPathPermitted(path, me: me),
              let url = container.webURL(path: path) else {
            return
        }
        webSheetItem = IdentifiableURL(url: url)
    }

    private func handleLinkPath(_ rawPath: String) {
        switch NotifLinkRouter.resolveFor(rawPath, me: me) {
        case .native(let routeStr):
            guard let typed = Routes.fromLegacyRoute(routeStr), Routes.isPermitted(typed, me: me) else { return }
            switch typed {
            case .home:
                selectedTab = .home
                homeStack = []
            case .rides:
                selectedTab = .rides
                ridesStack = []
            case .events:
                selectedTab = .events
                eventsStack = []
            case .profile:
                selectedTab = .profile
                profileStack = []
            default:
                selectedTab = .home
                homeStack = [typed]
            }
        case .web(let path):
            openWebPath(path)
        case .none:
            break
        }
    }
}

private struct FloatingGlassTabBar: View {
    let tabs: [MainTab]
    @Binding var selectedTab: MainTab
    let me: Me
    let container: AppContainer
    let lang: String
    let namespace: Namespace.ID

    @Environment(\.colorScheme) private var colorScheme

    private var activeInk: Color {
        colorScheme == .dark ? .white : Color(hex: 0x141517)
    }

    private var inactiveInk: Color {
        colorScheme == .dark ? .white.opacity(0.68) : Color(hex: 0x141517).opacity(0.64)
    }

    var body: some View {
        HStack(spacing: 0) {
            ForEach(tabs, id: \.self) { tab in
                tabButton(for: tab)
            }
        }
        .padding(5)
        .frame(maxWidth: 360)
        .frame(height: 56)
        .background {
            ZStack {
                Capsule().fill(.ultraThinMaterial)
                Capsule().fill(
                    colorScheme == .dark
                        ? Color.black.opacity(0.38)
                        : Color.white.opacity(0.30)
                )
            }
        }
        .overlay {
            Capsule()
                .strokeBorder(
                    LinearGradient(
                        colors: colorScheme == .dark
                            ? [.white.opacity(0.24), .white.opacity(0.06)]
                            : [.white.opacity(0.92), Color.black.opacity(0.08)],
                        startPoint: .top,
                        endPoint: .bottom
                    ),
                    lineWidth: 1
                )
        }
        .shadow(
            color: .black.opacity(colorScheme == .dark ? 0.42 : 0.14),
            radius: 16,
            x: 0,
            y: 8
        )
    }

    private func tabTitle(for tab: MainTab) -> String {
        switch tab {
        case .home: return L10n.tr("nav_home", lang: lang)
        case .rides: return L10n.tr("nav_rides", lang: lang)
        case .events: return L10n.tr("nav_events", lang: lang)
        case .profile: return L10n.tr("nav_profile", lang: lang)
        }
    }

    @ViewBuilder
    private func tabButton(for tab: MainTab) -> some View {
        let isSelected = selectedTab == tab
        Button {
            Haptics.selection()
            withAnimation(.spring(response: 0.32, dampingFraction: 0.78)) {
                selectedTab = tab
            }
        } label: {
            ZStack {
                if isSelected {
                    Capsule()
                        .fill(
                            colorScheme == .dark
                                ? Color.white.opacity(0.17)
                                : Color.black.opacity(0.08)
                        )
                        .overlay {
                            Capsule()
                                .strokeBorder(
                                    Color.white.opacity(colorScheme == .dark ? 0.14 : 0.60),
                                    lineWidth: 0.5
                                )
                        }
                        .matchedGeometryEffect(id: "activeTabPill", in: namespace)
                }

                Group {
                    if tab == .profile {
                        AvatarView(
                            name: me.name,
                            photoFileId: me.photoFileId,
                            avatarColor: me.avatarColor,
                            avatarIcon: me.avatarIcon,
                            size: 26,
                            container: container,
                            animated: false
                        )
                        .padding(2)
                        .overlay {
                            if isSelected {
                                Circle()
                                    .strokeBorder(activeInk, lineWidth: 2)
                            }
                        }
                    } else {
                        tabIcon(for: tab, selected: isSelected)
                            .font(.system(size: 19, weight: isSelected ? .semibold : .regular))
                            .foregroundStyle(isSelected ? activeInk : inactiveInk)
                    }
                }
                .scaleEffect(isSelected ? 1.08 : 1.0)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(tabTitle(for: tab))
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }

    @ViewBuilder
    private func tabIcon(for tab: MainTab, selected: Bool) -> some View {
        switch tab {
        case .home:
            Image(systemName: selected ? "house.fill" : "house")
        case .rides:
            Image(systemName: selected ? "car.fill" : "car")
        case .events:
            Image(systemName: "calendar")
                .fontWeight(selected ? .bold : .regular)
        case .profile:
            Image(systemName: selected ? "person.crop.circle.fill" : "person.crop.circle")
        }
    }
}
