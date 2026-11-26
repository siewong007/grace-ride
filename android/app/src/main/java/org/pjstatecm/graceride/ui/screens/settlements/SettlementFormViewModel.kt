package org.pjstatecm.graceride.ui.screens.settlements

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
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.SettlementInReq
import org.pjstatecm.graceride.data.api.SettlementItemInReq
import org.pjstatecm.graceride.data.api.SettlementParticipantInReq
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.Money

data class SettlementLineDraft(
    val key: Long,
    val label: String = "",
    val amountText: String = "",
    val receiptFileId: Long? = null,
)

data class SettlementParticipantOpt(
    val userId: Long,
    val name: String,
    val attended: Boolean = false,
    val selected: Boolean = false,
    val customAmountText: String = "",
)

data class SettlementFormUiState(
    val loading: Boolean = true,
    val submitting: Boolean = false,
    val editingId: Long? = null,
    val existingStatus: String = "draft",
    val events: List<EventItem> = emptyList(),
    val selectedEventId: Long? = null,
    val title: String = "",
    val splitMode: String = "equal",
    val subsidyText: String = "",
    val payeeName: String = "",
    val payeeAccount: String = "",
    val payeeQrFileId: Long? = null,
    val dueDate: String = "",
    val revisionReason: String = "",
    val items: List<SettlementLineDraft> = listOf(SettlementLineDraft(key = 1L)),
    val participants: List<SettlementParticipantOpt> = emptyList(),
    val error: String? = null,
    val savedId: Long? = null,
) {
    val grossCents: Long get() = items.sumOf { Money.parseCents(it.amountText) ?: 0L }
    val subsidyCents: Long get() = Money.parseCents(subsidyText) ?: 0L
    val netTotalCents: Long get() = (grossCents - subsidyCents).coerceAtLeast(0L)
    val selectedParticipants: List<SettlementParticipantOpt> get() = participants.filter { it.selected }
    val equalShareAmounts: List<Long> get() = Money.equalSplit(netTotalCents, selectedParticipants.size)
    val customTotalCents: Long get() = selectedParticipants.sumOf { Money.parseCents(it.customAmountText) ?: 0L }
    val customAmountsValid: Boolean
        get() = splitMode != "custom" || selectedParticipants.all { Money.parseCents(it.customAmountText) != null }
    val isCustomBalanced: Boolean
        get() = splitMode != "custom" || (customAmountsValid && netTotalCents > 0L && customTotalCents == netTotalCents)
}

class SettlementFormViewModel(
    private val container: AppContainer,
    private val initialSettlementId: Long?,
    private val initialEventId: Long?,
) : ViewModel() {
    private val _state = MutableStateFlow(SettlementFormUiState(editingId = initialSettlementId, selectedEventId = initialEventId))
    val state: StateFlow<SettlementFormUiState> = _state.asStateFlow()
    private var nextLineKey = 2L
    private var createdDraftId: Long? = initialSettlementId

    init {
        load()
    }

    fun load() {
        _state.value = _state.value.copy(loading = true, error = null)
        viewModelScope.launch {
            try {
                if (initialSettlementId != null) {
                    val (detail, signups) = coroutineScope {
                        val dDef = async { container.finance.settlementDetail(initialSettlementId) }
                        val detailRes = dDef.await()
                        val sDef = async {
                            runCatching { container.events.signups(detailRes.event.id) }.getOrDefault(emptyList())
                        }
                        detailRes to sDef.await()
                    }
                    val existingMap = detail.shares.associateBy { it.userId }
                    val signupList = signups.filter { it.status == "going" }
                    val participants = if (signupList.isNotEmpty()) {
                        signupList.map { su ->
                            val ex = existingMap[su.userId]
                            SettlementParticipantOpt(
                                userId = su.userId,
                                name = su.name,
                                attended = (su.attended ?: 0L) == 1L,
                                selected = ex != null,
                                customAmountText = ex?.let { "%.2f".format(it.amountCents / 100.0) } ?: "",
                            )
                        }
                    } else {
                        detail.shares.map { sh ->
                            SettlementParticipantOpt(
                                userId = sh.userId,
                                name = sh.name,
                                selected = true,
                                customAmountText = "%.2f".format(sh.amountCents / 100.0),
                            )
                        }
                    }
                    val lines = detail.items.mapIndexed { idx, it ->
                        SettlementLineDraft(
                            key = (idx + 1).toLong(),
                            label = it.label,
                            amountText = "%.2f".format(it.amountCents / 100.0),
                            receiptFileId = it.receiptFileId,
                        )
                    }.ifEmpty { listOf(SettlementLineDraft(key = 1L)) }
                    nextLineKey = (lines.maxOfOrNull { it.key } ?: 1L) + 1L
                    createdDraftId = detail.id
                    _state.value = _state.value.copy(
                        loading = false,
                        editingId = detail.id,
                        existingStatus = detail.status,
                        selectedEventId = detail.event.id,
                        title = detail.title,
                        splitMode = detail.splitMode,
                        subsidyText = if (detail.subsidyCents > 0) "%.2f".format(detail.subsidyCents / 100.0) else "",
                        payeeName = detail.payeeName,
                        payeeAccount = detail.payeeAccount,
                        payeeQrFileId = detail.payeeQrFileId,
                        dueDate = detail.dueDate.orEmpty(),
                        items = lines,
                        participants = participants,
                    )
                } else {
                    val events = container.events.upcoming().sortedBy { it.startsAt }
                    val evId = initialEventId ?: events.firstOrNull()?.id
                    val signups = if (evId != null) {
                        runCatching { container.events.signups(evId) }.getOrDefault(emptyList())
                    } else {
                        emptyList()
                    }
                    val me = container.auth.me
                    val ev = events.firstOrNull { it.id == evId }
                    _state.value = _state.value.copy(
                        loading = false,
                        events = events,
                        selectedEventId = evId,
                        title = ev?.title.orEmpty(),
                        payeeName = me?.name.orEmpty(),
                        payeeAccount = me?.phone.orEmpty(),
                        participants = signups.filter { it.status == "going" }.map { su ->
                            SettlementParticipantOpt(
                                userId = su.userId,
                                name = su.name,
                                attended = (su.attended ?: 0L) == 1L,
                                selected = (su.attended ?: 0L) == 1L,
                            )
                        },
                    )
                }
            } catch (e: ApiException) {
                _state.value = _state.value.copy(loading = false, error = e.message)
            }
        }
    }

    fun selectEvent(eventId: Long) {
        val ev = _state.value.events.firstOrNull { it.id == eventId }
        _state.value = _state.value.copy(
            selectedEventId = eventId,
            title = if (_state.value.title.isBlank()) ev?.title.orEmpty() else _state.value.title,
        )
        viewModelScope.launch {
            val signups = runCatching { container.events.signups(eventId) }.getOrDefault(emptyList())
            _state.value = _state.value.copy(
                participants = signups.filter { it.status == "going" }.map { su ->
                    SettlementParticipantOpt(
                        userId = su.userId,
                        name = su.name,
                        attended = (su.attended ?: 0L) == 1L,
                        selected = (su.attended ?: 0L) == 1L,
                    )
                },
            )
        }
    }

    fun update(block: (SettlementFormUiState) -> SettlementFormUiState) {
        _state.value = block(_state.value).copy(error = null)
    }

    fun addItem() {
        val key = nextLineKey++
        _state.value = _state.value.copy(items = _state.value.items + SettlementLineDraft(key = key))
    }

    fun removeItem(key: Long) {
        if (_state.value.items.size <= 1) return
        _state.value = _state.value.copy(items = _state.value.items.filterNot { it.key == key })
    }

    fun updateItem(key: Long, label: String? = null, amount: String? = null, receiptFileId: Long? = null) {
        _state.value = _state.value.copy(
            error = null,
            items = _state.value.items.map { item ->
                if (item.key != key) item
                else item.copy(
                    label = label ?: item.label,
                    amountText = amount ?: item.amountText,
                    receiptFileId = receiptFileId ?: item.receiptFileId,
                )
            },
        )
    }

    fun toggleParticipant(userId: Long) {
        _state.value = _state.value.copy(
            error = null,
            participants = _state.value.participants.map { p ->
                if (p.userId == userId) p.copy(selected = !p.selected) else p
            },
        )
    }

    fun selectParticipants(userIds: Set<Long>, selected: Boolean) {
        _state.value = _state.value.copy(
            error = null,
            participants = _state.value.participants.map { p ->
                if (p.userId in userIds) p.copy(selected = selected) else p
            },
        )
    }

    fun setParticipantAmount(userId: Long, amount: String) {
        _state.value = _state.value.copy(
            error = null,
            participants = _state.value.participants.map { p ->
                if (p.userId == userId) p.copy(customAmountText = amount) else p
            },
        )
    }

    fun uploadQr(context: Context, uri: Uri) {
        viewModelScope.launch {
            try {
                val prepared = withContext(Dispatchers.IO) {
                    prepareImageUpload(context, uri, prefix = "payee-qr", maxDimensionPx = 1200, quality = 85)
                }
                val fileRef = container.finance.uploadFile(prepared.bytes, prepared.filename, prepared.mime)
                _state.value = _state.value.copy(payeeQrFileId = fileRef.id)
            } catch (e: Exception) {
                _state.value = _state.value.copy(error = e.message)
            }
        }
    }

    fun submit(publishAfterSave: Boolean, fallbackErr: String, unbalancedErr: String = fallbackErr) {
        val s = _state.value
        if (s.submitting) return
        val eventId = s.selectedEventId ?: return
        val items = s.items.mapNotNull { line ->
            val cents = Money.parseCents(line.amountText)
            if (line.label.isBlank() || cents == null || cents <= 0L) null
            else SettlementItemInReq(line.label.trim(), cents, line.receiptFileId)
        }
        val selected = s.selectedParticipants
        if (s.title.isBlank() || items.isEmpty() || selected.isEmpty()) {
            _state.value = s.copy(error = fallbackErr)
            return
        }
        if ((publishAfterSave || s.existingStatus == "published") && !s.isCustomBalanced) {
            _state.value = s.copy(error = unbalancedErr)
            return
        }
        val participantsReq = if (s.splitMode == "custom") {
            selected.map { p ->
                SettlementParticipantInReq(p.userId, Money.parseCents(p.customAmountText) ?: 0L)
            }
        } else {
            selected.map { SettlementParticipantInReq(it.userId) }
        }
        val req = SettlementInReq(
            title = s.title.trim(),
            items = items,
            subsidy_cents = Money.parseCents(s.subsidyText)?.takeIf { it > 0L },
            split_mode = s.splitMode,
            participants = participantsReq,
            payee_name = s.payeeName.trim(),
            payee_account = s.payeeAccount.trim(),
            payee_qr_file_id = s.payeeQrFileId,
            due_date = s.dueDate.trim().ifBlank { null },
            reason = if (s.existingStatus == "published") s.revisionReason.trim().ifBlank { null } else null,
        )
        _state.value = s.copy(submitting = true, error = null)
        viewModelScope.launch {
            try {
                val targetId = createdDraftId?.let { id ->
                    container.finance.updateSettlement(id, req)
                    id
                } ?: container.finance.createSettlement(eventId, req).id.also { newId ->
                    createdDraftId = newId
                }
                if (publishAfterSave && s.existingStatus != "published") {
                    container.finance.publishSettlement(targetId)
                }
                _state.value = _state.value.copy(submitting = false, savedId = targetId)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(submitting = false, error = e.message)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, settlementId: Long?, eventId: Long?) = viewModelFactory {
            initializer { SettlementFormViewModel(container, settlementId, eventId) }
        }
    }
}
