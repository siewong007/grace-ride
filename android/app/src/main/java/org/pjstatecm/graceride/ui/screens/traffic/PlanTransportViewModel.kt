package org.pjstatecm.graceride.ui.screens.traffic

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.ArrangeRespDto
import org.pjstatecm.graceride.data.api.CreatePlanDraftReq
import org.pjstatecm.graceride.data.api.DraftVisitorEntryDto
import org.pjstatecm.graceride.data.api.PlanDataDto
import org.pjstatecm.graceride.data.api.PlanDraftDto
import org.pjstatecm.graceride.data.api.PlanDraftEntryDto
import org.pjstatecm.graceride.data.api.PlanDriverDto
import org.pjstatecm.graceride.data.api.PostEventClaimReq
import org.pjstatecm.graceride.data.api.PostEventDriversDto
import org.pjstatecm.graceride.data.api.PostEventReimbursementsReq
import org.pjstatecm.graceride.data.api.RequestPlanApprovalReq
import org.pjstatecm.graceride.data.api.ReturnPlanDraftReq
import org.pjstatecm.graceride.data.api.SavePlanDraftReq
import org.pjstatecm.graceride.data.api.SavePlanReq
import org.pjstatecm.graceride.data.api.UpdatePlanDraftReq
import org.pjstatecm.graceride.data.api.VisitorReq
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.PlanLogic
import java.util.UUID

enum class PlanNotice {
    SAVED,
    APPROVED,
    STALE_RELOAD,
    LOCK_ACQUIRED,
    LOCK_RELEASED,
    POST_EVENT_SUBMITTED,
}

data class PlanTransportUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val busy: Boolean = false,
    val lockBusy: Boolean = false,
    val error: String? = null,
    val message: String? = null,
    val notice: PlanNotice? = null,
    val plan: PlanDataDto? = null,
    val areas: List<Area> = emptyList(),
    val activeDraftId: Long? = null,
    val assign: Map<Long, Long?> = emptyMap(),
    val savedBaselineAssign: Map<Long, Long?> = emptyMap(),
    val undoStack: List<Map<Long, Long?>> = emptyList(),
    val draftVisitors: List<DraftVisitorEntryDto> = emptyList(),
    val visitorsDirty: Boolean = false,
    val legFilter: String = "all",
    val arrangeDiag: ArrangeRespDto? = null,
    val postEventData: PostEventDriversDto? = null,
    val postEventLoading: Boolean = false,
    val postEventBusy: Boolean = false,
    val showPostEventSheet: Boolean = false,
) {
    val activeDraft: PlanDraftDto?
        get() = plan?.drafts?.firstOrNull { it.id == activeDraftId }

    val lockedByOther: Boolean
        get() {
            val d = activeDraft ?: return false
            return d.lockedBy != null && !d.lockIsMine
        }

    val isEditable: Boolean
        get() {
            val d = activeDraft ?: return true
            if (lockedByOther) return false
            return d.status in setOf("draft", "returned")
        }

    val hasUnsavedChanges: Boolean
        get() = plan != null && (assign != savedBaselineAssign || visitorsDirty)
}

class PlanTransportViewModel(
    private val container: AppContainer,
    private val eventId: Long,
    private val initialDraftId: Long?,
) : ViewModel() {

    private val _state = MutableStateFlow(PlanTransportUiState())
    val state: StateFlow<PlanTransportUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(first = true, preferredDraftId = initialDraftId)
    }

    fun refresh() {
        load(first = false, preferredDraftId = _state.value.activeDraftId)
    }

    private fun load(first: Boolean, preferredDraftId: Long?, keepCurrentEdits: Boolean = false) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            if (first && _state.value.plan == null) {
                _state.value = _state.value.copy(loading = true, error = null)
            } else if (!keepCurrentEdits) {
                _state.value = _state.value.copy(refreshing = true)
            }
            val (planRes, areasList) = coroutineScope {
                val pDef = async { container.traffic.eventPlan(eventId) }
                val aDef = async { runCatching { container.rides.areas() }.getOrDefault(emptyList()) }
                pDef.await() to aDef.await()
            }
            planRes.onSuccess { dto ->
                val activeDrafts = dto.drafts.filter { it.status != "superseded" }
                val cleanDto = dto.copy(drafts = activeDrafts)
                val targetDraftId = when {
                    preferredDraftId != null && activeDrafts.any { it.id == preferredDraftId } -> preferredDraftId
                    preferredDraftId == null && !first -> null
                    else -> dto.draft?.id?.takeIf { id -> activeDrafts.any { it.id == id } }
                        ?: activeDrafts.firstOrNull { it.status in setOf("draft", "pending_approval", "returned") }?.id
                }
                val selectedDraft = activeDrafts.firstOrNull { it.id == targetDraftId }
                val nextAssign = if (keepCurrentEdits && _state.value.activeDraftId == targetDraftId) {
                    _state.value.assign
                } else {
                    baselineAssign(cleanDto, selectedDraft)
                }
                val nextVisitors = if (keepCurrentEdits && _state.value.activeDraftId == targetDraftId && _state.value.visitorsDirty) {
                    _state.value.draftVisitors
                } else {
                    selectedDraft?.visitors.orEmpty()
                }
                val nextSavedBaseline = if (keepCurrentEdits && _state.value.activeDraftId == targetDraftId) {
                    _state.value.savedBaselineAssign
                } else {
                    nextAssign
                }
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    error = null,
                    plan = cleanDto,
                    areas = areasList.ifEmpty { _state.value.areas },
                    activeDraftId = targetDraftId,
                    assign = nextAssign,
                    savedBaselineAssign = nextSavedBaseline,
                    undoStack = if (keepCurrentEdits) _state.value.undoStack else emptyList(),
                    draftVisitors = nextVisitors,
                    visitorsDirty = if (keepCurrentEdits) _state.value.visitorsDirty else false,
                )
            }.onFailure { err ->
                _state.value = _state.value.copy(
                    loading = false,
                    refreshing = false,
                    error = if (_state.value.plan == null) err.message else null,
                    message = if (_state.value.plan != null) err.message else null,
                )
            }
        }
    }

    private fun baselineAssign(plan: PlanDataDto, draft: PlanDraftDto?): Map<Long, Long?> {
        val base = plan.rides.associate { it.id to it.driverId }.toMutableMap()
        if (draft != null) {
            val driverIds = plan.drivers.map { it.id }.toSet()
            for (entry in draft.assignments) {
                if (entry.rideId in base) {
                    base[entry.rideId] = entry.driverId?.takeIf { it in driverIds }
                }
            }
        }
        return base
    }

    fun selectVersion(draftId: Long?) {
        val prevDraft = _state.value.activeDraft
        if (prevDraft != null && prevDraft.id != draftId && prevDraft.lockIsMine) {
            viewModelScope.launch {
                container.traffic.releasePlanDraftLock(eventId, prevDraft.id)
            }
        }
        val plan = _state.value.plan ?: return
        val draft = if (draftId == null) null else plan.drafts.firstOrNull { it.id == draftId }
        val base = baselineAssign(plan, draft)
        _state.value = _state.value.copy(
            activeDraftId = draft?.id,
            assign = base,
            savedBaselineAssign = base,
            undoStack = emptyList(),
            draftVisitors = draft?.visitors.orEmpty(),
            visitorsDirty = false,
            arrangeDiag = null,
        )
    }

    fun setLegFilter(leg: String) {
        _state.value = _state.value.copy(legFilter = leg)
    }

    /** Acquires or verifies our lease lock before mutating an editable draft. */
    private suspend fun ensureLock(): Boolean {
        val draft = _state.value.activeDraft ?: return true
        if (draft.status !in setOf("draft", "returned")) return false
        if (draft.lockedBy != null && !draft.lockIsMine) {
            load(first = false, preferredDraftId = draft.id, keepCurrentEdits = true)
            return false
        }
        if (draft.lockIsMine) return true
        val res = container.traffic.acquirePlanDraftLock(eventId, draft.id, force = false)
        var acquired = false
        res.onSuccess { mut ->
            mut.draft?.let { patchDraftInPlan(it) }
            acquired = true
        }.onFailure { err ->
            _state.value = _state.value.copy(message = err.message)
            if (err is ApiException && err.status == 423) {
                load(first = false, preferredDraftId = draft.id, keepCurrentEdits = true)
            }
        }
        return acquired
    }

    private fun patchDraftInPlan(updated: PlanDraftDto) {
        val curPlan = _state.value.plan ?: return
        val replaced = curPlan.drafts.map { if (it.id == updated.id) updated else it }
        _state.value = _state.value.copy(
            plan = curPlan.copy(
                draft = if (curPlan.draft?.id == updated.id) updated else curPlan.draft,
                drafts = if (replaced.any { it.id == updated.id }) replaced else (listOf(updated) + replaced),
            ),
        )
    }

    fun renewLock() {
        val draft = _state.value.activeDraft ?: return
        if (!draft.lockIsMine || draft.status !in setOf("draft", "returned")) return
        viewModelScope.launch {
            container.traffic.acquirePlanDraftLock(eventId, draft.id, force = false)
                .onSuccess { mut -> mut.draft?.let { patchDraftInPlan(it) } }
                .onFailure { err ->
                    if (err is ApiException && err.status in setOf(409, 423)) {
                        _state.value = _state.value.copy(message = err.message)
                        load(first = false, preferredDraftId = draft.id, keepCurrentEdits = true)
                    }
                }
        }
    }

    fun takeOverLock() {
        val draft = _state.value.activeDraft ?: return
        if (_state.value.lockBusy) return
        _state.value = _state.value.copy(lockBusy = true)
        viewModelScope.launch {
            container.traffic.acquirePlanDraftLock(eventId, draft.id, force = true)
                .onSuccess { mut ->
                    mut.draft?.let { patchDraftInPlan(it) }
                    _state.value = _state.value.copy(notice = PlanNotice.LOCK_ACQUIRED)
                }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(lockBusy = false)
        }
    }

    fun assignRide(rideId: Long, driverId: Long?) {
        if (!_state.value.isEditable) return
        viewModelScope.launch {
            if (!ensureLock()) return@launch
            val cur = _state.value
            val next = cur.assign.toMutableMap().apply { put(rideId, driverId) }
            if (next != cur.assign) {
                _state.value = cur.copy(
                    assign = next,
                    undoStack = (cur.undoStack + cur.assign).takeLast(30),
                )
            }
        }
    }

    fun applyLastTime() {
        val plan = _state.value.plan ?: return
        if (!_state.value.isEditable) return
        viewModelScope.launch {
            if (!ensureLock()) return@launch
            val driversById = plan.drivers.associateBy { it.id }
            val (next, placed) = PlanLogic.applyLastTime(plan.rides, _state.value.assign, driversById)
            if (placed > 0) {
                _state.value = _state.value.copy(
                    assign = next,
                    undoStack = (_state.value.undoStack + _state.value.assign).takeLast(30),
                )
            }
        }
    }

    fun applyMatchReturnDrivers() {
        val plan = _state.value.plan ?: return
        if (!_state.value.isEditable) return
        viewModelScope.launch {
            if (!ensureLock()) return@launch
            val driversById = plan.drivers.associateBy { it.id }
            val (next, matched) = PlanLogic.applyMatchReturnDrivers(plan.rides, _state.value.assign, driversById)
            if (matched > 0) {
                _state.value = _state.value.copy(
                    assign = next,
                    undoStack = (_state.value.undoStack + _state.value.assign).takeLast(30),
                )
            }
        }
    }

    fun autoArrange() {
        val plan = _state.value.plan ?: return
        if (!_state.value.isEditable || _state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            if (!ensureLock()) {
                _state.value = _state.value.copy(busy = false)
                return@launch
            }
            val curAssign = _state.value.assign
            val entries = PlanLogic.draftEntries(plan.rides, curAssign)
            container.traffic.arrangeEventPlan(eventId, entries)
                .onSuccess { resp ->
                    val next = curAssign.toMutableMap()
                    resp.placed.forEach { p -> next[p.rideId] = p.driverId }
                    _state.value = _state.value.copy(
                        assign = next,
                        undoStack = (_state.value.undoStack + curAssign).takeLast(30),
                        arrangeDiag = resp,
                    )
                }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun dismissArrangeDiag() {
        _state.value = _state.value.copy(arrangeDiag = null)
    }

    fun undo() {
        val stack = _state.value.undoStack
        if (stack.isEmpty() || !_state.value.isEditable) return
        _state.value = _state.value.copy(
            assign = stack.last(),
            undoStack = stack.dropLast(1),
        )
    }

    fun resetBoard() {
        val plan = _state.value.plan ?: return
        if (!_state.value.isEditable) return
        val draft = _state.value.activeDraft
        val base = baselineAssign(plan, draft)
        _state.value = _state.value.copy(
            assign = base,
            savedBaselineAssign = base,
            undoStack = emptyList(),
            draftVisitors = draft?.visitors.orEmpty(),
            visitorsDirty = false,
        )
    }

    fun reportMessage(msg: String) {
        _state.value = _state.value.copy(message = msg)
    }

    fun saveCurrentBoard(onSuccess: () -> Unit = {}) {
        val plan = _state.value.plan ?: return
        if (!_state.value.isEditable || _state.value.busy) return
        val activeId = _state.value.activeDraftId
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            if (activeId != null) {
                if (!ensureLock()) {
                    _state.value = _state.value.copy(busy = false)
                    return@launch
                }
                val entries = PlanLogic.draftEntries(plan.rides, _state.value.assign)
                val visitors = _state.value.draftVisitors
                container.traffic.updatePlanDraft(
                    eventId,
                    activeId,
                    UpdatePlanDraftReq(assignments = entries, visitors = visitors),
                ).onSuccess { mut ->
                    _state.value = _state.value.copy(
                        savedBaselineAssign = _state.value.assign,
                        visitorsDirty = false,
                        notice = PlanNotice.SAVED,
                        busy = false,
                    )
                    load(first = false, preferredDraftId = mut.draft?.id ?: activeId)
                    onSuccess()
                }.onFailure { err ->
                    _state.value = _state.value.copy(busy = false, message = err.message)
                    if (err is ApiException && err.status in setOf(409, 423)) {
                        load(first = false, preferredDraftId = activeId)
                    }
                }
            } else {
                val changes = PlanLogic.changesOf(plan.rides, _state.value.assign)
                container.traffic.saveEventPlan(eventId, SavePlanReq(changes = changes, draft_id = null))
                    .onSuccess {
                        _state.value = _state.value.copy(
                            savedBaselineAssign = _state.value.assign,
                            notice = PlanNotice.SAVED,
                            busy = false,
                        )
                        load(first = false, preferredDraftId = null)
                        onSuccess()
                    }
                    .onFailure { err ->
                        _state.value = _state.value.copy(busy = false)
                        if (err is ApiException && err.status == 409) {
                            _state.value = _state.value.copy(notice = PlanNotice.STALE_RELOAD)
                            load(first = false, preferredDraftId = null)
                        } else {
                            _state.value = _state.value.copy(message = err.message)
                        }
                    }
            }
        }
    }

    fun createNewDraft(label: String, cloneCurrent: Boolean) {
        val plan = _state.value.plan ?: return
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            val prevDraft = _state.value.activeDraft
            if (prevDraft != null && prevDraft.lockIsMine) {
                container.traffic.releasePlanDraftLock(eventId, prevDraft.id)
            }
            val req = if (cloneCurrent) {
                CreatePlanDraftReq(
                    label = label.trim().ifBlank { null },
                    clone_from_draft_id = _state.value.activeDraftId,
                    assignments = PlanLogic.draftEntries(plan.rides, _state.value.assign),
                    visitors = _state.value.draftVisitors,
                )
            } else {
                CreatePlanDraftReq(label = label.trim().ifBlank { null })
            }
            container.traffic.createPlanDraft(eventId, req)
                .onSuccess { resp ->
                    val created = resp.draft
                    if (created != null) {
                        container.traffic.acquirePlanDraftLock(eventId, created.id, force = false)
                    }
                    load(first = false, preferredDraftId = created?.id)
                    _state.value = _state.value.copy(notice = PlanNotice.SAVED)
                }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun saveDraft() {
        val plan = _state.value.plan ?: return
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            val activeId = _state.value.activeDraftId
            if (activeId != null && !ensureLock()) {
                _state.value = _state.value.copy(busy = false)
                return@launch
            }
            val entries = PlanLogic.draftEntries(plan.rides, _state.value.assign)
            val visitors = _state.value.draftVisitors
            val res = if (activeId != null) {
                container.traffic.updatePlanDraft(
                    eventId,
                    activeId,
                    UpdatePlanDraftReq(assignments = entries, visitors = visitors),
                )
            } else {
                container.traffic.saveLegacyPlanDraft(
                    eventId,
                    SavePlanDraftReq(draft_id = null, assignments = entries, visitors = visitors),
                )
            }
            res.onSuccess { mut ->
                _state.value = _state.value.copy(visitorsDirty = false, notice = PlanNotice.SAVED)
                load(first = false, preferredDraftId = mut.draft?.id ?: activeId)
            }.onFailure { err ->
                _state.value = _state.value.copy(message = err.message)
                if (err is ApiException && err.status in setOf(409, 423)) {
                    load(first = false, preferredDraftId = activeId)
                }
            }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun requestApproval() {
        val plan = _state.value.plan ?: return
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            val activeId = _state.value.activeDraftId
            if (activeId != null && !ensureLock()) {
                _state.value = _state.value.copy(busy = false)
                return@launch
            }
            val req = RequestPlanApprovalReq(
                draft_id = activeId,
                assignments = PlanLogic.draftEntries(plan.rides, _state.value.assign),
                visitors = _state.value.draftVisitors,
            )
            container.traffic.requestPlanApproval(eventId, req)
                .onSuccess { mut ->
                    _state.value = _state.value.copy(visitorsDirty = false, notice = PlanNotice.SAVED)
                    load(first = false, preferredDraftId = mut.draftId ?: mut.draft?.id ?: activeId)
                }
                .onFailure { err ->
                    _state.value = _state.value.copy(message = err.message)
                    if (err is ApiException && err.status in setOf(409, 423)) {
                        load(first = false, preferredDraftId = activeId)
                    }
                }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun returnDraft(notes: String?) {
        val activeId = _state.value.activeDraftId ?: return
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            container.traffic.returnPlanDraft(eventId, ReturnPlanDraftReq(draft_id = activeId, notes = notes))
                .onSuccess {
                    _state.value = _state.value.copy(notice = PlanNotice.SAVED)
                    load(first = false, preferredDraftId = activeId)
                }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun saveLive() {
        val plan = _state.value.plan ?: return
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            val changes = PlanLogic.changesOf(plan.rides, _state.value.assign)
            container.traffic.saveEventPlan(eventId, SavePlanReq(changes = changes, draft_id = null))
                .onSuccess {
                    _state.value = _state.value.copy(notice = PlanNotice.SAVED)
                    load(first = false, preferredDraftId = null)
                }
                .onFailure { err ->
                    if (err is ApiException && err.status == 409) {
                        _state.value = _state.value.copy(notice = PlanNotice.STALE_RELOAD)
                        load(first = false, preferredDraftId = null)
                    } else {
                        _state.value = _state.value.copy(message = err.message)
                    }
                }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun approveAndNotify() {
        val plan = _state.value.plan ?: return
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            val activeId = _state.value.activeDraftId
            if (activeId != null && _state.value.visitorsDirty) {
                val syncRes = container.traffic.updatePlanDraft(
                    eventId,
                    activeId,
                    UpdatePlanDraftReq(
                        assignments = PlanLogic.draftEntries(plan.rides, _state.value.assign),
                        visitors = _state.value.draftVisitors,
                    ),
                )
                if (syncRes.isFailure) {
                    val err = syncRes.exceptionOrNull()
                    _state.value = _state.value.copy(busy = false, message = err?.message)
                    if (err is ApiException && err.status in setOf(409, 423)) {
                        load(first = false, preferredDraftId = activeId)
                    }
                    return@launch
                }
            }
            val changes = PlanLogic.changesOf(plan.rides, _state.value.assign)
            container.traffic.approveEventPlan(eventId, SavePlanReq(changes = changes, draft_id = activeId))
                .onSuccess {
                    _state.value = _state.value.copy(visitorsDirty = false, notice = PlanNotice.APPROVED)
                    load(first = false, preferredDraftId = null)
                }
                .onFailure { err ->
                    if (err is ApiException && err.status == 409) {
                        _state.value = _state.value.copy(notice = PlanNotice.STALE_RELOAD)
                        load(first = false, preferredDraftId = activeId)
                    } else {
                        _state.value = _state.value.copy(message = err.message)
                    }
                }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun deleteDraft(draftId: Long) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            container.traffic.deletePlanDraft(eventId, draftId)
                .onSuccess { load(first = false, preferredDraftId = null) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(busy = false)
        }
    }

    // ---- Visitors (Live vs Draft staged ops) ----

    fun createLiveVisitor(req: VisitorReq) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            container.traffic.createPlanVisitor(eventId, req)
                .onSuccess { load(first = false, preferredDraftId = _state.value.activeDraftId) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun updateLiveVisitor(visitorId: Long, req: VisitorReq) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            container.traffic.updatePlanVisitor(eventId, visitorId, req)
                .onSuccess { load(first = false, preferredDraftId = _state.value.activeDraftId) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun deleteLiveVisitor(visitorId: Long) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            container.traffic.deletePlanVisitor(eventId, visitorId)
                .onSuccess { load(first = false, preferredDraftId = _state.value.activeDraftId) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(busy = false)
        }
    }

    fun stageAddVisitor(
        name: String,
        phone: String,
        notes: String,
        area: Area?,
        pickupAddress: String,
        seats: Long,
        leg: String,
        driver: PlanDriverDto?,
    ) {
        if (!_state.value.isEditable) return
        viewModelScope.launch {
            if (!ensureLock()) return@launch
            val entry = DraftVisitorEntryDto(
                tempId = "tmp-${UUID.randomUUID()}",
                op = "add",
                liveVisitorId = null,
                name = name.trim(),
                phone = phone.trim(),
                notes = notes.trim(),
                areaId = area?.id,
                areaName = area?.name,
                pickupAddress = pickupAddress.trim(),
                seats = seats,
                leg = leg,
                driverId = driver?.id,
                driverName = driver?.name,
            )
            _state.value = _state.value.copy(
                draftVisitors = _state.value.draftVisitors + entry,
                visitorsDirty = true,
            )
        }
    }

    fun stageUpdateVisitor(
        liveVisitorId: Long?,
        tempId: String?,
        name: String,
        phone: String,
        notes: String,
        area: Area?,
        pickupAddress: String,
        seats: Long,
        leg: String,
        driver: PlanDriverDto?,
    ) {
        if (!_state.value.isEditable) return
        viewModelScope.launch {
            if (!ensureLock()) return@launch
            val curList = _state.value.draftVisitors.toMutableList()
            if (liveVisitorId != null) {
                curList.removeAll { it.liveVisitorId == liveVisitorId }
                curList += DraftVisitorEntryDto(
                    tempId = tempId ?: "upd-$liveVisitorId",
                    op = "update",
                    liveVisitorId = liveVisitorId,
                    name = name.trim(),
                    phone = phone.trim(),
                    notes = notes.trim(),
                    areaId = area?.id,
                    areaName = area?.name,
                    pickupAddress = pickupAddress.trim(),
                    seats = seats,
                    leg = leg,
                    driverId = driver?.id,
                    driverName = driver?.name,
                )
            } else if (tempId != null) {
                val idx = curList.indexOfFirst { it.tempId == tempId && it.op == "add" }
                if (idx >= 0) {
                    curList[idx] = curList[idx].copy(
                        name = name.trim(),
                        phone = phone.trim(),
                        notes = notes.trim(),
                        areaId = area?.id,
                        areaName = area?.name,
                        pickupAddress = pickupAddress.trim(),
                        seats = seats,
                        leg = leg,
                        driverId = driver?.id,
                        driverName = driver?.name,
                    )
                }
            }
            _state.value = _state.value.copy(draftVisitors = curList, visitorsDirty = true)
        }
    }

    fun stageDeleteVisitor(liveVisitorId: Long?, tempId: String?, name: String) {
        if (!_state.value.isEditable) return
        viewModelScope.launch {
            if (!ensureLock()) return@launch
            val curList = _state.value.draftVisitors.toMutableList()
            if (liveVisitorId != null) {
                curList.removeAll { it.liveVisitorId == liveVisitorId }
                curList += DraftVisitorEntryDto(
                    tempId = "del-$liveVisitorId",
                    op = "delete",
                    liveVisitorId = liveVisitorId,
                    name = name,
                )
            } else if (tempId != null) {
                curList.removeAll { it.tempId == tempId && it.op == "add" }
            }
            _state.value = _state.value.copy(draftVisitors = curList, visitorsDirty = true)
        }
    }

    fun submitSpecialReimbursement(visitorId: Long, existingReimbId: Long?, amountCents: Long, reason: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            val res = if (existingReimbId != null) {
                container.traffic.resubmitVisitorSpecialReimbursement(eventId, visitorId, existingReimbId, amountCents, reason)
            } else {
                container.traffic.requestVisitorSpecialReimbursement(eventId, visitorId, amountCents, reason).map { }
            }
            res.onSuccess {
                _state.value = _state.value.copy(notice = PlanNotice.SAVED)
                load(first = false, preferredDraftId = _state.value.activeDraftId, keepCurrentEdits = true)
            }.onFailure { err ->
                _state.value = _state.value.copy(message = err.message)
            }
            _state.value = _state.value.copy(busy = false)
        }
    }

    // ---- Post-Event Driver Reimbursements ----

    fun openPostEventSheet() {
        _state.value = _state.value.copy(showPostEventSheet = true, postEventLoading = true)
        viewModelScope.launch {
            container.traffic.postEventDrivers(eventId)
                .onSuccess { dto -> _state.value = _state.value.copy(postEventData = dto, postEventLoading = false) }
                .onFailure { err -> _state.value = _state.value.copy(postEventLoading = false, showPostEventSheet = false, message = err.message) }
        }
    }

    fun closePostEventSheet() {
        _state.value = _state.value.copy(showPostEventSheet = false)
    }

    fun submitPostEventClaims(supperNote: String?, claims: List<PostEventClaimReq>) {
        if (claims.isEmpty() || _state.value.postEventBusy) return
        _state.value = _state.value.copy(postEventBusy = true)
        viewModelScope.launch {
            container.traffic.createPostEventReimbursements(
                eventId,
                PostEventReimbursementsReq(
                    supper_note = supperNote?.trim()?.ifBlank { null },
                    claims = claims,
                ),
            ).onSuccess {
                _state.value = _state.value.copy(
                    postEventBusy = false,
                    showPostEventSheet = false,
                    notice = PlanNotice.POST_EVENT_SUBMITTED,
                )
            }.onFailure { err ->
                _state.value = _state.value.copy(postEventBusy = false, message = err.message)
            }
        }
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null, notice = null)
    }

    override fun onCleared() {
        val draft = _state.value.activeDraft
        if (draft != null && draft.lockIsMine) {
            container.appScope.launch {
                withContext(NonCancellable) {
                    container.traffic.releasePlanDraftLock(eventId, draft.id)
                }
            }
        }
        super.onCleared()
    }

    companion object {
        fun factory(container: AppContainer, eventId: Long, initialDraftId: Long?): ViewModelProvider.Factory =
            viewModelFactory {
                initializer { PlanTransportViewModel(container, eventId, initialDraftId) }
            }
    }
}
