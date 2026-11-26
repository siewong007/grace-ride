package org.pjstatecm.graceride.ui.screens.settlements

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.SettlementDetailDto
import org.pjstatecm.graceride.data.api.SettlementRevisionDto
import org.pjstatecm.graceride.di.AppContainer

data class SettlementDetailUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val detail: SettlementDetailDto? = null,
    val revisions: List<SettlementRevisionDto> = emptyList(),
    val revisionsLoaded: Boolean = false,
    val busy: Boolean = false,
    val error: String? = null,
    val message: String? = null,
    val deleted: Boolean = false,
)

class SettlementDetailViewModel(
    private val container: AppContainer,
    private val id: Long,
) : ViewModel() {
    private val _state = MutableStateFlow(SettlementDetailUiState())
    val state: StateFlow<SettlementDetailUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
    }

    fun refresh() = load(first = false)

    fun load(first: Boolean = true) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.detail == null,
                refreshing = !first,
                error = null,
            )
            try {
                val detail = container.finance.settlementDetail(id)
                _state.value = _state.value.copy(loading = false, refreshing = false, detail = detail)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
            }
        }
    }

    fun loadRevisions() {
        viewModelScope.launch {
            try {
                val list = container.finance.settlementRevisions(id)
                _state.value = _state.value.copy(revisions = list, revisionsLoaded = true)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            }
        }
    }

    fun publish() = act { container.finance.publishSettlement(id) }

    fun closeBill() = act { container.finance.closeSettlement(id) }

    fun deleteBill() {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.finance.deleteSettlement(id)
                _state.value = _state.value.copy(busy = false, deleted = true)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, message = e.message)
            }
        }
    }

    fun verifyShare(shareId: Long, action: String, note: String?) = act {
        container.finance.verifyShare(shareId, action, note)
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    private fun act(block: suspend () -> Unit) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                block()
                val detail = container.finance.settlementDetail(id)
                _state.value = _state.value.copy(busy = false, detail = detail, revisionsLoaded = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, message = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { SettlementDetailViewModel(container, id) }
        }
    }
}
