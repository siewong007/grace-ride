import GraceRideKit
import PhotosUI
import SwiftUI

public struct AvatarEditorSheet: View {
    public let container: AppContainer
    public var vm: ProfileViewModel
    public let me: Me
    public let onOpenStudio: (String?, String?, String?) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var selectedBgColor: String
    @State private var selectedBgShape: String
    @State private var selectedIcon: String
    @State private var selectedPhoto: PhotosPickerItem?

    public init(
        container: AppContainer,
        vm: ProfileViewModel,
        me: Me,
        onOpenStudio: @escaping (String?, String?, String?) -> Void
    ) {
        self.container = container
        self.vm = vm
        self.me = me
        self.onOpenStudio = onOpenStudio
        let bg = AvatarStyleHelper.parseAvatarColor(me.avatarColor)
        let ic = AvatarStyleHelper.parseAvatarIcon(me.avatarIcon)
        self._selectedBgColor = State(initialValue: bg.bgColor)
        self._selectedBgShape = State(initialValue: bg.bgShape)
        self._selectedIcon = State(initialValue: ic.icon)
    }

    public var body: some View {
        let lang = container.lang
        let encodedColor = AvatarStyleHelper.encodeAvatarColor(selectedBgColor, bgShape: selectedBgShape)

        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    HStack {
                        Spacer()
                        AvatarView(
                            name: me.name,
                            photoFileId: selectedIcon.isEmpty ? me.photoFileId : nil,
                            avatarColor: encodedColor,
                            avatarIcon: selectedIcon,
                            size: 96,
                            container: container
                        )
                        Spacer()
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text(L10n.tr("profile_avatar_color", lang: lang)).font(GrTypography.label)
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack(spacing: 8) {
                                ForEach(AvatarStyleHelper.colors, id: \.self) { color in
                                    let pal = AvatarColorPalette.colors(for: color)
                                    Circle()
                                        .fill(pal.bg)
                                        .frame(width: 34, height: 34)
                                        .overlay(Circle().stroke(selectedBgColor == color ? GrColors.primary : GrColors.border, lineWidth: selectedBgColor == color ? 2.5 : 1))
                                        .onTapGesture { selectedBgColor = color }
                                }
                            }
                        }

                        Text(L10n.tr("profile_avatar_bg_shape", lang: lang)).font(GrTypography.label).padding(.top, 4)
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack(spacing: 8) {
                                ForEach(AvatarStyleHelper.avatarBgShapes, id: \.self) { shape in
                                    Button(shape.capitalized) {
                                        selectedBgShape = shape
                                    }
                                    .font(GrTypography.caption)
                                    .padding(.horizontal, 10)
                                    .padding(.vertical, 6)
                                    .background(Capsule().fill(selectedBgShape == shape ? GrColors.primary : GrColors.surface2))
                                    .foregroundStyle(selectedBgShape == shape ? GrColors.primaryInk : GrColors.text)
                                }
                            }
                        }
                    }
                    .grCard()

                    VStack(alignment: .leading, spacing: 10) {
                        Text(L10n.tr("profile_avatar_icon", lang: lang)).font(GrTypography.label)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 64))], spacing: 10) {
                            Button {
                                selectedIcon = "initials"
                            } label: {
                                AvatarView(name: me.name, avatarColor: encodedColor, avatarIcon: "initials", size: 52, container: container, animated: false)
                            }
                            .buttonStyle(.plain)

                            ForEach(AvatarStyleHelper.allIcons, id: \.self) { icon in
                                Button {
                                    selectedIcon = icon
                                } label: {
                                    AvatarView(name: me.name, avatarColor: encodedColor, avatarIcon: icon, size: 52, container: container, animated: true)
                                        .overlay(RoundedRectangle(cornerRadius: 12).stroke(selectedIcon == icon ? GrColors.primary : Color.clear, lineWidth: 2))
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                    .grCard()

                    VStack(spacing: 10) {
                        PhotosPicker(selection: $selectedPhoto, matching: .images) {
                            Label(L10n.tr("profile_avatar_choose_photo", lang: lang), systemImage: "photo")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                        .onChange(of: selectedPhoto) { _, item in
                            guard let item else { return }
                            Task {
                                if let prep = try? await ImageUploadPrep.prepare(from: item, prefix: "avatar"),
                                   await vm.saveAvatarPhoto(data: prep.data, filename: prep.fileName, mime: prep.mimeType, avatarColor: encodedColor) {
                                    dismiss()
                                }
                            }
                        }

                        Button {
                            dismiss()
                            let spec = GrokBotSpec.parse(me.avatarIcon)
                            onOpenStudio(spec?.shape, spec?.color, spec?.face)
                        } label: {
                            Label(L10n.tr("profile_open_grok_studio", lang: lang), systemImage: "sparkles")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(GrOutlineButtonStyle())
                    }
                }
                .padding()
            }
            .navigationTitle(L10n.tr("profile_edit_avatar", lang: lang))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.tr("common_cancel", lang: lang)) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(L10n.tr("profile_avatar_save", lang: lang)) {
                        Task {
                            let ok: Bool
                            if selectedIcon.isEmpty, let fid = me.photoFileId {
                                ok = await vm.saveAvatarPhotoColor(photoFileId: fid, avatarColor: encodedColor)
                            } else {
                                ok = await vm.saveAvatarStyle(avatarIcon: selectedIcon, avatarColor: encodedColor)
                            }
                            if ok { dismiss() }
                        }
                    }
                    .disabled(vm.busy)
                }
            }
        }
    }
}

public struct GrokBotStudioScreen: View {
    public let container: AppContainer
    @State private var spec: GrokBotSpec
    @State private var move: String = "idle"
    @State private var gaze: CGPoint = .zero
    @State private var animated = true
    @State private var tab = 0
    @State private var saving = false
    @State private var notice: String?
    @State private var exportURL: URL?
    @State private var showShare = false

    public init(container: AppContainer, initialSpec: GrokBotSpec = .default) {
        self.container = container
        self._spec = State(initialValue: initialSpec)
    }

    public var body: some View {
        let lang = container.lang

        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                VStack(spacing: 12) {
                    GrokBotCanvasView(spec: spec, move: move, interactiveGaze: gaze, animated: animated, size: 170)
                        .gesture(
                            DragGesture(minimumDistance: 0)
                                .onChanged { val in
                                    let dx = min(max((val.location.x - 85) / 85, -1), 1)
                                    let dy = min(max((val.location.y - 85) / 85, -1), 1)
                                    gaze = CGPoint(x: dx, y: dy)
                                }
                                .onEnded { _ in
                                    withAnimation(.spring) { gaze = .zero }
                                }
                        )

                    Text(L10n.tr("studio_cursor_hint", lang: lang))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)

                    HStack(spacing: 10) {
                        Button(L10n.tr("studio_surprise_me", lang: lang)) {
                            Haptics.selection()
                            spec = GrokBotSpec.random(excluding: spec)
                        }
                        .buttonStyle(GrOutlineButtonStyle())

                        Button {
                            Task { await saveAsAvatar(lang: lang) }
                        } label: {
                            HStack {
                                if saving { ProgressView().tint(.white) }
                                Text(L10n.tr("studio_save_avatar", lang: lang))
                            }
                        }
                        .buttonStyle(GrPrimaryButtonStyle())
                        .disabled(saving)
                    }
                }
                .frame(maxWidth: .infinity)
                .grCard()

                if let notice {
                    CalloutBanner(tone: .success, icon: "checkmark.circle.fill", text: notice)
                }

                Picker("", selection: $tab) {
                    Text(L10n.tr("studio_look_tab", lang: lang)).tag(0)
                    Text(L10n.tr("studio_move_tab", lang: lang)).tag(1)
                }
                .pickerStyle(.segmented)

                if tab == 0 {
                    VStack(alignment: .leading, spacing: 12) {
                        Text(L10n.tr("studio_shape_label", lang: lang)).font(GrTypography.label)
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack(spacing: 8) {
                                ForEach(GrokBotSpec.shapes, id: \.self) { sh in
                                    Button(L10n.tr("studio_shape_\(sh)", lang: lang)) {
                                        spec.shape = sh
                                    }
                                    .font(GrTypography.caption.weight(.semibold))
                                    .padding(.horizontal, 12)
                                    .padding(.vertical, 6)
                                    .background(Capsule().fill(spec.shape == sh ? GrColors.primary : GrColors.surface2))
                                    .foregroundStyle(spec.shape == sh ? GrColors.primaryInk : GrColors.text)
                                }
                            }
                        }

                        Text(L10n.tr("studio_color_label", lang: lang)).font(GrTypography.label)
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack(spacing: 8) {
                                ForEach(GrokBotSpec.colors, id: \.self) { col in
                                    let pal = GrokBotGeometry.colorPalettes[col] ?? GrokBotGeometry.colorPalettes["blue"]!
                                    Circle()
                                        .fill(Color(hex: pal.glowColor))
                                        .frame(width: 34, height: 34)
                                        .overlay(Circle().stroke(spec.color == col ? GrColors.text : Color.clear, lineWidth: 2.5))
                                        .onTapGesture { spec.color = col }
                                }
                            }
                        }

                        Text(L10n.tr("studio_face_label", lang: lang)).font(GrTypography.label)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 95))], spacing: 8) {
                            ForEach(GrokBotSpec.faces, id: \.self) { face in
                                Button(L10n.tr("studio_face_\(face)", lang: lang)) {
                                    spec.face = face
                                }
                                .font(GrTypography.caption)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 8)
                                .background(RoundedRectangle(cornerRadius: 8).fill(spec.face == face ? GrColors.primary : GrColors.surface2))
                                .foregroundStyle(spec.face == face ? GrColors.primaryInk : GrColors.text)
                            }
                        }
                    }
                    .grCard()
                } else {
                    VStack(alignment: .leading, spacing: 12) {
                        Toggle(L10n.tr("studio_animations_label", lang: lang), isOn: $animated)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 105))], spacing: 8) {
                            ForEach(GrokBotSpec.moves, id: \.self) { mv in
                                Button(L10n.tr("studio_move_\(mv)", lang: lang)) {
                                    move = mv
                                }
                                .font(GrTypography.caption)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 8)
                                .background(RoundedRectangle(cornerRadius: 8).fill(move == mv ? GrColors.primary : GrColors.surface2))
                                .foregroundStyle(move == mv ? GrColors.primaryInk : GrColors.text)
                            }
                        }
                    }
                    .grCard()
                }

                HStack(spacing: 10) {
                    Button(L10n.tr("studio_download_png", lang: lang)) {
                        if let png = GrokBotExporter.renderPNG(spec: spec) {
                            let url = FileManager.default.temporaryDirectory.appendingPathComponent(spec.filename(ext: "png"))
                            try? png.write(to: url)
                            exportURL = url
                            showShare = true
                        }
                    }
                    .buttonStyle(GrOutlineButtonStyle())

                    Button(L10n.tr("studio_download_svg", lang: lang)) {
                        let svg = GrokBotExporter.renderSVG(spec: spec)
                        let url = FileManager.default.temporaryDirectory.appendingPathComponent(spec.filename(ext: "svg"))
                        try? Data(svg.utf8).write(to: url)
                        exportURL = url
                        showShare = true
                    }
                    .buttonStyle(GrOutlineButtonStyle())
                }
            }
            .padding()
        }
        .background(GrColors.bg.ignoresSafeArea())
        .navigationTitle(L10n.tr("studio_title", lang: lang))
        .sheet(isPresented: $showShare) {
            if let exportURL {
                ShareSheet(items: [exportURL])
            }
        }
    }

    private func saveAsAvatar(lang: String) async {
        saving = true
        defer { saving = false }
        let vm = ProfileViewModel(container: container)
        let bg = AvatarStyleHelper.encodeAvatarColor(container.auth.me?.avatarColor, bgShape: spec.shape)
        if await vm.saveAvatarStyle(avatarIcon: spec.encode(), avatarColor: bg) {
            Haptics.success()
            notice = L10n.tr("studio_avatar_saved", lang: lang)
        }
    }
}
