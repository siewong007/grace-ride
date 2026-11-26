package org.pjstatecm.graceride.ui.screens.reimbursements

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
import org.pjstatecm.graceride.data.api.ReimbursementDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.notifications.RealtimeEvent

data class ReimbursementsUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val offline: Boolean = false,
    val error: String? = null,
    val items: List<ReimbursementDto> = emptyList(),
    val summaryLoaded: Boolean = false,
    val summaryPendingCents: Long = 0L,
    val summaryApprovedCents: Long = 0L,
    val summaryPaidCents: Long = 0L,
    val statusFilter: String = "all",
    val query: String = "",
    val page: Int = 1,
    val totalPages: Int = 1,
)

class ReimbursementsViewModel(private val container: AppContainer) : ViewModel() {
    private val _state = MutableStateFlow(ReimbursementsUiState())
    val state: StateFlow<ReimbursementsUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
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
        if (_state.value.statusFilter == status) return
        _state.value = _state.value.copy(statusFilter = status, page = 1)
        load(first = false)
    }

    fun setQuery(q: String) {
        _state.value = _state.value.copy(query = q, page = 1)
        load(first = false)
    }

    fun loadPage(page: Int) {
        _state.value = _state.value.copy(page = page.coerceAtLeast(1))
        load(first = false)
    }

    fun refresh() = load(first = false)

    private fun load(first: Boolean) {
        loadJob?.cancel()
        val s = _state.value
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && _state.value.items.isEmpty(),
                refreshing = !first,
                error = null,
            )
            try {
                val res = container.finance.listReimbursements(
                    scope = "mine",
                    q = s.query,
                    status = s.statusFilter,
                    page = s.page,
                    limit = 25,
                )
                val totalPages = res.totalPages.coerceAtLeast(1)
                val cur = _state.value
                val isUnfilteredFirstPage = s.statusFilter == "all" && s.query.isBlank() && res.page == 1
                var nextSummaryLoaded = cur.summaryLoaded
                var nextPending = cur.summaryPendingCents
                var nextApproved = cur.summaryApprovedCents
                var nextPaid = cur.summaryPaidCents
                if (isUnfilteredFirstPage) {
                    if (totalPages == 1 && res.items.isNotEmpty()) {
                        var pending = 0L
                        var approved = 0L
                        var paid = 0L
                        res.items.forEach { item ->
                            when (item.status) {
                                "pending", "returned" -> pending += item.amountCents
                                "approved" -> approved += item.amountCents
                                "paid" -> paid += item.amountCents
                            }
                        }
                        nextSummaryLoaded = true
                        nextPending = pending
                        nextApproved = approved
                        nextPaid = paid
                    } else {
                        nextSummaryLoaded = false
                        nextPending = 0L
                        nextApproved = 0L
                        nextPaid = 0L
                    }
                }
                _state.value = cur.copy(
                    loading = false,
                    refreshing = false,
                    offline = false,
                    items = res.items,
                    summaryLoaded = nextSummaryLoaded,
                    summaryPendingCents = nextPending,
                    summaryApprovedCents = nextApproved,
                    summaryPaidCents = nextPaid,
                    page = res.page,
                    totalPages = totalPages,
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
            initializer { ReimbursementsViewModel(container) }
        }
    }
}
