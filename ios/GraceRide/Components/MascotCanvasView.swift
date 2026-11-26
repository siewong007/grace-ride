import GraceRideKit
import SwiftUI

public enum AvatarColorPalette {
    public static func colors(for name: String) -> (bg: Color, fg: Color) {
        switch name {
        case "purple": return (Color(hex: 0xEDE9FE), Color(hex: 0x5B21B6))
        case "teal": return (Color(hex: 0xCCFBF1), Color(hex: 0x0F766E))
        case "coral": return (Color(hex: 0xFFE4E6), Color(hex: 0xBE123C))
        case "pink": return (Color(hex: 0xFCE7F3), Color(hex: 0x9D174D))
        case "blue": return (Color(hex: 0xDBEAFE), Color(hex: 0x1D4ED8))
        case "green": return (Color(hex: 0xDCFCE7), Color(hex: 0x15803D))
        case "amber": return (Color(hex: 0xFEF3C7), Color(hex: 0xB45309))
        case "gray": return (Color(hex: 0xE5E7EB), Color(hex: 0x374151))
        case "white": return (Color(hex: 0xFFFFFF), Color(hex: 0x1F5DB4))
        default: return (GrColors.primarySoft, GrColors.primary)
        }
    }

    public static func solidAccent(for name: String) -> Color {
        colors(for: name).fg
    }
}

public struct AvatarContainerShape: Shape {
    public let shapeName: String

    public init(_ shapeName: String) {
        self.shapeName = shapeName
    }

    public func path(in rect: CGRect) -> SwiftUI.Path {
        switch shapeName {
        case "squircle":
            return RoundedRectangle(cornerRadius: rect.width * 0.28, style: .continuous).path(in: rect)
        case "rounded":
            return RoundedRectangle(cornerRadius: rect.width * 0.18, style: .continuous).path(in: rect)
        case "hexagon":
            return regularHexPath(in: rect)
        case "cloud":
            return cloudPath(in: rect)
        case "pebble":
            return pebblePath(in: rect)
        default:
            return Circle().path(in: rect)
        }
    }

    private func regularHexPath(in rect: CGRect) -> SwiftUI.Path {
        let r = min(rect.width, rect.height) * 0.48
        let cx = rect.midX
        let cy = rect.midY
        var path = SwiftUI.Path()
        for i in 0..<6 {
            let angle = CGFloat(i) * .pi / 3.0 - .pi / 6.0
            let pt = CGPoint(x: cx + r * cos(angle), y: cy + r * sin(angle))
            if i == 0 { path.move(to: pt) } else { path.addLine(to: pt) }
        }
        path.closeSubpath()
        return path
    }

    private func cloudPath(in rect: CGRect) -> SwiftUI.Path {
        let w = rect.width
        let h = rect.height
        var path = SwiftUI.Path()
        path.addEllipse(in: CGRect(x: w * 0.12, y: h * 0.28, width: w * 0.44, height: h * 0.48))
        path.addEllipse(in: CGRect(x: w * 0.44, y: h * 0.28, width: w * 0.44, height: h * 0.48))
        path.addEllipse(in: CGRect(x: w * 0.25, y: h * 0.12, width: w * 0.50, height: h * 0.50))
        path.addRoundedRect(in: CGRect(x: w * 0.16, y: h * 0.34, width: w * 0.68, height: h * 0.48), cornerSize: CGSize(width: w * 0.2, height: h * 0.2))
        return path
    }

    private func pebblePath(in rect: CGRect) -> SwiftUI.Path {
        let inset = rect.insetBy(dx: rect.width * 0.04, dy: rect.height * 0.06)
        return SwiftUI.Path { p in
            p.move(to: CGPoint(x: inset.minX + inset.width * 0.52, y: inset.minY))
            p.addCurve(
                to: CGPoint(x: inset.maxX, y: inset.minY + inset.height * 0.48),
                control1: CGPoint(x: inset.minX + inset.width * 0.88, y: inset.minY),
                control2: CGPoint(x: inset.maxX, y: inset.minY + inset.height * 0.18)
            )
            p.addCurve(
                to: CGPoint(x: inset.minX + inset.width * 0.44, y: inset.maxY),
                control1: CGPoint(x: inset.maxX, y: inset.minY + inset.height * 0.82),
                control2: CGPoint(x: inset.minX + inset.width * 0.78, y: inset.maxY)
            )
            p.addCurve(
                to: CGPoint(x: inset.minX, y: inset.minY + inset.height * 0.52),
                control1: CGPoint(x: inset.minX + inset.width * 0.12, y: inset.maxY),
                control2: CGPoint(x: inset.minX, y: inset.minY + inset.height * 0.80)
            )
            p.addCurve(
                to: CGPoint(x: inset.minX + inset.width * 0.52, y: inset.minY),
                control1: CGPoint(x: inset.minX, y: inset.minY + inset.height * 0.18),
                control2: CGPoint(x: inset.minX + inset.width * 0.20, y: inset.minY)
            )
            p.closeSubpath()
        }
    }
}

/// Renders animated/static `grok` (ClayBot) or `muse` (sparkle bot) preset icons inside shapeable masks.
public struct MascotPresetAvatarView: View {
    public let icon: String
    public let shape: String
    public let fgColorName: String
    public let size: CGFloat
    public let animated: Bool

    public init(icon: String, shape: String?, fgColorName: String?, size: CGFloat, animated: Bool = true) {
        self.icon = icon
        self.shape = shape ?? AvatarStyleHelper.defaultAvatarShape(for: icon)
        self.fgColorName = fgColorName ?? (icon == "muse" ? "purple" : "blue")
        self.size = size
        self.animated = animated
    }

    public var body: some View {
        if icon == "grok" {
            let botColor = mapToGrokPalette(fgColorName)
            let botShape = GrokBotSpec.shapes.contains(shape) ? shape : "hexagon"
            GrokBotCanvasView(
                spec: GrokBotSpec(shape: botShape, face: "happy", color: botColor),
                move: "idle",
                animated: animated,
                size: size
            )
        } else {
            // Muse bot: dreamy sparkle cloud/shape emblem
            let palette = AvatarColorPalette.colors(for: fgColorName)
            ZStack {
                AvatarContainerShape(shape)
                    .fill(
                        LinearGradient(
                            colors: [palette.fg, palette.fg.opacity(0.65)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                HStack(spacing: size * 0.12) {
                    Circle().fill(.white).frame(width: size * 0.12, height: size * 0.16)
                    Circle().fill(.white).frame(width: size * 0.12, height: size * 0.16)
                }
                .offset(y: -size * 0.03)
                Image(systemName: "sparkle")
                    .font(.system(size: size * 0.22, weight: .bold))
                    .foregroundStyle(.white.opacity(0.9))
                    .offset(x: size * 0.22, y: -size * 0.24)
            }
            .frame(width: size, height: size)
        }
    }

    private func mapToGrokPalette(_ colorName: String) -> String {
        switch colorName {
        case "teal": return "turquoise"
        case "coral": return "red"
        case "gray": return "grey"
        case "white": return "cream"
        case "purple", "pink", "blue", "green", "amber": return colorName
        default: return "blue"
        }
    }
}
