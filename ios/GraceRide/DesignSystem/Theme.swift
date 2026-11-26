import GraceRideKit
import SwiftUI

public enum GrTypography {
    public static let heroTitle = Font.system(size: 32, weight: .bold, design: .rounded)
    public static let titleLarge = Font.system(size: 24, weight: .bold)
    public static let titleMedium = Font.system(size: 20, weight: .semibold)
    public static let titleSmall = Font.system(size: 17, weight: .semibold)
    public static let body = Font.system(size: 16, weight: .regular)
    public static let bodyMedium = Font.system(size: 16, weight: .medium)
    public static let subheadline = Font.system(size: 15, weight: .regular)
    public static let label = Font.system(size: 14, weight: .semibold)
    public static let caption = Font.system(size: 14, weight: .regular) // 14pt readable floor
    public static let badge = Font.system(size: 13, weight: .semibold)
}

public struct GrCardStyle: ViewModifier {
    var backgroundColor: Color = GrColors.surfaceLowest
    var borderColor: Color = GrColors.border
    var cornerRadius: CGFloat = 14
    var padding: CGFloat = 16

    public func body(content: Content) -> some View {
        content
            .padding(padding)
            .background(backgroundColor)
            .clipShape(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .stroke(borderColor, lineWidth: 1)
            )
    }
}

public extension View {
    func grCard(
        background: Color = GrColors.surfaceLowest,
        border: Color = GrColors.border,
        cornerRadius: CGFloat = 14,
        padding: CGFloat = 16
    ) -> some View {
        modifier(GrCardStyle(
            backgroundColor: background,
            borderColor: border,
            cornerRadius: cornerRadius,
            padding: padding
        ))
    }

    func grMinTouchTarget() -> some View {
        frame(minWidth: 44, minHeight: 44)
    }
}

public struct GrPrimaryButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled
    var destructive: Bool = false

    public init(destructive: Bool = false) {
        self.destructive = destructive
    }

    public func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(GrTypography.label)
            .foregroundStyle(destructive ? Color.white : GrColors.primaryInk)
            .frame(minHeight: 46)
            .padding(.horizontal, 16)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(
                        (destructive ? GrColors.danger : GrColors.primary)
                            .opacity(isEnabled ? (configuration.isPressed ? 0.85 : 1.0) : 0.45)
                    )
            )
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}

public struct GrOutlineButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled
    var tint: Color = GrColors.primary

    public init(tint: Color = GrColors.primary) {
        self.tint = tint
    }

    public func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(GrTypography.label)
            .foregroundStyle(tint.opacity(isEnabled ? 1.0 : 0.45))
            .frame(minHeight: 44)
            .padding(.horizontal, 14)
            .background(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(configuration.isPressed ? GrColors.surfaceHigh : Color.clear)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .stroke(tint.opacity(isEnabled ? 0.45 : 0.2), lineWidth: 1)
            )
    }
}

public extension ThemeMode {
    var colorScheme: ColorScheme? {
        switch self {
        case .system: return nil
        case .light: return .light
        case .dark: return .dark
        }
    }
}

public struct GrStatusColors {
    public let foreground: Color
    public let background: Color

    public static func forRideStatus(_ status: String) -> GrStatusColors {
        switch status {
        case "pending":
            return GrStatusColors(foreground: GrColors.warning, background: GrColors.warningSoft)
        case "matched", "driver_en_route", "picked_up":
            return GrStatusColors(foreground: GrColors.info, background: GrColors.infoSoft)
        case "completed":
            return GrStatusColors(foreground: GrColors.success, background: GrColors.successSoft)
        case "cancelled":
            return GrStatusColors(foreground: GrColors.muted, background: GrColors.surface2)
        default:
            return GrStatusColors(foreground: GrColors.muted, background: GrColors.surface2)
        }
    }
}
