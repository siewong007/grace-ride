import BackgroundTasks
import GraceRideKit
import SwiftUI
import UIKit
import UserNotifications
import WidgetKit

@main
public struct GraceRideApp: App {
    @UIApplicationDelegateAdaptor(GraceRideAppDelegate.self) private var appDelegate
    @Environment(\.scenePhase) private var scenePhase

    @State private var container: AppContainer
    @State private var locationManager: LiveTripLocationManager
    @State private var pendingLinkPath: String?

    private static let bgRefreshId = "org.pjstatecm.graceride.sync"

    public init() {
        let appContainer = AppContainer()
        let locManager = LiveTripLocationManager(container: appContainer)
        self._container = State(initialValue: appContainer)
        self._locationManager = State(initialValue: locManager)

        appContainer.onWidgetChanged = {
            WidgetCenter.shared.reloadAllTimelines()
        }
        appContainer.onNotifySurfaced = { items in
            Task {
                await Self.postLocalNotifications(items, container: appContainer)
            }
        }
        appContainer.onSessionCleared = {
            UNUserNotificationCenter.current().removeAllDeliveredNotifications()
            UNUserNotificationCenter.current().removeAllPendingNotificationRequests()
            UIApplication.shared.unregisterForRemoteNotifications()
        }
        appContainer.onSessionLocked = {
            UNUserNotificationCenter.current().removeAllDeliveredNotifications()
        }

        GraceRideAppDelegate.sharedContainer = appContainer
        GraceRideAppDelegate.onPendingRoute = { path in
            NotificationCenter.default.post(name: .graceridePendingRoute, object: path)
        }

        Self.registerBackgroundTasks(container: appContainer)
        Self.configureNotificationCategories(lang: appContainer.lang)
    }

    public var body: some Scene {
        WindowGroup {
            ZStack {
                GraceRideAppRoot(
                    container: container,
                    locationManager: locationManager,
                    pendingLinkPath: $pendingLinkPath
                )
                .onReceive(NotificationCenter.default.publisher(for: .graceridePendingRoute)) { note in
                    if let path = note.object as? String {
                        pendingLinkPath = path
                    }
                }
                if scenePhase != .active {
                    ZStack {
                        Rectangle().fill(.ultraThinMaterial).ignoresSafeArea()
                        VStack(spacing: 12) {
                            Image(systemName: "lock.shield.fill")
                                .font(.system(size: 44))
                                .foregroundStyle(.secondary)
                            Text(L10n.tr("app_name", lang: container.lang))
                                .font(.headline)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .transition(.opacity)
                    .accessibilityHidden(true)
                }
            }
        }
        .onChange(of: container.auth.state) { _, newState in
            if case .signedIn = newState, container.auth.storedApnsToken() == nil {
                Task {
                    let settings = await UNUserNotificationCenter.current().notificationSettings()
                    if settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional {
                        await MainActor.run {
                            UIApplication.shared.registerForRemoteNotifications()
                        }
                    }
                }
            }
        }
        .onChange(of: scenePhase) { _, phase in
            switch phase {
            case .active:
                container.auth.enforceIdleTimeoutIfNeeded()
                container.realtimeStream.setForeground(true)
                if case .loading = container.auth.state {
                    break
                } else if container.networkMonitor.isOnline {
                    Task {
                        try? await container.auth.refresh(throwOnError: false)
                    }
                }
            case .background:
                container.auth.recordBackgroundTimestamp()
                container.realtimeStream.setForeground(false)
                Self.scheduleBackgroundSync()
            default:
                break
            }
        }
    }

    private static func registerBackgroundTasks(container: AppContainer) {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: bgRefreshId, using: nil) { task in
            guard let refreshTask = task as? BGAppRefreshTask else {
                task.setTaskCompleted(success: false)
                return
            }
            scheduleBackgroundSync()
            let work = Task { @MainActor in
                await container.realtimeStream.syncLatestAndNotify(surfaceNew: true, passive: true)
                refreshTask.setTaskCompleted(success: true)
            }
            refreshTask.expirationHandler = {
                work.cancel()
            }
        }
    }

    private static func scheduleBackgroundSync() {
        let req = BGAppRefreshTaskRequest(identifier: bgRefreshId)
        req.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)
        try? BGTaskScheduler.shared.submit(req)
    }

    private static func configureNotificationCategories(lang: String) {
        let replyAction = UNTextInputNotificationAction(
            identifier: "GR_CHAT_REPLY",
            title: L10n.tr("notif_action_reply", lang: lang),
            options: [.authenticationRequired],
            textInputButtonTitle: L10n.tr("chat_send", lang: lang),
            textInputPlaceholder: L10n.tr("notif_reply_hint", lang: lang)
        )
        let readyAction = UNNotificationAction(
            identifier: "GR_RIDE_READY",
            title: L10n.tr("notif_action_ready", lang: lang),
            options: [.authenticationRequired]
        )
        let chatCat = UNNotificationCategory(
            identifier: "GR_TRIP_CHAT",
            actions: [replyAction],
            intentIdentifiers: [],
            options: []
        )
        let rideCat = UNNotificationCategory(
            identifier: "GR_RIDE_UPDATE",
            actions: [readyAction],
            intentIdentifiers: [],
            options: []
        )
        UNUserNotificationCenter.current().setNotificationCategories([chatCat, rideCat])
    }

    @MainActor
    private static func postLocalNotifications(_ items: [NotificationItem], container: AppContainer) async {
        let center = UNUserNotificationCenter.current()
        let settings = await center.notificationSettings()
        guard settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional else {
            return
        }
        let hideDetails = container.secureStore.getString(AuthRepository.hideLockDetailsKey) == "1"
        let lang = container.lang

        for item in items {
            let content = UNMutableNotificationContent()
            if hideDetails {
                content.title = L10n.tr("app_name", lang: lang)
                content.body = L10n.tr("notif_locked_generic", lang: lang)
            } else {
                content.title = item.title
                content.body = item.body
            }
            content.sound = .default
            content.interruptionLevel = item.priority == "urgent" ? .timeSensitive : .active
            var info: [AnyHashable: Any] = ["notifId": item.id]
            if let link = item.link {
                info["link"] = link
                if let rideId = NotifLinkRouter.extractRideId(link) {
                    info["rideId"] = rideId
                    if !hideDetails {
                        if item.category == "chat" {
                            content.categoryIdentifier = "GR_TRIP_CHAT"
                        } else if item.category == "ride" {
                            content.categoryIdentifier = "GR_RIDE_UPDATE"
                        }
                    }
                }
            }
            content.userInfo = info
            let trigger = UNTimeIntervalNotificationTrigger(timeInterval: 0.2, repeats: false)
            let req = UNNotificationRequest(identifier: "gr-notif-\(item.id)", content: content, trigger: trigger)
            try? await center.add(req)
        }
    }
}

extension Notification.Name {
    static let graceridePendingRoute = Notification.Name("org.pjstatecm.graceride.pendingRoute")
}

@MainActor
public final class GraceRideAppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    static var sharedContainer: AppContainer?
    static var onPendingRoute: ((String) -> Void)?

    public func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        UNUserNotificationCenter.current().delegate = self
        if let shortcut = launchOptions?[.shortcutItem] as? UIApplicationShortcutItem {
            handleShortcut(shortcut)
        }
        return true
    }

    public func application(
        _ application: UIApplication,
        performActionFor shortcutItem: UIApplicationShortcutItem,
        completionHandler: @escaping (Bool) -> Void
    ) {
        handleShortcut(shortcutItem)
        completionHandler(true)
    }

    public func application(_ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
        let hex = deviceToken.map { String(format: "%02x", $0) }.joined()
        guard let container = Self.sharedContainer else { return }
        container.auth.setApnsToken(hex)
        let label = UIDevice.current.name
        Task {
            _ = try? await container.community.registerMobileToken(MobilePushTokenReq(
                platform: "ios",
                provider: "apns",
                token: hex,
                deviceLabel: label
            ))
        }
    }

    nonisolated public func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .sound, .badge]
    }

    nonisolated public func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        let info = response.notification.request.content.userInfo
        let actionId = response.actionIdentifier
        let rideId = (info["rideId"] as? NSNumber)?.int64Value ?? (info["rideId"] as? Int64)
        let link = info["link"] as? String
        let textResp = (response as? UNTextInputNotificationResponse)?.userText

        await MainActor.run {
            guard let container = Self.sharedContainer else { return }
            if actionId == "GR_CHAT_REPLY", let rideId, let body = textResp?.trimmingCharacters(in: .whitespacesAndNewlines), !body.isEmpty {
                Task {
                    _ = try? await container.rides.postMessage(id: rideId, body: body)
                }
                return
            }
            if actionId == "GR_RIDE_READY", let rideId {
                Task {
                    _ = try? await container.rides.readiness(id: rideId, status: Readiness.ready)
                }
                return
            }
            if let link {
                Self.onPendingRoute?(link)
            }
        }
    }

    private func handleShortcut(_ item: UIApplicationShortcutItem) {
        switch item.type {
        case "org.pjstatecm.graceride.shortcut.newRide":
            Self.onPendingRoute?("/ride-new")
        case "org.pjstatecm.graceride.shortcut.events":
            Self.onPendingRoute?("/events")
        case "org.pjstatecm.graceride.shortcut.drive":
            Self.onPendingRoute?("/drive")
        default:
            break
        }
    }
}
