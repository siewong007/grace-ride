package org.pjstatecm.graceride.ui.screens.traffic

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.AdminRidesPageDto
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.AreaFareDto
import org.pjstatecm.graceride.data.api.AreaPhotosDto
import org.pjstatecm.graceride.data.api.LiveDriversDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.RideChangeDraftDto
import org.pjstatecm.graceride.data.api.TrafficEventPlanItemDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.notifications.RealtimeEvent
import java.time.DayOfWeek
import java.time.temporal.TemporalAdjusters

data class TrafficHubUiState(
    val tab: String = "plans",
    val me: Me? = null,
    val stats: AdminDashboardStatsDto? = null,
    val refreshing: Boolean = false,
    val message: String? = null,
    val batchApprovedCount: Int? = null,
    val batchFailedCount: Int = 0,
    val quickApprovingRideId: Long? = null,

    // Plans tab
    val planFilter: String = "all",
    val plans: List<TrafficEventPlanItemDto> = emptyList(),
    val plansLoading: Boolean = true,
    val plansError: String? = null,

    // Rides tab
    val ridesView: String = "rides", // "rides" | "drafts"
    val ridesQuery: String = "",
    val ridesStatus: String = "",
    val ridesDate: String = "", // "" | "today" | "sunday"
    val ridesAttention: Boolean = false,
    val ridesPage: AdminRidesPageDto = AdminRidesPageDto(),
    val ridesLoading: Boolean = false,
    val ridesError: String? = null,
    val selectedRideIds: Set<Long> = emptySet(),
    val batchBusy: Boolean = false,

    // Proposed Ride Change Drafts
    val rideDrafts: List<RideChangeDraftDto> = emptyList(),
    val rideDraftsFilter: String = "",
    val rideDraftsLoading: Boolean = false,
    val rideDraftsError: String? = null,
    val rideDraftsBusyId: Long? = null,

    // Live tab
    val liveData: LiveDriversDto? = null,
    val liveLoading: Boolean = false,
    val liveError: String? = null,
    val liveFocusPoint: Pair<Double, Double>? = null,
    val liveUpdatedAtMs: Long? = null,

    // Areas tab
    val areaSubTab: String = "photos",
    val areas: List<Area> = emptyList(),
    val areasError: String? = null,
    val selectedAreaId: Long? = null,
    val areaPhotos: AreaPhotosDto? = null,
    val areaPhotosLoading: Boolean = false,
    val areaPhotosError: String? = null,
    val areaFares: List<AreaFareDto> = emptyList(),
    val fareSearchQuery: String = "",
    val areasLoading: Boolean = false,
    val areasBusy: Boolean = false,
)

class TrafficHubViewModel(
    private val container: AppContainer,
    initialTab: String,
    initialRidesView: String,
    initialAreaTab: String,
    initialRidesStatus: String?,
    initialRidesAttention: Boolean,
    initialPlanFilter: String? = null,
) : ViewModel() {

    private val _state = MutableStateFlow(
        TrafficHubUiState(
            me = container.auth.me,
            tab = resolvePermittedTab(initialTab, container.auth.me),
            ridesView = if (initialRidesView == "drafts") "drafts" else "rides",
            areaSubTab = resolvePermittedAreaTab(initialAreaTab, container.auth.me),
            ridesStatus = initialRidesStatus.orEmpty(),
            ridesAttention = initialRidesAttention,
            planFilter = initialPlanFilter?.takeIf { it in setOf("all", "needs_work", "drafts", "pending", "returned") } ?: "all",
        ),
    )
    val state: StateFlow<TrafficHubUiState> = _state.asStateFlow()
    private var tabJob: Job? = null
    private var searchJob: Job? = null

    init {
        loadAllInitial()
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Chat) {
                    refreshActive(silent = true)
                }
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect {
                refreshActive(silent = true)
            }
        }
    }

    fun syncMe(me: Me?) {
        val currentTab = resolvePermittedTab(_state.value.tab, me)
        val currentAreaTab = resolvePermittedAreaTab(_state.value.areaSubTab, me)
        if (_state.value.me != me || currentTab != _state.value.tab || currentAreaTab != _state.value.areaSubTab) {
            _state.value = _state.value.copy(me = me, tab = currentTab, areaSubTab = currentAreaTab)
        }
    }

    private fun loadAllInitial() {
        tabJob?.cancel()
        tabJob = viewModelScope.launch {
            refreshStats(passive = true)
            loadForTab(_state.value.tab, silent = false)
        }
    }

    fun setTab(tab: String) {
        val allowed = resolvePermittedTab(tab, _state.value.me ?: container.auth.me)
        if (allowed == _state.value.tab) return
        _state.value = _state.value.copy(tab = allowed)
        refreshActive(silent = false)
    }

    fun refreshActive(silent: Boolean = false) {
        tabJob?.cancel()
        tabJob = viewModelScope.launch {
            if (!silent) _state.value = _state.value.copy(refreshing = true)
            refreshStats(passive = silent)
            loadForTab(_state.value.tab, silent = silent)
            if (!silent) _state.value = _state.value.copy(refreshing = false)
        }
    }

    private suspend fun refreshStats(passive: Boolean) {
        val me = _state.value.me ?: container.auth.me ?: return
        if (me.can("admin_dashboard") || me.can("admin_plan") || me.can("admin_rides")) {
            container.traffic.dashboardStats(passive = passive).onSuccess { stats ->
                _state.value = _state.value.copy(stats = stats)
            }
        }
    }

    private suspend fun loadForTab(tab: String, silent: Boolean) {
        when (tab) {
            "plans" -> loadPlans(silent)
            "rides" -> if (_state.value.ridesView == "drafts") loadRideDrafts(silent) else loadAdminRides(page = _state.value.ridesPage.page, silent = silent)
            "live" -> loadLive(silent)
            "areas" -> loadAreasData(silent)
        }
    }

    // ---- Plans ----

    fun setPlanFilter(filter: String) {
        _state.value = _state.value.copy(planFilter = filter)
    }

    fun jumpToPendingReview(isLead: Boolean) {
        val me = _state.value.me ?: container.auth.me
        val stats = _state.value.stats
        val hasPendingPlans = (stats?.pendingPlanDrafts ?: 0L) > 0L || _state.value.plans.any { it.pendingDraftCount > 0L }
        if (isLead && hasPendingPlans && me?.can("admin_plan") == true) {
            _state.value = _state.value.copy(tab = resolvePermittedTab("plans", me), planFilter = "pending")
            refreshActive(silent = false)
        } else if (me?.can("admin_rides") == true) {
            _state.value = _state.value.copy(
                tab = resolvePermittedTab("rides", me),
                ridesView = "drafts",
                rideDraftsFilter = "pending_approval",
                selectedRideIds = emptySet(),
            )
            tabJob?.cancel()
            tabJob = viewModelScope.launch { loadRideDrafts(silent = false) }
        }
    }

    fun jumpToReturnedDrafts() {
        val me = _state.value.me ?: container.auth.me
        val stats = _state.value.stats
        val hasReturnedPlans = (stats?.myReturnedPlanDrafts ?: 0L) > 0L || _state.value.plans.any { it.returnedDraftCount > 0L }
        if (hasReturnedPlans && me?.can("admin_plan") == true) {
            _state.value = _state.value.copy(tab = resolvePermittedTab("plans", me), planFilter = "returned")
            refreshActive(silent = false)
        } else if (me?.can("admin_rides") == true) {
            _state.value = _state.value.copy(
                tab = resolvePermittedTab("rides", me),
                ridesView = "drafts",
                rideDraftsFilter = "returned",
                selectedRideIds = emptySet(),
            )
            tabJob?.cancel()
            tabJob = viewModelScope.launch { loadRideDrafts(silent = false) }
        }
    }

    private suspend fun loadPlans(silent: Boolean) {
        if (!silent && _state.value.plans.isEmpty()) {
            _state.value = _state.value.copy(plansLoading = true, plansError = null)
        }
        val res = container.traffic.trafficPlans()
        res.onSuccess { list ->
            _state.value = _state.value.copy(plans = list, plansLoading = false, plansError = null)
        }.onFailure { err ->
            _state.value = _state.value.copy(plansLoading = false, plansError = err.message)
        }
    }

    // ---- Rides & Proposed Change Drafts ----

    fun setRidesView(view: String) {
        if (view == _state.value.ridesView) return
        searchJob?.cancel()
        _state.value = _state.value.copy(ridesView = view, selectedRideIds = emptySet())
        tabJob?.cancel()
        tabJob = viewModelScope.launch {
            if (view == "drafts") loadRideDrafts(silent = false) else loadAdminRides(page = 1, silent = false)
        }
    }

    fun setRidesQuery(query: String) {
        _state.value = _state.value.copy(ridesQuery = query)
        searchJob?.cancel()
        if (_state.value.tab == "rides" && _state.value.ridesView == "rides") {
            searchJob = viewModelScope.launch {
                delay(350L)
                loadAdminRides(page = 1, silent = false)
            }
        }
    }

    fun clearRidesQuery() {
        searchJob?.cancel()
        _state.value = _state.value.copy(ridesQuery = "")
        if (_state.value.tab == "rides" && _state.value.ridesView == "rides") {
            tabJob?.cancel()
            tabJob = viewModelScope.launch { loadAdminRides(page = 1, silent = false) }
        }
    }

    fun resetRidesFilters() {
        searchJob?.cancel()
        _state.value = _state.value.copy(
            ridesStatus = "",
            ridesDate = "",
            ridesAttention = false,
            ridesQuery = "",
            selectedRideIds = emptySet(),
        )
        tabJob?.cancel()
        tabJob = viewModelScope.launch {
            loadAdminRides(page = 1, silent = false)
        }
    }

    fun applyRidesFilters(
        status: String = _state.value.ridesStatus,
        date: String = _state.value.ridesDate,
        attention: Boolean = _state.value.ridesAttention,
    ) {
        searchJob?.cancel()
        _state.value = _state.value.copy(
            ridesStatus = status,
            ridesDate = date,
            ridesAttention = attention,
            selectedRideIds = emptySet(),
        )
        tabJob?.cancel()
        tabJob = viewModelScope.launch {
            loadAdminRides(page = 1, silent = false)
        }
    }

    fun goToRidesPage(page: Int) {
        tabJob?.cancel()
        tabJob = viewModelScope.launch {
            loadAdminRides(page = page.coerceAtLeast(1), silent = false)
        }
    }

    private fun resolveDateQuery(mode: String): String? = when (mode) {
        "today" -> KlTime.today()
        "sunday" -> {
            val today = KlTime.todayDate()
            val sunday = if (today.dayOfWeek == DayOfWeek.SUNDAY) today else today.with(TemporalAdjusters.next(DayOfWeek.SUNDAY))
            sunday.toString()
        }
        else -> null
    }

    private suspend fun loadAdminRides(page: Int, silent: Boolean) {
        val cur = _state.value
        if (!silent) _state.value = cur.copy(ridesLoading = true, ridesError = null)
        val res = container.traffic.adminRides(
            status = cur.ridesStatus.ifBlank { null },
            date = resolveDateQuery(cur.ridesDate),
            q = cur.ridesQuery.trim().ifBlank { null },
            attention = if (cur.ridesAttention) true else null,
            page = page,
            limit = 50,
        )
        res.onSuccess { paged ->
            _state.value = _state.value.copy(
                ridesPage = paged,
                ridesLoading = false,
                ridesError = null,
                selectedRideIds = _state.value.selectedRideIds.filter { id ->
                    paged.items.any { it.id == id && it.status == "pending_approval" }
                }.toSet(),
            )
        }.onFailure { err ->
            _state.value = _state.value.copy(ridesLoading = false, ridesError = err.message)
        }
    }

    fun toggleSelectRide(rideId: Long) {
        val next = _state.value.selectedRideIds.toMutableSet()
        if (!next.add(rideId)) next.remove(rideId)
        _state.value = _state.value.copy(selectedRideIds = next)
    }

    fun selectAllPendingRides() {
        val pendingIds = _state.value.ridesPage.items
            .filter { it.status == "pending_approval" }
            .map { it.id }
            .toSet()
        _state.value = _state.value.copy(selectedRideIds = pendingIds)
    }

    fun clearSelection() {
        _state.value = _state.value.copy(selectedRideIds = emptySet())
    }

    fun quickApproveRide(rideId: Long) {
        if (_state.value.batchBusy || _state.value.quickApprovingRideId != null) return
        _state.value = _state.value.copy(quickApprovingRideId = rideId)
        viewModelScope.launch {
            container.traffic.staffApproveRide(rideId)
                .onSuccess {
                    _state.value = _state.value.copy(
                        quickApprovingRideId = null,
                        selectedRideIds = _state.value.selectedRideIds - rideId,
                        batchApprovedCount = 1,
                        batchFailedCount = 0,
                    )
                    refreshStats(passive = true)
                    loadAdminRides(page = _state.value.ridesPage.page, silent = true)
                }
                .onFailure { err ->
                    _state.value = _state.value.copy(
                        quickApprovingRideId = null,
                        message = err.message,
                    )
                }
        }
    }

    fun batchApproveSelected() {
        val ids = _state.value.selectedRideIds.toList()
        if (ids.isEmpty() || _state.value.batchBusy) return
        _state.value = _state.value.copy(batchBusy = true)
        viewModelScope.launch {
            var approved = 0
            var failed = 0
            var lastErr: String? = null
            for (rideId in ids) {
                container.traffic.staffApproveRide(rideId)
                    .onSuccess { approved++ }
                    .onFailure {
                        failed++
                        lastErr = it.message
                    }
            }
            _state.value = _state.value.copy(
                batchBusy = false,
                selectedRideIds = emptySet(),
                batchApprovedCount = approved.takeIf { it > 0 },
                batchFailedCount = failed,
                message = if (approved == 0) lastErr else null,
            )
            refreshStats(passive = true)
            loadAdminRides(page = _state.value.ridesPage.page, silent = false)
        }
    }

    fun setRideDraftsFilter(status: String) {
        _state.value = _state.value.copy(rideDraftsFilter = status)
        tabJob?.cancel()
        tabJob = viewModelScope.launch {
            loadRideDrafts(silent = false)
        }
    }

    private suspend fun loadRideDrafts(silent: Boolean) {
        if (!silent) _state.value = _state.value.copy(rideDraftsLoading = true, rideDraftsError = null)
        val filterStatus = _state.value.rideDraftsFilter.ifBlank { null }
        container.traffic.listAllRideDrafts(status = filterStatus)
            .onSuccess { list ->
                _state.value = _state.value.copy(rideDrafts = list, rideDraftsLoading = false, rideDraftsError = null)
            }
            .onFailure { err ->
                _state.value = _state.value.copy(
                    rideDraftsLoading = false,
                    rideDraftsError = if (_state.value.rideDrafts.isEmpty()) err.message else null,
                    message = if (_state.value.rideDrafts.isNotEmpty() && !silent) err.message else _state.value.message,
                )
            }
    }

    fun approveRideDraft(rideId: Long, draftId: Long) {
        if (_state.value.rideDraftsBusyId != null) return
        _state.value = _state.value.copy(rideDraftsBusyId = draftId)
        viewModelScope.launch {
            container.traffic.approveRideDraft(rideId, draftId)
                .onSuccess {
                    refreshStats(passive = true)
                    loadRideDrafts(silent = true)
                }
                .onFailure { err ->
                    _state.value = _state.value.copy(message = err.message)
                    if (err is ApiException && err.status in setOf(409, 423)) loadRideDrafts(silent = true)
                }
            _state.value = _state.value.copy(rideDraftsBusyId = null)
        }
    }

    fun returnRideDraft(rideId: Long, draftId: Long, notes: String?) {
        if (_state.value.rideDraftsBusyId != null) return
        _state.value = _state.value.copy(rideDraftsBusyId = draftId)
        viewModelScope.launch {
            container.traffic.returnRideDraft(rideId, draftId, notes)
                .onSuccess {
                    refreshStats(passive = true)
                    loadRideDrafts(silent = true)
                }
                .onFailure { err ->
                    _state.value = _state.value.copy(message = err.message)
                }
            _state.value = _state.value.copy(rideDraftsBusyId = null)
        }
    }

    // ---- Live ----

    fun loadLive(silent: Boolean = false) {
        viewModelScope.launch {
            if (!silent && _state.value.liveData == null) {
                _state.value = _state.value.copy(liveLoading = true, liveError = null)
            }
            container.traffic.adminLive()
                .onSuccess { dto ->
                    _state.value = _state.value.copy(
                        liveData = dto,
                        liveLoading = false,
                        liveError = null,
                        liveUpdatedAtMs = System.currentTimeMillis(),
                    )
                }
                .onFailure { err ->
                    _state.value = _state.value.copy(liveLoading = false, liveError = err.message)
                }
        }
    }

    fun focusLiveDriver(point: Pair<Double, Double>?) {
        _state.value = _state.value.copy(liveFocusPoint = point)
    }

    // ---- Areas, Photos & Fares ----

    fun setAreaSubTab(subTab: String) {
        val permitted = resolvePermittedAreaTab(subTab, _state.value.me ?: container.auth.me)
        _state.value = _state.value.copy(areaSubTab = permitted)
    }

    fun setFareSearchQuery(query: String) {
        _state.value = _state.value.copy(fareSearchQuery = query)
    }

    fun reportMessage(message: String) {
        _state.value = _state.value.copy(message = message)
    }

    fun selectArea(areaId: Long) {
        if (_state.value.selectedAreaId != areaId) {
            _state.value = _state.value.copy(selectedAreaId = areaId, areaPhotos = null, areaPhotosError = null)
        }
        viewModelScope.launch {
            loadPhotosForArea(areaId)
        }
    }

    private suspend fun loadAreasData(silent: Boolean) {
        if (!silent && _state.value.areas.isEmpty()) {
            _state.value = _state.value.copy(areasLoading = true, areasError = null)
        }
        val me = _state.value.me ?: container.auth.me
        coroutineScope {
            val areasDef = async { runCatching { container.rides.areas(forceRefresh = true) } }
            val faresDef = async {
                if (me?.can("admin_fares") == true) container.traffic.areaFares() else null
            }
            val areasRes = areasDef.await()
            val areaErr = areasRes.exceptionOrNull()
            val areaList = areasRes.getOrElse { _state.value.areas }
            val chosenArea = _state.value.selectedAreaId?.takeIf { id -> areaList.any { it.id == id } }
                ?: areaList.firstOrNull { it.active }?.id
                ?: areaList.firstOrNull()?.id
            val faresRes = faresDef.await()
            val fares = faresRes?.getOrNull()?.fares ?: _state.value.areaFares
            val faresErr = faresRes?.exceptionOrNull()
            _state.value = _state.value.copy(
                areas = areaList,
                areasError = if (areaList.isEmpty()) (areaErr?.message ?: faresErr?.message) else null,
                selectedAreaId = chosenArea,
                areaFares = fares,
                areasLoading = false,
                message = if (areaList.isNotEmpty() && !silent) (areaErr?.message ?: faresErr?.message ?: _state.value.message) else _state.value.message,
            )
            if (chosenArea != null && me?.can("admin_area_photos") == true) {
                loadPhotosForArea(chosenArea)
            }
        }
    }

    private suspend fun loadPhotosForArea(areaId: Long) {
        val sameArea = _state.value.areaPhotos?.area?.id == areaId
        _state.value = _state.value.copy(
            areaPhotosLoading = !sameArea,
            areaPhotosError = null,
        )
        container.traffic.areaPhotos(areaId)
            .onSuccess { dto ->
                _state.value = _state.value.copy(
                    areaPhotos = dto,
                    areaPhotosLoading = false,
                    areaPhotosError = null,
                )
            }
            .onFailure { err ->
                _state.value = _state.value.copy(
                    areaPhotosLoading = false,
                    areaPhotosError = if (_state.value.areaPhotos?.area?.id != areaId) err.message else null,
                    message = if (_state.value.areaPhotos?.area?.id == areaId) err.message else _state.value.message,
                )
            }
    }

    fun uploadAndAddPhoto(areaId: Long, bytes: ByteArray, fileName: String, mimeType: String, caption: String?) {
        if (_state.value.areasBusy) return
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            val uploaded = container.traffic.uploadPhotoFile(bytes, fileName, mimeType)
            uploaded.onSuccess { fileRef ->
                container.traffic.addAreaPhoto(areaId, fileRef.id, caption)
                    .onSuccess { loadPhotosForArea(areaId) }
                    .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            }.onFailure { err ->
                _state.value = _state.value.copy(message = err.message)
            }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun updatePhotoCaption(areaId: Long, photoId: Long, caption: String?) {
        if (_state.value.areasBusy) return
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            container.traffic.updateAreaPhoto(areaId, photoId, caption)
                .onSuccess { loadPhotosForArea(areaId) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun movePhoto(areaId: Long, photoId: Long, delta: Int) {
        val photos = _state.value.areaPhotos?.photos ?: return
        val idx = photos.indexOfFirst { it.id == photoId }
        val target = idx + delta
        if (idx == -1 || target !in photos.indices || _state.value.areasBusy) return
        val reordered = photos.toMutableList().apply {
            val item = removeAt(idx)
            add(target, item)
        }
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            container.traffic.reorderAreaPhotos(areaId, reordered.map { it.id })
                .onSuccess { loadPhotosForArea(areaId) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun deletePhoto(areaId: Long, photoId: Long) {
        if (_state.value.areasBusy) return
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            container.traffic.deleteAreaPhoto(areaId, photoId)
                .onSuccess { loadPhotosForArea(areaId) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun putFare(areaAId: Long, areaBId: Long, amountCents: Long) {
        if (_state.value.areasBusy) return
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            container.traffic.putAreaFare(areaAId, areaBId, amountCents)
                .onSuccess {
                    container.traffic.areaFares().onSuccess { resp ->
                        _state.value = _state.value.copy(areaFares = resp.fares)
                    }
                }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun deleteFare(fareId: Long) {
        if (_state.value.areasBusy) return
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            container.traffic.deleteAreaFare(fareId)
                .onSuccess {
                    container.traffic.areaFares().onSuccess { resp ->
                        _state.value = _state.value.copy(areaFares = resp.fares)
                    }
                }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun createArea(name: String, active: Boolean) {
        if (_state.value.areasBusy) return
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            container.traffic.createArea(name, active)
                .onSuccess { loadAreasData(silent = true) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun updateArea(areaId: Long, name: String, active: Boolean) {
        if (_state.value.areasBusy) return
        _state.value = _state.value.copy(areasBusy = true)
        viewModelScope.launch {
            container.traffic.updateArea(areaId, name, active)
                .onSuccess { loadAreasData(silent = true) }
                .onFailure { err -> _state.value = _state.value.copy(message = err.message) }
            _state.value = _state.value.copy(areasBusy = false)
        }
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null, batchApprovedCount = null, batchFailedCount = 0)
    }

    companion object {
        fun resolvePermittedTab(requested: String, me: Me?): String {
            if (me == null) return requested
            val ordered = buildList {
                if (me.can("admin_plan")) add("plans")
                if (me.can("admin_rides")) add("rides")
                if (me.can("admin_live")) add("live")
                if (me.can("admin_area_photos") || me.can("admin_areas") || me.can("admin_fares")) add("areas")
            }
            return if (requested in ordered) requested else ordered.firstOrNull() ?: "plans"
        }

        fun resolvePermittedAreaTab(requested: String, me: Me?): String {
            if (me == null) return requested
            val ordered = buildList {
                if (me.can("admin_area_photos")) add("photos")
                if (me.can("admin_fares")) add("fares")
                if (me.can("admin_areas")) add("areas")
            }
            return if (requested in ordered) requested else ordered.firstOrNull() ?: "photos"
        }

        fun factory(
            container: AppContainer,
            initialTab: String,
            initialRidesView: String,
            initialAreaTab: String,
            initialRidesStatus: String?,
            initialRidesAttention: Boolean,
            initialPlanFilter: String? = null,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer {
                TrafficHubViewModel(
                    container = container,
                    initialTab = initialTab,
                    initialRidesView = initialRidesView,
                    initialAreaTab = initialAreaTab,
                    initialRidesStatus = initialRidesStatus,
                    initialRidesAttention = initialRidesAttention,
                    initialPlanFilter = initialPlanFilter,
                )
            }
        }
    }
}
