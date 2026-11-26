package org.pjstatecm.graceride.ui.screens.rides

import androidx.annotation.StringRes
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.R
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.CreateRideReq
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.data.api.SavedPlaceReq
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.inferAreaId
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
    val date: LocalDate = KlTime.todayDate().plusDays(1),
    val hour: Int = 9,
    val minute: Int = 0,
    val areaId: Long? = null,
    val address: String = "",
    val destination: String = "",
    val seats: Int = 1,
    val assistance: String = "",
    val notes: String = "",
    val saveAddress: Boolean = false,
    val saveAddressLabel: String = "",
    val returnDate: LocalDate = KlTime.todayDate().plusDays(1),
    val returnHour: Int = 12,
    val returnMinute: Int = 0,
    val suggestions: List<Place> = emptyList(),
    val pickedPlace: Place? = null,
    val submitting: Boolean = false,
    val loadError: String? = null,
    val error: String? = null,
    @param:StringRes @get:StringRes val errorRes: Int? = null,
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
    private var loadJob: Job? = null
    private var refreshPlacesJob: Job? = null
    private var searchJob: Job? = null
    private var lastAppliedPlaceNotes: String? = null

    init { retryLoad() }

    fun retryLoad() {
        loadJob?.cancel()
        _state.value = _state.value.copy(loading = eventId != null, loadError = null)
        loadJob = viewModelScope.launch {
            try {
                val (areas, saved, proxyRiders, event) = coroutineScope {
                    val areasDef = async { container.rides.areas() }
                    val savedDef = async { try { container.rides.savedPlaces() } catch (_: ApiException) { emptyList() } }
                    val proxiesDef = async { try { container.rides.proxyRiders() } catch (_: ApiException) { emptyList() } }
                    val eventDef = async { eventId?.let { container.events.detail(it) } }
                    LoadResult(
                        areas = areasDef.await(),
                        saved = savedDef.await(),
                        proxyRiders = proxiesDef.await(),
                        event = eventDef.await(),
                    )
                }
                val initialProxy = _state.value.isProxy
                val defaultPlace = if (initialProxy) {
                    null
                } else {
                    saved.firstOrNull { it.isDefault == 1 } ?: saved.firstOrNull()
                }
                val defaultNotes = defaultPlace?.gateNotes?.trim().orEmpty()
                if (defaultNotes.isNotBlank()) {
                    lastAppliedPlaceNotes = defaultNotes
                }
                var date = _state.value.date
                var hour = _state.value.hour
                var minute = _state.value.minute
                var returnDate = _state.value.returnDate
                var returnHour = _state.value.returnHour
                var returnMinute = _state.value.returnMinute
                var destination = _state.value.destination
                if (event != null) {
                    KlTime.parse(event.startsAt)?.let { dt ->
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
                    destination = event.location
                }
                _state.value = _state.value.copy(
                    loading = false,
                    loadError = null,
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
                    notes = if (_state.value.notes.isBlank() && defaultNotes.isNotBlank()) defaultNotes else _state.value.notes,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, loadError = e.message)
            }
        }
    }

    fun update(f: (RideNewUiState) -> RideNewUiState) {
        _state.value = f(_state.value).copy(error = null, errorRes = null)
    }

    fun refreshPlacesAndProxies() {
        refreshPlacesJob?.cancel()
        refreshPlacesJob = viewModelScope.launch {
            val (saved, proxyRiders) = coroutineScope {
                val savedDef = async {
                    try { container.rides.savedPlaces() } catch (_: ApiException) { _state.value.savedPlaces }
                }
                val proxyDef = async {
                    try { container.rides.proxyRiders() } catch (_: ApiException) { _state.value.proxyRiders }
                }
                savedDef.await() to proxyDef.await()
            }
            _state.value = _state.value.copy(savedPlaces = saved, proxyRiders = proxyRiders)
        }
    }

    private data class LoadResult(
        val areas: List<Area>,
        val saved: List<SavedPlace>,
        val proxyRiders: List<ProxyRider>,
        val event: EventItem?,
    )

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
                val comingFromReturn = s.tripType == TripType.RETURN
                val outDate = if (comingFromReturn) dt.toLocalDate() else s.date
                val outMins = if (comingFromReturn) (startMins - 40).coerceAtLeast(0) else (s.hour * 60 + s.minute)
                val defaultRetMins = (startMins + 120).coerceAtMost(23 * 60 + 59)
                val curRetMins = s.returnHour * 60 + s.returnMinute
                val validExistingReturn = !comingFromReturn && (
                    s.returnDate.isAfter(outDate) ||
                        (s.returnDate == outDate && curRetMins > outMins)
                    )
                val retDate = if (validExistingReturn) s.returnDate else outDate
                val retMins = if (validExistingReturn) {
                    curRetMins
                } else {
                    maxOf(defaultRetMins, (outMins + 30).coerceAtMost(23 * 60 + 59))
                }
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
                    date = outDate,
                    hour = outMins / 60,
                    minute = outMins % 60,
                    returnDate = if (newType == TripType.ROUND) retDate else s.returnDate,
                    returnHour = if (newType == TripType.ROUND) retMins / 60 else s.returnHour,
                    returnMinute = if (newType == TripType.ROUND) retMins % 60 else s.returnMinute,
                    destination = ev.location,
                    address = homeAddr,
                    suggestions = emptyList(),
                )
            }
        }
    }

    fun setProxyMode(proxy: Boolean) {
        val s = _state.value
        if (s.isProxy == proxy) return
        val cleanedNotes = stripAppliedNotes(s.notes, lastAppliedPlaceNotes)
        val defaultPlace = s.savedPlaces.firstOrNull { it.isDefault == 1 } ?: s.savedPlaces.firstOrNull()
        val isReturn = s.tripType == TripType.RETURN
        if (proxy) {
            lastAppliedPlaceNotes = null
            val defaultAddr = defaultPlace?.address
            _state.value = s.copy(
                isProxy = true,
                saveAddress = false,
                saveAddressLabel = "",
                notes = cleanedNotes,
                address = if (!isReturn && s.address == defaultAddr) "" else s.address,
                destination = if (isReturn && s.destination == defaultAddr) "" else s.destination,
            )
        } else {
            val gate = defaultPlace?.gateNotes?.trim().orEmpty()
            lastAppliedPlaceNotes = gate.ifEmpty { null }
            val mergedNotes = when {
                gate.isEmpty() -> cleanedNotes
                cleanedNotes.contains(gate) -> cleanedNotes
                cleanedNotes.isEmpty() -> gate
                else -> "$cleanedNotes\n$gate"
            }
            val shouldFill = defaultPlace != null && (if (isReturn) s.destination.isBlank() else s.address.isBlank())
            _state.value = s.copy(
                isProxy = false,
                areaId = if (shouldFill && defaultPlace?.areaId != null) defaultPlace.areaId else s.areaId,
                address = if (shouldFill && !isReturn) defaultPlace!!.address else s.address,
                destination = if (shouldFill && isReturn) defaultPlace!!.address else s.destination,
                notes = mergedNotes,
            )
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
        val combinedProxyNotes = listOf(rider.gateNotes.trim(), rider.notes.trim())
            .filter { it.isNotBlank() }
            .joinToString(" · ")
        val updatedNotes = when {
            combinedProxyNotes.isNotBlank() && (s.notes.isBlank() || s.notes == lastAppliedPlaceNotes) -> {
                lastAppliedPlaceNotes = combinedProxyNotes
                combinedProxyNotes
            }
            else -> s.notes
        }
        _state.value = s.copy(
            isProxy = true,
            riderName = rider.name,
            riderPhone = rider.phone,
            riderRelationship = rider.relationship.ifBlank { "parent" },
            areaId = rider.areaId ?: s.areaId,
            address = updatedAddress,
            destination = updatedDestination,
            notes = updatedNotes,
        )
    }

    fun onAddressChange(value: String) {
        _state.value = _state.value.copy(address = value)
        if (_state.value.tripType != TripType.RETURN) searchPlaces(value)
    }

    fun onDestinationChange(value: String) {
        _state.value = _state.value.copy(destination = value)
        if (_state.value.tripType == TripType.RETURN) searchPlaces(value)
    }

    private fun searchPlaces(value: String) {
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
        val cur = _state.value
        val inferred = cur.areaId ?: inferAreaId(label, cur.areas)
        _state.value = if (cur.tripType == TripType.RETURN) {
            cur.copy(destination = label, areaId = inferred, suggestions = emptyList())
        } else {
            cur.copy(address = label, areaId = inferred, suggestions = emptyList())
        }
    }

    fun pickSuggestion(place: Place) {
        val cur = _state.value
        val inferred = cur.areaId ?: inferAreaId(place.label, cur.areas)
        _state.value = if (cur.tripType == TripType.RETURN) {
            cur.copy(
                destination = place.label,
                pickedPlace = place,
                areaId = inferred,
                suggestions = emptyList(),
            )
        } else {
            cur.copy(
                address = place.label,
                pickedPlace = place,
                areaId = inferred,
                suggestions = emptyList(),
            )
        }
    }

    private fun stripAppliedNotes(curNotes: String, applied: String?): String {
        val clean = applied?.trim().orEmpty()
        if (clean.isEmpty()) return curNotes
        if (curNotes.trim() == clean) return ""
        return curNotes
            .split("\n")
            .filter { it.trim() != clean }
            .joinToString("\n")
            .trim()
    }

    fun applySavedPlace(place: SavedPlace) {
        val s = _state.value
        val prevNotes = lastAppliedPlaceNotes
        val nextNotes = stripAppliedNotes(s.notes, prevNotes)
        val gate = place.gateNotes.trim()
        val mergedNotes = when {
            gate.isEmpty() -> nextNotes
            nextNotes.contains(gate) -> nextNotes
            nextNotes.isEmpty() -> gate
            else -> "$nextNotes\n$gate"
        }
        lastAppliedPlaceNotes = gate.ifEmpty { null }
        val isReturn = s.tripType == TripType.RETURN
        _state.value = s.copy(
            areaId = place.areaId ?: s.areaId,
            address = if (isReturn) s.address else place.address,
            destination = if (isReturn) place.address else s.destination,
            notes = mergedNotes,
            suggestions = emptyList(),
            saveAddress = false,
            saveAddressLabel = "",
        )
    }

    fun submit() {
        val s = _state.value
        if (s.submitting) return
        val event = s.event
        val areaId = s.areaId
        if (event == null) {
            _state.value = s.copy(error = null, errorRes = R.string.ride_new_err_no_event)
            return
        }
        if (event.mySignup?.status != "going") {
            _state.value = s.copy(error = null, errorRes = R.string.ride_new_err_need_signup)
            return
        }
        if (s.isProxy && s.riderName.isBlank()) {
            _state.value = s.copy(error = null, errorRes = R.string.ride_new_err_rider_name)
            return
        }
        if (areaId == null || s.address.isBlank() || s.destination.isBlank()) {
            _state.value = s.copy(error = null, errorRes = R.string.ride_new_err_required)
            return
        }
        val pickupAt = KlTime.apiStamp(s.date, s.hour, s.minute)
        val isReturn = s.tripType == TripType.RETURN
        val isRound = s.tripType == TripType.ROUND
        val memberAddr = (if (isReturn) s.destination else s.address).trim()
        val label = s.saveAddressLabel.trim()
        val alreadySaved = s.savedPlaces.any { it.address.trim().equals(memberAddr, ignoreCase = true) }
        val isEventLocation = memberAddr.equals(event.location.trim(), ignoreCase = true)
        val shouldSaveAddress = !s.isProxy && s.saveAddress && memberAddr.isNotEmpty() && !alreadySaved && !isEventLocation && s.savedPlaces.size < 50
        if (shouldSaveAddress && label.isEmpty()) {
            _state.value = s.copy(error = null, errorRes = R.string.ride_new_err_save_label)
            return
        }
        val returnPickupAt = if (isRound) KlTime.apiStamp(s.returnDate, s.returnHour, s.returnMinute) else null
        if (isRound && returnPickupAt != null && returnPickupAt <= pickupAt) {
            _state.value = s.copy(error = null, errorRes = R.string.ride_new_err_return_after)
            return
        }
        _state.value = s.copy(submitting = true, error = null, errorRes = null)
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
                if (shouldSaveAddress) {
                    val matchingCoords = s.pickedPlace?.takeIf {
                        it.label.trim().equals(memberAddr, ignoreCase = true)
                    }
                    try {
                        container.rides.createSavedPlace(
                            SavedPlaceReq(
                                label = label,
                                area_id = areaId,
                                address = memberAddr,
                                gate_notes = s.notes.trim().take(300),
                                is_default = s.savedPlaces.isEmpty(),
                                lat = matchingCoords?.lat,
                                lng = matchingCoords?.lng,
                            )
                        )
                    } catch (_: ApiException) { }
                }
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
