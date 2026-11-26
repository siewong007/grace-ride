import GraceRideKit
import PhotosUI
import SwiftUI

public struct CostsScreen: View {
    public let container: AppContainer

    @State private var vm: CostsViewModel
    @State private var selectedPhoto: PhotosPickerItem?

    public init(container: AppContainer) {
        self.container = container
        self._vm = State(initialValue: CostsViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if vm.offline {
                    CalloutBanner(
                        tone: .warning,
                        icon: "wifi.slash",
                        text: L10n.tr("network_offline_live", lang: lang)
                    )
                }

                if let done = vm.doneMessage {
                    CalloutBanner(tone: .success, icon: "checkmark.circle.fill", text: done, actionTitle: L10n.tr("common_close", lang: lang)) {
                        vm.clearDone()
                    }
                }

                if let err = vm.error {
                    ErrorBannerView(
                        message: err,
                        retryTitle: L10n.tr("common_retry", lang: lang),
                        onRetry: { Task { await vm.refresh() } }
                    )
                }

                if vm.loading && vm.shares.isEmpty {
                    LoadingSkeletonView(rows: 3)
                } else {
                    let pending = vm.shares.filter { $0.status == "pending" }
                    if !pending.isEmpty {
                        let total = pending.reduce(Int64(0)) { $0 + $1.amountCents }
                        VStack(alignment: .leading, spacing: 4) {
                            Text(L10n.tr("costs_total_due", lang: lang))
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.muted)
                            Text(Money.rm(total))
                                .font(GrTypography.heroTitle)
                                .foregroundStyle(GrColors.text)
                            Text(L10n.tr("costs_items_due", lang: lang, Int64(pending.count)))
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.warning)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .grCard(background: GrColors.warningSoft, border: GrColors.warning.opacity(0.3))
                    }

                    if vm.shares.isEmpty {
                        EmptyStateCard(
                            systemImage: "banknote",
                            title: L10n.tr("costs_empty", lang: lang)
                        )
                    } else {
                        ForEach(vm.shares) { share in
                            shareCard(share: share, lang: lang)
                        }
                    }
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("costs_title", lang: lang))
        .refreshable {
            await vm.refresh()
        }
        .task {
            await vm.loadInitial()
        }
        .sheet(item: Binding(
            get: { vm.paying },
            set: { if $0 == nil { vm.closePay() } }
        )) { share in
            paySheet(share: share, lang: lang)
        }
    }

    @ViewBuilder
    private func shareCard(share: Share, lang: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(share.eventTitle.isEmpty ? share.title : share.eventTitle)
                    .font(GrTypography.bodyMedium)
                Spacer()
                Text(Money.rm(share.amountCents))
                    .font(GrTypography.titleSmall)
                    .foregroundStyle(GrColors.primary)
            }
            HStack {
                if let due = share.dueDate, !due.isEmpty {
                    Text(L10n.tr("costs_due", lang: lang, due))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                }
                Spacer()
                Text(statusLabel(share.status, lang: lang))
                    .font(GrTypography.badge)
                    .foregroundStyle(statusFg(share.status))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(Capsule().fill(statusBg(share.status)))
            }
            if !share.payeeName.isEmpty || !share.payeeAccount.isEmpty {
                Text("\(L10n.tr("costs_payee", lang: lang)): \(share.payeeName) · \(share.payeeAccount)")
                    .font(GrTypography.caption)
                    .foregroundStyle(GrColors.muted)
            }
            if share.status == "pending" || share.status == "submitted" {
                Button(share.status == "submitted" ? L10n.tr("costs_resubmit", lang: lang) : L10n.tr("costs_pay", lang: lang)) {
                    vm.openPay(share)
                }
                .buttonStyle(GrPrimaryButtonStyle())
            }
        }
        .grCard()
    }

    @ViewBuilder
    private func paySheet(share: Share, lang: String) -> some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(share.title).font(GrTypography.titleSmall)
                        Text(Money.rm(share.amountCents)).font(GrTypography.titleLarge).foregroundStyle(GrColors.primary)
                        if !share.payeeAccount.isEmpty {
                            HStack {
                                Text("\(share.payeeName) · \(share.payeeAccount)")
                                    .font(GrTypography.subheadline)
                                Spacer()
                                Button(L10n.tr("costs_copy_account", lang: lang)) {
                                    DeviceActions.copyToClipboard(share.payeeAccount, sensitive: true)
                                }
                                .buttonStyle(GrOutlineButtonStyle())
                            }
                        }
                    }
                    .grCard()

                    if let qrId = share.payeeQrFileId, qrId > 0 {
                        VStack(spacing: 6) {
                            Text(L10n.tr("costs_qr", lang: lang)).font(GrTypography.caption).foregroundStyle(GrColors.muted)
                            AuthenticatedRemoteImage(fileId: qrId, container: container, contentMode: .fit) {
                                ProgressView()
                            }
                            .frame(maxWidth: 220, maxHeight: 220)
                        }
                        .frame(maxWidth: .infinity)
                        .grCard()
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text(L10n.tr("costs_txn_ref", lang: lang)).font(GrTypography.label)
                        TextField("e.g. DUITNOW-123456", text: Binding(
                            get: { vm.txnRef },
                            set: { vm.setTxnRef($0) }
                        ))
                        .textFieldStyle(.roundedBorder)

                        Text(L10n.tr("costs_proof_hint", lang: lang))
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.muted)

                        PhotosPicker(selection: $selectedPhoto, matching: .images) {
                            Label(
                                vm.proofData == nil ? L10n.tr("costs_upload", lang: lang) : L10n.tr("costs_change_photo", lang: lang),
                                systemImage: "photo.on.rectangle"
                            )
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                        .onChange(of: selectedPhoto) { _, item in
                            guard let item else { return }
                            Task {
                                if let prep = try? await ImageUploadPrep.prepare(from: item, prefix: "proof") {
                                    vm.setProof(data: prep.data, filename: prep.fileName, mime: prep.mimeType)
                                }
                            }
                        }

                        if let data = vm.proofData, let uiImage = UIImage(data: data) {
                            Image(uiImage: uiImage)
                                .resizable()
                                .scaledToFit()
                                .frame(maxHeight: 200)
                                .clipShape(RoundedRectangle(cornerRadius: 10))
                        }
                    }
                    .grCard()

                    if let err = vm.sheetError ?? vm.sheetErrorKey.map({ L10n.tr($0, lang: lang) }) {
                        ErrorBannerView(message: err)
                    }

                    Button {
                        Task { await vm.submitProof() }
                    } label: {
                        HStack {
                            if vm.submitting { ProgressView().tint(.white) }
                            Text(L10n.tr("costs_submit_proof", lang: lang))
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                    .disabled(vm.submitting)
                }
                .padding()
            }
            .navigationTitle(L10n.tr("costs_pay", lang: lang))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.tr("common_cancel", lang: lang)) { vm.closePay() }
                }
            }
        }
    }

    private func statusLabel(_ status: String, lang: String) -> String {
        switch status {
        case "submitted": return L10n.tr("costs_status_submitted", lang: lang)
        case "confirmed": return L10n.tr("costs_status_confirmed", lang: lang)
        default: return L10n.tr("costs_status_pending", lang: lang)
        }
    }

    private func statusFg(_ status: String) -> Color {
        switch status {
        case "submitted": return GrColors.info
        case "confirmed": return GrColors.success
        default: return GrColors.warning
        }
    }

    private func statusBg(_ status: String) -> Color {
        switch status {
        case "submitted": return GrColors.infoSoft
        case "confirmed": return GrColors.successSoft
        default: return GrColors.warningSoft
        }
    }
}

public struct NotificationsScreen: View {
    public let container: AppContainer
    public let onOpenLink: (String) -> Void

    @State private var vm: NotificationsViewModel

    private static let categories: [(id: String?, key: String)] = [
        (nil, "notif_cat_all"),
        ("ride", "notif_cat_ride"),
        ("chat", "chat_title"),
        ("event", "notif_cat_event"),
        ("cost", "notif_cat_cost"),
        ("announcement", "notif_cat_announcement"),
    ]

    public init(container: AppContainer, onOpenLink: @escaping (String) -> Void) {
        self.container = container
        self.onOpenLink = onOpenLink
        self._vm = State(initialValue: NotificationsViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang

        VStack(spacing: 0) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    Toggle(L10n.tr("notif_filter_unread", lang: lang), isOn: Binding(
                        get: { vm.unreadOnly },
                        set: { enabled in Task { await vm.setUnreadOnly(enabled) } }
                    ))
                    .toggleStyle(.button)
                    .font(GrTypography.caption)

                    ForEach(Array(Self.categories.enumerated()), id: \.offset) { _, cat in
                        let selected = vm.category == cat.id
                        Button(L10n.tr(cat.key, lang: lang)) {
                            Task { await vm.setCategory(cat.id) }
                        }
                        .font(GrTypography.caption.weight(.semibold))
                        .foregroundStyle(selected ? GrColors.primaryInk : GrColors.text)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(Capsule().fill(selected ? GrColors.primary : GrColors.surface2))
                    }
                }
                .padding(.horizontal)
                .padding(.vertical, 8)
            }
            Divider()

            if let undo = vm.deletedItemForUndo {
                CalloutBanner(
                    tone: .info,
                    icon: "trash",
                    text: L10n.tr("notif_deleted", lang: lang),
                    actionTitle: L10n.tr("notif_undo", lang: lang)
                ) {
                    vm.undoDelete(id: undo.id)
                }
                .padding(.horizontal)
                .padding(.top, 8)
            }

            if let err = vm.error {
                ErrorBannerView(message: err, onDismiss: { vm.clearError() })
                    .padding()
            }

            List {
                if vm.loading && vm.items.isEmpty {
                    LoadingSkeletonView(rows: 3)
                        .listRowSeparator(.hidden)
                } else if vm.items.isEmpty {
                    EmptyStateCard(
                        systemImage: "bell.slash",
                        title: L10n.tr(vm.unreadOnly ? "notif_empty_unread" : (vm.category != nil ? "notif_empty_category" : "notif_empty"), lang: lang),
                        subtitle: L10n.tr("notif_empty_hint", lang: lang),
                        actionTitle: (vm.unreadOnly || vm.category != nil) ? L10n.tr("notif_reset_filters", lang: lang) : nil
                    ) {
                        Task { await vm.resetFilters() }
                    }
                    .listRowSeparator(.hidden)
                } else {
                    ForEach(vm.items) { item in
                        Button {
                            Task {
                                if !item.read { await vm.toggleRead(item) }
                                if let link = item.link, !link.isEmpty {
                                    onOpenLink(link)
                                }
                            }
                        } label: {
                            HStack(alignment: .top, spacing: 10) {
                                Circle()
                                    .fill(item.read ? Color.clear : GrColors.primary)
                                    .frame(width: 8, height: 8)
                                    .padding(.top, 6)
                                VStack(alignment: .leading, spacing: 4) {
                                    HStack {
                                        Text(item.title)
                                            .font(item.read ? GrTypography.body : GrTypography.bodyMedium)
                                            .foregroundStyle(GrColors.text)
                                        Spacer()
                                        if item.priority == "urgent" {
                                            Text(L10n.tr("notif_urgent", lang: lang))
                                                .font(.system(size: 11, weight: .bold))
                                                .foregroundStyle(GrColors.danger)
                                        }
                                    }
                                    if !item.body.isEmpty {
                                        Text(item.body)
                                            .font(GrTypography.subheadline)
                                            .foregroundStyle(GrColors.muted)
                                            .lineLimit(3)
                                    }
                                    Text(KlTime.display(item.createdAt, lang: lang))
                                        .font(.system(size: 12))
                                        .foregroundStyle(GrColors.muted)
                                }
                            }
                            .padding(.vertical, 4)
                        }
                        .buttonStyle(.plain)
                        .swipeActions(edge: .trailing) {
                            Button(role: .destructive) {
                                Task { await vm.stageDelete(item) }
                            } label: {
                                Label(L10n.tr("notif_delete", lang: lang), systemImage: "trash")
                            }
                            Button {
                                Task { await vm.toggleRead(item) }
                            } label: {
                                Label(L10n.tr(item.read ? "notif_mark_unread" : "notif_mark_read", lang: lang), systemImage: "envelope.open")
                            }
                            .tint(GrColors.primary)
                        }
                    }

                    if vm.nextBefore != nil {
                        Button {
                            Task { await vm.loadMore() }
                        } label: {
                            HStack {
                                if vm.loadingMore { ProgressView() }
                                Text(L10n.tr("notif_load_more", lang: lang))
                            }
                            .frame(maxWidth: .infinity)
                        }
                    }
                }
            }
            .listStyle(.plain)
            .refreshable {
                await vm.refresh()
            }
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("notif_title", lang: lang))
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button(L10n.tr("notif_mark_all_read", lang: lang)) {
                    Task { await vm.markAllRead() }
                }
                .disabled(vm.items.isEmpty)
            }
        }
        .task {
            await vm.load(reset: true)
        }
        .onDisappear {
            Task { await vm.commitAllPendingDeletes() }
        }
    }
}
