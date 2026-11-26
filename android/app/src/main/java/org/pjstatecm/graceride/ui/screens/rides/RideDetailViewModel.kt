package org.pjstatecm.graceride.ui.screens.rides

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.AdminDriverDto
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.AreaPhotoDto
import org.pjstatecm.graceride.data.api.CareLinkResp
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideChangeDraftDto
import org.pjstatecm.graceride.data.api.RideDraftInReq
import org.pjstatecm.graceride.data.api.RideLocationDto
import org.pjstatecm.graceride.data.api.runSuspendCatching
import org.pjstatecm.graceride.data.cache.OfflineActionQueue
import org.pjstatecm.graceride.data.cache.QueuedRideAction
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.RealtimeEvent
import org.pjstatecm.graceride.service.LiveTripLocationService

data class RideDetailUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val ride: Ride? = null,
    val liveLocation: RideLocationDto? = null,
    val sharingPassengerPin: Boolean = false,
    val riderLocationAutoStopped: Boolean = false,
    val error: String? = null,
    val busy: Boolean = false,
    val message: String? = null,
    val staleReload: Boolean = false,
    val drivers: List<AdminDriverDto> = emptyList(),
    val drafts: List<RideChangeDraftDto> = emptyList(),
    val draftBusy: Boolean = false,
    val nextPendingRide: Ride? = null,
    val areaPhotos: List<AreaPhotoDto> = emptyList(),
    val careLink: CareLinkResp? = null,
    val careBusy: Boolean = false,
    val queuedOffline: List<QueuedRideAction> = emptyList(),
    val queuedOfflineNotice: Boolean = false,
    val offlineSyncing: Boolean = false,
)

class RideDetailViewModel(private val container: AppContainer, private val id: Long) : ViewModel() {
    private val _state = MutableStateFlow(
        RideDetailUiState(queuedOffline = container.offlineQueue.pendingForRide(id)),
    )
    val state: StateFlow<RideDetailUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    private var loadedAreaPhotoId: Long? = null

    init {
        load()
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) load()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
        viewModelScope.launch {
            container.offlineQueue.pending.collect { all ->
                _state.value = _state.value.copy(queuedOffline = all.filter { it.rideId == id })
            }
        }
        viewModelScope.launch {
            container.offlineQueue.syncing.collect { syncing ->
                _state.value = _state.value.copy(offlineSyncing = syncing)
            }
        }
    }

    fun load() {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            reloadRide()
        }
    }

    fun refresh() {
        loadJob?.cancel()
        _state.value = _state.value.copy(refreshing = _state.value.ride != null)
        loadJob = viewModelScope.launch {
            reloadRide()
        }
    }

    private suspend fun reloadRide() {
        _state.value = _state.value.copy(loading = _state.value.ride == null)
        try {
            val ride = container.rides.detail(id)
            val canViewLoc = ride.viewer.isPassenger || ride.viewer.isDriver || ride.viewer.isStaff
            val loc = if (LiveTripLocationService.canShareRideLocation(ride) && canViewLoc) {
                runSuspendCatching { container.rides.rideLocation(id) }.getOrNull()
            } else {
                null
            }
            if (ride.viewer.isDriver && !LiveTripLocationService.canShareRideLocation(ride)) {
                stopLiveLocationIfNoActiveTrip()
            }
            val prevWasSharingPin = _state.value.sharingPassengerPin || _state.value.liveLocation?.passenger != null
            val isBoarded = ride.pickedUpAt != null || loc?.eta?.phase == "dropoff"
            val autoStopped = (prevWasSharingPin && isBoarded) ||
                (_state.value.riderLocationAutoStopped && isBoarded)
            val isTrafficStaff = ride.viewer.isStaff || container.auth.me?.can("admin_rides") == true
            val (draftsList, driversList) = if (isTrafficStaff) {
                coroutineScope {
                    val dDef = async { container.traffic.listRideDrafts(id).getOrElse { _state.value.drafts } }
                    val drvDef = async {
                        if (_state.value.drivers.isNotEmpty()) _state.value.drivers
                        else container.traffic.adminDrivers().getOrDefault(emptyList())
                    }
                    dDef.await() to drvDef.await()
                }
            } else {
                emptyList<RideChangeDraftDto>() to _state.value.drivers
            }
            _state.value = _state.value.copy(
                loading = false,
                refreshing = false,
                ride = ride,
                liveLocation = loc,
                sharingPassengerPin = if (isBoarded) false else (loc?.passenger != null),
                riderLocationAutoStopped = autoStopped,
                error = null,
                drafts = draftsList,
                drivers = driversList,
            )
        } catch (e: ApiException) {
            _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
        }
    }

    private fun action(
        onSuccess: (() -> Unit)? = null,
        offlineKind: String? = null,
        offlineMinutes: Int? = null,
        offlineReadiness: Pair<String, String?>? = null,
        block: suspend () -> Unit,
    ) {
        if (_state.value.busy) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null, queuedOfflineNotice = false)
        viewModelScope.launch {
            try {
                block()
                reloadRide()
                onSuccess?.invoke()
            } catch (e: ApiException) {
                if (e.status == 0 && offlineKind != null) {
                    container.offlineQueue.enqueue(
                        rideId = id,
                        kind = offlineKind,
                        minutes = offlineMinutes,
                        readinessStatus = offlineReadiness?.first,
                        notes = offlineReadiness?.second,
                    )
                    _state.value = _state.value.copy(queuedOfflineNotice = true)
                } else if (e.code == "RIDE_STATUS_CHANGED") {
                    // Optimistic concurrency: someone else moved the ride. Re-fetch.
                    _state.value = _state.value.copy(staleReload = true)
                    reloadRide()
                } else {
                    _state.value = _state.value.copy(message = e.message)
                }
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun syncOfflineQueue() {
        viewModelScope.launch {
            val synced = runSuspendCatching { container.offlineQueue.flush(container.rides) }.getOrDefault(0)
            if (synced > 0) reloadRide()
        }
    }

    /** Lazy load triggered only by UI when rendering area pickup photos (`areaId`). */
    fun loadAreaPhotos(areaId: Long) {
        if (loadedAreaPhotoId == areaId && _state.value.areaPhotos.isNotEmpty()) return
        loadedAreaPhotoId = areaId
        viewModelScope.launch {
            container.traffic.areaPhotos(areaId).onSuccess { dto ->
                _state.value = _state.value.copy(areaPhotos = dto.photos)
            }
        }
    }

    fun createCareLink(label: String?) {
        if (_state.value.careBusy) return
        _state.value = _state.value.copy(careBusy = true, message = null)
        viewModelScope.launch {
            try {
                val resp = container.rides.createCareLink(id, label)
                _state.value = _state.value.copy(careBusy = false, careLink = resp)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(careBusy = false, message = e.message)
            }
        }
    }

    fun revokeCareLink() {
        if (_state.value.careBusy) return
        _state.value = _state.value.copy(careBusy = true, message = null)
        viewModelScope.launch {
            try {
                container.rides.revokeCareLink(id)
                _state.value = _state.value.copy(careBusy = false, careLink = null)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(careBusy = false, message = e.message)
            }
        }
    }

    fun clearCareLinkResult() {
        _state.value = _state.value.copy(careLink = null)
    }

    fun clearQueuedNotice() {
        _state.value = _state.value.copy(queuedOfflineNotice = false)
    }

    private suspend fun stopLiveLocationIfNoActiveTrip() {
        val ctx = container.appContext ?: return
        if (!LiveTripLocationService.isSharing.value) return
        val stillLive = runSuspendCatching { container.rides.driverRoute().route?.live == true }.getOrDefault(true)
        if (!stillLive) {
            LiveTripLocationService.stop(ctx, notifyServer = false)
        }
    }

    fun cancel(reason: String?) = action {
        container.rides.cancel(id, reason)
        stopLiveLocationIfNoActiveTrip()
    }
    fun respond(accept: Boolean) = action { container.rides.respond(id, accept) }
    fun readiness(status: String, notes: String? = null) {
        val cleanNotes = notes?.trim()?.ifBlank { null }
        action(
            offlineKind = OfflineActionQueue.KIND_READINESS,
            offlineReadiness = status to cleanNotes,
        ) {
            container.rides.readiness(id, status, cleanNotes)
        }
    }
    fun supperChoice(choice: String) = action { container.rides.supperChoice(id, choice) }
    fun claim() = action { container.rides.claimRide(id) }
    fun confirmAssignment() = action { container.rides.confirmAssignment(id) }
    fun startRide(onStarted: ((Ride) -> Unit)? = null) = action(
        onSuccess = { if (onStarted != null) _state.value.ride?.let(onStarted) },
        offlineKind = OfflineActionQueue.KIND_START,
    ) {
        container.rides.startRide(id)
    }
    fun arrivedGate() = action(offlineKind = OfflineActionQueue.KIND_ARRIVED_GATE) {
        container.rides.arrivedGate(id)
    }
    fun pickup() = action(offlineKind = OfflineActionQueue.KIND_PICKUP) {
        container.rides.pickupRide(id)
    }
    fun noShow() = action {
        container.rides.noShowRide(id)
        stopLiveLocationIfNoActiveTrip()
    }
    fun complete() = action(offlineKind = OfflineActionQueue.KIND_COMPLETE) {
        container.rides.completeRide(id)
        stopLiveLocationIfNoActiveTrip()
    }
    fun driverDelay(minutes: Int) = action(
        offlineKind = OfflineActionQueue.KIND_DELAY,
        offlineMinutes = minutes,
    ) {
        container.rides.driverDelay(id, minutes)
    }
    fun withdraw(reason: String?) = action {
        container.rides.withdrawRide(id, reason)
        stopLiveLocationIfNoActiveTrip()
    }

    // ---- Traffic staff actions ----

    fun staffApprove() = action {
        container.traffic.staffApproveRide(id).getOrThrow()
        val next = container.traffic.adminRides(status = "pending_approval", limit = 2)
            .getOrNull()
            ?.items
            ?.firstOrNull { it.id != id }
        _state.value = _state.value.copy(nextPendingRide = next)
    }

    fun dismissNextPending() {
        _state.value = _state.value.copy(nextPendingRide = null)
    }

    fun staffReassign(driverId: Long) = action {
        container.traffic.staffReassignRide(id, driverId).getOrThrow()
    }

    fun staffReturn(reason: String?) = action {
        container.traffic.staffReturnRide(id, reason).getOrThrow()
    }

    fun staffAltTransport(reason: String?) = action {
        container.traffic.staffAltTransportRide(id, reason).getOrThrow()
    }

    fun staffSetReimbursementExcluded(excluded: Boolean) = action {
        container.traffic.staffSetReimbursementExcluded(id, excluded).getOrThrow()
    }

    // ---- Single-ride change drafts & lease locks ----

    fun createRideDraft(req: RideDraftInReq) {
        if (_state.value.draftBusy) return
        _state.value = _state.value.copy(draftBusy = true, message = null)
        viewModelScope.launch {
            container.traffic.createRideDraft(id, req)
                .onSuccess { mut ->
                    val created = mut.draft
                    if (created != null && req.submit != true && created.status in setOf("draft", "returned")) {
                        container.traffic.acquireRideDraftLock(id, created.id, force = false)
                    }
                    reloadDraftsOnly()
                }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(draftBusy = false)
        }
    }

    fun updateRideDraft(draftId: Long, req: RideDraftInReq) {
        if (_state.value.draftBusy) return
        _state.value = _state.value.copy(draftBusy = true, message = null)
        viewModelScope.launch {
            val cur = _state.value.drafts.firstOrNull { it.id == draftId }
            if (cur != null && !cur.lockIsMine) {
                val lockRes = container.traffic.acquireRideDraftLock(id, draftId, force = false)
                if (lockRes.isFailure) {
                    val err = lockRes.exceptionOrNull()
                    _state.value = _state.value.copy(draftBusy = false, message = err?.message)
                    reloadDraftsOnly()
                    return@launch
                }
            }
            container.traffic.updateRideDraft(id, draftId, req)
                .onSuccess { reloadDraftsOnly() }
                .onFailure { err ->
                    _state.value = _state.value.copy(message = err.message)
                    if (err is ApiException && err.status in setOf(409, 423)) reloadDraftsOnly()
                }
            _state.value = _state.value.copy(draftBusy = false)
        }
    }

    fun acquireRideDraftLock(draftId: Long, force: Boolean) {
        if (_state.value.draftBusy) return
        _state.value = _state.value.copy(draftBusy = true, message = null)
        viewModelScope.launch {
            container.traffic.acquireRideDraftLock(id, draftId, force = force)
                .onSuccess { reloadDraftsOnly() }
                .onFailure { err ->
                    _state.value = _state.value.copy(message = err.message)
                    reloadDraftsOnly()
                }
            _state.value = _state.value.copy(draftBusy = false)
        }
    }

    fun renewActiveRideDraftLocks() {
        val mine = _state.value.drafts.filter { it.lockIsMine && it.status in setOf("draft", "returned") }
        if (mine.isEmpty()) return
        viewModelScope.launch {
            mine.forEach { d ->
                container.traffic.acquireRideDraftLock(id, d.id, force = false)
            }
            reloadDraftsOnly()
        }
    }

    fun requestRideDraftApproval(draftId: Long) {
        if (_state.value.draftBusy) return
        _state.value = _state.value.copy(draftBusy = true, message = null)
        viewModelScope.launch {
            container.traffic.requestRideDraftApproval(id, draftId)
                .onSuccess { reloadDraftsOnly() }
                .onFailure { err ->
                    _state.value = _state.value.copy(message = err.message)
                    reloadDraftsOnly()
                }
            _state.value = _state.value.copy(draftBusy = false)
        }
    }

    fun approveRideDraft(draftId: Long) {
        if (_state.value.draftBusy) return
        _state.value = _state.value.copy(draftBusy = true, message = null)
        viewModelScope.launch {
            container.traffic.approveRideDraft(id, draftId)
                .onSuccess {
                    runCatching { reloadRide() }
                }
                .onFailure { err ->
                    _state.value = _state.value.copy(message = err.message)
                    if (err is ApiException && err.status in setOf(409, 423)) reloadDraftsOnly()
                }
            _state.value = _state.value.copy(draftBusy = false)
        }
    }

    fun returnRideDraft(draftId: Long, notes: String?) {
        if (_state.value.draftBusy) return
        _state.value = _state.value.copy(draftBusy = true, message = null)
        viewModelScope.launch {
            container.traffic.returnRideDraft(id, draftId, notes)
                .onSuccess { reloadDraftsOnly() }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(draftBusy = false)
        }
    }

    fun deleteRideDraft(draftId: Long) {
        if (_state.value.draftBusy) return
        _state.value = _state.value.copy(draftBusy = true, message = null)
        viewModelScope.launch {
            container.traffic.deleteRideDraft(id, draftId)
                .onSuccess { reloadDraftsOnly() }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(draftBusy = false)
        }
    }

    private suspend fun reloadDraftsOnly() {
        container.traffic.listRideDrafts(id).onSuccess { list ->
            _state.value = _state.value.copy(drafts = list)
        }
    }

    override fun onCleared() {
        val mine = _state.value.drafts.filter { it.lockIsMine && it.status in setOf("draft", "returned") }
        if (mine.isNotEmpty()) {
            container.appScope.launch {
                withContext(NonCancellable) {
                    mine.forEach { d -> container.traffic.releaseRideDraftLock(id, d.id) }
                }
            }
        }
        super.onCleared()
    }
    fun refreshLiveLocation() {
        val r = _state.value.ride ?: return
        val canViewLoc = r.viewer.isPassenger || r.viewer.isDriver || r.viewer.isStaff
        if (!LiveTripLocationService.canShareRideLocation(r) || !canViewLoc) return
        viewModelScope.launch {
            runSuspendCatching { container.rides.rideLocation(id) }.getOrNull()?.let { loc ->
                val prevWasSharing = _state.value.sharingPassengerPin || _state.value.liveLocation?.passenger != null
                val isBoarded = r.pickedUpAt != null || loc.eta?.phase == "dropoff"
                val autoStopped = (prevWasSharing && isBoarded) ||
                    (_state.value.riderLocationAutoStopped && isBoarded)
                _state.value = _state.value.copy(
                    liveLocation = loc,
                    sharingPassengerPin = if (isBoarded) false else (loc.passenger != null),
                    riderLocationAutoStopped = autoStopped,
                )
                if (isBoarded && r.pickedUpAt == null) {
                    reloadRide()
                }
            }
        }
    }

    fun sharePassengerSpot(lat: Double, lng: Double) {
        if (!LiveTripLocationService.canSharePassengerWaitingSpot(_state.value.ride)) return
        viewModelScope.launch {
            try {
                val res = container.rides.postPassengerLocation(id, lat, lng)
                val autoStopped = !res.sharing &&
                    (_state.value.ride?.pickedUpAt != null || _state.value.liveLocation?.eta?.phase == "dropoff")
                _state.value = _state.value.copy(
                    sharingPassengerPin = res.sharing,
                    riderLocationAutoStopped = if (autoStopped) true else false,
                )
                refreshLiveLocation()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            }
        }
    }

    fun stopPassengerSpot() {
        viewModelScope.launch {
            try {
                container.rides.stopPassengerLocation(id)
                _state.value = _state.value.copy(
                    sharingPassengerPin = false,
                    riderLocationAutoStopped = false,
                )
                refreshLiveLocation()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            }
        }
    }

    fun clearMessage() { _state.value = _state.value.copy(message = null, staleReload = false) }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { RideDetailViewModel(container, id) }
        }
    }
}
