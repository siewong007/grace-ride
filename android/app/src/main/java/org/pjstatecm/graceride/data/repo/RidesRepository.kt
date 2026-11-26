package org.pjstatecm.graceride.data.repo

import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CancelReq
import org.pjstatecm.graceride.data.api.CareLinkReq
import org.pjstatecm.graceride.data.api.CareLinkResp
import org.pjstatecm.graceride.data.api.ChatThreadDto
import org.pjstatecm.graceride.data.api.RidesPageDto
import org.pjstatecm.graceride.data.api.CreateProxyRiderRes
import org.pjstatecm.graceride.data.api.CreateRideReq
import org.pjstatecm.graceride.data.api.CreateRideRes
import org.pjstatecm.graceride.data.api.CreateSavedPlaceRes
import org.pjstatecm.graceride.data.api.DriverDelayReq
import org.pjstatecm.graceride.data.api.DriverLocationReq
import org.pjstatecm.graceride.data.api.DriverProfile
import org.pjstatecm.graceride.data.api.DriverProfileReq
import org.pjstatecm.graceride.data.api.DriverRouteResp
import org.pjstatecm.graceride.data.api.DriverVehicle
import org.pjstatecm.graceride.data.api.DriverVehicleReq
import org.pjstatecm.graceride.data.api.LocationShareResp
import org.pjstatecm.graceride.data.api.MarkReadReq
import org.pjstatecm.graceride.data.api.PassengerLocationReq
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.PostMessageReq
import org.pjstatecm.graceride.data.api.PostMessageRes
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.ProxyRiderReq
import org.pjstatecm.graceride.data.api.ReactReq
import org.pjstatecm.graceride.data.api.ReactRes
import org.pjstatecm.graceride.data.api.ReadinessReq
import org.pjstatecm.graceride.data.api.RespondReq
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideLocationDto
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.data.api.SavedPlaceReq
import org.pjstatecm.graceride.data.api.SelectVehicleRes
import org.pjstatecm.graceride.data.api.SupperChoiceReq
import org.pjstatecm.graceride.data.api.WithdrawReq
import org.pjstatecm.graceride.data.api.apiCall
import java.util.concurrent.atomic.AtomicLong

class RidesRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()
    private val areasMutex = Mutex()
    private val areasGeneration = AtomicLong(0L)
    @Volatile private var cachedAreas: List<Area>? = null
    @Volatile private var areasFetchedAtMs: Long = 0L

    suspend fun upcoming(passive: Boolean = false): List<Ride> =
        apiCall(bus) { api.myRides("upcoming", passivePoll = if (passive) "1" else null) }
    suspend fun past(): List<Ride> = apiCall(bus) { api.myRides("past") }
    suspend fun detail(id: Long): Ride = apiCall(bus) { api.ride(id) }

    /**
     * Fetches service areas with a 5-minute in-memory TTL cache so repeated navigation between
     * Ride New, Saved Addresses, Proxy Riders, and Driver Settings avoids duplicate round-trips.
     */
    suspend fun areas(forceRefresh: Boolean = false): List<Area> {
        val now = System.currentTimeMillis()
        val cached = cachedAreas
        if (!forceRefresh && cached != null && (now - areasFetchedAtMs) in 0 until AREAS_TTL_MS) {
            return cached
        }
        return areasMutex.withLock {
            val recheckNow = System.currentTimeMillis()
            val lockedCached = cachedAreas
            if (!forceRefresh && lockedCached != null && (recheckNow - areasFetchedAtMs) in 0 until AREAS_TTL_MS) {
                return@withLock lockedCached
            }
            val gen = areasGeneration.get()
            apiCall(bus) { api.areas() }.also { fresh ->
                if (areasGeneration.get() == gen) {
                    cachedAreas = fresh
                    areasFetchedAtMs = System.currentTimeMillis()
                }
            }
        }
    }

    fun clearAreasCache() {
        areasGeneration.incrementAndGet()
        cachedAreas = null
        areasFetchedAtMs = 0L
    }
    suspend fun places(q: String): List<Place> = apiCall(bus) { api.places(q) }
    suspend fun savedPlaces(): List<SavedPlace> = apiCall(bus) { api.savedPlaces() }
    suspend fun createSavedPlace(req: SavedPlaceReq): CreateSavedPlaceRes = apiCall(bus) { api.createSavedPlace(req) }
    suspend fun updateSavedPlace(id: Long, req: SavedPlaceReq) = apiCall(bus) { api.updateSavedPlace(id, req) }
    suspend fun deleteSavedPlace(id: Long) = apiCall(bus) { api.deleteSavedPlace(id) }
    suspend fun proxyRiders(): List<ProxyRider> = apiCall(bus) { api.proxyRiders() }
    suspend fun createProxyRider(req: ProxyRiderReq): CreateProxyRiderRes = apiCall(bus) { api.createProxyRider(req) }
    suspend fun updateProxyRider(id: Long, req: ProxyRiderReq) = apiCall(bus) { api.updateProxyRider(id, req) }
    suspend fun deleteProxyRider(id: Long) = apiCall(bus) { api.deleteProxyRider(id) }
    suspend fun create(req: CreateRideReq): CreateRideRes = apiCall(bus) { api.createRide(req) }
    suspend fun cancel(id: Long, reason: String?) = apiCall(bus) { api.cancelRide(id, CancelReq(reason)) }
    suspend fun respond(id: Long, accept: Boolean) = apiCall(bus) { api.respondRide(id, RespondReq(accept)) }
    suspend fun readiness(id: Long, status: String, notes: String? = null) =
        apiCall(bus) { api.readiness(id, ReadinessReq(status, notes)) }
    suspend fun confirmAssignment(id: Long) = apiCall(bus) { api.confirmAssignment(id) }
    suspend fun startRide(id: Long) = apiCall(bus) { api.startRide(id) }
    suspend fun arrivedGate(id: Long) = apiCall(bus) { api.arrivedGate(id) }
    suspend fun pickupRide(id: Long) = apiCall(bus) { api.pickupRide(id) }
    suspend fun noShowRide(id: Long) = apiCall(bus) { api.noShowRide(id) }
    suspend fun completeRide(id: Long) = apiCall(bus) { api.completeRide(id) }
    suspend fun supperChoice(id: Long, choice: String) = apiCall(bus) { api.supperChoice(id, SupperChoiceReq(choice)) }
    suspend fun driverDelay(id: Long, minutes: Int) = apiCall(bus) { api.driverDelay(id, DriverDelayReq(minutes)) }
    suspend fun withdrawRide(id: Long, reason: String?) = apiCall(bus) { api.withdrawRide(id, WithdrawReq(reason)) }
    suspend fun drivingToday(from: String, to: String): List<Ride> =
        apiCall(bus) { api.driverSchedule(from, to) }
    suspend fun driverRequests(
        mineAreas: Boolean? = null,
        eventId: Long? = null,
        areaId: Long? = null,
        date: String? = null,
    ): List<Ride> = apiCall(bus) {
        api.driverRequests(mineAreas = mineAreas, eventId = eventId, areaId = areaId, date = date)
    }
    suspend fun claimRide(id: Long) = apiCall(bus) { api.claimRide(id) }
    suspend fun driverRoute(): DriverRouteResp = apiCall(bus) { api.driverRoute() }
    suspend fun postDriverLocation(lat: Double, lng: Double, heading: Double? = null): LocationShareResp =
        apiCall(bus) { api.postDriverLocation(DriverLocationReq(lat, lng, heading)) }
    suspend fun stopDriverLocation() = apiCall(bus) { api.stopDriverLocation() }
    suspend fun rideLocation(id: Long): RideLocationDto = apiCall(bus) { api.rideLocation(id) }
    suspend fun postPassengerLocation(id: Long, lat: Double, lng: Double): LocationShareResp =
        apiCall(bus) { api.postPassengerLocation(id, PassengerLocationReq(lat, lng)) }
    suspend fun stopPassengerLocation(id: Long) = apiCall(bus) { api.stopPassengerLocation(id) }
    suspend fun rideMessages(id: Long, after: Long? = null): ChatThreadDto =
        apiCall(bus) { api.rideMessages(id, after) }
    suspend fun postMessage(id: Long, body: String): PostMessageRes =
        apiCall(bus) { api.postRideMessage(id, PostMessageReq(body)) }
    suspend fun markChatRead(id: Long, upTo: Long) =
        apiCall(bus) { api.markRideChatRead(id, MarkReadReq(upTo)) }
    suspend fun reactMessage(id: Long, messageId: Long, emoji: String): ReactRes =
        apiCall(bus) { api.reactRideMessage(id, messageId, ReactReq(emoji)) }
    suspend fun driverProfile(): DriverProfile = apiCall(bus) { api.driverProfile() }
    suspend fun updateDriverProfile(req: DriverProfileReq) = apiCall(bus) { api.updateDriverProfile(req) }
    suspend fun createDriverVehicle(req: DriverVehicleReq): DriverVehicle =
        apiCall(bus) { api.createDriverVehicle(req) }
    suspend fun updateDriverVehicle(id: Long, req: DriverVehicleReq): DriverVehicle =
        apiCall(bus) { api.updateDriverVehicle(id, req) }
    suspend fun deleteDriverVehicle(id: Long) = apiCall(bus) { api.deleteDriverVehicle(id) }
    suspend fun selectDriverVehicle(id: Long): SelectVehicleRes = apiCall(bus) { api.selectDriverVehicle(id) }
    suspend fun pastPage(before: String? = null, limit: Int = 30): RidesPageDto =
        apiCall(bus) { api.myRidesPage(scope = "past", before = before, limit = limit) }
    suspend fun createCareLink(id: Long, label: String? = null): CareLinkResp =
        apiCall(bus) { api.createCareLink(id, CareLinkReq(label = label?.trim()?.ifBlank { null })) }
    suspend fun revokeCareLink(id: Long) = apiCall(bus) { api.revokeCareLink(id) }

    companion object {
        private const val AREAS_TTL_MS = 5 * 60_000L
    }
}
