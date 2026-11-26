import GraceRideKit
import SwiftUI

public struct EmptyStateCard: View {
    public let systemImage: String
    public let title: String
    public var subtitle: String?
    public var actionTitle: String?
    public var action: (() -> Void)?

    public init(
        systemImage: String = "tray",
        title: String,
        subtitle: String? = nil,
        actionTitle: String? = nil,
        action: (() -> Void)? = nil
    ) {
        self.systemImage = systemImage
        self.title = title
        self.subtitle = subtitle
        self.actionTitle = actionTitle
        self.action = action
    }

    public var body: some View {
        VStack(spacing: 12) {
            Image(systemName: systemImage)
                .font(.system(size: 32, weight: .regular))
                .foregroundStyle(GrColors.muted)
            Text(title)
                .font(GrTypography.titleSmall)
                .foregroundStyle(GrColors.text)
                .multilineTextAlignment(.center)
            if let subtitle, !subtitle.isEmpty {
                Text(subtitle)
                    .font(GrTypography.subheadline)
                    .foregroundStyle(GrColors.muted)
                    .multilineTextAlignment(.center)
            }
            if let actionTitle, let action {
                Button(actionTitle, action: action)
                    .buttonStyle(GrOutlineButtonStyle())
                    .padding(.top, 4)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 28)
        .padding(.horizontal, 16)
        .grCard()
    }
}

public struct ErrorBannerView: View {
    public let message: String
    public var retryTitle: String?
    public var onRetry: (() -> Void)?
    public var onDismiss: (() -> Void)?

    public init(
        message: String,
        retryTitle: String? = nil,
        onRetry: (() -> Void)? = nil,
        onDismiss: (() -> Void)? = nil
    ) {
        self.message = message
        self.retryTitle = retryTitle
        self.onRetry = onRetry
        self.onDismiss = onDismiss
    }

    public var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(GrColors.danger)
                .padding(.top, 2)
            VStack(alignment: .leading, spacing: 6) {
                Text(message)
                    .font(GrTypography.subheadline)
                    .foregroundStyle(GrColors.danger)
                if let retryTitle, let onRetry {
                    Button(retryTitle, action: onRetry)
                        .font(GrTypography.label)
                        .foregroundStyle(GrColors.primary)
                }
            }
            Spacer(minLength: 0)
            if let onDismiss {
                Button {
                    onDismiss()
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption.weight(.bold))
                        .foregroundStyle(GrColors.muted)
                }
                .grMinTouchTarget()
            }
        }
        .padding(12)
        .background(GrColors.dangerSoft)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .stroke(GrColors.danger.opacity(0.35), lineWidth: 1)
        )
    }
}

public struct CalloutBanner: View {
    public enum Tone {
        case info, warning, success, danger
    }

    public let tone: Tone
    public let icon: String
    public let text: String
    public var actionTitle: String?
    public var action: (() -> Void)?

    public init(
        tone: Tone = .info,
        icon: String = "info.circle.fill",
        text: String,
        actionTitle: String? = nil,
        action: (() -> Void)? = nil
    ) {
        self.tone = tone
        self.icon = icon
        self.text = text
        self.actionTitle = actionTitle
        self.action = action
    }

    private var fg: Color {
        switch tone {
        case .info: return GrColors.info
        case .warning: return GrColors.warning
        case .success: return GrColors.success
        case .danger: return GrColors.danger
        }
    }

    private var bg: Color {
        switch tone {
        case .info: return GrColors.infoSoft
        case .warning: return GrColors.warningSoft
        case .success: return GrColors.successSoft
        case .danger: return GrColors.dangerSoft
        }
    }

    public var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: icon)
                .foregroundStyle(fg)
            Text(text)
                .font(GrTypography.caption)
                .foregroundStyle(GrColors.text)
                .fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 4)
            if let actionTitle, let action {
                Button(actionTitle, action: action)
                    .font(GrTypography.label)
                    .foregroundStyle(fg)
            }
        }
        .padding(12)
        .background(bg)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .stroke(fg.opacity(0.3), lineWidth: 1)
        )
    }
}

public struct RideStatusBadge: View {
    public let status: String
    public let lang: String

    public init(status: String, lang: String) {
        self.status = status
        self.lang = lang
    }

    public var body: some View {
        let palette = GrStatusColors.forRideStatus(status)
        Text(L10n.rideStatus(status, lang: lang))
            .font(GrTypography.badge)
            .foregroundStyle(palette.foreground)
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .background(Capsule().fill(palette.background))
    }
}

public struct LoadingSkeletonView: View {
    public let rows: Int

    public init(rows: Int = 3) {
        self.rows = rows
    }

    public var body: some View {
        VStack(spacing: 12) {
            ForEach(0..<rows, id: \.self) { _ in
                VStack(alignment: .leading, spacing: 10) {
                    RoundedRectangle(cornerRadius: 6)
                        .fill(GrColors.surfaceHigh)
                        .frame(width: 140, height: 14)
                    RoundedRectangle(cornerRadius: 6)
                        .fill(GrColors.surfaceHigh)
                        .frame(maxWidth: .infinity)
                        .frame(height: 18)
                    RoundedRectangle(cornerRadius: 6)
                        .fill(GrColors.surface2)
                        .frame(width: 210, height: 14)
                }
                .grCard()
                .redacted(reason: .placeholder)
            }
        }
    }
}
