package org.pjstatecm.graceride.ui.screens.drive

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.TripRouteDto
import org.pjstatecm.graceride.data.api.runSuspendCatching
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.notifications.RealtimeEvent
import org.pjstatecm.graceride.service.LiveTripLocationService

data class DriveHubUiState(
    val tab: Int = 0,
    val mineAreasOnly: Boolean = true,
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val busyRideId: Long? = null,
    val requests: List<Ride> = emptyList(),
    val schedule: List<Ride> = emptyList(),
    val route: TripRouteDto? = null,
    val offline: Boolean = false,
    val accessDenied: Boolean = false,
    val error: String? = null,
    val actionError: String? = null,
    val claimedNotice: Boolean = false,
)

class DriveHubViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(DriveHubUiState())
    val state: StateFlow<DriveHubUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) load(first = false)
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
    }

    fun setTab(tab: Int) {
        if (tab == _state.value.tab) return
        _state.value = _state.value.copy(tab = tab)
    }

    fun toggleMineAreas() {
        _state.value = _state.value.copy(mineAreasOnly = !_state.value.mineAreasOnly)
        load(first = false)
    }

    fun refresh() = load(first = false)

    fun load(first: Boolean = false) {
        loadJob?.cancel()
        if (container.auth.me?.can("drive", "view") != true) {
            _state.value = _state.value.copy(
                loading = false,
                refreshing = false,
                accessDenied = true,
                error = null,
            )
            return
        }
        val mineOnly = _state.value.mineAreasOnly
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                accessDenied = false,
                loading = first && _state.value.requests.isEmpty() && _state.value.schedule.isEmpty(),
                refreshing = !first || _state.value.requests.isNotEmpty() || _state.value.schedule.isNotEmpty(),
                error = null,
            )
            try {
                val from = KlTime.today()
                val to = KlTime.todayPlusDays(14)
                val (reqs, sched, routeResult) = coroutineScope {
                    val reqDef = async { container.rides.driverRequests(mineOnly) }
                    val schedDef = async { container.rides.drivingToday(from, to) }
                    val routeDef = async {
                        runSuspendCatching { container.rides.driverRoute().route }
                    }
                    Triple(reqDef.await(), schedDef.await(), routeDef.await())
                }
                val nextRoute = if (routeResult.isSuccess) {
                    val fetched = routeResult.getOrNull()
                    if (!LiveTripLocationService.isJourneyLive(fetched) && LiveTripLocationService.isSharing.value) {
                        container.appContext?.let { LiveTripLocationService.stop(it, notifyServer = false) }
                    }
                    fetched
                } else {
                    _state.value.route
                }
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    requests = reqs,
                    schedule = sched.sortedWith(compareBy<Ride>({ KlTime.sortKey(it.pickupAt) }, { it.id })),
                    route = nextRoute,
                    offline = false,
                    error = null,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    offline = e.status == 0,
                    error = if (e.status == 0) null else e.message,
                )
            }
        }
    }

    fun claim(rideId: Long) {
        if (_state.value.busyRideId != null) return
        _state.value = _state.value.copy(busyRideId = rideId, actionError = null, claimedNotice = false)
        viewModelScope.launch {
            try {
                container.rides.claimRide(rideId)
                _state.value = _state.value.copy(busyRideId = null, claimedNotice = true)
                load(first = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busyRideId = null, actionError = e.message)
            }
        }
    }

    fun clearClaimedNotice() {
        _state.value = _state.value.copy(claimedNotice = false)
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun clearActionError() {
        _state.value = _state.value.copy(actionError = null)
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { DriveHubViewModel(container) }
        }
    }
}
