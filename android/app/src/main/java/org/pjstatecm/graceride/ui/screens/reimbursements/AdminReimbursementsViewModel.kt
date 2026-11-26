package org.pjstatecm.graceride.ui.screens.reimbursements

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
import org.pjstatecm.graceride.data.api.CreatePayoutBatchReq
import org.pjstatecm.graceride.data.api.PayoutBatchDto
import org.pjstatecm.graceride.data.api.ReimbursementDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime

data class AdminReimbursementsUiState(
    val tab: String = "review",
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val busy: Boolean = false,
    val claims: List<ReimbursementDto> = emptyList(),
    val batches: List<PayoutBatchDto> = emptyList(),
    val statusFilter: String = "pending",
    val query: String = "",
    val page: Int = 1,
    val totalPages: Int = 1,
    val error: String? = null,
    val message: String? = null,
    val exportedBatchCsv: Pair<String, ByteArray>? = null,
)

class AdminReimbursementsViewModel(
    private val container: AppContainer,
    initialTab: String = "review",
) : ViewModel() {
    private val _state = MutableStateFlow(
        AdminReimbursementsUiState(
            tab = initialTab,
            statusFilter = if (initialTab == "payout") "approved" else "pending",
        ),
    )
    val state: StateFlow<AdminReimbursementsUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true)
    }

    fun selectTab(tab: String) {
        if (_state.value.tab == tab) return
        _state.value = _state.value.copy(
            tab = tab,
            statusFilter = when (tab) {
                "payout" -> "approved"
                "review" -> "pending"
                else -> "all"
            },
            page = 1,
        )
        load(first = false)
    }

    fun setStatusFilter(status: String) {
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
                loading = first && _state.value.claims.isEmpty() && _state.value.batches.isEmpty(),
                refreshing = !first,
                error = null,
            )
            try {
                if (s.tab == "batches") {
                    val batches = container.finance.listPayoutBatches()
                    _state.value = _state.value.copy(loading = false, refreshing = false, batches = batches)
                } else {
                    val scope = if (s.tab == "payout") "payout" else "review"
                    val res = container.finance.listReimbursements(
                        scope = scope,
                        q = s.query,
                        status = s.statusFilter,
                        page = s.page,
                        limit = 25,
                    )
                    _state.value = _state.value.copy(
                        loading = false,
                        refreshing = false,
                        claims = res.items,
                        page = res.page,
                        totalPages = res.totalPages.coerceAtLeast(1),
                    )
                }
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
            }
        }
    }

    fun review(id: Long, action: String, note: String?) = act {
        container.finance.reviewReimbursement(id, action, note)
    }

    fun pay(id: Long, payeeVerified: Boolean, paidRef: String) = act {
        container.finance.payReimbursement(id, payeeVerified, paidRef)
    }

    fun createBatch(month: String = KlTime.today().take(7)) = act {
        container.finance.createPayoutBatch(CreatePayoutBatchReq(month = month))
    }

    fun cancelBatch(batchId: Long) = act {
        container.finance.cancelPayoutBatch(batchId)
    }

    fun payBatch(batchId: Long, payeeVerified: Boolean, paidRef: String) = act {
        container.finance.payPayoutBatch(batchId, payeeVerified, paidRef)
    }

    fun downloadBatchCsv(batch: PayoutBatchDto) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                val bytes = container.finance.downloadBatchCsv(batch.id)
                _state.value = _state.value.copy(
                    busy = false,
                    exportedBatchCsv = "payout-batch-${batch.month}-${batch.id}.csv" to bytes,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, message = e.message)
            }
        }
    }

    fun clearExportedBatchCsv() {
        _state.value = _state.value.copy(exportedBatchCsv = null)
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
                _state.value = _state.value.copy(busy = false)
                load(first = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(busy = false, message = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, initialTab: String) = viewModelFactory {
            initializer { AdminReimbursementsViewModel(container, initialTab) }
        }
    }
}
