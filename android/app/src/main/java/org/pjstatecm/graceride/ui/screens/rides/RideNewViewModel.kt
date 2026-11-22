package org.pjstatecm.graceride.ui.screens.rides

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
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
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import java.time.LocalDate

enum class TripType { OUTBOUND, ROUND, RETURN }

data class RideNewUiState(
    val loading: Boolean = false,
    val event: EventItem? = null,
    val areas: List<Area> = emptyList(),
    val savedPlaces: List<SavedPlace> = emptyList(),
    val proxyRiders: List<ProxyRider> = emptyList(),
    val tripType: TripType = TripType.OUTBOUND,
    val isProxy: Boolean = false,
    val riderName: String = "",
    val riderPhone: String = "",
    val riderRelationship: String = "parent",
    val saveProxyRider: Boolean = false,
    val date: LocalDate = LocalDate.now().plusDays(1),
    val hour: Int = 9,
    val minute: Int = 0,
    val areaId: Long? = null,
    val address: String = "",
    val destination: String = "",
    val seats: Int = 1,
    val assistance: String = "",
    val notes: String = "",
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
class RideNewViewModel(
    private val container: AppContainer,
    private val eventId: Long?,
    initialProxy: Boolean = false,
) : ViewModel() {
    private val _state = MutableStateFlow(
        RideNewUiState(loading = eventId != null, isProxy = initialProxy)
    )
    val state: StateFlow<RideNewUiState> = _state.asStateFlow()
    private var searchJob: Job? = null

    init {
        viewModelScope.launch {
            try {
                val areas = container.rides.areas()
                val saved = try { container.rides.savedPlaces() } catch (_: ApiException) { emptyList() }
                val proxyRiders = try { container.rides.proxyRiders() } catch (_: ApiException) { emptyList() }
                val defaultPlace = saved.firstOrNull { it.isDefault == 1 } ?: saved.firstOrNull()
                var event: EventItem? = null
                var date = _state.value.date
                var hour = _state.value.hour
                var minute = _state.value.minute
                var returnDate = _state.value.returnDate
                var returnHour = _state.value.returnHour
                var returnMinute = _state.value.returnMinute
                var destination = _state.value.destination
                if (eventId != null) {
                    event = try { container.events.detail(eventId) } catch (_: ApiException) { null }
                    event?.let { e ->
                        KlTime.parse(e.startsAt)?.let { dt ->
                            val startMins = dt.hour * 60 + dt.minute
                            val outMins = (startMins - 40).coerceAtLeast(0)
                            val retMins = (startMins + 120).coerceAtMost(23 * 60 + 59)
                            date = dt.toLocalDate()
                            hour = outMins / 60
                            minute = outMins % 60
                            returnDate = date
                            returnHour = retMins / 60
                            returnMinute = retMins % 60
                        }
                        destination = e.location
                    }
                }
                _state.value = _state.value.copy(
                    loading = false,
                    event = event,
                    areas = areas.filter { it.active },
                    savedPlaces = saved,
                    proxyRiders = proxyRiders,
                    date = date,
                    hour = hour,
                    minute = minute,
                    returnDate = returnDate,
                    returnHour = returnHour,
                    returnMinute = returnMinute,
                    destination = destination,
                    areaId = defaultPlace?.areaId ?: _state.value.areaId,
                    address = defaultPlace?.address ?: "",
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun update(f: (RideNewUiState) -> RideNewUiState) { _state.value = f(_state.value) }

    fun setTripType(newType: TripType) {
        val s = _state.value
        val ev = s.event
        val dt = ev?.let { KlTime.parse(it.startsAt) }
        if (ev == null || dt == null) {
            if (newType == TripType.ROUND) {
                val outMins = s.hour * 60 + s.minute
                val curRetMins = s.returnHour * 60 + s.returnMinute
                val retMins = if (s.returnDate.isAfter(s.date) || curRetMins > outMins) {
                    curRetMins
                } else {
                    (outMins + 120).coerceAtMost(23 * 60 + 59)
                }
                _state.value = s.copy(
                    tripType = newType,
                    returnHour = retMins / 60,
                    returnMinute = retMins % 60,
                )
            } else {
                _state.value = s.copy(tripType = newType)
            }
            return
        }

        val defaultHomeAddress =
            s.savedPlaces.firstOrNull { it.isDefault == 1 }?.address
                ?: s.savedPlaces.firstOrNull()?.address
                ?: ""
        val startMins = dt.hour * 60 + dt.minute

        when (newType) {
            TripType.RETURN -> {
                val retMins = (startMins + 120).coerceAtMost(23 * 60 + 59)
                val homeDest = when {
                    s.address.isNotBlank() && s.address != ev.location -> s.address
                    s.destination.isNotBlank() && s.destination != ev.location -> s.destination
                    else -> defaultHomeAddress
                }
                _state.value = s.copy(
                    tripType = newType,
                    date = dt.toLocalDate(),
                    hour = retMins / 60,
                    minute = retMins % 60,
                    address = ev.location,
                    destination = homeDest,
                    suggestions = emptyList(),
                )
            }
            TripType.OUTBOUND, TripType.ROUND -> {
                val outMins = (startMins - 40).coerceAtLeast(0)
                val retMins = (startMins + 120).coerceAtMost(23 * 60 + 59)
                val homeAddr = if (s.address == ev.location) {
                    if (s.destination.isNotBlank() && s.destination != ev.location) {
                        s.destination
                    } else {
                        defaultHomeAddress
                    }
                } else {
                    s.address
                }
                _state.value = s.copy(
                    tripType = newType,
                    date = dt.toLocalDate(),
                    hour = outMins / 60,
                    minute = outMins % 60,
                    returnDate = if (newType == TripType.ROUND) dt.toLocalDate() else s.returnDate,
                    returnHour = if (newType == TripType.ROUND) retMins / 60 else s.returnHour,
                    returnMinute = if (newType == TripType.ROUND) retMins % 60 else s.returnMinute,
                    destination = ev.location,
                    address = homeAddr,
                )
            }
        }
    }

    fun selectProxyRider(rider: ProxyRider) {
        val s = _state.value
        val updatedAddress = if (s.tripType != TripType.RETURN && rider.pickupAddress.isNotBlank()) {
            rider.pickupAddress
        } else {
            s.address
        }
        val updatedDestination = if (s.tripType == TripType.RETURN && rider.pickupAddress.isNotBlank()) {
            rider.pickupAddress
        } else {
            s.destination
        }
        _state.value = s.copy(
            isProxy = true,
            riderName = rider.name,
            riderPhone = rider.phone,
            riderRelationship = rider.relationship.ifBlank { "parent" },
            areaId = rider.areaId ?: s.areaId,
            address = updatedAddress,
            destination = updatedDestination,
            notes = if (s.notes.isBlank() && rider.notes.isNotBlank()) rider.notes else s.notes,
        )
    }

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
        if (event.mySignup?.status != "going") {
            _state.value = s.copy(error = "Please sign up for this event first.")
            return
        }
        if (s.isProxy && s.riderName.isBlank()) {
            _state.value = s.copy(error = "Please enter the rider's name.")
            return
        }
        if (areaId == null || s.address.isBlank() || s.destination.isBlank()) {
            _state.value = s.copy(error = "Please fill in area, pickup address and destination.")
            return
        }
        val pickupAt = KlTime.apiStamp(s.date, s.hour, s.minute)
        val isReturn = s.tripType == TripType.RETURN
        val isRound = s.tripType == TripType.ROUND
        val returnPickupAt = if (isRound) KlTime.apiStamp(s.returnDate, s.returnHour, s.returnMinute) else null
        if (isRound && returnPickupAt != null && returnPickupAt <= pickupAt) {
            _state.value = s.copy(error = "Return pickup time must be after outbound pickup time.")
            return
        }
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
                        leg = if (isReturn) "return" else "outbound",
                        is_proxy = if (s.isProxy) true else null,
                        rider_name = if (s.isProxy) s.riderName.trim() else null,
                        rider_phone = if (s.isProxy) s.riderPhone.trim().ifBlank { null } else null,
                        rider_relationship = if (s.isProxy) s.riderRelationship else null,
                        save_proxy_rider = if (s.isProxy && s.saveProxyRider) true else null,
                        return_pickup_at = returnPickupAt,
                        return_pickup_area_id = if (isRound) areaId else null,
                        return_pickup_address = if (isRound) s.destination.trim() else null,
                        return_destination = if (isRound) s.address.trim() else null,
                    )
                )
                _state.value = _state.value.copy(submitting = false, createdId = res.id)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(submitting = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, eventId: Long?, initialProxy: Boolean = false) = viewModelFactory {
            initializer { RideNewViewModel(container, eventId, initialProxy) }
        }
    }
}
