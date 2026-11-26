import EventKit
import GraceRideKit
import MapKit
import SafariServices
import SwiftUI
import UIKit

@MainActor
public enum DeviceActions {
    public static func dial(phone: String) {
        let cleaned = phone.trimmingCharacters(in: .whitespacesAndNewlines)
            .filter { $0.isNumber || $0 == "+" }
        guard !cleaned.isEmpty, let url = URL(string: "tel://\(cleaned)") else { return }
        Haptics.selection()
        UIApplication.shared.open(url)
    }

    public static func copyToClipboard(_ text: String, sensitive: Bool = false) {
        if sensitive {
            UIPasteboard.general.setItems(
                [[UIPasteboard.typeAutomatic: text]],
                options: [
                    .localOnly: true,
                    .expirationDate: Date().addingTimeInterval(120),
                ]
            )
        } else {
            UIPasteboard.general.string = text
        }
        Haptics.success()
    }

    public static func openExternalURL(_ url: URL) {
        UIApplication.shared.open(url)
    }

    public static func openInAppleMaps(lat: Double?, lng: Double?, address: String?, label: String) {
        if let lat, let lng {
            let coord = CLLocationCoordinate2D(latitude: lat, longitude: lng)
            let item = MKMapItem(placemark: MKPlacemark(coordinate: coord))
            item.name = label
            item.openInMaps(launchOptions: [MKLaunchOptionsDirectionsModeKey: MKLaunchOptionsDirectionsModeDriving])
            return
        }
        guard let address, !address.isEmpty,
              let encoded = address.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed),
              let url = URL(string: "https://maps.apple.com/?daddr=\(encoded)&dirflg=d") else {
            return
        }
        UIApplication.shared.open(url)
    }

    public static func openInGoogleMaps(lat: Double?, lng: Double?, address: String?) {
        let dest: String?
        if let lat, let lng {
            dest = "\(lat),\(lng)"
        } else {
            dest = address?.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed)
        }
        guard let dest else { return }
        if let appURL = URL(string: "comgooglemaps://?daddr=\(dest)&directionsmode=driving"),
           UIApplication.shared.canOpenURL(appURL) {
            UIApplication.shared.open(appURL)
        } else if let webURL = URL(string: "https://www.google.com/maps/dir/?api=1&destination=\(dest)") {
            UIApplication.shared.open(webURL)
        }
    }

    public static func openInWaze(lat: Double?, lng: Double?, address: String?) {
        let url: URL?
        if let lat, let lng {
            url = URL(string: "https://waze.com/ul?ll=\(lat),\(lng)&navigate=yes")
        } else if let encoded = address?.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) {
            url = URL(string: "https://waze.com/ul?q=\(encoded)&navigate=yes")
        } else {
            url = nil
        }
        if let url {
            UIApplication.shared.open(url)
        }
    }

    public static func saveEventToCalendar(_ event: EventItem, lang: String) async -> (saved: Bool, message: String) {
        let store = EKEventStore()
        do {
            let granted = try await store.requestWriteOnlyAccessToEvents()
            guard granted else {
                return (false, L10n.tr("event_calendar_unavailable", lang: lang))
            }
            guard let start = KlTime.parse(event.startsAt) else {
                return (false, L10n.tr("common_error_title", lang: lang))
            }
            let ekEvent = EKEvent(eventStore: store)
            ekEvent.calendar = store.defaultCalendarForNewEvents
            ekEvent.title = event.title
            ekEvent.startDate = start
            ekEvent.endDate = start.addingTimeInterval(2 * 3600)
            ekEvent.location = event.location
            ekEvent.notes = event.description
            ekEvent.timeZone = KlTime.klCalendar.timeZone
            try store.save(ekEvent, span: .thisEvent)
            Haptics.success()
            return (true, L10n.tr("common_ok", lang: lang))
        } catch {
            Haptics.error()
            return (false, error.localizedDescription)
        }
    }
}

public struct SafariWebSheet: UIViewControllerRepresentable {
    public let url: URL

    public init(url: URL) {
        self.url = url
    }

    public func makeUIViewController(context: Context) -> SFSafariViewController {
        let vc = SFSafariViewController(url: url)
        vc.dismissButtonStyle = .done
        return vc
    }

    public func updateUIViewController(_ uiViewController: SFSafariViewController, context: Context) {}
}

public struct MapsDestinationChooser: View {
    public let title: String
    public let lat: Double?
    public let lng: Double?
    public let address: String?
    public let lang: String
    @Binding public var isPresented: Bool

    public init(
        title: String,
        lat: Double?,
        lng: Double?,
        address: String?,
        lang: String,
        isPresented: Binding<Bool>
    ) {
        self.title = title
        self.lat = lat
        self.lng = lng
        self.address = address
        self.lang = lang
        self._isPresented = isPresented
    }

    public var body: some View {
        EmptyView()
            .confirmationDialog(
                L10n.tr("ios_open_in_maps_title", lang: lang),
                isPresented: $isPresented,
                titleVisibility: .visible
            ) {
                Button(L10n.tr("ios_open_apple_maps", lang: lang)) {
                    DeviceActions.openInAppleMaps(lat: lat, lng: lng, address: address, label: title)
                }
                Button(L10n.tr("ios_open_google_maps", lang: lang)) {
                    DeviceActions.openInGoogleMaps(lat: lat, lng: lng, address: address)
                }
                Button(L10n.tr("ios_open_waze", lang: lang)) {
                    DeviceActions.openInWaze(lat: lat, lng: lng, address: address)
                }
                Button(L10n.tr("common_cancel", lang: lang), role: .cancel) {}
            }
    }
}

public struct ShareSheet: UIViewControllerRepresentable {
    public let items: [Any]

    public init(items: [Any]) {
        self.items = items
    }

    public func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }

    public func updateUIViewController(_ uiViewController: UIActivityViewController, context: Context) {}
}
