import GraceRideKit
import SwiftUI
import UIKit

@MainActor
public final class AuthenticatedImageCache {
    public static let shared = AuthenticatedImageCache()
    private let cache = NSCache<NSNumber, UIImage>()
    private static let ephemeralSession: URLSession = {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.urlCache = nil
        cfg.requestCachePolicy = .reloadIgnoringLocalCacheData
        cfg.httpShouldSetCookies = false
        cfg.httpCookieAcceptPolicy = .never
        return URLSession(configuration: cfg)
    }()

    private init() {
        cache.countLimit = 128
        NotificationCenter.default.addObserver(
            forName: AppContainer.clearImageCacheNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated {
                self?.cache.removeAllObjects()
            }
        }
    }

    public func clearAll() {
        cache.removeAllObjects()
    }

    nonisolated static var urlSession: URLSession { ephemeralSession }

    public func image(forFileId id: Int64) -> UIImage? {
        cache.object(forKey: NSNumber(value: id))
    }

    public func set(_ image: UIImage, forFileId id: Int64) {
        cache.setObject(image, forKey: NSNumber(value: id))
    }
}

/// Fetches an authenticated `/api/files/{id}` image using `AppContainer`'s session cookies.
public struct AuthenticatedRemoteImage<Placeholder: View>: View {
    public let fileId: Int64
    public let container: AppContainer?
    public let contentMode: ContentMode
    @ViewBuilder public let placeholder: () -> Placeholder

    @State private var loadedImage: UIImage?
    @State private var attempted = false

    public init(
        fileId: Int64,
        container: AppContainer?,
        contentMode: ContentMode = .fill,
        @ViewBuilder placeholder: @escaping () -> Placeholder
    ) {
        self.fileId = fileId
        self.container = container
        self.contentMode = contentMode
        self.placeholder = placeholder
    }

    public var body: some View {
        Group {
            if let img = loadedImage ?? AuthenticatedImageCache.shared.image(forFileId: fileId) {
                Image(uiImage: img)
                    .resizable()
                    .aspectRatio(contentMode: contentMode)
            } else {
                placeholder()
            }
        }
        .task(id: fileId) {
            if let cached = AuthenticatedImageCache.shared.image(forFileId: fileId) {
                loadedImage = cached
                return
            }
            guard !attempted, let container, let url = container.fileURL(id: fileId) else { return }
            attempted = true
            var request = URLRequest(url: url, cachePolicy: .reloadIgnoringLocalCacheData)
            request.httpMethod = "GET"
            if let cookieHeader = container.cookieJar.cookieHeader(for: url) {
                request.setValue(cookieHeader, forHTTPHeaderField: "Cookie")
            }
            if let (data, response) = try? await AuthenticatedImageCache.urlSession.data(for: request),
               let http = response as? HTTPURLResponse,
               (200..<300).contains(http.statusCode),
               let img = UIImage(data: data) {
                AuthenticatedImageCache.shared.set(img, forFileId: fileId)
                loadedImage = img
            }
        }
    }
}

/// Unified avatar renderer supporting uploaded photos, GrokBot SVG/Canvas specs,
/// animated/static WebP presets, SF icons, and colored initials fallback.
public struct AvatarView: View {
    public let name: String
    public var photoFileId: Int64?
    public var avatarColor: String?
    public var avatarIcon: String?
    public var size: CGFloat = 44
    public var container: AppContainer?
    public var animated: Bool = true

    public init(
        name: String,
        photoFileId: Int64? = nil,
        avatarColor: String? = nil,
        avatarIcon: String? = nil,
        size: CGFloat = 44,
        container: AppContainer? = nil,
        animated: Bool = true
    ) {
        self.name = name
        self.photoFileId = photoFileId
        self.avatarColor = avatarColor
        self.avatarIcon = avatarIcon
        self.size = size
        self.container = container
        self.animated = animated
    }

    public var body: some View {
        let bgParsed = AvatarStyleHelper.parseAvatarColor(avatarColor)
        let shape = AvatarContainerShape(bgParsed.bgShape)
        let bgPalette = AvatarColorPalette.colors(for: bgParsed.bgColor)

        ZStack {
            if let photoFileId, photoFileId > 0 {
                AuthenticatedRemoteImage(fileId: photoFileId, container: container, contentMode: .fill) {
                    avatarContent(bgPalette: bgPalette)
                }
                .frame(width: size, height: size)
                .clipShape(shape)
            } else if let grokSpec = GrokBotSpec.parse(avatarIcon) {
                shape.fill(bgPalette.bg.opacity(bgParsed.bgColor.isEmpty ? 0.35 : 0.9))
                GrokBotCanvasView(spec: grokSpec, move: "idle", animated: animated, size: size * 0.84)
            } else {
                shape.fill(bgPalette.bg)
                avatarContent(bgPalette: bgPalette)
            }
        }
        .frame(width: size, height: size)
        .clipShape(shape)
        .overlay(shape.stroke(GrColors.border.opacity(0.65), lineWidth: 1))
        .accessibilityLabel(name.isEmpty ? "Avatar" : name)
    }

    @ViewBuilder
    private func avatarContent(bgPalette: (bg: Color, fg: Color)) -> some View {
        let iconParsed = AvatarStyleHelper.parseAvatarIcon(avatarIcon)
        let iconFgName = iconParsed.fgColor ?? AvatarStyleHelper.defaultColor(for: avatarIcon)
        let fgColor = AvatarColorPalette.solidAccent(for: iconFgName)

        if iconParsed.icon == "grok" || iconParsed.icon == "muse" {
            MascotPresetAvatarView(
                icon: iconParsed.icon,
                shape: iconParsed.shape,
                fgColorName: iconFgName,
                size: size * 0.78,
                animated: animated
            )
        } else if let assetKey = AvatarStyleHelper.bundledAssetKey(icon: avatarIcon),
                  WebPAssetLoader.url(category: "avatars", filename: "\(assetKey).webp") != nil {
            WebPAnimatedImageView(category: "avatars", filename: "\(assetKey).webp", animated: animated)
                .frame(width: size * 0.78, height: size * 0.78)
        } else if let sfSymbol = Self.sfSymbol(for: iconParsed.icon) {
            Image(systemName: sfSymbol)
                .font(.system(size: size * 0.44, weight: .semibold))
                .foregroundStyle(iconParsed.fgColor != nil ? fgColor : bgPalette.fg)
        } else {
            Text(Self.initials(name))
                .font(.system(size: max(12, size * 0.38), weight: .bold, design: .rounded))
                .foregroundStyle(iconParsed.fgColor != nil ? fgColor : bgPalette.fg)
        }
    }

    public static func initials(_ name: String) -> String {
        let parts = name.trimmingCharacters(in: .whitespacesAndNewlines)
            .split(whereSeparator: \.isWhitespace)
            .map(String.init)
        if parts.isEmpty { return "?" }
        if parts.count == 1 {
            return String(parts[0].prefix(2)).uppercased()
        }
        let first = parts.first?.first.map(String.init) ?? ""
        let last = parts.last?.first.map(String.init) ?? ""
        return (first + last).uppercased()
    }

    private static func sfSymbol(for icon: String) -> String? {
        switch icon {
        case "car": return "car.fill"
        case "heart": return "heart.fill"
        case "star": return "star.fill"
        case "music": return "music.note"
        case "sun": return "sun.max.fill"
        case "leaf": return "leaf.fill"
        case "coffee": return "cup.and.saucer.fill"
        case "book": return "book.fill"
        case "church": return "building.columns.fill"
        case "bike": return "bicycle"
        case "smile": return "face.smiling.fill"
        case "flower": return "camera.macro"
        default: return nil
        }
    }
}
