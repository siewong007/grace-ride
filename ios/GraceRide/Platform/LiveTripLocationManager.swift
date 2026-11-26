import CoreLocation
import Foundation
import GraceRideKit
import Observation
import UIKit

public enum LocationError: Error, LocalizedError {
    case permissionDenied
    case unavailable
    case noActiveJourney

    public var errorDescription: String? {
        switch self {
        case .permissionDenied:
            return "Location permission was denied. Enable it in iOS Settings."
        case .unavailable:
            return "Unable to obtain your current GPS coordinates."
        case .noActiveJourney:
            return "Live GPS sharing is only permitted during an active in-progress journey."
        }
    }
}

/// Manages CoreLocation on-demand pin capture and strictly journey-bounded active driver sharing.
@MainActor
@Observable
public final class LiveTripLocationManager: NSObject, CLLocationManagerDelegate {
    private let container: AppContainer
    private let manager: CLLocationManager

    public private(set) var authorizationStatus: CLAuthorizationStatus
    public private(set) var isSharing: Bool = false
    public private(set) var lastSharedAt: Date?
    public private(set) var lastError: String?

    private var oneShotContinuation: CheckedContinuation<(lat: Double, lng: Double), Error>?
    private var authContinuation: CheckedContinuation<CLAuthorizationStatus, Never>?
    private var watchdogTask: Task<Void, Never>?
    private var lastPostedLocation: CLLocation?
    private var lastPostedDate: Date?
    private var consecutiveNetworkErrors = 0

    public init(container: AppContainer) {
        self.container = container
        let lm = CLLocationManager()
        self.manager = lm
        self.authorizationStatus = lm.authorizationStatus
        super.init()
        lm.delegate = self
        container.onStopLiveLocation = { [weak self] notifyServer in
            Task { @MainActor [weak self] in
                await self?.stopDriverSharing(notifyServer: notifyServer)
            }
        }
    }

    public func captureCurrentLocation() async throws -> (lat: Double, lng: Double) {
        let status = await ensureWhenInUseAuthorization()
        guard status == .authorizedWhenInUse || status == .authorizedAlways else {
            throw LocationError.permissionDenied
        }
        return try await withCheckedThrowingContinuation { cont in
            self.oneShotContinuation?.resume(throwing: CancellationError())
            self.oneShotContinuation = cont
            self.manager.desiredAccuracy = kCLLocationAccuracyNearestTenMeters
            self.manager.requestLocation()
        }
    }

    public func startDriverSharing() async throws {
        lastError = nil
        let status = await ensureWhenInUseAuthorization()
        guard status == .authorizedWhenInUse || status == .authorizedAlways else {
            throw LocationError.permissionDenied
        }
        let routeResp = try await container.rides.driverRoute()
        guard LocationServicePolicy.canShareDriverLocation(authState: container.auth.state, route: routeResp.route) else {
            throw LocationError.noActiveJourney
        }

        if status == .authorizedWhenInUse {
            manager.requestAlwaysAuthorization()
        }

        let bgModes = Bundle.main.object(forInfoDictionaryKey: "UIBackgroundModes") as? [String] ?? []
        if bgModes.contains("location") {
            manager.allowsBackgroundLocationUpdates = true
        }
        manager.showsBackgroundLocationIndicator = true
        manager.desiredAccuracy = kCLLocationAccuracyBestForNavigation
        manager.distanceFilter = 25
        manager.activityType = .automotiveNavigation
        manager.pausesLocationUpdatesAutomatically = false

        isSharing = true
        container.isLiveLocationSharing = true
        consecutiveNetworkErrors = 0
        lastPostedDate = nil
        lastPostedLocation = nil

        manager.startUpdatingLocation()
        if let current = manager.location {
            await postDriverLocationIfDue(current, force: true)
        }
        startWatchdog()
    }

    public func stopDriverSharing(notifyServer: Bool = true) async {
        watchdogTask?.cancel()
        watchdogTask = nil
        manager.stopUpdatingLocation()
        manager.allowsBackgroundLocationUpdates = false
        isSharing = false
        container.isLiveLocationSharing = false
        lastPostedLocation = nil
        lastPostedDate = nil

        if notifyServer, case .signedIn = container.auth.state {
            _ = try? await container.rides.stopDriverLocation()
        }
    }

    private func startWatchdog() {
        watchdogTask?.cancel()
        watchdogTask = Task { [weak self] in
            while let self, !Task.isCancelled {
                try? await Task.sleep(for: .seconds(60))
                if Task.isCancelled || !self.isSharing { break }
                await self.runWatchdogCheck()
            }
        }
    }

    private func runWatchdogCheck() async {
        var routeLive: Bool? = nil
        var authRejected = false
        do {
            let routeResp = try await container.rides.driverRoute()
            routeLive = LocationServicePolicy.isJourneyLive(routeResp.route)
            consecutiveNetworkErrors = 0
        } catch let e as ApiException {
            if e.status == 401 || e.status == 403 || e.code == "SESSION_LOCKED" {
                authRejected = true
            } else {
                consecutiveNetworkErrors += 1
            }
        } catch {
            consecutiveNetworkErrors += 1
        }

        let decision = LocationServicePolicy.evaluateJourneyCheck(
            authState: container.auth.state,
            routeLive: routeLive,
            authRejected: authRejected,
            consecutiveNetworkErrors: consecutiveNetworkErrors
        )
        switch decision {
        case .keepRunning:
            break
        case .stopAuthInvalid:
            await stopDriverSharing(notifyServer: false)
        case .stopJourneyEnded:
            await stopDriverSharing(notifyServer: routeLive == false)
        }
    }

    private func postDriverLocationIfDue(_ location: CLLocation, force: Bool = false) async {
        guard isSharing else { return }
        if !force {
            if let prevDate = lastPostedDate, Date().timeIntervalSince(prevDate) < 10 {
                if let prevLoc = lastPostedLocation, location.distance(from: prevLoc) < 25 {
                    return
                }
            }
        }
        lastPostedDate = Date()
        lastPostedLocation = location
        let heading: Double? = location.course >= 0 ? location.course : nil
        do {
            _ = try await container.rides.postDriverLocation(
                lat: location.coordinate.latitude,
                lng: location.coordinate.longitude,
                heading: heading
            )
            lastSharedAt = Date()
            consecutiveNetworkErrors = 0
        } catch let e as ApiException {
            if e.status == 401 || e.status == 403 || e.code == "SESSION_LOCKED" {
                await stopDriverSharing(notifyServer: false)
            } else if e.status == 400 || e.status == 409 {
                await stopDriverSharing(notifyServer: false)
            } else {
                lastError = e.localizedDescription
            }
        } catch {
            lastError = error.localizedDescription
        }
    }

    private func ensureWhenInUseAuthorization() async -> CLAuthorizationStatus {
        let current = manager.authorizationStatus
        authorizationStatus = current
        guard current == .notDetermined else { return current }
        return await withCheckedContinuation { cont in
            self.authContinuation = cont
            self.manager.requestWhenInUseAuthorization()
        }
    }

    // MARK: - CLLocationManagerDelegate

    nonisolated public func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        let status = manager.authorizationStatus
        Task { @MainActor [weak self] in
            guard let self else { return }
            self.authorizationStatus = status
            if let cont = self.authContinuation, status != .notDetermined {
                self.authContinuation = nil
                cont.resume(returning: status)
            }
            if self.isSharing && (status == .denied || status == .restricted) {
                await self.stopDriverSharing(notifyServer: false)
            }
        }
    }

    nonisolated public func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let latest = locations.last else { return }
        Task { @MainActor [weak self] in
            guard let self else { return }
            if let cont = self.oneShotContinuation {
                self.oneShotContinuation = nil
                cont.resume(returning: (latest.coordinate.latitude, latest.coordinate.longitude))
            }
            if self.isSharing {
                await self.postDriverLocationIfDue(latest)
            }
        }
    }

    nonisolated public func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        Task { @MainActor [weak self] in
            guard let self else { return }
            if let cont = self.oneShotContinuation {
                self.oneShotContinuation = nil
                cont.resume(throwing: error)
            }
            self.lastError = error.localizedDescription
        }
    }
}
