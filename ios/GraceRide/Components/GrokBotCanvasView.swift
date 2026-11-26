import GraceRideKit
import SwiftUI
import UIKit

/// Procedural Canvas + TimelineView renderer for `GrokBotSpec` (8 shapes, 16 faces, 12 clay palettes, 15 moves).
public struct GrokBotCanvasView: View {
    public let spec: GrokBotSpec
    public var move: String = "idle"
    public var interactiveGaze: CGPoint = .zero
    public var animated: Bool = true
    public var size: CGFloat = 120

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(
        spec: GrokBotSpec,
        move: String = "idle",
        interactiveGaze: CGPoint = .zero,
        animated: Bool = true,
        size: CGFloat = 120
    ) {
        self.spec = spec
        self.move = move
        self.interactiveGaze = interactiveGaze
        self.animated = animated
        self.size = size
    }

    public var body: some View {
        Group {
            if animated && !reduceMotion {
                TimelineView(.animation) { context in
                    let t = Float(context.date.timeIntervalSinceReferenceDate)
                    Canvas { gc, canvasSize in
                        GrokBotRenderer.draw(
                            in: &gc,
                            size: canvasSize,
                            spec: spec,
                            move: move,
                            gaze: interactiveGaze,
                            time: t
                        )
                    }
                }
            } else {
                Canvas { gc, canvasSize in
                    GrokBotRenderer.draw(
                        in: &gc,
                        size: canvasSize,
                        spec: spec,
                        move: move,
                        gaze: interactiveGaze,
                        time: 0
                    )
                }
            }
        }
        .frame(width: size, height: size)
        .accessibilityLabel("Grok Bot \(spec.shape) \(spec.face)")
    }
}

public enum GrokBotExporter {
    @MainActor
    public static func renderPNG(spec: GrokBotSpec, pixelSize: CGFloat = 512) -> Data? {
        let view = Canvas { gc, canvasSize in
            GrokBotRenderer.draw(in: &gc, size: canvasSize, spec: spec, move: "idle", gaze: .zero, time: 0)
        }
        .frame(width: pixelSize, height: pixelSize)

        let renderer = ImageRenderer(content: view)
        renderer.scale = 1.0
        return renderer.uiImage?.pngData()
    }

    public static func renderSVG(spec: GrokBotSpec, size: Int = 512) -> String {
        GrokBotGeometry.buildStandaloneSvg(spec: spec, size: size)
    }
}

public enum GrokBotRenderer {
    public static func draw(
        in gc: inout GraphicsContext,
        size: CGSize,
        spec: GrokBotSpec,
        move: String,
        gaze: CGPoint,
        time: Float
    ) {
        let minDim = min(size.width, size.height)
        guard minDim > 0 else { return }
        let scaleFactor = minDim / 250.0
        let cx = size.width / 2
        let cy = size.height / 2

        let palette = GrokBotGeometry.colorPalettes[spec.color] ?? GrokBotGeometry.colorPalettes["blue"]!
        let faceDef = GrokBotGeometry.faceDefs[spec.face] ?? GrokBotGeometry.faceDefs["neutral"]!

        // Determine animated morph target, scale, rotation, translation, gaze, and blink
        let baseCoords = GrokBotGeometry.shapeCoords[spec.shape] ?? GrokBotGeometry.shapeCoords["circle"]!
        var targetCoords = baseCoords
        var morphT: Float = 0
        var bodyScale: Float = 1
        var rotDeg: Float = 0
        var offsetX: Float = 0
        var offsetY: Float = 0
        var gazeX = Float(gaze.x)
        var gazeY = Float(gaze.y)
        var blinkScaleY: Float = 1
        var leftScaleY: Float = 1
        var rightScaleY: Float = 1

        if time > 0 {
            let breath = sin(time * 2.1) * 0.016
            bodyScale += breath
            let blinkCycle = time.truncatingRemainder(dividingBy: 3.8)
            if blinkCycle > 3.62 {
                blinkScaleY = 0.08
            }

            switch move {
            case "thinking":
                gazeX += 0.45 * sin(time * 1.4)
                gazeY -= 0.35
                rotDeg = sin(time * 1.6) * 4
            case "wink":
                let wCycle = time.truncatingRemainder(dividingBy: 2.2)
                if wCycle < 0.32 { rightScaleY = 0.08 }
                rotDeg = 5 * sin(time * 2.5)
            case "wide_eyes":
                leftScaleY = 1.25
                rightScaleY = 1.25
                bodyScale += 0.04 * abs(sin(time * 4))
            case "alert":
                offsetY = -4 * abs(sin(time * 5))
                gazeX = 0.65 * sin(time * 3.5)
            case "notification":
                rotDeg = 10 * sin(time * 8) * max(0, 1 - (time.truncatingRemainder(dividingBy: 1.5)))
                offsetY = -3 * abs(sin(time * 6))
            case "exclamation":
                bodyScale += 0.06 * abs(sin(time * 6))
                offsetY = -5 * abs(sin(time * 6))
            case "sleep":
                leftScaleY = 0.14
                rightScaleY = 0.14
                blinkScaleY = 1
                offsetY = 3 * sin(time * 1.2)
                rotDeg = -4
            case "egg":
                targetCoords = GrokBotGeometry.eggCoords
                morphT = 0.5 + 0.5 * sin(time * 2.0)
            case "hexagon":
                targetCoords = GrokBotGeometry.shapeCoords["hexagon"] ?? baseCoords
                morphT = 0.5 + 0.5 * sin(time * 2.0)
            case "play":
                rotDeg = sin(time * 3) * 8
                offsetY = -4 * sin(time * 4)
            case "orbit":
                offsetX = 6 * cos(time * 2.8)
                offsetY = 6 * sin(time * 2.8)
                gazeX = 0.5 * cos(time * 2.8)
                gazeY = 0.5 * sin(time * 2.8)
            case "swirl":
                rotDeg = time * 45
            case "burst":
                let pulse = abs(sin(time * 3.5))
                bodyScale += 0.08 * pulse
            case "comet":
                offsetX = 7 * sin(time * 3.2)
                offsetY = -5 * cos(time * 3.2)
                rotDeg = -8 * sin(time * 3.2)
            default:
                gazeX += 0.15 * sin(time * 0.9)
                offsetY = 2.0 * sin(time * 1.8)
            }
        }

        var morphed = [Float](repeating: 0, count: min(baseCoords.count, targetCoords.count))
        GrokBotGeometry.lerpCoords(
            baseCoords,
            targetCoords,
            t: morphT,
            out: &morphed,
            scale: bodyScale,
            rotDeg: rotDeg,
            tx: offsetX,
            ty: offsetY
        )

        var bodyPath = pathFromClosedCubicCoords(morphed)
        bodyPath = bodyPath.applying(
            CGAffineTransform(translationX: cx, y: cy)
                .scaledBy(x: scaleFactor, y: scaleFactor)
        )

        gc.drawLayer { layer in
            // Outer soft clay glow
            let glowColor = Color(hex: palette.glowColor, alpha: 0.22)
            layer.addFilter(.shadow(color: glowColor, radius: 10 * scaleFactor, x: 0, y: 4 * scaleFactor))

            let stops = zip([0.0, 0.32, 0.72, 1.0], palette.stopColors).map { loc, hex in
                Gradient.Stop(color: Color(hex: hex), location: loc)
            }
            let gradStart = CGPoint(x: cx - 40 * scaleFactor, y: cy - 50 * scaleFactor)
            layer.fill(
                bodyPath,
                with: .radialGradient(
                    Gradient(stops: stops),
                    center: gradStart,
                    startRadius: 0,
                    endRadius: 193.75 * scaleFactor
                )
            )
        }

        // Draw expressive dark eyes aligned in body local coordinates
        let (leftM, rightM) = GrokBotGeometry.computeFaceEyeTransforms(
            face: spec.face,
            gazeX: gazeX,
            gazeY: gazeY,
            blinkScaleY: blinkScaleY,
            shape: spec.shape
        )

        let bodyFollow = CGAffineTransform(translationX: cx + CGFloat(offsetX) * scaleFactor, y: cy + CGFloat(offsetY) * scaleFactor)
            .scaledBy(x: scaleFactor, y: scaleFactor)
            .rotated(by: CGFloat(rotDeg) * .pi / 180.0)

        let leftPath = eyePath(pill: faceDef.left, extraScaleY: leftScaleY)
            .applying(affine(from: leftM))
            .applying(bodyFollow)
        let rightPath = eyePath(pill: faceDef.right, extraScaleY: rightScaleY)
            .applying(affine(from: rightM))
            .applying(bodyFollow)

        let eyeColor = Color(hex: 0x090C15, alpha: 0.92)
        gc.fill(leftPath, with: .color(eyeColor))
        gc.fill(rightPath, with: .color(eyeColor))

        // Specular eye catchlights when eyes are open
        if blinkScaleY > 0.3 && min(leftScaleY, rightScaleY) > 0.3 {
            let dotR = max(1.0, CGFloat(2.2) * scaleFactor)
            let lCenter = CGPoint(x: CGFloat(leftM.tx) - 1.5, y: CGFloat(leftM.ty) - 2.0).applying(bodyFollow)
            let rCenter = CGPoint(x: CGFloat(rightM.tx) - 1.5, y: CGFloat(rightM.ty) - 2.0).applying(bodyFollow)
            gc.fill(SwiftUI.Path(ellipseIn: CGRect(x: lCenter.x - dotR, y: lCenter.y - dotR, width: dotR * 2, height: dotR * 2)), with: .color(.white.opacity(0.65)))
            gc.fill(SwiftUI.Path(ellipseIn: CGRect(x: rCenter.x - dotR, y: rCenter.y - dotR, width: dotR * 2, height: dotR * 2)), with: .color(.white.opacity(0.65)))
        }
    }

    private static func affine(from m: GrokEyeTransform) -> CGAffineTransform {
        CGAffineTransform(
            a: CGFloat(m.a),
            b: CGFloat(m.b),
            c: CGFloat(m.c),
            d: CGFloat(m.d),
            tx: CGFloat(m.tx),
            ty: CGFloat(m.ty)
        )
    }

    private static func eyePath(pill: GrokEyePill, extraScaleY: Float) -> SwiftUI.Path {
        let w = CGFloat(pill.halfW * 2)
        let h = CGFloat(max(2.0, pill.halfH * 2 * extraScaleY))
        let rx = min(CGFloat(pill.rx), w / 2)
        let ry = min(CGFloat(pill.ry) * CGFloat(extraScaleY), h / 2)
        let rect = CGRect(x: -w / 2, y: -h / 2, width: w, height: h)
        return SwiftUI.Path(roundedRect: rect, cornerSize: CGSize(width: rx, height: ry))
    }

    /// Reconstructs closed cubic Bézier contours from flattened `[x0, y0, c1x, c1y, c2x, c2y, ex, ey, ...]` anchor+cubic-segment arrays.
    private static func pathFromClosedCubicCoords(_ coords: [Float]) -> SwiftUI.Path {
        var path = SwiftUI.Path()
        guard coords.count >= 8 else { return path }
        path.move(to: CGPoint(x: CGFloat(coords[0]), y: CGFloat(coords[1])))
        var i = 2
        while i + 5 < coords.count {
            let c1 = CGPoint(x: CGFloat(coords[i]), y: CGFloat(coords[i + 1]))
            let c2 = CGPoint(x: CGFloat(coords[i + 2]), y: CGFloat(coords[i + 3]))
            let end = CGPoint(x: CGFloat(coords[i + 4]), y: CGFloat(coords[i + 5]))
            path.addCurve(to: end, control1: c1, control2: c2)
            i += 6
        }
        path.closeSubpath()
        return path
    }
}
