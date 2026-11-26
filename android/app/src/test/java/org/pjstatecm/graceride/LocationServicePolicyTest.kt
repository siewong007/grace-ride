package org.pjstatecm.graceride

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideArea
import org.pjstatecm.graceride.data.api.RidePassenger
import org.pjstatecm.graceride.data.api.RideViewer
import org.pjstatecm.graceride.data.api.RouteDestinationDto
import org.pjstatecm.graceride.data.api.RouteStopDto
import org.pjstatecm.graceride.data.api.TripRouteDto
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.service.LiveTripLocationService
import org.pjstatecm.graceride.service.LiveTripLocationService.Companion.WatchdogAction

class LocationServicePolicyTest {

    private val signedInMe = Me(
        id = 1L,
        name = "Driver",
        email = "driver@example.org",
        termsAccepted = true,
    )

    private val signedInState = AuthState.SignedIn(signedInMe)

    private fun sampleRide(
        status: String = "in_progress",
        isPassenger: Boolean = true,
        isDriver: Boolean = false,
        pickedUpAt: String? = null,
    ) = Ride(
        id = 10L,
        status = status,
        pickupAt = "2026-11-25 09:00:00",
        area = RideArea(1L, "SS2"),
        destination = "Church Hall",
        pickedUpAt = pickedUpAt,
        passenger = RidePassenger(2L, "Passenger"),
        viewer = RideViewer(isPassenger = isPassenger, isDriver = isDriver),
    )

    @Test fun activeJourneyTargetRequiresPositiveRideOrTripId() {
        assertFalse(LiveTripLocationService.hasActiveJourneyTarget(-1L, -1L))
        assertFalse(LiveTripLocationService.hasActiveJourneyTarget(0L, 0L))
        assertTrue(LiveTripLocationService.hasActiveJourneyTarget(10L, -1L))
        assertTrue(LiveTripLocationService.hasActiveJourneyTarget(-1L, 5L))
    }

    @Test fun driverLocationAllowedOnlyWhenSignedInAcceptedTermsAndRouteLive() {
        val liveRoute = TripRouteDto(tripId = 5L, live = true)
        val idleRoute = TripRouteDto(tripId = 5L, live = false)

        assertTrue(LiveTripLocationService.canShareDriverLocation(signedInState, liveRoute))
        assertFalse(LiveTripLocationService.canShareDriverLocation(signedInState, idleRoute))
        assertFalse(LiveTripLocationService.canShareDriverLocation(signedInState, null))
        assertFalse(LiveTripLocationService.canShareDriverLocation(AuthState.SignedOut, liveRoute))
        assertFalse(LiveTripLocationService.canShareDriverLocation(AuthState.Locked(signedInMe), liveRoute))
        assertFalse(
            LiveTripLocationService.canShareDriverLocation(
                AuthState.SignedIn(signedInMe.copy(termsAccepted = false)),
                liveRoute,
            ),
        )
    }

    @Test fun rideLocationAndPassengerWaitingSpotEnforceInProgressJourney() {
        assertTrue(LiveTripLocationService.canShareRideLocation(sampleRide(status = "in_progress")))
        assertFalse(LiveTripLocationService.canShareRideLocation(sampleRide(status = "confirmed")))
        assertFalse(LiveTripLocationService.canShareRideLocation(sampleRide(status = "completed")))
        assertFalse(LiveTripLocationService.canShareRideLocation(sampleRide(status = "cancelled")))
        assertFalse(LiveTripLocationService.canShareRideLocation(null))

        assertTrue(
            LiveTripLocationService.canSharePassengerWaitingSpot(
                sampleRide(status = "in_progress", isPassenger = true, pickedUpAt = null),
            ),
        )
        assertFalse(
            LiveTripLocationService.canSharePassengerWaitingSpot(
                sampleRide(status = "confirmed", isPassenger = true, pickedUpAt = null),
            ),
        )
        assertFalse(
            LiveTripLocationService.canSharePassengerWaitingSpot(
                sampleRide(status = "in_progress", isPassenger = false, isDriver = true, pickedUpAt = null),
            ),
        )
        assertFalse(
            LiveTripLocationService.canSharePassengerWaitingSpot(
                sampleRide(status = "in_progress", isPassenger = true, pickedUpAt = "2026-11-25 09:05:00"),
            ),
        )
    }

    @Test fun watchdogEvaluationStopsOnJourneyEndAuthFailureOrProlongedOffline() {
        assertEquals(
            WatchdogAction.KEEP_RUNNING,
            LiveTripLocationService.evaluateJourneyCheck(
                authState = signedInState,
                routeLive = true,
                authRejected = false,
                consecutiveNetworkErrors = 0,
            ),
        )
        assertEquals(
            WatchdogAction.STOP_JOURNEY_ENDED,
            LiveTripLocationService.evaluateJourneyCheck(
                authState = signedInState,
                routeLive = false,
                authRejected = false,
                consecutiveNetworkErrors = 0,
            ),
        )
        assertEquals(
            WatchdogAction.STOP_AUTH_INVALID,
            LiveTripLocationService.evaluateJourneyCheck(
                authState = AuthState.Locked(signedInMe),
                routeLive = true,
                authRejected = false,
                consecutiveNetworkErrors = 0,
            ),
        )
        assertEquals(
            WatchdogAction.STOP_AUTH_INVALID,
            LiveTripLocationService.evaluateJourneyCheck(
                authState = signedInState,
                routeLive = null,
                authRejected = true,
                consecutiveNetworkErrors = 1,
            ),
        )
        assertEquals(
            WatchdogAction.KEEP_RUNNING,
            LiveTripLocationService.evaluateJourneyCheck(
                authState = signedInState,
                routeLive = null,
                authRejected = false,
                consecutiveNetworkErrors = 3,
            ),
        )
        assertEquals(
            WatchdogAction.STOP_JOURNEY_ENDED,
            LiveTripLocationService.evaluateJourneyCheck(
                authState = signedInState,
                routeLive = null,
                authRejected = false,
                consecutiveNetworkErrors = LiveTripLocationService.MAX_CONSECUTIVE_ERRORS,
            ),
        )
    }

    @Test fun autoArrivalRequiresTwoFixesWithinRadiusAndExactLocation() {
        val stopExact = RouteStopDto(
            rideId = 101L,
            passenger = "Joshua",
            started = true,
            lat = 3.1180,
            lng = 101.6220,
            location = "live",
        )
        val stopApprox = RouteStopDto(
            rideId = 102L,
            passenger = "Mei",
            started = true,
            lat = 3.1180,
            lng = 101.6220,
            location = "approx_address",
        )
        val route = TripRouteDto(
            tripId = 7L,
            live = true,
            stops = listOf(stopExact, stopApprox),
            destination = RouteDestinationDto(name = "Church", lat = 3.1050, lng = 101.6400),
        )
        val hits = mutableMapOf<Long, Int>()
        val triggered = mutableSetOf<Long>()

        // Inaccurate fix (> 50m) is ignored.
        assertEquals(
            emptyList<Long>(),
            LiveTripLocationService.findEligibleAutoArrivalStopIds(
                lat = 3.1182,
                lng = 101.6221,
                accuracyM = 90f,
                hasAccuracy = true,
                autoEnabled = true,
                route = route,
                alreadyTriggered = triggered,
                hitCounts = hits,
            ),
        )

        // First accurate fix within ~80m arms counter (approx_address is ignored).
        assertEquals(
            emptyList<Long>(),
            LiveTripLocationService.findEligibleAutoArrivalStopIds(
                lat = 3.1182,
                lng = 101.6221,
                accuracyM = 12f,
                hasAccuracy = true,
                autoEnabled = true,
                route = route,
                alreadyTriggered = triggered,
                hitCounts = hits,
            ),
        )
        assertEquals(1, hits[101L])
        assertFalse(hits.containsKey(102L))

        // Second consecutive accurate fix triggers auto-arrival once.
        val second = LiveTripLocationService.findEligibleAutoArrivalStopIds(
            lat = 3.1181,
            lng = 101.6221,
            accuracyM = 10f,
            hasAccuracy = true,
            autoEnabled = true,
            route = route,
            alreadyTriggered = triggered,
            hitCounts = hits,
        )
        assertEquals(listOf(101L), second)
        triggered.addAll(second)

        // Already triggered stops do not re-fire.
        assertEquals(
            emptyList<Long>(),
            LiveTripLocationService.findEligibleAutoArrivalStopIds(
                lat = 3.1180,
                lng = 101.6220,
                accuracyM = 10f,
                hasAccuracy = true,
                autoEnabled = true,
                route = route,
                alreadyTriggered = triggered,
                hitCounts = hits,
            ),
        )
    }

    @Test fun destinationProximityTriggersOnlyWhenAllAboardAndWithinRadius() {
        val route = TripRouteDto(
            tripId = 8L,
            live = true,
            stops = emptyList(),
            onboard = 2,
            destination = RouteDestinationDto(name = "Church", lat = 3.1050, lng = 101.6400, approx = false),
        )
        assertTrue(
            LiveTripLocationService.isNearDestination(
                lat = 3.1054,
                lng = 101.6402,
                accuracyM = 15f,
                hasAccuracy = true,
                route = route,
            ),
        )
        assertFalse(
            LiveTripLocationService.isNearDestination(
                lat = 3.1080,
                lng = 101.6400,
                accuracyM = 15f,
                hasAccuracy = true,
                route = route,
            ),
        )
    }
}
