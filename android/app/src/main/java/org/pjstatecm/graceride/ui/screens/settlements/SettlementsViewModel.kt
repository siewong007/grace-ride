package org.pjstatecm.graceride.ui.screens.settlements

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.SettlementSummaryDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.RealtimeEvent

data class SettlementsUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val items: List<SettlementSummaryDto> = emptyList(),
    val statusFilter: String = "all",
    val query: String = "",
    val page: Int = 1,
    val totalPages: Int = 1,
) {
    val visibleItems: List<SettlementSummaryDto>
        get() = items.filter { item ->
            val matchStatus = statusFilter == "all" || item.status == statusFilter
            val q = query.trim().lowercase()
            val matchQ = q.isEmpty() ||
                item.title.lowercase().contains(q) ||
                item.eventTitle.lowercase().contains(q)
            matchStatus && matchQ
        }
}

class SettlementsViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(SettlementsUiState())
    val state: StateFlow<SettlementsUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true, page = 1)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) refresh()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
    }

    fun setStatusFilter(status: String) {
        _state.value = _state.value.copy(statusFilter = status)
    }

    fun setQuery(q: String) {
        _state.value = _state.value.copy(query = q)
    }

    fun loadPage(page: Int) = load(first = false, page = page.coerceAtLeast(1))

    fun refresh() = load(first = false, page = _state.value.page)

    private fun load(first: Boolean, page: Int) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.items.isEmpty(),
                refreshing = !first,
                error = null,
            )
            try {
                val res = container.finance.listSettlements(page = page, limit = 25)
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    offline = false,
                    items = res.items,
                    page = res.page,
                    totalPages = res.totalPages.coerceAtLeast(1),
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    offline = e.status == 0,
                    error = if (e.status == 0) null else e.message,
                )
            }
        }
    }

    companion object {
        fun factory(container: AppContainer) = viewModelFactory {
            initializer { SettlementsViewModel(container) }
        }
    }
}
