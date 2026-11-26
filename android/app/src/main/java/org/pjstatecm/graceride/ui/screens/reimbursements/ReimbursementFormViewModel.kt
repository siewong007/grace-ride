package org.pjstatecm.graceride.ui.screens.reimbursements

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.CreateReimbursementReq
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Money

data class ReimbursementFormUiState(
    val loading: Boolean = true,
    val submitting: Boolean = false,
    val uploadingReceipt: Boolean = false,
    val extractingOcr: Boolean = false,
    val ocrCandidates: List<String> = emptyList(),
    val editId: Long? = null,
    val kind: String = "grab",
    val events: List<EventItem> = emptyList(),
    val rides: List<Ride> = emptyList(),
    val selectedEventId: Long? = null,
    val selectedRideId: Long? = null,
    val amountText: String = "",
    val description: String = "",
    val receiptFileId: Long? = null,
    val payeeName: String = "",
    val payeePhone: String = "",
    val error: String? = null,
    val savedId: Long? = null,
)

class ReimbursementFormViewModel(
    private val container: AppContainer,
    private val editId: Long?,
) : ViewModel() {
    private val _state = MutableStateFlow(ReimbursementFormUiState(editId = editId))
    val state: StateFlow<ReimbursementFormUiState> = _state.asStateFlow()

    init {
        load()
    }

    fun load() {
        _state.value = _state.value.copy(loading = true, error = null)
        viewModelScope.launch {
            try {
                val (events, rides, existing) = coroutineScope {
                    val eDef = async { runCatching { container.events.upcoming() }.getOrDefault(emptyList()) }
                    val rDef = async {
                        runCatching {
                            (container.rides.upcoming() + container.rides.pastPage(limit = 20).items)
                                .distinctBy { it.id }
                        }.getOrDefault(emptyList())
                    }
                    val xDef = async { editId?.let { container.finance.reimbursementDetail(it) } }
                    Triple(eDef.await(), rDef.await(), xDef.await())
                }
                val me = container.auth.me
                _state.value = ReimbursementFormUiState(
                    loading = false,
                    editId = editId,
                    kind = existing?.kind?.takeIf { it in setOf("grab", "mileage") } ?: "grab",
                    events = events,
                    rides = rides,
                    selectedEventId = existing?.eventId ?: events.firstOrNull()?.id,
                    selectedRideId = existing?.rideId,
                    amountText = existing?.let { "%.2f".format(it.amountCents / 100.0) } ?: "",
                    description = existing?.description.orEmpty(),
                    receiptFileId = existing?.receiptFileId,
                    payeeName = existing?.payeeName ?: me?.name.orEmpty(),
                    payeePhone = existing?.payeePhone ?: me?.phone.orEmpty(),
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun update(block: (ReimbursementFormUiState) -> ReimbursementFormUiState) {
        _state.value = block(_state.value).copy(error = null)
    }

    fun uploadReceipt(context: Context, uri: Uri) {
        _state.value = _state.value.copy(uploadingReceipt = true, extractingOcr = false, ocrCandidates = emptyList(), error = null)
        viewModelScope.launch {
            try {
                val prepared = withContext(Dispatchers.IO) {
                    prepareImageUpload(context, uri, prefix = "receipt", maxDimensionPx = 1600, quality = 85)
                }
                val fileRef = container.finance.uploadFile(prepared.bytes, prepared.filename, prepared.mime)
                _state.value = _state.value.copy(
                    uploadingReceipt = false,
                    extractingOcr = true,
                    receiptFileId = fileRef.id,
                )
                val candidates = fetchOcrCandidates(fileRef.id)
                _state.value = _state.value.copy(extractingOcr = false, ocrCandidates = candidates)
            } catch (e: Exception) {
                _state.value = _state.value.copy(uploadingReceipt = false, extractingOcr = false, error = e.message)
            }
        }
    }

    internal suspend fun fetchOcrCandidates(fileId: Long): List<String> = withContext(Dispatchers.IO) {
        runCatching { container.shares.extractFileRef(fileId) }
            .map { resp ->
                (listOf(resp.ref) + resp.candidates)
                    .map { it.trim() }
                    .filter { it.isNotBlank() }
                    .distinct()
            }
            .getOrDefault(emptyList())
    }

    fun submit(invalidMsg: String) {
        val s = _state.value
        if (s.submitting) return
        val cents = Money.parseCents(s.amountText)
        if (cents == null || cents <= 0L || cents > 50_000L || s.payeeName.isBlank() || s.payeePhone.isBlank() ||
            (s.selectedEventId == null && s.selectedRideId == null)
        ) {
            _state.value = s.copy(error = invalidMsg)
            return
        }
        val req = CreateReimbursementReq(
            ride_id = s.selectedRideId,
            event_id = s.selectedEventId,
            amount_cents = cents,
            description = s.description.trim(),
            receipt_file_id = s.receiptFileId,
            payee_name = s.payeeName.trim(),
            payee_phone = s.payeePhone.trim(),
            kind = s.kind,
        )
        _state.value = s.copy(submitting = true, error = null)
        viewModelScope.launch {
            try {
                val targetId = if (s.editId != null) {
                    container.finance.updateReimbursement(s.editId, req)
                    s.editId
                } else {
                    container.finance.createReimbursement(req).id
                }
                _state.value = _state.value.copy(submitting = false, savedId = targetId)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(submitting = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, editId: Long?) = viewModelFactory {
            initializer { ReimbursementFormViewModel(container, editId) }
        }
    }
}
