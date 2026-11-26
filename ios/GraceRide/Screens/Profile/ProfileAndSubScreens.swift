import GraceRideKit
import SwiftUI
import UserNotifications

public struct ProfileScreen: View {
    @Bindable public var container: AppContainer
    public let onNavigate: (AppRoute) -> Void
    public let onOpenWeb: (String) -> Void

    @State private var vm: ProfileViewModel
    @State private var passkeyManager = PasskeyAuthManager()

    @State private var showEditProfile = false
    @State private var editName = ""
    @State private var editPhone = ""

    @State private var showAvatarEditor = false

    @State private var showNotifPrefsSheet = false
    @State private var notifAllowed = false

    @State private var confirmDeletePasskey: PasskeyInfo?
    @State private var confirmLogoutAll = false

    public init(
        container: AppContainer,
        onNavigate: @escaping (AppRoute) -> Void,
        onOpenWeb: @escaping (String) -> Void
    ) {
        self.container = container
        self.onNavigate = onNavigate
        self.onOpenWeb = onOpenWeb
        self._vm = State(initialValue: ProfileViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang
        let me = vm.me ?? container.auth.me

        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                if let msg = vm.message {
                    CalloutBanner(tone: .success, icon: "checkmark.circle.fill", text: L10n.tr("profile_saved", lang: lang), actionTitle: L10n.tr("common_close", lang: lang)) {
                        vm.clearMessage()
                    }
                }
                if let err = vm.error {
                    ErrorBannerView(message: err, onDismiss: { vm.clearError() })
                }

                if let me {
                    headerCard(me: me, lang: lang)
                    personalInfoCard(me: me, lang: lang)
                    ridesAndDirectoryCard(me: me, lang: lang)
                    if !vm.badges.isEmpty && !me.badgesOptOut {
                        badgesCard(lang: lang)
                    }
                    preferencesCard(me: me, lang: lang)
                    securityCard(me: me, lang: lang)
                    if Routes.canUseStaffWeb(me) {
                        staffWebCard(me: me, lang: lang)
                    }
                    moreAndSessionCard(lang: lang)
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("profile_title", lang: lang))
        .refreshable {
            await loadAll()
        }
        .task {
            await loadAll()
        }
        .sheet(isPresented: $showEditProfile) {
            NavigationStack {
                Form {
                    TextField(L10n.tr("profile_name", lang: lang), text: $editName)
                    TextField(L10n.tr("profile_phone_placeholder", lang: lang), text: $editPhone)
                        .keyboardType(.phonePad)
                }
                .navigationTitle(L10n.tr("profile_edit", lang: lang))
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button(L10n.tr("common_cancel", lang: lang)) { showEditProfile = false }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button(L10n.tr("profile_save", lang: lang)) {
                            showEditProfile = false
                            Task { await vm.saveProfile(name: editName, phone: editPhone) }
                        }
                    }
                }
            }
        }
        .sheet(isPresented: $showAvatarEditor) {
            if let me {
                AvatarEditorSheet(container: container, vm: vm, me: me) { shape, color, face in
                    onNavigate(.studio(shape: shape, color: color, face: face))
                }
            }
        }
        .sheet(isPresented: $showNotifPrefsSheet) {
            NotificationPrefsSheet(
                prefs: vm.notifPrefs ?? NotificationPrefsDto(
                    pushOptIn: me?.pushOptIn ?? true,
                    whatsappOptIn: me?.whatsappOptIn ?? false,
                    readinessPromptMins: me?.readinessPromptMins ?? 30,
                    driverRemindMins: 30,
                    mutedCategories: [],
                    quietHoursStart: "22:00",
                    quietHoursEnd: "07:00",
                    hideLockDetails: false
                ),
                lang: lang,
                notifAllowed: notifAllowed,
                onRequestPushPermission: {
                    let center = UNUserNotificationCenter.current()
                    let granted = (try? await center.requestAuthorization(options: [.alert, .sound, .badge])) ?? false
                    notifAllowed = granted
                    if granted {
                        UIApplication.shared.registerForRemoteNotifications()
                    }
                },
                onSave: { req in
                    await vm.saveNotificationPrefs(req)
                }
            )
        }
        .alert(L10n.tr("profile_passkey_delete_title", lang: lang), isPresented: Binding(
            get: { confirmDeletePasskey != nil },
            set: { if !$0 { confirmDeletePasskey = nil } }
        )) {
            Button(L10n.tr("common_cancel", lang: lang), role: .cancel) {}
            Button(L10n.tr("common_delete", lang: lang), role: .destructive) {
                guard let pk = confirmDeletePasskey else { return }
                confirmDeletePasskey = nil
                Task {
                    await vm.deletePasskey(id: pk.id) { start in
                        try await passkeyManager.signIn(start: start, fallbackRpId: container.baseURL.host)
                    }
                }
            }
        } message: {
            if let pk = confirmDeletePasskey {
                Text(L10n.tr("profile_passkey_delete_confirm", lang: lang, pk.label.isEmpty ? L10n.tr("profile_passkey_default", lang: lang, pk.id) : pk.label))
            }
        }
        .alert(L10n.tr("profile_logout_all", lang: lang), isPresented: $confirmLogoutAll) {
            Button(L10n.tr("common_cancel", lang: lang), role: .cancel) {}
            Button(L10n.tr("common_confirm", lang: lang), role: .destructive) {
                Task { await vm.logoutAll() }
            }
        } message: {
            Text(L10n.tr("profile_logout_all_confirm", lang: lang))
        }
    }

    private func loadAll() async {
        await vm.refreshMe()
        async let p: () = vm.loadPasskeys()
        async let b: () = vm.loadBadges()
        async let n: () = vm.loadNotificationPrefs()
        _ = await (p, b, n)
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        notifAllowed = settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional
    }

    @ViewBuilder
    private func headerCard(me: Me, lang: String) -> some View {
        HStack(spacing: 14) {
            Button {
                showAvatarEditor = true
            } label: {
                AvatarView(
                    name: me.name,
                    photoFileId: me.photoFileId,
                    avatarColor: me.avatarColor,
                    avatarIcon: me.avatarIcon,
                    size: 64,
                    container: container
                )
            }
            .buttonStyle(.plain)

            VStack(alignment: .leading, spacing: 4) {
                Text(me.name)
                    .font(GrTypography.titleMedium)
                Text(me.email)
                    .font(GrTypography.caption)
                    .foregroundStyle(GrColors.muted)
                if !me.roles.isEmpty {
                    Text(me.roles.map { L10n.permissionRole($0, lang: lang) }.joined(separator: " · "))
                        .font(GrTypography.badge)
                        .foregroundStyle(GrColors.primary)
                }
            }
            Spacer()
        }
        .grCard()
    }

    @ViewBuilder
    private func personalInfoCard(me: Me, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(L10n.tr("profile_personal", lang: lang))
                    .font(GrTypography.label)
                    .foregroundStyle(GrColors.muted)
                Spacer()
                Button(L10n.tr("profile_edit", lang: lang)) {
                    editName = me.name
                    editPhone = me.phone
                    showEditProfile = true
                }
                .font(GrTypography.label)
            }
            rowItem(L10n.tr("profile_name", lang: lang), me.name)
            rowItem(L10n.tr("profile_phone", lang: lang), me.phone.isEmpty ? L10n.tr("profile_phone_not_set", lang: lang) : me.phone)
            Button(L10n.tr("profile_edit_avatar", lang: lang)) {
                showAvatarEditor = true
            }
            .font(GrTypography.caption.weight(.semibold))
            .foregroundStyle(GrColors.primary)
        }
        .grCard()
    }

    @ViewBuilder
    private func ridesAndDirectoryCard(me: Me, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(L10n.tr("profile_section_rides", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            if me.can("drive") {
                navRow(icon: "steeringwheel", title: L10n.tr("driver_settings_title", lang: lang)) {
                    onNavigate(.driverSettings)
                }
            }
            if me.can("rides") {
                navRow(icon: "person.2.fill", title: L10n.tr("profile_family", lang: lang)) {
                    onNavigate(.proxyRiders)
                }
                navRow(icon: "mappin.and.ellipse", title: L10n.tr("saved_addresses_title", lang: lang)) {
                    onNavigate(.savedAddresses)
                }
                navRow(icon: "clock.arrow.circlepath", title: L10n.tr("history_title", lang: lang)) {
                    onNavigate(.history)
                }
            }
            if me.can("costs") {
                navRow(icon: "banknote", title: L10n.tr("costs_title", lang: lang)) {
                    onNavigate(.costs)
                }
            }
            navRow(icon: "sparkles", title: L10n.tr("profile_open_grok_studio", lang: lang)) {
                let spec = GrokBotSpec.parse(me.avatarIcon)
                onNavigate(.studio(shape: spec?.shape, color: spec?.color, face: spec?.face))
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func badgesCard(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("profile_admin_reports", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 78))], spacing: 10) {
                ForEach(vm.badges.prefix(12)) { badge in
                    VStack(spacing: 4) {
                        BadgeIconView(badgeCode: badge.icon ?? badge.key, size: 42, animated: badge.earned)
                            .opacity(badge.earned ? 1.0 : 0.35)
                        Text(lang == "zh" ? (badge.nameZh ?? badge.name) : badge.name)
                            .font(.system(size: 11, weight: .medium))
                            .multilineTextAlignment(.center)
                            .lineLimit(2)
                    }
                }
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func preferencesCard(me: Me, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(L10n.tr("profile_preferences", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            HStack {
                Text(L10n.tr("profile_language", lang: lang))
                Spacer()
                Picker("", selection: Binding(
                    get: { container.lang },
                    set: { val in Task { await vm.setLang(val) } }
                )) {
                    Text(L10n.tr("profile_lang_en", lang: lang)).tag("en")
                    Text(L10n.tr("profile_lang_zh", lang: lang)).tag("zh")
                }
                .pickerStyle(.segmented)
                .frame(width: 150)
            }

            HStack {
                Text(L10n.tr("profile_theme", lang: lang))
                Spacer()
                Picker("", selection: $container.themeMode) {
                    Text(L10n.tr("profile_theme_system", lang: lang)).tag(ThemeMode.system)
                    Text(L10n.tr("profile_theme_light", lang: lang)).tag(ThemeMode.light)
                    Text(L10n.tr("profile_theme_dark", lang: lang)).tag(ThemeMode.dark)
                }
                .pickerStyle(.menu)
            }

            HStack {
                Text(L10n.tr("profile_auto_lock", lang: lang))
                Spacer()
                Picker("", selection: Binding(
                    get: { me.autoLockMins },
                    set: { mins in Task { await vm.setAutoLock(minutes: mins) } }
                )) {
                    Text(L10n.tr("profile_auto_lock_off", lang: lang)).tag(0)
                    Text(L10n.tr("profile_auto_lock_mins", lang: lang, Int64(15))).tag(15)
                    Text(L10n.tr("profile_auto_lock_mins", lang: lang, Int64(30))).tag(30)
                    Text(L10n.tr("profile_auto_lock_mins", lang: lang, Int64(60))).tag(60)
                }
                .pickerStyle(.menu)
            }

            navRow(icon: "bell.badge", title: L10n.tr("profile_notifications", lang: lang)) {
                showNotifPrefsSheet = true
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func securityCard(me: Me, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(L10n.tr("profile_passkeys", lang: lang))
                    .font(GrTypography.label)
                    .foregroundStyle(GrColors.muted)
                Spacer()
                Text(L10n.tr("profile_passkey_count", lang: lang, Int64(vm.passkeys.count)))
                    .font(GrTypography.caption)
                    .foregroundStyle(GrColors.muted)
            }

            ForEach(vm.passkeys) { pk in
                HStack {
                    Image(systemName: "person.badge.key.fill").foregroundStyle(GrColors.primary)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(pk.label.isEmpty ? L10n.tr("profile_passkey_default", lang: lang, pk.id) : pk.label)
                            .font(GrTypography.bodyMedium)
                        Text(KlTime.display(pk.createdAt, lang: lang))
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.muted)
                    }
                    Spacer()
                    Button(role: .destructive) {
                        confirmDeletePasskey = pk
                    } label: {
                        Image(systemName: "trash")
                    }
                    .grMinTouchTarget()
                }
            }

            Button {
                Task {
                    await vm.addPasskey(
                        performRegistration: { start in
                            try await passkeyManager.register(start: start, fallbackRpId: container.baseURL.host)
                        },
                        performReauth: { start in
                            try await passkeyManager.signIn(start: start, fallbackRpId: container.baseURL.host)
                        }
                    )
                }
            } label: {
                Label(L10n.tr("profile_add_passkey", lang: lang), systemImage: "plus.circle.fill")
            }
            .buttonStyle(GrOutlineButtonStyle())
            .disabled(vm.busy)
        }
        .grCard()
    }

    @ViewBuilder
    private func staffWebCard(me: Me, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(L10n.tr("profile_section_web", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)
            if me.hasStaffAccess {
                navRow(icon: "map", title: L10n.tr("profile_web_staff", lang: lang)) {
                    onOpenWeb("/admin/traffic")
                }
            }
            if Routes.canManageSettlements(me) {
                navRow(icon: "doc.text.magnifyingglass", title: L10n.tr("profile_settlements", lang: lang)) {
                    onOpenWeb("/settlements")
                }
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func moreAndSessionCard(lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            navRow(icon: "questionmark.circle", title: L10n.tr("profile_help", lang: lang)) {
                onOpenWeb("/help")
            }
            navRow(icon: "exclamationmark.bubble", title: L10n.tr("profile_report", lang: lang)) {
                onOpenWeb("/report")
            }
            Divider()
            Button(L10n.tr("profile_logout", lang: lang)) {
                Task { await vm.logout() }
            }
            .buttonStyle(GrOutlineButtonStyle(tint: GrColors.danger))
            .frame(maxWidth: .infinity, alignment: .leading)

            Button(L10n.tr("profile_logout_all", lang: lang)) {
                confirmLogoutAll = true
            }
            .font(GrTypography.caption)
            .foregroundStyle(GrColors.danger)
        }
        .grCard()
    }

    private func rowItem(_ label: String, _ value: String) -> some View {
        HStack {
            Text(label).font(GrTypography.subheadline).foregroundStyle(GrColors.muted)
            Spacer()
            Text(value).font(GrTypography.bodyMedium)
        }
    }

    private func navRow(icon: String, title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .foregroundStyle(GrColors.primary)
                    .frame(width: 22)
                Text(title)
                    .font(GrTypography.body)
                    .foregroundStyle(GrColors.text)
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(GrColors.muted)
            }
            .padding(.vertical, 6)
        }
        .buttonStyle(.plain)
    }
}

private struct NotificationPrefsSheet: View {
    @State var prefs: NotificationPrefsDto
    let lang: String
    let notifAllowed: Bool
    let onRequestPushPermission: () async -> Void
    let onSave: (UpdateNotificationPrefsReq) async -> Void

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section(L10n.tr("ios_notif_permission_title", lang: lang)) {
                    if notifAllowed {
                        Label(L10n.tr("ios_notif_enabled", lang: lang), systemImage: "checkmark.circle.fill")
                            .foregroundStyle(GrColors.success)
                    } else {
                        Button(L10n.tr("ios_notif_enable", lang: lang)) {
                            Task { await onRequestPushPermission() }
                        }
                    }
                    Toggle(L10n.tr("notif_pref_push", lang: lang), isOn: $prefs.pushOptIn)
                    Toggle(L10n.tr("notif_pref_whatsapp", lang: lang), isOn: $prefs.whatsappOptIn)
                    Toggle(L10n.tr("notif_pref_hide_lock", lang: lang), isOn: $prefs.hideLockDetails)
                }

                Section(L10n.tr("notif_pref_quiet_hours", lang: lang)) {
                    TextField(L10n.tr("notif_pref_quiet_start", lang: lang), text: $prefs.quietHoursStart)
                    TextField(L10n.tr("notif_pref_quiet_end", lang: lang), text: $prefs.quietHoursEnd)
                    Stepper("\(L10n.tr("notif_pref_driver_remind", lang: lang)): \(prefs.driverRemindMins)", value: $prefs.driverRemindMins, in: 5...120, step: 5)
                }
            }
            .navigationTitle(L10n.tr("notif_prefs_title", lang: lang))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.tr("common_cancel", lang: lang)) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(L10n.tr("notif_pref_save", lang: lang)) {
                        dismiss()
                        Task {
                            await onSave(UpdateNotificationPrefsReq(
                                pushOptIn: prefs.pushOptIn,
                                whatsappOptIn: prefs.whatsappOptIn,
                                readinessPromptMins: prefs.readinessPromptMins,
                                driverRemindMins: prefs.driverRemindMins,
                                mutedCategories: prefs.mutedCategories,
                                quietHoursStart: prefs.quietHoursStart,
                                quietHoursEnd: prefs.quietHoursEnd,
                                hideLockDetails: prefs.hideLockDetails
                            ))
                        }
                    }
                }
            }
        }
    }
}

public struct DriverSettingsScreen: View {
    public let container: AppContainer
    @State private var vm: DriverSettingsViewModel
    @State private var showVehicleSheet = false
    @State private var editingVehicle: DriverVehicle?
    @State private var vModel = ""
    @State private var vColor = ""
    @State private var vPlate = ""
    @State private var vSeats = 4
    @State private var vType = "standard"
    @State private var vWheelchair = false
    @State private var vLowStep = false
    @State private var vChildSeat = false

    public init(container: AppContainer) {
        self.container = container
        self._vm = State(initialValue: DriverSettingsViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                if vm.accessDenied {
                    EmptyStateCard(systemImage: "steeringwheel.slash", title: L10n.tr("role_driver_access_required", lang: lang))
                } else {
                    if let msg = vm.message {
                        CalloutBanner(tone: .success, icon: "checkmark.circle.fill", text: msg, actionTitle: L10n.tr("common_close", lang: lang)) {
                            vm.clearMessage()
                        }
                    }
                    if let err = vm.error {
                        ErrorBannerView(message: err, onDismiss: { vm.clearError() })
                    }

                    VStack(alignment: .leading, spacing: 10) {
                        Text(L10n.tr("driver_areas_title", lang: lang)).font(GrTypography.titleSmall)
                        Text(L10n.tr("driver_areas_hint", lang: lang)).font(GrTypography.caption).foregroundStyle(GrColors.muted)
                        FlowLayout(spacing: 8) {
                            ForEach(vm.areas) { area in
                                let selected = vm.selectedAreaIds.contains(area.id)
                                Button(area.name) {
                                    vm.toggleArea(area.id)
                                }
                                .font(GrTypography.caption.weight(.semibold))
                                .foregroundStyle(selected ? GrColors.primaryInk : GrColors.text)
                                .padding(.horizontal, 12)
                                .padding(.vertical, 6)
                                .background(Capsule().fill(selected ? GrColors.primary : GrColors.surface2))
                            }
                        }
                    }
                    .grCard()

                    VStack(alignment: .leading, spacing: 10) {
                        Text(L10n.tr("driver_carpool_title", lang: lang)).font(GrTypography.titleSmall)
                        Toggle(
                            L10n.tr("driver_carpool_label", lang: lang),
                            isOn: Binding(get: { vm.carpoolEnabled }, set: { vm.setCarpool($0) })
                        )
                        Stepper(
                            "\(L10n.tr("driver_spare_seats", lang: lang)): \(vm.spareSeats)",
                            onIncrement: { vm.stepSpareSeats(1) },
                            onDecrement: { vm.stepSpareSeats(-1) }
                        )
                        Picker(L10n.tr("driver_gate_wait", lang: lang), selection: Binding(
                            get: { vm.gateWaitMins },
                            set: { vm.setGateWaitMins($0) }
                        )) {
                            Text(L10n.tr("driver_wait_3m", lang: lang)).tag(3)
                            Text(L10n.tr("driver_wait_5m", lang: lang)).tag(5)
                            Text(L10n.tr("driver_wait_8m", lang: lang)).tag(8)
                            Text(L10n.tr("driver_wait_10m", lang: lang)).tag(10)
                        }
                    }
                    .grCard()

                    if vm.dirty {
                        Button(L10n.tr("driver_save_settings", lang: lang)) {
                            Task { await vm.saveProfile() }
                        }
                        .buttonStyle(GrPrimaryButtonStyle())
                        .disabled(vm.busy)
                    }

                    VStack(alignment: .leading, spacing: 10) {
                        HStack {
                            Text(L10n.tr("driver_vehicles_title", lang: lang)).font(GrTypography.titleSmall)
                            Spacer()
                            Button(L10n.tr("driver_add_vehicle", lang: lang)) {
                                editingVehicle = nil
                                vModel = ""; vColor = ""; vPlate = ""; vSeats = 4; vType = "standard"
                                vWheelchair = false; vLowStep = false; vChildSeat = false
                                showVehicleSheet = true
                            }
                            .font(GrTypography.label)
                        }

                        if vm.vehicles.isEmpty {
                            Text(L10n.tr("driver_no_vehicles", lang: lang)).font(GrTypography.subheadline).foregroundStyle(GrColors.muted)
                        } else {
                            ForEach(vm.vehicles) { vehicle in
                                HStack {
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text("\(vehicle.color) \(vehicle.model) · \(vehicle.plate)")
                                            .font(GrTypography.bodyMedium)
                                        Text("\(vehicle.seats) seats\(vehicle.isActive ? " · \(L10n.tr("driver_vehicle_active", lang: lang))" : "")")
                                            .font(GrTypography.caption)
                                            .foregroundStyle(vehicle.isActive ? GrColors.success : GrColors.muted)
                                    }
                                    Spacer()
                                    if !vehicle.isActive, let vid = vehicle.id {
                                        Button(L10n.tr("driver_set_active", lang: lang)) {
                                            Task { await vm.selectVehicle(vid) }
                                        }
                                        .buttonStyle(GrOutlineButtonStyle())
                                    }
                                    Button {
                                        editingVehicle = vehicle
                                        vModel = vehicle.model; vColor = vehicle.color; vPlate = vehicle.plate
                                        vSeats = vehicle.seats; vType = vehicle.vehicleType
                                        vWheelchair = vehicle.wheelchairOk; vLowStep = vehicle.lowStep; vChildSeat = vehicle.childSeat
                                        showVehicleSheet = true
                                    } label: {
                                        Image(systemName: "pencil")
                                    }
                                    .grMinTouchTarget()
                                }
                                .padding(.vertical, 4)
                            }
                        }
                    }
                    .grCard()
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("driver_settings_title", lang: lang))
        .task { await vm.load() }
        .sheet(isPresented: $showVehicleSheet) {
            NavigationStack {
                Form {
                    TextField(L10n.tr("driver_vehicle_model", lang: lang), text: $vModel)
                    TextField(L10n.tr("driver_vehicle_color", lang: lang), text: $vColor)
                    TextField(L10n.tr("driver_vehicle_plate", lang: lang), text: $vPlate)
                        .textInputAutocapitalization(.characters)
                    Stepper("\(L10n.tr("driver_vehicle_seats", lang: lang)): \(vSeats)", value: $vSeats, in: 2...9)
                    Toggle(L10n.tr("driver_wheelchair_ok", lang: lang), isOn: $vWheelchair)
                    Toggle(L10n.tr("driver_low_step", lang: lang), isOn: $vLowStep)
                    Toggle(L10n.tr("driver_child_seat", lang: lang), isOn: $vChildSeat)
                }
                .navigationTitle(L10n.tr(editingVehicle == nil ? "driver_add_vehicle" : "driver_edit_vehicle", lang: lang))
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button(L10n.tr("common_cancel", lang: lang)) { showVehicleSheet = false }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button(L10n.tr("common_save", lang: lang)) {
                            Task {
                                let saved = await vm.saveVehicle(
                                    editingId: editingVehicle?.id,
                                    req: DriverVehicleReq(
                                        model: vModel, color: vColor, plate: vPlate, seats: vSeats,
                                        vehicleType: vType, wheelchairOk: vWheelchair, lowStep: vLowStep,
                                        childSeat: vChildSeat, isActive: editingVehicle?.isActive ?? false
                                    )
                                )
                                if saved { showVehicleSheet = false }
                            }
                        }
                    }
                }
            }
        }
    }
}

public struct ProxyRidersScreen: View {
    public let container: AppContainer
    @State private var vm: ProxyRidersViewModel
    @State private var showEditor = false
    @State private var editing: ProxyRider?
    @State private var name = ""
    @State private var phone = ""
    @State private var relationship = "parent"
    @State private var areaId: Int64?
    @State private var address = ""
    @State private var gateNotes = ""

    public init(container: AppContainer) {
        self.container = container
        self._vm = State(initialValue: ProxyRidersViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang
        List {
            if let err = vm.error {
                ErrorBannerView(message: err, onDismiss: { vm.clearError() })
            }
            if vm.riders.isEmpty && !vm.loading {
                EmptyStateCard(systemImage: "person.2", title: L10n.tr("proxy_riders_empty", lang: lang))
                    .listRowSeparator(.hidden)
            } else {
                ForEach(vm.riders) { rider in
                    Button {
                        editing = rider
                        name = rider.name; phone = rider.phone; relationship = rider.relationship
                        areaId = rider.areaId; address = rider.pickupAddress; gateNotes = rider.gateNotes
                        showEditor = true
                    } label: {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(rider.name).font(GrTypography.bodyMedium)
                            Text("\(rider.relationship) · \(rider.pickupAddress)")
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                        }
                    }
                    .swipeActions {
                        Button(role: .destructive) {
                            Task { _ = await vm.delete(id: rider.id) }
                        } label: {
                            Label(L10n.tr("proxy_riders_remove", lang: lang), systemImage: "trash")
                        }
                    }
                }
            }
        }
        .listStyle(.plain)
        .navigationTitle(L10n.tr("proxy_riders_title", lang: lang))
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    editing = nil; name = ""; phone = ""; relationship = "parent"; areaId = vm.areas.first?.id; address = ""; gateNotes = ""
                    showEditor = true
                } label: { Image(systemName: "plus") }
            }
        }
        .refreshable { await vm.refresh() }
        .task { await vm.load() }
        .sheet(isPresented: $showEditor) {
            NavigationStack {
                Form {
                    TextField(L10n.tr("ride_new_rider_name", lang: lang), text: $name)
                    TextField(L10n.tr("ride_new_rider_phone", lang: lang), text: $phone).keyboardType(.phonePad)
                    Picker(L10n.tr("ride_new_rider_relationship", lang: lang), selection: $relationship) {
                        Text(L10n.tr("ride_new_relationship_parent", lang: lang)).tag("parent")
                        Text(L10n.tr("ride_new_relationship_child", lang: lang)).tag("child")
                        Text(L10n.tr("ride_new_relationship_spouse", lang: lang)).tag("spouse")
                        Text(L10n.tr("ride_new_relationship_friend", lang: lang)).tag("friend")
                        Text(L10n.tr("proxy_rel_cell", lang: lang)).tag("cell_member")
                        Text(L10n.tr("ride_new_relationship_other", lang: lang)).tag("other")
                    }
                    Picker(L10n.tr("ride_new_area", lang: lang), selection: $areaId) {
                        Text("-").tag(Int64?.none)
                        ForEach(vm.areas) { a in Text(a.name).tag(Int64?.some(a.id)) }
                    }
                    TextField(L10n.tr("ride_new_address", lang: lang), text: $address)
                    TextField(L10n.tr("saved_addresses_gate_notes", lang: lang), text: $gateNotes)
                }
                .navigationTitle(L10n.tr(editing == nil ? "proxy_riders_add" : "proxy_riders_edit", lang: lang))
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button(L10n.tr("common_cancel", lang: lang)) { showEditor = false }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button(L10n.tr("common_save", lang: lang)) {
                            Task {
                                let ok = await vm.save(editingId: editing?.id, req: ProxyRiderReq(
                                    name: name, phone: phone, relationship: relationship, areaId: areaId,
                                    pickupAddress: address, gateNotes: gateNotes
                                ))
                                if ok { showEditor = false }
                            }
                        }
                    }
                }
            }
        }
    }
}

public struct SavedAddressesScreen: View {
    public let container: AppContainer
    public let locationManager: LiveTripLocationManager

    @State private var vm: SavedAddressesViewModel
    @State private var showEditor = false
    @State private var editing: SavedPlace?
    @State private var label = ""
    @State private var areaId: Int64?
    @State private var address = ""
    @State private var gateNotes = ""
    @State private var isDefault = false
    @State private var lat: Double?
    @State private var lng: Double?

    public init(container: AppContainer, locationManager: LiveTripLocationManager) {
        self.container = container
        self.locationManager = locationManager
        self._vm = State(initialValue: SavedAddressesViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang
        List {
            if let err = vm.error {
                ErrorBannerView(message: err, onDismiss: { vm.clearError() })
            }
            if vm.places.isEmpty && !vm.loading {
                EmptyStateCard(systemImage: "mappin.and.ellipse", title: L10n.tr("saved_addresses_empty", lang: lang))
                    .listRowSeparator(.hidden)
            } else {
                ForEach(vm.places) { place in
                    Button {
                        editing = place
                        label = place.label; areaId = place.areaId; address = place.address
                        gateNotes = place.gateNotes; isDefault = place.isDefault == 1
                        lat = place.lat; lng = place.lng
                        showEditor = true
                    } label: {
                        HStack {
                            VStack(alignment: .leading, spacing: 3) {
                                HStack {
                                    Text(place.label).font(GrTypography.bodyMedium)
                                    if place.isDefault == 1 {
                                        Text(L10n.tr("saved_addresses_default", lang: lang))
                                            .font(GrTypography.badge)
                                            .foregroundStyle(GrColors.primary)
                                            .padding(.horizontal, 6)
                                            .padding(.vertical, 2)
                                            .background(Capsule().fill(GrColors.primarySoft))
                                    }
                                }
                                Text(place.address).font(GrTypography.caption).foregroundStyle(GrColors.muted)
                            }
                            Spacer()
                            if place.lat != nil {
                                Image(systemName: "location.fill").font(.caption).foregroundStyle(GrColors.success)
                            }
                        }
                    }
                    .swipeActions {
                        Button(role: .destructive) {
                            Task { _ = await vm.delete(id: place.id) }
                        } label: {
                            Label(L10n.tr("saved_addresses_remove", lang: lang), systemImage: "trash")
                        }
                        if place.isDefault != 1 {
                            Button(L10n.tr("saved_addresses_default", lang: lang)) {
                                Task { await vm.setDefault(place) }
                            }
                            .tint(GrColors.primary)
                        }
                    }
                }
            }
        }
        .listStyle(.plain)
        .navigationTitle(L10n.tr("saved_addresses_title", lang: lang))
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    editing = nil; label = ""; areaId = vm.areas.first?.id; address = ""
                    gateNotes = ""; isDefault = vm.places.isEmpty; lat = nil; lng = nil
                    showEditor = true
                } label: { Image(systemName: "plus") }
            }
        }
        .refreshable { await vm.refresh() }
        .task { await vm.load() }
        .sheet(isPresented: $showEditor) {
            NavigationStack {
                Form {
                    TextField(L10n.tr("saved_addresses_label_hint", lang: lang), text: $label)
                    Picker(L10n.tr("saved_addresses_area", lang: lang), selection: $areaId) {
                        Text("-").tag(Int64?.none)
                        ForEach(vm.areas) { a in Text(a.name).tag(Int64?.some(a.id)) }
                    }
                    TextField(L10n.tr("saved_addresses_street", lang: lang), text: $address)
                    TextField(L10n.tr("saved_addresses_gate_notes", lang: lang), text: $gateNotes)
                    Toggle(L10n.tr("saved_addresses_use_default", lang: lang), isOn: $isDefault)
                    Section {
                        Button(L10n.tr("saved_addresses_use_gps", lang: lang)) {
                            Task {
                                if let coord = try? await locationManager.captureCurrentLocation() {
                                    lat = coord.lat; lng = coord.lng
                                }
                            }
                        }
                        if let lat, let lng {
                            HStack {
                                Text("\(String(format: "%.5f", lat)), \(String(format: "%.5f", lng))")
                                    .font(.caption)
                                Spacer()
                                Button(L10n.tr("saved_addresses_gps_clear", lang: lang)) {
                                    self.lat = nil; self.lng = nil
                                }
                            }
                        }
                    }
                }
                .navigationTitle(L10n.tr(editing == nil ? "saved_addresses_add" : "saved_addresses_edit", lang: lang))
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button(L10n.tr("common_cancel", lang: lang)) { showEditor = false }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button(L10n.tr("common_save", lang: lang)) {
                            Task {
                                let ok = await vm.save(
                                    editingId: editing?.id,
                                    label: label,
                                    areaId: areaId,
                                    address: address,
                                    gateNotes: gateNotes,
                                    isDefault: isDefault,
                                    lat: lat,
                                    lng: lng,
                                    clearCoords: editing != nil && lat == nil
                                )
                                if ok { showEditor = false }
                            }
                        }
                    }
                }
            }
        }
    }
}

private struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxW = proposal.width ?? 320
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowH: CGFloat = 0
        for sub in subviews {
            let s = sub.sizeThatFits(.unspecified)
            if x + s.width > maxW && x > 0 {
                x = 0
                y += rowH + spacing
                rowH = 0
            }
            x += s.width + spacing
            rowH = max(rowH, s.height)
        }
        return CGSize(width: maxW, height: y + rowH)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX
        var y = bounds.minY
        var rowH: CGFloat = 0
        for sub in subviews {
            let s = sub.sizeThatFits(.unspecified)
            if x + s.width > bounds.maxX && x > bounds.minX {
                x = bounds.minX
                y += rowH + spacing
                rowH = 0
            }
            sub.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(s))
            x += s.width + spacing
            rowH = max(rowH, s.height)
        }
    }
}
