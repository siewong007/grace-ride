import SwiftUI

/// Animated Grace Ride brand emblem: teal-blue location pin housing the red cross of grace,
/// dual live-signal orbital arcs, orbiting member dots, and radiating welcome ripples.
public struct GraceRideBrandMark: View {
    public let size: CGFloat
    public let animated: Bool

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var pulse: Bool = false

    public init(size: CGFloat = 56, animated: Bool = true) {
        self.size = size
        self.animated = animated
    }

    public var body: some View {
        ZStack {
            if animated && !reduceMotion {
                TimelineView(.animation) { context in
                    let t = context.date.timeIntervalSinceReferenceDate
                    Canvas { gc, canvasSize in
                        drawMark(in: &gc, size: canvasSize, phase: t)
                    }
                }
            } else {
                Canvas { gc, canvasSize in
                    drawMark(in: &gc, size: canvasSize, phase: 0)
                }
            }
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }

    private func drawMark(in gc: inout GraphicsContext, size: CGSize, phase: TimeInterval) {
        let s = min(size.width, size.height)
        let center = CGPoint(x: size.width / 2, y: size.height / 2)

        // Outer soft halo
        let haloRadius = s * 0.44
        let haloRect = CGRect(x: center.x - haloRadius, y: center.y - haloRadius, width: haloRadius * 2, height: haloRadius * 2)
        gc.fill(SwiftUI.Path(ellipseIn: haloRect), with: .color(GrColors.primarySoft))

        // Animated ripples
        if phase > 0 {
            let rippleProgress = (phase.truncatingRemainder(dividingBy: 2.6)) / 2.6
            let rRadius = s * (0.28 + 0.18 * CGFloat(rippleProgress))
            let rRect = CGRect(x: center.x - rRadius, y: center.y - rRadius, width: rRadius * 2, height: rRadius * 2)
            gc.stroke(
                SwiftUI.Path(ellipseIn: rRect),
                with: .color(GrColors.primary.opacity(0.28 * (1.0 - rippleProgress))),
                lineWidth: max(1.2, s * 0.025)
            )
        }

        // Signal arcs around pin
        let arcRadius = s * 0.36
        let arcStroke = StrokeStyle(lineWidth: max(1.8, s * 0.036), lineCap: .round)
        var leftArc = SwiftUI.Path()
        leftArc.addArc(center: center, radius: arcRadius, startAngle: .degrees(142), endAngle: .degrees(218), clockwise: false)
        gc.stroke(leftArc, with: .color(GrColors.primary.opacity(0.65)), style: arcStroke)

        var rightArc = SwiftUI.Path()
        rightArc.addArc(center: center, radius: arcRadius, startAngle: .degrees(-38), endAngle: .degrees(38), clockwise: false)
        gc.stroke(rightArc, with: .color(GrColors.primary.opacity(0.65)), style: arcStroke)

        // Orbiting member dots
        let orbitAngle = phase > 0 ? phase * 0.9 : 0.4
        let dotR = max(2.2, s * 0.048)
        for offset in [0.0, .pi] {
            let a = orbitAngle + offset
            let dx = center.x + cos(a) * arcRadius
            let dy = center.y + sin(a) * arcRadius
            let dotRect = CGRect(x: dx - dotR, y: dy - dotR, width: dotR * 2, height: dotR * 2)
            gc.fill(SwiftUI.Path(ellipseIn: dotRect), with: .color(offset == 0 ? GrColors.brandRed : GrColors.primary))
        }

        // Teardrop pin silhouette
        let pinW = s * 0.44
        let pinH = s * 0.56
        let pinTopY = center.y - pinH * 0.48
        let r = pinW / 2
        let cx = center.x
        let circleCY = pinTopY + r
        let tipY = pinTopY + pinH
        var pinPath = SwiftUI.Path()
        pinPath.move(to: CGPoint(x: cx, y: tipY))
        pinPath.addQuadCurve(
            to: CGPoint(x: cx - r, y: circleCY),
            control: CGPoint(x: cx - r * 0.98, y: circleCY + r * 0.85)
        )
        pinPath.addArc(
            center: CGPoint(x: cx, y: circleCY),
            radius: r,
            startAngle: .degrees(180),
            endAngle: .degrees(0),
            clockwise: false
        )
        pinPath.addQuadCurve(
            to: CGPoint(x: cx, y: tipY),
            control: CGPoint(x: cx + r * 0.98, y: circleCY + r * 0.85)
        )
        pinPath.closeSubpath()
        gc.fill(pinPath, with: .color(GrColors.primary))

        // White inner badge + BrandRed cross
        let innerR = r * 0.64
        let innerRect = CGRect(x: cx - innerR, y: circleCY - innerR, width: innerR * 2, height: innerR * 2)
        gc.fill(SwiftUI.Path(ellipseIn: innerRect), with: .color(.white))

        let armLen = innerR * 1.18
        let armThick = innerR * 0.38
        let vBar = RoundedRectangle(cornerRadius: armThick / 3)
            .path(in: CGRect(x: cx - armThick / 2, y: circleCY - armLen / 2, width: armThick, height: armLen))
        let hBar = RoundedRectangle(cornerRadius: armThick / 3)
            .path(in: CGRect(x: cx - armLen / 2, y: circleCY - armThick / 2, width: armLen, height: armThick))
        gc.fill(vBar, with: .color(GrColors.brandRed))
        gc.fill(hBar, with: .color(GrColors.brandRed))
    }
}

public struct GraceRideWordmark: View {
    public let lang: String
    public var subtitle: String?

    public init(lang: String, subtitle: String? = nil) {
        self.lang = lang
        self.subtitle = subtitle
    }

    public var body: some View {
        VStack(spacing: 4) {
            HStack(spacing: 8) {
                Text("恩典同行")
                    .font(.system(size: 26, weight: .bold, design: .rounded))
                    .foregroundStyle(GrColors.text)
                Text("·")
                    .font(.system(size: 22, weight: .semibold))
                    .foregroundStyle(GrColors.muted)
                Text("Grace Ride")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(GrColors.primary)
            }
            if let subtitle, !subtitle.isEmpty {
                Text(subtitle)
                    .font(GrTypography.subheadline)
                    .foregroundStyle(GrColors.muted)
                    .multilineTextAlignment(.center)
            }
        }
    }
}

public struct GraceRideSplashOverlay: View {
    public let lang: String

    public init(lang: String) {
        self.lang = lang
    }

    public var body: some View {
        ZStack {
            GrColors.bg.ignoresSafeArea()
            VStack(spacing: 18) {
                GraceRideBrandMark(size: 108, animated: true)
                GraceRideWordmark(
                    lang: lang,
                    subtitle: L10n.tr("splash_tagline", lang: lang)
                )
                ProgressView()
                    .tint(GrColors.primary)
                    .padding(.top, 8)
            }
            .padding()
        }
    }
}
