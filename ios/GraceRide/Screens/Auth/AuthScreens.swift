import GraceRideKit
import SwiftUI

public struct LoginScreen: View {
    @Bindable public var container: AppContainer
    @State private var vm: LoginViewModel
    @State private var passkeyManager = PasskeyAuthManager()
    @State private var showServerConfig = false
    @State private var serverInput: String = ""

    public init(container: AppContainer) {
        self.container = container
        self._vm = State(initialValue: LoginViewModel(container: container))
    }

    public var body: some View {
        let lang = container.lang
        NavigationStack {
            ScrollView {
                VStack(spacing: 20) {
                    HStack {
                        Spacer()
                        Picker("", selection: $container.lang) {
                            Text("EN").tag("en")
                            Text("中文").tag("zh")
                        }
                        .pickerStyle(.segmented)
                        .frame(width: 120)
                    }

                    VStack(spacing: 12) {
                        GraceRideBrandMark(size: 76, animated: true)
                        GraceRideWordmark(lang: lang, subtitle: L10n.tr("login_subtitle", lang: lang))
                    }
                    .padding(.top, 8)

                    VStack(alignment: .leading, spacing: 14) {
                        Text(L10n.tr("login_title", lang: lang))
                            .font(GrTypography.titleMedium)
                            .foregroundStyle(GrColors.text)

                        VStack(alignment: .leading, spacing: 6) {
                            Text(L10n.tr("login_email", lang: lang))
                                .font(GrTypography.label)
                                .foregroundStyle(GrColors.text)
                            TextField("name@church.org", text: Binding(
                                get: { vm.email },
                                set: { vm.setEmail($0) }
                            ))
                            .keyboardType(.emailAddress)
                            .textContentType(.username)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .padding(12)
                            .background(GrColors.surface2)
                            .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                            .overlay(
                                RoundedRectangle(cornerRadius: 10, style: .continuous)
                                    .stroke(GrColors.border, lineWidth: 1)
                            )
                        }

                        if let errMsg = vm.error ?? vm.errorKey.map({ L10n.tr($0, lang: lang) }) {
                            ErrorBannerView(message: errMsg) {
                                vm.clearError()
                            }
                        }

                        if vm.noPasskeyHint {
                            CalloutBanner(
                                tone: .warning,
                                icon: "person.badge.key.fill",
                                text: L10n.tr("login_no_passkey", lang: lang)
                            )
                        }

                        Button {
                            Task { await triggerPasskeyLogin(turnstileToken: nil) }
                        } label: {
                            HStack(spacing: 8) {
                                if vm.signingIn {
                                    ProgressView().tint(.white)
                                } else {
                                    Image(systemName: "person.badge.key.fill")
                                }
                                Text(L10n.tr("login_passkey", lang: lang))
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(GrPrimaryButtonStyle())
                        .disabled(vm.signingIn)
                    }
                    .grCard()

                    if vm.devAvailable {
                        VStack(alignment: .leading, spacing: 12) {
                            HStack {
                                Text(L10n.tr("login_dev_title", lang: lang))
                                    .font(GrTypography.label)
                                    .foregroundStyle(GrColors.text)
                                Spacer()
                                Button(L10n.tr("login_server_url", lang: lang)) {
                                    serverInput = vm.serverUrl
                                    showServerConfig.toggle()
                                }
                                .font(GrTypography.caption)
                                .foregroundStyle(GrColors.primary)
                            }

                            if showServerConfig {
                                VStack(spacing: 8) {
                                    TextField("http://127.0.0.1:8080/", text: $serverInput)
                                        .keyboardType(.URL)
                                        .textInputAutocapitalization(.never)
                                        .autocorrectionDisabled()
                                        .padding(10)
                                        .background(GrColors.surface2)
                                        .clipShape(RoundedRectangle(cornerRadius: 8))
                                    Button(L10n.tr("login_apply_server", lang: lang)) {
                                        Task { await vm.applyServer(serverInput) }
                                    }
                                    .buttonStyle(GrOutlineButtonStyle())
                                }
                            }

                            if !vm.devUsers.isEmpty {
                                ForEach(vm.devUsers) { user in
                                    Button {
                                        Task { await vm.devLogin(email: user.email) }
                                    } label: {
                                        HStack(spacing: 10) {
                                            AvatarView(name: user.name, size: 34, container: container, animated: false)
                                            VStack(alignment: .leading, spacing: 2) {
                                                Text(user.name)
                                                    .font(GrTypography.bodyMedium)
                                                    .foregroundStyle(GrColors.text)
                                                Text("\(user.email) · \(user.roles)")
                                                    .font(GrTypography.caption)
                                                    .foregroundStyle(GrColors.muted)
                                            }
                                            Spacer()
                                            Image(systemName: "arrow.right.circle.fill")
                                                .foregroundStyle(GrColors.primary)
                                        }
                                        .padding(10)
                                        .background(GrColors.surface2)
                                        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                                    }
                                    .buttonStyle(.plain)
                                    .disabled(vm.signingIn)
                                }
                            }
                        }
                        .grCard()
                    }
                }
                .padding()
            }
            .background(GrColors.bg.ignoresSafeArea())
            .task {
                await vm.loadTurnstileConfig()
            }
            .sheet(isPresented: Binding(
                get: { vm.turnstilePrompted },
                set: { if !$0 { vm.dismissTurnstile() } }
            )) {
                if let siteKey = vm.turnstileSiteKey {
                    TurnstileWebSheet(
                        baseURL: container.baseURL,
                        siteKey: siteKey,
                        lang: lang,
                        onToken: { token in
                            Task { await triggerPasskeyLogin(turnstileToken: token) }
                        },
                        onCancel: {
                            vm.dismissTurnstile()
                        }
                    )
                }
            }
        }
    }

    private func triggerPasskeyLogin(turnstileToken: String?) async {
        await vm.validateAndStartPasskey(turnstileToken: turnstileToken) { start in
            try await passkeyManager.signIn(start: start, fallbackRpId: container.baseURL.host)
        }
    }
}

public struct TermsScreen: View {
    public let container: AppContainer
    public let me: Me

    @State private var checkedOptIn = false
    @State private var busy = false
    @State private var error: String?

    private static let sections: [(String, String)] = [
        ("terms_service_title", "terms_service_body"),
        ("terms_controller_title", "terms_controller_body"),
        ("terms_info_title", "terms_info_body"),
        ("terms_sensitive_title", "terms_sensitive_body"),
        ("terms_location_title", "terms_location_body"),
        ("terms_purpose_title", "terms_purpose_body"),
        ("terms_who_title", "terms_who_body"),
        ("terms_see_title", "terms_see_body"),
        ("terms_third_title", "terms_third_body"),
        ("terms_external_title", "terms_external_body"),
        ("terms_retention_title", "terms_retention_body"),
        ("terms_rights_title", "terms_rights_body"),
        ("terms_conduct_title", "terms_conduct_body"),
        ("terms_drivers_title", "terms_drivers_body"),
        ("terms_ownership_title", "terms_ownership_body"),
        ("terms_changes_title", "terms_changes_body"),
        ("terms_accept_title", "terms_accept_body"),
    ]

    public init(container: AppContainer, me: Me) {
        self.container = container
        self.me = me
    }

    public var body: some View {
        let lang = container.lang
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    VStack(alignment: .leading, spacing: 6) {
                        Text(L10n.tr("terms_title", lang: lang))
                            .font(GrTypography.titleLarge)
                            .foregroundStyle(GrColors.text)
                        Text(L10n.tr("terms_updated", lang: lang))
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.primary)
                        Text(L10n.tr("terms_intro", lang: lang))
                            .font(GrTypography.body)
                            .foregroundStyle(GrColors.text)
                    }
                    .grCard()

                    ForEach(Array(Self.sections.enumerated()), id: \.offset) { _, pair in
                        VStack(alignment: .leading, spacing: 6) {
                            Text(L10n.tr(pair.0, lang: lang))
                                .font(GrTypography.titleSmall)
                                .foregroundStyle(GrColors.text)
                            Text(L10n.tr(pair.1, lang: lang))
                                .font(GrTypography.subheadline)
                                .foregroundStyle(GrColors.muted)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .grCard()
                    }

                    Toggle(isOn: $checkedOptIn) {
                        Text(L10n.tr("terms_opt_in", lang: lang))
                            .font(GrTypography.subheadline)
                            .foregroundStyle(GrColors.text)
                    }
                    .toggleStyle(.switch)
                    .tint(GrColors.primary)
                    .grCard()

                    if let error {
                        ErrorBannerView(message: error)
                    }

                    Button {
                        Task {
                            busy = true
                            error = nil
                            do {
                                try await container.auth.acceptTerms()
                                Haptics.success()
                            } catch {
                                self.error = error.localizedDescription
                            }
                            busy = false
                        }
                    } label: {
                        HStack {
                            if busy { ProgressView().tint(.white) }
                            Text(L10n.tr("terms_agree", lang: lang))
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(GrPrimaryButtonStyle())
                    .disabled(!checkedOptIn || busy)

                    Button(L10n.tr("terms_logout", lang: lang)) {
                        Task { await container.auth.logout() }
                    }
                    .buttonStyle(GrOutlineButtonStyle(tint: GrColors.muted))
                    .frame(maxWidth: .infinity)
                }
                .padding()
            }
            .background(GrColors.bg.ignoresSafeArea())
            .navigationBarTitleDisplayMode(.inline)
        }
    }
}

public struct UnlockScreen: View {
    public let container: AppContainer
    public let me: Me?

    @State private var code: String = ""
    @State private var useBackupCode = false
    @State private var busy = false
    @State private var error: String?
    @State private var passkeyManager = PasskeyAuthManager()

    public init(container: AppContainer, me: Me?) {
        self.container = container
        self.me = me
    }

    public var body: some View {
        let lang = container.lang
        NavigationStack {
            ScrollView {
                VStack(spacing: 18) {
                    GraceRideBrandMark(size: 64, animated: false)
                        .padding(.top, 12)
                    Text(L10n.tr("unlock_title", lang: lang))
                        .font(GrTypography.titleLarge)
                        .foregroundStyle(GrColors.text)
                    Text(L10n.tr("unlock_subtitle", lang: lang))
                        .font(GrTypography.subheadline)
                        .foregroundStyle(GrColors.muted)
                        .multilineTextAlignment(.center)

                    if let me {
                        HStack(spacing: 12) {
                            AvatarView(
                                name: me.name,
                                photoFileId: me.photoFileId,
                                avatarColor: me.avatarColor,
                                avatarIcon: me.avatarIcon,
                                size: 42,
                                container: container,
                                animated: false
                            )
                            VStack(alignment: .leading) {
                                Text(me.name).font(GrTypography.bodyMedium)
                                Text(me.email).font(GrTypography.caption).foregroundStyle(GrColors.muted)
                            }
                            Spacer()
                        }
                        .grCard()
                    }

                    VStack(spacing: 12) {
                        Button {
                            Task { await unlockWithPasskey() }
                        } label: {
                            Label(L10n.tr("unlock_passkey", lang: lang), systemImage: "person.badge.key.fill")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(GrPrimaryButtonStyle())
                        .disabled(busy)

                        Divider().padding(.vertical, 4)

                        Group {
                            if useBackupCode {
                                SecureField(
                                    L10n.tr("unlock_backup", lang: lang),
                                    text: $code
                                )
                                .keyboardType(.asciiCapable)
                            } else {
                                TextField(
                                    L10n.tr("unlock_code", lang: lang),
                                    text: $code
                                )
                                .keyboardType(.numberPad)
                            }
                        }
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .padding(12)
                        .background(GrColors.surface2)
                        .clipShape(RoundedRectangle(cornerRadius: 10))

                        Toggle(L10n.tr("unlock_backup", lang: lang), isOn: $useBackupCode)
                            .font(GrTypography.subheadline)

                        if let error {
                            ErrorBannerView(message: error)
                        }

                        Button(L10n.tr("unlock_submit", lang: lang)) {
                            Task { await unlockWithCode() }
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                        .disabled(busy || code.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

                        if container.debugMode {
                            Button(L10n.tr("unlock_dev_btn", lang: lang)) {
                                Task {
                                    busy = true
                                    do { try await container.auth.devUnlock() } catch { self.error = error.localizedDescription }
                                    busy = false
                                }
                            }
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.primary)
                        }
                    }
                    .grCard()

                    Button(L10n.tr("terms_logout", lang: lang)) {
                        Task { await container.auth.logout() }
                    }
                    .font(GrTypography.label)
                    .foregroundStyle(GrColors.danger)
                }
                .padding()
            }
            .background(GrColors.bg.ignoresSafeArea())
        }
    }

    private func unlockWithCode() async {
        busy = true
        error = nil
        do {
            try await container.auth.unlock(
                code: code.trimmingCharacters(in: .whitespacesAndNewlines),
                isBackupCode: useBackupCode
            )
            Haptics.success()
        } catch {
            self.error = error.localizedDescription
        }
        busy = false
    }

    private func unlockWithPasskey() async {
        busy = true
        error = nil
        do {
            let start = try await container.auth.passkeyReauthStart()
            let credential = try await passkeyManager.signIn(start: start, fallbackRpId: container.baseURL.host)
            try await container.auth.passkeyReauthFinish(challengeId: start.challengeId, credential: credential)
            Haptics.success()
        } catch PasskeyFlowError.cancelled {
            // Cancelled by user
        } catch {
            self.error = error.localizedDescription
        }
        busy = false
    }
}

public struct PendingScreen: View {
    public let container: AppContainer
    public let me: Me

    @State private var busy = false

    public init(container: AppContainer, me: Me) {
        self.container = container
        self.me = me
    }

    public var body: some View {
        let lang = container.lang
        NavigationStack {
            VStack(spacing: 20) {
                Spacer()
                GraceRideBrandMark(size: 76, animated: true)
                Text(L10n.tr("pending_title", lang: lang))
                    .font(GrTypography.titleLarge)
                    .foregroundStyle(GrColors.text)
                Text(L10n.tr("pending_body", lang: lang))
                    .font(GrTypography.body)
                    .foregroundStyle(GrColors.muted)
                    .multilineTextAlignment(.center)
                VStack(spacing: 12) {
                    Button {
                        Task {
                            busy = true
                            try? await container.auth.refresh(throwOnError: false)
                            busy = false
                        }
                    } label: {
                        HStack {
                            if busy { ProgressView() }
                            Text(L10n.tr("common_refresh", lang: lang))
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(GrPrimaryButtonStyle())

                    Button(L10n.tr("terms_logout", lang: lang)) {
                        Task { await container.auth.logout() }
                    }
                    .buttonStyle(GrOutlineButtonStyle(tint: GrColors.danger))
                }
                .padding(.top, 8)
                Spacer()
            }
            .padding()
            .background(GrColors.bg.ignoresSafeArea())
        }
    }
}
