package org.pjstatecm.graceride.ui.screens.rides

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.di.AppContainer

data class RideDetailUiState(
    val loading: Boolean = true,
    val ride: Ride? = null,
    val error: String? = null,
    val busy: Boolean = false,
    val message: String? = null,
    val staleReload: Boolean = false,
)

class RideDetailViewModel(private val container: AppContainer, private val id: Long) : ViewModel() {
    private val _state = MutableStateFlow(RideDetailUiState())
    val state: StateFlow<RideDetailUiState> = _state.asStateFlow()

    init { load() }

    fun load() {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = _state.value.ride == null)
            try {
                val ride = container.rides.detail(id)
                _state.value = _state.value.copy(loading = false, ride = ride, error = null)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    private fun action(block: suspend () -> Unit) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                block()
                load()
            } catch (e: ApiException) {
                if (e.code == "RIDE_STATUS_CHANGED") {
                    // Optimistic concurrency: someone else moved the ride. Re-fetch.
                    _state.value = _state.value.copy(staleReload = true)
                    load()
                } else {
                    _state.value = _state.value.copy(message = e.message)
                }
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun cancel(reason: String?) = action { container.rides.cancel(id, reason) }
    fun respond(accept: Boolean) = action { container.rides.respond(id, accept) }
    fun readiness(status: String) = action { container.rides.readiness(id, status) }
    fun clearMessage() { _state.value = _state.value.copy(message = null, staleReload = false) }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { RideDetailViewModel(container, id) }
        }
    }
}
