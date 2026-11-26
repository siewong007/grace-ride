import XCTest
@testable import GraceRideKit

final class NotifLinkRouterTests: XCTestCase {

    func testExtractsUniversalLinksAndCustomSchemeAndRejectsUntrustedHosts() {
        XCTAssertEqual(
            NotifLinkRouter.extractPathFromIntentUri("https://graceride.saliminn.my/rides/42"),
            "/rides/42"
        )
        XCTAssertEqual(
            NotifLinkRouter.extractPathFromIntentUri("graceride://open/drive"),
            "/drive"
        )
        XCTAssertEqual(
            NotifLinkRouter.extractPathFromIntentUri("https://graceride.saliminn.my/events/5/checkin?code=ABC"),
            "/events/5/checkin?code=ABC"
        )
        XCTAssertNil(NotifLinkRouter.extractPathFromIntentUri("https://evil.example.org/rides/42"))
        XCTAssertNil(NotifLinkRouter.extractPathFromIntentUri("http://graceride.saliminn.my/rides/42"))
    }

    func testResolveAndResolveForEnforceRolePermissions() {
        let member = Me(
            id: 1,
            name: "Member",
            email: "member@example.org",
            permissions: ["rides.view", "rides.create", "events.view", "costs.view", "settlements.view"]
        )
        let staff = Me(
            id: 2,
            name: "Staff",
            email: "staff@example.org",
            permissions: ["rides.view", "events.view", "admin_plan.view", "admin_rides.view", "settlements.update"]
        )

        XCTAssertEqual(NotifLinkRouter.resolveFor("/rides/12", me: member), .native(route: "ride/12"))
        XCTAssertEqual(NotifLinkRouter.resolveFor("/events/8/checkin?code=XYZ", me: member), .native(route: "event/8?code=XYZ"))
        XCTAssertEqual(NotifLinkRouter.resolveFor("/settlements/5", me: member), .native(route: "costs"))
        XCTAssertEqual(NotifLinkRouter.resolveFor("/me/family", me: member), .native(route: "proxy-riders"))
        XCTAssertEqual(NotifLinkRouter.resolveFor("/drive", me: member), .none)
        XCTAssertEqual(NotifLinkRouter.resolveFor("/admin/traffic", me: member), .none)

        XCTAssertEqual(NotifLinkRouter.resolveFor("/settlements/5", me: staff), .web(path: "/settlements/5"))
        XCTAssertEqual(NotifLinkRouter.resolveFor("/admin/traffic", me: staff), .web(path: "/admin/traffic"))
        XCTAssertEqual(NotifLinkRouter.resolveFor("https://external.example/path", me: staff), .none)
        XCTAssertEqual(NotifLinkRouter.resolveFor("/api/rides/1", me: staff), .none)
    }

    func testNotificationQuietHoursAndPriorityFiltering() {
        let prefs = NotificationPrefsDto(
            pushOptIn: true,
            mutedCategories: ["announcement"],
            quietHoursStart: "22:00",
            quietHoursEnd: "07:00"
        )
        let urgentRide = NotificationItem(id: 1, title: "Pickup now", category: "ride", priority: "urgent")
        let normalRide = NotificationItem(id: 2, title: "Matched", category: "ride", priority: "normal")
        let mutedAnn = NotificationItem(id: 3, title: "News", category: "announcement", priority: "normal")

        XCTAssertTrue(RealtimeStreamClient.shouldSurfaceNotification(urgentRide, prefs: prefs, nowHm: "23:30"))
        XCTAssertFalse(RealtimeStreamClient.shouldSurfaceNotification(normalRide, prefs: prefs, nowHm: "23:30"))
        XCTAssertTrue(RealtimeStreamClient.shouldSurfaceNotification(normalRide, prefs: prefs, nowHm: "10:00"))
        XCTAssertFalse(RealtimeStreamClient.shouldSurfaceNotification(mutedAnn, prefs: prefs, nowHm: "10:00"))
    }
}
