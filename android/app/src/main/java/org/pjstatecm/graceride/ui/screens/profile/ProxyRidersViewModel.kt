package org.pjstatecm.graceride.ui.screens.profile

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
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.Place
import org.pjstatecm.graceride.data.api.ProxyRider
import org.pjstatecm.graceride.data.api.ProxyRiderReq
import org.pjstatecm.graceride.di.AppContainer

data class ProxyRidersUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val riders: List<ProxyRider> = emptyList(),
    val areas: List<Area> = emptyList(),
    val suggestions: List<Place> = emptyList(),
    val busy: Boolean = false,
    val message: String? = null,
    val error: String? = null,
)

class ProxyRidersViewModel(private val container: AppContainer) : ViewModel() {

    private val _state = MutableStateFlow(ProxyRidersUiState())
    val state: StateFlow<ProxyRidersUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    private var searchJob: Job? = null

    init {
        load()
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != org.pjstatecm.graceride.notifications.RealtimeEvent.Resume) refresh()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
    }

    fun load() = loadInternal(refreshing = false)

    fun refresh() = loadInternal(refreshing = _state.value.riders.isNotEmpty())

    private fun loadInternal(refreshing: Boolean) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            val uid = container.auth.me?.id
            if (_state.value.riders.isEmpty() && uid != null) {
                kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                    container.offlineCache.read(
                        uid,
                        "proxy_riders",
                        kotlinx.serialization.builtins.ListSerializer(org.pjstatecm.graceride.data.api.ProxyRider.serializer()),
                    )
                }?.let { cached ->
                    _state.value = _state.value.copy(riders = cached.data, loading = false)
                }
            }
            _state.value = _state.value.copy(
                loading = _state.value.riders.isEmpty() && !refreshing,
                refreshing = refreshing,
                error = null,
            )
            try {
                val (riders, areas) = coroutineScope {
                    val ridersDef = async { container.rides.proxyRiders() }
                    val areasDef = async {
                        try {
                            container.rides.areas().filter { it.active }
                        } catch (_: ApiException) {
                            _state.value.areas
                        }
                    }
                    ridersDef.await() to areasDef.await()
                }
                if (uid != null) {
                    kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                        container.offlineCache.write(
                            uid,
                            "proxy_riders",
                            riders,
                            kotlinx.serialization.builtins.ListSerializer(org.pjstatecm.graceride.data.api.ProxyRider.serializer()),
                        )
                    }
                }
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    riders = riders,
                    areas = areas,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
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
        req: ProxyRiderReq,
        onDone: () -> Unit = {},
    ) {
        if (_state.value.busy) return
        val cleanName = req.name.trim()
        if (cleanName.isEmpty()) return
        _state.value = _state.value.copy(busy = true, error = null)
        viewModelScope.launch {
            try {
                val payload = req.copy(
                    name = cleanName,
                    phone = req.phone?.trim(),
                    relationship = req.relationship?.trim(),
                    pickup_address = req.pickup_address?.trim(),
                    gate_notes = req.gate_notes?.trim(),
                    notes = req.notes?.trim(),
                )
                if (editingId == null) {
                    container.rides.createProxyRider(payload)
                } else {
                    container.rides.updateProxyRider(editingId, payload)
                }
                val riders = container.rides.proxyRiders()
                container.auth.me?.id?.let { uid ->
                    kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                        container.offlineCache.write(
                            uid,
                            "proxy_riders",
                            riders,
                            kotlinx.serialization.builtins.ListSerializer(ProxyRider.serializer()),
                        )
                    }
                }
                _state.value = _state.value.copy(
                    busy = false,
                    riders = riders,
                    suggestions = emptyList(),
                    message = "saved",
                )
                onDone()
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
                container.rides.deleteProxyRider(id)
                val riders = container.rides.proxyRiders()
                container.auth.me?.id?.let { uid ->
                    kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
                        container.offlineCache.write(
                            uid,
                            "proxy_riders",
                            riders,
                            kotlinx.serialization.builtins.ListSerializer(ProxyRider.serializer()),
                        )
                    }
                }
                _state.value = _state.value.copy(
                    busy = false,
                    riders = riders,
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
            initializer { ProxyRidersViewModel(container) }
        }
    }
}
