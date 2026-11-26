import SwiftUI
import UIKit

public extension Color {
    init(hex: UInt32, alpha: Double = 1.0) {
        let r = Double((hex >> 16) & 0xFF) / 255.0
        let g = Double((hex >> 8) & 0xFF) / 255.0
        let b = Double(hex & 0xFF) / 255.0
        self.init(.sRGB, red: r, green: g, blue: b, opacity: alpha)
    }

    static func adaptive(light: UInt32, dark: UInt32, alpha: Double = 1.0) -> Color {
        Color(uiColor: UIColor { traits in
            let hex = traits.userInterfaceStyle == .dark ? dark : light
            return UIColor(
                red: CGFloat((hex >> 16) & 0xFF) / 255.0,
                green: CGFloat((hex >> 8) & 0xFF) / 255.0,
                blue: CGFloat(hex & 0xFF) / 255.0,
                alpha: alpha
            )
        })
    }
}

/// Design tokens matching `frontend/src/styles.css` and `android/.../ui/theme/Color.kt` —
/// one background, one accent, borders instead of heavy shadows.
public enum GrColors {
    public static let primary = Color.adaptive(light: 0x1F5DB4, dark: 0x7AA7EE)
    public static let primaryHover = Color.adaptive(light: 0x194E9C, dark: 0x9ABDF3)
    public static let primarySoft = Color.adaptive(light: 0xEEF3FB, dark: 0x1B2432)
    public static let primaryInk = Color.adaptive(light: 0xFFFFFF, dark: 0x0B1730)

    public static let bg = Color.adaptive(light: 0xFFFFFF, dark: 0x141517)
    public static let surface2 = Color.adaptive(light: 0xF4F4F3, dark: 0x1F2124)
    public static let surfaceLowest = Color.adaptive(light: 0xFFFFFF, dark: 0x0E0F11)
    public static let surfaceLow = Color.adaptive(light: 0xFAF9F7, dark: 0x191B1E)
    public static let surfaceContainer = Color.adaptive(light: 0xF5F5F4, dark: 0x1F2124)
    public static let surfaceHigh = Color.adaptive(light: 0xEFEFEE, dark: 0x272A2E)
    public static let surfaceHighest = Color.adaptive(light: 0xE8E8E6, dark: 0x30343A)
    public static let border = Color.adaptive(light: 0xE6E6E6, dark: 0x2E3135)
    public static let text = Color.adaptive(light: 0x1A1A1A, dark: 0xEDEDED)
    public static let muted = Color.adaptive(light: 0x6B6B6B, dark: 0xA3A3A3)

    public static let success = Color.adaptive(light: 0x2B7549, dark: 0x8FD4AC)
    public static let successSoft = Color.adaptive(light: 0xEEF6F1, dark: 0x1B2A22)
    public static let warning = Color.adaptive(light: 0x8A5800, dark: 0xE0B36A)
    public static let warningSoft = Color.adaptive(light: 0xFBF4E6, dark: 0x2C2417)
    public static let danger = Color.adaptive(light: 0xB4372F, dark: 0xF0938B)
    public static let dangerSoft = Color.adaptive(light: 0xFBEEEC, dark: 0x2E1D1B)
    public static let info = Color.adaptive(light: 0x2C5D9C, dark: 0x9DBCE8)
    public static let infoSoft = Color.adaptive(light: 0xEEF3FB, dark: 0x1B2432)

    public static let brandRed = Color(hex: 0xE8462F)

    public static let surfaceColors: (bg: Color, card: Color, subtle: Color, border: Color, text: Color, muted: Color) = (
        bg, surfaceLowest, surface2, border, text, muted
    )
}
