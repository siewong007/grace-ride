import AppIntents
import Foundation
import GraceRideKit
import WidgetKit

struct MarkRideReadyIntent: AppIntent {
    static var title: LocalizedStringResource = "Mark Ride Ready"
    static var description = IntentDescription("Confirm you are ready for your upcoming church ride pickup.")
    static var authenticationPolicy: IntentAuthenticationPolicy { .requiresLocalDeviceAuthentication }

    @Parameter(title: "Ride ID")
    var rideId: Int

    init() {}

    init(rideId: Int) {
        self.rideId = rideId
    }

    func perform() async throws -> some IntentResult {
        let suiteDefaults = UserDefaults(suiteName: WidgetSnapshotStore.appGroupSuiteName)
        let container = await MainActor.run {
            AppContainer(widgetUserDefaults: suiteDefaults)
        }
        _ = try await container.rides.readiness(id: Int64(rideId), status: Readiness.ready)
        await MainActor.run {
            WidgetSnapshotStore.markReadyLocally(
                rideId: Int64(rideId),
                status: Readiness.ready,
                store: container.secureStore,
                userDefaults: suiteDefaults
            )
        }
        WidgetCenter.shared.reloadAllTimelines()
        return .result()
    }
}
