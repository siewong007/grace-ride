import GraceRideKit
import SwiftUI

public struct EventsScreen: View {
    public let container: AppContainer
    public let onNavigate: (AppRoute) -> Void
    public let onOpenWeb: (String) -> Void

    @State private var vm: EventsViewModel

    public init(
        container: AppContainer,
        onNavigate: @escaping (AppRoute) -> Void,
        onOpenWeb: @escaping (String) -> Void
    ) {
        self.container = container
        self.onNavigate = onNavigate
        self.onOpenWeb = onOpenWeb
        self._vm = State(initialValue: EventsViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang
        let me = container.auth.me
        let canManage = me?.can("events", act: "update") == true || me?.can("events", act: "create") == true

        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Picker("", selection: Binding(
                    get: { vm.tab },
                    set: { newTab in Task { await vm.setTab(newTab) } }
                )) {
                    Text(L10n.tr("events_tab_upcoming", lang: lang)).tag(0)
                    Text(L10n.tr("events_tab_my_schedule", lang: lang)).tag(1)
                    Text(L10n.tr("events_tab_past", lang: lang)).tag(2)
                    if canManage {
                        Text(L10n.tr("events_tab_managing", lang: lang)).tag(3)
                    }
                }
                .pickerStyle(.segmented)

                if vm.offline {
                    CalloutBanner(
                        tone: .warning,
                        icon: "wifi.slash",
                        text: L10n.tr("network_offline_live", lang: lang)
                    )
                }

                if let notice = vm.actionNotice {
                    CalloutBanner(
                        tone: .success,
                        icon: "checkmark.circle.fill",
                        text: L10n.tr(notice == .published ? "event_published_notice" : "event_unpublished_notice", lang: lang),
                        actionTitle: L10n.tr("common_close", lang: lang)
                    ) {
                        vm.clearNotice()
                    }
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

                if vm.loading && vm.events.isEmpty {
                    LoadingSkeletonView(rows: 3)
                } else if vm.tab == 1 {
                    scheduleTab(lang: lang)
                } else {
                    if (vm.tab == 0 || vm.tab == 3) && vm.draftCount > 0 {
                        Button {
                            Task { await vm.toggleHideDrafts() }
                        } label: {
                            Label(
                                vm.hideDrafts
                                    ? L10n.tr("events_show_drafts", lang: lang)
                                    : L10n.tr("events_hide_drafts_count", lang: lang, Int64(vm.draftCount)),
                                systemImage: vm.hideDrafts ? "eye" : "eye.slash"
                            )
                            .font(GrTypography.caption)
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                    }

                    if vm.events.isEmpty {
                        EmptyStateCard(
                            systemImage: "calendar",
                            title: L10n.tr(vm.hideDrafts ? "events_drafts_hidden_empty" : "events_empty", lang: lang)
                        )
                    } else {
                        ForEach(vm.events) { ev in
                            eventCard(ev: ev, lang: lang)
                        }
                    }
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("events_title", lang: lang))
        .toolbar {
            if me?.can("events", act: "create") == true {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        onOpenWeb("/events/new")
                    } label: {
                        Image(systemName: "plus")
                    }
                }
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
    private func scheduleTab(lang: String) -> some View {
        let dayKeys = Array(Set(vm.events.compactMap { KlTime.date($0.startsAt) } + vm.myRides.compactMap { KlTime.date($0.pickupAt) })).sorted()
        let dayOptions: [DayOption] = [DayOption(key: "all", weekday: L10n.tr("events_all_days", lang: lang), dateLabel: "")]
            + dayKeys.map { key in
                let sample = "\(key) 09:00:00"
                return DayOption(
                    key: key,
                    weekday: KlTime.weekdayShort(sample, lang: lang),
                    dateLabel: KlTime.dayMonthShort(sample, lang: lang)
                )
            }

        let nodes: [TimelineNode] = {
            var out: [TimelineNode] = []
            for ev in vm.events where vm.selectedDay == "all" || KlTime.date(ev.startsAt) == vm.selectedDay {
                out.append(TimelineNode(
                    id: "ev-\(ev.id)",
                    timeLabel: KlTime.displayTime(ev.startsAt),
                    title: ev.title,
                    subtitle: ev.location,
                    kind: .event,
                    state: ev.mySignup != nil ? .active : .upcoming,
                    badge: ev.status == "draft" ? TimelineBadge(text: L10n.tr("event_status_draft", lang: lang), kind: .pending) : nil,
                    eventIcon: ev.icon,
                    eventKind: ev.kind,
                    onClick: { onNavigate(.eventDetail(id: ev.id, checkinCode: nil)) }
                ))
            }
            for ride in vm.myRides where vm.selectedDay == "all" || KlTime.date(ride.pickupAt) == vm.selectedDay {
                out.append(TimelineNode(
                    id: "ride-\(ride.id)",
                    timeLabel: KlTime.displayTime(ride.pickupAt),
                    title: L10n.tr("event_timeline_ride_to", lang: lang, ride.destination),
                    subtitle: L10n.rideStatus(ride.status, lang: lang),
                    kind: ride.leg == "return" ? .returnRide : .ride,
                    state: ride.status == "completed" ? .done : .active,
                    onClick: { onNavigate(.rideDetail(id: ride.id, justBooked: false)) }
                ))
            }
            for drive in vm.myDrives where vm.selectedDay == "all" || KlTime.date(drive.pickupAt) == vm.selectedDay {
                out.append(TimelineNode(
                    id: "drive-\(drive.id)",
                    timeLabel: KlTime.displayTime(drive.pickupAt),
                    title: L10n.tr("event_timeline_drive_for", lang: lang, drive.passenger.name),
                    subtitle: drive.area.name,
                    kind: .drive,
                    state: drive.status == "completed" ? .done : .active,
                    onClick: { onNavigate(.rideDetail(id: drive.id, justBooked: false)) }
                ))
            }
            return out.sorted { $0.timeLabel < $1.timeLabel }
        }()

        EventScheduleTimelineView(
            days: dayOptions,
            selectedDayKey: $vm.selectedDay,
            nodes: nodes,
            emptyText: L10n.tr("events_schedule_empty_desc", lang: lang)
        )
        .grCard()
    }

    @ViewBuilder
    private func eventCard(ev: EventItem, lang: String) -> some View {
        Button {
            onNavigate(.eventDetail(id: ev.id, checkinCode: nil))
        } label: {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 12) {
                    EventIconView(iconName: ev.icon, category: ev.kind, size: 42)
                    VStack(alignment: .leading, spacing: 3) {
                        HStack {
                            Text(ev.title)
                                .font(GrTypography.bodyMedium)
                                .foregroundStyle(GrColors.text)
                            Spacer()
                            if ev.status == "draft" {
                                Text(L10n.tr("event_status_draft", lang: lang))
                                    .font(GrTypography.badge)
                                    .foregroundStyle(GrColors.warning)
                                    .padding(.horizontal, 8)
                                    .padding(.vertical, 3)
                                    .background(Capsule().fill(GrColors.warningSoft))
                            }
                        }
                        Text("\(KlTime.display(ev.startsAt, lang: lang)) · \(ev.location)")
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.muted)
                    }
                }

                HStack(spacing: 12) {
                    if let left = ev.spotsLeft, ev.capacity > 0 {
                        Text(L10n.tr("event_spots_left", lang: lang, left))
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.muted)
                    }
                    if let reqCount = vm.requestsByEvent[ev.id], reqCount > 0 {
                        Text(L10n.tr("event_rides_needed", lang: lang, Int64(reqCount)))
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.primary)
                    }
                    Spacer()
                    if let signup = ev.mySignup {
                        Label(signup.status, systemImage: "checkmark.circle.fill")
                            .font(GrTypography.caption.weight(.semibold))
                            .foregroundStyle(GrColors.success)
                    }
                }
            }
            .grCard()
        }
        .buttonStyle(.plain)
    }
}

public struct EventDetailScreen: View {
    public let container: AppContainer
    public let eventId: Int64
    public let checkinCode: String?
    public let onNavigate: (AppRoute) -> Void
    public let onOpenWeb: (String) -> Void

    @State private var vm: EventDetailViewModel
    @State private var showSignupSheet = false
    @State private var needRide = false
    @State private var offerDrive = false
    @State private var offerSeats = 3
    @State private var selectedGroupId: Int64?
    @State private var showCheckinScanner = false
    @State private var showMemberScanner = false
    @State private var manualCheckinCode = ""
    @State private var calendarBanner: String?

    public init(
        container: AppContainer,
        eventId: Int64,
        checkinCode: String? = nil,
        onNavigate: @escaping (AppRoute) -> Void,
        onOpenWeb: @escaping (String) -> Void
    ) {
        self.container = container
        self.eventId = eventId
        self.checkinCode = checkinCode
        self.onNavigate = onNavigate
        self.onOpenWeb = onOpenWeb
        self._vm = State(initialValue: EventDetailViewModel(container: container, eventId: eventId))
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if let notice = vm.actionNotice {
                    CalloutBanner(
                        tone: .success,
                        icon: "checkmark.circle.fill",
                        text: L10n.tr(notice == .published ? "event_published_notice" : "event_unpublished_notice", lang: lang),
                        actionTitle: L10n.tr("common_close", lang: lang)
                    ) { vm.clearNotice() }
                }

                if let msg = vm.message {
                    ErrorBannerView(message: msg, onDismiss: { vm.clearMessage() })
                }

                if let cal = calendarBanner {
                    CalloutBanner(tone: .info, icon: "calendar", text: cal)
                }

                if let err = vm.error {
                    ErrorBannerView(
                        message: err,
                        retryTitle: L10n.tr("common_retry", lang: lang),
                        onRetry: { Task { await vm.refresh() } }
                    )
                }

                if vm.loading && vm.event == nil {
                    LoadingSkeletonView(rows: 4)
                } else if let ev = vm.event {
                    if ev.status == "draft" {
                        CalloutBanner(tone: .warning, icon: "eye.slash", text: L10n.tr("event_draft_banner", lang: lang))
                    }

                    VStack(alignment: .leading, spacing: 10) {
                        if let photoId = ev.photoFileId, photoId > 0 {
                            AuthenticatedRemoteImage(fileId: photoId, container: container, contentMode: .fill) {
                                Rectangle().fill(GrColors.surface2).frame(height: 180)
                            }
                            .frame(height: 180)
                            .clipShape(RoundedRectangle(cornerRadius: 12))
                        }

                        HStack(spacing: 12) {
                            EventIconView(iconName: ev.icon, category: ev.kind, size: 46)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(ev.title)
                                    .font(GrTypography.titleMedium)
                                Text(KlTime.display(ev.startsAt, lang: lang))
                                    .font(GrTypography.subheadline)
                                    .foregroundStyle(GrColors.primary)
                            }
                        }

                        if !ev.location.isEmpty {
                            Label(ev.location, systemImage: "mappin.and.ellipse")
                                .font(GrTypography.subheadline)
                        }
                        if !ev.costNote.isEmpty {
                            Label("\(L10n.tr("event_cost", lang: lang)): \(ev.costNote)", systemImage: "banknote")
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                        }
                        if let dl = ev.signupDeadline {
                            Label(L10n.tr("event_workflow_signup_deadline", lang: lang, KlTime.display(dl, lang: lang)), systemImage: "clock")
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                        }
                        if !ev.description.isEmpty {
                            Text(ev.description)
                                .font(GrTypography.body)
                                .foregroundStyle(GrColors.text)
                                .padding(.top, 4)
                        }

                        Button {
                            Task {
                                let res = await DeviceActions.saveEventToCalendar(ev, lang: lang)
                                calendarBanner = res.message
                            }
                        } label: {
                            Label(L10n.tr("event_add_to_calendar", lang: lang), systemImage: "calendar.badge.plus")
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                    }
                    .grCard()

                    rsvpAndTransportSection(ev: ev, lang: lang)

                    if ev.attendanceEnabled && ev.mySignup != nil {
                        checkinSection(ev: ev, lang: lang)
                    }

                    if ev.canManage || ev.canPublish || ev.canUnpublish {
                        manageSection(ev: ev, lang: lang)
                    }
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(vm.event?.title ?? L10n.tr("events_title", lang: lang))
        .navigationBarTitleDisplayMode(.inline)
        .refreshable {
            await vm.refresh()
        }
        .task {
            await vm.load()
            if let code = checkinCode, !code.isEmpty {
                manualCheckinCode = code
            }
        }
        .sheet(isPresented: $showSignupSheet) {
            if let ev = vm.event {
                signupSheet(ev: ev, lang: lang)
            }
        }
        .sheet(isPresented: $showCheckinScanner) {
            QrScannerSheet(lang: lang, title: L10n.tr("event_scan_qr", lang: lang)) { scanned in
                switch QrParsing.parseCheckinInput(scanned, currentEventId: eventId) {
                case .code(let code):
                    manualCheckinCode = code
                    Task { await vm.selfCheckin(code: code) }
                case .wrongEvent:
                    calendarBanner = L10n.tr("event_checkin_wrong_event", lang: lang)
                case .invalid:
                    break
                }
            }
        }
        .sheet(isPresented: $showMemberScanner) {
            QrScannerSheet(lang: lang, title: L10n.tr("event_scan_member_btn", lang: lang)) { scanned in
                switch QrParsing.parseMemberPassInput(scanned, currentEventId: eventId) {
                case .code(let code):
                    Task { await vm.scanMemberPass(passCode: code) }
                case .wrongEvent:
                    calendarBanner = L10n.tr("event_checkin_wrong_event", lang: lang)
                case .invalid:
                    break
                }
            }
        }
    }

    @ViewBuilder
    private func rsvpAndTransportSection(ev: EventItem, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            if let signup = ev.mySignup {
                HStack {
                    Label(L10n.tr("event_my_status", lang: lang), systemImage: "checkmark.seal.fill")
                        .font(GrTypography.bodyMedium)
                        .foregroundStyle(GrColors.success)
                    Spacer()
                    Text(signup.status.capitalized)
                        .font(GrTypography.badge)
                        .foregroundStyle(GrColors.success)
                }

                if let ride = vm.outboundRide {
                    Button {
                        onNavigate(.rideDetail(id: ride.id, justBooked: false))
                    } label: {
                        HStack {
                            Label("\(L10n.tr("event_my_ride", lang: lang)): \(L10n.rideStatus(ride.status, lang: lang))", systemImage: "car.fill")
                            Spacer()
                            Image(systemName: "chevron.right")
                        }
                        .padding(10)
                        .background(GrColors.surface2)
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                    }
                    .buttonStyle(.plain)
                } else if container.auth.me?.can("rides", act: "create") == true {
                    Button(L10n.tr("home_book_ride", lang: lang)) {
                        onNavigate(.rideNew(eventId: ev.id, proxy: false))
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                }

                if container.auth.me?.can("rides", act: "create") == true {
                    Button(L10n.tr("event_book_for_family", lang: lang)) {
                        onNavigate(.rideNew(eventId: ev.id, proxy: true))
                    }
                    .buttonStyle(GrOutlineButtonStyle())
                }

                Button(L10n.tr("event_withdraw", lang: lang)) {
                    Task { await vm.withdraw() }
                }
                .buttonStyle(GrOutlineButtonStyle(tint: GrColors.danger))
                .disabled(vm.busy)
            } else {
                Button(L10n.tr("event_signup", lang: lang)) {
                    selectedGroupId = ev.joinAs.first?.id
                    showSignupSheet = true
                }
                .buttonStyle(GrPrimaryButtonStyle())
                .disabled(vm.busy)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .grCard()
    }

    @ViewBuilder
    private func checkinSection(ev: EventItem, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("event_checkin_title", lang: lang))
                .font(GrTypography.titleSmall)

            if let passCode = ev.mySignup?.passCode, !passCode.isEmpty {
                VStack(spacing: 6) {
                    Text(L10n.tr("event_pass_label", lang: lang)).font(GrTypography.caption).foregroundStyle(GrColors.muted)
                    QrCodeView(payload: "grpass:\(ev.id):\(passCode)", size: 170, boostBrightness: true)
                    Text(passCode).font(.system(.caption, design: .monospaced))
                }
                .frame(maxWidth: .infinity)
            }

            if ev.mySignup?.attended == true {
                CalloutBanner(tone: .success, icon: "checkmark.circle.fill", text: L10n.tr("event_checkin_done", lang: lang))
            } else {
                HStack(spacing: 8) {
                    TextField(L10n.tr("event_enter_code", lang: lang), text: $manualCheckinCode)
                        .textFieldStyle(.roundedBorder)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                    Button(L10n.tr("event_checkin_btn", lang: lang)) {
                        Task { await vm.selfCheckin(code: manualCheckinCode.isEmpty ? nil : manualCheckinCode) }
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                    .disabled(vm.busy)
                }

                Button {
                    showCheckinScanner = true
                } label: {
                    Label(L10n.tr("event_scan_qr", lang: lang), systemImage: "qrcode.viewfinder")
                }
                .buttonStyle(GrOutlineButtonStyle())
            }

            if let cerr = vm.checkinError {
                ErrorBannerView(message: cerr, onDismiss: { vm.clearCheckinError() })
            }

            if let res = vm.checkinResult {
                CalloutBanner(
                    tone: .success,
                    icon: "checkmark.circle.fill",
                    text: L10n.tr(res.alreadyAttended ? "event_checkin_already" : "event_checkin_success", lang: lang, res.title)
                )
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func manageSection(ev: EventItem, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L10n.tr("common_manage", lang: lang))
                .font(GrTypography.label)
                .foregroundStyle(GrColors.muted)

            HStack(spacing: 8) {
                if ev.canPublish {
                    Button(L10n.tr("event_publish", lang: lang)) {
                        Task { await vm.publish() }
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                    .disabled(vm.busy)
                }
                if ev.canUnpublish {
                    Button(L10n.tr("event_unpublish", lang: lang)) {
                        Task { await vm.unpublish() }
                    }
                    .buttonStyle(GrOutlineButtonStyle(tint: GrColors.warning))
                    .disabled(vm.busy)
                }
                if ev.attendanceEnabled && ev.canSeeSignups {
                    Button {
                        showMemberScanner = true
                    } label: {
                        Label(L10n.tr("event_scan_member_btn", lang: lang), systemImage: "person.badge.shield.checkmark")
                    }
                    .buttonStyle(GrOutlineButtonStyle())
                }
            }

            if let scanRes = vm.scanMemberResult {
                CalloutBanner(
                    tone: .success,
                    icon: "person.crop.circle.badge.checkmark",
                    text: L10n.tr(scanRes.alreadyAttended ? "event_scan_member_already" : "event_scan_member_success", lang: lang, scanRes.memberName)
                )
            }

            Button(L10n.tr("event_open_web_full", lang: lang)) {
                onOpenWeb("/events/\(ev.id)/edit")
            }
            .font(GrTypography.caption)
            .foregroundStyle(GrColors.primary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .grCard()
    }

    @ViewBuilder
    private func signupSheet(ev: EventItem, lang: String) -> some View {
        NavigationStack {
            Form {
                if !ev.joinAs.isEmpty {
                    Picker(L10n.tr("event_join_as_group", lang: lang), selection: $selectedGroupId) {
                        ForEach(ev.joinAs) { g in
                            Text(g.name).tag(Int64?.some(g.id))
                        }
                    }
                }
                Toggle(L10n.tr("event_need_ride", lang: lang), isOn: $needRide)
                if container.auth.me?.can("drive") == true {
                    Toggle(L10n.tr("event_offer_drive", lang: lang), isOn: $offerDrive)
                    if offerDrive {
                        Stepper("\(L10n.tr("event_offer_seats", lang: lang)): \(offerSeats)", value: $offerSeats, in: 1...8)
                    }
                }
            }
            .navigationTitle(L10n.tr("event_signup", lang: lang))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.tr("common_cancel", lang: lang)) { showSignupSheet = false }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(L10n.tr("common_confirm", lang: lang)) {
                        showSignupSheet = false
                        Task {
                            await vm.signup(
                                needRide: needRide,
                                offerDrive: offerDrive,
                                asGroupId: selectedGroupId,
                                offerSeats: offerDrive ? offerSeats : nil
                            )
                        }
                    }
                }
            }
        }
    }
}
