package org.pjstatecm.graceride.ui.screens.announcements

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.CreateAnnouncementReq
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.UpdateAnnouncementReq
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.MemberAdminRules
import org.pjstatecm.graceride.notifications.RealtimeEvent

enum class AnnouncementNotice {
    PUBLISHED,
    UPDATED,
    PINNED,
    UNPINNED,
    DELETED,
}

data class AnnouncementsUiState(
    val manageMode: Boolean = false,
    val me: Me? = null,
    val items: List<Announcement> = emptyList(),
    val groups: List<GroupDto> = emptyList(),
    val query: String = "",
    val pinnedOnly: Boolean = false,
    val loading: Boolean = true,
    val refreshing: Boolean = false,
    val error: String? = null,
    val busy: Boolean = false,
    val busyId: Long? = null,
    val actionError: String? = null,
    val notice: AnnouncementNotice? = null,
)

class AnnouncementsViewModel(
    private val container: AppContainer,
    initialManage: Boolean = false,
) : ViewModel() {

    private val _state = MutableStateFlow(
        AnnouncementsUiState(
            me = container.auth.me,
            manageMode = initialManage && container.auth.me?.can("admin_announcements") == true,
        ),
    )
    val state: StateFlow<AnnouncementsUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        load(silent = false)
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) {
                    load(silent = true)
                }
            }
        }
        viewModelScope.launch {
            container.networkMonitor.isOnline.drop(1).filter { it }.collect {
                load(silent = true)
            }
        }
    }

    fun syncMe(me: Me?) {
        _state.update { cur ->
            val allowedManage = cur.manageMode && (me?.can("admin_announcements") == true)
            if (cur.me != me || allowedManage != cur.manageMode) {
                cur.copy(me = me, manageMode = allowedManage)
            } else {
                cur
            }
        }
    }

    fun refresh() = load(silent = false)

    fun setManageMode(enabled: Boolean) {
        val me = _state.value.me ?: container.auth.me
        val next = enabled && (me?.can("admin_announcements") == true)
        if (next == _state.value.manageMode) return
        _state.update { it.copy(manageMode = next) }
        load(silent = false)
    }

    fun setQuery(q: String) {
        _state.update { it.copy(query = q) }
    }

    fun setPinnedOnly(pinnedOnly: Boolean) {
        _state.update { it.copy(pinnedOnly = pinnedOnly) }
    }

    fun load(silent: Boolean) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            val me = _state.value.me ?: container.auth.me
            val manage = _state.value.manageMode && (me?.can("admin_announcements") == true)
            if (!silent) {
                _state.update { cur ->
                    cur.copy(
                        loading = cur.items.isEmpty(),
                        refreshing = cur.items.isNotEmpty(),
                        error = null,
                    )
                }
            }
            try {
                val (list, groupsList) = coroutineScope {
                    val annDef = async {
                        container.community.announcements(manage = if (manage) true else null)
                    }
                    val groupsDef = if (me?.can("admin_announcements", "create") == true) {
                        async { runCatching { container.community.groups() }.getOrNull() }
                    } else {
                        null
                    }
                    annDef.await() to groupsDef?.await()
                }
                _state.update { cur ->
                    cur.copy(
                        items = list,
                        groups = groupsList ?: cur.groups,
                        loading = false,
                        refreshing = false,
                        error = null,
                    )
                }
            } catch (ce: CancellationException) {
                throw ce
            } catch (e: ApiException) {
                _state.update { cur ->
                    cur.copy(
                        loading = false,
                        refreshing = false,
                        error = if (!silent || cur.items.isEmpty()) e.message else null,
                    )
                }
            }
        }
    }

    fun clearActionError() {
        _state.update { it.copy(actionError = null) }
    }

    fun clearNotice() {
        _state.update { it.copy(notice = null, actionError = null) }
    }

    fun createAnnouncement(
        title: String,
        body: String,
        pinned: Boolean,
        notifyAll: Boolean,
        groupIds: List<Long>,
        onSuccess: () -> Unit,
    ) {
        if (_state.value.busy) return
        viewModelScope.launch {
            _state.update { it.copy(busy = true, actionError = null) }
            try {
                container.community.createAnnouncement(
                    CreateAnnouncementReq(
                        title = title.trim(),
                        body = body.trim(),
                        pinned = pinned,
                        notify_all = notifyAll,
                        group_ids = groupIds,
                    ),
                )
                val updated = refreshAfterMutation()
                onSuccess()
                _state.update { cur ->
                    cur.copy(
                        items = updated ?: cur.items,
                        busy = false,
                        actionError = null,
                        notice = AnnouncementNotice.PUBLISHED,
                    )
                }
            } catch (ce: CancellationException) {
                throw ce
            } catch (e: ApiException) {
                _state.update { it.copy(busy = false, actionError = e.message) }
            }
        }
    }

    fun updateAnnouncement(
        item: Announcement,
        title: String,
        body: String,
        pinned: Boolean,
        groupIds: List<Long>,
        onSuccess: () -> Unit,
    ) {
        if (_state.value.busy) return
        viewModelScope.launch {
            _state.update { it.copy(busy = true, actionError = null) }
            val originalIds = item.groups.map { it.id }
            val changed = MemberAdminRules.announcementAudienceChanged(originalIds, groupIds)
            try {
                container.community.updateAnnouncement(
                    id = item.id,
                    req = UpdateAnnouncementReq(
                        title = title.trim(),
                        body = body.trim(),
                        pinned = pinned,
                        group_ids = if (changed) groupIds else null,
                    ),
                )
                val updated = refreshAfterMutation()
                onSuccess()
                _state.update { cur ->
                    cur.copy(
                        items = updated ?: cur.items,
                        busy = false,
                        actionError = null,
                        notice = AnnouncementNotice.UPDATED,
                    )
                }
            } catch (ce: CancellationException) {
                throw ce
            } catch (e: ApiException) {
                _state.update { it.copy(busy = false, actionError = e.message) }
            }
        }
    }

    fun togglePin(item: Announcement) {
        if (_state.value.busyId != null || _state.value.busy) return
        val nextPinned = !item.pinned
        viewModelScope.launch {
            _state.update { it.copy(busyId = item.id, actionError = null) }
            try {
                container.community.updateAnnouncement(
                    id = item.id,
                    req = UpdateAnnouncementReq(
                        title = item.title,
                        body = item.body,
                        pinned = nextPinned,
                        group_ids = null,
                    ),
                )
                val updated = refreshAfterMutation()
                _state.update { cur ->
                    cur.copy(
                        items = updated ?: cur.items,
                        busyId = null,
                        notice = if (nextPinned) AnnouncementNotice.PINNED else AnnouncementNotice.UNPINNED,
                    )
                }
            } catch (ce: CancellationException) {
                throw ce
            } catch (e: ApiException) {
                _state.update { it.copy(busyId = null, actionError = e.message) }
            }
        }
    }

    fun deleteAnnouncement(id: Long) {
        if (_state.value.busyId != null || _state.value.busy) return
        viewModelScope.launch {
            _state.update { it.copy(busyId = id, actionError = null) }
            try {
                container.community.deleteAnnouncement(id)
                val updated = refreshAfterMutation()
                _state.update { cur ->
                    cur.copy(
                        items = updated ?: cur.items.filterNot { it.id == id },
                        busyId = null,
                        notice = AnnouncementNotice.DELETED,
                    )
                }
            } catch (ce: CancellationException) {
                throw ce
            } catch (e: ApiException) {
                _state.update { it.copy(busyId = null, actionError = e.message) }
            }
        }
    }

    private suspend fun refreshAfterMutation(): List<Announcement>? {
        val me = _state.value.me ?: container.auth.me
        val manage = _state.value.manageMode && (me?.can("admin_announcements") == true)
        return try {
            container.community.announcements(manage = if (manage) true else null)
        } catch (ce: CancellationException) {
            throw ce
        } catch (_: Exception) {
            null
        }
    }

    companion object {
        fun factory(
            container: AppContainer,
            manage: Boolean = false,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer {
                AnnouncementsViewModel(container = container, initialManage = manage)
            }
        }
    }
}
