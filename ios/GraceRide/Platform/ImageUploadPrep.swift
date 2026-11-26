import ImageIO
import PhotosUI
import SwiftUI
import UIKit
import UniformTypeIdentifiers

public struct PreparedImageUpload: Sendable {
    public let data: Data
    public let fileName: String
    public let mimeType: String

    public init(data: Data, fileName: String, mimeType: String = "image/jpeg") {
        self.data = data
        self.fileName = fileName
        self.mimeType = mimeType
    }
}

/// Resizes selected photos to max 1600px on the longest edge and re-encodes as clean JPEG
/// (implicitly stripping EXIF GPS/device metadata). Matches Android `ImageUploadPrep`.
public enum ImageUploadPrep {
    public static let maxDimension: CGFloat = 1600
    public static let maxBytes: Int = 5 * 1024 * 1024

    public static func prepare(from item: PhotosPickerItem, prefix: String = "upload") async throws -> PreparedImageUpload? {
        guard let rawData = try await item.loadTransferable(type: Data.self) else {
            return nil
        }
        return prepare(rawData: rawData, prefix: prefix)
    }

    public static func prepare(rawData: Data, prefix: String = "upload") -> PreparedImageUpload? {
        guard let source = CGImageSourceCreateWithData(rawData as CFData, nil) else {
            return nil
        }

        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: Int(maxDimension),
        ]

        guard let cgImage = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else {
            guard let fallback = UIImage(data: rawData) else { return nil }
            return encodeJPEG(image: fallback, prefix: prefix)
        }

        return encodeJPEG(image: UIImage(cgImage: cgImage), prefix: prefix)
    }

    private static func encodeJPEG(image: UIImage, prefix: String) -> PreparedImageUpload? {
        var quality: CGFloat = 0.85
        var jpegData = image.jpegData(compressionQuality: quality)
        while let data = jpegData, data.count > maxBytes, quality > 0.35 {
            quality -= 0.12
            jpegData = image.jpegData(compressionQuality: quality)
        }
        guard let finalData = jpegData else { return nil }
        return PreparedImageUpload(
            data: finalData,
            fileName: "\(prefix)-\(Int(Date().timeIntervalSince1970)).jpg",
            mimeType: "image/jpeg"
        )
    }
}
