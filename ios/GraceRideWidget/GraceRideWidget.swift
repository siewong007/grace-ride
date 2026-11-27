import AppIntents
import GraceRideKit
import SwiftUI
import WidgetKit

struct GraceRideEntry: TimelineEntry {
    let date: Date
    let summary: WidgetSummary
    let lang: String
}

struct GraceRideProvider: TimelineProvider {
    func placeholder(in context: Context) -> GraceRideEntry {
        GraceRideEntry(
            date: Date(),
            summary: WidgetSummary(
                signedIn: true,
                isRide: true,
                rideId: 1,
                scheduledAt: "2026-12-06 08:30:00",
                badge: "Next Ride",
                title: "Sunday Service",
                subtitle: "Sun 8:30 AM · Confirmed",
                showReadyButton: true
            ),
            lang: resolvedLang()
        )
    }

    func getSnapshot(in context: Context, completion: @escaping (GraceRideEntry) -> Void) {
        completion(loadEntry(now: Date()))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<GraceRideEntry>) -> Void) {
        let now = Date()
        let entry = loadEntry(now: now)
        let nextRefresh = Calendar.current.date(byAdding: .minute, value: 30, to: now) ?? now.addingTimeInterval(1800)
        completion(Timeline(entries: [entry], policy: .after(nextRefresh)))
    }

    private func loadEntry(now: Date) -> GraceRideEntry {
        let defaults = UserDefaults(suiteName: WidgetSnapshotStore.appGroupSuiteName)
        let raw = WidgetSnapshotStore.load(from: KeychainSecureStore(), userDefaults: defaults)
        let fresh = WidgetSnapshotStore.freshForDisplay(raw, now: now)
        return GraceRideEntry(date: now, summary: fresh, lang: resolvedLang())
    }

    private func resolvedLang() -> String {
        let shared = UserDefaults(suiteName: WidgetSnapshotStore.appGroupSuiteName)?.string(forKey: "gr_lang")
        let pref = shared ?? Locale.preferredLanguages.first ?? "en"
        return pref.hasPrefix("zh") ? "zh" : "en"
    }
}

struct GraceRideNextTripWidget: Widget {
    let kind = "GraceRideNextTripWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: GraceRideProvider()) { entry in
            GraceRideWidgetEntryView(entry: entry)
                .containerBackground(.fill.tertiary, for: .widget)
        }
        .configurationDisplayName("Grace Ride · 恩典同行")
        .description("See your upcoming church ride or event and confirm readiness.")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

struct GraceRideWidgetEntryView: View {
    let entry: GraceRideEntry
    @Environment(\.widgetFamily) private var family

    private var zh: Bool { entry.lang.hasPrefix("zh") }

    private var deepLinkURL: URL {
        if entry.summary.isRide, let rideId = entry.summary.rideId {
            return URL(string: "graceride://open/rides/\(rideId)")!
        }
        if !entry.summary.isRide, let eventId = entry.summary.eventId {
            return URL(string: "graceride://open/events/\(eventId)")!
        }
        return URL(string: "graceride://open/")!
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Circle()
                    .fill(Color(red: 0.122, green: 0.365, blue: 0.706))
                    .frame(width: 8, height: 8)
                Text(entry.summary.badge.isEmpty ? "Grace Ride" : entry.summary.badge)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
            }

            if !entry.summary.signedIn {
                Spacer(minLength: 0)
                Text(zh ? "打开 恩典同行 登录以查看接送安排" : "Open Grace Ride to sign in")
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(.primary)
                    .lineLimit(3)
                Spacer(minLength: 0)
            } else if entry.summary.title.isEmpty {
                Spacer(minLength: 0)
                Text(zh ? "暂无即将进行的行程" : "No upcoming ride")
                    .font(.subheadline.weight(.semibold))
                Text(zh ? "前往活动页报名并预约接送" : "Sign up for an event to book a ride")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
                Spacer(minLength: 0)
            } else {
                Text(entry.summary.title)
                    .font(family == .systemSmall ? .subheadline.weight(.bold) : .headline)
                    .lineLimit(2)
                    .privacySensitive()
                if !entry.summary.subtitle.isEmpty {
                    Text(entry.summary.subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                        .privacySensitive()
                }
                Spacer(minLength: 0)
                if entry.summary.showReadyButton, let rideId = entry.summary.rideId {
                    Button(intent: MarkRideReadyIntent(rideId: Int(rideId))) {
                        Label(zh ? "我已准备好" : "I'm Ready", systemImage: "checkmark.circle.fill")
                            .font(.caption.weight(.semibold))
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(Color(red: 0.122, green: 0.365, blue: 0.706))
                    .privacySensitive()
                } else if entry.summary.readiness == Readiness.ready {
                    Label(zh ? "已标记准备好" : "Ready", systemImage: "checkmark.circle.fill")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(Color(red: 0.169, green: 0.459, blue: 0.286))
                        .privacySensitive()
                }
            }
        }
        .padding(.vertical, 2)
        .widgetURL(deepLinkURL)
    }
}
