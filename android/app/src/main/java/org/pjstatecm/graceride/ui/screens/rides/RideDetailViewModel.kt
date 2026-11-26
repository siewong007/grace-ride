package org.pjstatecm.graceride.ui.screens.rides

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideLocationDto
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
)

class RideDetailViewModel(private val container: AppContainer, private val id: Long) : ViewModel() {
    private val _state = MutableStateFlow(RideDetailUiState())
    val state: StateFlow<RideDetailUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load()
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume) load()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
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
                runCatching { container.rides.rideLocation(id) }.getOrNull()
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
            _state.value = _state.value.copy(
                loading = false,
                refreshing = false,
                ride = ride,
                liveLocation = loc,
                sharingPassengerPin = if (isBoarded) false else (loc?.passenger != null),
                riderLocationAutoStopped = autoStopped,
                error = null,
            )
        } catch (e: ApiException) {
            _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
        }
    }

    private fun action(
        onSuccess: (() -> Unit)? = null,
        block: suspend () -> Unit,
    ) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                block()
                reloadRide()
                onSuccess?.invoke()
            } catch (e: ApiException) {
                if (e.code == "RIDE_STATUS_CHANGED") {
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

    private suspend fun stopLiveLocationIfNoActiveTrip() {
        val ctx = container.appContext ?: return
        if (!LiveTripLocationService.isSharing.value) return
        val stillLive = runCatching { container.rides.driverRoute().route?.live == true }.getOrDefault(false)
        if (!stillLive) {
            LiveTripLocationService.stop(ctx, notifyServer = false)
        }
    }

    fun cancel(reason: String?) = action {
        container.rides.cancel(id, reason)
        stopLiveLocationIfNoActiveTrip()
    }
    fun respond(accept: Boolean) = action { container.rides.respond(id, accept) }
    fun readiness(status: String) = action { container.rides.readiness(id, status) }
    fun confirmAssignment() = action { container.rides.confirmAssignment(id) }
    fun startRide(onStarted: ((Ride) -> Unit)? = null) = action(
        onSuccess = { if (onStarted != null) _state.value.ride?.let(onStarted) },
    ) {
        container.rides.startRide(id)
    }
    fun arrivedGate() = action { container.rides.arrivedGate(id) }
    fun pickup() = action { container.rides.pickupRide(id) }
    fun complete() = action {
        container.rides.completeRide(id)
        stopLiveLocationIfNoActiveTrip()
    }
    fun driverDelay(minutes: Int) = action { container.rides.driverDelay(id, minutes) }
    fun withdraw(reason: String?) = action {
        container.rides.withdrawRide(id, reason)
        stopLiveLocationIfNoActiveTrip()
    }
    fun refreshLiveLocation() {
        val r = _state.value.ride ?: return
        val canViewLoc = r.viewer.isPassenger || r.viewer.isDriver || r.viewer.isStaff
        if (!LiveTripLocationService.canShareRideLocation(r) || !canViewLoc) return
        viewModelScope.launch {
            runCatching { container.rides.rideLocation(id) }.getOrNull()?.let { loc ->
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
            runCatching { container.rides.stopPassengerLocation(id) }
            _state.value = _state.value.copy(
                sharingPassengerPin = false,
                riderLocationAutoStopped = false,
            )
            refreshLiveLocation()
        }
    }

    fun clearMessage() { _state.value = _state.value.copy(message = null, staleReload = false) }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { RideDetailViewModel(container, id) }
        }
    }
}
