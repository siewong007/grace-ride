package org.pjstatecm.graceride.ui.screens.reimbursements

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.ReimbursementDto
import org.pjstatecm.graceride.di.AppContainer

data class ReimbursementDetailUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val claim: ReimbursementDto? = null,
    val error: String? = null,
)

class ReimbursementDetailViewModel(
    private val container: AppContainer,
    private val id: Long,
) : ViewModel() {
    private val _state = MutableStateFlow(ReimbursementDetailUiState())
    val state: StateFlow<ReimbursementDetailUiState> = _state.asStateFlow()

    init {
        load(first = true)
    }

    fun refresh() = load(first = false)

    fun load(first: Boolean = true) {
        viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.claim == null,
                refreshing = !first,
                error = null,
            )
            try {
                val c = container.finance.reimbursementDetail(id)
                _state.value = _state.value.copy(loading = false, refreshing = false, claim = c)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { ReimbursementDetailViewModel(container, id) }
        }
    }
}
