import AVFoundation
import CoreImage
import CoreImage.CIFilterBuiltins
import GraceRideKit
import SwiftUI
import UIKit

public enum QrGenerator {
    private static let context = CIContext()

    public static func makeUIImage(from string: String, size: CGFloat = 280) -> UIImage? {
        let data = Data(string.utf8)
        let filter = CIFilter.qrCodeGenerator()
        filter.message = data
        filter.correctionLevel = "M"

        guard let outputImage = filter.outputImage else { return nil }
        let extent = outputImage.extent.integral
        guard extent.width > 0, extent.height > 0 else { return nil }
        let scale = max(1, floor(size / extent.width))
        let transformed = outputImage.transformed(by: CGAffineTransform(scaleX: scale, y: scale))
        guard let cgImage = context.createCGImage(transformed, from: transformed.extent) else { return nil }
        return UIImage(cgImage: cgImage)
    }
}

public struct QrCodeView: View {
    public let payload: String
    public let size: CGFloat
    public let boostBrightness: Bool

    @State private var previousBrightness: CGFloat?

    public init(payload: String, size: CGFloat = 220, boostBrightness: Bool = false) {
        self.payload = payload
        self.size = size
        self.boostBrightness = boostBrightness
    }

    public var body: some View {
        Group {
            if let img = QrGenerator.makeUIImage(from: payload, size: size * 2) {
                Image(uiImage: img)
                    .interpolation(.none)
                    .resizable()
                    .scaledToFit()
                    .frame(width: size, height: size)
                    .padding(12)
                    .background(Color.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .stroke(GrColors.border, lineWidth: 1)
                    )
            } else {
                ProgressView().frame(width: size, height: size)
            }
        }
        .onAppear {
            guard boostBrightness else { return }
            UIApplication.shared.isIdleTimerDisabled = true
            previousBrightness = UIScreen.main.brightness
            UIScreen.main.brightness = max(UIScreen.main.brightness, 0.92)
        }
        .onDisappear {
            guard boostBrightness else { return }
            UIApplication.shared.isIdleTimerDisabled = false
            if let previous = previousBrightness {
                UIScreen.main.brightness = previous
            }
        }
    }
}

public struct QrScannerSheet: View {
    public let lang: String
    public let title: String
    public let onScanned: (String) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var cameraStatus: AVAuthorizationStatus = AVCaptureDevice.authorizationStatus(for: .video)
    @State private var scannerError: String?
    @State private var manualInput: String = ""

    public init(lang: String, title: String, onScanned: @escaping (String) -> Void) {
        self.lang = lang
        self.title = title
        self.onScanned = onScanned
    }

    public var body: some View {
        NavigationStack {
            VStack(spacing: 16) {
                switch cameraStatus {
                case .authorized:
                    CameraQrPreviewView { code in
                        Haptics.success()
                        onScanned(code)
                        dismiss()
                    } onError: { err in
                        scannerError = err
                    }
                    .frame(maxWidth: .infinity)
                    .frame(height: 300)
                    .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: 16, style: .continuous)
                            .stroke(GrColors.primary.opacity(0.6), lineWidth: 2)
                    )
                    if let scannerError {
                        Text(scannerError)
                            .font(GrTypography.caption)
                            .foregroundStyle(GrColors.danger)
                    }
                case .notDetermined:
                    ProgressView()
                        .task {
                            let granted = await AVCaptureDevice.requestAccess(for: .video)
                            cameraStatus = granted ? .authorized : .denied
                        }
                default:
                    VStack(spacing: 10) {
                        Image(systemName: "camera.fill")
                            .font(.largeTitle)
                            .foregroundStyle(GrColors.muted)
                        Text(L10n.tr("event_scanner_unavailable", lang: lang))
                            .font(GrTypography.body)
                            .foregroundStyle(GrColors.muted)
                            .multilineTextAlignment(.center)
                    }
                    .padding()
                    .frame(maxWidth: .infinity)
                    .grCard()
                }

                VStack(alignment: .leading, spacing: 8) {
                    Text(L10n.tr("event_code_hint", lang: lang))
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                    HStack {
                        TextField(L10n.tr("event_enter_code", lang: lang), text: $manualInput)
                            .textFieldStyle(.roundedBorder)
                            .autocorrectionDisabled()
                            .textInputAutocapitalization(.never)
                        Button(L10n.tr("common_confirm", lang: lang)) {
                            let trimmed = manualInput.trimmingCharacters(in: .whitespacesAndNewlines)
                            guard !trimmed.isEmpty else { return }
                            onScanned(trimmed)
                            dismiss()
                        }
                        .buttonStyle(GrPrimaryButtonStyle())
                    }
                }
                Spacer()
            }
            .padding()
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(L10n.tr("common_cancel", lang: lang)) { dismiss() }
                }
            }
        }
    }
}

private struct CameraQrPreviewView: UIViewControllerRepresentable {
    let onScanned: (String) -> Void
    let onError: (String) -> Void

    func makeUIViewController(context: Context) -> ScannerViewController {
        let vc = ScannerViewController()
        vc.onScanned = onScanned
        vc.onError = onError
        return vc
    }

    func updateUIViewController(_ uiViewController: ScannerViewController, context: Context) {}

    final class ScannerViewController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
        var onScanned: ((String) -> Void)?
        var onError: ((String) -> Void)?

        private let session = AVCaptureSession()
        private var previewLayer: AVCaptureVideoPreviewLayer?
        private var didEmit = false

        override func viewDidLoad() {
            super.viewDidLoad()
            view.backgroundColor = .black

            guard let device = AVCaptureDevice.default(for: .video),
                  let input = try? AVCaptureDeviceInput(device: device),
                  session.canAddInput(input) else {
                onError?("Camera unavailable")
                return
            }
            session.addInput(input)

            let output = AVCaptureMetadataOutput()
            guard session.canAddOutput(output) else {
                onError?("QR metadata output unavailable")
                return
            }
            session.addOutput(output)
            output.setMetadataObjectsDelegate(self, queue: .main)
            output.metadataObjectTypes = [.qr]

            let layer = AVCaptureVideoPreviewLayer(session: session)
            layer.videoGravity = .resizeAspectFill
            layer.frame = view.bounds
            view.layer.addSublayer(layer)
            previewLayer = layer

            DispatchQueue.global(qos: .userInitiated).async { [weak session] in
                session?.startRunning()
            }
        }

        override func viewDidLayoutSubviews() {
            super.viewDidLayoutSubviews()
            previewLayer?.frame = view.bounds
        }

        override func viewWillDisappear(_ animated: Bool) {
            super.viewWillDisappear(animated)
            if session.isRunning {
                DispatchQueue.global(qos: .userInitiated).async { [weak session] in
                    session?.stopRunning()
                }
            }
        }

        func metadataOutput(
            _ output: AVCaptureMetadataOutput,
            didOutput metadataObjects: [AVMetadataObject],
            from connection: AVCaptureConnection
        ) {
            guard !didEmit,
                  let obj = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
                  obj.type == .qr,
                  let raw = obj.stringValue,
                  !raw.isEmpty else {
                return
            }
            didEmit = true
            session.stopRunning()
            onScanned?(raw)
        }
    }
}
