import XCTest
@testable import GraceRideKit

@MainActor
final class ViewModelTests: XCTestCase {

    override func tearDown() {
        MockURLProtocol.reset()
        super.tearDown()
    }

    private func makeSignedInContainer() async throws -> AppContainer {
        let cfg = URLSessionConfiguration.ephemeral
        cfg.protocolClasses = [MockURLProtocol.self]
        let container = AppContainer(
            secureStore: InMemorySecureStore(),
            offlineCache: OfflineCache(cacheDir: nil),
            networkMonitor: NetworkMonitor(startMonitoring: false),
            uiDefaults: nil,
            widgetUserDefaults: nil,
            urlSessionConfiguration: cfg,
            debugMode: true,
            initialBaseURL: "http://127.0.0.1:8080/"
        )
        MockURLProtocol.setHandler { req in
            let json = #"{"id":10,"name":"Grace Member","email":"grace@example.org","permissions":["rides.view","rides.create","events.view","costs.view","drive.view"]}"#
            return (HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!, Data(json.utf8))
        }
        try await container.auth.refresh()
        return container
    }

    func testHomeViewModelLoadsSectionsAndHandlesPartialErrors() async throws {
        let container = try await makeSignedInContainer()
        let today = KlTime.today()
        MockURLProtocol.setHandler { req in
            let path = req.url?.path ?? ""
            let body: String
            let code: Int
            if path == "/api/rides" {
                code = 200
                body = #"[{"id":3,"status":"confirmed","pickup_at":"\#(today) 09:00:00","area":{"id":1,"name":"SS2"},"destination":"Church","passenger":{"id":10,"name":"Grace"},"viewer":{"is_passenger":true}}]"#
            } else if path == "/api/events" {
                code = 200
                body = #"[{"id":8,"title":"Youth Night","starts_at":"\#(today) 19:30:00","location":"Main Hall"}]"#
            } else if path == "/api/me/shares" {
                code = 500
                body = #"{"error":"Transient share failure"}"#
            } else if path == "/api/announcements" {
                code = 200
                body = #"[{"id":1,"title":"Welcome","pinned":true}]"#
            } else if path == "/api/driver/schedule" {
                code = 200
                body = "[]"
            } else {
                code = 200
                body = "[]"
            }
            return (HTTPURLResponse(url: req.url!, statusCode: code, httpVersion: nil, headerFields: nil)!, Data(body.utf8))
        }

        let vm = HomeViewModel(container: container, startObservers: false)
        await vm.loadInitial()
        XCTAssertFalse(vm.loading)
        XCTAssertNil(vm.error)
        XCTAssertEqual(vm.nextRide?.id, 3)
        XCTAssertEqual(vm.events.first?.title, "Youth Night")
        XCTAssertEqual(vm.announcements.first?.title, "Welcome")
    }

    func testRideDetailOptimisticConcurrencyReloadsOnRideStatusChanged() async throws {
        let container = try await makeSignedInContainer()
        var currentStatus = "confirmed"
        MockURLProtocol.setHandler { req in
            let path = req.url?.path ?? ""
            if path == "/api/rides/55/start" {
                currentStatus = "in_progress"
                let err = #"{"code":"RIDE_STATUS_CHANGED","error":"Ride changed"}"#
                return (HTTPURLResponse(url: req.url!, statusCode: 409, httpVersion: nil, headerFields: nil)!, Data(err.utf8))
            }
            if path == "/api/rides/55/location" {
                return (HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!, Data(#"{"sharing":false}"#.utf8))
            }
            let ride = #"{"id":55,"status":"\#(currentStatus)","pickup_at":"2026-11-26 09:00:00","area":{"id":1,"name":"SS2"},"passenger":{"id":10,"name":"Grace"},"viewer":{"is_driver":true}}"#
            return (HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!, Data(ride.utf8))
        }

        let vm = RideDetailViewModel(container: container, rideId: 55, startObservers: false)
        await vm.load()
        XCTAssertEqual(vm.ride?.status, "confirmed")

        await vm.startRide()
        XCTAssertTrue(vm.staleReload)
        XCTAssertEqual(vm.ride?.status, "in_progress")
    }

    func testCostsValidationAndLoginEmailGuard() async throws {
        XCTAssertEqual(CostsViewModel.validateProofInput(hasPhoto: false, txnRef: "1234"), "costs_err_missing_photo")
        XCTAssertEqual(CostsViewModel.validateProofInput(hasPhoto: true, txnRef: "  "), "costs_err_missing_ref")
        XCTAssertEqual(CostsViewModel.validateProofInput(hasPhoto: true, txnRef: "123"), "costs_err_ref_short")
        XCTAssertNil(CostsViewModel.validateProofInput(hasPhoto: true, txnRef: "MBB123456"))

        let container = try await makeSignedInContainer()
        let loginVM = LoginViewModel(container: container, devAvailable: false)
        await loginVM.validateAndStartPasskey { _ in .null }
        XCTAssertEqual(loginVM.errorKey, "login_email_required")
    }
}
