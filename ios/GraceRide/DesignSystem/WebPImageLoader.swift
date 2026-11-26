import ImageIO
import SwiftUI
import UIKit

/// Decodes bundled static and animated WebP assets (`Resources/Icons/{avatars,badges,events}/*.webp`)
/// using Apple's native ImageIO without third-party dependencies.
public enum WebPAssetLoader {
    private static let cache: NSCache<NSString, UIImage> = {
        let c = NSCache<NSString, UIImage>()
        c.countLimit = 96
        return c
    }()

    public static func url(category: String, filename: String, bundle: Bundle = .main) -> URL? {
        guard !filename.isEmpty else { return nil }
        let cleanName: String
        let ext: String
        if filename.lowercased().hasSuffix(".webp") {
            cleanName = String(filename.dropLast(5))
            ext = "webp"
        } else {
            cleanName = filename
            ext = "webp"
        }
        if let direct = bundle.url(forResource: cleanName, withExtension: ext, subdirectory: "Icons/\(category)") {
            return direct
        }
        return bundle.url(forResource: cleanName, withExtension: ext)
    }

    public static func image(category: String, filename: String, animated: Bool = true, bundle: Bundle = .main) -> UIImage? {
        let cacheKey = "\(category)/\(filename)#\(animated)" as NSString
        if let cached = cache.object(forKey: cacheKey) {
            return cached
        }
        guard let fileURL = url(category: category, filename: filename, bundle: bundle),
              let data = try? Data(contentsOf: fileURL),
              let source = CGImageSourceCreateWithData(data as CFData, nil) else {
            return nil
        }

        let count = CGImageSourceGetCount(source)
        guard count > 0 else { return nil }

        if !animated || count == 1 {
            guard let cgImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else { return nil }
            let image = UIImage(cgImage: cgImage)
            cache.setObject(image, forKey: cacheKey)
            return image
        }

        var frames: [UIImage] = []
        var totalDuration: Double = 0
        frames.reserveCapacity(count)

        for index in 0..<count {
            guard let cgImage = CGImageSourceCreateImageAtIndex(source, index, nil) else { continue }
            let frameDelay = frameDuration(source: source, index: index)
            totalDuration += frameDelay
            frames.append(UIImage(cgImage: cgImage))
        }

        guard !frames.isEmpty else { return nil }
        let result: UIImage?
        if frames.count == 1 {
            result = frames[0]
        } else {
            result = UIImage.animatedImage(with: frames, duration: max(totalDuration, Double(frames.count) * 0.06))
        }
        if let result {
            cache.setObject(result, forKey: cacheKey)
        }
        return result
    }

    private static func frameDuration(source: CGImageSource, index: Int) -> Double {
        guard let props = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any] else {
            return 0.08
        }
        if let webp = props[kCGImagePropertyWebPDictionary] as? [CFString: Any] {
            if let unclamped = webp[kCGImagePropertyWebPUnclampedDelayTime] as? Double, unclamped > 0.01 {
                return unclamped
            }
            if let delay = webp[kCGImagePropertyWebPDelayTime] as? Double, delay > 0.01 {
                return delay
            }
        }
        if let gif = props[kCGImagePropertyGIFDictionary] as? [CFString: Any] {
            if let unclamped = gif[kCGImagePropertyGIFUnclampedDelayTime] as? Double, unclamped > 0.01 {
                return unclamped
            }
            if let delay = gif[kCGImagePropertyGIFDelayTime] as? Double, delay > 0.01 {
                return delay
            }
        }
        return 0.08
    }
}

public struct WebPAnimatedImageView: UIViewRepresentable {
    public let category: String
    public let filename: String
    public let animated: Bool
    public let contentMode: UIView.ContentMode

    public init(category: String, filename: String, animated: Bool = true, contentMode: UIView.ContentMode = .scaleAspectFit) {
        self.category = category
        self.filename = filename
        self.animated = animated
        self.contentMode = contentMode
    }

    public func makeUIView(context: Context) -> UIImageView {
        let view = UIImageView()
        view.contentMode = contentMode
        view.clipsToBounds = true
        view.backgroundColor = .clear
        view.setContentHuggingPriority(.defaultLow, for: .horizontal)
        view.setContentHuggingPriority(.defaultLow, for: .vertical)
        view.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        view.setContentCompressionResistancePriority(.defaultLow, for: .vertical)
        return view
    }

    public func updateUIView(_ uiView: UIImageView, context: Context) {
        uiView.contentMode = contentMode
        uiView.image = WebPAssetLoader.image(category: category, filename: filename, animated: animated)
    }
}
