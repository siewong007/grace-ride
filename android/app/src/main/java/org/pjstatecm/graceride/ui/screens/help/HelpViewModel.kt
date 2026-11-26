package org.pjstatecm.graceride.ui.screens.help

import android.content.Context
import android.net.Uri
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.CreateTicketReq
import org.pjstatecm.graceride.data.api.GuideSummaryDto
import org.pjstatecm.graceride.data.api.HelpCountsDto
import org.pjstatecm.graceride.data.api.HelpMemberDto
import org.pjstatecm.graceride.data.api.TicketSummaryDto
import org.pjstatecm.graceride.data.media.prepareImageUpload
import org.pjstatecm.graceride.di.AppContainer

data class HelpUiState(
    val tab: String = "guides",
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val guides: List<GuideSummaryDto> = emptyList(),
    val tickets: List<TicketSummaryDto> = emptyList(),
    val counts: HelpCountsDto = HelpCountsDto(),
    val guideAudience: String = "all",
    val ticketScopeAll: Boolean = false,
    val ticketStatus: String = "all",
    val assignedToMe: Boolean = false,
    val query: String = "",
    val memberResults: List<HelpMemberDto> = emptyList(),
    val uploadingAttachment: Boolean = false,
    val uploadedFileId: Long? = null,
    val submitting: Boolean = false,
    val createdTicketId: Long? = null,
    val error: String? = null,
    val message: String? = null,
)

class HelpViewModel(
    private val container: AppContainer,
    initialRideId: Long? = null,
    initialTab: String? = null,
) : ViewModel() {
    private val _state = MutableStateFlow(
        HelpUiState(
            tab = when {
                initialTab in setOf("guides", "tickets") -> initialTab!!
                initialRideId != null -> "tickets"
                else -> "guides"
            },
        ),
    )
    val state: StateFlow<HelpUiState> = _state.asStateFlow()
    private var loadJob: Job? = null
    private var memberSearchJob: Job? = null

    init {
        load(first = true)
    }

    fun selectTab(tab: String) {
        if (_state.value.tab == tab) return
        _state.value = _state.value.copy(tab = tab)
    }

    fun setQuery(q: String) {
        _state.value = _state.value.copy(query = q)
    }

    fun setGuideAudience(audience: String) {
        _state.value = _state.value.copy(guideAudience = audience)
    }

    fun setTicketFilters(
        scopeAll: Boolean? = null,
        status: String? = null,
        assignedToMe: Boolean? = null,
    ) {
        _state.value = _state.value.copy(
            ticketScopeAll = scopeAll ?: _state.value.ticketScopeAll,
            ticketStatus = status ?: _state.value.ticketStatus,
            assignedToMe = assignedToMe ?: _state.value.assignedToMe,
        )
        load(first = false)
    }

    fun refresh() = load(first = false)

    fun load(first: Boolean = false) {
        loadJob?.cancel()
        val me = container.auth.me
        val includeDrafts = me?.can("admin_help") == true
        val isStaff = me?.can("admin_help") == true
        val s = _state.value
        loadJob = viewModelScope.launch {
            _state.value = _state.value.copy(
                loading = first && s.guides.isEmpty() && s.tickets.isEmpty(),
                refreshing = !first,
                error = null,
            )
            try {
                val (guides, tickets, counts) = coroutineScope {
                    val gDef = async {
                        runCatching { container.help.guides(includeDrafts = includeDrafts, full = true) }
                            .getOrDefault(emptyList())
                    }
                    val tDef = async {
                        container.help.tickets(
                            all = isStaff && s.ticketScopeAll,
                            status = s.ticketStatus,
                            q = s.query,
                            assigned = if (isStaff && s.assignedToMe) "me" else null,
                        )
                    }
                    val cDef = async {
                        runCatching { container.help.counts() }.getOrDefault(HelpCountsDto())
                    }
                    Triple(gDef.await(), tDef.await(), cDef.await())
                }
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    guides = guides,
                    tickets = tickets,
                    counts = counts,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    error = e.message,
                )
            }
        }
    }

    fun moveGuide(guideId: Long, delta: Int) {
        val current = _state.value.guides.toMutableList()
        val idx = current.indexOfFirst { it.id == guideId }
        val next = idx + delta
        if (idx < 0 || next !in current.indices) return
        val item = current.removeAt(idx)
        current.add(next, item)
        _state.value = _state.value.copy(guides = current)
        viewModelScope.launch {
            try {
                container.help.reorderGuides(current.map { it.id })
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
                load(first = false)
            }
        }
    }

    fun searchMembers(query: String) {
        memberSearchJob?.cancel()
        if (query.trim().length < 2) {
            _state.value = _state.value.copy(memberResults = emptyList())
            return
        }
        memberSearchJob = viewModelScope.launch {
            delay(250)
            val list = runCatching { container.help.members(query.trim()) }.getOrDefault(emptyList())
            _state.value = _state.value.copy(memberResults = list)
        }
    }

    fun uploadAttachment(context: Context, uri: Uri) {
        _state.value = _state.value.copy(uploadingAttachment = true, error = null)
        viewModelScope.launch {
            try {
                val prepared = withContext(Dispatchers.IO) {
                    prepareImageUpload(context, uri, prefix = "help", maxDimensionPx = 1600, quality = 85)
                }
                val fileRef = container.help.uploadFile(prepared.bytes, prepared.filename, prepared.mime)
                _state.value = _state.value.copy(uploadingAttachment = false, uploadedFileId = fileRef.id)
            } catch (e: Exception) {
                _state.value = _state.value.copy(uploadingAttachment = false, message = e.message)
            }
        }
    }

    fun clearAttachment() {
        _state.value = _state.value.copy(uploadedFileId = null)
    }

    fun createTicket(
        subject: String,
        body: String,
        rideId: Long?,
        forUserId: Long?,
    ) {
        if (_state.value.submitting || subject.isBlank() || body.isBlank()) return
        _state.value = _state.value.copy(submitting = true, message = null)
        viewModelScope.launch {
            try {
                val res = container.help.createTicket(
                    CreateTicketReq(
                        subject = subject.trim(),
                        body = body.trim(),
                        ride_id = rideId,
                        for_user_id = forUserId,
                        file_id = _state.value.uploadedFileId,
                    ),
                )
                _state.value = _state.value.copy(
                    submitting = false,
                    uploadedFileId = null,
                    createdTicketId = res.id,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(submitting = false, message = e.message)
            }
        }
    }

    fun consumeCreatedTicket() {
        _state.value = _state.value.copy(createdTicketId = null)
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    companion object {
        fun factory(container: AppContainer, initialRideId: Long?, initialTab: String?) = viewModelFactory {
            initializer { HelpViewModel(container, initialRideId, initialTab) }
        }
    }
}
