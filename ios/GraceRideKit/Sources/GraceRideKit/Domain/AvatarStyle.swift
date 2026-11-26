import Foundation

public struct ParsedAvatarColor: Equatable, Sendable {
    public var bgColor: String
    public var bgShape: String
    public init(bgColor: String, bgShape: String) {
        self.bgColor = bgColor
        self.bgShape = bgShape
    }
}

public struct ParsedAvatarIcon: Equatable, Sendable {
    public var icon: String
    public var shape: String?
    public var fgColor: String?
    public init(icon: String, shape: String?, fgColor: String?) {
        self.icon = icon
        self.shape = shape
        self.fgColor = fgColor
    }
}

public enum AvatarStyleHelper {
    public static let colors = ["purple", "teal", "coral", "pink", "blue", "green", "amber", "gray", "white"]
    public static let avatarShapes = ["hexagon", "circle", "pebble", "squircle", "capsule", "triangle", "cloud", "droplet"]
    public static let avatarBgShapes = ["circle", "squircle", "rounded", "hexagon", "cloud", "pebble"]
    public static let animatedIcons = [
        "grace_car", "peace_dove", "sunny_cloud", "happy_bear",
        "fellowship_cat", "joy_puppy", "star_lantern", "blossom_sprout",
        "grok", "muse",
    ]
    public static let defaultAnimatedIcon = "grace_car"
    public static let staticIcons = [
        "car", "heart", "star", "music", "sun", "leaf",
        "coffee", "book", "church", "bike", "smile", "flower",
    ]
    public static let allIcons = staticIcons + animatedIcons

    public static func parseAvatarColor(_ raw: String?) -> ParsedAvatarColor {
        let trimmed = raw?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !trimmed.isEmpty else { return ParsedAvatarColor(bgColor: "", bgShape: "circle") }
        let parts = trimmed.split(separator: ":", omittingEmptySubsequences: false).map(String.init)
        let bg = parts.first.flatMap { colors.contains($0) ? $0 : nil } ?? ""
        let shape = (parts.count > 1 && avatarBgShapes.contains(parts[1])) ? parts[1] : "circle"
        return ParsedAvatarColor(bgColor: bg, bgShape: shape)
    }

    public static func encodeAvatarColor(_ bgColor: String?, bgShape: String? = nil) -> String {
        let bg = bgColor?.trimmingCharacters(in: .whitespacesAndNewlines).flatMap { colors.contains($0) ? $0 : nil } ?? ""
        let shape = bgShape?.trimmingCharacters(in: .whitespacesAndNewlines).flatMap { avatarBgShapes.contains($0) ? $0 : nil } ?? "circle"
        return shape == "circle" ? bg : "\(bg):\(shape)"
    }

    public static func parseAvatarIcon(_ raw: String?) -> ParsedAvatarIcon {
        let trimmed = raw?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !trimmed.isEmpty else { return ParsedAvatarIcon(icon: "", shape: nil, fgColor: nil) }
        let parts = trimmed.split(separator: ":", omittingEmptySubsequences: false).map(String.init)
        let base = parts.first ?? ""
        let icon = (base == "initials" || allIcons.contains(base)) ? base : ""
        let shape = (parts.count > 1 && avatarShapes.contains(parts[1])) ? parts[1] : nil
        let fg = (parts.count > 2 && colors.contains(parts[2])) ? parts[2] : nil
        return ParsedAvatarIcon(icon: icon, shape: shape, fgColor: fg)
    }

    public static func defaultAvatarShape(for icon: String?) -> String {
        parseAvatarIcon(icon).icon == "muse" ? "cloud" : "hexagon"
    }

    public static func isShapeableAvatar(_ icon: String?) -> Bool {
        let i = parseAvatarIcon(icon).icon
        return i == "grok" || i == "muse"
    }

    public static func encodeAvatarIcon(_ icon: String?, shape: String? = nil, fgColor: String? = nil) -> String {
        let base = icon?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let fg = fgColor?.trimmingCharacters(in: .whitespacesAndNewlines).flatMap { colors.contains($0) ? $0 : nil }
        if base.isEmpty || base == "initials" {
            return fg.map { "initials::\($0)" } ?? ""
        }
        guard allIcons.contains(base) else { return "" }
        let validShape = shape?.trimmingCharacters(in: .whitespacesAndNewlines).flatMap {
            (avatarShapes.contains($0) && (base == "grok" || base == "muse")) ? $0 : nil
        }
        let includeShape = validShape != nil && validShape != defaultAvatarShape(for: base)
        if includeShape, let validShape, let fg {
            return "\(base):\(validShape):\(fg)"
        } else if let fg {
            return "\(base)::\(fg)"
        } else if includeShape, let validShape {
            return "\(base):\(validShape)"
        }
        return base
    }

    public static func defaultColor(for icon: String?) -> String {
        switch parseAvatarIcon(icon).icon {
        case "grace_car": return "blue"
        case "peace_dove": return "teal"
        case "sunny_cloud": return "amber"
        case "happy_bear": return "coral"
        case "fellowship_cat": return "pink"
        case "joy_puppy": return "green"
        case "star_lantern": return "purple"
        case "blossom_sprout": return "pink"
        case "grok": return "gray"
        case "muse": return "amber"
        default: return "blue"
        }
    }

    public static func bundledAssetKey(icon: String?) -> String? {
        let base = parseAvatarIcon(icon).icon
        guard !base.isEmpty, base != "initials", base != "grok", base != "muse" else { return nil }
        return allIcons.contains(base) ? base : nil
    }
}

private extension String {
    func flatMap(_ transform: (String) -> String?) -> String? {
        transform(self)
    }
}
