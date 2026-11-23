package org.pjstatecm.graceride.data.repo

import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CancelReq
import org.pjstatecm.graceride.data.api.CreateRideReq
import org.pjstatecm.graceride.data.api.CreateRideRes
import org.pjstatecm.graceride.data.api.CreateSavedPlaceRes
import org.pjstatecm.graceride.data.api.DriverDelayReq
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.ReadinessReq
import org.pjstatecm.graceride.data.api.RespondReq
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.data.api.SavedPlaceReq
import org.pjstatecm.graceride.data.api.WithdrawReq
import org.pjstatecm.graceride.data.api.apiCall

class RidesRepository(private val api: ApiService, private val bus: AuthEventBus) {
    suspend fun upcoming(): List<Ride> = apiCall(bus) { api.myRides("upcoming") }
    suspend fun past(): List<Ride> = apiCall(bus) { api.myRides("past") }
    suspend fun detail(id: Long): Ride = apiCall(bus) { api.ride(id) }
    suspend fun areas(): List<Area> = apiCall(bus) { api.areas() }
    suspend fun places(q: String): List<Place> = apiCall(bus) { api.places(q) }
    suspend fun savedPlaces(): List<SavedPlace> = apiCall(bus) { api.savedPlaces() }
    suspend fun createSavedPlace(req: SavedPlaceReq): CreateSavedPlaceRes = apiCall(bus) { api.createSavedPlace(req) }
    suspend fun updateSavedPlace(id: Long, req: SavedPlaceReq) = apiCall(bus) { api.updateSavedPlace(id, req) }
    suspend fun deleteSavedPlace(id: Long) = apiCall(bus) { api.deleteSavedPlace(id) }
    suspend fun proxyRiders(): List<ProxyRider> = apiCall(bus) { api.proxyRiders() }
    suspend fun create(req: CreateRideReq): CreateRideRes = apiCall(bus) { api.createRide(req) }
    suspend fun cancel(id: Long, reason: String?) = apiCall(bus) { api.cancelRide(id, CancelReq(reason)) }
    suspend fun respond(id: Long, accept: Boolean) = apiCall(bus) { api.respondRide(id, RespondReq(accept)) }
    suspend fun readiness(id: Long, status: String, notes: String? = null) =
        apiCall(bus) { api.readiness(id, ReadinessReq(status, notes)) }
    suspend fun confirmAssignment(id: Long) = apiCall(bus) { api.confirmAssignment(id) }
    suspend fun startRide(id: Long) = apiCall(bus) { api.startRide(id) }
    suspend fun arrivedGate(id: Long) = apiCall(bus) { api.arrivedGate(id) }
    suspend fun pickupRide(id: Long) = apiCall(bus) { api.pickupRide(id) }
    suspend fun completeRide(id: Long) = apiCall(bus) { api.completeRide(id) }
    suspend fun driverDelay(id: Long, minutes: Int) = apiCall(bus) { api.driverDelay(id, DriverDelayReq(minutes)) }
    suspend fun withdrawRide(id: Long, reason: String?) = apiCall(bus) { api.withdrawRide(id, WithdrawReq(reason)) }
    suspend fun drivingToday(from: String, to: String): List<Ride> =
        apiCall(bus) { api.driverSchedule(from, to) }
    suspend fun driverRequests(mineAreas: Boolean? = null): List<Ride> =
        apiCall(bus) { api.driverRequests(mineAreas) }
}
