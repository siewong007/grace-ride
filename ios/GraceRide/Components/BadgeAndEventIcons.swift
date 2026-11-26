import GraceRideKit
import SwiftUI

public struct BadgeIconView: View {
    public let badgeCode: String
    public var fallbackEmoji: String?
    public var size: CGFloat = 44
    public var animated: Bool = true

    private static let assetKeys: Set<String> = [
        "award", "trophy", "medal", "moon", "star", "heart", "shield", "crown",
        "sparkles", "flame", "zap", "car", "clock", "target", "sun", "coffee",
        "gift", "smile", "thumbsUp", "compass",
        "badge_joy_rider", "badge_fellowship_sprout", "badge_punctual_clock",
        "badge_warm_welcome", "badge_helping_wheels", "badge_traffic_light",
        "badge_route_compass", "badge_traffic_whistle", "badge_care_shield",
    ]

    public init(badgeCode: String, fallbackEmoji: String? = nil, size: CGFloat = 44, animated: Bool = true) {
        self.badgeCode = badgeCode
        self.fallbackEmoji = fallbackEmoji
        self.size = size
        self.animated = animated
    }

    public var body: some View {
        let trimmed = badgeCode.trimmingCharacters(in: .whitespacesAndNewlines)
        let filename = "\(trimmed).webp"
        if Self.assetKeys.contains(trimmed),
           WebPAssetLoader.url(category: "badges", filename: filename) != nil {
            WebPAnimatedImageView(category: "badges", filename: filename, animated: animated)
                .frame(width: size, height: size)
        } else if let emoji = (trimmed.isEmpty ? fallbackEmoji : trimmed), !emoji.isEmpty {
            Text(emoji)
                .font(.system(size: size * 0.62))
                .frame(width: size, height: size)
        } else {
            Image(systemName: "rosette")
                .font(.system(size: size * 0.54, weight: .semibold))
                .foregroundStyle(GrColors.primary)
                .frame(width: size, height: size)
                .background(Circle().fill(GrColors.primarySoft))
        }
    }
}

public struct EventIconView: View {
    public let iconName: String?
    public let category: String?
    public var size: CGFloat = 44
    public var animated: Bool = true

    private static let presetKeys: Set<String> = [
        "church", "bookOpen", "music", "flame", "heartHandshake", "sparkles",
        "utensils", "coffee", "cake", "users", "gift", "home", "trophy",
        "footprints", "waves", "bike", "tent", "mountain", "dumbbell", "target",
        "gamepad", "film", "camera", "bus", "baby", "calendar", "sun", "moon",
    ]

    public init(iconName: String?, category: String? = nil, size: CGFloat = 44, animated: Bool = true) {
        self.iconName = iconName
        self.category = category
        self.size = size
        self.animated = animated
    }

    public var body: some View {
        let trimmed = iconName?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if Self.presetKeys.contains(trimmed),
           WebPAssetLoader.url(category: "events", filename: "\(trimmed).webp") != nil {
            WebPAnimatedImageView(category: "events", filename: "\(trimmed).webp", animated: animated)
                .frame(width: size, height: size)
        } else if !trimmed.isEmpty {
            Text(trimmed)
                .font(.system(size: size * 0.65))
                .frame(width: size, height: size)
        } else {
            let fallbackKey = Self.defaultKindIcon(category)
            if WebPAssetLoader.url(category: "events", filename: "\(fallbackKey).webp") != nil {
                WebPAnimatedImageView(category: "events", filename: "\(fallbackKey).webp", animated: animated)
                    .frame(width: size, height: size)
            } else {
                Image(systemName: "calendar")
                    .font(.system(size: size * 0.5, weight: .semibold))
                    .foregroundStyle(GrColors.primary)
                    .frame(width: size, height: size)
                    .background(
                        RoundedRectangle(cornerRadius: size * 0.28, style: .continuous)
                            .fill(GrColors.primarySoft)
                    )
            }
        }
    }

    private static func defaultKindIcon(_ kind: String?) -> String {
        switch kind {
        case "badminton": return "trophy"
        case "bowling": return "target"
        case "kayak": return "waves"
        case "walk": return "footprints"
        case "other": return "sparkles"
        default: return "calendar"
        }
    }
}
