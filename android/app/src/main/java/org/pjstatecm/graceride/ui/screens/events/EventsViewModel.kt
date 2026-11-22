package org.pjstatecm.graceride.ui.screens.events

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime

data class EventsUiState(
    val tab: Int = 0,
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val events: List<EventItem> = emptyList(),
    val myRides: List<Ride> = emptyList(),
    val myDrives: List<Ride> = emptyList(),
    val selectedDay: String = "all",
    val requestsByEvent: Map<Long, Int> = emptyMap(),
)

class EventsViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(EventsUiState())
    val state: StateFlow<EventsUiState> = _state.asStateFlow()

    init { load(first = true) }

    fun setTab(tab: Int) {
        if (tab == _state.value.tab) return
        _state.value = _state.value.copy(tab = tab, selectedDay = "all")
        load(first = true)
    }

    fun selectDay(dayKey: String) {
        _state.value = _state.value.copy(selectedDay = dayKey)
    }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = first, refreshing = !first)
            try {
                val tab = _state.value.tab
                val events = if (tab == 2) container.events.past() else container.events.upcoming()
                val requestsByEvent = if (tab == 0 && container.auth.me?.can("drive") == true) {
                    try {
                        container.rides.driverRequests(mineAreas = true)
                            .mapNotNull { it.event?.id }
                            .groupingBy { it }
                            .eachCount()
                    } catch (_: ApiException) {
                        emptyMap()
                    }
                } else {
                    emptyMap()
                }
                val myRides = if (tab == 1 && container.auth.me?.can("rides") == true) {
                    try {
                        container.rides.upcoming()
                    } catch (_: ApiException) {
                        emptyList()
                    }
                } else {
                    emptyList()
                }
                val myDrives = if (tab == 1 && container.auth.me?.can("drive") == true) {
                    try {
                        container.rides.drivingToday(KlTime.today(), KlTime.todayPlusDays(60))
                    } catch (_: ApiException) {
                        emptyList()
                    }
                } else {
                    emptyList()
                }
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    events = events,
                    myRides = myRides,
                    myDrives = myDrives,
                    requestsByEvent = requestsByEvent,
                    error = null,
                    offline = false,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loading = false, refreshing = false,
                    error = if (e.status == 0) null else e.message,
                    offline = e.status == 0,
                )
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory { initializer { EventsViewModel(container) } }
    }
}
