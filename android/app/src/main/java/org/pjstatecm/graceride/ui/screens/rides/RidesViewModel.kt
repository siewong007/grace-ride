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

data class RidesUiState(
    val tab: Int = 0,
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val rides: List<Ride> = emptyList(),
)

class RidesViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(RidesUiState())
    val state: StateFlow<RidesUiState> = _state.asStateFlow()

    init { load(first = true) }

    fun setTab(tab: Int) {
        if (tab == _state.value.tab) return
        _state.value = _state.value.copy(tab = tab)
        load(first = true)
    }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        viewModelScope.launch {
            _state.value = _state.value.copy(loading = first, refreshing = !first)
            try {
                val rides = if (_state.value.tab == 0) container.rides.upcoming() else container.rides.past()
                _state.value = _state.value.copy(loading = false, refreshing = false, rides = rides, error = null, offline = false)
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
        fun factory(container: AppContainer) = viewModelFactory { initializer { RidesViewModel(container) } }
    }
}
