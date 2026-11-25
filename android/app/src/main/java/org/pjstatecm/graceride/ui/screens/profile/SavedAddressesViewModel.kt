package org.pjstatecm.graceride.ui.screens.profile

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
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.SavedPlace
import org.pjstatecm.graceride.data.api.SavedPlaceReq
import org.pjstatecm.graceride.di.AppContainer

data class SavedAddressesUiState(
    val loading: Boolean = true,
    val places: List<SavedPlace> = emptyList(),
    val areas: List<Area> = emptyList(),
    val suggestions: List<Place> = emptyList(),
    val busy: Boolean = false,
    val message: String? = null,
    val error: String? = null,
)

class SavedAddressesViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(SavedAddressesUiState())
    val state: StateFlow<SavedAddressesUiState> = _state.asStateFlow()
    private var searchJob: Job? = null

    init {
        load()
    }

    fun load() {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = _state.value.places.isEmpty(), error = null)
            try {
                val places = container.rides.savedPlaces()
                val areas = try {
                    container.rides.areas().filter { it.active }
                } catch (_: ApiException) {
                    _state.value.areas
                }
                _state.value = _state.value.copy(
                    loading = false,
                    places = places,
                    areas = areas,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun searchAddress(q: String) {
        searchJob?.cancel()
        val trimmed = q.trim()
        if (trimmed.length < 3) {
            _state.value = _state.value.copy(suggestions = emptyList())
            return
        }
        searchJob = viewModelScope.launch {
            delay(400)
            try {
                _state.value = _state.value.copy(suggestions = container.rides.places(trimmed))
            } catch (_: ApiException) {
                _state.value = _state.value.copy(suggestions = emptyList())
            }
        }
    }

    fun clearSuggestions() {
        searchJob?.cancel()
        _state.value = _state.value.copy(suggestions = emptyList())
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun save(
        editingId: Long?,
        label: String,
        areaId: Long?,
        address: String,
        gateNotes: String,
        isDefault: Boolean,
        lat: Double? = null,
        lng: Double? = null,
        onDone: () -> Unit = {},
    ) {
        if (_state.value.busy) return
        val cleanLabel = label.trim()
        val cleanAddress = address.trim()
        if (cleanLabel.isEmpty() || cleanAddress.isEmpty()) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                val req = SavedPlaceReq(
                    label = cleanLabel,
                    area_id = areaId,
                    address = cleanAddress,
                    gate_notes = gateNotes.trim(),
                    is_default = isDefault,
                    lat = lat,
                    lng = lng,
                )
                if (editingId == null) {
                    container.rides.createSavedPlace(req)
                } else {
                    container.rides.updateSavedPlace(editingId, req)
                }
                val places = container.rides.savedPlaces()
                _state.value = _state.value.copy(
                    busy = false,
                    places = places,
                    suggestions = emptyList(),
                    message = "saved",
                )
                onDone()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun setDefault(place: SavedPlace) {
        if (_state.value.busy || place.isDefault == 1) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                container.rides.updateSavedPlace(
                    place.id,
                    SavedPlaceReq(
                        label = place.label,
                        area_id = place.areaId,
                        address = place.address,
                        gate_notes = place.gateNotes,
                        is_default = true,
                        lat = place.lat,
                        lng = place.lng,
                    ),
                )
                val places = container.rides.savedPlaces()
                _state.value = _state.value.copy(
                    busy = false,
                    places = places,
                    message = "default_updated",
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun delete(id: Long, onDone: () -> Unit = {}) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                container.rides.deleteSavedPlace(id)
                val places = container.rides.savedPlaces()
                _state.value = _state.value.copy(
                    busy = false,
                    places = places,
                    message = "removed",
                )
                onDone()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { SavedAddressesViewModel(container) }
        }
    }
}
