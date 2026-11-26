package org.pjstatecm.graceride.ui.screens.admin

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
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
import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.AdminInviteDto
import org.pjstatecm.graceride.data.api.AdminRoleDefDto
import org.pjstatecm.graceride.data.api.AdminUserDto
import org.pjstatecm.graceride.data.api.AdminUsersPageDto
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.RecoveryRequestDto
import org.pjstatecm.graceride.data.api.UpdateAdminUserReq
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.MemberAdminRules
import org.pjstatecm.graceride.notifications.RealtimeEvent

enum class AdminMembersNotice {
    MEMBER_APPROVED,
    MEMBER_UPDATED,
    INVITE_CREATED,
    INVITE_REVOKED,
    RECOVERY_APPROVED,
    RECOVERY_AWAITING_SECOND,
    RECOVERY_REJECTED,
}

data class AdminMembersUiState(
    val tab: String = "members",
    val me: Me? = null,
    val stats: AdminDashboardStatsDto? = null,
    val refreshing: Boolean = false,
    val notice: AdminMembersNotice? = null,
    val batchSuccessCount: Int? = null,
    val batchFailureCount: Int = 0,
    val messageText: String? = null,

    // Members tab
    val query: String = "",
    val statusFilter: String = "",
    val roleFilter: String = "",
    val sortKey: String = "name",
    val sortDir: String = "asc",
    val page: Int = 1,
    val usersPage: AdminUsersPageDto = AdminUsersPageDto(),
    val membersLoading: Boolean = false,
    val membersError: String? = null,
    val roles: List<AdminRoleDefDto> = emptyList(),
    val groups: List<GroupDto> = emptyList(),
    val groupsLoaded: Boolean = false,
    val selectedUserIds: Set<Long> = emptySet(),
    val savingUser: Boolean = false,
    val saveError: String? = null,
    val batchBusy: Boolean = false,

    // Invites tab
    val invites: List<AdminInviteDto> = emptyList(),
    val invitesLoading: Boolean = false,
    val invitesError: String? = null,
    val inviteBusy: Boolean = false,
    val revokingInviteId: Long? = null,
    val lastCreatedInviteUrl: String? = null,

    // Recovery tab
    val recovery: List<RecoveryRequestDto> = emptyList(),
    val recoveryLoading: Boolean = false,
    val recoveryError: String? = null,
    val recoveryBusyId: Long? = null,
    val lastRecoveryUrl: String? = null,
)

class AdminMembersViewModel(
    private val container: AppContainer,
    initialTab: String = "members",
    initialStatus: String? = null,
) : ViewModel() {

    private val _state = MutableStateFlow(
        AdminMembersUiState(
            me = container.auth.me,
            tab = resolvePermittedTab(initialTab, container.auth.me),
            statusFilter = initialStatus.orEmpty(),
        ),
    )
    val state: StateFlow<AdminMembersUiState> = _state.asStateFlow()
    private var loadJob: Job? = null

    init {
        loadInitial()
        viewModelScope.launch {
            container.realtimeStream.events.collect { ev ->
                if (ev != RealtimeEvent.Resume && ev != RealtimeEvent.Chat) {
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
        var tabChanged = false
        _state.update { cur ->
            val currentTab = resolvePermittedTab(cur.tab, me)
            tabChanged = currentTab != cur.tab
            if (cur.me != me || tabChanged) {
                cur.copy(me = me, tab = currentTab)
            } else {
                cur
            }
        }
        if (tabChanged) refreshActive(silent = false)
    }

    private fun loadInitial() {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            coroutineScope {
                val statsDef = async { refreshStats(passive = true) }
                val metaDef = async { loadMetadataIfNeeded(force = false) }
                val tabDef = async { loadForTab(_state.value.tab, silent = false) }
                statsDef.await()
                metaDef.await()
                tabDef.await()
            }
        }
    }

    fun setTab(tab: String) {
        val allowed = resolvePermittedTab(tab, _state.value.me ?: container.auth.me)
        if (allowed == _state.value.tab) return
        _state.update { it.copy(tab = allowed) }
        refreshActive(silent = false)
    }

    fun refreshActive(silent: Boolean = false) {
        loadJob?.cancel()
        loadJob = viewModelScope.launch {
            if (!silent) _state.update { it.copy(refreshing = true) }
            refreshStats(passive = silent)
            if (_state.value.tab == "members") {
                loadMetadataIfNeeded(force = !silent)
            }
            loadForTab(_state.value.tab, silent = silent)
            if (!silent) _state.update { it.copy(refreshing = false) }
        }
    }

    private suspend fun refreshStats(passive: Boolean) {
        val me = _state.value.me ?: container.auth.me ?: return
        if (me.can("admin_dashboard")) {
            container.admin.dashboardStats(passive = passive).onSuccess { stats ->
                _state.update { it.copy(stats = stats) }
            }
        }
    }

    private suspend fun loadMetadataIfNeeded(force: Boolean) {
        val me = _state.value.me ?: container.auth.me ?: return
        if (!me.can("admin_members")) return
        val needRoles = force || _state.value.roles.isEmpty()
        val needGroups = force || !_state.value.groupsLoaded
        if (!needRoles && !needGroups) return

        coroutineScope {
            val rolesDef = if (needRoles && (me.can("admin_roles") || me.can("admin_members"))) {
                async { container.admin.roles() }
            } else {
                null
            }
            val groupsDef = if (needGroups) {
                async { container.admin.groups() }
            } else {
                null
            }
            rolesDef?.await()?.onSuccess { resp ->
                _state.update { it.copy(roles = resp.roles) }
            }
            groupsDef?.await()?.onSuccess { list ->
                _state.update { it.copy(groups = list, groupsLoaded = true) }
            }
        }
    }

    private suspend fun loadForTab(tab: String, silent: Boolean) {
        when (tab) {
            "members" -> loadUsers(page = _state.value.page, silent = silent)
            "invites" -> loadInvites(silent = silent)
            "recovery" -> loadRecovery(silent = silent)
        }
    }

    private suspend fun loadUsers(page: Int, silent: Boolean) {
        val me = _state.value.me ?: container.auth.me
        if (me?.can("admin_members") != true) return
        if (!silent) {
            _state.update { it.copy(membersLoading = true, membersError = null) }
        }
        val st = _state.value
        container.admin.users(
            q = st.query,
            role = st.roleFilter,
            status = st.statusFilter,
            sort = st.sortKey,
            dir = st.sortDir,
            page = page,
            limit = PAGE_SIZE,
        ).onSuccess { resp ->
            _state.update {
                it.copy(
                    page = resp.page,
                    usersPage = resp,
                    membersLoading = false,
                    membersError = null,
                )
            }
        }.onFailure { err ->
            _state.update { cur ->
                cur.copy(
                    membersLoading = false,
                    membersError = if (!silent || cur.usersPage.items.isEmpty()) err.message else null,
                )
            }
        }
    }

    private suspend fun loadInvites(silent: Boolean) {
        val me = _state.value.me ?: container.auth.me
        if (me?.can("admin_invites") != true) return
        if (!silent) {
            _state.update { it.copy(invitesLoading = true, invitesError = null) }
        }
        container.admin.invites()
            .onSuccess { list ->
                _state.update {
                    it.copy(
                        invites = list,
                        invitesLoading = false,
                        invitesError = null,
                    )
                }
            }
            .onFailure { err ->
                _state.update { cur ->
                    cur.copy(
                        invitesLoading = false,
                        invitesError = if (!silent || cur.invites.isEmpty()) err.message else null,
                    )
                }
            }
    }

    private suspend fun loadRecovery(silent: Boolean) {
        val me = _state.value.me ?: container.auth.me
        if (me?.can("admin_recovery") != true) return
        if (!silent) {
            _state.update { it.copy(recoveryLoading = true, recoveryError = null) }
        }
        container.admin.recoveryRequests()
            .onSuccess { list ->
                _state.update {
                    it.copy(
                        recovery = list,
                        recoveryLoading = false,
                        recoveryError = null,
                    )
                }
            }
            .onFailure { err ->
                _state.update { cur ->
                    cur.copy(
                        recoveryLoading = false,
                        recoveryError = if (!silent || cur.recovery.isEmpty()) err.message else null,
                    )
                }
            }
    }

    // ---- Member filter, search, sort & pagination ----

    fun setQuery(q: String) {
        _state.update { it.copy(query = q) }
    }

    fun submitSearch() {
        _state.update { it.copy(page = 1, selectedUserIds = emptySet()) }
        loadJob?.cancel()
        loadJob = viewModelScope.launch { loadUsers(page = 1, silent = false) }
    }

    fun clearQuery() {
        if (_state.value.query.isEmpty()) return
        _state.update { it.copy(query = "", page = 1, selectedUserIds = emptySet()) }
        loadJob?.cancel()
        loadJob = viewModelScope.launch { loadUsers(page = 1, silent = false) }
    }

    fun setStatusFilter(status: String) {
        if (_state.value.statusFilter == status) return
        _state.update { it.copy(statusFilter = status, page = 1, selectedUserIds = emptySet()) }
        loadJob?.cancel()
        loadJob = viewModelScope.launch { loadUsers(page = 1, silent = false) }
    }

    fun setRoleFilter(role: String) {
        if (_state.value.roleFilter == role) return
        _state.update { it.copy(roleFilter = role, page = 1, selectedUserIds = emptySet()) }
        loadJob?.cancel()
        loadJob = viewModelScope.launch { loadUsers(page = 1, silent = false) }
    }

    fun setSort(sortKey: String, sortDir: String) {
        if (_state.value.sortKey == sortKey && _state.value.sortDir == sortDir) return
        _state.update { it.copy(sortKey = sortKey, sortDir = sortDir, page = 1, selectedUserIds = emptySet()) }
        loadJob?.cancel()
        loadJob = viewModelScope.launch { loadUsers(page = 1, silent = false) }
    }

    fun clearFilters() {
        _state.update {
            it.copy(
                query = "",
                statusFilter = "",
                roleFilter = "",
                page = 1,
                selectedUserIds = emptySet(),
            )
        }
        loadJob?.cancel()
        loadJob = viewModelScope.launch { loadUsers(page = 1, silent = false) }
    }

    fun goToPage(page: Int) {
        val target = page.coerceIn(1, _state.value.usersPage.totalPages.coerceAtLeast(1))
        if (target == _state.value.page) return
        _state.update { it.copy(page = target, selectedUserIds = emptySet()) }
        loadJob?.cancel()
        loadJob = viewModelScope.launch { loadUsers(page = target, silent = false) }
    }

    fun toggleSelectUser(userId: Long) {
        _state.update { cur ->
            val current = cur.selectedUserIds
            cur.copy(
                selectedUserIds = if (userId in current) current - userId else current + userId,
            )
        }
    }

    fun selectAllEditableOnPage() {
        _state.update { cur ->
            val me = cur.me ?: container.auth.me
            val ids = cur.usersPage.items
                .filter { MemberAdminRules.isEditable(me, it) }
                .map { it.id }
                .toSet()
            cur.copy(selectedUserIds = ids)
        }
    }

    fun clearSelection() {
        _state.update { it.copy(selectedUserIds = emptySet()) }
    }

    // ---- Member Edit / Approve / Batch Approve ----

    fun clearSaveError() {
        _state.update { it.copy(saveError = null) }
    }

    fun saveUser(
        user: AdminUserDto,
        roles: List<String>,
        status: String,
        groupIds: List<Long>?,
        presidentAcknowledged: Boolean,
        isApproveMode: Boolean,
        onSuccess: () -> Unit,
    ) {
        if (_state.value.savingUser) return
        viewModelScope.launch {
            _state.update { it.copy(savingUser = true, saveError = null) }
            val safeGroupIds = if (_state.value.groupsLoaded) groupIds else null
            container.admin.updateUser(
                id = user.id,
                req = UpdateAdminUserReq(
                    roles = roles,
                    status = status,
                    group_ids = safeGroupIds,
                    president_acknowledged = presidentAcknowledged,
                ),
            ).onSuccess {
                onSuccess()
                _state.update {
                    it.copy(
                        savingUser = false,
                        saveError = null,
                        notice = if (isApproveMode) AdminMembersNotice.MEMBER_APPROVED else AdminMembersNotice.MEMBER_UPDATED,
                    )
                }
                refreshStats(passive = true)
                loadUsers(page = _state.value.page, silent = false)
            }.onFailure { err ->
                _state.update {
                    it.copy(
                        savingUser = false,
                        saveError = err.message,
                    )
                }
            }
        }
    }

    fun batchApprove(
        users: List<AdminUserDto>,
        roleKey: String,
        groupIds: List<Long>?,
        groupsTouched: Boolean,
        presidentAcknowledged: Boolean,
        onDone: () -> Unit,
    ) {
        if (_state.value.batchBusy || users.isEmpty()) return
        val me = _state.value.me ?: container.auth.me
        val canAssign = me?.can("admin_roles", "update") == true
        val rolesCatalog = _state.value.roles
        val groupsLoaded = _state.value.groupsLoaded
        viewModelScope.launch {
            _state.update { it.copy(batchBusy = true, saveError = null) }
            val payloadRoles = if (roleKey == "member") emptyList() else listOf(roleKey)
            var okCount = 0
            var failCount = 0
            var firstError: String? = null
            val succeededIds = HashSet<Long>()
            for (u in users) {
                val targetRoles = if (canAssign) payloadRoles else u.roles
                val promotingAdmin = canAssign && MemberAdminRules.promotingToAdmin(u.roles, targetRoles, rolesCatalog)
                val res = container.admin.updateUser(
                    id = u.id,
                    req = UpdateAdminUserReq(
                        roles = targetRoles,
                        status = "active",
                        group_ids = if (groupsLoaded && (groupsTouched || u.status == "pending")) groupIds else null,
                        president_acknowledged = presidentAcknowledged && promotingAdmin,
                    ),
                )
                res.onSuccess {
                    okCount++
                    succeededIds.add(u.id)
                }.onFailure { err ->
                    failCount++
                    if (firstError == null) firstError = err.message
                }
            }
            if (okCount > 0) {
                onDone()
            }
            _state.update { cur ->
                cur.copy(
                    batchBusy = false,
                    selectedUserIds = if (failCount == 0) emptySet() else cur.selectedUserIds - succeededIds,
                    batchSuccessCount = okCount,
                    batchFailureCount = failCount,
                    messageText = firstError,
                    saveError = if (okCount == 0) firstError else null,
                )
            }
            if (okCount > 0) {
                refreshStats(passive = true)
                loadUsers(page = _state.value.page, silent = false)
            }
        }
    }

    // ---- Invites ----

    fun createInvite() {
        if (_state.value.inviteBusy) return
        viewModelScope.launch {
            _state.update { it.copy(inviteBusy = true) }
            container.admin.createInvite()
                .onSuccess { res ->
                    _state.update {
                        it.copy(
                            inviteBusy = false,
                            lastCreatedInviteUrl = res.url.takeIf { url -> url.isNotBlank() },
                            notice = AdminMembersNotice.INVITE_CREATED,
                        )
                    }
                    refreshStats(passive = true)
                    loadInvites(silent = false)
                }
                .onFailure { err ->
                    _state.update {
                        it.copy(
                            inviteBusy = false,
                            messageText = err.message,
                        )
                    }
                }
        }
    }

    fun revokeInvite(id: Long) {
        if (_state.value.revokingInviteId != null) return
        viewModelScope.launch {
            _state.update { it.copy(revokingInviteId = id) }
            container.admin.revokeInvite(id)
                .onSuccess {
                    _state.update {
                        it.copy(
                            revokingInviteId = null,
                            notice = AdminMembersNotice.INVITE_REVOKED,
                        )
                    }
                    refreshStats(passive = true)
                    loadInvites(silent = false)
                }
                .onFailure { err ->
                    _state.update {
                        it.copy(
                            revokingInviteId = null,
                            messageText = err.message,
                        )
                    }
                }
        }
    }

    fun dismissCreatedInviteUrl() {
        _state.update { it.copy(lastCreatedInviteUrl = null) }
    }

    // ---- Recovery ----

    fun approveRecovery(id: Long) {
        if (_state.value.recoveryBusyId != null) return
        viewModelScope.launch {
            _state.update { it.copy(recoveryBusyId = id) }
            container.admin.approveRecovery(id)
                .onSuccess { res ->
                    _state.update { cur ->
                        cur.copy(
                            recoveryBusyId = null,
                            lastRecoveryUrl = res.url?.takeIf { url -> url.isNotBlank() } ?: cur.lastRecoveryUrl,
                            notice = when {
                                res.awaitingSecond -> AdminMembersNotice.RECOVERY_AWAITING_SECOND
                                !res.url.isNullOrBlank() -> AdminMembersNotice.RECOVERY_APPROVED
                                else -> AdminMembersNotice.RECOVERY_APPROVED
                            },
                        )
                    }
                    refreshStats(passive = true)
                    loadRecovery(silent = false)
                }
                .onFailure { err ->
                    _state.update {
                        it.copy(
                            recoveryBusyId = null,
                            messageText = err.message,
                        )
                    }
                }
        }
    }

    fun rejectRecovery(id: Long) {
        if (_state.value.recoveryBusyId != null) return
        viewModelScope.launch {
            _state.update { it.copy(recoveryBusyId = id) }
            container.admin.rejectRecovery(id)
                .onSuccess {
                    _state.update {
                        it.copy(
                            recoveryBusyId = null,
                            notice = AdminMembersNotice.RECOVERY_REJECTED,
                        )
                    }
                    refreshStats(passive = true)
                    loadRecovery(silent = false)
                }
                .onFailure { err ->
                    _state.update {
                        it.copy(
                            recoveryBusyId = null,
                            messageText = err.message,
                        )
                    }
                }
        }
    }

    fun dismissRecoveryUrl() {
        _state.update { it.copy(lastRecoveryUrl = null) }
    }

    fun clearNotices() {
        _state.update {
            it.copy(
                notice = null,
                batchSuccessCount = null,
                batchFailureCount = 0,
                messageText = null,
            )
        }
    }

    companion object {
        const val PAGE_SIZE = 50

        fun resolvePermittedTab(requested: String, me: Me?): String {
            if (me == null) return requested
            val ordered = buildList {
                if (me.can("admin_members")) add("members")
                if (me.can("admin_invites")) add("invites")
                if (me.can("admin_recovery")) add("recovery")
            }
            return if (requested in ordered) requested else ordered.firstOrNull() ?: "members"
        }

        fun factory(
            container: AppContainer,
            initialTab: String = "members",
            initialStatus: String? = null,
        ): ViewModelProvider.Factory = viewModelFactory {
            initializer {
                AdminMembersViewModel(
                    container = container,
                    initialTab = initialTab,
                    initialStatus = initialStatus,
                )
            }
        }
    }
}
