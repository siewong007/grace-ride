package org.pjstatecm.graceride.ui.screens.rides

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.builtins.ListSerializer
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.RealtimeEvent

data class RidesUiState(
    val tab: Int = 0,
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val rides: List<Ride> = emptyList(),
)

class RidesViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(RidesUiState())
    val state: StateFlow<RidesUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) refresh()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
    }

    fun setTab(tab: Int) {
        if (tab == _state.value.tab) return
        _state.value = _state.value.copy(tab = tab, rides = emptyList(), error = null, loading = true)
        load(first = true)
    }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        loadJob?.cancel()
        val tab = _state.value.tab
        val cacheKey = if (tab == 0) "rides_upcoming" else "rides_past"
        val uid = container.auth.me?.id
        loadJob = viewModelScope.launch {
            if (first && uid != null) {
                val cached = withContext(Dispatchers.IO) {
                    container.offlineCache.read(uid, cacheKey, ListSerializer(Ride.serializer()))
                }
                if (cached != null && _state.value.rides.isEmpty() && _state.value.tab == tab) {
                    _state.value = _state.value.copy(
                        loading = false,
                        refreshing = true,
                        rides = cached.data,
                    )
                } else if (_state.value.tab == tab) {
                    _state.value = _state.value.copy(loading = true, refreshing = false)
                }
            } else {
                _state.value = _state.value.copy(
                    loading = first && _state.value.rides.isEmpty(),
                    refreshing = !first || _state.value.rides.isNotEmpty(),
                )
            }
            try {
                val rides = if (tab == 0) container.rides.upcoming() else container.rides.past()
                if (uid != null) {
                    withContext(Dispatchers.IO) {
                        container.offlineCache.write(uid, cacheKey, rides, ListSerializer(Ride.serializer()))
                    }
                }
                if (_state.value.tab == tab) {
                    _state.value = _state.value.copy(loading = false, refreshing = false, rides = rides, error = null, offline = false)
                }
            } catch (e: ApiException) {
                if (_state.value.tab == tab) {
                    _state.value = _state.value.copy(
                        loading = false, refreshing = false,
                        error = if (e.status == 0) null else e.message,
                        offline = e.status == 0,
                    )
                }
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { RidesViewModel(container) } }
    }
}
