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

data class EventDetailUiState(
    val loading: Boolean = true,
    val event: EventItem? = null,
    val outboundRide: Ride? = null,
    val returnRide: Ride? = null,
    val drivingForEvent: List<Ride> = emptyList(),
    val error: String? = null,
    val busy: Boolean = false,
    val message: String? = null,
    val actionNotice: EventActionNotice? = null,
)

class EventDetailViewModel(private val container: AppContainer, private val id: Long) : ViewModel() {
    private val _state = MutableStateFlow(EventDetailUiState())
    val state: StateFlow<EventDetailUiState> = _state.asStateFlow()

    init { load() }

    fun load() {
        viewModelScope.launch {
            reloadDetail()
        }
    }

    private suspend fun reloadDetail() {
        _state.value = _state.value.copy(loading = _state.value.event == null)
        try {
            val event = container.events.detail(id)
            var outbound: Ride? = null
            var returnRide: Ride? = null
            var driving: List<Ride> = emptyList()

            if (event.mySignup?.status == "going") {
                val canRides = container.auth.me?.can("rides") == true
                if (canRides && (event.myRide != null || event.myReturnRide != null)) {
                    val upcoming = try {
                        container.rides.upcoming()
                    } catch (_: ApiException) {
                        emptyList()
                    }
                    outbound = event.myRide?.id?.let { rid -> upcoming.find { it.id == rid } }
                    returnRide = event.myReturnRide?.id?.let { rid -> upcoming.find { it.id == rid } }
                }
                if (container.auth.me?.can("drive") == true) {
                    val day = KlTime.date(event.startsAt)
                    if (day.length == 10) {
                        driving = try {
                            container.rides.drivingToday(day, day).filter { it.event?.id == event.id }
                        } catch (_: ApiException) {
                            emptyList()
                        }
                    }
                }
            }

            _state.value = _state.value.copy(
                loading = false,
                event = event,
                outboundRide = outbound,
                returnRide = returnRide,
                drivingForEvent = driving,
                error = null,
            )
        } catch (e: ApiException) {
            _state.value = _state.value.copy(loading = false, error = e.message)
        }
    }

    fun signup(needRide: Boolean, offerDrive: Boolean, asGroupId: Long?) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.signup(id, needRide, offerDrive, asGroupId)
                reloadDetail()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun withdraw() {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.withdraw(id)
                reloadDetail()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun publish() {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null, actionNotice = null)
        viewModelScope.launch {
            try {
                container.events.publish(id)
                reloadDetail()
                _state.value = _state.value.copy(actionNotice = EventActionNotice.PUBLISHED)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun unpublish() {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null, actionNotice = null)
        viewModelScope.launch {
            try {
                container.events.unpublish(id)
                reloadDetail()
                _state.value = _state.value.copy(actionNotice = EventActionNotice.UNPUBLISHED)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun clearMessage() { _state.value = _state.value.copy(message = null) }

    fun clearNotice() { _state.value = _state.value.copy(actionNotice = null) }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { EventDetailViewModel(container, id) }
        }
    }
}
