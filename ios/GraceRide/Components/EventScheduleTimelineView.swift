import GraceRideKit
import SwiftUI

public enum TimelineKind: Sendable {
    case signup, ride, returnRide, drive, event, checkin
}

public enum TimelineState: Sendable {
    case active, upcoming, done
}

public struct TimelineBadge: Sendable {
    public let text: String
    public let kind: RideStatusKind

    public init(text: String, kind: RideStatusKind) {
        self.text = text
        self.kind = kind
    }
}

public struct TimelineNode: Identifiable {
    public let id: String
    public let timeLabel: String
    public let title: String
    public let subtitle: String?
    public let kind: TimelineKind
    public let state: TimelineState
    public let badge: TimelineBadge?
    public let eventIcon: String?
    public let eventKind: String
    public let onClick: (() -> Void)?

    public init(
        id: String,
        timeLabel: String,
        title: String,
        subtitle: String? = nil,
        kind: TimelineKind,
        state: TimelineState,
        badge: TimelineBadge? = nil,
        eventIcon: String? = nil,
        eventKind: String = "",
        onClick: (() -> Void)? = nil
    ) {
        self.id = id
        self.timeLabel = timeLabel
        self.title = title
        self.subtitle = subtitle
        self.kind = kind
        self.state = state
        self.badge = badge
        self.eventIcon = eventIcon
        self.eventKind = eventKind
        self.onClick = onClick
    }
}

public struct DayOption: Identifiable, Hashable {
    public var id: String { key }
    public let key: String
    public let weekday: String
    public let dateLabel: String

    public init(key: String, weekday: String, dateLabel: String) {
        self.key = key
        self.weekday = weekday
        self.dateLabel = dateLabel
    }
}

public struct EventScheduleTimelineView: View {
    public let days: [DayOption]
    @Binding public var selectedDayKey: String
    public let nodes: [TimelineNode]
    public let emptyText: String

    public init(
        days: [DayOption],
        selectedDayKey: Binding<String>,
        nodes: [TimelineNode],
        emptyText: String
    ) {
        self.days = days
        self._selectedDayKey = selectedDayKey
        self.nodes = nodes
        self.emptyText = emptyText
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if !days.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(days) { day in
                            let selected = day.key == selectedDayKey
                            Button {
                                Haptics.selection()
                                selectedDayKey = day.key
                            } label: {
                                VStack(spacing: 2) {
                                    Text(day.weekday)
                                        .font(.system(size: 12, weight: .semibold))
                                    Text(day.dateLabel)
                                        .font(.system(size: 14, weight: .bold))
                                }
                                .foregroundStyle(selected ? GrColors.primaryInk : GrColors.text)
                                .padding(.horizontal, 14)
                                .padding(.vertical, 8)
                                .background(
                                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                                        .fill(selected ? GrColors.primary : GrColors.surface2)
                                )
                                .overlay(
                                    RoundedRectangle(cornerRadius: 12, style: .continuous)
                                        .stroke(selected ? GrColors.primary : GrColors.border, lineWidth: 1)
                                )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }

            if nodes.isEmpty {
                Text(emptyText)
                    .font(GrTypography.subheadline)
                    .foregroundStyle(GrColors.muted)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 8)
            } else {
                VStack(spacing: 0) {
                    ForEach(Array(nodes.enumerated()), id: \.element.id) { idx, node in
                        TimelineRowView(node: node, isLast: idx == nodes.count - 1)
                    }
                }
            }
        }
    }
}

private struct TimelineRowView: View {
    let node: TimelineNode
    let isLast: Bool

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(spacing: 0) {
                ZStack {
                    Circle()
                        .fill(nodeBg)
                        .frame(width: 30, height: 30)
                        .overlay(Circle().stroke(nodeBorder, lineWidth: 1.5))
                    if let iconName = node.eventIcon, !iconName.isEmpty, node.kind == .event {
                        EventIconView(iconName: iconName, category: node.eventKind, size: 18, animated: true)
                    } else {
                        Image(systemName: symbolName)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(nodeTint)
                    }
                }
                if !isLast {
                    Rectangle()
                        .fill(GrColors.border)
                        .frame(width: 2)
                        .frame(maxHeight: .infinity)
                        .padding(.vertical, 2)
                }
            }
            .frame(width: 30)

            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 8) {
                    Text(node.timeLabel)
                        .font(GrTypography.label)
                        .foregroundStyle(node.state == .done ? GrColors.muted : GrColors.primary)
                    Spacer()
                    if let badge = node.badge {
                        badgePill(badge)
                    }
                }
                Text(node.title)
                    .font(GrTypography.bodyMedium)
                    .foregroundStyle(node.state == .done ? GrColors.muted : GrColors.text)
                if let sub = node.subtitle, !sub.isEmpty {
                    Text(sub)
                        .font(GrTypography.caption)
                        .foregroundStyle(GrColors.muted)
                }
            }
            .padding(.bottom, isLast ? 4 : 16)
            .contentShape(Rectangle())
            .onTapGesture {
                node.onClick?()
            }
        }
    }

    private var symbolName: String {
        switch node.kind {
        case .signup: return "person.badge.plus"
        case .ride: return "car.fill"
        case .returnRide: return "arrow.uturn.backward"
        case .drive: return "steeringwheel"
        case .event: return "calendar"
        case .checkin: return "qrcode.viewfinder"
        }
    }

    private var nodeTint: Color {
        switch node.state {
        case .active: return GrColors.primary
        case .upcoming: return GrColors.text
        case .done: return GrColors.muted
        }
    }

    private var nodeBg: Color {
        switch node.state {
        case .active: return GrColors.primarySoft
        case .upcoming: return GrColors.surfaceLowest
        case .done: return GrColors.surface2
        }
    }

    private var nodeBorder: Color {
        switch node.state {
        case .active: return GrColors.primary
        case .upcoming: return GrColors.border
        case .done: return GrColors.border
        }
    }

    @ViewBuilder
    private func badgePill(_ badge: TimelineBadge) -> some View {
        let fg: Color = switch badge.kind {
        case .active: GrColors.info
        case .pending: GrColors.warning
        case .done: GrColors.success
        case .cancelled: GrColors.muted
        }
        let bg: Color = switch badge.kind {
        case .active: GrColors.infoSoft
        case .pending: GrColors.warningSoft
        case .done: GrColors.successSoft
        case .cancelled: GrColors.surface2
        }
        Text(badge.text)
            .font(GrTypography.badge)
            .foregroundStyle(fg)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(Capsule().fill(bg))
    }
}
