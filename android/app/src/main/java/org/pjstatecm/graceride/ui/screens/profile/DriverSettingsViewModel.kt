package org.pjstatecm.graceride.ui.screens.profile

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.DriverProfile
import org.pjstatecm.graceride.data.api.DriverProfileReq
import org.pjstatecm.graceride.data.api.DriverVehicle
import org.pjstatecm.graceride.data.api.DriverVehicleReq
import org.pjstatecm.graceride.di.AppContainer

data class DriverSettingsUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val busy: Boolean = false,
    val areas: List<Area> = emptyList(),
    val vehicles: List<DriverVehicle> = emptyList(),
    val activeVehicleId: Long? = null,
    val carpoolEnabled: Boolean = false,
    val spareSeats: Int = 1,
    val gateWaitMins: Int = 5,
    val selectedAreaIds: Set<Long> = emptySet(),
    val dirty: Boolean = false,
    val accessDenied: Boolean = false,
    val message: String? = null,
    val error: String? = null,
) {
    val activeVehicle: DriverVehicle?
        get() = vehicles.firstOrNull { activeVehicleId != null && it.id == activeVehicleId }
            ?: vehicles.firstOrNull { it.isActive }
            ?: vehicles.firstOrNull()

    val maxSpareSeats: Int
        get() = activeVehicle?.let { (it.seats - 1).coerceIn(1, 7) } ?: 6
}

class DriverSettingsViewModel(private val container: AppContainer) : ViewModel() {

    private data class SavedPrefs(
        val carpoolEnabled: Boolean,
        val spareSeats: Int,
        val gateWaitMins: Int,
        val areaIds: Set<Long>,
    )

    private val _state = MutableStateFlow(DriverSettingsUiState())
    val state: StateFlow<DriverSettingsUiState> = _state.asStateFlow()
    private var savedPrefs: SavedPrefs? = null
    private var loadJob: Job? = null

    init {
        load()
    }

    fun load() = loadInternal(refreshing = false)

    fun refresh() = loadInternal(refreshing = savedPrefs != null)

    private fun loadInternal(refreshing: Boolean) {
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
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                accessDenied = false,
                loading = savedPrefs == null && !refreshing,
                refreshing = refreshing,
                error = null,
            )
            try {
                val (profile, areas) = coroutineScope {
                    val profileDef = async { container.rides.driverProfile() }
                    val areasDef = async {
                        try {
                            container.rides.areas().filter { it.active }
                        } catch (_: ApiException) {
                            _state.value.areas
                        }
                    }
                    profileDef.await() to areasDef.await()
                }
                applyProfile(profile, areas, resetPrefs = true)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
            }
        }
    }

    private fun currentPrefsFromState(s: DriverSettingsUiState): SavedPrefs =
        SavedPrefs(
            carpoolEnabled = s.carpoolEnabled,
            spareSeats = s.spareSeats,
            gateWaitMins = s.gateWaitMins,
            areaIds = s.selectedAreaIds,
        )

    private fun applyProfile(
        profile: DriverProfile,
        areas: List<Area>,
        resetPrefs: Boolean,
        message: String? = null,
    ) {
        val vehicles = profile.vehicles.ifEmpty {
            listOfNotNull(profile.vehicle?.copy(isActive = true))
        }
        val active = vehicles.firstOrNull { profile.activeVehicleId != null && it.id == profile.activeVehicleId }
            ?: vehicles.firstOrNull { it.isActive }
            ?: vehicles.firstOrNull()
        val maxSpare = active?.let { (it.seats - 1).coerceIn(1, 7) } ?: 6

        if (resetPrefs || savedPrefs == null) {
            val prefs = SavedPrefs(
                carpoolEnabled = profile.carpoolEnabled,
                spareSeats = profile.spareSeats.coerceIn(1, maxSpare),
                gateWaitMins = profile.gateWaitMins,
                areaIds = profile.areaIds.toSet(),
            )
            savedPrefs = prefs
            _state.value = _state.value.copy(
                loading = false,
                refreshing = false,
                busy = false,
                areas = areas,
                vehicles = vehicles,
                activeVehicleId = active?.id,
                carpoolEnabled = prefs.carpoolEnabled,
                spareSeats = prefs.spareSeats,
                gateWaitMins = prefs.gateWaitMins,
                selectedAreaIds = prefs.areaIds,
                dirty = false,
                message = message,
                error = null,
            )
        } else {
            val prevSaved = savedPrefs!!
            val nextSaved = prevSaved.copy(spareSeats = prevSaved.spareSeats.coerceIn(1, maxSpare))
            savedPrefs = nextSaved
            val cur = _state.value
            val clampedSpare = cur.spareSeats.coerceIn(1, maxSpare)
            val updated = cur.copy(
                loading = false,
                refreshing = false,
                busy = false,
                areas = areas,
                vehicles = vehicles,
                activeVehicleId = active?.id,
                spareSeats = clampedSpare,
                message = message,
                error = null,
            )
            _state.value = updated.copy(dirty = currentPrefsFromState(updated) != nextSaved)
        }
    }

    private fun updatePrefs(transform: (DriverSettingsUiState) -> DriverSettingsUiState) {
        val next = transform(_state.value)
        val isDirty = savedPrefs?.let { currentPrefsFromState(next) != it } ?: false
        _state.value = next.copy(dirty = isDirty, error = null)
    }

    fun toggleArea(areaId: Long) {
        updatePrefs { s ->
            val next = if (areaId in s.selectedAreaIds) s.selectedAreaIds - areaId else s.selectedAreaIds + areaId
            s.copy(selectedAreaIds = next)
        }
    }

    fun setCarpool(enabled: Boolean) {
        updatePrefs { it.copy(carpoolEnabled = enabled) }
    }

    fun stepSpareSeats(delta: Int) {
        updatePrefs { s ->
            val max = s.maxSpareSeats
            s.copy(spareSeats = (s.spareSeats + delta).coerceIn(1, max))
        }
    }

    fun setGateWaitMins(mins: Int) {
        updatePrefs { it.copy(gateWaitMins = mins) }
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    fun saveProfile() {
        val s = _state.value
        if (s.busy) return
        _state.value = s.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                val clampedSpare = s.spareSeats.coerceIn(1, s.maxSpareSeats)
                container.rides.updateDriverProfile(
                    DriverProfileReq(
                        carpool_enabled = s.carpoolEnabled,
                        spare_seats = clampedSpare,
                        gate_wait_mins = s.gateWaitMins,
                        area_ids = s.selectedAreaIds.sorted(),
                        active_vehicle_id = s.activeVehicleId,
                    ),
                )
                val prefs = SavedPrefs(
                    carpoolEnabled = s.carpoolEnabled,
                    spareSeats = clampedSpare,
                    gateWaitMins = s.gateWaitMins,
                    areaIds = s.selectedAreaIds,
                )
                savedPrefs = prefs
                _state.value = _state.value.copy(
                    busy = false,
                    spareSeats = clampedSpare,
                    dirty = false,
                    message = "saved",
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun saveVehicle(editingId: Long?, req: DriverVehicleReq, onDone: () -> Unit = {}) {
        if (_state.value.busy) return
        val cleanModel = req.model.trim()
        val cleanColor = req.color.trim()
        val cleanPlate = req.plate.trim().uppercase()
        if (cleanModel.isEmpty() || cleanColor.isEmpty() || cleanPlate.isEmpty()) return

        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                val cleanReq = req.copy(
                    model = cleanModel,
                    color = cleanColor,
                    plate = cleanPlate,
                    seats = req.seats.coerceIn(2, 9),
                )
                if (editingId == null) {
                    container.rides.createDriverVehicle(cleanReq)
                } else {
                    container.rides.updateDriverVehicle(editingId, cleanReq)
                }
                val profile = container.rides.driverProfile()
                applyProfile(profile, _state.value.areas, resetPrefs = false, message = "vehicle_saved")
                onDone()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun selectVehicle(vehicleId: Long) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                container.rides.selectDriverVehicle(vehicleId)
                val profile = container.rides.driverProfile()
                applyProfile(profile, _state.value.areas, resetPrefs = false, message = "vehicle_selected")
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    fun deleteVehicle(vehicleId: Long, onDone: () -> Unit = {}) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                container.rides.deleteDriverVehicle(vehicleId)
                val profile = container.rides.driverProfile()
                applyProfile(profile, _state.value.areas, resetPrefs = false, message = "vehicle_deleted")
                onDone()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { DriverSettingsViewModel(container) }
        }
    }
}
