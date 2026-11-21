package org.pjstatecm.graceride.ui.screens.rides

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.CreateRideReq
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import java.time.LocalDate

data class RideNewUiState(
    val event: EventItem? = null,
    val areas: List<Area> = emptyList(),
    val savedPlaces: List<SavedPlace> = emptyList(),
    val date: LocalDate = LocalDate.now().plusDays(1),
    val hour: Int = 9,
    val minute: Int = 0,
    val areaId: Long? = null,
    val address: String = "",
    val destination: String = "",
    val seats: Int = 1,
    val assistance: String = "",
    val notes: String = "",
    val roundTrip: Boolean = false,
    val returnDate: LocalDate = LocalDate.now().plusDays(1),
    val returnHour: Int = 12,
    val returnMinute: Int = 0,
    val suggestions: List<Place> = emptyList(),
    val submitting: Boolean = false,
    val error: String? = null,
    val createdId: Long? = null,
)

/**
 * A ride is always attached to an event the member has signed up for
 * (backend rule: "Rides exist only for events"). Opened from an event detail
 * page with the event id, mirroring the web's /rides/new?event=.
 */
class RideNewViewModel(private val container: AppContainer, private val eventId: Long?) : ViewModel() {
    private val _state = MutableStateFlow(RideNewUiState())
    val state: StateFlow<RideNewUiState> = _state.asStateFlow()
    private var searchJob: Job? = null

    init {
        viewModelScope.launch {
            try {
                val areas = container.rides.areas()
                val saved = try { container.rides.savedPlaces() } catch (_: ApiException) { emptyList() }
                val defaultPlace = saved.firstOrNull { it.isDefault == 1 } ?: saved.firstOrNull()
                var event: EventItem? = null
                var date = _state.value.date
                var hour = _state.value.hour
                var minute = _state.value.minute
                var destination = _state.value.destination
                if (eventId != null) {
                    event = try { container.events.detail(eventId) } catch (_: ApiException) { null }
                    event?.let { e ->
                        KlTime.parse(e.startsAt)?.let { dt ->
                            date = dt.toLocalDate()
                            hour = dt.hour
                            minute = dt.minute
                        }
                        destination = e.location
                    }
                }
                _state.value = _state.value.copy(
                    event = event,
                    areas = areas.filter { it.active },
                    savedPlaces = saved,
                    date = date, hour = hour, minute = minute,
                    returnDate = date,
                    destination = destination,
                    areaId = defaultPlace?.areaId ?: _state.value.areaId,
                    address = defaultPlace?.address ?: "",
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(error = e.message)
            }
        }
    }

    fun update(f: (RideNewUiState) -> RideNewUiState) { _state.value = f(_state.value) }

    fun onAddressChange(value: String) {
        _state.value = _state.value.copy(address = value)
        searchJob?.cancel()
        if (value.trim().length < 3) {
            _state.value = _state.value.copy(suggestions = emptyList())
            return
        }
        searchJob = viewModelScope.launch {
            delay(400) // debounce: the server rate-limits place search per member
            try {
                _state.value = _state.value.copy(suggestions = container.rides.places(value.trim()))
            } catch (_: ApiException) {
                _state.value = _state.value.copy(suggestions = emptyList())
            }
        }
    }

    fun pickSuggestion(label: String) {
        _state.value = _state.value.copy(address = label, suggestions = emptyList())
    }

    fun submit() {
        val s = _state.value
        if (s.submitting) return
        val event = s.event
        val areaId = s.areaId
        if (event == null) {
            _state.value = s.copy(error = "Choose an event first.")
            return
        }
        if (areaId == null || s.address.isBlank() || s.destination.isBlank()) {
            _state.value = s.copy(error = "Please fill in area, pickup address and destination.")
            return
        }
        val pickupAt = KlTime.apiStamp(s.date, s.hour, s.minute)
        _state.value = s.copy(submitting = true, error = null)
        viewModelScope.launch {
            try {
                val res = container.rides.create(
                    CreateRideReq(
                        pickup_at = pickupAt,
                        pickup_area_id = areaId,
                        pickup_address = s.address.trim(),
                        destination = s.destination.trim(),
                        seats = s.seats.toLong(),
                        assistance = s.assistance.ifBlank { null },
                        notes = s.notes.trim().ifBlank { null },
                        event_id = event.id,
                        leg = "outbound",
                        return_pickup_at = if (s.roundTrip) KlTime.apiStamp(s.returnDate, s.returnHour, s.returnMinute) else null,
                        return_pickup_area_id = if (s.roundTrip) areaId else null,
                        return_pickup_address = if (s.roundTrip) s.address.trim() else null,
                        return_destination = if (s.roundTrip) s.destination.trim() else null,
                    )
                )
                _state.value = _state.value.copy(submitting = false, createdId = res.id)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(submitting = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, eventId: Long?) = viewModelFactory {
            initializer { RideNewViewModel(container, eventId) }
        }
    }
}
