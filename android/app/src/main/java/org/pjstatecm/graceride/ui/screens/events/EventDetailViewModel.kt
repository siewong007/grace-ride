package org.pjstatecm.graceride.ui.screens.events

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Area
import org.pjstatecm.graceride.data.api.CheckinResp
import org.pjstatecm.graceride.data.api.CreateVisitorPassReq
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.EventSignupItemDto
import org.pjstatecm.graceride.data.api.PostEventDriversDto
import org.pjstatecm.graceride.data.api.PostEventReimbursementsReq
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.ScanMemberResp
import org.pjstatecm.graceride.data.api.VisitorPassDto
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.notifications.RealtimeEvent

data class EventDetailUiState(
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val event: EventItem? = null,
    val outboundRide: Ride? = null,
    val returnRide: Ride? = null,
    val drivingForEvent: List<Ride> = emptyList(),
    val signups: List<EventSignupItemDto> = emptyList(),
    val visitors: List<VisitorPassDto> = emptyList(),
    val areas: List<Area> = emptyList(),
    val driveRequests: List<Ride> = emptyList(),
    val driveRequestsLoading: Boolean = false,
    val walkInCandidates: List<AdminUserDto> = emptyList(),
    val postEventDrivers: PostEventDriversDto? = null,
    val postEventOpen: Boolean = false,
    val visitorSharePair: Pair<String, String>? = null,
    val error: String? = null,
    val busy: Boolean = false,
    val message: String? = null,
    val checkinError: String? = null,
    val actionNotice: EventActionNotice? = null,
    val checkinResult: CheckinResp? = null,
    val scanMemberResult: ScanMemberResp? = null,
)

class EventDetailViewModel(private val container: AppContainer, private val id: Long) : ViewModel() {
    private val _state = MutableStateFlow(EventDetailUiState())
    val state: StateFlow<EventDetailUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load()
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) refresh()
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect { refresh() }
        }
    }

    fun load() {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            reloadDetail()
        }
    }

    fun refresh() {
        loadJob?.cancel()
        _state.value = _state.value.copy(refreshing = _state.value.event != null)
        loadJob = viewModelScope.launch {
            reloadDetail()
        }
    }

    private suspend fun reloadDetail() {
        _state.value = _state.value.copy(loading = _state.value.event == null)
        try {
            val event = container.events.detail(id)
            var outbound: Ride? = null
            var returnRide: Ride? = null
            var driving: List<Ride> = emptyList()

            if (event.mySignup?.status == "going") {
                val canRides = container.auth.me?.can("rides") == true
                val canDrive = container.auth.me?.can("drive") == true
                val day = KlTime.date(event.startsAt)
                val (ridesPair, drivingRes) = coroutineScope {
                    val passengerDef = async {
                        if (!canRides || (event.myRide == null && event.myReturnRide == null)) {
                            null to null
                        } else {
                            val upcoming = try {
                                container.rides.upcoming()
                            } catch (_: ApiException) {
                                emptyList()
                            }
                            var out = event.myRide?.id?.let { rid -> upcoming.find { it.id == rid } }
                            var ret = event.myReturnRide?.id?.let { rid -> upcoming.find { it.id == rid } }
                            if ((event.myRide != null && out == null) ||
                                (event.myReturnRide != null && ret == null)
                            ) {
                                val past = try {
                                    container.rides.past()
                                } catch (_: ApiException) {
                                    emptyList()
                                }
                                if (out == null) {
                                    out = event.myRide?.id?.let { rid -> past.find { it.id == rid } }
                                }
                                if (ret == null) {
                                    ret = event.myReturnRide?.id?.let { rid -> past.find { it.id == rid } }
                                }
                            }
                            out to ret
                        }
                    }
                    val driverDef = async {
                        if (!canDrive || day.length != 10) {
                            emptyList()
                        } else {
                            try {
                                container.rides.drivingToday(day, day).filter { it.event?.id == event.id }
                            } catch (_: ApiException) {
                                emptyList()
                            }
                        }
                    }
                    passengerDef.await() to driverDef.await()
                }
                outbound = ridesPair.first
                returnRide = ridesPair.second
                driving = drivingRes.sortedBy { KlTime.sortKey(it.pickupAt) }
            }

            val signupsList = if (event.canSeeSignups) {
                try {
                    container.events.signups(id)
                } catch (_: ApiException) {
                    emptyList()
                }
            } else {
                emptyList()
            }
            val canVisitorSection = event.status == "published" &&
                (event.mySignup?.status == "going" || event.canManage) &&
                container.auth.me?.can("rides", "create") == true
            val visitorsList = if (canVisitorSection) {
                try {
                    container.events.visitors(id)
                } catch (_: ApiException) {
                    emptyList()
                }
            } else {
                emptyList()
            }
            val areasList = if (canVisitorSection) {
                try {
                    container.rides.areas()
                } catch (_: ApiException) {
                    emptyList()
                }
            } else {
                emptyList()
            }

            _state.value = _state.value.copy(
                loading = false,
                refreshing = false,
                event = event,
                outboundRide = outbound,
                returnRide = returnRide,
                drivingForEvent = driving,
                signups = signupsList,
                visitors = visitorsList,
                areas = areasList,
                error = null,
            )
        } catch (e: ApiException) {
            _state.value = _state.value.copy(loading = false, refreshing = false, error = e.message)
        }
    }

    fun signup(
        needRide: Boolean,
        offerDrive: Boolean,
        asGroupId: Long?,
        offerSeats: Int? = null,
        selfTransport: Boolean? = null,
    ) {
        if (_state.value.busy) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.signup(id, needRide, offerDrive, asGroupId, offerSeats, selfTransport)
                reloadDetail()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun setTransport(selfTransport: Boolean) {
        if (_state.value.busy) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.setSelfTransport(id, selfTransport)
                reloadDetail()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun withdraw() {
        if (_state.value.busy) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.withdraw(id)
                reloadDetail()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun publish() {
        if (_state.value.busy) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null, actionNotice = null)
        viewModelScope.launch {
            try {
                container.events.publish(id)
                reloadDetail()
                _state.value = _state.value.copy(actionNotice = EventActionNotice.PUBLISHED)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun unpublish() {
        if (_state.value.busy) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null, actionNotice = null)
        viewModelScope.launch {
            try {
                container.events.unpublish(id)
                reloadDetail()
                _state.value = _state.value.copy(actionNotice = EventActionNotice.UNPUBLISHED)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun selfCheckin(code: String?) {
        if (_state.value.busy) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null, checkinError = null, checkinResult = null)
        viewModelScope.launch {
            try {
                val res = container.events.selfCheckin(id, code?.trim()?.ifBlank { null })
                reloadDetail()
                _state.value = _state.value.copy(checkinResult = res)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(checkinError = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun scanMemberPass(passCode: String) {
        val clean = passCode.trim()
        if (_state.value.busy || clean.isEmpty()) return
        loadJob?.cancel()
        _state.value = _state.value.copy(busy = true, message = null, checkinError = null, scanMemberResult = null)
        viewModelScope.launch {
            try {
                val res = container.events.scanMemberPass(id, clean)
                reloadDetail()
                _state.value = _state.value.copy(scanMemberResult = res)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(checkinError = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun clearMessage() { _state.value = _state.value.copy(message = null) }

    fun clearCheckinError() { _state.value = _state.value.copy(checkinError = null) }

    fun clearNotice() { _state.value = _state.value.copy(actionNotice = null) }

    fun clearCheckinResults() {
        _state.value = _state.value.copy(checkinResult = null, scanMemberResult = null, checkinError = null)
    }

    fun dismissVisitorShare() {
        _state.value = _state.value.copy(visitorSharePair = null)
    }

    fun finishEvent(doneMsg: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.finish(id)
                reloadDetail()
                _state.value = _state.value.copy(message = doneMsg)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun cancelEvent(doneMsg: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.cancel(id)
                reloadDetail()
                _state.value = _state.value.copy(message = doneMsg)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun resetCheckinCode(doneMsg: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                val res = container.events.resetCheckinCode(id)
                _state.value = _state.value.copy(
                    event = _state.value.event?.copy(checkinCode = res.checkinCode),
                    message = doneMsg,
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun markAttendance(userId: Long, attended: Boolean?) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.markAttendance(id, userId, attended)
                reloadDetail()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun searchWalkInMembers(query: String) {
        viewModelScope.launch {
            val res = container.admin.users(
                q = query.trim().takeIf { it.isNotEmpty() },
                status = "active",
                limit = 15,
            )
            res.onSuccess { page ->
                _state.value = _state.value.copy(walkInCandidates = page.items)
            }
        }
    }

    fun createVisitor(req: CreateVisitorPassReq) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                val res = container.events.createVisitor(id, req)
                reloadDetail()
                _state.value = _state.value.copy(visitorSharePair = req.name to container.webUrl(res.url))
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun reissueVisitor(pass: VisitorPassDto) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                val res = container.events.reissueVisitor(pass.id)
                reloadDetail()
                _state.value = _state.value.copy(visitorSharePair = pass.name to container.webUrl(res.url))
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun revokeVisitor(passId: Long) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.events.revokeVisitor(passId)
                reloadDetail()
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun loadDriveRequests() {
        _state.value = _state.value.copy(driveRequestsLoading = true)
        viewModelScope.launch {
            try {
                val list = container.rides.driverRequests(eventId = id)
                _state.value = _state.value.copy(driveRequests = list, driveRequestsLoading = false)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(driveRequestsLoading = false, message = e.message)
            }
        }
    }

    fun claimDriveRequest(rideId: Long, doneMsg: String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                container.rides.claimRide(rideId)
                val list = container.rides.driverRequests(eventId = id)
                reloadDetail()
                _state.value = _state.value.copy(driveRequests = list, message = doneMsg)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun openPostEventReimb() {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                val resp = container.events.postEventDrivers(id)
                _state.value = _state.value.copy(postEventDrivers = resp, postEventOpen = true)
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    fun closePostEventReimb() {
        _state.value = _state.value.copy(postEventOpen = false)
    }

    fun submitPostEventReimb(req: PostEventReimbursementsReq, formatDone: (Int, Long) -> String) {
        if (_state.value.busy) return
        _state.value = _state.value.copy(busy = true, message = null)
        viewModelScope.launch {
            try {
                val resp = container.events.createPostEventReimbursements(id, req)
                reloadDetail()
                _state.value = _state.value.copy(
                    postEventOpen = false,
                    message = formatDone(resp.createdCount, resp.totalCents),
                )
            } catch (e: ApiException) {
                _state.value = _state.value.copy(message = e.message)
            } finally {
                _state.value = _state.value.copy(busy = false)
            }
        }
    }

    companion object {
        fun factory(container: AppContainer, id: Long) = viewModelFactory {
            initializer { EventDetailViewModel(container, id) }
        }
    }
}
