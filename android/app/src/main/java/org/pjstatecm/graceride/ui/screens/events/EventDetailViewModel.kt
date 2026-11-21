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
import org.pjstatecm.graceride.di.AppContainer

data class EventDetailUiState(
    val loading: Boolean = true,
    val event: EventItem? = null,
    val error: String? = null,
    val busy: Boolean = false,
    val message: String? = null,
)

class EventDetailViewModel(private val container: AppContainer, private val id: Long) : ViewModel() {
    private val _state = MutableStateFlow(EventDetailUiState())
    val state: StateFlow<EventDetailUiState> = _state.asStateFlow()

    init { load() }

    fun load() {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = _state.value.event == null)
            try {
                _state.value = _state.value.copy(loading = false, event = container.events.detail(id), error = null)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun signup(needRide: Boolean, offerDrive: Boolean, asGroupId: Long?) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.signup(id, needRide, offerDrive, asGroupId)
                load()
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
                load()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun clearMessage() { _state.value = _state.value.copy(message = null) }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { EventDetailViewModel(container, id) }
        }
    }
}
